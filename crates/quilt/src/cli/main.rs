use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::time::Duration;

// Import protobuf definitions directly
pub mod quilt {
    tonic::include_proto!("quilt");
}

// Import CLI modules
#[path = "../cli/mod.rs"]
mod cli;
use cli::IccCommands;

use quilt::quilt_service_client::QuiltServiceClient;
use quilt::{
    CreateContainerRequest, CreateContainerResponse, 
    GetContainerStatusRequest, GetContainerStatusResponse,
    GetContainerLogsRequest, GetContainerLogsResponse,
    StopContainerRequest, StopContainerResponse,
    RemoveContainerRequest, RemoveContainerResponse,
    ContainerStatus,
};

// Use validation utilities from utils module
#[path = "../utils/mod.rs"]
mod utils;
use utils::validation::InputValidator;
use utils::console::ConsoleLogger;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
    #[clap(short, long, value_parser, default_value = "http://127.0.0.1:50051")]
    server_addr: String,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create a new container with advanced features
    Create {
        #[clap(long, help = "Path to the container image tarball")]
        image_path: String,
        
        #[arg(short, long, action = clap::ArgAction::Append, 
              help = "Environment variables in KEY=VALUE format",
              num_args = 0.., value_parser = InputValidator::parse_key_val)]
        env: Vec<(String, String)>,
        
        #[clap(long, help = "Setup commands for dynamic runtime installation (e.g., 'npm: typescript', 'pip: requests')", 
               num_args = 0..)]
        setup: Vec<String>,
        
        #[clap(long, help = "Working directory inside the container")]
        working_directory: Option<String>,
        
        // Resource limits
        #[clap(long, help = "Memory limit in megabytes (0 = default)", default_value = "0")]
        memory_limit: i32,
        
        #[clap(long, help = "CPU limit as percentage (0.0 = default)", default_value = "0.0")]
        cpu_limit: f32,
        
        // Namespace configuration
        #[clap(long, help = "Enable PID namespace isolation")]
        enable_pid_namespace: bool,
        
        #[clap(long, help = "Enable mount namespace isolation")]
        enable_mount_namespace: bool,
        
        #[clap(long, help = "Enable UTS namespace isolation (hostname)")]
        enable_uts_namespace: bool,
        
        #[clap(long, help = "Enable IPC namespace isolation")]
        enable_ipc_namespace: bool,
        
        #[clap(long, help = "Enable network namespace isolation")]
        enable_network_namespace: bool,
        
        #[clap(long, help = "Enable all namespace isolation features")]
        enable_all_namespaces: bool,
        
        /// The command and its arguments to run in the container
        #[clap(required = true, num_args = 1.., 
               help = "Command and its arguments (use -- to separate from CLI options)")]
        command_and_args: Vec<String>,
    },
    
    /// Get the status of a container
    Status { 
        #[clap(help = "ID of the container to get status for")]
        container_id: String 
    },
    
    /// Get logs from a container
    Logs {
        #[clap(help = "ID of the container to get logs from")]
        container_id: String,
    },
    
    /// Stop a container
    Stop { 
        #[clap(help = "ID of the container to stop")]
        container_id: String 
    },
    
    /// Remove a container
    Remove { 
        #[clap(help = "ID of the container to remove")]
        container_id: String,
        
        #[clap(long, short, help = "Force removal even if running")]
        force: bool,
    },
    
    /// Create a production-ready persistent container
    #[clap(name = "create-production")]
    CreateProduction {
        #[clap(help = "Container image tar.gz file")]
        image_path: String,
        #[clap(long, help = "Container name/identifier")]
        name: Option<String>,
        #[clap(long, help = "Setup commands (copy:src:dest, run:command, etc.)")]
        setup: Vec<String>,
        #[clap(long, help = "Environment variables in KEY=VALUE format")]
        env: Vec<String>,
        #[clap(long, help = "Memory limit in MB", default_value = "512")]
        memory: u64,
        #[clap(long, help = "CPU limit percentage", default_value = "50.0")]
        cpu: f64,
        #[clap(long, help = "Disable networking")]
        no_network: bool,
    },

    /// Inter-Container Communication commands
    #[clap(subcommand)]
    Icc(IccCommands),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Create a channel with extended timeout configuration for concurrent operations
    let channel = tonic::transport::Channel::from_shared(cli.server_addr.clone())?
        .timeout(Duration::from_secs(300))  // 5 minutes for complex operations
        .connect_timeout(Duration::from_secs(30))  // Extended connection timeout
        .tcp_keepalive(Some(Duration::from_secs(300)))  // 5 minute TCP keep-alive
        .http2_keep_alive_interval(Duration::from_secs(120))  // 2 minute HTTP/2 keep-alive
        .keep_alive_timeout(Duration::from_secs(60))  // 1 minute keep-alive timeout
        .keep_alive_while_idle(true)
        .http2_adaptive_window(true)  // Enable adaptive window sizing for better performance
        .connect()
        .await
        .map_err(|e| {
            eprintln!("❌ Failed to connect to server at {}: {}", cli.server_addr, e);
            eprintln!("   Make sure quiltd is running: ./dev.sh server-bg");
            e
        })?;

    let mut client = QuiltServiceClient::new(channel);

    match cli.command {
        Commands::Create { 
            image_path, 
            env, 
            setup,
            working_directory,
            memory_limit,
            cpu_limit,
            enable_pid_namespace,
            enable_mount_namespace,
            enable_uts_namespace,
            enable_ipc_namespace,
            enable_network_namespace,
            enable_all_namespaces,
            command_and_args 
        } => {
            println!("🚀 Creating container...");
            
            if command_and_args.is_empty() {
                eprintln!("❌ Error: Command cannot be empty.");
                std::process::exit(1);
            }

            let environment: HashMap<String, String> = env.into_iter().collect();
            
            // If enable_all_namespaces is true, enable all namespace options
            let (pid_ns, mount_ns, uts_ns, ipc_ns, net_ns) = if enable_all_namespaces {
                (true, true, true, true, true)
            } else {
                (
                    enable_pid_namespace,
                    enable_mount_namespace, 
                    enable_uts_namespace,
                    enable_ipc_namespace,
                    enable_network_namespace
                )
            };

            let request = tonic::Request::new(CreateContainerRequest {
                image_path,
                command: command_and_args,
                environment,
                working_directory: working_directory.unwrap_or_default(),
                setup_commands: setup,
                memory_limit_mb: memory_limit,
                cpu_limit_percent: cpu_limit,
                enable_pid_namespace: pid_ns,
                enable_mount_namespace: mount_ns,
                enable_uts_namespace: uts_ns,
                enable_ipc_namespace: ipc_ns,
                enable_network_namespace: net_ns,
                auto_start: true,  // CLI should auto-start containers
            });

            match client.create_container(request).await {
                Ok(response) => {
                    let res: CreateContainerResponse = response.into_inner();
                    if res.success {
                        println!("✅ Container created successfully!");
                        println!("   Container ID: {}", res.container_id);
                    } else {
                        println!("❌ Failed to create container: {}", res.error_message);
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error creating container: {}", e.message());
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Status { container_id } => {
            println!("📊 Getting status for container {}...", container_id);
            let mut request = tonic::Request::new(GetContainerStatusRequest {
                container_id: container_id.clone(),
            });
            request.set_timeout(Duration::from_secs(60)); // ELITE: Extended timeout for network load
            
            match client.get_container_status(request).await {
                Ok(response) => {
                    let res: GetContainerStatusResponse = response.into_inner();
                    let status_enum = match res.status {
                        0 => ContainerStatus::Pending,
                        1 => ContainerStatus::Running,
                        2 => ContainerStatus::Exited,
                        _ => ContainerStatus::Failed,
                    };
                    let status_str = match status_enum {
                        ContainerStatus::Unspecified => "UNKNOWN",
                        ContainerStatus::Pending => "PENDING",
                        ContainerStatus::Running => "RUNNING",
                        ContainerStatus::Exited => "EXITED",
                        ContainerStatus::Failed => "FAILED",
                    };
                    
                    // Use ConsoleLogger for consistent formatting
                    let created_at_formatted = utils::process::ProcessUtils::format_timestamp(res.created_at);
                    ConsoleLogger::format_container_status(
                        &res.container_id,
                        status_str,
                        &created_at_formatted,
                        &res.rootfs_path,
                        if res.pid > 0 { Some(res.pid) } else { None },
                        if res.exit_code != 0 || status_enum == ContainerStatus::Exited { Some(res.exit_code) } else { None },
                        &res.error_message,
                        if res.memory_usage_bytes > 0 { Some(res.memory_usage_bytes) } else { None },
                        if !res.ip_address.is_empty() { Some(&res.ip_address) } else { None },
                    );
                }
                Err(e) => {
                    eprintln!("❌ Error getting container status: {}", e.message());
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Logs { container_id } => {
            println!("📜 Getting logs for container {}...", container_id);
            let request = tonic::Request::new(GetContainerLogsRequest { container_id: container_id.clone() });
            match client.get_container_logs(request).await {
                Ok(response) => {
                    let res: GetContainerLogsResponse = response.into_inner();
                    
                    if res.logs.is_empty() {
                        println!("📝 No logs available for container {}", container_id);
                    } else {
                        println!("📝 Logs for container {}:", container_id);
                        ConsoleLogger::separator();
                        
                        for log_entry in res.logs {
                            let timestamp = log_entry.timestamp;
                            let message = log_entry.message;
                            
                            // Convert timestamp to human readable format
                            let formatted_time = utils::process::ProcessUtils::format_timestamp(timestamp);
                            
                            println!("[{}] {}", formatted_time, message);
                        }
                        ConsoleLogger::separator();
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error getting container logs: {}", e.message());
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Stop { container_id } => {
            println!("🛑 Stopping container {}...", container_id);
            let request = tonic::Request::new(StopContainerRequest { 
                container_id: container_id.clone(), 
                timeout_seconds: 10 
            });
            match client.stop_container(request).await {
                Ok(response) => {
                    let res: StopContainerResponse = response.into_inner();
                    if res.success {
                        println!("✅ Container {} stopped successfully", container_id);
                    } else {
                        println!("❌ Failed to stop container: {}", res.error_message);
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error stopping container: {}", e.message());
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Remove { container_id, force } => {
            println!("🗑️  Removing container {}...", container_id);
            let request = tonic::Request::new(RemoveContainerRequest { 
                container_id: container_id.clone(), 
                force 
            });
            match client.remove_container(request).await {
                Ok(response) => {
                    let res: RemoveContainerResponse = response.into_inner();
                    if res.success {
                        println!("✅ Container {} removed successfully", container_id);
                    } else {
                        println!("❌ Failed to remove container: {}", res.error_message);
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error removing container: {}", e.message());
                    std::process::exit(1);
                }
            }
        }
        
        Commands::CreateProduction { image_path, name, setup, env, memory, cpu, no_network } => {
            println!("🚀 Creating production container using the new event-driven readiness system...");
            
            // Parse environment variables
            let mut environment = std::collections::HashMap::new();
            for env_var in env {
                if let Some((key, value)) = env_var.split_once('=') {
                    environment.insert(key.to_string(), value.to_string());
                }
            }
            
            // Create production container using enhanced daemon runtime with event-driven readiness
            let create_request = CreateContainerRequest {
                image_path,
                command: vec!["sleep".to_string(), "3600".to_string()], // 1 hour instead of infinity
                environment,
                working_directory: String::new(), // Empty string instead of None
                setup_commands: setup,
                memory_limit_mb: if memory > 0 { memory as i32 } else { 512 },
                cpu_limit_percent: if cpu > 0.0 { cpu as f32 } else { 50.0 },
                enable_network_namespace: !no_network,
                enable_pid_namespace: true,
                enable_mount_namespace: true,
                enable_uts_namespace: true,
                enable_ipc_namespace: true,
                auto_start: true,  // Production containers should auto-start
            };

            match client.create_container(tonic::Request::new(create_request)).await {
                Ok(response) => {
                    let res = response.into_inner();
                    if res.success {
                        println!("✅ Production container created and ready with ID: {}", res.container_id);
                        println!("   Memory: {}MB", memory);
                        println!("   CPU: {}%", cpu);
                        println!("   Networking: {}", if !no_network { "enabled" } else { "disabled" });
                        println!("   Event-driven readiness: enabled");
                        println!("   Container automatically started with PID verification");
                        
                        if let Some(container_name) = name {
                            println!("   Custom name: {}", container_name);
                        }
                    } else {
                        eprintln!("❌ Failed to create production container: {}", res.error_message);
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error creating production container: {}", e.message());
                    std::process::exit(1);
                }
            }
        }

        Commands::Icc(icc_cmd) => {
            cli::icc::handle_icc_command(icc_cmd, client).await?
        }
    }

    Ok(())
} 