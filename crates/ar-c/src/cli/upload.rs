use clap::ArgMatches;
use anyhow::{Result, anyhow};
use std::path::Path;
use reqwest::Client;
use serde::Deserialize;
use tokio::fs;

use crate::cli::{print_status, print_error, print_info, print_warning};
use crate::bundle::AriaBundle;

/// Handle the 'arc upload' command
pub async fn handle_upload_command(matches: &ArgMatches) -> Result<()> {
    let bundle_path = matches.get_one::<String>("bundle").unwrap();
    let server_url = matches.get_one::<String>("server").unwrap();
    let auth_token = matches.get_one::<String>("auth-token");
    
    print_info(&format!("Uploading bundle: {}", bundle_path));
    print_info(&format!("Target server: {}", server_url));
    
    // Validate bundle exists
    if !Path::new(bundle_path).exists() {
        return Err(anyhow!("Bundle file not found: {}", bundle_path));
    }
    
    // Validate bundle integrity
    validate_bundle(bundle_path).await?;
    
    // Upload to server
    upload_bundle_to_server(bundle_path, server_url, auth_token.map(|s| s.as_str())).await?;
    
    print_status("Uploaded", &format!("Bundle deployed to {}", server_url));
    
    Ok(())
}

/// Validate bundle integrity before upload
async fn validate_bundle(bundle_path: &str) -> Result<()> {
    print_status("Validating", "bundle integrity...");
    
    // Load and validate bundle
    let bundle = AriaBundle::load_from_file(bundle_path).await?;
    
    // Basic validation
    if bundle.manifest.tools.is_empty() && bundle.manifest.agents.is_empty() {
        print_warning("Bundle contains no tools or agents");
    }
    
    // Check bundle size
    let metadata = fs::metadata(bundle_path).await?;
    let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
    
    if size_mb > 50.0 {
        print_warning(&format!("Large bundle size: {:.1} MB", size_mb));
    }
    
    print_info(&format!("Bundle validation passed"));
    print_info(&format!("  - Tools: {}", bundle.manifest.tools.len()));
    print_info(&format!("  - Agents: {}", bundle.manifest.agents.len()));
    print_info(&format!("  - Size: {:.2} MB", size_mb));
    
    Ok(())
}

/// Upload bundle to Aria runtime server
async fn upload_bundle_to_server(
    bundle_path: &str, 
    server_url: &str, 
    auth_token: Option<&str>
) -> Result<()> {
    print_status("Uploading", "bundle to server...");
    
    let client = Client::new();
    
    // Read bundle file
    let bundle_data = fs::read(bundle_path).await?;
    
    // Prepare upload request
    let upload_url = format!("{}/api/bundles/upload", server_url.trim_end_matches('/'));
    
    let mut request = client
        .post(&upload_url)
        .header("Content-Type", "application/octet-stream")
        .body(bundle_data);
    
    // Add authentication if provided
    if let Some(token) = auth_token {
        request = request.header("Authorization", format!("Bearer {}", token));
    }
    
    // Send request
    match request.send().await {
        Ok(response) => {
            if response.status().is_success() {
                // Parse response
                match response.json::<UploadResponse>().await {
                    Ok(upload_result) => {
                        print_status("Success", "Bundle uploaded successfully");
                        print_info(&format!("Bundle ID: {}", upload_result.bundle_id));
                        print_info(&format!("Deployment URL: {}", upload_result.deployment_url));
                        
                        if let Some(tools) = upload_result.registered_tools {
                            print_info(&format!("Registered {} tools", tools.len()));
                            if !tools.is_empty() {
                                for tool in tools.iter().take(5) {  // Show first 5
                                    println!("    - {}", tool);
                                }
                                if tools.len() > 5 {
                                    println!("    ... and {} more", tools.len() - 5);
                                }
                            }
                        }
                        
                        if let Some(agents) = upload_result.registered_agents {
                            print_info(&format!("Registered {} agents", agents.len()));
                            if !agents.is_empty() {
                                for agent in agents.iter().take(3) {  // Show first 3
                                    println!("    - {}", agent);
                                }
                                if agents.len() > 3 {
                                    println!("    ... and {} more", agents.len() - 3);
                                }
                            }
                        }
                    }
                    Err(_) => {
                        print_status("Success", "Bundle uploaded (response parse failed)");
                    }
                }
            } else {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                
                match status.as_u16() {
                    401 => {
                        print_error("Authentication failed. Check your auth token.");
                        print_info("Use: arc upload bundle.aria --server https://server.com --auth-token YOUR_TOKEN");
                    }
                    403 => {
                        print_error("Permission denied. You may not have upload permissions.");
                    }
                    413 => {
                        print_error("Bundle too large. Try reducing bundle size.");
                    }
                    422 => {
                        print_error("Invalid bundle format or content.");
                        if !error_text.is_empty() {
                            print_info(&format!("Details: {}", error_text));
                        }
                    }
                    500..=599 => {
                        print_error("Server error. Try again later.");
                    }
                    _ => {
                        print_error(&format!("Upload failed with status: {}", status));
                    }
                }
                
                return Err(anyhow!("Upload failed: {} - {}", status, error_text));
            }
        }
        Err(e) => {
            if e.is_connect() {
                print_error("Could not connect to server. Check the server URL.");
                print_info(&format!("Attempted to connect to: {}", upload_url));
            } else if e.is_timeout() {
                print_error("Upload timed out. The bundle may be too large or server is slow.");
            } else {
                print_error(&format!("Network error: {}", e));
            }
            return Err(anyhow!("Upload failed: {}", e));
        }
    }
    
    Ok(())
}

/// Response from bundle upload API
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct UploadResponse {
    bundle_id: String,
    deployment_url: String,
    registered_tools: Option<Vec<String>>,
    registered_agents: Option<Vec<String>>,
    message: Option<String>,
}

/// Check server health before upload
async fn check_server_health(server_url: &str) -> Result<bool> {
    let client = Client::new();
    let health_url = format!("{}/api/health", server_url.trim_end_matches('/'));
    
    match client.get(&health_url).send().await {
        Ok(response) => Ok(response.status().is_success()),
        Err(_) => Ok(false),
    }
}

/// Get server information
pub async fn get_server_info(server_url: &str) -> Result<ServerInfo> {
    let client = Client::new();
    let info_url = format!("{}/api/info", server_url.trim_end_matches('/'));
    
    let response = client.get(&info_url).send().await?;
    
    if response.status().is_success() {
        let server_info: ServerInfo = response.json().await?;
        Ok(server_info)
    } else {
        Err(anyhow!("Could not get server information"))
    }
}

/// Server information response
#[derive(Debug, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
    pub api_version: String,
    pub capabilities: Vec<String>,
    pub max_bundle_size_mb: Option<u64>,
}

/// Test connectivity to server
pub async fn test_connection(server_url: &str, auth_token: Option<&str>) -> Result<()> {
    print_info(&format!("Testing connection to: {}", server_url));
    
    // Check basic connectivity
    if !check_server_health(server_url).await? {
        return Err(anyhow!("Server is not responding or unhealthy"));
    }
    
    // Get server info
    match get_server_info(server_url).await {
        Ok(info) => {
            print_status("Connected", "Server is reachable");
            print_info(&format!("Server: {} v{}", info.name, info.version));
            print_info(&format!("API Version: {}", info.api_version));
            
            if let Some(max_size) = info.max_bundle_size_mb {
                print_info(&format!("Max bundle size: {} MB", max_size));
            }
        }
        Err(_) => {
            print_warning("Server reachable but info unavailable");
        }
    }
    
    // Test auth if token provided
    if let Some(token) = auth_token {
        test_auth(server_url, token).await?;
    }
    
    Ok(())
}

/// Test authentication
async fn test_auth(server_url: &str, auth_token: &str) -> Result<()> {
    let client = Client::new();
    let auth_url = format!("{}/api/auth/verify", server_url.trim_end_matches('/'));
    
    let response = client
        .get(&auth_url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .send()
        .await?;
    
    if response.status().is_success() {
        print_status("Authenticated", "Token is valid");
    } else {
        print_error("Authentication failed - invalid token");
        return Err(anyhow!("Invalid authentication token"));
    }
    
    Ok(())
} 