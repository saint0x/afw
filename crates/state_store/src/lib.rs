/*!
# State Store

Custom key-value store implementation with in-memory storage and planned disk persistence.
We'll implement our own storage layer without heavy dependencies.
*/


use std::collections::HashMap;

/// Result type for state store operations
pub type StateResult<T> = Result<T, StateError>;

/// State store specific errors
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("IO error: {0}")]
    Io(String),
}

pub struct StateStore {
    // In-memory storage for now - we'll add custom disk persistence later
    memory: HashMap<String, serde_json::Value>,
}

impl StateStore {
    pub async fn new() -> StateResult<Self> {
        Ok(Self {
            memory: HashMap::new(),
        })
    }

    pub async fn get(&self, key: &str) -> StateResult<Option<serde_json::Value>> {
        Ok(self.memory.get(key).cloned())
    }

    pub async fn set(&mut self, key: &str, value: serde_json::Value) -> StateResult<()> {
        self.memory.insert(key.to_string(), value);
        // TODO: Add custom disk persistence
        Ok(())
    }

    pub async fn delete(&mut self, key: &str) -> StateResult<()> {
        self.memory.remove(key);
        // TODO: Remove from disk persistence
        Ok(())
    }

    pub async fn exists(&self, key: &str) -> StateResult<bool> {
        Ok(self.memory.contains_key(key))
    }

    pub async fn keys(&self) -> StateResult<Vec<String>> {
        Ok(self.memory.keys().cloned().collect())
    }

    pub async fn clear(&mut self) -> StateResult<()> {
        self.memory.clear();
        // TODO: Clear disk persistence
        Ok(())
    }
} 