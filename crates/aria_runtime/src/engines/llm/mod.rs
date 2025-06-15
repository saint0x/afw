pub mod types;
pub mod providers;

use crate::errors::{AriaError, AriaResult, ErrorCategory, ErrorCode, ErrorSeverity};
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Duration;
use types::*;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Instant};

/// Main LLM handler interface for provider abstraction
#[async_trait]
pub trait LLMHandlerInterface: Send + Sync {
    /// Complete a text prompt with the specified provider
    async fn complete(&self, request: types::LLMRequest) -> AriaResult<types::LLMResponse>;
    
    /// Stream completion response (for future implementation)
    async fn stream_complete(&self, request: types::LLMRequest) -> AriaResult<types::LLMStreamResponse>;
    
    /// Get available providers
    fn get_providers(&self) -> Vec<String>;
    
    /// Set default provider
    async fn set_default_provider(&self, provider: &str) -> AriaResult<()>;
    
    /// Get provider capabilities
    async fn get_provider_capabilities(&self, provider: &str) -> AriaResult<types::ProviderCapabilities>;
    
    /// Health check for specific provider
    async fn health_check_provider(&self, provider: &str) -> AriaResult<bool>;
}

/// Configuration for LLM providers (matches Symphony SDK pattern)
#[derive(Debug, Clone)]
pub struct LLMConfig {
    pub provider: String,
    pub api_key: String,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub timeout: Option<u64>,
}

/// Request configuration (no sensitive data)
#[derive(Debug, Clone)]
pub struct LLMRequestConfig {
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub timeout: Option<u64>,
}

/// Production-grade LLM handler that matches Symphony SDK proven pattern
/// Singleton pattern for global access, environment-based initialization
pub struct LLMHandler {
    /// Registered LLM providers indexed by name
    providers: Arc<Mutex<HashMap<String, Box<dyn LLMProvider>>>>,
    /// Default provider name
    default_provider: Arc<Mutex<Option<String>>>,
    /// Response cache for cost optimization
    response_cache: Arc<Mutex<HashMap<String, CachedResponse>>>,
    /// Configuration
    config: LLMHandlerConfig,
}

#[derive(Debug, Clone)]
pub struct LLMHandlerConfig {
    pub cache_enabled: bool,
    pub cache_ttl_seconds: u64,
    pub max_cache_size: usize,
    pub default_timeout_seconds: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
}

impl Default for LLMHandlerConfig {
    fn default() -> Self {
        Self {
            cache_enabled: true,
            cache_ttl_seconds: 3600, // 1 hour
            max_cache_size: 1000,
            default_timeout_seconds: 30,
            max_retries: 3,
            retry_delay_ms: 1000,
        }
    }
}

#[derive(Debug, Clone)]
struct CachedResponse {
    response: LLMResponse,
    cached_at: std::time::SystemTime,
    ttl_seconds: u64,
}

// Singleton instance (matches Symphony SDK pattern)
static LLM_HANDLER_INSTANCE: OnceLock<Arc<LLMHandler>> = OnceLock::new();

impl LLMHandler {
    /// Get singleton instance (matches Symphony SDK pattern)
    pub fn get_instance() -> Arc<LLMHandler> {
        LLM_HANDLER_INSTANCE.get_or_init(|| {
            let handler = Arc::new(LLMHandler::new());
            
            // Initialize default providers in background (matches Symphony pattern)
            let handler_clone = Arc::clone(&handler);
            tokio::spawn(async move {
                if let Err(e) = handler_clone.initialize_default_providers().await {
                    eprintln!("Failed to initialize default LLM providers: {:?}", e);
                }
            });
            
            handler
        }).clone()
    }

    /// Create new handler instance (private, use get_instance)
    fn new() -> Self {
        Self {
            providers: Arc::new(Mutex::new(HashMap::new())),
            default_provider: Arc::new(Mutex::new(None)),
            response_cache: Arc::new(Mutex::new(HashMap::new())),
            config: LLMHandlerConfig::default(),
        }
    }

    /// Initialize default providers based on environment (matches Symphony pattern)
    async fn initialize_default_providers(&self) -> AriaResult<()> {
        // Initialize OpenAI if API key is provided (exactly like Symphony SDK)
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            if !api_key.is_empty() {
                let config = LLMConfig {
                    provider: "openai".to_string(),
                    api_key: api_key.clone(),
                    model: Some("gpt-3.5-turbo".to_string()),
                    temperature: Some(0.7),
                    max_tokens: Some(4000),
                    timeout: Some(30),
                };

                self.register_provider(config).await?;
                
                // Set OpenAI as default provider (matches Symphony pattern)
                {
                    let mut default_provider = self.default_provider.lock().unwrap();
                    *default_provider = Some("openai".to_string());
                }
                
                return Ok(());
            }
        }

        Err(AriaError::new(
            ErrorCode::LLMApiError,
            ErrorCategory::LLM,
            ErrorSeverity::High,
            "OpenAI API key is required in environment configuration"
        ))
    }

    /// Register a provider (matches Symphony SDK pattern)
    pub async fn register_provider(&self, config: LLMConfig) -> AriaResult<()> {
        let provider_name = config.provider.to_lowercase();
        
        // Only OpenAI supported for now (like Symphony initially)
        if provider_name != "openai" {
            return Err(AriaError::new(
                ErrorCode::LLMProviderNotFound,
                ErrorCategory::LLM,
                ErrorSeverity::High,
                &format!("Provider {} not supported for registration yet", provider_name)
            ));
        }

        // Create OpenAI provider
        let provider = providers::openai::OpenAIProvider::new(config.api_key.clone())
            .with_model(config.model.unwrap_or_else(|| "gpt-3.5-turbo".to_string()))
            .with_timeout(config.timeout.unwrap_or(30));

        // Initialize the provider
        provider.initialize().await?;
        
        // Register the provider
        {
            let mut providers = self.providers.lock().unwrap();
            providers.insert(provider_name.clone(), Box::new(provider));
        }
        
        // Set as default if no default exists
        {
            let mut default_provider = self.default_provider.lock().unwrap();
            if default_provider.is_none() {
                *default_provider = Some(provider_name);
            }
        }
        
        Ok(())
    }

    /// Get provider (matches Symphony pattern)
    pub fn get_provider(&self, name: Option<&str>) -> AriaResult<Box<dyn LLMProvider>> {
        let provider_name = name
            .map(|s| s.to_lowercase())
            .or_else(|| self.get_default_provider_sync())
            .ok_or_else(|| AriaError::new(
                ErrorCode::LLMProviderNotFound,
                ErrorCategory::LLM,
                ErrorSeverity::High,
                "No provider specified and no default provider available"
            ))?;

        let providers = self.providers.lock().unwrap();
        providers.get(&provider_name)
            .ok_or_else(|| AriaError::new(
                ErrorCode::LLMProviderNotFound,
                ErrorCategory::LLM,
                ErrorSeverity::High,
                &format!("Provider '{}' not found", provider_name)
            ))
            .map(|p| p.clone_box())
    }

    /// Complete an LLM request (matches Symphony pattern)
    pub async fn complete(&self, request: types::LLMRequest) -> AriaResult<types::LLMResponse> {
        println!("🔍 DEBUG: LLMHandler::complete called");
        println!("🔍 DEBUG: Request provider: {:?}", request.provider);
        println!("🔍 DEBUG: Request messages count: {}", request.messages.len());
        println!("🔍 DEBUG: Request config: {:?}", request.config);
        
        let target_provider_name = request.provider.clone()
            .or_else(|| self.get_default_provider_sync());

        println!("🔍 DEBUG: Target provider name: {:?}", target_provider_name);

        if target_provider_name.is_none() {
            println!("🔍 DEBUG: No provider available - returning error");
            return Err(AriaError::new(
                ErrorCode::LLMProviderNotFound,
                ErrorCategory::LLM,
                ErrorSeverity::High,
                "No provider specified in request and no default provider set"
            ));
        }

        let provider_name = target_provider_name.unwrap();
        println!("🔍 DEBUG: Using provider: {}", provider_name);
        
        // On-demand provider initialization (matches Symphony pattern)
        if !self.has_provider(&provider_name) {
            println!("🔍 DEBUG: Provider {} not found, attempting to register", provider_name);
            if provider_name == "openai" {
                if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
                    if !api_key.is_empty() {
                        println!("🔍 DEBUG: Found OpenAI API key, registering provider");
                        let config = LLMConfig {
                            provider: "openai".to_string(),
                            api_key,
                            model: request.config.model.clone(),
                            temperature: Some(request.config.temperature),
                            max_tokens: Some(request.config.max_tokens),
                            timeout: Some(30),
                        };
                        self.register_provider(config).await?;
                        println!("🔍 DEBUG: OpenAI provider registered successfully");
                    } else {
                        println!("🔍 DEBUG: OpenAI API key is empty");
                    }
                } else {
                    println!("🔍 DEBUG: OpenAI API key not found in environment");
                }
            }
        } else {
            println!("🔍 DEBUG: Provider {} already exists", provider_name);
        }

        println!("🔍 DEBUG: Getting provider instance");
        let provider = self.get_provider(Some(&provider_name))?;
        
        println!("🔍 DEBUG: Calling provider.complete()");
        let result = provider.complete(request).await;
        
        match &result {
            Ok(response) => {
                println!("🔍 DEBUG: Provider returned success!");
                println!("🔍 DEBUG: Response content length: {}", response.content.len());
                println!("🔍 DEBUG: Response content (first 200 chars): {}", 
                    response.content.chars().take(200).collect::<String>());
                println!("🔍 DEBUG: Response model: {}", response.model);
            }
            Err(e) => {
                println!("🔍 DEBUG: Provider returned error: {:?}", e);
            }
        }
        
        result
    }

    /// Simple inference method (matches Symphony pattern)
    pub async fn inference(&self, prompt: &str, llm_config: Option<LLMRequestConfig>) -> AriaResult<String> {
        let config = if let Some(req_config) = llm_config {
            types::LLMConfig {
                model: req_config.model,
                temperature: req_config.temperature.unwrap_or(0.7),
                max_tokens: req_config.max_tokens.unwrap_or(2048),
                top_p: None,
                frequency_penalty: None,
                presence_penalty: None,
            }
        } else {
            types::LLMConfig::default()
        };

        let request = types::LLMRequest {
            messages: vec![types::LLMMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
                tool_call_id: None,
                tool_calls: None,
            }],
            config,
            provider: None,
            tools: None,
            tool_choice: None,
            stream: None,
        };
        
        let response = self.complete(request).await?;
        Ok(response.content)
    }

    /// Check if provider exists
    fn has_provider(&self, name: &str) -> bool {
        let providers = self.providers.lock().unwrap();
        providers.contains_key(name)
    }

    /// Get default provider name (sync version, matches Symphony pattern)
    fn get_default_provider_sync(&self) -> Option<String> {
        // First check if we have a set default
        {
            let default_provider = self.default_provider.lock().unwrap();
            if let Some(ref provider) = *default_provider {
                return Some(provider.clone());
            }
        }
        
        // Fallback: if we have OpenAI API key, return "openai" as default
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            if !api_key.is_empty() {
                return Some("openai".to_string());
            }
        }
        None
    }

    /// Get available providers
    pub fn get_available_providers(&self) -> Vec<String> {
        let providers = self.providers.lock().unwrap();
        providers.keys().cloned().collect()
    }
}

/// Trait for LLM providers with enhanced capabilities
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Provider name
    fn name(&self) -> &str;
    
    /// Whether this provider supports streaming
    fn supports_streaming(&self) -> bool;
    
    /// Whether this provider supports function calling
    fn supports_functions(&self) -> bool;
    
    /// Initialize the provider
    async fn initialize(&self) -> AriaResult<()>;
    
    /// Complete a request
    async fn complete(&self, request: types::LLMRequest) -> AriaResult<types::LLMResponse>;
    
    /// Stream a request (TODO: implement)
    async fn complete_stream(&self, _request: types::LLMRequest) -> AriaResult<Box<dyn futures::Stream<Item = AriaResult<types::LLMResponse>> + Unpin + Send>> {
        Err(AriaError::new(
            ErrorCode::LLMApiError,
            ErrorCategory::LLM,
            ErrorSeverity::Medium,
            "Streaming not yet implemented"
        ))
    }
    
    /// Health check
    async fn health_check(&self) -> AriaResult<bool>;
    
    /// Clone the provider (for Arc storage)
    fn clone_box(&self) -> Box<dyn LLMProvider>;
}

impl Default for LLMRequestConfig {
    fn default() -> Self {
        Self {
            model: None,
            temperature: None,
            max_tokens: None,
            timeout: None,
        }
    }
}
