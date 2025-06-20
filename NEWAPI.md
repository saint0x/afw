# Aria Framework API Reference (`@...`)

This document provides the definitive reference for all special decorators (`@`) used in the Aria Framework. These decorators are the primary mechanism for declaring components, configuring their behavior, and interacting with the Aria Runtime's powerful systems like databases, asynchronous tasks, and memory.

## Guiding Principles

1.  **Declarative & Discoverable**: APIs should clearly state *what* they do, not *how*. They are designed to be self-documenting.
2.  **Code-Near Configuration**: Essential, structural configuration lives in code via decorators.
3.  **Tunable Configuration**: Optional, performance-related, or environmental configuration lives in `aria.toml`. The TOML file acts as a discoverable guide, with all possible options commented out.
4.  **Component Model**: The API follows a `component.method` pattern (e.g., `@db.get`, `@task.launch`). This provides a clear, organized structure.

---

## 1. Core Component Decorators

These decorators define the high-level building blocks of an Aria application.

### `@agent`
Declares a class as an autonomous agent.

**Syntax**:
`@agent({ ... })`

**Required Properties**:
*   `description: string`: A detailed description of the agent's purpose and capabilities. This is used by the planning engine.
*   `tools: Tool[]`: An array of tool classes that this agent is authorized to use.

**Optional Properties (in `aria.toml`)**:
*   `system_prompt`: A custom system prompt to guide the agent's behavior.
*   `llm`: The specific LLM provider and model to use (e.g., `{ provider = "openai", model = "gpt-4-turbo" }`).
*   `max_iterations`: The maximum number of steps the agent can take.
*   `temperature`: The creativity/randomness of the LLM responses.

**Example**:
```typescript
// src/agents/researcher.ts
import { FileTool, WebSearchTool } from "@aria-sdk/tools";
import { agent } from "@aria-sdk/core";

@agent({
  description: "A research agent that can browse the web and save findings to a file.",
  tools: [FileTool, WebSearchTool],
})
export class ResearchAgent {
  // Agent logic goes here
}
```
```toml
# aria.toml
[agents.ResearchAgent]
system_prompt = "You are a world-class researcher. Be thorough and cite your sources."
temperature = 0.0
```

### `@tool`
Declares a class as a tool that can be used by agents.

**Syntax**:
`@tool({ ... })`

**Required Properties**:
*   `name: string`: The programmatic name of the tool, used for invocation.
*   `description: string`: A detailed description of what the tool does, including its inputs and outputs. This is critical for the LLM to use the tool correctly.

**Example**:
```typescript
// src/tools/calculator.ts
import { tool } from "@aria-sdk/core";

@tool({
  name: "calculator",
  description: "A simple calculator that can perform addition, subtraction, multiplication, and division.",
})
export class CalculatorTool {
  execute(operation: string, a: number, b: number): number {
    // tool logic
  }
}
```

### `@pipeline`
Declares a class as a pipeline that orchestrates a sequence of agents or tools.

**Syntax**:
`@pipeline({ ... })`

**Required Properties**:
*   `description: string`: A description of the pipeline's overall goal.
*   `steps: Step[]`: An array of agent or tool classes defining the workflow.

**Example**:
```typescript
// src/pipelines/research_and_summarize.ts
import { pipeline } from "@aria-sdk/core";
import { ResearchAgent } from "../agents/researcher";
import { SummarizerAgent } from "../agents/summarizer";

@pipeline({
  description: "Runs a research agent and then a summarizer agent.",
  steps: [ResearchAgent, SummarizerAgent],
})
export class ResearchPipeline {}
```

### `@team`
*Future-facing: The `@team` decorator is reserved for multi-agent collaboration and is not yet implemented in the runtime.*

---

## 2. Runtime System Decorators

These decorators provide access to the Aria Runtime's stateful and asynchronous systems from within your agents and tools.

### Database (`@db`)
The `@db` decorators interact with the runtime's persistent key-value store, scoped to the current user and session. This is ideal for storing state, results, and user-specific data.

#### `@db.get(key: string)`
Retrieves a value from the database. The decorator injects the value as a property on the class instance.

**Example**:
```typescript
@agent({ ... })
export class DataProcessorAgent {
  @db.get("user:profile")
  userProfile: any;

  async run() {
    if (this.userProfile) {
      console.log(`Processing data for ${this.userProfile.name}`);
    }
  }
}
```

#### `@db.set(key: string)`
Returns a setter function that saves a value to the database under the given key.

**Example**:
```typescript
@agent({ ... })
export class DataProcessorAgent {
  @db.set("processing:result")
  saveResult: (result: any) => Promise<void>;

  async run() {
    const result = { status: "completed", timestamp: Date.now() };
    await this.saveResult(result);
  }
}
```

#### `@db.delete(key: string)`
Returns a deleter function that removes a key-value pair from the database.

**Example**:
```typescript
@agent({ ... })
export class CleanupAgent {
  @db.delete("session:cache")
  clearCache: () => Promise<void>;

  async run() {
    await this.clearCache();
    console.log("Session cache cleared.");
  }
}
```

#### `@db.query(prefix: string)`
Retrieves all key-value pairs where the key matches the given prefix.

**Example**:
```typescript
@agent({ ... })
export class ReportingAgent {
  @db.query("user:profile") // assuming multiple profiles like 'user:profile:1', 'user:profile:2'
  profiles: Map<string, any>;

  async run() {
    console.log(`Found ${this.profiles.size} user profiles.`);
  }
}
```

### Asynchronous Tasks (`@task`)
The `@task` decorators manage long-running, asynchronous operations that can execute outside the main agent loop, such as running a container, processing large files, or performing complex computations.

#### `@task.launch()`
Returns a function to launch a new asynchronous task. It takes the task type and payload as arguments and returns a `task_id`.

**Example**:
```typescript
@agent({ ... })
export class CodeRunnerAgent {
  @task.launch()
  launchTask: (task: { type: string, payload: any }) => Promise<string>;

  async run() {
    const taskId = await this.launchTask({
      type: "container:exec",
      payload: {
        image: "python:3.9-slim",
        command: ["python", "-c", "print('Hello from a container!')"],
      },
    });
    console.log(`Launched container task with ID: ${taskId}`);
  }
}
```

#### `@task.status()`
Returns a function to check the status of a previously launched task. It takes a `task_id` and returns the task's status record.

**Example**:
```typescript
@agent({ ... })
export class TaskMonitorAgent {
  @task.status()
  getTaskStatus: (taskId: string) => Promise<any>;

  async checkOnTask(id: string) {
    const status = await this.getTaskStatus(id);
    console.log(`Task ${id} status: ${status.status}`); // e.g., 'running', 'completed', 'failed'
  }
}
```

### Memory (`@memory`)
The `@memory` decorators provide access to a short-term, in-memory store for the current execution context. This is useful for passing data between steps in a plan without persisting it to the database.

#### `@memory.store(key: string)`
Returns a setter function to store a value in the current context's memory.

**Example**:
```typescript
@tool({ name: "data_fetcher", ... })
export class DataFetcherTool {
  @memory.store("raw_data")
  storeRawData: (data: any) => Promise<void>;

  async execute() {
    const data = { id: 123, value: "some important data" };
    await this.storeRawData(data);
  }
}
```

#### `@memory.retrieve(key: string)`
Injects a value from the context's memory as a property on the class instance.

**Example**:
```typescript
@tool({ name: "data_processor", ... })
export class DataProcessorTool {
  @memory.retrieve("raw_data")
  rawData: any;

  execute() {
    if (this.rawData) {
      console.log(`Processing retrieved data: ${this.rawData.value}`);
    }
  }
}
```

### Structured Logging (`@log`)
The `@log` decorators provide access to the runtime's structured logging system. Logs are automatically tagged with context like `user_id`, `session_id`, and `agent_name`.

#### `@log.info()`
Returns a function to write an informational log message.

**Example**:
```typescript
@agent({ ... })
export class MyAgent {
  @log.info()
  logInfo: (message: string, data?: any) => void;

  async run() {
    this.logInfo("Agent execution started.", { step: 1 });
  }
}
```

#### `@log.warn()`
Returns a function to write a warning message.

**Example**:
```typescript
@agent({ ... })
export class MyAgent {
  @log.warn()
  logWarn: (message: string, data?: any) => void;

  async run() {
    this.logWarn("API rate limit approaching.", { usage: "95%" });
  }
}
```

#### `@log.error()`
Returns a function to write an error message.

**Example**:
```typescript
@agent({ ... })
export class MyAgent {
  @log.error()
  logError: (message: string, error: Error, data?: any) => void;

  async run() {
    try {
      // ... risky operation ...
    } catch (e) {
      this.logError("Failed to complete operation.", e, { details: "..." });
    }
  }
}
``` 