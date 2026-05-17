# rookeeper

`rookeeper` 是一个面向工控单机场景的协调服务，目标是在 Linux 和 Windows 上以轻量、确定性、可恢复的方式提供本地树形 KV、Watcher、锁和服务注册发现能力。

当前仓库已完成 **Phase 0：项目基线搭建**，提供：

1. Cargo workspace 与模块边界划分。
1. 跨平台基础类型、协议草案、错误码与配置模型。
1. 数据目录布局说明与默认配置模板。
1. Linux / Windows 的 CI 编译与测试基线。

## Workspace 结构

```text
crates/
  rookeeper-protocol/  # 共享协议、数据模型、ACL、错误码、配置模型
  rookeeper-storage/   # 存储布局、WAL/快照路径与校验辅助
  rookeeper-platform/  # 平台差异抽象与默认 IPC 选择
  rookeeper-client/    # 客户端接入骨架
  rookeeper-server/    # 服务端启动骨架
  rookeeper-cli/       # 基础 CLI 骨架
config/
  rookeeper.default.toml
docs/
  architecture-baseline.md
  protocol-baseline.md
  storage-layout.md
```

## 快速开始

```powershell
cargo fmt --all
cargo test --workspace
cargo run -p rookeeper-server -- --print-layout
cargo run -p rookeeper-cli -- status
```

## 当前阶段边界

Phase 0 只完成工程基线与接口固化，不包含完整的 KV、Watcher、锁或持久化行为实现；这些内容将在后续 Phase 1 / Phase 2 中逐步落地。
