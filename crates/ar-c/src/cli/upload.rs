use clap::ArgMatches;
use anyhow::{Result, anyhow};
use std::path::Path;

use crate::cli::{print_status, print_info};
use crate::grpc::{QuiltClient, UploadProgress};

/// Handle the 'arc upload' command
pub async fn handle_upload_command(matches: &ArgMatches) -> Result<()> {
    let bundle_path = matches.get_one::<String>("bundle").unwrap();
    let socket_path = matches.get_one::<String>("socket").map(|s| s.as_str()).unwrap_or("/run/quilt/api.sock");
    
    print_info(&format!("Uploading bundle: {}", bundle_path));
    print_info(&format!("Quilt daemon socket: {}", socket_path));
    
    // Validate bundle exists
    if !Path::new(bundle_path).exists() {
        return Err(anyhow!("Bundle file not found: {}", bundle_path));
    }
    
    // Upload via gRPC to Quilt daemon
    upload_bundle_to_quilt(bundle_path, socket_path).await?;
    
    print_status("Uploaded", "Bundle deployed to Quilt daemon");
    
    Ok(())
}

/// Upload bundle to Quilt daemon via gRPC
async fn upload_bundle_to_quilt(bundle_path: &str, socket_path: &str) -> Result<()> {
    print_status("Transport", "gRPC via Unix socket");
    
    // Connect to Quilt daemon
    let mut client = QuiltClient::connect_to_socket(socket_path).await?;
    
    // Test connection
    client.test_connection().await?;
    
    // Upload with progress reporting
    let result = client.upload_bundle(bundle_path, |progress: UploadProgress| {
        if progress.percent as u64 % 10 == 0 {  // Report every 10%
            print_info(&format!("Progress: {:.1}% ({:.1}/{:.1} MB)", 
                progress.percent,
                progress.bytes_uploaded as f64 / (1024.0 * 1024.0),
                progress.total_bytes as f64 / (1024.0 * 1024.0)
            ));
                    }
    }).await?;
    
    if result.success {
        print_status("Success", "Bundle uploaded to Quilt daemon");
        print_info(&format!("Bundle ID: {}", result.bundle_id));
        print_info(&format!("Transfer rate: {:.2} MB/s", 
            (result.bytes_uploaded as f64 / (1024.0 * 1024.0)) / result.upload_time_seconds));
    } else {
        return Err(anyhow!("Upload failed: {}", 
            result.error_message.unwrap_or_else(|| "Unknown error".to_string())));
    }
    
    Ok(())
}

 