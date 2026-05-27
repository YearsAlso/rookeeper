//! rookeeper-cli 主程序入口
//! 
//! Phase 0 维护 CLI：提供状态查询和路径规范化等基础维护功能。
//! 
//! 设计原则：
//! - CLI 复用共享的 platform 和 protocol 类型，确保与服务端行为一致
//! - 命令输出面向机器解析（JSON/纯文本），便于脚本集成

use anyhow::Result;
use clap::{Parser, Subcommand};
use rookeeper_platform::{current_os, default_endpoint, default_ipc_transport};
use rookeeper_protocol::model::NodePath;
use rookeeper_protocol::wire::PROTOCOL_VERSION_V1;

/// CLI 入口结构
/// 
/// 使用 clap derive 模式自动生成帮助和命令补全
#[derive(Debug, Parser)]
#[command(name = "rookeeper-cli")]
#[command(about = "Phase 0 maintenance CLI for rookeeper")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// 可用命令列表
/// 
/// Phase 0 仅实现最基础的维护操作
/// 后续阶段将添加节点管理、ACL 修改等命令
#[derive(Debug, Subcommand)]
enum Command {
    /// 查询服务状态和协议信息
    Status,
    /// 规范化路径字符串（用于验证路径安全性）
    NormalizePath { path: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // Status 命令：输出协议版本、操作系统、传输方式、默认端点
        Command::Status => {
            println!("protocol_version={PROTOCOL_VERSION_V1}");
            println!("os={:?}", current_os());
            println!("default_transport={:?}", default_ipc_transport());
            println!("default_endpoint={}", default_endpoint("rookeeper"));
        }
        // NormalizePath 命令：验证并输出规范化后的路径
        // 失败时返回错误（路径包含 .. 等不安全内容）
        Command::NormalizePath { path } => {
            let normalized = NodePath::parse(&path).map_err(anyhow::Error::msg)?;
            println!("{}", normalized.as_str());
        }
    }

    Ok(())
}