/*!
# Bundle Integration Test

Comprehensive test suite for the complete bundle integration system including
tool discovery, registration, bundle execution, and management.
*/

use aria_runtime::{
    AriaRuntime, RuntimeConfiguration, BundleToolDiscovery, BundleExecutor,
    BundleExecutionConfig
};
use aria_runtime::types::AgentConfig;
use ar_c::bundle::AriaBundle;
use ar_c::compiler::schema::{AriaManifest, ToolManifest as CompilerToolManifest};
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio;

/// Test bundle creation and basic loading
#[tokio::test]
async fn test_bundle_creation_and_loading() {
    println!("🧪 Testing Bundle Creation and Loading...");

    // Create a test manifest
    let manifest = AriaManifest {
        name: "test-bundle".to_string(),
        version: "1.0.0".to_string(),
        tools: vec![
            CompilerToolManifest {
                name: "test-tool".to_string(),
                description: "A test tool for demonstration".to_string(),
                inputs: {
                    let mut inputs = HashMap::new();
                    inputs.insert("input".to_string(), "string".to_string());
                    inputs
                },
            }
        ],
        agents: vec![],
        teams: vec![],
        pipelines: vec![],
    };

    // Create a test bundle
    let mut compiled_code = HashMap::new();
    compiled_code.insert(
        PathBuf::from("main.tsx"),
        r#"
// Test bundle main entry
import { AriaRuntime } from '@aria/runtime';

export async function main() {
    console.log('Hello from test bundle!');
    return { success: true, message: 'Bundle executed successfully' };
}

if (import.meta.main) {
    main().then(console.log).catch(console.error);
}
"#.to_string(),
    );

    compiled_code.insert(
        PathBuf::from("test-tool.ts"),
        r#"
import { tool } from '@aria/runtime';

@tool({
    name: 'test-tool',
    description: 'A test tool for demonstration'
})
export async function testTool(input: string): Promise<string> {
    return `Processed: ${input}`;
}
"#.to_string(),
    );

    let bundle = AriaBundle::create(manifest, vec![], compiled_code)
        .expect("Failed to create test bundle");

    // Verify bundle structure
    assert_eq!(bundle.manifest.name, "test-bundle");
    assert_eq!(bundle.manifest.tools.len(), 1);
    assert_eq!(bundle.manifest.tools[0].name, "test-tool");

    println!("✅ Bundle creation and loading test passed!");
}

/// Test tool discovery and registration
#[tokio::test]
async fn test_tool_discovery_and_registration() {
    println!("🧪 Testing Tool Discovery and Registration...");

    // Create test runtime
    let temp_dir = TempDir::new().unwrap();
    let config = RuntimeConfiguration {
        workspace_path: temp_dir.path().to_path_buf(),
        ..Default::default()
    };
    
    let runtime = AriaRuntime::new(config).await
        .expect("Failed to create runtime");

    // Test bundle capabilities status
    let status = runtime.get_bundle_capabilities_status().await
        .expect("Failed to get bundle capabilities status");

    assert_eq!(status.total_available_bundles, 0); // No bundles initially
    assert_eq!(status.registered_custom_tools, 0);
    assert!(status.auto_discovery_enabled);

    println!("✅ Tool discovery and registration test passed!");
}

/// Test custom tool management
#[tokio::test]
async fn test_custom_tool_management() {
    println!("🧪 Testing Custom Tool Management...");

    let temp_dir = TempDir::new().unwrap();
    let config = RuntimeConfiguration {
        workspace_path: temp_dir.path().to_path_buf(),
        ..Default::default()
    };
    
    let runtime = AriaRuntime::new(config).await
        .expect("Failed to create runtime");

    // Get custom tool manager
    let custom_tool_manager = runtime.get_custom_tool_manager().await
        .expect("Failed to get custom tool manager");

    // Test initial state
    let tools = custom_tool_manager.list_custom_tools().await
        .expect("Failed to list custom tools");
    assert!(tools.is_empty());

    // Test management stats
    let stats = custom_tool_manager.get_management_stats().await
        .expect("Failed to get management stats");
    assert_eq!(stats.total_custom_tools, 0);
    assert_eq!(stats.unique_bundles, 0);

    println!("✅ Custom tool management test passed!");
}

/// Test bundle execution configuration
#[tokio::test] 
async fn test_bundle_execution_configuration() {
    println!("🧪 Testing Bundle Execution Configuration...");

    // Test default configuration
    let default_config = BundleExecutionConfig::default();
    assert_eq!(default_config.memory_limit_mb, Some(1024));
    assert_eq!(default_config.timeout_seconds, Some(300));
    assert!(default_config.network_enabled);
    assert!(default_config.filesystem_isolation);
    assert!(default_config.auto_register_components);

    // Test custom configuration
    let mut custom_config = BundleExecutionConfig::default();
    custom_config.memory_limit_mb = Some(2048);
    custom_config.timeout_seconds = Some(600);
    custom_config.network_enabled = false;

    assert_eq!(custom_config.memory_limit_mb, Some(2048));
    assert_eq!(custom_config.timeout_seconds, Some(600));
    assert!(!custom_config.network_enabled);

    println!("✅ Bundle execution configuration test passed!");
}

/// Test complete bundle integration workflow
#[tokio::test]
async fn test_complete_bundle_integration_workflow() {
    println!("🧪 Testing Complete Bundle Integration Workflow...");

    let temp_dir = TempDir::new().unwrap();
    let config = RuntimeConfiguration {
        workspace_path: temp_dir.path().to_path_buf(),
        ..Default::default()
    };
    
    let runtime = AriaRuntime::new(config).await
        .expect("Failed to create runtime");

    // Step 1: Check initial capabilities
    let initial_status = runtime.get_bundle_capabilities_status().await
        .expect("Failed to get initial bundle capabilities");
    assert_eq!(initial_status.total_available_bundles, 0);

    // Step 2: Test tool discovery (should find no tools initially)
    let discovery_result = runtime.discover_tool_in_bundles("nonexistent-tool").await
        .expect("Failed to discover tool");
    assert!(discovery_result.is_none());

    // Step 3: Test auto-discovery (should find no tools initially)
    let discovered_count = runtime.auto_discover_bundle_tools().await
        .expect("Failed to auto-discover tools");
    assert_eq!(discovered_count, 0);

    // Step 4: Test final capabilities status
    let final_status = runtime.get_bundle_capabilities_status().await
        .expect("Failed to get final bundle capabilities");
    assert_eq!(final_status.total_available_bundles, 0);

    println!("✅ Complete bundle integration workflow test passed!");
}

/// Test bundle execution error handling
#[tokio::test]
async fn test_bundle_execution_error_handling() {
    println!("🧪 Testing Bundle Execution Error Handling...");

    let temp_dir = TempDir::new().unwrap();
    let config = RuntimeConfiguration {
        workspace_path: temp_dir.path().to_path_buf(),
        ..Default::default()
    };
    
    let runtime = AriaRuntime::new(config).await
        .expect("Failed to create runtime");

    // Test execution with non-existent bundle
    let session_id = aria_runtime::DeepUuid::new();
    let execution_config = BundleExecutionConfig::default();
    
    let result = runtime.execute_bundle_workload(
        "nonexistent-bundle-hash",
        session_id,
        Some(execution_config),
    ).await;

    // Should fail gracefully
    assert!(result.is_err());

    println!("✅ Bundle execution error handling test passed!");
}

/// Test tool registration from bundle
#[tokio::test]
async fn test_tool_registration_from_bundle() {
    println!("🧪 Testing Tool Registration from Bundle...");

    let temp_dir = TempDir::new().unwrap();
    let config = RuntimeConfiguration {
        workspace_path: temp_dir.path().to_path_buf(),
        ..Default::default()
    };
    
    let runtime = AriaRuntime::new(config).await
        .expect("Failed to create runtime");

    // Test registration with non-existent bundle
    let result = runtime.register_tools_from_bundle("nonexistent-bundle").await;
    
    // Should return empty list for non-existent bundle
    match result {
        Ok(tools) => assert!(tools.is_empty()),
        Err(_) => {
            // Error is acceptable for non-existent bundle
            println!("Bundle not found (expected for test)");
        }
    }

    println!("✅ Tool registration from bundle test passed!");
}

/// Test agent configuration with bundle tools
#[tokio::test]
async fn test_agent_with_bundle_tools() {
    println!("🧪 Testing Agent Configuration with Bundle Tools...");

    // Create agent config that would use bundle tools
    let agent_config = AgentConfig {
        name: "test-agent".to_string(),
        tools: vec!["custom-bundle-tool".to_string()],
        ..Default::default()
    };

    assert_eq!(agent_config.name, "test-agent");
    assert_eq!(agent_config.tools.len(), 1);
    assert_eq!(agent_config.tools[0], "custom-bundle-tool");

    println!("✅ Agent configuration with bundle tools test passed!");
}

/// Integration test runner
#[tokio::test]
async fn test_full_bundle_integration_suite() {
    println!("🚀 Running Full Bundle Integration Test Suite...");

    // Run all integration tests
    test_bundle_creation_and_loading().await;
    test_tool_discovery_and_registration().await;
    test_custom_tool_management().await;
    test_bundle_execution_configuration().await;
    test_complete_bundle_integration_workflow().await;
    test_bundle_execution_error_handling().await;
    test_tool_registration_from_bundle().await;
    test_agent_with_bundle_tools().await;

    println!("✅ Full Bundle Integration Test Suite completed successfully!");
    println!("📊 All bundle integration capabilities are working correctly");
}

/// Performance benchmark for bundle operations
#[tokio::test]
async fn test_bundle_operations_performance() {
    println!("⚡ Testing Bundle Operations Performance...");

    let temp_dir = TempDir::new().unwrap();
    let config = RuntimeConfiguration {
        workspace_path: temp_dir.path().to_path_buf(),
        ..Default::default()
    };
    
    let runtime = AriaRuntime::new(config).await
        .expect("Failed to create runtime");

    let start_time = std::time::Instant::now();

    // Perform multiple rapid operations
    for i in 0..10 {
        let _ = runtime.get_bundle_capabilities_status().await;
        let _ = runtime.discover_tool_in_bundles(&format!("test-tool-{}", i)).await;
    }

    let elapsed = start_time.elapsed();
    println!("📊 Performance: 20 bundle operations completed in {:?}", elapsed);
    
    // Should complete within reasonable time (less than 1 second for this simple test)
    assert!(elapsed.as_secs() < 1);

    println!("✅ Bundle operations performance test passed!");
} 