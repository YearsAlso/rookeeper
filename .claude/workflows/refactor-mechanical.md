# Workflow: refactor-mechanical — 机械性代码修改

本文件是该 workflow 的唯一规范源。路由规则与场景信号见 `.claude/agents/leader.md`，主对话按本文件编排阶段序列执行。

## 适用场景

重命名代码 / 批量替换 / 方法迁移 / 跨类搬移 / 文件移动 / 批量格式化 / 结构调整（弱信号：重构 / 重命名 / 迁移 / 整理（代码）/ 拆分 / 合并类）。前提是**行为不变**——只改结构不改语义。

## 阶段序列

| 阶段 | 负责人 agent | 输入 | 产物 |
|------|-------------|------|------|
| 1. 影响面扫描 | code-indexer | 变更目标 | 引用清单（被改符号的全部使用点） |
| 2. 实施 | software-engineer | 引用清单 | 改名/搬移 + 全引用更新（一次改完，禁止遗留旧名） |
| 3. 验证 | unit-tester | 变更代码 | 编译通过 + 行为不变回归（针对性用例） |
| 4. 轻量审查 | reviewer | 变更代码 | rust-syntax-review + 无行为变化确认 |
| 5. 归档 | 主对话 | 全部产物 | check-assets / cargo build；无需 ADR |

## 裁剪规则

- 跳过 pm 需求评审、architect 方案设计、ADR/推演（机械修改无需求歧义、无架构决策）
- 保留实施、验证、审查三阶段（机械修改的最大风险是漏改引用与误改行为）

## 升级红线（任一命中 → 立即升级，场景不成立）

| 触发条件 | 升级目标 | 介入者 |
|----------|---------|--------|
| 跨层依赖 / DI 注册变更 | feature-dev | architect 架构评审 + reviewer |
| 公共 API / 对外契约变更（接口签名 / 路由 / DTO 对外字段） | feature-dev | pm 契约确认 + architect |
| DB schema / 迁移脚本变更 | feature-dev | architect + reviewer（sql-migration-review，hooks SQL 闸门兜底） |
| 外部行为变化（方法语义 / 返回值 / 异常行为） | bugfix / feature-dev | 按行为变化性质路由 |
| FDA 合规代码（audit / signature / WORM / hash chain / tpm） | feature-dev | reviewer security-review 必查 + 深度审查 |

## 边界与切换

- 与 bugfix：bugfix 改行为（修），本 workflow 不改行为（搬）；"修 bug 同时重命名"按 bugfix
- 与 feature-dev：feature-dev 加行为，本 workflow 零行为变化；"重构同时加功能"按 feature-dev
- 与 doc-tidy：按对象类型分流——代码文件 → 本 workflow，文档/目录 → doc-tidy
- 执行中发现行为变化风险 → 中止并切换目标 workflow（输出切换记录）
