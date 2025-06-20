// Placeholder for planning engine implementation
// This will be implemented in Phase 2: Advanced Planning

use crate::deep_size::DeepUuid;
use crate::engines::tool_registry::ToolRegistryInterface;
use crate::engines::{Engine, PlanningEngineInterface};
use crate::errors::{AriaError, AriaResult, ErrorCategory, ErrorCode, ErrorSeverity};
use crate::types::*;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Planning engine for task analysis and execution planning
pub struct PlanningEngine {
    enabled: bool,
    tool_registry: Arc<dyn ToolRegistryInterface>,
}

impl PlanningEngine {
    pub fn new(tool_registry: Arc<dyn ToolRegistryInterface>) -> Self {
        Self {
            enabled: true,
            tool_registry,
        }
    }

    /// Parse raw plan JSON into structured steps (matching Symphony's logic)
    fn parse_raw_plan_to_steps(
        &self,
        raw_plan: &str,
        agent_config: &AgentConfig,
    ) -> AriaResult<Vec<PlannedStep>> {
        if raw_plan.is_empty() {
            return Err(AriaError::new(
                ErrorCode::PlanningFailure,
                ErrorCategory::Planning,
                ErrorSeverity::Medium,
                "Received empty plan string",
            ));
        }

        // Try to parse as JSON first (like Symphony does)
        match serde_json::from_str::<serde_json::Value>(raw_plan) {
            Ok(parsed_json) => {
                let steps_array = self.extract_steps_from_json(&parsed_json)?;
                self.convert_json_steps_to_planned_steps(steps_array, agent_config)
            }
            Err(_) => {
                // Fallback to line-by-line parsing if JSON fails
                self.parse_plan_lines(raw_plan)
            }
        }
    }

    /// Extract steps array from parsed JSON (matching Symphony's robust extraction)
    fn extract_steps_from_json<'a>(
        &self,
        parsed_json: &'a serde_json::Value,
    ) -> AriaResult<&'a serde_json::Value> {
        if let Some(_steps) = parsed_json.get("steps").and_then(|s| s.as_array()) {
            return Ok(parsed_json.get("steps").unwrap());
        }
        if let Some(plan_steps) = parsed_json.get("plan").and_then(|p| p.get("steps")) {
            return Ok(plan_steps);
        }
        if parsed_json.is_array() {
            return Ok(parsed_json);
        }

        if let Some(obj) = parsed_json.as_object() {
            for (_key, value) in obj {
                if value.is_array() {
                    return Ok(value);
                }
            }
        }

        Err(AriaError::new(
            ErrorCode::PlanningFailure,
            ErrorCategory::Planning,
            ErrorSeverity::Medium,
            "Could not find valid steps array in parsed JSON",
        ))
    }

    /// Convert JSON steps to PlannedStep structs (matching Symphony's conversion)
    fn convert_json_steps_to_planned_steps(
        &self,
        steps_array: &serde_json::Value,
        agent_config: &AgentConfig,
    ) -> AriaResult<Vec<PlannedStep>> {
        let steps = steps_array.as_array().ok_or_else(|| {
            AriaError::new(
                ErrorCode::PlanningFailure,
                ErrorCategory::Planning,
                ErrorSeverity::Medium,
                "Steps is not an array",
            )
        })?;

        let mut planned_steps = Vec::new();

        for (_index, step) in steps.iter().enumerate() {
            if !step.is_object() {
                continue; // Skip invalid steps
            }

            let description = step
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or(&format!(
                    "Execute step for {}",
                    step.get("tool").and_then(|t| t.as_str()).unwrap_or("TBD")
                ))
                .to_string();

            let tool_name = if step.get("useTool").and_then(|u| u.as_bool()).unwrap_or(true) {
                step.get("tool")
                    .and_then(|t| t.as_str())
                    .filter(|&t| t != "none")
                    .map(|t| t.to_string())
            } else {
                None
            };

            if let Some(ref tool) = tool_name {
                if !agent_config.tools.contains(tool)
                    && tool != "ponderTool"
                    && tool != "createPlanTool"
                {
                    continue;
                }
            }

            let mut parameters = HashMap::new();
            if let Some(params) = step.get("parameters").and_then(|p| p.as_object()) {
                for (key, value) in params {
                    parameters.insert(key.clone(), value.clone().into());
                }
            }

            planned_steps.push(PlannedStep {
                id: DeepUuid(Uuid::new_v4()),
                description,
                step_type: if tool_name.is_some() {
                    StepType::ToolCall
                } else {
                    StepType::ReasoningStep
                },
                tool_name,
                agent_name: None,
                container_spec: None,
                parameters,
                success_criteria: "Step completes without error".to_string(),
                timeout_ms: Some(30000), // 30 seconds
                retry_count: Some(3),
            });
        }

        if planned_steps.is_empty() {
            return Err(AriaError::new(
                ErrorCode::PlanningFailure,
                ErrorCategory::Planning,
                ErrorSeverity::Medium,
                "No valid steps found in plan",
            ));
        }

        Ok(planned_steps)
    }

    /// Fallback line-by-line parsing (matching Symphony's fallback)
    fn parse_plan_lines(&self, raw_plan: &str) -> AriaResult<Vec<PlannedStep>> {
        let lines: Vec<PlannedStep> = raw_plan
            .lines()
            .enumerate()
            .filter_map(|(_index, line)| {
                let line = line.trim();
                if line.is_empty() || !line.chars().next().unwrap_or(' ').is_ascii_digit() {
                    return None;
                }

                let description = line
                    .split_once('.')
                    .map(|(_, desc)| desc.trim())
                    .unwrap_or(line)
                    .to_string();

                Some(PlannedStep {
                    id: DeepUuid(Uuid::new_v4()),
                    description,
                    step_type: StepType::ReasoningStep,
                    tool_name: None,
                    agent_name: None,
                    container_spec: None,
                    parameters: HashMap::new(),
                    success_criteria: "Step completes without error".to_string(),
                    timeout_ms: Some(30000),
                    retry_count: Some(3),
                })
            })
            .collect();

        if lines.is_empty() {
            return Err(AriaError::new(
                ErrorCode::PlanningFailure,
                ErrorCategory::Planning,
                ErrorSeverity::Medium,
                "No valid steps found in line-based parsing",
            ));
        }

        Ok(lines)
    }
}

impl Engine for PlanningEngine {
    fn initialize(&self) -> bool {
        // TODO: Check if createPlanTool is available (async version would be better)
        true
    }

    fn get_dependencies(&self) -> Vec<String> {
        vec!["tool_registry".to_string()]
    }

    fn get_state(&self) -> String {
        "ready".to_string()
    }

    fn health_check(&self) -> bool {
        // TODO: Check if createPlanTool is available (async version would be better)
        true
    }

    fn shutdown(&self) -> bool {
        true
    }
}

// PlanningEngineInterface is defined in engines/mod.rs

#[async_trait]
impl PlanningEngineInterface for PlanningEngine {
    async fn analyze_task(
        &self,
        task: &str,
        _context: &RuntimeContext,
    ) -> AriaResult<TaskAnalysis> {
        let keywords = [
            "then",
            "and then",
            "after that",
            "first",
            "second",
            "finally",
            "create a plan",
        ];
        let task_lower = task.to_lowercase();

        let requires_planning =
            keywords.iter().any(|&kw| task_lower.contains(kw)) || task.len() > 200;
        let complexity = if requires_planning {
            TaskComplexity::MultiStep
        } else {
            TaskComplexity::Simple
        };

        let reasoning = if requires_planning {
            "Task contains keywords or is long, suggesting multiple steps are needed".to_string()
        } else {
            "Task appears simple and suitable for single-shot execution".to_string()
        };

        Ok(TaskAnalysis {
            complexity,
            requires_planning,
            requires_containers: task_lower.contains("container")
                || task_lower.contains("docker")
                || task_lower.contains("image"),
            estimated_steps: if requires_planning { 3 } else { 1 },
            reasoning,
        })
    }

    async fn create_execution_plan(
        &self,
        task: &str,
        agent_config: &AgentConfig,
        context: &RuntimeContext,
    ) -> AriaResult<ExecutionPlan> {
        let analysis = self.analyze_task(task, context).await?;

        let plan_tool_params = serde_json::json!({
            "objective": task,
            "context": {
                "agentName": agent_config.name,
                "availableTools": agent_config.tools,
                "sessionId": context.session_id.to_string(),
                "currentStep": context.current_step,
                "totalSteps": context.total_steps
            }
        });

        let plan_tool_result = self
            .tool_registry
            .execute_tool("createPlanTool", plan_tool_params.into())
            .await?;

        if !plan_tool_result.success {
            return Err(AriaError::new(
                ErrorCode::PlanningFailure,
                ErrorCategory::Planning,
                ErrorSeverity::High,
                &format!(
                    "createPlanTool failed: {}",
                    plan_tool_result.error.as_ref().unwrap_or(&"Unknown error".to_string())
                ),
            ));
        }

        let plan_result = plan_tool_result.result.ok_or_else(|| {
            AriaError::new(
                ErrorCode::PlanningFailure,
                ErrorCategory::Planning,
                ErrorSeverity::High,
                "createPlanTool returned no result",
            )
        })?;

        let generated_plan_str = plan_result
            .get("plan")
            .and_then(|p| p.get("generatedPlan"))
            .and_then(|gp| gp.as_str())
            .ok_or_else(|| {
                AriaError::new(
                    ErrorCode::PlanningFailure,
                    ErrorCategory::Planning,
                    ErrorSeverity::High,
                    "Generated plan not found in tool result",
                )
            })?;

        let steps = self.parse_raw_plan_to_steps(generated_plan_str, agent_config)?;

        let confidence = if analysis.requires_planning && !steps.is_empty() {
            0.85
        } else if steps.is_empty() {
            0.3
        } else {
            0.7
        };

        let estimated_duration = steps.len() as u64 * 30 * 1000;
        let steps_len = steps.len() as u32;

        Ok(ExecutionPlan {
            id: DeepUuid(Uuid::new_v4()),
            task_description: task.to_string(),
            steps,
            confidence,
            estimated_duration: Some(estimated_duration),
            resource_requirements: ResourceRequirements {
                cpu_millis: 500 * steps_len,
                memory_mb: 128,
                disk_mb: 50,
                network_bandwidth_kbps: Some(1000),
                container_count: 0,
                cpu_cores: Some(1),
                timeout_seconds: Some((estimated_duration / 1000) as u64),
                max_concurrent: Some(1),
            },
        })
    }
} 