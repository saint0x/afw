use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::fs;
use crate::sync::error::{SyncError, SyncResult};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CleanupStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl CleanupStatus {
    pub fn to_string(&self) -> String {
        match self {
            CleanupStatus::Pending => "pending".to_string(),
            CleanupStatus::InProgress => "in_progress".to_string(),
            CleanupStatus::Completed => "completed".to_string(),
            CleanupStatus::Failed => "failed".to_string(),
        }
    }
    
    pub fn from_string(s: &str) -> SyncResult<Self> {
        match s {
            "pending" => Ok(CleanupStatus::Pending),
            "in_progress" => Ok(CleanupStatus::InProgress),
            "completed" => Ok(CleanupStatus::Completed),
            "failed" => Ok(CleanupStatus::Failed),
            _ => Err(SyncError::ValidationFailed {
                message: format!("Invalid cleanup status: {}", s),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResourceType {
    Rootfs,
    Network,
    Cgroup,
    Mounts,
}

impl ResourceType {
    pub fn to_string(&self) -> String {
        match self {
            ResourceType::Rootfs => "rootfs".to_string(),
            ResourceType::Network => "network".to_string(),
            ResourceType::Cgroup => "cgroup".to_string(),
            ResourceType::Mounts => "mounts".to_string(),
        }
    }
    
    pub fn from_string(s: &str) -> SyncResult<Self> {
        match s {
            "rootfs" => Ok(ResourceType::Rootfs),
            "network" => Ok(ResourceType::Network),
            "cgroup" => Ok(ResourceType::Cgroup),
            "mounts" => Ok(ResourceType::Mounts),
            _ => Err(SyncError::ValidationFailed {
                message: format!("Invalid resource type: {}", s),
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CleanupTask {
    pub id: i64,
    pub container_id: String,
    pub resource_type: ResourceType,
    pub resource_path: String,
    pub status: CleanupStatus,
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub error_message: Option<String>,
}

pub struct CleanupService {
    pool: SqlitePool,
}

impl CleanupService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
    
    pub async fn schedule_cleanup(&self, container_id: &str, resource_type: ResourceType, resource_path: &str) -> SyncResult<i64> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        
        let result = sqlx::query(r#"
            INSERT INTO cleanup_tasks (
                container_id, resource_type, resource_path, status, created_at
            ) VALUES (?, ?, ?, ?, ?)
        "#)
        .bind(container_id)
        .bind(resource_type.to_string())
        .bind(resource_path)
        .bind(CleanupStatus::Pending.to_string())
        .bind(now)
        .execute(&self.pool)
        .await?;
        
        let task_id = result.last_insert_rowid();
        tracing::info!("Scheduled cleanup task {} for container {} ({:?}: {})", 
                      task_id, container_id, resource_type, resource_path);
        
        Ok(task_id)
    }
    
    pub async fn schedule_container_cleanup(&self, container_id: &str, rootfs_path: Option<&str>) -> SyncResult<Vec<i64>> {
        let mut task_ids = Vec::new();
        
        // Schedule rootfs cleanup if provided
        if let Some(rootfs) = rootfs_path {
            if Path::new(rootfs).exists() {
                let task_id = self.schedule_cleanup(container_id, ResourceType::Rootfs, rootfs).await?;
                task_ids.push(task_id);
            }
        }
        
        // Schedule network cleanup (will be handled by network manager)
        let task_id = self.schedule_cleanup(container_id, ResourceType::Network, container_id).await?;
        task_ids.push(task_id);
        
        // Schedule cgroup cleanup
        let cgroup_path = format!("/sys/fs/cgroup/quilt/{}", container_id);
        if Path::new(&cgroup_path).exists() {
            let task_id = self.schedule_cleanup(container_id, ResourceType::Cgroup, &cgroup_path).await?;
            task_ids.push(task_id);
        }
        
        // Schedule mounts cleanup
        let task_id = self.schedule_cleanup(container_id, ResourceType::Mounts, container_id).await?;
        task_ids.push(task_id);
        
        tracing::info!("Scheduled {} cleanup tasks for container {}", task_ids.len(), container_id);
        Ok(task_ids)
    }
    
    pub async fn run_cleanup_worker(&self, max_concurrent: usize) -> SyncResult<()> {
        tracing::info!("Starting cleanup worker with max {} concurrent tasks", max_concurrent);
        
        loop {
            let pending_tasks = self.get_pending_tasks(max_concurrent).await?;
            
            if pending_tasks.is_empty() {
                // No pending tasks, sleep before checking again
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }
            
            // Process tasks concurrently
            let mut handles = Vec::new();
            
            for task in pending_tasks {
                let pool = self.pool.clone();
                let task_clone = task.clone();
                
                let handle = tokio::spawn(async move {
                    CleanupService::execute_cleanup_task(&pool, task_clone).await
                });
                
                handles.push(handle);
            }
            
            // Wait for all tasks to complete
            for handle in handles {
                if let Err(e) = handle.await {
                    tracing::error!("Cleanup task panicked: {}", e);
                }
            }
        }
    }
    
    pub async fn get_pending_tasks(&self, limit: usize) -> SyncResult<Vec<CleanupTask>> {
        let rows = sqlx::query(r#"
            SELECT id, container_id, resource_type, resource_path, status, 
                   created_at, completed_at, error_message
            FROM cleanup_tasks 
            WHERE status = 'pending'
            ORDER BY created_at ASC
            LIMIT ?
        "#)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        
        let mut tasks = Vec::new();
        for row in rows {
            let resource_type_str: String = row.get("resource_type");
            let status_str: String = row.get("status");
            
            tasks.push(CleanupTask {
                id: row.get("id"),
                container_id: row.get("container_id"),
                resource_type: ResourceType::from_string(&resource_type_str)?,
                resource_path: row.get("resource_path"),
                status: CleanupStatus::from_string(&status_str)?,
                created_at: row.get("created_at"),
                completed_at: row.get("completed_at"),
                error_message: row.get("error_message"),
            });
        }
        
        Ok(tasks)
    }
    
    pub async fn get_task_status(&self, task_id: i64) -> SyncResult<CleanupTask> {
        let row = sqlx::query(r#"
            SELECT id, container_id, resource_type, resource_path, status, 
                   created_at, completed_at, error_message
            FROM cleanup_tasks WHERE id = ?
        "#)
        .bind(task_id)
        .fetch_optional(&self.pool)
        .await?;
        
        match row {
            Some(row) => {
                let resource_type_str: String = row.get("resource_type");
                let status_str: String = row.get("status");
                
                Ok(CleanupTask {
                    id: row.get("id"),
                    container_id: row.get("container_id"),
                    resource_type: ResourceType::from_string(&resource_type_str)?,
                    resource_path: row.get("resource_path"),
                    status: CleanupStatus::from_string(&status_str)?,
                    created_at: row.get("created_at"),
                    completed_at: row.get("completed_at"),
                    error_message: row.get("error_message"),
                })
            }
            None => Err(SyncError::ValidationFailed {
                message: format!("Cleanup task {} not found", task_id),
            }),
        }
    }
    
    pub async fn list_container_cleanup_tasks(&self, container_id: &str) -> SyncResult<Vec<CleanupTask>> {
        let rows = sqlx::query(r#"
            SELECT id, container_id, resource_type, resource_path, status, 
                   created_at, completed_at, error_message
            FROM cleanup_tasks 
            WHERE container_id = ?
            ORDER BY created_at DESC
        "#)
        .bind(container_id)
        .fetch_all(&self.pool)
        .await?;
        
        let mut tasks = Vec::new();
        for row in rows {
            let resource_type_str: String = row.get("resource_type");
            let status_str: String = row.get("status");
            
            tasks.push(CleanupTask {
                id: row.get("id"),
                container_id: row.get("container_id"),
                resource_type: ResourceType::from_string(&resource_type_str)?,
                resource_path: row.get("resource_path"),
                status: CleanupStatus::from_string(&status_str)?,
                created_at: row.get("created_at"),
                completed_at: row.get("completed_at"),
                error_message: row.get("error_message"),
            });
        }
        
        Ok(tasks)
    }
    
    async fn execute_cleanup_task(pool: &SqlitePool, task: CleanupTask) -> SyncResult<()> {
        // Mark task as in progress
        Self::update_task_status(pool, task.id, CleanupStatus::InProgress, None).await?;
        
        let result = match task.resource_type {
            ResourceType::Rootfs => Self::cleanup_rootfs(&task.resource_path).await,
            ResourceType::Network => Self::cleanup_network(&task.container_id).await,
            ResourceType::Cgroup => Self::cleanup_cgroup(&task.resource_path).await,
            ResourceType::Mounts => Self::cleanup_mounts(&task.container_id).await,
        };
        
        match result {
            Ok(()) => {
                Self::update_task_status(pool, task.id, CleanupStatus::Completed, None).await?;
                tracing::info!("Completed cleanup task {} ({:?}: {})", 
                              task.id, task.resource_type, task.resource_path);
            }
            Err(e) => {
                let error_msg = e.to_string();
                Self::update_task_status(pool, task.id, CleanupStatus::Failed, Some(&error_msg)).await?;
                tracing::error!("Failed cleanup task {} ({:?}: {}): {}", 
                               task.id, task.resource_type, task.resource_path, error_msg);
            }
        }
        
        Ok(())
    }
    
    async fn update_task_status(pool: &SqlitePool, task_id: i64, status: CleanupStatus, error_message: Option<&str>) -> SyncResult<()> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        
        match status {
            CleanupStatus::Completed => {
                sqlx::query("UPDATE cleanup_tasks SET status = ?, completed_at = ? WHERE id = ?")
                    .bind(status.to_string())
                    .bind(now)
                    .bind(task_id)
                    .execute(pool)
                    .await?;
            }
            CleanupStatus::Failed => {
                sqlx::query("UPDATE cleanup_tasks SET status = ?, error_message = ? WHERE id = ?")
                    .bind(status.to_string())
                    .bind(error_message)
                    .bind(task_id)
                    .execute(pool)
                    .await?;
            }
            _ => {
                sqlx::query("UPDATE cleanup_tasks SET status = ? WHERE id = ?")
                    .bind(status.to_string())
                    .bind(task_id)
                    .execute(pool)
                    .await?;
            }
        }
        
        Ok(())
    }
    
    async fn cleanup_rootfs(rootfs_path: &str) -> SyncResult<()> {
        if !Path::new(rootfs_path).exists() {
            tracing::debug!("Rootfs path {} does not exist, skipping cleanup", rootfs_path);
            return Ok(());
        }
        
        tracing::debug!("Removing rootfs directory: {}", rootfs_path);
        fs::remove_dir_all(rootfs_path).await.map_err(|e| {
            SyncError::CleanupFailed {
                resource_type: "rootfs".to_string(),
                path: rootfs_path.to_string(),
                message: e.to_string(),
            }
        })?;
        
        Ok(())
    }
    
    async fn cleanup_network(container_id: &str) -> SyncResult<()> {
        // This would integrate with the network manager to clean up
        // network interfaces, IP allocations, etc.
        // For now, we'll just log the operation
        tracing::debug!("Cleaning up network resources for container: {}", container_id);
        
        // In real implementation, this would:
        // 1. Remove veth interfaces
        // 2. Update iptables rules
        // 3. Mark network allocation as cleaned
        // 4. Remove bridge assignments
        
        Ok(())
    }
    
    async fn cleanup_cgroup(cgroup_path: &str) -> SyncResult<()> {
        if !Path::new(cgroup_path).exists() {
            tracing::debug!("Cgroup path {} does not exist, skipping cleanup", cgroup_path);
            return Ok(());
        }
        
        tracing::debug!("Removing cgroup directory: {}", cgroup_path);
        fs::remove_dir_all(cgroup_path).await.map_err(|e| {
            SyncError::CleanupFailed {
                resource_type: "cgroup".to_string(),
                path: cgroup_path.to_string(),
                message: e.to_string(),
            }
        })?;
        
        Ok(())
    }
    
    async fn cleanup_mounts(container_id: &str) -> SyncResult<()> {
        // This would check for any remaining mounts related to the container
        // and unmount them safely
        tracing::debug!("Cleaning up mounts for container: {}", container_id);
        
        // In real implementation, this would:
        // 1. Check /proc/mounts for container-related entries
        // 2. Unmount any remaining filesystems
        // 3. Remove mount points
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::connection::ConnectionManager;
    use crate::sync::schema::SchemaManager;
    use tempfile::{NamedTempFile, TempDir};
    use tokio::fs;
    
    async fn setup_test_db() -> (ConnectionManager, CleanupService) {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();
        
        let conn_manager = ConnectionManager::new(db_path).await.unwrap();
        let schema_manager = SchemaManager::new(conn_manager.pool().clone());
        schema_manager.initialize_schema().await.unwrap();
        
        let cleanup_service = CleanupService::new(conn_manager.pool().clone());
        
        (conn_manager, cleanup_service)
    }
    
    #[tokio::test]
    async fn test_schedule_cleanup_task() {
        let (_conn, cleanup_service) = setup_test_db().await;
        
        let task_id = cleanup_service.schedule_cleanup(
            "test-container",
            ResourceType::Rootfs,
            "/tmp/test-rootfs"
        ).await.unwrap();
        
        assert!(task_id > 0);
        
        let task = cleanup_service.get_task_status(task_id).await.unwrap();
        assert_eq!(task.container_id, "test-container");
        assert_eq!(task.resource_type, ResourceType::Rootfs);
        assert_eq!(task.resource_path, "/tmp/test-rootfs");
        assert_eq!(task.status, CleanupStatus::Pending);
    }
    
    #[tokio::test]
    async fn test_schedule_container_cleanup() {
        let (_conn, cleanup_service) = setup_test_db().await;
        
        // Create a temporary directory for rootfs
        let temp_dir = TempDir::new().unwrap();
        let rootfs_path = temp_dir.path().to_str().unwrap();
        
        let task_ids = cleanup_service.schedule_container_cleanup(
            "test-container",
            Some(rootfs_path)
        ).await.unwrap();
        
        // Should schedule multiple tasks (rootfs, network, cgroup, mounts)
        assert!(task_ids.len() >= 2); // At least rootfs and network
        
        let tasks = cleanup_service.list_container_cleanup_tasks("test-container").await.unwrap();
        assert_eq!(tasks.len(), task_ids.len());
        
        // Check that rootfs task was scheduled
        let rootfs_task = tasks.iter().find(|t| t.resource_type == ResourceType::Rootfs);
        assert!(rootfs_task.is_some());
        assert_eq!(rootfs_task.unwrap().resource_path, rootfs_path);
    }
    
    #[tokio::test]
    async fn test_rootfs_cleanup() {
        let (_conn, cleanup_service) = setup_test_db().await;
        
        // Create a temporary directory with some content
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").await.unwrap();
        
        // Schedule and get cleanup task
        let task_id = cleanup_service.schedule_cleanup(
            "test-container",
            ResourceType::Rootfs,
            temp_dir.path().to_str().unwrap()
        ).await.unwrap();
        
        let task = cleanup_service.get_task_status(task_id).await.unwrap();
        
        // Execute cleanup
        CleanupService::execute_cleanup_task(&cleanup_service.pool, task).await.unwrap();
        
        // Verify directory was removed
        assert!(!temp_dir.path().exists());
        
        // Verify task was marked as completed
        let updated_task = cleanup_service.get_task_status(task_id).await.unwrap();
        assert_eq!(updated_task.status, CleanupStatus::Completed);
        assert!(updated_task.completed_at.is_some());
    }
} 