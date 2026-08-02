---
name: rust-syntax-review
description: Rust 语法审查与编码约束 Skill — 定义 rookeeper 项目的 Rust 语法规范（crate 分层/命名/错误处理/异步/不安全代码/序列化/测试/文档注释），供 reviewer 审查 Rust 变更，同时作为 software-engineer 编写 Rust 代码时必须遵循的编码约束；支持 Ollama 快扫 + Claude 深审两级模式
---

# Rust Syntax Review — Rust 语法规范与审查

本 skill 是 rookeeper 项目 Rust 代码的语法规范唯一事实源。**双用途**：
- **编码约束**：software-engineer 编写/修改 Rust 代码时必须遵循本规范
- **审查维度**：reviewer 审查 `**/*.rs` 变更时按本规范逐维度核查

## 审查维度

### 1. Crate 分层与依赖规范
- **依赖方向**：`protocol` 无依赖 → `storage` 依赖 `protocol` → `platform` 无依赖 → `client`/`server` 依赖 `protocol`+`platform` → `cli` 依赖全部
- `protocol` crate 禁止引用其他工作区 crate
- `storage` 只引用 `protocol`，不直接引用 `platform`/`client`/`server`/`cli`
- 模块通过 `pub mod` 和 `pub use` 控制可见性，不暴露内部实现细节
- 外部 crate 依赖在根 `Cargo.toml` 的 `[workspace.dependencies]` 中声明，子 crate 使用 `.workspace = true`

### 2. 命名规范
- 类型/trait/enum：`PascalCase`（`NodePath`、`FrameHeader`、`RequestKind`、`Acl`）
- 函数/方法/变量：`snake_case`（`parse`、`load_config`、`session_id`）
- 常量/静态变量：`SCREAMING_CASE`（`PROTOCOL_VERSION_V1`、`DEFAULT_LOCK_FILE`）
- 模块名：`snake_case`（`mod wire`、`mod config`）
- Cargo feature 标记：`kebab-case`
- 类型别名：`PascalCase`（`CommandId`、`SessionId`）
- 禁止：`camelCase` 变量名、`Camel_Snake`、`Capitalized_Snake_Case`

### 3. 错误处理规范
- 所有 fallible 函数返回 `Result<T, E>` 或 `anyhow::Result<T>`
- 领域错误使用 `thiserror` 定义枚举（`#[derive(Error)]`），如 `RookeeperError`
- 错误传播使用 `?` 运算符，禁止手动 `match` + `return`
- 使用 `anyhow::Context` 的 `.context()` / `.with_context(|| ...)` 为错误附加上下文
- 生产代码禁止 `unwrap()`、`expect()`、`panic!()`、`unreachable!()`
- 使用 `#[must_use]` 标记 Result 返回值，防止忽略错误
- 错误码使用 `ErrorCode` 枚举（`rookeeper-protocol::error::ErrorCode`）

### 4. 异步规范
- 异步函数使用 `async fn` 语法，返回 `impl Future` 的 trait 方法使用 `async_trait`（如需）
- `tokio` 为运行时（`tokio = { features = ["io-util", "macros", "net", "rt-multi-thread", "sync", "time"] }`）
- 使用 `tokio::select!` 处理超时和取消
- 禁止同步阻塞：`.blocking_recv()`、`.blocking_send()`、`futures::executor::block_on()`
- 状态机逻辑必须确定性（不依赖外部时间/随机数作为决策输入）
- `async fn main` 使用 `#[tokio::main]` 宏

### 5. 不安全代码规范
- `unsafe` 代码必须加 `// SAFETY:` 注释说明安全性前提条件
- `unsafe` 块必须尽可能小，不包裹非 unsafe 代码
- `unsafe` 函数必须有 `# Safety` 文档章节说明调用者责任
- FFI 边界必须使用 `#[repr(C)]` 确保内存布局兼容
- 裸指针操作必须验证：非空、对齐、别名、生命周期

### 6. 序列化规范
- 使用 `serde` + `#[derive(Serialize, Deserialize)]`
- 枚举使用 `#[serde(rename_all = "snake_case")]` 确保 JSON/TOML 兼容
- 配置结构使用 `#[serde(deny_unknown_fields)]` 防止静默吞掉未知字段
- 二进制协议使用 `bytes::Bytes` 而非 `Vec<u8>`（零拷贝）
- 序列化优先使用 `#[serde(rename_all = "snake_case")]` 而非手动 `#[serde(rename = "...")]`

### 7. 测试组织规范
- 单元测试使用 `#[cfg(test)] mod tests { use super::*; }` 内联
- 测试函数使用 `#[test]` 属性
- 预期 panic 使用 `#[should_panic]` 属性
- 异步测试使用 `#[tokio::test]`
- 使用 `cargo test -p {crate_name}` 指定 crate 运行
- 使用 `cargo test {test_name}` 过滤器运行单个测试
- 测试函数命名：`{fn}_{scenario}_{expected}`（`normalizes_backslashes`、`rejects_parent_segments`）
- 集成测试放在 `tests/` 目录（每个文件一个 crate）

### 8. 文档注释规范
- 公开 API 使用 `///` 文档注释，crate/模块入口使用 `//!` 注释
- 复杂方法使用文档章节：`# Examples`、`# Panics`、`# Errors`、`# Safety`（unsafe 函数）
- 类型定义使用 `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]` 惯例
- 使用 `cargo doc --no-deps` 生成文档验证

## 编码约束（software-engineer 使用）

编写 Rust 代码时逐条遵循上述 8 维度规范，编码完成后按规范自检（命名 → 错误处理 → 异步 → 所有权 → 注释 → 分层 → 架构）。

## 审查流程（reviewer 使用）

### Step 1: 获取变更范围
```bash
git diff main...HEAD
```
若 diff 为空或包含未提交变更，同时获取 `git diff HEAD`。

### Step 2: 逐文件分类审查
| 文件路径 | 适用维度 |
|----------|----------|
| `crates/rookeeper-protocol/src/**/*.rs` | 1, 2, 3, 6, 7, 8 |
| `crates/rookeeper-storage/src/**/*.rs` | 1, 2, 3, 4, 6, 7, 8 |
| `crates/rookeeper-platform/src/**/*.rs` | 1, 2, 3, 4, 5, 7, 8 |
| `crates/rookeeper-client/src/**/*.rs` | 1, 2, 3, 4, 7, 8 |
| `crates/rookeeper-server/src/**/*.rs` | 1, 2, 3, 4, 7, 8 |
| `crates/rookeeper-cli/src/**/*.rs` | 1, 2, 3, 4, 7, 8 |
| `Cargo.toml`（工作区或子 crate） | 1, 5 |
| `**/*.rs`（含 unsafe 块） | 5 |

### Step 3: Ollama 两级协作（可选）

```
1. curl -s --connect-timeout 3 --max-time 5 localhost:11434/api/tags
   ├─ 200 → Ollama 可用：发送 git diff(≤500行) 做机械快扫
   │   ├─ 返回 NO_ISSUES → 提示可跳过深审（低风险变更）
   │   └─ 返回 N 条线索 → 作为审查输入，逐条验证
   └─ 其他 → 跳过，直接完整审查
2. Claude 独立审查全维度（不受 Ollama 结果限制）
3. 合并输出，Ollama 发现的标记 [Ollama]
```

**Ollama 可承担**（机械模式）：命名违规、`unwrap()`/`expect()`/`panic!()`、`#[should_panic]` 缺失、`#[serde]` 属性遗漏
**Claude 必须做**（不可委托）：crate 分层判断、unsafe 代码安全审查、异步架构正确性、trait 设计合理性、任何需要 file:line 精确引用的断言
**质量门禁**：Ollama 报告仅作线索逐条验证；Ollama 未报告的问题仍需独立审查；误报过滤后不输出

### Step 4: 输出审查结论

按严重级别输出，每条发现标注 `文件:行号` + 违反维度 + 修复方案：
- 🔴 阻断：运行时缺陷/panic 风险、数据安全漏洞、crate 分层违规 → 必须修复
- 🟡 警告：命名/规范违反、错误处理不完善、异步模式错误 → 建议修复
- 🔵 建议：可改进的设计模式、测试性优化 → 可选
- ✅ 通过：符合规范
- 全部通过 → 输出：【代码审查通过】本次变更代码符合项目规范，无潜在问题

## 工作约束

- 只审查 diff 中新增/修改的代码，不要求批量重命名现有代码
- 审查时不修改源码，只输出审查报告
- 每个发现必须标注：`文件:行号` + 违反的具体规范条款 + 修复代码示例