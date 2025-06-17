// Async Task Database Operations
// Simple database operations for async task management

use crate::errors::{AriaError, AriaResult, ErrorCategory, ErrorCode, ErrorSeverity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Async task status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AsyncTaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Timeout,
}

impl std::fmt::Display for AsyncTaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AsyncTaskStatus::Pending => write!(f, "pending"),
            AsyncTaskStatus::Running => write!(f, "running"),
            AsyncTaskStatus::Completed => write!(f, "completed"),
            AsyncTaskStatus::Failed => write!(f, "failed"),
            AsyncTaskStatus::Cancelled => write!(f, "cancelled"),
            AsyncTaskStatus::Timeout => write!(f, "timeout"),
        }
    }
}

/// Simple async task record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncTaskRecord {
    pub task_id: String,
    pub user_id: String,
    pub session_id: String,
    pub container_id: Option<String>,
    pub task_type: String,
    pub command: Vec<String>,
    pub environment: HashMap<String, String>,
    pub status: AsyncTaskStatus,
    pub created_at: u64,
    pub completed_at: Option<u64>,
    pub exit_code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

/// Database operations for async tasks
pub struct AsyncTaskOps;

impl AsyncTaskOps {
    /// Create a new task record
    pub async fn create_task(
        pool: &sqlx::SqlitePool,
        user_id: &str,
        session_id: &str,
        task_type: &str,
        command: Vec<String>,
        environment: HashMap<String, String>,
        container_id: Option<String>,
    ) -> AriaResult<String> {
        let task_id = Uuid::new_v4().to_string();
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
            INSERT INTO async_tasks (
                task_id, user_id, session_id, container_id, task_type,
                command, environment, status, created_at,
                progress_percent
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&task_id)
        .bind(user_id)
        .bind(session_id)
        .bind(&container_id)
        .bind(task_type)
        .bind(command_json)
        .bind(environment_json)
        .bind("pending")
        .bind(now as i64)
        .bind(0.0)
        .execute(pool)
        .await
        .map_err(|e| AriaError::new(
            ErrorCode::DatabaseError,
            ErrorCategory::System,
            ErrorSeverity::High,
            &format!("Failed to create task: {}", e)
        ))?;

        Ok(task_id)
    }

    /// Update task status
    pub async fn update_task_status(
        pool: &sqlx::SqlitePool,
        task_id: &str,
        status: AsyncTaskStatus,
        exit_code: Option<i32>,
        stdout: Option<String>,
        stderr: Option<String>,
    ) -> AriaResult<()> {
        let now = if matches!(status, AsyncTaskStatus::Completed | AsyncTaskStatus::Failed | AsyncTaskStatus::Cancelled) {
            Some(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64)
        } else {
            None
        };

        sqlx::query(r#"
            UPDATE async_tasks SET
                status = ?, completed_at = ?, exit_code = ?, stdout = ?, stderr = ?
            WHERE task_id = ?
        "#)
        .bind(status.to_string())
        .bind(now)
        .bind(exit_code)
        .bind(stdout)
        .bind(stderr)
        .bind(task_id)
        .execute(pool)
        .await
        .map_err(|e| AriaError::new(
            ErrorCode::DatabaseError,
            ErrorCategory::System,
            ErrorSeverity::High,
            &format!("Failed to update task status: {}", e)
        ))?;

        Ok(())
    }

    /// Get task by ID
    pub async fn get_task(pool: &sqlx::SqlitePool, task_id: &str) -> AriaResult<AsyncTaskRecord> {
        let row: (String, String, String, Option<String>, String, String, String, String, i64, Option<i64>, Option<i32>, Option<String>, Option<String>) = 
            sqlx::query_as(r#"
                SELECT task_id, user_id, session_id, container_id, task_type, 
                       command, environment, status, created_at, completed_at, 
                       exit_code, stdout, stderr
                FROM async_tasks WHERE task_id = ?
            "#)
            .bind(task_id)
            .fetch_one(pool)
            .await
            .map_err(|e| AriaError::new(
                ErrorCode::DatabaseError,
                ErrorCategory::System,
                ErrorSeverity::High,
                &format!("Failed to get task: {}", e)
            ))?;

        let command: Vec<String> = serde_json::from_str(&row.5)
            .map_err(|e| AriaError::new(
                ErrorCode::SerializationError,
                ErrorCategory::System,
                ErrorSeverity::Medium,
                &format!("Failed to deserialize command: {}", e)
            ))?;

        let environment: HashMap<String, String> = serde_json::from_str(&row.6)
            .map_err(|e| AriaError::new(
                ErrorCode::SerializationError,
                ErrorCategory::System,
                ErrorSeverity::Medium,
                &format!("Failed to deserialize environment: {}", e)
            ))?;

        let status = match row.7.as_str() {
            "pending" => AsyncTaskStatus::Pending,
            "running" => AsyncTaskStatus::Running,
            "completed" => AsyncTaskStatus::Completed,
            "failed" => AsyncTaskStatus::Failed,
            "cancelled" => AsyncTaskStatus::Cancelled,
            "timeout" => AsyncTaskStatus::Timeout,
            _ => AsyncTaskStatus::Failed,
        };

        Ok(AsyncTaskRecord {
            task_id: row.0,
            user_id: row.1,
            session_id: row.2,
            container_id: row.3,
            task_type: row.4,
            command,
            environment,
            status,
            created_at: row.8 as u64,
            completed_at: row.9.map(|t| t as u64),
            exit_code: row.10,
            stdout: row.11,
            stderr: row.12,
        })
    }

    /// Get tasks for a user
    pub async fn get_user_tasks(
        pool: &sqlx::SqlitePool,
        user_id: &str,
        limit: Option<u32>,
    ) -> AriaResult<Vec<AsyncTaskRecord>> {
        let limit_clause = if let Some(l) = limit {
            format!("LIMIT {}", l)
        } else {
            String::new()
        };

        let query = format!(r#"
            SELECT task_id, user_id, session_id, container_id, task_type, 
                   command, environment, status, created_at, completed_at,
                   exit_code, stdout, stderr
            FROM async_tasks 
            WHERE user_id = ? 
            ORDER BY created_at DESC
            {}
        "#, limit_clause);

        let rows: Vec<(String, String, String, Option<String>, String, String, String, String, i64, Option<i64>, Option<i32>, Option<String>, Option<String>)> = 
            sqlx::query_as(&query)
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(|e| AriaError::new(
                ErrorCode::DatabaseError,
                ErrorCategory::System,
                ErrorSeverity::High,
                &format!("Failed to get user tasks: {}", e)
            ))?;

        let mut tasks = Vec::new();
        for row in rows {
            let command: Vec<String> = serde_json::from_str(&row.5).unwrap_or_default();
            let environment: HashMap<String, String> = serde_json::from_str(&row.6).unwrap_or_default();
            
            let status = match row.7.as_str() {
                "pending" => AsyncTaskStatus::Pending,
                "running" => AsyncTaskStatus::Running,
                "completed" => AsyncTaskStatus::Completed,
                "failed" => AsyncTaskStatus::Failed,
                "cancelled" => AsyncTaskStatus::Cancelled,
                "timeout" => AsyncTaskStatus::Timeout,
                _ => AsyncTaskStatus::Failed,
            };

            tasks.push(AsyncTaskRecord {
                task_id: row.0,
                user_id: row.1,
                session_id: row.2,
                container_id: row.3,
                task_type: row.4,
                command,
                environment,
                status,
                created_at: row.8 as u64,
                completed_at: row.9.map(|t| t as u64),
                exit_code: row.10,
                stdout: row.11,
                stderr: row.12,
            });
        }

        Ok(tasks)
    }
} 