use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use deepsize::DeepSizeOf;

use crate::deep_size::*;

/// Unique identifier for various runtime entities
pub type EntityId = DeepUuid;

/// Session identifier for execution context
pub type SessionId = String;

/// Task complexity assessment
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, DeepSizeOf)]
pub enum TaskComplexity {
    Simple,
    MultiStep,
    Complex,
    Enterprise,
    Adaptive,
}

/// Execution strategy for teams
#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub enum TeamStrategy {
    Parallel,
    Sequential,
    Pipeline,
    Collaborative,
    RoleBased,
    Adaptive,
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct LLMConfig {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub timeout: Option<DeepDuration>,
}

/// Execution metrics
#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct ExecutionMetrics {
    pub total_duration: DeepDuration,
    pub planning_duration: DeepDuration,
    pub execution_duration: DeepDuration,
    pub reflection_duration: DeepDuration,
    pub step_count: u32,
    pub tool_call_count: u32,
    pub llm_call_count: u32,
    pub error_count: u32,
    pub recovery_count: u32,
    pub cache_hit_rate: f32,
    pub success_rate: f32,
    pub start_time: DeepSystemTime,
    pub end_time: Option<DeepSystemTime>,
}

/// Resource token for Quilt integration
#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct ResourceToken {
    pub resource_type: String,
    pub resource_id: String,
    pub access_mode: AccessMode,
    pub priority: u8,
    pub ttl: Option<DeepDuration>,
}

/// Access mode for resource tokens
#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub enum AccessMode {
    Read,
    Write,
    ReadWrite,
    Append,
    Exclusive,
}

/// Working memory entry
#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct WorkingMemoryEntry {
    pub id: EntityId,
    pub key: String,
    pub value: DeepValue,
    pub entry_type: MemoryEntryType,
    pub created_at: DeepSystemTime,
    pub last_accessed: DeepSystemTime,
    pub access_count: u32,
    pub ttl: Option<DeepDuration>,
    pub tags: Vec<String>,
}

/// Memory entry type
#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
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
#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct ToolParameter {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
    pub default_value: Option<DeepValue>,
}

/// Agent capability
#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct AgentCapability {
    pub name: String,
    pub description: String,
    pub confidence: f32,
    pub prerequisites: Vec<String>,
}

/// Task priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, DeepSizeOf)]
pub enum Priority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

/// Execution step status
#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    Cancelled,
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, DeepSizeOf)]
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
            timeout: Some(DeepDuration(std::time::Duration::from_secs(30))),
        }
    }
}

impl Default for ExecutionMetrics {
    fn default() -> Self {
        Self {
            total_duration: DeepDuration(std::time::Duration::ZERO),
            planning_duration: DeepDuration(std::time::Duration::ZERO),
            execution_duration: DeepDuration(std::time::Duration::ZERO),
            reflection_duration: DeepDuration(std::time::Duration::ZERO),
            step_count: 0,
            tool_call_count: 0,
            llm_call_count: 0,
            error_count: 0,
            recovery_count: 0,
            cache_hit_rate: 0.0,
            success_rate: 0.0,
            start_time: DeepSystemTime(std::time::SystemTime::now()),
            end_time: None,
        }
    }
}

// ==========================================
// CORE RUNTIME TYPES
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct RuntimeResult {
    pub success: bool,
    pub mode: RuntimeExecutionMode,
    pub conversation: Option<ConversationJSON>,
    pub execution_details: ExecutionDetails,
    pub plan: Option<ExecutionPlan>,
    pub reflections: Vec<Reflection>,
    pub error: Option<String>,
    pub metrics: RuntimeMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub enum RuntimeExecutionMode {
    Legacy,
    LegacyWithConversation,
    EnhancedPlanning,
    AdaptiveReflection,
    ContainerWorkload,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct RuntimeMetrics {
    pub total_duration: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub step_count: u32,
    pub tool_calls: u32,
    pub container_calls: u32,
    pub agent_calls: u32,
    pub token_usage: Option<TokenUsage>,
    pub reflection_count: u32,
    pub adaptation_count: u32,
    pub memory_usage: MemoryUsage,
}

impl Default for RuntimeMetrics {
    fn default() -> Self {
        Self {
            total_duration: 0,
            start_time: 0,
            end_time: 0,
            step_count: 0,
            tool_calls: 0,
            container_calls: 0,
            agent_calls: 0,
            token_usage: None,
            reflection_count: 0,
            adaptation_count: 0,
            memory_usage: MemoryUsage {
                current_size: 0,
                max_size: 0,
                utilization_percent: 0.0,
                item_count: 0,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct TokenUsage {
    pub prompt: u32,
    pub completion: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct MemoryUsage {
    pub current_size: u64,
    pub max_size: u64,
    pub utilization_percent: f64,
    pub item_count: u32,
}

// ==========================================
// EXECUTION CONTEXT & STATE
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeContext {
    pub session_id: DeepUuid,
    pub agent_config: AgentConfig,
    pub created_at: u64,
    pub conversation: Option<ConversationJSON>,
    pub status: ExecutionStatus,
    pub current_plan: Option<ExecutionPlan>,
    pub execution_history: Vec<ExecutionStep>,
    #[serde(skip, default = "default_working_memory")]
    pub working_memory: Arc<RwLock<HashMap<String, DeepValue>>>,
    pub insights: Vec<Insight>,
    pub error_history: Vec<RuntimeError>,
    pub current_step: u32,
    pub total_steps: u32,
    pub remaining_steps: Vec<PlannedStep>,
    pub reflections: Vec<Reflection>,
    pub memory_size: u64,
    pub max_memory_size: u64,
}

fn default_working_memory() -> Arc<RwLock<HashMap<String, DeepValue>>> {
    Arc::new(RwLock::new(HashMap::new()))
}

// Custom DeepSizeOf implementation to handle working_memory field
impl DeepSizeOf for RuntimeContext {
    fn deep_size_of_children(&self, context: &mut deepsize::Context) -> usize {
        self.session_id.deep_size_of_children(context)
            + self.agent_config.deep_size_of_children(context)
            + self.created_at.deep_size_of_children(context)
            + self.conversation.deep_size_of_children(context)
            + self.status.deep_size_of_children(context)
            + self.current_plan.deep_size_of_children(context)
            + self.execution_history.deep_size_of_children(context)
            // Skip working_memory since RwLock<HashMap<String, DeepValue>> doesn't implement DeepSizeOf
            // We calculate this manually in context_manager.rs
            + self.insights.deep_size_of_children(context)
            + self.error_history.deep_size_of_children(context)
            + self.current_step.deep_size_of_children(context)
            + self.total_steps.deep_size_of_children(context)
            + self.remaining_steps.deep_size_of_children(context)
            + self.reflections.deep_size_of_children(context)
            + self.memory_size.deep_size_of_children(context)
            + self.max_memory_size.deep_size_of_children(context)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub enum ExecutionStatus {
    Running,
    Succeeded,
    Failed,
    Aborted,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct RuntimeSnapshot {
    pub session_id: DeepUuid,
    pub current_step: u32,
    pub total_steps: u32,
    pub execution_history: Vec<ExecutionStep>,
    pub insights: Vec<Insight>,
    pub timestamp: u64,
}

// ==========================================
// AGENT CONFIGURATION
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct AgentConfig {
    pub name: String,
    pub system_prompt: Option<String>,
    pub directives: Option<String>,
    pub tools: Vec<String>,
    pub agents: Vec<String>,
    pub llm: LLMConfig,
    pub max_iterations: Option<u32>,
    pub timeout_ms: Option<u64>,
    pub memory_limit: Option<u64>,
    pub agent_type: Option<String>,
    pub capabilities: Vec<String>,
    pub memory_enabled: Option<bool>,
}

// ==========================================
// PLANNING & EXECUTION
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct ExecutionPlan {
    pub id: DeepUuid,
    pub task_description: String,
    pub steps: Vec<PlannedStep>,
    pub confidence: f32,
    pub estimated_duration: Option<u64>,
    pub resource_requirements: ResourceRequirements,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct PlannedStep {
    pub id: DeepUuid,
    pub description: String,
    pub step_type: StepType,
    pub tool_name: Option<String>,
    pub agent_name: Option<String>,
    pub container_spec: Option<ContainerSpec>,
    pub parameters: HashMap<String, DeepValue>,
    pub success_criteria: String,
    pub timeout_ms: Option<u64>,
    pub retry_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub enum StepType {
    ToolCall,
    AgentInvocation,
    ContainerWorkload,
    PipelineExecution,
    ReasoningStep,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct ContainerSpec {
    pub image: String,
    pub command: Vec<String>,
    pub environment: HashMap<String, String>,
    pub working_dir: Option<String>,
    pub resource_limits: ResourceLimits,
    pub network_access: bool,
    pub mount_points: Vec<MountPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct ResourceLimits {
    pub cpu_millis: Option<u32>,
    pub memory_mb: Option<u64>,
    pub disk_mb: Option<u64>,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct MountPoint {
    pub host_path: String,
    pub container_path: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct ResourceRequirements {
    pub cpu_millis: u32,
    pub memory_mb: u64,
    pub disk_mb: u64,
    pub network_bandwidth_kbps: Option<u32>,
    pub container_count: u32,
    // Additional fields for tool registry compatibility
    pub cpu_cores: Option<u32>,
    pub timeout_seconds: Option<u64>,
    pub max_concurrent: Option<u32>,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            cpu_millis: 500,
            memory_mb: 128,
            disk_mb: 50,
            network_bandwidth_kbps: Some(1000),
            container_count: 0,
            cpu_cores: Some(1),
            timeout_seconds: Some(60),
            max_concurrent: Some(1),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct ExecutionStep {
    pub step_id: DeepUuid,
    pub description: String,
    pub start_time: u64,
    pub end_time: u64,
    pub duration: u64,
    pub success: bool,
    pub step_type: StepType,
    pub tool_used: Option<String>,
    pub agent_used: Option<String>,
    pub container_used: Option<String>,
    pub parameters: HashMap<String, DeepValue>,
    pub result: Option<DeepValue>,
    pub error: Option<String>,
    pub reflection: Option<Reflection>,
    pub summary: String,
    pub resource_usage: Option<ResourceUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct ResourceUsage {
    pub cpu_time_ms: u64,
    pub memory_peak_mb: u64,
    pub disk_io_mb: u64,
    pub network_io_kb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct TaskAnalysis {
    pub complexity: TaskComplexity,
    pub requires_planning: bool,
    pub requires_containers: bool,
    pub estimated_steps: u32,
    pub reasoning: String,
}

// ==========================================
// CONVERSATION SYSTEM
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct ConversationJSON {
    pub id: DeepUuid,
    pub original_task: String,
    pub turns: Vec<ConversationTurn>,
    pub final_response: String,
    pub reasoning_chain: Vec<String>,
    pub duration: u64,
    pub state: ConversationState,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct ConversationTurn {
    pub id: DeepUuid,
    pub role: ConversationRole,
    pub content: String,
    pub timestamp: u64,
    pub metadata: Option<ConversationMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub enum ConversationRole {
    User,
    Assistant,
    System,
    Tool,
    Agent,
    Container,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct ConversationMetadata {
    pub step_id: Option<DeepUuid>,
    pub tool_used: Option<String>,
    pub agent_used: Option<String>,
    pub action_type: Option<ActionType>,
    pub confidence: Option<f32>,
    pub reflection: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, DeepSizeOf)]
pub enum ConversationState {
    Initiated,
    Working,
    Reflecting,
    Adapting,
    Concluding,
    Completed,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub enum ActionType {
    Search,
    Analyze,
    Create,
    Transform,
    Communicate,
    Validate,
    Container,
    Other,
}

// ==========================================
// REFLECTION & ADAPTATION
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    pub id: DeepUuid,
    pub step_id: DeepUuid,
    pub assessment: ReflectionAssessment,
    pub suggested_action: SuggestedAction,
    pub reasoning: String,
    pub confidence: f32,
    pub timestamp: u64,
    pub improvements: Vec<String>,
}

impl DeepSizeOf for Reflection {
    fn deep_size_of_children(&self, context: &mut deepsize::Context) -> usize {
        self.id.deep_size_of_children(context)
            + self.step_id.deep_size_of_children(context)
            + self.assessment.deep_size_of_children(context)
            + self.suggested_action.deep_size_of_children(context)
            + self.reasoning.deep_size_of_children(context)
            + self.confidence.deep_size_of_children(context)
            + self.timestamp.deep_size_of_children(context)
            + self.improvements.deep_size_of_children(context)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct ReflectionAssessment {
    pub performance: PerformanceLevel,
    pub quality: QualityLevel,
    pub efficiency: EfficiencyLevel,
    pub suggested_improvements: Vec<String>,
    pub requires_replanning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub enum PerformanceLevel {
    Excellent,
    Good,
    Acceptable,
    Poor,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub enum QualityLevel {
    Optimal,
    Good,
    Suboptimal,
    Wrong,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub enum EfficiencyLevel {
    VeryEfficient,
    Efficient,
    Acceptable,
    Inefficient,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub enum SuggestedAction {
    Continue,
    Retry,
    Abort,
    ModifyPlan,
    UseContainer,
    UseDifferentTool,
    OptimizeParameters,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct ExecutionAdaptation {
    pub adaptation_type: AdaptationType,
    pub specific_changes: Vec<String>,
    pub plan_modifications: Vec<String>,
    pub conversation_adjustments: Vec<String>,
    pub confidence_boost_actions: Vec<String>,
    pub estimated_impact: ImpactLevel,
    pub requires_replanning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub enum AdaptationType {
    Continue,
    ModifyApproach,
    ChangeTools,
    UseContainers,
    ReplanningNeeded,
    Risky,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub enum ImpactLevel {
    Positive,
    Neutral,
    Risky,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub id: DeepUuid,
    pub insight_type: InsightType,
    pub description: String,
    pub confidence: f32,
    pub source: String,
    pub timestamp: u64,
    pub actionable: bool,
    pub metadata: HashMap<String, DeepValue>,
}

impl DeepSizeOf for Insight {
    fn deep_size_of_children(&self, context: &mut deepsize::Context) -> usize {
        self.id.deep_size_of_children(context)
            + self.insight_type.deep_size_of_children(context)
            + self.description.deep_size_of_children(context)
            + self.confidence.deep_size_of_children(context)
            + self.source.deep_size_of_children(context)
            + self.timestamp.deep_size_of_children(context)
            + self.actionable.deep_size_of_children(context)
            + deep_size_of_hashmap_value(&self.metadata, context)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub enum InsightType {
    PatternRecognition,
    PerformanceOptimization,
    ErrorPrevention,
    StrategyImprovement,
    ContextLearning,
    ContainerOptimization,
    ResourceOptimization,
}

// ==========================================
// ERROR HANDLING
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeError {
    pub id: DeepUuid,
    pub error_type: RuntimeErrorType,
    pub message: String,
    pub step_id: Option<DeepUuid>,
    pub tool_name: Option<String>,
    pub agent_name: Option<String>,
    pub container_id: Option<String>,
    pub timestamp: u64,
    pub context: HashMap<String, DeepValue>,
    pub recoverable: bool,
    pub recovery_actions: Vec<String>,
    pub user_guidance: Option<String>,
}

impl DeepSizeOf for RuntimeError {
    fn deep_size_of_children(&self, context: &mut deepsize::Context) -> usize {
        self.id.deep_size_of_children(context)
            + self.error_type.deep_size_of_children(context)
            + self.message.deep_size_of_children(context)
            + self.step_id.deep_size_of_children(context)
            + self.tool_name.deep_size_of_children(context)
            + self.agent_name.deep_size_of_children(context)
            + self.container_id.deep_size_of_children(context)
            + self.timestamp.deep_size_of_children(context)
            + deep_size_of_hashmap_value(&self.context, context)
            + self.recoverable.deep_size_of_children(context)
            + self.recovery_actions.deep_size_of_children(context)
            + self.user_guidance.deep_size_of_children(context)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub enum RuntimeErrorType {
    ToolExecution,
    AgentInvocation,
    ContainerExecution,
    PlanningFailure,
    ConversationError,
    ReflectionError,
    ContextError,
    SystemError,
    NetworkError,
    ResourceExhaustion,
    SecurityViolation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackStrategy {
    pub id: DeepUuid,
    pub description: String,
    pub trigger_condition: String,
    pub actions: Vec<String>,
    pub confidence: f32,
    pub max_retries: u32,
}

impl DeepSizeOf for FallbackStrategy {
    fn deep_size_of_children(&self, context: &mut deepsize::Context) -> usize {
        self.id.deep_size_of_children(context)
            + self.description.deep_size_of_children(context)
            + self.trigger_condition.deep_size_of_children(context)
            + self.actions.deep_size_of_children(context)
            + self.confidence.deep_size_of_children(context)
            + self.max_retries.deep_size_of_children(context)
    }
}

// ==========================================
// EXECUTION DETAILS
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct ExecutionDetails {
    pub mode: RuntimeExecutionMode,
    pub step_results: Vec<ExecutionStep>,
    pub participating_agents: Vec<String>,
    pub containers_used: Vec<String>,
    pub total_steps: u32,
    pub completed_steps: u32,
    pub failed_steps: u32,
    pub skipped_steps: u32,
    pub adaptations: Vec<ExecutionAdaptation>,
    pub insights: Vec<Insight>,
    pub resource_utilization: ResourceUtilization,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct ResourceUtilization {
    pub cpu_utilization_percent: f32,
    pub memory_utilization_percent: f32,
    pub disk_utilization_percent: f32,
    pub network_utilization_percent: f32,
    pub container_efficiency: f32,
}

impl Default for ResourceUtilization {
    fn default() -> Self {
        Self {
            cpu_utilization_percent: 0.0,
            memory_utilization_percent: 0.0,
            disk_utilization_percent: 0.0,
            network_utilization_percent: 0.0,
            container_efficiency: 0.0,
        }
    }
}

// ==========================================
// RUNTIME CONFIGURATION
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub struct RuntimeConfiguration {
    pub enhanced_runtime: bool,
    pub planning_threshold: TaskComplexity,
    pub reflection_enabled: bool,
    pub container_execution_enabled: bool,
    pub max_steps_per_plan: u32,
    pub timeout_ms: u64,
    pub retry_attempts: u32,
    pub debug_mode: bool,
    pub memory_limit_mb: u64,
    pub max_concurrent_containers: u32,
    pub container_timeout_seconds: u64,
    pub enable_icc_callbacks: bool,
    pub llm_providers: Vec<String>,
    pub default_llm_provider: String,
    pub enable_caching: bool,
    pub cache_ttl_seconds: u64,
}

impl Default for RuntimeConfiguration {
    fn default() -> Self {
        Self {
            enhanced_runtime: true,
            planning_threshold: TaskComplexity::MultiStep,
            reflection_enabled: true,
            container_execution_enabled: true,
            max_steps_per_plan: 10,
            timeout_ms: 300_000, // 5 minutes
            retry_attempts: 3,
            debug_mode: false,
            memory_limit_mb: 512,
            max_concurrent_containers: 5,
            container_timeout_seconds: 300, // 5 minutes
            enable_icc_callbacks: true,
            llm_providers: vec!["openai".to_string(), "anthropic".to_string()],
            default_llm_provider: "openai".to_string(),
            enable_caching: true,
            cache_ttl_seconds: 3600, // 1 hour
        }
    }
}

// ==========================================
// STATUS TYPES
// ==========================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, DeepSizeOf)]
pub enum RuntimeStatus {
    Initializing,
    Ready,
    Executing,
    Error,
    Shutdown,
}

// ==========================================
// TOOL & REGISTRY TYPES
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub result: Option<DeepValue>,
    pub error: Option<String>,
    pub metadata: HashMap<String, DeepValue>,
    pub execution_time_ms: u64,
    pub resource_usage: Option<ResourceUsage>,
}

impl Default for ToolResult {
    fn default() -> Self {
        Self {
            success: false,
            result: None,
            error: None,
            metadata: HashMap::new(),
            execution_time_ms: 0,
            resource_usage: None,
        }
    }
}

impl DeepSizeOf for ToolResult {
    fn deep_size_of_children(&self, context: &mut deepsize::Context) -> usize {
        self.success.deep_size_of_children(context)
            + deep_size_of_option_value(&self.result, context)
            + self.error.deep_size_of_children(context)
            + deep_size_of_hashmap_value(&self.metadata, context)
            + self.execution_time_ms.deep_size_of_children(context)
            + self.resource_usage.deep_size_of_children(context)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub entry_type: RegistryEntryType,
    pub bundle_id: Option<String>,
    pub version: String,
    pub metadata: HashMap<String, DeepValue>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl DeepSizeOf for RegistryEntry {
    fn deep_size_of_children(&self, context: &mut deepsize::Context) -> usize {
        self.name.deep_size_of_children(context)
            + self.entry_type.deep_size_of_children(context)
            + self.bundle_id.deep_size_of_children(context)
            + self.version.deep_size_of_children(context)
            + deep_size_of_hashmap_value(&self.metadata, context)
            + self.created_at.deep_size_of_children(context)
            + self.updated_at.deep_size_of_children(context)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, DeepSizeOf)]
pub enum RegistryEntryType {
    Tool,
    Agent,
    Container,
    Pipeline,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: "default-agent".to_string(),
            system_prompt: None,
            directives: None,
            tools: vec![],
            agents: vec![],
            llm: LLMConfig::default(),
            max_iterations: Some(10),
            timeout_ms: Some(300_000),
            memory_limit: Some(512 * 1024 * 1024),
            agent_type: None,
            capabilities: vec![],
            memory_enabled: Some(true),
        }
    }
}

 