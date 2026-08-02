---
name: architecture-review
description: 架构审查 Skill — 从 crate 依赖方向、trait 设计、错误类型设计、异步架构、模块组织、特征边界、可测试性、性能考量 8 个维度审查变更方案，供 reviewer 在架构/依赖/技术选型变更时调用
---

# Architecture Review — 架构审查

从技术架构视角审查变更方案，确保代码变更符合架构设计原则、Rust 最佳实践和项目技术栈约束。reviewer 在变更涉及架构/Cargo.toml 依赖/技术选型时调用本 skill。

## 审查维度

### 1. Crate 结构与依赖方向
- 验证 crate 分层正确：`protocol` → `storage`/`platform` → `client`/`server` → `cli`
- `protocol` crate 禁止引用其他工作区 crate
- `storage` 仅引用 `protocol`，`platform` 不引用其他工作区 crate
- `client`/`server` 引用 `protocol` + `platform`
- 新增类型是否放在正确的 crate 中（模型/协议/错误 → `protocol`，存储逻辑 → `storage`，平台抽象 → `platform`）
- 模块组织是否与 `lib.rs` 中的 `pub mod` 声明一致

### 2. Trait 设计与抽象
- Trait 边界是否合理（最小接口原则，避免胖 trait）
- 外部依赖是否通过 trait 抽象隔离（序列化、存储后端、传输）
- Trait 方法签名是否合理（参数类型、返回类型、错误类型）
- 是否使用 `#[async_trait]` 或直接 `async fn in trait`（Rust 1.82 支持 AFIT）
- 泛型参数是否过于复杂（`impl Trait<A, B, C, D>` 可能需要重构）

### 3. 错误类型设计
- 领域错误是否使用 `thiserror` 派生枚举
- 错误类型是否实现了 `std::fmt::Display` 和 `std::error::Error`
- 错误码是否按功能分组（0-99 业务/100-199 权限/200+ 系统）
- 是否混用 `anyhow::Error` 和自定义错误类型（边界清晰：库代码用自定义错误，应用代码用 `anyhow`）

### 4. 异步架构
- 异步边界是否清晰（同步函数不调用异步函数）
- `tokio` 运行时特性是否合理（`rt-multi-thread` 用于服务端，`rt` 用于 CLI 工具）
- `select!` 分支是否全面（`else` 分支处理超时，`complete` 分支处理已完成）
- 是否避免 `tokio::spawn` 导致的未捕获 panic
- 状态机逻辑是否确定性（不依赖 `Instant::now` 或随机数做决策）

### 5. 模块组织与可见性
- `pub` 导出是否最小化（不暴露内部实现细节）
- `pub use` 重导出是否合理（`prelude` 模块模式）
- 模块间耦合是否松散（消息传递优于共享状态）
- 循环依赖检测（Rust 编译器不允许循环模块引用，但检查 trait 层面的间接循环）

### 6. 特征边界与外部依赖
- 新增 crate 依赖是否必要（优先复用现有技术栈：`anyhow`、`bytes`、`serde`、`tokio`、`tracing`、`thiserror`、`toml`、`crc32fast`、`clap`）
- 依赖版本是否对齐根 `Cargo.toml` 的 `[workspace.dependencies]`
- 是否引入与现有技术栈冲突的依赖
- 新依赖的引入是否有充分的架构理由

### 7. 可测试性与可维护性
- 逻辑是否通过 `#[cfg(test)]` 内联测试覆盖
- 函数长度是否可控（≤ 60 行），圈复杂度是否过高
- 是否存在过度 `clone()` 影响性能
- 测试是否覆盖关键路径（错误处理、边界条件、状态机转换）

### 8. 性能考量
- 热路径是否有不必要的分配（优先 `&str` 而非 `String`，`Cow` 处理借用/拥有）
- 大结构是否使用 `Box<dyn Trait>` 或 `enum` 分发（避免 trait object 的 vtable 开销）
- 序列化/反序列化是否有性能瓶颈
- 二进制协议是否使用 `bytes::Bytes` 零拷贝切片

## 执行流程

### Step 1: 获取变更范围
```bash
git diff main...HEAD --name-only   # 分支差异
git diff HEAD --name-only          # 未提交变更
```

### Step 2: 按 crate 分类分析
| 变更路径 | 重点审核维度 |
|----------|-------------|
| `crates/rookeeper-protocol/**` | 1, 3, 5, 7 |
| `crates/rookeeper-storage/**` | 1, 2, 4, 6, 7, 8 |
| `crates/rookeeper-platform/**` | 1, 2, 5, 6, 7 |
| `crates/rookeeper-client/**` | 1, 2, 4, 7 |
| `crates/rookeeper-server/**` | 1, 2, 4, 7, 8 |
| `crates/rookeeper-cli/**` | 1, 4, 7 |
| `Cargo.toml` | 1, 6 |
| `docs/**` | 5 |

### Step 3: 输出架构审查结论

```
## 架构审查报告
### Crate 结构与依赖方向  ✅ / ⚠️ ...
### Trait 设计与抽象  ✅ / ⚠️ ...
### 错误类型设计  ✅ / ⚠️ ...
### 异步架构  ✅ / ⚠️ ...
### 模块组织与可见性  ✅ / ⚠️ ...
### 特征边界与外部依赖  ✅ / ⚠️ ...
### 可测试性与可维护性  ✅ / ⚠️ ...
### 性能考量  ✅ / ⚠️ ...
### 架构风险项
| 风险等级 | 文件:行号 | 问题描述 | 建议方案 |
### 结论
✅ 架构审核通过 / ⚠️ 存在 N 项建议 / ❌ 存在 M 项阻断
```

## 与相关 skill 的分工

| skill | 分工 |
|-------|------|
| `architecture-principles` | 本 skill 的审查底线：硬性约束（crate 分层/trait 抽象/重构纪律/代码质量/渐进改造/开闭原则）逐项校验，发现违规按输出规则告警 |
| `architecture-reasoning` | 前瞻推演（长期影响 + 短期需求变化可能性），本 skill 只审现状；架构变更审查前先核查推演记录是否存在 |
| `architect-memory` | 审查产生决策后，由 reviewer 调用追加 ADR 记录 |

## 工作约束

- 审查时不修改代码，只输出架构审查报告和建议
- 每个发现必须标注：`文件:行号` + 审核维度 + 风险等级 + 建议方案
- 不确定的技术判断标记"需人工确认"
- 审查聚焦于 diff 中的新增/修改代码，不要求重构现有代码
- 发现架构决策/技术选型结论时，由 reviewer 调用 architect-memory skill 记录 ADR