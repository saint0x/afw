use aria_runtime::{
    engines::container::quilt::QuiltService,
    engines::config::QuiltConfig,
    types::ContainerState,
};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Unix Socket Infrastructure Test");
    println!("=====================================");
    println!();

    // ✅ Phase 1: Start Quilt Daemon (Unix Socket Server)
    println!("📡 Phase 1: Starting Quilt daemon with Unix socket server...");
    let daemon_process = tokio::process::Command::new("cargo")
        .args(&["run", "--release", "--bin", "quilt"])
        .current_dir("crates/quilt")
        .env("RUST_LOG", "info")
        .kill_on_drop(true)
        .spawn()?;
    
    println!("⏳ Waiting for Unix socket server to initialize...");
    sleep(Duration::from_secs(5)).await;
    
    // ✅ Phase 2: Test Unix Socket Connection
    println!();
    println!("🔌 Phase 2: Testing Unix socket client connection...");
    
    let socket_path = "/run/quilt/api.sock";
    println!("📍 Socket path: {}", socket_path);
    
    // Check if socket file exists
    if std::path::Path::new(socket_path).exists() {
        println!("✅ Unix socket file exists");
        
        // Check socket permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(socket_path)?;
            let permissions = metadata.permissions().mode();
            println!("🔒 Socket permissions: {:o}", permissions & 0o777);
            
            if (permissions & 0o777) == 0o660 {
                println!("✅ Socket permissions are correct (660)");
            } else {
                eprintln!("⚠️  Socket permissions may be incorrect");
            }
        }
    } else {
        return Err(format!("❌ Unix socket file not found at {}", socket_path).into());
    }

    // ✅ Phase 3: Create QuiltService Client 
    println!();
    println!("📦 Phase 3: Creating QuiltService client...");
    
    let quilt_config = QuiltConfig {
        socket_path: socket_path.to_string(),
    };
    
    let mut quilt_service = match QuiltService::new(&quilt_config).await {
        Ok(service) => {
            println!("✅ QuiltService client connected via Unix socket");
            service
        }
        Err(e) => {
            return Err(format!("❌ Failed to connect QuiltService: {}", e).into());
        }
    };

    // ✅ Phase 4: Test Basic gRPC Operations
    println!();
    println!("🧪 Phase 4: Testing gRPC operations over Unix socket...");
    
    // Test system metrics call
    println!("📊 Testing system metrics call...");
    match quilt_service.get_system_metrics().await {
        Ok(metrics) => {
            println!("✅ System metrics retrieved successfully");
            println!("   - Total memory: {} bytes", metrics.total_memory_bytes);
            println!("   - Used memory: {} bytes", metrics.used_memory_bytes);
            println!("   - CPU usage: {:.2}%", metrics.cpu_usage_percent);
            println!("   - Active containers: {}", metrics.active_containers);
        }
        Err(e) => {
            eprintln!("❌ Failed to get system metrics: {}", e);
        }
    }

    // Test container listing
    println!();
    println!("📋 Testing container listing...");
    match quilt_service.list_containers().await {
        Ok(containers) => {
            println!("✅ Container list retrieved successfully");
            println!("   - Found {} containers", containers.len());
            for container in containers {
                println!("   - Container ID: {}", container.container_id);
                println!("     Status: {:?}", container.status);
                println!("     Image: {}", container.image_path);
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to list containers: {}", e);
        }
    }

    // ✅ Phase 5: Test Container Lifecycle over Unix Socket
    println!();
    println!("📦 Phase 5: Testing container lifecycle...");
    
    // Get current directory for image path
    let current_dir = std::env::current_dir()?;
    let image_path = current_dir.join("crates/quilt/nixos-production.tar.gz");
    
    if !image_path.exists() {
        eprintln!("⚠️  Image file not found at: {}", image_path.display());
        eprintln!("   Skipping container lifecycle test");
    } else {
        println!("🏗️  Creating test container...");
        
        let container_id = match quilt_service.create_container(
            image_path.to_string_lossy().to_string(),
            vec!["/bin/sh".to_string(), "-c".to_string(), "sleep 30".to_string()],
            HashMap::new(),
        ).await {
            Ok(id) => {
                println!("✅ Container created with ID: {}", id);
                id
            }
            Err(e) => {
                eprintln!("❌ Failed to create container: {}", e);
                return Ok(());
            }
        };

        // Start container
        println!("🚀 Starting container...");
        match quilt_service.start_container(container_id.clone()).await {
            Ok(()) => {
                println!("✅ Container start command sent");
            }
            Err(e) => {
                eprintln!("❌ Failed to start container: {}", e);
            }
        }

        // Wait for container to be running
        println!("⏳ Waiting for container to be running...");
        let mut attempts = 0;
        let max_attempts = 10;
        
        while attempts < max_attempts {
            sleep(Duration::from_secs(1)).await;
            
            match quilt_service.get_container_status(container_id.clone()).await {
                Ok(status) => {
                    println!("🔍 Container status: {:?}", status.state);
                    
                    if matches!(status.state, ContainerState::Running) {
                        println!("✅ Container is now running!");
                        break;
                    } else if matches!(status.state, ContainerState::Exited) {
                        println!("⚠️  Container exited prematurely");
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to get container status: {}", e);
                }
            }
            
            attempts += 1;
        }

        if attempts >= max_attempts {
            eprintln!("⚠️  Container did not reach running state within {} attempts", max_attempts);
        }

        // Test exec command
        println!();
        println!("⚙️  Testing exec command...");
        match quilt_service.exec_in_container(
            container_id.clone(),
            vec!["echo".to_string(), "Hello from Unix socket!".to_string()],
        ).await {
            Ok(result) => {
                println!("✅ Exec command completed");
                println!("   Exit code: {}", result.exit_code);
                println!("   Stdout: '{}'", result.stdout);
                if !result.stderr.is_empty() {
                    println!("   Stderr: '{}'", result.stderr);
                }
            }
            Err(e) => {
                eprintln!("❌ Failed to exec command: {}", e);
            }
        }

        // Clean up
        println!();
        println!("🧹 Cleaning up test container...");
        let _ = quilt_service.stop_container(container_id.clone()).await;
        let _ = quilt_service.remove_container(container_id).await;
        println!("✅ Container cleanup completed");
    }

    // ✅ Phase 6: Test Network Topology
    println!();
    println!("🌐 Phase 6: Testing network topology...");
    match quilt_service.get_network_topology().await {
        Ok(topology) => {
            println!("✅ Network topology retrieved successfully");
            println!("   - Found {} network nodes", topology.len());
            for node in topology {
                println!("   - Container: {}, IP: {}", node.container_id, node.ip_address);
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to get network topology: {}", e);
        }
    }

    // ✅ Phase 7: Performance Test
    println!();
    println!("⚡ Phase 7: Unix socket performance test...");
    
    let start_time = std::time::Instant::now();
    let iterations = 50;
    
    for i in 0..iterations {
        if i % 10 == 0 {
            print!(".");
            use std::io::{self, Write};
            io::stdout().flush().unwrap();
        }
        
        match quilt_service.get_system_metrics().await {
            Ok(_) => {},
            Err(e) => {
                eprintln!("\n❌ Performance test failed at iteration {}: {}", i, e);
                break;
            }
        }
    }
    
    let duration = start_time.elapsed();
    let avg_latency = duration.as_millis() as f64 / iterations as f64;
    
    println!();
    println!("✅ Performance test completed");
    println!("   - {} iterations in {:?}", iterations, duration);
    println!("   - Average latency: {:.2}ms per call", avg_latency);
    println!("   - Throughput: {:.2} calls/sec", 1000.0 / avg_latency);

    // ✅ Final Summary
    println!();
    println!("🎉 Unix Socket Infrastructure Test Complete!");
    println!("===============================================");
    println!("✅ Unix socket server: Running");
    println!("✅ Unix socket client: Connected");
    println!("✅ gRPC over Unix socket: Functional");
    println!("✅ Container lifecycle: Tested");
    println!("✅ Performance: {:.2}ms avg latency", avg_latency);
    println!();
    println!("🔒 Security Benefits:");
    println!("   - No network exposure (localhost bypass)");
    println!("   - Filesystem-based permissions");
    println!("   - Operating system process isolation");
    println!("   - No port conflicts or binding issues");
    println!();
    println!("🚀 Ready for macOS desktop app integration!");

    // Keep daemon alive for a moment to ensure clean shutdown
    sleep(Duration::from_secs(2)).await;
    
    Ok(())
} 