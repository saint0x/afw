// Container Database Operations
// Simple database operations for container management

use crate::errors::{AriaError, AriaResult, ErrorCategory, ErrorCode, ErrorSeverity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Container record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerRecord {
    pub container_id: String,
    pub user_id: String,
    pub session_id: Option<String>,
    pub name: Option<String>,
    pub image_path: String,
    pub status: String,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub stopped_at: Option<u64>,
}

/// Database operations for containers
pub struct ContainerOps;

impl ContainerOps {
    /// Create a container record
    pub async fn create_container(
        pool: &sqlx::SqlitePool,
        container_id: &str,
        user_id: &str,
        session_id: Option<String>,
        image_path: &str,
        command: Vec<String>,
        environment: HashMap<String, String>,
    ) -> AriaResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let command_json = serde_json::to_string(&command)
            .map_err(|e| AriaError::new(
                ErrorCode::SerializationError,
                ErrorCategory::System,
                ErrorSeverity::Medium,
                &format!("Failed to serialize command: {}", e)
            ))?;

        let environment_json = serde_json::to_string(&environment)
            .map_err(|e| AriaError::new(
                ErrorCode::SerializationError,
                ErrorCategory::System,
                ErrorSeverity::Medium,
                &format!("Failed to serialize environment: {}", e)
            ))?;

        sqlx::query(r#"
            INSERT INTO containers (
                container_id, user_id, session_id, image_path, command, 
                environment, status, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, 'created', ?)
        "#)
        .bind(container_id)
        .bind(user_id)
        .bind(&session_id)
        .bind(image_path)
        .bind(command_json)
        .bind(environment_json)
        .bind(now as i64)
        .execute(pool)
        .await
        .map_err(|e| AriaError::new(
            ErrorCode::DatabaseError,
            ErrorCategory::System,
            ErrorSeverity::High,
            &format!("Failed to create container record: {}", e)
        ))?;

        Ok(())
    }

    /// Update container status
    pub async fn update_container_status(
        pool: &sqlx::SqlitePool,
        container_id: &str,
        status: &str,
    ) -> AriaResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let (started_at, stopped_at) = match status {
            "running" => (Some(now as i64), None),
            "stopped" | "failed" => (None, Some(now as i64)),
            _ => (None, None),
        };

        sqlx::query(r#"
            UPDATE containers SET 
                status = ?, 
                started_at = COALESCE(started_at, ?),
                stopped_at = COALESCE(stopped_at, ?)
            WHERE container_id = ?
        "#)
        .bind(status)
        .bind(started_at)
        .bind(stopped_at)
        .bind(container_id)
        .execute(pool)
        .await
        .map_err(|e| AriaError::new(
            ErrorCode::DatabaseError,
            ErrorCategory::System,
            ErrorSeverity::High,
            &format!("Failed to update container status: {}", e)
        ))?;

        Ok(())
    }
} 