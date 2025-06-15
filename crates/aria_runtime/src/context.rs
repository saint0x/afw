use crate::errors::AriaResult;
use crate::types::{ExecutionPlan, RuntimeContext, SessionId};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct ContextManager {
    contexts: HashMap<SessionId, RuntimeContext>,
    storage_path: PathBuf,
}

impl ContextManager {
    pub fn new(storage_path: PathBuf) -> Self {
        Self {
            contexts: HashMap::new(),
            storage_path,
        }
    }

    pub fn create_context(&mut self, session_id: SessionId, task: String) -> AriaResult<&mut RuntimeContext> {
        let mut context = RuntimeContext::default();
        context.session_id = crate::deep_size::DeepUuid(uuid::Uuid::parse_str(&session_id)
            .unwrap_or_else(|_| uuid::Uuid::new_v4()));
        // context.task = task; // Assuming RuntimeContext will have a task field
        
        self.contexts.insert(session_id.clone(), context);
        Ok(self.contexts.get_mut(&session_id).unwrap())
    }

    pub fn get_context(&self, session_id: &SessionId) -> Option<&RuntimeContext> {
        self.contexts.get(session_id)
    }

    pub fn get_context_mut(&mut self, session_id: &SessionId) -> Option<&mut RuntimeContext> {
        self.contexts.get_mut(session_id)
    }

    pub fn remove_context(&mut self, session_id: &SessionId) -> Option<RuntimeContext> {
        self.contexts.remove(session_id)
    }

    pub async fn save_context(&self, _context: &RuntimeContext) -> AriaResult<()> {
        Ok(())
    }

    pub async fn load_context(&self, _session_id: &str) -> AriaResult<RuntimeContext> {
        Ok(RuntimeContext::default())
    }

    pub async fn update_plan(&self, _session_id: &str, _plan: ExecutionPlan) -> AriaResult<()> {
        Ok(())
    }
} 