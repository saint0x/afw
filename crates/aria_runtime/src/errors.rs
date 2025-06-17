use serde::{Deserialize, Serialize};
use std::fmt;

/// Main result type for Aria Runtime operations
pub type AriaResult<T> = Result<T, AriaError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorCode {
    // General Errors
    Unknown,
    NotSupported,
    SystemNotReady,
    Timeout,

    // Config & Init
    SystemInitializationFailure,
    InitializationFailed,
    ConfigError,

    // Execution Errors
    ExecutionError,
    ExecutionCancelled,
    StepExecutionError,
    ParameterResolutionError,

    // Tool Errors
    ToolNotFound,
    ToolInvalidParameters,
    ToolExecutionError,

    // LLM Errors
    LLMError,
    LLMProviderError,
    LLMProviderNotFound,
    LLMApiError,
    LLMTimeout,
    LLMInvalidResponse,
    LLMInvalidRequest,
    LLMAuthentication,
    LLMTokenLimitExceeded,

    // Context & Memory
    ContextError,
    ContextInitializationFailed,

    // Planning Errors
    PlanningFailure,

    // Reflection Errors
    ReflectionError,

    // Serialization Errors
    SerializationError,
    DeserializationError,

    // Container Errors
    ContainerError,
    ContainerOperationFailed,

    // Bundle Errors
    BundleError,

    // Network Errors
    NetworkError,
    UpstreamServiceError,

    // Security Errors
    AuthenticationFailed,
    PermissionDenied,
    
    // Tool Execution Errors
    ToolExecutionFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorCategory {
    System,
    Configuration,
    Execution,
    Tool,
    LLM,
    Context,
    Container,
    Bundle,
    Network,
    Planning,
    Reflection,
    Security,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub struct AriaError {
    pub code: ErrorCode,
    pub category: ErrorCategory,
    pub severity: ErrorSeverity,
    pub message: String,
}

impl AriaError {
    pub fn new(
        code: ErrorCode,
        category: ErrorCategory,
        severity: ErrorSeverity,
        message: &str,
    ) -> Self {
        Self {
            code,
            category,
            severity,
            message: message.to_string(),
        }
    }
}

impl fmt::Display for AriaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?}/{:?}] {}", self.category, self.code, self.message)
    }
}

impl std::error::Error for AriaError {}

impl AriaError {
    pub fn is_recoverable(&self) -> bool {
        match self.severity {
            ErrorSeverity::Low | ErrorSeverity::Medium => true,
            ErrorSeverity::High => {
                matches!(
                    self.code,
                    ErrorCode::Timeout
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

    pub fn is_retriable(&self) -> bool {
        matches!(
            self.code,
            ErrorCode::LLMError
                | ErrorCode::NetworkError
                | ErrorCode::UpstreamServiceError
                | ErrorCode::ToolExecutionError
                | ErrorCode::ContainerError
                | ErrorCode::LLMTokenLimitExceeded
        )
    }

    /// Determines if the error is a timeout.
    pub fn is_timeout(&self) -> bool {
        matches!(self.code, ErrorCode::Timeout)
    }
} 