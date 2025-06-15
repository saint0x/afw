use crate::types::*;
use std::collections::HashMap;

/// Production-grade system prompt service that generates context-aware prompts
/// for different execution modes, preserving Symphony's sophisticated prompt engineering
pub struct SystemPromptService {
    /// Base templates for different agent types
    base_templates: HashMap<String, String>,
    /// Tool-specific guidance templates
    tool_guidance: HashMap<String, String>,
}

impl SystemPromptService {
    pub fn new() -> Self {
        let mut service = Self {
            base_templates: HashMap::new(),
            tool_guidance: HashMap::new(),
        };
        
        service.initialize_templates();
        service
    }

    /// Initialize all prompt templates
    fn initialize_templates(&mut self) {
        self.load_base_templates();
        self.load_tool_guidance();
    }

    /// Load base system prompt templates for different agent types
    fn load_base_templates(&mut self) {
        // Default agent template
        self.base_templates.insert("default".to_string(), 
            "You are an intelligent AI assistant capable of reasoning, analysis, and task execution.\n\
            You approach problems systematically and provide clear, actionable responses.\n\
            You are precise, helpful, and focused on achieving the user's objectives efficiently.".to_string()
        );

        // Research-focused agent template
        self.base_templates.insert("research".to_string(),
            "You are a research-focused AI assistant with expertise in information gathering and analysis.\n\
            You excel at finding relevant information, synthesizing data from multiple sources, and providing \
            comprehensive insights. You are thorough, accurate, and cite your sources when possible.".to_string()
        );

        // Code-focused agent template
        self.base_templates.insert("coding".to_string(),
            "You are a software engineering AI assistant with deep expertise in programming and system design.\n\
            You write clean, efficient, and well-documented code. You follow best practices, consider edge cases, \
            and provide clear explanations of your implementation decisions.".to_string()
        );

        // Analysis-focused agent template
        self.base_templates.insert("analysis".to_string(),
            "You are an analytical AI assistant specialized in data analysis, pattern recognition, and strategic thinking.\n\
            You break down complex problems, identify key insights, and provide actionable recommendations based on \
            evidence and logical reasoning.".to_string()
        );

        // Creative agent template
        self.base_templates.insert("creative".to_string(),
            "You are a creative AI assistant with expertise in content creation, ideation, and innovative problem-solving.\n\
            You think outside the box, generate original ideas, and help users explore creative solutions to their challenges.".to_string()
        );
    }

    /// Load tool-specific guidance templates
    fn load_tool_guidance(&mut self) {
        // JSON response guidance for tool-enabled agents
        self.tool_guidance.insert("json_tools".to_string(),
            "\n\n--- TOOL EXECUTION PROTOCOL ---\n\
            YOU HAVE ACCESS TO TOOLS. Your response format depends on whether you need to use a tool:\n\n\
            IF YOU NEED TO USE A TOOL:\n\
            - Your ENTIRE response must be a single valid JSON object\n\
            - Include \"tool_name\" (string) with the exact tool name\n\
            - Include \"parameters\" (object) with the tool parameters\n\
            - Example: {\"tool_name\": \"webSearchTool\", \"parameters\": {\"query\": \"latest AI news\"}}\n\n\
            IF NO TOOL IS NEEDED:\n\
            - Your ENTIRE response must be a single valid JSON object\n\
            - Set \"tool_name\" to \"none\"\n\
            - Include \"response\" (string) with your direct answer\n\
            - Example: {\"tool_name\": \"none\", \"response\": \"Here is my direct answer...\"}\n\n\
            CRITICAL: Always respond with valid JSON. Never mix JSON with plain text.".to_string()
        );

        // Orchestration guidance for multi-tool workflows
        self.tool_guidance.insert("orchestration".to_string(),
            "\n\n--- MULTI-TOOL ORCHESTRATION MODE ---\n\
            You are executing a complex task that requires multiple tools in sequence.\n\n\
            ORCHESTRATION RULES:\n\
            1. Analyze the full task and identify ALL required steps\n\
            2. Execute ONE tool at a time, in logical order\n\
            3. After each tool execution, I will provide you the result\n\
            4. Use previous results to inform subsequent tool choices\n\
            5. When all tools have been executed, set \"tool_name\": \"none\" and provide final synthesis\n\n\
            RESPONSE FORMAT:\n\
            - Always respond with valid JSON\n\
            - For tool execution: {\"tool_name\": \"toolName\", \"parameters\": {...}}\n\
            - For final response: {\"tool_name\": \"none\", \"response\": \"Final synthesis...\"}\n\n\
            Think step-by-step and execute tools in the most logical sequence.".to_string()
        );

        // Planning guidance
        self.tool_guidance.insert("planning".to_string(),
            "\n\n--- PLANNING MODE ---\n\
            You are creating a detailed execution plan for a complex task.\n\n\
            PLANNING PRINCIPLES:\n\
            1. Break down the task into clear, actionable steps\n\
            2. Identify the most appropriate tool for each step\n\
            3. Consider dependencies between steps\n\
            4. Plan for error handling and alternative approaches\n\
            5. Ensure each step has clear success criteria\n\n\
            Your plan should be comprehensive yet efficient, avoiding unnecessary complexity.".to_string()
        );

        // Reflection guidance
        self.tool_guidance.insert("reflection".to_string(),
            "\n\n--- REFLECTION MODE ---\n\
            You are analyzing the outcome of a previous action to learn and improve.\n\n\
            REFLECTION FOCUS:\n\
            1. Assess what worked well and what didn't\n\
            2. Identify root causes of any issues\n\
            3. Consider alternative approaches that might be better\n\
            4. Provide specific, actionable recommendations\n\
            5. Rate your confidence in the assessment\n\n\
            Be honest about failures and constructive about improvements.".to_string()
        );
    }

    /// Generate a complete system prompt for an agent configuration
    pub fn generate_system_prompt(&self, agent_config: &AgentConfig, has_tools: bool) -> String {
        let mut prompt = String::new();

        // Start with base prompt
        if let Some(custom_prompt) = &agent_config.system_prompt {
            prompt.push_str(custom_prompt);
        } else {
            // Use template based on agent type or default
            let agent_type = agent_config.agent_type.as_deref().unwrap_or("default");
            let base_template = self.base_templates.get(agent_type)
                .or_else(|| self.base_templates.get("default"))
                .unwrap();
            prompt.push_str(base_template);
        }

        // Add agent-specific context
        if !agent_config.name.is_empty() {
            prompt.push_str(&format!("\n\nYou are operating as '{}' with the following capabilities:", agent_config.name));
        }

        // Add tool information if available
        if has_tools && !agent_config.tools.is_empty() {
            prompt.push_str("\n\nAVAILABLE TOOLS:");
            for tool_name in &agent_config.tools {
                prompt.push_str(&format!("\n- {}", tool_name));
            }
            
            // Add JSON tool guidance
            if let Some(json_guidance) = self.tool_guidance.get("json_tools") {
                prompt.push_str(json_guidance);
            }
        }

        // Add directives if specified
        if let Some(directives) = &agent_config.directives {
            prompt.push_str(&format!("\n\nADDITIONAL DIRECTIVES:\n{}", directives));
        }

        // Add capabilities context
        if !agent_config.capabilities.is_empty() {
            prompt.push_str("\n\nYOUR SPECIALIZED CAPABILITIES:");
            for capability in &agent_config.capabilities {
                prompt.push_str(&format!("\n- {}", capability));
            }
        }

        // Add memory context if available
        if agent_config.memory_enabled.unwrap_or(false) {
            prompt.push_str("\n\nMEMORY: You have access to conversation history and can reference previous interactions.");
        }

        prompt
    }

    /// Generate orchestration-specific system prompt
    pub fn generate_orchestration_prompt(&self, task: &str, agent_config: &AgentConfig) -> String {
        let mut prompt = self.generate_system_prompt(agent_config, true);
        
        // Add orchestration-specific guidance
        if let Some(orchestration_guidance) = self.tool_guidance.get("orchestration") {
            prompt.push_str(orchestration_guidance);
        }

        prompt.push_str(&format!("\n\nYOUR TASK: {}\n\nStart with the FIRST tool needed for this task.", task));
        
        prompt
    }

    /// Generate planning-specific system prompt
    pub fn generate_planning_prompt(&self, objective: &str, agent_config: &AgentConfig, available_tools: &[String]) -> String {
        let mut prompt = self.generate_system_prompt(agent_config, false);
        
        // Add planning-specific guidance
        if let Some(planning_guidance) = self.tool_guidance.get("planning") {
            prompt.push_str(planning_guidance);
        }

        prompt.push_str(&format!("\n\nOBJECTIVE: {}", objective));
        
        if !available_tools.is_empty() {
            prompt.push_str("\n\nAVAILABLE TOOLS FOR PLANNING:");
            for tool in available_tools {
                prompt.push_str(&format!("\n- {}", tool));
            }
        }

        prompt.push_str("\n\nCreate a detailed, step-by-step execution plan in JSON format.");
        
        prompt
    }

    /// Generate reflection-specific system prompt
    pub fn generate_reflection_prompt(&self, step_description: &str, outcome: &str, success: bool) -> String {
        let mut prompt = String::new();
        
        // Base reflection prompt
        prompt.push_str("You are an AI assistant specialized in analyzing execution outcomes and providing strategic insights.");
        
        // Add reflection-specific guidance
        if let Some(reflection_guidance) = self.tool_guidance.get("reflection") {
            prompt.push_str(reflection_guidance);
        }

        prompt.push_str(&format!("\n\nSTEP ANALYZED: {}", step_description));
        prompt.push_str(&format!("\nOUTCOME: {}", outcome));
        prompt.push_str(&format!("\nSUCCESS: {}", if success { "Yes" } else { "No" }));
        
        prompt.push_str("\n\nProvide a comprehensive analysis with specific recommendations for improvement.");
        
        prompt
    }

    /// Generate conversation-specific system prompt
    pub fn generate_conversation_prompt(&self, agent_config: &AgentConfig, conversation_context: &str) -> String {
        let mut prompt = self.generate_system_prompt(agent_config, !agent_config.tools.is_empty());
        
        prompt.push_str("\n\n--- CONVERSATION MODE ---");
        prompt.push_str("\nYou are engaged in a conversational interaction. Maintain context and provide helpful, engaging responses.");
        
        if !conversation_context.is_empty() {
            prompt.push_str(&format!("\n\nCONVERSATION CONTEXT:\n{}", conversation_context));
        }
        
        prompt
    }

    /// Generate container execution prompt
    pub fn generate_container_prompt(&self, workload_description: &str, context_vars: &HashMap<String, String>) -> String {
        let mut prompt = String::new();
        
        prompt.push_str("You are executing within a secure container environment with access to specific resources and capabilities.");
        prompt.push_str(&format!("\n\nWORKLOAD: {}", workload_description));
        
        if !context_vars.is_empty() {
            prompt.push_str("\n\nCONTEXT VARIABLES:");
            for (key, value) in context_vars {
                prompt.push_str(&format!("\n- {}: {}", key, value));
            }
        }
        
        prompt.push_str("\n\nExecute the workload efficiently and return structured results.");
        
        prompt
    }

    /// Add custom tool guidance
    pub fn add_tool_guidance(&mut self, tool_name: &str, guidance: String) {
        self.tool_guidance.insert(tool_name.to_string(), guidance);
    }

    /// Add custom base template
    pub fn add_base_template(&mut self, template_name: &str, template: String) {
        self.base_templates.insert(template_name.to_string(), template);
    }

    /// Get available template names
    pub fn get_available_templates(&self) -> Vec<String> {
        self.base_templates.keys().cloned().collect()
    }

    /// Get available tool guidance types
    pub fn get_available_guidance(&self) -> Vec<String> {
        self.tool_guidance.keys().cloned().collect()
    }
}

impl Default for SystemPromptService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_prompt_generation() {
        let service = SystemPromptService::new();
        let agent_config = AgentConfig {
            name: "TestAgent".to_string(),
            system_prompt: None,
            agent_type: Some("default".to_string()),
            tools: vec!["webSearchTool".to_string()],
            directives: Some("Be helpful and accurate".to_string()),
            capabilities: vec!["search".to_string(), "analysis".to_string()],
            memory_enabled: Some(true),
            llm: LLMConfig {
                model: "gpt-4".to_string(),
                temperature: Some(0.7),
                max_tokens: Some(2000),
                top_p: None,
                stop_sequences: None,
                stream: false,
            },
        };

        let prompt = service.generate_system_prompt(&agent_config, true);
        
        assert!(prompt.contains("TestAgent"));
        assert!(prompt.contains("webSearchTool"));
        assert!(prompt.contains("Be helpful and accurate"));
        assert!(prompt.contains("search"));
        assert!(prompt.contains("MEMORY"));
        assert!(prompt.contains("JSON"));
    }

    #[test]
    fn test_orchestration_prompt() {
        let service = SystemPromptService::new();
        let agent_config = AgentConfig {
            name: "OrchestratorAgent".to_string(),
            system_prompt: None,
            agent_type: Some("default".to_string()),
            tools: vec!["tool1".to_string(), "tool2".to_string()],
            directives: None,
            capabilities: vec![],
            memory_enabled: None,
            llm: LLMConfig {
                model: "gpt-4".to_string(),
                temperature: Some(0.7),
                max_tokens: Some(2000),
                top_p: None,
                stop_sequences: None,
                stream: false,
            },
        };

        let prompt = service.generate_orchestration_prompt("Complex multi-step task", &agent_config);
        
        assert!(prompt.contains("ORCHESTRATION"));
        assert!(prompt.contains("Complex multi-step task"));
        assert!(prompt.contains("FIRST tool"));
    }

    #[test]
    fn test_reflection_prompt() {
        let service = SystemPromptService::new();
        
        let prompt = service.generate_reflection_prompt(
            "Search for information",
            "Found 5 relevant results",
            true
        );
        
        assert!(prompt.contains("REFLECTION"));
        assert!(prompt.contains("Search for information"));
        assert!(prompt.contains("Found 5 relevant results"));
        assert!(prompt.contains("SUCCESS: Yes"));
    }
} 