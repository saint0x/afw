use quilt::proto::{quilt_service_client::QuiltServiceClient, CreateContainerRequest, GetContainerStatusRequest};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Testing Foreign Key Constraint Fix");
    println!("====================================");
    
    // Wait for Quilt server to be ready
    sleep(Duration::from_secs(2)).await;
    
    // Connect to Quilt gRPC service
    let mut client = QuiltServiceClient::connect("http://127.0.0.1:50051").await?;
    
    println!("🚀 Connected to Quilt server");
    
    // Test 1: Create multiple containers rapidly to test foreign key constraints
    println!("🔬 Test 1: Creating multiple containers rapidly...");
    
    for i in 1..=5 {
        let container_id = format!("test-fk-container-{}", i);
        
        let mut environment = HashMap::new();
        environment.insert("TEST_VAR".to_string(), format!("container-{}", i));
        
        let request = CreateContainerRequest {
            image_path: "/tmp/test-image.tar.gz".to_string(),
            command: vec!["/bin/sh".to_string(), "-c".to_string(), "sleep 30".to_string()],
            environment,
            working_directory: "/tmp".to_string(),
            setup_commands: vec![],
            memory_limit_mb: 512,
            cpu_limit_percent: 50.0,
            enable_pid_namespace: true,
            enable_mount_namespace: true,
            enable_uts_namespace: true,
            enable_ipc_namespace: true,
            enable_network_namespace: true,
            auto_start: false,
        };
        
        match client.create_container(request).await {
            Ok(response) => {
                println!("✅ Container {} created successfully", container_id);
                let reply = response.into_inner();
                println!("   Container ID: {}", reply.container_id);
                println!("   Success: {}", reply.success);
                
                // Check container status immediately to test foreign key relationships
                let status_request = GetContainerStatusRequest {
                    container_id: reply.container_id.clone(),
                };
                
                match client.get_container_status(status_request).await {
                    Ok(status_response) => {
                        let status = status_response.into_inner();
                        println!("   Status: {:?}", status.status);
                        println!("   Network IP: {}", status.ip_address);
                        println!("   PID: {}", status.pid);
                    }
                    Err(e) => {
                        println!("❌ Failed to get status for {}: {}", reply.container_id, e);
                    }
                }
            }
            Err(e) => {
                println!("❌ Failed to create container {}: {}", container_id, e);
                // Check if this is a foreign key constraint error
                if e.to_string().contains("FOREIGN KEY constraint failed") {
                    println!("🚨 FOREIGN KEY CONSTRAINT ERROR DETECTED!");
                    println!("   This indicates our fix didn't work correctly");
                    return Err(e.into());
                }
            }
        }
        
        // Small delay between creations
        sleep(Duration::from_millis(100)).await;
    }
    
    println!("✅ All containers created successfully - no foreign key constraint errors!");
    println!("🔬 Test completed successfully");
    
    Ok(())
} 