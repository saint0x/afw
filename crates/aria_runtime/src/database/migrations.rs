// Database Migration System for Aria Runtime
// Handles schema versioning and migration for both system and user databases

use crate::errors::{AriaError, AriaResult, ErrorCategory, ErrorCode, ErrorSeverity};
use crate::database::schema::{SYSTEM_SCHEMA, USER_SCHEMA};
use sqlx::SqlitePool;
use std::time::{SystemTime, UNIX_EPOCH};

/// Current schema version for system database
pub const SYSTEM_SCHEMA_VERSION: i32 = 1;

/// Current schema version for user databases
pub const USER_SCHEMA_VERSION: i32 = 1;

/// Migration metadata
#[derive(Debug, Clone)]
pub struct Migration {
    pub version: i32,
    pub description: String,
    pub sql: String,
    pub applied_at: Option<u64>,
}

/// Run all system database migrations
pub async fn run_system_migrations(pool: &SqlitePool) -> AriaResult<()> {
    tracing::info!("Running system database migrations");
    
    // Create migrations table if it doesn't exist
    create_migrations_table(pool).await?;
    
    // Get current version
    let current_version = get_current_version(pool).await?;
    tracing::info!("Current system schema version: {}", current_version);
    
    // Apply migrations
    let migrations = get_system_migrations();
    for migration in migrations {
        if migration.version > current_version {
            apply_migration(pool, &migration).await?;
        }
    }
    
    tracing::info!("System database migrations completed");
    Ok(())
}

/// Run all user database migrations
pub async fn run_user_migrations(pool: &SqlitePool) -> AriaResult<()> {
    tracing::debug!("Running user database migrations");
    
    // Create migrations table if it doesn't exist
    create_migrations_table(pool).await?;
    
    // Get current version
    let current_version = get_current_version(pool).await?;
    
    // Apply migrations
    let migrations = get_user_migrations();
    for migration in migrations {
        if migration.version > current_version {
            apply_migration(pool, &migration).await?;
        }
    }
    
    tracing::debug!("User database migrations completed");
    Ok(())
}

/// Create the migrations tracking table
async fn create_migrations_table(pool: &SqlitePool) -> AriaResult<()> {
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS _migrations (
            version INTEGER PRIMARY KEY,
            description TEXT NOT NULL,
            applied_at INTEGER NOT NULL,
            checksum TEXT NOT NULL
        )
    "#)
    .execute(pool)
    .await
    .map_err(|e| AriaError::new(
        ErrorCode::DatabaseError,
        ErrorCategory::System,
        ErrorSeverity::Critical,
        &format!("Failed to create migrations table: {}", e)
    ))?;
    
    Ok(())
}

/// Get current schema version
async fn get_current_version(pool: &SqlitePool) -> AriaResult<i32> {
    let result: Option<(i32,)> = sqlx::query_as(
        "SELECT MAX(version) FROM _migrations"
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AriaError::new(
        ErrorCode::DatabaseError,
        ErrorCategory::System,
        ErrorSeverity::High,
        &format!("Failed to get current schema version: {}", e)
    ))?;
    
    Ok(result.map(|(v,)| v).unwrap_or(0))
}

/// Apply a single migration
async fn apply_migration(pool: &SqlitePool, migration: &Migration) -> AriaResult<()> {
    tracing::info!("Applying migration {}: {}", migration.version, migration.description);
    
    // Start transaction
    let mut tx = pool.begin().await
        .map_err(|e| AriaError::new(
            ErrorCode::DatabaseError,
            ErrorCategory::System,
            ErrorSeverity::Critical,
            &format!("Failed to start migration transaction: {}", e)
        ))?;
    
    // Execute migration SQL
    sqlx::query(&migration.sql)
        .execute(&mut *tx)
        .await
        .map_err(|e| AriaError::new(
            ErrorCode::DatabaseError,
            ErrorCategory::System,
            ErrorSeverity::Critical,
            &format!("Failed to execute migration {}: {}", migration.version, e)
        ))?;
    
    // Record migration in _migrations table
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    
    let checksum = calculate_checksum(&migration.sql);
    
    sqlx::query(r#"
        INSERT INTO _migrations (version, description, applied_at, checksum)
        VALUES (?, ?, ?, ?)
    "#)
    .bind(migration.version)
    .bind(&migration.description)
    .bind(now)
    .bind(checksum)
    .execute(&mut *tx)
    .await
    .map_err(|e| AriaError::new(
        ErrorCode::DatabaseError,
        ErrorCategory::System,
        ErrorSeverity::Critical,
        &format!("Failed to record migration {}: {}", migration.version, e)
    ))?;
    
    // Commit transaction
    tx.commit().await
        .map_err(|e| AriaError::new(
            ErrorCode::DatabaseError,
            ErrorCategory::System,
            ErrorSeverity::Critical,
            &format!("Failed to commit migration {}: {}", migration.version, e)
        ))?;
    
    tracing::info!("Successfully applied migration {}", migration.version);
    Ok(())
}

/// Get system database migrations
fn get_system_migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            description: "Initial system schema".to_string(),
            sql: SYSTEM_SCHEMA.to_string(),
            applied_at: None,
        },
        // Future migrations will be added here
    ]
}

/// Get user database migrations
fn get_user_migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            description: "Initial user schema".to_string(),
            sql: USER_SCHEMA.to_string(),
            applied_at: None,
        },
        // Future migrations will be added here
    ]
}

/// Calculate a simple checksum for migration integrity
fn calculate_checksum(sql: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    sql.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Verify migration integrity
pub async fn verify_migrations(pool: &SqlitePool) -> AriaResult<bool> {
    let applied_migrations: Vec<(i32, String)> = sqlx::query_as(
        "SELECT version, checksum FROM _migrations ORDER BY version"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AriaError::new(
        ErrorCode::DatabaseError,
        ErrorCategory::System,
        ErrorSeverity::High,
        &format!("Failed to verify migrations: {}", e)
    ))?;
    
    // Check if we have any system or user migrations based on table existence
    let table_exists: bool = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT name FROM sqlite_master WHERE type='table' AND name='users')"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AriaError::new(
        ErrorCode::DatabaseError,
        ErrorCategory::System,
        ErrorSeverity::High,
        &format!("Failed to check table existence: {}", e)
    ))?;
    
    let migrations = if table_exists {
        get_system_migrations()
    } else {
        get_user_migrations()
    };
    
    for (version, stored_checksum) in applied_migrations {
        if let Some(migration) = migrations.iter().find(|m| m.version == version) {
            let calculated_checksum = calculate_checksum(&migration.sql);
            if calculated_checksum != stored_checksum {
                tracing::error!("Migration {} checksum mismatch", version);
                return Ok(false);
            }
        }
    }
    
    Ok(true)
}

/// Get migration history
pub async fn get_migration_history(pool: &SqlitePool) -> AriaResult<Vec<Migration>> {
    let rows: Vec<(i32, String, i64, String)> = sqlx::query_as(
        "SELECT version, description, applied_at, checksum FROM _migrations ORDER BY version"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AriaError::new(
        ErrorCode::DatabaseError,
        ErrorCategory::System,
        ErrorSeverity::High,
        &format!("Failed to get migration history: {}", e)
    ))?;
    
    let migrations = rows.into_iter()
        .map(|(version, description, applied_at, _checksum)| Migration {
            version,
            description,
            sql: String::new(), // Don't include SQL in history
            applied_at: Some(applied_at as u64),
        })
        .collect();
    
    Ok(migrations)
} 