pub mod types;

use crate::errors::{AriaError, AriaResult, ErrorCategory, ErrorCode, ErrorSeverity};
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Duration;
use types::*;

/// Main LLM handler interface for provider abstraction
#[async_trait]
pub trait LLMHandlerInterface: Send + Sync {
    /// Complete a text prompt with the specified provider
    async fn complete(&self, request: LLMRequest) -> AriaResult<LLMResponse>;
    
    /// Stream completion response (for future implementation)
    async fn stream_complete(&self, request: LLMRequest) -> AriaResult<LLMStreamResponse>;
    
    /// Get available providers
    fn get_providers(&self) -> Vec<String>;
    
    /// Set default provider
    async fn set_default_provider(&self, provider: &str) -> AriaResult<()>;
    
    /// Get provider capabilities
    async fn get_provider_capabilities(&self, provider: &str) -> AriaResult<ProviderCapabilities>;
    
    /// Health check for specific provider
    async fn health_check_provider(&self, provider: &str) -> AriaResult<bool>;
}

/// Concrete LLM handler implementation
pub struct LLMHandler {
    providers: HashMap<String, Box<dyn LLMProvider>>,
    default_provider: String,
    timeout: Duration,
    retry_attempts: u32,
}

impl LLMHandler {
    pub fn new() -> Self {
        let mut providers: HashMap<String, Box<dyn LLMProvider>> = HashMap::new();
        
        // Add default providers
        providers.insert("openai".to_string(), Box::new(OpenAIProvider::new()));
        providers.insert("anthropic".to_string(), Box::new(AnthropicProvider::new()));
        
        Self {
            providers,
            default_provider: "openai".to_string(),
            timeout: Duration::from_secs(30),
            retry_attempts: 3,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_retry_attempts(mut self, attempts: u32) -> Self {
        self.retry_attempts = attempts;
        self
    }

    pub fn add_provider(&mut self, name: String, provider: Box<dyn LLMProvider>) {
        self.providers.insert(name, provider);
    }

    /// Execute request with retry logic
    async fn execute_with_retry<F, T, Fut>(&self, mut operation: F) -> AriaResult<T>
    where
        F: FnMut() -> Fut,
        Fut: futures::Future<Output = AriaResult<T>>,
    {
        let mut last_error = None;
        
        for attempt in 1..=self.retry_attempts {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    last_error = Some(error);
                    
                    if attempt < self.retry_attempts {
                        // Exponential backoff
                        let delay = Duration::from_millis(100 * (2_u64.pow(attempt - 1)));
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| AriaError::new(
            ErrorCode::LLMApiError,
            ErrorCategory::LLM,
            ErrorSeverity::High,
            "All retry attempts failed"
        )))
    }
}

#[async_trait]
impl LLMHandlerInterface for LLMHandler {
    async fn complete(&self, request: LLMRequest) -> AriaResult<LLMResponse> {
        let provider_name = request.provider.as_deref().unwrap_or(&self.default_provider);
        
        let provider = self.providers.get(provider_name)
            .ok_or_else(|| AriaError::new(
                ErrorCode::LLMProviderNotFound,
                ErrorCategory::LLM,
                ErrorSeverity::High,
                format!("Provider '{}' not found", provider_name)
            ))?;

        self.execute_with_retry(|| {
            Box::pin(async {
                // Apply timeout wrapper
                tokio::time::timeout(self.timeout, provider.complete(request.clone())).await
                    .map_err(|_| AriaError::new(
                        ErrorCode::LLMTimeout,
                        ErrorCategory::LLM,
                        ErrorSeverity::Medium,
                        format!("LLM request timed out after {:?}", self.timeout)
                    ))?
            })
        }).await
    }

    async fn stream_complete(&self, request: LLMRequest) -> AriaResult<LLMStreamResponse> {
        let provider_name = request.provider.as_deref().unwrap_or(&self.default_provider);
        
        let provider = self.providers.get(provider_name)
            .ok_or_else(|| AriaError::new(
                ErrorCode::LLMProviderNotFound,
                ErrorCategory::LLM,
                ErrorSeverity::High,
                format!("Provider '{}' not found", provider_name)
            ))?;

        provider.stream_complete(request).await
    }

    fn get_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    async fn set_default_provider(&self, provider: &str) -> AriaResult<()> {
        if self.providers.contains_key(provider) {
            // Note: In a real implementation, we'd want to make this mutable
            // For now, we'll just validate the provider exists
            Ok(())
        } else {
            Err(AriaError::new(
                ErrorCode::LLMProviderNotFound,
                ErrorCategory::LLM,
                ErrorSeverity::Medium,
                format!("Provider '{}' not found", provider)
            ))
        }
    }

    async fn get_provider_capabilities(&self, provider: &str) -> AriaResult<ProviderCapabilities> {
        let provider_impl = self.providers.get(provider)
            .ok_or_else(|| AriaError::new(
                ErrorCode::LLMProviderNotFound,
                ErrorCategory::LLM,
                ErrorSeverity::Medium,
                format!("Provider '{}' not found", provider)
            ))?;

        provider_impl.get_capabilities().await
    }

    async fn health_check_provider(&self, provider: &str) -> AriaResult<bool> {
        let provider_impl = self.providers.get(provider)
            .ok_or_else(|| AriaError::new(
                ErrorCode::LLMProviderNotFound,
                ErrorCategory::LLM,
                ErrorSeverity::Medium,
                format!("Provider '{}' not found", provider)
            ))?;

        provider_impl.health_check().await
    }
}

/// Individual LLM provider trait
#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn complete(&self, request: LLMRequest) -> AriaResult<LLMResponse>;
    async fn stream_complete(&self, request: LLMRequest) -> AriaResult<LLMStreamResponse>;
    async fn get_capabilities(&self) -> AriaResult<ProviderCapabilities>;
    async fn health_check(&self) -> AriaResult<bool>;
}

/// OpenAI provider implementation
pub struct OpenAIProvider {
    api_key: Option<String>,
    base_url: String,
}

impl OpenAIProvider {
    pub fn new() -> Self {
        Self {
            api_key: std::env::var("OPENAI_API_KEY").ok(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn complete(&self, request: LLMRequest) -> AriaResult<LLMResponse> {
        // For now, return a mock response
        // TODO: Implement actual OpenAI API integration
        Ok(LLMResponse {
            content: serde_json::json!({
                "tool_name": "none",
                "response": "Mock OpenAI response - API integration not yet implemented"
            }).to_string(),
            model: request.config.model.clone(),
            provider: "openai".to_string(),
            token_usage: Some(TokenUsage {
                prompt: 100,
                completion: 50,
                total: 150,
            }),
            finish_reason: "stop".to_string(),
            tool_calls: None,
        })
    }

    async fn stream_complete(&self, _request: LLMRequest) -> AriaResult<LLMStreamResponse> {
        Err(AriaError::new(
            ErrorCode::LLMApiError,
            ErrorCategory::LLM,
            ErrorSeverity::Medium,
            "Streaming not yet implemented for OpenAI provider"
        ))
    }

    async fn get_capabilities(&self) -> AriaResult<ProviderCapabilities> {
        Ok(ProviderCapabilities {
            models: vec![
                "gpt-4o".to_string(),
                "gpt-4-turbo".to_string(),
                "gpt-3.5-turbo".to_string(),
            ],
            supported_modes: vec!["text".to_string(), "vision".to_string()],
            supports_streaming: true,
            supports_json_response: true,
            rate_limits: Some(RateLimits {
                requests_per_minute: 5000,
                tokens_per_minute: 600000,
            }),
        })
    }

    async fn health_check(&self) -> AriaResult<bool> {
        // Simple health check - verify API key is present
        Ok(self.api_key.is_some())
    }
}

/// Anthropic provider implementation
pub struct AnthropicProvider {
    api_key: Option<String>,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new() -> Self {
        Self {
            api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
            base_url: "https://api.anthropic.com".to_string(),
        }
    }

    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    async fn complete(&self, request: LLMRequest) -> AriaResult<LLMResponse> {
        // For now, return a mock response
        // TODO: Implement actual Anthropic API integration
        Ok(LLMResponse {
            content: serde_json::json!({
                "tool_name": "none",
                "response": "Mock Anthropic response - API integration not yet implemented"
            }).to_string(),
            model: request.config.model.clone(),
            provider: "anthropic".to_string(),
            token_usage: Some(TokenUsage {
                prompt: 120,
                completion: 60,
                total: 180,
            }),
            finish_reason: "stop".to_string(),
            tool_calls: None,
        })
    }

    async fn stream_complete(&self, _request: LLMRequest) -> AriaResult<LLMStreamResponse> {
        Err(AriaError::new(
            ErrorCode::LLMApiError,
            ErrorCategory::LLM,
            ErrorSeverity::Medium,
            "Streaming not yet implemented for Anthropic provider"
        ))
    }

    async fn get_capabilities(&self) -> AriaResult<ProviderCapabilities> {
        Ok(ProviderCapabilities {
            models: vec![
                "claude-3-opus-20240229".to_string(),
                "claude-3-sonnet-20240229".to_string(),
                "claude-3-haiku-20240307".to_string(),
            ],
            supported_modes: vec!["text".to_string()],
            supports_streaming: true,
            supports_json_response: false,
            rate_limits: Some(RateLimits {
                requests_per_minute: 1000,
                tokens_per_minute: 400000,
            }),
        })
    }

    async fn health_check(&self) -> AriaResult<bool> {
        // Simple health check - verify API key is present
        Ok(self.api_key.is_some())
    }
}

impl Default for OpenAIProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for AnthropicProvider {
    fn default() -> Self {
        Self::new()
    }
}
