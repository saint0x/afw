# Symphony SDK: API Reference

This document provides a detailed reference for the public API of the Symphony SDK, outlining the methods, parameters, and expected return types for interacting with its core modules. For functional examples, please refer to `USAGE.md`.

## 1. Symphony Client Initialization

The main entry point to the SDK is the `Symphony` class.

```typescript
import { Symphony, SymphonyConfig, LogLevel } from 'symphonic';

// Current SymphonyConfig Interface (based on src/types/symphony.ts)
interface SymphonyConfig {
  name?: string;
  version?: string;
  environment?: 'development' | 'staging' | 'production';
  
  // Logging configuration
  logging?: {
    level?: 'debug' | 'info' | 'warn' | 'error';
    format?: 'json' | 'simple';
  };
  
  // LLM configuration (uses RichLLMConfig from src/llm/types)
  llm?: {
    provider: string;       // e.g., 'openai', 'anthropic'
    model: string;          // e.g., 'gpt-4o-mini', 'claude-2'
    apiKey?: string;        // API key (can also be set via environment variables)
    temperature?: number;
    maxTokens?: number;
    timeout?: number;
  };
  
  // Database configuration with intelligent defaults
  db?: {
    enabled: boolean;
    adapter?: string;       // e.g., 'sqlite', 'postgres'
    path?: string;          // For file-based DBs like SQLite
    connection?: string;    // For server-based DBs
    options?: {
      timeout?: number;
      autoBackup?: boolean;
      maxSize?: string;
    };
  };
  
  // Cache Intelligence configuration
  cache?: {
    enablePatternMatching?: boolean;
    enableContextTrees?: boolean;
    fastPathThreshold?: number;
    contextMaxNodes?: number;
  };
  
  // Memory system configuration
  memory?: {
    shortTerm?: {
      defaultTTL?: number;     // Default Time-To-Live in milliseconds
      maxSize?: number;        // Max size in bytes
    };
    longTerm?: {
      defaultTTL?: number;
      maxSize?: number;
    };
    aggregationEnabled?: boolean;
  };
  
  // Streaming configuration
  streaming?: {
    enableRealTimeUpdates?: boolean;
    progressUpdateInterval?: number;
    maxConcurrentStreams?: number;
    enableWebSocket?: boolean;
    bufferSize?: number;
  };
  
  // Runtime configuration
  runtime?: {
    enhancedRuntime?: boolean;
    planningThreshold?: 'simple' | 'multi_step' | 'complex';
    reflectionEnabled?: boolean;
    maxStepsPerPlan?: number;
    timeoutMs?: number;
    retryAttempts?: number;
    debugMode?: boolean;
  };
  
  // Service registry configuration (NEW)
  serviceRegistry?: {
    enabled: boolean;
    maxRetries: number;
    retryDelay: number;
  };
  
  // Metrics configuration (NEW)
  metrics?: {
    enabled: boolean;
    detailed: boolean;
  };
}

// Constructor
const symphony = new Symphony(config: SymphonyConfig);

// Initialization
async function initialize(): Promise<void>;
```

### Database Configuration Intelligence

The Symphony SDK automatically configures the database based on your settings:

- **`shouldEnableDatabase()`**: Intelligent detection of database requirements
- **`configureDatabaseForProduction()`**: Automatic SQLite optimization for production
- **Mock Database Service**: Seamless in-memory fallback when database is disabled
- **Table Creation**: Automatic creation of required tables (`tool_executions`, `patterns`, `context_sessions`)

```typescript
// Database disabled - uses in-memory mock services
const devSymphony = new Symphony({
  db: { enabled: false },
  llm: { provider: 'openai', model: 'gpt-4o-mini', apiKey: process.env.OPENAI_API_KEY }
});

// Database enabled - auto-configures SQLite with production settings
const prodSymphony = new Symphony({
  db: { enabled: true },  // Automatically creates ./symphony.db with optimized settings
  llm: { provider: 'openai', model: 'gpt-4o-mini', apiKey: process.env.OPENAI_API_KEY }
});
```

- **`new Symphony(config: SymphonyConfig)`**: Creates a new Symphony client instance.
  - `config.llm`: Mandatory. Defines the primary LLM provider and model.
  - `config.db`: Optional. Configures database with intelligent defaults and mock fallback.
  - `config.cache`: Optional. Advanced cache intelligence settings.
  - `config.memory`: Optional. Configuration for short-term and long-term memory.
  - `config.streaming`: Optional. Settings for real-time progress updates.
  - `config.runtime`: Optional. Enhanced runtime configuration for planning and reflection.
  - `config.serviceRegistry`: Optional. Service management configuration.
  - `config.metrics`: Optional. Metrics collection settings.
- **`symphony.initialize(): Promise<void>`**: Asynchronously initializes the Symphony client, setting up database connections, services, and performing health checks. Must be called before operations.

## 2. Tool Management (`symphony.tool`)

Manages the creation and registration of tools using the current ToolConfig and ToolResult interfaces.

```typescript
import { ToolConfig, ToolResult } from 'symphonic';

// Current ToolConfig interface (from src/types/sdk.ts)
interface ToolConfig {
  name: string;
  description?: string;
  type: string;                    // e.g., 'filesystem', 'web', 'custom'
  nlp?: string;                   // Natural language pattern for tool invocation
  apiKey?: string;                // API key if tool needs external service
  timeout?: number;               // Timeout in milliseconds
  retryCount?: number;            // Number of retry attempts
  maxSize?: number;               // Maximum input/output size
  config: Record<string, any>;    // Tool-specific configuration
  inputs?: string[];              // Input parameter names
  outputs?: string[];             // Output field names
  capabilities?: string[];        // Tool capabilities/tags
  handler?: (params: any) => Promise<ToolResult<any>>; // Execution handler
}

// Current ToolResult interface with structured error handling
interface ToolResult<T = any> {
  success: boolean;
  result?: T;                     // Actual result data
  error?: string;                 // Human-readable error message
  details?: VerificationError[]; // Structured validation errors
  metrics?: {
    duration: number;
    startTime: number;
    endTime: number;
  };
}

interface VerificationError {
  path: string;
  message: string;
  expected?: any;
  received?: any;
}

// Create / Register a new tool
async function create(toolDefinition: ToolConfig): Promise<Tool>;
```

### Tool Creation Examples

```typescript
// Basic tool with error handling
const basicTool = await symphony.tool.create({
  name: 'processData',
  description: 'Processes input data with validation',
  type: 'data_processing',
  config: {
    inputSchema: {
      type: 'object',
      properties: {
        data: { type: 'object', required: true },
        options: { type: 'object' }
      },
      required: ['data']
    }
  },
  handler: async (params) => {
    try {
      if (!params.data) {
        return {
          success: false,
          error: 'Data parameter is required',
          details: [{
            path: 'data',
            message: 'Missing required parameter',
            expected: 'object',
            received: typeof params.data
          }]
        };
      }
      
      const processed = { ...params.data, processed: true };
      return {
        success: true,
        result: processed,
        metrics: {
          duration: Date.now() - startTime,
          startTime,
          endTime: Date.now()
        }
      };
    } catch (error) {
      return {
        success: false,
        error: `Processing failed: ${error.message}`,
        details: []
      };
    }
  }
});

// Advanced tool with capabilities and retry logic
const advancedTool = await symphony.tool.create({
  name: 'webApiCall',
  description: 'Makes HTTP requests with retry logic',
  type: 'web',
  nlp: 'make api call to web service',
  timeout: 30000,
  retryCount: 3,
  capabilities: ['http', 'json', 'retry'],
  config: {
    baseUrl: 'https://api.example.com',
    defaultHeaders: { 'Content-Type': 'application/json' }
  },
  handler: async (params) => {
    // Implementation with automatic retry on failure
    // Returns ToolResult with structured error handling
  }
});
```

### Tool Execution with Error Handling

```typescript
// Execute tool and handle results
const result = await symphony.tool.execute('processData', { 
  data: { id: 1, name: 'test' } 
});

if (result.success) {
  console.log('Tool succeeded:', result.result);
  console.log('Execution time:', result.metrics?.duration + 'ms');
} else {
  console.log('Tool failed:', result.error);
  
  // Handle structured validation errors
  if (result.details && result.details.length > 0) {
    console.log('Validation errors:');
    result.details.forEach(error => {
      console.log(`- ${error.path}: ${error.message}`);
      console.log(`  Expected: ${error.expected}, Received: ${error.received}`);
    });
  }
}

// Get available tools
const availableTools = symphony.tool.getAvailable();
console.log('Available tools:', availableTools);

// Get tool information
const toolInfo = symphony.tool.getInfo('processData');
console.log('Tool info:', toolInfo);
```

- **`symphony.tool.create(toolDefinition: ToolConfig): Promise<Tool>`**: Registers a new custom tool with the SDK's `ToolRegistry`.
  - `toolDefinition`: An object conforming to the current `ToolConfig` interface with comprehensive configuration options.
  - Returns a reference to the created/registered tool.
  - Tools automatically use structured error handling and validation.
- **`symphony.tool.execute(toolName: string, params: any): Promise<ToolResult>`**: Executes a registered tool with parameters.
  - Returns a `ToolResult` with success/failure status, structured errors, and execution metrics.
- **`symphony.tool.getAvailable(): string[]`**: Returns list of available tool names.
- **`symphony.tool.getInfo(toolName: string): any`**: Returns detailed information about a specific tool.

## 3. Agent Management (`symphony.agent`)

Manages the creation and execution of intelligent agents using the current AgentConfig interface.

```typescript
import { AgentConfig, AgentOptions, ServiceResult } from 'symphonic';

// Current AgentConfig interface (from src/types/sdk.ts)
interface AgentConfig {
  name: string;
  description: string;
  task: string;                    // Primary task or goal for the agent
  tools: string[];                 // Array of tool names the agent can use
  llm: LLMBaseConfig | string;     // LLM configuration or simple model string
  directives?: string;             // Additional instructions for the agent
  systemPrompt?: string;           // Custom system prompt
  maxCalls?: number;              // Maximum LLM calls per execution
  requireApproval?: boolean;       // Whether to require approval for actions
  timeout?: number;               // Execution timeout in milliseconds
  capabilities?: string[];         // Agent capabilities/tags for coordination
  enableCache?: boolean;          // Enable LLM response caching
  enableStreaming?: boolean;      // Enable real-time progress updates
  streamOptions?: {
    updateInterval?: number;
    includeIntermediateSteps?: boolean;
  };
  log?: {                         // Logging configuration
    inputs?: boolean;
    outputs?: boolean;
    llmCalls?: boolean;
    toolCalls?: boolean;
  };
}

// LLM configuration options
interface LLMBaseConfig {
  model: string;                  // LLM model (e.g., gpt-4, claude-2)
  provider?: string;              // LLM provider (e.g., openai, anthropic)
  apiKey?: string;               // API key (can be set globally)
  temperature?: number;          // Response creativity (0.0-1.0)
  maxTokens?: number;            // Maximum response tokens
}

// Agent execution options
interface AgentOptions {
  onProgress?: (update: { status: string; result?: any }) => void;
  onMetrics?: (metrics: { [key: string]: any }) => void;
  timeout?: number;
}

// Create a new agent
async function create(agentDefinition: AgentConfig): Promise<Agent>;
```

### Agent Creation Examples

```typescript
// Basic agent with tools
const basicAgent = await symphony.agent.create({
  name: 'DataAnalyst',
  description: 'Specialized in data analysis and reporting',
  task: 'Analyze datasets and generate insights reports',
  tools: ['webSearch', 'ponder', 'writeFile'],
  llm: 'gpt-4o-mini'  // Simple string configuration
});

// Advanced agent with detailed configuration
const advancedAgent = await symphony.agent.create({
  name: 'ResearchAssistant',
  description: 'Comprehensive research and analysis specialist',
  task: 'Conduct thorough research on complex topics',
  tools: ['webSearch', 'parseDocument', 'ponder', 'writeFile', 'createPlan'],
  llm: {
    model: 'gpt-4o-mini',
    provider: 'openai',
    temperature: 0.7,
    maxTokens: 4000
  },
  systemPrompt: 'You are an expert research assistant. Always cite sources and provide detailed analysis.',
  directives: 'Focus on accuracy and comprehensive coverage. Verify facts from multiple sources.',
  capabilities: ['research', 'analysis', 'writing', 'fact-checking'],
  maxCalls: 10,
  timeout: 300000,  // 5 minutes
  enableCache: true,
  enableStreaming: true,
  streamOptions: {
    updateInterval: 1000,
    includeIntermediateSteps: true
  },
  log: {
    inputs: true,
    outputs: true,
    llmCalls: true,
    toolCalls: true
  }
});

// Agent execution with progress tracking
const result = await advancedAgent.execute('Research the latest developments in quantum computing', {
  timeout: 600000,  // 10 minutes for complex research
  onProgress: (update) => {
    console.log(`Agent Status: ${update.status}`);
    if (update.result) {
      console.log('Intermediate Result:', update.result);
    }
  },
  onMetrics: (metrics) => {
    console.log('Performance Metrics:', {
      duration: metrics.duration,
      llmCalls: metrics.llmCalls,
      toolCalls: metrics.toolCalls
    });
  }
});
```

- **`symphony.agent.create(agentDefinition: AgentConfig): Promise<Agent>`**: Creates and configures a new agent instance.
  - Agents with tools automatically use JSON mode for reliable tool interaction
  - Supports comprehensive logging and metrics collection
  - Built-in streaming capabilities for long-running tasks
- **`agent.execute(taskDescription: string, options?: AgentOptions): Promise<ServiceResult>`**: Executes a task with the agent.
  - Returns detailed results with success/failure status and execution metrics
  - Supports progress callbacks and timeout configuration

## 4. Team Management (`symphony.team`)

Manages the creation and operation of teams of agents using the current TeamConfig interface.

```typescript
import { TeamConfig, TeamResult, TeamOptions, AgentConfig } from 'symphonic';

// Current TeamConfig interface (from src/types/sdk.ts)
interface TeamConfig {
  name: string;
  description: string;
  agents: Array<string | AgentConfig>;  // Agent names or full configurations
  capabilities?: string[];              // Team-level capabilities
  manager?: boolean;                   // Whether team has a designated manager
  strategy?: TeamStrategy;             // Coordination strategy
  delegationStrategy?: DelegationStrategy;  // Task delegation rules
  log?: {                             // Team-level logging
    inputs?: boolean;
    outputs?: boolean;
    metrics?: boolean;
  };
}

// Team coordination strategy
interface TeamStrategy {
  name?: string;                      // Strategy name (e.g., 'parallel', 'sequential')
  description?: string;               // Strategy description
  assignmentLogic?: (task: string, agents: string[]) => Promise<string[]>;
  coordinationRules?: {
    maxParallelTasks?: number;        // Maximum concurrent tasks
    taskTimeout?: number;             // Timeout for individual tasks
  };
}

// Task delegation strategy
interface DelegationStrategy {
  type: 'custom' | 'rule-based';      // Delegation approach
  customLogic?: (task: string, agents: string[]) => Promise<string[]>;
  rules?: Array<{                     // Rule-based delegation
    condition: string;
    assignTo: string[];
  }>;
}

// Team execution options
interface TeamOptions {
  onProgress?: (update: { status: string; agent?: string; result?: any }) => void;
  onMetrics?: (metrics: { [key: string]: any }) => void;
  timeout?: number;
}

// Create a new team
async function create(teamDefinition: TeamConfig): Promise<Team>;
```

### Team Creation Examples

```typescript
// Basic team with agent configurations
const developmentTeam = await symphony.team.create({
  name: 'SoftwareDevelopmentTeam',
  description: 'Full-stack development team with specialized roles',
  agents: [
    {
      name: 'BackendDeveloper',
      description: 'Server-side development specialist',
      task: 'Develop APIs, databases, and backend services',
      tools: ['writeCode', 'readFile', 'writeFile', 'ponder'],
      llm: 'gpt-4o-mini',
      capabilities: ['nodejs', 'python', 'database', 'api-design']
    },
    {
      name: 'FrontendDeveloper', 
      description: 'User interface development specialist',
      task: 'Create responsive and interactive user interfaces',
      tools: ['writeCode', 'readFile', 'writeFile', 'webSearch'],
      llm: {
        model: 'gpt-4o-mini',
        temperature: 0.6  // More creative for UI work
      },
      capabilities: ['react', 'typescript', 'css', 'ux-design']
    },
    {
      name: 'QAEngineer',
      description: 'Quality assurance and testing specialist', 
      task: 'Design and execute comprehensive testing strategies',
      tools: ['writeCode', 'readFile', 'ponder'],
      llm: 'gpt-4o-mini',
      capabilities: ['testing', 'automation', 'quality-assurance']
    }
  ],
  capabilities: ['software-development', 'web-applications'],
  strategy: {
    name: 'adaptive-coordination',
    description: 'Coordinates based on task complexity and agent availability',
    coordinationRules: {
      maxParallelTasks: 3,
      taskTimeout: 1800000  // 30 minutes per task
    }
  },
  delegationStrategy: {
    type: 'rule-based',
    rules: [
      {
        condition: 'task includes "backend" or "api" or "database"',
        assignTo: ['BackendDeveloper']
      },
      {
        condition: 'task includes "frontend" or "ui" or "interface"',
        assignTo: ['FrontendDeveloper']
      },
      {
        condition: 'task includes "test" or "quality" or "bug"',
        assignTo: ['QAEngineer']
      }
    ]
  },
  log: {
    inputs: true,
    outputs: true,
    metrics: true
  }
});

// Team execution with strategy options
const result = await developmentTeam.execute('Build a user authentication system with frontend and backend components', {
  timeout: 3600000,  // 1 hour for complex task
  onProgress: (update) => {
    console.log(`Team Progress: ${update.status}`);
    if (update.agent) {
      console.log(`Active Agent: ${update.agent}`);
    }
  },
  onMetrics: (metrics) => {
    console.log('Team Metrics:', {
      totalDuration: metrics.totalDuration,
      agentUtilization: metrics.agentUtilization,
      parallelTasks: metrics.parallelTasks
    });
  }
});

// Research team with sequential strategy
const researchTeam = await symphony.team.create({
  name: 'ResearchTeam',
  description: 'Collaborative research and analysis team',
  agents: ['PrimaryResearcher', 'DataAnalyst', 'ReportWriter'],  // Reference existing agents
  strategy: {
    name: 'sequential',
    description: 'Execute tasks in sequence, passing results between agents'
  },
  capabilities: ['research', 'analysis', 'reporting']
});
```

- **`symphony.team.create(teamDefinition: TeamConfig): Promise<Team>`**: Creates and configures a new team.
  - Supports both inline agent definitions and references to existing agents
  - Flexible coordination strategies for different collaboration patterns
  - Built-in delegation rules based on agent capabilities
- **`team.execute(taskDescription: string, options?: TeamOptions): Promise<TeamResult>`**: Executes a task with the team.
  - Automatically coordinates agent collaboration based on strategy
  - Provides detailed metrics on team performance and utilization

## 5. Pipeline Management (`symphony.pipeline`)

Manages the creation and execution of multi-step workflows.

```

## 6. Memory System (`symphony.memory`)

Manages short-term and long-term memory with comprehensive search and aggregation capabilities.

```typescript
// Memory storage options
interface MemoryStoreOptions {
  sessionId?: string;              // Session identifier for grouping
  namespace?: string;              // Logical grouping namespace
  tags?: string[];                // Searchable tags
  metadata?: Record<string, any>; // Additional metadata
  priority?: number;              // Priority level (1-10)
  ttl?: number;                   // Time-to-live override in milliseconds
}

// Memory retrieval options
interface MemoryRetrieveOptions {
  namespace?: string;             // Filter by namespace
  includeMetadata?: boolean;      // Include metadata in response
}

// Memory search query
interface MemorySearchQuery {
  type?: 'short_term' | 'long_term'; // Memory type filter
  namespace?: string;             // Namespace filter
  sessionId?: string;             // Session filter
  tags?: string[];               // Tag filters (AND logic)
  textSearch?: string;           // Full-text search
  limit?: number;                // Result limit (default: 50)
  sortBy?: 'timestamp' | 'priority' | 'size'; // Sort field
  sortOrder?: 'asc' | 'desc';    // Sort direction
  dateRange?: {                  // Date range filter
    start: Date;
    end: Date;
  };
}

// Memory entry structure
interface MemoryEntry {
  key: string;                   // Entry identifier
  value: any;                    // Stored value
  type: 'short_term' | 'long_term'; // Memory type
  namespace?: string;            // Namespace
  sessionId?: string;            // Session ID
  tags?: string[];              // Tags
  metadata?: Record<string, any>; // Metadata
  priority?: number;             // Priority level
  size: number;                  // Entry size in bytes
  createdAt: Date;              // Creation timestamp
  updatedAt: Date;              // Last update timestamp
  expiresAt?: Date;             // Expiration timestamp
}

// Basic Operations
async function store(key: string, value: any, type: 'short_term' | 'long_term', options?: MemoryStoreOptions): Promise<void>;
async function retrieve(key: string, type: 'short_term' | 'long_term', options?: MemoryRetrieveOptions): Promise<any | null>;
async function search(query: MemorySearchQuery): Promise<MemoryEntry[]>;
async function aggregate(query: { namespace?: string; tags?: string[]; dateRange?: { start: Date; end: Date } }): Promise<any>;

// Statistics and monitoring
async function getStats(): Promise<MemoryStats>;
async function getOperationalStats(): Promise<OperationalStats>;
async function healthCheck(): Promise<HealthStatus>;
```

### Memory Usage Examples

```typescript
// Store different types of memories
await symphony.memory.store('user_preferences', 
  { theme: 'dark', language: 'en', notifications: true }, 
  'short_term',
  {
    sessionId: 'user_session_123',
    namespace: 'user_settings',
    tags: ['preferences', 'ui'],
    priority: 8,
    metadata: { source: 'user_input', confidence: 0.95 }
  }
);

await symphony.memory.store('project_knowledge', 
  { architecture: 'microservices', technologies: ['node', 'react'], constraints: [] },
  'long_term',
  {
    namespace: 'project_context',
    tags: ['architecture', 'knowledge'],
    priority: 10,
    ttl: 30 * 24 * 60 * 60 * 1000  // 30 days
  }
);

// Advanced search with multiple criteria
const searchResults = await symphony.memory.search({
  type: 'short_term',
  namespace: 'user_settings',
  tags: ['preferences'],
  textSearch: 'theme',
  dateRange: {
    start: new Date(Date.now() - 24 * 60 * 60 * 1000), // Last 24 hours
    end: new Date()
  },
  sortBy: 'priority',
  sortOrder: 'desc',
  limit: 10
});

// Memory aggregation for insights
const aggregation = await symphony.memory.aggregate({
  namespace: 'user_behavior',
  tags: ['interaction'],
  dateRange: {
    start: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000), // Last week
    end: new Date()
  }
});
```

## 7. Streaming Service (`symphony.streaming`)

Handles real-time progress updates and event streaming for long-running operations.

```typescript
// Stream creation options
interface StreamOptions {
  id: string;                    // Unique stream identifier
  type: 'agent' | 'team' | 'pipeline' | 'tool' | 'chain'; // Stream type
  enableProgress?: boolean;      // Enable progress updates
  progressInterval?: number;     // Update interval in milliseconds
  metadata?: Record<string, any>; // Additional metadata
}

// Progress update structure
interface ProgressUpdate {
  id: string;                    // Stream ID
  type: 'agent' | 'team' | 'pipeline' | 'tool' | 'chain'; // Update type
  status: 'started' | 'progress' | 'completed' | 'error' | 'cancelled'; // Status
  progress: {                    // Progress information
    current: number;
    total: number;
    percentage: number;
  };
  message?: string;              // Status message
  data?: any;                   // Additional data
  timestamp: Date;              // Update timestamp
  duration?: number;            // Duration since start in milliseconds
}

// Streaming statistics
interface StreamingStats {
  activeStreams: number;         // Current active streams
  totalStreamsCreated: number;   // Total streams created
  messagesSent: number;          // Total messages sent
  averageStreamDuration: number; // Average stream duration
  peakConcurrentStreams: number; // Peak concurrent streams
}

// Stream management
function createStream(options: StreamOptions): string;  // Returns stream ID
function subscribe(streamId: string, callback: (update: ProgressUpdate) => void): () => void; // Returns unsubscribe function
function updateProgress(streamId: string, progress: Partial<ProgressUpdate>): void;
function completeStream(streamId: string, finalData?: any): void;
function errorStream(streamId: string, error: Error): void;

// Stream monitoring
function getActiveStreams(): string[];
function getStreamStatus(streamId: string): StreamContext | null;
function getStats(): StreamingStats;
async function healthCheck(): Promise<HealthStatus>;
```

### Streaming Usage Examples

```typescript
// Create and manage a stream for long-running operation
const streamId = symphony.streaming.createStream({
  id: 'data-processing-task',
  type: 'pipeline',
  enableProgress: true,
  progressInterval: 1000,  // Update every second
  metadata: { userId: 'user123', taskType: 'data-analysis' }
});

// Subscribe to stream updates
const unsubscribe = symphony.streaming.subscribe(streamId, (update) => {
  console.log(`${update.type} - ${update.status}: ${update.progress.percentage}%`);
  
  if (update.message) {
    console.log(`Message: ${update.message}`);
  }
  
  if (update.status === 'completed') {
    console.log('Task completed successfully!', update.data);
  } else if (update.status === 'error') {
    console.error('Task failed:', update.data);
  }
});

// Update progress during operation
symphony.streaming.updateProgress(streamId, {
  progress: { current: 25, total: 100, percentage: 25 },
  message: 'Processing data batch 1/4',
  data: { processed: 250, errors: 0 }
});

// Complete the stream
symphony.streaming.completeStream(streamId, {
  totalProcessed: 1000,
  successRate: 0.98,
  outputFile: 'results.json'
});

// Clean up
unsubscribe();
```

## 8. NLP Service (`symphony.nlp`)

Manages natural language patterns for tool invocation and context understanding.

```typescript
// NLP pattern definition
interface NlpPatternDefinition {
  toolName: string;              // Associated tool name
  nlpPattern: string;            // Natural language pattern
  id?: string;                   // Optional unique identifier
  version?: string;              // Pattern version
  isActive?: boolean;            // Whether pattern is active
  source?: string;               // Pattern source/origin
}

// Stored NLP pattern (database representation)
interface StoredNlpPattern extends NlpPatternDefinition {
  id: string;                    // Unique identifier (required in stored version)
  createdAt: Date;              // Creation timestamp
  updatedAt: Date;              // Last update timestamp
}

// Pattern management result
interface NlpManagementResult {
  created: number;               // Patterns created
  updated: number;               // Patterns updated
  skipped: number;               // Patterns skipped
  failed: number;                // Patterns that failed
  errors: ErrorDetail[];         // Detailed error information
  totalAttempted: number;        // Total patterns processed
}

// Seeding options
interface NlpSeedOptions {
  forceOverwrite?: boolean;      // Overwrite existing patterns
  defaultSource?: string;        // Default source attribution
}

// NLP Service Methods
async function seedPatterns(patterns: NlpPatternDefinition[], options?: NlpSeedOptions): Promise<NlpManagementResult>;
async function seedPatternsFromFile(filePath: string, options?: NlpSeedOptions): Promise<NlpManagementResult>;
async function getNlpPatternById(patternId: string): Promise<StoredNlpPattern | null>;
async function getNlpPatternsByTool(toolName: string): Promise<StoredNlpPattern[]>;
async function addNlpPattern(pattern: NlpPatternDefinition, options?: { allowUpdate?: boolean }): Promise<StoredNlpPattern>;
async function updateNlpPattern(patternId: string, updates: Partial<Omit<StoredNlpPattern, 'id' | 'createdAt' | 'updatedAt'>>): Promise<StoredNlpPattern>;
async function deleteNlpPattern(patternId: string): Promise<boolean>;
async function ensurePatternPersisted(pattern: NlpPatternDefinition, options?: { forceOverwrite?: boolean }): Promise<StoredNlpPattern>;
async function loadPatternToRuntime(pattern: NlpPatternDefinition): Promise<void>;
async function loadAllPersistedPatternsToRuntime(): Promise<{ loaded: number; failed: number; errors: ErrorDetail[] }>;
```

### NLP Service Usage Examples

```typescript
// Seed multiple patterns from configuration
const patterns: NlpPatternDefinition[] = [
  {
    toolName: 'webSearch',
    nlpPattern: 'search for information about',
    source: 'default_config'
  },
  {
    toolName: 'writeFile',
    nlpPattern: 'save content to file',
    source: 'default_config'
  },
  {
    toolName: 'ponder',
    nlpPattern: 'think deeply about',
    source: 'default_config'
  }
];

const seedResult = await symphony.nlp.seedPatterns(patterns, {
  forceOverwrite: false,
  defaultSource: 'system_initialization'
});

console.log(`Seeded ${seedResult.created} patterns, ${seedResult.failed} failed`);

// Load patterns from file
const fileResult = await symphony.nlp.seedPatternsFromFile('./nlp-patterns.json', {
  forceOverwrite: true
});

// Manage individual patterns
const customPattern = await symphony.nlp.addNlpPattern({
  toolName: 'customTool',
  nlpPattern: 'process custom data with special handling',
  source: 'user_defined',
  isActive: true
}, { allowUpdate: false });

// Load patterns to runtime for immediate use
await symphony.nlp.loadPatternToRuntime({
  toolName: 'emergencyTool',
  nlpPattern: 'handle emergency situation',
  source: 'runtime_addition'
});

// Load all persisted patterns at startup
const loadResult = await symphony.nlp.loadAllPersistedPatternsToRuntime();
console.log(`Loaded ${loadResult.loaded} patterns, ${loadResult.failed} failed`);
```

## 9. Validation Service (`symphony.validation`)

Provides comprehensive validation for configurations, inputs, and system state.

```typescript
// Validation result
interface ValidationResult {
  isValid: boolean;              // Overall validation result
  errors: string[];              // Validation error messages
  warnings?: string[];           // Non-blocking warnings
  details?: ValidationDetail[]; // Detailed validation information
}

// Detailed validation information
interface ValidationDetail {
  field: string;                 // Field that failed validation
  value: any;                   // Provided value
  expected: string;             // Expected format/type
  message: string;              // Detailed error message
  severity: 'error' | 'warning'; // Issue severity
}

// Validation options
interface ValidationOptions {
  strict?: boolean;             // Enable strict validation mode
  allowPartial?: boolean;       // Allow partial validation
  context?: string;            // Validation context for better errors
}

// Validation Service Methods
async function validate(config: any, type: string, options?: ValidationOptions): Promise<ValidationResult>;
async function validateAgentConfig(config: AgentConfig): Promise<ValidationResult>;
async function validateTeamConfig(config: TeamConfig): Promise<ValidationResult>;
async function validateToolConfig(config: ToolConfig): Promise<ValidationResult>;
async function validateSymphonyConfig(config: SymphonyConfig): Promise<ValidationResult>;
async function validateSchema(data: any, schema: object): Promise<ValidationResult>;
```

### Validation Service Usage Examples

```typescript
// Validate agent configuration
const agentValidation = await symphony.validation.validateAgentConfig({
  name: 'TestAgent',
  description: 'Test agent for validation',
  task: 'Perform test operations',
  tools: ['webSearch', 'invalidTool'], // invalidTool doesn't exist
  llm: 'gpt-4o-mini'
});

if (!agentValidation.isValid) {
  console.log('Agent validation failed:');
  agentValidation.errors.forEach(error => console.log(`  - ${error}`));
}

// Validate team configuration with detailed feedback
const teamValidation = await symphony.validation.validateTeamConfig({
  name: '', // Invalid: empty name
  description: 'Test team',
  agents: [],  // Warning: empty agents array
  strategy: {
    name: 'invalid_strategy' // Invalid strategy name
  }
});

// Validate custom schema
const customValidation = await symphony.validation.validateSchema(
  { name: 'test', age: 'invalid' }, // age should be number
  {
    type: 'object',
    properties: {
      name: { type: 'string', required: true },
      age: { type: 'number', required: true }
    }
  }
);

// Validate Symphony configuration
const symphonyValidation = await symphony.validation.validateSymphonyConfig({
  llm: {
    provider: 'openai',
    model: 'gpt-4o-mini'
    // Missing apiKey - validation will catch this
  },
  db: {
    enabled: true,
    adapter: 'postgresql', // Valid but may warn about configuration
    path: './invalid/path'  // Invalid path
  }
});
```