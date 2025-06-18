/// Intelligence Manager - Main orchestrator for context intelligence
/// Implements the unified intelligence interface from CTXPLAN.md Phase 4.1

use crate::database::DatabaseManager;
use crate::engines::observability::ObservabilityManager;
use crate::engines::intelligence::{IntelligenceConfig, current_timestamp, generate_id};
use crate::engines::intelligence::context_builder::{ExecutionContextBuilder, ContextBuilderConfig};
use crate::engines::intelligence::pattern_processor::{ContainerPatternProcessor, PatternProcessorConfig};
use crate::engines::intelligence::learning_engine::{WorkloadLearningEngine, WorkloadLearningConfig, WorkloadAnalysis};
use crate::engines::intelligence::types::*;
use crate::engines::streaming::StreamingService;
use crate::engines::Engine;
use crate::errors::{AriaError, AriaResult, ErrorCode, ErrorCategory, ErrorSeverity};
use crate::deep_size::DeepUuid;

use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Instant, Duration, SystemTime};
use tokio::sync::RwLock;
use serde_json;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Main intelligence manager that coordinates all intelligence components
pub struct IntelligenceManager {
    database: Arc<DatabaseManager>,
    observability: Arc<ObservabilityManager>,
    learning_engine: Arc<WorkloadLearningEngine>,
    context_builder: Arc<ExecutionContextBuilder>,
    config: IntelligenceConfig,
    
    // Core component placeholders - will be implemented in later phases
    // pattern_processor: Arc<ContainerPatternProcessor>,
    
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
        // Initialize Phase 2 learning components
        let learning_config = WorkloadLearningConfig::default();
        let learning_engine = Arc::new(WorkloadLearningEngine::new(
            database.clone(),
            observability.clone(),
            learning_config,
        ));

        // Initialize Phase 3 context builder
        let context_config = ContextBuilderConfig {
            max_context_depth: config.max_context_depth,
            max_context_nodes: config.max_context_nodes,
            context_cache_ttl_seconds: config.context_cache_ttl,
            max_cache_size: 100,
            min_priority_threshold: 3,
            session_context_limit: 20,
        };
        let context_builder = Arc::new(ExecutionContextBuilder::new(
            database.clone(),
            observability.clone(),
            context_config,
        ));

        Self {
            database,
            observability,
            learning_engine,
            context_builder,
            config,
            pattern_cache: Arc::new(RwLock::new(HashMap::new())),
            context_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Main intelligence interface - analyze request and provide recommendations
    pub async fn analyze_container_request(
        &self,
        request: &ContainerRequest,
        session_id: &str,
    ) -> AriaResult<IntelligenceResult> {
        let start_time = std::time::Instant::now();
        
        info!("Analyzing container request for session: {}", session_id);
        debug!("Request: {}", request.description);

        // 1. Build execution context (simplified for Phase 2)
        let context = self.build_basic_execution_context(request, session_id).await?;
        
        // 2. Use pattern processor to find matches
        let pattern_processor = self.learning_engine.get_pattern_processor();
        let pattern_match = pattern_processor.process_container_request(&request.description, &context).await?;
        
        // 3. Generate intelligent recommendation
        let recommendation = self.generate_recommendation(&pattern_match, &context, request).await?;
        
        // 4. Record intelligence query for learning
        self.record_intelligence_query(session_id, request, &pattern_match, &recommendation).await?;

        Ok(IntelligenceResult {
            request_id: request.request_id.clone(),
            session_id: session_id.to_string(),
            pattern_match: Some(pattern_match),
            context_summary: self.generate_context_summary(&context),
            recommendation,
            execution_time: start_time.elapsed(),
            timestamp: SystemTime::now(),
        })
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
                name: "get_context_for_prompt".to_string(),
                description: "Get execution context formatted for agent prompts".to_string(),
                parameters: self.get_context_prompt_schema(),
            },
            IntelligenceTool {
                name: "optimize_patterns".to_string(),
                description: "Optimize pattern performance and remove low-confidence patterns".to_string(),
                parameters: self.get_optimize_schema(),
            },
            IntelligenceTool {
                name: "get_context_cache_stats".to_string(),
                description: "Get context cache performance statistics".to_string(),
                parameters: serde_json::json!({"type": "object", "properties": {}}),
            },
            IntelligenceTool {
                name: "clear_context_cache".to_string(),
                description: "Clear the context cache to force fresh context building".to_string(),
                parameters: serde_json::json!({"type": "object", "properties": {}}),
            },
        ]
    }

    /// Get all patterns (for debugging and analytics)
    pub async fn get_all_patterns(&self) -> AriaResult<Vec<ContainerPattern>> {
        self.learning_engine.get_all_patterns().await
    }

    /// Get a single pattern by its ID.
    pub async fn get_pattern_by_id(&self, pattern_id: &str) -> AriaResult<ContainerPattern> {
        self.learning_engine
            .get_pattern_by_id(pattern_id)
            .await?
            .ok_or_else(|| {
                AriaError::new(
                    ErrorCode::ToolNotFound,
                    ErrorCategory::System,
                    ErrorSeverity::Low,
                    &format!("Pattern with ID '{}' not found.", pattern_id),
                )
            })
    }

    /// Removes a pattern by its ID.
    pub async fn remove_pattern(&self, pattern_id: &str) -> AriaResult<()> {
        self.learning_engine.remove_pattern(pattern_id).await
    }

    /// Get context tree for session (Phase 3 implementation)
    pub async fn get_context_tree(&self, session_id: &str) -> AriaResult<ExecutionContext> {
        info!("Building context tree for session: {}", session_id);
        
        // Use ExecutionContextBuilder to build full context tree
        self.context_builder.build_context_tree(session_id).await
    }

    /// Get context formatted for agent prompts (Phase 3)
    pub async fn get_context_for_prompt(&self, session_id: &str, max_nodes: Option<usize>) -> AriaResult<String> {
        info!("Getting context for prompt for session: {}", session_id);
        
        self.context_builder.get_context_for_prompt(session_id, max_nodes).await
    }

    /// Get context cache statistics (Phase 3)
    pub async fn get_context_cache_stats(&self) -> AriaResult<crate::engines::intelligence::context_builder::ContextCacheStats> {
        self.context_builder.get_cache_stats().await
    }

    /// Clear context cache (Phase 3)
    pub async fn clear_context_cache(&self) -> AriaResult<()> {
        info!("Clearing context cache");
        self.context_builder.clear_cache().await
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
        session_id: &str,
        request: &ContainerRequest,
        pattern_match: &PatternMatch,
        recommendation: &IntelligenceRecommendation,
    ) -> AriaResult<()> {
        let query_data = serde_json::json!({
            "query_id": Uuid::new_v4().to_string(),
            "session_id": session_id,
            "request_id": request.request_id,
            "request_description": request.description,
            "pattern_id": pattern_match.pattern.pattern_id,
            "pattern_confidence": pattern_match.confidence,
            "recommendation_action": recommendation.action,
            "recommendation_confidence": recommendation.confidence,
            "timestamp": SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
        });

        debug!("Recording intelligence query: {}", query_data);
        
        // TODO: Store in intelligence_queries table when database integration is complete
        
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

    fn get_context_prompt_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "Session ID to get context for"
                },
                "max_nodes": {
                    "type": "integer",
                    "description": "Maximum context nodes to include in prompt",
                    "default": 50
                }
            },
            "required": ["session_id"]
        })
    }

    /// Build basic execution context for Phase 2
    async fn build_basic_execution_context(
        &self,
        request: &ContainerRequest,
        session_id: &str,
    ) -> AriaResult<ExecutionContext> {
        Ok(ExecutionContext {
            context_id: format!("request_{}", request.request_id),
            session_id: session_id.to_string(),
            context_type: ContextType::Container,
            parent_id: None,
            context_data: serde_json::json!({
                "request_description": request.description,
                "requirements": request.requirements,
                "context_hints": request.context_hints,
                "timestamp": SystemTime::now()
            }),
            priority: 7,
            children: Vec::new(),
            metadata: ContextMetadata::default(),
            created_at: SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
            updated_at: SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
        })
    }

    /// Generate intelligent recommendation based on pattern match
    async fn generate_recommendation(
        &self,
        pattern_match: &PatternMatch,
        _context: &ExecutionContext,
        request: &ContainerRequest,
    ) -> AriaResult<IntelligenceRecommendation> {
        let action = if pattern_match.confidence > 0.8 {
            RecommendationAction::UsePattern
        } else if pattern_match.confidence > 0.5 {
            RecommendationAction::OptimizeExisting
        } else {
            RecommendationAction::CreateNew
        };

        let mut warnings = Vec::new();
        
        // Check for potential issues
        if pattern_match.confidence < 0.6 {
            warnings.push("Low confidence pattern match - consider manual review".to_string());
        }

        if let Some(requirements) = &request.requirements {
            if let Some(min_memory) = requirements.min_memory_mb {
                if let Some(config_memory) = pattern_match.container_config.resource_limits.as_ref()
                    .and_then(|r| r.memory_mb) {
                    if config_memory < min_memory {
                        warnings.push(format!("Pattern memory allocation ({} MB) below requirements ({} MB)", 
                                            config_memory, min_memory));
                    }
                }
            }
        }

        let reasoning = match action {
            RecommendationAction::UsePattern => {
                format!("High confidence pattern match ({:.1}%) - recommended for direct use", 
                       pattern_match.confidence * 100.0)
            }
            RecommendationAction::OptimizeExisting => {
                format!("Medium confidence pattern match ({:.1}%) - consider optimization", 
                       pattern_match.confidence * 100.0)
            }
            RecommendationAction::CreateNew => {
                format!("Low confidence match ({:.1}%) - new pattern creation recommended", 
                       pattern_match.confidence * 100.0)
            }
            RecommendationAction::RequestMoreInfo => {
                "Insufficient information to make recommendation".to_string()
            }
        };

        Ok(IntelligenceRecommendation {
            action,
            confidence: pattern_match.confidence,
            reasoning,
            suggested_config: Some(pattern_match.container_config.clone()),
            alternatives: Vec::new(), // Will be enhanced in future phases
            warnings,
        })
    }

    /// Generate context summary for intelligence result
    fn generate_context_summary(&self, context: &ExecutionContext) -> String {
        format!("Session: {} | Context: {:?} | Priority: {} | Created: {}", 
                context.session_id, 
                context.context_type,
                context.priority,
                context.created_at)
    }

    /// Phase 2: Learn from container workload execution
    pub async fn learn_from_container_execution(
        &self,
        workload: &ContainerWorkload,
        result: &ContainerExecutionResult,
    ) -> AriaResult<()> {
        info!("Learning from container execution: {} (success: {})", 
              workload.workload_id, result.success);

        // Use learning engine to process execution results
        self.learning_engine.learn_from_workload(workload, result).await?;

        // Emit observability event for learning
        let learning_event_data = serde_json::json!({
            "workload_id": workload.workload_id,
            "session_id": workload.session_id,
            "success": result.success,
            "execution_time_ms": result.execution_time.as_millis(),
            "pattern_used": result.pattern_id.is_some(),
            "confidence_delta": result.confidence_delta,
            "learning_source": "container_execution"
        });

        // Create ObservabilityEvent for learning update
        let event_data = serde_json::json!({
            "event_type": "IntelligenceUpdate",
            "data": learning_event_data,
            "timestamp": SystemTime::now()
        });
        
        // For now, we'll just log the event - proper ObservabilityEvent integration will be enhanced
        debug!("Intelligence learning event: {}", event_data);

        Ok(())
    }

    /// Phase 2: Get workload learning analytics
    pub async fn get_learning_analytics(&self, session_id: Option<&str>) -> AriaResult<LearningAnalytics> {
        if let Some(session) = session_id {
            // Get session-specific analytics through workload analysis
            let workload_analysis = self.learning_engine.analyze_workload_patterns(session).await?;
            
            // Convert to LearningAnalytics format
            Ok(LearningAnalytics {
                total_patterns: workload_analysis.total_patterns_available,
                high_confidence_patterns: 0, // Will be calculated properly
                recent_executions: workload_analysis.patterns_used_in_session as u32,
                success_rate: workload_analysis.avg_pattern_confidence,
                avg_confidence_improvement: 0.0, // Simplified for now
                patterns_pruned: 0,
                learning_events_processed: 0,
            })
        } else {
            // Get global analytics
            self.learning_engine.get_learning_stats().await
        }
    }

    /// Phase 2: Get workload analysis for session
    pub async fn analyze_session_workloads(&self, session_id: &str) -> AriaResult<WorkloadAnalysis> {
        self.learning_engine.analyze_workload_patterns(session_id).await
    }

    /// Phase 2: Force pattern optimization
    pub async fn optimize_patterns(&self) -> AriaResult<()> {
        info!("Forcing pattern optimization");
        self.learning_engine.force_optimization().await
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

        let result = manager.analyze_container_request(&request, "test_session").await;
        assert!(result.is_ok());
        
        let result = result.unwrap();
        assert_eq!(result.session_id, "test_session");
        assert!(result.execution_time.as_millis() < 100); // Should be fast in Phase 1
    }
} 