use crate::deep_size::DeepValue;
use crate::engines::llm::types::{LLMConfig, LLMMessage, LLMRequest};
use crate::engines::llm::LLMHandler;
use crate::errors::{AriaError, AriaResult, ErrorCategory, ErrorCode, ErrorSeverity};
use crate::types::{self, ResourceRequirements, ToolResult};
use crate::tools::standard::{
    create_plan_tool_handler, 
    ponder_tool_handler,
    web_search_tool_handler,
    write_file_tool_handler,
    read_file_tool_handler,
    parse_document_tool_handler,
    write_code_tool_handler
};
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
                Self::create_web_search_tool_static(),
                Self::create_write_file_tool_static(),
                Self::create_read_file_tool_static(),
                Self::create_parse_document_tool_static(),
                Self::create_write_code_tool_static(),
                Self::create_calculator_tool_static(),
                Self::create_text_analyzer_tool_static(),
                Self::create_file_writer_tool_static(),
                Self::create_data_formatter_tool_static(),
            ];
            let mut tools = tools_arc.write().await;
            for tool in builtin_tools {
                println!("🔧 Registering builtin tool: {}", tool.name);
                tools.insert(tool.name.clone(), tool);
            }
            println!("✅ Registered {} builtin tools", tools.len());
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

    fn create_calculator_tool_static() -> RegistryEntry {
        RegistryEntry {
            name: "calculator".to_string(),
            description: "Performs mathematical calculations including basic arithmetic, geometry, and advanced functions".to_string(),
            parameters: serde_json::json!({ 
                "type": "object", 
                "properties": { 
                    "operation": { "type": "string", "description": "The mathematical operation to perform" },
                    "expression": { "type": "string", "description": "The mathematical expression to calculate" }
                }, 
                "required": ["operation"] 
            }),
            tool_type: ToolType::LLM { provider: "openai".to_string(), model: "gpt-4".to_string() },
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["mathematics".to_string(), "calculation".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Safe,
        }
    }

    fn create_text_analyzer_tool_static() -> RegistryEntry {
        RegistryEntry {
            name: "text_analyzer".to_string(),
            description: "Analyzes text for patterns, relationships, insights, and extracts meaningful information".to_string(),
            parameters: serde_json::json!({ 
                "type": "object", 
                "properties": { 
                    "text": { "type": "string", "description": "The text to analyze" },
                    "analysis_type": { "type": "string", "description": "Type of analysis to perform" }
                }, 
                "required": ["text"] 
            }),
            tool_type: ToolType::LLM { provider: "openai".to_string(), model: "gpt-4".to_string() },
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["analysis".to_string(), "text_processing".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Safe,
        }
    }

    fn create_file_writer_tool_static() -> RegistryEntry {
        RegistryEntry {
            name: "file_writer".to_string(),
            description: "Creates and writes content to files with proper formatting and structure".to_string(),
            parameters: serde_json::json!({ 
                "type": "object", 
                "properties": { 
                    "filename": { "type": "string", "description": "The name of the file to create" },
                    "content": { "type": "string", "description": "The content to write to the file" }
                }, 
                "required": ["filename", "content"] 
            }),
            tool_type: ToolType::LLM { provider: "openai".to_string(), model: "gpt-4".to_string() },
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["file_operations".to_string(), "content_creation".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Limited,
        }
    }

    fn create_data_formatter_tool_static() -> RegistryEntry {
        RegistryEntry {
            name: "data_formatter".to_string(),
            description: "Formats data into structured, readable formats including tables, reports, and summaries".to_string(),
            parameters: serde_json::json!({ 
                "type": "object", 
                "properties": { 
                    "data": { "type": "string", "description": "The data to format" },
                    "format_type": { "type": "string", "description": "The desired output format" }
                }, 
                "required": ["data"] 
            }),
            tool_type: ToolType::LLM { provider: "openai".to_string(), model: "gpt-4".to_string() },
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["formatting".to_string(), "data_presentation".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Safe,
        }
    }

    fn create_web_search_tool_static() -> RegistryEntry {
        RegistryEntry {
            name: "webSearchTool".to_string(),
            description: "Search the web using Serper API with result caching and metadata extraction".to_string(),
            parameters: serde_json::json!({ 
                "type": "object", 
                "properties": { 
                    "query": { "type": "string", "description": "The search query" },
                    "type": { "type": "string", "description": "Type of search to perform" },
                    "num_results": { "type": "number", "description": "Number of results to return" }
                }, 
                "required": ["query"] 
            }),
            tool_type: ToolType::LLM { provider: "openai".to_string(), model: "gpt-4".to_string() },
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["web_search".to_string(), "information_retrieval".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Limited,
        }
    }

    fn create_write_file_tool_static() -> RegistryEntry {
        RegistryEntry {
            name: "writeFileTool".to_string(),
            description: "Write content to files with automatic directory creation and metadata extraction".to_string(),
            parameters: serde_json::json!({ 
                "type": "object", 
                "properties": { 
                    "path": { "type": "string", "description": "File path (legacy)" },
                    "filePath": { "type": "string", "description": "File path to write to" },
                    "content": { "type": "string", "description": "Content to write" },
                    "encoding": { "type": "string", "description": "File encoding (default: utf-8)" }
                }, 
                "required": ["content"] 
            }),
            tool_type: ToolType::LLM { provider: "openai".to_string(), model: "gpt-4".to_string() },
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["file_operations".to_string(), "content_creation".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Limited,
        }
    }

    fn create_read_file_tool_static() -> RegistryEntry {
        RegistryEntry {
            name: "readFileTool".to_string(),
            description: "Read file contents with format detection and comprehensive metadata".to_string(),
            parameters: serde_json::json!({ 
                "type": "object", 
                "properties": { 
                    "path": { "type": "string", "description": "File path (legacy)" },
                    "filePath": { "type": "string", "description": "File path to read from" },
                    "format": { "type": "string", "description": "Expected file format" }
                }, 
                "required": [] 
            }),
            tool_type: ToolType::LLM { provider: "openai".to_string(), model: "gpt-4".to_string() },
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["file_operations".to_string(), "content_reading".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Safe,
        }
    }

    fn create_parse_document_tool_static() -> RegistryEntry {
        RegistryEntry {
            name: "parseDocumentTool".to_string(),
            description: "LLM-powered document analysis with key point extraction and summarization".to_string(),
            parameters: serde_json::json!({ 
                "type": "object", 
                "properties": { 
                    "content": { "type": "string", "description": "Document content to analyze" },
                    "fileContent": { "type": "string", "description": "Document content to analyze (alias)" },
                    "format": { "type": "string", "description": "Document format" },
                    "extractionType": { "type": "string", "description": "Type of extraction to perform" }
                }, 
                "required": [] 
            }),
            tool_type: ToolType::LLM { provider: "openai".to_string(), model: "gpt-4".to_string() },
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["document_analysis".to_string(), "content_extraction".to_string(), "summarization".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Safe,
        }
    }

    fn create_write_code_tool_static() -> RegistryEntry {
        RegistryEntry {
            name: "writeCodeTool".to_string(),
            description: "LLM-powered code generation with language detection, file saving, and explanation".to_string(),
            parameters: serde_json::json!({ 
                "type": "object", 
                "properties": { 
                    "prompt": { "type": "string", "description": "Code generation prompt" },
                    "spec": { "type": "string", "description": "Code specification" },
                    "query": { "type": "string", "description": "Code query" },
                    "specification": { "type": "string", "description": "Code specification (alias)" },
                    "language": { "type": "string", "description": "Programming language" },
                    "context": { "type": "object", "description": "Additional context" },
                    "components": { "type": "object", "description": "Components to implement" },
                    "filePath": { "type": "string", "description": "File path to save code" }
                }, 
                "required": [] 
            }),
            tool_type: ToolType::LLM { provider: "openai".to_string(), model: "gpt-4".to_string() },
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["code_generation".to_string(), "programming".to_string(), "file_operations".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Limited,
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
                // Use specialized tool implementations for planning and cognitive tools
                match name {
                    "createPlanTool" => {
                        println!("🔧 Using specialized createPlanTool implementation");
                        create_plan_tool_handler(parameters, &self.llm_handler).await
                    }
                    "ponderTool" => {
                        println!("🔧 Using specialized ponderTool implementation");
                        ponder_tool_handler(parameters, &self.llm_handler).await
                    }
                    "webSearchTool" => {
                        println!("🔧 Using specialized webSearchTool implementation");
                        web_search_tool_handler(parameters, &self.llm_handler).await
                    }
                    "writeFileTool" => {
                        println!("🔧 Using specialized writeFileTool implementation");
                        write_file_tool_handler(parameters, &self.llm_handler).await
                    }
                    "readFileTool" => {
                        println!("🔧 Using specialized readFileTool implementation");
                        read_file_tool_handler(parameters, &self.llm_handler).await
                    }
                    "parseDocumentTool" => {
                        println!("🔧 Using specialized parseDocumentTool implementation");
                        parse_document_tool_handler(parameters, &self.llm_handler).await
                    }
                    "writeCodeTool" => {
                        println!("🔧 Using specialized writeCodeTool implementation");
                        write_code_tool_handler(parameters, &self.llm_handler).await
                    }
                    _ => {
                        // Generic LLM tool execution for other tools
                        let mut llm_params = HashMap::new();
                        if let Some(obj) = parameters.as_object() {
                            for (k,v) in obj {
                                llm_params.insert(k.clone(), v.clone());
                            }
                        }

                        // Safely extract query parameter or use tool name as fallback
                        let default_query = format!("Execute {} tool", name);
                        let query_content = if let Some(query_val) = llm_params.get("query").and_then(|v| v.as_str()) {
                            query_val
                        } else if let Some(first_val) = llm_params.values().next().and_then(|v| v.as_str()) {
                            first_val
                        } else {
                            &default_query
                        };

                        let request = LLMRequest {
                            messages: vec![LLMMessage { 
                                role: "user".to_string(), 
                                content: query_content.to_string(), 
                                tool_calls: None, 
                                tool_call_id: None 
                            }],
                            config: LLMConfig {
                                model: Some(model.clone()),
                                temperature: 0.7,
                                max_tokens: 2000,
                                top_p: None,
                                frequency_penalty: None,
                                presence_penalty: None,
                            },
                            provider: Some(provider.clone()),
                            tools: None,
                            tool_choice: None,
                            stream: Some(false),
                        };
                        
                        let response = self.llm_handler.complete(request).await?;
                        
                        Ok(ToolResult {
                            success: true,
                            result: Some(serde_json::json!({
                                "response": response.content,
                                "tool": name,
                                "provider": provider,
                                "model": model
                            }).into()),
                            error: None,
                            metadata: HashMap::new(),
                            execution_time_ms: 0,
                            resource_usage: None,
                        })
                    }
                }
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