/*!
# Orchestration Layer

This crate provides the orchestration layer that manages DAG planning and reinforcement learning loops.
It wraps the aria_runtime and integrates with the token_api for Quilt communication.
*/

use aria_runtime::{AriaRuntime, RuntimeConfig, AriaResult};
use token_api::{TokenApi, TokenResult};

pub struct Orchestrator {
    runtime: AriaRuntime,
    token_api: TokenApi,
}

impl Orchestrator {
    pub async fn new(config: RuntimeConfig) -> AriaResult<Self> {
        let runtime = AriaRuntime::new(config).await?;
        let token_api = TokenApi::new().await
            .map_err(|e| aria_runtime::AriaError::Quilt { 
                message: e.to_string(), 
                token: None 
            })?;
        
        Ok(Self {
            runtime,
            token_api,
        })
    }

    pub async fn execute_task(&self, task: &str) -> AriaResult<serde_json::Value> {
        // TODO: Implement orchestration logic
        // 1. Plan DAG with runtime
        // 2. Request tokens from Quilt via token_api
        // 3. Execute task with resource isolation
        // 4. Return results
        
        let result = self.runtime.execute(task).await?;
        Ok(result.result.unwrap_or_default())
    }
} 