# Workflow: design-doc — 方案设计

本文件是该 workflow 的唯一规范源。路由规则与场景信号见 `.claude/agents/leader.md`，主对话按本文件编排阶段序列执行。

## 适用场景

设计方案 / 架构设计 / 技术方案 / 流程设计 / 如何实现（弱信号：设计 / 规划）。产出设计文档，不实施。

## 阶段序列

| 阶段 | 负责人 agent | 输入 | 产物 |
|------|-------------|------|------|
| 1. 范围确认 | pm | 用户请求 | SDD 合规确认 + 需求上下文 |
| 2. 推演与设计 | architect | 需求上下文 | 推演 R-XXX（architect-reasoning.md）+ design 文档 |
| 3. 架构审查 | reviewer | design 文档 | architecture-review 分级报告 |
| 4. 决策 | architect | 审查报告 | ADR（architect-memory.md） |
| 5. 归档 | pm | 全部产物 | 设计资产归档（specs/ 或 docs/，pm-memory） |

## 裁剪规则

- 走 SDD 全套（Spec → Design）；架构变更受 architecture-principles 硬性约束，先推演再设计

## 边界与升级

- 本 workflow 只出设计产物，实施另起 feature-dev（设计资产复用）
- 设计涉及 DB schema / 对外契约 / FDA 合规 → reviewer 追加 sql-migration-review / security-review 维度
