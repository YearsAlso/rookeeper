---
name: api-smoke-test
description: CLI 冒烟测试 — 编译后对所有 CLI 子命令和服务器引导命令执行测试，验证输出格式和错误处理
---

# /api-smoke-test — CLI 冒烟测试

代码变更涉及 CLI 命令或服务器引导时使用。编译、运行命令、验证输出。

## 执行流程

### Step 1: 确定测试范围

```bash
git diff main...HEAD --name-only
```

根据变更文件确定需要测试的目标：
- `crates/rookeeper-cli/src/**/*.rs` → CLI 所有子命令
- `crates/rookeeper-server/src/**/*.rs` → 服务器引导命令
- `Cargo.toml` → 所有 crate 编译检查

### Step 2: 编译验证

```bash
cargo check --workspace 2>&1
```

### Step 3: 核心命令测试

#### 3.1 CLI 状态命令
```bash
cargo run -p rookeeper-cli -- status
# 预期输出协议版本、操作系统、默认传输方式、端点
```

#### 3.2 路径规范化命令
```bash
# 正常路径
cargo run -p rookeeper-cli -- normalize-path "\plant\\line1/robot3//config"
# 预期输出：/plant/line1/robot3/config

# 安全路径（拒绝父路径遍历）
cargo run -p rookeeper-cli -- normalize-path "/plant/../config"
# 预期输出：错误（parent path segments are not allowed）
```

#### 3.3 服务器引导
```bash
cargo run -p rookeeper-server -- --print-layout
# 预期输出存储布局信息
```

### Step 4: 验证检查点

| 检查项 | 通过条件 |
|--------|---------|
| 编译通过 | cargo check 返回 0 |
| 状态命令 | 输出包含 protocol_version, os, default_transport, default_endpoint |
| 路径规范化 | 反斜杠→正斜杠，重复分隔符去重，拒绝 `..` |
| 存储布局 | 输出包含 wal/snapshot/state 目录路径 |
| 错误处理 | 无效路径返回错误，非法参数返回 usage |

### Step 5: 清理

（无后台进程需要清理）

## 输出格式

```
## CLI 冒烟测试报告

### 测试范围
- 命令：N 个
- 编译方式：cargo check + cargo run

### 结果
| 命令 | 参数 | 退出码 | 输出验证 | 结果 |
|------|------|--------|---------|------|
| status | 无 | 0 | 含版本信息 | ✅ |
| normalize-path | 正常路径 | 0 | 规范化正确 | ✅ |
| normalize-path | 不安全路径 | 非0 | 错误信息 | ✅ |
| --print-layout | 无 | 0 | 含目录路径 | ✅ |

### 失败分析
| 命令 | 预期 | 实际 | 根因 |
|------|------|------|------|

### 结论
✅ / ⚠️ / ❌
```

## 前置条件

- Rust 工具链已安装（rustc 1.82+）
- 无编译错误

## 工作约束

- 不修改业务代码和测试代码
- 失败时分析根因（编译错误 / 运行时错误 / 输出格式不匹配）
- 与 `docs/architecture-baseline.md` 和 `docs/protocol-baseline.md` 比对一致性