use aria_runtime::engines::container::quilt::QuiltService;
use aria_runtime::engines::config::QuiltConfig;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Debug Container Test with Enhanced Process Introspection");
    println!("=========================================================");
    
    let image_path = "/root/aria-fw/crates/quilt/nixos-production.tar.gz";
    
    println!("📦 Image path: {}", image_path);
    println!("📦 Image exists: {}", std::path::Path::new(image_path).exists());
    
    println!("🚀 Starting quilt daemon...");
    
    // Start daemon in background
    let daemon_handle = tokio::spawn(async {
        std::process::Command::new("cargo")
            .args(&["run", "--release", "--bin", "quilt-daemon"])
            .env("RUST_LOG", "debug")
            .output()
            .expect("Failed to start daemon");
    });

    // Give daemon time to start
    sleep(Duration::from_secs(8)).await;
    
    println!("🔗 Testing QuiltService connection...");
    let config = QuiltConfig {
        socket_path: "/run/quilt/api.sock".to_string(),
    };
    let mut quilt = QuiltService::new(&config).await?;
    
    println!("✅ Connected to daemon");
    
    // Test 1: Simple sleep command with enhanced logging
    println!("📦 Test 1: Creating container with long-running sleep...");
    let container_id1 = quilt.create_container(
        image_path.to_string(),
        vec!["sleep".to_string(), "300".to_string()], // 5 minutes for testing
        HashMap::new(),
    ).await?;
    println!("✅ Container created: {}", container_id1);
    
    println!("🚀 Starting container...");
    quilt.start_container(container_id1.clone()).await?;
    println!("✅ Container start command sent");
    
    // Wait for container to be fully started
    println!("⏳ Waiting for container startup...");
    sleep(Duration::from_secs(5)).await;
    
    // Single status check 
    println!("🔍 Checking container status...");
    let status = quilt.get_container_status(container_id1.clone()).await?;
    println!("📊 Container status: {:?}", status);
    
    // Check host system for the process
    println!("🔍 Checking host system for container process...");
    let ps_output = std::process::Command::new("ps")
        .args(&["aux"])
        .output()
        .expect("Failed to run ps");
    
    let ps_string = String::from_utf8_lossy(&ps_output.stdout);
    let container_processes: Vec<&str> = ps_string
        .lines()
        .filter(|line| line.contains("sleep 300") || line.contains(&container_id1[..8]))
        .collect();
    
    if container_processes.is_empty() {
        println!("❌ No container process found in host system");
    } else {
        println!("✅ Found container processes:");
        for process in container_processes {
            println!("   {}", process);
        }
    }
    
    // Test exec immediately if container is running
    if status.state == aria_runtime::types::ContainerState::Running {
        println!("✅ Container is running, testing exec...");
        
        // Test 1: Simple echo command
        println!("🧪 Test exec 1: echo command");
        match quilt.exec_in_container(
            container_id1.clone(),
            vec!["echo".to_string(), "Hello from container".to_string()],
        ).await {
            Ok(result) => {
                println!("✅ Echo exec succeeded:");
                println!("   Exit code: {}", result.exit_code);
                println!("   Stdout: '{}'", result.stdout);
                println!("   Stderr: '{}'", result.stderr);
            }
            Err(e) => {
                println!("❌ Echo exec failed: {}", e);
            }
        }
        
        // Test 2: List directory command  
        println!("🧪 Test exec 2: ls command");
        match quilt.exec_in_container(
            container_id1.clone(),
            vec!["ls".to_string(), "-la".to_string(), "/bin".to_string()],
        ).await {
            Ok(result) => {
                println!("✅ Ls exec succeeded:");
                println!("   Exit code: {}", result.exit_code);
                println!("   Stdout: '{}'", result.stdout);
                if !result.stderr.is_empty() {
                    println!("   Stderr: '{}'", result.stderr);
                }
            }
            Err(e) => {
                println!("❌ Ls exec failed: {}", e);
            }
        }
    } else {
        println!("❌ Container is not in running state: {:?}", status.state);
        
        // Get container logs for debugging
        println!("📋 Fetching container logs...");
        match quilt.get_container_logs(container_id1.clone()).await {
            Ok(logs) => {
                println!("📋 Container logs:");
                println!("{}", logs);
            }
            Err(log_err) => {
                println!("❌ Failed to get logs: {}", log_err);
            }
        }
    }
    
    println!("🧹 Cleaning up...");
    let _ = quilt.stop_container(container_id1.clone()).await;
    let _ = quilt.remove_container(container_id1).await;
    
    // Kill daemon
    daemon_handle.abort();
    
    println!("✅ Test completed!");
    Ok(())
} 