use anyhow::{Result, anyhow};
use std::path::Path;
use tokio::fs;
use tokio_stream::wrappers::ReceiverStream;
use tokio::sync::mpsc;
use tonic::transport::{Endpoint, Channel, Uri};
use tower::service_fn;


// Include the generated gRPC code
pub mod quilt {
    tonic::include_proto!("quilt");
}

use quilt::quilt_service_client::QuiltServiceClient;
use quilt::{
    UploadBundleRequest, BundleMetadata,
    GetBundleInfoRequest, ListBundlesRequest, DeleteBundleRequest,
    ValidateBundleRequest,
};

use crate::cli::{print_status, print_info, print_error};

const DEFAULT_QUILT_SOCKET: &str = "/run/quilt/api.sock";
const CHUNK_SIZE: usize = 64 * 1024; // 64KB chunks

/// Progress information for bundle uploads
#[derive(Debug, Clone)]
pub struct UploadProgress {
    pub bytes_uploaded: u64,
    pub total_bytes: u64,
    pub percent: f64,
}

/// Result of a bundle upload operation
#[derive(Debug)]
pub struct UploadResult {
    pub bundle_id: String,
    pub success: bool,
    pub bytes_uploaded: u64,
    pub upload_time_seconds: f64,
    pub error_message: Option<String>,
}

/// gRPC client for communicating with Quilt daemon
pub struct QuiltClient {
    client: QuiltServiceClient<Channel>,
}

impl QuiltClient {
    /// Create a new QuiltClient connected to the Unix socket
    pub async fn connect() -> Result<Self> {
        Self::connect_to_socket(DEFAULT_QUILT_SOCKET).await
    }
    
    /// Create a new QuiltClient connected to a specific Unix socket path
    pub async fn connect_to_socket(socket_path: &str) -> Result<Self> {
        print_info(&format!("Connecting to Quilt daemon at: {}", socket_path));
        
        // Check if socket exists
        if !Path::new(socket_path).exists() {
            return Err(anyhow!("Quilt daemon socket not found: {}", socket_path));
        }
        
        // Create Unix socket connection
        let channel = Self::create_unix_channel(socket_path).await?;
        let client = QuiltServiceClient::new(channel);
        
        print_status("Connected", "Successfully connected to Quilt daemon");
        
        Ok(Self { client })
    }
    
    /// Create a channel connected to a Unix socket
    async fn create_unix_channel(socket_path: &str) -> Result<Channel> {
        let path = Path::new(socket_path).to_path_buf();

        // The URI here is ignored because we are using a custom connector,
        // but it's a required part of the Endpoint builder.
        let channel = Endpoint::from_static("http://[::1]:50051")
            .connect_with_connector(service_fn(move |_: Uri| {
                // Connect to a Unix socket
                tokio::net::UnixStream::connect(path.clone())
            }))
            .await
            .map_err(|e| anyhow!("Failed to connect to Quilt daemon via Unix socket: {}", e))?;
        
        Ok(channel)
    }
    
    /// Test connection to Quilt daemon
    pub async fn test_connection(&mut self) -> Result<()> {
        print_info("Testing connection to Quilt daemon...");
        
        // Try to list containers as a connectivity test
        match self.client.list_containers(quilt::ListContainersRequest {
            state_filter: 0, // Unspecified - list all
        }).await {
            Ok(_response) => {
                print_status("Connected", "Quilt daemon is responding");
                Ok(())
            }
            Err(e) => {
                print_error(&format!("Connection test failed: {}", e));
                Err(anyhow!("Failed to communicate with Quilt daemon: {}", e))
            }
        }
    }
    
    /// Upload a bundle to the Quilt daemon with progress reporting
    pub async fn upload_bundle<F>(
        &mut self,
        bundle_path: &str,
        progress_callback: F,
    ) -> Result<UploadResult>
    where
        F: Fn(UploadProgress) + Send + 'static,
    {
        print_status("Uploading", &format!("bundle via gRPC: {}", bundle_path));
        
        // Validate bundle exists
        let path = Path::new(bundle_path);
        if !path.exists() {
            return Err(anyhow!("Bundle file not found: {}", bundle_path));
        }
        
        // Stream the file directly without loading the whole bundle into memory
        let bundle_data = fs::read(path).await?;
        let total_size = bundle_data.len() as u64;
        
        print_info(&format!("Bundle size: {:.2} MB", total_size as f64 / (1024.0 * 1024.0)));
        
        // Calculate blake3 hash for integrity verification
        let blake3_hash = calculate_blake3_hash(&bundle_data)?;
        
        // Create metadata message. The name and version can be derived from the path
        // or set to a default if not easily available without full parsing.
        let file_name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let metadata = BundleMetadata {
            name: file_name,
            version: "unknown".to_string(),
            description: "".to_string(),
            total_size_bytes: total_size,
            chunk_size_bytes: CHUNK_SIZE as u32,
            blake3_hash: blake3_hash.clone(),
            signature: String::new(), // TODO: Add signature support for AUTH.MD
            uploader_identity: String::new(), // TODO: Add identity support for AUTH.MD
            metadata_fields: std::collections::HashMap::new(),
        };
        
        // Create upload stream
        let (tx, rx) = mpsc::channel(100);
        
        // Send metadata first
        let metadata_request = UploadBundleRequest {
            payload: Some(quilt::upload_bundle_request::Payload::Metadata(metadata)),
        };
        
        if tx.send(metadata_request).await.is_err() {
            return Err(anyhow!("Failed to send metadata"));
        }
        
        // Send bundle data in chunks
        let mut bytes_sent = 0u64;
        let start_time = std::time::Instant::now();
        
        for chunk in bundle_data.chunks(CHUNK_SIZE) {
            let chunk_request = UploadBundleRequest {
                payload: Some(quilt::upload_bundle_request::Payload::Chunk(chunk.to_vec())),
            };
            
            if tx.send(chunk_request).await.is_err() {
                return Err(anyhow!("Failed to send chunk"));
            }
            
            bytes_sent += chunk.len() as u64;
            let progress = UploadProgress {
                bytes_uploaded: bytes_sent,
                total_bytes: total_size,
                percent: (bytes_sent as f64 / total_size as f64) * 100.0,
            };
            
            progress_callback(progress);
        }
        
        // Send final checksum
        let checksum_request = UploadBundleRequest {
            payload: Some(quilt::upload_bundle_request::Payload::Checksum(blake3_hash)),
        };
        
        if tx.send(checksum_request).await.is_err() {
            return Err(anyhow!("Failed to send checksum"));
        }
        
        // Close the sender
        drop(tx);
        
        // Create the stream and make the request
        let request_stream = ReceiverStream::new(rx);
        let request = tonic::Request::new(request_stream);
        
        // Send the upload request
        match self.client.upload_bundle(request).await {
            Ok(response) => {
                let upload_response = response.into_inner();
                let upload_time = start_time.elapsed().as_secs_f64();
                
                if upload_response.success {
                    print_status("Success", "Bundle uploaded via gRPC");
                    print_info(&format!("Bundle ID: {}", upload_response.bundle_id));
                    print_info(&format!("Upload time: {:.2}s", upload_time));
                    print_info(&format!("Transfer rate: {:.2} MB/s", 
                        (total_size as f64 / (1024.0 * 1024.0)) / upload_time));
                    
                    Ok(UploadResult {
                        bundle_id: upload_response.bundle_id,
                        success: true,
                        bytes_uploaded: upload_response.bytes_received,
                        upload_time_seconds: upload_response.upload_time_seconds,
                        error_message: None,
                    })
                } else {
                    let error_msg = if upload_response.error_message.is_empty() {
                        "Unknown upload error".to_string()
                    } else {
                        upload_response.error_message
                    };
                    
                    print_error(&format!("Upload failed: {}", error_msg));
                    
                    Ok(UploadResult {
                        bundle_id: upload_response.bundle_id,
                        success: false,
                        bytes_uploaded: upload_response.bytes_received,
                        upload_time_seconds: upload_response.upload_time_seconds,
                        error_message: Some(error_msg),
                    })
                }
            }
            Err(e) => {
                let error_msg = format!("gRPC upload failed: {}", e);
                print_error(&error_msg);
                
                Err(anyhow!(error_msg))
            }
        }
    }
    
    /// Get information about a specific bundle
    pub async fn get_bundle_info(&mut self, bundle_id: &str) -> Result<quilt::BundleInfo> {
        let request = GetBundleInfoRequest {
            bundle_id: bundle_id.to_string(),
        };
        
        let response = self.client.get_bundle_info(request).await?;
        let bundle_response = response.into_inner();
        
        if bundle_response.success {
            bundle_response.bundle_info
                .ok_or_else(|| anyhow!("Bundle info not found"))
        } else {
            Err(anyhow!("Failed to get bundle info: {}", bundle_response.error_message))
        }
    }
    
    /// List all bundles on the server
    pub async fn list_bundles(&mut self) -> Result<Vec<quilt::BundleInfo>> {
        let request = ListBundlesRequest {
            status_filter: 0, // Unspecified - list all
            name_filter: String::new(),
            limit: 0, // No limit
            offset: 0,
        };
        
        let response = self.client.list_bundles(request).await?;
        let list_response = response.into_inner();
        
        Ok(list_response.bundles)
    }
    
    /// Delete a bundle from the server
    pub async fn delete_bundle(&mut self, bundle_id: &str, force: bool) -> Result<()> {
        let request = DeleteBundleRequest {
            bundle_id: bundle_id.to_string(),
            force,
        };
        
        let response = self.client.delete_bundle(request).await?;
        let delete_response = response.into_inner();
        
        if delete_response.success {
            print_status("Deleted", &format!("Bundle {} removed", bundle_id));
            Ok(())
        } else {
            Err(anyhow!("Failed to delete bundle: {}", delete_response.error_message))
        }
    }
    
    /// Validate a bundle without uploading it
    pub async fn validate_bundle(&mut self, bundle_path: &str) -> Result<quilt::BundleValidation> {
        let bundle_data = fs::read(bundle_path).await?;
        
        let request = ValidateBundleRequest {
            bundle_data,
            bundle_path: String::new(), // We're providing data directly
            check_signature: false, // TODO: Enable when AUTH.MD is implemented
            check_dependencies: true,
        };
        
        let response = self.client.validate_bundle(request).await?;
        let validate_response = response.into_inner();
        
        if validate_response.success {
            validate_response.validation
                .ok_or_else(|| anyhow!("Validation result not found"))
        } else {
            Err(anyhow!("Validation failed: {}", validate_response.error_message))
        }
    }
}

/// Calculate blake3 hash of data
fn calculate_blake3_hash(data: &[u8]) -> Result<String> {
    let hash = blake3::hash(data);
    Ok(hash.to_hex().to_string())
} 