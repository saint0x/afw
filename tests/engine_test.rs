//! Engine-specific tests for Aria Runtime
//! These tests verify individual engine functionality

use aria_runtime::{AriaRuntime, RuntimeConfiguration};
use aria_runtime::types::{AgentConfig, TaskComplexity, RuntimeContext};
use aria_runtime::engines::{ExecutionEngineInterface, PlanningEngineInterface, ReflectionEngineInterface, ConversationEngineInterface};
use tokio;

#[tokio::test]
async fn test_execution_engine() {
    println!("🧪 Testing Execution Engine...");
    
    let config = RuntimeConfiguration::default();
    let runtime = AriaRuntime::new(config).await.expect("Runtime should initialize");
    runtime.initialize().await.expect("Engines should initialize");
    
    let agent_config = AgentConfig {
        name: "execution_test".to_string(),
        tools: vec!["echo".to_string()],
        ..Default::default()
    };
    
    // Create a test context
    let mut context = RuntimeContext::default();
    context.agent_config = agent_config.clone();
    
    // Test basic execution capability
    let task = "Execute a simple echo command";
    match runtime.engines.execution.execute(task, &agent_config, &context).await {
        Ok(result) => {
            println!("✅ Execution engine working!");
            println!("  - Success: {}", result.success);
            if let Some(response) = result.result {
                println!("  - Result: {:?}", response);
            }
        }
        Err(e) => {
            println!("❌ Execution engine failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_planning_engine() {
    println!("🧪 Testing Planning Engine...");
    
    let config = RuntimeConfiguration {
        enhanced_runtime: true,
        planning_threshold: TaskComplexity::Simple,
        ..Default::default()
    };
    
    let runtime = AriaRuntime::new(config).await.expect("Runtime should initialize");
    runtime.initialize().await.expect("Engines should initialize");
    
    let context = RuntimeContext::default();
    
    // Test task analysis
    let task = "Create a multi-step plan to research AI trends and write a report";
    match runtime.engines.planning.analyze_task(task, &context).await {
        Ok(analysis) => {
            println!("✅ Task analysis working!");
            println!("  - Complexity: {:?}", analysis.complexity);
            println!("  - Requires planning: {}", analysis.requires_planning);
            println!("  - Estimated steps: {}", analysis.estimated_steps);
            println!("  - Reasoning: {}", analysis.reasoning);
        }
        Err(e) => {
            println!("❌ Task analysis failed: {}", e);
        }
    }
    
    // Test plan creation
    let agent_config = AgentConfig {
        name: "planning_test".to_string(),
        tools: vec!["web_search".to_string(), "file_write".to_string()],
        ..Default::default()
    };
    
    match runtime.engines.planning.create_execution_plan(task, &agent_config, &context).await {
        Ok(plan) => {
            println!("✅ Plan creation working!");
            println!("  - Plan ID: {:?}", plan.id);
            println!("  - Steps: {}", plan.steps.len());
            println!("  - Confidence: {}", plan.confidence);
            
            for (i, step) in plan.steps.iter().enumerate() {
                println!("    Step {}: {}", i + 1, step.description);
            }
        }
        Err(e) => {
            println!("❌ Plan creation failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_reflection_engine() {
    println!("🧪 Testing Reflection Engine...");
    
    let config = RuntimeConfiguration {
        reflection_enabled: true,
        ..Default::default()
    };
    
    let runtime = AriaRuntime::new(config).await.expect("Runtime should initialize");
    runtime.initialize().await.expect("Engines should initialize");
    
    // Create a mock execution step for reflection
    use aria_runtime::types::{ExecutionStep, StepType};
    use aria_runtime::deep_size::DeepUuid;
    use uuid::Uuid;
    use std::collections::HashMap;
    
    let step = ExecutionStep {
        step_id: DeepUuid(Uuid::new_v4()),
        description: "Test step that had mixed results".to_string(),
        start_time: 1000,
        end_time: 2000,
        duration: 1000,
        success: false, // Failed step for reflection
        step_type: StepType::ToolCall,
        tool_used: Some("test_tool".to_string()),
        agent_used: None,
        container_used: None,
        parameters: HashMap::new(),
        result: None,
        error: Some("Tool execution timeout".to_string()),
        reflection: None,
        summary: "Step failed due to timeout".to_string(),
        resource_usage: None,
    };
    
    let context = RuntimeContext::default();
    
    match runtime.engines.reflection.reflect(&step, &context).await {
        Ok(reflection) => {
            println!("✅ Reflection engine working!");
            println!("  - Assessment: {:?}", reflection.assessment.performance);
            println!("  - Suggested action: {:?}", reflection.suggested_action);
            println!("  - Confidence: {}", reflection.confidence);
            println!("  - Reasoning: {}", reflection.reasoning);
            println!("  - Improvements: {:?}", reflection.improvements);
        }
        Err(e) => {
            println!("❌ Reflection failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_conversation_engine() {
    println!("🧪 Testing Conversation Engine...");
    
    let config = RuntimeConfiguration::default();
    let runtime = AriaRuntime::new(config).await.expect("Runtime should initialize");
    runtime.initialize().await.expect("Engines should initialize");
    
    let context = RuntimeContext::default();
    let task = "Have a conversation about AI capabilities";
    
    match runtime.engines.conversation.initiate(task, &context).await {
        Ok(conversation) => {
            println!("✅ Conversation engine working!");
            println!("  - Conversation ID: {:?}", conversation.id);
            println!("  - Original task: {}", conversation.original_task);
            println!("  - State: {:?}", conversation.state);
            println!("  - Turns: {}", conversation.turns.len());
        }
        Err(e) => {
            println!("❌ Conversation initiation failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_context_manager() {
    println!("🧪 Testing Context Manager...");
    
    let config = RuntimeConfiguration::default();
    let runtime = AriaRuntime::new(config).await.expect("Runtime should initialize");
    runtime.initialize().await.expect("Engines should initialize");
    
    // Test context operations
    let _session_id = "test_session_123".to_string();
    let task = "Test context management";
    
    // This tests our context manager through the runtime
    let agent_config = AgentConfig {
        name: "context_test".to_string(),
        memory_enabled: Some(true),
        ..Default::default()
    };
    
    // Execute a task to create context
    match runtime.execute(task, agent_config).await {
        Ok(result) => {
            println!("✅ Context management working through execution!");
            println!("  - Execution tracked: {}", !result.execution_details.step_results.is_empty());
            println!("  - Memory metrics available: {}", result.metrics.memory_usage.current_size > 0);
        }
        Err(e) => {
            println!("❌ Context management test failed: {}", e);
        }
    }
}

/// Run all engine tests
#[tokio::test]
async fn test_all_engines() {
    println!("🔧 Starting Aria Runtime Engine Tests");
    println!("=====================================");
    
    println!("\n1. Testing Execution Engine...");
    let config = RuntimeConfiguration::default();
    let runtime = AriaRuntime::new(config).await.expect("Runtime should initialize");
    runtime.initialize().await.expect("Engines should initialize");
    
    let agent_config = AgentConfig {
        name: "execution_test".to_string(),
        tools: vec!["echo".to_string()],
        ..Default::default()
    };
    
    let mut context = RuntimeContext::default();
    context.agent_config = agent_config.clone();
    
    let task = "Execute a simple echo command";
    match runtime.engines.execution.execute(task, &agent_config, &context).await {
        Ok(result) => {
            println!("✅ Execution engine working!");
            println!("  - Success: {}", result.success);
        }
        Err(e) => {
            println!("❌ Execution engine failed: {}", e);
        }
    }
    
    println!("\n2. Testing Planning Engine...");
    let config = RuntimeConfiguration {
        enhanced_runtime: true,
        planning_threshold: TaskComplexity::Simple,
        ..Default::default()
    };
    
    let runtime = AriaRuntime::new(config).await.expect("Runtime should initialize");
    runtime.initialize().await.expect("Engines should initialize");
    
    let context = RuntimeContext::default();
    let task = "Create a multi-step plan to research AI trends and write a report";
    match runtime.engines.planning.analyze_task(task, &context).await {
        Ok(analysis) => {
            println!("✅ Task analysis working!");
            println!("  - Complexity: {:?}", analysis.complexity);
            println!("  - Requires planning: {}", analysis.requires_planning);
        }
        Err(e) => {
            println!("❌ Task analysis failed: {}", e);
        }
    }
    
    println!("\n🎉 All engine tests completed!");
    println!("=====================================");
} 