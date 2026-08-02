---
name: test-design
description: 测试用例设计 Skill — 在编码执行前针对变更 crate 设计测试用例清单（函数命名、边界条件、异常路径、隔离方案），供 unit-tester 在 PreToolUse 阶段调用，实现测试先行（TDD）；编码后按清单编写并运行测试
---

# Test Design — 测试用例设计

在**编码执行之前**针对本次变更的 crate 设计测试用例清单，实现测试先行（TDD）。unit-tester 在 PreToolUse 阶段（Write/Edit `**/*.rs` 触发）调用本 skill，输出用例设计供 software-engineer 编码参考、供编码后测试执行。

## 设计流程

### Step 1: 定位变更 crate 与测试范围

| 修改的文件 | 对应测试方式 | 说明 |
|-----------|-------------|------|
| `crates/rookeeper-protocol/src/model.rs` | `cargo test -p rookeeper-protocol` | 内联 `#[cfg(test)] mod tests` |
| `crates/rookeeper-protocol/src/wire.rs` | `cargo test -p rookeeper-protocol` | 内联测试 |
| `crates/rookeeper-storage/src/lib.rs` | `cargo test -p rookeeper-storage` | 内联测试 |
| `crates/rookeeper-server/src/lib.rs` | `cargo test -p rookeeper-server` | 内联测试 |
| `crates/rookeeper-cli/src/main.rs` | `cargo test -p rookeeper-cli` | 内联测试 |
| 纯 trait 定义/接口 | 跳过（trait 无需直接测试，测试具体实现） | — |
| 配置/常量定义 | 跳过或按需（值不变时无需测试） | — |

### Step 2: 阅读变更代码，提取可测行为

- 读取变更文件的公开函数签名、业务分支、状态流转
- 标记需要 Mock 的依赖（trait 类型）
- 标记输入约束（长度/格式/非空）与异常路径

### Step 3: 输出测试用例清单

```
## 测试用例设计（{TypeName}）

### 用例清单
| # | 测试函数名 | 场景 | 输入 | 预期结果 |
|---|-----------|------|------|---------|
| 1 | {fn}_{scenario}_{expected} | 正常路径 | ... | ... |
| 2 | {fn}_{scenario}_{expected} | 边界条件 | ... | ... |
| 3 | {fn}_{scenario}_{expected} | 异常路径 | ... | ... |

### 依赖隔离（Mock 策略）
- Mock：{trait1}（{用途}）
- Mock：{trait2}（{用途}）

### 覆盖检查
- [ ] 正常路径（Happy Path）
- [ ] 边界条件（空值/超长/临界值）
- [ ] 异常路径（非法输入/权限不足/不存在）
- [ ] 状态流转守卫（如适用）
```

## 用例设计规范

### 测试函数命名
`{fn_name}_{scenario}_{expected}`，如：
- `parse_normal_path_returns_ok`
- `parse_parent_segments_returns_err`
- `checksum_stable_input_returns_expected`

### 必覆盖场景
1. **正常路径**：合法输入 → 预期成功结果
2. **边界条件**：空值、超长、临界值、特殊字符
3. **异常路径**：非法输入、权限不足、目标不存在、状态不合法
4. **状态流转守卫**：确定性状态机前置条件（如适用）

### 约束
- 每个测试函数必须有断言（`assert_eq!`、`assert!` 等）
- 使用 `mockall` crate 进行 trait mock（如需）
- 遵循 Rust 标准 `#[cfg(test)]` + `#[test]` 模式
- 单次设计 ≤30s，只输出用例清单**不写测试文件**（编码后由 unit-tester 编写执行）

## 输出形式（PreToolUse hook 场景）

- 变更仅涉及注释/配置/文档 → 输出"✅ 无新测试需求，跳过测试设计"
- 变更涉及业务逻辑 → 输出完整用例清单
- 对应 crate 无测试 → 标注"⚠️ 需为 {crate_name} 添加测试"，由后续编码流程决定是否创建

## 工作约束

- 本 skill 只做**用例设计**，不编写测试代码（编写由 unit-tester 编码后执行）
- 设计基于变更代码的实际行为，不脑补未实现的功能
- 不确定的预期行为标注"需确认"