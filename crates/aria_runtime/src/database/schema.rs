// Database Schema Definitions for Aria Runtime
// Comprehensive schema supporting user management, sessions, async tasks, and containers

/// System database schema (shared across all users)
pub const SYSTEM_SCHEMA: &str = r#"
-- Users table (in system.db)
CREATE TABLE IF NOT EXISTS users (
    user_id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE,
    created_at INTEGER NOT NULL,
    last_active INTEGER,
    preferences TEXT, -- JSON blob
    storage_quota_bytes INTEGER DEFAULT 10737418240, -- 10GB default
    api_key_hash TEXT,
    status TEXT DEFAULT 'active' -- active, suspended, deleted
);

-- Global system configuration
CREATE TABLE IF NOT EXISTS system_config (
    config_key TEXT PRIMARY KEY,
    config_value TEXT NOT NULL,
    config_type TEXT NOT NULL, -- string, number, boolean, json
    description TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Global audit log
CREATE TABLE IF NOT EXISTS global_audit_logs (
    log_id TEXT PRIMARY KEY,
    user_id TEXT,
    event_type TEXT NOT NULL, -- system_start, system_stop, user_created, etc.
    event_data TEXT, -- JSON
    ip_address TEXT,
    user_agent TEXT,
    created_at INTEGER NOT NULL,
    severity TEXT NOT NULL -- info, warning, error, critical
);

-- Indexes for system tables
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_status ON users(status);
CREATE INDEX IF NOT EXISTS idx_users_last_active ON users(last_active);
CREATE INDEX IF NOT EXISTS idx_global_audit_user_id ON global_audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_global_audit_event_type ON global_audit_logs(event_type);
CREATE INDEX IF NOT EXISTS idx_global_audit_created_at ON global_audit_logs(created_at);
"#;

/// User database schema (per-user database)
pub const USER_SCHEMA: &str = r#"
-- Sessions table (per-user database)
CREATE TABLE IF NOT EXISTS sessions (
    session_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    agent_config_id TEXT,
    created_at INTEGER NOT NULL,
    ended_at INTEGER,
    session_type TEXT NOT NULL, -- interactive, batch, api
    context_data TEXT, -- JSON blob for session state
    total_tool_calls INTEGER DEFAULT 0,
    total_tokens_used INTEGER DEFAULT 0,
    status TEXT DEFAULT 'active' -- active, completed, failed, timeout
);

-- Agent configurations
CREATE TABLE IF NOT EXISTS agent_configs (
    config_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    system_prompt TEXT,
    tool_scopes TEXT NOT NULL, -- JSON array: ["primitive", "cognitive", "custom"]
    llm_provider TEXT NOT NULL,
    llm_model TEXT NOT NULL,
    max_tokens INTEGER DEFAULT 4096,
    temperature REAL DEFAULT 0.1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    is_default BOOLEAN DEFAULT FALSE
);

-- Agent conversation history
CREATE TABLE IF NOT EXISTS conversations (
    conversation_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    agent_config_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    ended_at INTEGER,
    total_messages INTEGER DEFAULT 0,
    status TEXT DEFAULT 'active',
    FOREIGN KEY (session_id) REFERENCES sessions(session_id)
);

-- Individual messages in conversations
CREATE TABLE IF NOT EXISTS messages (
    message_id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    role TEXT NOT NULL, -- user, assistant, system, tool
    content TEXT NOT NULL,
    tool_calls TEXT, -- JSON array if role=assistant with tool calls
    tool_results TEXT, -- JSON array if role=tool
    tokens_used INTEGER,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES conversations(conversation_id)
);

-- Async tasks (for unlimited-duration operations)
CREATE TABLE IF NOT EXISTS async_tasks (
    task_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    container_id TEXT,
    parent_task_id TEXT, -- For task dependencies/workflows
    task_type TEXT NOT NULL, -- exec, build, analysis, etc.
    command TEXT NOT NULL, -- JSON array
    working_directory TEXT,
    environment TEXT, -- JSON object
    timeout_seconds INTEGER DEFAULT 0, -- 0 = no timeout
    
    -- Status tracking
    status TEXT NOT NULL DEFAULT 'pending', -- pending, running, completed, failed, cancelled, timeout
    created_at INTEGER NOT NULL,
    started_at INTEGER,
    completed_at INTEGER,
    cancelled_at INTEGER,
    
    -- Results
    exit_code INTEGER,
    stdout TEXT,
    stderr TEXT,
    error_message TEXT,
    
    -- Progress tracking
    progress_percent REAL DEFAULT 0.0,
    current_operation TEXT,
    estimated_completion INTEGER, -- unix timestamp
    
    -- Resource usage
    max_memory_bytes INTEGER,
    total_cpu_time_ms INTEGER,
    
    FOREIGN KEY (session_id) REFERENCES sessions(session_id),
    FOREIGN KEY (parent_task_id) REFERENCES async_tasks(task_id)
);

-- Task progress logs (for detailed progress tracking)
CREATE TABLE IF NOT EXISTS task_progress (
    progress_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    progress_percent REAL NOT NULL,
    operation_description TEXT,
    details TEXT, -- JSON for structured data
    FOREIGN KEY (task_id) REFERENCES async_tasks(task_id)
);

-- Task dependencies (for complex workflows)
CREATE TABLE IF NOT EXISTS task_dependencies (
    dependency_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    depends_on_task_id TEXT NOT NULL,
    dependency_type TEXT NOT NULL, -- success, completion, data
    created_at INTEGER NOT NULL,
    FOREIGN KEY (task_id) REFERENCES async_tasks(task_id),
    FOREIGN KEY (depends_on_task_id) REFERENCES async_tasks(task_id)
);

-- Enhanced containers table
CREATE TABLE IF NOT EXISTS containers (
    container_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    session_id TEXT, -- NULL for persistent containers
    name TEXT,
    image_path TEXT NOT NULL,
    command TEXT NOT NULL, -- JSON array
    environment TEXT, -- JSON object
    working_directory TEXT,
    created_at INTEGER NOT NULL,
    started_at INTEGER,
    stopped_at INTEGER,
    auto_remove BOOLEAN DEFAULT FALSE,
    persistent BOOLEAN DEFAULT FALSE, -- survives session end
    resource_limits TEXT, -- JSON: memory, cpu, etc.
    network_config TEXT, -- JSON network configuration
    status TEXT NOT NULL -- created, starting, running, stopping, stopped, failed
);

-- Container resource usage metrics
CREATE TABLE IF NOT EXISTS container_metrics (
    metric_id TEXT PRIMARY KEY,
    container_id TEXT NOT NULL,
    recorded_at INTEGER NOT NULL,
    cpu_usage_percent REAL,
    memory_usage_bytes INTEGER,
    network_rx_bytes INTEGER,
    network_tx_bytes INTEGER,
    disk_read_bytes INTEGER,
    disk_write_bytes INTEGER,
    FOREIGN KEY (container_id) REFERENCES containers(container_id)
);

-- Tool usage tracking
CREATE TABLE IF NOT EXISTS tool_usage (
    usage_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    conversation_id TEXT,
    tool_name TEXT NOT NULL,
    parameters TEXT, -- JSON
    result TEXT, -- JSON
    execution_time_ms INTEGER,
    success BOOLEAN NOT NULL,
    error_message TEXT,
    used_at INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(session_id)
);

-- Security audit log
CREATE TABLE IF NOT EXISTS audit_logs (
    log_id TEXT PRIMARY KEY,
    user_id TEXT,
    session_id TEXT,
    event_type TEXT NOT NULL, -- login, tool_use, container_create, etc.
    event_data TEXT, -- JSON
    ip_address TEXT,
    user_agent TEXT,
    created_at INTEGER NOT NULL,
    severity TEXT NOT NULL -- info, warning, error, critical
);

-- ======================================
-- CONTEXT INTELLIGENCE SCHEMA EXTENSION
-- ======================================

-- Container execution patterns (learned from execution history)
CREATE TABLE IF NOT EXISTS container_patterns (
    pattern_id TEXT PRIMARY KEY,
    pattern_trigger TEXT NOT NULL,          -- "build rust project", "run tests"
    container_config TEXT NOT NULL,         -- JSON container configuration
    confidence_score REAL DEFAULT 0.5,     -- Learning confidence (0.0 - 1.0)
    success_count INTEGER DEFAULT 0,
    failure_count INTEGER DEFAULT 0,
    avg_execution_time_ms INTEGER DEFAULT 0,
    last_used INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    pattern_variables TEXT,                 -- JSON array of variable definitions
    usage_stats TEXT                        -- JSON usage statistics blob
);

-- Execution context trees (hierarchical execution relationships)
CREATE TABLE IF NOT EXISTS execution_contexts (
    context_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    parent_context_id TEXT,
    context_type TEXT NOT NULL,             -- "session", "workflow", "container", "tool", "agent", "environment"
    context_data TEXT NOT NULL,             -- JSON context information
    priority INTEGER DEFAULT 5,            -- Context priority (1-10)
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    metadata TEXT,                          -- JSON additional metadata
    FOREIGN KEY (parent_context_id) REFERENCES execution_contexts(context_id),
    FOREIGN KEY (session_id) REFERENCES sessions(session_id)
);

-- Learning feedback (captures execution results for pattern improvement)
CREATE TABLE IF NOT EXISTS learning_feedback (
    feedback_id TEXT PRIMARY KEY,
    pattern_id TEXT NOT NULL,
    execution_id TEXT NOT NULL,
    success BOOLEAN NOT NULL,
    execution_time_ms INTEGER,
    feedback_type TEXT NOT NULL,            -- "execution", "user", "system"
    confidence_delta REAL,                  -- Change in pattern confidence
    metadata TEXT,                          -- JSON additional data
    created_at INTEGER NOT NULL,
    FOREIGN KEY (pattern_id) REFERENCES container_patterns(pattern_id)
);

-- Container workload tracking (links containers to intelligence patterns)
CREATE TABLE IF NOT EXISTS container_workloads (
    workload_id TEXT PRIMARY KEY,
    container_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    pattern_id TEXT,                        -- Associated pattern if any
    workload_type TEXT NOT NULL,            -- "build", "test", "exec", "analysis"
    request_description TEXT NOT NULL,      -- Original user request
    execution_result TEXT,                  -- JSON execution result
    created_at INTEGER NOT NULL,
    completed_at INTEGER,
    FOREIGN KEY (container_id) REFERENCES containers(container_id),
    FOREIGN KEY (session_id) REFERENCES sessions(session_id),
    FOREIGN KEY (pattern_id) REFERENCES container_patterns(pattern_id)
);

-- Intelligence query log (tracks intelligence API usage)
CREATE TABLE IF NOT EXISTS intelligence_queries (
    query_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    query_type TEXT NOT NULL,               -- "pattern_match", "context_build", "learning_update"
    request_data TEXT NOT NULL,             -- JSON request data
    response_data TEXT,                     -- JSON response data
    execution_time_ms INTEGER,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(session_id)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_created_at ON sessions(created_at);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);

CREATE INDEX IF NOT EXISTS idx_agent_configs_user_id ON agent_configs(user_id);
CREATE INDEX IF NOT EXISTS idx_agent_configs_name ON agent_configs(name);
CREATE INDEX IF NOT EXISTS idx_agent_configs_is_default ON agent_configs(is_default);

CREATE INDEX IF NOT EXISTS idx_conversations_session_id ON conversations(session_id);
CREATE INDEX IF NOT EXISTS idx_conversations_created_at ON conversations(created_at);

CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON messages(conversation_id);
CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at);
CREATE INDEX IF NOT EXISTS idx_messages_role ON messages(role);

CREATE INDEX IF NOT EXISTS idx_async_tasks_user_id ON async_tasks(user_id);
CREATE INDEX IF NOT EXISTS idx_async_tasks_session_id ON async_tasks(session_id);
CREATE INDEX IF NOT EXISTS idx_async_tasks_status ON async_tasks(status);
CREATE INDEX IF NOT EXISTS idx_async_tasks_created_at ON async_tasks(created_at);
CREATE INDEX IF NOT EXISTS idx_async_tasks_parent_task_id ON async_tasks(parent_task_id);
CREATE INDEX IF NOT EXISTS idx_async_tasks_container_id ON async_tasks(container_id);

CREATE INDEX IF NOT EXISTS idx_task_progress_task_id ON task_progress(task_id);
CREATE INDEX IF NOT EXISTS idx_task_progress_timestamp ON task_progress(timestamp);

CREATE INDEX IF NOT EXISTS idx_task_dependencies_task_id ON task_dependencies(task_id);
CREATE INDEX IF NOT EXISTS idx_task_dependencies_depends_on ON task_dependencies(depends_on_task_id);

CREATE INDEX IF NOT EXISTS idx_containers_user_id ON containers(user_id);
CREATE INDEX IF NOT EXISTS idx_containers_session_id ON containers(session_id);
CREATE INDEX IF NOT EXISTS idx_containers_status ON containers(status);
CREATE INDEX IF NOT EXISTS idx_containers_created_at ON containers(created_at);

CREATE INDEX IF NOT EXISTS idx_container_metrics_container_id ON container_metrics(container_id);
CREATE INDEX IF NOT EXISTS idx_container_metrics_recorded_at ON container_metrics(recorded_at);

CREATE INDEX IF NOT EXISTS idx_tool_usage_session_id ON tool_usage(session_id);
CREATE INDEX IF NOT EXISTS idx_tool_usage_tool_name ON tool_usage(tool_name);
CREATE INDEX IF NOT EXISTS idx_tool_usage_used_at ON tool_usage(used_at);

CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_session_id ON audit_logs(session_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_event_type ON audit_logs(event_type);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at);

-- Context Intelligence Indexes for performance
CREATE INDEX IF NOT EXISTS idx_container_patterns_confidence ON container_patterns(confidence_score DESC);
CREATE INDEX IF NOT EXISTS idx_container_patterns_last_used ON container_patterns(last_used DESC);
CREATE INDEX IF NOT EXISTS idx_container_patterns_trigger ON container_patterns(pattern_trigger);

CREATE INDEX IF NOT EXISTS idx_execution_contexts_session ON execution_contexts(session_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_execution_contexts_parent ON execution_contexts(parent_context_id);
CREATE INDEX IF NOT EXISTS idx_execution_contexts_type ON execution_contexts(context_type);
CREATE INDEX IF NOT EXISTS idx_execution_contexts_priority ON execution_contexts(priority DESC);

CREATE INDEX IF NOT EXISTS idx_learning_feedback_pattern ON learning_feedback(pattern_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_learning_feedback_execution ON learning_feedback(execution_id);
CREATE INDEX IF NOT EXISTS idx_learning_feedback_success ON learning_feedback(success, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_container_workloads_session ON container_workloads(session_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_container_workloads_pattern ON container_workloads(pattern_id);
CREATE INDEX IF NOT EXISTS idx_container_workloads_type ON container_workloads(workload_type);

CREATE INDEX IF NOT EXISTS idx_intelligence_queries_session ON intelligence_queries(session_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_intelligence_queries_type ON intelligence_queries(query_type);
"#; 