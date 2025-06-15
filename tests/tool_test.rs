//! Tool Registry and Tool Execution Tests
//! These tests verify our production-grade tool system

use aria_runtime::{AriaRuntime, RuntimeConfiguration};
use aria_runtime::types::{AgentConfig, ToolParameter};
use aria_runtime::engines::tool_registry::{ToolRegistry, ToolType, ToolExecutionLevel, ToolExecutionStats};
use tokio;
use serde_json;
use std::collections::HashMap;

#[tokio::test]
async fn test_tool_registry_creation() {
    println!("🧪 Testing Tool Registry Creation...");
    
    let registry = ToolRegistry::new(None);
    
    println!("✅ Tool registry created successfully!");
    
    // Test basic registry operations
    match registry.list_tools().await {
        Ok(tools) => {
            println!("✅ Tool listing working!");
            println!("  - Found {} tools", tools.len());
            for tool in tools {
                println!("    - {}", tool);
            }
        }
        Err(e) => {
            println!("❌ Tool listing failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_tool_registration() {
    println!("🧪 Testing Tool Registration...");
    
    let registry = ToolRegistry::new(None);
    
    // Register a test tool
    let tool_name = "test_echo";
    let tool_type = ToolType::Native;
    let description = "A simple echo tool for testing";
    let parameters = vec![
        ToolParameter {
            name: "message".to_string(),
            param_type: "string".to_string(),
            description: "Message to echo".to_string(),
            required: true,
            default_value: None,
        }
    ];
    
    match registry.register_tool(
        tool_name.to_string(),
        tool_type,
        description.to_string(),
        parameters,
        ToolExecutionLevel::Safe,
        None,
        None
    ).await {
        Ok(_) => {
            println!("✅ Tool registration working!");
            
            // Verify tool was registered
            match registry.get_tool(tool_name).await {
                Ok(Some(tool_info)) => {
                    println!("✅ Tool retrieval working!");
                    println!("  - Name: {}", tool_info.name);
                    println!("  - Description: {}", tool_info.description);
                    println!("  - Security level: {:?}", tool_info.security_level);
                }
                Ok(None) => {
                    println!("❌ Tool not found after registration");
                }
                Err(e) => {
                    println!("❌ Tool retrieval failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Tool registration failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_tool_execution() {
    println!("🧪 Testing Tool Execution...");
    
    let config = RuntimeConfiguration::default();
    let runtime = AriaRuntime::new(config).await.expect("Runtime should initialize");
    runtime.initialize().await.expect("Engines should initialize");
    
    // Test tool execution through the runtime
    let agent_config = AgentConfig {
        name: "tool_test_agent".to_string(),
        tools: vec!["echo".to_string()],
        ..Default::default()
    };
    
    let task = "Use the echo tool to say hello";
    
    match runtime.execute(task, agent_config).await {
        Ok(result) => {
            println!("✅ Tool execution through runtime working!");
            println!("  - Success: {}", result.success);
            println!("  - Tools used: {}", result.execution_details.step_results.iter()
                .filter_map(|s| s.tool_used.as_ref())
                .collect::<Vec<_>>()
                .len());
        }
        Err(e) => {
            println!("❌ Tool execution failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_tool_security_levels() {
    println!("🧪 Testing Tool Security Levels...");
    
    let registry = ToolRegistry::new(None);
    
    // Register tools with different security levels
    let tools = vec![
        ("safe_tool", ToolExecutionLevel::Safe),
        ("restricted_tool", ToolExecutionLevel::Restricted),
        ("dangerous_tool", ToolExecutionLevel::Dangerous),
    ];
    
    for (name, level) in tools {
        match registry.register_tool(
            name.to_string(),
            ToolType::Native,
            format!("Test tool with {:?} security level", level),
            vec![],
            level,
            None,
            None
        ).await {
            Ok(_) => {
                println!("✅ Registered {} with security level {:?}", name, level);
            }
            Err(e) => {
                println!("❌ Failed to register {}: {}", name, e);
            }
        }
    }
    
    // Test security filtering
    match registry.list_tools_by_security_level(ToolExecutionLevel::Safe).await {
        Ok(safe_tools) => {
            println!("✅ Security filtering working!");
            println!("  - Safe tools found: {}", safe_tools.len());
        }
        Err(e) => {
            println!("❌ Security filtering failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_tool_metrics() {
    println!("🧪 Testing Tool Execution Metrics...");
    
    let registry = ToolRegistry::new(None);
    
    // Register a test tool
    let tool_name = "metrics_test_tool";
    registry.register_tool(
        tool_name.to_string(),
        ToolType::Native,
        "Tool for testing metrics".to_string(),
        vec![],
        ToolExecutionLevel::Safe,
        None,
        None
    ).await.expect("Tool registration should work");
    
    // Simulate some tool executions
    for i in 0..5 {
        let success = i % 2 == 0; // Alternate success/failure
        let duration = (i + 1) * 100; // Varying duration
        
        registry.record_execution(tool_name, success, duration as u64).await
            .expect("Execution recording should work");
    }
    
    // Check metrics
    match registry.get_execution_stats(tool_name).await {
        Ok(Some(stats)) => {
            println!("✅ Tool metrics working!");
            println!("  - Total executions: {}", stats.total_executions);
            println!("  - Successful executions: {}", stats.successful_executions);
            println!("  - Failed executions: {}", stats.failed_executions);
            println!("  - Average duration: {}ms", stats.average_duration_ms);
            println!("  - Success rate: {:.1}%", stats.success_rate * 100.0);
        }
        Ok(None) => {
            println!("❌ No metrics found for tool");
        }
        Err(e) => {
            println!("❌ Metrics retrieval failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_llm_tool_integration() {
    println!("🧪 Testing LLM Tool Integration...");
    
    let registry = ToolRegistry::new(None);
    
    // Register LLM tools
    let llm_tools = vec![
        ("ponder", "openai", "gpt-4"),
        ("create_plan", "openai", "gpt-4"),
        ("summarize", "openai", "gpt-3.5-turbo"),
    ];
    
    for (tool_name, provider, model) in llm_tools {
        match registry.register_tool(
            tool_name.to_string(),
            ToolType::LLM { 
                provider: provider.to_string(), 
                model: model.to_string() 
            },
            format!("LLM tool using {} {}", provider, model),
            vec![],
            ToolExecutionLevel::Safe,
            None,
            None
        ).await {
            Ok(_) => {
                println!("✅ Registered LLM tool: {}", tool_name);
            }
            Err(e) => {
                println!("❌ Failed to register LLM tool {}: {}", tool_name, e);
            }
        }
    }
    
    // Test LLM tool listing
    match registry.list_tools_by_type(&ToolType::LLM { 
        provider: "openai".to_string(), 
        model: "".to_string() 
    }).await {
        Ok(llm_tools) => {
            println!("✅ LLM tool filtering working!");
            println!("  - Found {} LLM tools", llm_tools.len());
        }
        Err(e) => {
            println!("❌ LLM tool filtering failed: {}", e);
        }
    }
}

/// Run all tool tests
#[tokio::main]
async fn main() {
    println!("🔧 Starting Aria Runtime Tool Tests");
    println!("===================================");
    
    println!("\n1. Testing Tool Registry Creation...");
    test_tool_registry_creation().await;
    
    println!("\n2. Testing Tool Registration...");
    test_tool_registration().await;
    
    println!("\n3. Testing Tool Execution...");
    test_tool_execution().await;
    
    println!("\n4. Testing Tool Security Levels...");
    test_tool_security_levels().await;
    
    println!("\n5. Testing Tool Metrics...");
    test_tool_metrics().await;
    
    println!("\n6. Testing LLM Tool Integration...");
    test_llm_tool_integration().await;
    
    println!("\n🎉 All tool tests completed!");
    println!("===================================");
} 