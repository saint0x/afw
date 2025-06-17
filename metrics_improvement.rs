// Metrics Improvement Demo for Aria Runtime
// Shows how to complete the metrics collection and add observability endpoints

use aria_runtime::{
    AriaResult, AriaError,
    engines::AriaEngines,
    types::{RuntimeMetrics, TokenUsage, MemoryUsage},
};
use serde_json::json;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// Enhanced metrics collector that tracks everything
#[derive(Debug, Clone)]
pub struct EnhancedMetricsCollector {
    // Current metrics from RuntimeMetrics
    pub runtime_metrics: RuntimeMetrics,
    
    // Additional tracking needed for production
    pub container_operations: HashMap<String, u32>, // operation -> count
    pub agent_invocations: HashMap<String, u32>,    // agent -> count  
    pub error_counts: HashMap<String, u32>,         // error_code -> count
    pub performance_metrics: PerformanceMetrics,
    pub system_health: SystemHealthMetrics,
}

#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub avg_response_time_ms: f64,
    pub p95_response_time_ms: f64,
    pub p99_response_time_ms: f64,
    pub throughput_requests_per_sec: f64,
    pub concurrent_sessions: u32,
}

#[derive(Debug, Clone)]
pub struct SystemHealthMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: u64,
    pub disk_usage_bytes: u64,
    pub network_connections: u32,
    pub database_connections: u32,
    pub uptime_seconds: u64,
}

impl EnhancedMetricsCollector {
    pub fn new() -> Self {
        Self {
            runtime_metrics: RuntimeMetrics::default(),
            container_operations: HashMap::new(),
            agent_invocations: HashMap::new(), 
            error_counts: HashMap::new(),
            performance_metrics: PerformanceMetrics {
                avg_response_time_ms: 0.0,
                p95_response_time_ms: 0.0,
                p99_response_time_ms: 0.0,
                throughput_requests_per_sec: 0.0,
                concurrent_sessions: 0,
            },
            system_health: SystemHealthMetrics {
                cpu_usage_percent: 0.0,
                memory_usage_bytes: 0,
                disk_usage_bytes: 0,
                network_connections: 0,
                database_connections: 0,
                uptime_seconds: 0,
            },
        }
    }

    // Track container operations (implements TODO: Track container calls)
    pub fn record_container_operation(&mut self, operation: &str) {
        *self.container_operations.entry(operation.to_string()).or_insert(0) += 1;
        self.runtime_metrics.container_calls += 1;
        
        tracing::info!("📦 Container operation: {} (total: {})", 
                      operation, self.runtime_metrics.container_calls);
    }

    // Track agent invocations (implements TODO: Track agent calls)
    pub fn record_agent_invocation(&mut self, agent_name: &str) {
        *self.agent_invocations.entry(agent_name.to_string()).or_insert(0) += 1;
        self.runtime_metrics.agent_calls += 1;
        
        tracing::info!("🤖 Agent invocation: {} (total: {})", 
                      agent_name, self.runtime_metrics.agent_calls);
    }

    // Track token usage (implements TODO: Track token usage)
    pub fn record_token_usage(&mut self, prompt_tokens: u32, completion_tokens: u32) {
        let total_tokens = prompt_tokens + completion_tokens;
        
        if let Some(existing) = &mut self.runtime_metrics.token_usage {
            existing.prompt += prompt_tokens;
            existing.completion += completion_tokens;
            existing.total += total_tokens;
        } else {
            self.runtime_metrics.token_usage = Some(TokenUsage {
                prompt: prompt_tokens,
                completion: completion_tokens,
                total: total_tokens,
            });
        }
        
        tracing::info!("🧠 Token usage: +{} prompt, +{} completion (total: {})", 
                      prompt_tokens, completion_tokens, total_tokens);
    }

    // Track errors for error rate monitoring  
    pub fn record_error(&mut self, error: &AriaError) {
        let error_key = format!("{:?}", error.code);
        *self.error_counts.entry(error_key.clone()).or_insert(0) += 1;
        
        tracing::error!("❌ Error recorded: {} - {} (count: {})", 
                       error_key, error.message, 
                       self.error_counts.get(&error_key).unwrap_or(&0));
    }

    // Generate Prometheus-style metrics
    pub fn to_prometheus_format(&self) -> String {
        let mut metrics = Vec::new();
        
        // Runtime metrics
        metrics.push(format!("aria_runtime_duration_total {}", self.runtime_metrics.total_duration));
        metrics.push(format!("aria_runtime_steps_total {}", self.runtime_metrics.step_count));
        metrics.push(format!("aria_runtime_tool_calls_total {}", self.runtime_metrics.tool_calls));
        metrics.push(format!("aria_runtime_container_calls_total {}", self.runtime_metrics.container_calls));
        metrics.push(format!("aria_runtime_agent_calls_total {}", self.runtime_metrics.agent_calls));
        
        // Token usage
        if let Some(tokens) = &self.runtime_metrics.token_usage {
            metrics.push(format!("aria_runtime_tokens_prompt_total {}", tokens.prompt));
            metrics.push(format!("aria_runtime_tokens_completion_total {}", tokens.completion));
            metrics.push(format!("aria_runtime_tokens_total {}", tokens.total));
        }
        
        // Memory usage
        metrics.push(format!("aria_runtime_memory_current_bytes {}", self.runtime_metrics.memory_usage.current_size));
        metrics.push(format!("aria_runtime_memory_max_bytes {}", self.runtime_metrics.memory_usage.max_size));
        metrics.push(format!("aria_runtime_memory_utilization_percent {}", self.runtime_metrics.memory_usage.utilization_percent));
        
        // Container operations breakdown
        for (operation, count) in &self.container_operations {
            metrics.push(format!("aria_container_operations_total{{operation=\"{}\"}} {}", operation, count));
        }
        
        // Agent invocations breakdown
        for (agent, count) in &self.agent_invocations {
            metrics.push(format!("aria_agent_invocations_total{{agent=\"{}\"}} {}", agent, count));
        }
        
        // Error counts
        for (error_code, count) in &self.error_counts {
            metrics.push(format!("aria_errors_total{{code=\"{}\"}} {}", error_code, count));
        }
        
        // System health
        metrics.push(format!("aria_system_cpu_usage_percent {}", self.system_health.cpu_usage_percent));
        metrics.push(format!("aria_system_memory_usage_bytes {}", self.system_health.memory_usage_bytes));
        metrics.push(format!("aria_system_uptime_seconds {}", self.system_health.uptime_seconds));
        
        metrics.join("\n")
    }

    // Generate JSON metrics for API consumption
    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            "runtime": {
                "total_duration": self.runtime_metrics.total_duration,
                "step_count": self.runtime_metrics.step_count,
                "tool_calls": self.runtime_metrics.tool_calls,
                "container_calls": self.runtime_metrics.container_calls,
                "agent_calls": self.runtime_metrics.agent_calls,
                "token_usage": self.runtime_metrics.token_usage,
                "memory_usage": self.runtime_metrics.memory_usage
            },
            "container_operations": self.container_operations,
            "agent_invocations": self.agent_invocations,
            "error_counts": self.error_counts,
            "performance": {
                "avg_response_time_ms": self.performance_metrics.avg_response_time_ms,
                "p95_response_time_ms": self.performance_metrics.p95_response_time_ms,
                "throughput_rps": self.performance_metrics.throughput_requests_per_sec,
                "concurrent_sessions": self.performance_metrics.concurrent_sessions
            },
            "system_health": {
                "cpu_usage_percent": self.system_health.cpu_usage_percent,
                "memory_usage_bytes": self.system_health.memory_usage_bytes,
                "uptime_seconds": self.system_health.uptime_seconds
            }
        })
    }
}

// Demo function showing how to integrate enhanced metrics
#[tokio::main]
async fn main() -> AriaResult<()> {
    // Initialize proper logging (no more println!)
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    tracing::info!("🚀 Enhanced Metrics Demo");
    
    let mut metrics_collector = EnhancedMetricsCollector::new();
    
    // Simulate metrics collection
    tracing::info!("📊 Simulating metrics collection...");
    
    // Record container operations
    metrics_collector.record_container_operation("createContainer");
    metrics_collector.record_container_operation("startContainer");
    metrics_collector.record_container_operation("execInContainer");
    metrics_collector.record_container_operation("stopContainer");
    
    // Record agent invocations
    metrics_collector.record_agent_invocation("planning_agent");
    metrics_collector.record_agent_invocation("execution_agent");
    metrics_collector.record_agent_invocation("reflection_agent");
    
    // Record token usage
    metrics_collector.record_token_usage(150, 75); // prompt, completion
    metrics_collector.record_token_usage(200, 100);
    
    // Record some errors
    let sample_error = AriaError::new(
        aria_runtime::errors::ErrorCode::ContainerError,
        aria_runtime::errors::ErrorCategory::Container,
        aria_runtime::errors::ErrorSeverity::Medium,
        "Sample container error for metrics"
    );
    metrics_collector.record_error(&sample_error);
    
    // Display metrics in different formats
    tracing::info!("📈 Metrics collected! Displaying in multiple formats...");
    
    // JSON format for APIs
    let json_metrics = metrics_collector.to_json();
    tracing::info!("JSON Metrics: {}", serde_json::to_string_pretty(&json_metrics).unwrap());
    
    // Prometheus format for monitoring systems
    let prometheus_metrics = metrics_collector.to_prometheus_format();
    tracing::info!("Prometheus Metrics:\n{}", prometheus_metrics);
    
    tracing::info!("✅ Enhanced metrics collection demonstration complete!");
    tracing::info!("🔄 This shows how to implement the TODO items and add production observability");
    
    Ok(())
} 