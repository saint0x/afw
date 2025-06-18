use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use crate::sync::{
    connection::ConnectionManager,
    schema::SchemaManager,
    containers::{ContainerManager, ContainerConfig, ContainerStatus, ContainerState},
    network::{NetworkManager, NetworkConfig, NetworkAllocation},
    monitor::ProcessMonitorService,
    cleanup::CleanupService,
    async_tasks::{AsyncTaskManager, AsyncTask, AsyncTaskStatus},
    error::{SyncError, SyncResult},
};
use std::collections::HashSet;

/// Main sync engine that coordinates all stateful resources
pub struct SyncEngine {
    connection_manager: Arc<ConnectionManager>,
    container_manager: Arc<ContainerManager>,
    network_manager: Arc<NetworkManager>,
    monitor_service: Arc<ProcessMonitorService>,
    cleanup_service: Arc<CleanupService>,
    async_task_manager: Arc<AsyncTaskManager>,
    
    // Background services control
    background_tasks: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
}

impl SyncEngine {
    /// Create a new sync engine with the given database path
    pub async fn new(database_path: &str) -> SyncResult<Self> {
        // Initialize connection
        let connection_manager = Arc::new(ConnectionManager::new(database_path).await?);
        
        // Initialize schema
        let schema_manager = SchemaManager::new(connection_manager.pool().clone());
        schema_manager.initialize_schema().await?;
        
        // Create component managers
        let container_manager = Arc::new(ContainerManager::new(connection_manager.pool().clone()));
        let network_manager = Arc::new(NetworkManager::new(connection_manager.pool().clone()));
        let monitor_service = Arc::new(ProcessMonitorService::new(connection_manager.pool().clone()));
        let cleanup_service = Arc::new(CleanupService::new(connection_manager.pool().clone()));
        let async_task_manager = Arc::new(AsyncTaskManager::new(connection_manager.pool().clone()));
        
        let engine = Self {
            connection_manager,
            container_manager,
            network_manager,
            monitor_service,
            cleanup_service,
            async_task_manager,
            background_tasks: Arc::new(RwLock::new(Vec::new())),
        };
        
        tracing::info!("Sync engine initialized with database: {}", database_path);
        Ok(engine)
    }
    
    /// Start background services for monitoring and cleanup
    pub async fn start_background_services(&self) -> SyncResult<()> {
        let mut tasks = self.background_tasks.write().await;
        
        // Start cleanup worker
        let cleanup_service = self.cleanup_service.clone();
        let cleanup_task = tokio::spawn(async move {
            if let Err(e) = cleanup_service.run_cleanup_worker(5).await {
                tracing::error!("Cleanup worker failed: {}", e);
            }
        });
        tasks.push(cleanup_task);
        
        // Start monitor cleanup task (runs every 5 minutes)
        let monitor_service = self.monitor_service.clone();
        let monitor_cleanup_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
            loop {
                interval.tick().await;
                if let Err(e) = monitor_service.cleanup_stale_monitors(Duration::from_secs(600)).await {
                    tracing::warn!("Failed to cleanup stale monitors: {}", e);
                }
            }
        });
        tasks.push(monitor_cleanup_task);
        
        // Start async task cleanup (runs every hour)
        let async_task_manager = self.async_task_manager.clone();
        let async_cleanup_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // 1 hour
            loop {
                interval.tick().await;
                if let Err(e) = async_task_manager.cleanup_old_tasks(Duration::from_secs(86400)).await { // 24 hours
                    tracing::warn!("Failed to cleanup old async tasks: {}", e);
                }
            }
        });
        tasks.push(async_cleanup_task);
        
        tracing::info!("Started {} background services", tasks.len());
        Ok(())
    }
    
    /// Stop all background services
    pub async fn stop_background_services(&self) {
        let mut tasks = self.background_tasks.write().await;
        
        for task in tasks.drain(..) {
            task.abort();
        }
        
        tracing::info!("Stopped all background services");
    }
    
    /// Close the sync engine and all connections
    pub async fn close(&self) {
        self.stop_background_services().await;
        self.connection_manager.close().await;
        tracing::info!("Sync engine closed");
    }
    
    // === Container Management ===
    
    /// Create a new container with coordinated network allocation
    pub async fn create_container(&self, config: ContainerConfig) -> SyncResult<NetworkConfig> {
        let container_id = config.id.clone();
        
        // Use explicit transaction to ensure container is committed before network allocation
        let mut tx = self.pool().begin().await?;
        
        // 1. Create container record in database FIRST to satisfy foreign key constraints
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64;
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
        .bind("created")
        .bind(config.memory_limit_mb)
        .bind(config.cpu_limit_percent)
        .bind(config.enable_network_namespace)
        .bind(config.enable_pid_namespace)
        .bind(config.enable_mount_namespace)
        .bind(config.enable_uts_namespace)
        .bind(config.enable_ipc_namespace)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        // 2. Allocate network resources if networking is enabled
        let network_config = if config.enable_network_namespace {
            // Find available IP
            let allocated_ips: Vec<(String,)> = sqlx::query_as(
                "SELECT ip_address FROM network_allocations WHERE status != 'cleaned'"
            ).fetch_all(&mut *tx).await?;
            
            let allocated_set: std::collections::HashSet<String> = allocated_ips
                .into_iter()
                .map(|(ip,)| ip)
                .collect();
            
            // Find first available IP in range (172.16.0.10 to 172.16.0.250)
            let start_int = u32::from(std::net::Ipv4Addr::new(172, 16, 0, 10));
            let end_int = u32::from(std::net::Ipv4Addr::new(172, 16, 0, 250));
            
            let mut available_ip = None;
            for ip_int in start_int..=end_int {
                let ip = std::net::Ipv4Addr::from(ip_int);
                let ip_str = ip.to_string();
                
                if !allocated_set.contains(&ip_str) {
                    available_ip = Some(ip_str);
                    break;
                }
            }
            
            let ip = available_ip.ok_or_else(|| SyncError::NoAvailableIp)?;
            
            // Insert network allocation within same transaction
            sqlx::query(r#"
                INSERT INTO network_allocations (
                    container_id, ip_address, allocation_time, setup_completed, status
                ) VALUES (?, ?, ?, ?, ?)
            "#)
            .bind(&container_id)
            .bind(&ip)
            .bind(now)
            .bind(false)
            .bind("allocated")
            .execute(&mut *tx)
            .await?;
            
            Some(NetworkConfig {
                container_id: container_id.clone(),
                ip_address: ip,
                bridge_interface: None,
                veth_host: None,
                veth_container: None,
                setup_required: true,
            })
        } else {
            None
        };
        
        // 3. Commit transaction to ensure atomicity
        tx.commit().await?;
        
        tracing::info!("Created container {} in database with network config", config.id);
        
        // 4. Return network configuration for setup
        Ok(network_config.unwrap_or(NetworkConfig {
            container_id,
            ip_address: String::new(),
            bridge_interface: None,
            veth_host: None,
            veth_container: None,
            setup_required: false,
        }))
    }
    
    /// Update container state with validation
    pub async fn update_container_state(&self, container_id: &str, new_state: ContainerState) -> SyncResult<()> {
        // Clone the state to use it after the move
        let state_for_check = new_state.clone();
        self.container_manager.update_container_state(container_id, new_state).await?;
        
        // Trigger cleanup if container is finished
        if matches!(state_for_check, ContainerState::Exited | ContainerState::Error) {
            self.trigger_cleanup(container_id).await?;
        }
        
        Ok(())
    }
    
    /// Update container state with optional PID and exit code atomically - prevents race conditions
    pub async fn update_container_state_with_details(
        &self, 
        container_id: &str, 
        new_state: ContainerState,
        pid: Option<i64>,
        exit_code: Option<i64>,
    ) -> SyncResult<()> {
        // Clone the state to use it after the move
        let state_for_check = new_state.clone();
        self.container_manager.update_container_state_with_details(container_id, new_state, pid, exit_code).await?;
        
        // Trigger cleanup if container is finished
        if matches!(state_for_check, ContainerState::Exited | ContainerState::Error) {
            self.trigger_cleanup(container_id).await?;
        }
        
        Ok(())
    }
    
    /// Set container PID and start monitoring
    pub async fn set_container_pid(&self, container_id: &str, pid: nix::unistd::Pid) -> SyncResult<()> {
        // Update container record
        self.container_manager.set_container_pid(container_id, pid.as_raw() as i64).await?;
        
        // Start background monitoring (non-blocking)
        self.monitor_service.start_monitoring(container_id, pid).await?;
        
        Ok(())
    }
    
    /// Set container exit code
    pub async fn set_container_exit_code(&self, container_id: &str, exit_code: i64) -> SyncResult<()> {
        self.container_manager.set_container_exit_code(container_id, exit_code).await
    }
    
    /// Set rootfs path
    pub async fn set_rootfs_path(&self, container_id: &str, rootfs_path: &str) -> SyncResult<()> {
        self.container_manager.set_rootfs_path(container_id, rootfs_path).await
    }
    
    /// Get container status (always fast - direct database query)
    pub async fn get_container_status(&self, container_id: &str) -> SyncResult<ContainerStatus> {
        self.container_manager.get_container_status(container_id).await
    }
    
    /// List containers with optional state filter
    pub async fn list_containers(&self, state_filter: Option<ContainerState>) -> SyncResult<Vec<ContainerStatus>> {
        self.container_manager.list_containers(state_filter).await
    }
    
    /// Delete container and all associated resources
    pub async fn delete_container(&self, container_id: &str) -> SyncResult<()> {
        // Stop monitoring if active
        let _ = self.monitor_service.stop_monitoring(container_id).await;
        
        // Get container info for cleanup
        let status = self.container_manager.get_container_status(container_id).await?;
        
        // Schedule cleanup tasks
        self.cleanup_service.schedule_container_cleanup(
            container_id,
            status.rootfs_path.as_deref()
        ).await?;
        
        // Mark network for cleanup
        if let Ok(_) = self.network_manager.get_network_allocation(container_id).await {
            self.network_manager.mark_network_cleanup_pending(container_id).await?;
        }
        
        // Delete container record
        self.container_manager.delete_container(container_id).await?;
        
        tracing::info!("Scheduled full cleanup for container {}", container_id);
        Ok(())
    }
    
    // === Network Management ===
    
    /// Check if container should have network setup
    pub async fn should_setup_network(&self, container_id: &str) -> SyncResult<bool> {
        self.network_manager.should_setup_network(container_id).await
    }
    
    /// Mark network setup as complete
    pub async fn mark_network_setup_complete(&self, container_id: &str, bridge_interface: &str, veth_host: &str, veth_container: &str) -> SyncResult<()> {
        self.network_manager.mark_network_setup_complete(container_id, bridge_interface, veth_host, veth_container).await
    }
    
    /// Get network allocation for container
    pub async fn get_network_allocation(&self, container_id: &str) -> SyncResult<NetworkAllocation> {
        self.network_manager.get_network_allocation(container_id).await
    }
    
    /// List all network allocations
    pub async fn list_network_allocations(&self) -> SyncResult<Vec<NetworkAllocation>> {
        self.network_manager.list_allocations(None).await
    }
    
    // === Process Monitoring ===
    
    /// Get process monitor status
    pub async fn get_monitor_status(&self, container_id: &str) -> SyncResult<crate::sync::monitor::ProcessMonitor> {
        self.monitor_service.get_monitor_status(container_id).await
    }
    
    /// List all active monitors
    pub async fn list_active_monitors(&self) -> SyncResult<Vec<crate::sync::monitor::ProcessMonitor>> {
        self.monitor_service.list_active_monitors().await
    }
    
    /// Stop monitoring a container
    pub async fn stop_monitoring(&self, container_id: &str) -> SyncResult<()> {
        self.monitor_service.stop_monitoring(container_id).await
    }
    
    // === Cleanup Management ===
    
    /// Trigger cleanup for a container
    pub async fn trigger_cleanup(&self, container_id: &str) -> SyncResult<()> {
        // Get container info
        let status = self.container_manager.get_container_status(container_id).await?;
        
        // Schedule cleanup tasks
        self.cleanup_service.schedule_container_cleanup(
            container_id,
            status.rootfs_path.as_deref()
        ).await?;
        
        // Mark network for cleanup if allocated
        if let Ok(_) = self.network_manager.get_network_allocation(container_id).await {
            self.network_manager.mark_network_cleanup_pending(container_id).await?;
        }
        
        Ok(())
    }
    
    /// List cleanup tasks for a container
    pub async fn list_cleanup_tasks(&self, container_id: &str) -> SyncResult<Vec<crate::sync::cleanup::CleanupTask>> {
        self.cleanup_service.list_container_cleanup_tasks(container_id).await
    }
    
    // === Async Task Management ===
    
    /// Submit an async exec task for a container
    pub async fn submit_async_exec_task(
        &self,
        container_id: &str,
        command: Vec<String>,
        timeout_seconds: Option<i64>,
    ) -> SyncResult<String> {
        self.async_task_manager.submit_exec_task(container_id, command, timeout_seconds).await
    }
    
    /// Cancel a running async task
    pub async fn cancel_async_task(&self, task_id: &str) -> SyncResult<bool> {
        self.async_task_manager.cancel_task(task_id).await
    }
    
    /// Get async task status and results
    pub async fn get_async_task_status(&self, task_id: &str) -> SyncResult<AsyncTask> {
        self.async_task_manager.get_task_status(task_id).await
    }
    
    /// List async tasks for a container
    pub async fn list_async_tasks(&self, container_id: &str) -> SyncResult<Vec<AsyncTask>> {
        self.async_task_manager.list_container_tasks(container_id).await
    }
    
    // === Utility Methods ===
    
    /// Check if container exists
    pub async fn container_exists(&self, container_id: &str) -> SyncResult<bool> {
        self.container_manager.container_exists(container_id).await
    }
    
    /// Get database connection pool for advanced operations
    pub fn pool(&self) -> &sqlx::SqlitePool {
        self.connection_manager.pool()
    }
    
    /// Get sync engine statistics
    pub async fn get_stats(&self) -> SyncResult<SyncEngineStats> {
        let containers = self.container_manager.list_containers(None).await?;
        let active_monitors = self.monitor_service.list_active_monitors().await?;
        let network_allocations = self.network_manager.list_allocations(None).await?;
        
        let running_containers = containers.iter().filter(|c| c.state == ContainerState::Running).count();
        let total_containers = containers.len();
        let active_networks = network_allocations.iter().filter(|n| n.setup_completed).count();
        let active_monitors_count = active_monitors.len();
        
        Ok(SyncEngineStats {
            total_containers,
            running_containers,
            active_networks,
            active_monitors: active_monitors_count,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SyncEngineStats {
    pub total_containers: usize,
    pub running_containers: usize,
    pub active_networks: usize,
    pub active_monitors: usize,
}

impl Drop for SyncEngine {
    fn drop(&mut self) {
        // Note: Can't call async methods in Drop, so background services
        // should be explicitly stopped before dropping
        tracing::debug!("SyncEngine dropped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::collections::HashMap;
    
    async fn setup_test_engine() -> SyncEngine {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();
        
        SyncEngine::new(db_path).await.unwrap()
    }
    
    #[tokio::test]
    async fn test_sync_engine_creation() {
        let engine = setup_test_engine().await;
        
        let stats = engine.get_stats().await.unwrap();
        assert_eq!(stats.total_containers, 0);
        assert_eq!(stats.running_containers, 0);
        assert_eq!(stats.active_networks, 0);
        assert_eq!(stats.active_monitors, 0);
        
        engine.close().await;
    }
    
    #[tokio::test]
    async fn test_container_lifecycle_integration() {
        let engine = setup_test_engine().await;
        
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
        let network_config = engine.create_container(config).await.unwrap();
        assert!(!network_config.ip_address.is_empty());
        assert!(network_config.setup_required);
        
        // Check initial status
        let status = engine.get_container_status("test-container").await.unwrap();
        assert_eq!(status.state, ContainerState::Created);
        assert_eq!(status.ip_address, Some(network_config.ip_address.clone()));
        
        // Transition through states
        engine.update_container_state("test-container", ContainerState::Starting).await.unwrap();
        
        // Set PID (would normally come from actual process creation)
        let test_pid = nix::unistd::Pid::from_raw(12345);
        engine.set_container_pid("test-container", test_pid).await.unwrap();
        
        engine.update_container_state("test-container", ContainerState::Running).await.unwrap();
        
        // Complete network setup
        engine.mark_network_setup_complete("test-container", "br0", "veth123", "eth0").await.unwrap();
        
        // Verify final state
        let final_status = engine.get_container_status("test-container").await.unwrap();
        assert_eq!(final_status.state, ContainerState::Running);
        assert_eq!(final_status.pid, Some(12345));
        
        let network_allocation = engine.get_network_allocation("test-container").await.unwrap();
        assert!(network_allocation.setup_completed);
        assert_eq!(network_allocation.bridge_interface, Some("br0".to_string()));
        
        // Clean up
        engine.delete_container("test-container").await.unwrap();
        engine.close().await;
    }
    
    #[tokio::test]
    async fn test_network_disabled_container() {
        let engine = setup_test_engine().await;
        
        let config = ContainerConfig {
            id: "no-network-container".to_string(),
            name: None,
            image_path: "/path/to/image".to_string(),
            command: "echo hello".to_string(),
            environment: HashMap::new(),
            memory_limit_mb: None,
            cpu_limit_percent: None,
            enable_network_namespace: false, // Networking disabled
            enable_pid_namespace: true,
            enable_mount_namespace: true,
            enable_uts_namespace: true,
            enable_ipc_namespace: true,
        };
        
        // Create container
        let network_config = engine.create_container(config).await.unwrap();
        assert_eq!(network_config.ip_address, "");
        assert!(!network_config.setup_required);
        
        // Should not have network allocation
        assert!(!engine.should_setup_network("no-network-container").await.unwrap());
        
        let status = engine.get_container_status("no-network-container").await.unwrap();
        assert_eq!(status.ip_address, None);
        
        engine.close().await;
    }
    
    #[tokio::test]
    async fn test_stats_collection() {
        let engine = setup_test_engine().await;
        
        // Create some test containers
        for i in 0..3 {
            let config = ContainerConfig {
                id: format!("container-{}", i),
                name: Some(format!("test-{}", i)),
                image_path: "/path/to/image".to_string(),
                command: "echo hello".to_string(),
                environment: HashMap::new(),
                memory_limit_mb: None,
                cpu_limit_percent: None,
                enable_network_namespace: i % 2 == 0, // Half with networking
                enable_pid_namespace: true,
                enable_mount_namespace: true,
                enable_uts_namespace: true,
                enable_ipc_namespace: true,
            };
            
            engine.create_container(config).await.unwrap();
            
            // Start one container
            if i == 0 {
                engine.update_container_state(&format!("container-{}", i), ContainerState::Starting).await.unwrap();
                engine.update_container_state(&format!("container-{}", i), ContainerState::Running).await.unwrap();
            }
        }
        
        let stats = engine.get_stats().await.unwrap();
        assert_eq!(stats.total_containers, 3);
        assert_eq!(stats.running_containers, 1);
        assert_eq!(stats.active_networks, 0); // None completed setup
        
        engine.close().await;
    }
} 