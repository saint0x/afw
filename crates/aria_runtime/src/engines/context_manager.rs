use crate::errors::{AriaError, AriaResult, ErrorCode, ErrorCategory, ErrorSeverity};
use crate::types::*;
use async_trait::async_trait;
use deepsize::DeepSizeOf;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// The format for context serialization.
#[derive(Debug, Clone, Copy)]
pub enum SerializationFormat {
    Json,
    Bincode,
}

/// Context manager engine for handling runtime context and memory.
/// This engine is the single source of truth for the state of an execution.
pub struct ContextManagerEngine {
    context: Arc<RwLock<RuntimeContext>>,
}

use crate::deep_size::DeepValue;

// Helper function to approximate the size of a DeepValue
fn size_of_deep_value(value: &DeepValue) -> usize {
    value.deep_size_of()
}

impl ContextManagerEngine {
    /// Creates a new ContextManagerEngine with a default RuntimeContext.
    pub fn new(agent_config: AgentConfig) -> Self {
        let max_memory_size = agent_config.memory_limit.unwrap_or(512 * 1024 * 1024); // 512MB default
        let context = RuntimeContext {
            session_id: crate::deep_size::DeepUuid(uuid::Uuid::new_v4()),
            agent_config,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            conversation: None,
            status: ExecutionStatus::Running,
            current_plan: None,
            execution_history: Vec::new(),
            working_memory: Arc::new(RwLock::new(HashMap::new())),
            insights: Vec::new(),
            error_history: Vec::new(),
            current_step: 0,
            total_steps: 0,
            remaining_steps: Vec::new(),
            reflections: Vec::new(),
            memory_size: 0,
            max_memory_size,
        };
        Self {
            context: Arc::new(RwLock::new(context)),
        }
    }

    /// Creates a ContextManagerEngine from a previously serialized context.
    pub fn from_serialized(
        data: &[u8],
        format: SerializationFormat,
    ) -> AriaResult<Self> {
        let context: RuntimeContext = match format {
            SerializationFormat::Json => serde_json::from_slice(data)
                .map_err(|e| AriaError::new(ErrorCode::DeserializationError, ErrorCategory::Context, ErrorSeverity::High, &e.to_string()))?,
            SerializationFormat::Bincode => bincode::deserialize(data)
                .map_err(|e| AriaError::new(ErrorCode::DeserializationError, ErrorCategory::Context, ErrorSeverity::High, &e.to_string()))?,
        };
        Ok(Self {
            context: Arc::new(RwLock::new(context)),
        })
    }

    /// Enforces the memory limits on the runtime context.
    /// If memory usage exceeds 80% of the maximum, it prunes the history
    /// until it's below 25%.
    async fn enforce_memory_limits(&self) -> AriaResult<()> {
        let mut context = self.context.write().await;
        let high_water_mark = (context.max_memory_size as f64 * 0.80) as u64;
        
        if context.memory_size > high_water_mark {
            let low_water_mark = (context.max_memory_size as f64 * 0.25) as u64;
            
            // Prune the oldest execution history steps first
            while context.memory_size > low_water_mark && !context.execution_history.is_empty() {
                let removed_step = context.execution_history.remove(0);
                context.memory_size -= removed_step.deep_size_of() as u64;
            }

            // If still over, consider pruning reflections or other large fields
            while context.memory_size > low_water_mark && !context.reflections.is_empty() {
                let removed_reflection = context.reflections.remove(0);
                context.memory_size -= removed_reflection.deep_size_of() as u64;
            }
        }
        Ok(())
    }

    /// Updates the calculated memory size of the context.
    async fn update_memory_size(&self) {
        // First, calculate the working memory size separately
        let working_memory_size = {
            let context = self.context.read().await;
            let memory = context.working_memory.read().await;
            let mut size = 0;
            for (k, v) in memory.iter() {
                size += k.deep_size_of();
                size += size_of_deep_value(v);
            }
            size
        };
        
        // Then update the context with the calculated size
        let mut context = self.context.write().await;
        let mut total_size = context.deep_size_of();
        total_size += working_memory_size;
        context.memory_size = total_size as u64;
    }
}

impl crate::engines::Engine for ContextManagerEngine {
    fn initialize(&self) -> bool {
        // TODO: Initialize memory size calculation (async version would be better)
        true
    }

    fn get_dependencies(&self) -> Vec<String> {
        vec![]
    }

    fn get_state(&self) -> String {
        "Ready".to_string()
    }

    fn health_check(&self) -> bool {
        true
    }

    fn shutdown(&self) -> bool {
        true
    }
}

#[async_trait]
impl crate::engines::ContextManagerInterface for ContextManagerEngine {
    async fn set_plan(&self, plan: ExecutionPlan) -> AriaResult<()> {
        let mut context = self.context.write().await;
        context.current_plan = Some(plan);
        drop(context);
        self.update_memory_size().await;
        Ok(())
    }

    async fn record_step(&self, step: ExecutionStep) -> AriaResult<()> {
        let mut context = self.context.write().await;
        context.execution_history.push(step);
        drop(context);
        self.update_memory_size().await;
        self.enforce_memory_limits().await?;
        Ok(())
    }

    async fn record_reflection(&self, reflection: Reflection) -> AriaResult<()> {
        let mut context = self.context.write().await;
        context.reflections.push(reflection);
        drop(context);
        self.update_memory_size().await;
        self.enforce_memory_limits().await?;
        Ok(())
    }

    async fn update_status(&self, status: ExecutionStatus) -> AriaResult<()> {
        let mut context = self.context.write().await;
        context.status = status;
        Ok(())
    }

    async fn get_execution_state(&self) -> AriaResult<RuntimeContext> {
        let context = self.context.read().await;
        Ok(context.clone())
    }

    async fn get_memory_usage(&self) -> AriaResult<MemoryUsage> {
        let context = self.context.read().await;
        Ok(MemoryUsage {
            current_size: context.memory_size,
            max_size: context.max_memory_size,
            utilization_percent: if context.max_memory_size > 0 {
                (context.memory_size as f64 / context.max_memory_size as f64) * 100.0
            } else {
                0.0
            },
            item_count: context.execution_history.len() as u32, // Or a more detailed count
        })
    }
    
    async fn serialize_context(&self, format: SerializationFormat) -> AriaResult<Vec<u8>> {
        let context = self.context.read().await;
        match format {
            SerializationFormat::Json => serde_json::to_vec(&*context)
                .map_err(|e| AriaError::new(ErrorCode::SerializationError, ErrorCategory::Context, ErrorSeverity::High, &e.to_string())),
            SerializationFormat::Bincode => bincode::serialize(&*context)
                .map_err(|e| AriaError::new(ErrorCode::SerializationError, ErrorCategory::Context, ErrorSeverity::High, &e.to_string())),
        }
    }
} 