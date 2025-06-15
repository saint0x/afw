use async_trait::async_trait;
use uuid::Uuid;

use crate::types::*;
use crate::errors::AriaResult;

pub mod execution;
pub mod planning;
pub mod llm;
pub mod conversation;
pub mod reflection;
pub mod context;
pub mod registry;
pub mod container;
pub mod icc;
pub mod tool_registry;
pub mod system_prompt;

// Re-export key types and traits for external use
pub use execution::{ExecutionEngine, OrchestrationStepResult};
pub use tool_registry::{ToolRegistry, ToolRegistryInterface};
pub use llm::{LLMHandler, LLMProvider, OpenAIProvider, AnthropicProvider, LLMHandlerInterface};

/// Factory for creating engine instances
pub struct EngineFactory;

impl EngineFactory {
    /// Create a complete set of Aria engines with proper dependencies
    pub async fn create_engines(config: &RuntimeConfiguration) -> AriaResult<AriaEngines> {
        // Create LLM handler
        let llm_handler = Box::new(
            LLMHandler::new()
                .with_timeout(std::time::Duration::from_secs(30))
                .with_retry_attempts(3)
        );

        // Create production tool registry
        let tool_registry = std::sync::Arc::new(ToolRegistry::new(None));

        // TODO: Create container manager if enabled
        let container_manager = if config.container_execution_enabled {
            // Some(Box::new(ContainerManager::new()))
            None // For now
        } else {
            None
        };

        // Create execution engine
        let execution_engine = Box::new(ExecutionEngine::new(
            tool_registry,
            llm_handler,
            container_manager,
        ));

        // Create planning engine (placeholder)
        let planning_engine = Box::new(MockPlanningEngine::new());
        
        // Create conversation engine (placeholder)
        let conversation_engine = Box::new(MockConversationEngine::new());
        
        // Create reflection engine (placeholder)
        let reflection_engine = Box::new(MockReflectionEngine::new());
        
        // Create context manager (placeholder)
        let context_manager = Box::new(MockContextManager::new());

        Ok(AriaEngines {
            execution: execution_engine,
            planning: planning_engine,
            conversation: conversation_engine,
            reflection: reflection_engine,
            context_manager: context_manager,
        })
    }
}

/// Collection of all Aria runtime engines
pub struct AriaEngines {
    pub execution: Box<dyn ExecutionEngineInterface>,
    pub planning: Box<dyn PlanningEngineInterface>,
    pub conversation: Box<dyn ConversationEngineInterface>,
    pub reflection: Box<dyn ReflectionEngineInterface>,
    pub context_manager: Box<dyn ContextManagerInterface>,
}

impl AriaEngines {
    /// Initialize all engines
    pub async fn initialize_all(&self) -> AriaResult<()> {
        self.execution.initialize().await?;
        self.planning.initialize().await?;
        self.conversation.initialize().await?;
        self.reflection.initialize().await?;
        self.context_manager.initialize().await?;
        Ok(())
    }
    
    /// Shutdown all engines
    pub async fn shutdown_all(&self) -> AriaResult<()> {
        self.execution.shutdown().await?;
        self.planning.shutdown().await?;
        self.conversation.shutdown().await?;
        self.reflection.shutdown().await?;
        self.context_manager.shutdown().await?;
        Ok(())
    }
    
    /// Health check all engines
    pub async fn health_check_all(&self) -> AriaResult<bool> {
        let checks = vec![
            self.execution.health_check().await?,
            self.planning.health_check().await?,
            self.conversation.health_check().await?,
            self.reflection.health_check().await?,
            self.context_manager.health_check().await?,
        ];
        Ok(checks.iter().all(|&check| check))
    }
}

/// Base trait for all runtime engines
#[async_trait]
pub trait RuntimeEngine: Send + Sync {
    /// Initialize the engine
    async fn initialize(&self) -> AriaResult<()>;
    
    /// Get list of dependencies this engine requires
    fn get_dependencies(&self) -> Vec<String>;
    
    /// Get current state of the engine
    fn get_state(&self) -> String;
    
    /// Perform health check
    async fn health_check(&self) -> AriaResult<bool>;
    
    /// Shutdown the engine gracefully
    async fn shutdown(&self) -> AriaResult<()>;
}

/// Interface for execution engines
#[async_trait]
pub trait ExecutionEngineInterface: RuntimeEngine {
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
        context: &RuntimeContext,
        session_id: uuid::Uuid,
    ) -> AriaResult<ToolResult>;

    /// Detect if a task requires multi-tool orchestration
    fn detect_multi_tool_requirement(&self, task: &str) -> bool;
    
    /// Resolve parameter placeholders using execution history
    fn resolve_placeholders(
        &self,
        parameters: &std::collections::HashMap<String, serde_json::Value>,
        history: &[ExecutionStep],
    ) -> AriaResult<std::collections::HashMap<String, serde_json::Value>>;
}

/// Interface for planning engines
#[async_trait]
pub trait PlanningEngineInterface: RuntimeEngine {
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
pub trait ConversationEngineInterface: RuntimeEngine {
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
pub trait ReflectionEngineInterface: RuntimeEngine {
    /// Reflect on execution step
    async fn reflect(
        &self,
        step: &ExecutionStep,
        context: &RuntimeContext,
    ) -> AriaResult<Reflection>;
}

/// Interface for context managers
#[async_trait]
pub trait ContextManagerInterface: RuntimeEngine {
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
}

/// Mock tool registry for development
pub struct MockToolRegistry {
    tools: std::collections::HashMap<String, String>,
}

impl MockToolRegistry {
    pub fn new() -> Self {
        let mut tools = std::collections::HashMap::new();
        tools.insert("test_tool".to_string(), "A test tool for development".to_string());
        tools.insert("echo_tool".to_string(), "Echoes input back".to_string());
        
        Self { tools }
    }
}

#[async_trait]
impl ToolRegistryInterface for MockToolRegistry {
    async fn execute_tool(&self, name: &str, parameters: serde_json::Value) -> AriaResult<ToolResult> {
        match name {
            "test_tool" => Ok(ToolResult {
                success: true,
                result: Some(serde_json::json!({
                    "message": "Test tool executed successfully",
                    "parameters": parameters
                })),
                error: None,
                metadata: std::collections::HashMap::new(),
                execution_time_ms: 10,
                resource_usage: None,
            }),
            "echo_tool" => Ok(ToolResult {
                success: true,
                result: Some(serde_json::json!({
                    "echo": parameters,
                    "message": "Input echoed back successfully"
                })),
                error: None,
                metadata: std::collections::HashMap::new(),
                execution_time_ms: 5,
                resource_usage: None,
            }),
            _ => Ok(ToolResult {
                success: false,
                result: None,
                error: Some(format!("Tool '{}' not found", name)),
                metadata: std::collections::HashMap::new(),
                execution_time_ms: 0,
                resource_usage: None,
            }),
        }
    }

    async fn get_tool_info(&self, name: &str) -> AriaResult<Option<RegistryEntry>> {
        if let Some(_description) = self.tools.get(name) {
            Ok(Some(RegistryEntry {
                name: name.to_string(),
                entry_type: RegistryEntryType::Tool,
                bundle_id: None,
                version: "1.0.0".to_string(),
                metadata: std::collections::HashMap::new(),
                created_at: 0,
                updated_at: 0,
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_available_tools(&self) -> AriaResult<Vec<String>> {
        Ok(self.tools.keys().cloned().collect())
    }
}

// Mock implementations for missing engines
pub struct MockPlanningEngine;
impl MockPlanningEngine {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl RuntimeEngine for MockPlanningEngine {
    async fn initialize(&self) -> AriaResult<()> { Ok(()) }
    fn get_dependencies(&self) -> Vec<String> { vec![] }
    fn get_state(&self) -> String { "ready".to_string() }
    async fn health_check(&self) -> AriaResult<bool> { Ok(true) }
    async fn shutdown(&self) -> AriaResult<()> { Ok(()) }
}

#[async_trait]
impl PlanningEngineInterface for MockPlanningEngine {
    async fn analyze_task(&self, _task: &str, _context: &RuntimeContext) -> AriaResult<TaskAnalysis> {
        Ok(TaskAnalysis {
            complexity: TaskComplexity::Simple,
            requires_planning: false,
            requires_containers: false,
            reasoning: "Mock analysis".to_string(),
            estimated_steps: 1,
        })
    }
    
    async fn create_execution_plan(&self, task: &str, _agent_config: &AgentConfig, _context: &RuntimeContext) -> AriaResult<ExecutionPlan> {
        Ok(ExecutionPlan {
            id: uuid::Uuid::new_v4(),
            task_description: task.to_string(),
            steps: vec![],
            confidence: 0.5,
            estimated_duration: Some(30000),
            resource_requirements: ResourceRequirements {
                cpu_millis: 100,
                memory_mb: 64,
                disk_mb: 10,
                network_bandwidth_kbps: None,
                container_count: 0,
                cpu_cores: Some(1),
                timeout_seconds: Some(30),
                max_concurrent: Some(1),
            },
        })
    }
}

pub struct MockConversationEngine;
impl MockConversationEngine {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl RuntimeEngine for MockConversationEngine {
    async fn initialize(&self) -> AriaResult<()> { Ok(()) }
    fn get_dependencies(&self) -> Vec<String> { vec![] }
    fn get_state(&self) -> String { "ready".to_string() }
    async fn health_check(&self) -> AriaResult<bool> { Ok(true) }
    async fn shutdown(&self) -> AriaResult<()> { Ok(()) }
}

#[async_trait]
impl ConversationEngineInterface for MockConversationEngine {
    async fn initiate(&self, task: &str, _context: &RuntimeContext) -> AriaResult<ConversationJSON> {
        Ok(ConversationJSON {
            id: uuid::Uuid::new_v4(),
            original_task: task.to_string(),
            turns: vec![],
            final_response: String::new(),
            reasoning_chain: vec![],
            duration: 0,
            state: ConversationState::Initiated,
        })
    }
    
    async fn update(&self, _conversation: &mut ConversationJSON, _step_result: &ExecutionStep) -> AriaResult<()> {
        Ok(())
    }
    
    async fn conclude(&self, _conversation: &mut ConversationJSON, _context: &RuntimeContext) -> AriaResult<()> {
        Ok(())
    }
    
    async fn finalize(&self, _conversation: &mut ConversationJSON) -> AriaResult<()> {
        Ok(())
    }
}

pub struct MockReflectionEngine;
impl MockReflectionEngine {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl RuntimeEngine for MockReflectionEngine {
    async fn initialize(&self) -> AriaResult<()> { Ok(()) }
    fn get_dependencies(&self) -> Vec<String> { vec![] }
    fn get_state(&self) -> String { "ready".to_string() }
    async fn health_check(&self) -> AriaResult<bool> { Ok(true) }
    async fn shutdown(&self) -> AriaResult<()> { Ok(()) }
}

#[async_trait]
impl ReflectionEngineInterface for MockReflectionEngine {
    async fn reflect(&self, _step: &ExecutionStep, _context: &RuntimeContext) -> AriaResult<Reflection> {
        Ok(Reflection {
            id: uuid::Uuid::new_v4(),
            step_id: uuid::Uuid::new_v4(),
            assessment: ReflectionAssessment {
                performance: PerformanceLevel::Good,
                quality: QualityLevel::Good,
                efficiency: EfficiencyLevel::Efficient,
                suggested_improvements: vec![],
            },
            suggested_action: SuggestedAction::Continue,
            reasoning: "Mock reflection".to_string(),
            confidence: 0.5,
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            improvements: vec![],
        })
    }
}

pub struct MockContextManager;
impl MockContextManager {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl RuntimeEngine for MockContextManager {
    async fn initialize(&self) -> AriaResult<()> { Ok(()) }
    fn get_dependencies(&self) -> Vec<String> { vec![] }
    fn get_state(&self) -> String { "ready".to_string() }
    async fn health_check(&self) -> AriaResult<bool> { Ok(true) }
    async fn shutdown(&self) -> AriaResult<()> { Ok(()) }
}

#[async_trait]
impl ContextManagerInterface for MockContextManager {
    async fn set_plan(&self, _plan: ExecutionPlan) -> AriaResult<()> { Ok(()) }
    async fn record_step(&self, _step: ExecutionStep) -> AriaResult<()> { Ok(()) }
    async fn record_reflection(&self, _reflection: Reflection) -> AriaResult<()> { Ok(()) }
    async fn update_status(&self, _status: ExecutionStatus) -> AriaResult<()> { Ok(()) }
    async fn get_execution_state(&self) -> AriaResult<RuntimeContext> {
        Ok(RuntimeContext {
            session_id: uuid::Uuid::new_v4(),
            agent_config: AgentConfig {
                name: "system_agent".to_string(),
                tools: vec![],
                agents: vec![],
                llm: LLMConfig {
                    provider: "openai".to_string(),
                    model: "gpt-4o-mini".to_string(),
                    api_key: None,
                    temperature: Some(0.7),
                    max_tokens: Some(1000),
                    timeout: None,
                },
                system_prompt: None,
                directives: None,
                max_iterations: None,
                timeout_ms: None,
                memory_limit: None,
                agent_type: Some("system".to_string()),
                capabilities: vec![],
                memory_enabled: Some(false),
            },
            created_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            conversation: None,
            status: ExecutionStatus::Running,
            current_plan: None,
            execution_history: vec![],
            working_memory: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            insights: vec![],
            error_history: vec![],
            current_step: 0,
            total_steps: 0,
            remaining_steps: vec![],
            reflections: vec![],
            memory_size: 0,
            max_memory_size: 1024 * 1024, // 1MB
        })
    }
}

/// Trait for container management interfaces
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

#[derive(Debug, Clone)]
pub struct ContainerExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub execution_time_ms: u64,
    pub resource_usage: Option<ResourceUsage>,
}

#[derive(Debug, Clone)]
pub struct ContainerStatus {
    pub id: String,
    pub state: ContainerState,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub exit_code: Option<i32>,
    pub resource_usage: Option<ResourceUsage>,
}

#[derive(Debug, Clone)]
pub enum ContainerState {
    Created,
    Running,
    Stopped,
    Failed,
    Removed,
}

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: ContainerStatus,
    pub session_id: Uuid,
}

#[derive(Debug, Clone)]
pub enum ICCServerStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct ICCConnection {
    pub id: String,
    pub container_id: String,
    pub connected_at: u64,
    pub last_activity: u64,
    pub request_count: u32,
}

/// Handler trait for ICC tool execution
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

/// Handler trait for ICC agent invocation
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

 