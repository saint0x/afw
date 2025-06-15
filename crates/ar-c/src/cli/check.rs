use clap::ArgMatches;
use anyhow::Result;
use std::time::Instant;

use crate::cli::{print_status, print_error, print_info};
use crate::compiler::AriaCompiler;

/// Handle the 'arc check' command
pub async fn handle_check_command(matches: &ArgMatches) -> Result<()> {
    let input_path = matches.get_one::<String>("input").unwrap();
    let verbose = matches.get_flag("verbose");

    let start_time = Instant::now();
    
    print_info(&format!("Checking Aria project in: {}", input_path));
    
    let compiler = AriaCompiler::new();
    
    match compiler.check_project(input_path, verbose).await {
        Ok(result) => {
            let duration = start_time.elapsed();
            
            print_status("Finished", &format!(
                "Check completed in {:.2}s", 
                duration.as_secs_f64()
            ));
            
            print_info("Project analysis:");
            print_info(&format!("  - Tools: {}", result.tools_count));
            print_info(&format!("  - Agents: {}", result.agents_count));
            print_info(&format!("  - Teams: {}", result.teams_count));
            print_info(&format!("  - Pipelines: {}", result.pipelines_count));
        }
        Err(e) => {
            print_error(&format!("Check failed: {}", e));
            return Err(e);
        }
    }
    
    Ok(())
} 