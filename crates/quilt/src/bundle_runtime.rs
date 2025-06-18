/*!
# Bundle Runtime Support for Quilt

Container workspace management for .aria bundles with filesystem mounting,
environment configuration, and bundle extraction capabilities.
*/

use pkg_store::bundle::LoadedBundle;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::process::Command;
use tracing::{debug, error, info, warn};

/// Bundle workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleWorkspaceConfig {
    pub base_path: PathBuf,
    pub container_id: String,
    pub bundle_hash: String,
    pub session_id: String,
    pub isolation_level: IsolationLevel,
    pub resource_limits: ResourceLimits,
    pub network_config: NetworkConfig,
}

/// Container isolation level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IsolationLevel {
    None,
    Process,
    Container,
    VM,
}

/// Resource limits for bundle execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub memory_mb: Option<u64>,
    pub cpu_cores: Option<f64>,
    pub disk_mb: Option<u64>,
    pub network_bandwidth_mbps: Option<u64>,
    pub execution_timeout_seconds: Option<u64>,
}

/// Network configuration for bundle containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub enabled: bool,
    pub allow_external: bool,
    pub allowed_hosts: Vec<String>,
    pub blocked_ports: Vec<u16>,
}

/// Bundle workspace manager
pub struct BundleWorkspaceManager {
    base_workspace_path: PathBuf,
    active_workspaces: HashMap<String, BundleWorkspaceConfig>,
}

impl BundleWorkspaceManager {
    /// Create a new bundle workspace manager
    pub fn new(base_workspace_path: PathBuf) -> Self {
        Self {
            base_workspace_path,
            active_workspaces: HashMap::new(),
        }
    }

    /// Mount bundle workspace for container execution
    pub async fn mount_bundle_workspace(
        &mut self,
        container_id: &str,
        bundle: &LoadedBundle,
        config: BundleWorkspaceConfig,
    ) -> Result<PathBuf, BundleRuntimeError> {
        info!("Mounting bundle workspace for container: {}", container_id);

        // Create container workspace directory
        let workspace_path = self.base_workspace_path
            .join("containers")
            .join(container_id);

        self.create_workspace_directory(&workspace_path).await?;

        // Extract bundle to workspace
        self.extract_bundle_to_workspace(bundle, &workspace_path).await?;

        // Create runtime environment structure
        self.create_runtime_structure(&workspace_path).await?;

        // Configure container environment
        self.configure_container_environment(&workspace_path, &config).await?;

        // Track active workspace
        self.active_workspaces.insert(container_id.to_string(), config);

        info!("Bundle workspace mounted successfully: {}", workspace_path.display());
        Ok(workspace_path)
    }

    /// Extract bundle contents to workspace
    async fn extract_bundle_to_workspace(
        &self,
        bundle: &LoadedBundle,
        workspace_path: &Path,
    ) -> Result<(), BundleRuntimeError> {
        debug!("Extracting bundle to workspace: {}", workspace_path.display());

        // Create bundle content directories
        let implementations_path = workspace_path.join("implementations");
        let sources_path = workspace_path.join("_sources");
        
        fs::create_dir_all(&implementations_path).await?;
        fs::create_dir_all(&sources_path).await?;

        // Write manifest.json
        let manifest_json = serde_json::to_string_pretty(&bundle.manifest)?;
        fs::write(workspace_path.join("manifest.json"), manifest_json).await?;

        // Write source files
        for (path, content) in &bundle.source_files {
            let full_path = sources_path.join(path);
            
            // Create parent directories if needed
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).await?;
            }
            
            fs::write(full_path, content).await?;
        }

        // Create implementation stubs based on bundle structure
        self.create_implementation_stubs(bundle, &implementations_path, &sources_path).await?;

        // Generate package.json
        let package_json = bundle.generate_package_json();
        fs::write(workspace_path.join("package.json"), package_json).await?;

        debug!("Bundle extraction completed");
        Ok(())
    }

    /// Create implementation stubs that reference actual source files
    async fn create_implementation_stubs(
        &self,
        bundle: &LoadedBundle,
        implementations_path: &Path,
        sources_path: &Path,
    ) -> Result<(), BundleRuntimeError> {
        debug!("Creating implementation stubs");

        // Create tools directory and stubs
        let tools_path = implementations_path.join("tools");
        fs::create_dir_all(&tools_path).await?;

        for tool in &bundle.manifest.tools {
            let tool_file = tools_path.join(format!("{}.js", tool.name));
            let source_ref = format!("export * from '../../_sources/{}.ts';", tool.name);
            fs::write(tool_file, source_ref).await?;
        }

        // Create agents directory and stubs
        let agents_path = implementations_path.join("agents");
        fs::create_dir_all(&agents_path).await?;

        for agent in &bundle.manifest.agents {
            let agent_file = agents_path.join(format!("{}.js", agent.name));
            let source_ref = format!("export * from '../../_sources/{}.ts';", agent.name);
            fs::write(agent_file, source_ref).await?;
        }

        // Create teams directory and stubs
        let teams_path = implementations_path.join("teams");
        fs::create_dir_all(&teams_path).await?;

        for team in &bundle.manifest.teams {
            let team_file = teams_path.join(format!("{}.js", team.name));
            let source_ref = format!("export * from '../../_sources/{}.ts';", team.name);
            fs::write(team_file, source_ref).await?;
        }

        // Create pipelines directory and stubs
        let pipelines_path = implementations_path.join("pipelines");
        fs::create_dir_all(&pipelines_path).await?;

        for pipeline in &bundle.manifest.pipelines {
            let pipeline_file = pipelines_path.join(format!("{}.js", pipeline.name));
            let source_ref = format!("export * from '../../_sources/{}.ts';", pipeline.name);
            fs::write(pipeline_file, source_ref).await?;
        }

        debug!("Implementation stubs created");
        Ok(())
    }



    /// Create runtime structure (logs, temp, etc.)
    async fn create_runtime_structure(&self, workspace_path: &Path) -> Result<(), BundleRuntimeError> {
        debug!("Creating runtime structure");

        // Create runtime directories
        let runtime_dirs = ["logs", "temp", "cache", "data"];
        for dir in &runtime_dirs {
            fs::create_dir_all(workspace_path.join(dir)).await?;
        }

        // Create runtime metadata
        let runtime_metadata = serde_json::json!({
            "created_at": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            "workspace_version": "1.0.0",
            "runtime_type": "bundle"
        });

        fs::write(
            workspace_path.join("runtime.json"),
            serde_json::to_string_pretty(&runtime_metadata)?,
        ).await?;

        debug!("Runtime structure created");
        Ok(())
    }

    /// Configure container environment
    async fn configure_container_environment(
        &self,
        workspace_path: &Path,
        config: &BundleWorkspaceConfig,
    ) -> Result<(), BundleRuntimeError> {
        debug!("Configuring container environment");

        // Create environment file
        let env_config = self.create_bundle_environment(config)?;
        let env_content = env_config
            .iter()
            .map(|(key, value)| format!("{}={}", key, value))
            .collect::<Vec<_>>()
            .join("\n");

        fs::write(workspace_path.join(".env"), env_content).await?;

        // Create container configuration
        let container_config = serde_json::json!({
            "container_id": config.container_id,
            "bundle_hash": config.bundle_hash,
            "session_id": config.session_id,
            "isolation_level": config.isolation_level,
            "resource_limits": config.resource_limits,
            "network_config": config.network_config,
            "workspace_path": workspace_path.display().to_string()
        });

        fs::write(
            workspace_path.join("container.json"),
            serde_json::to_string_pretty(&container_config)?,
        ).await?;

        debug!("Container environment configured");
        Ok(())
    }

    /// Create bundle environment variables
    fn create_bundle_environment(
        &self,
        config: &BundleWorkspaceConfig,
    ) -> Result<HashMap<String, String>, BundleRuntimeError> {
        let mut env_vars = HashMap::new();

        // Core Aria environment
        env_vars.insert("ARIA_CONTAINER_ID".to_string(), config.container_id.clone());
        env_vars.insert("ARIA_BUNDLE_HASH".to_string(), config.bundle_hash.clone());
        env_vars.insert("ARIA_SESSION_ID".to_string(), config.session_id.clone());
        env_vars.insert("ARIA_WORKSPACE_PATH".to_string(), config.base_path.display().to_string());

        // Runtime configuration
        env_vars.insert("NODE_ENV".to_string(), "production".to_string());
        env_vars.insert("BUN_ENV".to_string(), "production".to_string());

        // Resource limits as environment variables
        if let Some(memory_mb) = config.resource_limits.memory_mb {
            env_vars.insert("ARIA_MEMORY_LIMIT_MB".to_string(), memory_mb.to_string());
        }

        if let Some(cpu_cores) = config.resource_limits.cpu_cores {
            env_vars.insert("ARIA_CPU_LIMIT_CORES".to_string(), cpu_cores.to_string());
        }

        if let Some(timeout) = config.resource_limits.execution_timeout_seconds {
            env_vars.insert("ARIA_EXECUTION_TIMEOUT_SECONDS".to_string(), timeout.to_string());
        }

        // Network configuration
        env_vars.insert("ARIA_NETWORK_ENABLED".to_string(), config.network_config.enabled.to_string());
        env_vars.insert("ARIA_ALLOW_EXTERNAL".to_string(), config.network_config.allow_external.to_string());

        Ok(env_vars)
    }

    /// Create workspace directory with proper permissions
    async fn create_workspace_directory(&self, path: &Path) -> Result<(), BundleRuntimeError> {
        debug!("Creating workspace directory: {}", path.display());

        // Remove existing directory if it exists
        if path.exists() {
            fs::remove_dir_all(path).await?;
        }

        // Create directory with proper permissions
        fs::create_dir_all(path).await?;

        // Set permissions (755 - rwxr-xr-x)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path).await?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(path, perms).await?;
        }

        debug!("Workspace directory created successfully");
        Ok(())
    }

    /// Cleanup workspace for container
    pub async fn cleanup_workspace(&mut self, container_id: &str) -> Result<(), BundleRuntimeError> {
        info!("Cleaning up workspace for container: {}", container_id);

        // Remove from active workspaces
        self.active_workspaces.remove(container_id);

        // Remove workspace directory
        let workspace_path = self.base_workspace_path
            .join("containers")
            .join(container_id);

        if workspace_path.exists() {
            fs::remove_dir_all(&workspace_path).await?;
            info!("Workspace cleaned up: {}", workspace_path.display());
        }

        Ok(())
    }

    /// Get workspace path for container
    pub fn get_workspace_path(&self, container_id: &str) -> Option<PathBuf> {
        self.active_workspaces.get(container_id).map(|config| {
            self.base_workspace_path
                .join("containers")
                .join(container_id)
        })
    }

    /// List active workspaces
    pub fn list_active_workspaces(&self) -> Vec<&str> {
        self.active_workspaces.keys().map(|s| s.as_str()).collect()
    }

    /// Get workspace configuration
    pub fn get_workspace_config(&self, container_id: &str) -> Option<&BundleWorkspaceConfig> {
        self.active_workspaces.get(container_id)
    }
}

/// Bundle runtime errors
#[derive(Debug, thiserror::Error)]
pub enum BundleRuntimeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Bundle extraction failed: {0}")]
    ExtractionFailed(String),
    #[error("Environment configuration failed: {0}")]
    EnvironmentFailed(String),
    #[error("Workspace creation failed: {0}")]
    WorkspaceCreationFailed(String),
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_mb: Some(1024), // 1GB
            cpu_cores: Some(1.0),
            disk_mb: Some(5120), // 5GB
            network_bandwidth_mbps: Some(100),
            execution_timeout_seconds: Some(300), // 5 minutes
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allow_external: false,
            allowed_hosts: vec!["localhost".to_string(), "127.0.0.1".to_string()],
            blocked_ports: vec![22, 23, 25, 53, 80, 443, 993, 995], // Common sensitive ports
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_workspace_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = BundleWorkspaceManager::new(temp_dir.path().to_path_buf());
        
        assert_eq!(manager.list_active_workspaces().len(), 0);
    }

    #[tokio::test]
    async fn test_workspace_directory_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = BundleWorkspaceManager::new(temp_dir.path().to_path_buf());
        
        let test_path = temp_dir.path().join("test-workspace");
        manager.create_workspace_directory(&test_path).await.unwrap();
        
        assert!(test_path.exists());
        assert!(test_path.is_dir());
    }

    #[test]
    fn test_default_configs() {
        let resource_limits = ResourceLimits::default();
        assert_eq!(resource_limits.memory_mb, Some(1024));
        
        let network_config = NetworkConfig::default();
        assert!(network_config.enabled);
        assert!(!network_config.allow_external);
    }

    #[tokio::test]
    async fn test_bundle_environment_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = BundleWorkspaceManager::new(temp_dir.path().to_path_buf());
        
        let config = BundleWorkspaceConfig {
            base_path: temp_dir.path().to_path_buf(),
            container_id: "test-container".to_string(),
            bundle_hash: "test-hash".to_string(),
            session_id: "test-session".to_string(),
            isolation_level: IsolationLevel::Container,
            resource_limits: ResourceLimits::default(),
            network_config: NetworkConfig::default(),
        };
        
        let env_vars = manager.create_bundle_environment(&config).unwrap();
        
        assert_eq!(env_vars.get("ARIA_CONTAINER_ID").unwrap(), "test-container");
        assert_eq!(env_vars.get("ARIA_BUNDLE_HASH").unwrap(), "test-hash");
        assert_eq!(env_vars.get("ARIA_SESSION_ID").unwrap(), "test-session");
    }
} 