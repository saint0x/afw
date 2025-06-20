use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::{wrappers::ReceiverStream, Stream};
use tonic::{Request, Response, Status};
use uuid::Uuid;
use sqlx::Row;

use super::aria::{
    session_service_server::SessionService,
    Session, CreateSessionRequest, GetSessionRequest,
    ExecuteTurnRequest, TurnOutput, Message, ToolCall, ToolResult,
    MessageRole,
};

use crate::database::DatabaseManager;
use crate::engines::intelligence::IntelligenceEngine;
use crate::engines::tool_registry::{ToolRegistry, ToolRegistryInterface};
use crate::errors::{AriaError, AriaResult};

/// Implementation of the high-level SessionService
pub struct SessionServiceImpl {
    database: Arc<DatabaseManager>,
    intelligence: Arc<IntelligenceEngine>,
    tool_registry: Arc<ToolRegistry>,
}

impl SessionServiceImpl {
    pub fn new(
        database: Arc<DatabaseManager>,
        intelligence: Arc<IntelligenceEngine>, 
        tool_registry: Arc<ToolRegistry>
    ) -> Self {
        Self { 
            database,
            intelligence,
            tool_registry,
        }
    }

    /// Generate a unique session ID
    fn generate_session_id() -> String {
        Uuid::new_v4().to_string()
    }

    /// Store session in database
    async fn store_session(&self, session: &Session) -> AriaResult<()> {
        let query = r#"
            INSERT INTO sessions (id, user_id, created_at, status, context_data)
            VALUES (?, ?, ?, ?, ?)
        "#;
        
        let context_json = serde_json::to_string(&session.context_data)
            .map_err(|e| AriaError::database_error(&format!("Failed to serialize context: {}", e)))?;
        
        let created_at = session.created_at.as_ref()
            .map(|ts| ts.seconds)
            .unwrap_or_else(|| chrono::Utc::now().timestamp());
        
        let pool = self.database.pool().await?;
        sqlx::query(query)
            .bind(&session.id)
            .bind(&session.user_id)
            .bind(created_at)
            .bind(&session.status)
            .bind(context_json)
            .execute(&pool)
            .await
            .map_err(|e| AriaError::database_error(&format!("Failed to store session: {}", e)))?;
        
        Ok(())
    }

    /// Retrieve session from database
    async fn get_session_from_db(&self, session_id: &str) -> AriaResult<Session> {
        let query = r#"
            SELECT id, user_id, created_at, status, context_data
            FROM sessions 
            WHERE id = ?
        "#;
        
        let pool = self.database.pool().await?;
        let row = sqlx::query(query)
            .bind(session_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| AriaError::database_error(&format!("Failed to query session: {}", e)))?
            .ok_or_else(|| AriaError::not_found(&format!("Session not found: {}", session_id)))?;
        
        let context_json: String = row.get("context_data");
        let context_data: std::collections::HashMap<String, String> = 
            serde_json::from_str(&context_json)
                .map_err(|e| AriaError::database_error(&format!("Failed to deserialize context: {}", e)))?;
        
        let created_at: i64 = row.get("created_at");
        
        Ok(Session {
            id: row.get("id"),
            user_id: row.get("user_id"),
            created_at: Some(prost_types::Timestamp {
                seconds: created_at,
                nanos: 0,
            }),
            context_data,
            status: row.get("status"),
        })
    }

    /// Store a message in the database
    async fn store_message(&self, session_id: &str, message: &Message) -> AriaResult<()> {
        let query = r#"
            INSERT INTO messages (id, session_id, role, content, created_at)
            VALUES (?, ?, ?, ?, ?)
        "#;
        
        let created_at = message.created_at.as_ref()
            .map(|ts| ts.seconds)
            .unwrap_or_else(|| chrono::Utc::now().timestamp());
        
        let role_str = match MessageRole::try_from(message.role).unwrap_or(MessageRole::Unspecified) {
            MessageRole::System => "system",
            MessageRole::User => "user", 
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
            MessageRole::Unspecified => "unspecified",
        };
        
        let pool = self.database.pool().await?;
        sqlx::query(query)
            .bind(&message.id)
            .bind(session_id)
            .bind(role_str)
            .bind(&message.content)
            .bind(created_at)
            .execute(&pool)
            .await
            .map_err(|e| AriaError::database_error(&format!("Failed to store message: {}", e)))?;
        
        Ok(())
    }

    /// Execute a tool call and return the result
    async fn execute_tool(&self, tool_call: &ToolCall) -> ToolResult {
        tracing::info!("Executing tool: {}", tool_call.tool_name);
        
        // Parse tool parameters
        let parameters: serde_json::Value = match serde_json::from_str(&tool_call.parameters_json) {
            Ok(params) => params,
            Err(e) => {
                tracing::error!("Failed to parse tool parameters: {}", e);
                return ToolResult {
                    tool_name: tool_call.tool_name.clone(),
                    result_json: "{}".to_string(),
                    success: false,
                    error_message: Some(format!("Invalid parameters: {}", e)),
                };
            }
        };
        
        // Execute the tool via the tool registry
        match self.tool_registry.execute_tool(&tool_call.tool_name, crate::deep_size::DeepValue(parameters)).await {
            Ok(result) => {
                let result_json = serde_json::to_string(&result)
                    .unwrap_or_else(|_| "{}".to_string());
                
                ToolResult {
                    tool_name: tool_call.tool_name.clone(),
                    result_json,
                    success: true,
                    error_message: None,
                }
            }
            Err(e) => {
                tracing::error!("Tool execution failed: {}", e);
                ToolResult {
                    tool_name: tool_call.tool_name.clone(),
                    result_json: "{}".to_string(),
                    success: false,
                    error_message: Some(e.to_string()),
                }
            }
        }
    }
}

#[tonic::async_trait]
impl SessionService for SessionServiceImpl {
    async fn create_session(
        &self,
        request: Request<CreateSessionRequest>,
    ) -> Result<Response<Session>, Status> {
        let _req = request.into_inner();
        
        let session_id = Self::generate_session_id();
        let now = chrono::Utc::now().timestamp();
        
        tracing::info!("Creating new session: {}", session_id);
        
        let session = Session {
            id: session_id,
            user_id: "default".to_string(), // TODO: Extract from authentication context
            created_at: Some(prost_types::Timestamp {
                seconds: now,
                nanos: 0,
            }),
            context_data: std::collections::HashMap::new(),
            status: "active".to_string(),
        };
        
        // Store in database
        if let Err(e) = self.store_session(&session).await {
            tracing::error!("Failed to store session: {}", e);
            return Err(Status::internal("Failed to create session"));
        }
        
        tracing::info!("Session created successfully: {}", session.id);
        Ok(Response::new(session))
    }

    async fn get_session(
        &self,
        request: Request<GetSessionRequest>,
    ) -> Result<Response<Session>, Status> {
        let req = request.into_inner();
        
        tracing::info!("Getting session: {}", req.session_id);
        
        match self.get_session_from_db(&req.session_id).await {
            Ok(session) => Ok(Response::new(session)),
            Err(e) => {
                tracing::error!("Failed to get session: {}", e);
                Err(Status::not_found(format!("Session not found: {}", e)))
            }
        }
    }

    async fn execute_turn(
        &self,
        request: Request<ExecuteTurnRequest>,
    ) -> Result<Response<Self::ExecuteTurnStream>, Status> {
        let req = request.into_inner();
        
        tracing::info!("Executing turn for session: {}", req.session_id);
        
        // Verify session exists
        let session = match self.get_session_from_db(&req.session_id).await {
            Ok(session) => session,
            Err(e) => {
                tracing::error!("Session not found: {}", e);
                return Err(Status::not_found("Session not found"));
            }
        };
        
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        
        // Clone necessary data for the async task
        let session_id = req.session_id.clone();
        let user_input = req.input.clone();
        let database = Arc::clone(&self.database);
        let intelligence = Arc::clone(&self.intelligence);
        let tool_registry = Arc::clone(&self.tool_registry);
        let service_self = Self {
            database: database.clone(),
            intelligence: intelligence.clone(),
            tool_registry: tool_registry.clone(),
        };
        
        // Spawn the conversation execution task
        tokio::spawn(async move {
            // Store the user message
            let user_message = Message {
                id: Uuid::new_v4().to_string(),
                role: MessageRole::User as i32,
                content: user_input.clone(),
                created_at: Some(prost_types::Timestamp {
                    seconds: chrono::Utc::now().timestamp(),
                    nanos: 0,
                }),
            };
            
            // Send user message to stream
            let user_turn = TurnOutput {
                event: Some(super::aria::turn_output::Event::Message(user_message.clone())),
            };
            
            if tx.send(Ok(user_turn)).await.is_err() {
                return; // Client disconnected
            }
            
            // Store user message in database
            if let Err(e) = service_self.store_message(&session_id, &user_message).await {
                tracing::error!("Failed to store user message: {}", e);
            }
            
            // Process the input with the intelligence engine
            // For now, we'll simulate a simple conversation flow
            
            // Simulate thinking/processing
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            
            // Check if we need to use tools
            let needs_tool = user_input.to_lowercase().contains("weather") || 
                           user_input.to_lowercase().contains("search") ||
                           user_input.to_lowercase().contains("calculate");
            
            if needs_tool {
                // Simulate tool call
                let tool_call = ToolCall {
                    tool_name: if user_input.to_lowercase().contains("weather") {
                        "get_weather".to_string()
                    } else if user_input.to_lowercase().contains("search") {
                        "web_search".to_string()
                    } else {
                        "calculator".to_string()
                    },
                    parameters_json: serde_json::json!({
                        "query": user_input
                    }).to_string(),
                };
                
                // Send tool call to stream
                let tool_call_turn = TurnOutput {
                    event: Some(super::aria::turn_output::Event::ToolCall(tool_call.clone())),
                };
                
                if tx.send(Ok(tool_call_turn)).await.is_err() {
                    return; // Client disconnected
                }
                
                // Execute the tool
                let tool_result = service_self.execute_tool(&tool_call).await;
                
                // Send tool result to stream
                let tool_result_turn = TurnOutput {
                    event: Some(super::aria::turn_output::Event::ToolResult(tool_result)),
                };
                
                if tx.send(Ok(tool_result_turn)).await.is_err() {
                    return; // Client disconnected
                }
            }
            
            // Generate final response
            let response_content = if needs_tool {
                format!("I've processed your request about '{}' using the appropriate tools. Based on the results, here's what I found...", user_input)
            } else {
                format!("I understand you're asking about '{}'. Let me help you with that.", user_input)
            };
            
            // Send final response
            let final_response_turn = TurnOutput {
                event: Some(super::aria::turn_output::Event::FinalResponse(response_content.clone())),
            };
            
            if tx.send(Ok(final_response_turn)).await.is_err() {
                return; // Client disconnected
            }
            
            // Store assistant message in database
            let assistant_message = Message {
                id: Uuid::new_v4().to_string(),
                role: MessageRole::Assistant as i32,
                content: response_content,
                created_at: Some(prost_types::Timestamp {
                    seconds: chrono::Utc::now().timestamp(),
                    nanos: 0,
                }),
            };
            
            if let Err(e) = service_self.store_message(&session_id, &assistant_message).await {
                tracing::error!("Failed to store assistant message: {}", e);
            }
            
            tracing::info!("Turn execution completed for session: {}", session_id);
        });
        
        let stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream) as Self::ExecuteTurnStream))
    }

    type ExecuteTurnStream = Pin<Box<dyn Stream<Item = Result<TurnOutput, Status>> + Send>>;
} 