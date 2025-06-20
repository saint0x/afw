use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::{wrappers::ReceiverStream, Stream};
use tonic::{Request, Response, Status};
use uuid::Uuid;

use super::aria::{
    notification_service_server::NotificationService,
    Notification, StreamNotificationsRequest,
    BundleUploadEvent, TaskStatusEvent, TaskStatus,
};

use crate::database::DatabaseManager;
use crate::errors::{AriaError, AriaResult};

/// Implementation of the high-level NotificationService
pub struct NotificationServiceImpl {
    database: Arc<DatabaseManager>,
    // Channel for broadcasting notifications to subscribers
    notification_broadcaster: Arc<Mutex<tokio::sync::broadcast::Sender<Notification>>>,
}

impl NotificationServiceImpl {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(1000);
        Self { 
            database,
            notification_broadcaster: Arc::new(Mutex::new(tx)),
        }
    }

    /// Store notification in database (simplified for event-based notifications)
    async fn store_notification(&self, notification: &Notification) -> AriaResult<()> {
        let query = r#"
            INSERT INTO notifications (id, timestamp, event_type, event_data)
            VALUES (?, ?, ?, ?)
        "#;
        
        let timestamp = notification.timestamp.as_ref()
            .map(|ts| ts.seconds)
            .unwrap_or_else(|| chrono::Utc::now().timestamp());
        
        let (event_type, event_data) = match &notification.event_payload {
            Some(super::aria::notification::EventPayload::BundleUpload(event)) => {
                ("bundle_upload", format!("bundle_name: {}, progress: {}, success: {}", 
                    event.bundle_name, event.progress_percent, event.success))
            }
            Some(super::aria::notification::EventPayload::TaskStatus(event)) => {
                ("task_status", format!("task_id: {}, status: {}, message: {}", 
                    event.task_id, event.new_status, event.status_message))
            }
            None => ("unknown", "{}".to_string()),
        };
        
        let pool = self.database.pool().await?;
        sqlx::query(query)
            .bind(&notification.id)
            .bind(timestamp)
            .bind(event_type)
            .bind(event_data)
            .execute(&pool)
            .await
            .map_err(|e| AriaError::database_error(&format!("Failed to store notification: {}", e)))?;
        
        Ok(())
    }

    /// Broadcast a notification to all subscribers
    async fn broadcast_notification(&self, notification: &Notification) {
        let broadcaster = self.notification_broadcaster.lock().await;
        if let Err(e) = broadcaster.send(notification.clone()) {
            tracing::warn!("Failed to broadcast notification: {}", e);
        }
    }

    /// Create a bundle upload notification
    pub async fn notify_bundle_upload(
        &self,
        bundle_name: String,
        progress_percent: f64,
        status_message: String,
        success: bool,
        error_message: Option<String>,
    ) -> AriaResult<()> {
        let event = BundleUploadEvent {
            bundle_name,
            progress_percent,
            status_message,
            success,
            error_message,
        };

        let notification = Notification {
            id: Uuid::new_v4().to_string(),
            timestamp: Some(prost_types::Timestamp {
                seconds: chrono::Utc::now().timestamp(),
                nanos: 0,
            }),
            event_payload: Some(super::aria::notification::EventPayload::BundleUpload(event)),
        };

        // Store in database
        self.store_notification(&notification).await?;
        
        // Broadcast to subscribers
        self.broadcast_notification(&notification).await;
        
        Ok(())
    }

    /// Create a task status notification
    pub async fn notify_task_status(
        &self,
        task_id: String,
        new_status: TaskStatus,
        status_message: String,
        exit_code: Option<i32>,
    ) -> AriaResult<()> {
        let event = TaskStatusEvent {
            task_id,
            new_status: new_status as i32,
            status_message,
            exit_code,
        };

        let notification = Notification {
            id: Uuid::new_v4().to_string(),
            timestamp: Some(prost_types::Timestamp {
                seconds: chrono::Utc::now().timestamp(),
                nanos: 0,
            }),
            event_payload: Some(super::aria::notification::EventPayload::TaskStatus(event)),
        };

        // Store in database
        self.store_notification(&notification).await?;
        
        // Broadcast to subscribers
        self.broadcast_notification(&notification).await;
        
        Ok(())
    }
}

#[tonic::async_trait]
impl NotificationService for NotificationServiceImpl {
    async fn stream_notifications(
        &self,
        request: Request<StreamNotificationsRequest>,
    ) -> Result<Response<Self::StreamNotificationsStream>, Status> {
        let _req = request.into_inner();
        
        tracing::info!("Starting notification stream");
        
        let broadcaster = self.notification_broadcaster.lock().await;
        let mut receiver = broadcaster.subscribe();
        drop(broadcaster); // Release the lock
        
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        
        // Spawn task to forward notifications
        tokio::spawn(async move {
            while let Ok(notification) = receiver.recv().await {
                if tx.send(Ok(notification)).await.is_err() {
                    break; // Client disconnected
                }
            }
        });
        
        let stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream) as Self::StreamNotificationsStream))
    }

    type StreamNotificationsStream = Pin<Box<dyn Stream<Item = Result<Notification, Status>> + Send>>;
} 