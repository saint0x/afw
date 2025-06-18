/*!
# Tool Resolver for Auto-Discovery

Handles unknown tool resolution through bundle discovery and automatic
registration for seamless agent tool access.
*/

use crate::bundle_discovery::BundleToolDiscovery;
use crate::engines::tool_registry::{ToolRegistry, ToolRegistryInterface};
use crate::engines::tool_registry::bundle_integration::BundleToolRegistry;
use crate::errors::{AriaError, AriaResult, ErrorCategory, ErrorCode, ErrorSeverity};
use crate::types::RegistryEntry;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Tool resolution result
#[derive(Debug, Clone)]
pub enum ToolResolutionResult {
    /// Tool was already registered
    AlreadyRegistered(RegistryEntry),
    /// Tool was discovered and auto-registered
    AutoRegistered(RegistryEntry),
    /// Tool was not found in any bundles
    NotFound,
}

/// Tool resolver for handling unknown tool discovery
pub struct ToolResolver {
    /// Core tool registry
    tool_registry: Arc<ToolRegistry>,
    /// Bundle tool registry for bundle-specific operations
    bundle_registry: Arc<BundleToolRegistry>,
    /// Bundle discovery service
    discovery: Arc<BundleToolDiscovery>,
    /// Auto-registration enabled flag
    auto_registration_enabled: bool,
}

impl ToolResolver {
    /// Create a new tool resolver
    pub fn new(
        tool_registry: Arc<ToolRegistry>,
        bundle_registry: Arc<BundleToolRegistry>,
        discovery: Arc<BundleToolDiscovery>,
    ) -> Self {
        Self {
            tool_registry,
            bundle_registry,
            discovery,
            auto_registration_enabled: true,
        }
    }

    /// Resolve an unknown tool through discovery and optional auto-registration
    pub async fn resolve_unknown_tool(&self, tool_name: &str) -> AriaResult<ToolResolutionResult> {
        debug!("Resolving unknown tool: {}", tool_name);

        // First check if tool is already registered
        if let Ok(Some(entry)) = self.tool_registry.get_tool_info(tool_name).await {
            debug!("Tool '{}' already registered", tool_name);
            return Ok(ToolResolutionResult::AlreadyRegistered(entry));
        }

        // If auto-registration is disabled, just check discovery
        if !self.auto_registration_enabled {
            debug!("Auto-registration disabled, checking discovery only");
            return match self.discovery.discover_tool(tool_name).await? {
                Some(_) => Ok(ToolResolutionResult::NotFound), // Found but not registered
                None => Ok(ToolResolutionResult::NotFound),
            };
        }

        // Attempt auto-registration
        match self.auto_register_discovered_tool(tool_name).await? {
            true => {
                // Tool was registered, fetch the registry entry
                match self.tool_registry.get_tool_info(tool_name).await? {
                    Some(entry) => {
                        info!("Successfully resolved and registered tool: {}", tool_name);
                        Ok(ToolResolutionResult::AutoRegistered(entry))
                    }
                    None => {
                        warn!("Tool '{}' was registered but not found in registry", tool_name);
                        Err(AriaError::new(
                            ErrorCode::InternalError,
                            ErrorCategory::Tool,
                            ErrorSeverity::Medium,
                            &format!("Tool '{}' registration inconsistency", tool_name),
                        ))
                    }
                }
            }
            false => {
                debug!("Tool '{}' not found in any bundles", tool_name);
                Ok(ToolResolutionResult::NotFound)
            }
        }
    }

    /// Auto-register a discovered tool
    pub async fn auto_register_discovered_tool(&self, tool_name: &str) -> AriaResult<bool> {
        debug!("Auto-registering discovered tool: {}", tool_name);

        if !self.auto_registration_enabled {
            debug!("Auto-registration disabled");
            return Ok(false);
        }

        // Use bundle registry for auto-registration
        self.bundle_registry.auto_register_discovered_tool(tool_name).await
    }

    /// Enable or disable auto-registration
    pub fn set_auto_registration(&mut self, enabled: bool) {
        self.auto_registration_enabled = enabled;
        info!("Auto-registration {}", if enabled { "enabled" } else { "disabled" });
    }

    /// Check if auto-registration is enabled
    pub fn is_auto_registration_enabled(&self) -> bool {
        self.auto_registration_enabled
    }

    /// Get available tools from all sources (registry + discoverable)
    pub async fn get_all_available_tools(&self) -> AriaResult<AvailableToolsSummary> {
        debug!("Getting all available tools");

        // Get registered tools
        let registered_tools = self.tool_registry.list_available_tools().await?;

        // Get discoverable bundle tools
        let discoverable_tools = self.discovery.get_all_available_tools().await?;

        // Separate already registered from discoverable
        let mut registered_set: std::collections::HashSet<String> = registered_tools.iter().cloned().collect();
        let mut discoverable_only = Vec::new();

        for tool in discoverable_tools {
            if !registered_set.contains(&tool) {
                discoverable_only.push(tool);
            }
        }

        Ok(AvailableToolsSummary {
            registered_tools,
            discoverable_tools: discoverable_only,
            auto_registration_enabled: self.auto_registration_enabled,
        })
    }

    /// Bulk register multiple tools from discovery
    pub async fn bulk_register_tools(&self, tool_names: &[String]) -> AriaResult<BulkRegistrationResult> {
        debug!("Bulk registering {} tools", tool_names.len());

        let mut successful = Vec::new();
        let mut failed = Vec::new();
        let mut skipped = Vec::new();

        for tool_name in tool_names {
            match self.resolve_unknown_tool(tool_name).await? {
                ToolResolutionResult::AlreadyRegistered(_) => {
                    skipped.push(tool_name.clone());
                }
                ToolResolutionResult::AutoRegistered(_) => {
                    successful.push(tool_name.clone());
                }
                ToolResolutionResult::NotFound => {
                    failed.push(tool_name.clone());
                }
            }
        }

        info!("Bulk registration complete: {} successful, {} failed, {} skipped", 
              successful.len(), failed.len(), skipped.len());

        Ok(BulkRegistrationResult {
            successful,
            failed,
            skipped,
        })
    }

    /// Pre-warm the discovery cache
    pub async fn pre_warm_cache(&self) -> AriaResult<()> {
        info!("Pre-warming bundle discovery cache");
        self.discovery.refresh_cache().await?;
        info!("Bundle discovery cache pre-warmed");
        Ok(())
    }

    /// Get tool resolution statistics
    pub async fn get_resolution_stats(&self) -> AriaResult<ToolResolutionStats> {
        let registered_count = self.tool_registry.list_available_tools().await?.len();
        let bundle_stats = self.bundle_registry.get_bundle_tool_stats().await;
        let discoverable_count = self.discovery.get_all_available_tools().await?.len();

        Ok(ToolResolutionStats {
            total_registered_tools: registered_count,
            total_bundle_tools: bundle_stats.total_bundle_tools,
            total_discoverable_tools: discoverable_count,
            unique_bundles: bundle_stats.unique_bundles,
            auto_registration_enabled: self.auto_registration_enabled,
        })
    }
}

/// Summary of all available tools from different sources
#[derive(Debug, Clone)]
pub struct AvailableToolsSummary {
    pub registered_tools: Vec<String>,
    pub discoverable_tools: Vec<String>,
    pub auto_registration_enabled: bool,
}

/// Result of bulk tool registration
#[derive(Debug, Clone)]
pub struct BulkRegistrationResult {
    pub successful: Vec<String>,
    pub failed: Vec<String>,
    pub skipped: Vec<String>,
}

/// Tool resolution statistics
#[derive(Debug, Clone)]
pub struct ToolResolutionStats {
    pub total_registered_tools: usize,
    pub total_bundle_tools: usize,
    pub total_discoverable_tools: usize,
    pub unique_bundles: usize,
    pub auto_registration_enabled: bool,
}

/// Extension trait for ExecutionEngine integration
pub trait ToolResolverExtension {
    /// Resolve tool before execution
    async fn resolve_tool_for_execution(&self, tool_name: &str) -> AriaResult<RegistryEntry>;
}

impl ToolResolverExtension for ToolResolver {
    /// Resolve tool before execution, with auto-registration if needed
    async fn resolve_tool_for_execution(&self, tool_name: &str) -> AriaResult<RegistryEntry> {
        match self.resolve_unknown_tool(tool_name).await? {
            ToolResolutionResult::AlreadyRegistered(entry) |
            ToolResolutionResult::AutoRegistered(entry) => Ok(entry),
            ToolResolutionResult::NotFound => {
                Err(AriaError::new(
                    ErrorCode::ToolNotFound,
                    ErrorCategory::Tool,
                    ErrorSeverity::Medium,
                    &format!("Tool '{}' not found in registry or bundles", tool_name),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle_discovery::BundleToolDiscovery;
    use crate::engines::tool_registry::bundle_integration::BundleToolRegistry;
    use crate::pkg_store::PackageStore;
    use std::sync::Arc;
    use std::collections::HashMap;
    use tokio::sync::RwLock;

    async fn create_test_resolver() -> ToolResolver {
        let pkg_store = Arc::new(PackageStore::new().await.unwrap());
        let discovery = Arc::new(BundleToolDiscovery::new(pkg_store));
        let tools = Arc::new(RwLock::new(HashMap::new()));
        let bundle_registry = Arc::new(BundleToolRegistry::new(tools.clone(), discovery.clone()));
        
        // Create a mock tool registry
        let llm_handler = Arc::new(crate::engines::llm::LLMHandler::new(Default::default()).await.unwrap());
        let quilt_service = Arc::new(tokio::sync::Mutex::new(
            crate::engines::container::quilt::QuiltService::new("test-path".to_string()).await.unwrap()
        ));
        let tool_registry = Arc::new(ToolRegistry::new(llm_handler, quilt_service).await);

        ToolResolver::new(tool_registry, bundle_registry, discovery)
    }

    #[tokio::test]
    async fn test_tool_resolver_creation() {
        let resolver = create_test_resolver().await;
        assert!(resolver.is_auto_registration_enabled());
    }

    #[tokio::test]
    async fn test_auto_registration_toggle() {
        let mut resolver = create_test_resolver().await;
        
        assert!(resolver.is_auto_registration_enabled());
        resolver.set_auto_registration(false);
        assert!(!resolver.is_auto_registration_enabled());
        resolver.set_auto_registration(true);
        assert!(resolver.is_auto_registration_enabled());
    }

    #[tokio::test]
    async fn test_resolution_stats() {
        let resolver = create_test_resolver().await;
        let stats = resolver.get_resolution_stats().await.unwrap();
        
        assert!(stats.total_registered_tools >= 0);
        assert_eq!(stats.total_bundle_tools, 0); // No bundles in test
        assert!(stats.auto_registration_enabled);
    }
} 