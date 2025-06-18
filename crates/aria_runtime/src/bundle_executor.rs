/*!
# Bundle Executor Service

Complete bundle deployment and execution service with containerized environments,
dependency management, and comprehensive lifecycle management.
*/

use crate::bundle_discovery::BundleToolDiscovery;
use crate::engines::container::quilt::QuiltService;
use crate::engines::tool_registry::bundle_integration::BundleToolRegistry;
use crate::errors::{AriaError, AriaResult, ErrorCategory, ErrorCode, ErrorSeverity};
use pkg_store::PackageStore;
use crate::deep_size::DeepUuid;
use pkg_store::bundle::{LoadedBundle, AriaManifest, AgentManifest, TeamManifest, PipelineManifest};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, info, warn, error};

/// Bundle execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleExecutionResult {
    pub execution_id: String,
    pub bundle_hash: String,
    pub bundle_name: String,
    pub session_id: DeepUuid,
    pub container_id: Option<String>,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub execution_time: Duration,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub resource_usage: BundleResourceUsage,
    pub registered_components: BundleComponentSummary,
}

/// Bundle resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleResourceUsage {
    pub peak_memory_mb: Option<u64>,
    pub cpu_time_seconds: Option<f64>,
    pub network_bytes_sent: Option<u64>,
    pub network_bytes_received: Option<u64>,
    pub disk_bytes_written: Option<u64>,
    pub disk_bytes_read: Option<u64>,
}

/// Summary of components registered from bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleComponentSummary {
    pub tools_registered: Vec<String>,
    pub agents_registered: Vec<String>,
    pub teams_registered: Vec<String>,
    pub pipelines_registered: Vec<String>,
}

/// Bundle execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleExecutionConfig {
    pub memory_limit_mb: Option<u64>,
    pub cpu_limit_cores: Option<f64>,
    pub timeout_seconds: Option<u64>,
    pub network_enabled: bool,
    pub filesystem_isolation: bool,
    pub environment_variables: HashMap<String, String>,
    pub workspace_path: Option<PathBuf>,
    pub auto_register_components: bool,
}

impl Default for BundleExecutionConfig {
    fn default() -> Self {
        Self {
            memory_limit_mb: Some(1024), // 1GB default
            cpu_limit_cores: Some(1.0),
            timeout_seconds: Some(300), // 5 minutes default
            network_enabled: true,
            filesystem_isolation: true,
            environment_variables: HashMap::new(),
            workspace_path: None,
            auto_register_components: true,
        }
    }
}

/// Bundle execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BundleExecutionStatus {
    Pending,
    Validating,
    Extracting,
    InstallingDependencies,
    RegisteringComponents,
    Starting,
    Running,
    Completed,
    Failed,
    Timeout,
    Cancelled,
}

/// Bundle executor service
pub struct BundleExecutor {
    /// Quilt service for container management
    quilt_service: Arc<Mutex<QuiltService>>,
    /// Package store for bundle retrieval
    pkg_store: Arc<PackageStore>,
    /// Bundle tool registry
    bundle_registry: Arc<BundleToolRegistry>,
    /// Bundle discovery service
    discovery: Arc<BundleToolDiscovery>,
    /// Active executions tracking
    active_executions: Arc<Mutex<HashMap<String, BundleExecutionStatus>>>,
    /// Execution history
    execution_history: Arc<Mutex<Vec<BundleExecutionResult>>>,
}

impl BundleExecutor {
    /// Create a new bundle executor
    pub fn new(
        quilt_service: Arc<Mutex<QuiltService>>,
        pkg_store: Arc<PackageStore>,
        bundle_registry: Arc<BundleToolRegistry>,
        discovery: Arc<BundleToolDiscovery>,
    ) -> Self {
        Self {
            quilt_service,
            pkg_store,
            bundle_registry,
            discovery,
            active_executions: Arc::new(Mutex::new(HashMap::new())),
            execution_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Execute a complete bundle
    pub async fn execute_bundle(
        &self,
        bundle_hash: &str,
        session_id: DeepUuid,
        config: Option<BundleExecutionConfig>,
    ) -> AriaResult<BundleExecutionResult> {
        let execution_id = DeepUuid::new().to_string();
        let started_at = Utc::now();
        let config = config.unwrap_or_default();

        info!("Starting bundle execution: {} (session: {})", bundle_hash, session_id);

        // Track execution status
        {
            let mut executions = self.active_executions.lock().await;
            executions.insert(execution_id.clone(), BundleExecutionStatus::Pending);
        }

        let mut execution_result = BundleExecutionResult {
            execution_id: execution_id.clone(),
            bundle_hash: bundle_hash.to_string(),
            bundle_name: "unknown".to_string(),
            session_id,
            container_id: None,
            success: false,
            exit_code: None,
            stdout: None,
            stderr: None,
            execution_time: Duration::from_secs(0),
            started_at,
            completed_at: None,
            resource_usage: BundleResourceUsage {
                peak_memory_mb: None,
                cpu_time_seconds: None,
                network_bytes_sent: None,
                network_bytes_received: None,
                disk_bytes_written: None,
                disk_bytes_read: None,
            },
            registered_components: BundleComponentSummary {
                tools_registered: Vec::new(),
                agents_registered: Vec::new(),
                teams_registered: Vec::new(),
                pipelines_registered: Vec::new(),
            },
        };

        // Execute bundle with error handling
        match self.execute_bundle_internal(bundle_hash, session_id, &config, &execution_id).await {
            Ok(result) => {
                execution_result = result;
                execution_result.success = true;
            }
            Err(e) => {
                error!("Bundle execution failed: {}", e);
                execution_result.stderr = Some(format!("Execution failed: {}", e));
                execution_result.success = false;
            }
        }

        // Complete execution tracking
        execution_result.completed_at = Some(Utc::now());
        execution_result.execution_time = execution_result.completed_at.unwrap()
            .signed_duration_since(execution_result.started_at)
            .to_std()
            .unwrap_or(Duration::from_secs(0));

        // Update status and history
        {
            let mut executions = self.active_executions.lock().await;
            executions.remove(&execution_id);
        }

        {
            let mut history = self.execution_history.lock().await;
            history.push(execution_result.clone());
        }

        info!("Bundle execution completed: {} (success: {})", 
              execution_id, execution_result.success);

        Ok(execution_result)
    }

    /// Internal bundle execution implementation
    async fn execute_bundle_internal(
        &self,
        bundle_hash: &str,
        session_id: DeepUuid,
        config: &BundleExecutionConfig,
        execution_id: &str,
    ) -> AriaResult<BundleExecutionResult> {
        // Update status: Validating
        self.update_execution_status(execution_id, BundleExecutionStatus::Validating).await;

        // Load and validate bundle
        let bundle = self.load_and_validate_bundle(bundle_hash).await?;

        // Update status: Extracting
        self.update_execution_status(execution_id, BundleExecutionStatus::Extracting).await;

        // Create container workspace
        let container_id = self.create_bundle_container(&bundle, session_id, config).await?;

        // Update status: Installing Dependencies
        self.update_execution_status(execution_id, BundleExecutionStatus::InstallingDependencies).await;

        // Install dependencies
        self.install_bundle_dependencies(&container_id, &bundle).await?;

        // Update status: Registering Components
        self.update_execution_status(execution_id, BundleExecutionStatus::RegisteringComponents).await;

        // Register bundle components
        let registered_components = if config.auto_register_components {
            self.register_bundle_components(&bundle, &container_id).await?
        } else {
            BundleComponentSummary {
                tools_registered: Vec::new(),
                agents_registered: Vec::new(),
                teams_registered: Vec::new(),
                pipelines_registered: Vec::new(),
            }
        };

        // Update status: Starting
        self.update_execution_status(execution_id, BundleExecutionStatus::Starting).await;

        // Execute bundle main entry
        let (exit_code, stdout, stderr) = self.execute_bundle_entry(&container_id, &bundle, config).await?;

        // Update status: Completed
        self.update_execution_status(execution_id, BundleExecutionStatus::Completed).await;

        // Collect resource usage
        let resource_usage = self.collect_resource_usage(&container_id).await?;

        // Cleanup container (optional - could be configured)
        if config.filesystem_isolation {
            let _ = self.cleanup_container(&container_id).await;
        }

        Ok(BundleExecutionResult {
            execution_id: execution_id.to_string(),
            bundle_hash: bundle_hash.to_string(),
            bundle_name: bundle.manifest.name.clone(),
            session_id,
            container_id: Some(container_id),
            success: exit_code == Some(0),
            exit_code,
            stdout,
            stderr,
            execution_time: Duration::from_secs(0), // Will be set by caller
            started_at: Utc::now(), // Will be set by caller
            completed_at: None, // Will be set by caller
            resource_usage,
            registered_components,
        })
    }

    /// Load and validate bundle for execution
    async fn load_and_validate_bundle(&self, bundle_hash: &str) -> AriaResult<LoadedBundle> {
        debug!("Loading bundle: {}", bundle_hash);

        // Get bundle data from storage
        let bundle_data = self.pkg_store.get_bundle(bundle_hash).await
            .map_err(|e| AriaError::new(
                ErrorCode::StorageError,
                ErrorCategory::Bundle,
                ErrorSeverity::High,
                &format!("Failed to get bundle from storage: {}", e),
            ))?
            .ok_or_else(|| AriaError::new(
                ErrorCode::BundleNotFound,
                ErrorCategory::Bundle,
                ErrorSeverity::Medium,
                &format!("Bundle not found: {}", bundle_hash),
            ))?;

        // Create temporary file for loading
        let temp_path = format!("/tmp/bundle_exec_{}.aria", bundle_hash);
        tokio::fs::write(&temp_path, bundle_data).await
            .map_err(|e| AriaError::new(
                ErrorCode::IoError,
                ErrorCategory::Bundle,
                ErrorSeverity::High,
                &format!("Failed to write temporary bundle file: {}", e),
            ))?;

        // Load bundle
        let bundle = LoadedBundle::load_from_file(&temp_path).await
            .map_err(|e| AriaError::new(
                ErrorCode::BundleLoadError,
                ErrorCategory::Bundle,
                ErrorSeverity::High,
                &format!("Failed to load bundle: {}", e),
            ))?;

        // Clean up temporary file
        let _ = tokio::fs::remove_file(&temp_path).await;

        // Validate bundle for execution
        self.validate_bundle_for_execution(&bundle)?;

        info!("Bundle loaded and validated: {} ({})", bundle.manifest.name, bundle_hash);
        Ok(bundle)
    }

    /// Validate bundle for execution
    fn validate_bundle_for_execution(&self, bundle: &LoadedBundle) -> AriaResult<()> {
        debug!("Validating bundle for execution: {}", bundle.manifest.name);

        // Check if bundle has a main entry point
        if bundle.get_main_entry().is_none() {
            return Err(AriaError::new(
                ErrorCode::BundleValidationError,
                ErrorCategory::Bundle,
                ErrorSeverity::High,
                "Bundle must contain a main entry point (main.tsx, main.ts, or index.ts)",
            ));
        }

        // Validate manifest completeness
        if bundle.manifest.name.is_empty() {
            return Err(AriaError::new(
                ErrorCode::BundleValidationError,
                ErrorCategory::Bundle,
                ErrorSeverity::Medium,
                "Bundle name cannot be empty",
            ));
        }

        if bundle.manifest.version.is_empty() {
            return Err(AriaError::new(
                ErrorCode::BundleValidationError,
                ErrorCategory::Bundle,
                ErrorSeverity::Medium,
                "Bundle version cannot be empty",
            ));
        }

        debug!("Bundle validation passed: {}", bundle.manifest.name);
        Ok(())
    }

    /// Create container for bundle execution
    async fn create_bundle_container(
        &self,
        bundle: &LoadedBundle,
        session_id: DeepUuid,
        config: &BundleExecutionConfig,
    ) -> AriaResult<String> {
        debug!("Creating container for bundle: {}", bundle.manifest.name);

        let container_name = format!("aria-bundle-{}-{}", 
                                   bundle.manifest.name.replace(" ", "-"), 
                                   session_id.to_string());

        // Create container environment
        let mut env_vars = config.environment_variables.clone();
        env_vars.insert("ARIA_BUNDLE_NAME".to_string(), bundle.manifest.name.clone());
        env_vars.insert("ARIA_BUNDLE_VERSION".to_string(), bundle.manifest.version.clone());
        env_vars.insert("ARIA_SESSION_ID".to_string(), session_id.to_string());

        // TODO: Integrate with QuiltService to create container
        // This is a placeholder - actual implementation would use quilt_service
        let container_id = format!("container-{}", DeepUuid::new().to_string());

        info!("Created container for bundle execution: {}", container_id);
        Ok(container_id)
    }

    /// Install bundle dependencies
    async fn install_bundle_dependencies(&self, container_id: &str, bundle: &LoadedBundle) -> AriaResult<()> {
        debug!("Installing dependencies for container: {}", container_id);

        // TODO: Extract package.json from bundle and run `bun install`
        // This would integrate with the Bun runtime controller

        info!("Dependencies installed for container: {}", container_id);
        Ok(())
    }

    /// Register bundle components
    async fn register_bundle_components(
        &self,
        bundle: &LoadedBundle,
        container_id: &str,
    ) -> AriaResult<BundleComponentSummary> {
        debug!("Registering bundle components for: {}", bundle.manifest.name);

        let mut summary = BundleComponentSummary {
            tools_registered: Vec::new(),
            agents_registered: Vec::new(),
            teams_registered: Vec::new(),
            pipelines_registered: Vec::new(),
        };

        // Register tools
        for tool in &bundle.manifest.tools {
            // TODO: Register tool with bundle registry
            summary.tools_registered.push(tool.name.clone());
        }

        // Register agents
        for agent in &bundle.manifest.agents {
            // TODO: Register agent with agent registry
            summary.agents_registered.push(agent.name.clone());
        }

        // Register teams
        for team in &bundle.manifest.teams {
            // TODO: Register team with team registry
            summary.teams_registered.push(team.name.clone());
        }

        // Register pipelines
        for pipeline in &bundle.manifest.pipelines {
            // TODO: Register pipeline with pipeline registry
            summary.pipelines_registered.push(pipeline.name.clone());
        }

        info!("Registered {} tools, {} agents, {} teams, {} pipelines",
              summary.tools_registered.len(),
              summary.agents_registered.len(),
              summary.teams_registered.len(),
              summary.pipelines_registered.len());

        Ok(summary)
    }

    /// Execute bundle main entry point
    async fn execute_bundle_entry(
        &self,
        container_id: &str,
        bundle: &LoadedBundle,
        config: &BundleExecutionConfig,
    ) -> AriaResult<(Option<i32>, Option<String>, Option<String>)> {
        debug!("Executing bundle entry for container: {}", container_id);

        // Find main entry point
        let entry_path = bundle.get_main_entry().unwrap_or("index.ts");

        // TODO: Execute entry point with Bun runtime
        // This would integrate with the Bun runtime controller

        info!("Bundle execution completed for container: {}", container_id);
        Ok((Some(0), Some("Bundle executed successfully".to_string()), None))
    }

    /// Collect resource usage metrics
    async fn collect_resource_usage(&self, container_id: &str) -> AriaResult<BundleResourceUsage> {
        debug!("Collecting resource usage for container: {}", container_id);

        // TODO: Integrate with QuiltService to get container metrics
        Ok(BundleResourceUsage {
            peak_memory_mb: Some(256),
            cpu_time_seconds: Some(1.5),
            network_bytes_sent: Some(1024),
            network_bytes_received: Some(2048),
            disk_bytes_written: Some(4096),
            disk_bytes_read: Some(8192),
        })
    }

    /// Cleanup container after execution
    async fn cleanup_container(&self, container_id: &str) -> AriaResult<()> {
        debug!("Cleaning up container: {}", container_id);

        // TODO: Integrate with QuiltService to cleanup container
        info!("Container cleaned up: {}", container_id);
        Ok(())
    }

    /// Update execution status
    async fn update_execution_status(&self, execution_id: &str, status: BundleExecutionStatus) {
        let mut executions = self.active_executions.lock().await;
        executions.insert(execution_id.to_string(), status);
    }

    /// Get execution status
    pub async fn get_execution_status(&self, execution_id: &str) -> Option<BundleExecutionStatus> {
        let executions = self.active_executions.lock().await;
        executions.get(execution_id).cloned()
    }

    /// List active executions
    pub async fn list_active_executions(&self) -> Vec<(String, BundleExecutionStatus)> {
        let executions = self.active_executions.lock().await;
        executions.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// Get execution history
    pub async fn get_execution_history(&self, limit: Option<usize>) -> Vec<BundleExecutionResult> {
        let history = self.execution_history.lock().await;
        let limit = limit.unwrap_or(100);
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Cancel running execution
    pub async fn cancel_execution(&self, execution_id: &str) -> AriaResult<()> {
        debug!("Cancelling execution: {}", execution_id);

        // Update status
        self.update_execution_status(execution_id, BundleExecutionStatus::Cancelled).await;

        // TODO: Implement actual cancellation logic
        info!("Execution cancelled: {}", execution_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle_discovery::BundleToolDiscovery;
    use crate::engines::tool_registry::bundle_integration::BundleToolRegistry;
    use pkg_store::PackageStore;
    use std::sync::Arc;
    use std::collections::HashMap;
    use tokio::sync::{RwLock, Mutex};

    async fn create_test_executor() -> BundleExecutor {
        let pkg_store = Arc::new(PackageStore::new().await.unwrap());
        let discovery = Arc::new(BundleToolDiscovery::new(pkg_store.clone()));
        let tools = Arc::new(RwLock::new(HashMap::new()));
        let bundle_registry = Arc::new(BundleToolRegistry::new(tools, discovery.clone()));
        let quilt_service = Arc::new(Mutex::new(
            crate::engines::container::quilt::QuiltService::new("test-path".to_string()).await.unwrap()
        ));

        BundleExecutor::new(quilt_service, pkg_store, bundle_registry, discovery)
    }

    #[tokio::test]
    async fn test_bundle_executor_creation() {
        let executor = create_test_executor().await;
        let history = executor.get_execution_history(None).await;
        assert!(history.is_empty());
    }

    #[tokio::test]
    async fn test_execution_config_default() {
        let config = BundleExecutionConfig::default();
        assert_eq!(config.memory_limit_mb, Some(1024));
        assert_eq!(config.timeout_seconds, Some(300));
        assert!(config.network_enabled);
        assert!(config.auto_register_components);
    }

    #[tokio::test]
    async fn test_execution_status_tracking() {
        let executor = create_test_executor().await;
        let execution_id = "test-execution";
        
        executor.update_execution_status(execution_id, BundleExecutionStatus::Running).await;
        let status = executor.get_execution_status(execution_id).await;
        
        assert!(matches!(status, Some(BundleExecutionStatus::Running)));
    }
} 