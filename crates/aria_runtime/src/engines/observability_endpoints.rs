use crate::engines::observability::{ObservabilityManager, EventFilter, ErrorSeverity};
use crate::engines::streaming::{StreamingService, StreamQuery};
use crate::error::AriaError;
use axum::{
    extract::{Query, State, Path},
    http::{header, HeaderMap, StatusCode},
    response::{Json, Response, Sse},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

/// Query parameters for metrics endpoint
#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    /// Format (json or prometheus)
    #[serde(default = "default_format")]
    pub format: String,
    /// Include system metrics
    #[serde(default = "default_true")]
    pub system: bool,
    /// Include runtime metrics
    #[serde(default = "default_true")]
    pub runtime: bool,
    /// Include database metrics
    #[serde(default = "default_true")]
    pub database: bool,
    /// Include container metrics
    #[serde(default = "default_true")]
    pub containers: bool,
    /// Include LLM metrics
    #[serde(default = "default_true")]
    pub llm: bool,
}

fn default_format() -> String {
    "json".to_string()
}

fn default_true() -> bool {
    true
}

/// Query parameters for logs endpoint
#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    /// Number of log entries to return
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// Log level filter (debug, info, warn, error)
    pub level: Option<String>,
    /// Component filter
    pub component: Option<String>,
    /// Session ID filter
    pub session_id: Option<String>,
    /// Start timestamp
    pub since: Option<u64>,
    /// End timestamp
    pub until: Option<u64>,
}

fn default_limit() -> u32 {
    100
}

/// Query parameters for errors endpoint
#[derive(Debug, Deserialize)]
pub struct ErrorsQuery {
    /// Number of error entries to return
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// Minimum severity (low, medium, high, critical)
    pub min_severity: Option<String>,
    /// Component filter
    pub component: Option<String>,
    /// Session ID filter
    pub session_id: Option<String>,
    /// Start timestamp
    pub since: Option<u64>,
    /// End timestamp
    pub until: Option<u64>,
}

/// Health check request
#[derive(Debug, Deserialize)]
pub struct HealthQuery {
    /// Include detailed component health
    #[serde(default = "default_true")]
    pub detailed: bool,
}

/// Observability service state
pub struct ObservabilityEndpoints {
    observability: Arc<ObservabilityManager>,
    streaming: Arc<StreamingService>,
}

impl ObservabilityEndpoints {
    pub fn new(observability: Arc<ObservabilityManager>, streaming: Arc<StreamingService>) -> Self {
        Self {
            observability,
            streaming,
        }
    }
}

/// Create router for observability endpoints
pub fn create_observability_router(
    observability: Arc<ObservabilityManager>,
    streaming: Arc<StreamingService>,
) -> Router {
    let endpoints = Arc::new(ObservabilityEndpoints::new(observability, streaming));

    Router::new()
        // Metrics endpoints
        .route("/metrics", get(handle_metrics))
        .route("/metrics/prometheus", get(handle_prometheus_metrics))
        
        // Logs endpoints
        .route("/logs", get(handle_logs))
        .route("/logs/stream", get(handle_logs_stream))
        
        // Errors endpoints
        .route("/errors", get(handle_errors))
        .route("/errors/stream", get(handle_errors_stream))
        
        // Health endpoints
        .route("/health", get(handle_health))
        .route("/health/detailed", get(handle_detailed_health))
        
        // Streaming endpoints
        .route("/stream", get(handle_sse_stream))
        .route("/stream/stats", get(handle_stream_stats))
        .route("/stream/active", get(handle_active_streams))
        
        // Debug endpoints
        .route("/debug/metrics", get(handle_debug_metrics))
        .route("/debug/state", get(handle_debug_state))
        
        .with_state(endpoints)
}

/// Handle metrics endpoint
async fn handle_metrics(
    Query(query): Query<MetricsQuery>,
    State(endpoints): State<Arc<ObservabilityEndpoints>>,
) -> Result<Response, StatusCode> {
    match endpoints.observability.get_metrics().await {
        metrics => {
            match query.format.as_str() {
                "prometheus" => {
                    let prometheus_format = convert_to_prometheus_format(&metrics, &query);
                    Ok(Response::builder()
                        .header(header::CONTENT_TYPE, "text/plain; version=0.0.4")
                        .body(prometheus_format.into())
                        .unwrap())
                },
                "json" | _ => {
                    let filtered_metrics = filter_metrics(&metrics, &query);
                    Ok(Response::builder()
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(serde_json::to_string_pretty(&filtered_metrics).unwrap().into())
                        .unwrap())
                }
            }
        }
    }
}

/// Handle Prometheus metrics endpoint
async fn handle_prometheus_metrics(
    State(endpoints): State<Arc<ObservabilityEndpoints>>,
) -> Result<Response, StatusCode> {
    let metrics = endpoints.observability.get_metrics().await;
    let query = MetricsQuery {
        format: "prometheus".to_string(),
        system: true,
        runtime: true,
        database: true,
        containers: true,
        llm: true,
    };
    
    let prometheus_format = convert_to_prometheus_format(&metrics, &query);
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "text/plain; version=0.0.4")
        .body(prometheus_format.into())
        .unwrap())
}

/// Handle logs endpoint
async fn handle_logs(
    Query(query): Query<LogsQuery>,
    State(endpoints): State<Arc<ObservabilityEndpoints>>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    match endpoints.observability.get_recent_logs(query.limit, query.level).await {
        Ok(logs) => Ok(Json(logs)),
        Err(e) => {
            error!("Failed to get logs: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Handle logs streaming endpoint
async fn handle_logs_stream(
    Query(query): Query<StreamQuery>,
    State(endpoints): State<Arc<ObservabilityEndpoints>>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>, StatusCode> {
    // Create logs-only stream
    let logs_query = StreamQuery {
        stream_type: Some("logs".to_string()),
        events: Some("log".to_string()),
        ..query
    };
    
    match endpoints.streaming.create_sse_stream(logs_query).await {
        Ok(stream) => {
            Ok(Sse::new(stream)
                .keep_alive(
                    axum::response::sse::KeepAlive::new()
                        .interval(Duration::from_secs(30))
                        .text("keep-alive")
                ))
        },
        Err(e) => {
            error!("Failed to create logs stream: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Handle errors endpoint
async fn handle_errors(
    Query(query): Query<ErrorsQuery>,
    State(endpoints): State<Arc<ObservabilityEndpoints>>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    // Implementation would query audit_logs table for errors
    // For now, return empty array
    Ok(Json(vec![]))
}

/// Handle errors streaming endpoint
async fn handle_errors_stream(
    Query(query): Query<StreamQuery>,
    State(endpoints): State<Arc<ObservabilityEndpoints>>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>, StatusCode> {
    // Create errors-only stream
    let errors_query = StreamQuery {
        stream_type: Some("errors".to_string()),
        events: Some("error".to_string()),
        ..query
    };
    
    match endpoints.streaming.create_sse_stream(errors_query).await {
        Ok(stream) => {
            Ok(Sse::new(stream)
                .keep_alive(
                    axum::response::sse::KeepAlive::new()
                        .interval(Duration::from_secs(30))
                        .text("keep-alive")
                ))
        },
        Err(e) => {
            error!("Failed to create errors stream: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Handle health endpoint
async fn handle_health(
    State(endpoints): State<Arc<ObservabilityEndpoints>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let health = endpoints.observability.get_health().await;
    
    let response = serde_json::json!({
        "status": match health.overall {
            crate::engines::observability::HealthState::Healthy => "healthy",
            crate::engines::observability::HealthState::Degraded => "degraded",
            crate::engines::observability::HealthState::Unhealthy => "unhealthy",
            crate::engines::observability::HealthState::Unknown => "unknown",
        },
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        "components": health.components.len()
    });
    
    Ok(Json(response))
}

/// Handle detailed health endpoint
async fn handle_detailed_health(
    State(endpoints): State<Arc<ObservabilityEndpoints>>,
) -> Result<Json<crate::engines::observability::HealthStatus>, StatusCode> {
    let health = endpoints.observability.get_health().await;
    Ok(Json(health))
}

/// Handle SSE stream endpoint
async fn handle_sse_stream(
    Query(query): Query<StreamQuery>,
    State(endpoints): State<Arc<ObservabilityEndpoints>>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>, StatusCode> {
    match endpoints.streaming.create_sse_stream(query).await {
        Ok(stream) => {
            Ok(Sse::new(stream)
                .keep_alive(
                    axum::response::sse::KeepAlive::new()
                        .interval(Duration::from_secs(30))
                        .text("keep-alive")
                ))
        },
        Err(e) => {
            error!("Failed to create SSE stream: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Handle stream statistics endpoint
async fn handle_stream_stats(
    State(endpoints): State<Arc<ObservabilityEndpoints>>,
) -> Result<Json<crate::engines::streaming::StreamStats>, StatusCode> {
    let stats = endpoints.streaming.get_stats().await;
    Ok(Json(stats))
}

/// Handle active streams endpoint
async fn handle_active_streams(
    State(endpoints): State<Arc<ObservabilityEndpoints>>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let streams = endpoints.streaming.get_active_streams().await;
    Ok(Json(streams))
}

/// Handle debug metrics endpoint
async fn handle_debug_metrics(
    State(endpoints): State<Arc<ObservabilityEndpoints>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let metrics = endpoints.observability.get_metrics().await;
    let stream_stats = endpoints.streaming.get_stats().await;
    
    let debug_info = serde_json::json!({
        "metrics": metrics,
        "streaming": stream_stats,
        "debug_timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });
    
    Ok(Json(debug_info))
}

/// Handle debug state endpoint
async fn handle_debug_state(
    State(endpoints): State<Arc<ObservabilityEndpoints>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let health = endpoints.observability.get_health().await;
    let active_streams = endpoints.streaming.get_active_streams().await;
    
    let debug_state = serde_json::json!({
        "health": health,
        "active_streams": active_streams,
        "debug_timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });
    
    Ok(Json(debug_state))
}

/// Filter metrics based on query parameters
fn filter_metrics(
    metrics: &crate::engines::observability::RuntimeMetrics,
    query: &MetricsQuery,
) -> serde_json::Value {
    let mut filtered = serde_json::Map::new();
    
    filtered.insert("timestamp".to_string(), serde_json::Value::Number(metrics.timestamp.into()));
    
    if query.system {
        filtered.insert("system".to_string(), serde_json::to_value(&metrics.system).unwrap());
    }
    
    if query.runtime {
        filtered.insert("runtime".to_string(), serde_json::to_value(&metrics.runtime).unwrap());
    }
    
    if query.database {
        filtered.insert("database".to_string(), serde_json::to_value(&metrics.database).unwrap());
    }
    
    if query.containers {
        filtered.insert("containers".to_string(), serde_json::to_value(&metrics.containers).unwrap());
    }
    
    if query.llm {
        filtered.insert("llm".to_string(), serde_json::to_value(&metrics.llm).unwrap());
    }
    
    serde_json::Value::Object(filtered)
}

/// Convert metrics to Prometheus format
fn convert_to_prometheus_format(
    metrics: &crate::engines::observability::RuntimeMetrics,
    query: &MetricsQuery,
) -> String {
    let mut prometheus = String::new();
    
    // Add header
    prometheus.push_str("# HELP aria_runtime_info Runtime information\n");
    prometheus.push_str("# TYPE aria_runtime_info gauge\n");
    prometheus.push_str(&format!("aria_runtime_info{{version=\"1.0.0\"}} 1 {}\n", metrics.timestamp * 1000));
    
    if query.system {
        // System metrics
        prometheus.push_str("# HELP aria_system_cpu_usage_percent System CPU usage percentage\n");
        prometheus.push_str("# TYPE aria_system_cpu_usage_percent gauge\n");
        prometheus.push_str(&format!("aria_system_cpu_usage_percent {} {}\n", 
            metrics.system.cpu_usage_percent, metrics.timestamp * 1000));
            
        prometheus.push_str("# HELP aria_system_memory_usage_bytes System memory usage in bytes\n");
        prometheus.push_str("# TYPE aria_system_memory_usage_bytes gauge\n");
        prometheus.push_str(&format!("aria_system_memory_usage_bytes {} {}\n", 
            metrics.system.memory_usage_bytes, metrics.timestamp * 1000));
    }
    
    if query.runtime {
        // Runtime metrics
        prometheus.push_str("# HELP aria_runtime_active_sessions Number of active sessions\n");
        prometheus.push_str("# TYPE aria_runtime_active_sessions gauge\n");
        prometheus.push_str(&format!("aria_runtime_active_sessions {} {}\n", 
            metrics.runtime.active_sessions, metrics.timestamp * 1000));
            
        prometheus.push_str("# HELP aria_runtime_tool_executions_total Total number of tool executions\n");
        prometheus.push_str("# TYPE aria_runtime_tool_executions_total counter\n");
        prometheus.push_str(&format!("aria_runtime_tool_executions_total {} {}\n", 
            metrics.runtime.tool_executions, metrics.timestamp * 1000));
    }
    
    if query.containers {
        // Container metrics
        prometheus.push_str("# HELP aria_containers_running Number of running containers\n");
        prometheus.push_str("# TYPE aria_containers_running gauge\n");
        prometheus.push_str(&format!("aria_containers_running {} {}\n", 
            metrics.containers.containers_running, metrics.timestamp * 1000));
            
        prometheus.push_str("# HELP aria_containers_total_created Total number of containers created\n");
        prometheus.push_str("# TYPE aria_containers_total_created counter\n");
        prometheus.push_str(&format!("aria_containers_total_created {} {}\n", 
            metrics.containers.containers_created, metrics.timestamp * 1000));
    }
    
    if query.llm {
        // LLM metrics
        prometheus.push_str("# HELP aria_llm_requests_total Total number of LLM requests\n");
        prometheus.push_str("# TYPE aria_llm_requests_total counter\n");
        prometheus.push_str(&format!("aria_llm_requests_total {} {}\n", 
            metrics.llm.requests_total, metrics.timestamp * 1000));
            
        prometheus.push_str("# HELP aria_llm_tokens_used_total Total number of tokens used\n");
        prometheus.push_str("# TYPE aria_llm_tokens_used_total counter\n");
        prometheus.push_str(&format!("aria_llm_tokens_used_total {} {}\n", 
            metrics.llm.tokens_used, metrics.timestamp * 1000));
    }
    
    prometheus
} 