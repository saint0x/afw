use crate::errors::AriaResult;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use tokio::sync::RwLock;

use crate::errors::{AriaError, ErrorCode, ErrorCategory, ErrorSeverity};
use crate::engines::{AriaEngines, RuntimeEngine, ExecutionEngineInterface, PlanningEngineInterface, ConversationEngineInterface, ReflectionEngineInterface, ContextManagerInterface};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub enhanced_runtime: bool,
    pub planning_threshold: TaskComplexity,
    pub reflection_enabled: bool,
    pub max_steps_per_plan: usize,
    pub timeout_ms: u64,
    pub retry_attempts: u32,
    pub debug_mode: bool,
    pub memory_limit_mb: u64,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            enhanced_runtime: true,
            planning_threshold: TaskComplexity::MultiStep,
            reflection_enabled: true,
            max_steps_per_plan: 10,
            timeout_ms: 300_000, // 5 minutes
            retry_attempts: 3,
            debug_mode: false,
            memory_limit_mb: 512,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RuntimeResult {
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub metrics: ExecutionMetrics,
}

/// Main Aria Runtime orchestrator - preserves Symphony's cognitive architecture
/// while adding container orchestration capabilities
pub struct AriaRuntime {
    engines: Arc<AriaEngines>,
    config: RuntimeConfig,
    status: Arc<RwLock<RuntimeStatus>>,
    metrics: Arc<RwLock<RuntimeMetrics>>,
    active_sessions: Arc<RwLock<std::collections::HashMap<Uuid, RuntimeContext>>>,
}

impl AriaRuntime {
    /// Create a new Aria Runtime instance
    pub fn new(engines: AriaEngines, config: RuntimeConfig) -> Self {
        Self {
            engines: Arc::new(engines),
            config,
            status: Arc::new(RwLock::new(RuntimeStatus::Initializing)),
            metrics: Arc::new(RwLock::new(Self::create_initial_metrics())),
            active_sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Initialize the runtime and all engines
    pub async fn initialize(&self) -> AriaResult<()> {
        *self.status.write().await = RuntimeStatus::Initializing;
        
        // Initialize all engines
        self.engines.initialize_all().await.map_err(|e| {
            AriaError::new(
                ErrorCode::InitializationFailed,
                ErrorCategory::System,
                ErrorSeverity::Critical,
                format!("Failed to initialize engines: {}", e),
            )
            .with_component("AriaRuntime")
            .with_operation("initialize")
        })?;

        *self.status.write().await = RuntimeStatus::Ready;
        Ok(())
    }

    /// Execute a task using the full Aria cognitive architecture
    /// This preserves Symphony's intelligence while adding container capabilities
    pub async fn execute(
        &self,
        task: &str,
        agent_config: &AgentConfig,
        session_id: Option<Uuid>,
    ) -> AriaResult<RuntimeResult> {
        // Ensure runtime is ready
        let status = self.status.read().await.clone();
        if status != RuntimeStatus::Ready {
            return Err(AriaError::new(
                ErrorCode::ExecutionFailed,
                ErrorCategory::Execution,
                ErrorSeverity::High,
                format!("Runtime not ready. Status: {:?}", status),
            )
            .with_component("AriaRuntime")
            .with_operation("execute"));
        }

        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let session_id = session_id.unwrap_or_else(Uuid::new_v4);
        
        // Create runtime context
        let mut context = self.create_runtime_context(agent_config.clone(), session_id);
        
        // Update status to executing
        *self.status.write().await = RuntimeStatus::Executing;
        
        // Store active session
        self.active_sessions.write().await.insert(session_id, context.clone());

        let result = self.execute_with_context(task, &mut context).await;

        // Clean up session
        self.active_sessions.write().await.remove(&session_id);
        
        // Update status back to ready
        *self.status.write().await = RuntimeStatus::Ready;

        // Update metrics
        self.update_metrics(start_time, &result).await;

        result
    }

    /// Execute with full cognitive architecture (Symphony pattern)
    async fn execute_with_context(
        &self,
        task: &str,
        context: &mut RuntimeContext,
    ) -> AriaResult<RuntimeResult> {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Phase 1: Conversation Initiation
        let mut conversation = self.engines.conversation
            .initiate(task, context)
            .await?;

        // Phase 2: Task Analysis & Planning
        let task_analysis = self.engines.planning
            .analyze_task(task, context)
            .await?;

        let execution_mode = self.determine_execution_mode(&task_analysis, &context.agent_config);
        
        let result = match execution_mode {
            RuntimeExecutionMode::EnhancedPlanning if task_analysis.requires_planning => {
                self.execute_planned_task(task, context, &mut conversation).await?
            }
            RuntimeExecutionMode::ContainerWorkload if task_analysis.requires_containers => {
                self.execute_container_workload(task, context, &mut conversation).await?
            }
            _ => {
                self.execute_single_shot_task(task, context, &mut conversation).await?
            }
        };

        // Phase 3: Conversation Conclusion
        self.engines.conversation
            .conclude(&mut conversation, context)
            .await?;

        // Phase 4: Final Result Construction
        let final_result = self.construct_final_result(
            result.success,
            execution_mode,
            Some(conversation),
            context,
            start_time,
            result.error,
        ).await;

        Ok(final_result)
    }

    /// Execute a planned multi-step task (preserves Symphony's planning logic)
    async fn execute_planned_task(
        &self,
        task: &str,
        context: &mut RuntimeContext,
        conversation: &mut ConversationJSON,
    ) -> AriaResult<InternalExecutionResult> {
        // Create execution plan
        let plan = self.engines.planning
            .create_execution_plan(task, &context.agent_config, context)
            .await?;

        self.engines.context_manager.set_plan(plan.clone()).await?;
        context.current_plan = Some(plan.clone());
        context.total_steps = plan.steps.len() as u32;

        let mut overall_success = true;
        let mut primary_error: Option<String> = None;

        // Execute each step in the plan
        for (index, step) in plan.steps.iter().enumerate() {
            // Add conversation turn for step
            let step_content = format!("Executing step {}: {}", index + 1, step.description);
            self.add_conversation_turn(conversation, ConversationRole::Assistant, &step_content, Some(ConversationMetadata {
                step_id: Some(step.id),
                tool_used: step.tool_name.clone(),
                agent_used: step.agent_name.clone(),
                action_type: Some(self.step_type_to_action_type(&step.step_type)),
                confidence: None,
                reflection: Some(false),
            }));

            // Resolve parameters using Symphony's placeholder resolution
            let resolved_parameters = self.engines.execution
                .resolve_placeholders(&step.parameters, &context.execution_history)?;

            // Create planned step with resolved parameters
            let mut resolved_step = step.clone();
            resolved_step.parameters = resolved_parameters;

            // Execute the step
            let execution_step = self.engines.execution
                .execute_step(&resolved_step, context)
                .await?;

            // Record the step
            self.engines.context_manager
                .record_step(execution_step.clone())
                .await?;

            context.execution_history.push(execution_step.clone());
            context.current_step = (index + 1) as u32;

            // Reflection (if enabled and step failed)
            if self.config.reflection_enabled && !execution_step.success {
                let reflection = self.engines.reflection
                    .reflect(&execution_step, context)
                    .await?;

                self.engines.context_manager
                    .record_reflection(reflection.clone())
                    .await?;

                context.reflections.push(reflection.clone());

                // Add reflection to conversation
                let reflection_content = format!("Reflection: {}", reflection.reasoning);
                self.add_conversation_turn(conversation, ConversationRole::Assistant, &reflection_content, Some(ConversationMetadata {
                    step_id: Some(execution_step.step_id),
                    tool_used: None,
                    agent_used: None,
                    action_type: None,
                    confidence: Some(reflection.confidence),
                    reflection: Some(true),
                }));
            }

            // Check if step failed
            if !execution_step.success {
                overall_success = false;
                primary_error = execution_step.error.clone();
                
                // Add failure to conversation
                let failure_content = format!("Step failed: {}. Error: {}", 
                    step.description, 
                    execution_step.error.clone().unwrap_or_else(|| "Unknown error".to_string())
                );
                self.add_conversation_turn(conversation, ConversationRole::Assistant, &failure_content, None);
                
                // Update context status
                context.status = ExecutionStatus::Failed;
                self.engines.context_manager.update_status(ExecutionStatus::Failed).await?;
                break;
            }
        }

        if overall_success {
            context.status = ExecutionStatus::Succeeded;
            self.engines.context_manager.update_status(ExecutionStatus::Succeeded).await?;
        }

        Ok(InternalExecutionResult {
            success: overall_success,
            error: primary_error,
        })
    }

    /// Execute a single-shot task (preserves Symphony's single execution logic)
    async fn execute_single_shot_task(
        &self,
        task: &str,
        context: &mut RuntimeContext,
        conversation: &mut ConversationJSON,
    ) -> AriaResult<InternalExecutionResult> {
        // Add task to conversation
        self.add_conversation_turn(conversation, ConversationRole::Assistant, 
            &format!("Executing task: {}", task), None);

        // Execute using execution engine
        let execution_result = self.engines.execution
            .execute(task, &context.agent_config, context)
            .await?;

        // Create execution step
        let step_id = Uuid::new_v4();
        let execution_step = ExecutionStep {
            step_id,
            description: task.to_string(),
            start_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
            end_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
            duration: 0, // TODO: Calculate actual duration
            success: execution_result.success,
            step_type: StepType::ToolCall, // Default for single-shot
            tool_used: None, // Will be populated by execution engine
            agent_used: None,
            container_used: None,
            parameters: std::collections::HashMap::new(),
            result: execution_result.result.clone(),
            error: execution_result.error.clone(),
            reflection: None,
            summary: format!("Single-shot task execution: {}", 
                if execution_result.success { "succeeded" } else { "failed" }),
            resource_usage: execution_result.resource_usage.clone(),
        };

        // Record the step
        self.engines.context_manager
            .record_step(execution_step.clone())
            .await?;

        context.execution_history.push(execution_step.clone());
        context.current_step = 1;
        context.total_steps = 1;

        // Update conversation with result
        if execution_result.success {
            let result_content = execution_result.result
                .as_ref()
                .and_then(|r| r.get("response"))
                .and_then(|r| r.as_str())
                .unwrap_or("Task completed successfully");
            
            self.add_conversation_turn(conversation, ConversationRole::Assistant, 
                &format!("Task completed: {}", result_content), None);
            
            context.status = ExecutionStatus::Succeeded;
        } else {
            let error_msg = execution_result.error
                .as_deref()
                .unwrap_or("Unknown error");
                
            self.add_conversation_turn(conversation, ConversationRole::Assistant, 
                &format!("Task failed: {}", error_msg), None);
            
            context.status = ExecutionStatus::Failed;
        }

        self.engines.context_manager.update_status(context.status.clone()).await?;

        Ok(InternalExecutionResult {
            success: execution_result.success,
            error: execution_result.error,
        })
    }

    /// Execute container workload (Aria enhancement)
    async fn execute_container_workload(
        &self,
        task: &str,
        context: &mut RuntimeContext,
        conversation: &mut ConversationJSON,
    ) -> AriaResult<InternalExecutionResult> {
        // This will be implemented when we create the container engine
        // For now, fall back to regular execution
        self.add_conversation_turn(conversation, ConversationRole::Assistant, 
            "Container workload execution not yet implemented, falling back to regular execution", None);
        
        self.execute_single_shot_task(task, context, conversation).await
    }

    /// Determine execution mode based on task analysis
    fn determine_execution_mode(
        &self,
        analysis: &TaskAnalysis,
        _agent_config: &AgentConfig,
    ) -> RuntimeExecutionMode {
        if analysis.requires_containers {
            RuntimeExecutionMode::ContainerWorkload
        } else if self.config.enhanced_runtime && analysis.requires_planning {
            RuntimeExecutionMode::EnhancedPlanning
        } else if self.config.reflection_enabled {
            RuntimeExecutionMode::AdaptiveReflection
        } else {
            RuntimeExecutionMode::LegacyWithConversation
        }
    }

    /// Create runtime context for execution
    fn create_runtime_context(&self, agent_config: AgentConfig, session_id: Uuid) -> RuntimeContext {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        RuntimeContext {
            session_id,
            agent_config,
            created_at: now,
            conversation: None,
            status: ExecutionStatus::Running,
            current_plan: None,
            execution_history: Vec::new(),
            working_memory: Arc::new(RwLock::new(std::collections::HashMap::new())),
            insights: Vec::new(),
            error_history: Vec::new(),
            current_step: 0,
            total_steps: 0,
            remaining_steps: Vec::new(),
            reflections: Vec::new(),
            memory_size: 0,
            max_memory_size: self.config.memory_limit_mb * 1024 * 1024, // Convert MB to bytes
        }
    }

    /// Add a turn to the conversation
    fn add_conversation_turn(
        &self,
        conversation: &mut ConversationJSON,
        role: ConversationRole,
        content: &str,
        metadata: Option<ConversationMetadata>,
    ) {
        let turn = ConversationTurn {
            id: Uuid::new_v4(),
            role,
            content: content.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata,
        };
        conversation.turns.push(turn);
    }

    /// Convert step type to action type
    fn step_type_to_action_type(&self, step_type: &StepType) -> ActionType {
        match step_type {
            StepType::ToolCall => ActionType::Other,
            StepType::AgentInvocation => ActionType::Communicate,
            StepType::ContainerWorkload => ActionType::Container,
            StepType::PipelineExecution => ActionType::Other,
            StepType::ReasoningStep => ActionType::Analyze,
        }
    }

    /// Construct final result
    async fn construct_final_result(
        &self,
        success: bool,
        mode: RuntimeExecutionMode,
        conversation: Option<ConversationJSON>,
        context: &RuntimeContext,
        start_time: u64,
        _error: Option<String>,
    ) -> RuntimeResult {
        let end_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        RuntimeResult {
            success,
            result: if success { 
                Some(serde_json::json!({
                    "mode": mode,
                    "conversation": conversation,
                    "steps": context.execution_history.len(),
                    "reflections": context.reflections.len()
                }))
            } else { 
                None 
            },
            metrics: ExecutionMetrics {
                total_duration: Duration::from_millis(end_time - start_time),
                planning_duration: Duration::ZERO, // TODO: Track separately
                execution_duration: Duration::from_millis(end_time - start_time),
                reflection_duration: Duration::ZERO, // TODO: Track separately
                step_count: context.execution_history.len() as u32,
                tool_call_count: context.execution_history.iter().filter(|s| matches!(s.step_type, StepType::ToolCall)).count() as u32,
                llm_call_count: 0, // TODO: Track LLM calls
                error_count: context.execution_history.iter().filter(|s| !s.success).count() as u32,
                recovery_count: 0, // TODO: Track recovery attempts
                cache_hit_rate: 0.0, // TODO: Track cache hits
                success_rate: if context.execution_history.is_empty() { 
                    0.0 
                } else { 
                    context.execution_history.iter().filter(|s| s.success).count() as f32 / context.execution_history.len() as f32 
                },
                start_time: SystemTime::now(),
                end_time: Some(SystemTime::now()),
            },
        }
    }

    /// Update runtime metrics
    async fn update_metrics(&self, start_time: u64, result: &AriaResult<RuntimeResult>) {
        let mut metrics = self.metrics.write().await;
        
        let end_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
            
        metrics.total_duration += end_time - start_time;
        metrics.step_count += 1;
        
        if let Ok(runtime_result) = result {
            metrics.tool_calls += runtime_result.metrics.tool_call_count;
            metrics.container_calls += 0; // TODO: Track container calls
            metrics.agent_calls += 0; // TODO: Track agent calls
            metrics.reflection_count += 0; // TODO: Track reflections
            metrics.adaptation_count += 0; // TODO: Track adaptations
        }
    }

    /// Create initial metrics
    fn create_initial_metrics() -> RuntimeMetrics {
        RuntimeMetrics {
            total_duration: 0,
            start_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            end_time: 0,
            step_count: 0,
            tool_calls: 0,
            container_calls: 0,
            agent_calls: 0,
            token_usage: None,
            reflection_count: 0,
            adaptation_count: 0,
            memory_usage: MemoryUsage {
                current_size: 0,
                max_size: 0,
                utilization_percent: 0.0,
                item_count: 0,
            },
        }
    }

    /// Shutdown the runtime
    pub async fn shutdown(&self) -> AriaResult<()> {
        *self.status.write().await = RuntimeStatus::Shutdown;
        
        // Shutdown all engines
        self.engines.shutdown_all().await?;
        
        // Clear active sessions
        self.active_sessions.write().await.clear();
        
        Ok(())
    }

    /// Get runtime status
    pub async fn get_status(&self) -> RuntimeStatus {
        self.status.read().await.clone()
    }

    /// Get runtime metrics
    pub async fn get_metrics(&self) -> RuntimeMetrics {
        self.metrics.read().await.clone()
    }

    /// Health check
    pub async fn health_check(&self) -> AriaResult<std::collections::HashMap<String, bool>> {
        let overall_health = self.engines.health_check_all().await?;
        let mut health_map = std::collections::HashMap::new();
        health_map.insert("overall".to_string(), overall_health);
        health_map.insert("execution".to_string(), self.engines.execution.health_check().await?);
        health_map.insert("planning".to_string(), self.engines.planning.health_check().await?);
        health_map.insert("conversation".to_string(), self.engines.conversation.health_check().await?);
        health_map.insert("reflection".to_string(), self.engines.reflection.health_check().await?);
        health_map.insert("context_manager".to_string(), self.engines.context_manager.health_check().await?);
        Ok(health_map)
    }

    /// Get active sessions
    pub async fn get_active_sessions(&self) -> Vec<Uuid> {
        self.active_sessions.read().await.keys().cloned().collect()
    }
}

// Internal result type for execution phases
#[derive(Debug)]
struct InternalExecutionResult {
    success: bool,
    error: Option<String>,
} 