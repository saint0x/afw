use crate::errors::AriaResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enable_pattern_matching: bool,
    pub enable_context_trees: bool,
    pub fast_path_threshold: f32,
    pub max_entries: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enable_pattern_matching: true,
            enable_context_trees: true,
            fast_path_threshold: 0.85,
            max_entries: 1000,
        }
    }
}

pub struct CacheIntelligence {
    config: CacheConfig,
    cache: HashMap<String, serde_json::Value>,
}

impl CacheIntelligence {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            cache: HashMap::new(),
        }
    }

    pub async fn get(&self, key: &str) -> AriaResult<Option<serde_json::Value>> {
        Ok(self.cache.get(key).cloned())
    }

    pub async fn set(&mut self, key: &str, value: serde_json::Value) -> AriaResult<()> {
        self.cache.insert(key.to_string(), value);
        Ok(())
    }

    pub async fn has(&self, key: &str) -> AriaResult<bool> {
        Ok(self.cache.contains_key(key))
    }

    pub async fn clear(&mut self) -> AriaResult<()> {
        self.cache.clear();
        Ok(())
    }
} 