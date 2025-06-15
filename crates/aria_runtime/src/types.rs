use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Unique identifier for various runtime entities
pub type EntityId = Uuid;

/// Session identifier for execution context
pub type SessionId = String;

/// Task complexity assessment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskComplexity {
    Simple,
    MultiStep,
    Complex,
}

/// Execution strategy for teams
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TeamStrategy {
    Parallel,
    Sequential,
    Pipeline,
    Collaborative,
    RoleBased,
    Adaptive,
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub timeout: Option<Duration>,
}

/// Execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub total_duration: Duration,
    pub planning_duration: Duration,
    pub execution_duration: Duration,
    pub reflection_duration: Duration,
    pub step_count: u32,
    pub tool_call_count: u32,
    pub llm_call_count: u32,
    pub error_count: u32,
    pub recovery_count: u32,
    pub cache_hit_rate: f32,
    pub success_rate: f32,
    pub start_time: SystemTime,
    pub end_time: Option<SystemTime>,
}

/// Resource token for Quilt integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceToken {
    pub resource_type: String,
    pub resource_id: String,
    pub access_mode: AccessMode,
    pub priority: u8,
    pub ttl: Option<Duration>,
}

/// Access mode for resource tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessMode {
    Read,
    Write,
    ReadWrite,
    Append,
    Exclusive,
}

/// Working memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemoryEntry {
    pub id: EntityId,
    pub key: String,
    pub value: serde_json::Value,
    pub entry_type: MemoryEntryType,
    pub created_at: SystemTime,
    pub last_accessed: SystemTime,
    pub access_count: u32,
    pub ttl: Option<Duration>,
    pub tags: Vec<String>,
}

/// Memory entry type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryEntryType {
    Goal,
    Hypothesis,
    Evidence,
    Constraint,
    ToolPreference,
    Pattern,
    Learning,
}

/// Tool parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
}

/// Agent capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapability {
    pub name: String,
    pub description: String,
    pub confidence: f32,
    pub prerequisites: Vec<String>,
}

/// Task priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

/// Execution step status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    Cancelled,
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "gpt-4o-mini".to_string(),
            api_key: None,
            temperature: Some(0.7),
            max_tokens: Some(2000),
            timeout: Some(Duration::from_secs(30)),
        }
    }
}

impl Default for ExecutionMetrics {
    fn default() -> Self {
        Self {
            total_duration: Duration::ZERO,
            planning_duration: Duration::ZERO,
            execution_duration: Duration::ZERO,
            reflection_duration: Duration::ZERO,
            step_count: 0,
            tool_call_count: 0,
            llm_call_count: 0,
            error_count: 0,
            recovery_count: 0,
            cache_hit_rate: 0.0,
            success_rate: 0.0,
            start_time: SystemTime::now(),
            end_time: None,
        }
    }
} 