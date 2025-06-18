/*!
# Orchestration Layer

This crate provides the orchestration layer that manages DAG planning and reinforcement learning loops.
It wraps the aria_runtime for streamlined task execution.
*/

use aria_runtime::{AriaRuntime, RuntimeConfiguration, AriaResult};
use aria_runtime::errors::{AriaError, ErrorCode, ErrorCategory, ErrorSeverity};
use aria_runtime::types::AgentConfig;

pub struct Orchestrator {
    runtime: AriaRuntime,
}

impl Orchestrator {
    pub async fn new(config: RuntimeConfiguration) -> AriaResult<Self> {
        Ok(Self {
            runtime: AriaRuntime::new(config).await?,
        })
    }

    pub async fn execute_task(&self, task: &str) -> AriaResult<serde_json::Value> {
        // Direct execution through runtime - no token coordination needed
        // Quilt handles resource management internally through sync engine
        
        let agent_config = AgentConfig {
            name: "orchestrator".to_string(),
            system_prompt: Some("You are an intelligent orchestrator executing tasks efficiently.".to_string()),
            directives: None,
            tools: vec![],
            agents: vec![],
            llm: Default::default(),
            max_iterations: None,
            timeout_ms: None,
            memory_limit: None,
            agent_type: Some("orchestrator".to_string()),
            capabilities: vec![],
            memory_enabled: Some(false),
        };
        
        let result = self.runtime.execute(task, agent_config).await?;
        
        // Extract meaningful result from RuntimeResult
        match result.success {
            true => {
                // Try to get result from the first successful step
                if let Some(step) = result.execution_details.step_results.first() {
                    if let Some(step_result) = &step.result {
                        Ok(serde_json::json!({
                            "success": true,
                            "result": step_result,
                            "summary": step.summary,
                            "execution_time_ms": step.duration,
                            "steps_completed": result.execution_details.completed_steps
                        }))
                    } else {
                        Ok(serde_json::json!({
                            "success": true,
                            "message": "Task completed successfully",
                            "steps_completed": result.execution_details.completed_steps
                        }))
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": true,
                        "message": "Task completed successfully",
                        "execution_mode": result.mode
                    }))
                }
            }
            false => Ok(serde_json::json!({
                "success": false,
                "error": result.error.unwrap_or_else(|| "Unknown error".to_string()),
                "failed_steps": result.execution_details.failed_steps
            })),
        }
    }

    /// Get orchestrator runtime status
    pub fn get_status(&self) -> serde_json::Value {
        serde_json::json!({
            "status": "ready",
            "capabilities": ["task_execution", "container_orchestration", "intelligent_planning"],
            "version": env!("CARGO_PKG_VERSION")
        })
    }
} 