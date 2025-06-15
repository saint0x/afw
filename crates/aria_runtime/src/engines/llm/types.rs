use serde::{Deserialize, Serialize};

/// LLM message for conversation
#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct LLMMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Tool call definition
#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Function call definition (for compatibility)
#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Tool choice options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolChoice {
    None,
    Auto,
    Required,
    Function { name: String },
}

/// LLM configuration (removed Hash due to f32 fields)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LLMConfig {
    pub model: Option<String>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            model: None,
            temperature: 0.7,
            max_tokens: 2048,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
        }
    }
}

/// LLM request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    pub messages: Vec<LLMMessage>,
    pub config: LLMConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt: u32,
    pub completion: u32,
    pub total: u32,
}

/// LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: String,
    pub model: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_usage: Option<TokenUsage>,
    pub finish_reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Provider capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub models: Vec<String>,
    pub supports_streaming: bool,
    pub supports_functions: bool,
    pub supports_vision: bool,
    pub max_tokens: u32,
    pub rate_limits: Option<RateLimits>,
}

/// Rate limiting information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    pub requests_per_minute: u32,
    pub tokens_per_minute: u32,
    pub requests_per_day: Option<u32>,
}

/// Stream response wrapper (removed Debug derive due to trait object)
pub struct LLMStreamResponse {
    pub stream: Box<dyn futures::Stream<Item = crate::errors::AriaResult<LLMResponse>> + Unpin + Send>,
} 