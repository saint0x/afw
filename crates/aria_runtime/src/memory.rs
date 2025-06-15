use crate::errors::AriaResult;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

    pub async fn store(&mut self, key: &str, value: serde_json::Value, memory_type: &str) -> AriaResult<()> {
        let entry = WorkingMemoryEntry {
            id: uuid::Uuid::new_v4(),
            key: key.to_string(),
            value,
            entry_type: MemoryEntryType::Learning,
            created_at: std::time::SystemTime::now(),
            last_accessed: std::time::SystemTime::now(),
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
            ).with_component("MemorySystem").with_operation("store")),
        }

        Ok(())
    }

    pub async fn retrieve(&self, key: &str, memory_type: &str) -> AriaResult<Option<serde_json::Value>> {
        let entry = match memory_type {
            "short_term" => self.short_term.get(key),
            "long_term" => self.long_term.get(key),
            _ => return Err(crate::errors::AriaError::new(
                crate::errors::ErrorCode::ContextInitializationFailed,
                crate::errors::ErrorCategory::Context,
                crate::errors::ErrorSeverity::Medium,
                "Invalid memory type"
            ).with_component("MemorySystem").with_operation("retrieve")),
        };

        Ok(entry.map(|e| e.value.clone()))
    }
} 