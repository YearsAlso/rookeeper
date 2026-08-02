# Rust Result 错误处理强制规范

## 原则

**所有 Fallible 函数必须返回 `Result<T, E>`，禁止使用 `Option` 代替错误处理，禁止静默吞掉错误。**

## 规范说明

| 场景 | 必须使用 | 示例 |
|------|---------|------|
| 可恢复的错误 | `Result<T, E>` | `fn parse(raw: &str) -> Result<Self, &'static str>` |
| 不可恢复的错误 | `anyhow::Result<T>` | `fn load_config(path: Option<&Path>) -> anyhow::Result<ServiceConfig>` |
| 领域特定错误 | `thiserror::Error` + `enum ErrorCode` | `#[derive(Error)] enum RookeeperError` |
| 无返回值的错误 | `Result<(), E>` | `fn validate(&self) -> Result<(), ValidationError>` |

## 禁止行为

- ❌ `panic!()` / `unreachable!()` / `.unwrap()` / `.expect()` 在生产代码中（除非确定不会失败）
- ❌ 使用 `Option` 代替 `Result` 传递错误信息（`None` 丢失错误原因）
- ❌ 空 `catch` 等效：`let _ = fallible_fn();` 不处理 `Err`
- ❌ `println!` / `eprintln!` 代替结构化错误日志（使用 `tracing` crate）
- ❌ 返回 `String` 作为错误类型（丢失类型信息）

## 强制检查清单

每个 Fallible 函数必须满足：

1. ✅ 返回类型为 `Result<T, E>` 或 `anyhow::Result<T>`
2. ✅ 错误类型 `E` 实现了 `std::fmt::Display`（或使用 `anyhow::Error` / `thiserror`）
3. ✅ 错误传播使用 `?` 运算符，而非手动 `match` + `return`
4. ✅ `anyhow::Context` 的 `.context()` / `.with_context(|| ...)` 为错误附加上下文信息
5. ✅ 领域错误使用 `ErrorCode` 枚举（`rookeeper-protocol::error::ErrorCode`）
6. ✅ 错误码按功能分组（0-99 业务错误，100-199 权限/安全，200+ 系统错误）

## 例外

以下场景可豁免：

- **测试代码**：可安全使用 `.unwrap()`（测试失败意味着测试条件不成立）
- **main 函数**：可以返回 `anyhow::Result<()>`（框架自动处理）
- **初始化/启动阶段**：fail-fast 策略，可 panic 或 `expect` 明确失败原因
- **`infallible` 操作**：如 `Vec::push`、`HashMap::insert` 等确定不会失败的操作