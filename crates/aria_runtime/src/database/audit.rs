// Audit Log Database Operations
// Simple database operations for audit logging

use crate::errors::{AriaError, AriaResult, ErrorCategory, ErrorCode, ErrorSeverity};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Audit log record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogRecord {
    pub log_id: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub event_type: String,
    pub event_data: Option<serde_json::Value>,
    pub severity: String,
    pub created_at: u64,
}

/// Database operations for audit logs
pub struct AuditOps;

impl AuditOps {
    /// Create an audit log entry
    pub async fn log_event(
        pool: &sqlx::SqlitePool,
        user_id: Option<String>,
        session_id: Option<String>,
        event_type: &str,
        event_data: Option<serde_json::Value>,
        severity: &str,
    ) -> AriaResult<()> {
        let log_id = Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let event_data_json = if let Some(data) = event_data {
            serde_json::to_string(&data)
                .map_err(|e| AriaError::new(
                    ErrorCode::SerializationError,
                    ErrorCategory::System,
                    ErrorSeverity::Medium,
                    &format!("Failed to serialize event data: {}", e)
                ))?
        } else {
            "null".to_string()
        };

        sqlx::query(r#"
            INSERT INTO audit_logs (log_id, user_id, session_id, event_type, event_data, severity, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&log_id)
        .bind(&user_id)
        .bind(&session_id)
        .bind(event_type)
        .bind(event_data_json)
        .bind(severity)
        .bind(now as i64)
        .execute(pool)
        .await
        .map_err(|e| AriaError::new(
            ErrorCode::DatabaseError,
            ErrorCategory::System,
            ErrorSeverity::High,
            &format!("Failed to create audit log: {}", e)
        ))?;

        Ok(())
    }

    /// Get audit logs for a user
    pub async fn get_user_audit_logs(
        pool: &sqlx::SqlitePool,
        user_id: &str,
        limit: Option<u32>,
    ) -> AriaResult<Vec<AuditLogRecord>> {
        let limit_clause = if let Some(l) = limit {
            format!("LIMIT {}", l)
        } else {
            String::new()
        };

        let query = format!(r#"
            SELECT log_id, user_id, session_id, event_type, event_data, severity, created_at
            FROM audit_logs 
            WHERE user_id = ? 
            ORDER BY created_at DESC
            {}
        "#, limit_clause);

        let rows: Vec<(String, Option<String>, Option<String>, String, String, String, i64)> = 
            sqlx::query_as(&query)
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(|e| AriaError::new(
                ErrorCode::DatabaseError,
                ErrorCategory::System,
                ErrorSeverity::High,
                &format!("Failed to get audit logs: {}", e)
            ))?;

        let mut logs = Vec::new();
        for row in rows {
            let event_data = if row.4 != "null" {
                serde_json::from_str(&row.4).ok()
            } else {
                None
            };

            logs.push(AuditLogRecord {
                log_id: row.0,
                user_id: row.1,
                session_id: row.2,
                event_type: row.3,
                event_data,
                severity: row.5,
                created_at: row.6 as u64,
            });
        }

        Ok(logs)
    }
} 