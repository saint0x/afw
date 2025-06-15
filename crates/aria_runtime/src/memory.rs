use crate::deep_size::{DeepUuid, DeepValue, DeepSystemTime};
use crate::errors::AriaResult;
use crate::types::{AgentConfig, ConversationJSON, ExecutionStatus, RuntimeContext, WorkingMemoryEntry, MemoryEntryType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub short_term_ttl: std::time::Duration,
    pub long_term_ttl: std::time::Duration,
    pub max_entries: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            short_term_ttl: std::time::Duration::from_secs(3600), // 1 hour
            long_term_ttl: std::time::Duration::from_secs(30 * 24 * 3600), // 30 days
            max_entries: 10000,
        }
    }
}

pub struct MemorySystem {
    config: MemoryConfig,
    short_term: HashMap<String, WorkingMemoryEntry>,
    long_term: HashMap<String, WorkingMemoryEntry>,
}

impl MemorySystem {
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            config,
            short_term: HashMap::new(),
            long_term: HashMap::new(),
        }
    }

    pub async fn store(&mut self, key: &str, value: DeepValue, memory_type: &str) -> AriaResult<()> {
        let entry = WorkingMemoryEntry {
            id: DeepUuid(uuid::Uuid::new_v4()),
            key: key.to_string(),
            value,
            entry_type: MemoryEntryType::Learning,
            created_at: DeepSystemTime(std::time::SystemTime::now()),
            last_accessed: DeepSystemTime(std::time::SystemTime::now()),
            access_count: 0,
            ttl: None,
            tags: vec![],
        };

        match memory_type {
            "short_term" => { self.short_term.insert(key.to_string(), entry); },
            "long_term" => { self.long_term.insert(key.to_string(), entry); },
            _ => return Err(crate::errors::AriaError::new(
                crate::errors::ErrorCode::ContextInitializationFailed,
                crate::errors::ErrorCategory::Context,
                crate::errors::ErrorSeverity::Medium,
                "Invalid memory type"
            )),
        }

        Ok(())
    }

    pub async fn retrieve(&self, key: &str, memory_type: &str) -> AriaResult<Option<DeepValue>> {
        let entry = match memory_type {
            "short_term" => self.short_term.get(key),
            "long_term" => self.long_term.get(key),
            _ => return Err(crate::errors::AriaError::new(
                crate::errors::ErrorCode::ContextInitializationFailed,
                crate::errors::ErrorCategory::Context,
                crate::errors::ErrorSeverity::Medium,
                "Invalid memory type"
            )),
        };

        Ok(entry.map(|e| e.value.clone()))
    }
}

// Default implementation for RuntimeContext
impl Default for RuntimeContext {
    fn default() -> Self {
        RuntimeContext {
            session_id: DeepUuid(Uuid::new_v4()),
            agent_config: AgentConfig::default(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
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
            max_memory_size: 512 * 1024 * 1024, // Default 512MB
            conversation: None,
        }
    }
} 