use async_trait::async_trait;
use crate::types::*;
use crate::errors::{AriaResult, AriaError, ErrorCode, ErrorCategory, ErrorSeverity};
use crate::engines::llm::LLMHandler;
use crate::engines::llm::types::{LLMRequest, LLMMessage, LLMConfig};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::Value;

/// Production-grade tool registry that manages tool loading, validation, and execution
#[derive(Clone)]
pub struct ToolRegistry {
    /// Registered tools indexed by name
    tools: Arc<RwLock<HashMap<String, RegistryEntry>>>,
    /// Tool execution statistics for optimization
    execution_stats: Arc<RwLock<HashMap<String, ToolExecutionStats>>>,
    /// Bundle store for loading .aria bundles
    bundle_store: Option<Arc<dyn BundleStoreInterface>>,
    /// LLM handler for executing LLM-based tools
    llm_handler: Arc<LLMHandler>,
}

#[derive(Debug, Clone)]
pub struct RegistryEntry {
    pub name: String,
    pub description: String,
    pub parameters: Value, // JSON schema for parameters
    pub tool_type: ToolType,
    pub bundle_id: Option<String>,
    pub version: String,
    pub capabilities: Vec<String>,
    pub resource_requirements: ResourceRequirements,
    pub security_level: SecurityLevel,
}

#[derive(Debug, Clone)]
pub enum ToolType {
    /// Built-in system tools (always available)
    Builtin,
    /// Tools loaded from .aria bundles
    Bundle { bundle_path: String, entry_point: String },
    /// Container-based tools that run in isolation
    Container { image: String, command: Vec<String> },
    /// LLM-based tools that use language models
    LLM { provider: String, model: String },
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
    Safe,      // No external access, read-only operations
    Limited,   // Limited external access, controlled writes
    Elevated,  // Full system access, requires approval
    Dangerous, // Unrestricted access, requires explicit consent
}

impl ToolRegistry {
    pub fn new(bundle_store: Option<Arc<dyn BundleStoreInterface>>, llm_handler: Arc<LLMHandler>) -> Self {
        let registry = Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            execution_stats: Arc::new(RwLock::new(HashMap::new())),
            bundle_store,
            llm_handler,
        };
        
        // Register built-in tools asynchronously
        let tools_arc = registry.tools.clone();
        tokio::spawn(async move {
            let builtin_tools = vec![
                Self::create_ponder_tool_static(),
                Self::create_create_plan_tool_static(),
                Self::create_web_search_tool_static(),
                Self::create_file_operations_tool_static(),
                Self::create_code_execution_tool_static(),
            ];

            let mut tools = tools_arc.write().await;
            for tool in builtin_tools {
                tools.insert(tool.name.clone(), tool);
            }
        });
        
        registry
    }

    /// Register all built-in system tools
    async fn register_builtin_tools(&self) -> AriaResult<()> {
        let builtin_tools = vec![
            self.create_ponder_tool(),
            self.create_create_plan_tool(),
            self.create_web_search_tool(),
            self.create_file_operations_tool(),
            self.create_code_execution_tool(),
        ];

        let mut tools = self.tools.write().await;
        for tool in builtin_tools {
            tools.insert(tool.name.clone(), tool);
        }

        Ok(())
    }

    /// Create the ponder tool (critical for Symphony's reflection system)
    fn create_ponder_tool(&self) -> RegistryEntry {
        Self::create_ponder_tool_static()
    }

    /// Static version for async initialization
    fn create_ponder_tool_static() -> RegistryEntry {
        RegistryEntry {
            name: "ponderTool".to_string(),
            description: "Analyzes situations, reflects on outcomes, and provides strategic insights".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The question or situation to ponder about"
                    },
                    "context": {
                        "type": "object",
                        "description": "Additional context for the pondering process"
                    }
                },
                "required": ["query"]
            }),
            tool_type: ToolType::LLM {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
            },
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["reflection".to_string(), "analysis".to_string(), "strategy".to_string()],
            resource_requirements: ResourceRequirements {
                cpu_millis: 1000,
                memory_mb: 512,
                disk_mb: 10,
                network_bandwidth_kbps: None,
                container_count: 0,
                cpu_cores: Some(1),
                timeout_seconds: Some(30),
                max_concurrent: Some(5),
            },
            security_level: SecurityLevel::Safe,
        }
    }

    /// Create the create plan tool (critical for Symphony's planning system)
    fn create_create_plan_tool(&self) -> RegistryEntry {
        Self::create_create_plan_tool_static()
    }

    /// Static version for async initialization
    fn create_create_plan_tool_static() -> RegistryEntry {
        RegistryEntry {
            name: "createPlanTool".to_string(),
            description: "Creates detailed execution plans for complex multi-step tasks".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "objective": {
                        "type": "string",
                        "description": "The main objective to create a plan for"
                    },
                    "context": {
                        "type": "object",
                        "description": "Context including available tools and agent capabilities"
                    }
                },
                "required": ["objective"]
            }),
            tool_type: ToolType::LLM {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
            },
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["planning".to_string(), "strategy".to_string(), "decomposition".to_string()],
            resource_requirements: ResourceRequirements {
                cpu_millis: 1000,
                memory_mb: 1024,
                disk_mb: 20,
                network_bandwidth_kbps: None,
                container_count: 0,
                cpu_cores: Some(1),
                timeout_seconds: Some(60),
                max_concurrent: Some(3),
            },
            security_level: SecurityLevel::Safe,
        }
    }

    /// Create web search tool
    fn create_web_search_tool(&self) -> RegistryEntry {
        Self::create_web_search_tool_static()
    }

    /// Static version for async initialization
    fn create_web_search_tool_static() -> RegistryEntry {
        RegistryEntry {
            name: "webSearchTool".to_string(),
            description: "Searches the web for current information and returns relevant results".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of results to return",
                        "default": 5
                    }
                },
                "required": ["query"]
            }),
            tool_type: ToolType::Builtin,
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["search".to_string(), "information".to_string(), "web".to_string()],
            resource_requirements: ResourceRequirements {
                cpu_millis: 500,
                memory_mb: 256,
                disk_mb: 5,
                network_bandwidth_kbps: Some(1000),
                container_count: 0,
                cpu_cores: Some(1),
                timeout_seconds: Some(15),
                max_concurrent: Some(10),
            },
            security_level: SecurityLevel::Limited,
        }
    }

    /// Create file operations tool
    fn create_file_operations_tool(&self) -> RegistryEntry {
        Self::create_file_operations_tool_static()
    }

    /// Static version for async initialization
    fn create_file_operations_tool_static() -> RegistryEntry {
        RegistryEntry {
            name: "fileOperationsTool".to_string(),
            description: "Performs file system operations like read, write, list, and manage files".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["read", "write", "list", "delete", "create_directory"],
                        "description": "The file operation to perform"
                    },
                    "path": {
                        "type": "string",
                        "description": "The file or directory path"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content for write operations"
                    }
                },
                "required": ["operation", "path"]
            }),
            tool_type: ToolType::Builtin,
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["filesystem".to_string(), "io".to_string()],
            resource_requirements: ResourceRequirements {
                cpu_millis: 200,
                memory_mb: 128,
                disk_mb: 50,
                network_bandwidth_kbps: None,
                container_count: 0,
                cpu_cores: Some(1),
                timeout_seconds: Some(10),
                max_concurrent: Some(20),
            },
            security_level: SecurityLevel::Elevated,
        }
    }

    /// Create code execution tool
    fn create_code_execution_tool(&self) -> RegistryEntry {
        Self::create_code_execution_tool_static()
    }

    /// Static version for async initialization
    fn create_code_execution_tool_static() -> RegistryEntry {
        RegistryEntry {
            name: "codeExecutionTool".to_string(),
            description: "Executes code in various languages within secure containers".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "language": {
                        "type": "string",
                        "enum": ["python", "javascript", "typescript", "rust", "bash"],
                        "description": "The programming language"
                    },
                    "code": {
                        "type": "string",
                        "description": "The code to execute"
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Execution timeout in seconds",
                        "default": 30
                    }
                },
                "required": ["language", "code"]
            }),
            tool_type: ToolType::Container {
                image: "aria-runtime/code-executor".to_string(),
                command: vec!["execute".to_string()],
            },
            bundle_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["code".to_string(), "execution".to_string(), "programming".to_string()],
            resource_requirements: ResourceRequirements {
                cpu_millis: 2000,
                memory_mb: 2048,
                disk_mb: 100,
                network_bandwidth_kbps: None,
                container_count: 1,
                cpu_cores: Some(2),
                timeout_seconds: Some(60),
                max_concurrent: Some(5),
            },
            security_level: SecurityLevel::Dangerous,
        }
    }

    /// Load tools from an .aria bundle
    pub async fn load_bundle(&self, bundle_path: &str) -> AriaResult<Vec<String>> {
        if let Some(bundle_store) = &self.bundle_store {
            let bundle = bundle_store.load_bundle(bundle_path).await?;
            let mut loaded_tools = Vec::new();

            for tool_manifest in bundle.tools {
                let registry_entry = RegistryEntry {
                    name: tool_manifest.name.clone(),
                    description: tool_manifest.description,
                    parameters: tool_manifest.parameters,
                    tool_type: ToolType::Bundle {
                        bundle_path: bundle_path.to_string(),
                        entry_point: tool_manifest.entry_point,
                    },
                    bundle_id: Some(bundle.id.clone()),
                    version: bundle.version.clone(),
                    capabilities: tool_manifest.capabilities,
                    resource_requirements: tool_manifest.resource_requirements,
                    security_level: tool_manifest.security_level,
                };

                let mut tools = self.tools.write().await;
                tools.insert(tool_manifest.name.clone(), registry_entry);
                loaded_tools.push(tool_manifest.name);
            }

            Ok(loaded_tools)
        } else {
            Err(AriaError::new(
                ErrorCode::BundleLoadingFailed,
                ErrorCategory::Bundle,
                ErrorSeverity::High,
                "Bundle store not available for loading bundles"
            ))
        }
    }

    /// Update execution statistics for a tool
    async fn update_execution_stats(&self, tool_name: &str, success: bool, execution_time_ms: u64) {
        let mut stats = self.execution_stats.write().await;
        let tool_stats = stats.entry(tool_name.to_string()).or_insert(ToolExecutionStats {
            total_executions: 0,
            successful_executions: 0,
            average_execution_time_ms: 0.0,
            last_execution: None,
            error_rate: 0.0,
        });

        tool_stats.total_executions += 1;
        if success {
            tool_stats.successful_executions += 1;
        }

        // Update average execution time using exponential moving average
        let alpha = 0.1; // Smoothing factor
        tool_stats.average_execution_time_ms = 
            alpha * execution_time_ms as f64 + (1.0 - alpha) * tool_stats.average_execution_time_ms;

        tool_stats.last_execution = Some(std::time::SystemTime::now());
        tool_stats.error_rate = 1.0 - (tool_stats.successful_executions as f64 / tool_stats.total_executions as f64);
    }

    /// Get detailed tool information
    pub async fn get_tool_info(&self, tool_name: &str) -> Option<RegistryEntry> {
        let tools = self.tools.read().await;
        tools.get(tool_name).cloned()
    }

    /// Check if a tool is available for execution
    pub async fn is_tool_available(&self, tool_name: &str) -> bool {
        let tools = self.tools.read().await;
        tools.contains_key(tool_name)
    }
}

#[async_trait]
impl ToolRegistryInterface for ToolRegistry {
    async fn execute_tool(&self, name: &str, parameters: Value) -> AriaResult<ToolResult> {
        let start_time = std::time::Instant::now();
        
        // Get tool info
        let tool_entry = {
            let tools = self.tools.read().await;
            tools.get(name).cloned()
        };

        let tool_entry = tool_entry.ok_or_else(|| AriaError::new(
            ErrorCode::ToolNotFound,
            ErrorCategory::Tool,
            ErrorSeverity::Medium,
            format!("Tool '{}' not found in registry", name)
        ))?;

        // Execute based on tool type
        let result = match &tool_entry.tool_type {
            ToolType::Builtin => self.execute_builtin_tool(name, parameters).await,
            ToolType::Bundle { bundle_path, entry_point } => {
                self.execute_bundle_tool(bundle_path, entry_point, parameters).await
            },
            ToolType::Container { image, command } => {
                self.execute_container_tool(image, command, parameters).await
            },
            ToolType::LLM { provider, model } => {
                self.execute_llm_tool(provider, model, name, parameters).await
            },
        };

        // Update statistics
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        self.update_execution_stats(name, result.is_ok() && result.as_ref().unwrap().success, execution_time_ms).await;

        result
    }

    async fn get_tool_info(&self, name: &str) -> AriaResult<Option<crate::types::RegistryEntry>> {
        let tools = self.tools.read().await;
        if let Some(tool_entry) = tools.get(name) {
            // Convert our internal RegistryEntry to the types::RegistryEntry
            Ok(Some(crate::types::RegistryEntry {
                name: tool_entry.name.clone(),
                entry_type: crate::types::RegistryEntryType::Tool,
                bundle_id: tool_entry.bundle_id.clone(),
                version: tool_entry.version.clone(),
                metadata: std::collections::HashMap::new(),
                created_at: 0,
                updated_at: 0,
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_available_tools(&self) -> AriaResult<Vec<String>> {
        let tools = self.tools.read().await;
        Ok(tools.keys().cloned().collect())
    }
}

impl ToolRegistry {
    /// Execute a built-in tool
    async fn execute_builtin_tool(&self, name: &str, parameters: Value) -> AriaResult<ToolResult> {
        match name {
            "webSearchTool" => self.execute_web_search(parameters).await,
            "fileOperationsTool" => self.execute_file_operations(parameters).await,
            _ => Err(AriaError::new(
                ErrorCode::ToolExecutionFailed,
                ErrorCategory::Tool,
                ErrorSeverity::Medium,
                format!("Built-in tool '{}' not implemented", name)
            ))
        }
    }

    /// Execute a bundle-based tool
    async fn execute_bundle_tool(&self, _bundle_path: &str, _entry_point: &str, _parameters: Value) -> AriaResult<ToolResult> {
        // TODO: Implement bundle execution via container or direct execution
        Err(AriaError::new(
            ErrorCode::ToolExecutionFailed,
            ErrorCategory::Tool,
            ErrorSeverity::High,
            "Bundle tool execution not yet implemented"
        ))
    }

    /// Execute a container-based tool
    async fn execute_container_tool(&self, _image: &str, _command: &[String], _parameters: Value) -> AriaResult<ToolResult> {
        // TODO: Implement container tool execution via Quilt
        Err(AriaError::new(
            ErrorCode::ToolExecutionFailed,
            ErrorCategory::Tool,
            ErrorSeverity::High,
            "Container tool execution not yet implemented"
        ))
    }

    /// Execute an LLM-based tool (like ponderTool, createPlanTool)
    async fn execute_llm_tool(&self, provider: &str, model: &str, tool_name: &str, parameters: Value) -> AriaResult<ToolResult> {
        match tool_name {
            "ponderTool" => self.execute_ponder_tool(parameters, provider, model).await,
            "createPlanTool" => self.execute_create_plan_tool(parameters, provider, model).await,
            _ => Err(AriaError::new(
                ErrorCode::ToolExecutionFailed,
                ErrorCategory::Tool,
                ErrorSeverity::Medium,
                format!("LLM tool '{}' not implemented", tool_name)
            ))
        }
    }

    /// Execute the ponder tool (critical for Symphony's reflection)
    async fn execute_ponder_tool(&self, parameters: Value, provider: &str, model: &str) -> AriaResult<ToolResult> {
        let query = parameters.get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AriaError::new(
                ErrorCode::InvalidParameters,
                ErrorCategory::Tool,
                ErrorSeverity::Medium,
                "ponderTool requires 'query' parameter"
            ))?;

        let context = parameters.get("context").cloned().unwrap_or(Value::Null);

        let system_prompt = "You are an advanced cognitive engine designed for deep, structured thinking. Your purpose is to analyze problems with consciousness-emergent thought patterns. Your response must be a single JSON object with the keys: 'summary', 'confidence', 'nextSteps', 'analysisType'.";

        let user_prompt = format!(
            "Analyze the following situation and provide strategic insights in a JSON format.\n\nSituation: {}\n\nContext: {}",
            query,
            serde_json::to_string_pretty(&context).unwrap_or_default()
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
                    content: user_prompt,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            config: LLMConfig {
                model: Some(model.to_string()),
                temperature: 0.5,
                ..Default::default()
            },
            provider: Some(provider.to_string()),
            tools: None,
            tool_choice: None,
            stream: None,
        };

        let llm_response = self.llm_handler.complete(request).await?;

        let result_json: Value = serde_json::from_str(&llm_response.content)
            .map_err(|e| AriaError::new(
                ErrorCode::LLMInvalidResponse,
                ErrorCategory::Tool,
                ErrorSeverity::High,
                format!("Failed to parse ponderTool LLM response as JSON: {}", e)
            ))?;

        Ok(ToolResult {
            success: true,
            result: Some(result_json),
            error: None,
            metadata: std::collections::HashMap::new(),
            execution_time_ms: 150, // This should be updated with actuals
            resource_usage: None,
        })
    }

    /// Execute the create plan tool (critical for Symphony's planning)
    async fn execute_create_plan_tool(&self, parameters: Value, provider: &str, model: &str) -> AriaResult<ToolResult> {
        let objective = parameters.get("objective")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AriaError::new(
                ErrorCode::InvalidParameters,
                ErrorCategory::Tool,
                ErrorSeverity::Medium,
                "createPlanTool requires 'objective' parameter"
            ))?;

        let context = parameters.get("context").cloned().unwrap_or(Value::Null);
        
        let system_prompt = "You are a meticulous project planning assistant. Your sole purpose is to create a JSON execution plan. Given an objective and a list of available tools, you MUST generate a JSON array of plan steps. Each object in the array represents a single, concrete step and must have these exact keys: 'step', 'useTool', 'tool', 'description', 'parameters'. Your entire output must be a single, raw JSON array.";

        let user_prompt = format!(
            "Create a JSON execution plan for the objective: \"{}\".\n\nContext: {}",
            objective,
            serde_json::to_string_pretty(&context).unwrap_or_default()
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
                    content: user_prompt,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            config: LLMConfig {
                model: Some(model.to_string()),
                temperature: 0.1,
                ..Default::default()
            },
            provider: Some(provider.to_string()),
            tools: None,
            tool_choice: None,
            stream: None,
        };

        let llm_response = self.llm_handler.complete(request).await?;

        let plan_json: Value = serde_json::from_str(&llm_response.content)
            .map_err(|e| AriaError::new(
                ErrorCode::LLMInvalidResponse,
                ErrorCategory::Tool,
                ErrorSeverity::High,
                format!("Failed to parse createPlanTool LLM response as JSON: {}", e)
            ))?;

        Ok(ToolResult {
            success: true,
            result: Some(serde_json::json!({
                "plan": {
                    "generatedPlan": plan_json.to_string(),
                }
            })),
            error: None,
            metadata: std::collections::HashMap::new(),
            execution_time_ms: 250, // This should be updated with actuals
            resource_usage: None,
        })
    }

    /// Execute web search tool
    async fn execute_web_search(&self, parameters: Value) -> AriaResult<ToolResult> {
        let query = parameters.get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AriaError::new(
                ErrorCode::InvalidParameters,
                ErrorCategory::Tool,
                ErrorSeverity::Medium,
                "webSearchTool requires 'query' parameter"
            ))?;

        // TODO: Implement actual web search
        Ok(ToolResult {
            success: true,
            result: Some(serde_json::json!({
                "results": [
                    {
                        "title": format!("Search result for: {}", query),
                        "url": "https://example.com",
                        "snippet": "Mock search result content"
                    }
                ]
            })),
            error: None,
            metadata: std::collections::HashMap::new(),
            execution_time_ms: 500,
            resource_usage: None,
        })
    }

    /// Execute file operations tool
    async fn execute_file_operations(&self, parameters: Value) -> AriaResult<ToolResult> {
        let operation = parameters.get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AriaError::new(
                ErrorCode::InvalidParameters,
                ErrorCategory::Tool,
                ErrorSeverity::Medium,
                "fileOperationsTool requires 'operation' parameter"
            ))?;

        let path = parameters.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AriaError::new(
                ErrorCode::InvalidParameters,
                ErrorCategory::Tool,
                ErrorSeverity::Medium,
                "fileOperationsTool requires 'path' parameter"
            ))?;

        // TODO: Implement actual file operations with proper security checks
        match operation {
            "read" => {
                Ok(ToolResult {
                    success: true,
                    result: Some(serde_json::json!({
                        "content": format!("Mock content of file: {}", path)
                    })),
                    error: None,
                    metadata: std::collections::HashMap::new(),
                    execution_time_ms: 50,
                    resource_usage: None,
                })
            },
            "list" => {
                Ok(ToolResult {
                    success: true,
                    result: Some(serde_json::json!({
                        "files": ["file1.txt", "file2.txt", "directory/"]
                    })),
                    error: None,
                    metadata: std::collections::HashMap::new(),
                    execution_time_ms: 30,
                    resource_usage: None,
                })
            },
            _ => Err(AriaError::new(
                ErrorCode::ToolExecutionFailed,
                ErrorCategory::Tool,
                ErrorSeverity::Medium,
                format!("File operation '{}' not implemented", operation)
            ))
        }
    }
}

/// Trait for tool registry integration
#[async_trait]
pub trait ToolRegistryInterface: Send + Sync {
    async fn execute_tool(&self, name: &str, parameters: Value) -> AriaResult<ToolResult>;
    async fn get_tool_info(&self, name: &str) -> AriaResult<Option<crate::types::RegistryEntry>>;
    async fn list_available_tools(&self) -> AriaResult<Vec<String>>;
}

/// Trait for bundle store integration
#[async_trait]
pub trait BundleStoreInterface: Send + Sync {
    async fn load_bundle(&self, path: &str) -> AriaResult<BundleManifest>;
}

/// Bundle manifest structure
#[derive(Debug, Clone)]
pub struct BundleManifest {
    pub id: String,
    pub version: String,
    pub tools: Vec<ToolManifest>,
}

/// Tool manifest within a bundle
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