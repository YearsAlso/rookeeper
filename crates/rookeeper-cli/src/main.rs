use anyhow::Result;
use clap::{Parser, Subcommand};
use rookeeper_platform::{current_os, default_endpoint, default_ipc_transport};
use rookeeper_protocol::model::NodePath;
use rookeeper_protocol::wire::PROTOCOL_VERSION_V1;

#[derive(Debug, Parser)]
#[command(name = "rookeeper-cli")]
#[command(about = "Phase 0 maintenance CLI for rookeeper")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Status,
    NormalizePath { path: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Status => {
            println!("protocol_version={PROTOCOL_VERSION_V1}");
            println!("os={:?}", current_os());
            println!("default_transport={:?}", default_ipc_transport());
            println!("default_endpoint={}", default_endpoint("rookeeper"));
        }
        Command::NormalizePath { path } => {
            let normalized = NodePath::parse(&path).map_err(anyhow::Error::msg)?;
            println!("{}", normalized.as_str());
        }
    }

    Ok(())
}
