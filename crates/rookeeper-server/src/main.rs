//! rookeeper-server 主程序入口
//! 
//! Phase 0 引导程序：初始化追踪、解析参数、输出服务器摘要。
//! 
//! 实际服务运行逻辑将在后续阶段实现，当前仅提供基础设施验证。

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use rookeeper_server::{load_config, ServerBootstrap};
use tracing::info;
use tracing_subscriber::EnvFilter;

/// 命令行参数解析
/// 
/// 设计考量：
/// - `--config` 保留但当前不生效，明确告知用户配置加载尚未实现
/// - `--print-layout` 用于验证存储布局是否符合预期
#[derive(Debug, Parser)]
#[command(name = "rookeeper-server")]
#[command(about = "Phase 0 bootstrap for the rookeeper single-node coordinator")]
struct Args {
    /// 配置文件路径（当前未实现，保留用于 Phase 1）
    #[arg(long)]
    config: Option<PathBuf>,

    /// 打印存储布局详情而非运行时摘要
    #[arg(long)]
    print_layout: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化追踪日志系统，使用环境变量配置级别
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    
    // 加载配置（当前仅返回默认配置）
    let config = load_config(args.config.as_deref())?;
    let bootstrap = ServerBootstrap::from_config(config);

    info!(summary = %bootstrap.summary(), "rookeeper server bootstrap ready");

    // 根据参数决定输出格式
    if args.print_layout {
        // 打印完整的存储布局树，用于验证目录结构
        println!("{:#?}", bootstrap.storage_layout);
    } else {
        // 打印单行摘要，便于监控和脚本解析
        println!("{}", bootstrap.summary());
    }

    Ok(())
}