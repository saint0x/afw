use anyhow::Result;
use clap::{Arg, ArgAction, Command};

pub mod bundle;
pub mod cli;
pub mod compiler;
pub mod config;
pub mod grpc;

use crate::cli::{handle_build_command, handle_check_command, handle_new_command, handle_upload_command};

fn cli() -> Command {
    Command::new("arc")
        .about("Aria Compiler")
        .version("0.1.0")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("new")
                .about("Create a new Aria project")
                .arg(Arg::new("name").required(true).help("Project name"))
                .arg(Arg::new("template").short('t').long("template").default_value("basic").help("Project template (basic, advanced)"))
        )
        .subcommand(
            Command::new("build")
                .about("Build an Aria project into a .aria bundle")
                .arg(Arg::new("input").default_value(".").help("Input directory or file"))
                .arg(Arg::new("output").short('o').long("output").help("Output file path"))
                .arg(Arg::new("watch").short('w').long("watch").action(ArgAction::SetTrue).help("Watch for file changes"))
                .arg(Arg::new("verbose").short('v').long("verbose").action(ArgAction::SetTrue).help("Enable verbose output"))
        )
        .subcommand(
            Command::new("check")
                .about("Check an Aria project for errors")
                .arg(Arg::new("input").default_value(".").help("Input directory or file"))
                .arg(Arg::new("verbose").short('v').long("verbose").action(ArgAction::SetTrue).help("Enable verbose output"))
        )
        .subcommand(
            Command::new("upload")
                .about("Upload an Aria bundle to Quilt daemon via gRPC")
                .arg(Arg::new("bundle").required(true).help("Path to .aria bundle file"))
                .arg(Arg::new("socket").short('s').long("socket").help("Unix socket path to Quilt daemon (default: /run/quilt/api.sock)"))
        )
}

#[tokio::main]
async fn main() -> Result<()> {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("new", sub_matches)) => handle_new_command(sub_matches).await?,
        Some(("build", sub_matches)) => handle_build_command(sub_matches).await?,
        Some(("check", sub_matches)) => handle_check_command(sub_matches).await?,
        Some(("upload", sub_matches)) => handle_upload_command(sub_matches).await?,
        _ => unreachable!(),
    }

    Ok(())
}
