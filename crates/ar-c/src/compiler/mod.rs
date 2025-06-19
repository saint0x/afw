pub mod typescript;
pub mod schema;

use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use swc_core::common::{SourceMap, sync::Lrc};

use self::typescript::TypeScriptCompiler;
use self::typescript::visitor::ExtractedItem;
use crate::compiler::schema::{AgentManifest, ToolManifest, AriaManifest, TeamManifest, PipelineManifest};
use crate::bundle::AriaBundle;

/// Main Aria compiler that orchestrates the compilation process
pub struct AriaCompiler {
    typescript_compiler: Arc<TypeScriptCompiler>,
    // Future: dsl_compiler: dsl::DslCompiler,
}

impl AriaCompiler {
    /// Create a new Aria compiler instance
    pub fn new() -> Self {
        let cm = Lrc::new(SourceMap::default());
        Self {
            typescript_compiler: Arc::new(TypeScriptCompiler::new(cm)),
        }
    }
    
    /// Compile a project from input path to output bundle
    pub async fn compile_project(
        &self,
        input_path: &str,
        output_path: &PathBuf,
        verbose: bool,
    ) -> Result<CompilationResult> {
        let start_time = std::time::Instant::now();
        
        // 1. Discover source files
        let sources = self.discover_sources(input_path).await?;
        
        if verbose {
            println!("Found {} source files", sources.len());
        }
        
        // 2. Compile based on source language
        let mut compiled_files: Vec<CompiledFile> = Vec::new();
        let mut warnings = Vec::new();
        
        for source in sources {
            match source.language {
                SourceLanguage::TypeScript => {
                    match self.typescript_compiler.compile_file(&source).await {
                        Ok(compiled) => compiled_files.push(compiled),
                        Err(e) => return Err(e),
                    }
                }
                SourceLanguage::AriaSDL => {
                    // Future: DSL compilation
                    // For now, skip DSL files
                    warnings.push(format!("Skipping DSL file (not yet implemented): {}", source.path.display()));
                }
            }
        }
        
        if compiled_files.iter().all(|f| f.items.is_empty()) {
            warnings.push("No decorated functions or classes found".to_string());
        }
        
        // 3. Process compiled files into implementations and a code map
        let mut implementations = Vec::new();
        let mut compiled_code_map: HashMap<PathBuf, String> = HashMap::new();

        for file in compiled_files {
            let source_path = file.source.path.clone();
            compiled_code_map.insert(source_path.clone(), file.javascript_code);

            for item in file.items {
                let (name, details) = match item {
                    ExtractedItem::Tool { manifest } => (manifest.name.clone(), ImplementationDetails::Tool(manifest)),
                    ExtractedItem::Agent { manifest } => (manifest.name.clone(), ImplementationDetails::Agent(manifest)),
                    ExtractedItem::Team { manifest } => (manifest.name.clone(), ImplementationDetails::Team(manifest)),
                    ExtractedItem::Pipeline { manifest } => (manifest.name.clone(), ImplementationDetails::Pipeline(manifest)),
                };
                implementations.push(Implementation {
                    name,
                    details,
                    source_file_path: source_path.clone(),
                });
            }
        }
        
        // 4. Generate manifest
        let manifest = self.generate_manifest(&implementations)?;
        
        // 5. Validate cross-references
        if let Err(e) = self.validate_cross_references(&manifest) {
            return Err(e);
        }
        
        // 6. Get metrics before moving implementations
        let source_files_count = compiled_code_map.len();
        
        // 7. Create bundle (this consumes implementations)
        let mut bundle = AriaBundle::create(
            manifest,
            implementations,
            compiled_code_map,
        )?;
        
        // 8. Write to output
        bundle.save_to_file(output_path).await?;
        
        // 9. Calculate metrics
        let compilation_time = start_time.elapsed();
        let bundle_size = tokio::fs::metadata(output_path).await?.len();
        
        Ok(CompilationResult {
            bundle_size_kb: bundle_size as f64 / 1024.0,
            tools_count: bundle.manifest.tools.len(),
            agents_count: bundle.manifest.agents.len(),
            teams_count: bundle.manifest.teams.len(),
            pipelines_count: bundle.manifest.pipelines.len(),
            source_files_count,
            dependencies_count: 0, // TODO: Calculate actual dependencies
            compilation_time_secs: compilation_time.as_secs_f64(),
            compression_ratio: 0.7, // TODO: Calculate actual compression
            warnings,
        })
    }
    
    /// Discover source files in the input path
    async fn discover_sources(&self, input_path: &str) -> Result<Vec<SourceFile>> {
        let mut sources = Vec::new();
        let path = Path::new(input_path);
        
        if path.is_file() {
            // Single file
            let source = load_source_file(path).await?;
            sources.push(source);
        } else if path.is_dir() {
            // Directory - find all TypeScript files
            sources = discover_typescript_files(path).await?;
        } else {
            return Err(anyhow!("Input path does not exist: {}", input_path));
        }
        
        Ok(sources)
    }
    
    /// Generate manifest from implementations
    fn generate_manifest(&self, implementations: &[Implementation]) -> Result<AriaManifest> {
        let mut tools = Vec::new();
        let mut agents = Vec::new();
        let mut teams = Vec::new();
        let mut pipelines = Vec::new();
        
        for implementation in implementations {
            match &implementation.details {
                ImplementationDetails::Tool(tool_manifest) => tools.push(tool_manifest.clone()),
                ImplementationDetails::Agent(agent_manifest) => agents.push(agent_manifest.clone()),
                ImplementationDetails::Team(team_manifest) => teams.push(team_manifest.clone()),
                ImplementationDetails::Pipeline(pipeline_manifest) => pipelines.push(pipeline_manifest.clone()),
            }
        }
        
        Ok(AriaManifest {
            name: "Generated Bundle".to_string(), // TODO: Get from config
            version: "0.1.0".to_string(),
            tools,
            agents,
            teams,
            pipelines,
        })
    }

    /// Run a compilation check without creating a bundle.
    pub async fn check_project(
        &self,
        input_path: &str,
        verbose: bool,
    ) -> Result<CompilationResult> {
        let start_time = std::time::Instant::now();
        
        // 1. Discover source files
        let sources = self.discover_sources(input_path).await?;
        
        if verbose {
            println!("Found {} source files", sources.len());
        }
        
        // 2. Compile based on source language
        let mut compiled_files: Vec<CompiledFile> = Vec::new();
        let mut warnings = Vec::new();
        
        for source in sources {
            match source.language {
                SourceLanguage::TypeScript => {
                    match self.typescript_compiler.compile_file(&source).await {
                        Ok(compiled) => compiled_files.push(compiled),
                        Err(e) => return Err(e),
                    }
                }
                SourceLanguage::AriaSDL => {
                    warnings.push(format!("Skipping DSL file (not yet implemented): {}", source.path.display()));
                }
            }
        }
        
        if compiled_files.iter().all(|f| f.items.is_empty()) {
            warnings.push("No decorated functions or classes found".to_string());
        }
        
        // 3. Process compiled files into implementations
        let mut implementations = Vec::new();
        for file in &compiled_files {
            for item in &file.items {
                let (name, details) = match item {
                    ExtractedItem::Tool { manifest } => (manifest.name.clone(), ImplementationDetails::Tool(manifest.clone())),
                    ExtractedItem::Agent { manifest } => (manifest.name.clone(), ImplementationDetails::Agent(manifest.clone())),
                    ExtractedItem::Team { manifest } => (manifest.name.clone(), ImplementationDetails::Team(manifest.clone())),
                    ExtractedItem::Pipeline { manifest } => (manifest.name.clone(), ImplementationDetails::Pipeline(manifest.clone())),
                };
                implementations.push(Implementation {
                    name,
                    details,
                    source_file_path: file.source.path.clone(),
                });
            }
        }
        
        // 4. Generate manifest
        let manifest = self.generate_manifest(&implementations)?;
        
        // 5. Validate cross-references
        if let Err(e) = self.validate_cross_references(&manifest) {
            return Err(e);
        }
        
        let compilation_time = start_time.elapsed();
        
        Ok(CompilationResult {
            bundle_size_kb: 0.0, // Not applicable
            tools_count: manifest.tools.len(),
            agents_count: manifest.agents.len(),
            teams_count: manifest.teams.len(),
            pipelines_count: manifest.pipelines.len(),
            source_files_count: compiled_files.len(),
            dependencies_count: 0,
            compilation_time_secs: compilation_time.as_secs_f64(),
            compression_ratio: 0.0, // Not applicable
            warnings,
        })
    }

    /// Validates that all cross-references within the manifest are valid.
    /// For example, ensures that agents only reference tools that are actually defined.
    fn validate_cross_references(&self, manifest: &AriaManifest) -> Result<()> {
        let mut errors = Vec::new();

        // --- Tool validation ---
        let defined_tools: HashSet<_> = manifest.tools.iter().map(|t| &t.name).collect();

        for agent in &manifest.agents {
            for tool_name in &agent.tools {
                if !defined_tools.contains(tool_name) {
                    errors.push(format!(
                        "Agent '{}' references undefined tool: '{}'",
                        agent.name, tool_name
                    ));
                }
            }
        }

        // --- (Future) Team validation ---
        // let defined_agents: HashSet<_> = manifest.agents.iter().map(|a| &a.name).collect();
        // for team in &manifest.teams {
        //     for agent_name in &team.agents {
        //         if !defined_agents.contains(agent_name) {
        //             errors.push(format!(
        //                 "Team '{}' references undefined agent: '{}'",
        //                 team.name, agent_name
        //             ));
        //         }
        //     }
        // }

        if !errors.is_empty() {
            return Err(anyhow!(
                "Cross-reference validation failed:\n - {}",
                errors.join("\n - ")
            ));
        }

        Ok(())
    }
}

impl Default for AriaCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Source file with detected language
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: PathBuf,
    pub content: String,
    pub language: SourceLanguage,
}

/// A file that has been compiled, containing its original source,
/// the resulting JavaScript, and the Aria items discovered within it.
#[derive(Debug)]
pub struct CompiledFile {
    pub source: SourceFile,
    pub javascript_code: String,
    pub items: Vec<ExtractedItem>,
}

/// Supported source languages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SourceLanguage {
    TypeScript,
    AriaSDL, // Future
}

/// Final, bundle-ready implementation data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Implementation {
    pub name: String,
    pub details: ImplementationDetails,
    pub source_file_path: PathBuf,
}

/// Enum to hold manifest details for different implementation types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImplementationDetails {
    Tool(ToolManifest),
    Agent(AgentManifest),
    Team(TeamManifest),
    Pipeline(PipelineManifest),
}

/// Type of implementation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImplementationType {
    Function, // @tool functions
    Class,    // @agent classes
    Team,     // @team classes
}

/// Decorator metadata extracted from source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecoratorMetadata {
    pub decorator_type: String, // "tool", "agent", "team"
    pub properties: HashMap<String, serde_json::Value>,
}

/// Result of compilation process
#[derive(Debug)]
pub struct CompilationResult {
    pub bundle_size_kb: f64,
    pub tools_count: usize,
    pub agents_count: usize,
    pub teams_count: usize,
    pub pipelines_count: usize,
    pub source_files_count: usize,
    pub dependencies_count: usize,
    pub compilation_time_secs: f64,
    pub compression_ratio: f64,
    pub warnings: Vec<String>,
}

/// Discover TypeScript files in a directory
fn discover_typescript_files(dir: &Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<SourceFile>>> + Send + '_>> {
    Box::pin(async move {
        let mut sources = Vec::new();
        let mut entries = tokio::fs::read_dir(dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_dir() && !should_skip_directory(&path) {
                // Recursively search subdirectories
                let mut sub_sources = discover_typescript_files(&path).await?;
                sources.append(&mut sub_sources);
            } else if path.is_file() && is_typescript_file(&path) {
                let source = load_source_file(&path).await?;
                sources.push(source);
            }
        }

        Ok(sources)
    })
}

/// Load a single source file
async fn load_source_file(path: &Path) -> Result<SourceFile> {
    let canonical_path = std::fs::canonicalize(path)?;
    let content = tokio::fs::read_to_string(&canonical_path).await?;
    let language = detect_language(&canonical_path, &content);

    Ok(SourceFile {
        path: canonical_path,
        content,
        language,
    })
}

/// Check if directory should be skipped
fn should_skip_directory(path: &Path) -> bool {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        matches!(name, "node_modules" | "dist" | "target" | ".git" | ".next")
    } else {
        false
    }
}

/// Check if file is a TypeScript file
fn is_typescript_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        matches!(ext, "ts" | "tsx")
    } else {
        false
    }
}

/// Detect source language from file path and content
fn detect_language(path: &Path, _content: &str) -> SourceLanguage {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        match ext {
            "ts" | "tsx" => SourceLanguage::TypeScript,
            "aria" => SourceLanguage::AriaSDL, // Future
            _ => SourceLanguage::TypeScript, // Default
        }
    } else {
        SourceLanguage::TypeScript
    }
} 