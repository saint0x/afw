# Integration TODO - Aria Runtime gRPC API

## ✅ COMPLETED TASKS

### Core gRPC API Implementation
- ✅ **Complete gRPC Service Implementation** - All 5 services fully implemented
  - ✅ TaskService - Task management with Quilt integration
  - ✅ SessionService - Conversation management with tool execution
  - ✅ ContainerService - Container lifecycle management
  - ✅ NotificationService - Real-time notification system
  - ✅ BundleService - Bundle upload with streaming support

- ✅ **Main gRPC Server Binary** - Created `aria_server` binary
  - ✅ Unix Domain Socket server on `/run/aria/api.sock`
  - ✅ Proper service registration and initialization
  - ✅ Database and Quilt daemon integration
  - ✅ Graceful error handling and logging

- ✅ **Protobuf Integration** - Complete protobuf definitions and compilation
  - ✅ Comprehensive `aria.proto` with all service definitions
  - ✅ Proper build.rs configuration for protobuf compilation
  - ✅ Type conversions between Aria and Quilt proto types

- ✅ **Database Integration** - Enhanced database support
  - ✅ Added helper methods to DatabaseManager
  - ✅ Enhanced AriaError with database-specific error types
  - ✅ Proper async database operations in all services

- ✅ **Bundle Upload via gRPC** - Streaming bundle upload implementation
  - ✅ Streaming upload support with progress tracking
  - ✅ Integration with Quilt daemon for bundle processing
  - ✅ Tool registry integration for bundle tool registration

- ✅ **Test Infrastructure** - Basic testing framework
  - ✅ Created `test_server` binary for API testing
  - ✅ Connection testing for all gRPC services
  - ✅ Basic functional verification

## 🔄 IN PROGRESS TASKS

### Database Schema Enhancement
- 🔄 **Add Missing Database Tables** - Need to create additional tables
  - ⏳ Sessions table with proper foreign keys
  - ⏳ Messages table for conversation history
  - ⏳ Notifications table for real-time events
  - ⏳ Tool execution logs table

### Error Handling & Logging
- 🔄 **Enhanced Error Handling** - Improve error propagation
  - ⏳ Structured error responses in all services
  - ⏳ Proper error logging with context
  - ⏳ Client-friendly error messages

## 📋 REMAINING TASKS

### Core API Features
1. **Task Listing and Discovery** - ⚠️ Partially Complete
   - ✅ Basic task listing via TaskService
   - ⏳ Task filtering by status, session, user
   - ⏳ Task search and pagination
   - ⏳ Task metadata and execution history

2. **Agent and Tool Discovery** - ⚠️ Needs Implementation
   - ⏳ List available tools from registry
   - ⏳ Tool capability and parameter discovery
   - ⏳ Agent discovery and metadata
   - ⏳ Tool execution permissions and validation

3. **Settings Persistence** - ⚠️ Needs Implementation
   - ⏳ User preferences storage
   - ⏳ System configuration management
   - ⏳ Runtime settings API
   - ⏳ Configuration validation

### Advanced Features
4. **Real-time Streaming** - ⚠️ Partially Complete
   - ✅ Basic streaming infrastructure in NotificationService
   - ⏳ Task output streaming with real-time updates
   - ⏳ Session conversation streaming
   - ⏳ Container log streaming improvements

5. **Authentication & Authorization** - ⚠️ Needs Implementation
   - ⏳ User authentication system
   - ⏳ Session-based authorization
   - ⏳ Tool execution permissions
   - ⏳ Resource access control

6. **Metrics & Observability** - ⚠️ Needs Integration
   - ⏳ gRPC metrics collection
   - ⏳ Performance monitoring
   - ⏳ Health check endpoints
   - ⏳ Request tracing and debugging

### Integration & Testing
7. **End-to-End Testing** - ⚠️ Basic Framework Ready
   - ✅ Basic connection testing
   - ⏳ Full workflow testing (session → task → container)
   - ⏳ Bundle upload and tool registration testing
   - ⏳ Error scenario testing

8. **Performance Optimization** - ⚠️ Needs Implementation
   - ⏳ Connection pooling for database
   - ⏳ gRPC connection management
   - ⏳ Streaming optimization
   - ⏳ Memory usage optimization

## 🏗️ ARCHITECTURE NOTES

### Current Status
- **gRPC Server**: ✅ Fully functional with Unix Domain Socket
- **Service Layer**: ✅ All 5 core services implemented
- **Database Layer**: ✅ Basic integration, needs schema expansion
- **Quilt Integration**: ✅ Complete container and bundle management
- **Tool Registry**: ✅ Integrated with session and bundle services
- **Error Handling**: ✅ Basic structure, needs enhancement

### Next Priority Actions
1. **Database Schema** - Create missing tables for full functionality
2. **End-to-End Testing** - Comprehensive workflow testing
3. **Tool Discovery API** - Complete tool registry integration
4. **Real-time Streaming** - Enhanced streaming capabilities
5. **Authentication** - User and session management

## 🚀 DEPLOYMENT READINESS

### Ready for Testing
- ✅ gRPC server can be started with `cargo run --bin aria_server`
- ✅ Basic API testing with `cargo run --bin test_server`
- ✅ Container operations via ContainerService
- ✅ Bundle upload via BundleService
- ✅ Session management via SessionService

### Prerequisites for Production
- ⏳ Complete database schema implementation
- ⏳ Comprehensive error handling
- ⏳ Authentication and authorization
- ⏳ Performance testing and optimization
- ⏳ Monitoring and observability integration

---

**Last Updated**: January 2025  
**Status**: Core gRPC API implementation complete, database and advanced features in progress

# Aria Runtime API Integration - TODO

This document outlines the necessary engineering tasks to close the gaps between the current implementation of the Aria Runtime and the requirements for a robust, client-facing gRPC API. These actions are derived from the architectural review of `APICONTRACT.md` and `INTEGRATION.json`.

---

## 1. Implement Task Listing and Discovery

**Gap:** The `TaskService` provides no mechanism to list or discover existing tasks. A client cannot build a "Task List" view without this.

**Action:** Add a `ListTasks` RPC to the `TaskService`.

### Protobuf Definition (`task_service.proto`)

```protobuf
// ... existing TaskService definition
service TaskService {
    // ... existing RPCs
    
    // Lists tasks, with support for filtering and pagination.
    rpc ListTasks(ListTasksRequest) returns (ListTasksResponse);
}

// ... existing message definitions

message ListTasksRequest {
    // Optional: Filter tasks by the session that created them.
    optional string session_id = 1; 
    
    // Optional: Filter by one or more statuses.
    repeated TaskStatus filter_by_status = 2;

    // The maximum number of tasks to return.
    int32 page_size = 3;

    // A page token, received from a previous `ListTasks` call.
    string page_token = 4;
}

message ListTasksResponse {
    // The list of tasks found.
    repeated Task tasks = 1;

    // A token to retrieve the next page of results. If this field is
    // empty, there are no more results.
    string next_page_token = 2;
}
```

### Implementation Notes
- The backend implementation must support pagination (cursor-based, not offset) for scalability.
- Filtering logic should be implemented at the database or state-store level for performance.

---

## 2. Expose Bundle Upload via a Public API Service

**Gap:** The API contract does not define how a client should upload an `.aria` bundle. The existing `ar-c` implementation talks to a lower-level `Quilt` daemon, which should be an internal detail.

**Action:** Create a new `BundleService` within the public `aria.v1` API to proxy uploads to the internal `Quilt` service. This maintains a clean architectural boundary.

### Protobuf Definition (new file: `bundle_service.proto`)
```protobuf
// aria/v1/bundle_service.proto
syntax = "proto3";

package aria.v1;

// Service for managing and deploying .aria bundles.
service BundleService {
    // Uploads a bundle to the runtime via a client-side stream.
    // The first message in the stream must be a `Metadata` message.
    // All subsequent messages must be `Chunk` messages.
    rpc UploadBundle(stream UploadBundleRequest) returns (UploadBundleResponse);
}

message UploadBundleRequest {
    oneof payload {
        BundleMetadata metadata = 1;
        bytes chunk = 2;
    }
}

message BundleMetadata {
    string name = 1; // e.g., "weather-agent.aria"
    uint64 total_size_bytes = 2;
    // Future: string sha256_checksum = 3;
}

message UploadBundleResponse {
    string bundle_id = 1;
    bool success = 2;
    optional string error_message = 3;
}
```
### Implementation Notes
-   The `aria_runtime`'s `BundleService` implementation will receive the client stream.
-   It will then establish its own gRPC stream to the internal `Quilt` daemon, effectively streaming the data through.
-   This decouples the client from the internal service architecture.

---

## 3. Implement Agent and Tool Discovery

**Gap:** The API provides no way for a client to discover which agents or tools are available, what their capabilities are, or what their input schemas are.

**Action:** Design and implement a new `RegistryService` for capability discovery.

### Protobuf Definition (new file: `registry_service.proto`)
```protobuf
// aria/v1/registry_service.proto
syntax = "proto3";

package aria.v1;

// Service for discovering available agents and tools.
service RegistryService {
    // Lists all available agents registered in the runtime.
    rpc ListAgents(ListAgentsRequest) returns (ListAgentsResponse);

    // Gets detailed information about a specific agent, including its tools.
    rpc GetAgentDetails(GetAgentDetailsRequest) returns (AgentDefinition);
}

message ListAgentsRequest {}

message ListAgentsResponse {
    repeated AgentSummary agents = 1;
}

message AgentSummary {
    string agent_id = 1;
    string name = 2;
    string description = 3;
}

message GetAgentDetailsRequest {
    string agent_id = 1;
}

message AgentDefinition {
    string agent_id = 1;
    string name = 2;
    string description = 3;
    repeated ToolDefinition tools = 4;
}

message ToolDefinition {
    string name = 1; // The tool's name, e.g., "getCurrentWeather"
    string description = 2;
    string parameters_json_schema = 3; // The JSON schema for the tool's input
}
```

### Implementation Notes
-   The `RegistryService` will be populated by inspecting the manifests of successfully deployed bundles.
-   This service provides the necessary data for a UI to dynamically render agent/tool selection menus.

---

## 4. Fix gRPC Service Compilation Errors

**Gap:** The current gRPC service implementations have compilation errors that prevent the runtime from building.

**Action:** Systematically fix all compilation issues to ensure a working implementation.

### Required Fixes
- Add missing helper methods on `AriaError` (`not_found()`, `database_error()`)
- Fix database pool access methods on `DatabaseManager`
- Add missing trait imports for `ToolRegistryInterface`
- Fix protobuf enum variant name mismatches
- Correct notification structure to match generated protobuf types
- Add missing `prost-types` imports where needed

### Implementation Notes
- Ensure all services compile and integrate properly with existing infrastructure
- Maintain type safety and proper error propagation
- Follow established patterns from existing codebase

---

## 5. Add Database Schema for gRPC Services

**Gap:** The gRPC services require database tables for sessions, messages, and notifications that don't exist yet.

**Action:** Create comprehensive database schema and migration system.

### Required Tables
```sql
-- Sessions table
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    status TEXT NOT NULL,
    context_data TEXT NOT NULL -- JSON
);

-- Messages table  
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL, -- 'system', 'user', 'assistant', 'tool'
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

-- Notifications table
CREATE TABLE notifications (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    type TEXT NOT NULL, -- 'info', 'warning', 'error', 'success'
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    metadata_json TEXT NOT NULL, -- JSON
    created_at INTEGER NOT NULL,
    read BOOLEAN NOT NULL DEFAULT FALSE
);
```

### Implementation Notes
- Integrate with existing `DatabaseManager` initialization
- Add proper indices for performance
- Ensure foreign key constraints are enforced

---

## 6. Create Main gRPC Server Binary

**Gap:** No main server binary exists to start and coordinate all gRPC services.

**Action:** Create a production-ready server binary that initializes and serves all gRPC services.

### Requirements
- Initialize all engines (database, tool registry, intelligence, etc.)
- Start all gRPC services (Task, Session, Container, Notification, Bundle)
- Proper graceful shutdown handling
- Comprehensive logging and observability
- Configuration management
- Health checks and readiness probes

### Implementation Notes
- Use `tonic::transport::Server` for serving multiple services
- Implement proper signal handling for graceful shutdown
- Add structured logging with `tracing`
- Include metrics collection and health endpoints

---

## 7. Add Comprehensive Error Handling and Logging

**Gap:** Current error handling is incomplete and logging is inconsistent across services.

**Action:** Implement production-grade error handling and structured logging.

### Requirements
- Consistent error propagation patterns across all services
- Structured logging with correlation IDs
- Proper error context and stack traces
- Client-safe error messages (no internal details leaked)
- Metrics and alerting for error rates

### Implementation Notes
- Use `tracing` spans for request correlation
- Implement error middleware for consistent handling
- Add proper error categorization and severity levels
- Include performance metrics and timing information

---

## 8. End-to-End API Testing

**Gap:** No comprehensive testing framework exists for the complete gRPC API surface.

**Action:** Create thorough end-to-end test suite covering all services and integration scenarios.

### Test Coverage
- All gRPC service methods and streaming endpoints
- Error conditions and edge cases
- Database integration and transaction handling
- Tool registry and bundle integration
- Session lifecycle and conversation flows
- Notification broadcasting and subscription

### Implementation Notes
- Use `tonic-test` for gRPC service testing
- Create test fixtures and mock data
- Implement test database isolation
- Add performance and load testing scenarios
- Include integration tests with real containers and tools

---

## 9. Define and Implement Settings Persistence

**Gap:** The contract is silent on how client settings (e.g., model choice) are persisted.

**Action:** Establish a formal policy and create a `UserPreferenceService`.

### Policy
-   **Client-Side (`localStorage`):** Non-critical, purely cosmetic UI settings (e.g., theme, layout state).
-   **Server-Side:** Any setting that alters agent behavior or security context (e.g., active model, system prompts, API keys).

### Protobuf Definition (new file: `preference_service.proto`)
```protobuf
// aria/v1/preference_service.proto
syntax = "proto3";

package aria.v1;

import "google/protobuf/struct.proto";

// Service for managing user-specific settings.
service UserPreferenceService {
    // Updates user preferences. Sent as a partial update.
    rpc UpdateUserPreferences(UpdateUserPreferencesRequest) returns (UserPreferences);

    // Retrieves the current user's preferences.
    rpc GetUserPreferences(GetUserPreferencesRequest) returns (UserPreferences);
}

message UserPreferences {
    // A free-form map to store user settings.
    // Example keys: "llm.model", "llm.system_prompt", "agent.default_id"
    google.protobuf.Struct preferences = 1;
}

message UpdateUserPreferencesRequest {
    // The fields to update. Follows a field mask pattern.
    google.protobuf.Struct updates = 1;
}

message GetUserPreferencesRequest {}
``` 