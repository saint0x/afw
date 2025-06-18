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
    write_code_tool_handler,
};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::engines::container::quilt::QuiltService;
use tokio::sync::Mutex;

#[async_trait]
pub trait ToolRegistryInterface: Send + Sync {
    async fn execute_tool(&self, name: &str, parameters: DeepValue) -> AriaResult<ToolResult>;
    async fn get_tool_info(&self, name: &str) -> AriaResult<Option<types::RegistryEntry>>;
    async fn list_available_tools(&self) -> AriaResult<Vec<String>>;
    async fn is_tool_available(&self, tool_name: &str) -> bool;
    async fn list_primitive_tools(&self) -> AriaResult<Vec<String>>;
    async fn list_abstract_tools(&self) -> AriaResult<Vec<String>>;
    async fn list_tools_by_security_level(&self, level: SecurityLevel) -> AriaResult<Vec<String>>;
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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ToolScope {
    /// A high-level tool representing an intent (e.g., `writeFile`).
    /// The ExecutionEngine will "realize" this into a sequence of primitive tool calls.
    Abstract,
    /// A low-level tool that directly controls a system resource (e.g., `createContainer`).
    Primitive,
}

#[derive(Clone)]
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, RegistryEntry>>>,
    execution_stats: Arc<RwLock<HashMap<String, ToolExecutionStats>>>,
    bundle_store: Option<Arc<dyn BundleStoreInterface>>,
    llm_handler: Arc<LLMHandler>,
    quilt_service: Arc<Mutex<QuiltService>>,
}

#[derive(Debug, Clone)]
pub struct RegistryEntry {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub tool_type: ToolType,
    pub scope: ToolScope,
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

#[derive(Debug, Clone, PartialEq)]
pub enum SecurityLevel {
    Safe,
    Limited,
    Elevated,
    Dangerous,
}

impl ToolRegistry {
    pub async fn new(
        llm_handler: Arc<LLMHandler>,
        quilt_service: Arc<Mutex<QuiltService>>,
    ) -> Self {
        let registry = Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            execution_stats: Arc::new(RwLock::new(HashMap::new())),
            bundle_store: None,
            llm_handler,
            quilt_service,
        };
        
        registry.register_builtin_tools().await;
        registry.register_container_tools().await;
        registry.register_intelligence_tools().await;

        registry
    }

    async fn register_builtin_tools(&self) {
        // Builtin tools are deprecated in favor of agent sovereignty
        // Agents should use primitive tools directly or LLM tools for cognitive tasks
        let cognitive_tools = vec![
            Self::create_ponder_tool_static(),
            Self::create_create_plan_tool_static(),
            Self::create_web_search_tool_static(),
            Self::create_read_file_tool_static(),
            Self::create_parse_document_tool_static(),
            Self::create_write_code_tool_static(),
            Self::create_calculator_tool_static(),
            Self::create_text_analyzer_tool_static(),
            Self::create_data_formatter_tool_static(),
        ];
        let mut tools = self.tools.write().await;
        let tool_count = cognitive_tools.len();
        for tool in cognitive_tools {
            println!("🧠 Registering cognitive tool: {}", tool.name);
            tools.insert(tool.name.clone(), tool);
        }
        println!("✅ Registered {} cognitive tools (builtin container tools deprecated)", tool_count);
    }

    /// This function will register all the container-primitive tools.
    /// The tools themselves will be implemented in Phase 2.
    async fn register_container_tools(&self) {
        let container_tools = vec![
            crate::tools::container::create::create_container_tool(),
            crate::tools::container::start::start_container_tool(),
            crate::tools::container::exec::exec_in_container_tool(),
            crate::tools::container::stop::stop_container_tool(),
            crate::tools::container::remove::remove_container_tool(),
            crate::tools::container::list::list_containers_tool(),
            crate::tools::container::status::get_container_status_tool(),
            crate::tools::container::logs::get_container_logs_tool(),
            crate::tools::container::metrics::get_system_metrics_tool(),
            crate::tools::container::network_topology::get_network_topology_tool(),
            crate::tools::container::network_info::get_container_network_info_tool(),
        ];

        let mut tools = self.tools.write().await;
        for tool in container_tools {
            println!("📦 Registering container tool: {}", tool.name);
            tools.insert(tool.name.clone(), tool);
        }
    }

    /// Register intelligence tools for agents to use
    async fn register_intelligence_tools(&self) {
        let intelligence_tools = vec![
            Self::create_analyze_container_pattern_tool(),
            Self::create_get_execution_context_tool(),
            Self::create_get_context_for_prompt_tool(),
            Self::create_optimize_patterns_tool(),
            Self::create_get_learning_analytics_tool(),
            Self::create_analyze_session_workloads_tool(),
            Self::create_clear_context_cache_tool(),
        ];

        let mut tools = self.tools.write().await;
        for tool in intelligence_tools {
            println!("🧠 Registering intelligence tool: {}", tool.name);
            tools.insert(tool.name.clone(), tool);
        }
        println!("✅ Registered {} intelligence tools", 7);
    }

    fn create_ponder_tool_static() -> RegistryEntry {
        RegistryEntry {
            name: "ponderTool".to_string(),
            description: "Analyzes situations, reflects on outcomes, and provides strategic insights".to_string(),
            parameters: serde_json::json!({ "type": "object", "properties": { "query": { "type": "string" } }, "required": ["query"] }),
            tool_type: ToolType::LLM { provider: "openai".to_string(), model: "gpt-4".to_string() },
            scope: ToolScope::Abstract,
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
            scope: ToolScope::Abstract,
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
            scope: ToolScope::Abstract,
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
            scope: ToolScope::Abstract,
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["analysis".to_string(), "text_processing".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Safe,
        }
    }

    // file_writer removed - agents should use primitive container tools directly

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
            scope: ToolScope::Abstract,
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
            scope: ToolScope::Abstract,
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["web_search".to_string(), "information_retrieval".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Limited,
        }
    }

    // writeFileTool removed - agents should use primitive container tools directly

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
            scope: ToolScope::Abstract,
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
            scope: ToolScope::Abstract,
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
            scope: ToolScope::Abstract,
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["code_generation".to_string(), "programming".to_string(), "file_operations".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Limited,
        }
    }

    // Intelligence Tools for Agents

    fn create_analyze_container_pattern_tool() -> RegistryEntry {
        RegistryEntry {
            name: "analyzeContainerPattern".to_string(),
            description: "Analyze container request and provide intelligent recommendations based on learned patterns".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "request": {
                        "type": "string",
                        "description": "Container request description"
                    },
                    "session_id": {
                        "type": "string", 
                        "description": "Current session ID"
                    },
                    "requirements": {
                        "type": "object",
                        "description": "Container requirements (optional)",
                        "properties": {
                            "min_memory_mb": { "type": "number" },
                            "min_cpu_cores": { "type": "number" },
                            "required_tools": { "type": "array", "items": { "type": "string" } },
                            "timeout_seconds": { "type": "number" }
                        }
                    }
                },
                "required": ["request", "session_id"]
            }),
            tool_type: ToolType::LLM { provider: "aria_intelligence".to_string(), model: "pattern_analysis".to_string() },
            scope: ToolScope::Abstract,
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["pattern_analysis".to_string(), "container_intelligence".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Safe,
        }
    }

    fn create_get_execution_context_tool() -> RegistryEntry {
        RegistryEntry {
            name: "getExecutionContext".to_string(),
            description: "Get current execution context for intelligent decision making".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID to get context for"
                    },
                    "max_nodes": {
                        "type": "integer",
                        "description": "Maximum context nodes to return",
                        "default": 20
                    }
                },
                "required": ["session_id"]
            }),
            tool_type: ToolType::LLM { provider: "aria_intelligence".to_string(), model: "context_retrieval".to_string() },
            scope: ToolScope::Abstract,
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["context_management".to_string(), "session_intelligence".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Safe,
        }
    }

    fn create_get_context_for_prompt_tool() -> RegistryEntry {
        RegistryEntry {
            name: "getContextForPrompt".to_string(),
            description: "Get execution context formatted for agent prompts with priority filtering".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID to get context for"
                    },
                    "max_nodes": {
                        "type": "integer",
                        "description": "Maximum context nodes to include in prompt",
                        "default": 50
                    }
                },
                "required": ["session_id"]
            }),
            tool_type: ToolType::LLM { provider: "aria_intelligence".to_string(), model: "context_formatting".to_string() },
            scope: ToolScope::Abstract,
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["context_formatting".to_string(), "prompt_enhancement".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Safe,
        }
    }

    fn create_optimize_patterns_tool() -> RegistryEntry {
        RegistryEntry {
            name: "optimizePatterns".to_string(),
            description: "Optimize pattern performance and remove low-confidence patterns".to_string(),
            parameters: serde_json::json!({
                "type": "object", 
                "properties": {
                    "min_confidence": {
                        "type": "number",
                        "description": "Minimum confidence threshold for keeping patterns",
                        "default": 0.3
                    },
                    "max_age_days": {
                        "type": "integer",
                        "description": "Maximum age in days for patterns",
                        "default": 30
                    }
                }
            }),
            tool_type: ToolType::LLM { provider: "aria_intelligence".to_string(), model: "pattern_optimization".to_string() },
            scope: ToolScope::Abstract,
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["pattern_optimization".to_string(), "learning_enhancement".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Limited,
        }
    }

    fn create_get_learning_analytics_tool() -> RegistryEntry {
        RegistryEntry {
            name: "getLearningAnalytics".to_string(),
            description: "Get learning analytics and pattern performance statistics".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Optional session ID for specific analytics"
                    },
                    "include_detailed": {
                        "type": "boolean",
                        "description": "Include detailed performance metrics",
                        "default": false
                    }
                }
            }),
            tool_type: ToolType::LLM { provider: "aria_intelligence".to_string(), model: "analytics_retrieval".to_string() },
            scope: ToolScope::Abstract,
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["analytics".to_string(), "performance_monitoring".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Safe,
        }
    }

    fn create_analyze_session_workloads_tool() -> RegistryEntry {
        RegistryEntry {
            name: "analyzeSessionWorkloads".to_string(),
            description: "Analyze workload patterns and performance for a specific session".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID to analyze"
                    }
                },
                "required": ["session_id"]
            }),
            tool_type: ToolType::LLM { provider: "aria_intelligence".to_string(), model: "workload_analysis".to_string() },
            scope: ToolScope::Abstract,
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["workload_analysis".to_string(), "session_insights".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Safe,
        }
    }

    fn create_clear_context_cache_tool() -> RegistryEntry {
        RegistryEntry {
            name: "clearContextCache".to_string(),
            description: "Clear the context cache to force fresh context building".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
            tool_type: ToolType::LLM { provider: "aria_intelligence".to_string(), model: "cache_management".to_string() },
            scope: ToolScope::Abstract,
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["cache_management".to_string(), "context_refresh".to_string()],
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
            ToolType::Builtin => {
                // Builtin tools are deprecated in favor of agent sovereignty
                // Agents should use primitive tools directly for full control
                Err(AriaError::new(
                    ErrorCode::NotSupported,
                    ErrorCategory::Tool,
                    ErrorSeverity::Medium,
                    &format!("Builtin tool '{}' is deprecated. Use primitive container tools for full agent control.", name),
                ))
            }
            ToolType::Container { .. } => {
                println!("📦 Executing container tool: {}", name);
                let params_obj = parameters.as_object().ok_or_else(|| AriaError::new(
                    ErrorCode::ToolInvalidParameters,
                    ErrorCategory::Tool,
                    ErrorSeverity::Medium,
                    "Container tool parameters must be an object",
                ))?;

                match name {
                    "createContainer" => {
                        let image = params_obj.get("image").and_then(|v| v.as_str()).ok_or_else(|| AriaError::new(
                            ErrorCode::ToolInvalidParameters,
                            ErrorCategory::Tool,
                            ErrorSeverity::Medium,
                            "Missing required parameter 'image' for createContainer",
                        ))?.to_string();

                        let command = params_obj.get("command").and_then(|v| v.as_array()).map(|arr| {
                            arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()
                        }).unwrap_or_default();

                        let env = params_obj.get("env").and_then(|v| v.as_object()).map(|obj| {
                            obj.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect()
                        }).unwrap_or_default();
                        
                        let mut quilt = self.quilt_service.lock().await;
                        let container_id = quilt.create_container(image, command, env).await?;

                        Ok(ToolResult {
                            success: true,
                            result: Some(serde_json::json!({ "containerId": container_id }).into()),
                            error: None,
                            metadata: HashMap::new(),
                            execution_time_ms: 0,
                            resource_usage: None,
                        })
                    }
                    "startContainer" => {
                        let container_id = params_obj.get("containerId").and_then(|v| v.as_str()).ok_or_else(|| AriaError::new(
                            ErrorCode::ToolInvalidParameters,
                            ErrorCategory::Tool,
                            ErrorSeverity::Medium,
                            "Missing required parameter 'containerId' for startContainer",
                        ))?.to_string();
                        
                        let mut quilt = self.quilt_service.lock().await;
                        quilt.start_container(container_id.clone()).await?;

                        Ok(ToolResult {
                            success: true,
                            result: Some(serde_json::json!({ "containerId": container_id, "status": "starting" }).into()),
                            error: None,
                            metadata: HashMap::new(),
                            execution_time_ms: 0,
                            resource_usage: None,
                        })
                    }
                    "execInContainer" => {
                        let container_id = params_obj.get("containerId").and_then(|v| v.as_str()).ok_or_else(|| AriaError::new(
                            ErrorCode::ToolInvalidParameters,
                            ErrorCategory::Tool,
                            ErrorSeverity::Medium,
                            "Missing required parameter 'containerId' for execInContainer",
                        ))?.to_string();

                        let command = params_obj.get("command").and_then(|v| v.as_array()).map(|arr| {
                            arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()
                        }).ok_or_else(|| AriaError::new(
                            ErrorCode::ToolInvalidParameters,
                            ErrorCategory::Tool,
                            ErrorSeverity::Medium,
                            "Missing required parameter 'command' for execInContainer",
                        ))?;
                        
                        let mut quilt = self.quilt_service.lock().await;
                        let exec_result = quilt.exec_in_container(container_id, command).await?;

                        Ok(ToolResult {
                            success: exec_result.exit_code == 0,
                            result: Some(serde_json::json!({
                                "exitCode": exec_result.exit_code,
                                "stdout": exec_result.stdout,
                                "stderr": exec_result.stderr,
                            }).into()),
                            error: if exec_result.exit_code == 0 { None } else { Some(exec_result.stderr) },
                            metadata: HashMap::new(),
                            execution_time_ms: exec_result.execution_time_ms,
                            resource_usage: exec_result.resource_usage,
                        })
                    }
                    "stopContainer" => {
                        let container_id = params_obj.get("containerId").and_then(|v| v.as_str()).ok_or_else(|| AriaError::new(
                            ErrorCode::ToolInvalidParameters,
                            ErrorCategory::Tool,
                            ErrorSeverity::Medium,
                            "Missing required parameter 'containerId' for stopContainer",
                        ))?.to_string();
                        
                        let mut quilt = self.quilt_service.lock().await;
                        quilt.stop_container(container_id).await?;

                        Ok(ToolResult {
                            success: true,
                            result: Some(serde_json::json!({ "status": "stopped" }).into()),
                            error: None,
                            metadata: HashMap::new(),
                            execution_time_ms: 0,
                            resource_usage: None,
                        })
                    }
                    "removeContainer" => {
                        let container_id = params_obj.get("containerId").and_then(|v| v.as_str()).ok_or_else(|| AriaError::new(
                            ErrorCode::ToolInvalidParameters,
                            ErrorCategory::Tool,
                            ErrorSeverity::Medium,
                            "Missing required parameter 'containerId' for removeContainer",
                        ))?.to_string();
                        
                        let mut quilt = self.quilt_service.lock().await;
                        quilt.remove_container(container_id).await?;

                        Ok(ToolResult {
                            success: true,
                            result: Some(serde_json::json!({ "status": "removed" }).into()),
                            error: None,
                            metadata: HashMap::new(),
                            execution_time_ms: 0,
                            resource_usage: None,
                        })
                    }
                    "listContainers" => {
                        let mut quilt = self.quilt_service.lock().await;
                        let containers = quilt.list_containers().await?;

                        Ok(ToolResult {
                            success: true,
                            result: Some(serde_json::to_value(containers).unwrap_or_default().into()),
                            error: None,
                            metadata: HashMap::new(),
                            execution_time_ms: 0,
                            resource_usage: None,
                        })
                    }
                    "getContainerStatus" => {
                        let container_id = params_obj.get("containerId").and_then(|v| v.as_str()).ok_or_else(|| AriaError::new(
                            ErrorCode::ToolInvalidParameters,
                            ErrorCategory::Tool,
                            ErrorSeverity::Medium,
                            "Missing required parameter 'containerId' for getContainerStatus",
                        ))?.to_string();
                        
                        let mut quilt = self.quilt_service.lock().await;
                        let status = quilt.get_container_status(container_id).await?;

                        Ok(ToolResult {
                            success: true,
                            result: Some(serde_json::to_value(status).unwrap_or_default().into()),
                            error: None,
                            metadata: HashMap::new(),
                            execution_time_ms: 0,
                            resource_usage: None,
                        })
                    }
                    "getContainerLogs" => {
                        let container_id = params_obj.get("containerId").and_then(|v| v.as_str()).ok_or_else(|| AriaError::new(
                            ErrorCode::ToolInvalidParameters,
                            ErrorCategory::Tool,
                            ErrorSeverity::Medium,
                            "Missing required parameter 'containerId' for getContainerLogs",
                        ))?.to_string();
                        
                        let mut quilt = self.quilt_service.lock().await;
                        let logs = quilt.get_container_logs(container_id).await?;

                        Ok(ToolResult {
                            success: true,
                            result: Some(serde_json::json!({ "logs": logs }).into()),
                            error: None,
                            metadata: HashMap::new(),
                            execution_time_ms: 0,
                            resource_usage: None,
                        })
                    }
                    "getSystemMetrics" => {
                        let mut quilt = self.quilt_service.lock().await;
                        let metrics = quilt.get_system_metrics().await?;

                        Ok(ToolResult {
                            success: true,
                            result: Some(serde_json::to_value(metrics).unwrap_or_default().into()),
                            error: None,
                            metadata: HashMap::new(),
                            execution_time_ms: 0,
                            resource_usage: None,
                        })
                    }
                    "getNetworkTopology" => {
                        let mut quilt = self.quilt_service.lock().await;
                        let topology = quilt.get_network_topology().await?;

                        Ok(ToolResult {
                            success: true,
                            result: Some(serde_json::to_value(topology).unwrap_or_default().into()),
                            error: None,
                            metadata: HashMap::new(),
                            execution_time_ms: 0,
                            resource_usage: None,
                        })
                    }
                    "getContainerNetworkInfo" => {
                        let container_id = params_obj.get("containerId").and_then(|v| v.as_str()).ok_or_else(|| AriaError::new(
                            ErrorCode::ToolInvalidParameters,
                            ErrorCategory::Tool,
                            ErrorSeverity::Medium,
                            "Missing required parameter 'containerId' for getContainerNetworkInfo",
                        ))?.to_string();

                        let mut quilt = self.quilt_service.lock().await;
                        let info = quilt.get_container_network_info(container_id).await?;

                        Ok(ToolResult {
                            success: true,
                            result: Some(serde_json::to_value(info).unwrap_or_default().into()),
                            error: None,
                            metadata: HashMap::new(),
                            execution_time_ms: 0,
                            resource_usage: None,
                        })
                    }
                    _ => Err(AriaError::new(
                        ErrorCode::NotSupported,
                        ErrorCategory::Tool,
                        ErrorSeverity::Medium,
                        &format!("Unsupported container tool: {}", name),
                    )),
                }
            }
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
    
    /// Get all primitive tools (for agent empowerment)
    async fn list_primitive_tools(&self) -> AriaResult<Vec<String>> {
        let tools = self.tools.read().await;
        let primitive_tools: Vec<String> = tools
            .iter()
            .filter(|(_, entry)| entry.scope == ToolScope::Primitive)
            .map(|(name, _)| name.clone())
            .collect();
        Ok(primitive_tools)
    }
    
    /// Get all abstract tools (for convenience layer)
    async fn list_abstract_tools(&self) -> AriaResult<Vec<String>> {
        let tools = self.tools.read().await;
        let abstract_tools: Vec<String> = tools
            .iter()
            .filter(|(_, entry)| entry.scope == ToolScope::Abstract)
            .map(|(name, _)| name.clone())
            .collect();
        Ok(abstract_tools)
    }
    
    /// Get tools by security level (for access control)
    async fn list_tools_by_security_level(&self, level: SecurityLevel) -> AriaResult<Vec<String>> {
        let tools = self.tools.read().await;
        let filtered_tools: Vec<String> = tools
            .iter()
            .filter(|(_, entry)| entry.security_level == level)
            .map(|(name, _)| name.clone())
            .collect();
        Ok(filtered_tools)
    }
} 