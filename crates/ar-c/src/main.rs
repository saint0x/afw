use anyhow::Result;
use clap::{Arg, ArgAction, Command};

pub mod bundle;
pub mod cli;
pub mod compiler;
pub mod config;

use crate::cli::{handle_build_command, handle_check_command};

fn cli() -> Command {
    Command::new("arc")
        .about("Aria Compiler")
        .version("0.1.0")
        .subcommand_required(true)
        .arg_required_else_help(true)
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
}

#[tokio::main]
async fn main() -> Result<()> {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("build", sub_matches)) => handle_build_command(sub_matches).await?,
        Some(("check", sub_matches)) => handle_check_command(sub_matches).await?,
        _ => unreachable!(),
    }

    Ok(())
}
