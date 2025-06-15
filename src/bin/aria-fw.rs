use aria_runtime::{create_aria_runtime_default, RuntimeConfiguration, RUNTIME_VERSION};
use tracing::{info, error, warn};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with environment-based filtering
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("🚀 Aria Firmware starting up...");
    info!("Version: {}", RUNTIME_VERSION);

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let config_path = args.get(1).map(|s| s.as_str()).unwrap_or("aria.toml");

    // TODO: Load config from file
    let config = RuntimeConfiguration::default();
    
    info!("Configuration loaded:");
    info!("  - Enhanced runtime: {}", config.enhanced_runtime);
    info!("  - Planning threshold: {:?}", config.planning_threshold);
    info!("  - Reflection enabled: {}", config.reflection_enabled);
    info!("  - Max steps per plan: {}", config.max_steps_per_plan);
    info!("  - Timeout: {}ms", config.timeout_ms);

    // Initialize the Aria runtime
    let runtime = match create_aria_runtime_default().await {
        Ok(runtime) => {
            info!("✅ Aria Runtime initialized successfully");
            runtime
        },
        Err(e) => {
            error!("❌ Failed to initialize Aria Runtime: {}", e);
            warn!("🚧 Runtime implementation is not yet complete - this is expected during development");
            warn!("📚 Check ARIARUNTIME.md for implementation progress");
            return Ok(()); // Exit gracefully during development
        }
    };

    info!("🎯 Aria Firmware is ready to accept .aria bundles");

    // TODO: Start gRPC server on port 7600
    // TODO: Start WebSocket server on port 7601
    // TODO: Initialize Quilt integration
    // TODO: Start telemetry collection

    // Keep the firmware running
    info!("🔄 Entering main event loop...");
    
    // Graceful shutdown handling
    tokio::signal::ctrl_c().await?;
    warn!("🛑 Shutdown signal received");

    info!("👋 Aria Firmware shutdown complete");
    Ok(())
} 