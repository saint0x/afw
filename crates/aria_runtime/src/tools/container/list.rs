// src/tools/container/list.rs

use crate::engines::tool_registry::{RegistryEntry, ToolType, SecurityLevel, ToolScope};
use crate::types::ResourceRequirements;

pub fn list_containers_tool() -> RegistryEntry {
    RegistryEntry {
        name: "listContainers".to_string(),
        description: "Lists all active containers managed by the runtime.".to_string(),
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
        capabilities: vec!["container_introspection".to_string()],
        resource_requirements: ResourceRequirements::default(),
        security_level: SecurityLevel::Limited, // Listing containers is less dangerous than creating them
    }
} 