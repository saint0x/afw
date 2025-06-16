mod daemon;
mod utils;
mod icc;
mod sync;

use daemon::{ContainerConfig, CgroupLimits, NamespaceConfig};
use utils::console::ConsoleLogger;
use sync::{SyncEngine, containers::ContainerState};

use std::sync::Arc;
use std::collections::HashMap;
use std::time::Duration;
use tonic::{transport::Server, Request, Response, Status};
use uuid::Uuid;
use sqlx::Row;
use std::env;

// Include the generated protobuf code
pub mod quilt {
    tonic::include_proto!("quilt");
}

use quilt::quilt_service_server::{QuiltService, QuiltServiceServer};
use quilt::{
    CreateContainerRequest, CreateContainerResponse,
    StartContainerRequest, StartContainerResponse,
    GetContainerStatusRequest, GetContainerStatusResponse,
    GetContainerLogsRequest, GetContainerLogsResponse,
    StopContainerRequest, StopContainerResponse,
    RemoveContainerRequest, RemoveContainerResponse,
    ExecContainerRequest, ExecContainerResponse,
    ContainerStatus, ListContainersRequest, ListContainersResponse, ContainerInfo,
    GetSystemMetricsRequest, GetSystemMetricsResponse, GetNetworkTopologyRequest, GetNetworkTopologyResponse, NetworkNode,
    GetContainerNetworkInfoRequest, GetContainerNetworkInfoResponse,
    ExecContainerAsyncRequest, ExecContainerAsyncResponse,
    GetTaskStatusRequest, GetTaskStatusResponse,
    GetTaskResultRequest, GetTaskResultResponse,
    ListTasksRequest, ListTasksResponse, TaskInfo,
    CancelTaskRequest, CancelTaskResponse,
};
use sysinfo::System;

#[derive(Clone)]
pub struct QuiltServiceImpl {
    sync_engine: Arc<SyncEngine>,
}

impl QuiltServiceImpl {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Find the executable's path
        let mut db_path = env::current_exe()?;
        db_path.pop(); // Remove the executable name
        db_path.push("quilt.db");

        let db_path_str = db_path.to_str().ok_or("Failed to convert DB path to string")?;
        ConsoleLogger::info(&format!("Initializing sync engine with database at: {}", db_path_str));

        // Initialize sync engine with the robust database path
        let sync_engine = Arc::new(SyncEngine::new(db_path_str).await?);
        
        // Start background services for monitoring and cleanup
        sync_engine.start_background_services().await?;
        
        ConsoleLogger::success("Sync engine initialized with background services");
        
        Ok(Self {
            sync_engine,
        })
    }
}

#[tonic::async_trait]
impl QuiltService for QuiltServiceImpl {
    async fn create_container(
        &self,
        request: Request<CreateContainerRequest>,
    ) -> Result<Response<CreateContainerResponse>, Status> {
        let req = request.into_inner();
        let container_id = Uuid::new_v4().to_string();

        ConsoleLogger::container_created(&container_id);

        // Convert gRPC request to sync engine container config
        let config = sync::containers::ContainerConfig {
            id: container_id.clone(),
            name: None, // gRPC request doesn't have name field
            image_path: req.image_path,
            command: if req.command.is_empty() { 
                    "sleep 86400".to_string() // Default for long-running agents (24 hours)
            } else { 
                req.command.join(" ")
            },
            environment: req.environment,
            memory_limit_mb: if req.memory_limit_mb > 0 { Some(req.memory_limit_mb as i64) } else { None },
            cpu_limit_percent: if req.cpu_limit_percent > 0.0 { Some(req.cpu_limit_percent as f64) } else { None },
            enable_network_namespace: req.enable_network_namespace,
            enable_pid_namespace: req.enable_pid_namespace,
            enable_mount_namespace: req.enable_mount_namespace,
            enable_uts_namespace: req.enable_uts_namespace,
            enable_ipc_namespace: req.enable_ipc_namespace,
        };

        // ✅ NON-BLOCKING: Create container with coordinated network allocation
        match self.sync_engine.create_container(config).await {
            Ok(_network_config) => {
                // ✅ INSTANT RETURN: Container creation is coordinated but non-blocking
                ConsoleLogger::success(&format!("Container {} created with network config", container_id));
                
                // Only auto-start if requested (default: false for agent control)
                if req.auto_start {
                    // Start the actual container process in background
                    let sync_engine = self.sync_engine.clone();
                    let container_id_clone = container_id.clone();
                    tokio::spawn(async move {
                        if let Err(e) = start_container_process(&sync_engine, &container_id_clone).await {
                            ConsoleLogger::error(&format!("Failed to start container process {}: {}", container_id_clone, e));
                            let _ = sync_engine.update_container_state(&container_id_clone, ContainerState::Error).await;
                        }
                    });
                    ConsoleLogger::info(&format!("Container {} will auto-start in background", container_id));
                } else {
                    ConsoleLogger::info(&format!("Container {} created but not started (agent control)", container_id));
                }
                
                Ok(Response::new(CreateContainerResponse {
                    container_id,
                    success: true,
                    error_message: String::new(),
                }))
            }
            Err(e) => {
                ConsoleLogger::error(&format!("Failed to create container: {}", e));
                Ok(Response::new(CreateContainerResponse {
                    container_id: String::new(),
                    success: false,
                    error_message: e.to_string(),
                }))
            }
        }
    }

    async fn start_container(
        &self,
        request: Request<StartContainerRequest>,
    ) -> Result<Response<StartContainerResponse>, Status> {
        let req = request.into_inner();
        ConsoleLogger::info(&format!("🚀 [GRPC] Starting container: {}", req.container_id));

        // Check if container exists and is in correct state
        match self.sync_engine.get_container_status(&req.container_id).await {
            Ok(status) => {
                match status.state {
                    ContainerState::Created => {
                        // Container is ready to start
                        let sync_engine = self.sync_engine.clone();
                        let container_id = req.container_id.clone();
                        
                        // Start container process in background
                        tokio::spawn(async move {
                            if let Err(e) = start_container_process(&sync_engine, &container_id).await {
                                ConsoleLogger::error(&format!("Failed to start container process {}: {}", container_id, e));
                                let _ = sync_engine.update_container_state(&container_id, ContainerState::Error).await;
                            }
                        });

                        ConsoleLogger::success(&format!("Container {} start initiated", req.container_id));
                        Ok(Response::new(StartContainerResponse {
                            success: true,
                            error_message: String::new(),
                        }))
                    }
                    ContainerState::Running => {
                        ConsoleLogger::warning(&format!("Container {} is already running", req.container_id));
                        Ok(Response::new(StartContainerResponse {
                            success: true,
                            error_message: "Container is already running".to_string(),
                        }))
                    }
                    ContainerState::Starting => {
                        ConsoleLogger::info(&format!("Container {} is already starting", req.container_id));
                        Ok(Response::new(StartContainerResponse {
                            success: true,
                            error_message: "Container is already starting".to_string(),
                        }))
                    }
                    _ => {
                        let error_msg = format!("Container {} cannot be started from state: {:?}", req.container_id, status.state);
                        ConsoleLogger::error(&error_msg);
                        Ok(Response::new(StartContainerResponse {
                            success: false,
                            error_message: error_msg,
                        }))
                    }
                }
            }
            Err(_) => {
                let error_msg = format!("Container {} not found", req.container_id);
                ConsoleLogger::error(&error_msg);
                Ok(Response::new(StartContainerResponse {
                    success: false,
                    error_message: error_msg,
                }))
            }
        }
    }

    async fn get_container_status(
        &self,
        request: Request<GetContainerStatusRequest>,
    ) -> Result<Response<GetContainerStatusResponse>, Status> {
        let req = request.into_inner();
        ConsoleLogger::debug(&format!("🔍 [GRPC] Status request for: {}", req.container_id));
        
        // ✅ ALWAYS FAST: Direct database query, never blocks
        match self.sync_engine.get_container_status(&req.container_id).await {
            Ok(status) => {
                let grpc_status = match status.state {
                    ContainerState::Created => ContainerStatus::Pending,
                    ContainerState::Starting => ContainerStatus::Pending,
                    ContainerState::Running => ContainerStatus::Running,
                    ContainerState::Exited => ContainerStatus::Exited,
                    ContainerState::Error => ContainerStatus::Failed,
                };

                ConsoleLogger::debug(&format!("✅ [GRPC] Status for {}: {:?}", req.container_id, grpc_status));
                
                Ok(Response::new(GetContainerStatusResponse {
                    container_id: req.container_id,
                    status: grpc_status as i32,
                    exit_code: status.exit_code.unwrap_or(0) as i32,
                    error_message: if status.state == ContainerState::Error { "Container failed".to_string() } else { String::new() },
                    pid: status.pid.unwrap_or(0) as i32,
                    created_at: status.created_at as u64,
                    memory_usage_bytes: 0, // TODO: Implement memory monitoring in sync engine
                    rootfs_path: status.rootfs_path.unwrap_or_default(),
                    ip_address: status.ip_address.unwrap_or_default(),
                }))
            }
            Err(_) => {
                ConsoleLogger::debug(&format!("❌ [GRPC] Container not found: {}", req.container_id));
                Err(Status::not_found(format!("Container {} not found", req.container_id)))
            }
        }
    }

    async fn get_container_logs(
        &self,
        request: Request<GetContainerLogsRequest>,
    ) -> Result<Response<GetContainerLogsResponse>, Status> {
        let req = request.into_inner();

        // TODO: Implement structured logging in sync engine
        // For now, return empty logs since we're focusing on the core sync functionality
        Ok(Response::new(GetContainerLogsResponse {
            container_id: req.container_id,
            logs: vec![],
        }))
    }

    async fn stop_container(
        &self,
        request: Request<StopContainerRequest>,
    ) -> Result<Response<StopContainerResponse>, Status> {
        let req = request.into_inner();

        // ✅ NON-BLOCKING: Stop monitoring and update state
        match self.sync_engine.stop_monitoring(&req.container_id).await {
            Ok(()) => {
                // Update container state to trigger cleanup
                if let Err(e) = self.sync_engine.update_container_state(&req.container_id, ContainerState::Exited).await {
                    ConsoleLogger::warning(&format!("Failed to update container state: {}", e));
                }
                
                ConsoleLogger::success(&format!("Container {} stopped", req.container_id));
                Ok(Response::new(StopContainerResponse {
                    success: true,
                    error_message: String::new(),
                }))
            }
            Err(e) => {
                ConsoleLogger::error(&format!("Failed to stop container {}: {}", req.container_id, e));
                Ok(Response::new(StopContainerResponse {
                    success: false,
                    error_message: e.to_string(),
                }))
            }
        }
    }

    async fn remove_container(
        &self,
        request: Request<RemoveContainerRequest>,
    ) -> Result<Response<RemoveContainerResponse>, Status> {
        let req = request.into_inner();

        // ✅ NON-BLOCKING: Coordinated cleanup through sync engine
        match self.sync_engine.delete_container(&req.container_id).await {
            Ok(()) => {
                ConsoleLogger::success(&format!("Container {} removed", req.container_id));
                Ok(Response::new(RemoveContainerResponse {
                    success: true,
                    error_message: String::new(),
                }))
            }
            Err(e) => {
                ConsoleLogger::error(&format!("Failed to remove container {}: {}", req.container_id, e));
                Ok(Response::new(RemoveContainerResponse {
                    success: false,
                    error_message: e.to_string(),
                }))
            }
        }
    }

    async fn exec_container(
        &self,
        request: Request<ExecContainerRequest>,
    ) -> Result<Response<ExecContainerResponse>, Status> {
        let req = request.into_inner();
        ConsoleLogger::debug(&format!("🔍 [GRPC] Exec request for: {} with command: {:?}", req.container_id, req.command));
        
        // Get container status to check if it's running and get PID
        match self.sync_engine.get_container_status(&req.container_id).await {
            Ok(status) => {
                if status.state != ContainerState::Running {
                    return Ok(Response::new(ExecContainerResponse {
                        success: false,
                        exit_code: -1,
                        stdout: String::new(),
                        stderr: String::new(),
                        error_message: format!("Container {} is not running (state: {:?})", req.container_id, status.state),
                    }));
                }

                let pid = match status.pid {
                    Some(pid) => pid,
                    None => {
                        return Ok(Response::new(ExecContainerResponse {
                            success: false,
                            exit_code: -1,
                            stdout: String::new(),
                            stderr: String::new(),
                            error_message: "Container has no PID".to_string(),
                        }));
                    }
                };

                // Execute command using nsenter (async execution)
                let command_str = req.command.join(" ");
                ConsoleLogger::debug(&format!("🚀 [GRPC] Executing: nsenter -t {} -p -m -n -u -i -- /bin/sh -c '{}'", pid, command_str));

                // Use async tokio::process::Command instead of blocking std::process::Command
                let output = tokio::process::Command::new("nsenter")
                    .args(&[
                        "-t", &pid.to_string(),
                        "-p", "-m", "-n", "-u", "-i",
                        "--", "/bin/sh", "-c", &command_str
                    ])
                    .output()
                    .await;

                match output {
                    Ok(output) => {
                        let exit_code = output.status.code().unwrap_or(-1);
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        let success = output.status.success();
                        
                        ConsoleLogger::debug(&format!("✅ [GRPC] Exec completed with exit code: {}", exit_code));
                        
                        let error_message = if !success && !stderr.is_empty() {
                            stderr.clone()
                        } else {
                            String::new()
                        };

                        Ok(Response::new(ExecContainerResponse {
                            success,
                            exit_code,
                            stdout,
                            stderr,
                            error_message,
                        }))
                    }
                    Err(e) => {
                        ConsoleLogger::error(&format!("❌ [GRPC] Exec failed: {}", e));
                        Ok(Response::new(ExecContainerResponse {
                            success: false,
                            exit_code: -1,
                            stdout: String::new(),
                            stderr: String::new(),
                            error_message: format!("Failed to execute nsenter: {}", e),
                        }))
                    }
                }
            }
            Err(_) => {
                Err(Status::not_found(format!("Container {} not found", req.container_id)))
            }
        }
    }

    async fn exec_container_async(
        &self,
        request: Request<ExecContainerAsyncRequest>,
    ) -> Result<Response<ExecContainerAsyncResponse>, Status> {
        let req = request.into_inner();
        
        ConsoleLogger::info(&format!("🚀 [GRPC] Async exec request for: {} with command: {:?}", req.container_id, req.command));
        
        // Validate container exists and is running
        match self.sync_engine.get_container_status(&req.container_id).await {
            Ok(status) => {
                if status.state != sync::containers::ContainerState::Running {
                    return Ok(Response::new(ExecContainerAsyncResponse {
                        success: false,
                        task_id: String::new(),
                        error_message: format!("Container {} is not running (state: {:?})", req.container_id, status.state),
                    }));
                }
            }
            Err(e) => {
                return Ok(Response::new(ExecContainerAsyncResponse {
                    success: false,
                    task_id: String::new(),
                    error_message: format!("Container not found: {}", e),
                }));
            }
        }
        
        // Submit async task
        match self.sync_engine.submit_async_exec_task(
            &req.container_id,
            req.command,
            if req.timeout_seconds == 0 { None } else { Some(req.timeout_seconds as i64) },
        ).await {
            Ok(task_id) => {
                ConsoleLogger::info(&format!("✅ [GRPC] Submitted async task {} for container {}", task_id, req.container_id));
                Ok(Response::new(ExecContainerAsyncResponse {
                    success: true,
                    task_id,
                    error_message: String::new(),
                }))
            }
            Err(e) => {
                ConsoleLogger::error(&format!("❌ [GRPC] Failed to submit async task: {}", e));
                Ok(Response::new(ExecContainerAsyncResponse {
                    success: false,
                    task_id: String::new(),
                    error_message: format!("Failed to submit async task: {}", e),
                }))
            }
        }
    }

    async fn get_task_status(
        &self,
        request: Request<GetTaskStatusRequest>,
    ) -> Result<Response<GetTaskStatusResponse>, Status> {
        let req = request.into_inner();
        
        ConsoleLogger::debug(&format!("🔍 [GRPC] Task status request for: {}", req.task_id));
        
        match self.sync_engine.get_async_task_status(&req.task_id).await {
            Ok(task) => {
                use quilt::TaskStatus;
                let status = match task.status {
                    sync::async_tasks::AsyncTaskStatus::Pending => TaskStatus::TaskPending,
                    sync::async_tasks::AsyncTaskStatus::Running => TaskStatus::TaskRunning,
                    sync::async_tasks::AsyncTaskStatus::Completed => TaskStatus::TaskCompleted,
                    sync::async_tasks::AsyncTaskStatus::Failed => TaskStatus::TaskFailed,
                    sync::async_tasks::AsyncTaskStatus::Cancelled => TaskStatus::TaskCancelled,
                };
                
                let progress_percent = match task.status {
                    sync::async_tasks::AsyncTaskStatus::Pending => 0.0,
                    sync::async_tasks::AsyncTaskStatus::Running => 50.0,
                    sync::async_tasks::AsyncTaskStatus::Completed => 100.0,
                    sync::async_tasks::AsyncTaskStatus::Failed => 100.0,
                    sync::async_tasks::AsyncTaskStatus::Cancelled => 100.0,
                };
                
                let current_operation = match task.status {
                    sync::async_tasks::AsyncTaskStatus::Pending => "Task queued".to_string(),
                    sync::async_tasks::AsyncTaskStatus::Running => format!("Executing: {}", task.command.join(" ")),
                    sync::async_tasks::AsyncTaskStatus::Completed => "Task completed successfully".to_string(),
                    sync::async_tasks::AsyncTaskStatus::Failed => "Task failed".to_string(),
                    sync::async_tasks::AsyncTaskStatus::Cancelled => "Task was cancelled".to_string(),
                };
                
                Ok(Response::new(GetTaskStatusResponse {
                    task_id: req.task_id,
                    status: status as i32,
                    started_at: task.started_at.unwrap_or(0) as u64,
                    completed_at: task.completed_at.unwrap_or(0) as u64,
                    exit_code: task.exit_code.unwrap_or(0) as i32,
                    error_message: task.error_message.unwrap_or_default(),
                    progress_percent,
                    current_operation,
                }))
            }
            Err(e) => {
                ConsoleLogger::warning(&format!("Task {} not found: {}", req.task_id, e));
                Err(Status::not_found(format!("Task not found: {}", e)))
            }
        }
    }

    async fn get_task_result(
        &self,
        request: Request<GetTaskResultRequest>,
    ) -> Result<Response<GetTaskResultResponse>, Status> {
        let req = request.into_inner();
        
        ConsoleLogger::debug(&format!("🔍 [GRPC] Task result request for: {}", req.task_id));
        
        match self.sync_engine.get_async_task_status(&req.task_id).await {
            Ok(task) => {
                use quilt::TaskStatus;
                let status = match task.status {
                    sync::async_tasks::AsyncTaskStatus::Pending => TaskStatus::TaskPending,
                    sync::async_tasks::AsyncTaskStatus::Running => TaskStatus::TaskRunning,
                    sync::async_tasks::AsyncTaskStatus::Completed => TaskStatus::TaskCompleted,
                    sync::async_tasks::AsyncTaskStatus::Failed => TaskStatus::TaskFailed,
                    sync::async_tasks::AsyncTaskStatus::Cancelled => TaskStatus::TaskCancelled,
                };
                
                let success = matches!(task.status, sync::async_tasks::AsyncTaskStatus::Completed);
                
                let execution_time_ms = match (task.started_at, task.completed_at) {
                    (Some(start), Some(end)) => ((end - start) * 1000) as u64, // Convert seconds to milliseconds
                    _ => 0u64,
                };
                
                Ok(Response::new(GetTaskResultResponse {
                    task_id: req.task_id,
                    status: status as i32,
                    success,
                    exit_code: task.exit_code.unwrap_or(-1) as i32,
                    stdout: task.stdout.unwrap_or_default(),
                    stderr: task.stderr.unwrap_or_default(),
                    error_message: task.error_message.unwrap_or_default(),
                    started_at: task.started_at.unwrap_or(0) as u64,
                    completed_at: task.completed_at.unwrap_or(0) as u64,
                    execution_time_ms,
                }))
            }
            Err(e) => {
                ConsoleLogger::warning(&format!("Task {} not found: {}", req.task_id, e));
                Err(Status::not_found(format!("Task not found: {}", e)))
            }
        }
    }

    async fn list_tasks(
        &self,
        request: Request<ListTasksRequest>,
    ) -> Result<Response<ListTasksResponse>, Status> {
        let req = request.into_inner();
        
        ConsoleLogger::debug(&format!("🔍 [GRPC] List tasks request for container: {:?}", req.container_id));
        
        let container_id = if req.container_id.is_empty() {
            return Err(Status::invalid_argument("container_id is required"));
        } else {
            req.container_id
        };
        
        match self.sync_engine.list_async_tasks(&container_id).await {
            Ok(tasks) => {
                use quilt::{TaskStatus, TaskInfo};
                let task_infos: Vec<TaskInfo> = tasks.into_iter().map(|task| {
                    let status = match task.status {
                        sync::async_tasks::AsyncTaskStatus::Pending => TaskStatus::TaskPending,
                        sync::async_tasks::AsyncTaskStatus::Running => TaskStatus::TaskRunning,
                        sync::async_tasks::AsyncTaskStatus::Completed => TaskStatus::TaskCompleted,
                        sync::async_tasks::AsyncTaskStatus::Failed => TaskStatus::TaskFailed,
                        sync::async_tasks::AsyncTaskStatus::Cancelled => TaskStatus::TaskCancelled,
                    };
                    
                    TaskInfo {
                        task_id: task.task_id,
                        container_id: task.container_id,
                        command: task.command,
                        status: status as i32,
                        started_at: task.started_at.unwrap_or(0) as u64,
                        completed_at: task.completed_at.unwrap_or(0) as u64,
                    }
                }).collect();
                
                ConsoleLogger::debug(&format!("📋 [GRPC] Found {} tasks for container {}", task_infos.len(), container_id));
                Ok(Response::new(ListTasksResponse { tasks: task_infos }))
            }
            Err(e) => {
                ConsoleLogger::error(&format!("❌ [GRPC] Failed to list tasks for container {}: {}", container_id, e));
                Err(Status::internal(format!("Failed to list tasks: {}", e)))
            }
        }
    }

    async fn cancel_task(
        &self,
        request: Request<CancelTaskRequest>,
    ) -> Result<Response<CancelTaskResponse>, Status> {
        let req = request.into_inner();
        
        ConsoleLogger::info(&format!("🔄 [GRPC] Cancel task request for: {}", req.task_id));
        
        match self.sync_engine.cancel_async_task(&req.task_id).await {
            Ok(was_cancelled) => {
                if was_cancelled {
                    ConsoleLogger::info(&format!("✅ [GRPC] Successfully cancelled task {}", req.task_id));
                    Ok(Response::new(CancelTaskResponse {
                        success: true,
                        error_message: String::new(),
                    }))
                } else {
                    ConsoleLogger::warning(&format!("⚠️ [GRPC] Task {} was not running or already completed", req.task_id));
                    Ok(Response::new(CancelTaskResponse {
                        success: false,
                        error_message: "Task was not running or already completed".to_string(),
                    }))
                }
            }
            Err(e) => {
                ConsoleLogger::error(&format!("❌ [GRPC] Failed to cancel task {}: {}", req.task_id, e));
                Ok(Response::new(CancelTaskResponse {
                    success: false,
                    error_message: format!("Failed to cancel task: {}", e),
                }))
            }
        }
    }

    async fn list_containers(
        &self,
        request: Request<ListContainersRequest>,
    ) -> Result<Response<ListContainersResponse>, Status> {
        let req = request.into_inner();

        let state_filter = match quilt::ContainerStatus::from_i32(req.state_filter) {
            Some(quilt::ContainerStatus::Unspecified) => None,
            Some(quilt::ContainerStatus::Pending) => Some(sync::containers::ContainerState::Created),
            Some(quilt::ContainerStatus::Running) => Some(sync::containers::ContainerState::Running),
            Some(quilt::ContainerStatus::Exited) => Some(sync::containers::ContainerState::Exited),
            Some(quilt::ContainerStatus::Failed) => Some(sync::containers::ContainerState::Error),
            None => None,
        };

        match self.sync_engine.list_containers(state_filter).await {
            Ok(statuses) => {
                let mut containers = Vec::new();
                for status in statuses {
                    // This requires a second query to get image and command.
                    // This is N+1, but acceptable for now for a low-frequency introspection tool.
                    let details: (String, String) = match sqlx::query_as("SELECT image_path, command FROM containers WHERE id = ?")
                        .bind(&status.id)
                        .fetch_one(self.sync_engine.pool())
                        .await {
                            Ok(details) => details,
                            Err(_) => (String::from("unknown"), String::from("unknown")),
                        };
                    
                    let proto_status = match status.state {
                        sync::containers::ContainerState::Created => quilt::ContainerStatus::Pending,
                        sync::containers::ContainerState::Starting => quilt::ContainerStatus::Pending,
                        sync::containers::ContainerState::Running => quilt::ContainerStatus::Running,
                        sync::containers::ContainerState::Exited => quilt::ContainerStatus::Exited,
                        sync::containers::ContainerState::Error => quilt::ContainerStatus::Failed,
                    };

                    containers.push(ContainerInfo {
                        container_id: status.id,
                        status: proto_status.into(),
                        image_path: details.0,
                        command: details.1,
                        created_at: status.created_at as u64,
                    });
                }
                Ok(Response::new(ListContainersResponse { containers }))
            }
            Err(e) => Err(Status::internal(format!("Failed to list containers: {}", e))),
        }
    }

    async fn get_system_metrics(
        &self,
        _request: Request<GetSystemMetricsRequest>,
    ) -> Result<Response<GetSystemMetricsResponse>, Status> {
        let mut sys = System::new_all();
        sys.refresh_all();

        let active_containers = match self.sync_engine.list_containers(Some(ContainerState::Running)).await {
            Ok(containers) => containers.len() as u32,
            Err(_) => 0, // If query fails, report 0
        };

        let response = GetSystemMetricsResponse {
            total_memory_bytes: sys.total_memory(),
            used_memory_bytes: sys.used_memory(),
            total_swap_bytes: sys.total_swap(),
            used_swap_bytes: sys.used_swap(),
            cpu_usage_percent: sys.global_cpu_usage() as f64,
            active_containers,
        };

        Ok(Response::new(response))
    }

    async fn get_network_topology(
        &self,
        _request: Request<GetNetworkTopologyRequest>,
    ) -> Result<Response<GetNetworkTopologyResponse>, Status> {
        match self.sync_engine.list_network_allocations().await {
            Ok(allocations) => {
                let nodes = allocations
                    .into_iter()
                    .map(|alloc| NetworkNode {
                        container_id: alloc.container_id,
                        ip_address: alloc.ip_address,
                        connections: vec![], // Not tracked yet
                    })
                    .collect();
                Ok(Response::new(GetNetworkTopologyResponse { nodes }))
            }
            Err(e) => Err(Status::internal(format!("Failed to get network topology: {}", e))),
        }
    }

    async fn get_container_network_info(
        &self,
        request: Request<GetContainerNetworkInfoRequest>,
    ) -> Result<Response<GetContainerNetworkInfoResponse>, Status> {
        let req = request.into_inner();
        match self.sync_engine.get_network_allocation(&req.container_id).await {
            Ok(alloc) => {
                let response = GetContainerNetworkInfoResponse {
                    container_id: alloc.container_id,
                    ip_address: alloc.ip_address,
                    bridge_interface: alloc.bridge_interface.unwrap_or_default(),
                    veth_host: alloc.veth_host.unwrap_or_default(),
                    veth_container: alloc.veth_container.unwrap_or_default(),
                    setup_completed: alloc.setup_completed,
                    status: alloc.status.to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => Err(Status::not_found(format!("Network info not found for container {}: {}", req.container_id, e))),
        }
    }
}

// ✅ BACKGROUND CONTAINER PROCESS STARTUP
async fn start_container_process(sync_engine: &SyncEngine, container_id: &str) -> Result<(), String> {
    use daemon::runtime::ContainerRuntime;
    
    // Get container configuration from sync engine
    let _status = sync_engine.get_container_status(container_id).await
        .map_err(|e| format!("Failed to get container config: {}", e))?;

    // Get full container config from database to get image_path and command
    let container_record = sqlx::query("SELECT image_path, command FROM containers WHERE id = ?")
        .bind(container_id)
        .fetch_one(sync_engine.pool())
        .await
        .map_err(|e| format!("Failed to get container details: {}", e))?;
    
    let image_path: String = container_record.get("image_path");
    let command: String = container_record.get("command");

    // Convert sync engine config back to legacy format for actual container startup
    // TODO: Eventually replace this with native sync engine container startup
    let legacy_config = ContainerConfig {
        image_path,
        command: vec!["/bin/sh".to_string(), "-c".to_string(), command],
        environment: HashMap::new(), // TODO: Get from sync engine
        setup_commands: vec![],
        resource_limits: Some(CgroupLimits::default()),
        namespace_config: Some(NamespaceConfig::default()),
        working_directory: None,
    };

    // Create legacy runtime for actual process management (temporary)
    let runtime = ContainerRuntime::new();
    
    // Update state to Starting
    sync_engine.update_container_state(container_id, ContainerState::Starting).await
        .map_err(|e| format!("Failed to update state: {}", e))?;

    // Create container in legacy runtime
    runtime.create_container(container_id.to_string(), legacy_config)
        .map_err(|e| format!("Failed to create legacy container: {}", e))?;

    // Start the container
    match runtime.start_container(container_id, None) {
        Ok(()) => {
            // Get the PID from legacy runtime and store in sync engine
            if let Some(container) = runtime.get_container_info(container_id) {
                if let Some(pid) = container.pid {
                    sync_engine.set_container_pid(container_id, pid).await
                        .map_err(|e| format!("Failed to set PID: {}", e))?;
                }
                
                // Update state to Running
                sync_engine.update_container_state(container_id, ContainerState::Running).await
                    .map_err(|e| format!("Failed to update to running: {}", e))?;
            }
            
            ConsoleLogger::success(&format!("Container {} started successfully", container_id));
            Ok(())
        }
        Err(e) => {
            sync_engine.update_container_state(container_id, ContainerState::Error).await.ok();
            Err(format!("Failed to start container: {}", e))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ✅ SYNC ENGINE INITIALIZATION
    let service = Arc::new(QuiltServiceImpl::new().await
        .map_err(|e| format!("Failed to initialize sync engine: {}", e))?);
    
    let addr: std::net::SocketAddr = "127.0.0.1:50051".parse()?;

    ConsoleLogger::server_starting(&addr.to_string());
    ConsoleLogger::success("🚀 Quilt server running with SQLite sync engine - non-blocking operations enabled");

    // ✅ GRACEFUL SHUTDOWN
    let service_clone = service.clone();
    tokio::select! {
        result = Server::builder()
            .add_service(QuiltServiceServer::new((*service).clone()))
            .serve(addr) => {
            result?;
        }
        _ = tokio::signal::ctrl_c() => {
            ConsoleLogger::info("Received shutdown signal, cleaning up...");
            service_clone.sync_engine.close().await;
            ConsoleLogger::success("Sync engine closed gracefully");
        }
    }

    Ok(())
}
