# Aria SDK: The Definitive Guide

This guide provides exhaustive documentation for developing on the Aria platform. It reflects the modern Aria development model, which emphasizes a clean separation between logic (`.ts` files) and configuration (`aria.toml`). Every feature from the legacy Symphony SDK has been mapped to this new, more robust architecture.

## Table of Contents

- [**1. The Aria Development Model**](#1-the-aria-development-model)
- [**2. Project Setup & Global Configuration**](#2-project-setup--global-configuration)
  - [The `arc new` Command](#the-arc-new-command)
  - [Global Configuration (`aria.toml`)](#global-configuration-ariatoml)
- [**3. Components: Tools**](#3-components-tools)
  - [Tool Concept](#tool-concept)
  - [The `@tool` Decorator API](#the-tool-decorator-api)
  - [Tool Configuration (`aria.toml`)](#tool-configuration-ariatoml)
  - [Example: A Resilient Tool](#example-a-resilient-tool)
- [**4. Components: Agents**](#4-components-agents)
  - [Agent Concept](#agent-concept)
  - [The `@agent` Decorator API](#the-agent-decorator-api)
  - [Agent Configuration (`aria.toml`)](#agent-configuration-ariatoml)
  - [Example: A Fine-Tuned Agent](#example-a-fine-tuned-agent)
- [**5. Components: Teams**](#5-components-teams)
  - [Team Concept](#team-concept)
  - [The `@team` Decorator API](#the-team-decorator-api)
  - [Team Configuration (`aria.toml`)](#team-configuration-ariatoml)
- [**6. Components: Pipelines (Advanced Workflows)**](#6-components-pipelines-advanced-workflows)
  - [Pipeline Concept](#pipeline-concept)
  - [The `@pipeline` Decorator API](#the-pipeline-decorator-api)
  - [Pipeline Configuration (`aria.toml`)](#pipeline-configuration-ariatoml)
- [**7. The `arc` CLI: Your Control Plane**](#7-the-arc-cli-your-control-plane)
- [**8. Platform Intelligence & Runtimes**](#8-platform-intelligence--runtimes)
  - [The LLM Runtime](#the-llm-runtime)
  - [The Memory System](#the-memory-system)
  - [The Caching System](#the-caching-system)
  - [The Streaming & Observability System](#the-streaming--observability-system)
- [**9. Complete Example**](#9-complete-example)
- [**10. The `arc` CLI: Your Control Plane**](#10-the-arc-cli-your-control-plane)
- [**11. Platform Intelligence & Runtimes**](#11-platform-intelligence--runtimes)

---

## 1. The Aria Development Model

Aria's architecture is designed for clarity, power, and scalability. It is centered on two key files:

1.  **TypeScript Files (`.ts`) for Logic:** This is where you write the *how*. Your code implements the business logic for your tools and agents. You use simple decorators (`@tool`, `@agent`) that hold only the **required, structural** information.
2.  **`aria.toml` for Configuration:** This is where you define the *what* and *how it's tuned*. This file contains all **optional, tunable** parameters for your components and the global platform settings. It is your single source of truth for configuration.

---

## 2. Project Setup & Global Configuration

### The `arc new` Command

Start any project with `arc new <project-name>`. This generates a project with a comprehensive `aria.toml` file where all possible optional configurations are present but commented out, serving as a discoverable API surface.

### Global Configuration (`aria.toml`)

This file replaces the legacy `Symphony` initialization object. It controls the entire runtime environment.

```toml
# [project]
# Required. Global metadata for your project.
[project]
name = "my-aria-app"
version = "0.1.0"
description = "An advanced agentic system."
authors = ["Your Name <you@example.com>"] # Optional
license = "MIT"                           # Optional

# [llm]
# Optional. Default LLM settings for the entire project.
# Agents can override this with their own specific configuration.
[llm]
provider = "openai" # "openai", "anthropic", "google", etc.
model = "gpt-4o-mini"
api_key = "${env:OPENAI_API_KEY}" # Securely load secrets from environment variables
temperature = 0.7
# max_tokens = 2000
# top_p = 1.0

# [database]
# Optional. Configuration for the internal database.
[database]
enabled = true
adapter = "sqlite"
path = "./aria_storage.db" # Relative to project root

# [cache]
# Optional. Settings for the intelligent caching layer.
[cache]
# enabled = true
# ttl_sec = 3600 # Default cache TTL for all items
# enable_pattern_matching = true
# fast_path_threshold = 0.85

# [memory]
# Optional. Configuration for the agent memory system.
[memory]
# enabled = true

[memory.short_term]
# default_ttl_sec = 3600 # 1 hour
# max_size_mb = 100

[memory.long_term]
# default_ttl_sec = 2592000 # 30 days
# max_size_mb = 1024

# [streaming]
# Optional. Default settings for real-time observability streams.
[streaming]
# default_update_interval_ms = 100
```

---

## 3. Components: Tools

### Tool Concept
A Tool is a single, stateless TypeScript function that performs a specific, automatable task. Its input and output schema are automatically inferred from its TypeScript signature.

### The `@tool` Decorator API

The `@tool` decorator marks an exported function as a Tool.

| Parameter       | Location      | Type     | Required? | Description                                             |
| --------------- | ------------- | -------- | --------- | ------------------------------------------------------- |
| `description`   | `.ts`         | `string` | **Yes**   | High-level description of what the tool does.           |
| `name`          | Inferred      | `string` | **Yes**   | The tool's unique ID. Inferred from the function name.  |
| `parameters`    | Inferred      | `object` | **Yes**   | Inferred from the function's TypeScript parameters.     |

### Tool Configuration (`aria.toml`)

Create a section `[tool.<your-tool-name>]` to specify optional settings.

| Parameter     | Type      | Description                                                 |
| ------------- | --------- | ----------------------------------------------------------- |
| `type`        | `string`  | A category tag for organization (e.g., "data_processing").  |
| `nlp`         | `string`  | A natural language hint for the runtime's tool selection AI.|
| `timeout_ms`  | `integer` | Timeout in milliseconds for a single tool execution.        |
| `retry_count` | `integer` | Number of times to retry the tool on failure.               |
| `cache`       | `object`  | Override global cache settings for this specific tool.      |

**Cache Override Example:**
```toml
[tool.mySlowTool.cache]
enabled = true
ttl_sec = 7200 # Cache results for 2 hours
```

### Example: A Resilient Tool

**`src/main.ts`**
```typescript
import { tool } from '@aria/sdk';

@tool({
  description: "Fetches data from a potentially unreliable external API."
})
export async function fetchExternalData(params: { endpoint: string }): Promise<{ data: any }> {
  // Implementation...
  return { data: "live data" };
}
```

**`aria.toml`**
```toml
[tool.fetchExternalData]
type = "network"
timeout_ms = 5000  # 5-second timeout
retry_count = 3    # Retry 3 times
```

---

## 4. Components: Agents

### Agent Concept
An Agent is an intelligent class that leverages an LLM and a defined set of Tools to accomplish complex, multi-step tasks.

### The `@agent` Decorator API

The `@agent` decorator marks an exported class as an Agent.

| Parameter       | Location      | Type       | Required? | Description                                             |
| --------------- | ------------- | ---------- | --------- | ------------------------------------------------------- |
| `description`   | `.ts`         | `string`   | **Yes**   | High-level description of the agent's purpose.          |
| `task`          | `.ts`         | `string`   | **Yes**   | The default or guiding task for the agent.              |
| `tools`         | `.ts`         | `string[]` | **Yes**   | Array of tool names (string IDs) this agent can use.    |
| `name`          | Inferred      | `string`   | **Yes**   | The agent's unique ID. Inferred from the class name.    |

### Agent Configuration (`aria.toml`)

Create a section `[agent.<your-agent-name>]` to specify optional settings.

| Parameter       | Type       | Description                                                 |
| --------------- | ---------- | ----------------------------------------------------------- |
| `system_prompt` | `string`   | The primary instruction guiding the agent's reasoning.      |
| `directives`    | `string`   | Additional instructions appended to the system prompt.      |
| `capabilities`  | `string[]` | A list of descriptive tags for discovery or routing.        |
| `max_calls`     | `integer`  | Max LLM calls the agent can make in a single `.run()` invocation. |
| `timeout_ms`    | `integer`  | Overall timeout for a single agent run.                     |
| `llm`           | `object`   | Override the project's default LLM config for this agent.   |
| `memory`        | `object`   | Override the project's default memory settings.           |

**LLM Override Example:**
```toml
[agent.MyCreativeAgent.llm]
model = "gpt-4o"
temperature = 0.9 # Make this agent more creative
```

### Example: A Fine-Tuned Agent

**`src/main.ts`**
```typescript
import { agent } from '@aria/sdk';

@agent({
  description: "Writes marketing copy.",
  task: "Generate creative and compelling marketing copy based on a brief.",
  tools: ["webSearch"] // Can use the webSearch tool for inspiration
})
export class CopywriterAgent {
  public async run(task: string): Promise<string> {
    return `CopywriterAgent is ready to work on: "${task}"`;
  }
}
```

**`aria.toml`**
```toml
[agent.CopywriterAgent]
system_prompt = "You are a world-class marketing copywriter. Your tone is witty and engaging. You write for a modern, tech-savvy audience."
max_calls = 3

[agent.CopywriterAgent.llm]
model = "gpt-4o" # Use a more powerful model
temperature = 0.8
```

---

## 5. Components: Teams

### Team Concept
A Team is a group of Agents that collaborate to achieve a complex objective, coordinated by a defined strategy.

### The `@team` Decorator API

| Parameter       | Location      | Type       | Required? | Description                                          |
| --------------- | ------------- | ---------- | --------- | ---------------------------------------------------- |
| `description`   | `.ts`         | `string`   | **Yes**   | The team's overall purpose.                          |
| `agents`        | `.ts`         | `string[]` | **Yes**   | An array of agent names (string IDs) in the team.    |
| `name`          | Inferred      | `string`   | **Yes**   | The team's unique ID. Inferred from the class name.  |

### Team Configuration (`aria.toml`)

Create a section `[team.<your-team-name>]` to specify the collaboration strategy.

| Parameter     | Type      | Description                                                 |
| ------------- | --------- | ----------------------------------------------------------- |
| `strategy`    | `object`  | Defines how the agents in the team collaborate.             |

**Strategy Object:**
```toml
[team.MyTeam.strategy]
# "parallel": Agents work simultaneously on sub-tasks.
# "sequential": Agents work one after another.
# "hierarchical": A manager agent delegates tasks to others.
name = "hierarchical"
manager = "ManagerAgent" # Required for 'hierarchical' strategy

[team.MyTeam.strategy.coordinationRules]
max_parallel_tasks = 2
task_timeout_ms = 600000
```

---

## 6. Components: Pipelines (Advanced Workflows)

### Pipeline Concept
A Pipeline is a structured, multi-step workflow. It is the successor to the legacy "Tool Chain" concept, offering more power and flexibility. It can orchestrate Tools, Agents, and even Teams.

### The `@pipeline` Decorator API

The decorator is a simple marker, as all pipeline logic is declarative.

| Parameter     | Location      | Type     | Required? | Description                                          |
| ------------- | ------------- | -------- | --------- | ---------------------------------------------------- |
| `description` | `.ts`         | `string` | **Yes**   | A high-level description of the pipeline's goal.     |
| `name`        | Inferred      | `string` | **Yes**   | The pipeline's unique ID. Inferred from class name.  |

### Pipeline Configuration (`aria.toml`)

Create a section `[pipeline.<your-pipeline-name>]` to define the entire workflow.

```toml
[pipeline.MyPipeline]
# Default variables, can be overridden at runtime
[pipeline.MyPipeline.variables]
# topic = "AI in 2024"

# An array of steps to execute
[[pipeline.MyPipeline.steps]]
id = "research_step"
name = "Conduct Research"
type = "agent" # 'tool', 'agent', 'team', 'condition', 'parallel'
agent = "ResearchAgent"
inputs = { task_description = "Find information about '$topic'" } # Use a variable
outputs = { research_findings = ".result.analysis" } # Map output for use in later steps

[[pipeline.MyPipeline.steps]]
id = "write_report_step"
name = "Write Report"
type = "tool"
tool = "writeFile"
dependencies = ["research_step"] # Must run after research_step
inputs = { filename = "report.md", content = "@research_step.research_findings" } # Reference output

# Global error handling strategy for the pipeline
[pipeline.MyPipeline.errorStrategy]
type = "retry" # 'stop', 'continue', 'retry'
max_attempts = 2
```

---

## 7. The `arc` CLI: Your Control Plane

The `arc` CLI is your primary tool for managing your Aria project lifecycle. It is more than just a build tool; it's a complete control plane for development, testing, and observation.

| Command                               | Description                                                                                             |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `arc new <name>`                      | Create a new Aria project, scaffolding all necessary files including a fully-commented `aria.toml`.     |
| `arc check`                           | Statically analyze `.ts` files and `aria.toml` for errors. Verifies all references and configurations.    |
| `arc build`                           | Compile your project into a single, distributable `.aria` bundle.                                       |
| `arc run <component> [task/params]`   | Execute a specific component. Use `--params` for tools, and provide a task string for agents/teams.     |
| `arc run <component> --stream`        | Run a component and stream real-time observability events to your console as structured JSON.           |
| `arc db <subcommand>`                 | Interact directly with the project's internal SQLite database for advanced debugging.                   |
| `arc db query '<SQL>'`                | Execute a raw SQL query against the database.                                                           |
| `arc db recent-invocations <N>`       | View the `N` most recent component invocations logged in the database.                                  |
| `arc health`                          | Check the health of the Aria runtime and connected services (e.g., database connection).                |
| `arc stats`                           | Get detailed performance metrics for the runtime, including cache hit rates and average response times. |

---

## 8. Platform Intelligence & Runtimes

The legacy SDK's programmatic managers (e.g., `symphony.cache`, `symphony.memory`) are now powerful, automatic runtimes configured in `aria.toml`. The runtime is the invisible engine that brings your declarative configurations to life.

### The LLM Runtime

-   **Configuration:** `[llm]` and `[agent.*.llm]` sections in `aria.toml`.
-   **Functionality:**
    -   **Multi-Provider Support:** The runtime natively supports different LLM providers. You simply specify the `provider` and `model` in your configuration.
    -   **Automatic Prompt Engineering:** The runtime intelligently constructs the final prompt sent to the LLM. It combines the `system_prompt`, any `directives`, the user's task, and the schemas of the available tools into a single, optimized prompt.
    -   **JSON Mode Enforcement:** For any agent with one or more tools, the runtime automatically instructs the LLM to respond in a strict JSON format. It provides the LLM with a JSON schema that defines valid tool calls (`{ "tool_name": "...", "parameters": {...} }`) or a final answer (`{ "tool_name": "none", "response": "..." }`). This ensures reliable, parsable output without you needing to write complex JSON-handling logic.

### The Memory System

-   **Configuration:** `[memory]` section in `aria.toml`.
-   **Functionality:** The memory system provides agents with a sophisticated context mechanism, enabling them to recall past interactions and information.
    -   **Short-Term Memory:** This is session-based, volatile memory. The runtime automatically stores the history of the current conversation (user inputs and agent responses). This allows an agent to understand context like "what did you just say?" It is cleared when the `arc run` process ends.
    -   **Long-Term Memory:** This is persistent storage in the project's database. Your tools can write to this memory (e.g., using a `saveNote` tool). The runtime can then perform semantic searches over this memory, retrieving relevant information and injecting it into an agent's context. For example, if you ask an agent "What do you know about Project X?", the runtime can search the long-term memory for entries related to "Project X" and provide them to the agent.
    -   **Automatic Injection:** You do not need to manually manage memory retrieval. If memory is enabled, the runtime analyzes the user's task, searches for relevant long-term memories, and automatically includes them in the context provided to the LLM.

### The Caching System

-   **Configuration:** `[cache]` and `[tool.*.cache]` sections in `aria.toml`.
-   **Functionality:** The caching system is designed to dramatically improve performance and reduce costs by minimizing redundant computations.
    -   **Level 1: Simple Result Caching:** For any tool with caching enabled, the runtime will store the result of an execution. If the same tool is called again with the exact same parameters, the cached result is returned instantly without executing the tool's logic. This is controlled by `[tool.myTool.cache]`.
    -   **Level 2: Intelligent Pattern Matching:** When `enable_pattern_matching` is `true` in the global `[cache]` config, the runtime goes a step further. It analyzes the *intent* of a user's request. If a new request is semantically similar to a previous one that resulted in a specific tool call, the runtime can "fast-path" and suggest or execute that tool call directly, bypassing an initial LLM reasoning step.

### The Streaming & Observability System

-   **Configuration:** `[streaming]` section in `aria.toml`.
-   **Functionality:** This system powers the `--stream` flag. When enabled, the runtime emits a series of structured JSON events that allow you to observe the entire lifecycle of a task in real time. This is invaluable for debugging and for building rich UI frontends.
    -   **Event Types:**
        -   `{ "type": "status", "status": "thinking" | "executing_tool" | "responding" }`
        -   `{ "type": "log", "level": "info" | "warn", "message": "..." }`
        -   `{ "type": "tool_call", "tool_name": "...", "parameters": {...} }`
        -   `{ "type": "tool_result", "tool_name": "...", "result": {...} }`
        -   `{ "type": "final_response", "content": "..." }`
        -   `{ "type": "error", "message": "..." }`

---

## 9. Complete Example
This example ties everything together, showcasing a multi-tool agent with finely-tuned configuration.

### `src/main.ts`
```typescript
import { tool, agent } from '@aria/sdk';
import { promises as fs } from 'fs';

@tool({
  description: "Searches the web for a given query."
})
export async function webSearch(params: { query: string }): Promise<{ results: string[] }> {
  console.log(`Searching for: "${params.query}"`);
  return { results: [`Result for ${params.query}`] };
}

@tool({
  description: "Writes string content to a specified file."
})
export async function writeFile(params: { filename: string, content: string }): Promise<{ success: boolean }> {
  // Implementation...
  return { success: true };
}

@agent({
  description: "A research agent that can search the web and save findings to a file.",
  task: "Perform research and save the results.",
  tools: ["webSearch", "writeFile"]
})
export class ResearchAgent {
  public async run(task: string): Promise<string> {
    return `ResearchAgent is ready to perform task: "${task}"`;
  }
}
```

### `aria.toml`
```toml
[project]
name = "research-assistant"
version = "1.0.0"

[tool.webSearch]
timeout_ms = 10000
retry_count = 3

[agent.ResearchAgent]
system_prompt = "You are a world-class research assistant. Your goal is to find information using the webSearch tool and then synthesize your findings into a coherent report saved with the writeFile tool. Be thorough, precise, and cite your sources."
max_calls = 5
capabilities = ["web_research", "file_storage", "data_synthesis"]

[agent.ResearchAgent.llm]
provider = "openai"
model = "gpt-4o"
temperature = 0.2
```

---

## 10. Complete Example

This example ties everything together, showcasing a multi-tool agent with finely-tuned configuration.

### `src/main.ts`

```typescript
import { tool, agent } from '@aria/sdk';
import { promises as fs } from 'fs';

@tool({
  description: "Searches the web for a given query."
})
export async function webSearch(params: { query: string }): Promise<{ results: string[] }> {
  console.log(`Searching for: "${params.query}"`);
  return { results: [`Result for ${params.query}`] };
}

@tool({
  description: "Writes string content to a specified file."
})
export async function writeFile(params: { filename: string, content: string }): Promise<{ success: boolean }> {
  // Implementation...
  return { success: true };
}

@agent({
  description: "A research agent that can search the web and save findings to a file.",
  task: "Perform research and save the results.",
  tools: ["webSearch", "writeFile"]
})
export class ResearchAgent {
  public async run(task: string): Promise<string> {
    return `ResearchAgent is ready to perform task: "${task}"`;
  }
}
```

### `aria.toml`

```toml
[project]
name = "research-assistant"
version = "1.0.0"

[tool.webSearch]
timeout_ms = 10000
retry_count = 3

[agent.ResearchAgent]
system_prompt = "You are a world-class research assistant. Your goal is to find information using the webSearch tool and then synthesize your findings into a coherent report saved with the writeFile tool. Be thorough, precise, and cite your sources."
max_calls = 5
capabilities = ["web_research", "file_storage", "data_synthesis"]

[agent.ResearchAgent.llm]
provider = "openai"
model = "gpt-4o"
temperature = 0.2
``` 