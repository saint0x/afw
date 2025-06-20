use std::pin::Pin;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio_stream::{wrappers::ReceiverStream, Stream};
use tonic::{Request, Response, Status};

use super::aria::{
    container_service_server::ContainerService,
    Container, CreateContainerRequest, StartContainerRequest, StartContainerResponse,
    StopContainerRequest, StopContainerResponse, RemoveContainerRequest, RemoveContainerResponse,
    GetContainerRequest, ListContainersRequest, ListContainersResponse,
    StreamContainerLogsRequest, ContainerLog, KeyValuePair, TaskStatus,
};

use crate::engines::container::quilt::QuiltService;
use crate::engines::container::quilt::quilt_proto;
use crate::database::DatabaseManager;
use crate::errors::{AriaError, AriaResult};

/// Implementation of the high-level ContainerService
/// This service wraps the underlying Quilt daemon for container management
pub struct ContainerServiceImpl {
    quilt_service: Arc<Mutex<QuiltService>>,
}

impl ContainerServiceImpl {
    pub fn new(quilt_service: Arc<Mutex<QuiltService>>) -> Self {
        Self { quilt_service }
    }

    /// Convert Quilt ContainerInfo to Aria Container
    fn convert_quilt_container_to_aria(quilt_container: &quilt_proto::ContainerInfo) -> Container {
        let status = match quilt_container.status {
            1 => TaskStatus::Pending,   // PENDING
            2 => TaskStatus::Running,   // RUNNING
            3 => TaskStatus::Completed, // EXITED
            4 => TaskStatus::Failed,    // FAILED
            _ => TaskStatus::Pending,   // Default
        };

        Container {
            id: quilt_container.container_id.clone(),
            user_id: "system".to_string(), // TODO: Extract from context
            session_id: None, // TODO: Link to session if available
            name: format!("container-{}", &quilt_container.container_id[..8]),
            image_path: quilt_container.image_path.clone(),
            status: status as i32,
            created_at: Some(prost_types::Timestamp {
                seconds: quilt_container.created_at as i64,
                nanos: 0,
            }),
        }
    }

    /// Convert KeyValuePair to HashMap
    fn convert_env_vars(env_vars: &[KeyValuePair]) -> HashMap<String, String> {
        env_vars.iter()
            .map(|kv| (kv.key.clone(), kv.value.clone()))
            .collect()
    }
}

#[tonic::async_trait]
impl ContainerService for ContainerServiceImpl {
    async fn create_container(
        &self,
        request: Request<CreateContainerRequest>,
    ) -> Result<Response<Container>, Status> {
        let req = request.into_inner();
        
        tracing::info!("Creating container with image: {}", req.image_path);
        
        // Convert environment variables
        let environment = Self::convert_env_vars(&req.environment);
        
        // Set up default command if none provided
        let command = if req.image_path.contains("node") {
            vec!["node".to_string(), "--version".to_string()]
        } else if req.image_path.contains("python") {
            vec!["python".to_string(), "--version".to_string()]
        } else {
            vec!["sh".to_string(), "-c".to_string(), "echo 'Container ready'".to_string()]
        };
        
        let mut quilt_service = self.quilt_service.lock().await;
        
        match quilt_service.create_container(req.image_path.clone(), command, environment).await {
            Ok(container_id) => {
                tracing::info!("Container created successfully: {}", container_id);
                
                // Get container details to return full Container object
                match quilt_service.get_container_status(container_id.clone()).await {
                    Ok(status) => {
                        let container = Container {
                            id: container_id,
                            user_id: "system".to_string(),
                            session_id: None,
                            name: if req.name.is_empty() { 
                                "unnamed".to_string() 
                            } else { 
                                req.name 
                            },
                            image_path: req.image_path,
                            status: match status.state {
                                crate::types::ContainerState::Created => TaskStatus::Pending as i32,
                                crate::types::ContainerState::Running => TaskStatus::Running as i32,
                                crate::types::ContainerState::Exited => TaskStatus::Completed as i32,
                                crate::types::ContainerState::Failed => TaskStatus::Failed as i32,
                            },
                            created_at: Some(prost_types::Timestamp {
                                seconds: status.created_at as i64,
                                nanos: 0,
                            }),
                        };
                        
                        Ok(Response::new(container))
                    }
                    Err(e) => {
                        tracing::error!("Failed to get container status after creation: {}", e);
                        // Return a basic container object
                        let container = Container {
                            id: container_id,
                            user_id: "system".to_string(),
                            session_id: None,
                            name: if req.name.is_empty() { 
                                "unnamed".to_string() 
                            } else { 
                                req.name 
                            },
                            image_path: req.image_path,
                            status: TaskStatus::Pending as i32,
                            created_at: Some(prost_types::Timestamp {
                                seconds: chrono::Utc::now().timestamp(),
                                nanos: 0,
                            }),
                        };
                        
                        Ok(Response::new(container))
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to create container: {}", e);
                Err(Status::internal(format!("Failed to create container: {}", e)))
            }
        }
    }

    async fn start_container(
        &self,
        request: Request<StartContainerRequest>,
    ) -> Result<Response<StartContainerResponse>, Status> {
        let req = request.into_inner();
        
        tracing::info!("Starting container: {}", req.container_id);
        
        let mut quilt_service = self.quilt_service.lock().await;
        
        match quilt_service.start_container(req.container_id).await {
            Ok(()) => {
                tracing::info!("Container started successfully");
                Ok(Response::new(StartContainerResponse {}))
            }
            Err(e) => {
                tracing::error!("Failed to start container: {}", e);
                Err(Status::internal(format!("Failed to start container: {}", e)))
            }
        }
    }

    async fn stop_container(
        &self,
        request: Request<StopContainerRequest>,
    ) -> Result<Response<StopContainerResponse>, Status> {
        let req = request.into_inner();
        
        tracing::info!("Stopping container: {}", req.container_id);
        
        let mut quilt_service = self.quilt_service.lock().await;
        
        match quilt_service.stop_container(req.container_id).await {
            Ok(()) => {
                tracing::info!("Container stopped successfully");
                Ok(Response::new(StopContainerResponse {}))
            }
            Err(e) => {
                tracing::error!("Failed to stop container: {}", e);
                Err(Status::internal(format!("Failed to stop container: {}", e)))
            }
        }
    }

    async fn remove_container(
        &self,
        request: Request<RemoveContainerRequest>,
    ) -> Result<Response<RemoveContainerResponse>, Status> {
        let req = request.into_inner();
        
        tracing::info!("Removing container: {}", req.container_id);
        
        let mut quilt_service = self.quilt_service.lock().await;
        
        match quilt_service.remove_container(req.container_id).await {
            Ok(()) => {
                tracing::info!("Container removed successfully");
                Ok(Response::new(RemoveContainerResponse {}))
            }
            Err(e) => {
                tracing::error!("Failed to remove container: {}", e);
                Err(Status::internal(format!("Failed to remove container: {}", e)))
            }
        }
    }

    async fn get_container(
        &self,
        request: Request<GetContainerRequest>,
    ) -> Result<Response<Container>, Status> {
        let req = request.into_inner();
        
        tracing::info!("Getting container: {}", req.container_id);
        
        let mut quilt_service = self.quilt_service.lock().await;
        
        match quilt_service.get_container_status(req.container_id.clone()).await {
            Ok(status) => {
                let container = Container {
                    id: req.container_id,
                    user_id: "system".to_string(),
                    session_id: None,
                    name: format!("container-{}", &status.id[..8]),
                    image_path: "unknown".to_string(), // Not available in status response
                    status: match status.state {
                        crate::types::ContainerState::Created => TaskStatus::Pending as i32,
                        crate::types::ContainerState::Running => TaskStatus::Running as i32,
                        crate::types::ContainerState::Exited => TaskStatus::Completed as i32,
                        crate::types::ContainerState::Failed => TaskStatus::Failed as i32,
                    },
                    created_at: Some(prost_types::Timestamp {
                        seconds: status.created_at as i64,
                        nanos: 0,
                    }),
                };
                
                Ok(Response::new(container))
            }
            Err(e) => {
                tracing::error!("Failed to get container: {}", e);
                Err(Status::not_found(format!("Container not found: {}", e)))
            }
        }
    }

    async fn list_containers(
        &self,
        request: Request<ListContainersRequest>,
    ) -> Result<Response<ListContainersResponse>, Status> {
        let req = request.into_inner();
        
        tracing::info!("Listing containers with session filter: {:?}", req.session_id);
        
        let mut quilt_service = self.quilt_service.lock().await;
        
        match quilt_service.list_containers().await {
            Ok(quilt_containers) => {
                let mut containers = Vec::new();
                
                for quilt_container in quilt_containers {
                    let container = Self::convert_quilt_container_to_aria(&quilt_container);
                    
                    // Apply session filter if provided
                    if let Some(ref session_id) = req.session_id {
                        if !session_id.is_empty() {
                            // For now, we don't have session linking, so we skip this filter
                            // TODO: Implement session-container linking
                            continue;
                        }
                    }
                    
                    containers.push(container);
                }
                
                tracing::info!("Found {} containers", containers.len());
                Ok(Response::new(ListContainersResponse { containers }))
            }
            Err(e) => {
                tracing::error!("Failed to list containers: {}", e);
                Err(Status::internal(format!("Failed to list containers: {}", e)))
            }
        }
    }

    async fn stream_container_logs(
        &self,
        request: Request<StreamContainerLogsRequest>,
    ) -> Result<Response<Self::StreamContainerLogsStream>, Status> {
        let req = request.into_inner();
        
        tracing::info!("Streaming logs for container: {}, follow={}", req.container_id, req.follow);
        
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        
        let container_id = req.container_id.clone();
        let quilt_service = Arc::clone(&self.quilt_service);
        
        // Spawn task to fetch and stream logs
        tokio::spawn(async move {
            let mut quilt = quilt_service.lock().await;
            
            match quilt.get_container_logs(container_id.clone()).await {
                Ok(logs) => {
                    // Split logs by lines and send each line
                    for line in logs.lines() {
                        let log_entry = ContainerLog {
                            line: line.to_string(),
                            stream: super::aria::container_log::Stream::Stdout as i32,
                            timestamp: Some(prost_types::Timestamp {
                                seconds: chrono::Utc::now().timestamp(),
                                nanos: 0,
                            }),
                        };
                        
                        if tx.send(Ok(log_entry)).await.is_err() {
                            break; // Client disconnected
                        }
                    }
                    
                    // If follow is true, we would need to implement real-time log streaming
                    if req.follow {
                        tracing::warn!("Real-time log streaming not yet implemented");
                        // TODO: Implement real-time log streaming
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to get container logs: {}", e);
                    let _ = tx.send(Err(Status::internal(format!("Failed to get logs: {}", e)))).await;
                }
            }
        });
        
        let stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream) as Self::StreamContainerLogsStream))
    }

    type StreamContainerLogsStream = Pin<Box<dyn Stream<Item = Result<ContainerLog, Status>> + Send>>;
} 