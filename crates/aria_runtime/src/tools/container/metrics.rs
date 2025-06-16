use crate::engines::tool_registry::{RegistryEntry, ToolType, SecurityLevel, ToolScope};
use crate::types::ResourceRequirements;

pub fn get_system_metrics_tool() -> RegistryEntry {
    RegistryEntry {
        name: "getSystemMetrics".to_string(),
        description: "Gets system-level metrics from the host, such as CPU, memory, and container count.".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {}
        }),
        tool_type: ToolType::Container { // Categorized as a container tool for logical grouping
            image: "".to_string(),
            command: vec![],
        },
        scope: ToolScope::Primitive,
        bundle_id: None,
        version: "1.0.0".to_string(),
        capabilities: vec!["system_introspection".to_string()],
        resource_requirements: ResourceRequirements::default(),
        security_level: SecurityLevel::Limited,
    }
} 