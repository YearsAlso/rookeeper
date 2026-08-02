---
name: code-indexer
description: 代码索引导航 Skill — CocoIndex 语义/结构化检索优先、Grep/Glob 兜底；根据业务需求快速定位代码位置，建立需求→crate→模块→类型→方法的全链路映射；供 architect 推演前与 software-engineer 编码前引入
---

# Code Indexer Skill — 代码索引导航

给定一个业务需求或关键词，快速定位对应的代码文件、类型、方法、数据。**只做索引查询和路径导航，不修改任何代码。**

## 使用场景

- **architect**：架构推演（architecture-reasoning）与设计前引入，快速确认变更涉及的代码现状与影响面
- **software-engineer**：编码实现前引入，定位需求对应的 crate/模块/类型/方法，确保修改落点准确
- **reviewer**（可选）：审查前引入，定位变更范围与全链路影响

## 项目结构速查

```
crates/
├── rookeeper-protocol/     # 协议层：model, acl, error, config, wire
├── rookeeper-storage/      # 存储层：wal, snapshot, state
├── rookeeper-platform/     # 平台抽象：os, transport, endpoint
├── rookeeper-client/       # 客户端：config, request assembly
├── rookeeper-server/       # 服务端：config loading, bootstrap
├── rookeeper-cli/          # CLI：commands, output
```

## CocoIndex 快通道（优先使用，失败自动回退）

本仓库已启用 CocoIndex Code 持久化索引。**所有查询先走快通道；无结果 / 命令失败 / 输出异常时，自动回退到下方 Grep/Glob 模式，不阻塞导航。**

### 命令

```bash
ccc status                              # 索引健康（daemon 常驻，秒回）
ccc search "NodePath normalization" --lang rust --limit 5   # 语义检索，返回 File:行号 + score
ccc grep 'pub (fn|struct|enum|trait)' crates --lang rust --no-color  # 结构化检索
ccc doctor                              # 深度健康检查（慢，仅异常时用）
```

### 8 种查询模式映射

| 模式 | CocoIndex 用法 | 兜底（回退到） |
|------|----------------|----------------|
| 1 需求→crate 定位 | `ccc search "config definition" --lang rust` 语义定位 | grep `pub mod` |
| 2 需求→模块→类型 | `ccc search "NodePath implementation" --lang rust` 语义定位 | grep `pub struct` / `pub enum` |
| 3 需求→trait 定义 | `ccc search "trait abstract" --lang rust` 语义定位 | grep `pub trait` |
| 4 需求→枚举 | `ccc grep 'enum \NAME' crates/rookeeper-protocol/src` | ls + grep enum |
| 5 需求→配置 | `ccc search "ServiceConfig TOML"` | grep `ServiceConfig` |
| 6 需求→函数方法 | `ccc search "checksum calculation" --lang rust` | grep `pub fn` |
| 7 全链路追踪 | 语义检索仅辅助每跳定位（CocoIndex 无调用图） | **必须保留多步 Grep 串联** |
| 8 依赖查询 | 无需索引 | **直接 Read Cargo.toml** |

### 兜底规则（强制）

1. `ccc` 不存在 / `ccc status` 无有效输出（Ollama 未运行、daemon 异常）→ 跳过快通道
2. search/grep 无结果或 exit code ≠ 0 → 立即回退对应 Grep/Glob 模式
3. 语义结果仅作**候选**，关键落点（crate/模块/类型）须用下方精确 grep 或 Read 验证文件:行号
4. 模式 7 / 8 始终走 Grep/Read，不依赖语义检索

## 查询方法（Grep/Glob 动态定位——CocoIndex 兜底通道）

### 模式 1：需求 → crate 定位
```bash
# 搜索 crate 入口
grep -rn "pub mod" crates/rookeeper-protocol/src/lib.rs
# 搜索特定 crate 中的模块
grep -rn "pub mod" crates/rookeeper-storage/src/lib.rs
```

### 模式 2：需求 → 模块 → 类型
```bash
# 搜索 struct 定义
grep -rn "pub struct" crates/rookeeper-protocol/src/ --include="*.rs"
# 搜索 enum 定义
grep -rn "pub enum" crates/rookeeper-protocol/src/ --include="*.rs"
# 搜索类型使用
grep -rn "NodePath" crates/ --include="*.rs"
```

### 模式 3：需求 → trait 定义
```bash
# 搜索 trait 定义
grep -rn "pub trait" crates/ --include="*.rs"
# 搜索 trait 实现
grep -rn "impl.*for" crates/ --include="*.rs"
```

### 模式 4：需求 → 枚举
```bash
# 列出所有枚举
grep -rn "pub enum" crates/rookeeper-protocol/src/ --include="*.rs"
# 搜索特定枚举值使用
grep -rn "RequestKind\|EventKind\|ErrorCode\|NodeKind\|TransportMode" crates/ --include="*.rs"
```

### 模式 5：需求 → 配置
```bash
# 搜索配置结构
grep -rn "pub struct.*Config" crates/ --include="*.rs"
# 查看配置文件
cat config/rookeeper.default.toml
```

### 模式 6：需求 → 函数方法
```bash
# 搜索公开函数
grep -rn "pub fn" crates/rookeeper-storage/src/ --include="*.rs"
# 搜索特定方法
grep -rn "fn parse\|fn from_root\|fn load_config" crates/ --include="*.rs"
```

### 模式 7：全链路追踪
从入口追踪到数据：
1. `grep -rn "pub fn" crates/rookeeper-cli/src/` → 定位 CLI 命令
2. `grep -rn "rookeeper_protocol\|rookeeper_storage\|rookeeper_platform" crates/rookeeper-cli/src/` → 定位依赖使用
3. `grep -rn "pub struct" crates/rookeeper-protocol/src/` → 定位模型
4. `grep -rn "pub fn" crates/rookeeper-storage/src/` → 定位存储操作

### 模式 8：依赖查询
```bash
# 查看工作区依赖
cat Cargo.toml
# 查看特定 crate 的依赖
cat crates/rookeeper-protocol/Cargo.toml
```

## 输出格式

```
## 代码索引结果

### 需求：{用户需求}

### 全链路
- {crate} → {模块} → {类型} → {方法}

### 关键文件
| 层 | 文件 | 行号 |
|----|------|------|
| crate 入口 | `crates/rookeeper-protocol/src/lib.rs` | :N |
| 模块 | `crates/rookeeper-protocol/src/model.rs` | :N |
| 类型 | `crates/rookeeper-protocol/src/model.rs` | :N |
| 方法 | `crates/rookeeper-protocol/src/model.rs` | :N |
| 配置 | `Cargo.toml` | :N |

### 影响范围
- 涉及 crate：N 个
- 涉及模块：N 个
- 涉及类型：N 个
```

## 工作约束
- 不修改任何代码，只做索引查询和路径导航
- **优先 CocoIndex 快通道（ccc search/grep），无结果或失败自动回退 Grep/Glob 动态定位；不依赖记忆或硬编码映射**
- 每次查询给出文件:行号 级别的精确定位
- 需要时给出全链路追踪（从入口到数据）