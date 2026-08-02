# Workflow: bugfix — Bug 修复

本文件是该 workflow 的唯一规范源。路由规则与场景信号见 `.claude/agents/leader.md`，主对话按本文件编排阶段序列执行。

## 适用场景

报错 / bug / 崩溃 / 异常 / 不生效 / 404 / 500 / 修复 / 改一下（弱信号：逻辑问题 / 行为不符 / 验证）。行为修复（改行为使结果正确）。

## 阶段序列

| 阶段 | 负责人 agent | 输入 | 产物 |
|------|-------------|------|------|
| 1. 场景确认 | pm | 用户请求 + 复现信息 | bug 影响范围 + 合规边界 |
| 2. 定位 | code-indexer | bug 描述 | 根因分析（需求→API→Controller→Service→Repository→DB 全链路） |
| 3. 最小修复 | software-engineer | 根因 | 代码修改（<20 行免 SDD） |
| 4. 回归测试 | unit-tester | 修复代码 | 针对性用例 + 编译 + 单类测试 ≤10s |
| 5. 轻量审查 | reviewer | 修复代码 | security 必查；🔴 则 audit-report 落盘 |

## 裁剪规则

- 最小修复 <20 行免 SDD（sdd.md）；轻量审查不等同免审查，security 维度必查

## 边界与升级

- 修复同时伴随重命名/搬移 → 主体按本 workflow（行为修复优先）
- FDA 合规相关代码（audit / signature / WORM / hash chain / tpm）的 bug 修复 → security-review 必查，必要时升级深度审查（/review）
- 修复范围扩大为新增功能/重构 → 中止并切换 feature-dev（输出切换记录）
