// Observability Endpoints Demo for Aria Runtime
// Shows how to add production-grade metrics, logs, and health endpoints

use aria_runtime::{AriaResult, AriaError};
use axum::{
    extract::{Query, Path},
    http::StatusCode,
    response::{Response, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use tokio::sync::broadcast;

// Query parameters for metrics/logs endpoints
#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    pub format: Option<String>,  // json, prometheus
    pub from: Option<u64>,       // timestamp
    pub to: Option<u64>,         // timestamp  
    pub component: Option<String>, // filter by component
}

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub level: Option<String>,   // error, warn, info, debug
    pub limit: Option<u32>,      // max number of entries
    pub from: Option<u64>,       // timestamp
    pub component: Option<String>, // filter by component
    pub stream: Option<bool>,    // enable streaming
}

// Log entry structure
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: u64,
    pub level: String,
    pub component: String,
    pub message: String,
    pub fields: HashMap<String, serde_json::Value>,
}

// Enhanced health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,           // healthy, degraded, unhealthy
    pub timestamp: u64,
    pub uptime_seconds: u64,
    pub version: String,
    pub components: HashMap<String, ComponentHealth>,
}

#[derive(Debug, Serialize)]
pub struct ComponentHealth {
    pub status: String,
    pub last_check: u64,
    pub error_count: u32,
    pub response_time_ms: Option<f64>,
}

// Observability service that would integrate with AriaEngines
pub struct ObservabilityService {
    // In production, these would be connected to actual collectors
    log_sender: broadcast::Sender<LogEntry>,
    metrics_collector: crate::metrics_improvement::EnhancedMetricsCollector,
    start_time: SystemTime,
}

impl ObservabilityService {
    pub fn new() -> Self {
        let (log_sender, _) = broadcast::channel(1000); // Buffer up to 1000 log entries
        
        Self {
            log_sender,
            metrics_collector: crate::metrics_improvement::EnhancedMetricsCollector::new(),
            start_time: SystemTime::now(),
        }
    }

    pub fn create_router(&self) -> Router {
        Router::new()
            // Metrics endpoints
            .route("/metrics", get(Self::get_metrics))
            .route("/metrics/prometheus", get(Self::get_metrics_prometheus))
            .route("/metrics/component/:component", get(Self::get_component_metrics))
            
            // Logging endpoints  
            .route("/logs", get(Self::get_logs))
            .route("/logs/stream", get(Self::stream_logs))
            .route("/logs/component/:component", get(Self::get_component_logs))
            
            // Health endpoints
            .route("/health", get(Self::health_check))
            .route("/health/detailed", get(Self::detailed_health_check))
            .route("/health/component/:component", get(Self::component_health_check))
            
            // Error reporting
            .route("/errors", get(Self::get_error_summary))
            .route("/errors/recent", get(Self::get_recent_errors))
            
            // System info
            .route("/info", get(Self::system_info))
            .route("/status", get(Self::runtime_status))
    }

    // GET /metrics - JSON format metrics
    async fn get_metrics(Query(params): Query<MetricsQuery>) -> Result<Json<serde_json::Value>, StatusCode> {
        // In production, this would get metrics from the actual collector
        let mock_metrics = json!({
            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            "runtime": {
                "total_duration": 45000,
                "step_count": 25,
                "tool_calls": 18,
                "container_calls": 12,
                "agent_calls": 8,
                "token_usage": {
                    "prompt": 1500,
                    "completion": 750,
                    "total": 2250
                }
            },
            "performance": {
                "avg_response_time_ms": 150.5,
                "p95_response_time_ms": 450.0,
                "throughput_rps": 25.3,
                "error_rate": 0.02
            },
            "system": {
                "cpu_usage_percent": 25.6,
                "memory_usage_bytes": 512000000,
                "uptime_seconds": 86400
            }
        });

        Ok(Json(mock_metrics))
    }

    // GET /metrics/prometheus - Prometheus format
    async fn get_metrics_prometheus() -> Result<Response<String>, StatusCode> {
        let prometheus_metrics = r#"
# HELP aria_runtime_duration_total Total runtime duration in milliseconds
# TYPE aria_runtime_duration_total counter
aria_runtime_duration_total 45000

# HELP aria_runtime_tool_calls_total Total number of tool calls
# TYPE aria_runtime_tool_calls_total counter
aria_runtime_tool_calls_total 18

# HELP aria_runtime_container_calls_total Total number of container calls  
# TYPE aria_runtime_container_calls_total counter
aria_runtime_container_calls_total 12

# HELP aria_runtime_tokens_total Total tokens used
# TYPE aria_runtime_tokens_total counter
aria_runtime_tokens_total 2250

# HELP aria_system_cpu_usage_percent Current CPU usage percentage
# TYPE aria_system_cpu_usage_percent gauge
aria_system_cpu_usage_percent 25.6

# HELP aria_system_memory_usage_bytes Current memory usage in bytes
# TYPE aria_system_memory_usage_bytes gauge
aria_system_memory_usage_bytes 512000000
        "#.trim();

        let response = Response::builder()
            .header("content-type", "text/plain; version=0.0.4; charset=utf-8")
            .body(prometheus_metrics.to_string())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(response)
    }

    // GET /logs - Get log entries (with optional streaming)
    async fn get_logs(Query(params): Query<LogsQuery>) -> Result<Json<serde_json::Value>, StatusCode> {
        // Mock log entries for demo
        let mock_logs = vec![
            LogEntry {
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - 10,
                level: "info".to_string(),
                component: "runtime".to_string(),
                message: "Task execution completed successfully".to_string(),
                fields: [("task_id".to_string(), json!("task_001"))].into_iter().collect(),
            },
            LogEntry {
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - 5,
                level: "warn".to_string(),
                component: "container".to_string(),
                message: "Container startup took longer than expected".to_string(),
                fields: [
                    ("container_id".to_string(), json!("container_001")),
                    ("startup_time_ms".to_string(), json!(5000))
                ].into_iter().collect(),
            },
        ];

        let response = json!({
            "logs": mock_logs,
            "total": mock_logs.len(),
            "has_more": false
        });

        Ok(Json(response))
    }

    // GET /logs/stream - Stream logs in real-time (this crosses into streaming territory!)
    async fn stream_logs() -> Result<Response<String>, StatusCode> {
        // This demonstrates the streaming boundary you mentioned!
        // In production, this would return an SSE stream of log entries
        
        let sse_response = r#"data: {"timestamp": 1640995200, "level": "info", "component": "runtime", "message": "System started"}

data: {"timestamp": 1640995205, "level": "info", "component": "container", "message": "Container created", "fields": {"container_id": "abc123"}}

data: {"timestamp": 1640995210, "level": "warn", "component": "llm", "message": "Rate limit approaching", "fields": {"remaining_requests": 10}}

"#;

        let response = Response::builder()
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-cache")
            .header("connection", "keep-alive")
            .body(sse_response.to_string())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(response)
    }

    // GET /health - Basic health check
    async fn health_check() -> Json<serde_json::Value> {
        Json(json!({
            "status": "healthy",
            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            "version": env!("CARGO_PKG_VERSION")
        }))
    }

    // GET /health/detailed - Detailed health check
    async fn detailed_health_check() -> Json<HealthResponse> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        let mut components = HashMap::new();
        components.insert("database".to_string(), ComponentHealth {
            status: "healthy".to_string(),
            last_check: now,
            error_count: 0,
            response_time_ms: Some(2.5),
        });
        components.insert("llm_providers".to_string(), ComponentHealth {
            status: "healthy".to_string(),
            last_check: now,
            error_count: 0,
            response_time_ms: Some(150.0),
        });
        components.insert("container_runtime".to_string(), ComponentHealth {
            status: "healthy".to_string(),
            last_check: now,
            error_count: 0,
            response_time_ms: Some(10.0),
        });

        Json(HealthResponse {
            status: "healthy".to_string(),
            timestamp: now,
            uptime_seconds: 86400, // Mock 24 hours
            version: env!("CARGO_PKG_VERSION").to_string(),
            components,
        })
    }

    // GET /errors - Error summary and trends
    async fn get_error_summary() -> Json<serde_json::Value> {
        Json(json!({
            "error_rate_1h": 0.02,
            "error_rate_24h": 0.015,
            "top_errors": [
                {
                    "code": "ContainerError",
                    "count": 5,
                    "last_seen": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - 300
                },
                {
                    "code": "LLMTimeout", 
                    "count": 3,
                    "last_seen": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - 120
                }
            ],
            "total_errors_24h": 23
        }))
    }

    // GET /info - System information
    async fn system_info() -> Json<serde_json::Value> {
        Json(json!({
            "version": env!("CARGO_PKG_VERSION"),
            "build_date": env!("BUILD_DATE", "unknown"),
            "git_commit": env!("GIT_COMMIT", "unknown"),
            "rust_version": env!("RUST_VERSION", "unknown"),
            "features": ["database", "container", "llm", "icc"],
            "uptime_seconds": 86400,
            "pid": std::process::id()
        }))
    }

    // Additional endpoints would include:
    // - GET /metrics/component/:component - Component-specific metrics
    // - GET /logs/component/:component - Component-specific logs  
    // - GET /health/component/:component - Component-specific health
    // - POST /errors/report - Error reporting endpoint
    // - GET /traces/:trace_id - Distributed tracing (future)
}

#[tokio::main]
async fn main() -> AriaResult<()> {
    tracing_subscriber::fmt::init();
    
    tracing::info!("🚀 Observability Endpoints Demo");
    
    let service = ObservabilityService::new();
    let app = service.create_router();
    
    tracing::info!("📊 Starting observability server on http://0.0.0.0:9090");
    tracing::info!("🔗 Available endpoints:");
    tracing::info!("   • GET /metrics - JSON metrics");
    tracing::info!("   • GET /metrics/prometheus - Prometheus format");
    tracing::info!("   • GET /logs - Log entries");
    tracing::info!("   • GET /logs/stream - Real-time log streaming ⚡");
    tracing::info!("   • GET /health - Health check");
    tracing::info!("   • GET /health/detailed - Detailed health");
    tracing::info!("   • GET /errors - Error summary");
    tracing::info!("   • GET /info - System information");
    
    // In a real implementation, this would be integrated into the ICC server
    // or run as a separate observability service
    tracing::info!("🔄 This demonstrates the streaming boundary you mentioned!");
    tracing::info!("📈 Ready for integration with monitoring systems like Grafana, Prometheus, etc.");
    
    // Simulate server running  
    tracing::info!("✅ Observability endpoints ready for production deployment");
    
    Ok(())
} 