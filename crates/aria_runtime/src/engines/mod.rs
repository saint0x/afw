use async_trait::async_trait;
use uuid::Uuid;
use crate::deep_size::{DeepUuid, DeepValue};
use crate::errors::AriaResult;
use crate::types::{
    AgentConfig, ContainerSpec, ExecutionPlan, ExecutionStep, PlannedStep, Reflection,
    RuntimeContext, TaskAnalysis, ToolResult, ConversationJSON, ExecutionStatus, 
    MemoryUsage, ResourceUsage,
};
use crate::engines::execution::ExecutionEngine;
use crate::engines::planning::PlanningEngine;
use crate::engines::conversation::ConversationEngine;
use crate::engines::reflection::ReflectionEngine;
use crate::engines::context_manager::ContextManagerEngine;
use crate::engines::llm::{LLMHandler, LLMHandlerInterface};
use crate::engines::tool_registry::ToolRegistry;
use crate::engines::system_prompt::SystemPromptService;
use crate::engines::container::quilt::QuiltService;
use crate::engines::icc::ICCEngine;
use crate::engines::intelligence::{IntelligenceEngine, IntelligenceConfig};
use crate::engines::observability::ObservabilityManager;
use crate::engines::streaming::StreamingService;

use crate::database::{DatabaseManager, DatabaseConfig};
use std::collections::HashMap;
use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::Mutex;

pub mod execution;
pub mod planning;
pub mod conversation;
pub mod reflection;
pub mod context_manager;
pub mod llm;
pub mod tool_registry;
pub mod system_prompt;
pub mod container;
pub mod config;
pub mod icc;
pub mod context;
pub mod observability;
pub mod observability_endpoints;
pub mod streaming;
pub mod intelligence;

/// Main orchestrator for all Aria runtime engines
pub struct AriaEngines {
    pub execution: Arc<ExecutionEngine>,
    pub planning: Arc<PlanningEngine>,
    pub conversation: Arc<ConversationEngine>,
    pub reflection: Arc<ReflectionEngine>,
    pub context_manager: Arc<ContextManagerEngine>,
    pub llm_handler: Arc<LLMHandler>,
    pub tool_registry: Arc<ToolRegistry>,
    pub system_prompt: Arc<SystemPromptService>,
    pub quilt_service: Arc<Mutex<QuiltService>>,
    pub icc_engine: Arc<ICCEngine>,
    pub database: Arc<DatabaseManager>,
    pub observability: Arc<ObservabilityManager>,
    pub streaming: Arc<StreamingService>,
    pub intelligence: Arc<IntelligenceEngine>,
    pub pkg_store: Arc<pkg_store::PackageStore>,
}

impl AriaEngines {
    /// Initialize all engines
    pub async fn initialize_all(&self) -> AriaResult<()> {
        // Initialize engines in dependency order
        // First initialize core services
        self.observability.start().await?;
        self.streaming.start().await?;
        
        // Then initialize other engines
        if !self.execution.initialize() {
            return Err(crate::errors::AriaError::new(
                crate::errors::ErrorCode::EngineInitializationFailed,
                crate::errors::ErrorCategory::Engine,
                crate::errors::ErrorSeverity::High,
                "Failed to initialize execution engine"
            ));
        }
        
        if !self.planning.initialize() {
            return Err(crate::errors::AriaError::new(
                crate::errors::ErrorCode::EngineInitializationFailed,
                crate::errors::ErrorCategory::Engine,
                crate::errors::ErrorSeverity::High,
                "Failed to initialize planning engine"
            ));
        }
        
        if !self.conversation.initialize() {
            return Err(crate::errors::AriaError::new(
                crate::errors::ErrorCode::EngineInitializationFailed,
                crate::errors::ErrorCategory::Engine,
                crate::errors::ErrorSeverity::High,
                "Failed to initialize conversation engine"
            ));
        }
        
        if !self.reflection.initialize() {
            return Err(crate::errors::AriaError::new(
                crate::errors::ErrorCode::EngineInitializationFailed,
                crate::errors::ErrorCategory::Engine,
                crate::errors::ErrorSeverity::High,
                "Failed to initialize reflection engine"
            ));
        }
        
        if !self.context_manager.initialize() {
            return Err(crate::errors::AriaError::new(
                crate::errors::ErrorCode::EngineInitializationFailed,
                crate::errors::ErrorCategory::Engine,
                crate::errors::ErrorSeverity::High,
                "Failed to initialize context manager"
            ));
        }
        
        if !self.icc_engine.initialize() {
            return Err(crate::errors::AriaError::new(
                crate::errors::ErrorCode::EngineInitializationFailed,
                crate::errors::ErrorCategory::Engine,
                crate::errors::ErrorSeverity::High,
                "Failed to initialize ICC engine"
            ));
        }
        
        if !self.intelligence.initialize() {
            return Err(crate::errors::AriaError::new(
                crate::errors::ErrorCode::EngineInitializationFailed,
                crate::errors::ErrorCategory::Engine,
                crate::errors::ErrorSeverity::High,
                "Failed to initialize intelligence engine"
            ));
        }
        
        Ok(())
    }

    /// Shutdown all engines gracefully
    pub async fn shutdown_all(&self) -> AriaResult<()> {
        // Shutdown engines in reverse dependency order
        let mut shutdown_errors = Vec::new();
        
        if !self.icc_engine.shutdown() {
            shutdown_errors.push("ICC engine shutdown failed");
        }
        
        if !self.intelligence.shutdown() {
            shutdown_errors.push("Intelligence engine shutdown failed");
        }
        
        if !self.context_manager.shutdown() {
            shutdown_errors.push("Context manager shutdown failed");
        }
        
        if !self.reflection.shutdown() {
            shutdown_errors.push("Reflection engine shutdown failed");
        }
        
        if !self.conversation.shutdown() {
            shutdown_errors.push("Conversation engine shutdown failed");
        }
        
        if !self.planning.shutdown() {
            shutdown_errors.push("Planning engine shutdown failed");
        }
        
        if !self.execution.shutdown() {
            shutdown_errors.push("Execution engine shutdown failed");
        }
        
        // Shutdown observability and streaming services last
        if let Err(e) = self.streaming.stop().await {
            shutdown_errors.push("Streaming service shutdown failed");
        }
        
        if let Err(e) = self.observability.stop().await {
            shutdown_errors.push("Observability service shutdown failed");
        }
        
        if !shutdown_errors.is_empty() {
            return Err(crate::errors::AriaError::new(
                crate::errors::ErrorCode::EngineShutdownFailed,
                crate::errors::ErrorCategory::Engine,
                crate::errors::ErrorSeverity::Medium,
                &format!("Some engines failed to shutdown: {}", shutdown_errors.join(", "))
            ));
        }
        
        Ok(())
    }

    /// Health check for all engines
    pub async fn health_check_all(&self) -> AriaResult<crate::engines::observability::HealthStatus> {
        let health_status = self.observability.get_health().await;
        
        // Perform basic checks on other engines
        let all_healthy = self.execution.health_check() &&
            self.planning.health_check() &&
            self.conversation.health_check() &&
            self.reflection.health_check() &&
            self.context_manager.health_check() &&
            self.icc_engine.health_check() &&
            self.intelligence.health_check();
        
        if all_healthy {
            Ok(health_status)
        } else {
            Err(crate::errors::AriaError::new(
                crate::errors::ErrorCode::HealthCheckFailed,
                crate::errors::ErrorCategory::Engine,
                crate::errors::ErrorSeverity::Medium,
                "One or more engines failed health check"
            ))
        }
    }
    
    /// Get runtime metrics
    pub async fn get_metrics(&self) -> crate::engines::observability::RuntimeMetrics {
        self.observability.get_metrics().await
    }
    
    /// Record an error for observability
    pub async fn record_error(&self, error: &crate::errors::AriaError, component: &str, context: HashMap<String, String>) -> AriaResult<()> {
        self.observability.record_error(error, component, context).await
    }
    
    /// Record a tool execution for observability
    pub async fn record_tool_execution(&self, tool_name: &str, session_id: &str, duration_ms: u64, success: bool, error: Option<String>) -> AriaResult<()> {
        self.observability.record_tool_execution(tool_name, session_id, duration_ms, success, error).await
    }
    
    /// Record a container event for observability
    pub async fn record_container_event(&self, container_id: &str, event_type: &str, metadata: HashMap<String, String>) -> AriaResult<()> {
        self.observability.record_container_event(container_id, event_type, metadata).await
    }
    
    /// Record an agent execution for observability
    pub async fn record_agent_execution(&self, session_id: &str, agent_name: &str, step_count: u32, tokens_used: u32, duration_ms: u64, success: bool) -> AriaResult<()> {
        self.observability.record_agent_execution(session_id, agent_name, step_count, tokens_used, duration_ms, success).await
    }

    /// Get intelligent container configuration
    pub async fn get_intelligent_container_config(&self, request: &str, session_id: &str) -> AriaResult<crate::engines::intelligence::ContainerConfig> {
        use crate::engines::intelligence::{ContainerRequest, RecommendationAction, generate_id};
        
        let container_request = ContainerRequest {
            request_id: generate_id(),
            session_id: session_id.to_string(),
            description: request.to_string(),
            requirements: None,
            context_hints: Vec::new(),
        };

        let intelligence_result = self.intelligence.manager().analyze_container_request(&container_request, &session_id).await?;

        match intelligence_result.recommendation.action {
            RecommendationAction::UsePattern => {
                Ok(intelligence_result.pattern_match.unwrap().container_config)
            },
            RecommendationAction::CreateNew => {
                self.create_new_container_config(request).await
            },
            RecommendationAction::OptimizeExisting => {
                self.optimize_container_config(request, &intelligence_result.context_summary).await
            },
            RecommendationAction::RequestMoreInfo => {
                // Fall back to new config creation
                self.create_new_container_config(request).await
            },
        }
    }

    /// Create a new container configuration (Phase 1 implementation)
    async fn create_new_container_config(&self, request: &str) -> AriaResult<crate::engines::intelligence::ContainerConfig> {
        use crate::engines::intelligence::ContainerConfig;
        use std::collections::HashMap;
        
        // Phase 1 implementation - basic container config creation
        // Will be enhanced with more intelligent analysis in later phases
        
        let config = ContainerConfig {
            image: "ubuntu:22.04".to_string(), // Default to Ubuntu for Phase 1
            command: vec!["/bin/bash".to_string()],
            environment: HashMap::new(),
            working_directory: Some("/workspace".to_string()),
            resource_limits: None, // Will use system defaults
            network_config: None,
            volumes: Vec::new(),
        };

        tracing::info!("Created new container config for request: {}", request);
        Ok(config)
    }

    /// Optimize existing container configuration (Phase 1 implementation)
    async fn optimize_container_config(&self, request: &str, context_summary: &str) -> AriaResult<crate::engines::intelligence::ContainerConfig> {
        // Phase 1 implementation - basic optimization
        // Full optimization will be implemented in later phases
        tracing::debug!("Optimizing container config with context: {}", context_summary);
        self.create_new_container_config(request).await
    }
}

/// Wrapper to implement LLMHandlerInterface for LLMHandler
pub struct LLMHandlerWrapper {
    handler: std::sync::Arc<LLMHandler>,
}

impl LLMHandlerWrapper {
    pub fn new(handler: std::sync::Arc<LLMHandler>) -> Self {
        Self { handler }
    }
}

#[async_trait]
impl LLMHandlerInterface for LLMHandlerWrapper {
    async fn complete(&self, request: crate::engines::llm::types::LLMRequest) -> AriaResult<crate::engines::llm::types::LLMResponse> {
        self.handler.complete(request).await
    }
    
    async fn stream_complete(&self, _request: crate::engines::llm::types::LLMRequest) -> AriaResult<crate::engines::llm::types::LLMStreamResponse> {
        // For now, return an error since streaming isn't fully implemented
        Err(crate::errors::AriaError::new(
            crate::errors::ErrorCode::LLMApiError,
            crate::errors::ErrorCategory::LLM,
            crate::errors::ErrorSeverity::Medium,
            "Streaming not yet implemented"
        ))
    }
    
    fn get_providers(&self) -> Vec<String> {
        // Use the sync method from the singleton pattern
        self.handler.get_available_providers()
    }
    
    async fn set_default_provider(&self, _provider: &str) -> AriaResult<()> {
        // TODO: Implement when provider selection is added
        Ok(())
    }
    
    async fn get_provider_capabilities(&self, _provider: &str) -> AriaResult<crate::engines::llm::types::ProviderCapabilities> {
        // TODO: Implement when provider capabilities are added
        Ok(crate::engines::llm::types::ProviderCapabilities {
            models: vec!["gpt-4o".to_string()],
            supports_streaming: true,
            supports_functions: true,
            supports_vision: false,
            max_tokens: 4096,
            rate_limits: Some(crate::engines::llm::types::RateLimits {
                requests_per_minute: 5000,
                tokens_per_minute: 600000,
                requests_per_day: Some(100000),
            }),
        })
    }
    
    async fn health_check_provider(&self, _provider: &str) -> AriaResult<bool> {
        // Simple health check - check if any providers are available
        let providers = self.handler.get_available_providers();
        Ok(!providers.is_empty())
    }
}

/// Base trait for all runtime engines
/// Simplified to avoid trait object compatibility issues
pub trait Engine: Send + Sync {
    /// Get current state of the engine
    fn get_state(&self) -> String;
    
    /// Get list of dependencies this engine requires
    fn get_dependencies(&self) -> Vec<String>;
    
    /// Perform health check (simplified to sync)
    fn health_check(&self) -> bool;
    
    /// Initialize the engine (simplified to sync)
    fn initialize(&self) -> bool;
    
    /// Shutdown the engine gracefully (simplified to sync)
    fn shutdown(&self) -> bool;
}

/// Interface for execution engines
#[async_trait]
pub trait ExecutionEngineInterface: Engine {
    /// Execute a task using the agent configuration and context
    async fn execute(
        &self,
        task: &str,
        agent_config: &AgentConfig,
        context: &RuntimeContext,
    ) -> AriaResult<ToolResult>;

    /// Execute a single planned step
    async fn execute_step(
        &self,
        step: &PlannedStep,
        context: &RuntimeContext,
    ) -> AriaResult<ExecutionStep>;

    /// Execute a container workload
    async fn execute_container_workload(
        &self,
        spec: &ContainerSpec,
        exec_command: &Vec<String>,
        context: Option<&RuntimeContext>,
        session_id: DeepUuid,
    ) -> AriaResult<ToolResult>;

    /// Detect if a task requires multi-tool orchestration
    fn detect_multi_tool_requirement(&self, task: &str) -> bool;
    
    /// Resolve parameter placeholders using execution history
    fn resolve_placeholders(
        &self,
        parameters: &HashMap<String, DeepValue>,
        history: &[ExecutionStep],
    ) -> AriaResult<HashMap<String, DeepValue>>;
}

/// Interface for planning engines
#[async_trait]
pub trait PlanningEngineInterface: Engine {
    /// Analyze task complexity
    async fn analyze_task(
        &self,
        task: &str,
        context: &RuntimeContext,
    ) -> AriaResult<TaskAnalysis>;
    
    /// Create execution plan
    async fn create_execution_plan(
        &self,
        task: &str,
        agent_config: &AgentConfig,
        context: &RuntimeContext,
    ) -> AriaResult<ExecutionPlan>;
}

/// Interface for conversation engines
#[async_trait]
pub trait ConversationEngineInterface: Send + Sync {
    /// Initiate conversation
    async fn initiate(
        &self,
        task: &str,
        context: &RuntimeContext,
    ) -> AriaResult<ConversationJSON>;
    
    /// Update conversation
    async fn update(
        &self,
        conversation: &mut ConversationJSON,
        step_result: &ExecutionStep,
    ) -> AriaResult<()>;
    
    /// Conclude conversation
    async fn conclude(
        &self,
        conversation: &mut ConversationJSON,
        context: &RuntimeContext,
    ) -> AriaResult<()>;
    
    /// Finalize conversation
    async fn finalize(
        &self,
        conversation: &mut ConversationJSON,
    ) -> AriaResult<()>;
}

/// Interface for reflection engines
#[async_trait]
pub trait ReflectionEngineInterface: Engine {
    /// Reflect on execution step
    async fn reflect(
        &self,
        step: &ExecutionStep,
        context: &RuntimeContext,
    ) -> AriaResult<Reflection>;
}

/// Interface for context management engines
#[async_trait]
pub trait ContextManagerInterface: Send + Sync {
    /// Set execution plan
    async fn set_plan(&self, plan: ExecutionPlan) -> AriaResult<()>;
    
    /// Record execution step
    async fn record_step(&self, step: ExecutionStep) -> AriaResult<()>;
    
    /// Record reflection
    async fn record_reflection(&self, reflection: Reflection) -> AriaResult<()>;
    
    /// Update execution status
    async fn update_status(&self, status: ExecutionStatus) -> AriaResult<()>;
    
    /// Get execution state
    async fn get_execution_state(&self) -> AriaResult<RuntimeContext>;

    /// Get current memory usage
    async fn get_memory_usage(&self) -> AriaResult<MemoryUsage>;

    /// Serialize the entire runtime context
    async fn serialize_context(&self, format: context_manager::SerializationFormat) -> AriaResult<Vec<u8>>;
}

/// Interface for the container manager
#[async_trait]
pub trait ContainerManagerInterface: Send + Sync {
    async fn create_container(
        &self,
        spec: &ContainerSpec,
        session_id: uuid::Uuid,
        environment: std::collections::HashMap<String, String>,
    ) -> AriaResult<String>;
    
    async fn execute_in_container(
        &self,
        container_id: &str,
        command: &[String],
        timeout_seconds: u64,
    ) -> AriaResult<ContainerExecutionResult>;
    
    async fn cleanup_container(&self, container_id: &str) -> AriaResult<()>;
    
    async fn get_container_logs(&self, container_id: &str) -> AriaResult<String>;
    
    async fn health_check(&self) -> AriaResult<bool>;
}

// Result of executing a command in a container
pub struct ContainerExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub execution_time_ms: u64,
    pub resource_usage: Option<ResourceUsage>,
}

// Represents the status of a container
pub struct ContainerStatus {
    pub id: String,
    pub state: ContainerState,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub exit_code: Option<i32>,
    pub resource_usage: Option<ResourceUsage>,
}

// Represents the possible states of a container
pub enum ContainerState {
    Created,
    Running,
    Stopped,
    Failed,
    Removed,
}

// Detailed information about a container
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: ContainerStatus,
    pub session_id: Uuid,
}

// Status of the Inter-Container Communication (ICC) server
#[derive(Debug)]
pub enum ICCServerStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Error(String),
}

// Represents an active connection to the ICC server
pub struct ICCConnection {
    pub id: String,
    pub container_id: String,
    pub connected_at: u64,
    pub last_activity: u64,
    pub request_count: u32,
}

// Handler for tool calls coming from the ICC
#[async_trait]
pub trait ICCToolHandler: Send + Sync {
    async fn handle_tool_call(
        &self,
        tool_name: &str,
        parameters: serde_json::Value,
        container_id: &str,
        session_id: Uuid,
    ) -> AriaResult<serde_json::Value>;
}

// Handler for agent calls coming from the ICC
#[async_trait]
pub trait ICCAgentHandler: Send + Sync {
    async fn handle_agent_call(
        &self,
        agent_name: &str,
        message: &str,
        container_id: &str,
        session_id: Uuid,
    ) -> AriaResult<String>;
}

// Re-export observability types
pub use observability::{RuntimeMetrics, HealthStatus};
pub use streaming::{StreamingConfig, StreamType};

 