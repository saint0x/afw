use async_trait::async_trait;
use crate::engines::{ExecutionEngineInterface, ContainerManagerInterface, Engine};
use crate::engines::llm::types::{LLMConfig, LLMMessage, LLMRequest};
use crate::engines::llm::LLMHandler;
use crate::engines::tool_registry::ToolRegistryInterface;
use crate::engines::system_prompt::SystemPromptService;
use crate::engines::container::quilt::QuiltService;
use crate::types::*;
use crate::errors::{AriaResult, AriaError, ErrorCode, ErrorCategory, ErrorSeverity};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use serde_json::Value;
use regex::Regex;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::deep_size::{DeepUuid, DeepValue};
use futures::future::BoxFuture;

/// The ExecutionEngine is responsible for the core "magic" of tool execution.
/// It preserves Symphony's brilliant unconscious tool execution logic while
/// adding container orchestration capabilities.
pub struct ExecutionEngine {
    tool_registry: Arc<dyn ToolRegistryInterface>,
    llm_handler: Arc<LLMHandler>,
    quilt_service: Arc<Mutex<QuiltService>>,
    system_prompt_service: SystemPromptService,
    max_orchestration_steps: usize,
}

impl ExecutionEngine {
    pub fn new(
        tool_registry: Arc<dyn ToolRegistryInterface>,
        llm_handler: Arc<LLMHandler>,
        quilt_service: Arc<Mutex<QuiltService>>,
    ) -> Self {
        Self {
            tool_registry,
            llm_handler,
            quilt_service,
            system_prompt_service: SystemPromptService::new(),
            max_orchestration_steps: 5,
        }
    }

    /// Detects if a task requires multi-tool orchestration
    pub fn detect_multi_tool_requirement(&self, task: &str) -> bool {
        let multi_tool_indicators = [
            r"first.*then", r"step 1.*step 2", r"sequence", r"chain", r"orchestration",
            r"ponder.*write", r"analyze.*generate", r"think.*code", r"both",
            r"first.*second", r"after.*then", r"followed by", r"next.*tool",
            r"FIRST:", r"SECOND:", r"1\.", r"2\.", r"execute TWO tools"
        ];
        
        let task_lower = task.to_lowercase();
        multi_tool_indicators.iter().any(|&indicator| {
            if let Ok(regex) = Regex::new(&indicator.replace(".*", ".*?")) {
                regex.is_match(&task_lower)
            } else {
                false
            }
        })
    }

    /// Resolves parameter placeholders from execution history
    pub fn resolve_placeholders(
        &self,
        parameters: &HashMap<String, DeepValue>,
        history: &[ExecutionStep],
    ) -> AriaResult<HashMap<String, DeepValue>> {
        let mut resolved = parameters.clone();
        
        // Regex patterns for placeholder resolution
        let full_placeholder_regex = Regex::new(r"^\{\{step_(\d+)_output(?:\.(.+))?\}\}$")
            .map_err(|e| AriaError::new(
                ErrorCode::ParameterResolutionError,
                ErrorCategory::Execution,
                ErrorSeverity::Medium,
                &format!("Failed to compile placeholder regex: {}", e)
            ))?;
            
        let partial_placeholder_regex = Regex::new(r"\{\{step_(\d+)_output(?:\.(.+))?\}\}")
            .map_err(|e| AriaError::new(
                ErrorCode::ParameterResolutionError,
                ErrorCategory::Execution,
                ErrorSeverity::Medium,
                &format!("Failed to compile partial placeholder regex: {}", e)
            ))?;

        self.traverse_and_resolve(&mut resolved, history, &full_placeholder_regex, &partial_placeholder_regex)?;
        
        Ok(resolved)
    }

    /// Execute a container workload if container manager is available
    pub async fn execute_container_workload_impl(
        &self,
        spec: &ContainerSpec,
        exec_command: &Vec<String>,
        context_opt: Option<&RuntimeContext>,
        session_id: DeepUuid,
    ) -> AriaResult<ToolResult> {
        let mut quilt = self.quilt_service.lock().await;

        let default_context;
        let context = match context_opt {
            Some(ctx) => ctx,
            None => {
                default_context = RuntimeContext::default_for_session(session_id);
                &default_context
            }
        };

        let context_env = self.create_context_environment(context, session_id);
        
        // Clean agent-controlled lifecycle: create → start
        let container_id = quilt.create_container(
            spec.image.clone(),
            spec.command.clone(),
            context_env,
        ).await?;
        
        // Agent explicitly starts the container
        quilt.start_container(container_id.clone()).await?;
        
        // Poll for the container to be in a 'Running' state before proceeding.
        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(20); // 20-second timeout for startup

        loop {
            if start_time.elapsed() > timeout {
                // Ensure cleanup happens on timeout
                let _ = quilt.remove_container(container_id.clone()).await;
                return Err(AriaError::new(
                    ErrorCode::ContainerOperationFailed,
                    ErrorCategory::Container,
                    ErrorSeverity::High,
                    &format!("Container {} failed to start within the timeout period", container_id)
                ));
            }

            let status = quilt.get_container_status(container_id.clone()).await?;
            match status.state {
                ContainerState::Running => {
                    break; // Container is ready, proceed to execution.
                },
                ContainerState::Failed | ContainerState::Exited => {
                    // Container failed to start, so we clean up and return an error.
                    let _ = quilt.remove_container(container_id.clone()).await;
                    return Err(AriaError::new(
                        ErrorCode::ContainerOperationFailed,
                        ErrorCategory::Container,
                        ErrorSeverity::High,
                        &format!("Container {} entered state {:?} during startup", container_id, status.state)
                    ));
                },
                _ => {
                    // State is Created or Starting, so we wait and poll again.
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }
        
        // Container is running, now execute the specific command.
        let result = quilt.exec_in_container(container_id.clone(), exec_command.clone()).await;

        // Always attempt to clean up the container after execution.
        let _ = quilt.remove_container(container_id.clone()).await;
        
        match result {
            Ok(exec_result) => Ok(ToolResult {
                success: exec_result.exit_code == 0,
                result: Some(DeepValue(serde_json::json!({
                    "exit_code": exec_result.exit_code,
                    "stdout": exec_result.stdout,
                    "stderr": exec_result.stderr,
                    "execution_time_ms": exec_result.execution_time_ms
                }))),
                error: if exec_result.exit_code != 0 {
                    Some(exec_result.stderr)
                } else {
                    None
                },
                metadata: HashMap::new(),
                execution_time_ms: exec_result.execution_time_ms,
                resource_usage: exec_result.resource_usage,
            }),
            Err(e) => Err(e),
        }
    }

    /// Create context environment variables for container execution
    fn create_context_environment(
        &self,
        context: &RuntimeContext,
        session_id: DeepUuid,
    ) -> HashMap<String, String> {
        let mut env = HashMap::new();
        
        env.insert("ARIA_SESSION_ID".to_string(), session_id.to_string());
        env.insert("ARIA_AGENT_NAME".to_string(), context.agent_config.name.clone());
        env.insert("ARIA_STEP_COUNT".to_string(), context.current_step.to_string());
        env.insert("ARIA_TOTAL_STEPS".to_string(), context.total_steps.to_string());
        
        // Add memory context
        env.insert("ARIA_MEMORY_SIZE".to_string(), context.memory_size.to_string());
        env.insert("ARIA_MAX_MEMORY".to_string(), context.max_memory_size.to_string());
        
        // Add execution history summary
        let success_count = context.execution_history.iter().filter(|s| s.success).count();
        env.insert("ARIA_SUCCESS_COUNT".to_string(), success_count.to_string());
        env.insert("ARIA_FAILURE_COUNT".to_string(), 
                   (context.execution_history.len() - success_count).to_string());
        
        env
    }

    /// Execute with orchestration for multi-tool workflows
    /// This preserves Symphony's brilliant multi-tool orchestration logic
    async fn execute_with_orchestration(
        &self,
        task: &str,
        agent_config: &AgentConfig,
        _context: &RuntimeContext,
    ) -> AriaResult<ToolResult> {
        let mut conversation_history = Vec::new();
        let mut all_tool_results = Vec::new();
        let mut overall_success = true;
        let mut primary_error: Option<String> = None;
        let mut final_response = String::new();
        let mut orchestration_step = 0;

        // Generate sophisticated orchestration prompt using SystemPromptService
        let orchestration_prompt = self.system_prompt_service.generate_orchestration_prompt(task, agent_config);
        
        conversation_history.push(LLMMessage {
            role: "system".to_string(),
            content: orchestration_prompt,
            tool_calls: None,
            tool_call_id: None,
        });
        
        conversation_history.push(LLMMessage {
            role: "user".to_string(),
            content: format!("Execute the first step of this task: {}", task),
            tool_calls: None,
            tool_call_id: None,
        });

        while orchestration_step < self.max_orchestration_steps {
            orchestration_step += 1;
            
            // Execute current orchestration step
            let step_result = self.execute_single_orchestration_step(
                &conversation_history,
                agent_config,
            ).await?;
            
            if !step_result.success {
                overall_success = false;
                primary_error = step_result.error.clone();
                break;
            }

            // Check if this step executed a tool
            if let Some(tool_executed) = step_result.tool_executed {
                all_tool_results.push(tool_executed.clone());
                
                // Add tool result to conversation history for context
                conversation_history.push(LLMMessage {
                    role: "assistant".to_string(),
                    content: serde_json::json!({
                        "tool_name": tool_executed.get("name"),
                        "parameters": tool_executed.get("parameters").unwrap_or(&Value::Null)
                    }).to_string(),
                    tool_calls: None,
                    tool_call_id: None,
                });
                
                conversation_history.push(LLMMessage {
                    role: "user".to_string(),
                    content: format!(
                        "Tool \"{}\" completed. Result: {}. {}",
                        tool_executed.get("name").unwrap_or(&Value::Null),
                        serde_json::to_string(&tool_executed.get("result").unwrap_or(&Value::Null)).unwrap_or_default(),
                        if all_tool_results.len() < agent_config.tools.len() {
                            "Continue with the next required tool."
                        } else {
                            "If all required tools have been used, provide your final synthesis."
                        }
                    ),
                    tool_calls: None,
                    tool_call_id: None,
                });
            } else if let Some(response) = step_result.response {
                // No tool executed, this should be the final response
                final_response = response;
                break;
            }
        }

        if orchestration_step >= self.max_orchestration_steps {
            overall_success = false;
            primary_error = Some("Orchestration exceeded maximum step limit".to_string());
        }

        // Generate final response if we don't have one
        if final_response.is_empty() && !all_tool_results.is_empty() {
            if let Some(last_result) = all_tool_results.last() {
                final_response = last_result.get("result")
                    .and_then(|r| r.get("response"))
                    .and_then(|r| r.as_str())
                    .unwrap_or("Multi-tool orchestration completed")
                    .to_string();
            }
        }

        Ok(ToolResult {
            success: overall_success,
            result: Some(serde_json::json!({
                "response": final_response,
                "reasoning": format!(
                    "Orchestrated {} tools in sequence: {}",
                    all_tool_results.len(),
                    all_tool_results.iter()
                        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
                        .collect::<Vec<_>>()
                        .join(" → ")
                ),
                "agent": agent_config.name,
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                "tools_executed": all_tool_results,
                "orchestration_steps": orchestration_step
            }).into()),
            error: primary_error,
            metadata: HashMap::new(),
            execution_time_ms: 0, // TODO: Track execution time
            resource_usage: None,
        })
    }

    /// Execute a single step in the orchestration flow
    async fn execute_single_orchestration_step(
        &self,
        conversation_history: &[LLMMessage],
        agent_config: &AgentConfig,
    ) -> AriaResult<OrchestrationStepResult> {
        let llm_request = LLMRequest {
            messages: conversation_history.to_vec(),
            config: crate::engines::llm::types::LLMConfig {
                model: Some(agent_config.llm.model.clone()),
                temperature: agent_config.llm.temperature.unwrap_or(0.7),
                max_tokens: agent_config.llm.max_tokens.unwrap_or(2000),
                top_p: None,
                frequency_penalty: None,
                presence_penalty: None,
            },
            provider: None,
            tools: None,
            tool_choice: None,
            stream: Some(false),
        };

        let llm_response = self.llm_handler.complete(llm_request).await?;

        // Parse JSON response
        let parsed_json: Value = serde_json::from_str(&llm_response.content)
            .map_err(|e| AriaError::new(
                ErrorCode::LLMInvalidResponse,
                ErrorCategory::LLM,
                ErrorSeverity::Medium,
                &format!("Invalid JSON response from LLM: {}", e)
            ))?;

        let tool_name = parsed_json.get("tool_name")
            .or_else(|| parsed_json.get("toolName"))
            .and_then(|v| v.as_str());
            
        let parameters = parsed_json.get("parameters");

        if let Some(tool_name) = tool_name {
            if tool_name != "none" {
                if let Some(parameters) = parameters {
                    // Execute the tool
                    let tool_result = self.tool_registry.execute_tool(
                        tool_name,
                        parameters.clone().into(),
                    ).await?;
                    
                    return Ok(OrchestrationStepResult {
                        success: true,
                        tool_executed: Some(serde_json::json!({
                            "name": tool_name,
                            "parameters": parameters,
                            "success": tool_result.success,
                            "result": tool_result.result,
                            "error": tool_result.error,
                        })),
                        response: None,
                        error: None,
                    });
                } else {
                    return Ok(OrchestrationStepResult {
                        success: false,
                        tool_executed: None,
                        response: None,
                        error: Some(format!("Tool '{}' specified but no parameters provided", tool_name)),
                    });
                }
            } else {
                // Final response
                let response = parsed_json.get("response")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Task completed")
                    .to_string();
                    
                return Ok(OrchestrationStepResult {
                    success: true,
                    tool_executed: None,
                    response: Some(response),
                    error: None,
                });
            }
        }

        Ok(OrchestrationStepResult {
            success: false,
            tool_executed: None,
            response: None,
            error: Some("Invalid tool specification in LLM response".to_string()),
        })
    }

    /// Execute a single-step task (original behavior)
    /// This preserves Symphony's single-shot execution intelligence
    async fn execute_single_step(
        &self,
        task: &str,
        agent_config: &AgentConfig,
        _context: &RuntimeContext,
    ) -> AriaResult<ToolResult> {
        println!("🔍 DEBUG: Starting execute_single_step");
        println!("🔍 DEBUG: Task: {}", task);
        println!("🔍 DEBUG: Agent: {}", agent_config.name);
        println!("🔍 DEBUG: Agent tools: {:?}", agent_config.tools);
        
        let system_prompt = self.system_prompt_service.generate_system_prompt(agent_config, !agent_config.tools.is_empty());
        let has_tools = !agent_config.tools.is_empty();
        
        println!("🔍 DEBUG: Has tools: {}", has_tools);
        println!("🔍 DEBUG: System prompt length: {}", system_prompt.len());

        let messages = vec![
            LLMMessage {
                role: "system".to_string(),
                content: system_prompt,
                tool_calls: None,
                tool_call_id: None,
            },
            LLMMessage {
                role: "user".to_string(),
                content: task.to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        println!("🔍 DEBUG: Creating LLM request with {} messages", messages.len());

        let llm_request = LLMRequest {
            messages,
            config: crate::engines::llm::types::LLMConfig {
                model: Some(agent_config.llm.model.clone()),
                temperature: agent_config.llm.temperature.unwrap_or(0.7),
                max_tokens: agent_config.llm.max_tokens.unwrap_or(2000),
                top_p: None,
                frequency_penalty: None,
                presence_penalty: None,
            },
            provider: None,
            tools: None,
            tool_choice: None,
            stream: Some(false),
        };

        println!("🔍 DEBUG: Calling LLM handler...");
        let llm_response = self.llm_handler.complete(llm_request).await?;
        
        println!("🔍 DEBUG: LLM Response received!");
        println!("🔍 DEBUG: Response content length: {}", llm_response.content.len());
        println!("🔍 DEBUG: Response content (first 200 chars): {}", 
            &llm_response.content.chars().take(200).collect::<String>());
        println!("🔍 DEBUG: Response model: {}", llm_response.model);

        if has_tools {
            println!("🔍 DEBUG: Agent has tools, attempting to parse response for tool execution");
            // Parse JSON response and potentially execute tools
            self.parse_and_execute_tools(&llm_response.content, agent_config).await
        } else {
            println!("🔍 DEBUG: Agent has no tools, returning direct LLM response");
            // Direct LLM response
            let result = ToolResult {
                success: true,
                result: Some(serde_json::json!({
                    "response": llm_response.content,
                    "reasoning": "Direct LLM response as agent has no tools",
                    "agent": agent_config.name,
                    "model": llm_response.model
                }).into()),
                error: None,
                metadata: HashMap::new(),
                execution_time_ms: 0,
                resource_usage: None,
            };
            
            println!("🔍 DEBUG: Created ToolResult with success: {}", result.success);
            println!("🔍 DEBUG: ToolResult response: {:?}", result.result);
            
            Ok(result)
        }
    }

    /// Parse LLM response and execute tools if specified
    async fn parse_and_execute_tools(
        &self,
        content: &str,
        agent_config: &AgentConfig,
    ) -> AriaResult<ToolResult> {
        let parsed_json: Value = serde_json::from_str(content)
            .map_err(|e| AriaError::new(
                ErrorCode::LLMInvalidResponse,
                ErrorCategory::LLM,
                ErrorSeverity::Medium,
                &format!("Failed to parse LLM response as JSON: {}", e)
            ))?;

        let tool_name = parsed_json.get("tool_name")
            .or_else(|| parsed_json.get("toolName"))
            .and_then(|v| v.as_str());

        if let Some(tool_name) = tool_name {
            if tool_name == "none" {
                // No tool execution needed
                let response = parsed_json.get("response")
                    .and_then(|v| v.as_str())
                    .unwrap_or("No further action taken")
                    .to_string();
                    
                return Ok(ToolResult {
                    success: true,
                    result: Some(serde_json::json!({
                        "response": response,
                        "reasoning": "No tool actions taken as per LLM decision",
                        "agent": agent_config.name
                    }).into()),
                    error: None,
                    metadata: HashMap::new(),
                    execution_time_ms: 0,
                    resource_usage: None,
                });
            } else if let Some(parameters) = parsed_json.get("parameters") {
                // Validate agent has access to this tool
                if !agent_config.tools.contains(&tool_name.to_string()) {
                    return Err(AriaError::new(
                        ErrorCode::ToolNotFound,
                        ErrorCategory::Tool,
                        ErrorSeverity::High,
                        &format!("Agent '{}' is not authorized to use tool '{}'", agent_config.name, tool_name)
                    ));
                }
                
                // Execute the specified tool
                let tool_result = self.tool_registry.execute_tool(tool_name, parameters.clone().into()).await?;
                
                let response = if tool_result.success {
                    format!("Tool {} executed successfully. Result: {}", 
                            tool_name, 
                            serde_json::to_string(&tool_result.result).unwrap_or_default())
                } else {
                    format!("Tool {} failed: {}", tool_name, tool_result.error.as_ref().unwrap_or(&"Unknown error".to_string()))
                };

                return Ok(ToolResult {
                    success: tool_result.success,
                    result: Some(serde_json::json!({
                        "response": response,
                        "reasoning": format!("Executed tool: {}", tool_name),
                        "agent": agent_config.name,
                        "tools_executed": [{
                            "name": tool_name,
                            "success": tool_result.success,
                            "result": tool_result.result,
                            "error": tool_result.error
                        }]
                    }).into()),
                    error: if tool_result.success { None } else { tool_result.error },
                    metadata: HashMap::new(),
                    execution_time_ms: tool_result.execution_time_ms,
                    resource_usage: tool_result.resource_usage,
                });
            }
        }

        Err(AriaError::new(
            ErrorCode::LLMInvalidResponse,
            ErrorCategory::LLM,
            ErrorSeverity::Medium,
            "LLM response did not specify a valid tool or response"
        ))
    }

    /// Recursive function to traverse and resolve placeholders
    fn traverse_and_resolve(
        &self,
        obj: &mut HashMap<String, DeepValue>,
        history: &[ExecutionStep],
        full_regex: &Regex,
        partial_regex: &Regex,
    ) -> AriaResult<()> {
        for (_key, value) in obj.iter_mut() {
            match &mut value.0 {
                Value::String(s) => {
                    // Check for full placeholder replacement
                    if let Some(caps) = full_regex.captures(s) {
                        let step_index: usize = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
                        let property_path = caps.get(2).map(|m| m.as_str());
                        
                        if let Some(resolved_value) = self.resolve_step_output(step_index, property_path, history) {
                            *value = resolved_value;
                        }
                    } else {
                        // Check for partial placeholder replacement
                        let resolved_string = partial_regex.replace_all(s, |caps: &regex::Captures| {
                            let step_index: usize = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
                            let property_path = caps.get(2).map(|m| m.as_str());
                            
                            if let Some(resolved_value) = self.resolve_step_output(step_index, property_path, history) {
                                match &resolved_value.0 {
                                    Value::String(s) => s.clone(),
                                    other => serde_json::to_string(&other).unwrap_or_default(),
                                }
                            } else {
                                caps.get(0).unwrap().as_str().to_string()
                            }
                        });
                        *value = DeepValue(Value::String(resolved_string.to_string()));
                    }
                },
                Value::Object(_nested_obj) => {
                    if let Ok(mut nested_map) = serde_json::from_value::<HashMap<String, DeepValue>>(value.0.clone()) {
                        self.traverse_and_resolve(&mut nested_map, history, full_regex, partial_regex)?;
                        *value = serde_json::to_value(nested_map).unwrap_or(Value::Null).into();
                    }
                },
                Value::Array(arr) => {
                    for item in arr.iter_mut() {
                        if let Value::Object(_) = item {
                            if let Ok(mut item_map) = serde_json::from_value::<HashMap<String, DeepValue>>(item.clone()) {
                                self.traverse_and_resolve(&mut item_map, history, full_regex, partial_regex)?;
                                *item = serde_json::to_value(item_map).unwrap_or(Value::Null);
                            }
                        }
                    }
                },
                _ => {} // Other types don't need resolution
            }
        }
        Ok(())
    }

    /// Resolve a step output reference
    fn resolve_step_output(
        &self,
        step_index: usize,
        property_path: Option<&str>,
        history: &[ExecutionStep],
    ) -> Option<DeepValue> {
        // Convert 1-based index to 0-based
        let actual_index = step_index.saturating_sub(1);
        
        if let Some(step) = history.get(actual_index) {
            if step.success {
                if let Some(result) = &step.result {
                    let mut current_value = result.clone();
                    
                    if let Some(path) = property_path {
                        for prop in path.split('.') {
                            if let Some(obj) = current_value.as_object() {
                                if let Some(next_value) = obj.get(prop) {
                                    current_value = next_value.clone().into();
                                } else {
                                    return None; // Property not found
                                }
                            } else {
                                return None; // Not an object
                            }
                        }
                    }
                    
                    return Some(current_value);
                }
            }
        }
        
        None
    }
}

impl Engine for ExecutionEngine {
    fn initialize(&self) -> bool {
        // TODO: Initialize tool registry and LLM handler
        true
    }
    
    fn get_dependencies(&self) -> Vec<String> {
        vec![
            "tool_registry".to_string(),
            "llm_handler".to_string(),
            "container_manager".to_string(),
        ]
    }
    
    fn get_state(&self) -> String {
        "ready".to_string()
    }
    
    fn health_check(&self) -> bool {
        // TODO: Check if all dependencies are healthy
        true
    }
    
    fn shutdown(&self) -> bool {
        // TODO: Cleanup resources
        true
    }
}

#[async_trait]
impl ExecutionEngineInterface for ExecutionEngine {
    async fn execute(
        &self,
        task: &str,
        agent_config: &AgentConfig,
        context: &RuntimeContext,
    ) -> AriaResult<ToolResult> {
        // Check if task requires multi-tool orchestration
        let requires_orchestration = self.detect_multi_tool_requirement(task);
        
        if requires_orchestration && !agent_config.tools.is_empty() {
            self.execute_with_orchestration(task, agent_config, context).await
        } else {
            self.execute_single_step(task, agent_config, context).await
        }
    }

    async fn execute_step(
        &self,
        step: &PlannedStep,
        context: &RuntimeContext,
    ) -> AriaResult<ExecutionStep> {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Resolve parameters using execution history
        let resolved_parameters = self.resolve_placeholders(&step.parameters, &context.execution_history)?;

        let result = if step.tool_name.as_deref() == Some("none") {
            // Non-tool step, just return a success result
            ToolResult {
                success: true,
                result: Some(serde_json::json!({
                    "response": format!("Executed reasoning step: {}", step.description)
                }).into()),
                error: None,
                metadata: HashMap::new(),
                execution_time_ms: 0,
                resource_usage: None,
            }
        } else if let Some(tool_name) = &step.tool_name {
            // Execute the tool
            let params_value = serde_json::to_value(&resolved_parameters).unwrap_or(Value::Null);
            self.tool_registry.execute_tool(tool_name, params_value.into()).await?
        } else {
            return Err(AriaError::new(
                ErrorCode::StepExecutionError,
                ErrorCategory::Execution,
                ErrorSeverity::High,
                "Step has no tool name specified"
            ));
        };

        let end_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(ExecutionStep {
            step_id: DeepUuid(Uuid::new_v4()),
            description: step.description.clone(),
            start_time,
            end_time,
            duration: end_time - start_time,
            success: result.success,
            step_type: StepType::ToolCall,
            tool_used: step.tool_name.clone(),
            agent_used: None,
            container_used: None,
            parameters: resolved_parameters,
            result: result.result,
            error: result.error,
            reflection: None,
            summary: format!("Step '{}' {}", 
                           step.description, 
                           if result.success { "succeeded" } else { "failed" }),
            resource_usage: result.resource_usage,
        })
    }

    async fn execute_container_workload(
        &self,
        spec: &ContainerSpec,
        exec_command: &Vec<String>,
        context: Option<&RuntimeContext>,
        session_id: DeepUuid,
    ) -> AriaResult<ToolResult> {
        self.execute_container_workload_impl(spec, exec_command, context, session_id).await
    }

    fn detect_multi_tool_requirement(&self, task: &str) -> bool {
        self.detect_multi_tool_requirement(task)
    }
    
    fn resolve_placeholders(
        &self,
        parameters: &HashMap<String, DeepValue>,
        history: &[ExecutionStep],
    ) -> AriaResult<HashMap<String, DeepValue>> {
        self.resolve_placeholders(parameters, history)
    }
}

/// Result of a single orchestration step
pub struct OrchestrationStepResult {
    pub success: bool,
    pub tool_executed: Option<Value>,
    pub response: Option<String>,
    pub error: Option<String>,
}

// ExecutionEngineInterface is defined in engines/mod.rs

 