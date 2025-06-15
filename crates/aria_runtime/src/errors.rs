use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Main result type for Aria Runtime operations
pub type AriaResult<T> = Result<T, AriaError>;

/// Comprehensive error type for Aria Runtime
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub struct AriaError {
    pub code: ErrorCode,
    pub category: ErrorCategory,
    pub severity: ErrorSeverity, 
    pub message: String,
    pub details: HashMap<String, serde_json::Value>,
    pub context: ErrorContext,
    pub user_guidance: Option<String>,
    pub recovery_actions: Vec<String>,
    pub timestamp: u64,
    pub component: String,
    pub operation: String,
    pub session_id: Option<Uuid>,
    pub step_id: Option<Uuid>,
    pub container_id: Option<String>,
    pub tool_name: Option<String>,
    pub agent_name: Option<String>,
}

impl fmt::Display for AriaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}:{}] {} - {} ({})", 
            self.component, 
            self.operation,
            self.code,
            self.message,
            self.category
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorCode {
    // Tool Execution Errors
    ToolNotFound,
    ToolExecutionFailed,
    ToolParameterValidationFailed,
    ToolTimeout,
    ToolResourceExhaustion,
    
    // Agent Errors
    AgentNotFound,
    AgentInvocationFailed,
    AgentConfigurationInvalid,
    AgentTimeout,
    
    // Container Errors
    ContainerCreationFailed,
    ContainerExecutionFailed,
    ContainerNotFound,
    ContainerTimeout,
    ContainerResourceLimitExceeded,
    ContainerSecurityViolation,
    ContainerNetworkError,
    ContainerMountError,
    
    // Planning Errors
    PlanningFailed,
    PlanValidationFailed,
    PlanExecutionFailed,
    TaskAnalysisFailed,
    PlanAdaptationFailed,
    
    // Conversation Errors
    ConversationInitFailed,
    ConversationStateMachineError,
    ConversationSerializationFailed,
    
    // Reflection Errors
    ReflectionFailed,
    ReflectionAnalysisFailed,
    ReflectionActionFailed,
    
    // Context Errors
    ContextInitializationFailed,
    ContextMemoryExhausted,
    ContextSerializationFailed,
    ContextDeserializationFailed,
    ContextCorrupted,
    
    // LLM Errors
    LLMProviderNotFound,
    LLMApiError,
    LLMRateLimitExceeded,
    LLMInvalidResponse,
    LLMTokenLimitExceeded,
    LLMTimeout,
    
    // Registry Errors
    RegistryNotFound,
    RegistryCorrupted,
    RegistryEntryNotFound,
    RegistryDuplicateEntry,
    RegistryPermissionDenied,
    
    // Bundle Errors
    BundleNotFound,
    BundleCorrupted,
    BundleValidationFailed,
    BundleDependencyNotFound,
    BundleLoadingFailed,
    
    // System Errors
    SystemError,
    NetworkError,
    FileSystemError,
    PermissionDenied,
    ResourceExhaustion,
    ConfigurationError,
    InitializationFailed,
    
    // Execution Errors
    ExecutionFailed,
    ExecutionTimeout,
    ExecutionAborted,
    ParameterResolutionFailed,
    StepExecutionFailed,
    
    // Security Errors
    AuthenticationFailed,
    AuthorizationFailed,
    SecurityViolation,
    InputValidationFailed,
    InvalidParameters,
    
    // ICC Errors
    ICCServerStartFailed,
    ICCConnectionFailed,
    ICCAuthenticationFailed,
    ICCProtocolError,
    
    // Unknown/Generic
    Unknown,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCode::ToolNotFound => write!(f, "TOOL_NOT_FOUND"),
            ErrorCode::ToolExecutionFailed => write!(f, "TOOL_EXECUTION_FAILED"),
            ErrorCode::ToolParameterValidationFailed => write!(f, "TOOL_PARAMETER_VALIDATION_FAILED"),
            ErrorCode::ToolTimeout => write!(f, "TOOL_TIMEOUT"),
            ErrorCode::ToolResourceExhaustion => write!(f, "TOOL_RESOURCE_EXHAUSTION"),
            
            ErrorCode::AgentNotFound => write!(f, "AGENT_NOT_FOUND"),
            ErrorCode::AgentInvocationFailed => write!(f, "AGENT_INVOCATION_FAILED"),
            ErrorCode::AgentConfigurationInvalid => write!(f, "AGENT_CONFIGURATION_INVALID"),
            ErrorCode::AgentTimeout => write!(f, "AGENT_TIMEOUT"),
            
            ErrorCode::ContainerCreationFailed => write!(f, "CONTAINER_CREATION_FAILED"),
            ErrorCode::ContainerExecutionFailed => write!(f, "CONTAINER_EXECUTION_FAILED"),
            ErrorCode::ContainerNotFound => write!(f, "CONTAINER_NOT_FOUND"),
            ErrorCode::ContainerTimeout => write!(f, "CONTAINER_TIMEOUT"),
            ErrorCode::ContainerResourceLimitExceeded => write!(f, "CONTAINER_RESOURCE_LIMIT_EXCEEDED"),
            ErrorCode::ContainerSecurityViolation => write!(f, "CONTAINER_SECURITY_VIOLATION"),
            ErrorCode::ContainerNetworkError => write!(f, "CONTAINER_NETWORK_ERROR"),
            ErrorCode::ContainerMountError => write!(f, "CONTAINER_MOUNT_ERROR"),
            
            ErrorCode::PlanningFailed => write!(f, "PLANNING_FAILED"),
            ErrorCode::PlanValidationFailed => write!(f, "PLAN_VALIDATION_FAILED"),
            ErrorCode::PlanExecutionFailed => write!(f, "PLAN_EXECUTION_FAILED"),
            ErrorCode::TaskAnalysisFailed => write!(f, "TASK_ANALYSIS_FAILED"),
            ErrorCode::PlanAdaptationFailed => write!(f, "PLAN_ADAPTATION_FAILED"),
            
            ErrorCode::ConversationInitFailed => write!(f, "CONVERSATION_INIT_FAILED"),
            ErrorCode::ConversationStateMachineError => write!(f, "CONVERSATION_STATE_MACHINE_ERROR"),
            ErrorCode::ConversationSerializationFailed => write!(f, "CONVERSATION_SERIALIZATION_FAILED"),
            
            ErrorCode::ReflectionFailed => write!(f, "REFLECTION_FAILED"),
            ErrorCode::ReflectionAnalysisFailed => write!(f, "REFLECTION_ANALYSIS_FAILED"),
            ErrorCode::ReflectionActionFailed => write!(f, "REFLECTION_ACTION_FAILED"),
            
            ErrorCode::ContextInitializationFailed => write!(f, "CONTEXT_INITIALIZATION_FAILED"),
            ErrorCode::ContextMemoryExhausted => write!(f, "CONTEXT_MEMORY_EXHAUSTED"),
            ErrorCode::ContextSerializationFailed => write!(f, "CONTEXT_SERIALIZATION_FAILED"),
            ErrorCode::ContextDeserializationFailed => write!(f, "CONTEXT_DESERIALIZATION_FAILED"),
            ErrorCode::ContextCorrupted => write!(f, "CONTEXT_CORRUPTED"),
            
            ErrorCode::LLMProviderNotFound => write!(f, "LLM_PROVIDER_NOT_FOUND"),
            ErrorCode::LLMApiError => write!(f, "LLM_API_ERROR"),
            ErrorCode::LLMRateLimitExceeded => write!(f, "LLM_RATE_LIMIT_EXCEEDED"),
            ErrorCode::LLMInvalidResponse => write!(f, "LLM_INVALID_RESPONSE"),
            ErrorCode::LLMTokenLimitExceeded => write!(f, "LLM_TOKEN_LIMIT_EXCEEDED"),
            ErrorCode::LLMTimeout => write!(f, "LLM_TIMEOUT"),
            
            ErrorCode::RegistryNotFound => write!(f, "REGISTRY_NOT_FOUND"),
            ErrorCode::RegistryCorrupted => write!(f, "REGISTRY_CORRUPTED"),
            ErrorCode::RegistryEntryNotFound => write!(f, "REGISTRY_ENTRY_NOT_FOUND"),
            ErrorCode::RegistryDuplicateEntry => write!(f, "REGISTRY_DUPLICATE_ENTRY"),
            ErrorCode::RegistryPermissionDenied => write!(f, "REGISTRY_PERMISSION_DENIED"),
            
            ErrorCode::BundleNotFound => write!(f, "BUNDLE_NOT_FOUND"),
            ErrorCode::BundleCorrupted => write!(f, "BUNDLE_CORRUPTED"),
            ErrorCode::BundleValidationFailed => write!(f, "BUNDLE_VALIDATION_FAILED"),
            ErrorCode::BundleDependencyNotFound => write!(f, "BUNDLE_DEPENDENCY_NOT_FOUND"),
            ErrorCode::BundleLoadingFailed => write!(f, "BUNDLE_LOADING_FAILED"),
            
            ErrorCode::SystemError => write!(f, "SYSTEM_ERROR"),
            ErrorCode::NetworkError => write!(f, "NETWORK_ERROR"),
            ErrorCode::FileSystemError => write!(f, "FILE_SYSTEM_ERROR"),
            ErrorCode::PermissionDenied => write!(f, "PERMISSION_DENIED"),
            ErrorCode::ResourceExhaustion => write!(f, "RESOURCE_EXHAUSTION"),
            ErrorCode::ConfigurationError => write!(f, "CONFIGURATION_ERROR"),
            ErrorCode::InitializationFailed => write!(f, "INITIALIZATION_FAILED"),
            
            ErrorCode::ExecutionFailed => write!(f, "EXECUTION_FAILED"),
            ErrorCode::ExecutionTimeout => write!(f, "EXECUTION_TIMEOUT"),
            ErrorCode::ExecutionAborted => write!(f, "EXECUTION_ABORTED"),
            ErrorCode::ParameterResolutionFailed => write!(f, "PARAMETER_RESOLUTION_FAILED"),
            ErrorCode::StepExecutionFailed => write!(f, "STEP_EXECUTION_FAILED"),
            
            ErrorCode::AuthenticationFailed => write!(f, "AUTHENTICATION_FAILED"),
            ErrorCode::AuthorizationFailed => write!(f, "AUTHORIZATION_FAILED"),
            ErrorCode::SecurityViolation => write!(f, "SECURITY_VIOLATION"),
            ErrorCode::InputValidationFailed => write!(f, "INPUT_VALIDATION_FAILED"),
            ErrorCode::InvalidParameters => write!(f, "INVALID_PARAMETERS"),
            
            ErrorCode::ICCServerStartFailed => write!(f, "ICC_SERVER_START_FAILED"),
            ErrorCode::ICCConnectionFailed => write!(f, "ICC_CONNECTION_FAILED"),
            ErrorCode::ICCAuthenticationFailed => write!(f, "ICC_AUTHENTICATION_FAILED"),
            ErrorCode::ICCProtocolError => write!(f, "ICC_PROTOCOL_ERROR"),
            
            ErrorCode::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorCategory {
    Tool,
    Agent,
    Container,
    Planning,
    Conversation,
    Reflection,
    Context,
    LLM,
    Registry,
    Bundle,
    System,
    Execution,
    Security,
    ICC,
    Configuration,
    Network,
    Resource,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCategory::Tool => write!(f, "Tool"),
            ErrorCategory::Agent => write!(f, "Agent"),
            ErrorCategory::Container => write!(f, "Container"),
            ErrorCategory::Planning => write!(f, "Planning"),
            ErrorCategory::Conversation => write!(f, "Conversation"),
            ErrorCategory::Reflection => write!(f, "Reflection"),
            ErrorCategory::Context => write!(f, "Context"),
            ErrorCategory::LLM => write!(f, "LLM"),
            ErrorCategory::Registry => write!(f, "Registry"),
            ErrorCategory::Bundle => write!(f, "Bundle"),
            ErrorCategory::System => write!(f, "System"),
            ErrorCategory::Execution => write!(f, "Execution"),
            ErrorCategory::Security => write!(f, "Security"),
            ErrorCategory::ICC => write!(f, "ICC"),
            ErrorCategory::Configuration => write!(f, "Configuration"),
            ErrorCategory::Network => write!(f, "Network"),
            ErrorCategory::Resource => write!(f, "Resource"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum ErrorSeverity {
    Low,      // Warning level, recoverable
    Medium,   // Error level, may need intervention
    High,     // Critical error, operation failed
    Critical, // System-level error, may need restart
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Low => write!(f, "LOW"),
            ErrorSeverity::Medium => write!(f, "MEDIUM"),
            ErrorSeverity::High => write!(f, "HIGH"),
            ErrorSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub component: String,
    pub operation: String,
    pub session_id: Option<Uuid>,
    pub request_id: Option<Uuid>,
    pub trace_id: Option<String>,
    pub additional_context: HashMap<String, serde_json::Value>,
}

impl AriaError {
    pub fn new(
        code: ErrorCode,
        category: ErrorCategory,
        severity: ErrorSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            category,
            severity,
            message: message.into(),
            details: HashMap::new(),
            context: ErrorContext {
                component: "unknown".to_string(),
                operation: "unknown".to_string(),
                session_id: None,
                request_id: None,
                trace_id: None,
                additional_context: HashMap::new(),
            },
            user_guidance: None,
            recovery_actions: Vec::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            component: "unknown".to_string(),
            operation: "unknown".to_string(),
            session_id: None,
            step_id: None,
            container_id: None,
            tool_name: None,
            agent_name: None,
        }
    }

    pub fn with_component(mut self, component: impl Into<String>) -> Self {
        self.component = component.into();
        self.context.component = self.component.clone();
        self
    }

    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = operation.into();
        self.context.operation = self.operation.clone();
        self
    }

    pub fn with_session_id(mut self, session_id: Uuid) -> Self {
        self.session_id = Some(session_id);
        self.context.session_id = Some(session_id);
        self
    }

    pub fn is_recoverable(&self) -> bool {
        match self.severity {
            ErrorSeverity::Low | ErrorSeverity::Medium => true,
            ErrorSeverity::High => {
                matches!(
                    self.code,
                    ErrorCode::ToolTimeout
                        | ErrorCode::AgentTimeout
                        | ErrorCode::ContainerTimeout
                        | ErrorCode::LLMRateLimitExceeded
                        | ErrorCode::NetworkError
                )
            }
            ErrorSeverity::Critical => false,
        }
    }
    
    pub fn category(&self) -> &ErrorCategory {
        &self.category
    }
    
    pub fn severity(&self) -> &ErrorSeverity {
        &self.severity
    }
} 