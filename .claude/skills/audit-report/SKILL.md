---
name: audit-report
description: 审计报告编写 Skill — reviewer 完成审查后按 docs/templates/template-审计报告.md 模板，将分级审查发现落盘为 docs/audit-reports/YYYY-MM-DD-{scope}.md 规范审计报告，并登记 docs/audit-reports/README.md 发现账本（open/fixed/waived），供历史检索与重复问题统计
---

# Audit Report — 审计报告编写规范

将 reviewer 的审查发现固化为**可检索、可跟踪、可复用**的审计报告。审计报告是过程证据（谁审了什么、发现了什么、状态如何），与 `architect-memory` / `pm-memory` 的结论沉淀互补：报告回答"这次审查发现了什么"，记忆回答"沉淀了什么规则"。

## 触发时机

| 触发 | 是否落盘 | 说明 |
|------|---------|------|
| `/review` 全维度审查完成 | **必做** | 每次深度审查都必须产出审计报告 |
| hooks 快速审查出现 🔴 阻断级发现 | **必做** | 阻断级问题必须留痕，便于跟踪修复 |
| hooks 快速审查仅 🟡/🔵 | 不落盘 | 轻量反馈，避免噪音 |
| 用户显式要求审计 | **必做** | 按用户指定范围审计 |

## 报告落盘位置

- 报告文件：`docs/audit-reports/{YYYY-MM-DD}-{scope}.md`
  - `scope` = 变更主题短名，如 `workorder-approval`、`device-auth-middleware`
  - 同日多份时追加序号：`{YYYY-MM-DD}-{scope}-{n}.md`
- 发现账本：`docs/audit-reports/README.md`（每份报告追加一行，见"账本登记"）
- 模板来源：`docs/templates/template-审计报告.md`（doc-template 检查顺序：先 `docs/template/` 单数目录，再 `docs/templates/` 复数目录；本模板登记在复数目录）

## 执行流程

### Step 1: 收集审查输入

- 变更范围：`git diff main...HEAD --name-only`（或传入文件列表）
- 各审查维度输出：rust-syntax-review / security-review / architecture-review / docs-consistency-review / sql-migration-review / ui-impact-review 的分级发现
- 核心边界命中：变更是否触及哈希链 / WORM / 审计日志 / 认证签名路径（FDA 合规主线），命中则单独标注"合规影响"

### Step 2: 按模板生成报告

按 `docs/templates/template-审计报告.md` 的章节结构填写，**禁止自由发挥结构**：

1. 元信息（审计时间 / 触发方式 / 审查范围 / 路由维度）
2. 执行摘要（1-3 句 + 3 项最高优先级）
3. 统计总览（分级计数、维度计数）
4. 详细发现（P0 / P1 / P2 分级，每条含状态与证据）
5. 合规影响（FDA Part 11 相关发现单独小节）
6. 修复建议（按优先级 + 估计工时）
7. 附录

分级映射（与 reviewer 输出一致）：

| reviewer 级别 | 报告级别 | 含义 |
|--------------|---------|------|
| 🔴 阻断 | P0 | 运行时缺陷 / 安全漏洞 / 架构分层违规 / 合规风险 → 必须修复 |
| 🟡 警告 | P1 | 命名规范 / 异常处理 / 文档不一致 → 建议修复 |
| 🔵 建议 | P2 | 可改进设计 / 测试性优化 → 可选 |
| ✅ 通过 | — | 计入统计，不单列条目 |

### Step 3: 落盘报告文件

- 文件写入 `docs/audit-reports/{YYYY-MM-DD}-{scope}.md`
- 占位符 `{...}` 必须全部替换为实际内容，不保留模板示例
- 每条发现必须带 `文件:行号` 证据；误报条目必须附带"教训"

### Step 4: 账本登记

在 `docs/audit-reports/README.md` 的发现账本表追加一行：

```markdown
| {YYYY-MM-DD} | {scope} | 🔴{N} 🟡{N} 🔵{N} | {一句话核心发现} | open | {关联 spec/PR} |
```

若该表不存在则先创建 README.md（含"报告索引"与"发现账本"两节）。

### Step 5: 发现状态跟踪

| 状态 | 含义 | 更新时机 |
|------|------|---------|
| open | 已登记未修复 | 报告落盘时 |
| fixed | 已修复并经验证 | 后续审查确认修复后，在原报告状态区 + 账本更新 |
| waived | 人工批准豁免 | 人工明确批准放弃修复时，注明批准人与理由 |

修复确认后**回到原报告文件**更新对应发现的状态字段（不新建报告），保持单一事实源。

## 与记忆转交的关系

- 审计报告 = 过程证据（落盘 `docs/audit-reports/`）
- 记忆转交 = 结论沉淀（reviewer Step 5 转交 architect/pm）
- **两者都要做**：审计报告不替代 ADR / pm-memory 记录；审查产生架构决策时，报告落盘 + 记忆转交并行执行

## 工作约束

- 本 skill 只负责报告编写与落盘，不修改业务代码
- 报告内容必须与审查事实一致，禁止虚构发现或夸大级别
- 报告文件属于 `docs/**`，会被 docs-consistency-review hook 检查；审计报告是过程记录，比对时聚焦"发现与代码事实是否一致"
- 不向报告写入密钥、Token、PII 等敏感信息（发现涉及敏感泄露时，描述位置与类别，不复制明文）
- 单次审计报告生成耗时 ≤60s（复用已有审查输出，不重复审查）
