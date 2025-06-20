use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Server;
use tracing::{info, error};
use std::path::PathBuf;

use aria_runtime::{
    database::{DatabaseManager, DatabaseConfig},
    engines::{
        container::quilt::QuiltService,
        tool_registry::ToolRegistry,
        llm::LLMHandler,
        intelligence::IntelligenceEngine,
        observability::ObservabilityManager,
    },
    grpc::{
        aria::{
            task_service_server::TaskServiceServer,
            session_service_server::SessionServiceServer,
            container_service_server::ContainerServiceServer,
            notification_service_server::NotificationServiceServer,
            bundle_service_server::BundleServiceServer,
        },
        task_service::TaskServiceImpl,
        session_service::SessionServiceImpl,
        container_service::ContainerServiceImpl,
        notification_service::NotificationServiceImpl,
        bundle_service::BundleServiceImpl,
    },
    errors::AriaResult,
};

/// Configuration for the Aria Runtime gRPC server
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub socket_path: String,
    pub database_path: String,
    pub quilt_socket_path: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            socket_path: "/run/aria/api.sock".to_string(),
            database_path: "./aria.db".to_string(),
            quilt_socket_path: "/run/quilt/api.sock".to_string(),
        }
    }
}

/// Main Aria Runtime gRPC server
pub struct AriaServer {
    config: ServerConfig,
    database: Arc<DatabaseManager>,
    quilt_service: Arc<Mutex<QuiltService>>,
    tool_registry: Arc<ToolRegistry>,
    intelligence_engine: Arc<IntelligenceEngine>,
}

impl AriaServer {
    /// Create a new Aria Runtime server
    pub async fn new(config: ServerConfig) -> AriaResult<Self> {
        info!("Initializing Aria Runtime server");
        
        // Initialize database
        let database_config = DatabaseConfig {
            base_path: config.database_path.clone().into(),
            system_db_path: PathBuf::from(&config.database_path).join("system.db"),
            enable_wal_mode: true,
            connection_timeout_seconds: 30,
            max_connections: 10,
            auto_vacuum: true,
        };
        let database = Arc::new(DatabaseManager::new(database_config));
        database.initialize().await?;
        info!("Database initialized at: {}", config.database_path);
        
        // Initialize Quilt service
        let quilt_config = aria_runtime::engines::config::QuiltConfig {
            socket_path: config.quilt_socket_path.clone(),
        };
        let quilt_service = Arc::new(Mutex::new(
            QuiltService::new(&quilt_config).await
                .map_err(|e| aria_runtime::errors::AriaError::new(
                    aria_runtime::errors::ErrorCode::UpstreamServiceError,
                    aria_runtime::errors::ErrorCategory::Network,
                    aria_runtime::errors::ErrorSeverity::High,
                    &format!("Failed to connect to Quilt daemon: {}", e)
                ))?
        ));
        info!("Connected to Quilt daemon at: {}", config.quilt_socket_path);
        
        // Initialize tool registry (requires llm_handler as first parameter)
        let llm_handler = LLMHandler::get_instance();
        let tool_registry = Arc::new(ToolRegistry::new(
            llm_handler,
            Arc::clone(&quilt_service),
        ).await);
        info!("Tool registry initialized");
        
        // Initialize observability (requires database)
        let observability = Arc::new(
            ObservabilityManager::new(
                Arc::clone(&database),
                1000 // buffer size
            )?
        );
        
        // Initialize intelligence engine (requires database and observability)
        let intelligence_engine = Arc::new(
            IntelligenceEngine::new(
                Arc::clone(&database),
                observability,
                aria_runtime::engines::intelligence::IntelligenceConfig::default()
            )
        );
        
        Ok(Self {
            config,
            database,
            quilt_service,
            tool_registry,
            intelligence_engine,
        })
    }
    
    /// Start the gRPC server
    pub async fn serve(self) -> AriaResult<()> {
        info!("Starting Aria Runtime gRPC server on: {}", self.config.socket_path);
        
        // Create service implementations
        let task_service = TaskServiceImpl::new(
            Arc::clone(&self.quilt_service),
            Arc::clone(&self.database),
        );
        
        let session_service = SessionServiceImpl::new(
            Arc::clone(&self.database),
            Arc::clone(&self.intelligence_engine),
            Arc::clone(&self.tool_registry),
        );
        
        let container_service = ContainerServiceImpl::new(
            Arc::clone(&self.quilt_service),
        );
        
        let notification_service = NotificationServiceImpl::new(
            Arc::clone(&self.database),
        );
        
        let bundle_service = BundleServiceImpl::new(
            Arc::clone(&self.quilt_service),
        );
        
        // Remove existing socket file if it exists
        if std::path::Path::new(&self.config.socket_path).exists() {
            std::fs::remove_file(&self.config.socket_path)
                .map_err(|e| aria_runtime::errors::AriaError::new(
                    aria_runtime::errors::ErrorCode::InitializationFailed,
                    aria_runtime::errors::ErrorCategory::System,
                    aria_runtime::errors::ErrorSeverity::High,
                    &format!("Failed to remove existing socket: {}", e)
                ))?;
        }
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = std::path::Path::new(&self.config.socket_path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| aria_runtime::errors::AriaError::new(
                    aria_runtime::errors::ErrorCode::InitializationFailed,
                    aria_runtime::errors::ErrorCategory::System,
                    aria_runtime::errors::ErrorSeverity::High,
                    &format!("Failed to create socket directory: {}", e)
                ))?;
        }
        
        // Create Unix Domain Socket listener
        let incoming = {
            let uds = tokio::net::UnixListener::bind(&self.config.socket_path)
                .map_err(|e| aria_runtime::errors::AriaError::new(
                    aria_runtime::errors::ErrorCode::NetworkError,
                    aria_runtime::errors::ErrorCategory::Network,
                    aria_runtime::errors::ErrorSeverity::High,
                    &format!("Failed to bind to Unix socket: {}", e)
                ))?;
            
            info!("gRPC server listening on Unix socket: {}", self.config.socket_path);
            
            tokio_stream::wrappers::UnixListenerStream::new(uds)
        };
        
        // Build and start the server
        let result = Server::builder()
            .add_service(TaskServiceServer::new(task_service))
            .add_service(SessionServiceServer::new(session_service))
            .add_service(ContainerServiceServer::new(container_service))
            .add_service(NotificationServiceServer::new(notification_service))
            .add_service(BundleServiceServer::new(bundle_service))
            .serve_with_incoming(incoming)
            .await;
        
        match result {
            Ok(()) => {
                info!("Aria Runtime gRPC server shut down gracefully");
                Ok(())
            }
            Err(e) => {
                error!("gRPC server error: {}", e);
                Err(aria_runtime::errors::AriaError::new(
                    aria_runtime::errors::ErrorCode::NetworkError,
                    aria_runtime::errors::ErrorCategory::Network,
                    aria_runtime::errors::ErrorSeverity::High,
                    &format!("gRPC server failed: {}", e)
                ))
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    info!("Starting Aria Runtime gRPC Server");
    
    // Parse command line arguments or use defaults
    let config = ServerConfig::default();
    
    // Create and start server
    let server = AriaServer::new(config).await?;
    server.serve().await?;
    
    Ok(())
} 