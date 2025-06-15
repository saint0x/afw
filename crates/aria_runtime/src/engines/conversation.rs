use crate::deep_size::DeepUuid;
use crate::engines::llm::types::{LLMConfig, LLMMessage, LLMRequest};
use crate::engines::llm::{LLMHandler, LLMHandlerInterface};
use crate::engines::{ConversationEngineInterface, Engine};
use crate::errors::AriaResult;
use crate::types::*;
use async_trait::async_trait;
use uuid::Uuid;

/// Conversation engine for managing conversational flow with LLM integration
pub struct ConversationEngine {
    llm_handler: Option<Box<dyn LLMHandlerInterface>>,
}

impl ConversationEngine {
    pub fn new(llm_handler: Option<Box<dyn LLMHandlerInterface>>) -> Self {
        Self {
            llm_handler,
        }
    }

    /// Generate opening response using LLM (like Symphony does)
    async fn generate_opening_response(&self, task: &str) -> AriaResult<String> {
        if let Some(ref llm_handler) = self.llm_handler {
            let request = LLMRequest {
                messages: vec![
                    LLMMessage {
                        role: "system".to_string(),
                        content: "You are a helpful AI assistant. Generate a brief, professional acknowledgment that you understand the task and are starting work.".to_string(),
                        tool_calls: None,
                        tool_call_id: None,
                    },
                    LLMMessage {
                        role: "user".to_string(),
                        content: format!("I need you to: {}", task),
                        tool_calls: None,
                        tool_call_id: None,
                    },
                ],
                config: LLMConfig {
                    model: Some("gpt-4".to_string()),
                    temperature: 0.5,
                    max_tokens: 150,
                    top_p: None,
                    frequency_penalty: None,
                    presence_penalty: None,
                },
                provider: None,
                tools: None,
                tool_choice: None,
                stream: Some(false),
            };

            match llm_handler.complete(request).await {
                Ok(response) => Ok(response.content),
                Err(_) => Ok(format!("Understood. Starting task: {}", task)),
            }
        } else {
            Ok(format!("Understood. Starting task: {}", task))
        }
    }

    /// Generate final summary using LLM (matching Symphony's conclude method)
    async fn generate_final_summary(&self, conversation: &ConversationJSON, context: &RuntimeContext) -> AriaResult<String> {
        if let Some(ref llm_handler) = self.llm_handler {
            // Create conversation history for summary
            let history = conversation.turns.iter()
                .map(|turn| format!("{}: {}", 
                    match turn.role {
                        ConversationRole::User => "user",
                        ConversationRole::Assistant => "assistant",
                        ConversationRole::System => "system",
                        ConversationRole::Tool => "tool",
                        ConversationRole::Agent => "agent",
                        ConversationRole::Container => "container",
                    },
                    turn.content
                ))
                .collect::<Vec<_>>()
                .join("\n");

            let summary_prompt = format!(
                "Based on the following conversation and execution context, provide a concise summary of what was accomplished:\n\n{}\n\nExecution Summary:\n- Steps completed: {}\n- Success rate: {:.1}%\n- Agent: {}",
                history,
                context.execution_history.len(),
                (context.execution_history.iter().filter(|s| s.success).count() as f32 / context.execution_history.len().max(1) as f32) * 100.0,
                context.agent_config.name
            );

            let request = LLMRequest {
                messages: vec![
                    LLMMessage {
                        role: "system".to_string(),
                        content: "You are an expert at summarizing task completion. Provide a clear, concise summary of what was accomplished.".to_string(),
                        tool_calls: None,
                        tool_call_id: None,
                    },
                    LLMMessage {
                        role: "user".to_string(),
                        content: summary_prompt,
                        tool_calls: None,
                        tool_call_id: None,
                    },
                ],
                config: LLMConfig {
                    model: Some(context.agent_config.llm.model.clone()),
                    temperature: 0.5,
                    max_tokens: 300,
                    top_p: None,
                    frequency_penalty: None,
                    presence_penalty: None,
                },
                provider: None,
                tools: None,
                tool_choice: None,
                stream: Some(false),
            };

            match llm_handler.complete(request).await {
                Ok(response) => Ok(response.content),
                Err(_) => Ok("I was unable to summarize my work, but the task execution is complete.".to_string()),
            }
        } else {
            Ok("Task execution completed. LLM handler not available for detailed summary.".to_string())
        }
    }
}

impl Engine for ConversationEngine {
    fn initialize(&self) -> bool {
        true
    }
    
    fn get_dependencies(&self) -> Vec<String> {
        vec!["llm_handler".to_string()]
    }
    
    fn get_state(&self) -> String {
        "ready".to_string()
    }
    
    fn health_check(&self) -> bool {
        true // ConversationEngine can work even without LLM handler
    }
    
    fn shutdown(&self) -> bool {
        true
    }
}

#[async_trait]
impl ConversationEngineInterface for ConversationEngine {
    async fn initiate(
        &self,
        task: &str,
        context: &RuntimeContext,
    ) -> AriaResult<ConversationJSON> {
        // Generate intelligent opening response (like Symphony does)
        let opening_response = self.generate_opening_response(task).await
            .unwrap_or_else(|_| format!("Understood. Starting task: {}", task));

        let conversation = ConversationJSON {
            id: DeepUuid(Uuid::new_v4()),
            original_task: task.to_string(),
            turns: vec![
                // User's initial request
                ConversationTurn {
                    id: DeepUuid(Uuid::new_v4()),
                    role: ConversationRole::User,
                    content: task.to_string(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    metadata: Some(ConversationMetadata {
                        step_id: None,
                        tool_used: None,
                        agent_used: Some(context.agent_config.name.clone()),
                        action_type: Some(ActionType::Other),
                        confidence: None,
                        reflection: Some(false),
                    }),
                },
                // Assistant's acknowledgment
                ConversationTurn {
                    id: DeepUuid(Uuid::new_v4()),
                    role: ConversationRole::Assistant,
                    content: opening_response,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    metadata: Some(ConversationMetadata {
                        step_id: None,
                        tool_used: None,
                        agent_used: Some(context.agent_config.name.clone()),
                        action_type: Some(ActionType::Communicate),
                        confidence: Some(0.9),
                        reflection: Some(false),
                    }),
                }
            ],
            final_response: String::new(),
            reasoning_chain: vec![],
            duration: 0,
            state: ConversationState::Working,
        };
        Ok(conversation)
    }
    
    async fn update(
        &self,
        conversation: &mut ConversationJSON,
        step_result: &ExecutionStep,
    ) -> AriaResult<()> {
        // Add step result to conversation (matching Symphony's pattern)
        let step_summary = if step_result.success {
            format!("✓ Completed: {}", step_result.description)
        } else {
            format!("✗ Failed: {} (Error: {})", 
                   step_result.description, 
                   step_result.error.as_ref().unwrap_or(&"Unknown error".to_string()))
        };

        conversation.turns.push(ConversationTurn {
            id: DeepUuid(Uuid::new_v4()),
            role: ConversationRole::Assistant,
            content: step_summary,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: Some(ConversationMetadata {
                step_id: Some(step_result.step_id),
                tool_used: step_result.tool_used.clone(),
                agent_used: step_result.agent_used.clone(),
                action_type: Some(match step_result.step_type {
                    StepType::ToolCall => ActionType::Other,
                    StepType::AgentInvocation => ActionType::Communicate,
                    StepType::ContainerWorkload => ActionType::Container,
                    StepType::ReasoningStep => ActionType::Analyze,
                    StepType::PipelineExecution => ActionType::Other,
                }),
                confidence: Some(if step_result.success { 0.8 } else { 0.3 }),
                reflection: Some(false),
            }),
        });

        // Update reasoning chain
        if let Some(ref tool) = step_result.tool_used {
            conversation.reasoning_chain.push(
                format!("Used {} to: {}", tool, step_result.description)
            );
        }

        // Update conversation state based on execution progress
        conversation.state = if step_result.success {
            ConversationState::Working
        } else {
            ConversationState::Error
        };

        Ok(())
    }
    
    async fn conclude(
        &self,
        conversation: &mut ConversationJSON,
        context: &RuntimeContext,
    ) -> AriaResult<()> {
        conversation.state = ConversationState::Concluding;

        // Generate sophisticated final summary using LLM (like Symphony does)
        let final_response = self.generate_final_summary(conversation, context).await?;

        // Add final summary to conversation
        conversation.turns.push(ConversationTurn {
            id: DeepUuid(Uuid::new_v4()),
            role: ConversationRole::Assistant,
            content: final_response.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: Some(ConversationMetadata {
                step_id: None,
                tool_used: None,
                agent_used: Some(context.agent_config.name.clone()),
                action_type: Some(ActionType::Communicate),
                confidence: Some(0.9),
                reflection: Some(false),
            }),
        });

        conversation.final_response = final_response;
        conversation.state = ConversationState::Completed;

        Ok(())
    }
    
    async fn finalize(
        &self,
        conversation: &mut ConversationJSON,
    ) -> AriaResult<()> {
        // Calculate final duration
        if let (Some(first), Some(last)) = (conversation.turns.first(), conversation.turns.last()) {
            conversation.duration = last.timestamp - first.timestamp;
        }

        // Ensure conversation is marked as completed
        if conversation.state != ConversationState::Error {
            conversation.state = ConversationState::Completed;
        }

        Ok(())
    }
}
