use crate::errors::AriaResult;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub metrics: ExecutionMetrics,
}

// Simplified tool representation for dyn compatibility
#[derive(Debug, Clone)]
pub enum Tool {
    WebSearch { description: String },
    FileRead { description: String },
    FileWrite { description: String },
    Ponder { description: String },
    Custom { name: String, description: String },
}

impl Tool {
    pub fn name(&self) -> &str {
        match self {
            Tool::WebSearch { .. } => "webSearch",
            Tool::FileRead { .. } => "readFile",
            Tool::FileWrite { .. } => "writeFile",
            Tool::Ponder { .. } => "ponder",
            Tool::Custom { name, .. } => name,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Tool::WebSearch { description } => description,
            Tool::FileRead { description } => description,
            Tool::FileWrite { description } => description,
            Tool::Ponder { description } => description,
            Tool::Custom { description, .. } => description,
        }
    }

    pub async fn execute(&self, params: serde_json::Value) -> AriaResult<ToolResult> {
        // Simplified execution - just return success for scaffolding
        Ok(ToolResult {
            success: true,
            result: Some(serde_json::json!({
                "tool": self.name(),
                "input": params,
                "output": "Tool execution placeholder"
            })),
            error: None,
            metrics: ExecutionMetrics::default(),
        })
    }
}

pub struct ToolRegistry {
    tools: HashMap<String, Tool>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register_tool(&mut self, tool: Tool) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub async fn execute_tool(&self, name: &str, params: serde_json::Value) -> AriaResult<ToolResult> {
        match self.tools.get(name) {
            Some(tool) => tool.execute(params).await,
            None => Ok(ToolResult {
                success: false,
                result: None,
                error: Some(format!("Tool '{}' not found", name)),
                metrics: ExecutionMetrics::default(),
            }),
        }
    }

    pub fn get_available_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    pub fn get_tool_info(&self, name: &str) -> Option<&Tool> {
        self.tools.get(name)
    }
} 