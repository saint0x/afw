use aria_runtime::{init, RuntimeConfig};
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Aria Firmware Runtime...");

    // Initialize the Aria runtime
    let runtime = match init().await {
        Ok(runtime) => {
            info!("Aria Runtime initialized successfully");
            runtime
        },
        Err(e) => {
            error!("Failed to initialize Aria Runtime: {}", e);
            return Err(e.into());
        }
    };

    // Example task execution
    match runtime.execute("Hello from Aria Firmware!").await {
        Ok(result) => {
            info!("Task executed successfully: {:?}", result);
        },
        Err(e) => {
            error!("Task execution failed: {}", e);
        }
    }

    info!("Aria Firmware Runtime shutdown complete");
    Ok(())
}
