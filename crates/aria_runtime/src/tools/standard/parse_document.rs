use crate::engines::llm::LLMHandler;
use crate::engines::llm::types::{LLMConfig, LLMMessage, LLMRequest};
use crate::types::ToolResult;
use crate::deep_size::DeepValue;
use crate::errors::AriaResult;
use std::collections::HashMap;
use serde_json::json;
use regex::Regex;

pub async fn parse_document_tool_handler(parameters: DeepValue, llm_handler: &LLMHandler) -> AriaResult<ToolResult> {
    // Extract parameters
    let mut params: HashMap<String, DeepValue> = HashMap::new();
    if let Some(obj) = parameters.as_object() {
        for (k, v) in obj {
            params.insert(k.clone(), DeepValue(v.clone()));
        }
    }

    let raw_content = params.get("content").and_then(|v| v.as_str());
    let alias_content = params.get("fileContent").and_then(|v| v.as_str());
    let content = raw_content.or(alias_content);
    let format = params.get("format").and_then(|v| v.as_str()).unwrap_or("text");
    let extraction_type = params.get("extractionType").and_then(|v| v.as_str()).unwrap_or("summary");

    if content.is_none() {
        return Ok(ToolResult {
            success: false,
            result: None,
            error: Some("Content (or fileContent) parameter is required".to_string()),
            metadata: HashMap::new(),
            execution_time_ms: 0,
            resource_usage: None,
        });
    }

    let content = content.unwrap();
    let start_time = std::time::Instant::now();

    println!("[PARSEDOCUMENT] Analyzing {} characters of {} content", content.len(), format);

    // Basic metadata extraction
    let metadata = json!({
        "format": format,
        "size": content.len(),
        "lineCount": content.lines().count(),
        "wordCount": content.split_whitespace().count()
    });

    // If content is very short or extraction is disabled, return basic parsing
    if content.len() < 100 || extraction_type == "none" {
        let execution_time = start_time.elapsed().as_millis() as u64;
        let summary = if content.len() > 200 {
            format!("{}...", &content[..200])
        } else {
            content.to_string()
        };

        let result = json!({
            "content": summary,
            "originalContent": content,
            "metadata": metadata,
            "summary": summary,
            "keyPoints": [],
            "details": "",
            "analysis": "Basic parsing - content too short for LLM analysis",
            "extractionType": extraction_type
        });

        return Ok(ToolResult {
            success: true,
            result: Some(result.into()),
            error: None,
            metadata: HashMap::new(),
            execution_time_ms: execution_time,
            resource_usage: None,
        });
    }

    println!("[PARSEDOCUMENT] Using LLM to extract key concepts and summary...");

    // Use LLM to intelligently parse and extract key information
    let system_prompt = "You are an expert document analyzer. Extract key concepts, principles, and information from the provided content.

Your task:
1. Create a clear, concise summary of the main topic
2. Extract the most important key points, concepts, or principles
3. Organize information in a way that would be useful for someone who needs to understand or implement the concepts

Focus on actionable information and core principles rather than metadata or peripheral details.";

    let user_prompt = format!("Analyze this content and extract the key information:

{}

Provide:
1. A clear summary (2-3 sentences)
2. A list of key points/principles (bullet points)
3. Any important details or concepts that would be needed for implementation

Format your response as:

SUMMARY:
[Your summary here]

KEY POINTS:
• [Point 1]
• [Point 2]
• [Point 3]
...

DETAILS:
[Any important implementation details or concepts]", content);

    let request = LLMRequest {
        messages: vec![
            LLMMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
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
            temperature: 0.3,
            max_tokens: 1000,
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
            let analysis_text = response.content;
            let execution_time = start_time.elapsed().as_millis() as u64;

            // Parse the LLM response to extract structured data
            let summary = extract_section(&analysis_text, "SUMMARY")
                .unwrap_or_else(|| "Summary not available".to_string());
            
            let key_points_text = extract_section(&analysis_text, "KEY POINTS")
                .unwrap_or_default();
            
            let details = extract_section(&analysis_text, "DETAILS")
                .unwrap_or_default();

            // Extract key points into array
            let key_points: Vec<String> = key_points_text
                .lines()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .map(|line| {
                    // Remove bullet point markers
                    let cleaned = line.trim_start_matches('•')
                        .trim_start_matches('-')
                        .trim_start_matches('*')
                        .trim();
                    cleaned.to_string()
                })
                .filter(|point| !point.is_empty())
                .collect();

            println!("[PARSEDOCUMENT] Extracted {} key points from {} characters", 
                     key_points.len(), content.len());

            // Create enhanced metadata
            let mut result_metadata = HashMap::new();
            result_metadata.insert("format".to_string(), DeepValue(json!(format)));
            result_metadata.insert("extraction_type".to_string(), DeepValue(json!(extraction_type)));
            result_metadata.insert("key_points_count".to_string(), DeepValue(json!(key_points.len())));
            result_metadata.insert("execution_time_ms".to_string(), DeepValue(json!(execution_time)));

            let result = json!({
                "content": summary, // Return the summary as the main content for downstream tools
                "originalContent": content,
                "metadata": metadata,
                "summary": summary,
                "keyPoints": key_points,
                "details": details,
                "analysis": analysis_text,
                "extractionType": extraction_type,
                "success": true,
                "llmAnalysis": true
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
            println!("[PARSEDOCUMENT] Analysis failed, falling back to basic parsing: {}", e);
            
            // Fallback to basic parsing if LLM fails
            let execution_time = start_time.elapsed().as_millis() as u64;
            let summary = if content.len() > 200 {
                format!("{}...", &content[..200])
            } else {
                content.to_string()
            };

            let result = json!({
                "content": content,
                "originalContent": content,
                "metadata": metadata,
                "summary": summary,
                "keyPoints": [],
                "details": "",
                "analysis": "Fallback parsing - LLM analysis failed",
                "extractionType": extraction_type,
                "fallback": true,
                "error": format!("LLM analysis failed: {}", e)
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

// Helper function to extract sections from LLM response
fn extract_section(text: &str, section_name: &str) -> Option<String> {
    let pattern = format!(r"{}:\s*(.*?)(?=\n[A-Z ]+:|$)", section_name);
    let re = Regex::new(&pattern).ok()?;
    
    re.captures(text)
        .map(|captures| captures[1].trim().to_string())
} 