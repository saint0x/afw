// Session Database Operations
// Simple database operations for session management

use crate::errors::{AriaError, AriaResult, ErrorCategory, ErrorCode, ErrorSeverity};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Session record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_id: String,
    pub user_id: String,
    pub agent_config_id: Option<String>,
    pub created_at: u64,
    pub ended_at: Option<u64>,
    pub session_type: String,
    pub status: String,
    pub total_tool_calls: u32,
    pub total_tokens_used: u32,
}

/// Database operations for sessions
pub struct SessionOps;

impl SessionOps {
    /// Create a new session
    pub async fn create_session(
        pool: &sqlx::SqlitePool,
        user_id: &str,
        session_type: &str,
        agent_config_id: Option<String>,
    ) -> AriaResult<String> {
        let session_id = Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        sqlx::query(r#"
            INSERT INTO sessions (session_id, user_id, agent_config_id, created_at, session_type, status)
            VALUES (?, ?, ?, ?, ?, 'active')
        "#)
        .bind(&session_id)
        .bind(user_id)
        .bind(&agent_config_id)
        .bind(now as i64)
        .bind(session_type)
        .execute(pool)
        .await
        .map_err(|e| AriaError::new(
            ErrorCode::DatabaseError,
            ErrorCategory::System,
            ErrorSeverity::High,
            &format!("Failed to create session: {}", e)
        ))?;

        Ok(session_id)
    }

    /// End a session
    pub async fn end_session(pool: &sqlx::SqlitePool, session_id: &str) -> AriaResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        sqlx::query(r#"
            UPDATE sessions SET ended_at = ?, status = 'completed' WHERE session_id = ?
        "#)
        .bind(now as i64)
        .bind(session_id)
        .execute(pool)
        .await
        .map_err(|e| AriaError::new(
            ErrorCode::DatabaseError,
            ErrorCategory::System,
            ErrorSeverity::High,
            &format!("Failed to end session: {}", e)
        ))?;

        Ok(())
    }

    /// Get session by ID
    pub async fn get_session(pool: &sqlx::SqlitePool, session_id: &str) -> AriaResult<SessionRecord> {
        let row: (String, String, Option<String>, i64, Option<i64>, String, String, i32, i32) = 
            sqlx::query_as(r#"
                SELECT session_id, user_id, agent_config_id, created_at, ended_at,
                       session_type, status, total_tool_calls, total_tokens_used
                FROM sessions WHERE session_id = ?
            "#)
            .bind(session_id)
            .fetch_one(pool)
            .await
            .map_err(|e| AriaError::new(
                ErrorCode::DatabaseError,
                ErrorCategory::System,
                ErrorSeverity::High,
                &format!("Failed to get session: {}", e)
            ))?;

        Ok(SessionRecord {
            session_id: row.0,
            user_id: row.1,
            agent_config_id: row.2,
            created_at: row.3 as u64,
            ended_at: row.4.map(|t| t as u64),
            session_type: row.5,
            status: row.6,
            total_tool_calls: row.7 as u32,
            total_tokens_used: row.8 as u32,
        })
    }
} 