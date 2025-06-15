//! Defines the structured metadata for entities within an Aria bundle.
//!
//! These structs are serialized into the `manifest.json` file, which is the
//! central contract between the compiler and the Aria Runtime.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// The root of the bundle manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AriaManifest {
    pub name: String,
    pub version: String,
    pub tools: Vec<ToolManifest>,
    pub agents: Vec<AgentManifest>,
    pub teams: Vec<TeamManifest>,
    pub pipelines: Vec<PipelineManifest>,
}

/// Metadata for a decorated `@tool` function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManifest {
    pub name: String,
    pub description: String,
    pub inputs: HashMap<String, String>, // Placeholder
}

/// Metadata for a decorated `@agent` class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManifest {
    pub name: String,
    pub description: String,
    pub tools: Vec<String>, // Names of tools used by this agent
}

/// Metadata for a decorated `@team` class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamManifest {
    pub name: String,
    pub description: String,
    pub members: Vec<String>, // Names of agents in this team
}

/// Metadata for a decorated `@pipeline` class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineManifest {
    pub name: String,
    pub description: String,
    // Add other pipeline-specific fields here later
} 