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
        let llm_handler = LLMHandler::get_instance();
        let tool_registry = ToolRegistry::new(Arc::clone(&llm_handler));
        let system_prompt = SystemPromptService::new();
        let quilt_config = QuiltConfig {
            endpoint: "http://127.0.0.1:50051".to_string(),
        };
        let quilt_service = QuiltService::new(&quilt_config).await.expect("Failed to connect to Quilt daemon");

        let execution = ExecutionEngine::new(
            Arc::new(tool_registry.clone()) as Arc<dyn ToolRegistryInterface>,
            Arc::clone(&llm_handler),
            quilt_service.clone(),
        );
        
        let planning = PlanningEngine::new(
            Arc::new(tool_registry.clone()) as Arc<dyn ToolRegistryInterface>
        );
        
        let conversation = ConversationEngine::new(
            Some(Box::new(crate::engines::LLMHandlerWrapper::new(Arc::clone(&llm_handler))))
        );
        
        let reflection = ReflectionEngine::new(
            Arc::new(tool_registry.clone()) as Arc<dyn ToolRegistryInterface>
        );
        
        let context_manager = ContextManagerEngine::new(
            crate::types::AgentConfig::default()
        );

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