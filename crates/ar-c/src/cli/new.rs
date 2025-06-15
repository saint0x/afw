use clap::ArgMatches;
use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

use crate::cli::{print_status, print_info};

/// Handle the 'arc new' command
pub async fn handle_new_command(matches: &ArgMatches) -> Result<()> {
    let project_name = matches.get_one::<String>("name").unwrap();
    let template = matches.get_one::<String>("template").unwrap();
    
    print_info(&format!("Creating new Aria project: {}", project_name));
    print_info(&format!("Using template: {}", template));
    
    // Check if directory already exists
    if Path::new(project_name).exists() {
        return Err(anyhow!("Directory '{}' already exists", project_name));
    }
    
    // Create project directory
    fs::create_dir_all(project_name)?;
    
    match template.as_str() {
        "basic" => create_basic_template(project_name).await?,
        "sdk" => create_sdk_template(project_name).await?,
        "advanced" => create_advanced_template(project_name).await?,
        _ => return Err(anyhow!("Unknown template: {}", template)),
    }
    
    print_status("Created", &format!("Aria project '{}'", project_name));
    print_info("Next steps:");
    println!("    cd {}", project_name);
    println!("    arc build ./src");
    
    Ok(())
}

/// Create basic project template
async fn create_basic_template(project_name: &str) -> Result<()> {
    let base_path = Path::new(project_name);
    
    // Create directory structure
    fs::create_dir_all(base_path.join("src"))?;
    fs::create_dir_all(base_path.join("dist"))?;
    
    // Create aria.toml
    let aria_toml = format!(r#"[project]
name = "{}"
version = "0.1.0"
description = "An Aria agentic application"

[build]
output = "dist/{}.aria"
target = "typescript"

[runtime]
bun_version = "latest"
"#, project_name, project_name);
    
    fs::write(base_path.join("aria.toml"), aria_toml)?;
    
    // Create package.json
    let package_json = format!(r#"{{
  "name": "{}",
  "version": "0.1.0",
  "description": "An Aria agentic application",
  "main": "src/index.ts",
  "scripts": {{
    "build": "arc build ./src",
    "dev": "arc build ./src --watch"
  }},
  "dependencies": {{
    "@aria/sdk": "^0.1.0"
  }},
  "devDependencies": {{
    "typescript": "^5.0.0",
    "@types/node": "^20.0.0"
  }}
}}
"#, project_name);
    
    fs::write(base_path.join("package.json"), package_json)?;
    
    // Create basic TypeScript file
    let index_ts = r#"import { tool } from '@aria/sdk';

@tool({
  name: "greet",
  description: "Simple greeting tool",
  inputs: { name: "string" },
  outputs: { message: "string" }
})
export async function greet(params: { name: string }) {
  return {
    message: `Hello, ${params.name}! Welcome to Aria.`
  };
}
"#;
    
    fs::write(base_path.join("src/index.ts"), index_ts)?;
    
    // Create tsconfig.json
    let tsconfig = r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "node",
    "strict": true,
    "experimentalDecorators": true,
    "emitDecoratorMetadata": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "outDir": "./dist",
    "rootDir": "./src"
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}
"#;
    
    fs::write(base_path.join("tsconfig.json"), tsconfig)?;
    
    // Create README.md
    let readme = format!(r#"# {}

An Aria agentic application built with TypeScript SDK.

## Getting Started

1. Install dependencies:
   ```bash
   npm install
   ```

2. Build the project:
   ```bash
   arc build ./src
   ```

3. The compiled `.aria` bundle will be in the `dist/` directory.

## Project Structure

- `src/` - TypeScript source files with Aria decorators
- `dist/` - Compiled `.aria` bundles
- `aria.toml` - Project configuration
- `tsconfig.json` - TypeScript configuration

## Available Tools

- `greet` - Simple greeting tool

## Learn More

- [Aria Documentation](https://aria.dev/docs)
- [TypeScript SDK Guide](https://aria.dev/sdk)
"#, project_name);
    
    fs::write(base_path.join("README.md"), readme)?;
    
    Ok(())
}

/// Create SDK template (more advanced with multiple tools/agents)
async fn create_sdk_template(project_name: &str) -> Result<()> {
    // Create basic template first
    create_basic_template(project_name).await?;
    
    let base_path = Path::new(project_name);
    
    // Create additional directories
    fs::create_dir_all(base_path.join("src/tools"))?;
    fs::create_dir_all(base_path.join("src/agents"))?;
    
    // Create a sample tool
    let sample_tool = r#"import { tool } from '@aria/sdk';

@tool({
  name: "calculateSum",
  description: "Calculate the sum of two numbers",
  inputs: { 
    a: "number", 
    b: "number" 
  },
  outputs: { 
    sum: "number" 
  }
})
export async function calculateSum(params: { a: number; b: number }) {
  return {
    sum: params.a + params.b
  };
}

@tool({
  name: "validateEmail",
  description: "Validate email address format",
  inputs: { email: "string" },
  outputs: { valid: "boolean", error?: "string" }
})
export async function validateEmail(params: { email: string }) {
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  const valid = emailRegex.test(params.email);
  
  return {
    valid,
    error: valid ? undefined : "Invalid email format"
  };
}
"#;
    
    fs::write(base_path.join("src/tools/utilities.ts"), sample_tool)?;
    
    // Create a sample agent
    let sample_agent = r#"import { agent, tool } from '@aria/sdk';

@agent({
  name: "MathAssistant",
  description: "Helpful assistant for mathematical operations",
  capabilities: ["calculateSum", "validateEmail"]
})
export class MathAssistant {
  @tool({
    name: "solveEquation",
    description: "Solve simple linear equations",
    inputs: { equation: "string" },
    outputs: { result: "number", steps: "string[]" }
  })
  async solveEquation(params: { equation: string }) {
    // Simple implementation for demonstration
    // In practice, this would be more sophisticated
    const steps = [`Parsing equation: ${params.equation}`];
    
    // Very basic parsing for x + n = m format
    const match = params.equation.match(/x\s*\+\s*(\d+)\s*=\s*(\d+)/);
    if (match) {
      const n = parseInt(match[1]);
      const m = parseInt(match[2]);
      const result = m - n;
      steps.push(`Subtract ${n} from both sides`);
      steps.push(`x = ${m} - ${n}`);
      steps.push(`x = ${result}`);
      
      return { result, steps };
    }
    
    throw new Error("Unsupported equation format");
  }
  
  private formatSteps(steps: string[]): string[] {
    return steps.map((step, i) => `${i + 1}. ${step}`);
  }
}
"#;
    
    fs::write(base_path.join("src/agents/MathAssistant.ts"), sample_agent)?;
    
    // Update index.ts to export everything
    let index_ts = r#"// Export all tools
export * from './tools/utilities';

// Export all agents  
export * from './agents/MathAssistant';

// Main greeting tool
import { tool } from '@aria/sdk';

@tool({
  name: "greet",
  description: "Simple greeting tool",
  inputs: { name: "string" },
  outputs: { message: "string" }
})
export async function greet(params: { name: string }) {
  return {
    message: `Hello, ${params.name}! Welcome to Aria SDK.`
  };
}
"#;
    
    fs::write(base_path.join("src/index.ts"), index_ts)?;
    
    Ok(())
}

/// Create advanced template (with teams, complex workflows)
async fn create_advanced_template(project_name: &str) -> Result<()> {
    // Create SDK template first
    create_sdk_template(project_name).await?;
    
    let base_path = Path::new(project_name);
    
    // Create additional directories
    fs::create_dir_all(base_path.join("src/teams"))?;
    fs::create_dir_all(base_path.join("src/workflows"))?;
    
    // Create a sample team
    let sample_team = r#"import { team } from '@aria/sdk';
import { MathAssistant } from '../agents/MathAssistant';

@team({
  name: "AnalyticsTeam",
  description: "Team of agents working together on data analysis",
  agents: ["MathAssistant", "DataProcessor"],
  workflow: "collaborative"
})
export class AnalyticsTeam {
  private mathAssistant: MathAssistant;
  
  constructor() {
    this.mathAssistant = new MathAssistant();
  }
  
  async processDataSet(data: number[]): Promise<{
    sum: number;
    average: number;
    analysis: string;
  }> {
    // Team coordination logic would go here
    const sum = data.reduce((a, b) => a + b, 0);
    const average = sum / data.length;
    
    return {
      sum,
      average,
      analysis: `Processed ${data.length} data points. Sum: ${sum}, Average: ${average.toFixed(2)}`
    };
  }
}
"#;
    
    fs::write(base_path.join("src/teams/AnalyticsTeam.ts"), sample_team)?;
    
    print_info("Advanced template includes teams and workflows");
    
    Ok(())
} 