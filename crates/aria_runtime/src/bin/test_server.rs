use std::time::Duration;
use tokio::time::sleep;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;
use hyper_util::rt::TokioIo;

use aria_runtime::grpc::aria::{
    task_service_client::TaskServiceClient,
    session_service_client::SessionServiceClient,
    container_service_client::ContainerServiceClient,
    ListTasksRequest, CreateSessionRequest, ListContainersRequest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("🧪 Testing Aria Runtime gRPC API");
    
    // Wait a moment for server to be ready
    sleep(Duration::from_secs(2)).await;
    
    // Connect to the server
    let socket_path = "/run/aria/api.sock";
    let channel = create_channel(socket_path).await?;
    
    // Test TaskService
    println!("📋 Testing TaskService...");
    let mut task_client = TaskServiceClient::new(channel.clone());
    
    let task_response = task_client.list_tasks(tonic::Request::new(ListTasksRequest {
        session_id: None,
        filter_by_status: vec![],
        page_size: 10,
        page_token: "".to_string(),
    })).await;
    
    match task_response {
        Ok(response) => {
            println!("✅ TaskService working! Found {} tasks", response.into_inner().tasks.len());
        }
        Err(e) => {
            println!("❌ TaskService failed: {}", e);
        }
    }
    
    // Test SessionService
    println!("💬 Testing SessionService...");
    let mut session_client = SessionServiceClient::new(channel.clone());
    
    let session_response = session_client.create_session(tonic::Request::new(CreateSessionRequest {
        // Empty request as per proto definition
    })).await;
    
    match session_response {
        Ok(response) => {
            println!("✅ SessionService working! Created session: {}", response.into_inner().id);
        }
        Err(e) => {
            println!("❌ SessionService failed: {}", e);
        }
    }
    
    // Test ContainerService
    println!("🐳 Testing ContainerService...");
    let mut container_client = ContainerServiceClient::new(channel.clone());
    
    let container_response = container_client.list_containers(tonic::Request::new(ListContainersRequest {
        session_id: None,
    })).await;
    
    match container_response {
        Ok(response) => {
            println!("✅ ContainerService working! Found {} containers", response.into_inner().containers.len());
        }
        Err(e) => {
            println!("❌ ContainerService failed: {}", e);
        }
    }
    
    println!("🎉 Basic gRPC API test completed!");
    
    Ok(())
}

async fn create_channel(socket_path: &str) -> Result<Channel, tonic::transport::Error> {
    let socket_path = socket_path.to_string();
    
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(move |_: Uri| {
            let socket_path = socket_path.clone();
            async move {
                let unix_stream = tokio::net::UnixStream::connect(socket_path).await?;
                Ok::<_, std::io::Error>(TokioIo::new(unix_stream))
            }
        }))
        .await?;
    
    Ok(channel)
} 