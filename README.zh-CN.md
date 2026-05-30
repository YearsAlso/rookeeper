# Rookeeper

**面向工控单机场景的确定性、轻量级协调服务。**

Rookeeper 提供本地树形 KV 存储、Watch 通知、单机锁和服务注册发现能力 — 适用于 Linux 和 Windows 环境，部署于 PLC、边缘网关、机器人控制器和工业 PC。

---

## 产品定位

Rookeeper 是一个**本地状态机驱动的协调内核**，部署在单台工业设备上，为设备内多个控制程序、驱动模块、边缘代理和运维工具提供统一的协调平面。

### 核心能力

| 能力 | 说明 |
|---|---|
| **树形 KV** | 路径化节点、版本控制、ACL |
| **WAL + 快照** | 确定性重放，断电后 < 500ms 恢复到一致状态 |
| **单机锁** | 多进程 / 多线程互斥，租约可选 |
| **服务注册发现** | 同机实例注册、健康状态、属性查询 |
| **持久 Watcher** | 路径级、前缀级变更通知 |
| **ACL 与审计** | 访问控制和可回放的审计日志 |

### 典型场景

- 机器人控制程序读写工艺参数、发布自身状态
- 网关采集进程与协议转换进程互相发现
- 工控机重启后快速恢复到最后一致配置
- Linux / Windows 跨平台统一 SDK 接入

### V1.0 明确不做

- 多机部署、主从复制、选主、故障转移
- Raft、Zab 等共识协议
- 跨主机服务发现
- 强制传输加密与 mTLS 落地
- Web 控制台
- 复杂 SQL 查询、全文检索

---

## 项目阶段

| 阶段 | 状态 | 说明 |
|---|---|---|
| **Phase 0** | ✅ 已完成 | 项目工程基线、协议草案、平台抽象、CI 基线 |
| **Phase 1** | ✅ 已完成 | 树形 KV、WAL、快照、恢复、ACL 检查、基础 CLI |
| **Phase 2** | 🔜 规划中 | Watch、单机锁、服务注册发现 |
| **Phase 3** | 📋 未来 | 完整 IPC（UDS / Named Pipe）、结构化日志、指标暴露 |

---

## 项目结构

```
rookeeper/
├── crates/
│   ├── rookeeper-protocol/   # 共享协议、数据模型、ACL、错误码
│   ├── rookeeper-storage/    # 存储布局、WAL/快照路径与校验辅助
│   ├── rookeeper-platform/   # Linux / Windows 平台差异抽象
│   ├── rookeeper-client/     # 客户端 SDK 骨架
│   ├── rookeeper-server/     # 服务端入口、配置加载
│   └── rookeeper-cli/        # 本地运维 CLI
├── config/
│   └── rookeeper.default.toml
├── docs/
│   ├── architecture-baseline.md   # 模块职责与设计原则
│   ├── protocol-baseline.md      # 二进制协议头与请求类型
│   ├── storage-layout.md         # WAL 与快照命名规范
│   └── industrial-single-node-coordination-service-v1.0-prd.md  # 完整 PRD
├── README.md                # 英文版（当前页）
└── README.zh-CN.md          # 本文件
```

---

## 快速开始

```bash
# 格式化、构建、测试
cargo fmt --all
cargo build --workspace
cargo test --workspace

# 打印默认存储布局
cargo run -p rookeeper-server -- --print-layout

# 运行 CLI
cargo run -p rookeeper-cli -- status
```

---

## 多语言支持

本文档提供多语言版本：

- [English](README.md)
- [简体中文](README.zh-CN.md) — 当前页面

---

## 设计原则

1. **状态机纯净** — 业务结果只由命令输入决定，关键路径不含随机数或墙上时钟。
2. **平台差异下沉** — 平台相关逻辑只能出现在 `rookeeper-platform` 及部署资产中。
3. **协议先行** — 请求类型、错误码、配置字段在 Phase 0 就应稳定下来。
4. **轻量优先** — 基线只引入首版真正需要的依赖。

---

## 协议与架构

详见 [docs/](docs/) 目录：

- [架构基线](docs/architecture-baseline.md) — 模块职责与 5 层逻辑结构
- [协议基线](docs/protocol-baseline.md) — 24 字节二进制头与请求类型
- [存储布局](docs/storage-layout.md) — WAL 与快照文件命名规范
- [PRD](docs/industrial-single-node-coordination-service-v1.0-prd.md) — 完整产品需求文档