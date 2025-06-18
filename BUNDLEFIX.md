# Bundle Integration Implementation Checklist

## Overview

Implementation of .aria bundle capabilities with two interaction modes:
1. **Tool Registry Injection** (Lightweight) - Agent self-discovery of custom tools
2. **Full Bundle Execution** (Heavy) - Complete bundle deployment and execution

## Current Status: 90% Foundation Complete ✅

### Already Implemented
- ✅ **Arc Compiler (`ar-c`)**: TypeScript AST parsing with decorator extraction
- ✅ **Bundle Format**: ZIP structure with manifest.json, source files, package.json
- ✅ **Manifest Schema**: Complete metadata for tools, agents, teams, pipelines
- ✅ **Package Store**: Content-addressed storage with Blake3 hashing
- ✅ **Tool Registry**: Dynamic registration infrastructure
- ✅ **Container Infrastructure**: Unix socket gRPC + ICC bridge
- ✅ **Bundle Loading**: `AriaBundle::load_from_file()` with manifest parsing

## Mode 1: Tool Registry Injection ⚡ (Agent Self-Discovery)

### Use Case
Agent encounters unknown custom tool → automatically discovers from bundles → registers → executes

### Implementation Tasks

#### 1. Bundle Tool Discovery Service
- [ ] **File**: `crates/aria_runtime/src/bundle_discovery.rs`
- [ ] `BundleToolDiscovery::new(pkg_store, tool_registry)`
- [ ] `discover_tool(&self, tool_name: &str) -> Option<ToolManifest>`
- [ ] `scan_available_bundles(&self) -> Vec<(bundle_hash, tool_names)>`
- [ ] Cache bundle manifests for fast tool lookup

#### 2. Tool Registry Integration
- [ ] **File**: `crates/aria_runtime/src/engines/tool_registry/bundle_integration.rs`
- [ ] `register_tool_from_manifest(&mut self, tool: &ToolManifest, bundle_hash: &str)`
- [ ] `create_registry_entry_from_manifest(&self, tool: &ToolManifest) -> RegistryEntry`
- [ ] `extract_tool_capabilities(&self, tool: &ToolManifest) -> Vec<String>`
- [ ] `generate_tool_description_for_llm(&self, tool: &ToolManifest) -> String`

#### 3. Agent Tool Auto-Discovery
- [ ] **File**: `crates/aria_runtime/src/engines/execution/tool_resolver.rs`
- [ ] `resolve_unknown_tool(&self, tool_name: &str) -> AriaResult<Option<RegistryEntry>>`
- [ ] `auto_register_discovered_tool(&mut self, tool_name: &str) -> AriaResult<bool>`
- [ ] Integration with `ExecutionEngine::execute_tool_call()`

#### 4. Custom Tool Management API
- [ ] **File**: `crates/aria_runtime/src/tools/management/custom_tools.rs`
- [ ] `list_custom_tools(&self) -> Vec<CustomToolEntry>`
- [ ] `remove_custom_tool(&mut self, tool_name: &str) -> AriaResult<()>`
- [ ] `get_tool_source_info(&self, tool_name: &str) -> Option<ToolSourceInfo>`

## Mode 2: Full Bundle Execution 🚀 (Complete Deployment)

### Use Case
Execute complete .aria bundle as containerized application with agents, teams, pipelines

### Implementation Tasks

#### 1. Bundle Executor Service
- [ ] **File**: `crates/aria_runtime/src/bundle_executor.rs`
- [ ] `BundleExecutor::new(quilt_service, pkg_store, tool_registry)`
- [ ] `execute_bundle(&self, bundle_hash: &str, session_id: DeepUuid) -> AriaResult<ExecutionResult>`
- [ ] `validate_bundle_for_execution(&self, bundle: &AriaBundle) -> AriaResult<()>`

#### 2. Container Bundle Workspace
- [ ] **File**: `crates/quilt/src/bundle_runtime.rs`
- [ ] `mount_bundle_workspace(&self, container_id: &str, bundle: &AriaBundle) -> AriaResult<()>`
- [ ] `create_bundle_environment(&self, bundle: &AriaBundle, session_id: DeepUuid) -> AriaResult<HashMap<String, String>>`
- [ ] `extract_bundle_to_workspace(&self, bundle_data: &[u8], workspace_path: &Path) -> AriaResult<()>`

#### 3. Bun Runtime Controller
- [ ] **File**: `crates/quilt/src/bun_controller.rs`
- [ ] `install_dependencies(&self, container_id: &str, package_json_path: &str) -> AriaResult<()>`
- [ ] `execute_main_entry(&self, container_id: &str, entry_path: &str) -> AriaResult<ExecutionResult>`
- [ ] `configure_bun_environment(&self, container_id: &str, env_vars: HashMap<String, String>) -> AriaResult<()>`

#### 4. Bundle Component Registration
- [ ] **File**: `crates/aria_runtime/src/bundle_registration.rs`
- [ ] `register_bundle_components(&self, bundle: &AriaBundle, container_id: &str) -> AriaResult<()>`
- [ ] `register_bundle_tools(&self, tools: &[ToolManifest], bundle_hash: &str) -> AriaResult<()>`
- [ ] `register_bundle_agents(&self, agents: &[AgentManifest], bundle_hash: &str) -> AriaResult<()>`
- [ ] `register_bundle_teams(&self, teams: &[TeamManifest], bundle_hash: &str) -> AriaResult<()>`

## Integration Points 🔗

### 1. AriaRuntime API Extensions
- [ ] **File**: `crates/aria_runtime/src/lib.rs`
- [ ] `register_tools_from_bundle(&self, bundle_hash: &str) -> AriaResult<Vec<String>>`
- [ ] `execute_bundle_workload(&self, bundle_hash: &str, session_id: DeepUuid) -> AriaResult<ExecutionResult>`
- [ ] `discover_tool_in_bundles(&self, tool_name: &str) -> AriaResult<Option<String>>`

### 2. gRPC API Extensions
- [ ] **File**: `crates/quilt/proto/quilt.proto`
- [ ] `MountBundleWorkspace` RPC method
- [ ] `ExecuteBundleWorkload` RPC method  
- [ ] `BundleExecutionStatus` message types

### 3. Tool Registry Bundle Awareness
- [ ] **File**: `crates/aria_runtime/src/engines/tool_registry/mod.rs`
- [ ] `register_from_bundle_manifest(&mut self, tool: &ToolManifest, bundle_hash: &str) -> AriaResult<()>`
- [ ] `get_tool_bundle_source(&self, tool_name: &str) -> Option<String>`
- [ ] `remove_bundle_tools(&mut self, bundle_hash: &str) -> AriaResult<Vec<String>>`

## Data Structures & Types 📊

### 1. Bundle Tool Discovery
- [ ] **File**: `crates/aria_runtime/src/types/bundle_types.rs`
```rust
pub struct CustomToolEntry {
    pub name: String,
    pub description: String,
    pub bundle_hash: String,
    pub bundle_name: String,
    pub registered_at: DateTime<Utc>,
}

pub struct ToolSourceInfo {
    pub bundle_hash: String,
    pub bundle_name: String,
    pub bundle_version: String,
    pub tool_manifest: ToolManifest,
}

pub struct BundleExecutionResult {
    pub execution_id: String,
    pub bundle_hash: String,
    pub container_id: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub execution_time: Duration,
}
```

### 2. Bundle Registry Cache
- [ ] **File**: `crates/aria_runtime/src/cache/bundle_cache.rs`
```rust
pub struct BundleManifestCache {
    manifests: HashMap<String, AriaManifest>, // bundle_hash -> manifest
    tool_index: HashMap<String, Vec<String>>, // tool_name -> [bundle_hashes]
}
```

## Configuration & Environment 🔧

### 1. Bundle Execution Config
- [ ] **File**: `crates/aria_runtime/src/config/bundle_config.rs`
```rust
pub struct BundleExecutionConfig {
    pub default_memory_mb: u64,
    pub default_timeout_secs: u64,
    pub workspace_base_path: PathBuf,
    pub enable_auto_tool_discovery: bool,
    pub max_concurrent_bundles: usize,
}
```

### 2. Container Environment Templates
- [ ] **File**: `crates/quilt/src/templates/bundle_env.rs`
- [ ] Default environment variables for bundle execution
- [ ] ICC connection configuration
- [ ] Aria runtime API endpoints

## Testing 🧪

### 1. Bundle Tool Discovery Tests
- [ ] **File**: `tests/bundle_tool_discovery_test.rs`
- [ ] Test auto-discovery of unknown tools
- [ ] Test tool registration from bundles
- [ ] Test tool removal and cleanup

### 2. Bundle Execution Tests  
- [ ] **File**: `tests/bundle_execution_test.rs`
- [ ] Test complete bundle deployment
- [ ] Test Bun dependency installation
- [ ] Test main.tsx execution
- [ ] Test bundle environment configuration

### 3. Integration Tests
- [ ] **File**: `tests/bundle_integration_test.rs`
- [ ] Test Mode 1 → Mode 2 workflow
- [ ] Test multiple bundles with shared tools
- [ ] Test bundle updates and versioning

## Documentation 📚

### 1. Bundle API Documentation
- [ ] **File**: `docs/BUNDLE_API.md`
- [ ] Tool discovery API reference
- [ ] Bundle execution API reference
- [ ] Configuration options

### 2. Bundle Development Guide
- [ ] **File**: `docs/BUNDLE_DEVELOPMENT.md`
- [ ] How to create .aria bundles
- [ ] Best practices for tool design
- [ ] Container environment guidelines

## Performance & Optimization ⚡

### 1. Bundle Manifest Caching
- [ ] In-memory cache for frequently accessed manifests
- [ ] LRU eviction for memory management
- [ ] Cache warming on startup

### 2. Tool Discovery Optimization
- [ ] Pre-index all bundle tools on startup
- [ ] Lazy loading of bundle contents
- [ ] Tool resolution priority (registry → bundles)

### 3. Container Resource Management
- [ ] Container pooling for bundle execution
- [ ] Resource limit enforcement
- [ ] Cleanup and garbage collection

## Security & Validation 🔒

### 1. Bundle Validation
- [ ] Signature verification for bundles
- [ ] Manifest schema validation
- [ ] Tool capability validation

### 2. Execution Sandboxing
- [ ] Container resource limits
- [ ] Network access restrictions
- [ ] File system isolation

## Monitoring & Observability 📊

### 1. Bundle Metrics
- [ ] Tool discovery success/failure rates
- [ ] Bundle execution performance
- [ ] Container resource usage

### 2. Tool Registry Analytics
- [ ] Most frequently discovered tools
- [ ] Bundle tool usage statistics
- [ ] Tool registration/removal events

---

## Implementation Timeline 📅

### Week 1: Mode 1 Implementation
- [ ] Bundle tool discovery service
- [ ] Tool registry integration
- [ ] Agent auto-discovery
- [ ] Custom tool management API

### Week 2: Mode 2 Implementation  
- [ ] Bundle executor service
- [ ] Container workspace mounting
- [ ] Bun runtime controller
- [ ] Component registration

### Week 3: Integration & Testing
- [ ] AriaRuntime API extensions
- [ ] gRPC protocol updates
- [ ] Comprehensive testing
- [ ] Performance optimization

### Week 4: Documentation & Polish
- [ ] API documentation
- [ ] Development guides
- [ ] Monitoring integration
- [ ] Security hardening

---

**Total Estimated Effort**: 3-4 weeks
**Priority**: High (Core platform capability)
**Dependencies**: Current firmware infrastructure (already complete) 