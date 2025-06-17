// ICC (Inter-Container Communication) Engine
// HTTP server for container → runtime communication over bridge network

use crate::errors::{AriaError, AriaResult, ErrorCategory, ErrorCode, ErrorSeverity};
use crate::types::{AgentConfig, RuntimeContext, ToolResult};
use crate::engines::{Engine, ICCToolHandler, ICCAgentHandler, ICCServerStatus, ICCConnection};
use crate::engines::tool_registry::{ToolRegistry, ToolRegistryInterface};
use crate::engines::llm::{LLMHandler, types::{LLMRequest, LLMResponse}};
use crate::deep_size::DeepUuid;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;

/// Session token for container authentication
#[derive(Debug, Clone)]
pub struct SessionToken {
    pub token: String,
    pub session_id: Uuid,
    pub container_id: Option<String>,
    pub created_at: u64,
    pub expires_at: u64,
    pub permissions: Vec<String>,
}

/// Request context for ICC operations
#[derive(Debug, Clone)]
pub struct ICCRequestContext {
    pub session_id: Uuid,
    pub container_id: String,
    pub request_id: String,
    pub authenticated: bool,
    pub permissions: Vec<String>,
}

/// Tool execution request from container
#[derive(Debug, Deserialize)]
pub struct ToolExecutionRequest {
    pub tool_name: String,
    pub parameters: serde_json::Value,
    pub timeout_seconds: Option<u64>,
    pub capture_output: Option<bool>,
}

/// Tool execution response to container
#[derive(Debug, Serialize)]
pub struct ToolExecutionResponse {
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub execution_time_ms: u64,
}

/// Agent invocation request from container
#[derive(Debug, Deserialize)]
pub struct AgentInvocationRequest {
    pub agent_name: String,
    pub message: String,
    pub context: Option<serde_json::Value>,
    pub max_turns: Option<u32>,
}

/// Agent invocation response to container
#[derive(Debug, Serialize)]
pub struct AgentInvocationResponse {
    pub success: bool,
    pub response: Option<String>,
    pub conversation_id: Option<String>,
    pub error_message: Option<String>,
    pub turn_count: u32,
}

/// LLM completion request from container
#[derive(Debug, Deserialize)]
pub struct LLMCompletionRequest {
    pub messages: Vec<LLMMessage>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
}

/// LLM message for ICC requests
#[derive(Debug, Deserialize, Serialize)]
pub struct LLMMessage {
    pub role: String,
    pub content: String,
}

/// Context access request from container
#[derive(Debug, Deserialize)]
pub struct ContextAccessRequest {
    pub filter: Option<String>,
    pub limit: Option<u32>,
    pub include_history: Option<bool>,
    pub include_memory: Option<bool>,
}

/// Context access response to container
#[derive(Debug, Serialize)]
pub struct ContextAccessResponse {
    pub session_id: String,
    pub execution_history: Option<Vec<serde_json::Value>>,
    pub working_memory: Option<serde_json::Value>,
    pub current_step: Option<serde_json::Value>,
    pub agent_config: Option<serde_json::Value>,
}

/// ICC Engine for container communication
pub struct ICCEngine {
    /// Bridge IP address to bind server to
    bridge_ip: String,
    /// Port to bind server to
    port: u16,
    /// Current server status
    status: Arc<RwLock<ICCServerStatus>>,
    /// Active session tokens
    session_tokens: Arc<RwLock<HashMap<String, SessionToken>>>,
    /// Active connections
    connections: Arc<RwLock<HashMap<String, ICCConnection>>>,
    /// Tool registry for tool execution
    tool_registry: Arc<ToolRegistry>,
    /// LLM handler for LLM proxy
    llm_handler: Arc<LLMHandler>,
    /// Server shutdown signal
    shutdown_signal: Arc<tokio::sync::Notify>,
}

impl ICCEngine {
    /// Create new ICC engine
    pub fn new(
        bridge_ip: String,
        port: u16,
        tool_registry: Arc<ToolRegistry>,
        llm_handler: Arc<LLMHandler>,
    ) -> Self {
        Self {
            bridge_ip,
            port,
            status: Arc::new(RwLock::new(ICCServerStatus::Stopped)),
            session_tokens: Arc::new(RwLock::new(HashMap::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
            tool_registry,
            llm_handler,
            shutdown_signal: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Start the ICC HTTP server
    pub async fn start_server(&self) -> AriaResult<()> {
        {
            let mut status = self.status.write().unwrap();
            *status = ICCServerStatus::Starting;
        }

        let bind_addr = format!("{}:{}", self.bridge_ip, self.port);
        
        tracing::info!("Starting ICC server on {}", bind_addr);
        
        // Create HTTP router with ICC endpoints
        let app = self.create_http_router();
        
        // Bind and serve
        let listener = tokio::net::TcpListener::bind(&bind_addr).await
            .map_err(|e| AriaError::new(
                ErrorCode::NetworkError,
                ErrorCategory::System,
                ErrorSeverity::Critical,
                &format!("Failed to bind ICC server to {}: {}", bind_addr, e)
            ))?;

        {
            let mut status = self.status.write().unwrap();
            *status = ICCServerStatus::Running;
        }

        tracing::info!("ICC server started successfully on {}", bind_addr);
        
        // Clone shutdown signal for the closure
        let shutdown_signal = self.shutdown_signal.clone();
        
        // Serve with graceful shutdown
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                shutdown_signal.notified().await;
            })
            .await
            .map_err(|e| AriaError::new(
                ErrorCode::NetworkError,
                ErrorCategory::System,
                ErrorSeverity::Critical,
                &format!("ICC server error: {}", e)
            ))?;

        tracing::info!("ICC server shutdown complete");
        Ok(())
    }

    /// Stop the ICC server
    pub async fn stop_server(&self) -> AriaResult<()> {
        {
            let mut status = self.status.write().unwrap();
            *status = ICCServerStatus::Stopping;
        }

        // Signal shutdown
        self.shutdown_signal.notify_waiters();

        // Clear active connections
        {
            let mut connections = self.connections.write().unwrap();
            connections.clear();
        }

        {
            let mut status = self.status.write().unwrap();
            *status = ICCServerStatus::Stopped;
        }

        tracing::info!("ICC server stopped");
        Ok(())
    }

    /// Generate session token for container authentication
    pub fn generate_session_token(
        &self,
        session_id: Uuid,
        container_id: Option<String>,
        permissions: Vec<String>,
        ttl_seconds: u64,
    ) -> AriaResult<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let token = format!("aria_{}_{}", session_id, uuid::Uuid::new_v4());
        
        let session_token = SessionToken {
            token: token.clone(),
            session_id,
            container_id,
            created_at: now,
            expires_at: now + ttl_seconds,
            permissions,
        };

        {
            let mut tokens = self.session_tokens.write().unwrap();
            tokens.insert(token.clone(), session_token);
        }

        Ok(token)
    }

    /// Validate session token and extract request context
    pub fn validate_token(&self, token: &str) -> AriaResult<ICCRequestContext> {
        let tokens = self.session_tokens.read().unwrap();
        
        let session_token = tokens.get(token)
            .ok_or_else(|| AriaError::new(
                ErrorCode::AuthenticationFailed,
                ErrorCategory::Security,
                ErrorSeverity::High,
                "Invalid session token"
            ))?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if now > session_token.expires_at {
            return Err(AriaError::new(
                ErrorCode::AuthenticationFailed,
                ErrorCategory::Security,
                ErrorSeverity::High,
                "Session token expired"
            ));
        }

        Ok(ICCRequestContext {
            session_id: session_token.session_id,
            container_id: session_token.container_id.clone().unwrap_or_default(),
            request_id: uuid::Uuid::new_v4().to_string(),
            authenticated: true,
            permissions: session_token.permissions.clone(),
        })
    }

    /// Execute tool via ICC
    pub async fn execute_tool_icc(
        &self,
        request: ToolExecutionRequest,
        context: ICCRequestContext,
    ) -> AriaResult<ToolExecutionResponse> {
        let start_time = std::time::Instant::now();

        // Check permissions
        if !context.permissions.contains(&"tools".to_string()) && 
           !context.permissions.contains(&"all".to_string()) {
            return Ok(ToolExecutionResponse {
                success: false,
                result: None,
                error_message: Some("Insufficient permissions for tool execution".to_string()),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
            });
        }

        // Execute tool via tool registry
        match self.tool_registry.execute_tool(
            &request.tool_name,
            request.parameters.into(),
        ).await {
            Ok(result) => {
                // Convert ToolResult to JSON
                let result_json = serde_json::to_value(&result)
                    .map_err(|e| AriaError::new(
                        ErrorCode::SerializationError,
                        ErrorCategory::System,
                        ErrorSeverity::Medium,
                        &format!("Failed to serialize tool result: {}", e)
                    ))?;

                Ok(ToolExecutionResponse {
                    success: true,
                    result: Some(result_json),
                    error_message: None,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                })
            },
            Err(e) => Ok(ToolExecutionResponse {
                success: false,
                result: None,
                error_message: Some(e.message),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
            })
        }
    }

    /// Handle LLM completion via ICC
    pub async fn handle_llm_completion(
        &self,
        request: LLMCompletionRequest,
        context: ICCRequestContext,
    ) -> AriaResult<serde_json::Value> {
        // Check permissions
        if !context.permissions.contains(&"llm".to_string()) && 
           !context.permissions.contains(&"all".to_string()) {
            return Err(AriaError::new(
                ErrorCode::PermissionDenied,
                ErrorCategory::Security,
                ErrorSeverity::High,
                "Insufficient permissions for LLM access"
            ));
        }

        // Convert ICC messages to LLM request format
        let llm_messages: Vec<crate::engines::llm::types::LLMMessage> = request.messages
            .into_iter()
            .map(|msg| crate::engines::llm::types::LLMMessage {
                role: msg.role,
                content: msg.content,
                tool_calls: None,
                tool_call_id: None,
            })
            .collect();

        let llm_request = LLMRequest {
            messages: llm_messages,
            config: crate::engines::llm::types::LLMConfig {
                model: Some(request.model.unwrap_or_else(|| "gpt-4o".to_string())),
                temperature: request.temperature.unwrap_or(0.7),
                max_tokens: request.max_tokens.unwrap_or(2048),
                top_p: None,
                frequency_penalty: None,
                presence_penalty: None,
            },
            provider: None,
            tools: None,
            tool_choice: None,
            stream: Some(request.stream.unwrap_or(false)),
        };

        // Execute LLM request
        match self.llm_handler.complete(llm_request).await {
            Ok(response) => {
                serde_json::to_value(&response).map_err(|e| AriaError::new(
                    ErrorCode::SerializationError,
                    ErrorCategory::System,
                    ErrorSeverity::Medium,
                    &format!("Failed to serialize LLM response: {}", e)
                ))
            },
            Err(e) => Err(e)
        }
    }

    /// Handle context access via ICC (internal method)
    async fn handle_context_access_internal(
        &self,
        request: ContextAccessRequest,
        context: ICCRequestContext,
    ) -> AriaResult<ContextAccessResponse> {
        // Check permissions
        if !context.permissions.contains(&"context".to_string()) && 
           !context.permissions.contains(&"all".to_string()) {
            return Err(AriaError::new(
                ErrorCode::PermissionDenied,
                ErrorCategory::Security,
                ErrorSeverity::High,
                "Insufficient permissions for context access"
            ));
        }

        // TODO: Implement actual context retrieval from context manager
        // For now, return basic structure
        Ok(ContextAccessResponse {
            session_id: context.session_id.to_string(),
            execution_history: if request.include_history.unwrap_or(false) {
                Some(vec![])
            } else { None },
            working_memory: if request.include_memory.unwrap_or(false) {
                Some(serde_json::Value::Object(serde_json::Map::new()))
            } else { None },
            current_step: None,
            agent_config: None,
        })
    }

    /// Get current server status
    pub fn get_status(&self) -> ICCServerStatus {
        let status = self.status.read().unwrap();
        match &*status {
            ICCServerStatus::Starting => ICCServerStatus::Starting,
            ICCServerStatus::Running => ICCServerStatus::Running,
            ICCServerStatus::Stopping => ICCServerStatus::Stopping,
            ICCServerStatus::Stopped => ICCServerStatus::Stopped,
            ICCServerStatus::Error(e) => ICCServerStatus::Error(e.clone()),
        }
    }

    /// Get active connections count
    pub fn get_connections_count(&self) -> usize {
        let connections = self.connections.read().unwrap();
        connections.len()
    }

    /// Cleanup expired tokens
    pub async fn cleanup_expired_tokens(&self) -> AriaResult<usize> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut tokens = self.session_tokens.write().unwrap();
        let initial_count = tokens.len();
        
        tokens.retain(|_, token| token.expires_at > now);
        
        let cleaned_count = initial_count - tokens.len();
        
        if cleaned_count > 0 {
            tracing::debug!("Cleaned {} expired session tokens", cleaned_count);
        }
        
        Ok(cleaned_count)
    }

    /// Create environment variables for container with ICC access
    pub fn create_icc_environment(
        &self,
        session_id: Uuid,
        container_id: String,
        permissions: Vec<String>,
    ) -> AriaResult<HashMap<String, String>> {
        let token = self.generate_session_token(
            session_id,
            Some(container_id.clone()),
            permissions,
            3600, // 1 hour TTL
        )?;

        let mut env = HashMap::new();
        
        // ICC server endpoint
        env.insert("ARIA_ICC_ENDPOINT".to_string(), 
                  format!("http://{}:{}", self.bridge_ip, self.port));
        
        // Authentication token
        env.insert("ARIA_SESSION_TOKEN".to_string(), token);
        
        // Session context
        env.insert("ARIA_SESSION_ID".to_string(), session_id.to_string());
        env.insert("ARIA_CONTAINER_ID".to_string(), container_id);
        
        // API endpoints
        env.insert("ARIA_TOOLS_URL".to_string(), 
                  format!("http://{}:{}/tools", self.bridge_ip, self.port));
        env.insert("ARIA_AGENTS_URL".to_string(), 
                  format!("http://{}:{}/agents", self.bridge_ip, self.port));
        env.insert("ARIA_LLM_URL".to_string(), 
                  format!("http://{}:{}/llm/complete", self.bridge_ip, self.port));
        env.insert("ARIA_CONTEXT_URL".to_string(), 
                  format!("http://{}:{}/context", self.bridge_ip, self.port));

        Ok(env)
    }

    /// Create HTTP router with all ICC endpoints
    fn create_http_router(&self) -> Router {
        // Create shared state for handlers
        let engine = Arc::new(self.clone());
        
        Router::new()
            // Tool execution endpoint
            .route("/tools/:tool_name", post(Self::handle_tool_execution))
            // Agent invocation endpoint
            .route("/agents/:agent_name", post(Self::handle_agent_invocation))
            // LLM completion endpoint
            .route("/llm/complete", post(Self::handle_llm_proxy))
            // Context access endpoint
            .route("/context", get(handle_context_access))
            // Context suggestion endpoint
            .route("/context/add", post(Self::handle_context_suggestion))
            // Health check endpoint
            .route("/health", get(Self::handle_health_check))
            // Server status endpoint
            .route("/status", get(Self::handle_server_status))
            // Add CORS layer for browser requests
            .layer(CorsLayer::permissive())
            // Add shared state
            .with_state(engine)
    }

    /// HTTP handler for tool execution
    async fn handle_tool_execution(
        Path(tool_name): Path<String>,
        State(engine): State<Arc<ICCEngine>>,
        headers: HeaderMap,
        Json(mut request): Json<ToolExecutionRequest>,
    ) -> Result<Json<ToolExecutionResponse>, StatusCode> {
        // Extract and validate auth token
        let token = extract_auth_token(&headers)?;
        let context = engine.validate_token(&token).map_err(|_| StatusCode::UNAUTHORIZED)?;
        
        // Set tool name from path
        request.tool_name = tool_name;
        
        // Execute tool
        match engine.execute_tool_icc(request, context).await {
            Ok(response) => Ok(Json(response)),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    /// HTTP handler for agent invocation
    async fn handle_agent_invocation(
        Path(agent_name): Path<String>,
        State(engine): State<Arc<ICCEngine>>,
        headers: HeaderMap,
        Json(request): Json<AgentInvocationRequest>,
    ) -> Result<Json<AgentInvocationResponse>, StatusCode> {
        // Extract and validate auth token
        let token = extract_auth_token(&headers)?;
        let context = engine.validate_token(&token).map_err(|_| StatusCode::UNAUTHORIZED)?;
        
        // TODO: Implement agent invocation
        let response = AgentInvocationResponse {
            success: true,
            response: Some(format!("Agent '{}' invoked with message: {}", agent_name, request.message)),
            conversation_id: Some(uuid::Uuid::new_v4().to_string()),
            error_message: None,
            turn_count: 1,
        };
        
        Ok(Json(response))
    }

    /// HTTP handler for LLM proxy
    async fn handle_llm_proxy(
        State(engine): State<Arc<ICCEngine>>,
        headers: HeaderMap,
        Json(request): Json<LLMCompletionRequest>,
    ) -> Result<Json<serde_json::Value>, StatusCode> {
        // Extract and validate auth token
        let token = extract_auth_token(&headers)?;
        let context = engine.validate_token(&token).map_err(|_| StatusCode::UNAUTHORIZED)?;
        
        // Handle LLM completion
        match engine.handle_llm_completion(request, context).await {
            Ok(response) => Ok(Json(response)),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }



    /// HTTP handler for context suggestions
    async fn handle_context_suggestion(
        State(engine): State<Arc<ICCEngine>>,
        headers: HeaderMap,
        Json(suggestion): Json<serde_json::Value>,
    ) -> Result<Json<serde_json::Value>, StatusCode> {
        // Extract and validate auth token
        let token = extract_auth_token(&headers)?;
        let _context = engine.validate_token(&token).map_err(|_| StatusCode::UNAUTHORIZED)?;
        
        // TODO: Implement context suggestion handling
        let response = serde_json::json!({
            "success": true,
            "message": "Context suggestion received",
            "suggestion_id": uuid::Uuid::new_v4().to_string()
        });
        
        Ok(Json(response))
    }

    /// HTTP handler for health check
    async fn handle_health_check(
        State(engine): State<Arc<ICCEngine>>,
    ) -> Json<serde_json::Value> {
        let healthy = engine.health_check();
        Json(serde_json::json!({
            "status": if healthy { "healthy" } else { "unhealthy" },
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "connections": engine.get_connections_count()
        }))
    }

    /// HTTP handler for server status
    async fn handle_server_status(
        State(engine): State<Arc<ICCEngine>>,
    ) -> Json<serde_json::Value> {
        let status = engine.get_status();
        Json(serde_json::json!({
            "status": format!("{:?}", status),
            "connections": engine.get_connections_count(),
            "uptime": "TODO", // TODO: Add uptime tracking
            "version": env!("CARGO_PKG_VERSION")
        }))
    }
}

/// Clone implementation for ICCEngine (needed for Arc<ICCEngine> in handlers)
impl Clone for ICCEngine {
    fn clone(&self) -> Self {
        Self {
            bridge_ip: self.bridge_ip.clone(),
            port: self.port,
            status: self.status.clone(),
            session_tokens: self.session_tokens.clone(),
            connections: self.connections.clone(),
            tool_registry: self.tool_registry.clone(),
            llm_handler: self.llm_handler.clone(),
            shutdown_signal: self.shutdown_signal.clone(),
        }
    }
}

/// HTTP handler for context access
async fn handle_context_access(
    State(engine): State<Arc<ICCEngine>>,
    headers: HeaderMap,
    Query(params): Query<ContextAccessRequest>,
) -> Result<Json<ContextAccessResponse>, StatusCode> {
    // Extract and validate auth token
    let token = extract_auth_token(&headers)?;
    let context = engine.validate_token(&token).map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    // Handle context access
    match engine.handle_context_access_internal(params, context).await {
        Ok(response) => Ok(Json(response)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Extract authentication token from HTTP headers
fn extract_auth_token(headers: &HeaderMap) -> Result<String, StatusCode> {
    let auth_header = headers
        .get("authorization")
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    if let Some(token) = auth_header.strip_prefix("Bearer ") {
        Ok(token.to_string())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

// Implement Engine trait for ICC Engine
impl Engine for ICCEngine {
    fn get_state(&self) -> String {
        let status = self.get_status();
        let connections = self.get_connections_count();
        format!("ICC Engine - Status: {:?}, Connections: {}", status, connections)
    }

    fn get_dependencies(&self) -> Vec<String> {
        vec![
            "tool_registry".to_string(),
            "llm_handler".to_string(),
            "network_bridge".to_string(),
        ]
    }

    fn health_check(&self) -> bool {
        matches!(self.get_status(), ICCServerStatus::Running)
    }

    fn initialize(&self) -> bool {
        // ICC engine initialization happens via start_server()
        true
    }

    fn shutdown(&self) -> bool {
        // ICC engine shutdown happens via stop_server()
        true
    }
}

// Implement tool handler for ICC
#[async_trait::async_trait]
impl ICCToolHandler for ICCEngine {
    async fn handle_tool_call(
        &self,
        tool_name: &str,
        parameters: serde_json::Value,
        container_id: &str,
        session_id: Uuid,
    ) -> AriaResult<serde_json::Value> {
        let request = ToolExecutionRequest {
            tool_name: tool_name.to_string(),
            parameters,
            timeout_seconds: Some(300), // 5 minute default timeout
            capture_output: Some(true),
        };

        let context = ICCRequestContext {
            session_id,
            container_id: container_id.to_string(),
            request_id: uuid::Uuid::new_v4().to_string(),
            authenticated: true,
            permissions: vec!["tools".to_string()],
        };

        let response = self.execute_tool_icc(request, context).await?;
        
        if response.success {
            Ok(response.result.unwrap_or(serde_json::Value::Null))
        } else {
            Err(AriaError::new(
                ErrorCode::ToolExecutionFailed,
                ErrorCategory::Tool,
                ErrorSeverity::Medium,
                &response.error_message.unwrap_or_else(|| "Tool execution failed".to_string())
            ))
        }
    }
}

// Implement agent handler for ICC
#[async_trait::async_trait]
impl ICCAgentHandler for ICCEngine {
    async fn handle_agent_call(
        &self,
        agent_name: &str,
        message: &str,
        container_id: &str,
        session_id: Uuid,
    ) -> AriaResult<String> {
        // TODO: Implement agent invocation via agent registry
        // For now, return a placeholder response
        Ok(format!(
            "Agent '{}' received message from container {}: {}",
            agent_name, container_id, message
        ))
    }
}
