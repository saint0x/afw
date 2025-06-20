// Database Module for Aria Runtime
// Implements production-grade SQLite database management with user-space deployment

use crate::errors::{AriaError, AriaResult, ErrorCategory, ErrorCode, ErrorSeverity};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod schema;
pub mod migrations;
pub mod async_tasks;
pub mod sessions;
pub mod users;
pub mod containers;
pub mod audit;

/// Database configuration for Aria Runtime
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub base_path: PathBuf,
    pub system_db_path: PathBuf,
    pub enable_wal_mode: bool,
    pub connection_timeout_seconds: u64,
    pub max_connections: u32,
    pub auto_vacuum: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let base_path = PathBuf::from(home).join(".aria");
        
        Self {
            system_db_path: base_path.join("system").join("system.db"),
            base_path,
            enable_wal_mode: true,
            connection_timeout_seconds: 30,
            max_connections: 10,
            auto_vacuum: true,
        }
    }
}

/// Main database manager for Aria Runtime
pub struct DatabaseManager {
    config: DatabaseConfig,
    system_pool: Arc<RwLock<Option<sqlx::SqlitePool>>>,
    user_pools: Arc<RwLock<HashMap<String, sqlx::SqlitePool>>>,
}

impl DatabaseManager {
    /// Create a new database manager
    pub fn new(config: DatabaseConfig) -> Self {
        Self {
            config,
            system_pool: Arc::new(RwLock::new(None)),
            user_pools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize the database system
    pub async fn initialize(&self) -> AriaResult<()> {
        // Create base directories
        self.create_directories().await?;
        
        // Initialize system database
        self.initialize_system_database().await?;
        
        tracing::info!("Database system initialized successfully");
        Ok(())
    }

    /// Create necessary directories
    async fn create_directories(&self) -> AriaResult<()> {
        let system_dir = self.config.system_db_path.parent().unwrap();
        let users_dir = self.config.base_path.join("users");
        
        tokio::fs::create_dir_all(system_dir).await
            .map_err(|e| AriaError::new(
                ErrorCode::SystemNotReady,
                ErrorCategory::System,
                ErrorSeverity::Critical,
                &format!("Failed to create system directory: {}", e)
            ))?;

        tokio::fs::create_dir_all(users_dir).await
            .map_err(|e| AriaError::new(
                ErrorCode::SystemNotReady,
                ErrorCategory::System,
                ErrorSeverity::Critical,
                &format!("Failed to create users directory: {}", e)
            ))?;

        Ok(())
    }

    /// Initialize system database
    async fn initialize_system_database(&self) -> AriaResult<()> {
        let pool = self.create_pool(&self.config.system_db_path).await?;
        
        // Run system migrations
        migrations::run_system_migrations(&pool).await?;
        
        // Store the pool
        {
            let mut system_pool = self.system_pool.write().await;
            *system_pool = Some(pool);
        }

        Ok(())
    }

    /// Get or create user database
    pub async fn get_user_database(&self, user_id: &str) -> AriaResult<sqlx::SqlitePool> {
        // Check if we already have this user's pool
        {
            let user_pools = self.user_pools.read().await;
            if let Some(pool) = user_pools.get(user_id) {
                return Ok(pool.clone());
            }
        }

        // Create new user database
        let user_db_path = self.config.base_path
            .join("users")
            .join(user_id)
            .join("aria.db");

        // Create user directory
        if let Some(user_dir) = user_db_path.parent() {
            tokio::fs::create_dir_all(user_dir).await
                .map_err(|e| AriaError::new(
                    ErrorCode::SystemNotReady,
                    ErrorCategory::System,
                    ErrorSeverity::High,
                    &format!("Failed to create user directory: {}", e)
                ))?;
        }

        let pool = self.create_pool(&user_db_path).await?;
        
        // Run user migrations
        migrations::run_user_migrations(&pool).await?;

        // Store the pool
        {
            let mut user_pools = self.user_pools.write().await;
            user_pools.insert(user_id.to_string(), pool.clone());
        }

        Ok(pool)
    }

    /// Get system database pool
    pub async fn get_system_database(&self) -> AriaResult<sqlx::SqlitePool> {
        let system_pool = self.system_pool.read().await;
        system_pool.as_ref()
            .cloned()
            .ok_or_else(|| AriaError::new(
                ErrorCode::SystemNotReady,
                ErrorCategory::System,
                ErrorSeverity::Critical,
                "System database not initialized"
            ))
    }

    /// Get system database pool (convenience method)
    pub fn pool(&self) -> impl std::future::Future<Output = AriaResult<sqlx::SqlitePool>> + '_ {
        self.get_system_database()
    }

    /// Create a new SQLite connection pool
    async fn create_pool(&self, db_path: &Path) -> AriaResult<sqlx::SqlitePool> {
        let database_url = format!("sqlite:{}", db_path.display());
        
        let mut options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true);

        // Configure SQLite options for performance
        if self.config.enable_wal_mode {
            options = options.journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
        }

        if self.config.auto_vacuum {
            options = options.auto_vacuum(sqlx::sqlite::SqliteAutoVacuum::Incremental);
        }

        options = options
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .busy_timeout(std::time::Duration::from_secs(self.config.connection_timeout_seconds));

        let pool = sqlx::SqlitePool::connect_with(options).await
            .map_err(|e| AriaError::new(
                ErrorCode::NetworkError,
                ErrorCategory::System,
                ErrorSeverity::Critical,
                &format!("Failed to create database pool: {}", e)
            ))?;

        Ok(pool)
    }

    /// Close all database connections
    pub async fn shutdown(&self) -> AriaResult<()> {
        // Close system database
        {
            let mut system_pool = self.system_pool.write().await;
            if let Some(pool) = system_pool.take() {
                pool.close().await;
            }
        }

        // Close all user databases
        {
            let mut user_pools = self.user_pools.write().await;
            for (_, pool) in user_pools.drain() {
                pool.close().await;
            }
        }

        tracing::info!("Database system shutdown complete");
        Ok(())
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> AriaResult<DatabaseStats> {
        let system_pool = self.system_pool.read().await;
        let user_pools = self.user_pools.read().await;

        let system_connections = system_pool.as_ref()
            .map(|p| p.size())
            .unwrap_or(0);

        let total_user_connections: u32 = user_pools.values()
            .map(|p| p.size())
            .sum();

        let user_databases = user_pools.len();

        Ok(DatabaseStats {
            system_connections,
            total_user_connections,
            user_databases,
            total_connections: system_connections + total_user_connections,
        })
    }
}

/// Database statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub system_connections: u32,
    pub total_user_connections: u32,
    pub user_databases: usize,
    pub total_connections: u32,
}

/// Database error types specific to Aria
#[derive(Debug, Clone)]
pub enum DatabaseError {
    ConnectionFailed(String),
    MigrationFailed(String),
    QueryFailed(String),
    UserNotFound(String),
    SessionNotFound(String),
    TaskNotFound(String),
    IntegrityViolation(String),
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::ConnectionFailed(msg) => write!(f, "Database connection failed: {}", msg),
            DatabaseError::MigrationFailed(msg) => write!(f, "Database migration failed: {}", msg),
            DatabaseError::QueryFailed(msg) => write!(f, "Database query failed: {}", msg),
            DatabaseError::UserNotFound(id) => write!(f, "User not found: {}", id),
            DatabaseError::SessionNotFound(id) => write!(f, "Session not found: {}", id),
            DatabaseError::TaskNotFound(id) => write!(f, "Task not found: {}", id),
            DatabaseError::IntegrityViolation(msg) => write!(f, "Database integrity violation: {}", msg),
        }
    }
}

impl std::error::Error for DatabaseError {} 