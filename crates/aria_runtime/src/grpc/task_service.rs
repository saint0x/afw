use std::pin::Pin;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio_stream::{wrappers::ReceiverStream, Stream};
use tonic::{Request, Response, Status, Streaming};

use super::aria::{
    task_service_server::TaskService,
    Task, TaskStatus, LaunchTaskRequest, LaunchTaskResponse, GetTaskRequest,
    ListTasksRequest, ListTasksResponse, StreamTaskOutputRequest, TaskOutput,
    CancelTaskRequest, CancelTaskResponse,
};

use crate::engines::container::quilt::QuiltService;
use crate::engines::container::quilt::quilt_proto;
use crate::database::DatabaseManager;
use crate::errors::{AriaError, AriaResult};

/// Implementation of the high-level TaskService
pub struct TaskServiceImpl {
    quilt_service: Arc<Mutex<QuiltService>>,
    database: Arc<DatabaseManager>,
}

impl TaskServiceImpl {
    pub fn new(quilt_service: Arc<Mutex<QuiltService>>, database: Arc<DatabaseManager>) -> Self {
        Self { 
            quilt_service,
            database,
        }
    }

    /// Convert from Quilt's TaskInfo to our high-level Task
    fn convert_quilt_task_info_to_aria_task(quilt_task: &quilt::quilt_proto::TaskInfo) -> Task {
        let status = match quilt_task.status {
            1 => TaskStatus::Pending,      // TASK_PENDING
            2 => TaskStatus::Running,      // TASK_RUNNING  
            3 => TaskStatus::Completed,    // TASK_COMPLETED
            4 => TaskStatus::Failed,       // TASK_FAILED
            5 => TaskStatus::Cancelled,    // TASK_CANCELLED
            6 => TaskStatus::Timeout,      // TASK_TIMEOUT
            _ => TaskStatus::Pending,      // Default fallback
        };

        Task {
            id: quilt_task.task_id.clone(),
            user_id: "system".to_string(), // System-level tasks for now
            session_id: "".to_string(),    // TODO: Link to session when available
            container_id: quilt_task.container_id.clone(),
            parent_task_id: None,
            
            r#type: "container:exec".to_string(),
            command_json: serde_json::to_string(&quilt_task.command).unwrap_or_else(|_| "[]".to_string()),
            environment: HashMap::new(), // Quilt TaskInfo doesn't include environment
            timeout_seconds: 0, // Not available in TaskInfo
            
            status: status as i32,
            created_at: if quilt_task.started_at > 0 {
                Some(prost_types::Timestamp {
                    seconds: quilt_task.started_at as i64,
                    nanos: 0,
                })
            } else {
                Some(prost_types::Timestamp::default())
            },
            started_at: if quilt_task.started_at > 0 {
                Some(prost_types::Timestamp {
                    seconds: quilt_task.started_at as i64,
                    nanos: 0,
                })
            } else {
                None
            },
            completed_at: if quilt_task.completed_at > 0 {
                Some(prost_types::Timestamp {
                    seconds: quilt_task.completed_at as i64,
                    nanos: 0,
                })
            } else {
                None
            },
            
            exit_code: None, // Available in GetTaskResult, not TaskInfo
            error_message: None, // Available in GetTaskResult, not TaskInfo
            progress_percent: 0.0, // Available in GetTaskStatus, not TaskInfo
            current_operation: "".to_string(), // Available in GetTaskStatus, not TaskInfo
        }
    }

    /// Convert from Quilt's GetTaskStatusResponse to our high-level Task
    fn convert_quilt_task_status_to_aria_task(
        task_id: String, 
        container_id: String,
        status_response: &quilt::quilt_proto::GetTaskStatusResponse
    ) -> Task {
        let status = match status_response.status {
            1 => TaskStatus::Pending,      // TASK_PENDING
            2 => TaskStatus::Running,      // TASK_RUNNING  
            3 => TaskStatus::Completed,    // TASK_COMPLETED
            4 => TaskStatus::Failed,       // TASK_FAILED
            5 => TaskStatus::Cancelled,    // TASK_CANCELLED
            6 => TaskStatus::Timeout,      // TASK_TIMEOUT
            _ => TaskStatus::Pending,      // Default fallback
        };

        Task {
            id: task_id,
            user_id: "system".to_string(),
            session_id: "".to_string(),
            container_id,
            parent_task_id: None,
            
            r#type: "container:exec".to_string(),
            command_json: "[]".to_string(), // Not available in status response
            environment: HashMap::new(),
            timeout_seconds: 0,
            
            status: status as i32,
            created_at: if status_response.started_at > 0 {
                Some(prost_types::Timestamp {
                    seconds: status_response.started_at as i64,
                    nanos: 0,
                })
            } else {
                Some(prost_types::Timestamp::default())
            },
            started_at: if status_response.started_at > 0 {
                Some(prost_types::Timestamp {
                    seconds: status_response.started_at as i64,
                    nanos: 0,
                })
            } else {
                None
            },
            completed_at: if status_response.completed_at > 0 {
                Some(prost_types::Timestamp {
                    seconds: status_response.completed_at as i64,
                    nanos: 0,
                })
            } else {
                None
            },
            
            exit_code: if status_response.exit_code != 0 {
                Some(status_response.exit_code)
            } else {
                None
            },
            error_message: if !status_response.error_message.is_empty() {
                Some(status_response.error_message.clone())
            } else {
                None
            },
            progress_percent: status_response.progress_percent,
            current_operation: status_response.current_operation.clone(),
        }
    }

    /// Convert Aria TaskStatus filter to Quilt TaskStatus
    fn convert_aria_status_to_quilt_status(status: TaskStatus) -> quilt_proto::TaskStatus {
        match status {
            TaskStatus::Unspecified => quilt_proto::TaskStatus::TaskUnspecified,
            TaskStatus::Pending => quilt_proto::TaskStatus::TaskPending,
            TaskStatus::Running => quilt_proto::TaskStatus::TaskRunning,
            TaskStatus::Completed => quilt_proto::TaskStatus::TaskCompleted,
            TaskStatus::Failed => quilt_proto::TaskStatus::TaskFailed,
            TaskStatus::Cancelled => quilt_proto::TaskStatus::TaskCancelled,
            TaskStatus::Timeout => quilt_proto::TaskStatus::TaskTimeout,
        }
    }
}

#[tonic::async_trait]
impl TaskService for TaskServiceImpl {
    async fn launch_task(
        &self,
        request: Request<LaunchTaskRequest>,
    ) -> Result<Response<LaunchTaskResponse>, Status> {
        let req = request.into_inner();
        
        tracing::info!("Launching task for session {}: {}", req.session_id, req.r#type);
        
        // Parse the command from JSON
        let command: Vec<String> = serde_json::from_str(&req.command_json)
            .map_err(|e| Status::invalid_argument(format!("Invalid command JSON: {}", e)))?;
        
        if command.is_empty() {
            return Err(Status::invalid_argument("Command cannot be empty"));
        }
        
        // For now, we need a container_id to launch a task. In the future, this could be
        // derived from the session_id or we could have a default execution environment.
        // For MVP, we'll require that the type includes the container_id.
        let container_id = if req.r#type.starts_with("container:") {
            // Extract container_id from type like "container:exec:container_id_here"
            let parts: Vec<&str> = req.r#type.split(':').collect();
            if parts.len() >= 3 {
                parts[2].to_string()
            } else {
                return Err(Status::invalid_argument("Container ID required for container tasks"));
            }
        } else {
            return Err(Status::unimplemented("Only container tasks are currently supported"));
        };
        
        // Launch the async task via Quilt
        let mut quilt_service = self.quilt_service.lock().await;
        let timeout_seconds = if req.timeout_seconds > 0 {
            Some(req.timeout_seconds)
        } else {
            None
        };
        
        match quilt_service.exec_container_async(container_id, command, timeout_seconds).await {
            Ok(task_id) => {
                tracing::info!("Successfully launched task: {}", task_id);
                Ok(Response::new(LaunchTaskResponse { task_id }))
            }
            Err(e) => {
                tracing::error!("Failed to launch task: {}", e);
                Err(Status::internal(format!("Failed to launch task: {}", e)))
            }
        }
    }

    async fn get_task(
        &self,
        request: Request<GetTaskRequest>,
    ) -> Result<Response<Task>, Status> {
        let req = request.into_inner();
        
        tracing::info!("Getting task: {}", req.task_id);
        
        let mut quilt_service = self.quilt_service.lock().await;
        
        // Get detailed task status from Quilt
        match quilt_service.get_task_status(req.task_id.clone()).await {
            Ok(status_response) => {
                // We need the container_id to build a complete Task response
                // For now, we'll try to get it from the task result if available
                let container_id = match quilt_service.get_task_result(req.task_id.clone()).await {
                    Ok(_result) => "unknown".to_string(), // TaskResult doesn't include container_id
                    Err(_) => "unknown".to_string(),
                };
                
                let task = Self::convert_quilt_task_status_to_aria_task(
                    req.task_id,
                    container_id,
                    &status_response
                );
                
                Ok(Response::new(task))
            }
            Err(e) => {
                tracing::error!("Failed to get task status: {}", e);
                Err(Status::not_found(format!("Task not found: {}", e)))
            }
        }
    }

    /// The critical ListTasks implementation from our INTEGRATIONTODO.md
    async fn list_tasks(
        &self,
        request: Request<ListTasksRequest>,
    ) -> Result<Response<ListTasksResponse>, Status> {
        let req = request.into_inner();
        
        tracing::info!("Listing tasks with filters: session_id={:?}, statuses={:?}", 
                      req.session_id, req.filter_by_status);
        
        let mut quilt_service = self.quilt_service.lock().await;
        
        // Get all containers first
        let containers = match quilt_service.list_containers().await {
            Ok(containers) => containers,
            Err(e) => {
                tracing::error!("Failed to list containers: {}", e);
                return Err(Status::internal(format!("Failed to list containers: {}", e)));
            }
        };
        
        let mut all_tasks = Vec::new();
        
        // For each container, list its tasks
        for container in containers {
            // Convert status filter if provided
            let status_filter = if !req.filter_by_status.is_empty() {
                // Use the first status filter for now
                Some(Self::convert_aria_status_to_quilt_status(
                    TaskStatus::try_from(req.filter_by_status[0]).unwrap_or(TaskStatus::Unspecified)
                ))
            } else {
                None
            };
            
            match quilt_service.list_tasks(container.container_id, status_filter).await {
                Ok(tasks) => {
                    for task_info in tasks {
                        let aria_task = Self::convert_quilt_task_info_to_aria_task(&task_info);
                        all_tasks.push(aria_task);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to list tasks for container {}: {}", container.container_id, e);
                    // Continue with other containers
                }
            }
        }
        
        // Apply session_id filter if provided
        if let Some(session_id) = &req.session_id {
            if !session_id.is_empty() {
                all_tasks.retain(|task| task.session_id == *session_id);
            }
        }
        
        // Apply pagination (simple implementation for now)
        let page_size = if req.page_size > 0 { req.page_size as usize } else { 50 };
        let start_index = if req.page_token.is_empty() {
            0
        } else {
            req.page_token.parse::<usize>().unwrap_or(0)
        };
        
        let end_index = std::cmp::min(start_index + page_size, all_tasks.len());
        let page_tasks = all_tasks[start_index..end_index].to_vec();
        
        let next_page_token = if end_index < all_tasks.len() {
            end_index.to_string()
        } else {
            "".to_string()
        };
        
        tracing::info!("Returning {} tasks (page {}-{})", page_tasks.len(), start_index, end_index);
        
        Ok(Response::new(ListTasksResponse {
            tasks: page_tasks,
            next_page_token,
        }))
    }

    async fn stream_task_output(
        &self,
        request: Request<StreamTaskOutputRequest>,
    ) -> Result<Response<Self::StreamTaskOutputStream>, Status> {
        let req = request.into_inner();
        
        tracing::info!("Streaming output for task: {}, follow={}", req.task_id, req.follow);
        
        // Create a channel for streaming task output
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        
        let task_id = req.task_id.clone();
        let quilt_service = Arc::clone(&self.quilt_service);
        
        // Spawn a task to fetch and stream the output
        tokio::spawn(async move {
            let mut quilt = quilt_service.lock().await;
            
            // Get the task result which includes stdout/stderr
            match quilt.get_task_result(task_id.clone()).await {
                Ok(result) => {
                    // Send stdout if available
                    if !result.stdout.is_empty() {
                        for line in result.stdout.lines() {
                            let output = TaskOutput {
                                task_id: task_id.clone(),
                                timestamp: Some(prost_types::Timestamp {
                                    seconds: result.completed_at as i64,
                                    nanos: 0,
                                }),
                                output: Some(super::aria::task_output::Output::StdoutLine(line.to_string())),
                            };
                            
                            if tx.send(Ok(output)).await.is_err() {
                                break; // Client disconnected
                            }
                        }
                    }
                    
                    // Send stderr if available
                    if !result.stderr.is_empty() {
                        for line in result.stderr.lines() {
                            let output = TaskOutput {
                                task_id: task_id.clone(),
                                timestamp: Some(prost_types::Timestamp {
                                    seconds: result.completed_at as i64,
                                    nanos: 0,
                                }),
                                output: Some(super::aria::task_output::Output::StderrLine(line.to_string())),
                            };
                            
                            if tx.send(Ok(output)).await.is_err() {
                                break; // Client disconnected
                            }
                        }
                    }
                    
                    // If follow is true and task is still running, we would need to implement
                    // real-time streaming. For now, we just send what we have.
                    if req.follow {
                        // TODO: Implement real-time streaming for running tasks
                        tracing::warn!("Real-time task output streaming not yet implemented");
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to get task result: {}", e);
                    let _ = tx.send(Err(Status::internal(format!("Failed to get task output: {}", e)))).await;
                }
            }
        });
        
        let stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream) as Self::StreamTaskOutputStream))
    }

    async fn cancel_task(
        &self,
        request: Request<CancelTaskRequest>,
    ) -> Result<Response<CancelTaskResponse>, Status> {
        let req = request.into_inner();
        
        tracing::info!("Cancelling task: {}", req.task_id);
        
        let mut quilt_service = self.quilt_service.lock().await;
        
        match quilt_service.cancel_task(req.task_id).await {
            Ok(cancelled) => {
                Ok(Response::new(CancelTaskResponse {
                    cancellation_initiated: cancelled,
                }))
            }
            Err(e) => {
                tracing::error!("Failed to cancel task: {}", e);
                Err(Status::internal(format!("Failed to cancel task: {}", e)))
            }
        }
    }

    type StreamTaskOutputStream = Pin<Box<dyn Stream<Item = Result<TaskOutput, Status>> + Send>>;
} 