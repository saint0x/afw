/*!
# Telemetry

Custom metrics and observability for the Aria Firmware.
Provides performance monitoring and health checks.
*/

use aria_runtime::AriaResult;
use std::collections::HashMap;

pub struct TelemetrySystem {
    metrics: HashMap<String, f64>,
}

impl TelemetrySystem {
    pub fn new() -> Self {
        Self {
            metrics: HashMap::new(),
        }
    }

    pub fn record_counter(&mut self, name: &str, value: f64) {
        *self.metrics.entry(name.to_string()).or_insert(0.0) += value;
    }

    pub fn record_gauge(&mut self, name: &str, value: f64) {
        self.metrics.insert(name.to_string(), value);
    }

    pub fn get_metrics(&self) -> &HashMap<String, f64> {
        &self.metrics
    }

    pub async fn health_check(&self) -> AriaResult<serde_json::Value> {
        Ok(serde_json::json!({
            "status": "healthy",
            "metrics_count": self.metrics.len()
        }))
    }
} 