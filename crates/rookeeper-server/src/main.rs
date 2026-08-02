//! rookeeper-server 主程序入口
//!
//! Phase 1: 完整的 TCP Binary Server 实现
//!
//! 功能：
//! - TCP 监听（自定义二进制协议，bincode 序列化）
//! - 分层命名空间 CRUD
//! - Watch 持久订阅（TCP 推送）
//! - TTL 支持（后台清理）
//! - 运行时模式切换（memory/persistent）

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use rookeeper_server::{load_config, RookeeperServer};
use tracing::info;
use tracing_subscriber::EnvFilter;

/// 命令行参数解析
///
/// 设计考量：
/// - `--config` 指定配置文件路径
/// - `--print-layout` 用于验证存储布局
#[derive(Debug, Parser)]
#[command(name = "rookeeper-server")]
#[command(about = "Rookeeper single-node coordinator - Phase 1 MVP")]
struct Args {
    /// 配置文件路径
    #[arg(long)]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化追踪日志系统
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    // 加载配置
    let config = load_config(args.config.as_deref())?;

    info!(
        "Starting Rookeeper server (version: {})",
        env!("CARGO_PKG_VERSION")
    );
    info!("Runtime mode: {:?}", config.runtime_mode());
    info!("TCP bind: {}", config.tcp_bind);

    // 创建并启动 TCP 服务器
    let server = RookeeperServer::new(config).await?;
    server.run().await?;

    Ok(())
}
