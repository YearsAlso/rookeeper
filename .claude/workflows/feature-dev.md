# Workflow: feature-dev — 新需求开发

本文件是该 workflow 的唯一规范源。路由规则与场景信号见 `.claude/agents/leader.md`，主对话按本文件编排阶段序列执行。

## 适用场景

新功能 / 新接口 / 新模块 / 实现 / 开发 / 需求（弱信号：增加 / 支持）。涉及新增行为的变更（含重构同时加功能）。

## 阶段序列

| 阶段 | 负责人 agent | 输入 | 产物 |
|------|-------------|------|------|
| 1. 需求评审 | pm | 用户请求 + 既有上下文 | 需求上下文（pm-memory） |
| 2. 方案设计 | architect | 需求上下文 | 推演 R-XXX（architect-reasoning.md）+ design 文档 + ADR（architect-memory.md） |
| 3. 实施 | software-engineer | design 文档 | 代码 + docs 同步更新（doc-template） |
| 4. 测试 | unit-tester | 代码 | 测试用例（test-design，编码前）+ 编译 + 单类测试 ≤10s |
| 5. 审查 | reviewer | 代码 + docs | 分级报告（🔴 则 audit-report 落盘） |
| 6. 归档 | pm | 全部产物 | 文档基线更新（docs-consistency-review + pm-memory） |

## 裁剪规则

- 非平凡变更必须先出规范（sdd.md：Spec → Design → Implement → Verify → Archive），评审阶段（pm → architect → reviewer）不可跳过
- 单文件 <20 行修复不属于本 workflow（走 bugfix，免 SDD）

## 边界与升级

- 新增行为为本 workflow 的判定核心；涉及 DB schema / 对外契约 / FDA 合规代码时，security-review 与 sql-migration-review 必查（hooks SQL 闸门兜底）
- 与 refactor-mechanical 的边界：行为变化（加功能）→ 本 workflow；零行为变化（搬移/改名）→ refactor-mechanical
