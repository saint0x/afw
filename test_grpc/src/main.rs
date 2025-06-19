// test_grpc_upload.rs - Test script for gRPC upload functionality
// Run with: cargo run --bin test_grpc_upload

use anyhow::Result;
use std::fs;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let start_time = std::time::Instant::now();
    let project_name = "e2e-test-project";

    // Get the workspace root (aria-fw directory)
    let current_dir = env::current_dir()?;
    let workspace_root = if current_dir.ends_with("test_grpc") {
        current_dir.parent().unwrap().to_path_buf()
    } else {
        current_dir
    };
    let project_path = workspace_root.join(project_name);

    println!("🚀 Testing Aria `new -> check -> build -> upload` Workflow");
    println!("======================================================");
    println!("📁 Test project will be created at: {}", project_path.display());
    println!();
    let ar_c_manifest = workspace_root.join("crates/ar-c/Cargo.toml");

    // --- Step 1: `arc new` ---
    println!("1️⃣  Running `arc new`...");
    
    // Clean up any existing test project
    if project_path.exists() {
        println!("   Cleaning up existing test project...");
        fs::remove_dir_all(&project_path)?;
    }
    
    let new_output = Command::new("cargo")
        .args(&[
            "run", "--manifest-path", ar_c_manifest.to_str().unwrap(), "--quiet", "--",
            "new", project_name,
        ])
        .current_dir(&workspace_root)
        .output()?;

    if !new_output.status.success() {
        eprintln!("❌ `arc new` failed!");
        eprintln!("Stderr: {}", String::from_utf8_lossy(&new_output.stderr));
        std::process::exit(1);
    }
    
    // Show output from arc new
    let stdout = String::from_utf8_lossy(&new_output.stdout);
    let stderr = String::from_utf8_lossy(&new_output.stderr);
    if !stdout.trim().is_empty() {
        for line in stdout.lines() {
            println!("   [NEW] {}", line);
        }
    }
    if !stderr.trim().is_empty() {
        for line in stderr.lines() {
            println!("   [NEW] {}", line);
        }
    }
    println!("✅ `arc new` completed successfully.\n");

    // --- Step 2: `arc check` ---
    println!("2️⃣  Running `arc check`...");
    let check_output = Command::new("cargo")
        .args(&[
            "run", "--manifest-path", ar_c_manifest.to_str().unwrap(), "--quiet", "--",
            "check",
        ])
        .current_dir(&project_path)
        .output()?;

    if !check_output.status.success() {
        eprintln!("❌ `arc check` failed!");
        eprintln!("Stderr: {}", String::from_utf8_lossy(&check_output.stderr));
        std::process::exit(1);
    }
    
    // Show output from arc check
    let stdout = String::from_utf8_lossy(&check_output.stdout);
    let stderr = String::from_utf8_lossy(&check_output.stderr);
    if !stdout.trim().is_empty() {
        for line in stdout.lines() {
            println!("   [CHECK] {}", line);
        }
    }
    if !stderr.trim().is_empty() {
        for line in stderr.lines() {
            println!("   [CHECK] {}", line);
        }
    }
    println!("✅ `arc check` completed successfully.\n");

    // --- Step 3: `arc build` ---
    println!("3️⃣  Running `arc build`...");
    let build_output = Command::new("cargo")
        .args(&[
            "run", "--manifest-path", ar_c_manifest.to_str().unwrap(), "--quiet", "--",
            "build",
        ])
        .current_dir(&project_path)
        .output()?;

    if !build_output.status.success() {
        eprintln!("❌ `arc build` failed!");
        eprintln!("Stderr: {}", String::from_utf8_lossy(&build_output.stderr));
        std::process::exit(1);
    }
    
    // Show output from arc build
    let stdout = String::from_utf8_lossy(&build_output.stdout);
    let stderr = String::from_utf8_lossy(&build_output.stderr);
    if !stdout.trim().is_empty() {
        for line in stdout.lines() {
            println!("   [BUILD] {}", line);
        }
    }
    if !stderr.trim().is_empty() {
        for line in stderr.lines() {
            println!("   [BUILD] {}", line);
        }
    }
    println!("✅ `arc build` completed successfully.\n");
    
    // Find the created bundle file
    let bundle_file = fs::read_dir(project_path.join("dist"))?
        .filter_map(|entry| entry.ok())
        .find(|entry| entry.path().extension().map_or(false, |ext| ext == "aria"))
        .ok_or_else(|| anyhow::anyhow!("Could not find built .aria file"))?
        .path();

    // --- Step 4: `arc upload` ---
    println!("4️⃣  Running `arc upload` and streaming output...");
    let mut upload_cmd = Command::new("cargo")
        .args(&[
            "run", "--manifest-path", ar_c_manifest.to_str().unwrap(), "--quiet", "--",
            "upload", bundle_file.to_str().unwrap(),
        ])
        .current_dir(&project_path)
        .stdout(Stdio::piped())
        .spawn()?;

    let stdout = upload_cmd.stdout.take().expect("Failed to capture stdout");
    let reader = BufReader::new(stdout);

    let mut found_progress = false;
    let mut found_complete = false;

    for line in reader.lines() {
        let line = line?;
        println!("   [UPLOAD] {}", line);
        if line.contains("Progress") {
            found_progress = true;
        }
        if line.contains("Bundle deployed") {
            found_complete = true;
        }
    }

    let status = upload_cmd.wait()?;
    if !status.success() {
        eprintln!("❌ `arc upload` process failed!");
        std::process::exit(1);
    }

    if !found_progress {
        eprintln!("❌ Did not find progress indicator in upload output.");
    }

    if !found_complete {
        eprintln!("❌ Did not find completion message in upload output.");
    }
    println!("✅ `arc upload` completed successfully.\n");

    let total_time = start_time.elapsed();
    println!("🎉 End-to-end test completed in {:.2}s", total_time.as_secs_f64());
    println!("📁 Test project remains at: {}", project_path.display());
    println!("   To clean up, run: rm -rf {}", project_path.display());

    Ok(())
} 