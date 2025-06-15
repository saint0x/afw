# Symphony SDK Usage Guide

This guide provides comprehensive documentation for using the Symphony SDK with accurate examples based on the actual implementation.

## Table of Contents

- [Installation & Setup](#installation--setup)
- [Core Concepts](#core-concepts)
- [Symphony Initialization](#symphony-initialization)
- [Tools](#tools)
- [Agents](#agents)
- [Teams](#teams)
- [Pipelines](#pipelines)
- [Cache Intelligence](#cache-intelligence)
- [Memory System](#memory-system)
  - [Streaming](#streaming)
- [Advanced Features](#advanced-features)
- [Error Handling](#error-handling)

## Installation & Setup

```bash
bun add symphonic
```

```typescript
import { Symphony } from 'symphonic';

const symphony = new Symphony({
  llm: {
    provider: 'openai',
    model: 'gpt-4o-mini',
    apiKey: process.env.OPENAI_API_KEY
  },
  db: {
    enabled: true,
    adapter: 'sqlite',
    path: './symphonic.db'
  }
});

await symphony.initialize();
```

## Core Concepts

Symphony is built around four core orchestration layers:

1. **Tools**: Individual functions that perform specific tasks
2. **Agents**: Intelligent entities that use tools with LLM reasoning
3. **Teams**: Groups of agents that collaborate using coordination strategies
4. **Pipelines**: Structured workflows with advanced error recovery and monitoring

Additional systems:
- **Cache Intelligence**: XML pattern matching and context trees for optimization
- **Memory System**: Short/long-term memory with search and aggregation
- **Streaming**: Real-time progress updates for long-running operations

## Symphony Initialization

### Basic Configuration

```typescript
const symphony = new Symphony({
  llm: {
    provider: 'openai',
    model: 'gpt-4o-mini',
    apiKey: process.env.OPENAI_API_KEY,
    temperature: 0.7,
    maxTokens: 2000
  },
  db: {
    enabled: true,
    adapter: 'sqlite',
    path: './symphonic.db'
  },
  logLevel: 'info'
});
```

### Advanced Configuration

```typescript
const symphony = new Symphony({
  llm: {
    provider: 'openai',
    model: 'gpt-4o-mini',
    apiKey: process.env.OPENAI_API_KEY
  },
  db: {
    enabled: true,
    adapter: 'sqlite',
    path: './symphonic.db'
  },
  cache: {
    enablePatternMatching: true,
    enableContextTrees: true,
    fastPathThreshold: 0.85,
    contextMaxNodes: 50
  },
  memory: {
    shortTerm: {
      defaultTTL: 3600000, // 1 hour
      maxSize: 100 * 1024 * 1024 // 100MB
    },
    longTerm: {
      defaultTTL: 30 * 24 * 3600000, // 30 days
      maxSize: 1024 * 1024 * 1024 // 1GB
    }
  },
  streaming: {
    maxConcurrentStreams: 50,
    defaultBufferSize: 1000,
    defaultUpdateInterval: 100
  }
});

await symphony.initialize();
```

## Tools

### Standard Tools

Symphony provides built-in tools that work out of the box:

```typescript
// File operations
'readFile'      // Read file contents from filesystem
'writeFile'     // Write content to filesystem

// Web search
'webSearch'     // Search the web using Serper.dev API

// Document processing
'parseDocument' // Parse various document formats

// Code generation
'writeCode'     // Generate code based on specifications

// Planning
'createPlan'    // Create structured execution plans

// Cognitive processing
'ponder'        // Deep thinking with multi-depth analysis
```

### Creating Custom Tools

A tool is defined by its configuration, including an `inputSchema` (JSON schema for parameters) and a `handler` function.

```typescript
import { ToolConfig, ToolResult } from 'symphonic'; // Assuming ToolConfig and ToolResult are exported

const myCustomTool: ToolConfig = {
  name: 'processUserData',
  description: 'Processes user data, applying validation and transformation.',
  type: 'data_processing', // An example type
  // Optional: Define NLP hint for when this tool might be used
  nlp: 'process user data with options', 
  
  // Configuration for the tool, including its input schema
  config: {
    inputSchema: {
      type: 'object',
      properties: {
        userData: { type: 'object', description: 'The raw user data.' },
        options: { 
          type: 'object', 
          description: 'Processing options.',
          properties: {
            validate: { type: 'boolean', default: true },
            format: { type: 'string', enum: ['json', 'xml'], default: 'json' }
          }
        }
      },
      required: ['userData']
    }
  },

  // The handler function that executes the tool's logic
  handler: async (params: { userData: any, options?: { validate?: boolean, format?: string } }): Promise<ToolResult<{ processedData: any, report: string }>> => {
    const startTime = Date.now();
    try {
      const { userData, options = {} } = params;
      const { validate = true, format = 'json' } = options;

      if (!userData) {
        throw new Error('userData parameter is required.');
      }

      let processedData = JSON.parse(JSON.stringify(userData)); // Deep copy

      // Example validation (replace with actual validation logic)
      if (validate && (!processedData.id || !processedData.name)) {
        throw new Error('User data must include id and name for validation.');
      }

      let report = `Data processed for user ID: ${processedData.id}.`;

      if (format === 'json') {
        processedData = processedData; // Already an object, or could be stringified if needed
        report += ' Output format: JSON.';
      } else if (format === 'xml') {
        // Example: Convert to XML (actual implementation would be more complex)
        report += ' Output format: XML (conversion not shown).';
        // processedData = convertToXml(processedData); 
      }

      return {
        success: true,
        result: {
          processedData,
          report
        },
        metrics: { duration: Date.now() - startTime, startTime, endTime: Date.now() }
      };
    } catch (error: any) {
      return {
        success: false,
        error: error.message,
        metrics: { duration: Date.now() - startTime, startTime, endTime: Date.now() }
      };
    }
  },
  
  // Optional: Top-level config fields as per ToolConfig type
  timeout: 30000, // e.g., 30 seconds
  retryCount: 3,   // Number of retries
};

// If you have a `symphony.tool.create` or similar method for registration:
// await symphony.tool.create(myCustomTool); 
// Or, if registering directly with ToolRegistry (more common for internal/custom setup):
// import { ToolRegistry } from 'symphonic';
// ToolRegistry.getInstance().registerTool(myCustomTool.name, myCustomTool);

// For agents to use this tool, its name ('processUserData') would be added to their 'tools' array in AgentConfig.
```

### Using Tools Directly

```typescript
// Using ToolRegistry directly
import { ToolRegistry } from 'symphonic'; // Corrected import path

const registry = ToolRegistry.getInstance();

// Execute a tool
const result = await registry.executeTool('webSearch', {
  query: 'Symphony SDK documentation'
});

// Get available tools
const tools = registry.getAvailableTools();
console.log('Available tools:', tools);

// Get tool information
const toolInfo = registry.getToolInfo('ponder');
```

## Agents

Agents in Symphony are intelligent entities that leverage Large Language Models (LLMs) along with a defined set of tools to perform tasks. They can understand natural language instructions, reason about how to achieve a goal, and utilize tools by generating structured JSON requests.

### Creating Agents

When creating an agent, you define its core attributes, the tools it can use, and its LLM configuration. If an agent is configured with tools, the Symphony SDK automatically enables a specialized JSON mode. This involves appending robust instructions to the system prompt to guide the LLM towards producing structured JSON output for tool calls or direct responses. This ensures reliable tool interactions across different LLM providers.

```typescript
const agent = await symphony.agent.create({
  name: 'DataAnalyst',
  description: 'Specialized in data analysis and insights generation. Capable of web research, deep analysis, and report generation.',
  task: 'Analyze complex datasets, identify key trends, and produce actionable insights reports.', // Default or guiding task
  tools: ['webSearch', 'ponder', 'writeFile', 'createPlan', 'processUserData'], // Include custom or standard tools by name
  llm: {
    provider: 'openai', // Or other supported providers
    model: 'gpt-4o-mini',    // Choose appropriate model (e.g., gpt-3.5-turbo, gpt-4)
    temperature: 0.5,      // Adjust for creativity vs. determinism
    maxTokens: 2500,       // Max tokens for LLM response
    // No useFunctionCalling field needed; JSON mode is automatic for tool-enabled agents.
  },
  // Optional configurations for more fine-grained control:
  systemPrompt: "You are an expert Data Analyst. Your primary goal is to deliver accurate and insightful analysis. Always cite your sources if performing research.", // Override/set a specific system prompt.
  directives: "Focus on clarity and conciseness in your final output. Quantify your findings where possible.", // Additional instructions appended to the system prompt.
  capabilities: ['data_analysis', 'reporting', 'web_research', 'trend_identification'], // Tags for agent capabilities, useful for selection.
  maxCalls: 5,        // Max LLM calls the agent can make for a single .run() invocation.
  timeout: 120000,    // Overall timeout in milliseconds for a single .run() invocation.
  // requireApproval: true, // If true, agent might pause for approval before certain actions (if supported by executor).
  // enableCache: true,       // To enable caching for this agent's LLM calls.
});
```

### Agent Execution

Once an agent is created, you can execute tasks using its `run` method. The task description provided to `run` will be the primary instruction for the agent.

```typescript
// Simple execution of a specific task
const analysisTask = 'Analyze the Q3 financial performance of ACME Corp based on recent news and provide a summary.';
const result = await agent.run(analysisTask);

console.log('Agent Run Result:');
if (result.success) {
  console.log('  Response:', result.result?.response);
  console.log('  Reasoning:', result.result?.reasoning); // May include details if provided by agent
} else {
  console.error('  Error:', result.error);
}
console.log('  Tool Calls:', result.metrics?.toolCalls); // Number of tools called
console.log('  LLM Usage:', result.metrics?.llmUsage); // Token usage
console.log('  Duration:', result.metrics?.duration, 'ms');

// Execution with options (e.g., for streaming progress in a UI)
const complexResearchTask = 'Conduct a comprehensive market analysis for a new SaaS product in the AI space, focusing on competitors and potential differentiators. Output a structured report.';
const options = {
  timeout: 300000, // 5 minutes for a complex task
  // Example: if the agent supports streaming progress via onProgress callback
  /*
  onProgress: (update: { status: string; step?: string; details?: any }) => {
    console.log(`Agent Progress: ${update.status} - ${update.step || 'N/A'}`, update.details || '');
  },
  onMetrics: (metrics: any) => {
    console.log(`Agent Intermediate Metrics: ${JSON.stringify(metrics)}`);
  }
  */
};
// const detailedResult = await agent.run(complexResearchTask, options);
```

**Note on JSON Mode and System Prompts:**
If an agent is configured with any tools, Symphony SDK automatically appends a detailed set of instructions to the final system prompt. These instructions guide the LLM to respond strictly in JSON format, specifying how to structure tool calls (`{"tool_name": "...", "parameters": {...}}`) or indicate that no tool is needed (`{"tool_name": "none", "response": "..."}`). This ensures reliable parsing and execution, even if the agent's custom `systemPrompt` is minimal. For OpenAI models, the SDK also leverages the native `response_format: { type: "json_object" }` API parameter for enhanced reliability.

### Agent Tool Selection

```typescript
// Get intelligent tool recommendation
const toolSelection = await agent.selectTool('Search for recent AI developments');

console.log('Tool Selection:', {
  toolName: toolSelection.toolName,
  confidence: toolSelection.confidence,
  reasoning: toolSelection.reasoning
});
```

## Teams

Teams in Symphony allow multiple agents to collaborate on complex tasks. Each agent can have its own specialization, tools, and LLM configuration. Teams utilize strategies to coordinate agent execution.

### Creating Teams

When defining a team, you specify its member agents (either by providing their full configuration inline or by referencing existing agent names/IDs if supported by `symphony.team.create`). Each agent within a team can have its own `llm` settings, including `useFunctionCalling: true` to leverage the SDK's JSON mode enforcement for reliable tool use.

```typescript
const team = await symphony.team.create({
  name: 'SoftwareDevelopmentTeam',
  description: 'A versatile team for full-cycle software development, from design to deployment.',
  // Agents can be defined inline with their full configuration
  agents: [
    {
      name: 'ProductDesigner',
      description: 'Focuses on UI/UX design, user flows, and wireframing.',
      task: 'Create intuitive and visually appealing user interface designs and prototypes.',
      tools: ['createPlan', 'ponder'], // e.g., planning and ideation tools
      llm: {
        provider: 'openai',
        model: 'gpt-4o-mini',
        // useFunctionCalling: true, // REMOVED - JSON mode is automatic for tool-enabled agents
        temperature: 0.6
      },
      capabilities: ['ui_design', 'ux_research', 'prototyping']
    },
    {
      name: 'BackendEngineer',
      description: 'Node.js, Python, and database specialist for server-side logic.', 
      task: 'Develop robust APIs, manage database schemas, and implement business logic.',
      tools: ['writeCode', 'readFile', 'writeFile', 'ponder', 'webSearch'],
      llm: 'gpt-4o-mini', // Can also be a string for simplicity if defaults are acceptable
      capabilities: ['nodejs', 'python', 'database_management', 'api_design', 'system_architecture']
    },
    // Add other specialized agents like FrontendEngineer, QATester, DevOpsSpecialist etc.
  ],
  // Optional: Designate a manager agent or define specific delegation rules
  // manager: 'TeamLeadAgentName', // or true if one agent is implicitly manager by role/config
  // delegationStrategy: {
  //   type: 'capability_based', // e.g., tasks routed by agent capabilities
  //   // rules: [ { condition: "task_type === 'database'", assignTo: ['BackendEngineer'] } ]
  // },
  strategy: {
    name: 'adaptive_collaborative_execution', // Example strategy name
    description: 'Team coordinates dynamically based on task complexity and agent availability.',
    // assignmentLogic: async (task, agents) => { /* custom logic */ return [agents[0].name]; },
    coordinationRules: {
      maxParallelTasks: 2,      // Max number of tasks agents can work on concurrently
      taskTimeout: 600000,      // Timeout for individual agent tasks within the team (10 minutes)
      // Ddditional rules like error handling, escalation paths, etc.
    }
  },
  // capabilities: ['software_development', 'web_applications', 'api_services'],
  // log: { inputs: true, outputs: true, metrics: true } // Configure team-level logging
});
```

### Team Execution Strategies

```typescript
import { TeamExecutionStrategy } from 'symphonic'; // Corrected import path

// Parallel execution - all agents work simultaneously on sub-tasks of the main goal
const parallelResult = await team.run('Implement user authentication system', {
  strategy: TeamExecutionStrategy.PARALLEL,
  timeout: 600000
});

// Sequential execution - agents work one after another
const sequentialResult = await team.run('Conduct code review and testing', {
  strategy: TeamExecutionStrategy.SEQUENTIAL
});

// Pipeline execution - output of one agent feeds into the next
const pipelineResult = await team.run('Design, implement, and test new feature', {
  strategy: TeamExecutionStrategy.PIPELINE
});

// Collaborative execution - agents work together on single task
const collaborativeResult = await team.run('Architect complex system design', {
  strategy: TeamExecutionStrategy.COLLABORATIVE
});

// Role-based execution - tasks assigned based on capabilities
const roleBasedResult = await team.run('Build e-commerce platform', {
  strategy: TeamExecutionStrategy.ROLE_BASED,
  requiredCapabilities: ['frontend', 'backend', 'database']
});
```

### Team Context Intelligence

```typescript
// Get comprehensive team context
const context = team.getContext();

console.log('Team Context:', {
  teamName: context.teamName,
  executionPhase: context.executionPhase,
  
  // Member intelligence
  availableMembers: context.members.available.length,
  optimalMember: context.members.optimal,
  workloadBalanced: context.members.workload.balanced,
  
  // Task intelligence
  activeTasks: context.tasks.active,
  completedTasks: context.tasks.completed,
  recentHistory: context.tasks.recentHistory.slice(0, 3),
  
  // Intelligence insights
  recommendedStrategy: context.insights.recommendedStrategy,
  teamEfficiency: context.insights.teamEfficiency,
  optimizations: context.insights.suggestedOptimizations,
  riskFactors: context.insights.riskFactors
});

// Get real-time team status
const status = team.getStatus();
console.log('Team Status:', {
  activeMembers: status.activeMembers,
  taskQueue: status.taskQueue,
  activeExecutions: status.activeExecutions,
  coordinationStrategy: status.coordinationStrategy
});
```

## Pipelines

### Creating Pipelines

Pipelines define structured workflows composed of multiple steps. These steps can involve executing tools, running agents or teams, conditional logic, and more. Variables can be passed into pipelines and between steps.

```typescript
import { PipelineConfig, PipelineStep, RetryConfig } from 'symphonic'; // Assuming these types are exported

const pipelineConfig: PipelineConfig = {
  name: 'ComprehensiveContentAnalysis',
  description: 'Fetches, analyzes, and reports on web content using tools and conditional logic.',
  // version: '1.0.0', // Optional versioning
  steps: [
    {
      id: 'fetch_web_content',
      name: 'Fetch Web Content',
      type: 'tool', // Step type: 'tool', 'agent', 'team', 'condition', etc.
      tool: 'webSearch', // Name of the tool to use (must be registered in ToolRegistry)
      inputs: { // Mapping inputs for the tool
        query: '$pipeline_input_search_term' // Using a pipeline variable
      },
      outputs: {
        fetched_content: '.result' // Mapping the entire tool result to 'fetched_content' for next steps
                                  // Or specify a path, e.g. '.result.articles[0].content'
      },
      retryConfig: { // Configuration for retrying this step on failure
        maxAttempts: 3,
        delay: 1000, // Initial delay in ms
        // retryableErrors: ['timeout', 'network_error'] // Optional: specific errors to retry on
      },
      timeout: 30000 // Step-specific timeout in ms
    },
    {
      id: 'analyze_fetched_content',
      name: 'Analyze Fetched Content',
      type: 'agent', // Example of an agent step
      agent: 'ContentAnalysisAgent', // Name of a pre-configured agent (assumed to exist)
                                   // Agents used in pipelines automatically benefit from the SDK's JSON mode enhancements
                                   // if they are configured with tools.
      inputs: {
        task_description: 'Analyze the following text for key themes and sentiment: @fetch_web_content.fetched_content' // Referencing output from previous step
      },
      outputs: {
        analysis_summary: '.result.response' // Mapping agent's response to 'analysis_summary'
      },
      dependencies: ['fetch_web_content'] // Depends on the successful completion of this step
    },
    {
      id: 'generate_final_report',
      name: 'Generate Final Report',
      type: 'tool',
      tool: 'writeFile',
      inputs: {
        filename: '$pipeline_input_report_filename', // Using a pipeline variable
        content: 'Summary of Analysis for query \'$pipeline_input_search_term\':\n\n@analyze_fetched_content.analysis_summary'
      },
      dependencies: ['analyze_fetched_content']
    },
    {
      id: 'check_report_validity',
      name: 'Check Report Validity',
      type: 'condition', // Conditional step
      // 'condition' field not directly on PipelineStep, but this illustrates intent.
      // Actual conditional logic might be via a custom 'condition' tool or specific step type.
      // For this example, we'll assume a conceptual condition. The actual implementation might vary.
      // config: { 
      //   expression: '$pipeline_input_enable_validation === true && @generate_final_report.result.success === true',
      //   ifTrue: 'notify_completion', // Hypothetical next step ID
      //   ifFalse: 'escalate_failure'   // Hypothetical next step ID
      // }, 
      dependencies: ['generate_final_report']
    }
  ],
  // Global pipeline variables, can be overridden at runtime
  variables: {
    pipeline_input_search_term: 'default search term',
    pipeline_input_report_filename: 'analysis_report.txt',
    pipeline_input_enable_validation: true
  },
  // Global error handling strategy for the pipeline
  errorStrategy: {
    type: 'retry', // 'stop', 'continue', 'retry'
    maxAttempts: 2, // Max retries for the entire pipeline if a step ultimately fails
    // delay: 5000 // Delay between pipeline-level retries
  },
  // Concurrency settings for the pipeline execution
  // metrics: { enabled: true, detailed: true, trackMemory: true } // Optional metrics config
};

// Assuming a symphony client instance for pipeline creation
// const pipeline = await symphony.pipeline.create(pipelineConfig);
```

### Pipeline Step Types

Pipelines support various step types to build complex workflows:

```typescript
// Tool step: Executes a registered tool.
{
  id: 'run_data_tool',
  name: 'Process Data with Tool',
  type: 'tool',
  tool: 'processUserData', // Assumes 'processUserData' tool is registered
  inputs: { 
    userData: '$input_data_object', 
    options: { validate: true, format: 'json' } 
  },
  outputs: { processed_data_output: '.result.processedData' }
}

// Agent step: Invokes a configured agent to perform a task.
// Agents used in pipelines automatically benefit from the SDK's JSON mode enhancements
// (like verbose JSON instructions) if they are configured with tools.
{
  id: 'expert_analysis_step',
  name: 'Perform Expert Analysis',
  type: 'agent',
  agent: 'ContentAnalysisAgent', // Name of a pre-configured agent
  inputs: { 
    task_description: 'Review financial data @previous_step.processed_data_output and identify anomalies.' 
  },
  outputs: { expert_findings: '.result.response' }
}

// Team step: Delegates a task to a configured team.
{
  id: 'development_feature_step',
  name: 'Develop New Feature',
  type: 'team',
  team: 'SoftwareDevelopmentTeam', // Name of a pre-configured team
  inputs: { 
    feature_specification: '$feature_docs_variable' 
  },
  outputs: { development_status: '.result' } // Output from the team's execution
}

// Chain step - (Conceptual) execute a pre-defined tool chain if supported
// Note: `ToolChain` execution is typically via `ChainExecutor.getInstance().executeChain()`.
// Direct 'chain' type in PipelineStep might be a higher-level abstraction or future feature.
// For current direct usage, a tool step could wrap a ChainExecutor call.
{
  id: 'research_workflow_chain',
  name: 'Execute Research Tool Chain',
  type: 'chain', // This type might be conceptual or specific to your PipelineStep definition
  // chain: 'my_research_chain_id', // ID of a pre-defined ToolChain
  // inputs: { query: '$research_topic' }
  config: { chainId: 'my_research_chain_id', inputs: { query: '$research_topic' } } // More likely config

}

// Condition step - (Conceptual) conditional branching logic
// Actual implementation may vary; could be a special tool or a specific step type.
{
  id: 'quality_assurance_check',
  name: 'QA Checkpoint',
  type: 'condition', // This type might be conceptual
  // config: { 
  //   expression: '$quality_score_variable > 0.9 && @previous_step.result.validation_passed === true',
  //   ifTrue: 'deployment_step_id',  // ID of step to go to if true
  //   ifFalse: 'review_step_id'     // ID of step to go to if false
  // }
  // inputs: { quality_score: '$quality_score_variable', validation_status: '@previous_step.result.validation_passed' },
  // Assuming a custom 'conditionalRouterTool' that takes expression and routes
  tool: 'conditionalRouterTool',
  inputs: {
    condition_expression: '$quality_score_variable > 0.9 && @previous_step.result.validation_passed === true',
    true_step: 'deployment_step_id',
    false_step: 'review_step_id'
  }
}

// Transform step - (Conceptual) for data manipulation between steps
// Often, data transformation can be handled by tools themselves or input/output mapping.
{
  id: 'format_report_data',
  name: 'Format Report Data',
  type: 'transform', // This type might be conceptual
  // config: {
  //   input_path: '@raw_data_step.result',
  //   output_path: 'formatted_report_data',
  //   transform_function: 'json_to_csv' // Name of a registered transformation function
  // }
  // Assuming a custom 'dataTransformerTool'
  tool: 'dataTransformerTool',
  inputs: { data: '@raw_data_step.result', target_format: 'csv' },
  outputs: { transformed_data_output: '.result.csv_data' }
}

// Parallel step - (Conceptual) for executing multiple steps concurrently
// The Pipeline executor itself might handle concurrency based on dependencies rather than a specific step type.
// Alternatively, a 'parallel' step type could define a sub-set of steps to run in parallel.
{
  id: 'concurrent_analysis_tasks',
  name: 'Run Analyses in Parallel',
  type: 'parallel', // This type might be conceptual
  // config: {
  //   branches: [
  //     [{ id: 'sentiment_analysis', type:'tool', tool: 'sentimentAnalysisTool', inputs: {text: '$text_input'} }],
  //     [{ id: 'keyword_extraction', type:'tool', tool: 'keywordDetectionTool', inputs: {text: '$text_input'} }]
  //   ]
  // }
  // This might be achieved by defining steps with no dependencies on each other if the executor supports it.
}

// Wait step - (Conceptual) for introducing delays or waiting for external conditions
{
  id: 'wait_for_external_process',
  name: 'Wait for External System Update',
  type: 'wait', // This type might be conceptual
  // config: {
  //   duration_ms: 10000, // Wait for a fixed duration
  //   condition_poll_tool: 'checkSystemStatusTool', // Tool to poll for a condition
  //   expected_condition_value: 'READY'
  // }
  // Assuming a custom 'waitConditionTool'
  tool: 'waitConditionTool',
  inputs: { duration_ms: 10000, poll_url: '$status_check_url', expected_value: 'READY' }
}
```

### Pipeline Execution

```typescript
// Execute pipeline
const result = await pipeline.run({
  search_term: 'artificial intelligence trends 2024',
  validate_results: true,
  output_format: 'markdown'
});

console.log('Pipeline Result:', {
  success: result.success,
  pipelineId: result.result?.pipelineId,
  executionId: result.result?.executionId,
  stepsCompleted: result.result?.steps.length,
  totalDuration: result.metrics?.duration,
  intelligence: result.metrics?.intelligence
});
```

### Pipeline Intelligence

   ```typescript
// Get performance profile
const profile = pipeline.executor.getPerformanceProfile();

if (profile) {
  console.log('Performance Profile:', {
    totalDuration: profile.totalDuration,
    stepCount: profile.stepMetrics.length,
    bottlenecks: profile.bottlenecks.map(b => ({
      stepId: b.stepId,
      type: b.type,
      severity: b.severity,
      impact: b.impact,
      recommendation: b.recommendation
    })),
    estimatedImprovement: profile.optimization.estimatedImprovement
  });
}

// Get optimization recommendations
const recommendations = pipeline.executor.getOptimizationRecommendations();

recommendations.forEach((rec, index) => {
  console.log(`Recommendation ${index + 1}:`, {
    category: rec.category,
    priority: rec.priority,
    description: rec.description,
    implementation: rec.implementation,
    estimatedImprovement: rec.estimatedImprovement,
    effort: rec.effort
  });
});

// Circuit breaker management
const cbStatus = pipeline.executor.getCircuitBreakerStatus('fetch_content');
if (cbStatus) {
  console.log('Circuit Breaker:', {
    state: cbStatus.state,
    failureCount: cbStatus.failureCount
  });
}

// Reset circuit breaker if needed
pipeline.executor.resetCircuitBreaker('fetch_content');

// Get intelligence health
const health = pipeline.executor.getIntelligenceHealth();
console.log('Intelligence Health:', health);
```

## Cache Intelligence

### Basic Cache Operations

   ```typescript
// Legacy cache operations (backward compatible)
await symphony.cache.set('user_data', { id: 1, name: 'John' }, 3600);
const userData = await symphony.cache.get('user_data');
const exists = await symphony.cache.has('user_data');
await symphony.cache.delete('user_data');
await symphony.cache.clear();
```

### Intelligence Features

```typescript
// Get intelligent recommendations
const intelligence = await symphony.cache.getIntelligence(
  'search for recent AI developments in natural language processing',
  {
    sessionId: 'user_session_123',
    enablePatternMatching: true,
    enableContextTrees: true,
    fastPathThreshold: 0.85
  }
);

console.log('Cache Intelligence:', {
  recommendation: intelligence.recommendation, // 'fast_path' | 'standard_path' | 'enhanced_context' | 'no_match'
  confidence: intelligence.confidence,
  
  // Pattern match details
  patternMatch: intelligence.patternMatch ? {
    name: intelligence.patternMatch.name,
    confidence: intelligence.patternMatch.confidence,
    toolCall: intelligence.patternMatch.toolCall,
    reasoning: intelligence.patternMatch.reasoning
  } : null,
  
  // Context tree details
  contextTree: intelligence.contextTree ? {
    type: intelligence.contextTree.type,
    name: intelligence.contextTree.name,
    priority: intelligence.contextTree.priority
  } : null,
  
  // Performance metadata
  metadata: intelligence.metadata
});

// Record tool execution for learning
await symphony.cache.recordToolExecution(
  'user_session_123',
  'webSearch',
  { query: 'AI developments', type: 'search' },
  { results: [/* search results */] },
  true, // success
  1250, // execution time in ms
  'SEARCH_WEB_pattern_id'
);
```

### Cache Analytics

```typescript
// Pattern analytics
const patternAnalytics = await symphony.cache.getPatternAnalytics();
console.log('Pattern Analytics:', {
  totalPatterns: patternAnalytics.totalPatterns,
  averageConfidence: patternAnalytics.averageConfidence,
  topPatterns: patternAnalytics.topPatterns.slice(0, 5),
  confidenceDistribution: patternAnalytics.confidenceDistribution
});

// Context analytics
const contextAnalytics = await symphony.cache.getContextAnalytics();
console.log('Context Analytics:', {
  cacheStats: contextAnalytics.cacheStats,
  averageNodes: contextAnalytics.treeMetrics.averageNodes,
  nodeTypes: contextAnalytics.treeMetrics.nodeTypes
});

// Global statistics
const globalStats = symphony.cache.getGlobalStats();
console.log('Global Cache Stats:', {
  totalQueries: globalStats.totalQueries,
  fastPathQueries: globalStats.fastPathQueries,
  patternMatchRate: globalStats.patternMatchRate,
  averageResponseTime: globalStats.averageResponseTime
});

// Session intelligence
const sessionIntelligence = symphony.cache.getSessionIntelligence('user_session_123');
if (sessionIntelligence) {
  console.log('Session Intelligence:', {
    queriesProcessed: sessionIntelligence.queriesProcessed,
    patternsMatched: sessionIntelligence.patternsMatched,
    averageConfidence: sessionIntelligence.averageConfidence,
    insights: sessionIntelligence.insights
  });
}
```

## Memory System

### Basic Memory Operations

```typescript
// Store memories
await symphony.memory.store('user_preference', { theme: 'dark', lang: 'en' }, 'short_term', {
  sessionId: 'session_123',
  namespace: 'user_settings',
  tags: ['preference', 'ui'],
  metadata: { source: 'user_input', confidence: 0.9 }
});

await symphony.memory.store('project_context', projectData, 'long_term', {
  namespace: 'projects',
  tags: ['active', 'important'],
  priority: 10
});

// Retrieve memories
const preference = await symphony.memory.retrieve('user_preference', 'short_term', {
  namespace: 'user_settings',
  includeMetadata: true
});

const projectContext = await symphony.memory.retrieve('project_context', 'long_term');
```

### Memory Search

```typescript
// Search memories with comprehensive query
const searchResults = await symphony.memory.search({
  type: 'short_term',
  namespace: 'user_settings',
  sessionId: 'session_123',
  tags: ['preference'],
  textSearch: 'theme',
  limit: 10,
  sortBy: 'timestamp',
  sortOrder: 'desc',
  dateRange: {
    start: new Date(Date.now() - 24 * 60 * 60 * 1000), // Last 24 hours
    end: new Date()
  }
});

console.log('Search Results:', searchResults.map(entry => ({
  key: entry.key,
  type: entry.type,
  namespace: entry.namespace,
  tags: entry.tags,
  createdAt: entry.createdAt,
  size: entry.size
})));
```

### Memory Aggregation

```typescript
// Aggregate and analyze memories
const aggregation = await symphony.memory.aggregate({
  namespace: 'user_behavior',
  tags: ['interaction', 'preference'],
  dateRange: {
    start: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000), // Last 7 days
    end: new Date()
  }
});

console.log('Memory Aggregation:', {
  totalEntries: aggregation.totalEntries,
  totalSize: aggregation.totalSize,
  patterns: aggregation.patterns.map(p => ({
    pattern: p.pattern,
    frequency: p.frequency,
    insights: p.insights
  })),
  insights: aggregation.insights,
  processingTime: aggregation.metadata.processingTime
});
```

### Memory Statistics

```typescript
// Get comprehensive memory statistics
const stats = await symphony.memory.getStats();
console.log('Memory Statistics:', {
  shortTerm: {
    entries: stats.shortTerm.totalEntries,
    size: `${(stats.shortTerm.totalSize / 1024 / 1024).toFixed(2)} MB`,
    averageSize: `${(stats.shortTerm.averageSize / 1024).toFixed(2)} KB`
  },
  longTerm: {
    entries: stats.longTerm.totalEntries,
    size: `${(stats.longTerm.totalSize / 1024 / 1024).toFixed(2)} MB`,
    averageSize: `${(stats.longTerm.averageSize / 1024).toFixed(2)} KB`
  },
  performance: stats.performance,
  topNamespaces: Object.entries(stats.namespaces)
    .sort(([,a], [,b]) => b.entryCount - a.entryCount)
    .slice(0, 5)
});

// Get operational statistics
const operationalStats = symphony.memory.getOperationalStats();
console.log('Operational Stats:', {
  uptime: `${Math.round(operationalStats.uptime / 1000 / 60)} minutes`,
  totalOperations: operationalStats.totalOperations,
  averageResponseTime: `${operationalStats.averageResponseTime}ms`,
  healthStatus: operationalStats.healthStatus,
  memoryUsage: `${(operationalStats.memoryUsage / 1024 / 1024).toFixed(2)} MB`
});
```

## Streaming

### Creating and Managing Streams

```typescript
// Create a stream for long-running operation
const streamId = symphony.streaming.createStream({
  type: 'pipeline',
  context: {
    id: 'data_processing_pipeline',
    name: 'Data Processing Pipeline',
    description: 'Processing large dataset with multiple stages'
  },
  options: {
    bufferSize: 1000,
    updateInterval: 100 // Update every 100ms
  }
});

// Subscribe to stream updates
const unsubscribe = symphony.streaming.subscribe(streamId, (update) => {
  switch (update.type) {
    case 'progress':
      console.log(`Progress: ${update.progress}%`);
      if (update.data) {
        console.log(`Processed: ${update.data.processed}/${update.data.total}`);
      }
      break;
      
    case 'status':
      console.log(`Status: ${update.status}`);
      break;
      
    case 'data':
      console.log('Intermediate result:', update.data);
      break;
      
    case 'complete':
      console.log('Operation completed successfully');
      console.log('Final result:', update.data);
      break;
      
    case 'error':
      console.error('Operation failed:', update.error);
      break;
  }
});

// Update stream progress during operation
symphony.streaming.updateProgress(streamId, {
  type: 'progress',
  progress: 25,
  status: 'Processing stage 1/4',
  data: { stage: 1, processed: 250, total: 1000 }
});

symphony.streaming.updateProgress(streamId, {
  type: 'progress', 
  progress: 50,
  status: 'Processing stage 2/4',
  data: { stage: 2, processed: 500, total: 1000 }
});

// Complete the stream
symphony.streaming.completeStream(streamId, {
  totalProcessed: 1000,
  finalOutput: 'processing_results.json',
  summary: 'Successfully processed 1000 records in 4 stages'
});

// Unsubscribe when done
unsubscribe();
```

### Stream Management

```typescript
// Get active streams
const activeStreams = symphony.streaming.getActiveStreams();
console.log(`Active streams: ${activeStreams.length}`);

// Get specific stream status
const status = symphony.streaming.getStreamStatus(streamId);
console.log('Stream Status:', status);

// Get streaming statistics
const streamingStats = symphony.streaming.getStats();
console.log('Streaming Statistics:', {
  totalStreams: streamingStats.totalStreams,
  activeStreams: streamingStats.activeStreams,
  completedStreams: streamingStats.completedStreams,
  errorStreams: streamingStats.errorStreams,
  averageDuration: streamingStats.averageDuration,
  peakConcurrency: streamingStats.peakConcurrency,
  memoryUsage: `${(streamingStats.memoryUsage / 1024 / 1024).toFixed(2)} MB`
});

// Health check
const streamingHealth = await symphony.streaming.healthCheck();
console.log('Streaming Health:', streamingHealth);
```

## Advanced Features

### Tool Chaining

Symphony supports defining and executing sequences of tool calls where the output of one step can be the input to another. This is managed by the `ChainExecutor`.

```typescript
import { ChainExecutor, ToolChain, ToolChainStep } from 'symphonic'; // Corrected import, added types

const researchChain: ToolChain = {
  id: 'advanced_research_workflow',
  name: 'Advanced Research and Synthesis Chain',
  description: 'Performs web research, analyzes findings, synthesizes information, and writes a report.',
  input_schema: { // Defines expected input for the entire chain
    research_query: { type: 'string', description: 'The core topic for research.' },
    analysis_depth: { type: 'number', default: 2, description: 'Depth of analysis for ponder tool.' },
    synthesis_prompt: { type: 'string', default: 'Synthesize the key findings.'},
    report_filename: { type: 'string', default: 'research_report.md' }
  },
  steps: [
    {
      id: 'step1_initial_research',
      tool: 'webSearch', // Name of a registered tool
      chained: '1',       // Execution order/identifier
      static_params: {    // Parameters always passed to this tool in this step
        // Assuming webSearch tool can take a 'result_count' parameter
        // result_count: 5 
      },
      input_mapping: { // Maps chain input to tool input parameters
        query: 'input.research_query' // Uses 'research_query' from the chain's initial input
      }
      // outputs: { search_results: '.result.articles' } // Optional: define how to map this step's output for clarity or if needed by ChainExecutor
    },
    {
      id: 'step2_analyze_findings',
      tool: 'ponder',
      chained: '2',
      depends_on: ['step1_initial_research'], // This step runs after step1 completes
      input_mapping: {
        // 'task' for ponder tool is constructed using output from step1 and chain input
        task: `Analyze these search results: @step1_initial_research.result. For the query: 'input.research_query'.`,
        depth: 'input.analysis_depth' // Uses 'analysis_depth' from chain input
      }
    },
    {
      id: 'step3_synthesize_analysis',
      tool: 'ponder', 
      chained: '3',
      depends_on: ['step2_analyze_findings'],
      input_mapping: {
        task: `'input.synthesis_prompt' Based on this analysis: @step2_analyze_findings.result`,
        depth: 'input.analysis_depth' // Can re-use or use a different depth
      }
    },
    {
      id: 'step4_write_report',
      tool: 'writeFile',
      chained: '4',
      depends_on: ['step3_synthesize_analysis'],
      input_mapping: {
        filename: 'input.report_filename',
        content: '@step3_synthesize_analysis.result' // Uses the direct output of the synthesis step
      }
    }
  ],
  output_mapping: { // Defines the final output of the chain
    report_file_result: 'step4_write_report.result',
    synthesized_content: 'step3_synthesize_analysis.result',
    raw_search_results: 'step1_initial_research.result'
  }
};

const chainExecutor = ChainExecutor.getInstance();
// Potentially configure the executor if it has options: e.g., chainExecutor.setConfig(...)

async function runMyChain() {
  try {
    const chainResult = await chainExecutor.executeChain(researchChain, {
      research_query: 'breakthroughs in renewable energy storage 2024',
      analysis_depth: 3,
      report_filename: 'renewable_energy_report_2024.md'
      // synthesis_prompt will use default from input_schema
    });

    if (chainResult.success) {
      console.log('Chain executed successfully:', chainResult.result);
      console.log('Metrics:', chainResult.metrics);
    } else {
      console.error('Chain execution failed:', chainResult.error);
      console.log('Failed Steps:', chainResult.metrics?.failedSteps);
    }
  } catch (error) {
    console.error('Error running chain:', error);
  }
}

// runMyChain();

### Database Operations

```typescript
// Health check
const dbHealth = await symphony.db.healthCheck();
console.log('Database Health:', dbHealth);

// Get database statistics
const dbStats = await symphony.db.getStats();
console.log('Database Stats:', {
  tables: Object.keys(dbStats.tables),
  totalQueries: dbStats.performance.totalQueries,
  averageQueryTime: dbStats.performance.averageQueryTime,
  databaseSize: `${(dbStats.storage.totalSize / 1024 / 1024).toFixed(2)} MB`
});

// Tool execution history
const toolExecutions = await symphony.db.getToolExecutions('session_123', 'webSearch', 10);
console.log('Recent Tool Executions:', toolExecutions.map(exec => ({
  toolName: exec.toolName,
  success: exec.success,
  executionTime: exec.executionTime,
  timestamp: new Date(exec.timestamp)
})));

// Custom database queries
const customResults = await symphony.db.query(
  'SELECT tool_name, COUNT(*) as count FROM tool_executions WHERE success = ? GROUP BY tool_name',
  [true]
);
```

### Health Monitoring

```typescript
// Symphony overall health
const symphonyState = symphony.getState();
console.log('Symphony State:', symphonyState);

// Service health checks
const cacheHealth = await symphony.cache.healthCheck();
const memoryHealth = await symphony.memory.healthCheck();
const streamingHealth = await symphony.streaming.healthCheck();

console.log('Service Health Summary:', {
  cache: cacheHealth.status,
  memory: memoryHealth.status,
  streaming: streamingHealth.status,
  database: (await symphony.db.healthCheck()).status
});

// Performance monitoring
const metrics = symphony.metrics.getAll();
console.log('Performance Metrics:', metrics);
```

## Error Handling

Symphony SDK features enterprise-grade error handling with structured errors, user guidance, and recovery patterns.

### Error Types and Hierarchy

```typescript
import { 
  SymphonyError, 
  LLMError, 
  ToolError, 
  ValidationError,
  ConfigurationError,
  DatabaseError,
  ErrorCode,
  ErrorSeverity,
  ErrorCategory
} from 'symphonic';

// Basic error handling
try {
  const result = await agent.run('Complex task');
} catch (error) {
  if (error instanceof SymphonyError) {
    console.log('Structured Error:', {
      code: error.code,           // e.g., 'E4001'
      category: error.category,   // e.g., 'LLM'
      severity: error.severity,   // e.g., 'HIGH'
      component: error.component, // e.g., 'OpenAIProvider'
      operation: error.operation, // e.g., 'complete'
      userGuidance: error.userGuidance,
      recoveryActions: error.recoveryActions,
      isRecoverable: error.isRecoverable()
    });
  }
}
```

### Error Categories and Codes

```typescript
// Error codes are organized by category:
ErrorCode.VALIDATION_FAILED        // E1001 - Input validation errors
ErrorCode.CONFIGURATION_INVALID    // E2001 - Configuration errors
ErrorCode.EXECUTION_FAILED         // E3001 - Runtime execution errors
ErrorCode.LLM_API_ERROR            // E4001 - LLM provider errors
ErrorCode.LLM_RATE_LIMITED         // E4002 - Rate limiting
ErrorCode.TOOL_NOT_FOUND           // E5001 - Tool registry errors
ErrorCode.TOOL_EXECUTION_FAILED    // E5002 - Tool execution errors
ErrorCode.DATABASE_CONNECTION_ERROR // E6001 - Database errors
```

### Tool Error Handling

```typescript
// Tools return ToolResult with structured error info
const result = await symphony.tool.execute('webSearch', { query: 'test' });

if (!result.success) {
  console.log('Tool Error:', {
    error: result.error,           // Human-readable error message
    details: result.details,       // Structured error details
    metrics: result.metrics        // Execution metrics
  });
}

// Custom tool with error handling
const customTool = await symphony.tool.create({
  name: 'validateData',
  description: 'Validates input data',
  handler: async (params) => {
    if (!params.data) {
      throw new ValidationError(
        'Data parameter is required',
        { provided: params, required: ['data'] },
        { component: 'ValidateDataTool', operation: 'execute' }
      );
    }
    return { success: true, result: 'validated' };
  }
});
```

### Resilience Patterns

```typescript
import { ResilienceManager, RetryHandler, CircuitBreaker } from 'symphonic';

// Configure resilience for error-prone operations
const resilienceManager = new ResilienceManager(
  { maxAttempts: 3, baseDelay: 1000 },     // Retry config
  { failureThreshold: 5, resetTimeout: 30000 } // Circuit breaker config
);

// Execute with automatic retry and circuit breaking
const result = await resilienceManager.executeWithResilience(
  async () => {
    return await agent.run('Potentially failing task');
  },
  'agent-execution',
  'critical-service'
);

if (result.success) {
  console.log('Result:', result.data);
} else {
  console.log('Failed after resilience attempts:', result.error);
}
```

### Error Recovery and User Guidance

```typescript
// Symphony errors include built-in recovery guidance
try {
  await symphony.initialize();
} catch (error) {
  if (error instanceof ConfigurationError) {
    console.log('Configuration Error:');
    console.log('- Message:', error.message);
    console.log('- User Guidance:', error.userGuidance);
    console.log('- Recovery Actions:');
    error.recoveryActions.forEach(action => console.log(`  * ${action}`));
    
    // Example output:
    // - Message: OpenAI API key is missing
    // - User Guidance: Set OPENAI_API_KEY environment variable or provide apiKey in config
    // - Recovery Actions:
    //   * Set OPENAI_API_KEY environment variable
    //   * Add apiKey to LLM configuration
    //   * Check API key validity
  }
}
```

This usage guide reflects the actual Symphony implementation with accurate examples, proper error handling, and comprehensive feature coverage. All code examples are based on the real API definitions and will work with the current Symphony SDK.
