# Workflow: design-eval — 方案评估

本文件是该 workflow 的唯一规范源。路由规则与场景信号见 `.claude/agents/leader.md`，主对话按本文件编排阶段序列执行。

## 适用场景

评估 / 对比 / 选型 / 可行性 / 该不该 / A还是B（弱信号：方案 / 考虑 / 权衡）。只评估不实施。

## 阶段序列

| 阶段 | 负责人 agent | 输入 | 产物 |
|------|-------------|------|------|
| 1. 范围确认 | pm | 用户请求 | 需求上下文 + 评估范围 |
| 2. 方案对比 | architect | 评估范围 | ≥3 方案收益/风险分析 |
| 3. 决策 | architect + pm | 对比分析 | ADR（architect-memory.md）+ pm 记录（pm-memory.md） |

## 裁剪规则

- 轻量场景收敛阶段，不强制全套五段（无需实施/测试/审查阶段）

## 边界与升级

- 评估结论若进入实施 → 另起 feature-dev（评估资产复用，如 ADR）
- 涉及技术选型/架构方向 → architect 按 architecture-reasoning 完成推演后决策
