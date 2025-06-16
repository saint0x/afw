// src/tools/container/exec.rs

use crate::engines::tool_registry::{RegistryEntry, ToolType, SecurityLevel, ToolScope};
use crate::types::ResourceRequirements;

pub fn exec_in_container_tool() -> RegistryEntry {
    RegistryEntry {
        name: "execInContainer".to_string(),
        description: "Executes a command inside a running container.".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "containerId": {
                    "type": "string",
                    "description": "The ID of the container to execute the command in."
                },
                "command": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "The command and its arguments to execute."
                }
            },
            "required": ["containerId", "command"]
        }),
        tool_type: ToolType::Container {
            image: "".to_string(),
            command: vec![],
        },
        scope: ToolScope::Primitive,
        bundle_id: None,
        version: "1.0.0".to_string(),
        capabilities: vec!["container_execution".to_string()],
        resource_requirements: ResourceRequirements::default(),
        security_level: SecurityLevel::Dangerous,
    }
} 