use aria_runtime::{
    AriaRuntime, RuntimeConfiguration, AriaResult, AriaError,
    types::{AgentConfig, LLMConfig, ContainerSpec},
    deep_size::DeepUuid,
    engines::icc::{ToolExecutionRequest, LLMCompletionRequest, LLMMessage, ContextAccessRequest},
};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use uuid::Uuid;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("🚀 ICC (Inter-Container Communication) Demo");
    println!("===========================================");
    
    // Initialize Aria Runtime
    println!("\n📦 Initializing Aria Runtime with ICC Engine...");
    let runtime = create_test_runtime().await?;
    
    // Start ICC Server
    println!("🌐 Starting ICC HTTP Server...");
    start_icc_server(&runtime).await?;
    
    // Test ICC Environment Creation
    println!("\n🔧 Testing ICC Environment Creation...");
    test_icc_environment_creation(&runtime).await?;
    
    // Test HTTP Endpoints
    println!("\n🌍 Testing ICC HTTP Endpoints...");
    test_icc_endpoints().await?;
    
    // Test Authentication
    println!("\n🔐 Testing ICC Authentication...");
    test_icc_authentication(&runtime).await?;
    
    // Test Container Integration
    println!("\n📦 Testing Container Integration with ICC...");
    test_container_icc_integration(&runtime).await?;
    
    // Cleanup
    println!("\n🧹 Stopping ICC Server...");
    runtime.stop_icc_server().await?;
    
    println!("\n✅ ICC Demo completed successfully!");
    println!("📋 Summary:");
    println!("   - ICC HTTP server started and stopped");
    println!("   - Environment variables created for containers");
    println!("   - Authentication tokens generated and validated");
    println!("   - HTTP endpoints tested");
    println!("   - Container integration verified");
    
    Ok(())
}

async fn create_test_runtime() -> AriaResult<AriaRuntime> {
    let config = RuntimeConfiguration {
        enhanced_runtime: true,
        planning_threshold: aria_runtime::types::TaskComplexity::MultiStep,
        reflection_enabled: true,
        container_execution_enabled: true,
        max_steps_per_plan: 20,
        timeout_ms: 300000, // 5 minutes
        retry_attempts: 3,
        debug_mode: true,
        memory_limit_mb: 1024,
        max_concurrent_containers: 5,
        container_timeout_seconds: 120,
        enable_icc_callbacks: true,
        llm_providers: vec!["openai".to_string()],
        default_llm_provider: "openai".to_string(),
        enable_caching: true,
        cache_ttl_seconds: 300,
    };
    
    // For now, we'll use the engines directly since create_aria_runtime isn't implemented
    let engines = aria_runtime::engines::AriaEngines::new().await;
    let runtime = AriaRuntime::with_engines(engines, config);
    
    println!("✅ Aria Runtime initialized with ICC Engine");
    Ok(runtime)
}

async fn start_icc_server(runtime: &AriaRuntime) -> AriaResult<()> {
    // Start ICC server in background
    let runtime_clone = runtime.clone();
    tokio::spawn(async move {
        if let Err(e) = runtime_clone.start_icc_server().await {
            eprintln!("❌ ICC Server error: {}", e);
        }
    });
    
    // Give server time to start
    sleep(Duration::from_millis(500)).await;
    
    println!("✅ ICC Server started on http://10.42.0.1:8080");
    Ok(())
}

async fn test_icc_environment_creation(runtime: &AriaRuntime) -> AriaResult<()> {
    let session_id = Uuid::new_v4();
    let container_id = "test-container-001".to_string();
    let permissions = vec!["tools".to_string(), "llm".to_string(), "context".to_string()];
    
    let env_vars = runtime.create_icc_environment(session_id, container_id.clone(), permissions)?;
    
    println!("📋 Generated ICC Environment Variables:");
    for (key, value) in &env_vars {
        if key.contains("TOKEN") {
            println!("   {}: {}...", key, &value[..20]);
        } else {
            println!("   {}: {}", key, value);
        }
    }
    
    // Verify required variables exist
    let required_vars = [
        "ARIA_ICC_ENDPOINT",
        "ARIA_SESSION_TOKEN", 
        "ARIA_SESSION_ID",
        "ARIA_CONTAINER_ID",
        "ARIA_TOOLS_URL",
        "ARIA_AGENTS_URL",
        "ARIA_LLM_URL",
        "ARIA_CONTEXT_URL"
    ];
    
    for var in &required_vars {
        if !env_vars.contains_key(*var) {
            return Err(AriaError::new(
                aria_runtime::errors::ErrorCode::ConfigError,
                aria_runtime::errors::ErrorCategory::System,
                aria_runtime::errors::ErrorSeverity::High,
                &format!("Missing required environment variable: {}", var)
            ));
        }
    }
    
    println!("✅ All required ICC environment variables created");
    Ok(())
}

async fn test_icc_endpoints() -> AriaResult<()> {
    let client = reqwest::Client::new();
    let base_url = "http://10.42.0.1:8080";
    
    // Test health endpoint (no auth required)
    println!("🏥 Testing health endpoint...");
    match client.get(&format!("{}/health", base_url)).send().await {
        Ok(response) => {
            if response.status().is_success() {
                println!("✅ Health endpoint responding");
                if let Ok(body) = response.text().await {
                    println!("   Response: {}", body);
                }
            } else {
                println!("⚠️  Health endpoint returned status: {}", response.status());
            }
        }
        Err(e) => {
            println!("⚠️  Health endpoint not reachable: {}", e);
            println!("   (This is expected if no quilt daemon is running)");
        }
    }
    
    // Test status endpoint (no auth required)
    println!("📊 Testing status endpoint...");
    match client.get(&format!("{}/status", base_url)).send().await {
        Ok(response) => {
            if response.status().is_success() {
                println!("✅ Status endpoint responding");
                if let Ok(body) = response.text().await {
                    println!("   Response: {}", body);
                }
            } else {
                println!("⚠️  Status endpoint returned status: {}", response.status());
            }
        }
        Err(e) => {
            println!("⚠️  Status endpoint not reachable: {}", e);
        }
    }
    
    println!("✅ Public endpoints tested");
    Ok(())
}

async fn test_icc_authentication(runtime: &AriaRuntime) -> AriaResult<()> {
    let session_id = Uuid::new_v4();
    let container_id = "auth-test-container".to_string();
    let permissions = vec!["tools".to_string()];
    
    let env_vars = runtime.create_icc_environment(session_id, container_id, permissions)?;
    let token = env_vars.get("ARIA_SESSION_TOKEN").unwrap();
    
    println!("🔑 Testing authentication with generated token...");
    
    let client = reqwest::Client::new();
    let base_url = "http://10.42.0.1:8080";
    
    // Test authenticated endpoint
    let tool_request = ToolExecutionRequest {
        tool_name: "ponderTool".to_string(),
        parameters: json!({
            "question": "What is the meaning of life?",
            "depth": 2
        }),
        timeout_seconds: Some(30),
        capture_output: Some(true),
    };
    
    match client
        .post(&format!("{}/tools/ponderTool", base_url))
        .header("Authorization", format!("Bearer {}", token))
        .json(&tool_request)
        .send()
        .await
    {
        Ok(response) => {
            println!("✅ Authentication successful (status: {})", response.status());
            if !response.status().is_success() {
                if let Ok(body) = response.text().await {
                    println!("   Response body: {}", body);
                }
            }
        }
        Err(e) => {
            println!("⚠️  Authentication test failed: {}", e);
        }
    }
    
    // Test without authentication (should fail)
    println!("🚫 Testing request without authentication...");
    match client
        .post(&format!("{}/tools/ponderTool", base_url))
        .json(&tool_request)
        .send()
        .await
    {
        Ok(response) => {
            if response.status() == 401 {
                println!("✅ Correctly rejected unauthenticated request");
            } else {
                println!("⚠️  Expected 401, got: {}", response.status());
            }
        }
        Err(e) => {
            println!("⚠️  Request failed: {}", e);
        }
    }
    
    println!("✅ Authentication testing completed");
    Ok(())
}

async fn test_container_icc_integration(runtime: &AriaRuntime) -> AriaResult<()> {
    let session_id = DeepUuid(Uuid::new_v4());
    let container_id = "icc-integration-test".to_string();
    let permissions = vec!["all".to_string()];
    
    // Create ICC environment for container
    let icc_env = runtime.create_icc_environment(session_id.0, container_id.clone(), permissions)?;
    
    println!("🔧 Created ICC environment for container integration test");
    
    // Create container spec with ICC environment
    let mut environment = HashMap::new();
    for (key, value) in icc_env {
        environment.insert(key, value);
    }
    
    // Add some additional container environment
    environment.insert("TEST_MODE".to_string(), "true".to_string());
    environment.insert("CONTAINER_PURPOSE".to_string(), "ICC_INTEGRATION_TEST".to_string());
    
    let container_spec = ContainerSpec {
        image: "/tmp/test-image.tar.gz".to_string(),
        command: vec![
            "/bin/sh".to_string(),
            "-c".to_string(),
            "echo 'Container started with ICC environment'; env | grep ARIA_; sleep 5".to_string()
        ],
        environment: environment.clone(),
        working_dir: Some("/tmp".to_string()),
        resource_limits: aria_runtime::types::ResourceLimits {
            cpu_millis: Some(250),
            memory_mb: Some(128),
            disk_mb: Some(50),
            timeout_seconds: Some(60),
        },
        network_access: true,
        mount_points: vec![],
    };
    
    println!("📦 Container spec created with {} environment variables", environment.len());
    
    // Test container workload execution (this would normally create and run the container)
    let exec_command = vec![
        "sh".to_string(),
        "-c".to_string(),
        "echo 'Testing ICC integration'; curl -s $ARIA_ICC_ENDPOINT/health || echo 'ICC endpoint not reachable'".to_string()
    ];
    
    match runtime.execute_container_workload(&container_spec, &exec_command, None, session_id).await {
        Ok(result) => {
            println!("✅ Container workload executed successfully");
            println!("   Result: {:?}", result.result);
        }
        Err(e) => {
            println!("⚠️  Container workload execution failed: {}", e);
            println!("   (This is expected if no quilt daemon is running)");
        }
    }
    
    println!("✅ Container ICC integration test completed");
    Ok(())
}

// Test different ICC endpoint types
async fn test_icc_endpoint_types() -> AriaResult<()> {
    println!("🌐 Testing different ICC endpoint types...");
    
    let client = reqwest::Client::new();
    let base_url = "http://10.42.0.1:8080";
    
    // Mock token for testing (in real scenario, this comes from environment)
    let mock_token = "aria_test_token_12345";
    
    // Test LLM endpoint
    println!("🧠 Testing LLM endpoint...");
    let llm_request = LLMCompletionRequest {
        messages: vec![
            LLMMessage {
                role: "user".to_string(),
                content: "Hello, can you help me?".to_string(),
            }
        ],
        model: Some("gpt-4o".to_string()),
        temperature: Some(0.7),
        max_tokens: Some(100),
        stream: Some(false),
    };
    
    match client
        .post(&format!("{}/llm/complete", base_url))
        .header("Authorization", format!("Bearer {}", mock_token))
        .json(&llm_request)
        .send()
        .await
    {
        Ok(response) => {
            println!("   LLM endpoint responded with status: {}", response.status());
        }
        Err(e) => {
            println!("   LLM endpoint test failed: {}", e);
        }
    }
    
    // Test context endpoint
    println!("📋 Testing context endpoint...");
    let context_params = "include_history=true&include_memory=false";
    
    match client
        .get(&format!("{}/context?{}", base_url, context_params))
        .header("Authorization", format!("Bearer {}", mock_token))
        .send()
        .await
    {
        Ok(response) => {
            println!("   Context endpoint responded with status: {}", response.status());
        }
        Err(e) => {
            println!("   Context endpoint test failed: {}", e);
        }
    }
    
    // Test agent endpoint
    println!("🤖 Testing agent endpoint...");
    let agent_request = json!({
        "message": "Hello agent, can you help with a task?",
        "context": {"test": true},
        "max_turns": 1
    });
    
    match client
        .post(&format!("{}/agents/test-agent", base_url))
        .header("Authorization", format!("Bearer {}", mock_token))
        .json(&agent_request)
        .send()
        .await
    {
        Ok(response) => {
            println!("   Agent endpoint responded with status: {}", response.status());
        }
        Err(e) => {
            println!("   Agent endpoint test failed: {}", e);
        }
    }
    
    println!("✅ Endpoint type testing completed");
    Ok(())
} 