use std::process::Command;
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Starting minimal container test with debug logging...");
    
    // Test 0: Check if daemon is running by trying to get status of a non-existent container
    // This should connect to the daemon and fail gracefully with "container not found"
    println!("\n=== TEST 0: CHECK DAEMON STATUS ===");
    let output = Command::new("./target/debug/cli")
        .args(&["status", "non-existent-container"])
        .output()?;
    
    println!("Daemon status check:");
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    println!("exit code: {}", output.status.code().unwrap_or(-1));
    
    if output.status.code().unwrap_or(-1) == 1 && 
       String::from_utf8_lossy(&output.stderr).contains("Failed to connect to server") {
        println!("❌ Daemon not responding - trying to start it...");
        
        // Start daemon in background using the correct binary
        let _ = Command::new("./target/debug/quilt")
            .spawn()?;
        
        thread::sleep(Duration::from_secs(3));
        
        // Check again
        let output = Command::new("./target/debug/cli")
            .args(&["status", "non-existent-container"])
            .output()?;
        
        if String::from_utf8_lossy(&output.stderr).contains("Failed to connect to server") {
            return Err("Daemon still not responding".into());
        }
    }
    
    println!("✅ Daemon is responding!");
    
    // Test 1: Create container using CLI binary
    println!("\n=== TEST 1: CREATE CONTAINER ===");
    let output = Command::new("./target/debug/cli")
        .args(&["create", 
               "--image-path", "/root/aria-fw/crates/quilt/nixos-minimal.tar.gz",
               "/bin/sleep", "3600"])
        .output()?;
    
    println!("Create output:");
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    println!("exit code: {}", output.status.code().unwrap_or(-1));
    
    if !output.status.success() {
        println!("❌ Create failed - stopping test");
        return Err("Create failed".into());
    }
    
    let container_id = String::from_utf8_lossy(&output.stdout)
        .lines()
        .find(|line| line.contains("Container ID:"))
        .and_then(|line| line.split(": ").nth(1))
        .unwrap_or("")
        .trim()
        .to_string();
    
    if container_id.is_empty() {
        return Err("Could not extract container ID".into());
    }
    
    println!("✅ Container created: {}", container_id);
    
    // Wait for container to start
    thread::sleep(Duration::from_secs(2));
    
    // Test 2: Check container status
    println!("\n=== TEST 2: CHECK CONTAINER STATUS ===");
    let output = Command::new("./target/debug/cli")
        .args(&["status", &container_id])
        .output()?;
    
    println!("Status output:");
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    println!("exit code: {}", output.status.code().unwrap_or(-1));
    
    // Test 3: Check if sleep binary exists via exec
    println!("\n=== TEST 3: CHECK SLEEP BINARY ===");
    let output = Command::new("./target/debug/cli")
        .args(&["icc", "exec", &container_id, "--", "/bin/ls", "-la", "/bin/sleep"])
        .output()?;
    
    println!("Sleep binary check:");
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    println!("exit code: {}", output.status.code().unwrap_or(-1));
    
    // Test 4: Check what's actually in /bin
    println!("\n=== TEST 4: LIST /bin CONTENTS ===");
    let output = Command::new("./target/debug/cli")
        .args(&["icc", "exec", &container_id, "--", "/bin/ls", "-la", "/bin/"])
        .output()?;
    
    println!("Bin directory contents:");
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    
    // Test 5: Try sleep command directly
    println!("\n=== TEST 5: TRY SLEEP DIRECTLY ===");
    let output = Command::new("./target/debug/cli")
        .args(&["icc", "exec", &container_id, "--", "/bin/sleep", "1"])
        .output()?;
    
    println!("Direct sleep test:");
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    println!("exit code: {}", output.status.code().unwrap_or(-1));
    
    // Test 6: Try with shell wrapper (like the failing case)
    println!("\n=== TEST 6: TRY SHELL WRAPPER ===");
    let output = Command::new("./target/debug/cli")
        .args(&["icc", "exec", &container_id, "--", "/bin/sh", "-c", "/bin/sleep 1"])
        .output()?;
    
    println!("Shell wrapper test:");
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    println!("exit code: {}", output.status.code().unwrap_or(-1));
    
    // Test 7: Check PATH inside container
    println!("\n=== TEST 7: CHECK PATH ENVIRONMENT ===");
    let output = Command::new("./target/debug/cli")
        .args(&["icc", "exec", &container_id, "--", "/bin/sh", "-c", "echo PATH=$PATH"])
        .output()?;
    
    println!("PATH check:");
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    
    // Test 8: Try with which command
    println!("\n=== TEST 8: WHICH SLEEP ===");
    let output = Command::new("./target/debug/cli")
        .args(&["icc", "exec", &container_id, "--", "/bin/sh", "-c", "which sleep || echo 'sleep not in PATH'"])
        .output()?;
    
    println!("Which sleep:");
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    
    // Cleanup
    println!("\n=== CLEANUP ===");
    let _cleanup = Command::new("./target/debug/cli")
        .args(&["remove", &container_id, "--force"])
        .output();
    
    println!("✅ Test completed!");
    Ok(())
} 