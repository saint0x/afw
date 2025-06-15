use crate::engines::llm::LLMHandler;
use crate::engines::llm::types::{LLMConfig, LLMMessage, LLMRequest};
use crate::types::ToolResult;
use crate::deep_size::DeepValue;
use crate::errors::AriaResult;
use std::collections::HashMap;
use serde_json::{json, Map};

pub async fn create_plan_tool_handler(parameters: DeepValue, llm_handler: &LLMHandler) -> AriaResult<ToolResult> {
    // Accept both 'objective' and 'query' for flexibility
    let mut params = HashMap::new();
    if let Some(obj) = parameters.as_object() {
        for (k, v) in obj {
            params.insert(k.clone(), v.clone());
        }
    }

    let objective = params.get("objective")
        .and_then(|v| v.as_str())
        .or_else(|| params.get("query").and_then(|v| v.as_str()));
    
    // Create default empty maps to avoid temporary value issues
    let empty_constraints = Map::new();
    let empty_context = Map::new();
    
    let constraints = params.get("constraints")
        .and_then(|v| v.as_object())
        .unwrap_or(&empty_constraints);
    
    let context = params.get("context")
        .and_then(|v| v.as_object())
        .unwrap_or(&empty_context);

    if objective.is_none() {
        return Ok(ToolResult {
            success: false,
            result: None,
            error: Some("Objective or query parameter is required".to_string()),
            metadata: HashMap::new(),
            execution_time_ms: 0,
            resource_usage: None,
        });
    }

    let objective_str = objective.unwrap();
    
    println!("[CREATEPLAN] Making real LLM call to generate plan...");

    // REAL LLM CALL - Generate actual plan using AI
    let system_prompt = r#"You are a meticulous project planning assistant. Your sole purpose is to create a JSON execution plan.
Given an objective and a list of available tools, you MUST generate a JSON array of plan steps.
Each object in the array represents a single, concrete step.

**RULES:**
1.  **Tool-Centric:** Every single step MUST map directly to one of the provided tools, or be a non-tool step for reasoning or summarization.
2.  **JSON ONLY:** Your entire output must be a single, raw JSON array. Do not include any text, markdown, or explanations before or after the JSON.
3.  **Data Flow:** The 'parameters' for a step can and should reference the output of a previous step. Use a placeholder string like '{{step_1_output}}' to indicate this.
4.  **Complete Parameters:** Include ALL required parameters for each tool. For writeCode, include 'language' and 'filePath' when saving code to files.
5.  **File Operations:** When the objective mentions saving files to specific paths, ensure the filePath parameter is included.
6.  **Strict Schema:** Each JSON object in the array must have these exact keys:
    - "step": A number representing the order of execution.
    - "useTool": A boolean (true/false) indicating if a tool is being called.
    - "tool": If useTool is true, the exact name of the tool. If useTool is false, this MUST be "none".
    - "description": A concise description of what this step achieves.
    - "parameters": If useTool is true, a JSON object of the parameters for the tool. If useTool is false, this can be an empty object.

**Tool Parameter Guidelines:**
- webSearch: Use "query" parameter
- writeFile: Use "filePath" and "content" parameters  
- readFile: Use "filePath" parameter
- parseDocument: Use "content" parameter
- writeCode: Use "spec" parameter, and include "language" and "filePath" when saving code
- ponder: Use "query" parameter
- calculator: Use "operation" and "expression" parameters
- text_analyzer: Use "text" parameter
- file_writer: Use "filename" and "content" parameters
- data_formatter: Use "data" parameter

**Example:**
[
  {
    "step": 1,
    "useTool": true,
    "tool": "calculator",
    "description": "Calculate the area of a circle with radius 7.5.",
    "parameters": {
      "operation": "area_circle",
      "radius": 7.5
    }
  },
  {
    "step": 2,
    "useTool": true,
    "tool": "calculator",
    "description": "Calculate the circumference of a circle with radius 7.5.",
    "parameters": {
      "operation": "circumference_circle", 
      "radius": 7.5
    }
  },
  {
    "step": 3,
    "useTool": true,
    "tool": "text_analyzer",
    "description": "Analyze the relationship between area and circumference values.",
    "parameters": {
      "text": "Area: {{step_1_output}}, Circumference: {{step_2_output}}"
    }
  },
  {
    "step": 4,
    "useTool": true,
    "tool": "file_writer",
    "description": "Save the complete mathematical analysis to a file.",
    "parameters": {
      "filename": "circle_analysis.txt",
      "content": "Circle Analysis Report\n\nArea: {{step_1_output}}\nCircumference: {{step_2_output}}\nAnalysis: {{step_3_output}}"
    }
  }
]"#;

    let available_tools = context.get("availableTools")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec!["calculator", "text_analyzer", "file_writer", "data_formatter"]);

    let user_content = format!(
        "Create a JSON execution plan for the objective: \"{}\"\n\nAvailable Tools: {:?}\nConstraints: {}\nContext: {}",
        objective_str,
        available_tools,
        serde_json::to_string(&constraints).unwrap_or_else(|_| "{}".to_string()),
        serde_json::to_string(&context).unwrap_or_else(|_| "{}".to_string())
    );

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
                content: user_content,
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        config: LLMConfig {
            model: Some("gpt-4".to_string()),
            temperature: 0.1,
            max_tokens: 2048,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
        },
        provider: Some("openai".to_string()),
        tools: None,
        tool_choice: None,
        stream: Some(false),
    };

    let response = llm_handler.complete(request).await?;
    let plan_content = response.content;

    println!("[CREATEPLAN] Generated {} character plan", plan_content.len());

    // Structure the response exactly like TypeScript version
    let plan = json!({
        "objective": objective_str,
        "constraints": constraints,
        "context": context,
        "generatedPlan": plan_content,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "model": "LLM-generated",
        "planLength": plan_content.len()
    });

    Ok(ToolResult {
        success: true,
        result: Some(json!({ "plan": plan }).into()),
        error: None,
        metadata: HashMap::new(),
        execution_time_ms: 0,
        resource_usage: None,
    })
} 