// User Database Operations
// Simple database operations for user management

use crate::errors::{AriaError, AriaResult, ErrorCategory, ErrorCode, ErrorSeverity};
use serde::{Deserialize, Serialize};

/// User record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub user_id: String,
    pub username: String,
    pub email: Option<String>,
    pub created_at: u64,
    pub last_active: Option<u64>,
    pub status: String,
}

/// Database operations for users
pub struct UserOps;

impl UserOps {
    /// Create a new user
    pub async fn create_user(
        pool: &sqlx::SqlitePool,
        user_id: &str,
        username: &str,
        email: Option<String>,
    ) -> AriaResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        sqlx::query(r#"
            INSERT INTO users (user_id, username, email, created_at, status)
            VALUES (?, ?, ?, ?, 'active')
        "#)
        .bind(user_id)
        .bind(username)
        .bind(&email)
        .bind(now as i64)
        .execute(pool)
        .await
        .map_err(|e| AriaError::new(
            ErrorCode::DatabaseError,
            ErrorCategory::System,
            ErrorSeverity::High,
            &format!("Failed to create user: {}", e)
        ))?;

        Ok(())
    }

    /// Get user by ID
    pub async fn get_user(pool: &sqlx::SqlitePool, user_id: &str) -> AriaResult<UserRecord> {
        let row: (String, String, Option<String>, i64, Option<i64>, String) = 
            sqlx::query_as(r#"
                SELECT user_id, username, email, created_at, last_active, status
                FROM users WHERE user_id = ?
            "#)
            .bind(user_id)
            .fetch_one(pool)
            .await
            .map_err(|e| AriaError::new(
                ErrorCode::DatabaseError,
                ErrorCategory::System,
                ErrorSeverity::High,
                &format!("Failed to get user: {}", e)
            ))?;

        Ok(UserRecord {
            user_id: row.0,
            username: row.1,
            email: row.2,
            created_at: row.3 as u64,
            last_active: row.4.map(|t| t as u64),
            status: row.5,
        })
    }
} 