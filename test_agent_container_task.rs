use std::process::Command;
use std::thread;
use std::time::Duration;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Agent Container Task Execution - Increment Number");
    println!("================================================================");
    
    // Test parameters
    let input_number = 42;
    let expected_output = input_number + 1;
    
    println!("📋 Test Setup:");
    println!("   Input Number: {}", input_number);
    println!("   Expected Output: {}", expected_output);
    println!();
    
    // Step 1: Create container
    println!("🚀 Step 1: Creating container...");
    let create_output = Command::new("./target/debug/cli")
        .args(&[
            "create", 
            "--image-path", "/root/aria-fw/crates/quilt/nixos-production.tar.gz",
            "/bin/sleep", "1800"  // 30 minutes
        ])
        .output()?;
    
    if !create_output.status.success() {
        println!("❌ Container creation failed:");
        println!("stdout: {}", String::from_utf8_lossy(&create_output.stdout));
        println!("stderr: {}", String::from_utf8_lossy(&create_output.stderr));
        return Err("Container creation failed".into());
    }
    
    let create_stdout = String::from_utf8_lossy(&create_output.stdout);
    let container_id = extract_container_id(&create_stdout)?;
    println!("✅ Container created: {}", container_id);
    
    // Step 2: Wait for container to be ready
    println!("⏳ Step 2: Waiting for container to be ready...");
    thread::sleep(Duration::from_secs(3));
    
    // Step 3: Create task script inside container
    println!("📝 Step 3: Creating task script inside container...");
    let script_content = format!(r#"#!/bin/bash
# Agent Task: Increment Number
# Input: {}
# Expected Output: {}

echo "🤖 Agent starting increment task..."
echo "Input number: {}"

# Perform the increment operation
result=$(({} + 1))

echo "Calculated result: $result"
echo "Expected result: {}"

# Verify the result
if [ "$result" -eq {} ]; then
    echo "✅ Task completed successfully!"
    echo "RESULT: $result"
    exit 0
else
    echo "❌ Task failed! Expected {}, got $result"
    exit 1
fi
"#, input_number, expected_output, input_number, input_number, expected_output, expected_output, expected_output);
    
    // Write script to temp file 
    fs::write("/tmp/increment_task.sh", &script_content)?;
        
    // Use echo to create the script inside container
    let create_script_output = Command::new("./target/debug/cli")
        .args(&[
            "icc", "exec", &container_id, "--",
            "/bin/sh", "-c", &format!(r#"cat > /tmp/increment_task.sh << 'EOF'
{}
EOF
chmod +x /tmp/increment_task.sh"#, script_content.replace("\"", "\\\""))
        ])
        .output()?;
    
    if !create_script_output.status.success() {
        println!("❌ Failed to create script in container:");
        println!("stdout: {}", String::from_utf8_lossy(&create_script_output.stdout));
        println!("stderr: {}", String::from_utf8_lossy(&create_script_output.stderr));
        return Err("Script creation failed".into());
    }
    
    println!("✅ Task script created in container");
    
    // Step 4: Execute the agent task
    println!("🤖 Step 4: Executing agent task...");
    let task_output = Command::new("./target/debug/cli")
        .args(&[
            "icc", "exec", &container_id, "--",
            "/bin/bash", "/tmp/increment_task.sh"
        ])
        .output()?;
    
    println!("📤 Task Execution Output:");
    println!("{}", String::from_utf8_lossy(&task_output.stdout));
    
    if !String::from_utf8_lossy(&task_output.stderr).is_empty() {
        println!("⚠️  Task Execution Stderr:");
        println!("{}", String::from_utf8_lossy(&task_output.stderr));
    }
    
    // Step 5: Verify results
    println!("🔍 Step 5: Verifying results...");
    let success = task_output.status.success();
    let output_text = String::from_utf8_lossy(&task_output.stdout);
    
    if success && output_text.contains(&format!("RESULT: {}", expected_output)) {
        println!("✅ SUCCESS: Agent task completed successfully!");
        println!("   ✅ Container execution: Working");
        println!("   ✅ Task logic: Correct");
        println!("   ✅ Result verification: Passed");
    } else {
        println!("❌ FAILURE: Agent task failed!");
        println!("   Exit code: {}", task_output.status.code().unwrap_or(-1));
        println!("   Expected to find: RESULT: {}", expected_output);
        println!("   Actual output: {}", output_text);
    }
    
    // Step 6: Test container introspection
    println!("🔍 Step 6: Testing container introspection...");
    let status_output = Command::new("./target/debug/cli")
        .args(&["status", &container_id])
        .output()?;
    
    println!("📊 Container Status:");
    println!("{}", String::from_utf8_lossy(&status_output.stdout));
    
    // Step 7: Test file system persistence
    println!("💾 Step 7: Testing file system persistence...");
    let file_test_output = Command::new("./target/debug/cli")
        .args(&[
            "icc", "exec", &container_id, "--",
            "/bin/sh", "-c", "echo 'Agent was here' > /tmp/agent_trace.txt && cat /tmp/agent_trace.txt"
        ])
        .output()?;
    
    println!("📁 File Persistence Test:");
    println!("{}", String::from_utf8_lossy(&file_test_output.stdout));
    
    // Step 8: Cleanup
    println!("🧹 Step 8: Cleanup...");
    let stop_output = Command::new("./target/debug/cli")
        .args(&["stop", &container_id])
        .output()?;
    
    if stop_output.status.success() {
        println!("✅ Container stopped successfully");
    } else {
        println!("⚠️  Container stop may have failed (this is often normal)");
    }
    
    // Final summary
    println!();
    println!("🎯 TEST SUMMARY");
    println!("================");
    if success {
        println!("✅ OVERALL: Agent container task execution is WORKING!");
        println!("✅ The container runtime is ready for agent workloads");
        println!("✅ Basic agent task execution pattern validated");
    } else {
        println!("❌ OVERALL: Agent container task execution needs work");
        println!("❌ Issues detected in container runtime or task execution");
    }
    
    Ok(())
}

fn extract_container_id(output: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Look for "Container ID: " pattern in output
    for line in output.lines() {
        if line.contains("Container ID:") {
            if let Some(id_part) = line.split("Container ID:").nth(1) {
                let id = id_part.trim();
                return Ok(id.to_string());
            }
        }
    }
    
    // Alternative: look for UUID pattern
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.len() == 36 && trimmed.chars().filter(|&c| c == '-').count() == 4 {
            return Ok(trimmed.to_string());
        }
    }
    
    Err("Could not extract container ID from output".into())
} 