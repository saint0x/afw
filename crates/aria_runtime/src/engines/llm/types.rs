use serde::{Deserialize, Serialize};

/// Represents a single message in a conversation sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
}

/// Defines the configuration for an LLM request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub top_p: Option<f32>,
    pub stop_sequences: Option<Vec<String>>,
    pub stream: bool,
}

/// Represents a request to an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    pub messages: Vec<LLMMessage>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
    pub tools: Option<Vec<ToolDefinition>>,
    pub config: LLMConfig,
    pub expects_json: bool,
}

/// Represents a token usage summary for an LLM call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt: u32,
    pub completion: u32,
    pub total: u32,
}

/// Represents a single tool call requested by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Represents a tool definition for LLM function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Represents the response from an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: String,
    pub model: String,
    pub provider: String,
    pub token_usage: Option<TokenUsage>,
    pub finish_reason: String,
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Represents a streaming response from an LLM (not yet implemented).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMStreamResponse;

/// Describes the rate limits for a given LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    pub requests_per_minute: u32,
    pub tokens_per_minute: u32,
}

/// Describes the capabilities of an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub models: Vec<String>,
    pub supported_modes: Vec<String>, // e.g., "text", "vision"
    pub supports_streaming: bool,
    pub supports_json_response: bool,
    pub rate_limits: Option<RateLimits>,
} 