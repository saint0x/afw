use clap::ArgMatches;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::cli::{print_status, print_error, print_info, print_warning};
use crate::compiler::AriaCompiler;
use crate::config::ProjectConfig;

/// Handle the 'arc build' command
pub async fn handle_build_command(matches: &ArgMatches) -> Result<()> {
    let input_path = matches.get_one::<String>("input").unwrap();
    let output_path = matches.get_one::<String>("output");
    let watch_mode = matches.get_flag("watch");
    let verbose = matches.get_flag("verbose");
    
    print_info(&format!("Building Aria project from: {}", input_path));
    
    // Load project configuration
    let config = load_project_config(input_path).await?;
    
    // Determine output path
    let output = determine_output_path(output_path, &config, input_path)?;
    
    if watch_mode {
        print_info("Starting watch mode...");
        start_watch_mode(input_path, &output, verbose).await?;
    } else {
        build_project(input_path, &output, verbose).await?;
    }
    
    Ok(())
}

/// Load project configuration from aria.toml
async fn load_project_config(input_path: &str) -> Result<ProjectConfig> {
    let config_path = find_config_file(input_path)?;
    
    match config_path {
        Some(path) => {
            print_info(&format!("Found configuration: {}", path.display()));
            ProjectConfig::load_from_file(&path).await
        }
        None => {
            print_warning("No aria.toml found, using default configuration");
            Ok(ProjectConfig::default())
        }
    }
}

/// Find aria.toml configuration file
fn find_config_file(start_path: &str) -> Result<Option<PathBuf>> {
    let mut current = Path::new(start_path).canonicalize()?;
    
    loop {
        let config_path = current.join("aria.toml");
        if config_path.exists() {
            return Ok(Some(config_path));
        }
        
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => return Ok(None),
        }
    }
}

/// Determine the output path for the .aria bundle
fn determine_output_path(
    output_arg: Option<&String>, 
    config: &ProjectConfig, 
    input_path: &str
) -> Result<PathBuf> {
    if let Some(output) = output_arg {
        return Ok(PathBuf::from(output));
    }
    
    if let Some(output) = &config.build.output {
        return Ok(PathBuf::from(output));
    }
    
    // Default: create output based on input directory name
    let input_dir = Path::new(input_path);
    let dir_name = input_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("bundle");
    
    Ok(PathBuf::from(format!("dist/{}.aria", dir_name)))
}

/// Build the project once
async fn build_project(input_path: &str, output_path: &PathBuf, verbose: bool) -> Result<()> {
    let start_time = Instant::now();
    
    print_status("Compiling", "TypeScript sources...");
    
    // Initialize compiler
    let compiler = AriaCompiler::new();
    
    // Compile the project
    match compiler.compile_project(input_path, output_path, verbose).await {
        Ok(result) => {
            let duration = start_time.elapsed();
            
            print_status("Finished", &format!(
                "Build completed in {:.2}s", 
                duration.as_secs_f64()
            ));
            
            print_info(&format!("Bundle created: {}", output_path.display()));
            print_info(&format!("  - Tools: {}", result.tools_count));
            print_info(&format!("  - Agents: {}", result.agents_count));
            print_info(&format!("  - Teams: {}", result.teams_count));
            print_info(&format!("  - Pipelines: {}", result.pipelines_count));
            print_info(&format!("Bundle size: {:.2} KB", result.bundle_size_kb));
            
            if verbose {
                print_diagnostics(&result);
            }
        }
        Err(e) => {
            print_error(&format!("Build failed: {}", e));
            return Err(e);
        }
    }
    
    Ok(())
}

/// Start watch mode for continuous building
async fn start_watch_mode(_input_path: &str, _output_path: &PathBuf, _verbose: bool) -> Result<()> {
    print_info("Watch mode not yet implemented");
    print_info("For now, use: arc build ./src");
    
    // TODO: Implement file watching with notify crate
    // This would:
    // 1. Watch for changes in input_path
    // 2. Debounce rapid changes
    // 3. Rebuild on changes
    // 4. Show incremental build times
    
    Ok(())
}

/// Print detailed build diagnostics
fn print_diagnostics(result: &crate::compiler::CompilationResult) {
    if !result.warnings.is_empty() {
        print_warning(&format!("Build completed with {} warnings:", result.warnings.len()));
        for warning in &result.warnings {
            println!("    - {}", warning);
        }
    }
    
    if verbose_enabled() {
        print_info("Detailed build information:");
        println!("    - Source files processed: {}", result.source_files_count);
        println!("    - Dependencies resolved: {}", result.dependencies_count);
        println!("    - Compilation time: {:.2}s", result.compilation_time_secs);
        println!("    - Bundle compression: {:.1}%", result.compression_ratio * 100.0);
    }
}

/// Check if verbose mode is enabled (stub for now)
fn verbose_enabled() -> bool {
    // TODO: Pass verbose flag through properly
    false
} 