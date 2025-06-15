use crate::engines::llm::LLMHandler;
use crate::types::ToolResult;
use crate::deep_size::DeepValue;
use crate::errors::AriaResult;
use std::collections::HashMap;
use serde_json::json;
use tokio::fs;
use std::path::Path;

pub async fn write_file_tool_handler(parameters: DeepValue, _llm_handler: &LLMHandler) -> AriaResult<ToolResult> {
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
    let content = params.get("content").and_then(|v| v.as_str());
    let encoding = params.get("encoding").and_then(|v| v.as_str()).unwrap_or("utf-8");

    if file_path.is_none() || content.is_none() {
        return Ok(ToolResult {
            success: false,
            result: None,
            error: Some("Path (or filePath) and content parameters are required".to_string()),
            metadata: HashMap::new(),
            execution_time_ms: 0,
            resource_usage: None,
        });
    }

    let file_path = file_path.unwrap();
    let content = content.unwrap();

    println!("[WRITEFILE] Writing {} bytes to {}", content.len(), file_path);

    let start_time = std::time::Instant::now();

    // Ensure directory exists
    let path = Path::new(file_path);
    if let Some(dir_path) = path.parent() {
        if let Err(e) = fs::create_dir_all(dir_path).await {
            return Ok(ToolResult {
                success: false,
                result: None,
                error: Some(format!("Failed to create directory {}: {}", dir_path.display(), e)),
                metadata: HashMap::new(),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                resource_usage: None,
            });
        }
    }

    // Write file
    match fs::write(file_path, content).await {
        Ok(()) => {
            // Get file stats
            match fs::metadata(file_path).await {
                Ok(metadata) => {
                    let file_size = metadata.len();
                    let execution_time = start_time.elapsed().as_millis() as u64;

                    println!("[WRITEFILE] Successfully wrote {} bytes to {}", file_size, file_path);

                    // Create result metadata
                    let mut result_metadata = HashMap::new();
                    result_metadata.insert("path".to_string(), DeepValue(json!(file_path)));
                    result_metadata.insert("size".to_string(), DeepValue(json!(file_size)));
                    result_metadata.insert("encoding".to_string(), DeepValue(json!(encoding)));
                    result_metadata.insert("execution_time_ms".to_string(), DeepValue(json!(execution_time)));

                    let result = json!({
                        "path": file_path,
                        "size": file_size,
                        "encoding": encoding,
                        "message": format!("File written successfully to {}", file_path),
                        "success": true,
                        "metadata": {
                            "fileSize": file_size,
                            "encoding": encoding,
                            "created": true,
                            "executionTime": execution_time
                        }
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
                    // File was written but couldn't get stats
                    let execution_time = start_time.elapsed().as_millis() as u64;
                    println!("[WRITEFILE] File written but failed to get metadata: {}", e);

                    let result = json!({
                        "path": file_path,
                        "size": content.len(),
                        "encoding": encoding,
                        "message": format!("File written successfully to {} (metadata unavailable)", file_path),
                        "success": true,
                        "warning": "Could not retrieve file metadata"
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
            println!("[WRITEFILE] Failed to write file: {}", e);

            Ok(ToolResult {
                success: false,
                result: None,
                error: Some(format!("Failed to write file {}: {}", file_path, e)),
                metadata: HashMap::new(),
                execution_time_ms: execution_time,
                resource_usage: None,
            })
        }
    }
} 