use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, oneshot};
use tokio::process::Command;
use uuid::Uuid;
use crate::sync::error::{SyncError, SyncResult};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AsyncTaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl AsyncTaskStatus {
    pub fn to_string(&self) -> String {
        match self {
            AsyncTaskStatus::Pending => "pending".to_string(),
            AsyncTaskStatus::Running => "running".to_string(),
            AsyncTaskStatus::Completed => "completed".to_string(),
            AsyncTaskStatus::Failed => "failed".to_string(),
            AsyncTaskStatus::Cancelled => "cancelled".to_string(),
        }
    }
    
    pub fn from_string(s: &str) -> SyncResult<Self> {
        match s {
            "pending" => Ok(AsyncTaskStatus::Pending),
            "running" => Ok(AsyncTaskStatus::Running),
            "completed" => Ok(AsyncTaskStatus::Completed),
            "failed" => Ok(AsyncTaskStatus::Failed),
            "cancelled" => Ok(AsyncTaskStatus::Cancelled),
            _ => Err(SyncError::ValidationFailed {
                message: format!("Invalid async task status: {}", s),
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AsyncTask {
    pub task_id: String,
    pub container_id: String,
    pub command: Vec<String>,
    pub status: AsyncTaskStatus,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub exit_code: Option<i64>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub error_message: Option<String>,
    pub timeout_seconds: Option<i64>,
}

/// Handle for cancelling running tasks
pub struct TaskHandle {
    task_id: String,
    abort_handle: tokio::task::AbortHandle,
    cancel_sender: Option<oneshot::Sender<()>>,
}

/// Production-grade async task management service
pub struct AsyncTaskManager {
    pool: SqlitePool,
    /// Running tasks with their handles for cancellation
    running_tasks: Arc<RwLock<HashMap<String, TaskHandle>>>,
}

impl AsyncTaskManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            running_tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Submit a new async exec task
    pub async fn submit_exec_task(
        &self,
        container_id: &str,
        command: Vec<String>,
        timeout_seconds: Option<i64>,
    ) -> SyncResult<String> {
        let task_id = Uuid::new_v4().to_string();
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        
        // Store task in database
        sqlx::query(r#"
            INSERT INTO async_tasks (
                task_id, container_id, command, status, created_at, timeout_seconds
            ) VALUES (?, ?, ?, ?, ?, ?)
        "#)
        .bind(&task_id)
        .bind(container_id)
        .bind(serde_json::to_string(&command)?)
        .bind(AsyncTaskStatus::Pending.to_string())
        .bind(now)
        .bind(timeout_seconds)
        .execute(&self.pool)
        .await?;
        
        tracing::info!("🚀 [ASYNC] Submitted exec task {} for container {} with command: {:?}", 
                      task_id, container_id, command);
        
        // Start execution immediately
        self.execute_task_async(&task_id).await?;
        
        Ok(task_id)
    }
    
    /// Execute a task asynchronously using tokio::spawn
    async fn execute_task_async(&self, task_id: &str) -> SyncResult<()> {
        let task = self.get_task_by_id(task_id).await?;
        
        if task.status != AsyncTaskStatus::Pending {
            return Err(SyncError::ValidationFailed {
                message: format!("Task {} is not in pending state: {:?}", task_id, task.status),
            });
        }
        
        // Update status to running
        self.update_task_status(task_id, AsyncTaskStatus::Running, None, None, None, None).await?;
        
        let pool = self.pool.clone();
        let running_tasks = self.running_tasks.clone();
        let task_id_clone = task_id.to_string();
        let task_clone = task.clone();
        
        // Create cancellation channel
        let (cancel_sender, cancel_receiver) = oneshot::channel::<()>();
        
        // Spawn the actual execution task
        let abort_handle = tokio::spawn(async move {
            Self::execute_task_impl(pool, task_clone, cancel_receiver).await
        }).abort_handle();
        
        // Store task handle for cancellation
        {
            let mut tasks = running_tasks.write().await;
            tasks.insert(task_id_clone.clone(), TaskHandle {
                task_id: task_id_clone.clone(),
                abort_handle,
                cancel_sender: Some(cancel_sender),
            });
        }
        
        tracing::info!("🔄 [ASYNC] Started execution for task {}", task_id);
        Ok(())
    }
    
    /// Internal task execution implementation
    async fn execute_task_impl(
        pool: SqlitePool,
        task: AsyncTask,
        mut cancel_receiver: oneshot::Receiver<()>,
    ) -> SyncResult<()> {
        let task_id = task.task_id.clone();
        let container_id = task.container_id.clone();
        
        // Update started_at timestamp
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        if let Err(e) = Self::update_task_timestamp(&pool, &task_id, Some(now), None).await {
            tracing::error!("Failed to update task start time: {}", e);
        }
        
        tracing::info!("🕐 [EXEC] Task {} execution started for container {}", task_id, container_id);
        
        // Get the container PID from the database
        let pid_row = sqlx::query("SELECT pid FROM containers WHERE id = ?")
            .bind(&container_id)
            .fetch_optional(&pool)
            .await;
        
        let container_pid = match pid_row {
            Ok(Some(row)) => {
                let pid: Option<i64> = row.get("pid");
                match pid {
                    Some(pid) if pid > 0 => pid.to_string(),
                    _ => {
                        tracing::error!("❌ [EXEC] No valid PID found for container {} in database", container_id);
                        let _ = Self::update_task_status_direct(&pool, &task_id, AsyncTaskStatus::Failed, Some(-1), None, None, Some("Container process not found".to_string())).await;
                        return Ok(());
                    }
                }
            }
            Ok(None) => {
                tracing::error!("❌ [EXEC] Container {} not found in database", container_id);
                let _ = Self::update_task_status_direct(&pool, &task_id, AsyncTaskStatus::Failed, Some(-1), None, None, Some("Container not found".to_string())).await;
                return Ok(());
            }
            Err(e) => {
                tracing::error!("❌ [EXEC] Database error when getting PID for container {}: {}", container_id, e);
                let _ = Self::update_task_status_direct(&pool, &task_id, AsyncTaskStatus::Failed, Some(-1), None, None, Some("Database error".to_string())).await;
                return Ok(());
            }
        };
        
        tracing::debug!("🔍 [EXEC] Found container PID {} for {}", container_pid, container_id);
        
        // Build the nsenter command for container execution
        let mut nsenter_cmd = Command::new("nsenter");
        nsenter_cmd
            .arg("--target")
            .arg(&container_pid)
            .arg("--mount")
            .arg("--uts")
            .arg("--ipc")
            .arg("--net")
            .arg("--pid");
        
        // Add the actual command
        for arg in &task.command {
            nsenter_cmd.arg(arg);
        }
        
        // Set up timeout if specified
        let timeout_duration = task.timeout_seconds.map(|s| Duration::from_secs(s as u64));
        
        // Execute with timeout and cancellation
        let execution_result = match timeout_duration {
            Some(timeout) => {
                tokio::select! {
                    // Task execution
                    result = nsenter_cmd.output() => {
                        match result {
                            Ok(output) => Some(output),
                            Err(e) => {
                                tracing::error!("Command execution failed: {}", e);
                                None
                            }
                        }
                    }
                    // Timeout
                    _ = tokio::time::sleep(timeout) => {
                        tracing::warn!("⏰ [EXEC] Task {} timed out after {}s", task_id, timeout.as_secs());
                        None
                    }
                    // Cancellation
                    _ = &mut cancel_receiver => {
                        tracing::info!("🚫 [EXEC] Task {} was cancelled", task_id);
                        let _ = Self::update_task_status_direct(&pool, &task_id, AsyncTaskStatus::Cancelled, None, None, None, Some("Task was cancelled".to_string())).await;
                        return Ok(());
                    }
                }
            }
            None => {
                tokio::select! {
                    // Task execution without timeout
                    result = nsenter_cmd.output() => {
                        match result {
                            Ok(output) => Some(output),
                            Err(e) => {
                                tracing::error!("Command execution failed: {}", e);
                                None
                            }
                        }
                    }
                    // Cancellation only
                    _ = &mut cancel_receiver => {
                        tracing::info!("🚫 [EXEC] Task {} was cancelled", task_id);
                        let _ = Self::update_task_status_direct(&pool, &task_id, AsyncTaskStatus::Cancelled, None, None, None, Some("Task was cancelled".to_string())).await;
                        return Ok(());
                    }
                }
            }
        };
        
        // Process results
        match execution_result {
            Some(output) => {
                let exit_code = output.status.code().unwrap_or(-1) as i64;
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                
                let status = if output.status.success() {
                    AsyncTaskStatus::Completed
                } else {
                    AsyncTaskStatus::Failed
                };
                
                let completed_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
                
                if let Err(e) = Self::update_task_status_direct(
                    &pool,
                    &task_id,
                    status.clone(),
                    Some(exit_code),
                    Some(stdout),
                    Some(stderr),
                    None,
                ).await {
                    tracing::error!("Failed to update task completion: {}", e);
                }
                
                if let Err(e) = Self::update_task_timestamp(&pool, &task_id, None, Some(completed_at)).await {
                    tracing::error!("Failed to update task completion time: {}", e);
                }
                
                match status {
                    AsyncTaskStatus::Completed => {
                        tracing::info!("✅ [EXEC] Task {} completed successfully (exit code: {})", task_id, exit_code);
                    }
                    AsyncTaskStatus::Failed => {
                        tracing::warn!("❌ [EXEC] Task {} failed (exit code: {})", task_id, exit_code);
                    }
                    _ => {}
                }
            }
            None => {
                // Timeout or execution failure
                let error_msg = format!("Command execution failed or timed out");
                let completed_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
                
                if let Err(e) = Self::update_task_status_direct(
                    &pool,
                    &task_id,
                    AsyncTaskStatus::Failed,
                    Some(-1),
                    None,
                    None,
                    Some(error_msg),
                ).await {
                    tracing::error!("Failed to update task failure: {}", e);
                }
                
                if let Err(e) = Self::update_task_timestamp(&pool, &task_id, None, Some(completed_at)).await {
                    tracing::error!("Failed to update task completion time: {}", e);
                }
                
                tracing::error!("❌ [EXEC] Task {} failed due to timeout or execution error", task_id);
            }
        }
        
        Ok(())
    }
    
    /// Cancel a running task
    pub async fn cancel_task(&self, task_id: &str) -> SyncResult<bool> {
        let mut tasks = self.running_tasks.write().await;
        
        if let Some(handle) = tasks.remove(task_id) {
            // Send cancellation signal
            if let Some(sender) = handle.cancel_sender {
                let _ = sender.send(());
            }
            
            // Abort the task
            handle.abort_handle.abort();
            
            tracing::info!("🚫 [ASYNC] Cancelled task {}", task_id);
            Ok(true)
        } else {
            tracing::warn!("Task {} not found in running tasks", task_id);
            Ok(false)
        }
    }
    
    /// Get task status by ID
    pub async fn get_task_status(&self, task_id: &str) -> SyncResult<AsyncTask> {
        self.get_task_by_id(task_id).await
    }
    
    /// List tasks for a container
    pub async fn list_container_tasks(&self, container_id: &str) -> SyncResult<Vec<AsyncTask>> {
        let rows = sqlx::query(r#"
            SELECT task_id, container_id, command, status, created_at, started_at, 
                   completed_at, exit_code, stdout, stderr, error_message, timeout_seconds
            FROM async_tasks 
            WHERE container_id = ?
            ORDER BY created_at DESC
        "#)
        .bind(container_id)
        .fetch_all(&self.pool)
        .await?;
        
        let mut tasks = Vec::new();
        for row in rows {
            let command_json: String = row.get("command");
            let status_str: String = row.get("status");
            
            tasks.push(AsyncTask {
                task_id: row.get("task_id"),
                container_id: row.get("container_id"),
                command: serde_json::from_str(&command_json)?,
                status: AsyncTaskStatus::from_string(&status_str)?,
                created_at: row.get("created_at"),
                started_at: row.get("started_at"),
                completed_at: row.get("completed_at"),
                exit_code: row.get("exit_code"),
                stdout: row.get("stdout"),
                stderr: row.get("stderr"),
                error_message: row.get("error_message"),
                timeout_seconds: row.get("timeout_seconds"),
            });
        }
        
        Ok(tasks)
    }
    
    /// Get task by ID (internal helper)
    async fn get_task_by_id(&self, task_id: &str) -> SyncResult<AsyncTask> {
        let row = sqlx::query(r#"
            SELECT task_id, container_id, command, status, created_at, started_at, 
                   completed_at, exit_code, stdout, stderr, error_message, timeout_seconds
            FROM async_tasks 
            WHERE task_id = ?
        "#)
        .bind(task_id)
        .fetch_optional(&self.pool)
        .await?;
        
        match row {
            Some(row) => {
                let command_json: String = row.get("command");
                let status_str: String = row.get("status");
                
                Ok(AsyncTask {
                    task_id: row.get("task_id"),
                    container_id: row.get("container_id"),
                    command: serde_json::from_str(&command_json)?,
                    status: AsyncTaskStatus::from_string(&status_str)?,
                    created_at: row.get("created_at"),
                    started_at: row.get("started_at"),
                    completed_at: row.get("completed_at"),
                    exit_code: row.get("exit_code"),
                    stdout: row.get("stdout"),
                    stderr: row.get("stderr"),
                    error_message: row.get("error_message"),
                    timeout_seconds: row.get("timeout_seconds"),
                })
            }
            None => Err(SyncError::NotFound {
                container_id: format!("async_task:{}", task_id),
            }),
        }
    }
    
    /// Update task status (internal)
    async fn update_task_status(
        &self,
        task_id: &str,
        status: AsyncTaskStatus,
        exit_code: Option<i64>,
        stdout: Option<String>,
        stderr: Option<String>,
        error_message: Option<String>,
    ) -> SyncResult<()> {
        Self::update_task_status_direct(&self.pool, task_id, status, exit_code, stdout, stderr, error_message).await
    }
    
    /// Update task status (static for use in spawned tasks)
    async fn update_task_status_direct(
        pool: &SqlitePool,
        task_id: &str,
        status: AsyncTaskStatus,
        exit_code: Option<i64>,
        stdout: Option<String>,
        stderr: Option<String>,
        error_message: Option<String>,
    ) -> SyncResult<()> {
        sqlx::query(r#"
            UPDATE async_tasks 
            SET status = ?, exit_code = ?, stdout = ?, stderr = ?, error_message = ?
            WHERE task_id = ?
        "#)
        .bind(status.to_string())
        .bind(exit_code)
        .bind(stdout)
        .bind(stderr)
        .bind(error_message)
        .bind(task_id)
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    /// Update task timestamps (static for use in spawned tasks)
    async fn update_task_timestamp(
        pool: &SqlitePool,
        task_id: &str,
        started_at: Option<i64>,
        completed_at: Option<i64>,
    ) -> SyncResult<()> {
        if let Some(started) = started_at {
            sqlx::query("UPDATE async_tasks SET started_at = ? WHERE task_id = ?")
                .bind(started)
                .bind(task_id)
                .execute(pool)
                .await?;
        }
        
        if let Some(completed) = completed_at {
            sqlx::query("UPDATE async_tasks SET completed_at = ? WHERE task_id = ?")
                .bind(completed)
                .bind(task_id)
                .execute(pool)
                .await?;
        }
        
        Ok(())
    }
    
    /// Cleanup completed tasks older than specified duration
    pub async fn cleanup_old_tasks(&self, older_than: Duration) -> SyncResult<usize> {
        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs()
            .saturating_sub(older_than.as_secs()) as i64;
        
        let result = sqlx::query(r#"
            DELETE FROM async_tasks 
            WHERE status IN ('completed', 'failed', 'cancelled') 
              AND completed_at < ?
        "#)
        .bind(cutoff_time)
        .execute(&self.pool)
        .await?;
        
        let deleted_count = result.rows_affected() as usize;
        if deleted_count > 0 {
            tracing::info!("🧹 [CLEANUP] Removed {} old async tasks", deleted_count);
        }
        
        Ok(deleted_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::connection::ConnectionManager;
    use tempfile::NamedTempFile;
    
    async fn setup_test_db() -> (ConnectionManager, AsyncTaskManager) {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();
        
        let conn_manager = ConnectionManager::new(db_path).await.unwrap();
        let schema_manager = crate::sync::schema::SchemaManager::new(conn_manager.pool().clone());
        schema_manager.initialize_schema().await.unwrap();
        
        let async_manager = AsyncTaskManager::new(conn_manager.pool().clone());
        (conn_manager, async_manager)
    }
    
    #[tokio::test]
    async fn test_task_submission() {
        let (_conn, manager) = setup_test_db().await;
        
        let task_id = manager.submit_exec_task(
            "test-container",
            vec!["/bin/echo".to_string(), "hello".to_string()],
            Some(30),
        ).await.unwrap();
        
        assert!(!task_id.is_empty());
        
        let task = manager.get_task_status(&task_id).await.unwrap();
        assert_eq!(task.container_id, "test-container");
        assert_eq!(task.command, vec!["/bin/echo", "hello"]);
    }
    
    #[tokio::test]
    async fn test_task_cancellation() {
        let (_conn, manager) = setup_test_db().await;
        
        let task_id = manager.submit_exec_task(
            "test-container",
            vec!["/bin/sleep".to_string(), "60".to_string()],
            None,
        ).await.unwrap();
        
        // Give task a moment to start
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let cancelled = manager.cancel_task(&task_id).await.unwrap();
        assert!(cancelled);
    }
    
    #[tokio::test]
    async fn test_cleanup_old_tasks() {
        let (_conn, manager) = setup_test_db().await;
        
        // Submit a task and mark it as completed
        let task_id = manager.submit_exec_task(
            "test-container",
            vec!["/bin/echo".to_string(), "test".to_string()],
            None,
        ).await.unwrap();
        
        // Simulate old completion
        let old_time = SystemTime::now()
            .duration_since(UNIX_EPOCH).unwrap()
            .as_secs()
            .saturating_sub(3600) as i64; // 1 hour ago
        
        sqlx::query("UPDATE async_tasks SET status = 'completed', completed_at = ? WHERE task_id = ?")
            .bind(old_time)
            .bind(&task_id)
            .execute(&manager.pool)
            .await.unwrap();
        
        let deleted = manager.cleanup_old_tasks(Duration::from_secs(1800)).await.unwrap(); // 30 minutes
        assert_eq!(deleted, 1);
    }
} 