use crate::deep_size::DeepUuid;
use crate::engines::tool_registry::ToolRegistryInterface;
use crate::engines::{Engine, ReflectionEngineInterface};
use crate::errors::{AriaError, AriaResult, ErrorCategory, ErrorCode, ErrorSeverity};
use crate::types::*;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

/// Reflection engine for self-assessment and improvement using ponderTool
pub struct ReflectionEngine {
    tool_registry: Arc<dyn ToolRegistryInterface>,
}

impl ReflectionEngine {
    pub fn new(tool_registry: Arc<dyn ToolRegistryInterface>) -> Self {
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
                ErrorCode::ReflectionError,
                ErrorCategory::Reflection,
                ErrorSeverity::High,
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
            requires_replanning: !step.success,
        };

        Ok(Reflection {
            id: DeepUuid(Uuid::new_v4()),
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
            id: DeepUuid(Uuid::new_v4()),
            step_id: step.step_id,
            assessment: ReflectionAssessment {
                performance: PerformanceLevel::Poor,
                quality: QualityLevel::Wrong,
                efficiency: EfficiencyLevel::Inefficient,
                suggested_improvements: vec!["Reflection tool failed - manual review needed".to_string()],
                requires_replanning: true,
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

impl Engine for ReflectionEngine {
    fn initialize(&self) -> bool {
        // TODO: Verify ponder tool is available (async version would be better)
        true
    }
    
    fn get_dependencies(&self) -> Vec<String> {
        vec!["tool_registry".to_string(), "ponderTool".to_string()]
    }
    
    fn get_state(&self) -> String {
        "ready".to_string()
    }
    
    fn health_check(&self) -> bool {
        // TODO: Check if ponderTool is available (async version would be better)
        true
    }
    
    fn shutdown(&self) -> bool {
        true
    }
}

#[async_trait]
impl ReflectionEngineInterface for ReflectionEngine {
    async fn reflect(
        &self,
        step: &ExecutionStep,
        context: &RuntimeContext,
    ) -> AriaResult<Reflection> {
        // Ensure ponderTool is available
        if !self.tool_registry.is_tool_available("ponderTool").await {
            return Err(AriaError::new(
                ErrorCode::ToolNotFound,
                ErrorCategory::Tool,
                ErrorSeverity::High,
                "ponderTool not available for reflection",
            ));
        }

        let last_step = context.execution_history.last().ok_or_else(|| {
            AriaError::new(
                ErrorCode::ContextError,
                ErrorCategory::Context,
                ErrorSeverity::High,
                "No execution history available for reflection",
            )
        })?;

        let ponder_query = self.create_ponder_query(last_step);

        let ponder_result = self
            .tool_registry
            .execute_tool(
                "ponderTool",
                serde_json::json!({ "query": ponder_query }).into(),
            )
            .await?;

        if !ponder_result.success {
            return Err(AriaError::new(
                ErrorCode::ToolExecutionError,
                ErrorCategory::Tool,
                ErrorSeverity::Medium,
                &format!(
                    "ponderTool failed: {}",
                    ponder_result.error.unwrap_or_else(|| "Unknown error".to_string())
                ),
            ));
        }

        let conclusion = ponder_result
            .result
            .as_ref()
            .and_then(|r| r.get("conclusion"))
            .and_then(|c| c.as_str())
            .unwrap_or("No conclusion reached.")
            .to_string();

        let suggested_action = match conclusion.to_lowercase().as_str() {
            "continue" => SuggestedAction::Continue,
            "retry" => SuggestedAction::Retry,
            _ => SuggestedAction::ModifyPlan,
        };

        Ok(Reflection {
            id: DeepUuid(Uuid::new_v4()),
            step_id: last_step.step_id,
            assessment: ReflectionAssessment {
                performance: PerformanceLevel::Good,
                quality: QualityLevel::Good,
                efficiency: EfficiencyLevel::Efficient,
                suggested_improvements: vec![],
                requires_replanning: false,
            },
            suggested_action,
            reasoning: conclusion,
            confidence: ponder_result
                .result
                .as_ref()
                .and_then(|r| r.get("confidence"))
                .and_then(|c| c.as_f64())
                .unwrap_or(0.7) as f32,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            improvements: vec![],
        })
    }
}

// ReflectionEngineInterface is defined in engines/mod.rs
