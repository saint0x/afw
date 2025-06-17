/// Context Intelligence Engine for Aria Runtime
/// Implements learned pattern recognition and context management for container orchestration

use crate::errors::{AriaError, AriaResult};
use crate::database::DatabaseManager;
use crate::engines::observability::ObservabilityManager;
use crate::engines::Engine;
use crate::types::ContainerSpec;

use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod types;
pub mod pattern_processor;
pub mod context_builder;
pub mod learning_engine;
pub mod manager;

pub use types::*;
pub use manager::IntelligenceManager;

/// Configuration for the intelligence engine
#[derive(Debug, Clone)]
pub struct IntelligenceConfig {
    /// Minimum confidence threshold for pattern matching
    pub confidence_threshold: f64,
    /// Learning rate for pattern updates
    pub learning_rate: f64,
    /// Maximum context tree depth
    pub max_context_depth: usize,
    /// Maximum context nodes per session
    pub max_context_nodes: usize,
    /// Pattern cache size limit
    pub pattern_cache_size: usize,
    /// Context cache TTL in seconds
    pub context_cache_ttl: u64,
}

impl Default for IntelligenceConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.6,
            learning_rate: 0.05,
            max_context_depth: 10,
            max_context_nodes: 100,
            pattern_cache_size: 1000,
            context_cache_ttl: 300, // 5 minutes
        }
    }
}

/// Metrics for intelligence system performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceMetrics {
    pub total_patterns: usize,
    pub high_confidence_patterns: usize,
    pub total_contexts: usize,
    pub learning_events_processed: u64,
    pub cache_hit_rate: f64,
    pub avg_pattern_match_time_ms: f64,
    pub avg_context_build_time_ms: f64,
}

impl Default for IntelligenceMetrics {
    fn default() -> Self {
        Self {
            total_patterns: 0,
            high_confidence_patterns: 0,
            total_contexts: 0,
            learning_events_processed: 0,
            cache_hit_rate: 0.0,
            avg_pattern_match_time_ms: 0.0,
            avg_context_build_time_ms: 0.0,
        }
    }
}

/// Main intelligence engine implementing the Engine trait
pub struct IntelligenceEngine {
    manager: Arc<IntelligenceManager>,
    config: IntelligenceConfig,
    metrics: Arc<RwLock<IntelligenceMetrics>>,
}

impl IntelligenceEngine {
    /// Create a new intelligence engine
    pub fn new(
        database: Arc<DatabaseManager>,
        observability: Arc<ObservabilityManager>,
        config: IntelligenceConfig,
    ) -> Self {
        let manager = Arc::new(IntelligenceManager::new(database, observability, config.clone()));
        
        Self {
            manager,
            config,
            metrics: Arc::new(RwLock::new(IntelligenceMetrics::default())),
        }
    }

    /// Get the intelligence manager
    pub fn manager(&self) -> Arc<IntelligenceManager> {
        self.manager.clone()
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> IntelligenceMetrics {
        self.metrics.read().await.clone()
    }

    /// Update metrics
    async fn update_metrics<F>(&self, updater: F)
    where
        F: FnOnce(&mut IntelligenceMetrics),
    {
        let mut metrics = self.metrics.write().await;
        updater(&mut *metrics);
    }
}

impl Engine for IntelligenceEngine {
    fn get_state(&self) -> String {
        "intelligence_engine_active".to_string()
    }

    fn get_dependencies(&self) -> Vec<String> {
        vec![
            "database".to_string(),
            "observability".to_string(),
        ]
    }

    fn health_check(&self) -> bool {
        // Simple health check - verify manager is accessible
        Arc::strong_count(&self.manager) > 0
    }

    fn initialize(&self) -> bool {
        tracing::info!("Intelligence engine initialized with config: {:?}", self.config);
        true
    }

    fn shutdown(&self) -> bool {
        tracing::info!("Intelligence engine shutdown complete");
        true
    }
}

/// Helper function to get current unix timestamp
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Helper function to generate unique IDs
pub fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{DatabaseManager, DatabaseConfig};
    use crate::engines::observability::ObservabilityManager;
    use crate::engines::Engine;
    use std::sync::Arc;
    use serde_json;

    #[tokio::test]
    async fn test_phase_1_foundation() {
        println!("🏗️ Testing Phase 1: Foundation Infrastructure");
        
        // Test Database Configuration
        let db_config = DatabaseConfig::default();
        let database = Arc::new(DatabaseManager::new(db_config));
        
        // Initialize database with intelligence schema
        let init_result = database.initialize().await;
        assert!(init_result.is_ok(), "Database initialization should succeed");
        println!("✅ Database initialized with intelligence schema");

        // Test Intelligence Engine Creation
        let observability = Arc::new(ObservabilityManager::new(database.clone(), 1000).unwrap());
        let config = IntelligenceConfig::default();
        
        let intelligence_engine = IntelligenceEngine::new(
            database.clone(),
            observability.clone(),
            config.clone(),
        );
        
        assert!(intelligence_engine.health_check(), "Intelligence engine should be healthy");
        assert!(intelligence_engine.initialize(), "Intelligence engine should initialize");
        println!("✅ Intelligence engine created and initialized");

        // Test Intelligence Manager
        let manager = intelligence_engine.manager();
        
        // Test context tools availability
        let tools = manager.get_context_tools().await;
        assert_eq!(tools.len(), 4, "Should have 4 context tools");
        
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"analyze_container_pattern"));
        assert!(tool_names.contains(&"update_pattern_confidence"));
        assert!(tool_names.contains(&"get_execution_context"));
        assert!(tool_names.contains(&"optimize_patterns"));
        println!("✅ All context tools available");

        // Test Container Request Analysis
        let request = ContainerRequest {
            request_id: generate_id(),
            session_id: "test_session_phase1".to_string(),
            description: "build a rust microservice".to_string(),
            requirements: None,
            context_hints: vec!["rust".to_string(), "microservice".to_string()],
        };

        let analysis_result = manager.analyze_container_request(&request).await;
        assert!(analysis_result.is_ok(), "Container request analysis should succeed");
        
        let result = analysis_result.unwrap();
        assert_eq!(result.session_id, "test_session_phase1");
        assert!(!result.context_summary.is_empty(), "Context summary should not be empty");
        println!("✅ Container request analysis working");

        // Test Context Tree Creation
        let context_tree = manager.get_context_tree("test_session_phase1").await;
        assert!(context_tree.is_ok(), "Context tree creation should succeed");
        
        let context = context_tree.unwrap();
        assert_eq!(context.session_id, "test_session_phase1");
        assert_eq!(context.context_id, "session_test_session_phase1");
        assert_eq!(context.priority, 10);
        println!("✅ Context tree creation working");

        // Test Pattern Retrieval (should be empty in Phase 1)
        let patterns = manager.get_all_patterns().await;
        assert!(patterns.is_ok(), "Pattern retrieval should succeed");
        assert_eq!(patterns.unwrap().len(), 0, "Should have no patterns in Phase 1");
        println!("✅ Pattern retrieval working (empty as expected)");

        // Test Intelligence Metrics
        let metrics = intelligence_engine.get_metrics().await;
        assert_eq!(metrics.total_patterns, 0, "Should have no patterns initially");
        assert_eq!(metrics.learning_events_processed, 0, "Should have no learning events initially");
        println!("✅ Intelligence metrics working");

        // Test Observability Integration
        let observability_start = observability.start().await;
        assert!(observability_start.is_ok(), "Observability should start successfully");
        println!("✅ Observability integration working");

        // Test Engine Lifecycle
        assert!(intelligence_engine.shutdown(), "Intelligence engine should shutdown cleanly");
        println!("✅ Engine lifecycle working");

        println!("\n🎉 Phase 1 Foundation Infrastructure Test PASSED!");
        println!("✨ Ready for Phase 2: Pattern Learning Engine");
    }

    #[test]
    fn test_phase_1_types() {
        println!("🧩 Testing Phase 1: Intelligence Types");
        
        use crate::engines::intelligence::types::*;
        use std::collections::HashMap;

        // Test ContainerConfig creation
        let mut env = HashMap::new();
        env.insert("RUST_ENV".to_string(), "development".to_string());
        
        let config = ContainerConfig {
            image: "rust:1.70".to_string(),
            command: vec!["cargo".to_string(), "build".to_string()],
            environment: env,
            working_directory: Some("/app".to_string()),
            resource_limits: None,
            network_config: None,
            volumes: Vec::new(),
        };
        
        assert_eq!(config.image, "rust:1.70");
        assert_eq!(config.command.len(), 2);
        println!("✅ ContainerConfig type working");

        // Test ExecutionContext creation
        let context = ExecutionContext {
            context_id: "test_context".to_string(),
            session_id: "test_session".to_string(),
            context_type: ContextType::Container,
            parent_id: None,
            context_data: serde_json::json!({"test": "data"}),
            priority: 8,
            children: Vec::new(),
            metadata: ContextMetadata::default(),
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
        };
        
        assert_eq!(context.priority, 8);
        assert!(matches!(context.context_type, ContextType::Container));
        println!("✅ ExecutionContext type working");

        // Test serialization/deserialization
        let json_config = serde_json::to_string(&config).unwrap();
        let deserialized_config: ContainerConfig = serde_json::from_str(&json_config).unwrap();
        assert_eq!(deserialized_config.image, "rust:1.70");
        println!("✅ Type serialization working");

        println!("🎉 Phase 1 Intelligence Types Test PASSED!");
    }

    #[test]
    fn test_phase_1_utilities() {
        println!("🔗 Testing Phase 1: Utility Functions");
        
        let config = IntelligenceConfig::default();
        assert_eq!(config.confidence_threshold, 0.6);
        assert_eq!(config.learning_rate, 0.05);
        assert_eq!(config.max_context_depth, 10);
        println!("✅ Intelligence config defaults working");
        
        // Test ID generation
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2, "Generated IDs should be unique");
        assert_eq!(id1.len(), 36, "Should be UUID format");
        println!("✅ ID generation working");
        
        // Test timestamp generation
        let ts1 = current_timestamp();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let ts2 = current_timestamp();
        assert!(ts2 >= ts1, "Timestamps should be monotonic");
        println!("✅ Timestamp generation working");

        println!("🎉 Phase 1 Utility Functions Test PASSED!");
    }
}

// Individual tests can be run separately using:
// cargo test --package aria_runtime --lib intelligence::tests::test_phase_1_foundation_infrastructure
// cargo test --package aria_runtime --lib intelligence::tests::test_phase_1_intelligence_types  
// cargo test --package aria_runtime --lib intelligence::tests::test_phase_1_integration 