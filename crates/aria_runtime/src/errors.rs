use thiserror::Error;

/// Main result type for Aria Runtime operations
pub type AriaResult<T> = Result<T, AriaError>;

/// Comprehensive error type for Aria Runtime
#[derive(Error, Debug)]
pub enum AriaError {
    #[error("Planning error: {message}")]
    Planning { message: String, context: Option<String> },
    
    #[error("Execution error: {message}")]
    Execution { message: String, step: Option<String> },
    
    #[error("Tool error: {tool_name} - {message}")]
    Tool { tool_name: String, message: String, details: Option<String> },
    
    #[error("Agent error: {agent_name} - {message}")]
    Agent { agent_name: String, message: String, details: Option<String> },
    
    #[error("Team error: {team_name} - {message}")]
    Team { team_name: String, message: String, details: Option<String> },
    
    #[error("LLM provider error: {provider} - {message}")]
    LLM { provider: String, message: String, error_code: Option<String> },
    
    #[error("Memory system error: {message}")]
    Memory { message: String, operation: Option<String> },
    
    #[error("Cache error: {message}")]
    Cache { message: String, key: Option<String> },
    
    #[error("Quilt integration error: {message}")]
    Quilt { message: String, token: Option<String> },
    
    #[error("Configuration error: {message}")]
    Configuration { message: String, field: Option<String> },
    
    #[error("Validation error: {message}")]
    Validation { message: String, field: Option<String> },
    
    #[error("Serialization error: {message}")]
    Serialization { message: String },
    
    #[error("IO error: {message}")]
    Io { message: String, path: Option<String> },
    
    #[error("Network error: {message}")]
    Network { message: String, endpoint: Option<String> },
    
    #[error("Timeout error: {message}")]
    Timeout { message: String, duration: Option<std::time::Duration> },
    
    #[error("Internal error: {message}")]
    Internal { message: String, cause: Option<String> },
}

impl AriaError {
    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            AriaError::Network { .. } => true,
            AriaError::Timeout { .. } => true,
            AriaError::LLM { .. } => true,
            AriaError::Tool { .. } => true,
            AriaError::Quilt { .. } => true,
            AriaError::Cache { .. } => true,
            AriaError::Memory { .. } => true,
            AriaError::Io { .. } => false,
            AriaError::Configuration { .. } => false,
            AriaError::Validation { .. } => false,
            AriaError::Serialization { .. } => false,
            AriaError::Planning { .. } => true,
            AriaError::Execution { .. } => true,
            AriaError::Agent { .. } => true,
            AriaError::Team { .. } => true,
            AriaError::Internal { .. } => false,
        }
    }
    
    /// Get error category
    pub fn category(&self) -> ErrorCategory {
        match self {
            AriaError::Planning { .. } => ErrorCategory::Planning,
            AriaError::Execution { .. } => ErrorCategory::Execution,
            AriaError::Tool { .. } => ErrorCategory::Tool,
            AriaError::Agent { .. } => ErrorCategory::Agent,
            AriaError::Team { .. } => ErrorCategory::Team,
            AriaError::LLM { .. } => ErrorCategory::LLM,
            AriaError::Memory { .. } => ErrorCategory::Memory,
            AriaError::Cache { .. } => ErrorCategory::Cache,
            AriaError::Quilt { .. } => ErrorCategory::Integration,
            AriaError::Configuration { .. } => ErrorCategory::Configuration,
            AriaError::Validation { .. } => ErrorCategory::Validation,
            AriaError::Serialization { .. } => ErrorCategory::Serialization,
            AriaError::Io { .. } => ErrorCategory::IO,
            AriaError::Network { .. } => ErrorCategory::Network,
            AriaError::Timeout { .. } => ErrorCategory::Timeout,
            AriaError::Internal { .. } => ErrorCategory::Internal,
        }
    }
    
    /// Get error severity
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            AriaError::Configuration { .. } => ErrorSeverity::High,
            AriaError::Validation { .. } => ErrorSeverity::High,
            AriaError::Internal { .. } => ErrorSeverity::Critical,
            AriaError::Planning { .. } => ErrorSeverity::Medium,
            AriaError::Execution { .. } => ErrorSeverity::Medium,
            _ => ErrorSeverity::Low,
        }
    }
}

/// Error category classification
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorCategory {
    Planning,
    Execution,
    Tool,
    Agent,
    Team,
    LLM,
    Memory,
    Cache,
    Integration,
    Configuration,
    Validation,
    Serialization,
    IO,
    Network,
    Timeout,
    Internal,
}

/// Error severity levels
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

// Conversion implementations for common error types
impl From<serde_json::Error> for AriaError {
    fn from(err: serde_json::Error) -> Self {
        AriaError::Serialization {
            message: err.to_string(),
        }
    }
}

impl From<std::io::Error> for AriaError {
    fn from(err: std::io::Error) -> Self {
        AriaError::Io {
            message: err.to_string(),
            path: None,
        }
    }
}

impl From<reqwest::Error> for AriaError {
    fn from(err: reqwest::Error) -> Self {
        AriaError::Network {
            message: err.to_string(),
            endpoint: err.url().map(|u| u.to_string()),
        }
    }
}

impl From<tokio::time::error::Elapsed> for AriaError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        AriaError::Timeout {
            message: err.to_string(),
            duration: None,
        }
    }
} 