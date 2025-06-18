use aria_runtime::{
    engines::container::quilt::QuiltService,
    engines::config::QuiltConfig,
    types::ContainerState,
};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Container Diagnostic Test");
    println!("===========================");
    println!();

    // Connect to Quilt daemon
    let quilt_config = QuiltConfig {
        socket_path: "/run/quilt/api.sock".to_string(),
    };
    
    let mut quilt_service = QuiltService::new(&quilt_config).await?;
    println!("✅ Connected to Quilt daemon");

    // ✅ Test 1: Check if image file exists and is readable
    println!();
    println!("📦 Test 1: Image file validation...");
    
    let current_dir = std::env::current_dir()?;
    let image_path = current_dir.join("crates/quilt/nixos-production.tar.gz");
    
    if !image_path.exists() {
        eprintln!("❌ Image file not found: {}", image_path.display());
        return Ok(());
    }
    
    let metadata = std::fs::metadata(&image_path)?;
    println!("✅ Image file exists: {} ({} bytes)", image_path.display(), metadata.len());
    
    // Try to read the first few bytes to verify it's a valid gzip file
    use std::io::Read;
    let mut file = std::fs::File::open(&image_path)?;
    let mut buffer = [0u8; 10];
    let bytes_read = file.read(&mut buffer)?;
    
    if bytes_read >= 2 && buffer[0] == 0x1f && buffer[1] == 0x8b {
        println!("✅ Image file has valid gzip header");
    } else {
        eprintln!("⚠️  Image file may not be a valid gzip archive");
        println!("   First {} bytes: {:02x?}", bytes_read, &buffer[..bytes_read]);
    }

    // ✅ Test 2: Create container with simple command
    println!();
    println!("🏗️  Test 2: Container creation with simple command...");
    
    let container_id = match quilt_service.create_container(
        image_path.to_string_lossy().to_string(),
        vec!["/bin/echo".to_string(), "Hello".to_string()],
        HashMap::new(),
    ).await {
        Ok(id) => {
            println!("✅ Container created: {}", id);
            id
        }
        Err(e) => {
            eprintln!("❌ Container creation failed: {}", e);
            return Ok(());
        }
    };

    // ✅ Test 3: Start container and monitor very closely
    println!();
    println!("🚀 Test 3: Container startup monitoring...");
    
    let start_time = std::time::Instant::now();
    
    match quilt_service.start_container(container_id.clone()).await {
        Ok(()) => {
            println!("✅ Container start command sent at {:?}", start_time.elapsed());
        }
        Err(e) => {
            eprintln!("❌ Container start failed: {}", e);
            return Ok(());
        }
    }

    // Monitor status changes very frequently
    println!();
    println!("🔍 Test 4: High-frequency status monitoring...");
    
    for i in 0..20 {
        sleep(Duration::from_millis(100)).await;
        
        match quilt_service.get_container_status(container_id.clone()).await {
            Ok(status) => {
                let elapsed = start_time.elapsed();
                println!("   {:02}: {:?} at {:?}", i, status.state, elapsed);
                
                if matches!(status.state, ContainerState::Exited) {
                    println!("   Exit code: {:?}", status.exit_code);
                    break;
                }
                if matches!(status.state, ContainerState::Running) {
                    println!("✅ Container reached running state!");
                    sleep(Duration::from_secs(2)).await;
                    break;
                }
            }
            Err(e) => {
                eprintln!("   {:02}: Status check failed: {}", i, e);
            }
        }
    }

    // ✅ Test 5: Try different command types
    println!();
    println!("🧪 Test 5: Testing different command types...");
    
    let test_commands = vec![
        vec!["/bin/true".to_string()],
        vec!["/bin/echo".to_string(), "test".to_string()],
        vec!["/bin/sh".to_string(), "-c".to_string(), "echo working".to_string()],
        vec!["/bin/sleep".to_string(), "1".to_string()],
    ];

    for (i, command) in test_commands.iter().enumerate() {
        println!("   Testing command {}: {:?}", i + 1, command);
        
        let test_id = match quilt_service.create_container(
            image_path.to_string_lossy().to_string(),
            command.clone(),
            HashMap::new(),
        ).await {
            Ok(id) => id,
            Err(e) => {
                eprintln!("   ❌ Create failed: {}", e);
                continue;
            }
        };

        let start = std::time::Instant::now();
        if let Ok(()) = quilt_service.start_container(test_id.clone()).await {
            // Wait and check final state
            sleep(Duration::from_millis(500)).await;
            
            match quilt_service.get_container_status(test_id.clone()).await {
                Ok(status) => {
                    println!("   ✅ Result: {:?} (exit code: {:?}) in {:?}", 
                             status.state, status.exit_code, start.elapsed());
                }
                Err(e) => {
                    eprintln!("   ❌ Status failed: {}", e);
                }
            }
        } else {
            eprintln!("   ❌ Start failed");
        }

        // Cleanup
        let _ = quilt_service.remove_container(test_id).await;
        sleep(Duration::from_millis(100)).await;
    }

    // Cleanup main test container
    let _ = quilt_service.remove_container(container_id).await;

    println!();
    println!("🎯 Diagnostic Summary:");
    println!("   - Check logs above for specific failure points");
    println!("   - Look for patterns in exit codes and timing");
    println!("   - Verify which commands work vs fail");
    
    Ok(())
} 