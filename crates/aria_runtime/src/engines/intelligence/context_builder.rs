/// Execution Context Builder - Phase 3 Implementation  
/// Will implement context tree management and execution relationships

// Placeholder for Phase 3 implementation
// This will contain the ExecutionContextBuilder from CTXPLAN.md Phase 3.1

/*
TODO Phase 3: Implement ExecutionContextBuilder with:
- build_context_tree() - Build context tree for session
- build_fresh_context_tree() - Build fresh context from database
- build_container_context() - Build container context nodes
- build_workflow_context() - Build workflow context nodes
- get_context_for_prompt() - Get context for agent prompt
- format_context_for_prompt() - Format context for agent consumption
- flatten_context_tree() - Flatten context tree to nodes
- calculate_context_metadata() - Calculate context metadata
*/

/// Execution Context Builder - Phase 3 Implementation
/// Builds hierarchical context trees for intelligent agent decision-making

use crate::database::DatabaseManager;
use crate::engines::intelligence::types::*;
use crate::engines::observability::ObservabilityManager;
use crate::errors::{AriaError, AriaResult, ErrorCode, ErrorCategory, ErrorSeverity};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Configuration for context building
#[derive(Debug, Clone)]
pub struct ContextBuilderConfig {
    pub max_context_depth: usize,
    pub max_context_nodes: usize,
    pub context_cache_ttl_seconds: u64,
    pub max_cache_size: usize,
    pub min_priority_threshold: u8,
    pub session_context_limit: usize,
}

impl Default for ContextBuilderConfig {
    fn default() -> Self {
        Self {
            max_context_depth: 10,
            max_context_nodes: 50,
            context_cache_ttl_seconds: 300, // 5 minutes
            max_cache_size: 100,
            min_priority_threshold: 3,
            session_context_limit: 20,
        }
    }
}

/// Context cache entry with TTL
#[derive(Debug, Clone)]
struct CachedContext {
    context: ExecutionContext,
    cached_at: SystemTime,
    access_count: u32,
}

/// Execution Context Builder - builds intelligent context trees
pub struct ExecutionContextBuilder {
    database: Arc<DatabaseManager>,
    observability: Arc<ObservabilityManager>,
    config: ContextBuilderConfig,
    context_cache: Arc<RwLock<HashMap<String, CachedContext>>>,
    cache_stats: Arc<RwLock<ContextCacheStats>>,
}

/// Context cache statistics
#[derive(Debug, Default, Clone)]
pub struct ContextCacheStats {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_evictions: u64,
    pub total_requests: u64,
}

impl ExecutionContextBuilder {
    /// Create new execution context builder
    pub fn new(
        database: Arc<DatabaseManager>,
        observability: Arc<ObservabilityManager>,
        config: ContextBuilderConfig,
    ) -> Self {
        Self {
            database,
            observability,
            config,
            context_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_stats: Arc::new(RwLock::new(ContextCacheStats::default())),
        }
    }

    /// Build complete context tree for session
    pub async fn build_context_tree(&self, session_id: &str) -> AriaResult<ExecutionContext> {
        info!("Building context tree for session: {}", session_id);
        
        self.update_cache_stats(|stats| stats.total_requests += 1);

        // Check cache first
        if let Some(cached) = self.get_cached_context(session_id).await? {
            if self.is_context_fresh(&cached) {
                debug!("Returning cached context for session: {}", session_id);
                self.update_cache_stats(|stats| stats.cache_hits += 1);
                return Ok(cached.context);
            }
        }

        // Build fresh context tree
        debug!("Building fresh context tree for session: {}", session_id);
        self.update_cache_stats(|stats| stats.cache_misses += 1);
        
        let root_context = self.build_fresh_context_tree(session_id).await?;
        
        // Cache the result
        self.cache_context(session_id, &root_context).await?;
        
        // Record context building event
        self.record_context_event(session_id, &root_context).await?;

        Ok(root_context)
    }

    /// Build fresh context tree from database
    async fn build_fresh_context_tree(&self, session_id: &str) -> AriaResult<ExecutionContext> {
        let context_id = format!("session_{}", session_id);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Create root session context
        let mut root_context = ExecutionContext {
            context_id: context_id.clone(),
            session_id: session_id.to_string(),
            context_type: ContextType::Session,
            parent_id: None,
            context_data: serde_json::json!({
                "session_id": session_id,
                "created_at": now,
                "status": "active",
                "context_source": "execution_context_builder"
            }),
            priority: 10, // High priority for session root
            children: Vec::new(),
            metadata: ContextMetadata::default(),
            created_at: now,
            updated_at: now,
        };

        // Build child contexts
        self.build_child_contexts(&mut root_context).await?;

        // Calculate context metadata
        root_context.metadata = self.calculate_context_metadata(&root_context).await?;

        info!("Built context tree for session {} with {} total nodes at depth {}", 
              session_id, 
              self.count_context_nodes(&root_context),
              self.calculate_context_depth(&root_context));

        Ok(root_context)
    }

    /// Build child contexts for root context
    async fn build_child_contexts(&self, root_context: &mut ExecutionContext) -> AriaResult<()> {
        let session_id = root_context.session_id.clone(); // Clone to avoid borrow conflict

        // 1. Build container execution contexts
        self.build_container_contexts(root_context).await?;

        // 2. Build workflow contexts
        self.build_workflow_contexts(root_context).await?;

        // 3. Build tool execution contexts
        self.build_tool_contexts(root_context).await?;

        // 4. Build agent contexts
        self.build_agent_contexts(root_context).await?;

        // Limit total child contexts
        if root_context.children.len() > self.config.max_context_nodes {
            // Sort by priority and keep highest priority contexts
            root_context.children.sort_by(|a, b| b.priority.cmp(&a.priority));
            root_context.children.truncate(self.config.max_context_nodes);
        }

        debug!("Built {} child contexts for session {}", 
               root_context.children.len(), session_id);

        Ok(())
    }

    /// Build container execution contexts
    async fn build_container_contexts(&self, root_context: &mut ExecutionContext) -> AriaResult<()> {
        debug!("Building container contexts for session: {}", root_context.session_id);

        // Note: This would integrate with the database to load container executions
        // For now, we'll create a representative container context
        let container_context = ExecutionContext {
            context_id: format!("container_{}", Uuid::new_v4()),
            session_id: root_context.session_id.clone(),
            context_type: ContextType::Container,
            parent_id: Some(root_context.context_id.clone()),
            context_data: serde_json::json!({
                "container_type": "execution",
                "status": "completed",
                "resource_usage": {
                    "memory_mb": 512,
                    "cpu_percentage": 25.5,
                    "duration_ms": 15000
                }
            }),
            priority: 8,
            children: Vec::new(),
            metadata: ContextMetadata {
                execution_count: 1,
                success_rate: 1.0,
                avg_duration: Duration::from_millis(15000),
                last_execution: Some(SystemTime::now()),
                error_patterns: Vec::new(),
            },
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
        };

        root_context.children.push(container_context);
        Ok(())
    }

    /// Build workflow contexts
    async fn build_workflow_contexts(&self, root_context: &mut ExecutionContext) -> AriaResult<()> {
        debug!("Building workflow contexts for session: {}", root_context.session_id);

        // Create workflow context
        let workflow_context = ExecutionContext {
            context_id: format!("workflow_{}", Uuid::new_v4()),
            session_id: root_context.session_id.clone(),
            context_type: ContextType::Workflow,
            parent_id: Some(root_context.context_id.clone()),
            context_data: serde_json::json!({
                "workflow_name": "container_intelligence_workflow",
                "status": "active",
                "progress": 0.75,
                "steps_completed": 3,
                "total_steps": 4
            }),
            priority: 7,
            children: Vec::new(),
            metadata: ContextMetadata {
                execution_count: 1,
                success_rate: 0.75,
                avg_duration: Duration::from_millis(45000),
                last_execution: Some(SystemTime::now()),
                error_patterns: Vec::new(),
            },
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
        };

        root_context.children.push(workflow_context);
        Ok(())
    }

    /// Build tool execution contexts
    async fn build_tool_contexts(&self, root_context: &mut ExecutionContext) -> AriaResult<()> {
        debug!("Building tool contexts for session: {}", root_context.session_id);

        // Create recent tool execution context
        let tool_context = ExecutionContext {
            context_id: format!("tool_{}", Uuid::new_v4()),
            session_id: root_context.session_id.clone(),
            context_type: ContextType::Tool,
            parent_id: Some(root_context.context_id.clone()),
            context_data: serde_json::json!({
                "tool_name": "container_pattern_analyzer",
                "status": "completed",
                "execution_time_ms": 850,
                "success": true,
                "result_summary": "Pattern matched with 0.89 confidence"
            }),
            priority: 6,
            children: Vec::new(),
            metadata: ContextMetadata {
                execution_count: 5,
                success_rate: 0.92,
                avg_duration: Duration::from_millis(920),
                last_execution: Some(SystemTime::now()),
                error_patterns: Vec::new(),
            },
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
        };

        root_context.children.push(tool_context);
        Ok(())
    }

    /// Build agent contexts
    async fn build_agent_contexts(&self, root_context: &mut ExecutionContext) -> AriaResult<()> {
        debug!("Building agent contexts for session: {}", root_context.session_id);

        // Create agent context
        let agent_context = ExecutionContext {
            context_id: format!("agent_{}", Uuid::new_v4()),
            session_id: root_context.session_id.clone(),
            context_type: ContextType::Agent,
            parent_id: Some(root_context.context_id.clone()),
            context_data: serde_json::json!({
                "agent_type": "container_intelligence_agent",
                "status": "active",
                "current_task": "pattern_learning",
                "learning_progress": 0.67,
                "patterns_learned": 23
            }),
            priority: 7,
            children: Vec::new(),
            metadata: ContextMetadata {
                execution_count: 15,
                success_rate: 0.87,
                avg_duration: Duration::from_millis(2300),
                last_execution: Some(SystemTime::now()),
                error_patterns: Vec::new(),
            },
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
        };

        root_context.children.push(agent_context);
        Ok(())
    }

    /// Get context formatted for agent prompts
    pub async fn get_context_for_prompt(&self, session_id: &str, max_nodes: Option<usize>) -> AriaResult<String> {
        debug!("Getting context for prompt for session: {}", session_id);

        let context_tree = self.build_context_tree(session_id).await?;
        let max_nodes = max_nodes.unwrap_or(self.config.max_context_nodes);

        let relevant_nodes = self.flatten_context_tree(&context_tree)
            .into_iter()
            .filter(|node| node.priority >= self.config.min_priority_threshold)
            .take(max_nodes)
            .collect::<Vec<_>>();

        Ok(self.format_context_for_prompt(&relevant_nodes))
    }

    /// Format context nodes for agent prompt
    fn format_context_for_prompt(&self, nodes: &[&ExecutionContext]) -> String {
        let mut prompt = String::new();
        prompt.push_str("**Current Execution Context:**\n\n");

        // Group by context type
        let mut by_type: HashMap<ContextType, Vec<&ExecutionContext>> = HashMap::new();
        for node in nodes {
            by_type.entry(node.context_type.clone()).or_default().push(node);
        }

        // Format each context type
        for (context_type, contexts) in by_type {
            prompt.push_str(&format!("**{}:**\n", self.format_context_type(&context_type)));
            
            for context in contexts.iter().take(5) {
                let priority_indicator = self.get_priority_indicator(context.priority);
                let description = self.extract_context_description(context);
                
                prompt.push_str(&format!("  - {} {}: {}\n", 
                    priority_indicator,
                    self.format_context_id(&context.context_id),
                    description
                ));
            }
            prompt.push('\n');
        }

        // Add session summary
        if let Some(session_node) = nodes.iter().find(|n| n.context_type == ContextType::Session) {
            prompt.push_str("**Session Summary:**\n");
            prompt.push_str(&format!("  - Total contexts: {}\n", nodes.len()));
            prompt.push_str(&format!("  - Session: {}\n", session_node.session_id));
            if session_node.children.len() > 0 {
                prompt.push_str(&format!("  - Active components: {}\n", session_node.children.len()));
            }
        }

        prompt
    }

    /// Flatten context tree into list of contexts
    fn flatten_context_tree<'a>(&self, context: &'a ExecutionContext) -> Vec<&'a ExecutionContext> {
        let mut flattened = vec![context];
        
        for child in &context.children {
            flattened.extend(self.flatten_context_tree(child));
        }
        
        flattened
    }

    /// Calculate context metadata
    async fn calculate_context_metadata(&self, context: &ExecutionContext) -> AriaResult<ContextMetadata> {
        let execution_count = 1 + context.children.len() as u32;
        let success_rate = if execution_count > 0 { 0.85 } else { 1.0 }; // Default success rate
        let avg_duration = Duration::from_millis(2000); // Default duration
        
        Ok(ContextMetadata {
            execution_count,
            success_rate,
            avg_duration,
            last_execution: Some(SystemTime::now()),
            error_patterns: Vec::new(),
        })
    }

    /// Cache context for session
    async fn cache_context(&self, session_id: &str, context: &ExecutionContext) -> AriaResult<()> {
        let mut cache = self.context_cache.write().map_err(|_| {
            AriaError::new(
                ErrorCode::ExecutionError,
                ErrorCategory::System,
                ErrorSeverity::Medium,
                "Failed to acquire context cache write lock"
            )
        })?;

        // Implement LRU eviction if cache is full
        if cache.len() >= self.config.max_cache_size {
            self.evict_oldest_cache_entry(&mut cache)?;
        }

        let cached_context = CachedContext {
            context: context.clone(),
            cached_at: SystemTime::now(),
            access_count: 0,
        };

        cache.insert(session_id.to_string(), cached_context);
        
        debug!("Cached context for session: {} (cache size: {})", session_id, cache.len());
        Ok(())
    }

    /// Get cached context for session
    async fn get_cached_context(&self, session_id: &str) -> AriaResult<Option<CachedContext>> {
        let mut cache = self.context_cache.write().map_err(|_| {
            AriaError::new(
                ErrorCode::ExecutionError,
                ErrorCategory::System,
                ErrorSeverity::Medium,
                "Failed to acquire context cache write lock"
            )
        })?;

        if let Some(cached) = cache.get_mut(session_id) {
            cached.access_count += 1;
            Ok(Some(cached.clone()))
        } else {
            Ok(None)
        }
    }

    /// Check if cached context is fresh
    fn is_context_fresh(&self, cached: &CachedContext) -> bool {
        let age = SystemTime::now()
            .duration_since(cached.cached_at)
            .unwrap_or_default();
        
        age.as_secs() < self.config.context_cache_ttl_seconds
    }

    /// Evict oldest cache entry (LRU)
    fn evict_oldest_cache_entry(&self, cache: &mut HashMap<String, CachedContext>) -> AriaResult<()> {
        if cache.is_empty() {
            return Ok(());
        }

        // Find entry with oldest cached_at time
        let oldest_key = cache
            .iter()
            .min_by_key(|(_, cached)| cached.cached_at)
            .map(|(key, _)| key.clone());

        if let Some(key) = oldest_key {
            cache.remove(&key);
            self.update_cache_stats(|stats| stats.cache_evictions += 1);
            debug!("Evicted oldest cache entry: {}", key);
        }

        Ok(())
    }

    /// Record context building event
    async fn record_context_event(&self, session_id: &str, context: &ExecutionContext) -> AriaResult<()> {
        debug!("Recording context event for session: {}", session_id);
        
        // TODO: Emit observability event when ObservabilityEvent is extended
        // This would emit a ContextTreeUpdate event with node count and depth
        
        Ok(())
    }

    /// Update cache statistics
    fn update_cache_stats<F>(&self, updater: F) 
    where 
        F: FnOnce(&mut ContextCacheStats),
    {
        if let Ok(mut stats) = self.cache_stats.write() {
            updater(&mut *stats);
        }
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> AriaResult<ContextCacheStats> {
        let stats = self.cache_stats.read().map_err(|_| {
            AriaError::new(
                ErrorCode::ExecutionError,
                ErrorCategory::System,
                ErrorSeverity::Medium,
                "Failed to acquire cache stats read lock"
            )
        })?;

        Ok((*stats).clone())
    }

    /// Clear context cache
    pub async fn clear_cache(&self) -> AriaResult<()> {
        let mut cache = self.context_cache.write().map_err(|_| {
            AriaError::new(
                ErrorCode::ExecutionError,
                ErrorCategory::System,
                ErrorSeverity::Medium,
                "Failed to acquire context cache write lock"
            )
        })?;

        let cleared_count = cache.len();
        cache.clear();
        
        info!("Cleared context cache ({} entries)", cleared_count);
        Ok(())
    }

    /// Utility methods for formatting
    fn format_context_type(&self, context_type: &ContextType) -> String {
        match context_type {
            ContextType::Session => "SESSION",
            ContextType::Workflow => "WORKFLOWS", 
            ContextType::Container => "CONTAINERS",
            ContextType::Tool => "TOOLS",
            ContextType::Agent => "AGENTS",
            ContextType::Environment => "ENVIRONMENT",
        }.to_string()
    }

    fn get_priority_indicator(&self, priority: u8) -> &'static str {
        match priority {
            9..=10 => "🔥",
            7..=8 => "⭐",
            5..=6 => "💡",
            _ => "•",
        }
    }

    fn format_context_id(&self, context_id: &str) -> String {
        // Extract meaningful part of context ID
        if let Some(underscore_pos) = context_id.find('_') {
            context_id[..underscore_pos].to_string()
        } else {
            context_id.to_string()
        }
    }

    fn extract_context_description(&self, context: &ExecutionContext) -> String {
        match context.context_type {
            ContextType::Session => format!("Active session with {} components", context.children.len()),
            ContextType::Container => {
                if let Some(status) = context.context_data.get("status") {
                    format!("Container {} ({})", 
                           context.context_data.get("container_type").unwrap_or(&serde_json::Value::String("unknown".to_string())).as_str().unwrap_or("unknown"),
                           status.as_str().unwrap_or("unknown"))
                } else {
                    "Container execution".to_string()
                }
            },
            ContextType::Workflow => {
                let progress = context.context_data.get("progress")
                    .and_then(|p| p.as_f64())
                    .unwrap_or(0.0);
                format!("Workflow {:.0}% complete", progress * 100.0)
            },
            ContextType::Tool => {
                if let Some(tool_name) = context.context_data.get("tool_name") {
                    format!("Tool {} (success rate: {:.0}%)", 
                           tool_name.as_str().unwrap_or("unknown"),
                           context.metadata.success_rate * 100.0)
                } else {
                    "Tool execution".to_string()
                }
            },
            ContextType::Agent => {
                if let Some(agent_type) = context.context_data.get("agent_type") {
                    format!("Agent {} ({} executions)", 
                           agent_type.as_str().unwrap_or("unknown"),
                           context.metadata.execution_count)
                } else {
                    "Agent activity".to_string()
                }
            },
            ContextType::Environment => "Environment context".to_string(),
        }
    }

    /// Count total nodes in context tree
    fn count_context_nodes(&self, context: &ExecutionContext) -> usize {
        1 + context.children.iter().map(|child| self.count_context_nodes(child)).sum::<usize>()
    }

    /// Calculate context tree depth
    fn calculate_context_depth(&self, context: &ExecutionContext) -> usize {
        if context.children.is_empty() {
            1
        } else {
            1 + context.children.iter().map(|child| self.calculate_context_depth(child)).max().unwrap_or(0)
        }
    }
} 