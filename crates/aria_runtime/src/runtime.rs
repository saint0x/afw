use crate::errors::AriaResult;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use tokio::sync::RwLock;
use crate::deep_size::DeepUuid;
use std::path::PathBuf;

use crate::errors::{AriaError, ErrorCode, ErrorCategory, ErrorSeverity};
use crate::engines::{AriaEngines, ExecutionEngineInterface, PlanningEngineInterface, ConversationEngineInterface, ReflectionEngineInterface, ContextManagerInterface};
use downcast_rs::{impl_downcast, Downcast};
use std::collections::HashMap;

// RuntimeConfig is now defined in types.rs as RuntimeConfiguration

// RuntimeResult is defined in types.rs

// Add bundle-related imports
use crate::bundle_discovery::BundleToolDiscovery;
use crate::bundle_executor::{BundleExecutor, BundleExecutionResult, BundleExecutionConfig};
use crate::engines::tool_registry::bundle_integration::BundleToolRegistry;
use crate::engines::execution::tool_resolver::ToolResolver;
use crate::tools::management::custom_tools::CustomToolManager;

/// Main Aria Runtime orchestrator - preserves Symphony's cognitive architecture
/// while adding container orchestration capabilities
#[derive(Clone)]
pub struct AriaRuntime {
    pub config: RuntimeConfiguration,
    pub engines: Arc<AriaEngines>,
    pub status: Arc<RwLock<RuntimeStatus>>,
    
    // TODO: Implement comprehensive metrics collection and reporting
    pub metrics: Arc<RwLock<RuntimeMetrics>>,
    
    // TODO: Implement full session management with persistence and cleanup
    pub active_sessions: Arc<RwLock<HashMap<uuid::Uuid, RuntimeContext>>>,
}

impl AriaRuntime {
    /// Creates a new instance of the Aria Runtime.
    pub async fn new(config: RuntimeConfiguration) -> AriaResult<Self> {
        let engines = Arc::new(AriaEngines::new().await);
        Ok(Self {
            config,
            engines,
            status: Arc::new(RwLock::new(RuntimeStatus::Ready)),
            metrics: Arc::new(RwLock::new(Self::create_initial_metrics())),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Creates a new instance of the Aria Runtime with pre-made engines.
    pub fn with_engines(engines: AriaEngines, config: RuntimeConfiguration) -> Self {
        Self {
            config,
            engines: Arc::new(engines),
            status: Arc::new(RwLock::new(RuntimeStatus::Ready)),
            metrics: Arc::new(RwLock::new(Self::create_initial_metrics())),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Executes a task based on the provided configuration and context.
    pub async fn execute(
        &self,
        task: &str,
        agent_config: AgentConfig,
    ) -> AriaResult<RuntimeResult> {
        println!("🔍 DEBUG: AriaRuntime::execute called");
        println!("🔍 DEBUG: Task: {}", task);
        println!("🔍 DEBUG: Agent: {}", agent_config.name);
        
        let session_id = Uuid::new_v4();
        println!("🔍 DEBUG: Created session ID: {}", session_id);
        
        let mut context = self.create_runtime_context(agent_config, session_id);
        println!("🔍 DEBUG: Created runtime context");
        
        // Track session
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.insert(session_id, context.clone());
            println!("🔍 DEBUG: Added session to active sessions");
        }

        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        println!("🔍 DEBUG: Calling execute_with_context...");
        let result = self.execute_with_context(task, &mut context).await;
        println!("🔍 DEBUG: execute_with_context returned");

        // Clean up session
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.remove(&session_id);
            println!("🔍 DEBUG: Removed session from active sessions");
        }

        // Update metrics
        self.update_metrics(start_time, &result).await;
        println!("🔍 DEBUG: Updated metrics");

        result
    }

    /// Initialize the runtime and all engines
    pub async fn initialize(&self) -> AriaResult<()> {
        *self.status.write().await = RuntimeStatus::Initializing;
        
        // Initialize all engines
        if let Err(e) = self.engines.initialize_all().await {
            return Err(AriaError::new(
                ErrorCode::SystemNotReady,
                ErrorCategory::System,
                ErrorSeverity::Critical,
                &format!("Failed to initialize engines: {}", e),
            ));
        }

        *self.status.write().await = RuntimeStatus::Ready;
        Ok(())
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
                self._internal_execute_container_workload(task, context, &mut conversation).await?
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
        println!("🔍 DEBUG: Starting execute_single_shot_task");
        println!("🔍 DEBUG: Task: {}", task);
        println!("🔍 DEBUG: Agent: {}", context.agent_config.name);
        
        // Add task to conversation
        self.add_conversation_turn(conversation, ConversationRole::Assistant, 
            &format!("Executing task: {}", task), None);

        println!("🔍 DEBUG: Calling execution engine...");
        // Execute using execution engine
        let execution_result = self.engines.execution
            .execute(task, &context.agent_config, context)
            .await?;

        println!("🔍 DEBUG: Execution engine returned!");
        println!("🔍 DEBUG: Success: {}", execution_result.success);
        println!("🔍 DEBUG: Result: {:?}", execution_result.result);
        println!("🔍 DEBUG: Error: {:?}", execution_result.error);

        // Create execution step
        let step_id = crate::deep_size::DeepUuid(Uuid::new_v4());
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        let end_time = start_time + 1; // Minimal duration for now
        
        let execution_step = ExecutionStep {
            step_id,
            description: task.to_string(),
            start_time,
            end_time,
            duration: end_time - start_time,
            success: execution_result.success,
            step_type: StepType::ToolCall, // Default for single-shot
            tool_used: None, // Will be populated by execution engine
            agent_used: Some(context.agent_config.name.clone()),
            container_used: None,
            parameters: std::collections::HashMap::new(),
            result: execution_result.result.clone(),
            error: execution_result.error.clone(),
            reflection: None,
            summary: format!("Single-shot task execution: {}", 
                if execution_result.success { "succeeded" } else { "failed" }),
            resource_usage: execution_result.resource_usage.clone(),
        };

        println!("🔍 DEBUG: Created ExecutionStep with ID: {:?}", execution_step.step_id);
        println!("🔍 DEBUG: ExecutionStep success: {}", execution_step.success);

        // Record the step
        println!("🔍 DEBUG: Recording step in context manager...");
        self.engines.context_manager
            .record_step(execution_step.clone())
            .await?;

        println!("🔍 DEBUG: Adding step to execution history...");
        context.execution_history.push(execution_step.clone());
        context.current_step = 1;
        context.total_steps = 1;
        
        println!("🔍 DEBUG: Updated context - current_step: {}, total_steps: {}", 
            context.current_step, context.total_steps);
        println!("🔍 DEBUG: Execution history length: {}", context.execution_history.len());

        // Update conversation with result
        if execution_result.success {
            let result_content = execution_result.result
                .as_ref()
                .and_then(|r| r.get("response"))
                .and_then(|r| r.as_str())
                .unwrap_or("Task completed successfully");
            
            println!("🔍 DEBUG: Extracted response content: {}", result_content);
            
            self.add_conversation_turn(conversation, ConversationRole::Assistant, 
                &format!("Task completed: {}", result_content), None);
            
            // CRITICAL FIX: Set the final_response field for success detection
            conversation.final_response = result_content.to_string();
            conversation.state = ConversationState::Completed;
            
            context.status = ExecutionStatus::Succeeded;
        } else {
            let error_msg = execution_result.error
                .as_deref()
                .unwrap_or("Unknown error");
                
            println!("🔍 DEBUG: Task failed with error: {}", error_msg);
                
            self.add_conversation_turn(conversation, ConversationRole::Assistant, 
                &format!("Task failed: {}", error_msg), None);
            
            // Set final_response even for failures (for debugging)
            conversation.final_response = format!("ERROR: {}", error_msg);
            conversation.state = ConversationState::Error;
            
            context.status = ExecutionStatus::Failed;
        }

        self.engines.context_manager.update_status(context.status.clone()).await?;

        println!("🔍 DEBUG: execute_single_shot_task completed");
        println!("🔍 DEBUG: Final success: {}", execution_result.success);

        Ok(InternalExecutionResult {
            success: execution_result.success,
            error: execution_result.error,
        })
    }

    /// Execute container workload (Aria enhancement)
    async fn _internal_execute_container_workload(
        &self,
        task: &str,
        context: &mut RuntimeContext,
        conversation: &mut ConversationJSON,
    ) -> AriaResult<InternalExecutionResult> {
        // This is a placeholder for a more complex container execution flow.
        // For now, it's a simple wrapper around a tool call.
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
            session_id: crate::deep_size::DeepUuid(session_id),
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
            id: crate::deep_size::DeepUuid(Uuid::new_v4()),
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
        error: Option<String>,
    ) -> RuntimeResult {
        println!("🔍 DEBUG: Constructing final result");
        println!("🔍 DEBUG: Success: {}", success);
        println!("🔍 DEBUG: Mode: {:?}", mode);
        println!("🔍 DEBUG: Context current_step: {}", context.current_step);
        println!("🔍 DEBUG: Context total_steps: {}", context.total_steps);
        println!("🔍 DEBUG: Context execution_history length: {}", context.execution_history.len());
        
        // Debug execution history
        for (i, step) in context.execution_history.iter().enumerate() {
            println!("🔍 DEBUG: Step {}: {} - Success: {}", i, step.description, step.success);
            if let Some(result) = &step.result {
                if let Some(response) = result.get("response") {
                    println!("🔍 DEBUG: Step {} response: {}", i, response);
                } else {
                    println!("🔍 DEBUG: Step {} has result but no 'response' field: {:?}", i, result);
                }
            } else {
                println!("🔍 DEBUG: Step {} has no result", i);
            }
        }
        
        let end_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let metrics = RuntimeMetrics {
            total_duration: end_time - start_time,
            start_time,
            end_time,
            step_count: context.execution_history.len() as u32,
            tool_calls: context.execution_history.iter().filter(|s| matches!(s.step_type, StepType::ToolCall)).count() as u32,
            container_calls: 0, // TODO: Track container calls
            agent_calls: 0, // TODO: Track agent calls
            token_usage: None, // TODO: Track token usage
            reflection_count: context.reflections.len() as u32,
            adaptation_count: 0, // TODO: Track adaptations
            memory_usage: MemoryUsage {
                current_size: context.memory_size,
                max_size: context.max_memory_size,
                utilization_percent: if context.max_memory_size > 0 {
                    (context.memory_size as f64 / context.max_memory_size as f64) * 100.0
                } else {
                    0.0
                },
                item_count: context.execution_history.len() as u32,
            },
        };
        
        println!("🔍 DEBUG: Computed metrics:");
        println!("🔍 DEBUG: - total_duration: {}", metrics.total_duration);
        println!("🔍 DEBUG: - step_count: {}", metrics.step_count);
        println!("🔍 DEBUG: - tool_calls: {}", metrics.tool_calls);

        RuntimeResult {
            success,
            mode: mode.clone(),
            conversation,
            execution_details: ExecutionDetails {
                mode,
                step_results: context.execution_history.clone(),
                participating_agents: vec![context.agent_config.name.clone()],
                containers_used: vec![],
                total_steps: context.total_steps,
                completed_steps: context.current_step,
                failed_steps: context.execution_history.iter().filter(|s| !s.success).count() as u32,
                skipped_steps: 0,
                adaptations: vec![], // TODO: Track adaptations
                insights: context.insights.clone(),
                resource_utilization: ResourceUtilization::default(),
            },
            plan: context.current_plan.clone(),
            reflections: context.reflections.clone(),
            error,
            metrics,
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
            metrics.tool_calls += runtime_result.metrics.tool_calls;
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
        if let Err(e) = self.engines.shutdown_all().await {
            return Err(AriaError::new(
                ErrorCode::SystemNotReady,
                ErrorCategory::System,
                ErrorSeverity::Critical,
                &format!("Failed to shutdown engines: {}", e),
            ));
        }
        
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
        let overall_health = match self.engines.health_check_all().await {
            Ok(_) => true,
            Err(_) => false,
        };
        let mut health_map = std::collections::HashMap::new();
        health_map.insert("overall".to_string(), overall_health);
        
        // TODO: Implement individual engine health checks when Engine trait is accessible
        health_map.insert("execution".to_string(), true);
        health_map.insert("planning".to_string(), true);
        health_map.insert("conversation".to_string(), true);
        health_map.insert("reflection".to_string(), true);
        health_map.insert("context_manager".to_string(), true);
        
        Ok(health_map)
    }

    /// Get active sessions
    pub async fn get_active_sessions(&self) -> Vec<Uuid> {
        self.active_sessions.read().await.keys().cloned().collect()
    }

    /// Register tools from a bundle
    pub async fn register_tools_from_bundle(&self, bundle_hash: &str) -> AriaResult<Vec<String>> {
        info!("Registering tools from bundle: {}", bundle_hash);

        // TODO: Need to add pkg_store to AriaEngines and expose bundle functionality
        return Err(AriaError::new(
            ErrorCode::NotSupported,
            ErrorCategory::Bundle,
            ErrorSeverity::Medium,
            "Bundle tool registration not yet implemented - pkg_store not available",
        ));


    }

    /// Execute a bundle workload
    pub async fn execute_bundle_workload(
        &self,
        bundle_hash: &str,
        session_id: DeepUuid,
        config: Option<BundleExecutionConfig>,
    ) -> AriaResult<BundleExecutionResult> {
        info!("Executing bundle workload: {} (session: {})", bundle_hash, session_id);

        // TODO: Need to add pkg_store to AriaEngines and expose bundle functionality
        return Err(AriaError::new(
            ErrorCode::NotSupported,
            ErrorCategory::Bundle,
            ErrorSeverity::Medium,
            "Bundle execution not yet implemented - infrastructure not available",
        ));
    }

    /// Discover a tool in available bundles
    pub async fn discover_tool_in_bundles(&self, tool_name: &str) -> AriaResult<Option<String>> {
        debug!("Discovering tool in bundles: {}", tool_name);

        // TODO: Need to add pkg_store to AriaEngines and expose bundle functionality
        return Err(AriaError::new(
            ErrorCode::NotSupported,
            ErrorCategory::Bundle,
            ErrorSeverity::Medium,
            "Bundle tool discovery not yet implemented - pkg_store not available",
        ));
    }

    /// Get custom tool manager for advanced bundle tool management
    pub async fn get_custom_tool_manager(&self) -> AriaResult<CustomToolManager> {
        debug!("Creating custom tool manager");

        // TODO: Need to add pkg_store to AriaEngines and expose bundle functionality
        return Err(AriaError::new(
            ErrorCode::NotSupported,
            ErrorCategory::Bundle,
            ErrorSeverity::Medium,
            "Custom tool manager not yet implemented - infrastructure not available",
        ));
    }

    /// Auto-discover and register all available bundle tools
    pub async fn auto_discover_bundle_tools(&self) -> AriaResult<usize> {
        info!("Auto-discovering and registering all bundle tools");

        let custom_tool_manager = self.get_custom_tool_manager().await?;
        let discovery_result = custom_tool_manager.discover_and_register_all_tools().await?;

        let registered_count = discovery_result.registration_results.successful.len();
        info!("Auto-discovery completed: {} tools registered", registered_count);

        Ok(registered_count)
    }

    /// Get bundle execution capabilities status
    pub async fn get_bundle_capabilities_status(&self) -> AriaResult<BundleCapabilitiesStatus> {
        debug!("Getting bundle capabilities status");

        // TODO: Need to add pkg_store to AriaEngines and expose bundle functionality
        return Err(AriaError::new(
            ErrorCode::NotSupported,
            ErrorCategory::Bundle,
            ErrorSeverity::Medium,
            "Bundle capabilities status not yet implemented - infrastructure not available",
        ));
    }
}

// Internal result type for execution phases
#[derive(Debug)]
struct InternalExecutionResult {
    success: bool,
    error: Option<String>,
}

/// Bundle capabilities status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleCapabilitiesStatus {
    pub total_available_bundles: usize,
    pub total_available_tools: usize,
    pub registered_custom_tools: usize,
    pub unique_registered_bundles: usize,
    pub auto_discovery_enabled: bool,
} 