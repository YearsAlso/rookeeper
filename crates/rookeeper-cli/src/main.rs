//! rookeeper-cli 主程序入口
//!
//! Phase 0/2 维护 CLI：提供状态查询、节点管理、锁、会话和服务注册等维护功能。
//!
//! 设计原则：
//! - CLI 复用共享的 platform 和 protocol 类型，确保与服务端行为一致
//! - 命令输出面向机器解析（JSON/纯文本），便于脚本集成

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use rookeeper_platform::{current_os, default_endpoint, default_ipc_transport};
use rookeeper_protocol::model::NodePath;
use rookeeper_protocol::wire::PROTOCOL_VERSION_V1;

/// CLI 入口结构
///
/// 使用 clap derive 模式自动生成帮助和命令补全
#[derive(Debug, Parser)]
#[command(name = "rookeeper-cli")]
#[command(about = "Rookeeper maintenance CLI — nodes, sessions, locks, services")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// 可用命令列表
#[derive(Debug, Subcommand)]
enum Command {
    /// 查询服务状态和协议信息
    Status,
    /// 规范化路径字符串（用于验证路径安全性）
    NormalizePath { path: String },

    // ── Session 管理 ───────────────────────────────────────────────
    /// 创建新会话（打印 session_id）
    SessionCreate {
        /// 会话超时时间（秒），默认 30
        #[arg(short, long, default_value = "30")]
        timeout_secs: u64,
    },
    /// 发送心跳续期会话
    SessionHeartbeat {
        /// 会话 ID
        #[arg(value_name = "SESSION_ID")]
        session_id: u64,
    },
    /// 关闭会话
    SessionClose {
        /// 会话 ID
        #[arg(value_name = "SESSION_ID")]
        session_id: u64,
    },

    // ── 锁管理 ────────────────────────────────────────────────────
    /// 获取锁（非阻塞）
    LockAcquire {
        /// 锁名称
        #[arg(value_name = "LOCK_NAME")]
        lock_name: String,
        /// 会话 ID
        #[arg(short, long)]
        session: u64,
        /// 模式：nonblocking（默认）/ blocking / timeout:<秒>
        #[arg(short, long, default_value = "nonblocking")]
        mode: String,
    },
    /// 释放锁
    LockRelease {
        /// 锁名称
        #[arg(value_name = "LOCK_NAME")]
        lock_name: String,
        /// 会话 ID
        #[arg(short, long)]
        session: u64,
    },
    /// 查询锁状态
    LockStat {
        /// 锁名称
        #[arg(value_name = "LOCK_NAME")]
        lock_name: String,
    },

    // ── 服务注册 ──────────────────────────────────────────────────
    /// 注册服务实例
    ServiceRegister {
        /// 服务名称
        #[arg(value_name = "SERVICE_NAME")]
        service_name: String,
        /// 实例 ID（唯一）
        #[arg(value_name = "INSTANCE_ID")]
        instance_id: String,
        /// 主机地址
        #[arg(value_name = "HOST")]
        host: String,
        /// 端口
        #[arg(value_name = "PORT")]
        port: u16,
        /// 元数据（JSON，选项）
        #[arg(short, long)]
        metadata: Option<String>,
    },
    /// 列举服务实例
    ServiceList {
        /// 服务名称
        #[arg(value_name = "SERVICE_NAME")]
        service_name: String,
    },
    /// 下线服务实例
    ServiceDeregister {
        /// 服务名称
        #[arg(value_name = "SERVICE_NAME")]
        service_name: String,
        /// 实例 ID
        #[arg(value_name = "INSTANCE_ID")]
        instance_id: String,
    },

    // ── Watch ─────────────────────────────────────────────────────
    /// 订阅路径变更
    Watch {
        /// 路径
        #[arg(value_name = "PATH")]
        path: String,
        /// 递归监听子路径
        #[arg(short, long)]
        recursive: bool,
    },
    /// 取消订阅
    WatchCancel {
        /// 订阅 ID
        #[arg(value_name = "SUBSCRIPTION_ID")]
        subscription_id: usize,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // ── 状态 ─────────────────────────────────────────────────
        Command::Status => {
            println!("protocol_version={PROTOCOL_VERSION_V1}");
            println!("os={:?}", current_os());
            println!("default_transport={:?}", default_ipc_transport());
            println!("default_endpoint={}", default_endpoint("rookeeper"));
        }
        Command::NormalizePath { path } => {
            let normalized = NodePath::parse(&path).map_err(|e| anyhow!("{e}"))?;
            println!("{}", normalized.as_str());
        }

        // ── Session ───────────────────────────────────────────────
        Command::SessionCreate { timeout_secs } => {
            println!("session_timeout_secs={timeout_secs}");
            // Phase 2 服务端实现后，这里调用 SDK
            println!("NOTE: session creation requires running server + SDK");
        }
        Command::SessionHeartbeat { session_id } => {
            println!("session_id={session_id}");
            println!("NOTE: heartbeat requires running server + SDK");
        }
        Command::SessionClose { session_id } => {
            println!("closed session_id={session_id}");
            println!("NOTE: close requires running server + SDK");
        }

        // ── 锁 ───────────────────────────────────────────────────
        Command::LockAcquire { lock_name, session, mode } => {
            println!("lock_name={lock_name}");
            println!("session={session}");
            println!("mode={mode}");
            println!("NOTE: lock acquire requires running server + SDK");
        }
        Command::LockRelease { lock_name, session } => {
            println!("released lock={lock_name} session={session}");
            println!("NOTE: lock release requires running server + SDK");
        }
        Command::LockStat { lock_name } => {
            println!("lock_name={lock_name}");
            println!("NOTE: lock stat requires running server + SDK");
        }

        // ── 服务注册 ──────────────────────────────────────────────
        Command::ServiceRegister { service_name, instance_id, host, port, metadata } => {
            println!("service_name={service_name}");
            println!("instance_id={instance_id}");
            println!("host={host}");
            println!("port={port}");
            if let Some(ref m) = metadata {
                println!("metadata={m}");
            }
            println!("NOTE: service register requires running server + SDK");
        }
        Command::ServiceList { service_name } => {
            println!("service_name={service_name}");
            println!("NOTE: service list requires running server + SDK");
        }
        Command::ServiceDeregister { service_name, instance_id } => {
            println!("deregistered {service_name}/{instance_id}");
            println!("NOTE: service deregister requires running server + SDK");
        }

        // ── Watch ─────────────────────────────────────────────────
        Command::Watch { path, recursive } => {
            println!("path={path}");
            println!("recursive={recursive}");
            println!("NOTE: watch requires running server + SDK");
        }
        Command::WatchCancel { subscription_id } => {
            println!("cancelled subscription_id={subscription_id}");
            println!("NOTE: watch cancel requires running server + SDK");
        }
    }

    Ok(())
}
