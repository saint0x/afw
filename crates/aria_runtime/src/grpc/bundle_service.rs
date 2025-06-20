use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status, Streaming};
use futures_util::StreamExt;

use super::aria::{
    bundle_service_server::BundleService,
    UploadBundleRequest, UploadBundleResponse, BundleMetadata,
};

use crate::engines::container::quilt::QuiltService;
use crate::engines::container::quilt::quilt_proto;
use crate::engines::tool_registry::{ToolRegistry, ToolRegistryInterface};
use crate::database::DatabaseManager;
use crate::errors::AriaResult;

/// Implementation of the high-level BundleService
/// This service acts as a proxy to the lower-level Quilt bundle management
pub struct BundleServiceImpl {
    quilt_service: Arc<Mutex<QuiltService>>,
}

impl BundleServiceImpl {
    pub fn new(quilt_service: Arc<Mutex<QuiltService>>) -> Self {
        Self { quilt_service }
    }
}

#[tonic::async_trait]
impl BundleService for BundleServiceImpl {
    async fn upload_bundle(
        &self,
        request: Request<Streaming<UploadBundleRequest>>,
    ) -> Result<Response<UploadBundleResponse>, Status> {
        let mut stream = request.into_inner();
        
        tracing::info!("Starting bundle upload stream");
        
        // First message should contain metadata
        let first_message = match stream.next().await {
            Some(Ok(msg)) => msg,
            Some(Err(e)) => {
                tracing::error!("Error reading first message: {}", e);
                return Err(Status::invalid_argument("Failed to read upload stream"));
            }
            None => {
                return Err(Status::invalid_argument("Empty upload stream"));
            }
        };
        
        let metadata = match first_message.payload {
            Some(super::aria::upload_bundle_request::Payload::Metadata(meta)) => meta,
            Some(super::aria::upload_bundle_request::Payload::Chunk(_)) => {
                return Err(Status::invalid_argument("First message must contain metadata"));
            }
            None => {
                return Err(Status::invalid_argument("First message must contain metadata"));
            }
        };
        
        tracing::info!("Received bundle metadata: name={}, size={} bytes", 
                      metadata.name, metadata.total_size_bytes);
        
        // Collect all chunks
        let mut bundle_data = Vec::with_capacity(metadata.total_size_bytes as usize);
        
        while let Some(message) = stream.next().await {
            match message {
                Ok(msg) => {
                    match msg.payload {
                        Some(super::aria::upload_bundle_request::Payload::Chunk(chunk)) => {
                            bundle_data.extend_from_slice(&chunk);
                            
                            // Progress logging
                            if bundle_data.len() % (1024 * 1024) == 0 { // Every MB
                                tracing::debug!("Received {} MB of bundle data", bundle_data.len() / (1024 * 1024));
                            }
                        }
                        Some(super::aria::upload_bundle_request::Payload::Metadata(_)) => {
                            return Err(Status::invalid_argument("Metadata can only be sent in first message"));
                        }
                        None => {
                            tracing::warn!("Received empty message in upload stream");
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Error reading chunk: {}", e);
                    return Err(Status::internal("Failed to read upload stream"));
                }
            }
        }
        
        // Verify size matches metadata
        if bundle_data.len() != metadata.total_size_bytes as usize {
            return Err(Status::invalid_argument(format!(
                "Bundle size mismatch: expected {} bytes, received {} bytes",
                metadata.total_size_bytes,
                bundle_data.len()
            )));
        }
        
        tracing::info!("Received complete bundle: {} bytes", bundle_data.len());
        
        // Now upload to Quilt via the existing gRPC streaming interface
        let mut quilt_service = self.quilt_service.lock().await;
        
        // Create a stream for Quilt upload
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        
        // Create bundle metadata for Quilt
        let quilt_metadata = quilt_proto::BundleMetadata {
            name: metadata.name.clone(),
            version: "1.0.0".to_string(), // Default version if not specified
            description: "Bundle uploaded via Aria Runtime API".to_string(),
            author: "aria-runtime".to_string(),
            tags: std::collections::HashMap::new(),
            total_size: bundle_data.len() as u64,
            checksum: format!("{:x}", md5::compute(&bundle_data)),
            dependencies: std::collections::HashMap::new(),
            entry_point: "".to_string(),
        };
        
        // Send metadata first
        let metadata_request = quilt_proto::UploadBundleRequest {
            payload: Some(quilt_proto::upload_bundle_request::Payload::Metadata(quilt_metadata)),
        };
        
        if let Err(_) = tx.send(Ok(metadata_request)).await {
            return Err(Status::internal("Failed to send metadata to Quilt"));
        }
        
        // Send data in chunks
        let chunk_size = 64 * 1024; // 64KB chunks
        for chunk in bundle_data.chunks(chunk_size) {
            let chunk_request = quilt_proto::UploadBundleRequest {
                payload: Some(quilt_proto::upload_bundle_request::Payload::Chunk(chunk.to_vec())),
            };
            
            if let Err(_) = tx.send(Ok(chunk_request)).await {
                return Err(Status::internal("Failed to send chunk to Quilt"));
            }
        }
        
        // Close the stream
        drop(tx);
        
        // Convert the receiver to a stream for Quilt
        let upload_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        
        // Upload to Quilt
        match quilt_service.upload_bundle(upload_stream).await {
            Ok(result) => {
                
                if result.success {
                    tracing::info!("Bundle uploaded successfully: bundle_id={}", result.bundle_id);
                    
                    // Register the bundle's tools with our tool registry
                    // This is critical for making the bundle's tools available
                    if let Some(bundle_info) = result.bundle_info {
                        if let Some(manifest) = bundle_info.manifest {
                            tracing::info!("Registering {} tools from bundle {}", 
                                         manifest.tools.len(), result.bundle_id);
                            
                            // TODO: Integrate with ToolRegistry to register the tools
                            // This would call something like:
                            // tool_registry.register_tools_from_bundle(&result.bundle_id).await?;
                        }
                    }
                    
                    Ok(Response::new(UploadBundleResponse {
                        bundle_id: result.bundle_id,
                        success: true,
                        error_message: None,
                    }))
                } else {
                    tracing::error!("Bundle upload failed: {}", result.error_message);
                    Ok(Response::new(UploadBundleResponse {
                        bundle_id: "".to_string(),
                        success: false,
                        error_message: Some(result.error_message),
                    }))
                }
            }
            Err(e) => {
                tracing::error!("Failed to upload bundle to Quilt: {}", e);
                Err(Status::internal(format!("Bundle upload failed: {}", e)))
            }
        }
    }
} 