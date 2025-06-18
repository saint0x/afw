/*!
# Bundle Tool Discovery Service

Fast tool discovery service for .aria bundles with intelligent caching and
automatic tool registration capabilities.
*/

use crate::errors::{AriaError, AriaResult, ErrorCategory, ErrorCode, ErrorSeverity};
use crate::engines::tool_registry::{ToolManifest, SecurityLevel};
use pkg_store::PackageStore;
use pkg_store::bundle::{LoadedBundle, AriaManifest, ToolManifest as BundleToolManifest};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Bundle tool discovery cache entry
#[derive(Debug, Clone)]
pub struct BundleToolEntry {
    pub bundle_hash: String,
    pub bundle_name: String,
    pub bundle_version: String,
    pub tool_manifest: ToolManifest,
    pub cached_at: u64,
}

/// Bundle manifest cache for fast tool lookup
#[derive(Debug, Clone)]
pub struct BundleManifestCache {
    /// bundle_hash -> manifest
    manifests: HashMap<String, AriaManifest>,
    /// tool_name -> [bundle_hashes]
    tool_index: HashMap<String, Vec<String>>,
    /// Last cache update timestamp
    last_updated: u64,
}

impl BundleManifestCache {
    pub fn new() -> Self {
        Self {
            manifests: HashMap::new(),
            tool_index: HashMap::new(),
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Add a bundle manifest to the cache
    pub fn add_bundle(&mut self, bundle_hash: String, manifest: AriaManifest) {
        // Update tool index
        for tool in &manifest.tools {
            self.tool_index
                .entry(tool.name.clone())
                .or_insert_with(Vec::new)
                .push(bundle_hash.clone());
        }

        // Store manifest
        self.manifests.insert(bundle_hash, manifest);
        self.last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Get bundles that contain a specific tool
    pub fn get_tool_bundles(&self, tool_name: &str) -> Vec<String> {
        self.tool_index
            .get(tool_name)
            .cloned()
            .unwrap_or_default()
    }

    /// Get all available tools
    pub fn get_all_tools(&self) -> Vec<String> {
        self.tool_index.keys().cloned().collect()
    }

    /// Get manifest for a bundle
    pub fn get_manifest(&self, bundle_hash: &str) -> Option<&AriaManifest> {
        self.manifests.get(bundle_hash)
    }

    /// Check if cache needs refresh
    pub fn needs_refresh(&self, max_age_seconds: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now - self.last_updated > max_age_seconds
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.manifests.clear();
        self.tool_index.clear();
        self.last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}

/// Bundle tool discovery service
pub struct BundleToolDiscovery {
    pkg_store: Arc<PackageStore>,
    cache: Arc<RwLock<BundleManifestCache>>,
    /// Cache TTL in seconds
    cache_ttl: u64,
}

impl BundleToolDiscovery {
    /// Create a new bundle tool discovery service
    pub fn new(pkg_store: Arc<PackageStore>) -> Self {
        Self {
            pkg_store,
            cache: Arc::new(RwLock::new(BundleManifestCache::new())),
            cache_ttl: 300, // 5 minutes default
        }
    }

    /// Discover a tool in available bundles
    pub async fn discover_tool(&self, tool_name: &str) -> AriaResult<Option<BundleToolEntry>> {
        debug!("Discovering tool: {}", tool_name);

        // Refresh cache if needed
        self.refresh_cache_if_needed().await?;

        let cache = self.cache.read().await;
        let bundle_hashes = cache.get_tool_bundles(tool_name);

        if bundle_hashes.is_empty() {
            debug!("Tool '{}' not found in any bundles", tool_name);
            return Ok(None);
        }

        // Return the first match (could be extended to support priority)
        let bundle_hash = &bundle_hashes[0];
        let manifest = cache
            .get_manifest(bundle_hash)
            .ok_or_else(|| {
                AriaError::new(
                    ErrorCode::InternalError,
                    ErrorCategory::Bundle,
                    ErrorSeverity::Medium,
                    &format!("Bundle manifest missing for hash: {}", bundle_hash),
                )
            })?;

        let tool_manifest = manifest
            .tools
            .iter()
            .find(|t| t.name == tool_name)
            .ok_or_else(|| {
                AriaError::new(
                    ErrorCode::ToolNotFound,
                    ErrorCategory::Tool,
                    ErrorSeverity::Medium,
                    &format!("Tool '{}' not found in bundle manifest", tool_name),
                )
            })?;

        let bundle_tool_entry = BundleToolEntry {
            bundle_hash: bundle_hash.clone(),
            bundle_name: manifest.name.clone(),
            bundle_version: manifest.version.clone(),
            tool_manifest: self.convert_tool_manifest(tool_manifest, bundle_hash)?,
            cached_at: cache.last_updated,
        };

        info!("Discovered tool '{}' in bundle '{}'", tool_name, manifest.name);
        Ok(Some(bundle_tool_entry))
    }

    /// Scan all available bundles and return tool summary
    pub async fn scan_available_bundles(&self) -> AriaResult<Vec<(String, Vec<String>)>> {
        debug!("Scanning available bundles");

        self.refresh_cache_if_needed().await?;

        let cache = self.cache.read().await;
        let mut results = Vec::new();

        for (bundle_hash, manifest) in &cache.manifests {
            let tool_names = manifest.tools.iter().map(|t| t.name.clone()).collect();
            results.push((bundle_hash.clone(), tool_names));
        }

        info!("Scanned {} bundles", results.len());
        Ok(results)
    }

    /// Get all available tools from all bundles
    pub async fn get_all_available_tools(&self) -> AriaResult<Vec<String>> {
        self.refresh_cache_if_needed().await?;

        let cache = self.cache.read().await;
        Ok(cache.get_all_tools())
    }

    /// Force refresh the bundle cache
    pub async fn refresh_cache(&self) -> AriaResult<()> {
        info!("Refreshing bundle cache");

        let bundle_hashes = self.pkg_store.list_bundles().await.map_err(|e| {
            AriaError::new(
                ErrorCode::StorageError,
                ErrorCategory::Bundle,
                ErrorSeverity::High,
                &format!("Failed to list bundles: {}", e),
            )
        })?;

        let mut cache = self.cache.write().await;
        cache.clear();

        for bundle_hash in bundle_hashes.iter() {
            match self.load_bundle_manifest(bundle_hash).await {
                Ok(manifest) => {
                    cache.add_bundle(bundle_hash.clone(), manifest);
                }
                Err(e) => {
                    warn!("Failed to load bundle manifest for {}: {}", bundle_hash, e);
                }
            }
        }

        info!("Bundle cache refreshed with {} bundles", cache.manifests.len());
        Ok(())
    }

    /// Check if cache needs refresh and refresh if necessary
    async fn refresh_cache_if_needed(&self) -> AriaResult<()> {
        let cache = self.cache.read().await;
        if cache.needs_refresh(self.cache_ttl) {
            drop(cache); // Release read lock
            self.refresh_cache().await?;
        }
        Ok(())
    }

    /// Load a bundle manifest from storage
    async fn load_bundle_manifest(&self, bundle_hash: &str) -> AriaResult<AriaManifest> {
        let bundle_data = self
            .pkg_store
            .get_bundle(bundle_hash)
            .await
            .map_err(|e| {
                AriaError::new(
                    ErrorCode::StorageError,
                    ErrorCategory::Bundle,
                    ErrorSeverity::High,
                    &format!("Failed to get bundle from storage: {}", e),
                )
            })?
            .ok_or_else(|| {
                AriaError::new(
                    ErrorCode::BundleNotFound,
                    ErrorCategory::Bundle,
                    ErrorSeverity::Medium,
                    &format!("Bundle not found: {}", bundle_hash),
                )
            })?;

        // Create temporary file for AriaBundle::load_from_file
        let temp_path = format!("/tmp/bundle_{}.aria", bundle_hash);
        tokio::fs::write(&temp_path, bundle_data).await.map_err(|e| {
            AriaError::new(
                ErrorCode::IoError,
                ErrorCategory::Bundle,
                ErrorSeverity::High,
                &format!("Failed to write temporary bundle file: {}", e),
            )
        })?;

        let bundle = LoadedBundle::load_from_file(&temp_path)
            .await
            .map_err(|e| {
                AriaError::new(
                    ErrorCode::BundleLoadError,
                    ErrorCategory::Bundle,
                    ErrorSeverity::High,
                    &format!("Failed to load bundle: {}", e),
                )
            })?;

        // Clean up temporary file
        let _ = tokio::fs::remove_file(&temp_path).await;

        Ok(bundle.manifest)
    }

    /// Convert bundle tool manifest to runtime tool manifest
    fn convert_tool_manifest(
        &self,
        bundle_manifest: &BundleToolManifest,
        bundle_hash: &str,
    ) -> AriaResult<ToolManifest> {
        // Convert inputs HashMap to JSON schema
        let parameters = if bundle_manifest.inputs.is_empty() {
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            })
        } else {
            let mut properties = serde_json::Map::new();
            let mut required = Vec::new();

            for (key, type_hint) in &bundle_manifest.inputs {
                properties.insert(
                    key.clone(),
                    serde_json::json!({
                        "type": type_hint,
                        "description": format!("Parameter: {}", key)
                    }),
                );
                required.push(key.clone());
            }

            serde_json::json!({
                "type": "object",
                "properties": properties,
                "required": required
            })
        };

        Ok(ToolManifest {
            name: bundle_manifest.name.clone(),
            description: bundle_manifest.description.clone(),
            parameters,
            entry_point: format!("implementations/tools/{}.js", bundle_manifest.name),
            capabilities: vec!["custom".to_string()], // Default capability
            resource_requirements: crate::types::ResourceRequirements::default(),
            security_level: SecurityLevel::Limited, // Default security level for bundle tools
        })
    }

    /// Set cache TTL (for testing and configuration)
    pub fn set_cache_ttl(&mut self, ttl_seconds: u64) {
        self.cache_ttl = ttl_seconds;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pkg_store::PackageStore;

    #[tokio::test]
    async fn test_bundle_manifest_cache() {
        let mut cache = BundleManifestCache::new();

        let manifest = ar_c::compiler::schema::AriaManifest {
            name: "test-bundle".to_string(),
            version: "1.0.0".to_string(),
            tools: vec![ar_c::compiler::schema::ToolManifest {
                name: "test-tool".to_string(),
                description: "A test tool".to_string(),
                inputs: HashMap::new(),
            }],
            agents: vec![],
            teams: vec![],
            pipelines: vec![],
        };

        cache.add_bundle("hash123".to_string(), manifest);

        assert_eq!(cache.get_tool_bundles("test-tool"), vec!["hash123"]);
        assert!(cache.get_manifest("hash123").is_some());
    }

    #[tokio::test]
    async fn test_bundle_tool_discovery_creation() {
        let pkg_store = Arc::new(PackageStore::new().await.unwrap());
        let discovery = BundleToolDiscovery::new(pkg_store);

        assert_eq!(discovery.cache_ttl, 300);
    }
} 