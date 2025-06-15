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

pub use errors::{AriaError, AriaResult};
pub use types::{RuntimeConfiguration, RuntimeResult};
pub use runtime::AriaRuntime;

use crate::engines::{
    AriaEngines,
    Engine,
    execution::ExecutionEngine,
    planning::PlanningEngine,
    conversation::ConversationEngine,
    reflection::ReflectionEngine,
    tool_registry::{ToolRegistry, ToolRegistryInterface},
    context_manager::ContextManagerEngine,
    llm::LLMHandler,
    system_prompt::SystemPromptService,
};
use std::{collections::HashMap, sync::Arc, path::PathBuf};
use tokio::sync::RwLock;

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
    pub fn new() -> Self {
        // Create LLM handler with default configuration
        let llm_config = crate::engines::llm::LLMHandlerConfig::default();
        let llm_handler = LLMHandler::new(llm_config);

        // Create tool registry with LLM handler
        let tool_registry = ToolRegistry::new(Arc::new(llm_handler.clone()));

        // Create system prompt service
        let system_prompt = SystemPromptService::new();

        // Create all engines with concrete types
        let execution = ExecutionEngine::new(
            Arc::new(tool_registry.clone()) as Arc<dyn crate::engines::tool_registry::ToolRegistryInterface>,
            Arc::new(llm_handler.clone()),
            None, // No container manager for now
        );
        
        let planning = PlanningEngine::new(
            Arc::new(tool_registry.clone()) as Arc<dyn crate::engines::tool_registry::ToolRegistryInterface>
        );
        
        let conversation = ConversationEngine::new(
            Some(Box::new(crate::engines::LLMHandlerWrapper::new(llm_handler.clone())))
        );
        
        let reflection = ReflectionEngine::new(
            Arc::new(tool_registry.clone()) as Arc<dyn crate::engines::tool_registry::ToolRegistryInterface>
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
        }
    }
} 