use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::sync::error::{SyncError, SyncResult};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContainerState {
    Created,
    Starting,
    Running,
    Exited,
    Error,
}

impl ContainerState {
    pub fn to_string(&self) -> String {
        match self {
            ContainerState::Created => "created".to_string(),
            ContainerState::Starting => "starting".to_string(),
            ContainerState::Running => "running".to_string(),
            ContainerState::Exited => "exited".to_string(),
            ContainerState::Error => "error".to_string(),
        }
    }
    
    pub fn from_string(s: &str) -> SyncResult<Self> {
        match s {
            "created" => Ok(ContainerState::Created),
            "starting" => Ok(ContainerState::Starting),
            "running" => Ok(ContainerState::Running),
            "exited" => Ok(ContainerState::Exited),
            "error" => Ok(ContainerState::Error),
            _ => Err(SyncError::ValidationFailed {
                message: format!("Invalid container state: {}", s),
            }),
        }
    }
    
    pub fn can_transition_to(&self, new_state: &ContainerState) -> bool {
        match (self, new_state) {
            (ContainerState::Created, ContainerState::Starting) => true,
            (ContainerState::Starting, ContainerState::Running) => true,
            (ContainerState::Starting, ContainerState::Error) => true,
            (ContainerState::Running, ContainerState::Exited) => true,
            (ContainerState::Running, ContainerState::Error) => true,
            (_, ContainerState::Error) => true, // Can always transition to error
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    pub id: String,
    pub name: Option<String>,
    pub image_path: String,
    pub command: String,
    pub environment: HashMap<String, String>,
    pub memory_limit_mb: Option<i64>,
    pub cpu_limit_percent: Option<f64>,
    
    // Namespace configuration
    pub enable_network_namespace: bool,
    pub enable_pid_namespace: bool,
    pub enable_mount_namespace: bool,
    pub enable_uts_namespace: bool,
    pub enable_ipc_namespace: bool,
}

#[derive(Debug, Clone)]
pub struct ContainerStatus {
    pub id: String,
    pub name: Option<String>,
    pub state: ContainerState,
    pub pid: Option<i64>,
    pub exit_code: Option<i64>,
    pub ip_address: Option<String>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub exited_at: Option<i64>,
    pub rootfs_path: Option<String>,
}

pub struct ContainerManager {
    pool: SqlitePool,
}

impl ContainerManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
    
    /// Check if a PID is still alive by checking /proc filesystem
    fn is_pid_alive(pid: i64) -> bool {
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
    }
    
    pub async fn create_container(&self, config: ContainerConfig) -> SyncResult<()> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        let environment_json = serde_json::to_string(&config.environment)?;
        
        sqlx::query(r#"
            INSERT INTO containers (
                id, name, image_path, command, environment, state,
                memory_limit_mb, cpu_limit_percent,
                enable_network_namespace, enable_pid_namespace, enable_mount_namespace,
                enable_uts_namespace, enable_ipc_namespace,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&config.id)
        .bind(&config.name)
        .bind(&config.image_path)
        .bind(&config.command)
        .bind(&environment_json)
        .bind(ContainerState::Created.to_string())
        .bind(config.memory_limit_mb)
        .bind(config.cpu_limit_percent)
        .bind(config.enable_network_namespace)
        .bind(config.enable_pid_namespace)
        .bind(config.enable_mount_namespace)
        .bind(config.enable_uts_namespace)
        .bind(config.enable_ipc_namespace)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;
        
        tracing::info!("Created container {} in database", config.id);
        Ok(())
    }
    
    pub async fn update_container_state(&self, container_id: &str, new_state: ContainerState) -> SyncResult<()> {
        let current_state = self.get_container_state(container_id).await?;
        
        if !current_state.can_transition_to(&new_state) {
            return Err(SyncError::InvalidStateTransition {
                from: current_state.to_string(),
                to: new_state.to_string(),
            });
        }
        
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        
        // Handle state-specific updates
        let query = match new_state {
            ContainerState::Running => {
                sqlx::query("UPDATE containers SET state = ?, started_at = ?, updated_at = ? WHERE id = ?")
                    .bind(new_state.to_string())
                    .bind(now)
                    .bind(now)
                    .bind(container_id)
            },
            ContainerState::Exited => {
                sqlx::query("UPDATE containers SET state = ?, exited_at = ?, updated_at = ? WHERE id = ?")
                    .bind(new_state.to_string())
                    .bind(now)
                    .bind(now)
                    .bind(container_id)
            },
            _ => {
                sqlx::query("UPDATE containers SET state = ?, updated_at = ? WHERE id = ?")
                    .bind(new_state.to_string())
                    .bind(now)
                    .bind(container_id)
            }
        };
        
        let result = query.execute(&self.pool).await?;
        
        if result.rows_affected() == 0 {
            return Err(SyncError::NotFound {
                container_id: container_id.to_string(),
            });
        }
        
        tracing::debug!("Updated container {} state to {}", container_id, new_state.to_string());
        Ok(())
    }
    
    pub async fn set_container_pid(&self, container_id: &str, pid: i64) -> SyncResult<()> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        
        let result = sqlx::query("UPDATE containers SET pid = ?, updated_at = ? WHERE id = ?")
            .bind(pid)
            .bind(now)
            .bind(container_id)
            .execute(&self.pool)
            .await?;
        
        if result.rows_affected() == 0 {
            return Err(SyncError::NotFound {
                container_id: container_id.to_string(),
            });
        }
        
        tracing::debug!("Set container {} pid to {}", container_id, pid);
        Ok(())
    }
    
    pub async fn set_container_exit_code(&self, container_id: &str, exit_code: i64) -> SyncResult<()> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        
        let result = sqlx::query("UPDATE containers SET exit_code = ?, updated_at = ? WHERE id = ?")
            .bind(exit_code)
            .bind(now)
            .bind(container_id)
            .execute(&self.pool)
            .await?;
        
        if result.rows_affected() == 0 {
            return Err(SyncError::NotFound {
                container_id: container_id.to_string(),
            });
        }
        
        tracing::debug!("Set container {} exit code to {}", container_id, exit_code);
        Ok(())
    }
    
    pub async fn set_rootfs_path(&self, container_id: &str, rootfs_path: &str) -> SyncResult<()> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        
        let result = sqlx::query("UPDATE containers SET rootfs_path = ?, updated_at = ? WHERE id = ?")
            .bind(rootfs_path)
            .bind(now)
            .bind(container_id)
            .execute(&self.pool)
            .await?;
        
        if result.rows_affected() == 0 {
            return Err(SyncError::NotFound {
                container_id: container_id.to_string(),
            });
        }
        
        Ok(())
    }
    
    pub async fn get_container_status(&self, container_id: &str) -> SyncResult<ContainerStatus> {
        let row = sqlx::query(r#"
            SELECT 
                c.id, c.name, c.state, c.pid, c.exit_code, c.created_at, 
                c.started_at, c.exited_at, c.rootfs_path,
                n.ip_address
            FROM containers c 
            LEFT JOIN network_allocations n ON c.id = n.container_id 
            WHERE c.id = ?
        "#)
        .bind(container_id)
        .fetch_optional(&self.pool)
        .await?;
        
        match row {
            Some(row) => {
                let state_str: String = row.get("state");
                let mut state = ContainerState::from_string(&state_str)?;
                let mut pid: Option<i64> = row.get("pid");
                let mut exit_code: Option<i64> = row.get("exit_code");
                
                // CRITICAL FIX: Validate PID is still alive if container claims to be running
                if state == ContainerState::Running {
                    if let Some(container_pid) = pid {
                        // Check if the process actually exists
                        if !Self::is_pid_alive(container_pid) {
                            tracing::warn!("Container {} claims to be running but PID {} is dead - auto-correcting state", container_id, container_pid);
                            
                            // Auto-correct the stale state
                            state = ContainerState::Exited;
                            pid = None;
                            exit_code = Some(127); // Process died unexpectedly
                            
                            // Update database asynchronously (don't block the query)
                            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64;
                            let _ = sqlx::query("UPDATE containers SET state = 'exited', pid = NULL, exit_code = 127, exited_at = ?, updated_at = ? WHERE id = ?")
                                .bind(now)
                                .bind(now)
                                .bind(container_id)
                                .execute(&self.pool)
                                .await;
                        }
                    } else {
                        // Running state without PID is invalid
                        tracing::warn!("Container {} claims to be running but has no PID - correcting state", container_id);
                        state = ContainerState::Error;
                        
                        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64;
                        let _ = sqlx::query("UPDATE containers SET state = 'error', updated_at = ? WHERE id = ?")
                            .bind(now)
                            .bind(container_id)
                            .execute(&self.pool)
                            .await;
                    }
                }
                
                Ok(ContainerStatus {
                    id: row.get("id"),
                    name: row.get("name"),
                    state,
                    pid,
                    exit_code,
                    ip_address: row.get("ip_address"),
                    created_at: row.get("created_at"),
                    started_at: row.get("started_at"),
                    exited_at: row.get("exited_at"),
                    rootfs_path: row.get("rootfs_path"),
                })
            }
            None => Err(SyncError::NotFound {
                container_id: container_id.to_string(),
            }),
        }
    }
    
    pub async fn get_container_state(&self, container_id: &str) -> SyncResult<ContainerState> {
        let state_str: Option<String> = sqlx::query_scalar("SELECT state FROM containers WHERE id = ?")
            .bind(container_id)
            .fetch_optional(&self.pool)
            .await?;
        
        match state_str {
            Some(state) => ContainerState::from_string(&state),
            None => Err(SyncError::NotFound {
                container_id: container_id.to_string(),
            }),
        }
    }
    
    pub async fn list_containers(&self, state_filter: Option<ContainerState>) -> SyncResult<Vec<ContainerStatus>> {
        let mut query = "
            SELECT 
                c.id, c.name, c.state, c.pid, c.exit_code, c.created_at, 
                c.started_at, c.exited_at, c.rootfs_path,
                n.ip_address
            FROM containers c 
            LEFT JOIN network_allocations n ON c.id = n.container_id
        ".to_string();
        
        if let Some(state) = state_filter {
            query.push_str(&format!(" WHERE c.state = '{}'", state.to_string()));
        }
        
        query.push_str(" ORDER BY c.created_at DESC");
        
        let rows = sqlx::query(&query).fetch_all(&self.pool).await?;
        
        let mut containers = Vec::new();
        for row in rows {
            let state_str: String = row.get("state");
            let state = ContainerState::from_string(&state_str)?;
            
            containers.push(ContainerStatus {
                id: row.get("id"),
                name: row.get("name"),
                state,
                pid: row.get("pid"),
                exit_code: row.get("exit_code"),
                ip_address: row.get("ip_address"),
                created_at: row.get("created_at"),
                started_at: row.get("started_at"),
                exited_at: row.get("exited_at"),
                rootfs_path: row.get("rootfs_path"),
            });
        }
        
        Ok(containers)
    }
    
    pub async fn delete_container(&self, container_id: &str) -> SyncResult<()> {
        let result = sqlx::query("DELETE FROM containers WHERE id = ?")
            .bind(container_id)
            .execute(&self.pool)
            .await?;
        
        if result.rows_affected() == 0 {
            return Err(SyncError::NotFound {
                container_id: container_id.to_string(),
            });
        }
        
        tracing::info!("Deleted container {} from database", container_id);
        Ok(())
    }
    
    pub async fn container_exists(&self, container_id: &str) -> SyncResult<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM containers WHERE id = ?")
            .bind(container_id)
            .fetch_one(&self.pool)
            .await?;
        
        Ok(count > 0)
    }
    
    pub async fn get_containers_needing_cleanup(&self) -> SyncResult<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT id FROM containers WHERE state IN ('exited', 'error') AND id NOT IN (SELECT container_id FROM cleanup_tasks WHERE status = 'completed')"
        ).fetch_all(&self.pool).await?;
        
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::connection::ConnectionManager;
    use crate::sync::schema::SchemaManager;
    use tempfile::NamedTempFile;
    use std::collections::HashMap;
    
    async fn setup_test_db() -> (ConnectionManager, ContainerManager) {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();
        
        let conn_manager = ConnectionManager::new(db_path).await.unwrap();
        let schema_manager = SchemaManager::new(conn_manager.pool().clone());
        schema_manager.initialize_schema().await.unwrap();
        
        let container_manager = ContainerManager::new(conn_manager.pool().clone());
        
        (conn_manager, container_manager)
    }
    
    #[tokio::test]
    async fn test_container_lifecycle() {
        let (_conn, container_manager) = setup_test_db().await;
        
        let config = ContainerConfig {
            id: "test-container".to_string(),
            name: Some("test".to_string()),
            image_path: "/path/to/image".to_string(),
            command: "echo hello".to_string(),
            environment: HashMap::new(),
            memory_limit_mb: Some(1024),
            cpu_limit_percent: Some(50.0),
            enable_network_namespace: true,
            enable_pid_namespace: true,
            enable_mount_namespace: true,
            enable_uts_namespace: true,
            enable_ipc_namespace: true,
        };
        
        // Create container
        container_manager.create_container(config).await.unwrap();
        
        // Check initial state
        let status = container_manager.get_container_status("test-container").await.unwrap();
        assert_eq!(status.state, ContainerState::Created);
        
        // Transition to starting
        container_manager.update_container_state("test-container", ContainerState::Starting).await.unwrap();
        let status = container_manager.get_container_status("test-container").await.unwrap();
        assert_eq!(status.state, ContainerState::Starting);
        
        // Set PID and transition to running
        container_manager.set_container_pid("test-container", 12345).await.unwrap();
        container_manager.update_container_state("test-container", ContainerState::Running).await.unwrap();
        
        let status = container_manager.get_container_status("test-container").await.unwrap();
        assert_eq!(status.state, ContainerState::Running);
        assert_eq!(status.pid, Some(12345));
        
        // Finish with exit code
        container_manager.set_container_exit_code("test-container", 0).await.unwrap();
        container_manager.update_container_state("test-container", ContainerState::Exited).await.unwrap();
        
        let status = container_manager.get_container_status("test-container").await.unwrap();
        assert_eq!(status.state, ContainerState::Exited);
        assert_eq!(status.exit_code, Some(0));
    }
    
    #[tokio::test]
    async fn test_invalid_state_transition() {
        let (_conn, container_manager) = setup_test_db().await;
        
        let config = ContainerConfig {
            id: "test-container-2".to_string(),
            name: None,
            image_path: "/path/to/image".to_string(),
            command: "echo hello".to_string(),
            environment: HashMap::new(),
            memory_limit_mb: None,
            cpu_limit_percent: None,
            enable_network_namespace: false,
            enable_pid_namespace: false,
            enable_mount_namespace: false,
            enable_uts_namespace: false,
            enable_ipc_namespace: false,
        };
        
        container_manager.create_container(config).await.unwrap();
        
        // Try invalid transition from created to running (should go through starting)
        let result = container_manager.update_container_state("test-container-2", ContainerState::Running).await;
        assert!(result.is_err());
        
        if let Err(SyncError::InvalidStateTransition { from, to }) = result {
            assert_eq!(from, "created");
            assert_eq!(to, "running");
        } else {
            panic!("Expected InvalidStateTransition error");
        }
    }
} 