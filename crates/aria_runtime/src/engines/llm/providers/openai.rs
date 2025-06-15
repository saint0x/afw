use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use reqwest::{Client, header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE}};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::time::{timeout, Duration};

use crate::engines::llm::{LLMProvider, types::*};
use crate::errors::{AriaResult, AriaError, ErrorCode, ErrorCategory, ErrorSeverity};

/// Production-grade OpenAI provider with streaming, function calling, and comprehensive error handling
#[derive(Clone)]
pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    base_url: String,
    default_model: String,
    timeout_seconds: u64,
}

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: f32,
    max_tokens: Option<u32>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAITool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAIToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAITool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAIFunction,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIFunction {
    name: String,
    description: String,
    parameters: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OpenAIFunctionCall,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    index: u32,
    message: OpenAIMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChunk {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<OpenAIStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    index: u32,
    delta: OpenAIMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIError {
    error: OpenAIErrorDetails,
}

#[derive(Debug, Deserialize)]
struct OpenAIErrorDetails {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
    code: Option<String>,
}

impl OpenAIProvider {
    pub fn new(api_key: String) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key))
                .expect("Invalid API key format")
        );

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
            default_model: "gpt-4".to_string(),
            timeout_seconds: 60,
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.default_model = model;
        self
    }

    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    /// Convert Aria LLM request to OpenAI format
    fn convert_request(&self, request: &LLMRequest) -> OpenAIRequest {
        let messages = request.messages.iter().map(|msg| {
            OpenAIMessage {
                role: msg.role.clone(),
                content: Some(msg.content.clone()),
                tool_calls: msg.tool_calls.as_ref().map(|calls| {
                    calls.iter().map(|call| OpenAIToolCall {
                        id: call.id.clone(),
                        call_type: "function".to_string(),
                        function: OpenAIFunctionCall {
                            name: call.name.clone(),
                            arguments: call.arguments.clone(),
                        },
                    }).collect()
                }),
                tool_call_id: msg.tool_call_id.clone(),
            }
        }).collect();

        let tools = request.tools.as_ref().map(|tools| {
            tools.iter().map(|tool| OpenAITool {
                tool_type: "function".to_string(),
                function: OpenAIFunction {
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    parameters: tool.parameters.clone(),
                },
            }).collect()
        });

        let tool_choice = request.tool_choice.as_ref().map(|choice| {
            match choice {
                ToolChoice::None => json!("none"),
                ToolChoice::Auto => json!("auto"),
                ToolChoice::Required => json!("required"),
                ToolChoice::Function { name } => json!({
                    "type": "function",
                    "function": { "name": name }
                }),
            }
        });

        OpenAIRequest {
            model: request.config.model.clone().unwrap_or_else(|| self.default_model.clone()),
            messages,
            temperature: request.config.temperature,
            max_tokens: Some(request.config.max_tokens),
            stream: false,
            tools,
            tool_choice,
        }
    }

    /// Convert OpenAI response to Aria format
    fn convert_response(&self, response: OpenAIResponse) -> AriaResult<LLMResponse> {
        let choice = response.choices.into_iter().next()
            .ok_or_else(|| AriaError::new(
                ErrorCode::LLMInvalidResponse,
                ErrorCategory::LLM,
                ErrorSeverity::High,
                "No choices in OpenAI response"
            ))?;

        let tool_calls = choice.message.tool_calls.map(|calls| {
            calls.into_iter().map(|call| ToolCall {
                id: call.id,
                name: call.function.name,
                arguments: call.function.arguments,
            }).collect()
        });

        let usage = response.usage.map(|u| TokenUsage {
            prompt: u.prompt_tokens,
            completion: u.completion_tokens,
            total: u.total_tokens,
        });

        Ok(LLMResponse {
            content: choice.message.content.unwrap_or_default(),
            model: response.model,
            provider: "openai".to_string(),
            token_usage: usage,
            finish_reason: choice.finish_reason.unwrap_or_else(|| "stop".to_string()),
            tool_calls,
        })
    }

    /// Handle OpenAI API errors
    fn handle_api_error(&self, status: u16, body: &str) -> AriaError {
        // Try to parse as OpenAI error format
        if let Ok(error_response) = serde_json::from_str::<OpenAIError>(body) {
            let error_details = error_response.error;
            
            let (code, severity) = match status {
                400 => (ErrorCode::LLMInvalidRequest, ErrorSeverity::Medium),
                401 => (ErrorCode::LLMAuthentication, ErrorSeverity::Critical),
                429 => (ErrorCode::LLMTokenLimitExceeded, ErrorSeverity::Medium),
                500..=599 => (ErrorCode::LLMProviderError, ErrorSeverity::High),
                _ => (ErrorCode::LLMError, ErrorSeverity::Medium),
            };

            AriaError::new(
                code,
                ErrorCategory::LLM,
                severity,
                &format!("OpenAI API error ({}): {}", status, error_details.message)
            )
        } else {
            AriaError::new(
                ErrorCode::LLMError,
                ErrorCategory::LLM,
                ErrorSeverity::Medium,
                &format!("OpenAI API error ({}): {}", status, body)
            )
        }
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_functions(&self) -> bool {
        true
    }

    async fn initialize(&self) -> AriaResult<()> {
        if self.api_key.is_empty() {
            return Err(AriaError::new(
                ErrorCode::LLMProviderNotFound,
                ErrorCategory::LLM,
                ErrorSeverity::Critical,
                "OpenAI API key not provided"
            ));
        }
        
        // Test API key with a minimal request
        match self.health_check().await {
            Ok(true) => Ok(()),
            Ok(false) => Err(AriaError::new(
                ErrorCode::LLMProviderNotFound,
                ErrorCategory::LLM,
                ErrorSeverity::Critical,
                &format!("OpenAI initialization failed: {}", "API key validation failed")
            )),
            Err(e) => Err(AriaError::new(
                ErrorCode::LLMProviderNotFound,
                ErrorCategory::LLM,
                ErrorSeverity::Critical,
                &format!("OpenAI initialization failed: {}", e)
            )),
        }
    }

    async fn complete(&self, request: LLMRequest) -> AriaResult<LLMResponse> {
        let openai_request = self.convert_request(&request);
        
        let response = match tokio::time::timeout(
            Duration::from_secs(self.timeout_seconds),
            self.client.post(&format!("{}/chat/completions", self.base_url))
                .json(&openai_request)
                .send()
        ).await {
            Ok(Ok(response)) => response,
            Ok(Err(e)) => return Err(AriaError::new(
                ErrorCode::LLMApiError,
                ErrorCategory::LLM,
                ErrorSeverity::High,
                &format!("OpenAI request failed: {}", e)
            )),
            Err(_) => return Err(AriaError::new(
                ErrorCode::LLMApiError,
                ErrorCategory::LLM,
                ErrorSeverity::High,
                &format!("OpenAI request timeout after {} seconds", self.timeout_seconds)
            )),
        };

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(self.handle_api_error(status, &body));
        }

        let openai_response: OpenAIResponse = response.json().await
            .map_err(|e| AriaError::new(
                ErrorCode::LLMInvalidResponse,
                ErrorCategory::LLM,
                ErrorSeverity::High,
                &format!("Failed to parse OpenAI response: {}", e)
            ))?;

        self.convert_response(openai_response)
    }

    async fn complete_stream(&self, request: LLMRequest) -> AriaResult<Box<dyn Stream<Item = AriaResult<LLMResponse>> + Unpin + Send>> {
        let mut openai_request = self.convert_request(&request);
        openai_request.stream = true;

        let response = tokio::time::timeout(
            Duration::from_secs(self.timeout_seconds),
            self.client.post(&format!("{}/chat/completions", self.base_url))
                .json(&openai_request)
                .send()
        ).await
        .map_err(|_| AriaError::new(
            ErrorCode::LLMTimeout,
            ErrorCategory::LLM,
            ErrorSeverity::Medium,
            &format!("OpenAI stream request timeout after {} seconds", self.timeout_seconds)
        ))?
        .map_err(|e| AriaError::new(
            ErrorCode::LLMError,
            ErrorCategory::LLM,
            ErrorSeverity::High,
            &format!("OpenAI stream request failed: {}", e)
        ))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(self.handle_api_error(status, &body));
        }

        let stream = OpenAIStreamWrapper::new(Box::pin(response.bytes_stream()));
        Ok(Box::new(stream))
    }

    async fn health_check(&self) -> AriaResult<bool> {
        // Simple health check with minimal request using raw HTTP
        let test_request = serde_json::json!({
            "model": "gpt-3.5-turbo",
            "messages": [{"role": "user", "content": "test"}],
            "max_tokens": 1
        });
        
        match tokio::time::timeout(
            Duration::from_secs(self.timeout_seconds),
            self.client.post(&format!("{}/chat/completions", self.base_url))
                .json(&test_request)
                .send()
        ).await {
            Ok(Ok(response)) => Ok(response.status().is_success()),
            Ok(Err(_)) => Ok(false),
            Err(_) => Ok(false), // Timeout means not healthy
        }
    }

    fn clone_box(&self) -> Box<dyn LLMProvider> {
        Box::new(self.clone())
    }
}

/// Stream wrapper for OpenAI streaming responses
pub struct OpenAIStreamWrapper {
    stream: Pin<Box<dyn Stream<Item = reqwest::Result<Bytes>> + Send>>,
    buffer: String,
}

impl OpenAIStreamWrapper {
    fn new(stream: Pin<Box<dyn Stream<Item = reqwest::Result<Bytes>> + Send>>) -> Self {
        Self {
            stream,
            buffer: String::new(),
        }
    }

    fn parse_chunk(&mut self) -> Option<AriaResult<LLMResponse>> {
        // Find the first complete message ("\n\n")
        if let Some(end_of_message) = self.buffer.find("\n\n") {
            let message_str = self.buffer[..end_of_message].to_string();
            // Remove the processed message from the buffer
            self.buffer.drain(..end_of_message + 2);

            if message_str.starts_with("data: ") {
                let data = &message_str[6..];
                if data.trim() == "[DONE]" {
                    // End of stream signal from OpenAI
                    return None;
                }

                match serde_json::from_str::<OpenAIStreamChunk>(data) {
                    Ok(stream_chunk) => {
                        if let Some(choice) = stream_chunk.choices.into_iter().next() {
                            let response = LLMResponse {
                                content: choice.delta.content.unwrap_or_default(),
                                model: stream_chunk.model,
                                provider: "openai".to_string(),
                                token_usage: None, // Usage is typically only in the final chunk which is not handled here
                                finish_reason: choice.finish_reason.unwrap_or_else(|| "streaming".to_string()),
                                tool_calls: choice.delta.tool_calls.map(|calls| {
                                    calls.into_iter().map(|call| ToolCall {
                                        id: call.id,
                                        name: call.function.name,
                                        arguments: call.function.arguments.clone(),
                                    }).collect()
                                }),
                            };
                            return Some(Ok(response));
                        }
                    }
                    Err(e) => {
                        // Failed to parse a chunk, which is a stream error
                        return Some(Err(AriaError::new(
                            ErrorCode::LLMInvalidResponse,
                            ErrorCategory::LLM,
                            ErrorSeverity::Medium,
                            &format!("Failed to parse stream chunk: {}", e),
                        )));
                    }
                }
            }
        }
        // No complete message in buffer yet
        None
    }
}

impl Stream for OpenAIStreamWrapper {
    type Item = AriaResult<LLMResponse>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            // First, try to parse a message from the existing buffer
            if let Some(message_result) = self.parse_chunk() {
                return Poll::Ready(Some(message_result));
            }

            // If no full message is in the buffer, poll the byte stream for more data
            match self.stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    // Append new data to the buffer.
                    self.buffer.push_str(&String::from_utf8_lossy(&bytes));
                    // Loop again to try parsing the updated buffer
                }
                Poll::Ready(Some(Err(e))) => {
                    // The underlying byte stream produced an error
                    return Poll::Ready(Some(Err(AriaError::new(
                        ErrorCode::LLMError,
                        ErrorCategory::LLM,
                        ErrorSeverity::High,
                        &format!("Underlying stream error: {}", e),
                    ))));
                }
                Poll::Ready(None) => {
                    // The byte stream is finished. If the buffer is empty, the stream is done.
                    // If the buffer is not empty, it means there's a partial message left, which is an error.
                    return if self.buffer.is_empty() {
                        Poll::Ready(None)
                    } else {
                        Poll::Ready(Some(Err(AriaError::new(
                            ErrorCode::LLMInvalidResponse,
                            ErrorCategory::LLM,
                            ErrorSeverity::Medium,
                            "Stream ended with incomplete data",
                        ))))
                    };
                }
                Poll::Pending => {
                    // The byte stream is not ready, so we are not ready.
                    return Poll::Pending;
                }
            }
        }
    }
}

impl Unpin for OpenAIStreamWrapper {} 