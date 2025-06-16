use crate::engines::tool_registry::{RegistryEntry, ToolType, SecurityLevel, ToolScope};
use crate::types::ResourceRequirements;

pub fn get_container_network_info_tool() -> RegistryEntry {
    RegistryEntry {
        name: "getContainerNetworkInfo".to_string(),
        description: "Gets detailed network information for a specific container.".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "containerId": {
                    "type": "string",
                    "description": "The ID of the container to get network info for."
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
        capabilities: vec!["network_introspection".to_string()],
        resource_requirements: ResourceRequirements::default(),
        security_level: SecurityLevel::Limited,
    }
} 