/*!
# Custom Tool Management API

Comprehensive management interface for custom bundle-based tools with
lifecycle management, metadata tracking, and usage analytics.
*/

use crate::bundle_discovery::BundleToolDiscovery;
use crate::engines::tool_registry::bundle_integration::{
    BundleToolRegistry, CustomToolEntry, ToolSourceInfo, BundleToolStats
};
use crate::errors::{AriaError, AriaResult, ErrorCategory, ErrorCode, ErrorSeverity};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};
use tokio::sync::RwLock;

/// Custom tool management service
pub struct CustomToolManager {
    /// Bundle tool registry
    bundle_registry: Arc<BundleToolRegistry>,
    /// Bundle discovery service
    discovery: Arc<BundleToolDiscovery>,
}

/// Extended custom tool entry with usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedCustomToolEntry {
    pub name: String,
    pub description: String,
    pub bundle_hash: String,
    pub bundle_name: String,
    pub bundle_version: String,
    pub registered_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub usage_count: u64,
    pub capabilities: Vec<String>,
    pub security_level: String,
    pub resource_requirements: CustomToolResourceRequirements,
}

/// Custom tool resource requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomToolResourceRequirements {
    pub memory_mb: Option<u64>,
    pub cpu_cores: Option<f64>,
    pub timeout_seconds: Option<u64>,
    pub network_access: bool,
    pub file_system_access: bool,
}

/// Tool discovery and registration result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDiscoveryResult {
    pub discovered_tools: Vec<String>,
    pub already_registered: Vec<String>,
    pub registration_results: RegistrationResults,
    pub discovery_timestamp: DateTime<Utc>,
}

/// Registration results summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationResults {
    pub successful: Vec<String>,
    pub failed: Vec<(String, String)>, // (tool_name, error_message)
}

/// Custom tool management statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomToolManagementStats {
    pub total_custom_tools: usize,
    pub unique_bundles: usize,
    pub tools_by_bundle: HashMap<String, usize>,
    pub tools_by_security_level: HashMap<String, usize>,
    pub most_used_tools: Vec<(String, u64)>,
    pub recently_registered_tools: Vec<String>,
}

impl CustomToolManager {
    /// Create a new custom tool manager
    pub fn new(
        bundle_registry: Arc<BundleToolRegistry>,
        discovery: Arc<BundleToolDiscovery>,
    ) -> Self {
        Self {
            bundle_registry,
            discovery,
        }
    }

    /// List all custom tools with extended information
    pub async fn list_custom_tools(&self) -> AriaResult<Vec<ExtendedCustomToolEntry>> {
        debug!("Listing all custom tools");

        let basic_entries = self.bundle_registry.list_custom_tools().await;
        let mut extended_entries = Vec::new();

        for entry in basic_entries {
            if let Some(source_info) = self.bundle_registry.get_tool_source_info(&entry.name).await {
                let extended_entry = ExtendedCustomToolEntry {
                    name: entry.name.clone(),
                    description: entry.description,
                    bundle_hash: entry.bundle_hash,
                    bundle_name: entry.bundle_name,
                    bundle_version: source_info.bundle_version,
                    registered_at: DateTime::from_timestamp(entry.registered_at as i64, 0)
                        .unwrap_or_else(Utc::now),
                    last_used: None, // TODO: Implement usage tracking
                    usage_count: 0, // TODO: Implement usage tracking
                    capabilities: source_info.tool_manifest.capabilities,
                    security_level: format!("{:?}", source_info.tool_manifest.security_level),
                    resource_requirements: CustomToolResourceRequirements {
                        memory_mb: Some(source_info.tool_manifest.resource_requirements.memory_mb),
                        cpu_cores: source_info.tool_manifest.resource_requirements.cpu_cores.map(|c| c as f64),
                        timeout_seconds: source_info.tool_manifest.resource_requirements.timeout_seconds,
                        network_access: true, // Default to true for bundle tools
                        file_system_access: true, // Default to true for bundle tools
                    },
                };
                extended_entries.push(extended_entry);
            }
        }

        info!("Listed {} custom tools", extended_entries.len());
        Ok(extended_entries)
    }

    /// Remove a custom tool
    pub async fn remove_custom_tool(&self, tool_name: &str) -> AriaResult<()> {
        info!("Removing custom tool: {}", tool_name);

        // Verify it's a custom tool
        if !self.bundle_registry.is_bundle_tool(tool_name).await {
            return Err(AriaError::new(
                ErrorCode::ToolNotFound,
                ErrorCategory::Tool,
                ErrorSeverity::Medium,
                &format!("Tool '{}' is not a custom bundle tool", tool_name),
            ));
        }

        // Remove the tool
        self.bundle_registry.remove_custom_tool(tool_name).await?;

        info!("Successfully removed custom tool: {}", tool_name);
        Ok(())
    }

    /// Get detailed tool source information
    pub async fn get_tool_source_info(&self, tool_name: &str) -> AriaResult<Option<ToolSourceInfo>> {
        debug!("Getting source info for tool: {}", tool_name);

        Ok(self.bundle_registry.get_tool_source_info(tool_name).await)
    }

    /// Discover and register tools from all available bundles
    pub async fn discover_and_register_all_tools(&self) -> AriaResult<ToolDiscoveryResult> {
        info!("Discovering and registering all available tools");

        // Get all discoverable tools
        let discoverable_tools = self.discovery.get_all_available_tools().await?;
        
        // Get already registered tools
        let registered_tools = self.bundle_registry.list_custom_tools().await
            .into_iter()
            .map(|entry| entry.name)
            .collect::<Vec<_>>();

        // Filter out already registered tools
        let mut tools_to_register = Vec::new();
        let mut already_registered = Vec::new();

        for tool in &discoverable_tools {
            if registered_tools.contains(tool) {
                already_registered.push(tool.clone());
            } else {
                tools_to_register.push(tool.clone());
            }
        }

        // Manually register new tools
        let mut successful = Vec::new();
        let mut failed = Vec::new();

        for tool_name in &tools_to_register {
            match self.discovery.discover_tool(tool_name).await {
                Ok(Some(tool_entry)) => {
                    match self.bundle_registry.register_tool_from_manifest(&tool_entry).await {
                        Ok(_) => {
                            successful.push(tool_name.clone());
                            info!("Successfully registered tool: {}", tool_name);
                        }
                        Err(e) => {
                            failed.push((tool_name.clone(), e.to_string()));
                            warn!("Failed to register tool '{}': {}", tool_name, e);
                        }
                    }
                }
                Ok(None) => {
                    failed.push((tool_name.clone(), "Tool not found in bundles".to_string()));
                }
                Err(e) => {
                    failed.push((tool_name.clone(), format!("Discovery failed: {}", e)));
                }
            }
        }

        let registration_results = RegistrationResults {
            successful,
            failed,
        };

        let result = ToolDiscoveryResult {
            discovered_tools: discoverable_tools,
            already_registered,
            registration_results,
            discovery_timestamp: Utc::now(),
        };

        info!("Discovery and registration complete: {} discovered, {} registered", 
              result.discovered_tools.len(), result.registration_results.successful.len());

        Ok(result)
    }

    /// Remove all tools from a specific bundle
    pub async fn remove_bundle_tools(&self, bundle_hash: &str) -> AriaResult<Vec<String>> {
        info!("Removing all tools from bundle: {}", bundle_hash);

        let removed_tools = self.bundle_registry.remove_bundle_tools(bundle_hash).await?;

        info!("Removed {} tools from bundle", removed_tools.len());
        Ok(removed_tools)
    }

    /// Get custom tool management statistics
    pub async fn get_management_stats(&self) -> AriaResult<CustomToolManagementStats> {
        debug!("Getting custom tool management statistics");

        let bundle_stats = self.bundle_registry.get_bundle_tool_stats().await;
        let custom_tools = self.list_custom_tools().await?;

        // Convert security level counts
        let tools_by_security_level: HashMap<String, usize> = bundle_stats
            .security_level_counts
            .into_iter()
            .map(|(level, count)| (format!("{:?}", level), count))
            .collect();

        // Get most used tools (placeholder - would need usage tracking)
        let most_used_tools = custom_tools
            .iter()
            .map(|tool| (tool.name.clone(), tool.usage_count))
            .collect::<Vec<_>>();

        // Get recently registered tools (last 10)
        let mut recently_registered = custom_tools.clone();
        recently_registered.sort_by(|a, b| b.registered_at.cmp(&a.registered_at));
        let recently_registered_tools = recently_registered
            .into_iter()
            .take(10)
            .map(|tool| tool.name)
            .collect();

        Ok(CustomToolManagementStats {
            total_custom_tools: bundle_stats.total_bundle_tools,
            unique_bundles: bundle_stats.unique_bundles,
            tools_by_bundle: bundle_stats.bundle_tool_counts,
            tools_by_security_level,
            most_used_tools,
            recently_registered_tools,
        })
    }

    /// Search custom tools by criteria
    pub async fn search_custom_tools(&self, criteria: ToolSearchCriteria) -> AriaResult<Vec<ExtendedCustomToolEntry>> {
        debug!("Searching custom tools with criteria: {:?}", criteria);

        let all_tools = self.list_custom_tools().await?;
        let mut filtered_tools = Vec::new();

        for tool in all_tools {
            let mut matches = true;

            // Filter by name pattern
            if let Some(ref name_pattern) = criteria.name_pattern {
                if !tool.name.to_lowercase().contains(&name_pattern.to_lowercase()) {
                    matches = false;
                }
            }

            // Filter by bundle
            if let Some(ref bundle_name) = criteria.bundle_name {
                if tool.bundle_name != *bundle_name {
                    matches = false;
                }
            }

            // Filter by capabilities
            if !criteria.required_capabilities.is_empty() {
                let has_all_capabilities = criteria.required_capabilities
                    .iter()
                    .all(|cap| tool.capabilities.contains(cap));
                if !has_all_capabilities {
                    matches = false;
                }
            }

            // Filter by security level
            if let Some(ref security_level) = criteria.security_level {
                if tool.security_level != *security_level {
                    matches = false;
                }
            }

            if matches {
                filtered_tools.push(tool);
            }
        }

        info!("Found {} tools matching search criteria", filtered_tools.len());
        Ok(filtered_tools)
    }

    /// Validate tool configuration and dependencies
    pub async fn validate_tool(&self, tool_name: &str) -> AriaResult<ToolValidationResult> {
        debug!("Validating tool: {}", tool_name);

        let source_info = self.get_tool_source_info(tool_name).await?
            .ok_or_else(|| AriaError::new(
                ErrorCode::ToolNotFound,
                ErrorCategory::Tool,
                ErrorSeverity::Medium,
                &format!("Tool '{}' not found", tool_name),
            ))?;

        let mut validation_result = ToolValidationResult {
            tool_name: tool_name.to_string(),
            is_valid: true,
            issues: Vec::new(),
            warnings: Vec::new(),
        };

        // Validate resource requirements
        let req = &source_info.tool_manifest.resource_requirements;
        let memory_mb = req.memory_mb;
        if memory_mb > 4096 {
            validation_result.warnings.push(
                format!("High memory requirement: {}MB", memory_mb)
            );
        }

        // Validate security level
        match source_info.tool_manifest.security_level {
            crate::engines::tool_registry::SecurityLevel::Dangerous => {
                validation_result.warnings.push(
                    "Tool has dangerous security level".to_string()
                );
            }
            _ => {}
        }

        // Check if bundle still exists
        if let Err(_) = self.discovery.discover_tool(tool_name).await {
            validation_result.is_valid = false;
            validation_result.issues.push(
                "Bundle is no longer available".to_string()
            );
        }

        info!("Tool validation complete for '{}': {}", 
              tool_name, if validation_result.is_valid { "VALID" } else { "INVALID" });

        Ok(validation_result)
    }

    /// Pre-warm discovery cache for better performance
    pub async fn pre_warm_cache(&self) -> AriaResult<()> {
        info!("Pre-warming custom tool management cache");
        // Force cache refresh in discovery service
        self.discovery.refresh_cache().await?;
        info!("Cache pre-warming complete");
        Ok(())
    }
}

/// Tool search criteria
#[derive(Debug, Clone, Default)]
pub struct ToolSearchCriteria {
    pub name_pattern: Option<String>,
    pub bundle_name: Option<String>,
    pub required_capabilities: Vec<String>,
    pub security_level: Option<String>,
}

/// Tool validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolValidationResult {
    pub tool_name: String,
    pub is_valid: bool,
    pub issues: Vec<String>,
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle_discovery::BundleToolDiscovery;
    use crate::engines::tool_registry::bundle_integration::BundleToolRegistry;
    use crate::engines::execution::tool_resolver::ToolResolver;
    use crate::pkg_store::PackageStore;
    use std::sync::Arc;
    use std::collections::HashMap;
    use tokio::sync::RwLock;

    async fn create_test_manager() -> CustomToolManager {
        let pkg_store = Arc::new(PackageStore::new().await.unwrap());
        let discovery = Arc::new(BundleToolDiscovery::new(pkg_store));
        let tools = Arc::new(RwLock::new(HashMap::new()));
        let bundle_registry = Arc::new(BundleToolRegistry::new(tools.clone(), discovery.clone()));

        CustomToolManager::new(bundle_registry, discovery)
    }

    #[tokio::test]
    async fn test_custom_tool_manager_creation() {
        let manager = create_test_manager().await;
        let stats = manager.get_management_stats().await.unwrap();
        assert_eq!(stats.total_custom_tools, 0); // No tools initially
    }

    #[tokio::test]
    async fn test_tool_search_criteria() {
        let criteria = ToolSearchCriteria {
            name_pattern: Some("web".to_string()),
            bundle_name: Some("test-bundle".to_string()),
            required_capabilities: vec!["web_access".to_string()],
            security_level: Some("Safe".to_string()),
        };

        assert_eq!(criteria.name_pattern.as_ref().unwrap(), "web");
        assert_eq!(criteria.required_capabilities.len(), 1);
    }

    #[tokio::test]
    async fn test_empty_tool_list() {
        let manager = create_test_manager().await;
        let tools = manager.list_custom_tools().await.unwrap();
        assert!(tools.is_empty());
    }
} 