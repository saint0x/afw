# a-fw (The Aria Firmware)

**Intelligent Container Orchestration Platform with Context Learning**

Aria Framework is a production-grade container orchestration platform that learns from execution patterns to provide intelligent container configuration recommendations and automated optimization.

## Architecture

### Core Components

- **AriaRuntime**: Primary execution engine with intelligent container management
- **Quilt**: Container runtime interface providing Docker-compatible operations
- **Intelligence Engine**: Pattern learning and context analysis system
- **Observability Manager**: Real-time metrics, logging, and event streaming
- **Tool Registry**: Extensible tool system for agent interactions

### Intelligence Layer

- **Pattern Learning**: Learns from container execution patterns to improve future recommendations
- **Context Trees**: Hierarchical execution context management with LRU caching
- **Workload Analytics**: Performance analysis and optimization recommendations
- **Agent Tools**: 7 intelligence tools for autonomous agent operations

## Features

- **Intelligent Container Selection**: Automatic image and configuration selection based on learned patterns
- **Context-Aware Execution**: Execution context building with priority-based filtering
- **Real-time Learning**: Continuous improvement from execution feedback
- **Pattern Optimization**: Automatic pruning of low-confidence patterns
- **Streaming Observability**: Real-time metrics and event streaming
- **Agent Integration**: Rich tool ecosystem for LLM agent interactions

## Quick Start

### Prerequisites

```bash
# Rust 1.70+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Docker (for container operations)
curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh
```

### Environment Setup

```bash
export OPENAI_API_KEY="your-openai-key"
export SERPER_API_KEY="your-serper-key"
```

### Build & Run

```bash
git clone <repo-url>
cd aria-fw
cargo build --release
cargo run
```

### Verification

```bash
cargo run --bin phase_4_intelligence_test
```

## API Endpoints

### Container Operations
- `POST /containers` - Create container with intelligent configuration
- `GET /containers` - List all containers
- `POST /containers/{id}/start` - Start container
- `POST /containers/{id}/exec` - Execute command in container
- `DELETE /containers/{id}` - Remove container

### Intelligence API
- `POST /intelligence/analyze` - Analyze container request for patterns
- `GET /intelligence/patterns` - Retrieve learned patterns
- `POST /intelligence/patterns/{id}/optimize` - Optimize specific pattern
- `GET /intelligence/context/{session_id}` - Get execution context tree
- `GET /intelligence/analytics` - Get learning analytics

### Observability
- `GET /metrics` - System metrics (Prometheus format)
- `GET /health` - Health check endpoint
- `GET /events/stream` - Server-sent events stream

## Configuration

### Database (SQLite)
```toml
[database]
path = "./data/aria.db"
max_connections = 10
```

### Intelligence
```toml
[intelligence]
pattern_confidence_threshold = 0.5
learning_rate = 0.05
max_context_depth = 5
cache_ttl_seconds = 3600
```

### Observability
```toml
[observability]
metrics_interval_seconds = 30
event_buffer_size = 1000
log_level = "info"
```

## Development

### Project Structure

```
aria-fw/
├── crates/
│   ├── aria_runtime/     # Core runtime engine
│   ├── quilt/           # Container runtime
│   ├── orc/             # Orchestrator
│   └── crypto/          # Cryptographic utilities
├── examples/            # Usage examples
└── tests/              # Integration tests
```

### Running Tests

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test '*'

# Intelligence system test
cargo run --bin phase_4_intelligence_test
```

### Key Interfaces

#### Container Management
```rust
use aria_runtime::engines::AriaEngines;

let engines = AriaEngines::new().await;
let config = engines.get_intelligent_container_config("python web server").await?;
let container = engines.container.create_container(config).await?;
```

#### Intelligence Tools
```rust
// Available for agent integration
let tools = engines.tool_registry.list_abstract_tools().await?;
// Returns: analyzeContainerPattern, getExecutionContext, optimizePatterns, etc.
```

#### Pattern Learning
```rust
let result = engines.intelligence
    .manager()
    .analyze_container_request(&request, "session-123")
    .await?;
```

## Intelligence Tools (Agent Integration)

| Tool | Description | Security |
|------|-------------|----------|
| `analyzeContainerPattern` | Pattern analysis for container requests | Safe |
| `getExecutionContext` | Retrieve execution context for decisions | Safe |
| `getContextForPrompt` | Format context for LLM prompts | Safe |
| `optimizePatterns` | Pattern optimization and pruning | Safe |
| `getLearningAnalytics` | Learning performance analytics | Safe |
| `analyzeSessionWorkloads` | Session workload analysis | Safe |
| `clearContextCache` | Cache management operations | Safe |

## Database Schema

### Intelligence Tables
- `container_patterns` - Learned container configuration patterns
- `execution_contexts` - Hierarchical execution contexts
- `learning_feedback` - Success/failure feedback for pattern learning
- `container_workloads` - Workload execution analytics
- `workload_analytics` - Performance metrics and insights

## Performance

- **Pattern Matching**: <50ms average latency
- **Context Building**: <100ms for complex trees
- **Memory Overhead**: <200MB for intelligence layer
- **Learning Convergence**: ~10 executions for pattern stabilization

## License

[Add your license here]

## Contributing

[Add contribution guidelines here] 