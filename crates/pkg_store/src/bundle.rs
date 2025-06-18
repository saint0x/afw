/*!
# Bundle Schema and Loading

Defines the .aria bundle format schema and loading capabilities.
The firmware consumes pre-compiled .aria bundles without depending on the compiler.
*/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::io::Read;
use zip::ZipArchive;

/// Aria bundle manifest schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AriaManifest {
    pub name: String,
    pub version: String,
    pub tools: Vec<ToolManifest>,
    pub agents: Vec<AgentManifest>,
    pub teams: Vec<TeamManifest>,
    pub pipelines: Vec<PipelineManifest>,
}

/// Tool manifest schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManifest {
    pub name: String,
    pub description: String,
    pub inputs: HashMap<String, String>,
}

/// Agent manifest schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManifest {
    pub name: String,
    pub description: String,
    pub tools: Vec<String>,
}

/// Team manifest schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamManifest {
    pub name: String,
    pub description: String,
    pub members: Vec<String>,
}

/// Pipeline manifest schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineManifest {
    pub name: String,
    pub description: String,
}

/// Bundle metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleMetadata {
    pub created_at: u64,
    pub compiler_version: String,
    pub bundle_version: String,
}

impl Default for BundleMetadata {
    fn default() -> Self {
        Self {
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            compiler_version: "unknown".to_string(),
            bundle_version: "1.0.0".to_string(),
        }
    }
}

/// Bundle loading error
#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Bundle validation error: {0}")]
    Validation(String),
}

/// Loaded bundle representation
#[derive(Debug, Clone)]
pub struct LoadedBundle {
    pub manifest: AriaManifest,
    pub source_files: HashMap<PathBuf, String>,
    pub metadata: BundleMetadata,
}

impl LoadedBundle {
    /// Load bundle from .aria file (ZIP format)
    pub async fn load_from_file(path: &str) -> Result<Self, BundleError> {
        let file = std::fs::File::open(path)?;
        let mut archive = ZipArchive::new(file)?;

        // Read manifest.json
        let manifest = {
            let mut manifest_file = archive.by_name("manifest.json")?;
            let mut manifest_content = String::new();
            manifest_file.read_to_string(&mut manifest_content)?;
            serde_json::from_str::<AriaManifest>(&manifest_content)?
        };

        // Read source files from _sources directory
        let mut source_files = HashMap::new();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let file_path = file.name().to_string();
            
            // Only read source files and main entry
            if file_path.starts_with("_sources/") || 
               file_path == "main.tsx" || 
               file_path == "main.ts" || 
               file_path == "index.ts" {
                
                let mut content = String::new();
                if file.read_to_string(&mut content).is_ok() {
                    let path_buf = PathBuf::from(file_path.trim_start_matches("_sources/"));
                    source_files.insert(path_buf, content);
                }
            }
        }

        // Try to read metadata
        let metadata = match archive.by_name("metadata/build.json") {
            Ok(mut metadata_file) => {
                let mut metadata_content = String::new();
                if metadata_file.read_to_string(&mut metadata_content).is_ok() {
                    serde_json::from_str(&metadata_content).unwrap_or_default()
                } else {
                    BundleMetadata::default()
                }
            }
            Err(_) => BundleMetadata::default(),
        };

        let bundle = Self {
            manifest,
            source_files,
            metadata,
        };

        bundle.validate()?;
        Ok(bundle)
    }

    /// Get the main entry file path
    pub fn get_main_entry(&self) -> Option<&str> {
        if self.source_files.contains_key(&PathBuf::from("main.tsx")) {
            Some("main.tsx")
        } else if self.source_files.contains_key(&PathBuf::from("main.ts")) {
            Some("main.ts")
        } else if self.source_files.contains_key(&PathBuf::from("index.ts")) {
            Some("index.ts")
        } else {
            None
        }
    }

    /// Validate bundle structure
    pub fn validate(&self) -> Result<(), BundleError> {
        if self.manifest.name.is_empty() {
            return Err(BundleError::Validation("Bundle name cannot be empty".to_string()));
        }

        if self.manifest.version.is_empty() {
            return Err(BundleError::Validation("Bundle version cannot be empty".to_string()));
        }

        // Check for main entry point
        if self.get_main_entry().is_none() {
            return Err(BundleError::Validation(
                "Bundle must contain a main entry point (main.tsx, main.ts, or index.ts)".to_string()
            ));
        }

        Ok(())
    }

    /// Generate package.json content for the bundle
    pub fn generate_package_json(&self) -> String {
        let package = serde_json::json!({
            "name": self.manifest.name,
            "version": self.manifest.version,
            "description": format!("Aria bundle with {} tools and {} agents", 
                self.manifest.tools.len(), 
                self.manifest.agents.len()
            ),
            "main": self.get_main_entry().unwrap_or("index.ts"),
            "type": "module",
            "scripts": {
                "start": format!("bun run {}", self.get_main_entry().unwrap_or("index.ts")),
                "dev": format!("bun run --watch {}", self.get_main_entry().unwrap_or("index.ts"))
            },
            "dependencies": {
                "@aria/runtime": "^0.1.0",
                "bun-types": "latest"
            },
            "devDependencies": {
                "typescript": "^5.0.0"
            }
        });

        serde_json::to_string_pretty(&package).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_serialization() {
        let manifest = AriaManifest {
            name: "test-bundle".to_string(),
            version: "1.0.0".to_string(),
            tools: vec![ToolManifest {
                name: "test-tool".to_string(),
                description: "A test tool".to_string(),
                inputs: HashMap::new(),
            }],
            agents: vec![],
            teams: vec![],
            pipelines: vec![],
        };

        let json = serde_json::to_string(&manifest).unwrap();
        let deserialized: AriaManifest = serde_json::from_str(&json).unwrap();
        
        assert_eq!(manifest.name, deserialized.name);
        assert_eq!(manifest.tools.len(), deserialized.tools.len());
    }

    #[test]
    fn test_bundle_metadata_default() {
        let metadata = BundleMetadata::default();
        assert_eq!(metadata.compiler_version, "unknown");
        assert_eq!(metadata.bundle_version, "1.0.0");
        assert!(metadata.created_at > 0);
    }
} 