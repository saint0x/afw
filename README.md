# Aria Framework (AFW)

**Production-Grade Agent Execution Platform with Intelligent Container Orchestration**

Aria Framework is a comprehensive execution platform that enables AI agents to perform complex tasks through sophisticated tool orchestration, intelligent container management, and context-aware execution. Built for production workloads with enterprise-grade reliability.

## 🚀 Core Architecture

### **AriaRuntime** - Intelligent Agent Execution Engine
- **Multi-Tool Orchestration**: Complex task decomposition with parameter resolution
- **LLM Integration**: OpenAI provider with response caching and fallback strategies  
- **Context Management**: Intelligent memory management with execution history
- **Planning & Reflection**: Self-correcting execution with performance assessment
- **Agent Conversations**: Turn-based conversational flow with state management

### **Quilt** - Container Orchestration Platform
- **Unix Domain Sockets**: Production-grade gRPC over filesystem-based communication
- **Container Lifecycle**: Full CRUD operations (create, start, exec, stop, remove)
- **Network Topology**: Automated IP allocation and network management
- **Resource Management**: CPU, memory, and disk resource monitoring
- **Security**: Filesystem permissions with process isolation

### **Intelligence Engine** - Context Learning System  
- **Pattern Learning**: Learns from execution patterns for intelligent recommendations
- **Context Trees**: Hierarchical execution context with LRU caching
- **Workload Analytics**: Performance analysis and optimization suggestions
- **Agent Tools**: 7 specialized tools for autonomous agent operations

### **Database System** - SQLite-Based Persistence
- **User & Session Management**: Multi-user support with session tracking
- **Async Task Management**: Long-running task execution with progress tracking
- **Audit Logging**: Complete security audit trail for all operations
- **Migration System**: Versioned schema evolution with integrity checks

## ✨ Key Features

### **🤖 Agent Capabilities**
- **Sophisticated Planning**: LLM-powered execution plan generation with JSON schema validation
- **Deep Thinking**: 5 cognitive patterns (first principles, lateral, systems, dialectical, metacognitive)
- **Tool Orchestration**: Automatic multi-tool workflows with parameter resolution
- **Container Sovereignty**: Full control over container lifecycles through primitive tools
- **Context Awareness**: Execution history and session state management

### **📦 Container Operations**  
- **Intelligent Configuration**: Automatic image and configuration selection
- **Real-time Execution**: Non-blocking command execution with streaming output
- **Network Management**: Automated IP allocation with bridge interface setup
- **Resource Monitoring**: CPU, memory, and disk usage tracking
- **Production Stability**: Robust process management with graceful cleanup

### **🔧 Development Experience**
- **Unix Socket Infrastructure**: Zero network exposure with filesystem-based security
- **gRPC API**: Full container and task management over Unix domain sockets
- **CLI Integration**: Command-line tools for development and debugging
- **Real-time Metrics**: System monitoring with performance analytics
- **Comprehensive Logging**: Structured logging with multiple severity levels

## 🏗️ Quick Start

### Prerequisites

```bash
# Rust 1.70+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Docker (for container operations)  
curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh
```

### Environment Setup

```bash
export OPENAI_API_KEY="your-openai-api-key"
export SERPER_API_KEY="your-serper-api-key"  # For web search capabilities
```

### Build & Run

```bash
git clone <repository-url>
cd aria-fw
cargo build --release

# Start Quilt daemon (Unix socket server)
cd crates/quilt && cargo run --release --bin quilt &

# Verify Unix socket infrastructure
cargo run --bin unix_socket_infrastructure_test

# Test complete system integration
cargo run --bin phase_4_intelligence_test
```

## 📡 API Reference

### **Container Operations** (Unix Socket gRPC)
```bash
# Container lifecycle management
POST   /containers          # Create container with intelligent config
GET    /containers          # List all containers with status
POST   /containers/{id}/start    # Start container
POST   /containers/{id}/exec     # Execute command in container  
POST   /containers/{id}/stop     # Stop running container
DELETE /containers/{id}          # Remove container and cleanup resources

# System monitoring
GET    /metrics             # System resource metrics
GET    /network/topology    # Network topology and IP allocations
GET    /containers/{id}/logs     # Container execution logs
```

### **Agent Integration** (AriaRuntime)
```rust
use aria_runtime::AriaRuntime;

// Initialize runtime with configuration
let runtime = AriaRuntime::new(config).await?;

// Multi-tool orchestration
let result = runtime.execute_with_orchestration(&session_id, &task).await?;

// Container execution
let container_result = runtime.execute_container_workload(&workload).await?;

// Intelligence tools for agents
let tools = runtime.tool_registry.list_abstract_tools().await?;
```

### **Intelligence & Analytics**
```bash
# Pattern learning and context management
POST   /intelligence/analyze         # Analyze requests for patterns
GET    /intelligence/patterns        # Retrieve learned patterns  
GET    /intelligence/context/{id}    # Get execution context trees
GET    /intelligence/analytics       # Learning performance metrics
POST   /intelligence/optimize        # Pattern optimization
```

## 🛠️ Agent Tool Ecosystem

### **Cognitive Tools**
- `ponderTool` - Deep recursive thinking with 5 cognitive patterns
- `createPlanTool` - Sophisticated execution plan generation
- `webSearchTool` - Serper API integration with caching
- `parseDocumentTool` - LLM-powered document analysis

### **Container Primitive Tools**  
- `createContainer` - Create containers with intelligent configuration
- `startContainer` - Start container with network setup
- `execInContainer` - Execute commands with streaming output
- `getContainerStatus` - Real-time status and resource monitoring
- `listContainers` - Container inventory with filtering
- `stopContainer` / `removeContainer` - Lifecycle management

### **Intelligence Tools**
- `analyzeContainerPattern` - Pattern analysis for optimization
- `getExecutionContext` - Context trees for decision making  
- `optimizePatterns` - Pattern pruning and optimization
- `getLearningAnalytics` - Performance and learning metrics

### **File & Data Tools**
- `writeFileTool` / `readFileTool` - File system operations
- `writeCodeTool` - LLM-powered code generation
- `textAnalyzerTool` - Text processing and analysis

## 🏢 Production Features

### **Security & Isolation**
- **Unix Domain Sockets**: No network exposure, filesystem-based permissions
- **Process Isolation**: Container namespaces with resource limits
- **Secure Authentication**: Session tokens for container communication
- **Audit Logging**: Complete security audit trail

### **Reliability & Performance**  
- **Graceful Degradation**: Circuit breakers and fallback strategies
- **Resource Management**: Intelligent cleanup and resource recycling
- **Performance Monitoring**: Real-time metrics with alerting
- **Database Transactions**: ACID compliance with connection pooling

### **Observability**
- **Structured Logging**: Multiple severity levels with context
- **Real-time Metrics**: System and container resource monitoring  
- **Event Streaming**: Server-sent events for real-time updates
- **Health Checks**: Comprehensive system health monitoring

## 📊 System Specifications

### **Performance Benchmarks**
- **Unix Socket Latency**: ~90ms average for gRPC operations
- **Container Startup**: <2s for lightweight images
- **Pattern Matching**: <50ms for intelligent configuration
- **Memory Overhead**: <200MB for full intelligence layer
- **Tool Orchestration**: Multi-step workflows with <100ms planning

### **Scalability** 
- **Concurrent Containers**: Limited by system resources
- **Database Connections**: Configurable connection pooling
- **Session Management**: Multi-user with isolated contexts
- **Task Execution**: Async task management with progress tracking

## 🧪 Development & Testing

### **Project Structure**
```
aria-fw/
├── crates/
│   ├── aria_runtime/       # Core agent execution engine
│   ├── quilt/             # Container orchestration platform  
│   ├── orc/               # High-level orchestrator
│   ├── crypto/            # Cryptographic utilities
│   ├── hostfx/            # Host function extensions
│   ├── vec_cache/         # Vector caching system
│   └── telemetry/         # Metrics and observability
├── examples/              # Usage examples and demos
└── tests/                 # Integration test suites
```

### **Testing Commands**
```bash
# Core system tests
cargo test

# Unix socket infrastructure test  
cargo run --bin unix_socket_infrastructure_test

# Complete intelligence system test
cargo run --bin phase_4_intelligence_test

# Container diagnostic test
cargo run --bin container_diagnostic_test

# Multi-tool orchestration demo
cargo run --bin test_async_agent_tasks
```

### **Key Integration Points**
```rust
// Agent execution with full orchestration
let engines = AriaEngines::new().await?;
let result = engines.execute_with_context(&session_id, &task).await?;

// Container management with intelligence
let container_config = engines.intelligence.get_intelligent_config(&request).await?;
let container = engines.container.create_container(container_config).await?;

// Unix socket communication (for macOS desktop app)
let quilt_config = QuiltConfig { socket_path: "/run/quilt/api.sock".to_string() };
let client = QuiltService::new(&quilt_config).await?;
```

## 🎯 Use Cases

### **AI Agent Development**
- **Autonomous Task Execution**: Agents with full container control
- **Context-Aware Execution**: Session state and execution history
- **Intelligent Planning**: LLM-powered task decomposition 
- **Multi-Tool Workflows**: Complex orchestration with parameter resolution

### **Container Orchestration**
- **Development Environments**: Isolated development containers
- **Task Automation**: Long-running async task execution
- **Resource Management**: Intelligent resource allocation and cleanup
- **Network Management**: Automated IP allocation and routing

### **Desktop Integration** (macOS App Ready)
- **Unix Socket Communication**: Zero network configuration required
- **Filesystem Permissions**: Secure process isolation
- **Real-time Updates**: Event streaming for UI integration
- **Background Processing**: Async task management with progress tracking

## 📈 Roadmap

### **Current Status** ✅
- ✅ **Core Runtime**: Complete agent execution platform
- ✅ **Container Orchestration**: Full Unix socket infrastructure  
- ✅ **Intelligence Engine**: Pattern learning and context management
- ✅ **Database System**: Multi-user SQLite with async task management
- ✅ **Tool Ecosystem**: 20+ production-grade tools for agents

### **Upcoming Features** 🚧
- 🚧 **Bundle System**: Dynamic capability loading with `.aria` bundles
- 🚧 **Multi-Provider LLM**: Anthropic, local models, and edge inference
- 🚧 **Advanced Security**: Role-based access control and sandboxing
- 🚧 **Kubernetes Integration**: Cloud-native deployment patterns
- 🚧 **Streaming Protocols**: WebSocket and Server-Sent Events

## 📄 License

[Add your license here]

## 🤝 Contributing  

[Add contribution guidelines here]

---

**Aria Framework**: Where intelligent agents meet production-grade infrastructure. 🚀 