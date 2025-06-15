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

pub mod runtime;
pub mod planning;
pub mod reflection;
pub mod tools;
pub mod agents;
pub mod teams;
pub mod memory;
pub mod cache;
pub mod context;
pub mod errors;
pub mod types;

// Re-export main components
pub use runtime::{AriaRuntime, RuntimeConfig, RuntimeResult};
pub use planning::{PlanningEngine, ExecutionPlan};
pub use reflection::{ReflectionEngine, ReflectionResult};
pub use tools::{ToolRegistry, Tool, ToolResult, ToolConfig};
pub use agents::{Agent, AgentConfig, AgentResult};
pub use teams::{Team, TeamConfig, TeamResult};
pub use memory::{MemorySystem, MemoryConfig};
pub use cache::{CacheIntelligence, CacheConfig};
pub use context::{ContextManager, ExecutionContext};
pub use errors::{AriaError, AriaResult};
pub use types::{TaskComplexity, TeamStrategy, LLMConfig, ExecutionMetrics, Priority}; // Direct export from types

/// Runtime version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the Aria Runtime with default configuration
pub async fn init() -> AriaResult<AriaRuntime> {
    AriaRuntime::new(RuntimeConfig::default()).await
}

/// Initialize the Aria Runtime with custom configuration
pub async fn init_with_config(config: RuntimeConfig) -> AriaResult<AriaRuntime> {
    AriaRuntime::new(config).await
} 