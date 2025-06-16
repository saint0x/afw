use crate::engines::tool_registry::{RegistryEntry, ToolType, SecurityLevel, ToolScope};
use crate::types::ResourceRequirements;

pub fn start_container_tool() -> RegistryEntry {
    RegistryEntry {
        name: "startContainer".to_string(),
        description: "Starts a created container that is in PENDING state.".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "containerId": {
                    "type": "string",
                    "description": "The ID of the container to start."
                }
            },
            "required": ["containerId"]
        }),
        tool_type: ToolType::Container {
            image: "".to_string(), // Not applicable for start operation
            command: vec![],
        },
        scope: ToolScope::Primitive,
        bundle_id: None,
        version: "1.0.0".to_string(),
        capabilities: vec!["container_lifecycle".to_string()],
        resource_requirements: ResourceRequirements::default(),
        security_level: SecurityLevel::Dangerous, // Starting containers is a privileged operation
    }
} 