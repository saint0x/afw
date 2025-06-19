use clap::ArgMatches;
use anyhow::{Result, anyhow, Context};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::cli::{print_status, print_info, print_error};

/// Handle the 'arc new' command according to NEWSDK.md specification
pub async fn handle_new_command(matches: &ArgMatches) -> Result<()> {
    let project_name = matches.get_one::<String>("name").unwrap();
    let template = matches.get_one::<String>("template").map(|s| s.as_str()).unwrap_or("basic");
    
    print_info(&format!("Creating new Aria project: {}", project_name));
    
    // Validate project name
    if !is_valid_project_name(project_name) {
        return Err(anyhow!("Invalid project name: '{}'. Project names must be valid directory names", project_name));
    }
    
    // Check if directory already exists
    if Path::new(project_name).exists() {
        return Err(anyhow!("Directory '{}' already exists", project_name));
    }
    
    // Create project according to NEWSDK.md standard structure
    create_project_structure(project_name, template).await?;
    
    print_status("Created", &format!("Aria project '{}'", project_name));
    print_info("Next steps:");
    println!("    cd {}", project_name);
    println!("    bun install");
    println!("    arc check");
    println!("    arc build");
    println!("    arc upload dist/{}.aria", project_name);
    
    Ok(())
}

/// Create the standard project structure as defined in NEWSDK.md
async fn create_project_structure(project_name: &str, _template: &str) -> Result<()> {
    let base_path = Path::new(project_name);
    
    // TODO: Use template parameter for different project variations in the future
    // For now, we only support the standard NEWSDK.md structure
    
    // Create standard directory structure from NEWSDK.md:
    // my-aria-project/
    // ├── src/
    // │   └── main.ts
    // ├── config/
    // │   └── package.json
    // ├── aria.toml
    // └── llm.xml
    
    fs::create_dir_all(base_path.join("src"))
        .context("Failed to create src/ directory")?;
    
    fs::create_dir_all(base_path.join("config"))
        .context("Failed to create config/ directory")?;
    
    // Generate project class name (PascalCase from kebab-case)
    let project_class_name = to_pascal_case(project_name);
    
    // Create src/main.ts from template
    create_file_from_template(
        &base_path.join("src/main.ts"),
        include_str!("../templates/main.ts.template"),
        project_name,
        &project_class_name
    )?;
    
    // Create config/package.json from template
    create_file_from_template(
        &base_path.join("config/package.json"),
        include_str!("../templates/package.json.template"),
        project_name,
        &project_class_name
    )?;
    
    // Create aria.toml from template
    create_file_from_template(
        &base_path.join("aria.toml"),
        include_str!("../templates/aria.toml.template"),
        project_name,
        &project_class_name
    )?;
    
    // Create llm.xml from template (optional but included by default)
    create_file_from_template(
        &base_path.join("llm.xml"),
        include_str!("../templates/llm.xml.template"),
        project_name,
        &project_class_name
    )?;
    
    // Initialize git repository as specified in NEWSDK.md
    init_git_repository(base_path)?;
    
    Ok(())
}

/// Create a file from template, replacing placeholders
fn create_file_from_template(
    file_path: &Path,
    template_content: &str,
    project_name: &str,
    project_class_name: &str
) -> Result<()> {
    let content = template_content
        .replace("{{PROJECT_NAME}}", project_name)
        .replace("{{PROJECT_CLASS_NAME}}", project_class_name);
    
    fs::write(file_path, content)
        .with_context(|| format!("Failed to create file: {}", file_path.display()))?;
    
    Ok(())
}

/// Initialize git repository as specified in NEWSDK.md
fn init_git_repository(project_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .arg("init")
        .current_dir(project_path)
        .output();
    
    match output {
        Ok(output) if output.status.success() => {
            print_info("Initialized git repository");
        },
        Ok(output) => {
            print_error(&format!(
                "Failed to initialize git repository: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        },
        Err(_) => {
            print_error("Git not found - skipping repository initialization");
        }
    }
    
    Ok(())
}

/// Validate project name
fn is_valid_project_name(name: &str) -> bool {
    !name.is_empty() 
        && !name.starts_with('.') 
        && !name.contains('/') 
        && !name.contains('\\')
        && name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Convert kebab-case to PascalCase
fn to_pascal_case(s: &str) -> String {
    s.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let mut result = first.to_uppercase().collect::<String>();
                    result.push_str(&chars.as_str().to_lowercase());
                    result
                }
            }
        })
        .collect()
} 