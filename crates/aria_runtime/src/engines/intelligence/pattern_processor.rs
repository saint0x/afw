/// Container Pattern Processor - Phase 2 Implementation
/// Processes container requests and learns from execution patterns

use crate::database::DatabaseManager;
use crate::engines::intelligence::types::*;
use crate::errors::{AriaError, AriaResult, ErrorCode, ErrorCategory, ErrorSeverity};
use regex::Regex;
use serde_json;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Configuration for pattern processing
#[derive(Debug, Clone)]
pub struct PatternProcessorConfig {
    pub confidence_threshold: f64,
    pub learning_rate: f64,
    pub max_patterns: usize,
    pub min_confidence: f64,
    pub max_confidence: f64,
}

impl Default for PatternProcessorConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.7,
            learning_rate: 0.05,
            max_patterns: 1000,
            min_confidence: 0.1,
            max_confidence: 0.99,
        }
    }
}

/// Container Pattern Processor - learns and matches container execution patterns
pub struct ContainerPatternProcessor {
    patterns: Arc<RwLock<HashMap<String, ContainerPattern>>>,
    database: Arc<DatabaseManager>,
    config: PatternProcessorConfig,
}

impl ContainerPatternProcessor {
    /// Create new pattern processor instance
    pub fn new(database: Arc<DatabaseManager>, config: PatternProcessorConfig) -> Self {
        Self {
            patterns: Arc::new(RwLock::new(HashMap::new())),
            database,
            config,
        }
    }

    /// Initialize pattern processor by loading patterns from database
    pub async fn initialize(&self) -> AriaResult<()> {
        info!("Initializing ContainerPatternProcessor");
        
        // Load existing patterns from database
        match self.load_patterns_from_database().await {
            Ok(count) => {
                info!("Loaded {} container patterns from database", count);
                Ok(())
            }
            Err(e) => {
                warn!("Failed to load patterns from database: {}. Starting fresh.", e);
                Ok(()) // Don't fail initialization if patterns can't be loaded
            }
        }
    }

    /// Process container request and find best pattern match
    pub async fn process_container_request(
        &self,
        request: &str,
        context: &ExecutionContext,
    ) -> AriaResult<PatternMatch> {
        debug!("Processing container request: {}", request);
        
        let patterns = self.patterns.read().map_err(|_| {
            AriaError::new(
                ErrorCode::ExecutionError,
                ErrorCategory::System,
                ErrorSeverity::Medium,
                "Failed to acquire patterns read lock"
            )
        })?;

        let mut best_match: Option<PatternMatch> = None;
        let mut best_score = 0.0;

        // Find best matching pattern
        for pattern in patterns.values() {
            let score = self.calculate_match_score(request, pattern, context).await?;
            
            if score > best_score && score >= self.config.confidence_threshold {
                best_score = score;
                
                let extracted_variables = self.extract_variables(request, pattern)?;
                let container_config = self.build_container_config(pattern, context, &extracted_variables).await?;
                
                best_match = Some(PatternMatch {
                    pattern: pattern.clone(),
                    confidence: score,
                    extracted_variables,
                    container_config,
                });
            }
        }

        match best_match {
            Some(match_result) => {
                info!("Found pattern match: {} (confidence: {:.3})", 
                      match_result.pattern.pattern_id, match_result.confidence);
                Ok(match_result)
            }
            None => {
                info!("No suitable pattern found, creating new pattern");
                self.create_new_pattern(request, context).await
            }
        }
    }

    /// Learn from container execution results
    pub async fn learn_from_execution(
        &self,
        pattern_id: &str,
        execution_result: &ContainerExecutionResult,
    ) -> AriaResult<()> {
        debug!("Learning from execution for pattern: {}", pattern_id);

        let mut patterns = self.patterns.write().map_err(|_| {
            AriaError::new(
                ErrorCode::ExecutionError,
                ErrorCategory::System,
                ErrorSeverity::Medium,
                "Failed to acquire patterns write lock"
            )
        })?;

        if let Some(pattern) = patterns.get_mut(pattern_id) {
            // Update usage statistics
            let previous_total = pattern.usage_stats.total_executions;
            pattern.usage_stats.total_executions += 1;
            
            if execution_result.success {
                pattern.usage_stats.success_count += 1;
            } else {
                pattern.usage_stats.failure_count += 1;
            }

            // Update average execution time
            let new_time = execution_result.execution_time;
            pattern.usage_stats.avg_execution_time = if previous_total == 0 {
                new_time
            } else {
                Duration::from_millis(
                    (pattern.usage_stats.avg_execution_time.as_millis() as u64 * previous_total as u64 
                     + new_time.as_millis() as u64) / (previous_total + 1) as u64
                )
            };

            pattern.usage_stats.last_used = Some(SystemTime::now());

            // Calculate new confidence based on success rate and execution time
            let success_rate = pattern.usage_stats.success_count as f64 / pattern.usage_stats.total_executions as f64;
            let time_factor = self.calculate_time_factor(execution_result.execution_time);
            let confidence_adjustment = self.config.learning_rate * success_rate * time_factor;

            let old_confidence = pattern.confidence;
            pattern.confidence = (pattern.confidence + confidence_adjustment)
                .max(self.config.min_confidence)
                .min(self.config.max_confidence);

            pattern.updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            info!("Updated pattern {} confidence: {:.3} -> {:.3} (success: {}, time: {}ms)",
                  pattern_id, old_confidence, pattern.confidence, 
                  execution_result.success, execution_result.execution_time.as_millis());

            // Save pattern to database
            self.save_pattern_to_database(pattern).await?;

            // Record learning feedback
            let feedback = LearningFeedback {
                feedback_id: Uuid::new_v4().to_string(),
                pattern_id: pattern_id.to_string(),
                execution_id: execution_result.execution_id.clone(),
                success: execution_result.success,
                execution_time: execution_result.execution_time,
                feedback_type: FeedbackType::Execution,
                confidence_delta: pattern.confidence - old_confidence,
                metadata: Some(serde_json::to_value(&execution_result.metadata)?),
                created_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            };

            self.record_learning_feedback(&feedback).await?;
        } else {
            warn!("Attempted to learn from execution for unknown pattern: {}", pattern_id);
        }

        Ok(())
    }

    /// Extract variables from pattern matching using regex
    fn extract_variables(
        &self,
        request: &str,
        pattern: &ContainerPattern,
    ) -> AriaResult<HashMap<String, String>> {
        let mut variables = HashMap::new();

        // Build regex from pattern trigger
        let trigger_regex = self.build_trigger_regex(&pattern.trigger)?;

        if let Some(captures) = trigger_regex.captures(request) {
            for (i, variable) in pattern.variables.iter().enumerate() {
                if let Some(capture) = captures.get(i + 1) {
                    let value = capture.as_str().trim().to_string();
                    
                    // Apply type conversion
                    let converted_value = self.convert_variable_value(&value, &variable.variable_type)?;
                    variables.insert(variable.name.clone(), converted_value);
                }
            }
        }

        // Add default values for missing variables
        for variable in &pattern.variables {
            if !variables.contains_key(&variable.name) {
                if let Some(default) = &variable.default_value {
                    variables.insert(variable.name.clone(), default.clone());
                }
            }
        }

        Ok(variables)
    }

    /// Calculate match score between request and pattern
    async fn calculate_match_score(
        &self,
        request: &str,
        pattern: &ContainerPattern,
        _context: &ExecutionContext,
    ) -> AriaResult<f64> {
        // Start with pattern's base confidence
        let mut score = pattern.confidence;

        // Calculate text similarity
        let text_similarity = self.calculate_text_similarity(request, &pattern.trigger);
        score *= text_similarity;

        // Boost score based on usage statistics
        if pattern.usage_stats.total_executions > 0 {
            let success_rate = pattern.usage_stats.success_count as f64 / pattern.usage_stats.total_executions as f64;
            score *= (0.5 + 0.5 * success_rate); // Boost between 0.5x and 1.0x
        }

        // Penalty for old patterns
        if let Some(last_used) = pattern.usage_stats.last_used {
            let age_days = SystemTime::now()
                .duration_since(last_used)
                .unwrap_or_default()
                .as_secs() / 86400; // Convert to days
            
            if age_days > 7 {
                score *= 0.9; // 10% penalty for old patterns
            }
        }

        Ok(score.max(0.0).min(1.0))
    }

    /// Create new pattern from request and context
    async fn create_new_pattern(
        &self,
        request: &str,
        context: &ExecutionContext,
    ) -> AriaResult<PatternMatch> {
        info!("Creating new pattern from request: {}", request);

        // Extract pattern trigger and variables
        let (trigger, variables) = self.analyze_request_for_pattern(request)?;
        
        // Create basic container configuration
        let container_config = self.create_default_container_config(request, context)?;

        let pattern_id = Uuid::new_v4().to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let pattern = ContainerPattern {
            pattern_id: pattern_id.clone(),
            trigger: trigger.clone(),
            container_config: container_config.clone(),
            confidence: 0.5, // Start with medium confidence
            usage_stats: PatternUsageStats::default(),
            variables,
            created_at: now,
            updated_at: now,
        };

        // Save to memory and database
        {
            let mut patterns = self.patterns.write().map_err(|_| {
                AriaError::new(
                    ErrorCode::ExecutionError,
                    ErrorCategory::System,
                    ErrorSeverity::Medium,
                    "Failed to acquire patterns write lock"
                )
            })?;
            patterns.insert(pattern_id.clone(), pattern.clone());
        }

        self.save_pattern_to_database(&pattern).await?;

        Ok(PatternMatch {
            pattern,
            confidence: 0.5,
            extracted_variables: HashMap::new(),
            container_config,
        })
    }

    /// Build container configuration from pattern and variables
    async fn build_container_config(
        &self,
        pattern: &ContainerPattern,
        _context: &ExecutionContext,
        variables: &HashMap<String, String>,
    ) -> AriaResult<ContainerConfig> {
        let mut config = pattern.container_config.clone();

        // Substitute variables in configuration
        config.image = self.substitute_variables(&config.image, variables)?;
        
        for cmd in &mut config.command {
            *cmd = self.substitute_variables(cmd, variables)?;
        }

        for (key, value) in &mut config.environment {
            *value = self.substitute_variables(value, variables)?;
        }

        if let Some(ref mut workdir) = config.working_directory {
            *workdir = self.substitute_variables(workdir, variables)?;
        }

        Ok(config)
    }

    /// Build regex pattern from trigger string
    fn build_trigger_regex(&self, trigger: &str) -> AriaResult<Regex> {
        // Convert pattern triggers like "build * project" to regex
        let escaped = regex::escape(trigger);
        let pattern = escaped.replace(r"\*", r"([^\\s]+)").replace(r"\?\?\?", r"(.+?)");
        
        Regex::new(&format!("(?i)^{}$", pattern))
            .map_err(|e| AriaError::new(
                ErrorCode::ParameterResolutionError,
                ErrorCategory::System,
                ErrorSeverity::Medium,
                &format!("Invalid regex pattern: {}", e)
            ))
    }

    /// Calculate text similarity between request and trigger
    fn calculate_text_similarity(&self, text1: &str, text2: &str) -> f64 {
        let text1_lower = text1.to_lowercase();
        let text2_lower = text2.to_lowercase();
        let words1: Vec<&str> = text1_lower.split_whitespace().collect();
        let words2: Vec<&str> = text2_lower.split_whitespace().collect();

        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }

        let common_words = words1.iter()
            .filter(|word| words2.contains(word))
            .count();

        let total_unique_words = words1.len().max(words2.len());
        common_words as f64 / total_unique_words as f64
    }

    /// Calculate time factor for confidence adjustment
    fn calculate_time_factor(&self, execution_time: Duration) -> f64 {
        // Reward fast executions, penalize slow ones
        let time_ms = execution_time.as_millis() as f64;
        
        if time_ms < 1000.0 {
            1.2 // 20% bonus for sub-second execution
        } else if time_ms < 5000.0 {
            1.0 // Normal factor
        } else if time_ms < 30000.0 {
            0.8 // 20% penalty for slow execution
        } else {
            0.6 // 40% penalty for very slow execution
        }
    }

    /// Analyze request to extract pattern trigger and variables
    fn analyze_request_for_pattern(&self, request: &str) -> AriaResult<(String, Vec<PatternVariable>)> {
        let words: Vec<&str> = request.split_whitespace().collect();
        let mut trigger = String::new();
        let mut variables = Vec::new();
        
        for (i, word) in words.iter().enumerate() {
            // Identify potential variables (file names, paths, etc.)
            if word.contains('.') || word.starts_with('/') || word.starts_with("./") {
                trigger.push_str("*");
                variables.push(PatternVariable {
                    name: format!("param_{}", variables.len()),
                    pattern: r"[\w./\-]+".to_string(),
                    variable_type: if word.contains('.') { 
                        VariableType::Path 
                    } else { 
                        VariableType::String 
                    },
                    default_value: Some(word.to_string()),
                });
            } else {
                if i > 0 {
                    trigger.push(' ');
                }
                trigger.push_str(word);
            }
        }

        Ok((trigger, variables))
    }

    /// Create default container configuration for new patterns
    fn create_default_container_config(
        &self,
        request: &str,
        _context: &ExecutionContext,
    ) -> AriaResult<ContainerConfig> {
        // Analyze request to determine likely container needs
        let request_lower = request.to_lowercase();
        
        let (image, command) = if request_lower.contains("rust") || request_lower.contains("cargo") {
            ("rust:latest".to_string(), vec!["cargo".to_string(), "build".to_string()])
        } else if request_lower.contains("node") || request_lower.contains("npm") {
            ("node:latest".to_string(), vec!["npm".to_string(), "run".to_string()])
        } else if request_lower.contains("python") || request_lower.contains("pip") {
            ("python:3.11".to_string(), vec!["python".to_string()])
        } else if request_lower.contains("docker") {
            ("docker:latest".to_string(), vec!["docker".to_string()])
        } else {
            ("ubuntu:22.04".to_string(), vec!["bash".to_string(), "-c".to_string()])
        };

        Ok(ContainerConfig {
            image,
            command,
            environment: HashMap::new(),
            working_directory: Some("/workspace".to_string()),
            resource_limits: Some(ResourceLimits {
                memory_mb: Some(1024),
                cpu_cores: Some(1.0),
                disk_mb: Some(2048),
            }),
            network_config: None,
            volumes: vec![VolumeMount {
                host_path: ".".to_string(),
                container_path: "/workspace".to_string(),
                read_only: false,
            }],
        })
    }

    /// Convert variable value based on type
    fn convert_variable_value(&self, value: &str, var_type: &VariableType) -> AriaResult<String> {
        match var_type {
            VariableType::String | VariableType::Path | VariableType::Command => Ok(value.to_string()),
            VariableType::Integer => {
                value.parse::<i64>()
                    .map(|_| value.to_string())
                    .map_err(|_| AriaError::new(
                        ErrorCode::ParameterResolutionError,
                        ErrorCategory::System,
                        ErrorSeverity::Medium,
                        &format!("Invalid integer: {}", value)
                    ))
            }
            VariableType::Float => {
                value.parse::<f64>()
                    .map(|_| value.to_string())
                    .map_err(|_| AriaError::new(
                        ErrorCode::ParameterResolutionError,
                        ErrorCategory::System,
                        ErrorSeverity::Medium,
                        &format!("Invalid float: {}", value)
                    ))
            }
            VariableType::Boolean => {
                match value.to_lowercase().as_str() {
                    "true" | "yes" | "1" | "on" => Ok("true".to_string()),
                    "false" | "no" | "0" | "off" => Ok("false".to_string()),
                    _ => Err(AriaError::new(
                        ErrorCode::ParameterResolutionError,
                        ErrorCategory::System,
                        ErrorSeverity::Medium,
                        &format!("Invalid boolean: {}", value)
                    ))
                }
            }
        }
    }

    /// Substitute variables in text with their values
    fn substitute_variables(&self, text: &str, variables: &HashMap<String, String>) -> AriaResult<String> {
        let mut result = text.to_string();
        
        for (name, value) in variables {
            let placeholder = format!("${{{}}}", name);
            result = result.replace(&placeholder, value);
        }
        
        Ok(result)
    }

    /// Load patterns from database
    async fn load_patterns_from_database(&self) -> AriaResult<usize> {
        debug!("Loading container patterns from database");
        
        // Note: This would use the intelligence database tables from Phase 1
        // For now, we'll start with empty patterns as the database integration
        // will be completed when the database queries are implemented
        
        Ok(0) // Return 0 patterns loaded for now
    }

    /// Save pattern to database
    async fn save_pattern_to_database(&self, pattern: &ContainerPattern) -> AriaResult<()> {
        debug!("Saving pattern {} to database", pattern.pattern_id);
        
        // TODO: Implement database save using the container_patterns table
        // This will be implemented with the database integration
        
        Ok(())
    }

    /// Record learning feedback in database
    async fn record_learning_feedback(&self, feedback: &LearningFeedback) -> AriaResult<()> {
        debug!("Recording learning feedback for pattern {}", feedback.pattern_id);
        
        // TODO: Implement database save using the learning_feedback table
        // This will be implemented with the database integration
        
        Ok(())
    }

    /// Get all patterns (for optimization and analysis)
    pub async fn get_all_patterns(&self) -> AriaResult<Vec<ContainerPattern>> {
        let patterns = self.patterns.read().map_err(|_| {
            AriaError::new(
                ErrorCode::ExecutionError,
                ErrorCategory::System,
                ErrorSeverity::Medium,
                "Failed to acquire patterns read lock"
            )
        })?;
        
        Ok(patterns.values().cloned().collect())
    }

    /// Remove pattern by ID
    pub async fn remove_pattern(&self, pattern_id: &str) -> AriaResult<()> {
        info!("Removing pattern: {}", pattern_id);
        
        let mut patterns = self.patterns.write().map_err(|_| {
            AriaError::new(
                ErrorCode::ExecutionError,
                ErrorCategory::System,
                ErrorSeverity::Medium,
                "Failed to acquire patterns write lock"
            )
        })?;
        
        patterns.remove(pattern_id);
        
        // TODO: Remove from database as well
        
        Ok(())
    }

    /// Boost pattern confidence
    pub async fn boost_pattern_confidence(&self, pattern_id: &str, boost: f64) -> AriaResult<()> {
        let mut patterns = self.patterns.write().map_err(|_| {
            AriaError::new(
                ErrorCode::ExecutionError,
                ErrorCategory::System,
                ErrorSeverity::Medium,
                "Failed to acquire patterns write lock"
            )
        })?;
        
        if let Some(pattern) = patterns.get_mut(pattern_id) {
            let old_confidence = pattern.confidence;
            pattern.confidence = (pattern.confidence + boost).min(self.config.max_confidence);
            pattern.updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            info!("Boosted pattern {} confidence: {:.3} -> {:.3}",
                  pattern_id, old_confidence, pattern.confidence);
            
            self.save_pattern_to_database(pattern).await?;
        }
        
        Ok(())
    }

    /// Get pattern statistics
    pub async fn get_pattern_stats(&self) -> AriaResult<(usize, f64, f64)> {
        let patterns = self.patterns.read().map_err(|_| {
            AriaError::new(
                ErrorCode::ExecutionError,
                ErrorCategory::System,
                ErrorSeverity::Medium,
                "Failed to acquire patterns read lock"
            )
        })?;
        
        let total_patterns = patterns.len();
        let avg_confidence = if total_patterns > 0 {
            patterns.values().map(|p| p.confidence).sum::<f64>() / total_patterns as f64
        } else {
            0.0
        };
        
        let total_executions = patterns.values()
            .map(|p| p.usage_stats.total_executions as f64)
            .sum::<f64>();
        
        Ok((total_patterns, avg_confidence, total_executions))
    }
} 