/// Workload Learning Engine - Phase 2 Implementation
/// Learns from container workload execution patterns and optimizes performance

use crate::database::DatabaseManager;
use crate::engines::intelligence::pattern_processor::{ContainerPatternProcessor, PatternProcessorConfig};
use crate::engines::intelligence::types::*;
use crate::engines::observability::ObservabilityManager;
use crate::errors::{AriaError, AriaResult, ErrorCode, ErrorCategory, ErrorSeverity};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Configuration for workload learning
#[derive(Debug, Clone)]
pub struct WorkloadLearningConfig {
    pub min_executions_for_optimization: u32,
    pub pattern_creation_threshold: f64,
    pub optimization_interval_hours: u64,
    pub max_failed_patterns_to_keep: usize,
    pub confidence_boost_amount: f64,
    pub low_confidence_threshold: f64,
    pub high_performance_threshold_ms: u64,
}

impl Default for WorkloadLearningConfig {
    fn default() -> Self {
        Self {
            min_executions_for_optimization: 10,
            pattern_creation_threshold: 0.8, // Only create patterns for successful executions
            optimization_interval_hours: 24,
            max_failed_patterns_to_keep: 50,
            confidence_boost_amount: 0.05,
            low_confidence_threshold: 0.3,
            high_performance_threshold_ms: 5000,
        }
    }
}

/// Workload Learning Engine - learns from container workload execution
pub struct WorkloadLearningEngine {
    database: Arc<DatabaseManager>,
    pattern_processor: Arc<ContainerPatternProcessor>,
    observability: Arc<ObservabilityManager>,
    config: WorkloadLearningConfig,
    last_optimization: std::sync::RwLock<Option<SystemTime>>,
}

impl WorkloadLearningEngine {
    /// Create new workload learning engine
    pub fn new(
        database: Arc<DatabaseManager>,
        observability: Arc<ObservabilityManager>,
        config: WorkloadLearningConfig,
    ) -> Self {
        let pattern_config = PatternProcessorConfig::default();
        let pattern_processor = Arc::new(ContainerPatternProcessor::new(database.clone(), pattern_config));
        
        Self {
            database,
            pattern_processor,
            observability,
            config,
            last_optimization: std::sync::RwLock::new(None),
        }
    }

    /// Initialize the learning engine
    pub async fn initialize(&self) -> AriaResult<()> {
        info!("Initializing WorkloadLearningEngine");
        
        // Initialize pattern processor
        self.pattern_processor.initialize().await?;
        
        // Set initial optimization time
        {
            let mut last_opt = self.last_optimization.write().map_err(|_| {
                AriaError::new(
                    ErrorCode::ExecutionError,
                    ErrorCategory::System,
                    ErrorSeverity::Medium,
                    "Failed to acquire optimization lock"
                )
            })?;
            *last_opt = Some(SystemTime::now());
        }
        
        info!("WorkloadLearningEngine initialized successfully");
        Ok(())
    }

    /// Learn from container workload execution
    pub async fn learn_from_workload(
        &self,
        workload: &ContainerWorkload,
        result: &ContainerExecutionResult,
    ) -> AriaResult<()> {
        info!("Learning from workload execution: {} (success: {})", 
              workload.workload_id, result.success);

        // 1. Update pattern confidence if pattern was used
        if let Some(pattern_id) = &result.pattern_id {
            debug!("Updating pattern {} from execution result", pattern_id);
            self.pattern_processor.learn_from_execution(pattern_id, result).await?;
        }

        // 2. Analyze execution for new pattern creation
        if result.success && result.execution_time.as_millis() < self.config.high_performance_threshold_ms as u128 {
            self.consider_new_pattern_creation(workload, result).await?;
        }

        // 3. Update execution context with results
        self.update_execution_context(workload, result).await?;

        // 4. Record learning analytics
        self.record_learning_analytics(workload, result).await?;

        // 5. Trigger pattern optimization if needed
        if self.should_optimize_patterns().await? {
            info!("Triggering pattern optimization");
            self.optimize_pattern_performance().await?;
        }

        Ok(())
    }

    /// Consider creating new pattern from successful execution
    async fn consider_new_pattern_creation(
        &self,
        workload: &ContainerWorkload,
        result: &ContainerExecutionResult,
    ) -> AriaResult<()> {
        // Only create patterns for highly successful executions
        if !result.success || result.execution_time.as_millis() > self.config.high_performance_threshold_ms as u128 {
            return Ok(());
        }

        // Check if we already have a similar pattern
        let patterns = self.pattern_processor.get_all_patterns().await?;
        let similar_pattern_exists = patterns.iter().any(|pattern| {
            self.calculate_request_similarity(&workload.request_description, &pattern.trigger) > 0.8
        });

        if similar_pattern_exists {
            debug!("Similar pattern already exists, skipping pattern creation");
            return Ok(());
        }

        info!("Creating new pattern from successful workload: {}", workload.workload_id);

        // Create execution context for pattern creation
        let execution_context = ExecutionContext {
            context_id: format!("workload_{}", workload.workload_id),
            session_id: workload.session_id.clone(),
            context_type: ContextType::Container,
            parent_id: None,
            context_data: serde_json::json!({
                "workload_type": workload.workload_type,
                "execution_time": result.execution_time.as_millis(),
                "success": result.success,
                "resource_usage": result.resource_usage
            }),
            priority: 8, // High priority for successful new patterns
            children: Vec::new(),
            metadata: ContextMetadata {
                execution_count: 1,
                success_rate: 1.0,
                avg_duration: result.execution_time,
                last_execution: Some(SystemTime::now()),
                error_patterns: Vec::new(),
            },
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
        };

        // Use pattern processor to create new pattern
        match self.pattern_processor.process_container_request(&workload.request_description, &execution_context).await {
            Ok(pattern_match) => {
                info!("Created new pattern: {} with confidence {:.3}", 
                      pattern_match.pattern.pattern_id, pattern_match.confidence);
                
                // Boost confidence slightly since this was a successful execution
                self.pattern_processor.boost_pattern_confidence(
                    &pattern_match.pattern.pattern_id,
                    self.config.confidence_boost_amount
                ).await?;
            }
            Err(e) => {
                warn!("Failed to create pattern from workload: {}", e);
            }
        }

        Ok(())
    }

    /// Update execution context with workload results
    async fn update_execution_context(
        &self,
        workload: &ContainerWorkload,
        result: &ContainerExecutionResult,
    ) -> AriaResult<()> {
        debug!("Updating execution context for workload: {}", workload.workload_id);

        // TODO: Implement context tree updates
        // This will be integrated with the ExecutionContextBuilder in Phase 3
        // For now, we'll log the context update intent

        debug!("Context update recorded for session: {} workload: {} success: {}", 
               workload.session_id, workload.workload_id, result.success);

        Ok(())
    }

    /// Record learning analytics for performance tracking
    async fn record_learning_analytics(
        &self,
        workload: &ContainerWorkload,
        result: &ContainerExecutionResult,
    ) -> AriaResult<()> {
        // Create learning analytics event
        let analytics_data = serde_json::json!({
            "workload_id": workload.workload_id,
            "workload_type": workload.workload_type,
            "session_id": workload.session_id,
            "execution_time_ms": result.execution_time.as_millis(),
            "success": result.success,
            "pattern_used": result.pattern_id.is_some(),
            "pattern_id": result.pattern_id,
            "confidence_delta": result.confidence_delta,
            "resource_usage": result.resource_usage,
            "exit_code": result.exit_code,
            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
        });

        // Record analytics through observability system
        // This integrates with the observability events from Phase 1
        debug!("Recording learning analytics: {}", analytics_data);

        Ok(())
    }

    /// Optimize pattern performance based on usage statistics
    pub async fn optimize_pattern_performance(&self) -> AriaResult<()> {
        info!("Starting pattern performance optimization");

        let patterns = self.pattern_processor.get_all_patterns().await?;
        let mut optimization_stats = OptimizationStats::default();

        for pattern in patterns {
            let total_executions = pattern.usage_stats.total_executions;
            
            // Skip patterns with insufficient data
            if total_executions < self.config.min_executions_for_optimization {
                continue;
            }

            let success_rate = pattern.usage_stats.success_count as f64 / total_executions as f64;
            
            // Remove low-confidence patterns with poor performance
            if pattern.confidence < self.config.low_confidence_threshold && success_rate < 0.3 {
                info!("Removing low-performance pattern: {} (confidence: {:.3}, success: {:.1}%)",
                      pattern.pattern_id, pattern.confidence, success_rate * 100.0);
                
                self.pattern_processor.remove_pattern(&pattern.pattern_id).await?;
                optimization_stats.patterns_removed += 1;
                continue;
            }
            
            // Boost high-performing patterns
            if success_rate > 0.9 && total_executions >= 5 {
                info!("Boosting high-performance pattern: {} (success: {:.1}%)",
                      pattern.pattern_id, success_rate * 100.0);
                
                self.pattern_processor.boost_pattern_confidence(
                    &pattern.pattern_id,
                    self.config.confidence_boost_amount
                ).await?;
                optimization_stats.patterns_boosted += 1;
            }

            // Track patterns that need attention
            if pattern.confidence < 0.5 {
                optimization_stats.low_confidence_patterns += 1;
            }

            if let Some(last_used) = pattern.usage_stats.last_used {
                let days_since_used = SystemTime::now()
                    .duration_since(last_used)
                    .unwrap_or_default()
                    .as_secs() / 86400;
                
                if days_since_used > 30 {
                    optimization_stats.stale_patterns += 1;
                }
            }
        }

        // Update optimization timestamp
        {
            let mut last_opt = self.last_optimization.write().map_err(|_| {
                AriaError::new(
                    ErrorCode::ExecutionError,
                    ErrorCategory::System,
                    ErrorSeverity::Medium,
                    "Failed to acquire optimization lock"
                )
            })?;
            *last_opt = Some(SystemTime::now());
        }

        info!("Pattern optimization completed: {} removed, {} boosted, {} low confidence, {} stale",
              optimization_stats.patterns_removed,
              optimization_stats.patterns_boosted,
              optimization_stats.low_confidence_patterns,
              optimization_stats.stale_patterns);

        Ok(())
    }

    /// Check if pattern optimization should run
    async fn should_optimize_patterns(&self) -> AriaResult<bool> {
        let last_opt = self.last_optimization.read().map_err(|_| {
            AriaError::new(
                ErrorCode::ExecutionError,
                ErrorCategory::System,
                ErrorSeverity::Medium,
                "Failed to acquire optimization lock"
            )
        })?;

        match *last_opt {
            Some(last_time) => {
                let hours_since_last = SystemTime::now()
                    .duration_since(last_time)
                    .unwrap_or_default()
                    .as_secs() / 3600;
                
                Ok(hours_since_last >= self.config.optimization_interval_hours)
            }
            None => Ok(true), // Never optimized, should run
        }
    }

    /// Calculate similarity between two request strings
    fn calculate_request_similarity(&self, request1: &str, request2: &str) -> f64 {
        let request1_lower = request1.to_lowercase();
        let request2_lower = request2.to_lowercase();
        let words1: Vec<&str> = request1_lower.split_whitespace().collect();
        let words2: Vec<&str> = request2_lower.split_whitespace().collect();

        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }

        let common_words = words1.iter()
            .filter(|word| words2.contains(word))
            .count();

        let total_unique_words = words1.len().max(words2.len());
        common_words as f64 / total_unique_words as f64
    }

    /// Get learning engine statistics
    pub async fn get_learning_stats(&self) -> AriaResult<LearningAnalytics> {
        let patterns = self.pattern_processor.get_all_patterns().await?;
        let (total_patterns, avg_confidence, total_executions) = self.pattern_processor.get_pattern_stats().await?;
        
        let high_confidence_patterns = patterns.iter()
            .filter(|p| p.confidence > 0.8)
            .count();

        let recent_executions = patterns.iter()
            .map(|p| p.usage_stats.total_executions)
            .sum();

        let success_executions = patterns.iter()
            .map(|p| p.usage_stats.success_count)
            .sum::<u32>();

        let success_rate = if total_executions > 0.0 {
            success_executions as f64 / total_executions
        } else {
            0.0
        };

        // Calculate average confidence improvement (simplified)
        let avg_confidence_improvement = patterns.iter()
            .filter(|p| p.usage_stats.total_executions > 0)
            .map(|p| p.confidence - 0.5) // Assume patterns start at 0.5
            .filter(|&improvement| improvement > 0.0)
            .sum::<f64>() / patterns.len().max(1) as f64;

        Ok(LearningAnalytics {
            total_patterns,
            high_confidence_patterns,
            recent_executions,
            success_rate,
            avg_confidence_improvement,
            patterns_pruned: 0, // Would be tracked in real implementation
            learning_events_processed: total_executions as u64,
        })
    }

    /// Analyze workload patterns for insights
    pub async fn analyze_workload_patterns(&self, session_id: &str) -> AriaResult<WorkloadAnalysis> {
        debug!("Analyzing workload patterns for session: {}", session_id);

        let patterns = self.pattern_processor.get_all_patterns().await?;
        
        // Filter patterns used in this session (simplified)
        let session_patterns: Vec<_> = patterns.iter()
            .filter(|p| p.usage_stats.total_executions > 0)
            .collect();

        let mut analysis = WorkloadAnalysis {
            session_id: session_id.to_string(),
            total_patterns_available: patterns.len(),
            patterns_used_in_session: session_patterns.len(),
            avg_pattern_confidence: if session_patterns.is_empty() {
                0.0
            } else {
                session_patterns.iter().map(|p| p.confidence).sum::<f64>() / session_patterns.len() as f64
            },
            most_successful_patterns: Vec::new(),
            recommended_optimizations: Vec::new(),
            performance_insights: Vec::new(),
        };

        // Find most successful patterns
        let mut successful_patterns: Vec<_> = session_patterns.iter()
            .filter(|p| p.usage_stats.total_executions >= 3)
            .map(|p| {
                let success_rate = p.usage_stats.success_count as f64 / p.usage_stats.total_executions as f64;
                (p, success_rate)
            })
            .collect();

        successful_patterns.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        for (pattern, success_rate) in successful_patterns.iter().take(5) {
            analysis.most_successful_patterns.push(format!(
                "{}: {:.1}% success, {:.3} confidence, {}ms avg",
                pattern.trigger,
                success_rate * 100.0,
                pattern.confidence,
                pattern.usage_stats.avg_execution_time.as_millis()
            ));
        }

        // Generate optimization recommendations
        if analysis.avg_pattern_confidence < 0.6 {
            analysis.recommended_optimizations.push(
                "Consider running more executions to improve pattern confidence".to_string()
            );
        }

        if session_patterns.len() < 3 {
            analysis.recommended_optimizations.push(
                "Session has limited patterns - more diverse workloads could improve learning".to_string()
            );
        }

        // Performance insights
        let fast_patterns = session_patterns.iter()
            .filter(|p| p.usage_stats.avg_execution_time.as_millis() < 2000)
            .count();

        if fast_patterns > 0 {
            analysis.performance_insights.push(format!(
                "{} patterns execute in under 2 seconds - good performance profile",
                fast_patterns
            ));
        }

        Ok(analysis)
    }

    /// Get reference to pattern processor
    pub fn get_pattern_processor(&self) -> &Arc<ContainerPatternProcessor> {
        &self.pattern_processor
    }

    /// Get all patterns from the underlying processor
    pub async fn get_all_patterns(&self) -> AriaResult<Vec<ContainerPattern>> {
        self.pattern_processor.get_all_patterns().await
    }

    /// Get a single pattern by its ID from the underlying processor
    pub async fn get_pattern_by_id(&self, pattern_id: &str) -> AriaResult<Option<ContainerPattern>> {
        self.pattern_processor.get_pattern_by_id(pattern_id).await
    }

    /// Remove a pattern by its ID from the underlying processor
    pub async fn remove_pattern(&self, pattern_id: &str) -> AriaResult<()> {
        self.pattern_processor.remove_pattern(pattern_id).await
    }

    /// Force pattern optimization (for testing/admin use)
    pub async fn force_optimization(&self) -> AriaResult<()> {
        info!("Forcing pattern optimization");
        self.optimize_pattern_performance().await
    }
}

/// Statistics from pattern optimization
#[derive(Debug, Default)]
struct OptimizationStats {
    patterns_removed: usize,
    patterns_boosted: usize,
    low_confidence_patterns: usize,
    stale_patterns: usize,
}

/// Workload analysis results
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkloadAnalysis {
    pub session_id: String,
    pub total_patterns_available: usize,
    pub patterns_used_in_session: usize,
    pub avg_pattern_confidence: f64,
    pub most_successful_patterns: Vec<String>,
    pub recommended_optimizations: Vec<String>,
    pub performance_insights: Vec<String>,
}

// Explicit Send + Sync for Axum compatibility
unsafe impl Send for WorkloadAnalysis {}
unsafe impl Sync for WorkloadAnalysis {} 