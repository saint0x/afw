/*!
# Tool Registry Bundle Integration

Extensions to the ToolRegistry for seamless bundle tool integration with
automatic registration, validation, and lifecycle management.
*/

use crate::bundle_discovery::{BundleToolDiscovery, BundleToolEntry};
use crate::engines::tool_registry::{RegistryEntry, ToolType, ToolScope, SecurityLevel};
use crate::errors::{AriaError, AriaResult, ErrorCategory, ErrorCode, ErrorSeverity};
use crate::types::ResourceRequirements;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Bundle tool registration result
#[derive(Debug, Clone)]
pub struct BundleToolRegistration {
    pub tool_name: String,
    pub bundle_hash: String,
    pub bundle_name: String,
    pub registered_at: u64,
    pub registry_entry: RegistryEntry,
}

/// Tool source information for bundle tools
#[derive(Debug, Clone)]
pub struct ToolSourceInfo {
    pub bundle_hash: String,
    pub bundle_name: String,
    pub bundle_version: String,
    pub tool_manifest: crate::engines::tool_registry::ToolManifest,
    pub registered_at: u64,
}

/// Bundle-aware tool registry extensions
pub struct BundleToolRegistry {
    /// Core tool registry storage
    tools: Arc<RwLock<HashMap<String, RegistryEntry>>>,
    /// Bundle tool source tracking
    bundle_tools: Arc<RwLock<HashMap<String, ToolSourceInfo>>>,
    /// Bundle discovery service
    discovery: Arc<BundleToolDiscovery>,
}

impl BundleToolRegistry {
    /// Create new bundle tool registry with discovery service
    pub fn new(
        tools: Arc<RwLock<HashMap<String, RegistryEntry>>>,
        discovery: Arc<BundleToolDiscovery>,
    ) -> Self {
        Self {
            tools,
            bundle_tools: Arc::new(RwLock::new(HashMap::new())),
            discovery,
        }
    }

    /// Register a tool from a bundle manifest
    pub async fn register_tool_from_manifest(
        &self,
        bundle_entry: &BundleToolEntry,
    ) -> AriaResult<BundleToolRegistration> {
        debug!("Registering tool '{}' from bundle '{}'", 
               bundle_entry.tool_manifest.name, bundle_entry.bundle_name);

        // Create registry entry from manifest
        let registry_entry = self.create_registry_entry_from_manifest(bundle_entry)?;

        // Register in tool registry
        {
            let mut tools = self.tools.write().await;
            tools.insert(bundle_entry.tool_manifest.name.clone(), registry_entry.clone());
        }

        // Track bundle source info
        let source_info = ToolSourceInfo {
            bundle_hash: bundle_entry.bundle_hash.clone(),
            bundle_name: bundle_entry.bundle_name.clone(),
            bundle_version: bundle_entry.bundle_version.clone(),
            tool_manifest: bundle_entry.tool_manifest.clone(),
            registered_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        {
            let mut bundle_tools = self.bundle_tools.write().await;
            bundle_tools.insert(bundle_entry.tool_manifest.name.clone(), source_info);
        }

        let registration = BundleToolRegistration {
            tool_name: bundle_entry.tool_manifest.name.clone(),
            bundle_hash: bundle_entry.bundle_hash.clone(),
            bundle_name: bundle_entry.bundle_name.clone(),
            registered_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            registry_entry,
        };

        info!("Successfully registered tool '{}' from bundle '{}'", 
              registration.tool_name, registration.bundle_name);

        Ok(registration)
    }

    /// Create registry entry from bundle tool manifest
    fn create_registry_entry_from_manifest(
        &self,
        bundle_entry: &BundleToolEntry,
    ) -> AriaResult<RegistryEntry> {
        let capabilities = self.extract_tool_capabilities(&bundle_entry.tool_manifest);
        let description = self.generate_tool_description_for_llm(&bundle_entry.tool_manifest, bundle_entry);

        Ok(RegistryEntry {
            name: bundle_entry.tool_manifest.name.clone(),
            description,
            parameters: bundle_entry.tool_manifest.parameters.clone(),
            tool_type: ToolType::Bundle {
                bundle_path: bundle_entry.bundle_hash.clone(),
                entry_point: bundle_entry.tool_manifest.entry_point.clone(),
            },
            scope: ToolScope::Abstract, // Bundle tools are abstract by default
            bundle_id: Some(bundle_entry.bundle_hash.clone()),
            version: bundle_entry.bundle_version.clone(),
            capabilities,
            resource_requirements: bundle_entry.tool_manifest.resource_requirements.clone(),
            security_level: bundle_entry.tool_manifest.security_level.clone(),
        })
    }

    /// Extract tool capabilities from manifest
    fn extract_tool_capabilities(&self, tool_manifest: &crate::engines::tool_registry::ToolManifest) -> Vec<String> {
        let mut capabilities = tool_manifest.capabilities.clone();
        
        // Add inferred capabilities based on tool name and description
        let name_lower = tool_manifest.name.to_lowercase();
        let desc_lower = tool_manifest.description.to_lowercase();

        if name_lower.contains("file") || desc_lower.contains("file") {
            capabilities.push("file_operations".to_string());
        }
        if name_lower.contains("web") || desc_lower.contains("http") || desc_lower.contains("api") {
            capabilities.push("web_access".to_string());
        }
        if name_lower.contains("data") || desc_lower.contains("database") {
            capabilities.push("data_processing".to_string());
        }
        if name_lower.contains("analyze") || desc_lower.contains("process") {
            capabilities.push("analysis".to_string());
        }

        capabilities.sort();
        capabilities.dedup();
        capabilities
    }

    /// Generate enhanced tool description for LLM
    fn generate_tool_description_for_llm(
        &self,
        tool_manifest: &crate::engines::tool_registry::ToolManifest,
        bundle_entry: &BundleToolEntry,
    ) -> String {
        let base_description = &tool_manifest.description;
        let capabilities = tool_manifest.capabilities.join(", ");
        
        format!(
            "{}\n\nBundle: {} (v{})\nCapabilities: [{}]\nSecurity Level: {:?}",
            base_description,
            bundle_entry.bundle_name,
            bundle_entry.bundle_version,
            capabilities,
            tool_manifest.security_level
        )
    }

    /// Auto-register a discovered tool
    pub async fn auto_register_discovered_tool(&self, tool_name: &str) -> AriaResult<bool> {
        debug!("Auto-registering discovered tool: {}", tool_name);

        // Check if tool is already registered
        {
            let tools = self.tools.read().await;
            if tools.contains_key(tool_name) {
                debug!("Tool '{}' already registered", tool_name);
                return Ok(false);
            }
        }

        // Discover tool in bundles
        match self.discovery.discover_tool(tool_name).await? {
            Some(bundle_entry) => {
                self.register_tool_from_manifest(&bundle_entry).await?;
                info!("Auto-registered tool '{}' from bundle '{}'", 
                      tool_name, bundle_entry.bundle_name);
                Ok(true)
            }
            None => {
                debug!("Tool '{}' not found in any bundles", tool_name);
                Ok(false)
            }
        }
    }

    /// Get tool source information
    pub async fn get_tool_source_info(&self, tool_name: &str) -> Option<ToolSourceInfo> {
        let bundle_tools = self.bundle_tools.read().await;
        bundle_tools.get(tool_name).cloned()
    }

    /// List all custom (bundle) tools
    pub async fn list_custom_tools(&self) -> Vec<CustomToolEntry> {
        let bundle_tools = self.bundle_tools.read().await;
        
        bundle_tools
            .iter()
            .map(|(name, info)| CustomToolEntry {
                name: name.clone(),
                description: info.tool_manifest.description.clone(),
                bundle_hash: info.bundle_hash.clone(),
                bundle_name: info.bundle_name.clone(),
                registered_at: info.registered_at,
            })
            .collect()
    }

    /// Remove a custom tool and its bundle association
    pub async fn remove_custom_tool(&self, tool_name: &str) -> AriaResult<()> {
        debug!("Removing custom tool: {}", tool_name);

        // Check if it's a bundle tool
        let is_bundle_tool = {
            let bundle_tools = self.bundle_tools.read().await;
            bundle_tools.contains_key(tool_name)
        };

        if !is_bundle_tool {
            return Err(AriaError::new(
                ErrorCode::ToolNotFound,
                ErrorCategory::Tool,
                ErrorSeverity::Medium,
                &format!("Tool '{}' is not a custom bundle tool", tool_name),
            ));
        }

        // Remove from tool registry
        {
            let mut tools = self.tools.write().await;
            tools.remove(tool_name);
        }

        // Remove bundle tracking
        {
            let mut bundle_tools = self.bundle_tools.write().await;
            bundle_tools.remove(tool_name);
        }

        info!("Removed custom tool: {}", tool_name);
        Ok(())
    }

    /// Remove all tools from a specific bundle
    pub async fn remove_bundle_tools(&self, bundle_hash: &str) -> AriaResult<Vec<String>> {
        debug!("Removing all tools from bundle: {}", bundle_hash);

        let mut removed_tools = Vec::new();

        // Find tools from this bundle
        let tools_to_remove: Vec<String> = {
            let bundle_tools = self.bundle_tools.read().await;
            bundle_tools
                .iter()
                .filter(|(_, info)| info.bundle_hash == bundle_hash)
                .map(|(name, _)| name.clone())
                .collect()
        };

        // Remove each tool
        for tool_name in tools_to_remove {
            match self.remove_custom_tool(&tool_name).await {
                Ok(()) => removed_tools.push(tool_name),
                Err(e) => warn!("Failed to remove tool '{}': {}", tool_name, e),
            }
        }

        info!("Removed {} tools from bundle '{}'", removed_tools.len(), bundle_hash);
        Ok(removed_tools)
    }

    /// Get the bundle source for a tool
    pub async fn get_tool_bundle_source(&self, tool_name: &str) -> Option<String> {
        let bundle_tools = self.bundle_tools.read().await;
        bundle_tools.get(tool_name).map(|info| info.bundle_hash.clone())
    }

    /// Check if a tool is from a bundle
    pub async fn is_bundle_tool(&self, tool_name: &str) -> bool {
        let bundle_tools = self.bundle_tools.read().await;
        bundle_tools.contains_key(tool_name)
    }

    /// Get bundle tool statistics
    pub async fn get_bundle_tool_stats(&self) -> BundleToolStats {
        let bundle_tools = self.bundle_tools.read().await;
        
        let mut bundle_counts: HashMap<String, usize> = HashMap::new();
        let mut security_levels: HashMap<SecurityLevel, usize> = HashMap::new();

        for (_, info) in bundle_tools.iter() {
            *bundle_counts.entry(info.bundle_name.clone()).or_insert(0) += 1;
            *security_levels.entry(info.tool_manifest.security_level.clone()).or_insert(0) += 1;
        }

        BundleToolStats {
            total_bundle_tools: bundle_tools.len(),
            unique_bundles: bundle_counts.len(),
            bundle_tool_counts: bundle_counts,
            security_level_counts: security_levels,
        }
    }
}

/// Custom tool registry entry
#[derive(Debug, Clone)]
pub struct CustomToolEntry {
    pub name: String,
    pub description: String,
    pub bundle_hash: String,
    pub bundle_name: String,
    pub registered_at: u64,
}

/// Bundle tool statistics
#[derive(Debug, Clone)]
pub struct BundleToolStats {
    pub total_bundle_tools: usize,
    pub unique_bundles: usize,
    pub bundle_tool_counts: HashMap<String, usize>,
    pub security_level_counts: HashMap<SecurityLevel, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle_discovery::BundleToolEntry;
    use crate::engines::tool_registry::ToolManifest;
    use crate::types::ResourceRequirements;

    fn create_test_bundle_entry() -> BundleToolEntry {
        BundleToolEntry {
            bundle_hash: "test-hash".to_string(),
            bundle_name: "test-bundle".to_string(),
            bundle_version: "1.0.0".to_string(),
            tool_manifest: ToolManifest {
                name: "test-tool".to_string(),
                description: "A test tool for file operations".to_string(),
                parameters: serde_json::json!({}),
                entry_point: "implementations/tools/test-tool.js".to_string(),
                capabilities: vec!["custom".to_string()],
                resource_requirements: ResourceRequirements::default(),
                security_level: SecurityLevel::Safe,
            },
            cached_at: 1234567890,
        }
    }

    #[tokio::test]
    async fn test_registry_entry_creation() {
        let tools = Arc::new(RwLock::new(HashMap::new()));
        let discovery = Arc::new(
            BundleToolDiscovery::new(Arc::new(crate::pkg_store::PackageStore::new().await.unwrap()))
        );
        let registry = BundleToolRegistry::new(tools, discovery);

        let bundle_entry = create_test_bundle_entry();
        let registry_entry = registry.create_registry_entry_from_manifest(&bundle_entry).unwrap();

        assert_eq!(registry_entry.name, "test-tool");
        assert!(registry_entry.description.contains("file operations"));
        assert!(registry_entry.capabilities.contains(&"file_operations".to_string()));
        assert!(matches!(registry_entry.tool_type, ToolType::Bundle { .. }));
    }

    #[tokio::test]
    async fn test_capability_extraction() {
        let tools = Arc::new(RwLock::new(HashMap::new()));
        let discovery = Arc::new(
            BundleToolDiscovery::new(Arc::new(crate::pkg_store::PackageStore::new().await.unwrap()))
        );
        let registry = BundleToolRegistry::new(tools, discovery);

        let mut tool_manifest = ToolManifest {
            name: "web-scraper".to_string(),
            description: "Scrapes web pages and analyzes data".to_string(),
            parameters: serde_json::json!({}),
            entry_point: "implementations/tools/web-scraper.js".to_string(),
            capabilities: vec![],
            resource_requirements: ResourceRequirements::default(),
            security_level: SecurityLevel::Limited,
        };

        let capabilities = registry.extract_tool_capabilities(&tool_manifest);
        
        assert!(capabilities.contains(&"web_access".to_string()));
        assert!(capabilities.contains(&"analysis".to_string()));
        assert!(capabilities.contains(&"data_processing".to_string()));
    }
} 