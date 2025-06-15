pub mod types;
pub mod providers;

use crate::errors::{AriaError, AriaResult, ErrorCategory, ErrorCode, ErrorSeverity};
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Duration;
use types::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Instant};

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

/// Production-grade LLM handler that manages multiple providers and implements
/// intelligent caching, fallbacks, and cost optimization
pub struct LLMHandler {
    /// Registered LLM providers indexed by name
    providers: Arc<RwLock<HashMap<String, Box<dyn LLMProvider>>>>,
    /// Default provider name
    default_provider: Arc<RwLock<Option<String>>>,
    /// Response cache for cost optimization
    response_cache: Arc<RwLock<HashMap<String, CachedResponse>>>,
    /// Request metrics for monitoring
    metrics: Arc<RwLock<LLMMetrics>>,
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
    pub enable_metrics: bool,
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
            enable_metrics: true,
        }
    }
}

#[derive(Debug, Clone)]
struct CachedResponse {
    response: LLMResponse,
    cached_at: std::time::SystemTime,
    ttl_seconds: u64,
}

#[derive(Debug, Clone, Default)]
pub struct LLMMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub total_tokens_used: u64,
    pub total_cost_usd: f64,
    pub average_response_time_ms: f64,
    pub provider_usage: HashMap<String, u64>,
}

impl LLMHandler {
    pub fn new(config: LLMHandlerConfig) -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            default_provider: Arc::new(RwLock::new(None)),
            response_cache: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(LLMMetrics::default())),
            config,
        }
    }

    /// Register a new LLM provider
    pub async fn register_provider(&self, provider: Box<dyn LLMProvider>) -> AriaResult<()> {
        let provider_name = provider.name().to_string();
        
        // Initialize the provider
        provider.initialize().await?;
        
        // Register the provider
        {
            let mut providers = self.providers.write().await;
            providers.insert(provider_name.clone(), provider);
        }
        
        // Set as default if no default exists
        {
            let mut default_provider = self.default_provider.write().await;
            if default_provider.is_none() {
                *default_provider = Some(provider_name.clone());
            }
        }
        
        Ok(())
    }

    /// Complete an LLM request with intelligent caching and fallbacks
    pub async fn complete(&self, request: LLMRequest) -> AriaResult<LLMResponse> {
        let start_time = Instant::now();
        
        // Check cache first
        if self.config.cache_enabled {
            if let Some(cached) = self.get_cached_response(&request).await {
                self.update_metrics_cache_hit().await;
                return Ok(cached);
            }
        }
        
        // Get provider
        let provider_name = request.provider.clone()
            .or_else(|| self.get_default_provider_sync())
            .ok_or_else(|| AriaError::new(
                ErrorCode::LLMProviderNotFound,
                ErrorCategory::LLM,
                ErrorSeverity::High,
                "No provider specified and no default provider available"
            ))?;

        let provider = {
            let providers = self.providers.read().await;
            providers.get(&provider_name)
                .ok_or_else(|| AriaError::new(
                    ErrorCode::LLMProviderNotFound,
                    ErrorCategory::LLM,
                    ErrorSeverity::High,
                    format!("Provider '{}' not found", provider_name)
                ))?
                .clone_box()
        };

        // Execute request with retries
        let mut last_error = None;
        for attempt in 0..=self.config.max_retries {
            match provider.complete(request.clone()).await {
                Ok(response) => {
                    // Cache successful response
                    if self.config.cache_enabled {
                        self.cache_response(&request, &response).await;
                    }
                    
                    // Update metrics
                    self.update_metrics_success(&provider_name, &response, start_time.elapsed()).await;
                    
                    return Ok(response);
                },
                Err(error) => {
                    last_error = Some(error);
                    
                    // Don't retry on certain errors
                    if let Some(ref err) = last_error {
                        if matches!(err.code, ErrorCode::LLMInvalidResponse | ErrorCode::LLMTokenLimitExceeded) {
                            break;
                        }
                    }
                    
                    // Wait before retry
                    if attempt < self.config.max_retries {
                        tokio::time::sleep(Duration::from_millis(
                            self.config.retry_delay_ms * (attempt + 1) as u64
                        )).await;
                    }
                }
            }
        }
        
        // Update failure metrics
        self.update_metrics_failure(&provider_name).await;
        
        Err(last_error.unwrap_or_else(|| AriaError::new(
            ErrorCode::LLMApiError,
            ErrorCategory::LLM,
            ErrorSeverity::High,
            "LLM request failed after all retries"
        )))
    }

    /// Stream an LLM request
    pub async fn complete_stream(&self, request: LLMRequest) -> AriaResult<Box<dyn futures::Stream<Item = AriaResult<LLMResponse>> + Unpin + Send>> {
        let provider_name = request.provider.clone()
            .or_else(|| self.get_default_provider_sync())
            .ok_or_else(|| AriaError::new(
                ErrorCode::LLMProviderNotFound,
                ErrorCategory::LLM,
                ErrorSeverity::High,
                "No provider specified and no default provider available"
            ))?;

        let provider = {
            let providers = self.providers.read().await;
            providers.get(&provider_name)
                .ok_or_else(|| AriaError::new(
                    ErrorCode::LLMProviderNotFound,
                    ErrorCategory::LLM,
                    ErrorSeverity::High,
                    format!("Provider '{}' not found", provider_name)
                ))?
                .clone_box()
        };

        provider.complete_stream(request).await
    }

    /// Get cached response if available and not expired
    async fn get_cached_response(&self, request: &LLMRequest) -> Option<LLMResponse> {
        let cache_key = self.generate_cache_key(request);
        let cache = self.response_cache.read().await;
        
        if let Some(cached) = cache.get(&cache_key) {
            let now = std::time::SystemTime::now();
            let elapsed = now.duration_since(cached.cached_at).ok()?;
            
            if elapsed.as_secs() < cached.ttl_seconds {
                return Some(cached.response.clone());
            }
        }
        
        None
    }

    /// Cache a successful response
    async fn cache_response(&self, request: &LLMRequest, response: &LLMResponse) {
        if !self.config.cache_enabled {
            return;
        }
        
        let cache_key = self.generate_cache_key(request);
        let cached_response = CachedResponse {
            response: response.clone(),
            cached_at: std::time::SystemTime::now(),
            ttl_seconds: self.config.cache_ttl_seconds,
        };
        
        let mut cache = self.response_cache.write().await;
        
        // Implement LRU eviction if cache is full
        if cache.len() >= self.config.max_cache_size {
            // Remove oldest entry
            if let Some(oldest_key) = cache.keys().next().cloned() {
                cache.remove(&oldest_key);
            }
        }
        
        cache.insert(cache_key, cached_response);
    }

    /// Generate cache key for request
    fn generate_cache_key(&self, request: &LLMRequest) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        
        // Hash the essential parts of the request (skip f32 fields for now)
        request.messages.hash(&mut hasher);
        if let Some(ref model) = request.config.model {
            model.hash(&mut hasher);
        }
        request.config.max_tokens.hash(&mut hasher);
        
        format!("llm_cache_{:x}", hasher.finish())
    }

    /// Update metrics for cache hit
    async fn update_metrics_cache_hit(&self) {
        if !self.config.enable_metrics {
            return;
        }
        
        let mut metrics = self.metrics.write().await;
        metrics.cache_hits += 1;
    }

    /// Update metrics for successful request
    async fn update_metrics_success(&self, provider_name: &str, response: &LLMResponse, duration: Duration) {
        if !self.config.enable_metrics {
            return;
        }
        
        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;
        metrics.successful_requests += 1;
        metrics.cache_misses += 1;
        
        if let Some(usage) = &response.token_usage {
            metrics.total_tokens_used += usage.total as u64;
        }
        
        // Update average response time
        let new_avg = (metrics.average_response_time_ms * (metrics.successful_requests - 1) as f64 + duration.as_millis() as f64) / metrics.successful_requests as f64;
        metrics.average_response_time_ms = new_avg;
        
        // Update provider usage
        *metrics.provider_usage.entry(provider_name.to_string()).or_insert(0) += 1;
    }

    /// Update metrics for failed request
    async fn update_metrics_failure(&self, provider_name: &str) {
        if !self.config.enable_metrics {
            return;
        }
        
        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;
        metrics.failed_requests += 1;
        metrics.cache_misses += 1;
        
        // Update provider usage (even for failures)
        *metrics.provider_usage.entry(provider_name.to_string()).or_insert(0) += 1;
    }

    /// Get default provider name (sync version for internal use)
    fn get_default_provider_sync(&self) -> Option<String> {
        // This is a simplified sync version - in production you'd want to handle this better
        None // TODO: Implement proper sync access or restructure
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> LLMMetrics {
        self.metrics.read().await.clone()
    }

    /// Clear cache
    pub async fn clear_cache(&self) {
        let mut cache = self.response_cache.write().await;
        cache.clear();
    }

    /// Get available providers
    pub async fn get_available_providers(&self) -> Vec<String> {
        let providers = self.providers.read().await;
        providers.keys().cloned().collect()
    }
}

impl Clone for LLMHandler {
    fn clone(&self) -> Self {
        Self {
            providers: Arc::clone(&self.providers),
            default_provider: Arc::clone(&self.default_provider),
            response_cache: Arc::clone(&self.response_cache),
            metrics: Arc::clone(&self.metrics),
            config: self.config.clone(),
        }
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
    async fn complete(&self, request: LLMRequest) -> AriaResult<LLMResponse>;
    
    /// Stream a request
    async fn complete_stream(&self, request: LLMRequest) -> AriaResult<Box<dyn futures::Stream<Item = AriaResult<LLMResponse>> + Unpin + Send>>;
    
    /// Health check
    async fn health_check(&self) -> AriaResult<bool>;
    
    /// Clone the provider (for Arc storage)
    fn clone_box(&self) -> Box<dyn LLMProvider>;
}
