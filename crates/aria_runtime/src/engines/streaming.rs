use crate::engines::observability::{ObservabilityEvent, EventFilter, ObservabilityManager};
use crate::errors::AriaError;
use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{Response, Sse},
    routing::get,
    Router,
};
use futures_util::stream::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Stream configuration for clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    pub stream_id: String,
    pub stream_type: StreamType,
    pub filters: EventFilter,
    pub buffer_size: usize,
    pub keep_alive_interval: Duration,
}

/// Types of streams available
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamType {
    /// All observability events
    All,
    /// Only metrics updates
    Metrics,
    /// Only error events
    Errors,
    /// Only log entries
    Logs,
    /// Only health updates
    Health,
    /// Tool execution events
    Tools,
    /// Container lifecycle events
    Containers,
    /// Agent execution events
    Agents,
    /// Custom filtered stream
    Custom,
}

/// SSE event for frontend compatibility
#[derive(Debug, Clone, Serialize)]
pub struct SseEvent {
    pub id: String,
    pub event: String,
    pub data: serde_json::Value,
    pub timestamp: u64,
}

/// Stream subscription query parameters
#[derive(Debug, Deserialize)]
pub struct StreamQuery {
    /// Stream type
    #[serde(default)]
    pub stream_type: Option<String>,
    /// Event types filter (comma-separated)
    pub events: Option<String>,
    /// Components filter (comma-separated)
    pub components: Option<String>,
    /// Session ID filter
    pub session_id: Option<String>,
    /// Minimum error severity
    pub min_severity: Option<String>,
    /// Buffer size for the stream
    #[serde(default = "default_buffer_size")]
    pub buffer_size: usize,
}

fn default_buffer_size() -> usize {
    1000
}

/// Stream statistics for monitoring
#[derive(Debug, Clone, Serialize)]
pub struct StreamStats {
    pub active_streams: u32,
    pub total_streams_created: u64,
    pub events_sent: u64,
    pub events_dropped: u64,
    pub avg_latency_ms: f64,
}

/// Active stream information
#[derive(Debug)]
struct ActiveStream {
    pub id: String,
    pub config: StreamConfig,
    pub created_at: std::time::Instant,
    pub events_sent: u64,
    pub last_activity: std::time::Instant,
}

/// Streaming service for event distribution
pub struct StreamingService {
    /// Reference to observability manager
    observability: Arc<ObservabilityManager>,
    
    /// Active streams registry
    active_streams: Arc<RwLock<HashMap<String, ActiveStream>>>,
    
    /// Stream statistics
    stats: Arc<RwLock<StreamStats>>,
    
    /// Service configuration
    config: StreamingConfig,
}

/// Streaming service configuration
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    pub max_concurrent_streams: usize,
    pub default_buffer_size: usize,
    pub keep_alive_interval: Duration,
    pub stream_timeout: Duration,
    pub max_event_size: usize,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            max_concurrent_streams: 1000,
            default_buffer_size: 1000,
            keep_alive_interval: Duration::from_secs(30),
            stream_timeout: Duration::from_secs(300), // 5 minutes
            max_event_size: 64 * 1024, // 64KB
        }
    }
}

impl StreamingService {
    /// Create new streaming service
    pub fn new(observability: Arc<ObservabilityManager>, config: StreamingConfig) -> Self {
        let initial_stats = StreamStats {
            active_streams: 0,
            total_streams_created: 0,
            events_sent: 0,
            events_dropped: 0,
            avg_latency_ms: 0.0,
        };

        info!("StreamingService initialized with max concurrent streams: {}", config.max_concurrent_streams);

        Self {
            observability,
            active_streams: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(initial_stats)),
            config,
        }
    }

    /// Start the streaming service
    pub async fn start(&self) -> Result<(), AriaError> {
        info!("Starting StreamingService...");

        // Start cleanup task for expired streams
        self.spawn_cleanup_task().await?;

        // Start statistics collection task
        self.spawn_stats_collector().await?;

        info!("StreamingService started successfully");
        Ok(())
    }

    /// Stop the streaming service
    pub async fn stop(&self) -> Result<(), AriaError> {
        info!("Stopping StreamingService...");

        // Clear all active streams
        let mut streams = self.active_streams.write().await;
        streams.clear();

        info!("StreamingService stopped");
        Ok(())
    }

    /// Create a new stream subscription
    pub async fn create_stream(&self, query: StreamQuery) -> Result<(String, broadcast::Receiver<ObservabilityEvent>), AriaError> {
        // Check concurrent stream limit
        let active_count = self.active_streams.read().await.len();
        if active_count >= self.config.max_concurrent_streams {
            return Err(AriaError::new(
                crate::errors::ErrorCode::SystemNotReady,
                crate::errors::ErrorCategory::System,
                crate::errors::ErrorSeverity::Medium,
                "Too many concurrent streams"
            ));
        }

        // Parse stream configuration
        let stream_type = self.parse_stream_type(&query.stream_type)?;
        let filter = self.build_event_filter(&query)?;
        
        let stream_id = Uuid::new_v4().to_string();
        let buffer_size = if query.buffer_size > 0 { query.buffer_size } else { self.config.default_buffer_size };

        let stream_config = StreamConfig {
            stream_id: stream_id.clone(),
            stream_type,
            filters: filter.clone(),
            buffer_size,
            keep_alive_interval: self.config.keep_alive_interval,
        };

        // Subscribe to observability events
        let receiver = self.observability.subscribe(filter).await?;

        // Register active stream
        let active_stream = ActiveStream {
            id: stream_id.clone(),
            config: stream_config.clone(),
            created_at: std::time::Instant::now(),
            events_sent: 0,
            last_activity: std::time::Instant::now(),
        };

        let mut streams = self.active_streams.write().await;
        streams.insert(stream_id.clone(), active_stream);

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.active_streams = streams.len() as u32;
        stats.total_streams_created += 1;

        debug!("Created new stream: {} (type: {:?})", stream_id, stream_config.stream_type);
        Ok((stream_id, receiver))
    }

    /// Remove a stream subscription
    pub async fn remove_stream(&self, stream_id: &str) -> Result<(), AriaError> {
        let mut streams = self.active_streams.write().await;
        if streams.remove(stream_id).is_some() {
            let mut stats = self.stats.write().await;
            stats.active_streams = streams.len() as u32;
            debug!("Removed stream: {}", stream_id);
        }
        Ok(())
    }

    /// Get streaming statistics
    pub async fn get_stats(&self) -> StreamStats {
        self.stats.read().await.clone()
    }

    /// Get active streams information
    pub async fn get_active_streams(&self) -> Vec<serde_json::Value> {
        let streams = self.active_streams.read().await;
        streams.values().map(|stream| {
            serde_json::json!({
                "id": stream.id,
                "type": stream.config.stream_type,
                "created_at": stream.created_at.elapsed().as_secs(),
                "events_sent": stream.events_sent,
                "last_activity": stream.last_activity.elapsed().as_secs()
            })
        }).collect()
    }

    /// Create SSE event stream
    pub async fn create_sse_stream(&self, query: StreamQuery) -> Result<impl Stream<Item = Result<axum::response::sse::Event, Infallible>>, AriaError> {
        let (stream_id, mut receiver) = self.create_stream(query).await?;
        let active_streams = Arc::clone(&self.active_streams);
        let stats = Arc::clone(&self.stats);
        let keep_alive_interval = self.config.keep_alive_interval;

        // Create SSE stream
        let stream = async_stream::stream! {
            let mut keep_alive_timer = tokio::time::interval(keep_alive_interval);
            
            loop {
                tokio::select! {
                    // Receive observability events
                    event_result = receiver.recv() => {
                        match event_result {
                            Ok(event) => {
                                // Convert to SSE event
                                let sse_event = convert_to_sse_event(event);
                                
                                // Update stream activity
                                {
                                    let mut streams = active_streams.write().await;
                                    if let Some(stream_info) = streams.get_mut(&stream_id) {
                                        stream_info.events_sent += 1;
                                        stream_info.last_activity = std::time::Instant::now();
                                    }
                                }
                                
                                // Update statistics
                                {
                                    let mut stream_stats = stats.write().await;
                                    stream_stats.events_sent += 1;
                                }
                                
                                // Yield SSE event
                                yield Ok(axum::response::sse::Event::default()
                                    .id(sse_event.id)
                                    .event(sse_event.event)
                                    .data(sse_event.data.to_string()));
                            },
                            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                                warn!("Stream {} lagged, skipped {} events", stream_id, skipped);
                                
                                // Update statistics
                                {
                                    let mut stream_stats = stats.write().await;
                                    stream_stats.events_dropped += skipped;
                                }
                                
                                // Send lag notification
                                yield Ok(axum::response::sse::Event::default()
                                    .event("lag")
                                    .data(format!("{{\"skipped\": {}}}", skipped)));
                            },
                            Err(broadcast::error::RecvError::Closed) => {
                                debug!("Stream {} closed", stream_id);
                                break;
                            }
                        }
                    },
                    
                    // Send keep-alive pings
                    _ = keep_alive_timer.tick() => {
                        yield Ok(axum::response::sse::Event::default()
                            .event("ping")
                            .data("{}"));
                    }
                }
            }
            
            // Cleanup on stream end
            let _ = active_streams.write().await.remove(&stream_id);
            {
                let mut stream_stats = stats.write().await;
                stream_stats.active_streams = stream_stats.active_streams.saturating_sub(1);
            }
        };

        Ok(stream)
    }

    /// Parse stream type from query
    fn parse_stream_type(&self, stream_type: &Option<String>) -> Result<StreamType, AriaError> {
        match stream_type.as_deref() {
            Some("all") | None => Ok(StreamType::All),
            Some("metrics") => Ok(StreamType::Metrics),
            Some("errors") => Ok(StreamType::Errors),
            Some("logs") => Ok(StreamType::Logs),
            Some("health") => Ok(StreamType::Health),
            Some("tools") => Ok(StreamType::Tools),
            Some("containers") => Ok(StreamType::Containers),
            Some("agents") => Ok(StreamType::Agents),
            Some("custom") => Ok(StreamType::Custom),
            Some(other) => Err(AriaError::new(
                crate::errors::ErrorCode::NotSupported,
                crate::errors::ErrorCategory::System,
                crate::errors::ErrorSeverity::Low,
                &format!("Unknown stream type: {}", other)
            )),
        }
    }

    /// Build event filter from query parameters
    fn build_event_filter(&self, query: &StreamQuery) -> Result<EventFilter, AriaError> {
        let event_types = query.events.as_ref().map(|events| {
            events.split(',').map(|s| s.trim().to_string()).collect()
        });

        let components = query.components.as_ref().map(|components| {
            components.split(',').map(|s| s.trim().to_string()).collect()
        });

        let severity_min = query.min_severity.as_ref().and_then(|severity| {
            match severity.to_lowercase().as_str() {
                "low" => Some(crate::engines::observability::ErrorSeverity::Low),
                "medium" => Some(crate::engines::observability::ErrorSeverity::Medium),
                "high" => Some(crate::engines::observability::ErrorSeverity::High),
                "critical" => Some(crate::engines::observability::ErrorSeverity::Critical),
                _ => None,
            }
        });

        Ok(EventFilter {
            event_types,
            components,
            severity_min,
            session_id: query.session_id.clone(),
        })
    }

    /// Spawn cleanup task for expired streams
    async fn spawn_cleanup_task(&self) -> Result<(), AriaError> {
        let active_streams = Arc::clone(&self.active_streams);
        let stats = Arc::clone(&self.stats);
        let timeout = self.config.stream_timeout;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // Check every minute
            
            loop {
                interval.tick().await;
                
                let mut streams = active_streams.write().await;
                let initial_count = streams.len();
                
                // Remove expired streams
                streams.retain(|_, stream| {
                    stream.last_activity.elapsed() < timeout
                });
                
                let removed_count = initial_count - streams.len();
                if removed_count > 0 {
                    debug!("Cleaned up {} expired streams", removed_count);
                    
                    // Update statistics
                    {
                        let mut stream_stats = stats.write().await;
                        stream_stats.active_streams = streams.len() as u32;
                    }
                }
            }
        });

        Ok(())
    }

    /// Spawn statistics collection task
    async fn spawn_stats_collector(&self) -> Result<(), AriaError> {
        let stats = Arc::clone(&self.stats);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                // Calculate average latency and other metrics
                // This is a simplified implementation
                {
                    let mut stream_stats = stats.write().await;
                    stream_stats.avg_latency_ms = 5.0; // Placeholder
                }
            }
        });

        Ok(())
    }
}

/// Convert observability event to SSE event
fn convert_to_sse_event(event: ObservabilityEvent) -> SseEvent {
    let (event_type, data, timestamp) = match &event {
        ObservabilityEvent::MetricsUpdate { timestamp, metrics } => {
            ("metrics", serde_json::to_value(metrics).unwrap(), *timestamp)
        },
        ObservabilityEvent::ErrorOccurred { timestamp, error } => {
            ("error", serde_json::to_value(error).unwrap(), *timestamp)
        },
        ObservabilityEvent::LogEntry { timestamp, level, message, target, fields } => {
            let log_data = serde_json::json!({
                "level": level,
                "message": message,
                "target": target,
                "fields": fields
            });
            ("log", log_data, *timestamp)
        },
        ObservabilityEvent::HealthUpdate { timestamp, status } => {
            ("health", serde_json::to_value(status).unwrap(), *timestamp)
        },
        ObservabilityEvent::ToolExecution { timestamp, tool_name, session_id, duration_ms, success, error } => {
            let tool_data = serde_json::json!({
                "tool_name": tool_name,
                "session_id": session_id,
                "duration_ms": duration_ms,
                "success": success,
                "error": error
            });
            ("tool", tool_data, *timestamp)
        },
        ObservabilityEvent::ContainerEvent { timestamp, container_id, event_type, metadata } => {
            let container_data = serde_json::json!({
                "container_id": container_id,
                "event_type": event_type,
                "metadata": metadata
            });
            ("container", container_data, *timestamp)
        },
        ObservabilityEvent::AgentExecution { timestamp, session_id, agent_name, step_count, tokens_used, duration_ms, success } => {
            let agent_data = serde_json::json!({
                "session_id": session_id,
                "agent_name": agent_name,
                "step_count": step_count,
                "tokens_used": tokens_used,
                "duration_ms": duration_ms,
                "success": success
            });
            ("agent", agent_data, *timestamp)
        },
        ObservabilityEvent::IntelligenceUpdate { timestamp, pattern_id, confidence_delta, learning_context } => {
            let intelligence_data = serde_json::json!({
                "pattern_id": pattern_id,
                "confidence_delta": confidence_delta,
                "learning_context": learning_context
            });
            ("intelligence", intelligence_data, *timestamp)
        },
        ObservabilityEvent::PatternMatch { timestamp, pattern_id, confidence, request, session_id } => {
            let pattern_data = serde_json::json!({
                "pattern_id": pattern_id,
                "confidence": confidence,
                "request": request,
                "session_id": session_id
            });
            ("pattern", pattern_data, *timestamp)
        },
        ObservabilityEvent::ContextTreeUpdate { timestamp, session_id, node_count, depth } => {
            let context_data = serde_json::json!({
                "session_id": session_id,
                "node_count": node_count,
                "depth": depth
            });
            ("context", context_data, *timestamp)
        },
    };

    SseEvent {
        id: Uuid::new_v4().to_string(),
        event: event_type.to_string(),
        data,
        timestamp,
    }
}

/// Create router for streaming endpoints
pub fn create_streaming_router(streaming_service: Arc<StreamingService>) -> Router {
    Router::new()
        .route("/stream", get(handle_sse_stream))
        .route("/stream/stats", get(handle_stream_stats))
        .route("/stream/active", get(handle_active_streams))
        .with_state(streaming_service)
}

/// Handle SSE stream endpoint
async fn handle_sse_stream(
    Query(query): Query<StreamQuery>,
    State(streaming_service): State<Arc<StreamingService>>,
) -> Result<Sse<impl Stream<Item = Result<axum::response::sse::Event, Infallible>>>, StatusCode> {
    match streaming_service.create_sse_stream(query).await {
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
    State(streaming_service): State<Arc<StreamingService>>,
) -> Result<axum::Json<StreamStats>, StatusCode> {
    let stats = streaming_service.get_stats().await;
    Ok(axum::Json(stats))
}

/// Handle active streams endpoint
async fn handle_active_streams(
    State(streaming_service): State<Arc<StreamingService>>,
) -> Result<axum::Json<Vec<serde_json::Value>>, StatusCode> {
    let streams = streaming_service.get_active_streams().await;
    Ok(axum::Json(streams))
} 