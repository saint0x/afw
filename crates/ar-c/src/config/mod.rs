use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use std::path::Path;
use tokio::fs;

/// Project configuration loaded from aria.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub project: ProjectInfo,
    pub build: BuildConfig,
    pub runtime: RuntimeConfig,
}

impl ProjectConfig {
    /// Load configuration from aria.toml file
    pub async fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path).await?;
        let config: ProjectConfig = toml::from_str(&content)
            .map_err(|e| anyhow!("Failed to parse aria.toml: {}", e))?;
        
        // Validate configuration
        config.validate()?;
        
        Ok(config)
    }
    
    /// Save configuration to aria.toml file
    pub async fn save_to_file(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| anyhow!("Failed to serialize config: {}", e))?;
        
        fs::write(path, content).await?;
        Ok(())
    }
    
    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        if self.project.name.is_empty() {
            return Err(anyhow!("Project name cannot be empty"));
        }
        
        if self.project.version.is_empty() {
            return Err(anyhow!("Project version cannot be empty"));
        }
        
        // Validate build target
        match self.build.target.as_str() {
            "typescript" | "aria-dsl" => {},
            _ => return Err(anyhow!("Invalid build target: {}", self.build.target)),
        }
        
        Ok(())
    }
    
    /// Get the output path, resolving relative paths
    pub fn get_output_path(&self) -> Option<&str> {
        self.build.output.as_deref()
    }
    
    /// Get the source directories
    pub fn get_source_dirs(&self) -> Vec<&str> {
        self.build.source_dirs.iter().map(|s| s.as_str()).collect()
    }
    
    /// Check if watch mode is enabled by default
    pub fn is_watch_enabled(&self) -> bool {
        self.build.watch.unwrap_or(false)
    }
    
    /// Get exclude patterns for file discovery
    pub fn get_exclude_patterns(&self) -> Vec<&str> {
        self.build.exclude.iter().map(|s| s.as_str()).collect()
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            project: ProjectInfo {
                name: "aria-project".to_string(),
                version: "0.1.0".to_string(),
                description: "An Aria agentic application".to_string(),
                authors: vec![],
                license: None,
                repository: None,
            },
            build: BuildConfig {
                target: "typescript".to_string(),
                output: Some("dist/bundle.aria".to_string()),
                source_dirs: vec!["src".to_string()],
                exclude: vec![
                    "node_modules".to_string(),
                    "dist".to_string(),
                    "target".to_string(),
                    ".git".to_string(),
                ],
                watch: Some(false),
                optimization: Some(OptimizationLevel::Release),
            },
            runtime: RuntimeConfig {
                bun_version: "latest".to_string(),
                node_version: None,
                environment: vec![],
            },
        }
    }
}

/// Project information section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub authors: Vec<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
}

/// Build configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub target: String, // "typescript" or "aria-dsl"
    pub output: Option<String>,
    #[serde(default = "default_source_dirs")]
    pub source_dirs: Vec<String>,
    #[serde(default = "default_exclude_patterns")]
    pub exclude: Vec<String>,
    pub watch: Option<bool>,
    pub optimization: Option<OptimizationLevel>,
}

/// Runtime configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub bun_version: String,
    pub node_version: Option<String>,
    #[serde(default)]
    pub environment: Vec<EnvironmentVariable>,
}

/// Environment variable configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentVariable {
    pub name: String,
    pub value: String,
    pub required: Option<bool>,
}

/// Build optimization level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationLevel {
    #[serde(rename = "debug")]
    Debug,
    #[serde(rename = "release")]
    Release,
    #[serde(rename = "size")]
    Size,
}

/// Default source directories
fn default_source_dirs() -> Vec<String> {
    vec!["src".to_string()]
}

/// Default exclude patterns
fn default_exclude_patterns() -> Vec<String> {
    vec![
        "node_modules".to_string(),
        "dist".to_string(),
        "target".to_string(),
        ".git".to_string(),
        "*.log".to_string(),
        ".env".to_string(),
    ]
}

/// Configuration builder for programmatic config creation
pub struct ConfigBuilder {
    config: ProjectConfig,
}

impl ConfigBuilder {
    /// Create a new config builder
    pub fn new(name: &str) -> Self {
        let mut config = ProjectConfig::default();
        config.project.name = name.to_string();
        
        Self { config }
    }
    
    /// Set project version
    pub fn version(mut self, version: &str) -> Self {
        self.config.project.version = version.to_string();
        self
    }
    
    /// Set project description
    pub fn description(mut self, description: &str) -> Self {
        self.config.project.description = description.to_string();
        self
    }
    
    /// Add an author
    pub fn author(mut self, author: &str) -> Self {
        self.config.project.authors.push(author.to_string());
        self
    }
    
    /// Set build target
    pub fn target(mut self, target: &str) -> Self {
        self.config.build.target = target.to_string();
        self
    }
    
    /// Set output path
    pub fn output(mut self, output: &str) -> Self {
        self.config.build.output = Some(output.to_string());
        self
    }
    
    /// Add source directory
    pub fn source_dir(mut self, dir: &str) -> Self {
        self.config.build.source_dirs.push(dir.to_string());
        self
    }
    
    /// Set optimization level
    pub fn optimization(mut self, level: OptimizationLevel) -> Self {
        self.config.build.optimization = Some(level);
        self
    }
    
    /// Set runtime version
    pub fn bun_version(mut self, version: &str) -> Self {
        self.config.runtime.bun_version = version.to_string();
        self
    }
    
    /// Add environment variable
    pub fn env_var(mut self, name: &str, value: &str, required: bool) -> Self {
        self.config.runtime.environment.push(EnvironmentVariable {
            name: name.to_string(),
            value: value.to_string(),
            required: Some(required),
        });
        self
    }
    
    /// Build the configuration
    pub fn build(self) -> ProjectConfig {
        self.config
    }
}

/// Configuration templates for different project types
pub struct ConfigTemplates;

impl ConfigTemplates {
    /// Basic TypeScript project template
    pub fn basic_typescript(name: &str) -> ProjectConfig {
        ConfigBuilder::new(name)
            .description("A basic Aria TypeScript project")
            .target("typescript")
            .output(&format!("dist/{}.aria", name))
            .optimization(OptimizationLevel::Release)
            .bun_version("latest")
            .build()
    }
    
    /// Advanced TypeScript SDK project template
    pub fn typescript_sdk(name: &str) -> ProjectConfig {
        ConfigBuilder::new(name)
            .description("An advanced Aria TypeScript SDK project")
            .target("typescript")
            .output(&format!("dist/{}.aria", name))
            .source_dir("src/tools")
            .source_dir("src/agents")
            .source_dir("src/teams")
            .optimization(OptimizationLevel::Release)
            .bun_version("latest")
            .build()
    }
    
    /// Future: Aria DSL project template
    pub fn aria_dsl(name: &str) -> ProjectConfig {
        ConfigBuilder::new(name)
            .description("An Aria DSL stateful application")
            .target("aria-dsl")
            .output(&format!("dist/{}.aria", name))
            .optimization(OptimizationLevel::Size)
            .bun_version("latest")
            .env_var("ARIA_ENV", "production", false)
            .build()
    }
}

/// Utilities for working with configurations
pub struct ConfigUtils;

impl ConfigUtils {
    /// Merge two configurations (right takes precedence)
    pub fn merge(base: ProjectConfig, override_config: ProjectConfig) -> ProjectConfig {
        ProjectConfig {
            project: ProjectInfo {
                name: if override_config.project.name != "aria-project" {
                    override_config.project.name
                } else {
                    base.project.name
                },
                version: if override_config.project.version != "0.1.0" {
                    override_config.project.version
                } else {
                    base.project.version
                },
                description: if override_config.project.description != "An Aria agentic application" {
                    override_config.project.description
                } else {
                    base.project.description
                },
                authors: if !override_config.project.authors.is_empty() {
                    override_config.project.authors
                } else {
                    base.project.authors
                },
                license: override_config.project.license.or(base.project.license),
                repository: override_config.project.repository.or(base.project.repository),
            },
            build: BuildConfig {
                target: if override_config.build.target != "typescript" {
                    override_config.build.target
                } else {
                    base.build.target
                },
                output: override_config.build.output.or(base.build.output),
                source_dirs: if !override_config.build.source_dirs.is_empty() {
                    override_config.build.source_dirs
                } else {
                    base.build.source_dirs
                },
                exclude: if !override_config.build.exclude.is_empty() {
                    override_config.build.exclude
                } else {
                    base.build.exclude
                },
                watch: override_config.build.watch.or(base.build.watch),
                optimization: override_config.build.optimization.or(base.build.optimization),
            },
            runtime: RuntimeConfig {
                bun_version: if override_config.runtime.bun_version != "latest" {
                    override_config.runtime.bun_version
                } else {
                    base.runtime.bun_version
                },
                node_version: override_config.runtime.node_version.or(base.runtime.node_version),
                environment: if !override_config.runtime.environment.is_empty() {
                    override_config.runtime.environment
                } else {
                    base.runtime.environment
                },
            },
        }
    }
    
    /// Validate a configuration file exists and is readable
    pub async fn validate_config_file(path: &Path) -> Result<()> {
        if !path.exists() {
            return Err(anyhow!("Configuration file does not exist: {}", path.display()));
        }
        
        if !path.is_file() {
            return Err(anyhow!("Configuration path is not a file: {}", path.display()));
        }
        
        // Try to read and parse the file
        let content = fs::read_to_string(path).await?;
        toml::from_str::<ProjectConfig>(&content)
            .map_err(|e| anyhow!("Invalid configuration file: {}", e))?;
        
        Ok(())
    }
} 