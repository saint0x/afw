use crate::engines::tool_registry::{RegistryEntry, ToolType, SecurityLevel, ToolScope};
use crate::types::ResourceRequirements;

pub fn stop_container_tool() -> RegistryEntry {
    RegistryEntry {
        name: "stopContainer".to_string(),
        description: "Stops a running container.".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "containerId": {
                    "type": "string",
                    "description": "The ID of the container to stop."
                }
            },
            "required": ["containerId"]
        }),
        tool_type: ToolType::Container {
            image: "".to_string(),
            command: vec![],
        },
        scope: ToolScope::Primitive,
        bundle_id: None,
        version: "1.0.0".to_string(),
        capabilities: vec!["container_lifecycle".to_string()],
        resource_requirements: ResourceRequirements::default(),
        security_level: SecurityLevel::Dangerous,
    }
} 