#[derive(Debug, Deserialize)]
pub struct AnalysisQuery {
    pub session_id: Option<String>,
    pub max_context_nodes: Option<usize>,
    pub include_patterns: Option<bool>,
}

/// Query parameters for pattern endpoints
#[derive(Debug, Deserialize)]
pub struct PatternQuery {
    pub pattern_type: Option<String>,
    pub limit: Option<usize>,
}

/// Query parameters for context endpoints
#[derive(Debug, Deserialize)]
pub struct ContextQuery {
    pub max_depth: Option<usize>,
    pub priority_threshold: Option<u8>,
    pub format: Option<String>, // "json" or "prompt"
}

/// Query parameters for analytics endpoints
#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub session_id: Option<String>,
    pub time_range_hours: Option<u64>,
    pub include_detailed: Option<bool>,
}

/// Request body for intelligence analysis
#[derive(Debug, Clone, Deserialize)]
pub struct IntelligenceAnalysisRequest {
    pub session_id: String,
    pub requirements: Option<ContainerRequirements>,
    pub context_hints: Option<Vec<String>>,
}

/// Request body for pattern confidence updates
#[derive(Debug, Clone, Deserialize)]
pub struct PatternConfidenceUpdateRequest {
    pub execution_success: bool,
    pub execution_time_ms: Option<u64>,
    pub feedback_notes: Option<String>,
}

/// Request body for pattern optimization
#[derive(Debug, Clone, Deserialize)]
pub struct PatternOptimizationRequest {
    pub min_confidence_threshold: Option<f64>,
    pub max_pattern_age_days: Option<u32>,
    pub force_optimization: Option<bool>,
}

/// Response for intelligence analysis
#[derive(Debug, Serialize)]
pub struct IntelligenceAnalysisResponse {
    pub success: bool,
    pub result: Option<IntelligenceResult>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// Response for pattern list
#[derive(Debug, Serialize)]
pub struct PatternListResponse {
    pub success: bool,
    pub patterns: Vec<ContainerPattern>,
    pub total_count: usize,
    pub high_confidence_count: usize,
}

/// Response for context tree
#[derive(Debug, Serialize)]
pub struct ContextTreeResponse {
    pub success: bool,
    pub context: Option<ExecutionContext>,
    pub formatted_prompt: Option<String>,
    pub cache_stats: Option<serde_json::Value>,
}

/// Response for learning analytics
#[derive(Debug, Serialize)]
pub struct LearningAnalyticsResponse {
    pub success: bool,
    pub analytics: Option<LearningAnalytics>,
    pub workload_analysis: Option<WorkloadAnalysis>,
    pub intelligence_metrics: Option<IntelligenceMetrics>,
}

/// Generic success response 