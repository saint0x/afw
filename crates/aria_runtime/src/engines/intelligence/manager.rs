/// Intelligence Manager - Main orchestrator for context intelligence
/// Implements the unified intelligence interface from CTXPLAN.md Phase 4.1

use crate::errors::{AriaError, AriaResult};
use crate::database::DatabaseManager;
use crate::engines::observability::ObservabilityManager;
use crate::engines::intelligence::{IntelligenceConfig, current_timestamp, generate_id};
use crate::engines::intelligence::types::*;

use std::sync::Arc;
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::RwLock;
use serde_json;

/// Main intelligence manager that coordinates all intelligence components
pub struct IntelligenceManager {
    database: Arc<DatabaseManager>,
    observability: Arc<ObservabilityManager>,
    config: IntelligenceConfig,
    
    // Core component placeholders - will be implemented in later phases
    // pattern_processor: Arc<ContainerPatternProcessor>,
    // context_builder: Arc<ExecutionContextBuilder>, 
    // learning_engine: Arc<WorkloadLearningEngine>,
    
    // Runtime state
    pattern_cache: Arc<RwLock<HashMap<String, ContainerPattern>>>,
    context_cache: Arc<RwLock<HashMap<String, ExecutionContext>>>,
}

impl IntelligenceManager {
    /// Create a new intelligence manager
    pub fn new(
        database: Arc<DatabaseManager>,
        observability: Arc<ObservabilityManager>,
        config: IntelligenceConfig,
    ) -> Self {
        Self {
            database,
            observability,
            config,
            pattern_cache: Arc::new(RwLock::new(HashMap::new())),
            context_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Main intelligence interface - analyze request and provide recommendations
    pub async fn analyze_container_request(
        &self,
        request: &ContainerRequest,
    ) -> AriaResult<IntelligenceResult> {
        let start_time = Instant::now();
        
        tracing::debug!(
            "Analyzing container request: {} for session {}",
            request.description,
            request.session_id
        );

        // Phase 1 implementation - basic structure, will be enhanced in later phases
        let result = IntelligenceResult {
            request_id: request.request_id.clone(),
            session_id: request.session_id.clone(),
            pattern_match: None, // Will be implemented in Phase 2
            context_summary: self.get_basic_context_summary(&request.session_id).await?,
            recommendation: self.generate_basic_recommendation(request).await?,
            execution_time: start_time.elapsed(),
            timestamp: std::time::SystemTime::now(),
        };

        // Record intelligence query for observability
        self.record_intelligence_query(request, &result).await?;

        Ok(result)
    }

    /// Learn from container execution result
    pub async fn learn_from_execution(
        &self,
        execution_result: &ContainerExecutionResult,
    ) -> AriaResult<()> {
        tracing::debug!(
            "Learning from execution: {} (success: {})",
            execution_result.execution_id,
            execution_result.success
        );

        // Phase 1 implementation - log the learning event
        // Full learning implementation will be in Phase 2

        // Emit intelligence event for observability
        self.observability.emit_event(
            crate::engines::observability::ObservabilityEvent::IntelligenceUpdate {
                timestamp: current_timestamp(),
                pattern_id: execution_result.pattern_id.clone(),
                confidence_delta: execution_result.confidence_delta,
                learning_context: serde_json::to_value(&execution_result.metadata)?,
            }
        ).await?;

        Ok(())
    }

    /// Provide context management tools for agents
    pub async fn get_context_tools(&self) -> Vec<IntelligenceTool> {
        vec![
            IntelligenceTool {
                name: "analyze_container_pattern".to_string(),
                description: "Analyze container request and provide intelligent recommendations".to_string(),
                parameters: self.get_analyze_pattern_schema(),
            },
            IntelligenceTool {
                name: "update_pattern_confidence".to_string(),
                description: "Update pattern confidence based on execution feedback".to_string(),
                parameters: self.get_update_confidence_schema(),
            },
            IntelligenceTool {
                name: "get_execution_context".to_string(),
                description: "Get current execution context for intelligent decision making".to_string(),
                parameters: self.get_context_schema(),
            },
            IntelligenceTool {
                name: "optimize_patterns".to_string(),
                description: "Optimize pattern performance and remove low-confidence patterns".to_string(),
                parameters: self.get_optimize_schema(),
            },
        ]
    }

    /// Get all patterns (for debugging and analytics)
    pub async fn get_all_patterns(&self) -> AriaResult<Vec<ContainerPattern>> {
        // Phase 1 implementation - return empty patterns
        // Will be implemented with database queries in Phase 2
        Ok(Vec::new())
    }

    /// Get context tree for session
    pub async fn get_context_tree(&self, session_id: &str) -> AriaResult<ExecutionContext> {
        // Phase 1 implementation - basic context tree structure
        // Full implementation will be in Phase 3
        
        let context = ExecutionContext {
            context_id: format!("session_{}", session_id),
            session_id: session_id.to_string(),
            context_type: ContextType::Session,
            parent_id: None,
            context_data: serde_json::json!({
                "session_id": session_id,
                "created_at": current_timestamp(),
                "status": "active"
            }),
            priority: 10,
            children: Vec::new(),
            metadata: ContextMetadata::default(),
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
        };

        Ok(context)
    }

    // Private helper methods

    async fn get_basic_context_summary(&self, session_id: &str) -> AriaResult<String> {
        // Phase 1 implementation - basic context summary
        Ok(format!(
            "**Current Execution Context:**\n\n**SESSION:** {}\n  - Status: Active\n  - Intelligence: Pattern learning enabled\n",
            session_id
        ))
    }

    async fn generate_basic_recommendation(
        &self,
        request: &ContainerRequest,
    ) -> AriaResult<IntelligenceRecommendation> {
        // Phase 1 implementation - basic recommendation logic
        // Full pattern matching will be implemented in Phase 2
        
        Ok(IntelligenceRecommendation {
            action: RecommendationAction::CreateNew,
            confidence: 0.5, // Neutral confidence for Phase 1
            reasoning: "Pattern learning system initializing - creating new container configuration".to_string(),
            suggested_config: None,
            alternatives: Vec::new(),
            warnings: vec![
                "Intelligence system is in learning mode - patterns will improve with usage".to_string()
            ],
        })
    }

    async fn record_intelligence_query(
        &self,
        request: &ContainerRequest,
        result: &IntelligenceResult,
    ) -> AriaResult<()> {
        // Phase 1 implementation - basic logging
        // Database persistence will be implemented in Phase 2
        
        tracing::info!(
            "Intelligence query: session={}, request={}, execution_time={:?}",
            request.session_id,
            request.description,
            result.execution_time
        );

        Ok(())
    }

    // Tool schema definitions
    
    fn get_analyze_pattern_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "request": {
                    "type": "string",
                    "description": "Container request description"
                },
                "session_id": {
                    "type": "string", 
                    "description": "Current session ID"
                }
            },
            "required": ["request", "session_id"]
        })
    }

    fn get_update_confidence_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern_id": {
                    "type": "string",
                    "description": "Pattern ID to update"
                },
                "success": {
                    "type": "boolean",
                    "description": "Whether execution was successful"
                },
                "execution_time_ms": {
                    "type": "integer",
                    "description": "Execution time in milliseconds"
                }
            },
            "required": ["pattern_id", "success"]
        })
    }

    fn get_context_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "Session ID to get context for"
                },
                "max_nodes": {
                    "type": "integer",
                    "description": "Maximum context nodes to return",
                    "default": 20
                }
            },
            "required": ["session_id"]
        })
    }

    fn get_optimize_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", 
            "properties": {
                "min_confidence": {
                    "type": "number",
                    "description": "Minimum confidence threshold for keeping patterns",
                    "default": 0.3
                },
                "max_age_days": {
                    "type": "integer",
                    "description": "Maximum age in days for patterns",
                    "default": 30
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DatabaseConfig;

    #[tokio::test]
    async fn test_intelligence_manager_creation() {
        let db_config = DatabaseConfig::default();
        let database = Arc::new(DatabaseManager::new(db_config));
        let observability = Arc::new(ObservabilityManager::new(database.clone(), 1000).unwrap());
        let config = IntelligenceConfig::default();

        let manager = IntelligenceManager::new(database, observability, config);
        
        // Basic functionality test
        let tools = manager.get_context_tools().await;
        assert_eq!(tools.len(), 4);
        assert_eq!(tools[0].name, "analyze_container_pattern");
    }

    #[tokio::test]
    async fn test_container_request_analysis() {
        let db_config = DatabaseConfig::default();
        let database = Arc::new(DatabaseManager::new(db_config));
        let observability = Arc::new(ObservabilityManager::new(database.clone(), 1000).unwrap());
        let config = IntelligenceConfig::default();

        let manager = IntelligenceManager::new(database, observability, config);
        
        let request = ContainerRequest {
            request_id: generate_id(),
            session_id: "test_session".to_string(),
            description: "build a rust web server".to_string(),
            requirements: None,
            context_hints: Vec::new(),
        };

        let result = manager.analyze_container_request(&request).await;
        assert!(result.is_ok());
        
        let result = result.unwrap();
        assert_eq!(result.session_id, "test_session");
        assert!(result.execution_time.as_millis() < 100); // Should be fast in Phase 1
    }
} 