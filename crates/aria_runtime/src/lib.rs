/*!
# Aria Runtime - Core Aria Runtime

This crate contains the core agentic runtime.
It provides sophisticated planning, execution, reflection, and learning capabilities
for agentic programs running inside the Aria Firmware.

## Architecture

The runtime consists of several key components:

- **Runtime Engine**: Main execution orchestrator with multi-phase lifecycle
- **Planning Engine**: Task decomposition and execution plan creation  
- **Reflection Engine**: Self-correction and adaptation capabilities
- **Tool Registry**: Tool discovery, validation, and execution
- **Agent Manager**: Agent lifecycle and execution management
- **Team Coordinator**: Multi-agent collaboration and coordination
- **Memory System**: Short-term and long-term memory management
- **Cache Intelligence**: Pattern matching and optimization
- **Context Manager**: Execution context and learning state
*/

#![doc = include_str!("../../../ARIARUNTIME.md")]

pub mod context;
pub mod deep_size;
pub mod engines;
pub mod errors;
pub mod memory;
pub mod planning;
pub mod reflection;
pub mod runtime;
pub mod tools;
pub mod types;

use std::sync::Arc;
use futures::future::BoxFuture;
use tokio::sync::Mutex;

pub use errors::{AriaError, AriaResult};
pub use types::{RuntimeConfiguration, RuntimeResult, ContainerSpec, ToolResult, RuntimeContext};
pub use runtime::AriaRuntime;
pub use deep_size::DeepUuid;

use crate::engines::{
    AriaEngines,
    execution::ExecutionEngine,
    planning::PlanningEngine,
    conversation::ConversationEngine,
    reflection::ReflectionEngine,
    tool_registry::{ToolRegistry, ToolRegistryInterface},
    context_manager::ContextManagerEngine,
    llm::LLMHandler,
    system_prompt::SystemPromptService,
    container::quilt::QuiltService,
    config::QuiltConfig,
    ExecutionEngineInterface,
    LLMHandlerWrapper,
};

/// Runtime version
pub const RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Create a new Aria Runtime with default configuration
pub async fn create_aria_runtime_default() -> AriaResult<AriaRuntime> {
    // For now, this is not implemented since we need actual engine implementations
    Err(AriaError::new(
        errors::ErrorCode::InitializationFailed,
        errors::ErrorCategory::System,
        errors::ErrorSeverity::Critical,
        "Engine factory not yet implemented - runtime creation not available",
    ))
}

/// Create a new Aria Runtime with custom configuration
pub async fn create_aria_runtime(_config: RuntimeConfiguration) -> AriaResult<AriaRuntime> {
    // For now, this is not implemented since we need actual engine implementations
    Err(AriaError::new(
        errors::ErrorCode::InitializationFailed,
        errors::ErrorCategory::System,
        errors::ErrorSeverity::Critical,
        "Engine factory not yet implemented - runtime creation not available",
    ))
}

impl AriaEngines {
    pub async fn new() -> Self {
        // 1. Core services that everything else depends on
        let llm_handler = LLMHandler::get_instance();
        let quilt_config = QuiltConfig {
            endpoint: "http://127.0.0.1:50051".to_string(),
        };
        let quilt_service = Arc::new(Mutex::new(
            QuiltService::new(&quilt_config)
                .await
                .expect("Failed to connect to Quilt daemon"),
        ));
        let system_prompt = Arc::new(SystemPromptService::new());

        // 2. Tool Registry, which depends on core services
        let tool_registry = Arc::new(ToolRegistry::new(
            llm_handler.clone(),
            quilt_service.clone(),
        ).await);

        // 3. Other engines, which depend on tool registry and core services
        let execution = Arc::new(ExecutionEngine::new(
            tool_registry.clone(),
            llm_handler.clone(),
            quilt_service.clone(),
        ));

        let planning = Arc::new(PlanningEngine::new(tool_registry.clone()));

        let conversation = Arc::new(ConversationEngine::new(Some(Box::new(
            LLMHandlerWrapper::new(llm_handler.clone()),
        ))));

        let reflection = Arc::new(ReflectionEngine::new(tool_registry.clone()));

        let context_manager = Arc::new(ContextManagerEngine::new(
            crate::types::AgentConfig::default(),
        ));

        // 4. Assemble the final struct
        Self {
            execution,
            planning,
            conversation,
            reflection,
            context_manager,
            llm_handler,
            tool_registry,
            system_prompt,
            quilt_service,
        }
    }
}

impl AriaRuntime {
    pub async fn execute_container_workload(
        &self,
        spec: &ContainerSpec,
        exec_command: &Vec<String>,
        context: Option<&RuntimeContext>,
        session_id: DeepUuid,
    ) -> AriaResult<ToolResult> {
        self.engines.execution.execute_container_workload(spec, exec_command, context, session_id).await
    }
} 