use crate::engines::tool_registry::{RegistryEntry, ToolType, SecurityLevel, ToolScope};
use crate::types::ResourceRequirements;

pub fn get_network_topology_tool() -> RegistryEntry {
    RegistryEntry {
        name: "getNetworkTopology".to_string(),
        description: "Gets the network topology of all managed containers.".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {}
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