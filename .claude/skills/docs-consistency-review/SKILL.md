---
name: docs-consistency-review
description: 文档一致性与 SDD 合规审查 Skill — 比对代码实现与 docs/ 文档一致性，审核 SDD（Spec→Design→Implement→Verify→Archive）流程合规，分析变更影响范围，供 reviewer 在 docs/specs 变更及综合审查时调用
---

# Docs Consistency Review — 文档一致性与 SDD 合规审查

审查代码实现与 `docs/` 技术文档的一致性，并监督 SDD（Spec-Driven Development）流程合规。reviewer 在 `docs/**`、`specs/**` 变更及综合审查时调用本 skill。

## 职责

### 1. 文档一致性审核（核心职责）

比对代码实现与 `docs/` 文档的一致性：

| 文档 | 比对内容 |
|------|--------|
| `docs/architecture-baseline.md` | 项目结构、crate 职责、分层原则 |
| `docs/protocol-baseline.md` | 帧格式、请求/事件类型、版本号 |
| `docs/storage-layout.md` | 目录布局、WAL/快照命名、校验规则 |

**差异处理**：
- 文档正确，代码有偏差 → 修复代码
- 代码正确，文档未更新 → 更新文档（由 software-engineer 编码时同步，此处标记差异）
- 文档模糊无法判断 → 向用户提问确认

**审核维度**：
- **架构一致性**：crate 分层、依赖方向、模块职责
- **协议一致性**：帧头结构、请求/事件类型、错误码
- **存储一致性**：目录布局、文件命名、校验和算法

### 2. SDD 流程合规检查

审核本次变更是否符合 SDD 规范：
- 非平凡变更是否走 Spec → Design → Implement 流程
- 是否存在对应的 spec 文档
- 实现是否与 spec/design 一致
- 变更后是否更新了 `docs/` 文档（代码与文档同改原则）

**Spec 内容要求（每个 Spec 必须包含）**：业务背景、功能范围、接口契约、数据模型、合规影响、边界条件

**SDD 工作流**：`specs/<name>/spec.md` → `specs/<name>/design.md` → 代码实现 → 验证 → 归档到 `docs/`

### 3. 变更影响分析

分析本次变更的影响范围：
- 影响哪些 `docs/` 文档需要更新
- 是否影响协议兼容性（帧格式、请求类型）
- 是否影响存储布局（是否需要新数据目录）
- 是否影响客户端/CLI 接口

## 执行流程

### Step 1: 获取变更范围
```bash
git diff main...HEAD --name-only
```

### Step 2: 确定相关文档
根据变更文件路径，确定需要比对的 `docs/` 文档：
- `crates/rookeeper-protocol/**` → 协议文档 + 架构文档
- `crates/rookeeper-storage/**` → 存储布局文档 + 架构文档
- `crates/rookeeper-server/**` → 架构文档
- `crates/rookeeper-cli/**` → 架构文档
- `crates/rookeeper-platform/**` → 架构文档

### Step 3: 逐项比对
打开相关文档，逐项比对代码实现与文档描述；同时检查 `docs/memory/` 中 PM 记忆记录的已知差异基线。

### Step 4: 输出报告

```
## 文档一致性审查报告

### 变更概览
- 分支：xxx
- 变更文件：N 个
- 变更类型：新功能 / Bug 修复 / 重构 / 文档

### SDD 合规检查
- ✅ 流程合规 / ⚠️ 非平凡变更缺少 Spec

### 文档一致性审核
- ✅ 架构一致性：...
- ✅ 协议一致性：...
- ✅ 存储一致性：...

### 差异项
| 文件:行号 | 文档描述 | 代码实现 | 建议 |
|-----------|---------|---------|------|
|           |          |          | 更新文档 / 修复代码 / 需确认 |

### 影响分析
- 📋 需更新的文档：...
- ⚠️ 风险项：...

### 结论
✅ / ⚠️ / ❌
```

## 工作约束

- 审查时不修改业务代码和文档，只输出审查报告和差异分析
- 发现差异时给出明确处理建议（更新文档 / 修复代码 / 需确认）
- 报告必须包含文件:行号级别的精确定位
- 发现文档一致性差异/需求上下文结论时，由 reviewer 调用 pm-memory skill 追加记录