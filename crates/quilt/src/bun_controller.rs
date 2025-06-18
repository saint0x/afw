/*!
# Bun Runtime Controller

Manages Bun runtime execution for .aria bundles including dependency installation,
environment configuration, and execution monitoring with resource management.
*/

use crate::bundle_runtime::{BundleWorkspaceConfig, ResourceLimits, NetworkConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Bun execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BunExecutionResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub execution_time: Duration,
    pub resource_usage: BunResourceUsage,
}

/// Bun resource usage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BunResourceUsage {
    pub peak_memory_mb: Option<u64>,
    pub cpu_time_seconds: Option<f64>,
    pub wall_time_seconds: f64,
}

/// Bun runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BunRuntimeConfig {
    pub bun_executable: PathBuf,
    pub workspace_path: PathBuf,
    pub environment_variables: HashMap<String, String>,
    pub resource_limits: ResourceLimits,
    pub network_config: NetworkConfig,
    pub timeout_seconds: Option<u64>,
}

/// Bun controller for managing bundle execution
pub struct BunController {
    config: BunRuntimeConfig,
    active_processes: HashMap<String, Child>,
}

impl BunController {
    /// Create a new Bun controller
    pub fn new(config: BunRuntimeConfig) -> Self {
        Self {
            config,
            active_processes: HashMap::new(),
        }
    }

    /// Install dependencies for a bundle
    pub async fn install_dependencies(
        &self,
        container_id: &str,
        workspace_path: &Path,
    ) -> Result<BunExecutionResult, BunControllerError> {
        info!("Installing dependencies for container: {}", container_id);

        let package_json_path = workspace_path.join("package.json");
        
        // Verify package.json exists
        if !package_json_path.exists() {
            return Err(BunControllerError::PackageJsonNotFound(
                package_json_path.display().to_string()
            ));
        }

        let start_time = Instant::now();

        // Run bun install
        let mut command = Command::new(&self.config.bun_executable);
        command
            .arg("install")
            .current_dir(workspace_path)
            .env_clear();

        // Set environment variables
        for (key, value) in &self.config.environment_variables {
            command.env(key, value);
        }

        // Configure stdio
        command.stdout(Stdio::piped()).stderr(Stdio::piped());

        debug!("Executing: bun install in {}", workspace_path.display());

        let child = command.spawn()
            .map_err(|e| BunControllerError::ProcessSpawnFailed(e.to_string()))?;

        // Execute with timeout
        let timeout_duration = Duration::from_secs(
            self.config.timeout_seconds.unwrap_or(300)
        );

        let result = timeout(timeout_duration, child.wait_with_output()).await
            .map_err(|_| BunControllerError::ExecutionTimeout)?
            .map_err(|e| BunControllerError::ProcessExecutionFailed(e.to_string()))?;

        let execution_time = start_time.elapsed();

        let stdout = String::from_utf8_lossy(&result.stdout).to_string();
        let stderr = String::from_utf8_lossy(&result.stderr).to_string();

        let execution_result = BunExecutionResult {
            success: result.status.success(),
            exit_code: result.status.code(),
            stdout,
            stderr,
            execution_time,
            resource_usage: BunResourceUsage {
                peak_memory_mb: None, // TODO: Implement resource tracking
                cpu_time_seconds: None,
                wall_time_seconds: execution_time.as_secs_f64(),
            },
        };

        if execution_result.success {
            info!("Dependencies installed successfully for container: {}", container_id);
        } else {
            warn!("Dependency installation failed for container: {}", container_id);
        }

        Ok(execution_result)
    }

    /// Execute main entry point of a bundle
    pub async fn execute_main_entry(
        &self,
        container_id: &str,
        workspace_path: &Path,
        entry_path: &str,
    ) -> Result<BunExecutionResult, BunControllerError> {
        info!("Executing main entry for container: {} (entry: {})", container_id, entry_path);

        let entry_file_path = workspace_path.join(entry_path);
        
        // Verify entry file exists
        if !entry_file_path.exists() {
            return Err(BunControllerError::EntryFileNotFound(
                entry_file_path.display().to_string()
            ));
        }

        let start_time = Instant::now();

        // Run bun run
        let mut command = Command::new(&self.config.bun_executable);
        command
            .arg("run")
            .arg(entry_path)
            .current_dir(workspace_path)
            .env_clear();

        // Configure environment
        self.configure_bun_environment(&mut command)?;

        // Configure stdio for streaming
        command.stdout(Stdio::piped()).stderr(Stdio::piped());

        debug!("Executing: bun run {} in {}", entry_path, workspace_path.display());

        let mut child = command.spawn()
            .map_err(|e| BunControllerError::ProcessSpawnFailed(e.to_string()))?;

        // Track the process
        let process_id = format!("{}-{}", container_id, entry_path);

        // Execute with timeout and resource monitoring
        let timeout_duration = Duration::from_secs(
            self.config.timeout_seconds.unwrap_or(300)
        );

        let result = timeout(timeout_duration, child.wait_with_output()).await
            .map_err(|_| BunControllerError::ExecutionTimeout)?
            .map_err(|e| BunControllerError::ProcessExecutionFailed(e.to_string()))?;

        let execution_time = start_time.elapsed();

        let stdout = String::from_utf8_lossy(&result.stdout).to_string();
        let stderr = String::from_utf8_lossy(&result.stderr).to_string();

        let execution_result = BunExecutionResult {
            success: result.status.success(),
            exit_code: result.status.code(),
            stdout,
            stderr,
            execution_time,
            resource_usage: BunResourceUsage {
                peak_memory_mb: None, // TODO: Implement resource tracking
                cpu_time_seconds: None,
                wall_time_seconds: execution_time.as_secs_f64(),
            },
        };

        if execution_result.success {
            info!("Bundle execution completed successfully for container: {}", container_id);
        } else {
            warn!("Bundle execution failed for container: {}", container_id);
        }

        Ok(execution_result)
    }

    /// Configure Bun environment variables
    fn configure_bun_environment(&self, command: &mut Command) -> Result<(), BunControllerError> {
        debug!("Configuring Bun environment");

        // Set base environment variables
        for (key, value) in &self.config.environment_variables {
            command.env(key, value);
        }

        // Set Bun-specific configuration
        command.env("BUN_ENV", "production");
        command.env("NODE_ENV", "production");

        // Configure resource limits
        if let Some(memory_mb) = self.config.resource_limits.memory_mb {
            command.env("BUN_MAX_MEMORY", (memory_mb * 1024 * 1024).to_string());
        }

        // Network configuration
        if !self.config.network_config.enabled {
            command.env("BUN_DISABLE_NETWORK", "true");
        }

        debug!("Bun environment configured");
        Ok(())
    }

    /// Execute a custom Bun command
    pub async fn execute_custom_command(
        &self,
        container_id: &str,
        workspace_path: &Path,
        args: &[String],
    ) -> Result<BunExecutionResult, BunControllerError> {
        info!("Executing custom Bun command for container: {} (args: {:?})", container_id, args);

        let start_time = Instant::now();

        let mut command = Command::new(&self.config.bun_executable);
        command
            .args(args)
            .current_dir(workspace_path)
            .env_clear();

        // Configure environment
        self.configure_bun_environment(&mut command)?;

        // Configure stdio
        command.stdout(Stdio::piped()).stderr(Stdio::piped());

        debug!("Executing: bun {:?} in {}", args, workspace_path.display());

        let child = command.spawn()
            .map_err(|e| BunControllerError::ProcessSpawnFailed(e.to_string()))?;

        // Execute with timeout
        let timeout_duration = Duration::from_secs(
            self.config.timeout_seconds.unwrap_or(300)
        );

        let result = timeout(timeout_duration, child.wait_with_output()).await
            .map_err(|_| BunControllerError::ExecutionTimeout)?
            .map_err(|e| BunControllerError::ProcessExecutionFailed(e.to_string()))?;

        let execution_time = start_time.elapsed();

        let stdout = String::from_utf8_lossy(&result.stdout).to_string();
        let stderr = String::from_utf8_lossy(&result.stderr).to_string();

        Ok(BunExecutionResult {
            success: result.status.success(),
            exit_code: result.status.code(),
            stdout,
            stderr,
            execution_time,
            resource_usage: BunResourceUsage {
                peak_memory_mb: None,
                cpu_time_seconds: None,
                wall_time_seconds: execution_time.as_secs_f64(),
            },
        })
    }

    /// Stream execution output for real-time monitoring
    pub async fn execute_with_streaming(
        &self,
        container_id: &str,
        workspace_path: &Path,
        entry_path: &str,
        output_callback: impl Fn(String) + Send + 'static,
    ) -> Result<BunExecutionResult, BunControllerError> {
        info!("Executing with streaming for container: {} (entry: {})", container_id, entry_path);

        let entry_file_path = workspace_path.join(entry_path);
        
        if !entry_file_path.exists() {
            return Err(BunControllerError::EntryFileNotFound(
                entry_file_path.display().to_string()
            ));
        }

        let start_time = Instant::now();

        let mut command = Command::new(&self.config.bun_executable);
        command
            .arg("run")
            .arg(entry_path)
            .current_dir(workspace_path)
            .env_clear()
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        self.configure_bun_environment(&mut command)?;

        let mut child = command.spawn()
            .map_err(|e| BunControllerError::ProcessSpawnFailed(e.to_string()))?;

        // Stream stdout
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            
            tokio::spawn(async move {
                while let Ok(Some(line)) = lines.next_line().await {
                    output_callback(line);
                }
            });
        }

        // Wait for completion with timeout
        let timeout_duration = Duration::from_secs(
            self.config.timeout_seconds.unwrap_or(300)
        );

        let result = timeout(timeout_duration, child.wait()).await
            .map_err(|_| BunControllerError::ExecutionTimeout)?
            .map_err(|e| BunControllerError::ProcessExecutionFailed(e.to_string()))?;

        let execution_time = start_time.elapsed();

        Ok(BunExecutionResult {
            success: result.success(),
            exit_code: result.code(),
            stdout: String::new(), // Streamed separately
            stderr: String::new(), // Could be streamed separately too
            execution_time,
            resource_usage: BunResourceUsage {
                peak_memory_mb: None,
                cpu_time_seconds: None,
                wall_time_seconds: execution_time.as_secs_f64(),
            },
        })
    }

    /// Check if Bun is available and working
    pub async fn health_check(&self) -> Result<BunHealthStatus, BunControllerError> {
        debug!("Performing Bun health check");

        let mut command = Command::new(&self.config.bun_executable);
        command.arg("--version").stdout(Stdio::piped()).stderr(Stdio::piped());

        let result = command.output().await
            .map_err(|e| BunControllerError::HealthCheckFailed(e.to_string()))?;

        let version = if result.status.success() {
            String::from_utf8_lossy(&result.stdout).trim().to_string()
        } else {
            return Ok(BunHealthStatus {
                available: false,
                version: None,
                error: Some(String::from_utf8_lossy(&result.stderr).to_string()),
            });
        };

        Ok(BunHealthStatus {
            available: true,
            version: Some(version),
            error: None,
        })
    }

    /// Kill a running process
    pub async fn kill_process(&mut self, process_id: &str) -> Result<(), BunControllerError> {
        info!("Killing process: {}", process_id);

        if let Some(mut child) = self.active_processes.remove(process_id) {
            child.kill().await
                .map_err(|e| BunControllerError::ProcessKillFailed(e.to_string()))?;
            info!("Process killed: {}", process_id);
        }

        Ok(())
    }

    /// List active processes
    pub fn list_active_processes(&self) -> Vec<String> {
        self.active_processes.keys().cloned().collect()
    }

    /// Update runtime configuration
    pub fn update_config(&mut self, config: BunRuntimeConfig) {
        info!("Updating Bun controller configuration");
        self.config = config;
    }

    /// Get current configuration
    pub fn get_config(&self) -> &BunRuntimeConfig {
        &self.config
    }
}

/// Bun health check status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BunHealthStatus {
    pub available: bool,
    pub version: Option<String>,
    pub error: Option<String>,
}

/// Bun controller errors
#[derive(Debug, thiserror::Error)]
pub enum BunControllerError {
    #[error("Package.json not found: {0}")]
    PackageJsonNotFound(String),
    #[error("Entry file not found: {0}")]
    EntryFileNotFound(String),
    #[error("Process spawn failed: {0}")]
    ProcessSpawnFailed(String),
    #[error("Process execution failed: {0}")]
    ProcessExecutionFailed(String),
    #[error("Execution timeout")]
    ExecutionTimeout,
    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),
    #[error("Process kill failed: {0}")]
    ProcessKillFailed(String),
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

impl Default for BunRuntimeConfig {
    fn default() -> Self {
        Self {
            bun_executable: PathBuf::from("bun"),
            workspace_path: PathBuf::from("/tmp"),
            environment_variables: HashMap::new(),
            resource_limits: ResourceLimits::default(),
            network_config: NetworkConfig::default(),
            timeout_seconds: Some(300),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config(workspace_path: PathBuf) -> BunRuntimeConfig {
        BunRuntimeConfig {
            bun_executable: PathBuf::from("echo"), // Use echo for testing
            workspace_path,
            environment_variables: HashMap::new(),
            resource_limits: ResourceLimits::default(),
            network_config: NetworkConfig::default(),
            timeout_seconds: Some(30),
        }
    }

    #[tokio::test]
    async fn test_bun_controller_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path().to_path_buf());
        let controller = BunController::new(config);
        
        assert_eq!(controller.list_active_processes().len(), 0);
    }

    #[test]
    fn test_bun_runtime_config_default() {
        let config = BunRuntimeConfig::default();
        assert_eq!(config.bun_executable, PathBuf::from("bun"));
        assert_eq!(config.timeout_seconds, Some(300));
    }

    #[tokio::test]
    async fn test_environment_configuration() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path().to_path_buf());
        let controller = BunController::new(config);
        
        let mut command = Command::new("echo");
        let result = controller.configure_bun_environment(&mut command);
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_health_status_serialization() {
        let status = BunHealthStatus {
            available: true,
            version: Some("1.0.0".to_string()),
            error: None,
        };
        
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("available"));
        assert!(json.contains("1.0.0"));
    }

    #[tokio::test]
    async fn test_execution_result_tracking() {
        let result = BunExecutionResult {
            success: true,
            exit_code: Some(0),
            stdout: "test output".to_string(),
            stderr: "".to_string(),
            execution_time: Duration::from_secs(1),
            resource_usage: BunResourceUsage {
                peak_memory_mb: Some(256),
                cpu_time_seconds: Some(0.5),
                wall_time_seconds: 1.0,
            },
        };
        
        assert!(result.success);
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.stdout, "test output");
    }
} 