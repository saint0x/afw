# Symphony SDK: Core Usage Examples

This document provides practical examples demonstrating core functionalities of the Symphony SDK, including the creation and usage of Tools, Agents, Teams, and Pipelines. These examples aim to be comprehensive yet succinct, showcasing full syntax and common patterns.

## 1. Tools

Tools are the fundamental building blocks, representing specific actions or capabilities that can be invoked.

### Defining and Registering a Custom Tool

This example shows how to define a custom tool with an input schema, a handler function for its logic, and then register it with the `ToolRegistry`.

```typescript
import { ToolConfig, ToolResult, ToolRegistry } from 'symphonic'; // Assuming package name is 'symphonic'

// Define the configuration for a custom tool
const customEmailTool: ToolConfig = {
  name: 'sendTransactionalEmail',
  description: 'Sends a transactional email (e.g., welcome email, password reset) to a user.',
  type: 'communication', // Arbitrary type for categorization
  
  // Optional: NLP hint for when this tool might be suggested by an intelligent agent
  nlp: 'send an email to a user with subject and body',
  
  // Configuration specific to this tool, including its input schema
  config: {
    // inputSchema defines the expected parameters for this tool using JSON Schema
    inputSchema: {
      type: 'object',
      properties: {
        to: { type: 'string', description: "The recipient's email address.", format: 'email' },
        subject: { type: 'string', description: "The subject line of the email." },
        body: { type: 'string', description: "The HTML or plain text body of the email." },
        templateId: { type: 'string', description: "Optional ID of a pre-defined email template to use." },
        templateVariables: { type: 'object', additionalProperties: true, description: "Key-value pairs for template personalization." }
      },
      required: ['to', 'subject', 'body'] // 'to', 'subject', and 'body' are mandatory
    }
    // Other tool-specific configurations can be added here, e.g.:
    // defaultSender: 'noreply@example.com',
    // mailProviderConfig: { ... }
  },

  // The handler function contains the core logic of the tool
  handler: async (params: {
    to: string;
    subject: string;
    body: string;
    templateId?: string;
    templateVariables?: Record<string, any>;
  }): Promise<ToolResult<{ messageId: string; deliveryStatus: string }>> => {
    const startTime = Date.now();
    console.log(`[sendTransactionalEmail] Attempting to send email to: ${params.to} with subject: ${params.subject}`);

    // --- Simulating email sending logic ---
    try {
      if (!params.to.includes('@')) {
        throw new Error('Invalid recipient email address.');
      }
      // Simulate API call to an email service
      await new Promise(resolve => setTimeout(resolve, 500)); // Simulate network delay
      
      const messageId = `msg_${Date.now()}`;
      const deliveryStatus = 'sent';
      console.log(`[sendTransactionalEmail] Email sent successfully. Message ID: ${messageId}`);

      return {
        success: true,
        result: {
          messageId,
          deliveryStatus,
        },
        metrics: { duration: Date.now() - startTime, startTime, endTime: Date.now() }
      };
    } catch (error: any) {
      console.error(`[sendTransactionalEmail] Failed to send email: ${error.message}`);
      return {
        success: false,
        error: error.message,
        metrics: { duration: Date.now() - startTime, startTime, endTime: Date.now() }
      };
    }
  },
  
  // Optional: Top-level configuration fields from ToolConfig
  timeout: 10000, // Timeout for the tool execution in milliseconds (e.g., 10 seconds)
  retryCount: 2,  // Number of retries on failure (if retry logic is implemented by executor or wrapper)
  // inputs: ['to', 'subject', 'body'], // Optional: simple list of input param names (less descriptive than inputSchema)
  // outputs: ['messageId', 'deliveryStatus'], // Optional: simple list of output field names
  // capabilities: ['email_sending', 'transactional_communication'], // Optional: tags for tool capabilities
};

// Register the tool with the global ToolRegistry
const toolRegistry = ToolRegistry.getInstance();
toolRegistry.registerTool(customEmailTool.name, customEmailTool);

console.log(`Tool '${customEmailTool.name}' registered successfully.`);

// Example of directly executing the registered tool (for testing or direct use)
async function testEmailTool() {
  console.log('\n--- Testing customEmailTool ---');
  const result = await toolRegistry.executeTool(customEmailTool.name, {
    to: 'testuser@example.com',
    subject: 'Welcome to Symphony!',
    body: '<h1>Hello!</h1><p>Thanks for joining our platform.</p>',
    templateVariables: { userName: 'TestUser' }
  });

  if (result.success) {
    console.log('Direct tool execution successful:', result.result);
  } else {
    console.error('Direct tool execution failed:', result.error);
  }
  console.log('Execution metrics:', result.metrics);
}

// testEmailTool(); // Uncomment to run the test

## 2. Agents

Agents are intelligent entities that use LLMs to reason about tasks and utilize tools to achieve goals. When an agent is configured with tools, Symphony SDK automatically enables a specialized JSON mode for LLM interactions, ensuring reliable, structured communication for tool usage.

### Defining and Using an Agent with Custom Tools

This example defines an agent that can use the `sendTransactionalEmail` tool created previously. It demonstrates how the SDK handles JSON mode for tool invocation implicitly.

```typescript
import { AgentConfig, AgentExecutor, ToolRegistry } from 'symphonic'; // Assuming AgentExecutor is the primary way to run agents if not using symphony.agent.create
// If using a high-level symphony client: import { Symphony } from 'symphonic';

// Ensure the customEmailTool is registered (as shown in the Tools section)
// const toolRegistry = ToolRegistry.getInstance();
// if (!toolRegistry.getToolInfo('sendTransactionalEmail')) {
//   toolRegistry.registerTool(customEmailTool.name, customEmailTool);
//   console.log("customEmailTool re-registered for agent example if not already present.");
// }

// Define the configuration for an agent that can send emails
const emailAgentConfig: AgentConfig = {
  name: 'NotificationAgent',
  description: 'An agent responsible for sending various user notifications via email.',
  task: 'Send transactional and notification emails to users based on system events or direct requests.', // General task/purpose
  
  tools: ['sendTransactionalEmail'], // The agent is equipped with our custom email tool
  
  llm: {
    provider: 'openai', // Or your configured default provider
    model: 'gpt-3.5-turbo',    // Or any other capable model
    temperature: 0.2,      // Lower temperature for more predictable tool usage
    maxTokens: 1500,
    // No 'useFunctionCalling' flag needed; JSON mode is automatic for agents with tools.
    // The SDK will append robust JSON instructions to the system prompt.
  },
  
  // Optional: A more specific system prompt for this agent's role
  systemPrompt: "You are the Notification Agent. Your job is to send emails using the 'sendTransactionalEmail' tool. When asked to send an email, carefully prepare the parameters (to, subject, body) for the tool based on the request. If essential information is missing, you can ask for clarification, but prefer to act if enough is provided.",
  
  maxCalls: 3, // Maximum LLM calls for a single task execution by this agent
  // timeout: 60000, // Optional: Overall timeout for the agent's task execution
};

// Instantiate the agent executor (or use symphony.agent.create if available)
// For this example, we assume direct use of AgentExecutor if symphony.agent.create isn't shown
// or if we want to highlight the core execution.
const emailAgent = new AgentExecutor(emailAgentConfig);

console.log(`Agent '${emailAgent.name}' configured successfully.`);

// Example of executing a task with the agent
async function testEmailAgent() {
  console.log('\n--- Testing NotificationAgent ---');
  const taskDescription = "Send a welcome email to new_user@example.com with the subject 'Welcome Aboard!' and a body that says 'Hi there, thanks for signing up!'.";
  
  const result = await emailAgent.executeTask(taskDescription);

  console.log('Agent execution finished.');
  if (result.success) {
    console.log('Agent task reported success.');
    console.log('  Agent Response/Final Output:', result.result?.response);
    if (result.result?.toolsExecuted && result.result.toolsExecuted.length > 0) {
      console.log('  Tools Executed:');
      result.result.toolsExecuted.forEach(toolExec => {
        console.log(`    - Tool: ${toolExec.name}, Success: ${toolExec.success}`);
        console.log(`      Result/Error:`, toolExec.result || toolExec.error);
      });
    } else {
      console.log('  No tools were executed, direct LLM response given (or it decided no tool was needed).');
    }
  } else {
    console.error('Agent task failed:', result.error);
    console.log('  Agent Response/Final Output (on failure):', result.result?.response);
  }
  // console.log('Full Agent Metrics:', result.metrics); // Contains token usage, duration etc.
}

// testEmailAgent(); // Uncomment to run the test

## 3. Teams

Teams enable multiple specialized agents to collaborate on larger, more complex tasks. You can define team structures, member agents (with their individual configurations), and coordination strategies.

### Defining and Using a Team of Agents

This example creates a customer support team that includes our `NotificationAgent` (for sending emails) and another specialized agent for handling queries.

```typescript
import { TeamConfig, AgentConfig, AgentExecutor, ToolRegistry } from 'symphonic'; // Assuming TeamConfig and necessary agent types
// If using a high-level symphony client: import { Symphony } from 'symphonic';

// Assume customEmailTool and emailAgentConfig (for NotificationAgent) are defined and registered as in previous sections.
// For this example, we might need to re-register the tool if this were a standalone script.
// const toolRegistry = ToolRegistry.getInstance();
// if (!toolRegistry.getToolInfo('sendTransactionalEmail')) { 
//   toolRegistry.registerTool(customEmailTool.name, customEmailTool); 
// }

// Define a simple query-handling agent for the team
const querySupportAgentConfig: AgentConfig = {
  name: 'QuerySupportAgent',
  description: 'Handles user queries by providing information or escalating to other specialists.',
  task: 'Understand user queries, provide answers from a knowledge base, or identify when to delegate for specific actions like sending an email.',
  tools: [], // This agent might not use tools directly, or could have a knowledge base access tool
  llm: {
    provider: 'openai',
    model: 'gpt-3.5-turbo',
    temperature: 0.3,
    // No useFunctionCalling here; if it had tools, JSON mode would be auto.
  },
  systemPrompt: "You are a helpful Query Support Agent. Answer user questions clearly. If a user needs an email sent, indicate that the NotificationAgent should handle it.",
  maxCalls: 2,
};

// Define the Team Configuration
const customerSupportTeamConfig: TeamConfig = {
  name: 'CustomerSupportTeam',
  description: 'A team to handle customer inquiries and notifications.',
  // Agents can be specified by their full AgentConfig objects or by registered name/ID (if supported)
  agents: [
    emailAgentConfig,    // Using the config for our NotificationAgent defined earlier
    querySupportAgentConfig // Adding the new query support agent
  ],
  // Optional: Define a manager agent for the team or specific delegation rules
  // manager: 'SupportTeamLeadAgent', // Name or config of a manager agent
  /*
  delegationStrategy: {
    type: 'llm_driven', // Example: LLM decides which agent gets a sub-task
    // Or rule-based:
    // type: 'rule_based',
    // rules: [
    //   { condition: "task.description.includes('send email')", assignTo: ['NotificationAgent'] },
    //   { condition: "task.description.includes('question')", assignTo: ['QuerySupportAgent'] }
    // ]
  },
  */
  strategy: {
    name: 'coordinated_delegation', // Example strategy name
    description: 'A manager agent (or team coordinator logic) delegates tasks to appropriate members.',
    coordinationRules: {
      maxParallelTasks: 1, // For this simple team, let's assume sequential task handling by default
      taskTimeout: 180000, // Timeout for tasks handled by the team
      // Could include rules for communication, shared context updates, etc.
    }
  },
  // capabilities: ['customer_support', 'email_notifications', 'query_resolution'],
};

// If using a high-level client:
// const supportTeam = await symphony.team.create(customerSupportTeamConfig);

// For this example, we might simulate how a TeamCoordinator (if that's a class) would use this config.
// Direct execution of a TeamConfig usually involves a TeamCoordinator or similar orchestrator.
// The following is a conceptual representation of how team execution might be invoked.

console.log(`Team '${customerSupportTeamConfig.name}' configured.`);

async function testSupportTeam() {
  console.log('\n--- Testing CustomerSupportTeam (Conceptual Execution) ---');
  // Simulating a task coming to the team
  const teamTask = "A new user, new_signup@example.com, just signed up. Send them a welcome email with the subject 'Welcome to Our Service!' and a nice welcome message.";
  
  // In a real scenario, a TeamCoordinator would manage this.
  // It would parse the teamTask, potentially use an LLM to decide which agent(s) to activate,
  // and then dispatch sub-tasks to those agents (e.g., to NotificationAgent).
  
  // Let's simulate the orchestrator dispatching to NotificationAgent based on the task.
  // We already have `emailAgent` (an AgentExecutor instance for NotificationAgent)
  // const emailAgent = new AgentExecutor(emailAgentConfig); // (defined in Agent section example)

  console.log(`Team Task: ${teamTask}`);
  console.log('Simulating team coordinator dispatching task to NotificationAgent...');
  
  // We need to ensure emailAgent is available or re-instantiate it for this scope if needed.
  // For this example, we'll assume it's accessible or we create it.
  let localEmailAgentExecutor;
  try {
    // Check if customEmailTool is in the global registry first for the agent
    const registry = ToolRegistry.getInstance();
    if(!registry.getToolInfo('sendTransactionalEmail')) {
        // If not, an error would typically be thrown or handled during agent creation.
        // For this example, we'd need `customEmailTool` definition accessible here to register it.
        console.warn("sendTransactionalEmail tool not found in registry for testSupportTeam. Agent might fail if it tries to use it.");
    }
    localEmailAgentExecutor = new AgentExecutor(emailAgentConfig); 
  } catch (e) {
      console.error("Could not instantiate emailAgentExecutor for test: ", e);
      return;
  }

  // The task for the agent is derived from the team task.
  const agentSpecificTask = "Send a welcome email to new_signup@example.com with subject 'Welcome to Our Service!' and body 'Welcome aboard, we are glad to have you!'.";
  const result = await localEmailAgentExecutor.executeTask(agentSpecificTask);

  console.log('NotificationAgent execution (simulated via team dispatch) finished.');
  if (result.success) {
    console.log('  Agent task (within team context) reported success.');
    console.log('    Agent Response:', result.result?.response);
    if (result.result?.toolsExecuted && result.result.toolsExecuted.length > 0) {
      console.log('    Tool Executed by Agent:', result.result.toolsExecuted[0].name);
    }
  } else {
    console.error('  Agent task (within team context) failed:', result.error);
  }
}

// testSupportTeam(); // Uncomment to run the test
```

Key aspects demonstrated:
- Defining a `TeamConfig` with `name`, `description`, and an `agents` array.
- Agents within a team can be specified using their `AgentConfig` objects.
- Highlighting that agents in teams also benefit from automatic JSON mode if they have tools and appropriate `llm` settings (though `useFunctionCalling` is no longer needed).
- Showing conceptual `delegationStrategy` and `strategy` (with `coordinationRules`) to illustrate team management features.
- The example conceptually outlines how a team task might be processed, leading to the invocation of a specific agent (like our `NotificationAgent`) to handle a part of the task.
- Direct execution of a full `TeamConfig` usually requires a `TeamCoordinator` or a similar orchestrator provided by the SDK (e.g., via `symphony.team.run()`). This example simulates a part of that process.

## 4. Pipelines

Pipelines orchestrate sequences of operations, involving tools, agents, or even other pipelines, to accomplish complex workflows. They support parameterization, data flow between steps, and error handling.

### Defining and Using a Pipeline

This example defines a pipeline for processing a user support request. It first uses an agent to understand the request, then conditionally uses our `NotificationAgent` (via its `AgentExecutor`) if an email needs to be sent.

```typescript
import { PipelineConfig, PipelineStep, AgentExecutor, ToolRegistry } from 'symphonic'; // Assuming necessary types
// If using a high-level symphony client: import { Symphony } from 'symphonic';

// Assume emailAgentConfig and querySupportAgentConfig are defined as in previous sections.
// Also, customEmailTool should be registered if NotificationAgent is to succeed.

// Define the Pipeline Configuration
const supportRequestPipelineConfig: PipelineConfig = {
  name: 'UserSupportRequestPipeline',
  description: 'Handles incoming user support requests, categorizes them, and takes appropriate action (e.g., sending an email via an agent).',
  
  // Define global variables for the pipeline, can be overridden at runtime
  variables: {
    userInputQuery: 'I need to reset my password, please help!',
    userEmail: 'user_to_help@example.com',
    defaultSubject: 'Regarding your Support Request'
  },
  
  steps: [
    {
      id: 'step1_understand_request',
      name: 'Understand User Request',
      type: 'agent', // This step uses an agent
      agent: 'QuerySupportAgent', // Name/ID of the QuerySupportAgent (assuming its config is known/registered)
      inputs: {
        // The task for the QuerySupportAgent is constructed using a pipeline variable
        task_description: `A user submitted the following query: '$userInputQuery'. Determine if an email needs to be sent. If so, state 'NEEDS_EMAIL' in your response, otherwise explain the resolution.`
      },
      outputs: {
        // We map the agent's direct response to a pipeline context variable
        agent_analysis: '.result.response' 
      }
    },
    {
      id: 'step2_conditional_email',
      name: 'Conditionally Send Email',
      type: 'tool', // Using a "custom tool" here to simulate conditional agent execution
                     // In a more advanced system, this could be a dedicated 'condition' step type
                     // or the pipeline executor might have richer conditional logic.
      tool: 'conditionalAgentRunnerTool', // A HYPOTHETICAL tool that runs an agent based on input
      dependencies: ['step1_understand_request'],
      inputs: {
        condition: '@step1_understand_request.agent_analysis', // Output from the previous agent step
        matchKeyword: 'NEEDS_EMAIL',
        agentToRun: 'NotificationAgent', // Name/ID of the NotificationAgent
        agentTask: `Send a password reset instruction email to '$userEmail' with subject 'Password Reset Request'. Body should include a reset link (placeholder: [RESET_LINK]). Query was: '$userInputQuery'. Analysis: @step1_understand_request.agent_analysis`,
        // Pass necessary parameters for the NotificationAgent if it were run
        emailParams: {
            to: '$userEmail',
            subject: 'Password Reset Request',
            body: `Hi there,\n\nPlease use this link to reset your password: [RESET_LINK]\n\nOriginal query: '$userInputQuery'\nSupport analysis: @step1_understand_request.agent_analysis`
        }
      },
      outputs: {
        email_dispatch_result: '.result' // Result of the conditional agent run
      }
    },
    {
      id: 'step3_log_completion',
      name: 'Log Pipeline Completion',
      type: 'tool', // Example: using a simple logging tool (conceptual)
      tool: 'pipelineLoggerTool', // A HYPOTHETICAL logging tool
      dependencies: ['step2_conditional_email'],
      inputs: {
        message: `Pipeline '${supportRequestPipelineConfig.name}' completed. Email dispatch result: @step2_conditional_email.email_dispatch_result`,
        finalStatus: '.success' // Accessing success status of the entire pipeline (conceptual)
      }
    }
  ],
  
  // Optional: Define global error handling for the pipeline
  errorStrategy: {
    type: 'stop', // 'stop', 'continue', 'retry' for pipeline-level errors
    // maxAttempts: 1
  }
};

// --- Mocking for the HYPOTHETICAL tools used in the pipeline ---
const toolRegistry = ToolRegistry.getInstance();

if (!toolRegistry.getToolInfo('conditionalAgentRunnerTool')) {
  const conditionalAgentRunnerTool: ToolConfig = {
    name: 'conditionalAgentRunnerTool',
    description: 'Runs a specified agent with a given task IF a condition is met.',
    type: 'utility',
    config: { inputSchema: { /* ... define schema ... */ } },
    handler: async (params: any): Promise<ToolResult<any>> => {
      console.log(`[conditionalAgentRunnerTool] Condition: ${params.condition}, Match: ${params.matchKeyword}`);
      if (params.condition && params.condition.includes(params.matchKeyword)) {
        console.log(`[conditionalAgentRunnerTool] Condition MET. Running agent: ${params.agentToRun} with task: ${params.agentTask}`);
        // In a real implementation, this would instantiate and run the specified agent.
        // For this example, we'll simulate running the NotificationAgent if its config is available.
        if (params.agentToRun === 'NotificationAgent') {
          // We'd need emailAgentConfig to be in scope here.
          // const agentToRunExecutor = new AgentExecutor(emailAgentConfig); 
          // const agentResult = await agentToRunExecutor.executeTask(params.agentTask);
          // return agentResult; // This would be more realistic
          
          // Simplified mock for NotificationAgent execution via the hypothetical tool
          const emailTool = ToolRegistry.getInstance().getToolInfo('sendTransactionalEmail');
          if (emailTool && emailTool.handler) {
            console.log("[conditionalAgentRunnerTool] Directly calling sendTransactionalEmail handler with params:", params.emailParams);
            return await emailTool.handler(params.emailParams);
          } else {
            return { success: false, error: "NotificationAgent or sendTransactionalEmail tool not fully mocked for conditional run." };
          }
        }
        return { success: true, result: `Simulated run of agent ${params.agentToRun}` };
      }
      console.log('[conditionalAgentRunnerTool] Condition NOT MET.');
      return { success: true, result: 'Condition not met, no agent run.' };
    }
  };
  toolRegistry.registerTool(conditionalAgentRunnerTool.name, conditionalAgentRunnerTool);
}

if (!toolRegistry.getToolInfo('pipelineLoggerTool')) {
  const pipelineLoggerTool: ToolConfig = {
    name: 'pipelineLoggerTool',
    description: 'Logs messages from the pipeline.',
    type: 'utility',
    config: { inputSchema: { properties: { message: {type: "string"} } } },
    handler: async (params: { message: string }): Promise<ToolResult<any>> => {
      console.log(`[PIPELINE LOGGER] ${params.message}`);
      return { success: true, result: 'Logged' };
    }
  };
  toolRegistry.registerTool(pipelineLoggerTool.name, pipelineLoggerTool);
}
// -- End Mocking --

console.log(`Pipeline '${supportRequestPipelineConfig.name}' configured.`);

// Execution of a pipeline typically involves a PipelineExecutor
// For this example, we will not run it directly as it requires full setup of agents and tools
// and a PipelineExecutor instance.

// Conceptual execution:
// import { PipelineExecutor } from 'symphonic';
// const pipelineExecutor = new PipelineExecutor(); // Or PipelineExecutor.getInstance();
// async function testSupportPipeline() {
//   console.log('\n--- Testing UserSupportRequestPipeline ---');
//   const result = await pipelineExecutor.run(supportRequestPipelineConfig, {
//     userInputQuery: "I forgot my password and can't log in!",
//     userEmail: "panic_user@example.com"
//   });
//   if (result.success) {
//     console.log('Pipeline executed successfully:', result.result);
//   } else {
//     console.error('Pipeline execution failed:', result.error);
//   }
// }
// testSupportPipeline();

```

Key aspects demonstrated:
- Defining a `PipelineConfig` with `name`, `description`, global `variables`, and `steps`.
- Pipeline `steps` can be of `type: 'agent'` or `type: 'tool'` (or other types like `team`).
  - Agent steps refer to an agent by name/ID; these agents benefit from the SDK's automatic JSON mode for their tool interactions.
  - Tool steps refer to a registered tool by name.
- Using `inputs` to map pipeline variables (`$varName`) or outputs of previous steps (`@stepId.outputPath`) to step inputs.
- Using `outputs` to map a step's result to a variable in the pipeline context for subsequent steps.
- Specifying `dependencies` to control the order of execution.
- Conceptual illustration of a `conditional` step type (actual implementation might involve a specialized tool or executor logic).
- Global `errorStrategy` for the pipeline.
- The example includes mocks for hypothetical tools (`conditionalAgentRunnerTool`, `pipelineLoggerTool`) to make the pipeline definition runnable in a limited test context.
- Actual pipeline execution would typically be done via a `PipelineExecutor` from the SDK.

--- 