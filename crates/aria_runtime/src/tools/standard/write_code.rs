use crate::engines::llm::LLMHandler;
use crate::engines::llm::types::{LLMConfig, LLMMessage, LLMRequest};
use crate::types::ToolResult;
use crate::deep_size::DeepValue;
use crate::errors::AriaResult;
use std::collections::HashMap;
use serde_json::json;
use tokio::fs;
use std::path::Path;

pub async fn write_code_tool_handler(parameters: DeepValue, llm_handler: &LLMHandler) -> AriaResult<ToolResult> {
    // Extract parameters
    let mut params: HashMap<String, DeepValue> = HashMap::new();
    if let Some(obj) = parameters.as_object() {
        for (k, v) in obj {
            params.insert(k.clone(), DeepValue(v.clone()));
        }
    }

    // Accept 'prompt', 'spec', 'query', or 'specification' for flexibility
    let prompt_param = params.get("prompt").and_then(|v| v.as_str());
    let spec_param = params.get("spec").and_then(|v| v.as_str());
    let query_param = params.get("query").and_then(|v| v.as_str());
    let specification_param = params.get("specification").and_then(|v| v.as_str());
    
    let mut spec = prompt_param.or(spec_param).or(query_param).or(specification_param);
    
    let context = params.get("context");
    let components = params.get("components");
    let file_path = params.get("filePath").and_then(|v| v.as_str());
    let language_override = params.get("language").and_then(|v| v.as_str());

    // Handle components parameter
    let mut components_spec = String::new();
    if let Some(components_value) = components {
        let components_str = if components_value.is_string() {
            components_value.as_str().unwrap_or("").to_string()
        } else {
            serde_json::to_string_pretty(&components_value.0).unwrap_or_else(|_| "{}".to_string())
        };
        components_spec = format!("Implement the following components: {}", components_str);
        spec = Some(&components_spec);
    }

    if spec.is_none() {
        return Ok(ToolResult {
            success: false,
            result: None,
            error: Some("Prompt, spec, query, or specification parameter is required".to_string()),
            metadata: HashMap::new(),
            execution_time_ms: 0,
            resource_usage: None,
        });
    }

    let spec = spec.unwrap();
    let start_time = std::time::Instant::now();

    // Auto-detect language from spec or filePath
    let language = if let Some(lang) = language_override {
        lang.to_string()
    } else if let Some(path) = file_path {
        detect_language_from_path(path)
    } else {
        detect_language_from_spec(spec)
    };

    println!("[WRITECODE] Generating {} code for: \"{}\"", language, spec);

    // Prepare context string
    let context_str = if let Some(ctx) = context {
        if ctx.is_string() {
            ctx.as_str().unwrap_or("").to_string()
        } else {
            serde_json::to_string_pretty(&ctx.0).unwrap_or_else(|_| "{}".to_string())
        }
    } else {
        "{}".to_string()
    };

    // Generate code using LLM
    let system_prompt = format!("You are an expert {} developer. Generate clean, production-ready code with proper documentation and error handling.", language);
    
    let user_prompt = format!("Generate {} code for: {}

Context: {}

Requirements:
- Write clean, readable code
- Include proper error handling
- Add comments for complex logic
- Follow {} best practices
- Make it production-ready
- IMPORTANT: Generate ONLY the code, no markdown formatting or code blocks

Provide working {} code that can be immediately used.", language, spec, context_str, language, language);

    let request = LLMRequest {
        messages: vec![
            LLMMessage {
                role: "system".to_string(),
                content: system_prompt,
                tool_calls: None,
                tool_call_id: None,
            },
            LLMMessage {
                role: "user".to_string(),
                content: user_prompt,
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        config: LLMConfig {
            model: Some("gpt-4".to_string()),
            temperature: 0.3, // Lower temperature for more consistent code
            max_tokens: 1500,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
        },
        provider: Some("openai".to_string()),
        tools: None,
        tool_choice: None,
        stream: Some(false),
    };

    match llm_handler.complete(request).await {
        Ok(response) => {
            let mut generated_code = response.content.trim().to_string();
            
            // Clean up any markdown code blocks that might have been generated
            generated_code = clean_markdown_code_blocks(generated_code);
            
            println!("[WRITECODE] Generated {} characters of {} code", generated_code.len(), language);

            // Save to file if filePath is provided
            let saved_to_file = if let Some(path) = file_path {
                match save_code_to_file(path, &generated_code).await {
                    Ok(()) => {
                        println!("[WRITECODE] Saved code to {}", path);
                        true
                    }
                    Err(e) => {
                        println!("[WRITECODE] Failed to save code to {}: {}", path, e);
                        false
                    }
                }
            } else {
                false
            };

            // Generate explanation
            let explanation = generate_code_explanation(&generated_code, &language, llm_handler).await
                .unwrap_or_else(|| "Code explanation not available".to_string());

            let execution_time = start_time.elapsed().as_millis() as u64;

            // Create enhanced metadata
            let mut result_metadata = HashMap::new();
            result_metadata.insert("language".to_string(), DeepValue(json!(language)));
            result_metadata.insert("code_length".to_string(), DeepValue(json!(generated_code.len())));
            result_metadata.insert("saved_to_file".to_string(), DeepValue(json!(saved_to_file)));
            result_metadata.insert("execution_time_ms".to_string(), DeepValue(json!(execution_time)));

            let result = json!({
                "code": generated_code,
                "explanation": explanation,
                "language": language,
                "context": context_str,
                "spec": spec,
                "codeLength": generated_code.len(),
                "filePath": file_path,
                "savedToFile": saved_to_file,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "model": "LLM-generated",
                "success": true
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
            let execution_time = start_time.elapsed().as_millis() as u64;
            println!("[WRITECODE] LLM call failed: {}", e);

            Ok(ToolResult {
                success: false,
                result: None,
                error: Some(format!("Code generation failed: {}", e)),
                metadata: HashMap::new(),
                execution_time_ms: execution_time,
                resource_usage: None,
            })
        }
    }
}

// Helper function to detect language from file path
fn detect_language_from_path(file_path: &str) -> String {
    let path = Path::new(file_path);
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase());

    match extension.as_deref() {
        Some("rs") => "rust".to_string(),
        Some("js") => "javascript".to_string(),
        Some("ts") => "typescript".to_string(),
        Some("py") => "python".to_string(),
        Some("go") => "go".to_string(),
        Some("java") => "java".to_string(),
        Some("cpp") | Some("cc") | Some("cxx") => "cpp".to_string(),
        Some("c") => "c".to_string(),
        Some("cs") => "csharp".to_string(),
        Some("rb") => "ruby".to_string(),
        Some("php") => "php".to_string(),
        Some("sh") => "bash".to_string(),
        _ => "javascript".to_string(), // default
    }
}

// Helper function to detect language from spec content
fn detect_language_from_spec(spec: &str) -> String {
    let spec_lower = spec.to_lowercase();
    
    if spec_lower.contains("rust") || spec_lower.contains("cargo") {
        "rust".to_string()
    } else if spec_lower.contains("typescript") {
        "typescript".to_string()
    } else if spec_lower.contains("python") {
        "python".to_string()
    } else if spec_lower.contains("java") {
        "java".to_string()
    } else if spec_lower.contains("golang") || spec_lower.contains("go ") {
        "go".to_string()
    } else if spec_lower.contains("c++") || spec_lower.contains("cpp") {
        "cpp".to_string()
    } else {
        "javascript".to_string() // default
    }
}

// Helper function to clean markdown code blocks
fn clean_markdown_code_blocks(code: String) -> String {
    let lines: Vec<&str> = code.lines().collect();
    
    // Check if wrapped in markdown code blocks
    if lines.len() >= 2 && lines[0].starts_with("```") {
        let mut result_lines = lines;
        
        // Remove first line if it's a code block start
        if result_lines[0].starts_with("```") {
            result_lines.remove(0);
        }
        
        // Remove last line if it's a code block end
        if !result_lines.is_empty() && result_lines[result_lines.len() - 1].trim() == "```" {
            result_lines.pop();
        }
        
        result_lines.join("\n")
    } else {
        code
    }
}

// Helper function to save code to file
async fn save_code_to_file(file_path: &str, code: &str) -> Result<(), std::io::Error> {
    let path = Path::new(file_path);
    
    // Create directory if it doesn't exist
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).await?;
    }
    
    // Write file
    fs::write(file_path, code).await
}

// Helper function to generate code explanation
async fn generate_code_explanation(code: &str, language: &str, llm_handler: &LLMHandler) -> Option<String> {
    let request = LLMRequest {
        messages: vec![
            LLMMessage {
                role: "system".to_string(),
                content: "You are a code documentation expert. Explain code implementations clearly and concisely.".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            LLMMessage {
                role: "user".to_string(),
                content: format!("Explain this {} code implementation:\n\n{}\n\nProvide a brief explanation of:\n1. What the code does\n2. Key components and their roles\n3. Any important implementation details", language, code),
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        config: LLMConfig {
            model: Some("gpt-4".to_string()),
            temperature: 0.5,
            max_tokens: 500,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
        },
        provider: Some("openai".to_string()),
        tools: None,
        tool_choice: None,
        stream: Some(false),
    };

    match llm_handler.complete(request).await {
        Ok(response) => Some(response.content),
        Err(_) => None,
    }
} 