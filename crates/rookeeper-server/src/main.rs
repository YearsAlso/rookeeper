use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use rookeeper_server::{load_config, ServerBootstrap};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "rookeeper-server")]
#[command(about = "Phase 0 bootstrap for the rookeeper single-node coordinator")]
struct Args {
    #[arg(long)]
    config: Option<PathBuf>,

    #[arg(long)]
    print_layout: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let config = load_config(args.config.as_deref())?;
    let bootstrap = ServerBootstrap::from_config(config);

    info!(summary = %bootstrap.summary(), "rookeeper server bootstrap ready");

    if args.print_layout {
        println!("{:#?}", bootstrap.storage_layout);
    } else {
        println!("{}", bootstrap.summary());
    }

    Ok(())
}
