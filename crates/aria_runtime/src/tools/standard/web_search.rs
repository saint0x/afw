use crate::engines::llm::LLMHandler;
use crate::types::ToolResult;
use crate::deep_size::DeepValue;
use crate::errors::AriaResult;
use std::collections::HashMap;
use serde_json::json;
use reqwest;
use std::time::Duration;

pub async fn web_search_tool_handler(parameters: DeepValue, _llm_handler: &LLMHandler) -> AriaResult<ToolResult> {
    // Extract parameters
    let mut params: HashMap<String, DeepValue> = HashMap::new();
    if let Some(obj) = parameters.as_object() {
        for (k, v) in obj {
            params.insert(k.clone(), DeepValue(v.clone()));
        }
    }

    let query = params.get("query").and_then(|v| v.as_str());
    let search_type = params.get("type").and_then(|v| v.as_str()).unwrap_or("search");
    let num_results = params.get("num_results").and_then(|v| v.as_u64()).unwrap_or(10);

    if query.is_none() {
        return Ok(ToolResult {
            success: false,
            result: None,
            error: Some("Query parameter is required".to_string()),
            metadata: HashMap::new(),
            execution_time_ms: 0,
            resource_usage: None,
        });
    }

    let query = query.unwrap();

    // Get API key from environment
    let api_key = match std::env::var("SERPER_API_KEY") {
        Ok(key) if !key.is_empty() => key,
        _ => {
            return Ok(ToolResult {
                success: false,
                result: None,
                error: Some("SERPER_API_KEY environment variable is not set".to_string()),
                metadata: HashMap::new(),
                execution_time_ms: 0,
                resource_usage: None,
            });
        }
    };

    println!("[WEBSEARCH] Searching for: \"{}\" (type: {})", query, search_type);

    let start_time = std::time::Instant::now();

    // TODO: Implement caching like TypeScript version
    let cache_key = format!("websearch:{}:{}", search_type, query);
    println!("[WEBSEARCH] Cache key: {} (caching not yet implemented)", cache_key);

    // Create HTTP client with timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| crate::errors::AriaError::new(
            crate::errors::ErrorCode::ToolExecutionError,
            crate::errors::ErrorCategory::Tool,
            crate::errors::ErrorSeverity::High,
            &format!("Failed to create HTTP client: {}", e)
        ))?;

    // Prepare request body
    let request_body = json!({
        "q": query,
        "type": search_type,
        "num": num_results
    });

    println!("[WEBSEARCH] Making API request to Serper with {} results", num_results);

    // Make the request
    let response = client
        .post("https://google.serper.dev/search")
        .header("Content-Type", "application/json")
        .header("X-API-KEY", &api_key)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| crate::errors::AriaError::new(
            crate::errors::ErrorCode::ToolExecutionError,
            crate::errors::ErrorCategory::Tool,
            crate::errors::ErrorSeverity::High,
            &format!("HTTP request failed: {}", e)
        ))?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Ok(ToolResult {
            success: false,
            result: None,
            error: Some(format!("Search request failed with status {}: {}", status, error_text)),
            metadata: HashMap::new(),
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            resource_usage: None,
        });
    }

    // Parse response
    let search_results: serde_json::Value = response.json().await
        .map_err(|e| crate::errors::AriaError::new(
            crate::errors::ErrorCode::ToolExecutionError,
            crate::errors::ErrorCategory::Tool,
            crate::errors::ErrorSeverity::Medium,
            &format!("Failed to parse response JSON: {}", e)
        ))?;

    let execution_time = start_time.elapsed().as_millis() as u64;

    println!("[WEBSEARCH] Search completed successfully in {}ms", execution_time);

    // Extract key information for summary
    let empty_vec = vec![];
    let organic_results = search_results.get("organic").and_then(|v| v.as_array()).unwrap_or(&empty_vec);
    let answer_box = search_results.get("answerBox");
    let knowledge_graph = search_results.get("knowledgeGraph");

    // Create enhanced metadata
    let mut metadata = HashMap::new();
    metadata.insert("query".to_string(), DeepValue(json!(query)));
    metadata.insert("search_type".to_string(), DeepValue(json!(search_type)));
    metadata.insert("results_count".to_string(), DeepValue(json!(organic_results.len())));
    metadata.insert("has_answer_box".to_string(), DeepValue(json!(answer_box.is_some())));
    metadata.insert("has_knowledge_graph".to_string(), DeepValue(json!(knowledge_graph.is_some())));
    metadata.insert("execution_time_ms".to_string(), DeepValue(json!(execution_time)));

    // Structure the result
    let result = json!({
        "query": query,
        "type": search_type,
        "organic": organic_results,
        "answerBox": answer_box,
        "knowledgeGraph": knowledge_graph,
        "summary": {
            "query": query,
            "totalResults": organic_results.len(),
            "hasDirectAnswer": answer_box.is_some(),
            "hasKnowledgeGraph": knowledge_graph.is_some(),
            "searchType": search_type
        },
        "metadata": {
            "source": "serper",
            "cached": false,
            "executionTime": execution_time
        }
    });

    Ok(ToolResult {
        success: true,
        result: Some(result.into()),
        error: None,
        metadata,
        execution_time_ms: execution_time,
        resource_usage: None,
    })
} 