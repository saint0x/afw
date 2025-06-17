/// Core intelligence data structures for Aria Runtime
/// Following CTXPLAN.md Phase 1.2 specifications

use crate::errors::{AriaError, AriaResult};
use crate::types::ContainerSpec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Container execution pattern learned from execution history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerPattern {
    pub pattern_id: String,
    pub trigger: String,                    // "build rust project"
    pub container_config: ContainerConfig,  // Optimal configuration
    pub confidence: f64,                    // 0.0 - 1.0
    pub usage_stats: PatternUsageStats,
    pub variables: Vec<PatternVariable>,    // Extracted parameters
    pub created_at: u64,
    pub updated_at: u64,
}

/// Container configuration for pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    pub image: String,
    pub command: Vec<String>,
    pub environment: HashMap<String, String>,
    pub working_directory: Option<String>,
    pub resource_limits: Option<ResourceLimits>,
    pub network_config: Option<NetworkConfig>,
    pub volumes: Vec<VolumeMount>,
}

impl From<ContainerSpec> for ContainerConfig {
    fn from(spec: ContainerSpec) -> Self {
        Self {
            image: spec.image,
            command: spec.command,
            environment: spec.environment,
            working_directory: spec.working_dir,
            resource_limits: Some(spec.resource_limits.into()),
            network_config: None, // Will be enhanced later
            volumes: spec.mount_points.into_iter().map(Into::into).collect(),
        }
    }
}

/// Resource limits for containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub memory_mb: Option<u64>,
    pub cpu_cores: Option<f64>,
    pub disk_mb: Option<u64>,
}

impl From<crate::types::ResourceLimits> for ResourceLimits {
    fn from(limits: crate::types::ResourceLimits) -> Self {
        Self {
            memory_mb: limits.memory_mb,
            cpu_cores: limits.cpu_millis.map(|millis| millis as f64 / 1000.0), // Convert millis to cores
            disk_mb: limits.disk_mb,
        }
    }
}

/// Network configuration for containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub network_mode: String,
    pub port_mappings: Vec<PortMapping>,
}

/// Port mapping for network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub host_port: u16,
    pub container_port: u16,
    pub protocol: String, // "tcp" or "udp"
}

/// Volume mount configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    pub host_path: String,
    pub container_path: String,
    pub read_only: bool,
}

impl From<crate::types::MountPoint> for VolumeMount {
    fn from(mount: crate::types::MountPoint) -> Self {
        Self {
            host_path: mount.host_path,
            container_path: mount.container_path,
            read_only: mount.read_only,
        }
    }
}

/// Usage statistics for a pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternUsageStats {
    pub success_count: u32,
    pub failure_count: u32,
    pub avg_execution_time: Duration,
    pub last_used: Option<SystemTime>,
    pub total_executions: u32,
}

impl Default for PatternUsageStats {
    fn default() -> Self {
        Self {
            success_count: 0,
            failure_count: 0,
            avg_execution_time: Duration::from_secs(0),
            last_used: None,
            total_executions: 0,
        }
    }
}

/// Variable extracted from pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternVariable {
    pub name: String,
    pub pattern: String,      // Regex pattern for extraction
    pub variable_type: VariableType,
    pub default_value: Option<String>,
}

/// Types of variables that can be extracted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VariableType {
    String,
    Integer,
    Float,
    Boolean,
    Path,
    Command,
}

/// Execution context representing hierarchical execution relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub context_id: String,
    pub session_id: String,
    pub context_type: ContextType,
    pub parent_id: Option<String>,
    pub context_data: serde_json::Value,
    pub priority: u8,
    pub children: Vec<ExecutionContext>,
    pub metadata: ContextMetadata,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Types of execution contexts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextType {
    Session,
    Workflow,
    Container,
    Tool,
    Agent,
    Environment,
}

/// Metadata for execution contexts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMetadata {
    pub execution_count: u32,
    pub success_rate: f64,
    pub avg_duration: Duration,
    pub last_execution: Option<SystemTime>,
    pub error_patterns: Vec<String>,
}

impl Default for ContextMetadata {
    fn default() -> Self {
        Self {
            execution_count: 0,
            success_rate: 0.0,
            avg_duration: Duration::from_secs(0),
            last_execution: None,
            error_patterns: Vec::new(),
        }
    }
}

/// Learning feedback for pattern improvement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningFeedback {
    pub feedback_id: String,
    pub pattern_id: String,
    pub execution_id: String,
    pub success: bool,
    pub execution_time: Duration,
    pub feedback_type: FeedbackType,
    pub confidence_delta: f64,
    pub metadata: Option<serde_json::Value>,
    pub created_at: u64,
}

/// Types of learning feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackType {
    Execution,
    User,
    System,
}

/// Result of pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMatch {
    pub pattern: ContainerPattern,
    pub confidence: f64,
    pub extracted_variables: HashMap<String, String>,
    pub container_config: ContainerConfig,
}

/// Container execution result for learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerExecutionResult {
    pub execution_id: String,
    pub container_id: String,
    pub success: bool,
    pub execution_time: Duration,
    pub exit_code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub resource_usage: Option<ResourceUsage>,
    pub pattern_id: Option<String>,
    pub confidence_delta: f64,
    pub metadata: HashMap<String, serde_json::Value>,
    pub workload: ContainerWorkload,
}

/// Resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_time_ms: u64,
    pub max_memory_bytes: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
}

/// Container workload description
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerWorkload {
    pub workload_id: String,
    pub workload_type: WorkloadType,
    pub request_description: String,
    pub session_id: String,
    pub container_spec: ContainerSpec,
}

/// Types of container workloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkloadType {
    Build,
    Test,
    Exec,
    Analysis,
    Deploy,
    Custom(String),
}

/// Container request for intelligence analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerRequest {
    pub request_id: String,
    pub session_id: String,
    pub description: String,
    pub requirements: Option<ContainerRequirements>,
    pub context_hints: Vec<String>,
}

/// Requirements for container execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerRequirements {
    pub min_memory_mb: Option<u64>,
    pub min_cpu_cores: Option<f64>,
    pub required_tools: Vec<String>,
    pub environment_vars: HashMap<String, String>,
    pub timeout_seconds: Option<u64>,
}

/// Intelligence analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceResult {
    pub request_id: String,
    pub session_id: String,
    pub pattern_match: Option<PatternMatch>,
    pub context_summary: String,
    pub recommendation: IntelligenceRecommendation,
    pub execution_time: Duration,
    pub timestamp: SystemTime,
}

/// Intelligence recommendation for container execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceRecommendation {
    pub action: RecommendationAction,
    pub confidence: f64,
    pub reasoning: String,
    pub suggested_config: Option<ContainerConfig>,
    pub alternatives: Vec<ContainerConfig>,
    pub warnings: Vec<String>,
}

/// Recommended actions for container execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationAction {
    UsePattern,
    CreateNew,
    OptimizeExisting,
    RequestMoreInfo,
}

/// Learning configuration for the intelligence system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    pub min_pattern_confidence: f64,
    pub max_pattern_age_days: u32,
    pub learning_rate: f64,
    pub pruning_threshold: f64,
    pub min_executions_for_confidence: u32,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            min_pattern_confidence: 0.3,
            max_pattern_age_days: 30,
            learning_rate: 0.05,
            pruning_threshold: 0.2,
            min_executions_for_confidence: 5,
        }
    }
}

/// Analytics data for learning performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningAnalytics {
    pub total_patterns: usize,
    pub high_confidence_patterns: usize,
    pub recent_executions: u32,
    pub success_rate: f64,
    pub avg_confidence_improvement: f64,
    pub patterns_pruned: u32,
    pub learning_events_processed: u64,
}

/// Intelligence tools available to agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceTool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Tool result for intelligence operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceToolResult {
    pub success: bool,
    pub result: serde_json::Value,
    pub error: Option<String>,
    pub execution_time: Duration,
} 