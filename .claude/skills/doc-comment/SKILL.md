---
name: doc-comment
description: 注释编写规范 Skill — rookeeper 项目的 Rust 文档注释编写规范（/// 文档注释、//! 模块注释、文档章节、禁止场景），software-engineer 编码时强制遵循，doc-writer 补齐注释时同样使用；支持 Ollama 生成初稿 + Claude 复审
---

# Doc Comment — 注释编写规范

rookeeper 项目 Rust 代码的注释规范唯一事实源。**双用途**：
- **编码约束**：software-engineer 编写/修改 Rust 代码时按本规范添加注释
- **补齐工具**：doc-writer 为已变更代码补齐缺失注释时按本规范执行

核心准则：**只补缺失注释、绝不堆砌冗余说明，所有注释统一简体中文，只描述业务含义，不复述代码逻辑**。

## 注释格式强制规则

### Rust 文档注释

`///` 文档注释用于公开 API 的文档，`//!` 用于模块级（crate 或 mod）文档：

```rust
// ✅ 正确：/// 文档注释在函数上方
/// 解析并规范化路径字符串
///
/// 失败场景：路径包含 `..` 父路径遍历时返回错误
fn parse(raw: &str) -> Result<Self, &'static str>;

// ✅ 正确：//! 模块级注释在文件顶部
//! 模型定义 - 节点路径、会话、节点元数据等核心数据结构
```

### 单行注释

不强制使用 `///`，简单函数和字段优先使用 `//` 单行注释：

```rust
// 幂等构造：从根目录计算所有子路径
pub fn from_root(root: impl AsRef<Path>) -> Self;

// ECDSA P-256 签名，返回 Base64
pub fn sign_hash(data_hash: &str) -> String;
```

两种形式选择标准：
- **业务语义复杂**、需要多句描述 → `///` 多行文档注释
- **一句话能说清** → `//` 单行注释

### 文档章节（可选，复杂方法使用）

```rust
/// 从根目录构造完整的存储布局
///
/// 所有子目录和文件都相对于根目录计算
///
/// # Examples
/// ```
/// let layout = StorageLayout::from_root("data");
/// assert!(layout.wal_segment_path(7).ends_with(".log"));
/// ```
///
/// # Errors
/// 如果根路径不存在或无法访问，返回 `io::Error`
pub fn from_root(root: impl AsRef<Path>) -> Self;
```

## 分文件强制注释规范

### 1. Crate 入口（lib.rs）
每个 crate 的 `lib.rs` 顶部必须有 `//!` 模块注释，说明该 crate 的职责和设计原则：
```rust
//! rookeeper-protocol
//!
//! 协调服务的共享协议定义，作为整个项目的核心类型来源。
//! 其他所有 crate 都从此处导入类型，而不是自行重新定义。
```

### 2. 公开类型定义（struct / enum）
- 公开 struct 必须有 `///` 文档注释说明用途
- 业务含义不明显的字段加行注释（如 `modify_command_id`、`create_command_id`）
- 含义不直观的枚举值加 `///` 注释
- 自解释的字段/枚举值不重复注释（如 `version: u64`、`session_id: SessionId`）

### 3. 公开函数和方法
- 公开函数必须有 `///` 文档注释说明业务作用
- 返回 `Result` 且存在特定错误场景时，建议加 `# Errors` 章节
- `pub(crate)` / 私有函数不强制加注释，复杂逻辑按需加 `//` 注释

### 4. Trait 定义
- Trait 名必须有 `///` 文档注释说明职责
- 每个 trait 方法必须有 `///` 文档注释

### 5. 模块（mod）
- 每个 `mod` 定义上方加 `//!` 或 `///` 注释说明模块职责
- 子模块内容在 `mod.rs` 或 `lib.rs` 中声明时加注释

## 绝对禁止添加注释的场景

1. 自解释的字段名（`id: u64`、`name: String`、`version: u64`、`session_id: SessionId`）
2. `pub use` 重导出（文档已存在于原定义处）
3. 明显的 getter/setter 方法
4. 简单 Lambda/闭包 `|x| x.id == id`
5. 一行能看懂的链式调用
6. 标准 trait 实现（`Display`、`Debug`、`Default`、`Serialize`、`Deserialize`）

## Ollama 两级协作（可选）

> 注释初稿生成复用 `/ollama-call` skill 的 docs 模板（探测命令、超时、回退逻辑见该 skill，此处为执行摘要）。

```
1. curl -s --connect-timeout 3 --max-time 5 localhost:11434/api/tags
   ├─ 200 → Ollama 可用：发送方法签名+上下文(≤50行) 生成注释初稿
   │   ├─ 返回含 UNCERTAIN → 丢弃，直接编写
   │   └─ 返回正常 → 复审术语准确性 + 规范合规性
   └─ 其他 → 跳过，直接编写
2. 复审：检查 /// 注释格式、禁止场景、业务语义正确性
3. 执行 Edit/Write 写入文件
```

**质量门禁**：
- Ollama 输出含 "UNCERTAIN" → 丢弃，重写
- Ollama 生成的注释违反"绝对禁止"规则 → 删除
- 术语不符合项目 domain（节点/路径/会话/命令/租约）→ 修正

## 工作约束

1. 仅新增/调整注释文本，**严禁改动任何业务代码逻辑**
2. 注释语言精简，一句话讲清业务目的
3. 若当前文件注释全部规范完整，仅输出：【文档校验完成】当前文件注释规范完整，无需补充