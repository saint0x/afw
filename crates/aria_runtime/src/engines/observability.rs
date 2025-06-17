use crate::database::DatabaseManager;
use crate::errors::{AriaError, ErrorSeverity as AriaErrorSeverity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Observability event types for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ObservabilityEvent {
    /// Metrics updated
    MetricsUpdate {
        timestamp: u64,
        metrics: RuntimeMetrics,
    },
    /// Error occurred
    ErrorOccurred {
        timestamp: u64,
        error: ErrorEvent,
    },
    /// Log entry
    LogEntry {
        timestamp: u64,
        level: String,
        message: String,
        target: String,
        fields: HashMap<String, String>,
    },
    /// System health check
    HealthUpdate {
        timestamp: u64,
        status: HealthStatus,
    },
    /// Tool execution event
    ToolExecution {
        timestamp: u64,
        tool_name: String,
        session_id: String,
        duration_ms: u64,
        success: bool,
        error: Option<String>,
    },
    /// Container lifecycle event
    ContainerEvent {
        timestamp: u64,
        container_id: String,
        event_type: String, // created, started, stopped, removed
        metadata: HashMap<String, String>,
    },
    /// Agent execution event
    AgentExecution {
        timestamp: u64,
        session_id: String,
        agent_name: String,
        step_count: u32,
        tokens_used: u32,
        duration_ms: u64,
        success: bool,
    },
    /// Intelligence update event
    IntelligenceUpdate {
        timestamp: u64,
        pattern_id: Option<String>,
        confidence_delta: f64,
        learning_context: serde_json::Value,
    },
    /// Pattern match event
    PatternMatch {
        timestamp: u64,
        pattern_id: String,
        confidence: f64,
        request: String,
        session_id: String,
    },
    /// Context tree update event
    ContextTreeUpdate {
        timestamp: u64,
        session_id: String,
        node_count: usize,
        depth: usize,
    },
}

/// Runtime metrics structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeMetrics {
    pub timestamp: u64,
    pub system: SystemMetrics,
    pub runtime: RuntimeStats,
    pub database: DatabaseStats,
    pub containers: ContainerStats,
    pub llm: LlmStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: u64,
    pub memory_total_bytes: u64,
    pub disk_usage_bytes: u64,
    pub disk_total_bytes: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStats {
    pub active_sessions: u32,
    pub total_sessions: u64,
    pub active_tasks: u32,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub tool_executions: u64,
    pub agent_invocations: u64,
    pub errors_last_hour: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub connections_active: u32,
    pub connections_total: u64,
    pub queries_executed: u64,
    pub queries_failed: u64,
    pub avg_query_time_ms: f64,
    pub database_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStats {
    pub containers_running: u32,
    pub containers_total: u64,
    pub containers_created: u64,
    pub containers_stopped: u64,
    pub containers_failed: u64,
    pub total_cpu_usage_percent: f64,
    pub total_memory_usage_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmStats {
    pub requests_total: u64,
    pub requests_successful: u64,
    pub requests_failed: u64,
    pub tokens_used: u64,
    pub tokens_cached: u64,
    pub avg_response_time_ms: f64,
    pub cost_estimate_usd: f64,
}

/// Health status for system components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub overall: HealthState,
    pub components: HashMap<String, ComponentHealth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub status: HealthState,
    pub last_check: u64,
    pub message: Option<String>,
    pub metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Error event for observability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub error_id: String,
    pub error_type: String,
    pub message: String,
    pub context: HashMap<String, String>,
    pub severity: ErrorSeverity,
    pub component: String,
    pub stack_trace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl From<AriaErrorSeverity> for ErrorSeverity {
    fn from(severity: AriaErrorSeverity) -> Self {
        match severity {
            AriaErrorSeverity::Low => ErrorSeverity::Low,
            AriaErrorSeverity::Medium => ErrorSeverity::Medium,
            AriaErrorSeverity::High => ErrorSeverity::High,
            AriaErrorSeverity::Critical => ErrorSeverity::Critical,
        }
    }
}

/// Event stream subscriber for SSE clients
#[derive(Debug)]
pub struct EventSubscriber {
    pub id: String,
    pub filter: EventFilter,
    pub sender: broadcast::Sender<ObservabilityEvent>,
}

/// Filter for observability events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    pub event_types: Option<Vec<String>>,
    pub components: Option<Vec<String>>,
    pub severity_min: Option<ErrorSeverity>,
    pub session_id: Option<String>,
}

impl Default for EventFilter {
    fn default() -> Self {
        Self {
            event_types: None,
            components: None,
            severity_min: None,
            session_id: None,
        }
    }
}

/// Main observability manager following AriaEngines patterns
pub struct ObservabilityManager {
    /// Database manager for persistent storage
    database: Arc<DatabaseManager>,
    
    /// Current runtime metrics
    metrics: Arc<RwLock<RuntimeMetrics>>,
    
    /// System health status
    health: Arc<RwLock<HealthStatus>>,
    
    /// Event broadcasting for streaming
    event_broadcaster: broadcast::Sender<ObservabilityEvent>,
    
    /// Active subscribers for SSE
    subscribers: Arc<RwLock<HashMap<String, EventSubscriber>>>,
    
    /// Metrics collection enabled flag
    enabled: bool,
}

impl ObservabilityManager {
    /// Create new observability manager
    pub fn new(database: Arc<DatabaseManager>, buffer_size: usize) -> Result<Self, AriaError> {
        let (event_broadcaster, _) = broadcast::channel(buffer_size);
        
        let initial_metrics = RuntimeMetrics {
            timestamp: current_timestamp(),
            system: SystemMetrics {
                cpu_usage_percent: 0.0,
                memory_usage_bytes: 0,
                memory_total_bytes: 0,
                disk_usage_bytes: 0,
                disk_total_bytes: 0,
                network_rx_bytes: 0,
                network_tx_bytes: 0,
                uptime_seconds: 0,
            },
            runtime: RuntimeStats {
                active_sessions: 0,
                total_sessions: 0,
                active_tasks: 0,
                completed_tasks: 0,
                failed_tasks: 0,
                tool_executions: 0,
                agent_invocations: 0,
                errors_last_hour: 0,
            },
            database: DatabaseStats {
                connections_active: 0,
                connections_total: 0,
                queries_executed: 0,
                queries_failed: 0,
                avg_query_time_ms: 0.0,
                database_size_bytes: 0,
            },
            containers: ContainerStats {
                containers_running: 0,
                containers_total: 0,
                containers_created: 0,
                containers_stopped: 0,
                containers_failed: 0,
                total_cpu_usage_percent: 0.0,
                total_memory_usage_bytes: 0,
            },
            llm: LlmStats {
                requests_total: 0,
                requests_successful: 0,
                requests_failed: 0,
                tokens_used: 0,
                tokens_cached: 0,
                avg_response_time_ms: 0.0,
                cost_estimate_usd: 0.0,
            },
        };

        let initial_health = HealthStatus {
            overall: HealthState::Unknown,
            components: HashMap::new(),
        };

        info!("ObservabilityManager initialized with buffer size: {}", buffer_size);

        Ok(Self {
            database,
            metrics: Arc::new(RwLock::new(initial_metrics)),
            health: Arc::new(RwLock::new(initial_health)),
            event_broadcaster,
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            enabled: true,
        })
    }

    /// Start the observability manager
    pub async fn start(&self) -> Result<(), AriaError> {
        if !self.enabled {
            debug!("ObservabilityManager is disabled, skipping start");
            return Ok(());
        }

        info!("Starting ObservabilityManager...");

        // Start metrics collection task
        self.spawn_metrics_collector().await?;
        
        // Start health monitoring task
        self.spawn_health_monitor().await?;
        
        // Start cleanup task for old data
        self.spawn_cleanup_task().await?;

        info!("ObservabilityManager started successfully");
        Ok(())
    }

    /// Stop the observability manager
    pub async fn stop(&self) -> Result<(), AriaError> {
        info!("Stopping ObservabilityManager...");
        
        // Clear subscribers
        let mut subscribers = self.subscribers.write().await;
        subscribers.clear();
        
        info!("ObservabilityManager stopped");
        Ok(())
    }

    /// Record a tool execution event
    pub async fn record_tool_execution(
        &self,
        tool_name: &str,
        session_id: &str,
        duration_ms: u64,
        success: bool,
        error: Option<String>,
    ) -> Result<(), AriaError> {
        if !self.enabled {
            return Ok(());
        }

        let event = ObservabilityEvent::ToolExecution {
            timestamp: current_timestamp(),
            tool_name: tool_name.to_string(),
            session_id: session_id.to_string(),
            duration_ms,
            success,
            error,
        };

        self.emit_event(event).await?;
        
        // Update runtime metrics
        let mut metrics = self.metrics.write().await;
        metrics.runtime.tool_executions += 1;
        metrics.timestamp = current_timestamp();

        Ok(())
    }

    /// Record a container event
    pub async fn record_container_event(
        &self,
        container_id: &str,
        event_type: &str,
        metadata: HashMap<String, String>,
    ) -> Result<(), AriaError> {
        if !self.enabled {
            return Ok(());
        }

        let event = ObservabilityEvent::ContainerEvent {
            timestamp: current_timestamp(),
            container_id: container_id.to_string(),
            event_type: event_type.to_string(),
            metadata,
        };

        self.emit_event(event).await?;

        // Update container metrics based on event type
        let mut metrics = self.metrics.write().await;
        match event_type {
            "created" => metrics.containers.containers_created += 1,
            "started" => metrics.containers.containers_running += 1,
            "stopped" => {
                metrics.containers.containers_running = metrics.containers.containers_running.saturating_sub(1);
                metrics.containers.containers_stopped += 1;
            },
            "failed" => metrics.containers.containers_failed += 1,
            _ => {}
        }
        metrics.timestamp = current_timestamp();

        Ok(())
    }

    /// Record an agent execution event
    pub async fn record_agent_execution(
        &self,
        session_id: &str,
        agent_name: &str,
        step_count: u32,
        tokens_used: u32,
        duration_ms: u64,
        success: bool,
    ) -> Result<(), AriaError> {
        if !self.enabled {
            return Ok(());
        }

        let event = ObservabilityEvent::AgentExecution {
            timestamp: current_timestamp(),
            session_id: session_id.to_string(),
            agent_name: agent_name.to_string(),
            step_count,
            tokens_used,
            duration_ms,
            success,
        };

        self.emit_event(event).await?;

        // Update runtime metrics
        let mut metrics = self.metrics.write().await;
        metrics.runtime.agent_invocations += 1;
        metrics.llm.tokens_used += tokens_used as u64;
        metrics.timestamp = current_timestamp();

        Ok(())
    }

    /// Record an error event
    pub async fn record_error(
        &self,
        error: &AriaError,
        component: &str,
        context: HashMap<String, String>,
    ) -> Result<(), AriaError> {
        if !self.enabled {
            return Ok(());
        }

        let error_event = ErrorEvent {
            error_id: Uuid::new_v4().to_string(),
            error_type: format!("{:?}", error),
            message: error.to_string(),
            context,
            severity: error.severity.clone().into(),
            component: component.to_string(),
            stack_trace: Some(format!("{:?}", error)),
        };

        let event = ObservabilityEvent::ErrorOccurred {
            timestamp: current_timestamp(),
            error: error_event,
        };

        self.emit_event(event).await?;

        // Update error metrics
        let mut metrics = self.metrics.write().await;
        metrics.runtime.errors_last_hour += 1;
        metrics.timestamp = current_timestamp();

        Ok(())
    }

    /// Subscribe to events for SSE streaming
    pub async fn subscribe(&self, filter: EventFilter) -> Result<broadcast::Receiver<ObservabilityEvent>, AriaError> {
        let subscriber_id = Uuid::new_v4().to_string();
        let (sender, receiver) = broadcast::channel(1000);
        
        let subscriber = EventSubscriber {
            id: subscriber_id.clone(),
            filter,
            sender,
        };

        let mut subscribers = self.subscribers.write().await;
        subscribers.insert(subscriber_id.clone(), subscriber);

        debug!("New event subscriber added: {}", subscriber_id);
        Ok(receiver)
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> RuntimeMetrics {
        self.metrics.read().await.clone()
    }

    /// Get current health status
    pub async fn get_health(&self) -> HealthStatus {
        self.health.read().await.clone()
    }

    /// Get recent log entries from database
    pub async fn get_recent_logs(&self, limit: u32, level: Option<String>) -> Result<Vec<serde_json::Value>, AriaError> {
        // Implementation would query audit_logs table
        // Placeholder for now
        Ok(vec![])
    }

    /// Emit an event to all subscribers
    pub async fn emit_event(&self, event: ObservabilityEvent) -> Result<(), AriaError> {
        // Broadcast to main channel (ignore "no receivers" errors)
        if let Err(e) = self.event_broadcaster.send(event.clone()) {
            // Only warn if there should be receivers but the send still failed
            if self.event_broadcaster.receiver_count() > 0 {
                warn!("Failed to broadcast event with active receivers: {}", e);
            } else {
                debug!("No active receivers for event broadcast (this is normal)");
            }
        }

        // Send to filtered subscribers
        let subscribers = self.subscribers.read().await;
        for subscriber in subscribers.values() {
            if self.event_matches_filter(&event, &subscriber.filter) {
                if let Err(e) = subscriber.sender.send(event.clone()) {
                    debug!("Failed to send event to subscriber {}: {}", subscriber.id, e);
                }
            }
        }

        Ok(())
    }

    /// Check if event matches filter
    fn event_matches_filter(&self, event: &ObservabilityEvent, filter: &EventFilter) -> bool {
        // Filter by event type
        if let Some(ref event_types) = filter.event_types {
            let event_type = match event {
                ObservabilityEvent::MetricsUpdate { .. } => "metrics",
                ObservabilityEvent::ErrorOccurred { .. } => "error",
                ObservabilityEvent::LogEntry { .. } => "log",
                ObservabilityEvent::HealthUpdate { .. } => "health",
                ObservabilityEvent::ToolExecution { .. } => "tool",
                ObservabilityEvent::ContainerEvent { .. } => "container",
                ObservabilityEvent::AgentExecution { .. } => "agent",
                ObservabilityEvent::IntelligenceUpdate { .. } => "intelligence",
                ObservabilityEvent::PatternMatch { .. } => "pattern", 
                ObservabilityEvent::ContextTreeUpdate { .. } => "context",
            };
            
            if !event_types.contains(&event_type.to_string()) {
                return false;
            }
        }

        // Filter by session ID
        if let Some(ref session_filter) = filter.session_id {
            let event_session = match event {
                ObservabilityEvent::ToolExecution { session_id, .. } => Some(session_id),
                ObservabilityEvent::AgentExecution { session_id, .. } => Some(session_id),
                _ => None,
            };
            
            if event_session != Some(session_filter) {
                return false;
            }
        }

        // Filter by error severity
        if let Some(ref min_severity) = filter.severity_min {
            if let ObservabilityEvent::ErrorOccurred { error, .. } = event {
                let severity_level = match error.severity {
                    ErrorSeverity::Low => 0,
                    ErrorSeverity::Medium => 1,
                    ErrorSeverity::High => 2,
                    ErrorSeverity::Critical => 3,
                };
                
                let min_level = match min_severity {
                    ErrorSeverity::Low => 0,
                    ErrorSeverity::Medium => 1,
                    ErrorSeverity::High => 2,
                    ErrorSeverity::Critical => 3,
                };
                
                if severity_level < min_level {
                    return false;
                }
            }
        }

        true
    }

    /// Spawn metrics collection task
    async fn spawn_metrics_collector(&self) -> Result<(), AriaError> {
        let metrics = Arc::clone(&self.metrics);
        let event_broadcaster = self.event_broadcaster.clone();
        let database = Arc::clone(&self.database);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                // Collect system metrics
                let system_metrics = collect_system_metrics().await;
                let database_stats = collect_database_stats(&database).await;
                
                // Update metrics
                let mut current_metrics = metrics.write().await;
                current_metrics.timestamp = current_timestamp();
                current_metrics.system = system_metrics;
                current_metrics.database = database_stats;
                
                let metrics_event = ObservabilityEvent::MetricsUpdate {
                    timestamp: current_metrics.timestamp,
                    metrics: current_metrics.clone(),
                };
                
                // Emit metrics event
                if let Err(e) = event_broadcaster.send(metrics_event) {
                    if event_broadcaster.receiver_count() > 0 {
                        debug!("Failed to send metrics event with active receivers: {}", e);
                    }
                    // Silently ignore if no receivers
                }
            }
        });

        Ok(())
    }

    /// Spawn health monitoring task
    async fn spawn_health_monitor(&self) -> Result<(), AriaError> {
        let health = Arc::clone(&self.health);
        let event_broadcaster = self.event_broadcaster.clone();
        let database = Arc::clone(&self.database);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                // Check component health
                let mut components = HashMap::new();
                
                // Database health
                let db_health = check_database_health(&database).await;
                components.insert("database".to_string(), db_health);
                
                // Determine overall health
                let overall = if components.values().all(|h| matches!(h.status, HealthState::Healthy)) {
                    HealthState::Healthy
                } else if components.values().any(|h| matches!(h.status, HealthState::Unhealthy)) {
                    HealthState::Unhealthy
                } else {
                    HealthState::Degraded
                };
                
                let health_status = HealthStatus {
                    overall,
                    components,
                };
                
                // Update health
                *health.write().await = health_status.clone();
                
                let health_event = ObservabilityEvent::HealthUpdate {
                    timestamp: current_timestamp(),
                    status: health_status,
                };
                
                // Emit health event
                if let Err(e) = event_broadcaster.send(health_event) {
                    if event_broadcaster.receiver_count() > 0 {
                        debug!("Failed to send health event with active receivers: {}", e);
                    }
                    // Silently ignore if no receivers
                }
            }
        });

        Ok(())
    }

    /// Spawn cleanup task for old data
    async fn spawn_cleanup_task(&self) -> Result<(), AriaError> {
        let database = Arc::clone(&self.database);
        let subscribers = Arc::clone(&self.subscribers);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // Every hour
            
            loop {
                interval.tick().await;
                
                // Clean up old audit logs (keep last 30 days)
                let cutoff_time = current_timestamp() - (30 * 24 * 60 * 60);
                
                // TODO: Implement database cleanup
                // database.cleanup_old_logs(cutoff_time).await;
                
                // Clean up inactive subscribers
                let mut subs = subscribers.write().await;
                subs.retain(|_, subscriber| subscriber.sender.receiver_count() > 0);
                
                debug!("Cleanup completed: {} active subscribers", subs.len());
            }
        });

        Ok(())
    }
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Collect system metrics
async fn collect_system_metrics() -> SystemMetrics {
    // Implementation would use system APIs
    // Placeholder values for now
    SystemMetrics {
        cpu_usage_percent: 0.0,
        memory_usage_bytes: 0,
        memory_total_bytes: 0,
        disk_usage_bytes: 0,
        disk_total_bytes: 0,
        network_rx_bytes: 0,
        network_tx_bytes: 0,
        uptime_seconds: 0,
    }
}

/// Collect database statistics
async fn collect_database_stats(database: &DatabaseManager) -> DatabaseStats {
    // Implementation would query database stats
    // Placeholder values for now
    DatabaseStats {
        connections_active: 0,
        connections_total: 0,
        queries_executed: 0,
        queries_failed: 0,
        avg_query_time_ms: 0.0,
        database_size_bytes: 0,
    }
}

/// Check database health
async fn check_database_health(database: &DatabaseManager) -> ComponentHealth {
    // Implementation would actually check database
    // Placeholder for now
    ComponentHealth {
        status: HealthState::Healthy,
        last_check: current_timestamp(),
        message: Some("Database operational".to_string()),
        metrics: HashMap::new(),
    }
} 