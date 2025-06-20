# Aria Runtime API Contract

This document provides the definitive technical specification for clients interacting with the Aria Runtime backend. It covers service discovery, communication protocols, and the full definition of all available gRPC services.

## 1. Architecture Overview

The Aria Runtime exposes its primary API via **gRPC over a Unix Domain Socket (UDS)**. This provides a high-performance, secure, and strongly-typed interface for local clients (e.g., a desktop GUI, a web frontend's local server).

-   **Primary Client Interface**: gRPC over Unix Domain Socket

---

## 2. Service Discovery

### 2.1. Unix Domain Socket (UDS)

Clients must connect to the Aria Runtime's gRPC server via a Unix socket.

-   **Default Path**: `~/.aria/runtime.sock`
-   **Permissions**: The socket's permissions will be set to allow access only by the user running the Aria daemon. The client must run as the same user.

The client should treat this path as potentially configurable via an environment variable (`ARIA_RUNTIME_SOCK`) in the future, but default to the path above.

---

## 3. gRPC Services API (Primary Interface)

This is the main API for controlling the runtime. All services are exposed over the single Unix socket connection.

*Note: Authentication is not yet implemented but will be added in a future update via gRPC metadata.*

### 3.1. Protobuf Definitions

Below are the Protobuf v3 definitions for all services and associated data models.

#### `aria.proto`
```protobuf
// aria/v1/aria.proto
syntax = "proto3";

package aria.v1;

// Represents a standard key-value pair, often used for environment variables or labels.
message KeyValuePair {
    string key = 1;
    string value = 2;
}

// Represents the status of a long-running operation.
enum TaskStatus {
    TASK_STATUS_UNSPECIFIED = 0;
    PENDING = 1;
    RUNNING = 2;
    COMPLETED = 3;
    FAILED = 4;
    CANCELLED = 5;
    TIMEOUT = 6;
}

// Represents the role in a conversation.
enum MessageRole {
    MESSAGE_ROLE_UNSPECIFIED = 0;
    SYSTEM = 1;
    USER = 2;
    ASSISTANT = 3;
    TOOL = 4;
}
```

#### `notification_service.proto`
```protobuf
// aria/v1/notification_service.proto
syntax = "proto3";

package aria.v1;

import "google/protobuf/timestamp.proto";
import "aria/v1/aria.proto";

// Service for streaming real-time events from the runtime to the client.
service NotificationService {
    // Establishes a persistent stream for the client to receive notifications.
    rpc StreamNotifications(StreamNotificationsRequest) returns (stream Notification);
}

// Initial request to subscribe to notifications. Can be used to filter events in the future.
message StreamNotificationsRequest {
    // For now, it's empty and subscribes to all events.
}

// A single notification event from the runtime.
message Notification {
    string id = 1; // Unique ID for the notification event
    google.protobuf.Timestamp timestamp = 2;

    oneof event_payload {
        BundleUploadEvent bundle_upload = 3;
        TaskStatusEvent task_status = 4;
    }
}

// Event for when a .aria bundle upload status changes.
message BundleUploadEvent {
    string bundle_name = 1;
    double progress_percent = 2; // e.g., 50.5 for 50.5%
    string status_message = 3;   // e.g., "Uploading...", "Processing...", "Deploying..."
    bool success = 4;            // True if upload and deployment succeeded
    optional string error_message = 5;
}

// Event for when a task's status changes.
message TaskStatusEvent {
    string task_id = 1;
    TaskStatus new_status = 2;
    string status_message = 3;
    optional int32 exit_code = 4;
}
```

#### `task_service.proto`
```protobuf
// aria/v1/task_service.proto
syntax = "proto3";

package aria.v1;

import "google/protobuf/timestamp.proto";
import "aria/v1/aria.proto";

// Service for managing and interacting with long-running asynchronous tasks.
service TaskService {
    // Launches a new asynchronous task.
    rpc LaunchTask(LaunchTaskRequest) returns (LaunchTaskResponse);

    // Retrieves the current status and details of a task.
    rpc GetTask(GetTaskRequest) returns (Task);

    // Streams the output (stdout/stderr) and progress of a running task.
    rpc StreamTaskOutput(StreamTaskOutputRequest) returns (stream TaskOutput);

    // Cancels a pending or running task.
    rpc CancelTask(CancelTaskRequest) returns (CancelTaskResponse);
}

// Full representation of an asynchronous task.
message Task {
    string id = 1;
    string user_id = 2;
    string session_id = 3;
    string container_id = 4;
    optional string parent_task_id = 5;

    string type = 6; // e.g., "container:exec", "bundle:build"
    string command_json = 7; // JSON array representing the command
    map<string, string> environment = 8;
    int32 timeout_seconds = 9;

    TaskStatus status = 10;
    google.protobuf.Timestamp created_at = 11;
    optional google.protobuf.Timestamp started_at = 12;
    optional google.protobuf.Timestamp completed_at = 13;

    optional int32 exit_code = 14;
    optional string error_message = 15;
    
    double progress_percent = 16;
    string current_operation = 17;
}

message LaunchTaskRequest {
    string session_id = 1;
    string type = 2;
    string command_json = 3;
    map<string, string> environment = 4;
    int32 timeout_seconds = 5;
}

message LaunchTaskResponse {
    string task_id = 1;
}

message GetTaskRequest {
    string task_id = 1;
}

// Represents a single log line or progress update from a task.
message TaskOutput {
    string task_id = 1;
    google.protobuf.Timestamp timestamp = 2;

    oneof output {
        string stdout_line = 3;
        string stderr_line = 4;
        ProgressUpdate progress = 5;
    }
}

message ProgressUpdate {
    double percent_complete = 1;
    string operation_description = 2;
}

message StreamTaskOutputRequest {
    string task_id = 1;
    bool follow = 2; // If true, stream stays open for new output.
}

message CancelTaskRequest {
    string task_id = 1;
}

message CancelTaskResponse {
    bool cancellation_initiated = 1;
}
```

#### `container_service.proto`
```protobuf
// aria/v1/container_service.proto
syntax = "proto3";

package aria.v1;

import "google/protobuf/timestamp.proto";
import "aria/v1/aria.proto";

// Service for direct, low-level management of containers.
// Wraps the underlying quilt daemon.
service ContainerService {
    rpc CreateContainer(CreateContainerRequest) returns (Container);
    rpc StartContainer(StartContainerRequest) returns (StartContainerResponse);
    rpc StopContainer(StopContainerRequest) returns (StopContainerResponse);
    rpc RemoveContainer(RemoveContainerRequest) returns (RemoveContainerResponse);
    rpc GetContainer(GetContainerRequest) returns (Container);
    rpc ListContainers(ListContainersRequest) returns (ListContainersResponse);
    rpc StreamContainerLogs(StreamContainerLogsRequest) returns (stream ContainerLog);
}

message Container {
    string id = 1;
    string user_id = 2;
    optional string session_id = 3;
    string name = 4;
    string image_path = 5;
    TaskStatus status = 6;
    google.protobuf.Timestamp created_at = 7;
}

message CreateContainerRequest {
    string name = 1;
    string image_path = 2;
    repeated KeyValuePair environment = 3;
    bool persistent = 4; // If true, container survives session end
}

message StartContainerRequest {
    string container_id = 1;
}
message StartContainerResponse {}

message StopContainerRequest {
    string container_id = 1;
}
message StopContainerResponse {}

message RemoveContainerRequest {
    string container_id = 1;
}
message RemoveContainerResponse {}

message GetContainerRequest {
    string container_id = 1;
}

message ListContainersRequest {
    optional string session_id = 1; // Filter by session
}
message ListContainersResponse {
    repeated Container containers = 1;
}

message StreamContainerLogsRequest {
    string container_id = 1;
    bool follow = 2;
    optional google.protobuf.Timestamp since = 3;
}
message ContainerLog {
    string line = 1;
    enum Stream {
        STREAM_UNSPECIFIED = 0;
        STDOUT = 1;
        STDERR = 2;
    }
    Stream stream = 2;
    google.protobuf.Timestamp timestamp = 3;
}
```

#### `session_service.proto`
```protobuf
// aria/v1/session_service.proto
syntax = "proto3";

package aria.v1;

import "google/protobuf/timestamp.proto";
import "aria/v1/aria.proto";

// Service for managing user sessions and conversations.
service SessionService {
    // Creates a new session for a user.
    rpc CreateSession(CreateSessionRequest) returns (Session);

    // Gets details for a specific session.
    rpc GetSession(GetSessionRequest) returns (Session);

    // Executes a "turn" in a conversation within a session.
    rpc ExecuteTurn(ExecuteTurnRequest) returns (stream TurnOutput);
}

message Session {
    string id = 1;
    string user_id = 2;
    google.protobuf.Timestamp created_at = 3;
    map<string, string> context_data = 4;
    string status = 5; // e.g., "active", "completed", "failed"
}

message CreateSessionRequest {
    // Future: Add agent config, context, etc.
    // For now, it's simple.
}

message GetSessionRequest {
    string session_id = 1;
}

message ExecuteTurnRequest {
    string session_id = 1;
    string input = 2; // User's message/prompt
}

// Represents an event happening during an agent's turn.
message TurnOutput {
    oneof event {
        Message message = 1; // A message from user, assistant, or tool
        ToolCall tool_call = 2;
        ToolResult tool_result = 3;
        string final_response = 4; // Final assistant response
    }
}

message Message {
    string id = 1;
    MessageRole role = 2;
    string content = 3;
    google.protobuf.Timestamp created_at = 4;
}

message ToolCall {
    string tool_name = 1;
    string parameters_json = 2; // JSON object of parameters
}

message ToolResult {
    string tool_name = 1;
    string result_json = 2; // JSON object of the result
    bool success = 3;
    optional string error_message = 4;
}
```

---

## 4. Error Handling (gRPC)

Errors will be returned using standard gRPC status codes. For more detailed, structured errors, the response will include a `google.rpc.Status` payload in the error details.

-   `PERMISSION_DENIED` (7): Invalid or missing API key (once implemented).
-   `NOT_FOUND` (5): The requested resource (e.g., task, container) does not exist.
-   `INVALID_ARGUMENT` (3): The request payload is malformed or missing required fields.
-   `INTERNAL` (13): An unhandled error occurred in the runtime.

Clients should be prepared to handle these standard codes and inspect the error details for a more specific, machine-readable error message. 