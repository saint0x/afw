use crate::errors::AriaResult;
use crate::types::*;
use crate::planning::ExecutionPlan;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub session_id: SessionId,
    pub task: String,
    pub execution_plan: Option<ExecutionPlan>,
    pub current_step: usize,
    pub working_memory: std::collections::HashMap<String, serde_json::Value>,
    pub metrics: ExecutionMetrics,
    pub created_at: std::time::SystemTime,
}

pub struct ContextManager {
    contexts: std::collections::HashMap<SessionId, ExecutionContext>,
}

impl ContextManager {
    pub fn new() -> Self {
        Self {
            contexts: std::collections::HashMap::new(),
        }
    }

    pub fn create_context(&mut self, session_id: SessionId, task: String) -> AriaResult<&mut ExecutionContext> {
        let context = ExecutionContext {
            session_id: session_id.clone(),
            task,
            execution_plan: None,
            current_step: 0,
            working_memory: std::collections::HashMap::new(),
            metrics: ExecutionMetrics::default(),
            created_at: std::time::SystemTime::now(),
        };

        self.contexts.insert(session_id.clone(), context);
        Ok(self.contexts.get_mut(&session_id).unwrap())
    }

    pub fn get_context(&self, session_id: &SessionId) -> Option<&ExecutionContext> {
        self.contexts.get(session_id)
    }

    pub fn get_context_mut(&mut self, session_id: &SessionId) -> Option<&mut ExecutionContext> {
        self.contexts.get_mut(session_id)
    }

    pub fn remove_context(&mut self, session_id: &SessionId) -> Option<ExecutionContext> {
        self.contexts.remove(session_id)
    }
} 