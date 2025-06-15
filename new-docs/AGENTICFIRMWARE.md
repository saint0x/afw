# Agentic Firmware: Architecture and Workflow

This document outlines the architecture of the agentic firmware environment and the role of the Containerism runtime within it. It is designed to support a flexible and high-performance system for executing user-defined code ("tools") in isolated environments.

## 1. Overall Vision & Workflow

The core concept is to enable users to write decorator-based TypeScript code, compile it into `.aria` bundles, and deploy it to a cloud-based (or potentially bare-metal) firmware instance. This firmware is equipped with an intelligent agent capable of executing these user-defined "tools" dynamically.

The typical workflow is envisioned as follows:

1.  **Development**: A user writes decorator-based TypeScript code defining tools, agents, teams, and pipelines.
2.  **Compilation**: The TypeScript is parsed by Arc (Aria Compiler) to extract configurations and package them into `.aria` bundles.
3.  **Deployment**: The `.aria` bundle is uploaded (via gRPC) to the Aria Firmware, which can either execute it in isolation or extract components into global registries.
4.  **Tasking the Agent**: An external system (or user interaction) tasks the agent residing within a specific firmware instance to execute a particular tool (which was previously deployed). This tasking likely involves specifying the tool name and any necessary input parameters.
5.  **Tool/Agent Execution**:
    *   The agent on the firmware receives the execution request.
    *   To run the code in a secure, isolated environment, the agent utilizes **Quilt** for container orchestration.
    *   The agent requests Quilt to spin up a new container with Bun runtime, configured with the necessary environment and resource tokens.
    *   The TypeScript code from the .aria bundle is executed directly within this container.
6.  **Result Handling**: The output or result from the execution within the container is captured by Quilt and passed back to the Aria Runtime. The orchestrator then forwards this result to the original requester or takes further action.

The primary goal is to create an **extremely lightweight, extremely fast, and flexible runtime environment** where the agent can dynamically execute TypeScript code with necessary dependencies, all managed securely and efficiently by Quilt's sync-engine.

## 2. Key Components within the Firmware

The firmware itself runs on a lightweight Linux distribution (e.g., Alpine Linux). Key components include:

### 2.1. Aria Runtime

*   **Role**: The central orchestration component residing on the firmware. It's responsible for receiving .aria bundles, managing tool/agent registries, coordinating execution, and interfacing with Quilt for container management.
*   **Implementation**: Rust application built with 7 core crates for performance and safety.
*   **Communication**:
    *   Receives .aria bundles and execution requests via gRPC/WebSocket APIs.
    *   Communicates with Quilt sync-engine via **Unix Domain Sockets** for resource token management and container orchestration.

### 2.2. Quilt Sync-Engine

*   **Role**: External orchestration system that provides resource token management and container lifecycle control. Manages resource contention and scheduling across the cluster.
*   **Implementation**: Separate service that Aria Firmware interfaces with through its token_api crate.
*   **Responsibilities**:
    *   Managing resource tokens (KV, vector, LLM, network access, etc.).
    *   Container lifecycle management with proper isolation.
    *   Resource scheduling and deadlock detection.
    *   Enforcing security policies through cgroups and namespaces.
    *   Event notification for resource availability and completion.
*   **Communication**:
    *   Exposes its API via **Unix Domain Sockets** to Aria Runtime.
    *   Handles resource coordination and container management autonomously.

### 2.3. Arc CLI (Aria Compiler)

*   **Role**: Command-line tool that compiles decorator-based TypeScript into .aria bundles and manages deployment.
*   **Implementation**: TypeScript/Node.js application.
*   **Functionality**: Provides commands for the complete development lifecycle:
    *   `arc new` - Create new .aria projects with templates.
    *   `arc build` - Parse decorators and package into .aria bundles.
    *   `arc upload` - Deploy bundles to Aria Firmware instances.
    *   `arc publish/install` - Manage bundle sharing and dependencies.
*   **Communication**: Interacts with Aria Firmware via gRPC for bundle deployment and management.

### 2.4. Bun Execution Containers

*   **Nature**: Lightweight, ephemeral execution environments managed by Quilt with pre-installed Bun runtime.
*   **Isolation**: Isolated from the host system and from each other using Linux namespaces and cgroups enforced by Quilt.
*   **Purpose**: To execute TypeScript code from .aria bundles with proper resource token constraints and security isolation.
*   **Content**: Minimal container image with Bun runtime pre-installed, allowing direct execution of TypeScript without compilation overhead. Containers receive the source code and manifest from .aria bundles at runtime.

## 3. Interaction Model: Aria Runtime and Quilt

The primary interaction flow for .aria bundle execution is:

1.  Aria Runtime receives a request to execute `Agent_X` with specific parameters.
2.  Runtime determines the resource tokens required from the .aria bundle manifest (e.g., `kv:orders:rw`, `llm:openai:gpt4`).
3.  Runtime makes a token request to Quilt via `token_api`:
    *   Specifies required resource tokens from the manifest.
    *   Includes the TypeScript source code and execution context.
    *   Defines security constraints and resource limits.
4.  Quilt sync-engine:
    *   Validates and schedules the requested resource tokens.
    *   Creates a new Bun container with appropriate isolation.
    *   Provides the container with access to approved resources.
    *   Streams execution logs and results back to Aria Runtime.
    *   Monitors resource usage and enforces limits.
5.  Aria Runtime receives status updates and final results from Quilt.
6.  Upon completion, Quilt automatically cleans up the ephemeral container and releases resource tokens for other requests.

## 4. Motivation for Aria Firmware Architecture

This architecture is designed to provide a specialized runtime environment optimized for agentic workloads. By leveraging Quilt for orchestration and focusing on TypeScript/Bun execution, we achieve several key advantages over general-purpose container platforms.

Key drivers include:

*   **Developer Experience**: Decorator-based TypeScript provides familiar syntax with powerful agentic primitives.
*   **Performance**: Bun runtime offers fast TypeScript execution without compilation overhead.
*   **Resource Management**: Quilt's token-based system prevents resource contention and deadlocks.
*   **Composability**: .aria bundles can be shared, reused, and composed into larger systems.
*   **Security**: Fine-grained resource tokens and container isolation provide robust security boundaries.
*   **Growing Intelligence**: Firmware becomes more capable over time as bundles populate the tool/agent registries.

This architecture enables a new model of agentic computing where capabilities accumulate and compose naturally, creating increasingly intelligent systems through incremental deployment of specialized .aria bundles. 