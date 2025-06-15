use crate::deep_size::DeepValue;
use crate::engines::llm::types::{LLMConfig, LLMMessage, LLMRequest};
use crate::engines::llm::LLMHandler;
use crate::errors::{AriaError, AriaResult, ErrorCategory, ErrorCode, ErrorSeverity};
use crate::types::{self, ResourceRequirements, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[async_trait]
pub trait ToolRegistryInterface: Send + Sync {
    async fn execute_tool(&self, name: &str, parameters: DeepValue) -> AriaResult<ToolResult>;
    async fn get_tool_info(&self, name: &str) -> AriaResult<Option<types::RegistryEntry>>;
    async fn list_available_tools(&self) -> AriaResult<Vec<String>>;
    async fn is_tool_available(&self, tool_name: &str) -> bool;
}

#[async_trait]
pub trait BundleStoreInterface: Send + Sync {
    async fn load_bundle(&self, path: &str) -> AriaResult<BundleManifest>;
}

#[derive(Debug, Clone)]
pub struct BundleManifest {
    pub id: String,
    pub version: String,
    pub tools: Vec<ToolManifest>,
}

#[derive(Debug, Clone)]
pub struct ToolManifest {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub entry_point: String,
    pub capabilities: Vec<String>,
    pub resource_requirements: ResourceRequirements,
    pub security_level: SecurityLevel,
}

#[derive(Clone)]
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, RegistryEntry>>>,
    execution_stats: Arc<RwLock<HashMap<String, ToolExecutionStats>>>,
    bundle_store: Option<Arc<dyn BundleStoreInterface>>,
    llm_handler: Arc<LLMHandler>,
}

#[derive(Debug, Clone)]
pub struct RegistryEntry {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub tool_type: ToolType,
    pub bundle_id: Option<String>,
    pub version: String,
    pub capabilities: Vec<String>,
    pub resource_requirements: ResourceRequirements,
    pub security_level: SecurityLevel,
}

#[derive(Debug, Clone)]
pub enum ToolType {
    Builtin,
    Bundle {
        bundle_path: String,
        entry_point: String,
    },
    Container {
        image: String,
        command: Vec<String>,
    },
    LLM {
        provider: String,
        model: String,
    },
}

#[derive(Debug, Clone)]
pub struct ToolExecutionStats {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub average_execution_time_ms: f64,
    pub last_execution: Option<std::time::SystemTime>,
    pub error_rate: f64,
}

#[derive(Debug, Clone)]
pub enum SecurityLevel {
    Safe,
    Limited,
    Elevated,
    Dangerous,
}

impl ToolRegistry {
    pub fn new(
        llm_handler: Arc<LLMHandler>,
    ) -> Self {
        let registry = Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            execution_stats: Arc::new(RwLock::new(HashMap::new())),
            bundle_store: None,
            llm_handler,
        };
        let tools_arc = registry.tools.clone();
        tokio::spawn(async move {
            let builtin_tools = vec![
                Self::create_ponder_tool_static(),
                Self::create_create_plan_tool_static(),
            ];
            let mut tools = tools_arc.write().await;
            for tool in builtin_tools {
                tools.insert(tool.name.clone(), tool);
            }
        });
        registry
    }

    fn create_ponder_tool_static() -> RegistryEntry {
        RegistryEntry {
            name: "ponderTool".to_string(),
            description: "Analyzes situations, reflects on outcomes, and provides strategic insights".to_string(),
            parameters: serde_json::json!({ "type": "object", "properties": { "query": { "type": "string" } }, "required": ["query"] }),
            tool_type: ToolType::LLM { provider: "openai".to_string(), model: "gpt-4".to_string() },
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["reflection".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Safe,
        }
    }

    fn create_create_plan_tool_static() -> RegistryEntry {
        RegistryEntry {
            name: "createPlanTool".to_string(),
            description: "Creates detailed execution plans for complex multi-step tasks".to_string(),
            parameters: serde_json::json!({ "type": "object", "properties": { "objective": { "type": "string" } }, "required": ["objective"] }),
            tool_type: ToolType::LLM { provider: "openai".to_string(), model: "gpt-4".to_string() },
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["planning".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Safe,
        }
    }
}

#[async_trait]
impl ToolRegistryInterface for ToolRegistry {
    async fn execute_tool(&self, name: &str, parameters: DeepValue) -> AriaResult<ToolResult> {
        let tool_entry = self.tools.read().await.get(name).cloned().ok_or_else(|| {
            AriaError::new(
                ErrorCode::ToolNotFound,
                ErrorCategory::Tool,
                ErrorSeverity::Medium,
                &format!("Tool '{}' not found in registry", name),
            )
        })?;

        match &tool_entry.tool_type {
            ToolType::LLM { provider, model } => {
                let mut llm_params = HashMap::new();
                if let Some(obj) = parameters.as_object() {
                    for (k,v) in obj {
                        llm_params.insert(k.clone(), v.clone());
                    }
                }

                let request = LLMRequest {
                    messages: vec![LLMMessage { role: "user".to_string(), content: llm_params.get("query").unwrap().to_string(), tool_calls: None, tool_call_id: None }],
                    config: LLMConfig::default(),
                    provider: Some(provider.clone()),
                    tools: None,
                    tool_choice: None,
                    stream: Some(false),
                };
                let response = self.llm_handler.complete(request).await?;
                let result_val: Value = serde_json::from_str(&response.content)
                    .map_err(|e| AriaError::new(
                        ErrorCode::LLMInvalidResponse,
                        ErrorCategory::LLM,
                        ErrorSeverity::Medium,
                        &format!("Failed to parse tool result: {}", e)
                    ))?;
                Ok(ToolResult {
                    success: true,
                    result: Some(result_val.into()),
                    ..Default::default()
                })
            }
            _ => Err(AriaError::new(
                ErrorCode::NotSupported,
                ErrorCategory::Tool,
                ErrorSeverity::Medium,
                "Only LLM tools are supported in this version",
            )),
        }
    }

    async fn get_tool_info(&self, name: &str) -> AriaResult<Option<types::RegistryEntry>> {
        Ok(self.tools.read().await.get(name).map(|entry| types::RegistryEntry {
            name: entry.name.clone(),
            entry_type: types::RegistryEntryType::Tool,
            bundle_id: entry.bundle_id.clone(),
            version: entry.version.clone(),
            metadata: HashMap::new(),
            created_at: 0,
            updated_at: 0,
        }))
    }

    async fn list_available_tools(&self) -> AriaResult<Vec<String>> {
        Ok(self.tools.read().await.keys().cloned().collect())
    }

    async fn is_tool_available(&self, tool_name: &str) -> bool {
        self.tools.read().await.contains_key(tool_name)
    }
} 