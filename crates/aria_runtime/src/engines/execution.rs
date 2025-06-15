use async_trait::async_trait;
use crate::engines::{ExecutionEngineInterface, RuntimeEngine, ContainerManagerInterface};
use crate::engines::llm::types::*;
use crate::engines::llm::LLMHandlerInterface;
use crate::types::*;
use crate::errors::{AriaResult, AriaError, ErrorCode, ErrorCategory, ErrorSeverity};
use std::collections::HashMap;
use uuid::Uuid;
use serde_json::Value;
use regex::Regex;
use std::time::{SystemTime, UNIX_EPOCH};

/// The ExecutionEngine is responsible for the core "magic" of tool execution.
/// It preserves Symphony's brilliant unconscious tool execution logic while
/// adding container orchestration capabilities.
pub struct ExecutionEngine {
    tool_registry: Box<dyn ToolRegistryInterface>,
    llm_handler: Box<dyn LLMHandlerInterface>,
    container_manager: Option<Box<dyn ContainerManagerInterface>>,
    max_orchestration_steps: usize,
}

impl ExecutionEngine {
    pub fn new(
        tool_registry: Box<dyn ToolRegistryInterface>,
        llm_handler: Box<dyn LLMHandlerInterface>,
        container_manager: Option<Box<dyn ContainerManagerInterface>>,
    ) -> Self {
        Self {
            tool_registry,
            llm_handler,
            container_manager,
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
        parameters: &HashMap<String, Value>,
        history: &[ExecutionStep],
    ) -> AriaResult<HashMap<String, Value>> {
        let mut resolved = parameters.clone();
        
        // Regex patterns for placeholder resolution
        let full_placeholder_regex = Regex::new(r"^\{\{step_(\d+)_output(?:\.(.+))?\}\}$")
            .map_err(|e| AriaError::new(
                ErrorCode::ParameterResolutionFailed,
                ErrorCategory::Execution,
                ErrorSeverity::Medium,
                format!("Failed to compile placeholder regex: {}", e)
            ))?;
            
        let partial_placeholder_regex = Regex::new(r"\{\{step_(\d+)_output(?:\.(.+))?\}\}")
            .map_err(|e| AriaError::new(
                ErrorCode::ParameterResolutionFailed,
                ErrorCategory::Execution,
                ErrorSeverity::Medium,
                format!("Failed to compile partial placeholder regex: {}", e)
            ))?;

        self.traverse_and_resolve(&mut resolved, history, &full_placeholder_regex, &partial_placeholder_regex)?;
        
        Ok(resolved)
    }

    /// Execute a container workload if container manager is available
    pub async fn execute_container_workload(
        &self,
        spec: &ContainerSpec,
        context: &RuntimeContext,
        session_id: Uuid,
    ) -> AriaResult<ToolResult> {
        if let Some(container_manager) = &self.container_manager {
            // Create context environment variables
            let context_env = self.create_context_environment(context, session_id);
            
            // Create and start container
            let container_id = container_manager.create_container(spec, session_id, context_env).await?;
            
            // Execute the workload
            let result = container_manager.execute_in_container(
                &container_id,
                &spec.command,
                spec.resource_limits.timeout_seconds.unwrap_or(300),
            ).await;
            
            // Cleanup container
            let _ = container_manager.cleanup_container(&container_id).await;
            
            match result {
                Ok(exec_result) => Ok(ToolResult {
                    success: exec_result.exit_code == 0,
                    result: Some(serde_json::json!({
                        "exit_code": exec_result.exit_code,
                        "stdout": exec_result.stdout,
                        "stderr": exec_result.stderr,
                        "execution_time_ms": exec_result.execution_time_ms
                    })),
                    error: if exec_result.exit_code != 0 { 
                        Some(exec_result.stderr) 
                    } else { 
                        None 
                    },
                    metadata: HashMap::new(),
                    execution_time_ms: exec_result.execution_time_ms,
                    resource_usage: exec_result.resource_usage,
                }),
                Err(e) => Ok(ToolResult {
                    success: false,
                    result: None,
                    error: Some(format!("Container execution failed: {}", e)),
                    metadata: HashMap::new(),
                    execution_time_ms: 0,
                    resource_usage: None,
                }),
            }
        } else {
            Err(AriaError::new(
                ErrorCode::ContainerExecutionFailed,
                ErrorCategory::Container,
                ErrorSeverity::High,
                "Container manager not available for workload execution"
            ))
        }
    }

    /// Create context environment variables for container execution
    fn create_context_environment(
        &self,
        context: &RuntimeContext,
        session_id: Uuid,
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

        // Enhanced system prompt for orchestration
        let orchestration_prompt = self.build_orchestration_prompt(task, agent_config);
        
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
            })),
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
            provider: None,
            model: Some(agent_config.llm.model.clone()),
            temperature: Some(agent_config.llm.temperature.unwrap_or(0.7)),
            max_tokens: Some(agent_config.llm.max_tokens.unwrap_or(2000)),
            stream: false,
            tools: None,
            config: crate::engines::llm::types::LLMConfig {
                model: agent_config.llm.model.clone(),
                temperature: agent_config.llm.temperature.unwrap_or(0.7),
                max_tokens: agent_config.llm.max_tokens.unwrap_or(2000),
                top_p: None,
                stop_sequences: None,
                stream: false,
            },
            expects_json: true,
        };

        let llm_response = self.llm_handler.complete(llm_request).await?;

        // Parse JSON response
        let parsed_json: Value = serde_json::from_str(&llm_response.content)
            .map_err(|e| AriaError::new(
                ErrorCode::LLMInvalidResponse,
                ErrorCategory::LLM,
                ErrorSeverity::Medium,
                format!("Invalid JSON response from LLM: {}", e)
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
                        parameters.clone(),
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
    async fn execute_single_step(
        &self,
        task: &str,
        agent_config: &AgentConfig,
        _context: &RuntimeContext,
    ) -> AriaResult<ToolResult> {
        let system_prompt = self.build_system_prompt(agent_config);
        let has_tools = !agent_config.tools.is_empty();

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

        let llm_request = LLMRequest {
            messages,
            provider: None,
            model: Some(agent_config.llm.model.clone()),
            temperature: Some(agent_config.llm.temperature.unwrap_or(0.7)),
            max_tokens: Some(agent_config.llm.max_tokens.unwrap_or(2000)),
            stream: false,
            tools: None,
            config: crate::engines::llm::types::LLMConfig {
                model: agent_config.llm.model.clone(),
                temperature: agent_config.llm.temperature.unwrap_or(0.7),
                max_tokens: agent_config.llm.max_tokens.unwrap_or(2000),
                top_p: None,
                stop_sequences: None,
                stream: false,
            },
            expects_json: has_tools,
        };

        let llm_response = self.llm_handler.complete(llm_request).await?;

        if has_tools {
            // Parse JSON response and potentially execute tools
            self.parse_and_execute_tools(&llm_response.content, agent_config).await
        } else {
            // Direct LLM response
            Ok(ToolResult {
                success: true,
                result: Some(serde_json::json!({
                    "response": llm_response.content,
                    "reasoning": "Direct LLM response as agent has no tools",
                    "agent": agent_config.name,
                    "model": llm_response.model
                })),
                error: None,
                metadata: HashMap::new(),
                execution_time_ms: 0,
                resource_usage: None,
            })
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
                format!("Failed to parse LLM response as JSON: {}", e)
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
                    })),
                    error: None,
                    metadata: HashMap::new(),
                    execution_time_ms: 0,
                    resource_usage: None,
                });
            } else if let Some(parameters) = parsed_json.get("parameters") {
                // Execute the specified tool
                let tool_result = self.tool_registry.execute_tool(tool_name, parameters.clone()).await?;
                
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
                    })),
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

    /// Build system prompt for agent
    fn build_system_prompt(&self, agent_config: &AgentConfig) -> String {
        let mut prompt = agent_config.system_prompt.clone()
            .unwrap_or_else(|| "You are a helpful AI assistant.".to_string());

        if !agent_config.tools.is_empty() {
            prompt.push_str(&format!(
                "\n\nYou have access to the following tools: {}\n\n",
                agent_config.tools.join(", ")
            ));
            
            prompt.push_str("--- BEGIN SDK JSON REQUIREMENTS ---\n");
            prompt.push_str("YOUR ENTIRE RESPONSE MUST BE A SINGLE VALID JSON OBJECT.\n");
            prompt.push_str("TO USE A TOOL: your JSON object MUST contain a \"tool_name\" (string) key AND a \"parameters\" (object) key.\n");
            prompt.push_str("IF NO TOOL IS NEEDED: your JSON object MUST contain a \"tool_name\" (string) key set EXPLICITLY to \"none\", AND a \"response\" (string) key with your direct textual answer.\n");
            prompt.push_str("FAILURE TO ADHERE TO THIS JSON STRUCTURE WILL RESULT IN AN ERROR.\n");
            prompt.push_str("--- END SDK JSON REQUIREMENTS ---");
        }

        if let Some(directives) = &agent_config.directives {
            prompt.push_str(&format!("\n\nAdditional Directives:\n{}", directives));
        }

        prompt
    }

    /// Build orchestration prompt for multi-tool workflows
    fn build_orchestration_prompt(&self, task: &str, agent_config: &AgentConfig) -> String {
        let mut prompt = self.build_system_prompt(agent_config);
        
        prompt.push_str(&format!(
            "\n\n--- ORCHESTRATION MODE ---\n\
            You are executing a multi-step task that requires using multiple tools in sequence.\n\n\
            IMPORTANT ORCHESTRATION RULES:\n\
            1. Analyze the full task and identify ALL required steps\n\
            2. Execute ONE tool at a time, in logical order\n\
            3. After each tool execution, I will provide you the result\n\
            4. Continue with the next tool based on the previous results\n\
            5. When all required tools have been executed, respond with \"tool_name\": \"none\" and provide final synthesis\n\n\
            Your task: {}\n\n\
            Start with the FIRST tool needed for this task.", 
            task
        ));

        prompt
    }

    /// Recursive function to traverse and resolve placeholders
    fn traverse_and_resolve(
        &self,
        obj: &mut HashMap<String, Value>,
        history: &[ExecutionStep],
        full_regex: &Regex,
        partial_regex: &Regex,
    ) -> AriaResult<()> {
        for (_key, value) in obj.iter_mut() {
            match value {
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
                                match resolved_value {
                                    Value::String(s) => s,
                                    other => serde_json::to_string(&other).unwrap_or_default(),
                                }
                            } else {
                                caps.get(0).unwrap().as_str().to_string()
                            }
                        });
                        *value = Value::String(resolved_string.to_string());
                    }
                },
                Value::Object(_nested_obj) => {
                    if let Ok(mut nested_map) = serde_json::from_value::<HashMap<String, Value>>(value.clone()) {
                        self.traverse_and_resolve(&mut nested_map, history, full_regex, partial_regex)?;
                        *value = serde_json::to_value(nested_map).unwrap_or(Value::Null);
                    }
                },
                Value::Array(arr) => {
                    for item in arr.iter_mut() {
                        if let Value::Object(_) = item {
                            if let Ok(mut item_map) = serde_json::from_value::<HashMap<String, Value>>(item.clone()) {
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
    ) -> Option<Value> {
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
                                    current_value = next_value.clone();
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

#[async_trait]
impl RuntimeEngine for ExecutionEngine {
    async fn initialize(&self) -> AriaResult<()> {
        // TODO: Initialize tool registry and LLM handler
        Ok(())
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
    
    async fn health_check(&self) -> AriaResult<bool> {
        // TODO: Check if all dependencies are healthy
        Ok(true)
    }
    
    async fn shutdown(&self) -> AriaResult<()> {
        // TODO: Cleanup resources
        Ok(())
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
                })),
                error: None,
                metadata: HashMap::new(),
                execution_time_ms: 0,
                resource_usage: None,
            }
        } else if let Some(tool_name) = &step.tool_name {
            // Execute the tool
            let params_value = serde_json::to_value(&resolved_parameters).unwrap_or(Value::Null);
            self.tool_registry.execute_tool(tool_name, params_value).await?
        } else {
            return Err(AriaError::new(
                ErrorCode::StepExecutionFailed,
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
            step_id: Uuid::new_v4(),
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
        context: &RuntimeContext,
        session_id: Uuid,
    ) -> AriaResult<ToolResult> {
        self.execute_container_workload(spec, context, session_id).await
    }

    fn detect_multi_tool_requirement(&self, task: &str) -> bool {
        self.detect_multi_tool_requirement(task)
    }
    
    fn resolve_placeholders(
        &self,
        parameters: &HashMap<String, serde_json::Value>,
        history: &[ExecutionStep],
    ) -> AriaResult<HashMap<String, serde_json::Value>> {
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

/// Trait for tool registry integration
#[async_trait]
pub trait ToolRegistryInterface: Send + Sync {
    async fn execute_tool(&self, name: &str, parameters: Value) -> AriaResult<ToolResult>;
    async fn get_tool_info(&self, name: &str) -> AriaResult<Option<RegistryEntry>>;
    async fn list_available_tools(&self) -> AriaResult<Vec<String>>;
} 