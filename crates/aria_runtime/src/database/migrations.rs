// Database Migration System for Aria Runtime
// Handles schema versioning and migration for both system and user databases

use crate::errors::{AriaError, AriaResult, ErrorCategory, ErrorCode, ErrorSeverity};
use crate::database::schema::{SYSTEM_SCHEMA, USER_SCHEMA};
use sqlx::SqlitePool;
use std::time::{SystemTime, UNIX_EPOCH};

/// Current schema version for system database
pub const SYSTEM_SCHEMA_VERSION: i32 = 1;

/// Current schema version for user databases
pub const USER_SCHEMA_VERSION: i32 = 2;

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
        Migration {
            version: 2,
            description: "Context Intelligence schema extension".to_string(),
            sql: r#"
-- ======================================
-- CONTEXT INTELLIGENCE SCHEMA EXTENSION
-- ======================================

-- Container execution patterns (learned from execution history)
CREATE TABLE IF NOT EXISTS container_patterns (
    pattern_id TEXT PRIMARY KEY,
    pattern_trigger TEXT NOT NULL,          -- "build rust project", "run tests"
    container_config TEXT NOT NULL,         -- JSON container configuration
    confidence_score REAL DEFAULT 0.5,     -- Learning confidence (0.0 - 1.0)
    success_count INTEGER DEFAULT 0,
    failure_count INTEGER DEFAULT 0,
    avg_execution_time_ms INTEGER DEFAULT 0,
    last_used INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    pattern_variables TEXT,                 -- JSON array of variable definitions
    usage_stats TEXT                        -- JSON usage statistics blob
);

-- Execution context trees (hierarchical execution relationships)
CREATE TABLE IF NOT EXISTS execution_contexts (
    context_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    parent_context_id TEXT,
    context_type TEXT NOT NULL,             -- "session", "workflow", "container", "tool", "agent", "environment"
    context_data TEXT NOT NULL,             -- JSON context information
    priority INTEGER DEFAULT 5,            -- Context priority (1-10)
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    metadata TEXT,                          -- JSON additional metadata
    FOREIGN KEY (parent_context_id) REFERENCES execution_contexts(context_id),
    FOREIGN KEY (session_id) REFERENCES sessions(session_id)
);

-- Learning feedback (captures execution results for pattern improvement)
CREATE TABLE IF NOT EXISTS learning_feedback (
    feedback_id TEXT PRIMARY KEY,
    pattern_id TEXT NOT NULL,
    execution_id TEXT NOT NULL,
    success BOOLEAN NOT NULL,
    execution_time_ms INTEGER,
    feedback_type TEXT NOT NULL,            -- "execution", "user", "system"
    confidence_delta REAL,                  -- Change in pattern confidence
    metadata TEXT,                          -- JSON additional data
    created_at INTEGER NOT NULL,
    FOREIGN KEY (pattern_id) REFERENCES container_patterns(pattern_id)
);

-- Container workload tracking (links containers to intelligence patterns)
CREATE TABLE IF NOT EXISTS container_workloads (
    workload_id TEXT PRIMARY KEY,
    container_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    pattern_id TEXT,                        -- Associated pattern if any
    workload_type TEXT NOT NULL,            -- "build", "test", "exec", "analysis"
    request_description TEXT NOT NULL,      -- Original user request
    execution_result TEXT,                  -- JSON execution result
    created_at INTEGER NOT NULL,
    completed_at INTEGER,
    FOREIGN KEY (container_id) REFERENCES containers(container_id),
    FOREIGN KEY (session_id) REFERENCES sessions(session_id),
    FOREIGN KEY (pattern_id) REFERENCES container_patterns(pattern_id)
);

-- Intelligence query log (tracks intelligence API usage)
CREATE TABLE IF NOT EXISTS intelligence_queries (
    query_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    query_type TEXT NOT NULL,               -- "pattern_match", "context_build", "learning_update"
    request_data TEXT NOT NULL,             -- JSON request data
    response_data TEXT,                     -- JSON response data
    execution_time_ms INTEGER,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(session_id)
);

-- Context Intelligence Indexes for performance
CREATE INDEX IF NOT EXISTS idx_container_patterns_confidence ON container_patterns(confidence_score DESC);
CREATE INDEX IF NOT EXISTS idx_container_patterns_last_used ON container_patterns(last_used DESC);
CREATE INDEX IF NOT EXISTS idx_container_patterns_trigger ON container_patterns(pattern_trigger);

CREATE INDEX IF NOT EXISTS idx_execution_contexts_session ON execution_contexts(session_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_execution_contexts_parent ON execution_contexts(parent_context_id);
CREATE INDEX IF NOT EXISTS idx_execution_contexts_type ON execution_contexts(context_type);
CREATE INDEX IF NOT EXISTS idx_execution_contexts_priority ON execution_contexts(priority DESC);

CREATE INDEX IF NOT EXISTS idx_learning_feedback_pattern ON learning_feedback(pattern_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_learning_feedback_execution ON learning_feedback(execution_id);
CREATE INDEX IF NOT EXISTS idx_learning_feedback_success ON learning_feedback(success, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_container_workloads_session ON container_workloads(session_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_container_workloads_pattern ON container_workloads(pattern_id);
CREATE INDEX IF NOT EXISTS idx_container_workloads_type ON container_workloads(workload_type);

CREATE INDEX IF NOT EXISTS idx_intelligence_queries_session ON intelligence_queries(session_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_intelligence_queries_type ON intelligence_queries(query_type);
            "#.to_string(),
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