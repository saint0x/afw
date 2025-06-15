use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use zip::{ZipWriter, ZipArchive};
use std::io::Write;
use std::fs::File;
use zip::write::{FileOptions};
use zip::CompressionMethod;

use crate::compiler::{Implementation, ImplementationDetails};
use crate::compiler::schema::{AriaManifest, AgentManifest};

/// Aria bundle containing manifest and implementations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AriaBundle {
    pub manifest: AriaManifest,
    pub implementations: HashMap<String, Implementation>,
    #[serde(skip)]
    pub compiled_code: HashMap<PathBuf, String>,
    pub metadata: BundleMetadata,
}

impl AriaBundle {
    /// Create a new bundle from manifest and implementations
    pub fn create(
        manifest: AriaManifest,
        implementations: Vec<Implementation>,
        compiled_code: HashMap<PathBuf, String>,
    ) -> Result<Self> {
        let mut impl_map = HashMap::new();
        
        for implementation in implementations {
            impl_map.insert(implementation.name.clone(), implementation);
        }
        
        Ok(Self {
            manifest,
            implementations: impl_map,
            compiled_code,
            metadata: BundleMetadata::new(),
        })
    }
    
    /// Save bundle to a .aria file (ZIP format)
    pub async fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        let file = std::fs::File::create(path)?;
        let mut zip = ZipWriter::new(file);
        
        // Add manifest.json
        let options: FileOptions<'_, ()> = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o755);
        
        zip.start_file("manifest.json", options)?;
        let manifest_json = serde_json::to_string_pretty(&self.manifest)?;
        zip.write_all(manifest_json.as_bytes())?;
        
        // --- Re-Export Strategy ---
        // 1. Write all unique, transpiled source files to a `_sources` directory.
        zip.add_directory("implementations/_sources", options)?;
        let mut source_map: HashMap<PathBuf, String> = HashMap::new();
        let mut i = 0;
        for (original_path, code) in &self.compiled_code {
            let source_bundle_path = format!("implementations/_sources/{}.js", i);
            zip.start_file(&source_bundle_path, options)?;
            zip.write_all(code.as_bytes())?;
            source_map.insert(original_path.clone(), source_bundle_path);
            i += 1;
        }

        // 2. Create re-export stubs for each implementation.
        for (name, implementation) in &self.implementations {
            if let Some(source_bundle_path) = source_map.get(&implementation.source_file_path) {
                let (implementation_type_dir, relative_path) = match &implementation.details {
                    ImplementationDetails::Tool(_) => ("tools", "../../_sources/"),
                    ImplementationDetails::Agent(_) => ("agents", "../../_sources/"),
                    ImplementationDetails::Team(_) => ("teams", "../../_sources/"),
                    ImplementationDetails::Pipeline(_) => ("pipelines", "../../_sources/"),
                };

                let stub_path = format!("implementations/{}/{}.js", implementation_type_dir, name);
                let source_file_name = Path::new(source_bundle_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                let re_export_content = format!("export * from '{}{}';", relative_path, source_file_name);
                
                zip.start_file(&stub_path, options)?;
                zip.write_all(re_export_content.as_bytes())?;
            }
        }
        
        // Add package.json for dependencies
        let package_json = self.generate_package_json();
        zip.start_file("package.json", options)?;
        zip.write_all(package_json.as_bytes())?;
        
        // Add metadata
        zip.start_file("metadata/build.json", options)?;
        let metadata_json = serde_json::to_string_pretty(&self.metadata)?;
        zip.write_all(metadata_json.as_bytes())?;
        
        zip.finish()?;
        
        Ok(())
    }
    
    /// Load bundle from a .aria file
    pub async fn load_from_file(path: &str) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let mut archive = ZipArchive::new(file)?;
        
        // Read manifest
        let manifest = {
            let mut manifest_file = archive.by_name("manifest.json")?;
            let mut manifest_content = String::new();
            std::io::Read::read_to_string(&mut manifest_file, &mut manifest_content)?;
            serde_json::from_str::<AriaManifest>(&manifest_content)?
        };
        
        // Read implementations (basic loading for now)
        let implementations = HashMap::new();
        
        // Try to read metadata
        let metadata = match archive.by_name("metadata/build.json") {
            Ok(mut metadata_file) => {
                let mut metadata_content = String::new();
                std::io::Read::read_to_string(&mut metadata_file, &mut metadata_content)?;
                serde_json::from_str(&metadata_content).unwrap_or_else(|_| BundleMetadata::new())
            }
            Err(_) => BundleMetadata::new(),
        };
        
        Ok(Self {
            manifest,
            implementations,
            compiled_code: HashMap::new(),
            metadata,
        })
    }
    
    /// Generate package.json for the bundle
    fn generate_package_json(&self) -> String {
        let package = PackageJson {
            name: self.manifest.name.clone(),
            version: self.manifest.version.clone(),
            description: format!("Aria bundle with {} tools and {} agents", 
                self.manifest.tools.len(), 
                self.manifest.agents.len()
            ),
            main: "implementations/index.js".to_string(),
            dependencies: self.extract_dependencies(),
        };
        
        serde_json::to_string_pretty(&package).unwrap_or_else(|_| "{}".to_string())
    }
    
    /// Extract dependencies from implementations
    fn extract_dependencies(&self) -> HashMap<String, String> {
        let mut deps = HashMap::new();
        
        // Add common Aria runtime dependencies
        deps.insert("@aria/runtime".to_string(), "^0.1.0".to_string());
        
        // TODO: Extract actual dependencies from implementations
        // This would involve parsing import statements and resolving versions
        
        deps
    }
    
    /// Get bundle size in bytes
    pub async fn get_size(&self, path: &Path) -> Result<u64> {
        let metadata = fs::metadata(path).await?;
        Ok(metadata.len())
    }
    
    /// Validate bundle integrity
    pub fn validate(&self) -> Result<Vec<String>> {
        let mut issues = Vec::new();
        
        // Check manifest completeness
        if self.manifest.name.is_empty() {
            issues.push("Bundle name is empty".to_string());
        }
        
        if self.manifest.version.is_empty() {
            issues.push("Bundle version is empty".to_string());
        }
        
        // Check implementations exist for manifest entries
        for tool in &self.manifest.tools {
            if !self.implementations.contains_key(&tool.name) {
                issues.push(format!("Missing implementation for tool: {}", tool.name));
            }
        }
        
        for agent in &self.manifest.agents {
            if !self.implementations.contains_key(&agent.name) {
                issues.push(format!("Missing implementation for agent: {}", agent.name));
            }
        }
        
        // Check for orphaned implementations
        for (name, implementation) in &self.implementations {
            let found_in_manifest = match &implementation.details {
                ImplementationDetails::Tool(_) => {
                    self.manifest.tools.iter().any(|t| &t.name == name)
                }
                ImplementationDetails::Agent(_) => {
                    self.manifest.agents.iter().any(|a| &a.name == name)
                }
                ImplementationDetails::Team(_) => {
                    self.manifest.teams.iter().any(|t| &t.name == name)
                }
                ImplementationDetails::Pipeline(_) => {
                    self.manifest.pipelines.iter().any(|p| &p.name == name)
                }
            };
            
            if !found_in_manifest {
                issues.push(format!("Implementation '{}' not found in manifest", name));
            }
        }
        
        Ok(issues)
    }

    pub fn add_agent(&mut self, agent: AgentManifest) {
        self.manifest.agents.push(agent);
    }
}

/// Bundle metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleMetadata {
    pub created_at: String,
    pub compiler_version: String,
    pub source_language: String,
    pub build_hash: String,
}

impl BundleMetadata {
    pub fn new() -> Self {
        Self {
            created_at: chrono::Utc::now().to_rfc3339(),
            compiler_version: env!("CARGO_PKG_VERSION").to_string(),
            source_language: "typescript".to_string(),
            build_hash: "placeholder".to_string(), // TODO: Generate actual hash
        }
    }
}

impl Default for BundleMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Package.json structure for bundle dependencies
#[derive(Debug, Serialize)]
struct PackageJson {
    name: String,
    version: String,
    description: String,
    main: String,
    dependencies: HashMap<String, String>,
}

/// Bundle statistics for reporting
#[derive(Debug)]
pub struct BundleStats {
    pub size_kb: f64,
    pub tools_count: usize,
    pub agents_count: usize,
    // pub teams_count: usize,
    // pub applications_count: usize,
    pub compression_ratio: f64,
}

impl AriaBundle {
    /// Get bundle statistics
    pub async fn get_stats(&self, bundle_path: &Path) -> Result<BundleStats> {
        let size = self.get_size(bundle_path).await?;
        
        Ok(BundleStats {
            size_kb: size as f64 / 1024.0,
            tools_count: self.manifest.tools.len(),
            agents_count: self.manifest.agents.len(),
            // teams_count: self.manifest.teams.len(),
            // applications_count: self.manifest.applications.len(),
            compression_ratio: 0.7, // TODO: Calculate actual compression ratio
        })
    }
    
    /// Extract a specific implementation
    pub fn get_implementation(&self, name: &str) -> Option<&Implementation> {
        self.implementations.get(name)
    }
    
    /// List all tool names
    pub fn list_tools(&self) -> Vec<&str> {
        self.manifest.tools.iter().map(|t| t.name.as_str()).collect()
    }
    
    /// List all agent names
    pub fn list_agents(&self) -> Vec<&str> {
        self.manifest.agents.iter().map(|a| a.name.as_str()).collect()
    }
}

pub fn create_bundle(manifest: &AriaManifest, implementations: &HashMap<String, String>) -> Result<()> {
    let bundle_filename = format!("{}.aria", manifest.name);
    let path = Path::new(&bundle_filename);
    let file = File::create(&path)?;

    let mut zip = ZipWriter::new(file);

    let options: FileOptions<'_, ()> = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o755);

    zip.start_file("manifest.json", options)?;
    let manifest_json = serde_json::to_string_pretty(manifest)?;
    zip.write_all(manifest_json.as_bytes())?;

    for (name, code) in implementations {
        zip.start_file(format!("implementations/{}.js", name), options)?;
        zip.write_all(code.as_bytes())?;
    }

    zip.finish()?;

    Ok(())
} 