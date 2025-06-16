use crate::daemon::namespace::{NamespaceManager, NamespaceConfig};
use crate::daemon::cgroup::{CgroupManager, CgroupLimits};
use crate::daemon::manager::RuntimeManager;
use crate::daemon::readiness::{ContainerReadinessManager, ReadinessConfig, cleanup_readiness_signal};
use crate::utils::{ConsoleLogger, FileSystemUtils, CommandExecutor, ProcessUtils, ImageManager, ConcurrentContainerRegistry};
use crate::icc::network::{ContainerNetworkConfig, NetworkManager};
use std::collections::HashMap;
use std::sync::Arc;
use std::process::Command;
use std::fs;
use std::path::Path;
use flate2::read::GzDecoder;
use tar::Archive;
use nix::unistd::{chroot, chdir, Pid, execv};
use std::os::unix::fs::PermissionsExt;
use std::ffi::CString;
use crate::daemon::resource::ResourceManager;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub enum ContainerState {
    PENDING,
    RUNNING,
    EXITED(i32),
    FAILED(String),
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: u64,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ContainerConfig {
    pub image_path: String,
    pub command: Vec<String>,
    pub environment: HashMap<String, String>,
    pub setup_commands: Vec<String>,  // Setup commands specification
    pub resource_limits: Option<CgroupLimits>,
    pub namespace_config: Option<NamespaceConfig>,
    #[allow(dead_code)]
    pub working_directory: Option<String>,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        ContainerConfig {
            image_path: String::new(),
            command: vec!["/bin/sh".to_string()],
            environment: HashMap::new(),
            setup_commands: vec![],
            resource_limits: Some(CgroupLimits::default()),
            namespace_config: Some(NamespaceConfig::default()),
            working_directory: None,
        }
    }
}

#[derive(Debug)]
pub struct Container {
    #[allow(dead_code)]
    pub id: String,
    pub config: ContainerConfig,
    pub state: ContainerState,
    pub logs: Vec<LogEntry>,
    pub pid: Option<Pid>,
    pub rootfs_path: String,
    pub created_at: u64,
    pub network_config: Option<ContainerNetworkConfig>,
    // Task management to prevent leaks
    pub monitoring_task: Option<tokio::task::JoinHandle<()>>,
}

impl Clone for Container {
    fn clone(&self) -> Self {
        Container {
            id: self.id.clone(),
            config: self.config.clone(),
            state: self.state.clone(),
            logs: self.logs.clone(),
            pid: self.pid,
            rootfs_path: self.rootfs_path.clone(),
            created_at: self.created_at,
            network_config: self.network_config.clone(),
            // JoinHandle cannot be cloned, so we set it to None
            monitoring_task: None,
        }
    }
}

impl Container {
    pub fn new(id: String, config: ContainerConfig) -> Self {
        let timestamp = ProcessUtils::get_timestamp();

        Container {
            id: id.clone(),
            config,
            state: ContainerState::PENDING,
            logs: Vec::new(),
            pid: None,
            rootfs_path: format!("/tmp/quilt-containers/{}", id),
            created_at: timestamp,
            network_config: None,
            monitoring_task: None,
        }
    }

    pub fn add_log(&mut self, message: String) {
        let timestamp = ProcessUtils::get_timestamp();

        self.logs.push(LogEntry {
            timestamp,
            message,
        });
    }
}

pub struct ContainerRuntime {
    containers: Arc<ConcurrentContainerRegistry<Container>>,
    namespace_manager: NamespaceManager,
    runtime_manager: RuntimeManager,
    resource_manager: ResourceManager,
    readiness_manager: ContainerReadinessManager,
}

impl ContainerRuntime {
    pub fn new() -> Self {
        ContainerRuntime {
            containers: Arc::new(ConcurrentContainerRegistry::new()),
            namespace_manager: NamespaceManager::new(),
            runtime_manager: RuntimeManager::new(),
            resource_manager: ResourceManager::new(),
            readiness_manager: ContainerReadinessManager::new(ReadinessConfig::default()),
        }
    }

    pub fn create_container(&self, id: String, config: ContainerConfig) -> Result<(), String> {
        ConsoleLogger::progress(&format!("Creating container: {}", id));
        
        let container = Container::new(id.clone(), config);

        // Lock-free container insertion
        self.containers.insert(id.clone(), container);

        // Setup rootfs
        if let Err(e) = self.setup_rootfs(&id) {
            ConsoleLogger::error(&format!("[CREATE] Rootfs setup failed for {}: {}", id, e));
            // Rollback: remove container from map
            self.containers.remove(&id);
            return Err(e);
        }

        self.update_container_state(&id, ContainerState::PENDING);

        ConsoleLogger::container_created(&id);
        Ok(())
    }

    pub fn start_container(&self, id: &str, network_config: Option<ContainerNetworkConfig>) -> Result<(), String> {
        ConsoleLogger::progress(&format!("[START] Starting container: {}", id));

        // Get container configuration (lock-free read)
        let (config, rootfs_path) = self.containers.with_container(id, |container| {
            (container.config.clone(), container.rootfs_path.clone())
        }).ok_or_else(|| format!("Container {} not found", id))?;

        // Register mounts with ResourceManager
        let mount_points = vec![
            format!("{}/proc", rootfs_path),
            format!("{}/sys", rootfs_path),
            format!("{}/dev/pts", rootfs_path),
            rootfs_path.clone(),
        ];
        let resource_manager = ResourceManager::global();
        resource_manager.register_mounts(id, mount_points);

        // Register network config with ResourceManager if available
        if let Some(ref net_config) = network_config {
            resource_manager.register_network(id, net_config.clone());
        }

        // Create cgroups
        let mut cgroup_manager = CgroupManager::new(id.to_string());
        if let Some(limits) = &config.resource_limits {
            if let Err(e) = cgroup_manager.create_cgroups(limits) {
                ConsoleLogger::warning(&format!("Failed to create cgroups: {}", e));
            }
        }

        // Parse and execute setup commands
        let setup_commands = if !config.setup_commands.is_empty() {
            let setup_spec = config.setup_commands.join("\n");
            self.runtime_manager.parse_setup_spec(&setup_spec)?
        } else {
            vec![]
        };

        // Create namespaced process for container execution
        let namespace_config = config.namespace_config.unwrap_or_default();
        
        // Reduce memory footprint - prepare everything needed outside the closure
        let id_for_logs = id.to_string();
        let command_for_logs = format!("{:?}", config.command);
        
        // Add log entry (per-container lock)
        self.containers.update(id, |container| {
            container.add_log(format!("Starting container execution with command: {}", command_for_logs));
        });
        
        // Prepare all data needed by child process (avoid heavy captures)
        // ENHANCED: Inject readiness check into command
        let enhanced_command = self.readiness_manager.inject_readiness_into_command(id, config.command.clone());
        let command_clone = enhanced_command;
        let environment_clone = config.environment.clone();
        let rootfs_path_clone = rootfs_path.clone();
        let setup_commands_clone = setup_commands.clone();
        let network_enabled = namespace_config.network; // Capture network flag for child process

        // Create new lightweight runtime manager for child (not clone of existing)
        let child_func = move || -> i32 {
            // This runs in the child process with new namespaces
            // Keep memory allocation to minimum in child process
            
            // Setup mount namespace
            let namespace_manager = NamespaceManager::new();
            if let Err(e) = namespace_manager.setup_mount_namespace(&rootfs_path_clone) {
                eprintln!("Failed to setup mount namespace: {}", e);
                return 1;
            }

            // Setup basic network namespace ONLY if networking is enabled
            if network_enabled {
                if let Err(e) = namespace_manager.setup_network_namespace() {
                    eprintln!("Failed to setup network namespace: {}", e);
                    // Non-fatal, continue
                }
            } else {
                println!("Skipping network namespace setup (networking disabled for container)");
            }

            // Set container hostname
            if let Err(e) = namespace_manager.set_container_hostname(&id_for_logs) {
                eprintln!("Failed to set container hostname: {}", e);
                // Non-fatal, continue
            }

            // Change root to container filesystem
            if let Err(e) = chroot(rootfs_path_clone.as_str()) {
                eprintln!("Failed to chroot to {}: {}", rootfs_path_clone, e);
                return 1;
            }

            // Change to root directory inside container
            if let Err(e) = chdir("/") {
                eprintln!("Failed to chdir to /: {}", e);
                return 1;
            }

            // Initialize container system environment first
            let mut runtime_manager = RuntimeManager::new(); // Create fresh instance
            if let Err(e) = runtime_manager.initialize_container() {
                eprintln!("Failed to initialize container environment: {}", e);
                return 1;
            }

            // Execute setup commands inside the container
            if !setup_commands_clone.is_empty() {
                println!("Executing {} setup commands in container {}", setup_commands_clone.len(), id_for_logs);
                if let Err(e) = runtime_manager.execute_setup_commands(&setup_commands_clone) {
                    eprintln!("Setup commands failed: {}", e);
                    return 1;
                }
            }

            // Set environment variables
            for (key, value) in environment_clone {
                std::env::set_var(key, value);
            }

            // Execute the main command with reduced memory overhead
            println!("Executing main command in container: {:?}", command_clone);
            
            // Prepare the final command to execute - IMPROVED LOGIC
            let (final_program, final_args) = if command_clone.len() >= 3 
                && (command_clone[0].ends_with("/sh") || command_clone[0].ends_with("/bash"))
                && command_clone[1] == "-c" {
                // Command is already a shell command like ["/bin/sh", "-c", "actual command"]
                // Use it directly to avoid double-shell wrapping
                (command_clone[0].clone(), command_clone[1..].to_vec())
            } else if command_clone.len() == 2 && command_clone[0] == "sleep" {
                // Special handling for sleep commands to ensure they work properly
                let sleep_duration = &command_clone[1];
                // Validate sleep duration
                if sleep_duration.parse::<u64>().is_ok() || sleep_duration == "infinity" {
                    ("/bin/sh".to_string(), vec!["-c".to_string(), format!("exec /bin/sleep {}", sleep_duration)])
                } else {
                    ConsoleLogger::warning(&format!("Invalid sleep duration: {}, using default", sleep_duration));
                    ("/bin/sh".to_string(), vec!["-c".to_string(), "exec /bin/sleep 3600".to_string()])
                }
            } else if command_clone.len() == 1 {
                // Single command - execute it through shell with full path if it's a known command
                let cmd = &command_clone[0];
                let full_path_cmd = if cmd == "echo" || cmd == "cat" || cmd == "ls" || cmd == "sleep" {
                    format!("/bin/{}", cmd)
                } else if cmd.starts_with("/") {
                    cmd.clone() // Already full path
                } else {
                    cmd.clone() // Hope it's in PATH
                };
                ("/bin/sh".to_string(), vec!["-c".to_string(), format!("exec {}", full_path_cmd)])
            } else {
                // Multiple arguments - for known commands use full paths
                let mut full_cmd_parts = Vec::new();
                for (i, part) in command_clone.iter().enumerate() {
                    if i == 0 {
                        // First part is the command
                        if part == "echo" || part == "cat" || part == "ls" || part == "sleep" {
                            full_cmd_parts.push(format!("/bin/{}", part));
                        } else if part.starts_with("/") {
                            full_cmd_parts.push(part.clone()); // Already full path
                        } else {
                            full_cmd_parts.push(part.clone()); // Hope it's in PATH
                        }
                    } else {
                        full_cmd_parts.push(part.clone()); // Arguments as-is
                    }
                }
                ("/bin/sh".to_string(), vec!["-c".to_string(), format!("exec {}", full_cmd_parts.join(" "))])
            };

            // Convert to CString for exec (do this once, outside any fork)
            let program_cstring = match CString::new(final_program.clone()) {
                Ok(cs) => cs,
                Err(e) => {
                    eprintln!("Failed to create program CString: {}", e);
                    return 1;
                }
            };
                    
            // Prepare all arguments as CStrings with proper lifetime management
            let mut all_args = vec![final_program];
            all_args.extend(final_args);
            
            let args_cstrings: Vec<CString> = match all_args.iter()
                .map(|s| CString::new(s.clone()))
                .collect::<Result<Vec<CString>, _>>() {
                Ok(cstrings) => cstrings,
                Err(e) => {
                    eprintln!("Failed to prepare command arguments: {}", e);
                    return 1;
                            }
            };

            // Create references with proper lifetime (after cstrings is owned)
            let arg_refs: Vec<&CString> = args_cstrings.iter().collect();

            // Direct exec without nested fork - this replaces the current process
            println!("Executing: {} {:?}", program_cstring.to_string_lossy(), 
                     arg_refs.iter().map(|cs| cs.to_string_lossy()).collect::<Vec<_>>());
            
            // Log the actual command details for debugging
            let exec_start = std::time::SystemTime::now();
            println!("🕐 [EXEC] Command execution started at: {:?}", exec_start);
            println!("🕐 [EXEC] Full command: {} {}", program_cstring.to_string_lossy(), 
                     arg_refs[1..].iter().map(|cs| cs.to_string_lossy()).collect::<Vec<_>>().join(" "));
            
            // This will replace the current process entirely
            match execv(&program_cstring, &arg_refs) {
                Ok(_) => {
                    // This should never be reached if exec succeeds
                    0
                }
                Err(e) => {
                    eprintln!("Failed to exec command: {}", e);
                    1
                }
            }
        };

        // Create the namespaced process
        match self.namespace_manager.create_namespaced_process(&namespace_config, child_func) {
            Ok(pid) => {
                ConsoleLogger::debug(&format!("🚀 Container process created, PID: {} - verifying readiness...", ProcessUtils::pid_to_i32(pid)));
                
                // Add process to cgroups
                if let Err(e) = cgroup_manager.add_process(pid) {
                    ConsoleLogger::warning(&format!("Failed to add process to cgroups: {}", e));
                }

                // Finalize cgroup limits after process is started
                if let Some(limits) = &config.resource_limits {
                    if let Err(e) = cgroup_manager.finalize_limits(limits) {
                        ConsoleLogger::warning(&format!("Failed to finalize cgroup limits: {}", e));
                    }
                }

                // ✅ CRITICAL: Event-driven readiness verification - NO POLLING
                match self.readiness_manager.wait_for_container_ready(id, pid, &rootfs_path) {
                    Ok(()) => {
                        // Now container is truly ready
                        ConsoleLogger::container_started(id, Some(ProcessUtils::pid_to_i32(pid)));
                        
                        ConsoleLogger::debug(&format!("[START] Locking containers map to update state for {}", id));
                        // Update container state using lock-free concurrent operations
                        self.containers.update(id, |container| {
                            container.pid = Some(pid);
                            container.state = ContainerState::RUNNING;
                            container.add_log(format!("Container started with PID: {} and verified ready (event-driven)", pid));
                        });
                        ConsoleLogger::debug(&format!("[START] Unlocked containers map for {}", id));
                    }
                    Err(e) => {
                        ConsoleLogger::error(&format!("Container {} failed event-driven readiness check: {}", id, e));
                        // Kill the process since it's not working properly
                        let _ = ProcessUtils::terminate_process(pid, 2);
                        // Clean up readiness signal
                        cleanup_readiness_signal(id);
                        self.update_container_state(id, ContainerState::FAILED(e.clone()));
                        return Err(format!("Container {} failed to become ready (event-driven): {}", id, e));
                    }
                }

                // Wait for process completion in a separate task - MANAGED TO PREVENT LEAKS
                let id_clone = id.to_string();
                let start_time = std::time::SystemTime::now();
                let containers_ref = self.containers.clone(); // Clone the Arc for the task
                let resource_manager = ResourceManager::global();
                
                // ✅ CRITICAL FIX: Use a JoinHandle to manage the task lifecycle
                let wait_task = tokio::spawn(async move {
                    ConsoleLogger::debug(&format!("🕐 [TIMING] Started waiting for process {} at {:?}", ProcessUtils::pid_to_i32(pid), start_time));
                    
                    let exit_code = match NamespaceManager::new().wait_for_process(pid) {
                        Ok(exit_code) => {
                            let elapsed = start_time.elapsed().unwrap_or_default();
                            ConsoleLogger::success(&format!("Container {} exited with code: {} after {:?}", id_clone, exit_code, elapsed));
                            if elapsed.as_secs() < 10 {
                                ConsoleLogger::warning(&format!("⚠️ Container {} exited suspiciously quickly (in {:?})", id_clone, elapsed));
                            }
                            Some(exit_code)
                        }
                        Err(e) => {
                            let elapsed = start_time.elapsed().unwrap_or_default();
                            ConsoleLogger::container_failed(&id_clone, &e);
                            ConsoleLogger::warning(&format!("Process wait failed after {:?}", elapsed));
                            None
                        }
                    };

                    // Update container state to EXITED
                    containers_ref.update(&id_clone, |container| {
                        if let Some(code) = exit_code {
                            container.state = ContainerState::EXITED(code);
                        } else {
                            container.state = ContainerState::FAILED("Process monitoring failed".to_string());
                        }
                        container.pid = None;
                        container.add_log("Container process completed".to_string());
                    });

                    // Comprehensive resource cleanup using ResourceManager
                    if let Err(e) = resource_manager.cleanup_container_resources(&id_clone, Some(pid)) {
                        ConsoleLogger::warning(&format!("Resource cleanup failed for {}: {}", id_clone, e));
                    }
                    
                    ConsoleLogger::debug(&format!("✅ Container {} monitoring task completed", id_clone));
                });

                // Store the task handle in container metadata for later cleanup if needed
                // For now, we'll let it run to completion since it cleans up after itself
                
                // Update container state to store the monitoring task
                self.containers.update(id, |container| {
                    container.monitoring_task = Some(wait_task);
                });

                Ok(())
            }
            Err(e) => {
                self.update_container_state(id, ContainerState::FAILED(e.clone()));
                Err(format!("Failed to start container {}: {}", id, e))
            }
        }
    }

    fn setup_rootfs(&self, container_id: &str) -> Result<(), String> {
        // Lock-free read of container configuration
        let image_path = self.containers.with_container(container_id, |container| {
            container.config.image_path.clone()
        }).ok_or_else(|| format!("Container {} not found", container_id))?;

        // Use ImageManager for efficient copy-on-write setup
        if FileSystemUtils::is_file(&image_path) {
            let rootfs_path = ImageManager::setup_container_rootfs(container_id, &image_path)?;
            
            // Fix broken symlinks and ensure working binaries
            self.fix_container_binaries(&rootfs_path)?;
            
            ConsoleLogger::success(&format!("Rootfs setup completed for container {}", container_id));
            Ok(())
        } else {
            Err(format!("Image file not found: {}", image_path))
        }
    }

    /// Fix broken symlinks in Nix-generated containers and ensure working binaries
    fn fix_container_binaries(&self, rootfs_path: &str) -> Result<(), String> {
        ConsoleLogger::debug("Fixing container binaries and symlinks...");

        // All containers use busybox - verify essential symlinks
        let busybox_path = format!("{}/bin/busybox", rootfs_path);
        if !FileSystemUtils::is_file(&busybox_path) || !FileSystemUtils::is_executable(&busybox_path) {
            return Err(format!("Container missing or non-executable busybox at: {}", busybox_path));
        }

        ConsoleLogger::debug("Container has busybox - verifying essential symlinks");
        self.verify_busybox_symlinks(rootfs_path)?;

        // Ensure basic shell works
        self.verify_container_shell(rootfs_path)?;

        ConsoleLogger::success("Container binaries fixed and verified");
        Ok(())
    }

    /// Setup essential library directories
    fn setup_library_directories(&self, rootfs_path: &str) -> Result<(), String> {
        let lib_dirs = vec![
            format!("{}/lib", rootfs_path),
            format!("{}/lib64", rootfs_path),
            format!("{}/lib/x86_64-linux-gnu", rootfs_path),
        ];

        for dir in lib_dirs {
            if let Err(e) = FileSystemUtils::create_dir_all_with_logging(&dir, "library directory") {
                ConsoleLogger::warning(&format!("Failed to create library directory {}: {}", dir, e));
            }
        }

        Ok(())
    }

    /// Copy essential libraries needed by binaries
    fn copy_essential_libraries(&self, rootfs_path: &str) -> Result<(), String> {
        let essential_libs = vec![
            ("/lib/x86_64-linux-gnu/libc.so.6", "lib/x86_64-linux-gnu/libc.so.6"),
            ("/lib64/ld-linux-x86-64.so.2", "lib64/ld-linux-x86-64.so.2"),
            ("/lib/x86_64-linux-gnu/libtinfo.so.6", "lib/x86_64-linux-gnu/libtinfo.so.6"),
            ("/lib/x86_64-linux-gnu/libdl.so.2", "lib/x86_64-linux-gnu/libdl.so.2"),
        ];

        for (host_lib, container_lib) in essential_libs {
            if FileSystemUtils::is_file(host_lib) {
                let container_lib_path = format!("{}/{}", rootfs_path, container_lib);
                match FileSystemUtils::copy_file(host_lib, &container_lib_path) {
                    Ok(_) => {
                        ConsoleLogger::debug(&format!("Copied essential library: {}", container_lib));
                    }
                    Err(e) => {
                        ConsoleLogger::warning(&format!("Failed to copy library {}: {}", host_lib, e));
                        continue;
                    }
                }
            }
        }

        Ok(())
    }

    /// Verify that essential busybox symlinks exist and work properly
    fn verify_busybox_symlinks(&self, rootfs_path: &str) -> Result<(), String> {
        let essential_utils = vec!["sh", "echo", "ls", "cat", "sleep"];
        let busybox_path = format!("{}/bin/busybox", rootfs_path);

        for util in essential_utils {
            let util_path = format!("{}/bin/{}", rootfs_path, util);
            
            if !FileSystemUtils::is_file(&util_path) {
                ConsoleLogger::warning(&format!("Missing busybox symlink: {}", util));
                self.create_busybox_symlink(&util_path, &busybox_path)?;
            } else if FileSystemUtils::is_broken_symlink(&util_path) {
                ConsoleLogger::warning(&format!("Broken busybox symlink: {}", util));
                FileSystemUtils::remove_path(&util_path)?;
                self.create_busybox_symlink(&util_path, &busybox_path)?;
            } else {
                ConsoleLogger::debug(&format!("Busybox utility {} exists and is linked", util));
            }
        }

        ConsoleLogger::success("All essential busybox symlinks verified");
        Ok(())
    }

    /// Create a symlink to busybox for a utility
    fn create_busybox_symlink(&self, util_path: &str, busybox_path: &str) -> Result<(), String> {
        // Create symlink to busybox
        match std::os::unix::fs::symlink("busybox", util_path) {
            Ok(_) => {
                let util_name = std::path::Path::new(util_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                ConsoleLogger::success(&format!("Created busybox symlink: {}", util_name));
                Ok(())
            }
            Err(e) => Err(format!("Failed to create symlink {}: {}", util_path, e))
        }
    }






    /// Verify that the container shell works
    fn verify_container_shell(&self, rootfs_path: &str) -> Result<(), String> {
        let shell_path = format!("{}/bin/sh", rootfs_path);
        
        if !FileSystemUtils::is_file(&shell_path) {
            ConsoleLogger::warning("No shell found in container, basic commands may not work");
            return Ok(());
        }

        if !FileSystemUtils::is_executable(&shell_path) {
            ConsoleLogger::warning("Shell exists but is not executable");
            return Ok(());
        }

        ConsoleLogger::success("Container shell verification completed");
                    Ok(())
    }

    fn extract_image(&self, image_path: &str, rootfs_path: &str) -> Result<(), String> {
        // Open and decompress the tar file
        let tar_file = std::fs::File::open(image_path)
            .map_err(|e| format!("Failed to open image file: {}", e))?;

        let tar = GzDecoder::new(tar_file);
        let mut archive = Archive::new(tar);

        // Extract to rootfs directory
        archive.unpack(rootfs_path)
            .map_err(|e| format!("Failed to extract image: {}", e))?;

        ConsoleLogger::success(&format!("Successfully extracted image to {}", rootfs_path));
        Ok(())
    }

    fn update_container_state(&self, container_id: &str, new_state: ContainerState) {
        // Per-container lock for state update
        self.containers.update(container_id, |container| {
            container.state = new_state;
        });
    }

    #[allow(dead_code)]
    pub fn get_container_state(&self, container_id: &str) -> Option<ContainerState> {
        self.containers.with_container(container_id, |container| container.state.clone())
    }

    pub fn get_container_logs(&self, container_id: &str) -> Option<Vec<LogEntry>> {
        self.containers.with_container(container_id, |container| container.logs.clone())
    }

    pub fn get_container_info(&self, container_id: &str) -> Option<Container> {
        self.containers.get(container_id)
    }

    // Internal method for getting container stats
    fn get_container_stats_for_container(&self, container: &Container, container_id: &str) -> Result<HashMap<String, String>, String> {

        let mut stats = HashMap::new();
        
        if let Some(pid) = container.pid {
            // Get memory usage from cgroups
            let cgroup_manager = CgroupManager::new(container_id.to_string());
            if let Ok(memory_usage) = cgroup_manager.get_memory_usage() {
                stats.insert("memory_usage_bytes".to_string(), memory_usage.to_string());
            }
        }

        // Get container state
        match &container.state {
            ContainerState::PENDING => stats.insert("state".to_string(), "pending".to_string()),
            ContainerState::RUNNING => stats.insert("state".to_string(), "running".to_string()),
            ContainerState::EXITED(code) => stats.insert("state".to_string(), format!("exited({})", code)),
            ContainerState::FAILED(msg) => stats.insert("state".to_string(), format!("failed: {}", msg)),
        };

        // Get PID if available
        if let Some(pid) = container.pid {
            stats.insert("pid".to_string(), ProcessUtils::pid_to_i32(pid).to_string());
        }

        Ok(stats)
    }

    pub fn get_container_stats(&self, container_id: &str) -> Result<HashMap<String, String>, String> {
        self.containers.with_container(container_id, |container| {
            self.get_container_stats_for_container(container, container_id)
        }).unwrap_or_else(|| Err(format!("Container {} not found", container_id)))
    }

    pub fn get_container_info_and_stats(&self, container_id: &str) -> (Option<Container>, Result<HashMap<String, String>, String>) {
        let container_info = self.containers.get(container_id);
        let container_stats = self.get_container_stats(container_id);
        (container_info, container_stats)
    }

    pub fn stop_container(&self, container_id: &str) -> Result<(), String> {
        ConsoleLogger::progress(&format!("Stopping container: {}", container_id));

        // Get container PID and monitoring task
        let (pid, monitoring_task) = self.containers.with_container(container_id, |container| {
            (container.pid, container.monitoring_task.as_ref().map(|t| t.abort_handle()))
        }).ok_or_else(|| format!("Container {} not found", container_id))?;

        let pid = pid.ok_or_else(|| format!("Container {} is not running", container_id))?;

        // Abort the monitoring task to prevent resource leaks
        if let Some(abort_handle) = monitoring_task {
            abort_handle.abort();
            ConsoleLogger::debug(&format!("Aborted monitoring task for container {}", container_id));
        }

        match ProcessUtils::terminate_process(pid, 10) {
            Ok(()) => {
                // Update container state
                self.containers.update(container_id, |container| {
                    container.state = ContainerState::EXITED(0);
                    container.pid = None;
                    container.monitoring_task = None; // Clear the task handle
                    container.add_log("Container stopped by user request".to_string());
                });
                
                // Comprehensive resource cleanup using ResourceManager
                let resource_manager = ResourceManager::global();
                if let Err(e) = resource_manager.cleanup_container_resources(container_id, Some(pid)) {
                    ConsoleLogger::warning(&format!("Resource cleanup failed for {}: {}", container_id, e));
                }
                
                ConsoleLogger::container_stopped(container_id);
                Ok(())
            }
            Err(e) => {
                Err(format!("Failed to stop container {}: {}", container_id, e))
            }
        }
    }

    pub fn remove_container(&self, container_id: &str) -> Result<(), String> {
        ConsoleLogger::progress(&format!("Removing container: {}", container_id));

        // Get container PID before stopping if it's running
        let container_pid = self.containers.with_container(container_id, |container| container.pid)
            .flatten(); // This converts Option<Option<Pid>> to Option<Pid>

        // Stop the container first if it's running
        if let Err(e) = self.stop_container(container_id) {
            ConsoleLogger::warning(&format!("Error stopping container before removal: {}", e));
        }

        // Remove container from registry
        self.containers.remove(container_id)
            .ok_or_else(|| format!("Container {} not found", container_id))?;

        // Clean up readiness signal files
        cleanup_readiness_signal(container_id);

        // Use ResourceManager for comprehensive cleanup
        let resource_manager = ResourceManager::global();
        if let Err(e) = resource_manager.cleanup_container_resources(container_id, container_pid) {
            ConsoleLogger::warning(&format!("Resource cleanup failed during removal: {}", e));
            // Try emergency cleanup as fallback
            if let Err(e2) = resource_manager.emergency_cleanup(container_id) {
                return Err(format!("Failed to remove container {}: {} (emergency cleanup also failed: {})", container_id, e, e2));
            }
        }

        // Clean up image layers and overlay mounts
        if let Err(e) = ImageManager::cleanup_container_image(container_id) {
            ConsoleLogger::warning(&format!("Image cleanup failed: {}", e));
        }

        ConsoleLogger::container_removed(container_id);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn list_containers(&self) -> Vec<String> {
        self.containers.keys()
    }

    /// Set the network configuration for a container
    pub fn set_container_network(&self, container_id: &str, network_config: ContainerNetworkConfig) -> Result<(), String> {
        self.containers.update(container_id, |container| {
            container.network_config = Some(network_config);
        }).ok_or_else(|| format!("Container {} not found", container_id))
    }

    /// Get the network configuration for a container
    pub fn get_container_network(&self, container_id: &str) -> Option<ContainerNetworkConfig> {
        self.containers.with_container(container_id, |container| container.network_config.clone())?
    }

    /// Configure network for a running container
    pub fn setup_container_network_post_start(&self, container_id: &str, network_manager: &NetworkManager) -> Result<(), String> {
        let (network_config, pid) = self.containers.with_container(container_id, |container| {
            let network_config = container.network_config
                .as_ref()
                .ok_or_else(|| format!("No network config for container {}", container_id))?;
            
            let pid = container.pid
                .ok_or_else(|| format!("Container {} is not running", container_id))?;
            
            Result::<(ContainerNetworkConfig, nix::unistd::Pid), String>::Ok((network_config.clone(), pid))
        }).ok_or_else(|| format!("Container {} not found", container_id))??;

        // Setup the container's network interface using the network manager
        network_manager.setup_container_network(&network_config, pid.as_raw())?;
        Ok(())
    }

    /// Execute a command in a running container
    pub fn exec_container(
        &self,
        container_id: &str,
        command: Vec<String>,
        working_directory: Option<String>,
        environment: HashMap<String, String>,
        capture_output: bool,
    ) -> Result<(i32, String, String), String> {
        ConsoleLogger::progress(&format!("Executing command in container {}: {:?}", container_id, command));
        ConsoleLogger::debug(&format!("🔍 [EXEC] Working dir: {:?}, Env vars: {}, Capture output: {}", 
                                     working_directory, environment.len(), capture_output));

        let pid = self.containers.with_container(container_id, |container| {
            // Check if container is running
            match container.state {
                ContainerState::RUNNING => {
                    ConsoleLogger::debug(&format!("✅ [EXEC] Container {} is running", container_id));
                    container.pid.ok_or_else(|| format!("Container {} has no PID", container_id))
                },
                ref state => {
                    let state_msg = match state {
                        ContainerState::PENDING => "PENDING",
                        ContainerState::EXITED(code) => &format!("EXITED({})", code),
                        ContainerState::FAILED(msg) => &format!("FAILED({})", msg),
                        _ => "UNKNOWN",
                    };
                    ConsoleLogger::debug(&format!("❌ [EXEC] Container {} is not running, state: {}", container_id, state_msg));
                    Err(format!("Container {} is not running", container_id))
                }
            }
        }).ok_or_else(|| format!("Container {} not found", container_id))??;
        ConsoleLogger::debug(&format!("🔓 [EXEC] Released containers lock, got PID: {}", ProcessUtils::pid_to_i32(pid)));

        // Prepare the command to execute
        let cmd_str = if command.len() == 1 {
            command[0].clone()
        } else {
            command.join(" ")
        };
        ConsoleLogger::debug(&format!("📝 [EXEC] Prepared command string: '{}'", cmd_str));

        // Build nsenter command to enter container's namespaces
        let mut nsenter_cmd = vec![
            "nsenter".to_string(),
            "-t".to_string(), pid.as_raw().to_string(),
            "-p".to_string(), "-m".to_string(), "-n".to_string(), "-u".to_string(), "-i".to_string(),
        ];

        // Add working directory if specified
        if let Some(workdir) = working_directory {
            ConsoleLogger::debug(&format!("📁 [EXEC] Setting working directory: {}", workdir));
            nsenter_cmd.extend(vec!["--wd".to_string(), workdir]);
        }

        // Add environment variables
        for (key, value) in environment {
            ConsoleLogger::debug(&format!("🌍 [EXEC] Setting env var: {}={}", key, value));
            nsenter_cmd.extend(vec!["-E".to_string(), format!("{}={}", key, value)]);
        }

        // Add the actual command
        nsenter_cmd.extend(vec!["--".to_string(), "/bin/sh".to_string(), "-c".to_string(), cmd_str.clone()]);
        
        ConsoleLogger::debug(&format!("🚀 [EXEC] Full nsenter command: {:?}", nsenter_cmd));
        let exec_start = std::time::SystemTime::now();

        // Execute the command using nsenter
        let output = Command::new("nsenter")
            .args(&nsenter_cmd[1..]) // Skip the "nsenter" part since we're calling it directly
            .output()
            .map_err(|e| format!("Failed to execute nsenter: {}", e))?;

        let elapsed = exec_start.elapsed().unwrap_or_default();
        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        
        ConsoleLogger::debug(&format!("⏱️ [EXEC] Command completed in {:?}, exit code: {}", elapsed, exit_code));
        if !stdout.is_empty() {
            ConsoleLogger::debug(&format!("📤 [EXEC] stdout: {}", stdout.trim()));
        }
        if !stderr.is_empty() {
            ConsoleLogger::debug(&format!("📤 [EXEC] stderr: {}", stderr.trim()));
        }

        Ok((exit_code, stdout, stderr))
    }

    // OLD POLLING-BASED VERIFICATION REMOVED - REPLACED WITH EVENT-DRIVEN READINESS SYSTEM
}