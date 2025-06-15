/*!
# Orchestration Layer

This crate provides the orchestration layer that manages DAG planning and reinforcement learning loops.
It wraps the aria_runtime and integrates with the token_api for Quilt communication.
*/

use aria_runtime::{AriaRuntime, RuntimeConfiguration, AriaResult};
use aria_runtime::errors::{AriaError, ErrorCode, ErrorCategory, ErrorSeverity};
use aria_runtime::types::AgentConfig;
use token_api::TokenApi;

pub struct Orchestrator {
    runtime: AriaRuntime,
    token_api: TokenApi,
}

impl Orchestrator {
    pub async fn new(config: RuntimeConfiguration) -> AriaResult<Self> {
        // TODO: For now, return an error since engines are not implemented yet
        if true {
            let token_api = TokenApi::new().await.map_err(|_| {
                AriaError::new(
                    ErrorCode::SystemInitializationFailure,
                    ErrorCategory::System,
                    ErrorSeverity::Critical,
                    "Failed to initialize TokenApi"
                )
            })?;
            
            Ok(Self {
                runtime: AriaRuntime::new(config).await?,
                token_api,
            })
        } else {
            Err(AriaError::new(
                ErrorCode::SystemInitializationFailure,
                ErrorCategory::System,
                ErrorSeverity::Critical,
                "Orchestrator failed to initialize"
            ))
        }
    }

    pub async fn execute_task(&self, task: &str) -> AriaResult<serde_json::Value> {
        // TODO: Implement orchestration logic
        // 1. Plan DAG with runtime
        // 2. Request tokens from Quilt via token_api
        // 3. Execute task with resource isolation
        // 4. Return results
        
        // TODO: Implement proper task execution with agent config
        let agent_config = AgentConfig {
            name: "default".to_string(),
            system_prompt: None,
            directives: None,
            tools: vec![],
            agents: vec![],
            llm: Default::default(),
            max_iterations: None,
            timeout_ms: None,
            memory_limit: None,
            agent_type: Some("default".to_string()),
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
                            "summary": step.summary
                        }))
                    } else {
                        Ok(serde_json::json!({
                            "success": true,
                            "message": "Task completed successfully"
                        }))
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": true,
                        "message": "Task completed successfully"
                    }))
                }
            }
            false => Ok(serde_json::json!({
                "success": false,
                "error": result.error.unwrap_or_else(|| "Unknown error".to_string())
            })),
        }
    }
} 