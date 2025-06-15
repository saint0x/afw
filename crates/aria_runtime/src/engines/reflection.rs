use crate::errors::{AriaResult, AriaError, ErrorCode, ErrorCategory, ErrorSeverity};
use crate::types::*;
use crate::engines::{ToolRegistry, RuntimeEngine, ReflectionEngineInterface};
use crate::engines::tool_registry::ToolRegistryInterface;
use async_trait::async_trait;
use std::sync::Arc;

/// Reflection engine for self-assessment and improvement using ponderTool
pub struct ReflectionEngine {
    tool_registry: Arc<ToolRegistry>,
}

impl ReflectionEngine {
    pub fn new(tool_registry: Arc<ToolRegistry>) -> Self {
        Self {
            tool_registry,
        }
    }

    /// Create sophisticated ponder query based on step success/failure (matching Symphony)
    fn create_ponder_query(&self, step: &ExecutionStep) -> String {
        if step.success {
            format!(
                "My last action to '{}' succeeded. Was this the most optimal and efficient approach? Analyze my method and suggest any potential optimizations or alternative strategies for similar future tasks.",
                step.description
            )
        } else {
            format!(
                "My attempt to '{}' failed with the error: \"{}\". Analyze the root cause of this failure. Consider the tool used, the parameters, and the overall goal. Suggest a concrete, actionable correction strategy. Should I retry, use a different tool, modify the parameters, or abort the plan?",
                step.description,
                step.error.as_ref().unwrap_or(&"Unknown error".to_string())
            )
        }
    }

    /// Parse ponder result into structured reflection (matching Symphony's logic)
    fn parse_ponder_result_to_reflection(
        &self,
        step: &ExecutionStep,
        ponder_result: &serde_json::Value,
    ) -> AriaResult<Reflection> {
        let conclusion = ponder_result.get("conclusion")
            .ok_or_else(|| AriaError::new(
                ErrorCode::ReflectionFailed,
                ErrorCategory::Reflection,
                ErrorSeverity::Medium,
                "Ponder result missing conclusion"
            ))?;

        let summary = conclusion.get("summary")
            .and_then(|s| s.as_str())
            .unwrap_or("No summary provided by ponderTool")
            .to_string();

        let confidence = conclusion.get("confidence")
            .and_then(|c| c.as_f64())
            .unwrap_or(0.7) as f32;

        let next_steps = conclusion.get("nextSteps")
            .and_then(|ns| ns.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
            .unwrap_or_else(|| vec!["Continue with current approach".to_string()]);

        // Determine suggested action based on ponder analysis (like Symphony)
        let suggested_action = if !step.success {
            let summary_lower = summary.to_lowercase();
            if summary_lower.contains("retry") {
                SuggestedAction::Retry
            } else if summary_lower.contains("modify") || summary_lower.contains("alternative") {
                SuggestedAction::ModifyPlan
            } else if summary_lower.contains("abort") {
                SuggestedAction::Abort
            } else if summary_lower.contains("different tool") {
                SuggestedAction::UseDifferentTool
            } else {
                SuggestedAction::Retry // Default for failures
            }
        } else {
            SuggestedAction::Continue
        };

        // Create assessment based on step outcome and ponder analysis
        let assessment = ReflectionAssessment {
            performance: if step.success { PerformanceLevel::Good } else { PerformanceLevel::Poor },
            quality: if step.success {
                if summary.contains("optimal") || summary.contains("excellent") {
                    QualityLevel::Optimal
                } else {
                    QualityLevel::Good
                }
            } else {
                QualityLevel::Wrong
            },
            efficiency: if step.success {
                if summary.contains("efficient") {
                    EfficiencyLevel::Efficient
                } else {
                    EfficiencyLevel::Acceptable
                }
            } else {
                EfficiencyLevel::Inefficient
            },
            suggested_improvements: next_steps.clone(),
        };

        Ok(Reflection {
            id: uuid::Uuid::new_v4(),
            step_id: step.step_id,
            assessment,
            suggested_action,
            reasoning: summary,
            confidence,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            improvements: next_steps,
        })
    }

    /// Create fallback reflection when ponder tool fails (matching Symphony)
    fn create_fallback_reflection(&self, step: &ExecutionStep, reason: &str) -> Reflection {
        Reflection {
            id: uuid::Uuid::new_v4(),
            step_id: step.step_id,
            assessment: ReflectionAssessment {
                performance: PerformanceLevel::Poor,
                quality: QualityLevel::Wrong,
                efficiency: EfficiencyLevel::Inefficient,
                suggested_improvements: vec!["Reflection tool failed - manual review needed".to_string()],
            },
            suggested_action: SuggestedAction::Abort,
            reasoning: format!("Reflection failed: {}", reason),
            confidence: 0.9, // High confidence that there's a problem
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            improvements: vec!["Check ponder tool availability and configuration".to_string()],
        }
    }
}

#[async_trait]
impl RuntimeEngine for ReflectionEngine {
    async fn initialize(&self) -> AriaResult<()> {
        // Verify ponder tool is available
        if !self.tool_registry.is_tool_available("ponderTool").await {
            return Err(AriaError::new(
                ErrorCode::ReflectionFailed,
                ErrorCategory::Reflection,
                ErrorSeverity::High,
                "ponderTool not available for reflection engine"
            ));
        }
        Ok(())
    }
    
    fn get_dependencies(&self) -> Vec<String> {
        vec!["tool_registry".to_string(), "ponderTool".to_string()]
    }
    
    fn get_state(&self) -> String {
        "ready".to_string()
    }
    
    async fn health_check(&self) -> AriaResult<bool> {
        Ok(self.tool_registry.is_tool_available("ponderTool").await)
    }
    
    async fn shutdown(&self) -> AriaResult<()> {
        Ok(())
    }
}

#[async_trait]
impl ReflectionEngineInterface for ReflectionEngine {
    async fn reflect(
        &self,
        step: &ExecutionStep,
        context: &RuntimeContext,
    ) -> AriaResult<Reflection> {
        // Create sophisticated ponder query like Symphony does
        let query = self.create_ponder_query(step);

        // Prepare context for ponder tool (matching Symphony's context structure)
        let ponder_context = serde_json::json!({
            "agentConfig": {
                "name": context.agent_config.name,
                "tools": context.agent_config.tools
            },
            "fullPlan": context.current_plan.as_ref().map(|p| serde_json::json!({
                "id": p.id,
                "task_description": p.task_description,
                "steps": p.steps.len()
            })),
            "executionHistory": context.execution_history.iter().map(|s| serde_json::json!({
                "stepId": s.step_id,
                "description": s.description,
                "success": s.success,
                "tool_used": s.tool_used
            })).collect::<Vec<_>>(),
            "currentStep": context.current_step,
            "totalSteps": context.total_steps
        });

        let ponder_params = serde_json::json!({
            "query": query,
            "context": ponder_context
        });

        // Execute ponder tool like Symphony does
        match self.tool_registry.execute_tool("ponderTool", ponder_params).await {
            Ok(ponder_result) => {
                if !ponder_result.success {
                    return Ok(self.create_fallback_reflection(step, "Ponder tool execution failed"));
                }

                if let Some(result) = ponder_result.result {
                    match self.parse_ponder_result_to_reflection(step, &result) {
                        Ok(reflection) => Ok(reflection),
                        Err(_) => Ok(self.create_fallback_reflection(step, "Failed to parse ponder result"))
                    }
                } else {
                    Ok(self.create_fallback_reflection(step, "Ponder tool returned no result"))
                }
            },
            Err(_) => {
                Ok(self.create_fallback_reflection(step, "Ponder tool execution threw an error"))
            }
        }
    }
}
