# Hermetic Container Runtime: Architectural Principles & Implementation Plan

> **Core Principle: Absolute Isolation. Zero Trust.**
>
> The architectural rule is simple and uncompromising: **Every non-conversational action is a containerized workload.**
>
> The runtime's primary directive is to instantiate a new, purpose-built, single-use container, execute a task hermetically inside it, and then destroy the container. The host environment is sacred and is never touched directly by agentic logic. The `ExecutionEngine` acts as a high-security kernel, translating abstract *intent* into sandboxed *reality*.

---

## The Two-Tiered Tool Architecture

We will partition the tool ecosystem into two distinct scopes to ensure a separation of concerns between high-level reasoning and low-level system control.

### 1. `ToolScope::Abstract` (The "What")
These are the high-level tools the agent uses for planning. They represent capabilities or intents, not specific implementations. These tool definitions are declarative schemas and contain no execution logic themselves.
- **Examples**: `writeFile`, `runPythonScript`, `analyzeImage`.
- **Execution**: When the `ExecutionEngine` receives a plan with an `Abstract` tool, it invokes an internal "Realizer" that translates the single abstract step into a sequence of `Primitive` tool calls to achieve the goal in a hermetic environment (e.g., `create` -> `exec` -> `remove`).

### 2. `ToolScope::Primitive` (The "How")
This is the exhaustive, low-level suite of tools that provides direct, programmatic control over the `quilt` subsystem. They are the physical building blocks of the runtime, making direct gRPC calls to the `quiltd` daemon.
- **Agent Control**: The agent has full, explicit control over the container lifecycle. For complex workflows requiring persistent state (e.g., running a database), the agent can and should include these `Primitive` tools directly in its plan to create, manage, and ultimately destroy containers as it sees fit.
- **Security**: These tools are not exposed for general-purpose reasoning unless an agent is explicitly trusted and configured to use them.

---

## Implementation Checklist

### Phase 1: Foundational Tooling & Dependency Injection
- [ ] **Architecture:** Create new module `tools::container`.
- [ ] **DI:** Refactor `AriaEngines` to instantiate `QuiltService` and inject it into the `ToolRegistry`.
- [ ] **DI:** Refactor `ToolRegistry` to accept and hold the `ContainerManager` dependency.
- [ ] **Tool Registration:** Add a new `register_container_tools` function to the `ToolRegistry`.

### Phase 2: Core Lifecycle & Execution Primitives
- [ ] **`createContainer`**: `tools::container::create.rs`
- [ ] **`startContainer`**: `tools::container::start.rs`
- [ ] **`stopContainer`**: `tools::container::stop.rs`
- [ ] **`removeContainer`**: `tools::container::remove.rs`
- [ ] **`execInContainer`**: `tools::container::exec.rs`

### Phase 3: Introspection & Monitoring Primitives
- [ ] **`listContainers`**: `tools::container::list.rs`
- [ ] **`getContainerStatus`**: `tools::container::status.rs`
- [ ] **`getContainerLogs`**: `tools::container::logs.rs`
- [ ] **`getSystemMetrics`**: `tools::container::metrics.rs`

### Phase 4: Networking & Service Mesh Primitives
- [ ] **`getNetworkTopology`**: `tools::container::network_topology.rs`
- [ ] **`getContainerNetworkInfo`**: `tools::container::network_info.rs`
- [ ] **`establishConnection`**: `tools::container::network_connect.rs`
- [ ] **`severConnection`**: `tools::container::network_disconnect.rs`

### Phase 5: Runtime Integration
- [ ] **`ExecutionEngine`:** Implement the "Realizer" logic for core `Abstract` tools like `writeFile`.
- [ ] **Agent Empowerment:** Update test scripts to grant the agent access to `Primitive` tools for validation.
- [ ] **System Prompts:** Develop a system prompt that clearly explains the `Primitive` tool suite to the agent, framing it as its native control panel for the container subsystem. 