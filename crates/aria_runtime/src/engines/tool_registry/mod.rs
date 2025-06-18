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

pub mod bundle_integration;

pub use bundle_integration::{
    BundleToolRegistry, BundleToolRegistration, ToolSourceInfo, 
    CustomToolEntry, BundleToolStats
};

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
    pub tools: Arc<RwLock<HashMap<String, RegistryEntry>>>,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
            description: "Optimize learned patterns based on execution outcomes and performance metrics".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID to optimize patterns for"
                    }
                },
                "required": ["session_id"]
            }),
            tool_type: ToolType::LLM { provider: "aria_intelligence".to_string(), model: "pattern_optimization".to_string() },
            scope: ToolScope::Abstract,
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["pattern_optimization".to_string(), "learning".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Safe,
        }
    }

    fn create_get_learning_analytics_tool() -> RegistryEntry {
        RegistryEntry {
            name: "getLearningAnalytics".to_string(),
            description: "Get comprehensive learning analytics and pattern performance metrics".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID to get analytics for"
                    },
                    "time_range": {
                        "type": "string",
                        "description": "Time range for analytics (hour, day, week)",
                        "default": "day"
                    }
                },
                "required": ["session_id"]
            }),
            tool_type: ToolType::LLM { provider: "aria_intelligence".to_string(), model: "learning_analytics".to_string() },
            scope: ToolScope::Abstract,
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["analytics".to_string(), "learning_insights".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Safe,
        }
    }

    fn create_analyze_session_workloads_tool() -> RegistryEntry {
        RegistryEntry {
            name: "analyzeSessionWorkloads".to_string(),
            description: "Analyze session workloads for optimization opportunities and resource planning".to_string(),
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
            capabilities: vec!["workload_analysis".to_string(), "resource_optimization".to_string()],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Safe,
        }
    }

    fn create_clear_context_cache_tool() -> RegistryEntry {
        RegistryEntry {
            name: "clearContextCache".to_string(),
            description: "Clear context cache for a session to reset learning state".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID to clear cache for"
                    }
                },
                "required": ["session_id"]
            }),
            tool_type: ToolType::LLM { provider: "aria_intelligence".to_string(), model: "cache_management".to_string() },
            scope: ToolScope::Abstract,
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["cache_management".to_string(), "session_reset".to_string()],
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
            ToolType::LLM { provider: _, model: _ } => {
                // Execute LLM-based tools using the handler
                match name {
                    "ponderTool" => ponder_tool_handler(parameters, &self.llm_handler).await,
                    "createPlanTool" => create_plan_tool_handler(parameters, &self.llm_handler).await,
                    "webSearchTool" => web_search_tool_handler(parameters, &self.llm_handler).await,
                    "readFileTool" => read_file_tool_handler(parameters, &self.llm_handler).await,
                    "parseDocumentTool" => parse_document_tool_handler(parameters, &self.llm_handler).await,
                    "writeCodeTool" => write_code_tool_handler(parameters, &self.llm_handler).await,
                    _ => {
                        // For other LLM tools, use a generic handler
                        self.execute_llm_tool(name, parameters, &tool_entry).await
                    }
                }
            }
            ToolType::Container { .. } => {
                // Container tools use the quilt service for execution
                self.execute_container_tool(name, parameters, &tool_entry).await
            }
            ToolType::Bundle { .. } => {
                // Bundle tools would be executed in their respective containers
                // For now, return an error as bundle execution is not yet implemented
                Err(AriaError::new(
                    ErrorCode::NotSupported,
                    ErrorCategory::Tool,
                    ErrorSeverity::Medium,
                    &format!("Bundle tool '{}' execution not yet implemented", name),
                ))
            }
        }
    }

    async fn get_tool_info(&self, name: &str) -> AriaResult<Option<types::RegistryEntry>> {
        let tools = self.tools.read().await;
        if let Some(entry) = tools.get(name) {
            // Convert from internal RegistryEntry to types::RegistryEntry
            let registry_entry = types::RegistryEntry {
                name: entry.name.clone(),
                entry_type: types::RegistryEntryType::Tool,
                bundle_id: entry.bundle_id.clone(),
                version: entry.version.clone(),
                metadata: {
                    let mut metadata = HashMap::new();
                    metadata.insert("description".to_string(), DeepValue::string(entry.description.clone()));
                    metadata.insert("capabilities".to_string(), DeepValue::array(
                        entry.capabilities.iter().map(|c| DeepValue::string(c.clone())).collect()
                    ));
                    metadata
                },
                created_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                updated_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            };
            Ok(Some(registry_entry))
        } else {
            Ok(None)
        }
    }

    async fn list_available_tools(&self) -> AriaResult<Vec<String>> {
        let tools = self.tools.read().await;
        Ok(tools.keys().cloned().collect())
    }

    async fn is_tool_available(&self, tool_name: &str) -> bool {
        let tools = self.tools.read().await;
        tools.contains_key(tool_name)
    }

    async fn list_primitive_tools(&self) -> AriaResult<Vec<String>> {
        let tools = self.tools.read().await;
        Ok(tools
            .iter()
            .filter(|(_, entry)| entry.scope == ToolScope::Primitive)
            .map(|(name, _)| name.clone())
            .collect())
    }

    async fn list_abstract_tools(&self) -> AriaResult<Vec<String>> {
        let tools = self.tools.read().await;
        Ok(tools
            .iter()
            .filter(|(_, entry)| entry.scope == ToolScope::Abstract)
            .map(|(name, _)| name.clone())
            .collect())
    }

    async fn list_tools_by_security_level(&self, level: SecurityLevel) -> AriaResult<Vec<String>> {
        let tools = self.tools.read().await;
        Ok(tools
            .iter()
            .filter(|(_, entry)| entry.security_level == level)
            .map(|(name, _)| name.clone())
            .collect())
    }
}

impl ToolRegistry {
    async fn execute_llm_tool(&self, name: &str, parameters: DeepValue, entry: &RegistryEntry) -> AriaResult<ToolResult> {
        // Generic LLM tool execution
        let start_time = std::time::Instant::now();
        
        // For intelligence tools, execute them directly
        match name {
            "analyzeContainerPattern" => {
                // TODO: Implement actual intelligence engine integration
                Ok(ToolResult {
                    success: true,
                    result: Some(DeepValue::string("Pattern analysis completed".to_string())),
                    error: None,
                    metadata: HashMap::new(),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    resource_usage: Some(types::ResourceUsage::default()),
                })
            }
            "getExecutionContext" | "getContextForPrompt" | "optimizePatterns" | 
            "getLearningAnalytics" | "analyzeSessionWorkloads" | "clearContextCache" => {
                // TODO: Implement actual intelligence engine integration
                Ok(ToolResult {
                    success: true,
                    result: Some(DeepValue::string(format!("{} completed", name))),
                    error: None,
                    metadata: HashMap::new(),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    resource_usage: Some(types::ResourceUsage::default()),
                })
            }
            _ => {
                Err(AriaError::new(
                    ErrorCode::NotSupported,
                    ErrorCategory::Tool,
                    ErrorSeverity::Medium,
                    &format!("LLM tool '{}' execution not implemented", name),
                ))
            }
        }
    }

    async fn execute_container_tool(&self, name: &str, parameters: DeepValue, entry: &RegistryEntry) -> AriaResult<ToolResult> {
        // Container tool execution using quilt service
        let start_time = std::time::Instant::now();
        
        // For now, return a placeholder result
        // TODO: Implement actual container tool execution
        Ok(ToolResult {
            success: true,
            result: Some(DeepValue::string(format!("Container tool '{}' executed", name))),
            error: None,
            metadata: HashMap::new(),
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            resource_usage: Some(types::ResourceUsage::default()),
        })
    }
} 