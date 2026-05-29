# Rookeeper 架构审核报告

**审核日期**: 2026-05-27  
**审核版本**: V1.0 PRD vs Phase 0 实现  
**项目路径**: ~/Project/rookeeper

---

## 1. 概述

本报告对照 `industrial-single-node-coordination-service-v1.0-prd.md` 对 Rookeeper 项目进行架构审核。

**项目当前阶段**: Phase 0（项目基线阶段）

**核心发现**: Phase 0 完成了工程边界固定和模块职责划分，但业务功能尚未实现。目前实现的是**类型定义、协议框架、存储布局约定、传输层抽象**，而非完整的 KV 存储、WAL、快照、Watch、锁等服务端核心能力。

**审核结论**: 当前实现与 PRD 存在显著差距，主要缺失集中在核心存储引擎、持久化恢复机制、Watch 通知系统、单机锁实现、服务注册发现等方面。Phase 0 为后续 Phase 1/2/3 的功能实现奠定了良好基础，但业务逻辑层面几乎是空白。

---

## 2. 符合项

### 2.1 模块划分与架构分层

| PRD 要求 | 实现状态 | 说明 |
| --- | --- | --- |
| 单机协调内核定位 | ✅ 符合 | CLAUDE.md 明确定位为"单节点协调服务"，模块划分合理 |
| 模块化架构（协议/存储/平台/客户端/服务端） | ✅ 符合 | 6 个 crate 分工明确，依赖方向正确 |
| 状态机纯函数化原则 | ✅ 符合 | 架构基线明确要求"业务结果只由命令输入决定" |
| 平台差异下沉原则 | ✅ 符合 | `rookeeper-platform` 统一封装 OS 检测和 IPC 传输选择 |

### 2.2 协议层（rookeeper-protocol）

| PRD 要求 | 实现状态 | 说明 |
| --- | --- | --- |
| 24 字节固定帧头 | ✅ 符合 | `wire::FRAME_HEADER_LEN = 24`，字段布局正确 |
| 协议版本字段 (u16) | ✅ 符合 | `PROTOCOL_VERSION_V1 = 1` |
| 请求类型 (10 种) | ✅ 符合 | `RequestKind` 枚举包含 Get/Create/Set/Delete/List/Watch/AcquireLock/ReleaseLock/RegisterService/GetStatus |
| 事件类型 (8 种) | ✅ 符合 | `EventKind` 枚举完整实现 |
| 错误码 (0-11 + 255) | ✅ 符合 | `ErrorCode` 枚举与 PRD 完全一致 |
| 路径规范化 | ✅ 符合 | `NodePath::parse` 正确处理 `\` 转 `/`，拒绝 `..` 和 `.` |

### 2.3 数据模型

| PRD 要求 | 实现状态 | 说明 |
| --- | --- | --- |
| 树形节点结构 | ✅ 符合 | `NodeRecord` 包含 path/value/metadata/acl |
| 节点类型 (Persistent/Ephemeral/Sequential/EphemeralSequential) | ✅ 符合 | `NodeKind` 枚举完整 |
| 版本号支持 | ✅ 符合 | `NodeMetadata.version` |
| ACL 模型 | ✅ 符合 | `Acl`/`AclEntry`/`Permission` 完整，支持读/写/创建/删除/管理权限 |
| 会话租约抽象 | ✅ 符合 | `SessionLease` 包含 session_id 和 timeout_ms |
| 命令号追踪 | ✅ 符合 | `CommandId` 类型用于 create_command_id/modify_command_id |

### 2.4 配置模型

| PRD 要求 | 实现状态 | 说明 |
| --- | --- | --- |
| TransportMode 抽象 | ✅ 符合 | Auto/UDS/NamedPipe/LocalTcp 四种模式 |
| AuthMode 抽象 | ✅ 符合 | Disabled/TokenFile 模式 |
| 存储配置 | ✅ 符合 | StorageConfig 包含 root_dir/wal_dir/snapshot_dir 及滚动参数 |
| 可观测性配置 | ✅ 符合 | ObservabilityConfig 支持 JSON 日志和 metrics |
| 兼容性配置 | ✅ 符合 | CompatibilityConfig 处理路径规范化选项 |

### 2.5 存储布局约定

| PRD 要求 | 实现状态 | 说明 |
| --- | --- | --- |
| 目录结构 (wal/snapshot/state) | ✅ 符合 | `StorageLayout` 正确定义 |
| WAL 命名规则 (wal-{segment_id:016}.log) | ✅ 符合 | `wal_segment_path()` 正确实现 |
| 快照命名规则 (snapshot-{generation:016}.bin) | ✅ 符合 | `snapshot_path()` 正确实现 |
| 单实例锁文件 (rookeeper.lock) | ✅ 符合 | `DEFAULT_LOCK_FILE` 定义 |
| 校验和辅助函数 | ✅ 符合 | `checksum()` 使用 crc32fast |

### 2.6 平台适配

| PRD 要求 | 实现状态 | 说明 |
| --- | --- | --- |
| Linux UDS / Windows NamedPipe | ✅ 符合 | `default_ipc_transport()` 正确选择 |
| LocalTcp 兜底 | ✅ 符合 | 作为 fallback |
| 跨平台路径处理 | ✅ 符合 | PathBuf 跨平台兼容 |

---

## 3. 差距项

### 3.1 核心存储引擎（PRD 7.1）

| 需求 | 差距说明 |
| --- | --- |
| 树形 KV 存储实现 | ❌ **完全缺失**。仅有 `NodeRecord` 类型定义，无实际存储引擎、树形结构、节点 CRUD 实现 |
| 原子写操作 | ❌ **未实现**。无事务、版本检查、原子提交机制 |
| 读路径热点优化 | ❌ **未实现**。无缓存、无锁读等优化 |
| TTL 支持 | ❌ **未实现**。NodeMetadata 无 TTL 字段 |

### 3.2 持久化与恢复（PRD 7.6）

| 需求 | 差距说明 |
| --- | --- |
| WAL 写入逻辑 | ❌ **完全缺失**。仅 `StorageLayout` 定义路径，无实际 WAL 写入/滚动实现 |
| 快照写入逻辑 | ❌ **完全缺失**。仅定义路径命名规则，无快照序列化实现 |
| 启动恢复流程 | ❌ **未实现**。无加载快照、无 WAL 重放、无恢复状态机 |
| 命令日志确定性 | ❌ **未验证**。无命令日志格式、无回放测试 |
| 恢复时间目标 (<500ms) | ❌ **未验证**。无恢复流程实现 |

### 3.3 Watch 通知系统（PRD 7.3）

| 需求 | 差距说明 |
| --- | --- |
| 持久订阅存储 | ❌ **完全缺失**。仅定义 `EventKind` 枚举，无订阅关系管理 |
| 事件排序与投递 | ❌ **未实现**。无事件队列、无顺序保证 |
| 前缀过滤 | ❌ **未实现**。无路径前缀匹配逻辑 |
| 背压策略 | ❌ **未实现**。无积压保护、无慢消费者隔离 |
| 订阅恢复 | ❌ **未实现**。重启后订阅丢失 |

### 3.4 单机锁（PRD 7.4）

| 需求 | 差距说明 |
| --- | --- |
| 互斥锁语义实现 | ❌ **完全缺失**。仅有 `RequestKind::AcquireLock`/`ReleaseLock` 枚举值，无锁状态机 |
| 阻塞/非阻塞/超时获取 | ❌ **未实现** |
| 锁状态查询 | ❌ **未实现**。无持有者/等待队列查询接口 |
| 异常退出锁释放 | ❌ **未实现**. 基于会话终止的锁清理逻辑缺失 |
| 锁事件通知 | ❌ **未实现**. 无 LockAcquired/LockReleased 事件投递 |

### 3.5 服务注册发现（PRD 7.5）

| 需求 | 差距说明 |
| --- | --- |
| 服务实例注册 | ❌ **完全缺失**。仅有 `RequestKind::RegisterService` 枚举值，无实际注册逻辑 |
| 临时节点生命周期 | ❌ **未实现**. 无会话关联的临时节点自动清理 |
| 实例属性查询 | ❌ **未实现**. 无属性存储/查询机制 |
| 服务发现查询 | ❌ **未实现**. 无列举/查询已注册服务接口 |

### 3.6 客户端 SDK（PRD 7.2）

| 需求 | 差距说明 |
| --- | --- |
| 实际网络传输 | ❌ **完全缺失**. 仅 `ClientBootstrap` 骨架，无 UDS/NamedPipe/TCP 连接实现 |
| 同步/异步调用 | ❌ **未实现**. 无 tokio 异步客户端实现 |
| 请求/响应序列化 | ❌ **未实现**. 仅定义 `RequestFrame`/`ResponseFrame` 结构，无编解码实现 |
| 重连/超时逻辑 | ❌ **未实现**. 无连接管理 |

### 3.7 服务端运行时（PRD 核心）

| 需求 | 差距说明 |
| --- | --- |
| 请求处理循环 | ❌ **完全缺失**. `ServerBootstrap` 仅组装配置，无实际服务器启动 |
| 会话管理 | ❌ **未实现**. 无客户端连接跟踪、无会话超时清理 |
| 状态机命令处理 | ❌ **未实现**. 无命令解释器、无状态转换 |
| ACL 检查执行 | ❌ **未实现**. 仅定义 ACL 类型，无权限校验逻辑 |

### 3.8 CLI（PRD 7.10）

| 需求 | 差距说明 |
| --- | --- |
| 完整命令集 (get/set/create/delete/ls/watch/lock/acl/status) | ❌ **部分缺失**. 仅实现 `status` 和 `normalize-path`，缺少 get/set/create/delete/ls/watch/lock/acl |
| JSON 输出模式 | ❌ **未实现** |
| 日志导出/回放 | ❌ **未实现** |
| 服务安装辅助 | ❌ **未实现** |

### 3.9 可观测性（PRD 7.7）

| 需求 | 差距说明 |
| --- | --- |
| JSON 结构化日志 | ❌ **未实现**. 无日志框架集成 |
| 核心指标暴露 | ❌ **未实现**. 无 metrics 端点 |
| `/metrics` HTTP 接口 | ❌ **未实现** |
| OPC UA / MQTT 输出 | ❌ **未实现**（但 PRD 标注为 P1 可选） |

---

## 4. 风险项

### 4.1 架构完整性风险

**风险**: Phase 0 仅有类型定义和骨架代码，无法验证 PRD 核心指标（<1MB 二进制、<500ms 恢复、P99<1ms 延迟）。

**缓解建议**: Phase 1 需尽早实现核心路径并测量实际资源占用和延迟。

### 4.2 配置加载风险

**风险**: `load_config()` 当前总是返回 `ServiceConfig::default()`，明确拒绝外部配置文件。PRD 要求"首次部署尽量少依赖，单二进制+配置文件即可启动"。

**缓解建议**: Phase 1 需实现完整的 TOML 配置解析和验证逻辑。

### 4.3 传输层实现风险

**风险**: 无任何实际网络传输实现（UDS/NamedPipe/TCP）。客户端和服务端均无法真正通信。

**缓解建议**: Phase 3 优先实现一种 IPC 传输（推荐 UDS）并通过集成测试验证。

### 4.4 状态机确定性风险

**风险**: 架构要求"状态机行为必须确定性"，但尚未实现状态机，无法验证确定性约束。

**缓解建议**: Phase 1 设计状态机时需显式记录非确定性来源（如时间、随机数），并建立回放测试。

### 4.5 Watch 系统架构风险

**风险**: PRD 要求"持久 Watch"且"订阅关系需可恢复"，但当前仅定义了事件类型，无订阅模型设计。

**缓解建议**: Phase 2 需先设计订阅持久化方案（随 WAL 还是独立存储）。

### 4.6 Windows IPC 风险

**风险**: Windows Named Pipe 行为与 UDS 存在差异，文件锁语义也不同。PRD 明确标注此风险。

**缓解建议**: Phase 3 需建立跨平台一致性测试套件。

---

## 5. 建议项

### 5.1 Phase 1 优先级建议

1. **实现核心状态机和 KV 存储**
   - 定义命令枚举（CreateNode/SetValue/DeleteNode 等）
   - 实现内存树形结构（建议使用 HashMap<String, NodeRecord> 简单实现）
   - 实现版本检查和原子写
   - 实现基础 ACL 检查逻辑

2. **实现 WAL 持久化**
   - 定义命令日志格式（应与协议帧格式解耦）
   - 实现 WAL 写入和 fsync
   - 实现 WAL 段滚动逻辑
   - 实现损坏检测和基本恢复

3. **实现快照**
   - 定义快照格式
   - 实现快照写入
   - 实现启动时加载快照 + 重放 WAL

4. **实现配置加载**
   - 实现 TOML 配置解析
   - 实现配置验证
   - 支持外部配置文件路径

### 5.2 Phase 2 建议

1. **实现 Watch 系统**
   - 设计订阅存储模型（持久化）
   - 实现事件生成和排序
   - 实现按路径/前缀过滤
   - 实现背压策略

2. **实现锁机制**
   - 基于临时顺序节点封装锁语义
   - 实现阻塞/非阻塞/超时获取
   - 实现会话终止时的锁清理

3. **实现服务注册发现**
   - 实现临时节点与会话绑定
   - 实现实例属性存储
   - 实现服务发现查询接口

### 5.3 Phase 3 建议

1. **实现完整 IPC 传输**
   - 优先实现 UDS（Linux）和 NamedPipe（Windows）
   - 实现 LocalTcp 作为 fallback
   - 通过集成测试验证跨平台行为一致性

2. **实现指标和日志**
   - 集成日志框架（tracing）
   - 实现 metrics 端点
   - 实现关键操作延迟追踪

3. **完善 CLI**
   - 实现所有 PRD 要求的命令
   - 实现 JSON 输出模式
   - 实现服务安装辅助

### 5.4 架构优化建议

1. **建立演进空间验证机制**
   - 确保 WAL 格式可扩展为复制日志
   - 确保传输层可替换为 TCP+mTLS
   - 确保状态机纯函数化

2. **建立测试基线**
   - 实现路径规范化的单元测试
   - 实现 ACL 权限检查的单元测试
   - 实现存储布局的集成测试

3. **建立性能基准**
   - 尽早测量二进制大小
   - 尽早测量内存占用
   - 尽早测量基础操作延迟

---

## 6. 总结

| 维度 | 评估 |
| --- | --- |
| 工程架构完整性 | ✅ 良好 - 模块划分清晰，依赖方向正确 |
| 协议层完整性 | ✅ 良好 - 帧格式、错误码、事件类型与 PRD 完全对齐 |
| 业务功能完整性 | ❌ 显著差距 - Phase 0 仅定义类型，无业务逻辑实现 |
| PRD 符合度 | 当前实现约 20% - 主要在协议层和模型层 |
| 架构风险 | 中等 - 传输层、持久化、状态机均未实现，Phase 1 工作量大 |

**Phase 0 评价**: 作为项目基线，Phase 0 完成了其"固定工程边界"的目标，协议和模型定义完整扎实。但距离 PRD 定义的"可用的单机协调服务"还有很长的路要走。Phase 1 是关键阶段，需要实现核心存储引擎、WAL 和恢复机制。

---

## 附录：PRD 与实现对照表

| PRD 章节 | PRD 要求 | 实现文件 | 实现状态 |
| --- | --- | --- | --- |
| 7.1.3.1 | 树形节点结构 | `model.rs:NodeRecord` | ✅ 类型定义 |
| 7.1.3.2 | 路径规范化 | `model.rs:NodePath::parse` | ✅ 已实现 |
| 7.1.3.3 | 原子写操作 | - | ❌ 未实现 |
| 7.1.3.5 | 版本号 | `model.rs:NodeMetadata.version` | ✅ 类型定义 |
| 7.1.3.6 | 节点类型 | `model.rs:NodeKind` | ✅ 枚举定义 |
| 7.2.2 | 帧格式 | `wire.rs:FrameHeader` | ✅ 24字节帧头 |
| 7.2.3 | 请求类型 | `wire.rs:RequestKind` | ✅ 10种请求 |
| 7.3 | Watch 系统 | `wire.rs:EventKind` | ⚠️ 仅枚举定义 |
| 7.4 | 单机锁 | `wire.rs:AcquireLock/ReleaseLock` | ⚠️ 仅枚举定义 |
| 7.5 | 服务注册 | `wire.rs:RegisterService` | ⚠️ 仅枚举定义 |
| 7.6.3 | WAL + 快照 | `storage-layout.md`, `storage/lib.rs` | ⚠️ 仅路径约定 |
| 7.7 | 可观测性 | `config.rs:ObservabilityConfig` | ⚠️ 仅配置结构 |
| 7.8 | 平台适配 | `platform/lib.rs` | ✅ 传输选择正确 |
| 7.9 | ACL | `acl.rs` | ✅ 类型定义，未实现检查 |
| 7.10 | CLI | `cli/main.rs` | ❌ 仅有 status/normalize-path |