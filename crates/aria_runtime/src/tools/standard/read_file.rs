use crate::engines::llm::LLMHandler;
use crate::types::ToolResult;
use crate::deep_size::DeepValue;
use crate::errors::AriaResult;
use std::collections::HashMap;
use serde_json::json;
use tokio::fs;
use std::path::Path;

pub async fn read_file_tool_handler(parameters: DeepValue, _llm_handler: &LLMHandler) -> AriaResult<ToolResult> {
    // Extract parameters
    let mut params: HashMap<String, DeepValue> = HashMap::new();
    if let Some(obj) = parameters.as_object() {
        for (k, v) in obj {
            params.insert(k.clone(), DeepValue(v.clone()));
        }
    }

    let legacy_path = params.get("path").and_then(|v| v.as_str());
    let new_path = params.get("filePath").and_then(|v| v.as_str());
    let file_path = new_path.or(legacy_path);
    let format_override = params.get("format").and_then(|v| v.as_str());

    if file_path.is_none() {
        return Ok(ToolResult {
            success: false,
            result: None,
            error: Some("Path (or filePath) parameter is required".to_string()),
            metadata: HashMap::new(),
            execution_time_ms: 0,
            resource_usage: None,
        });
    }

    let file_path = file_path.unwrap();

    println!("[READFILE] Reading file: {}", file_path);

    let start_time = std::time::Instant::now();

    // Read file content
    match fs::read_to_string(file_path).await {
        Ok(content) => {
            // Get file metadata
            match fs::metadata(file_path).await {
                Ok(file_metadata) => {
                    let file_size = file_metadata.len();
                    let execution_time = start_time.elapsed().as_millis() as u64;

                    // Auto-detect format from file extension or use override
                    let detected_format = if let Some(format) = format_override {
                        format.to_string()
                    } else {
                        Path::new(file_path)
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| ext.to_lowercase())
                            .unwrap_or_else(|| "text".to_string())
                    };

                    println!("[READFILE] Successfully read {} bytes from {} (format: {})", 
                             file_size, file_path, detected_format);

                    // Create metadata structure
                    let metadata_obj = json!({
                        "format": detected_format,
                        "size": file_size,
                        "path": file_path,
                        "type": "file",
                        "encoding": "utf-8",
                        "lines": content.lines().count(),
                        "characters": content.len(),
                        "executionTime": execution_time
                    });

                    // Create result metadata for ToolResult
                    let mut result_metadata = HashMap::new();
                    result_metadata.insert("path".to_string(), DeepValue(json!(file_path)));
                    result_metadata.insert("size".to_string(), DeepValue(json!(file_size)));
                    result_metadata.insert("format".to_string(), DeepValue(json!(detected_format)));
                    result_metadata.insert("execution_time_ms".to_string(), DeepValue(json!(execution_time)));

                    let result = json!({
                        "content": content,
                        "metadata": metadata_obj,
                        "success": true,
                        "filePath": file_path,
                        "fileSize": file_size,
                        "format": detected_format
                    });

                    Ok(ToolResult {
                        success: true,
                        result: Some(result.into()),
                        error: None,
                        metadata: result_metadata,
                        execution_time_ms: execution_time,
                        resource_usage: None,
                    })
                }
                Err(e) => {
                    // File read but couldn't get metadata
                    let execution_time = start_time.elapsed().as_millis() as u64;
                    let detected_format = format_override.unwrap_or("text").to_string();

                    println!("[READFILE] File read but failed to get metadata: {}", e);

                    let metadata_obj = json!({
                        "format": detected_format,
                        "size": content.len(),
                        "path": file_path,
                        "type": "file",
                        "encoding": "utf-8",
                        "lines": content.lines().count(),
                        "characters": content.len(),
                        "warning": "File metadata unavailable"
                    });

                    let result = json!({
                        "content": content,
                        "metadata": metadata_obj,
                        "success": true,
                        "warning": "Could not retrieve file system metadata"
                    });

                    Ok(ToolResult {
                        success: true,
                        result: Some(result.into()),
                        error: None,
                        metadata: HashMap::new(),
                        execution_time_ms: execution_time,
                        resource_usage: None,
                    })
                }
            }
        }
        Err(e) => {
            let execution_time = start_time.elapsed().as_millis() as u64;
            println!("[READFILE] Failed to read file: {}", e);

            Ok(ToolResult {
                success: false,
                result: None,
                error: Some(format!("Failed to read file {}: {}", file_path, e)),
                metadata: HashMap::new(),
                execution_time_ms: execution_time,
                resource_usage: None,
            })
        }
    }
} 