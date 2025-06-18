/// Intelligence HTTP API Endpoints
/// Provides REST API access to the intelligence system for pattern management,
/// context analysis, and learning analytics.

use crate::{
    engines::intelligence::{types::*, IntelligenceEngine},
    engines::AriaEngines,
};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
    Router,
};

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use tracing::{debug, info, warn};
use chrono;
use tokio::time::Instant;

/// Intelligence endpoints state
#[derive(Clone)]
pub struct IntelligenceEndpoints {
    intelligence: Arc<IntelligenceEngine>,
}

impl IntelligenceEndpoints {
    /// Create new intelligence endpoints
    pub fn new(intelligence: Arc<IntelligenceEngine>) -> Self {
        Self { intelligence }
    }
}

/// Query parameters for analysis endpoints
#[derive(Debug, Deserialize)]
pub struct AnalysisQuery {
    pub session_id: Option<String>,
    pub max_context_nodes: Option<usize>,
    pub include_patterns: Option<bool>,
}

/// Query parameters for pattern endpoints
#[derive(Debug, Deserialize)]
pub struct PatternQuery {
    pub min_confidence: Option<f64>,
    pub limit: Option<usize>,
}

/// Query parameters for context endpoints
#[derive(Debug, Deserialize)]
pub struct ContextQuery {
    pub max_depth: Option<u8>,
    pub include_metadata: Option<bool>,
}

/// Query parameters for analytics endpoints
#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub days: Option<u32>,
    pub include_details: Option<bool>,
}

/// Request body for intelligence analysis
#[derive(Debug, Clone, Deserialize)]
pub struct IntelligenceAnalysisRequest {
    pub request: String,
    pub session_id: String,
    pub requirements: Option<ContainerRequirements>,
    pub context_hints: Option<Vec<String>>,
}

/// Request body for pattern confidence updates
#[derive(Debug, Clone, Deserialize)]
pub struct PatternUpdateRequest {
    pub execution_success: bool,
    pub execution_time_ms: Option<u64>,
    pub feedback_notes: Option<String>,
}

/// Request body for pattern optimization
#[derive(Debug, Clone, Deserialize)]
pub struct OptimizationRequest {
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
pub struct AnalyticsResponse {
    pub success: bool,
    pub analytics: Option<LearningAnalytics>,
    pub workload_analysis: Option<WorkloadAnalysis>,
    pub intelligence_metrics: Option<IntelligenceMetrics>,
}

/// Generic success response
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

/// Create intelligence HTTP router
pub fn create_intelligence_router() -> Router<Arc<AriaEngines>> {
    Router::new()
        .route("/health", get(handle_health))
}

async fn handle_health() -> Result<Json<SuccessResponse>, StatusCode> {
    Ok(Json(SuccessResponse {
        success: true,
        message: "Intelligence system operational".to_string(),
    }))
} 