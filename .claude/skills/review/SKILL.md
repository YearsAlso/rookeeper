---
name: review
description: 全维度代码审查 — reviewer 单 agent 按变更位置执行全部审查 skill（rust-syntax-review / security-review / architecture-review / docs-consistency-review / sql-migration-review / ui-impact-review），汇总分级报告后按需执行 unit-tester / doc-writer / /api-smoke-test / /db-migrate，审查后更新架构师/PM 记忆
---

# /review — 全维度审查工作流

在准备提交或完成功能模块后使用，替代 hooks 快速审查的浅层模式。

> 本 skill 由 `reviewer` agent 执行。按 reviewer 的路由规则调用对应审查 skill，汇总统一分级报告。

## 工作流

### Phase 1: 按变更位置审查

由 reviewer 按变更文件位置路由审查 skill：

| 变更位置 | 执行 skill | 说明 |
|---------|-----------|------|
| `**/*.rs` | rust-syntax-review + security-review | 代码规范 + 安全双维度 |
| `**/*.sql` | sql-migration-review | 迁移脚本规范 |
| `crates/rookeeper-server/src/**`、`crates/rookeeper-cli/src/**` | ui-impact-review（附加） | 协议/CLI 影响分析 |
| 架构/Cargo.toml 依赖变更 | architecture-review | 技术架构质量 |
| `docs/**`、`specs/**` | docs-consistency-review | 文档一致性 + SDD 合规 |

**总耗时**：约 60-90s（按变更范围取最慢 skill，非累加）

### Phase 2: 汇总报告

按严重级别去重排序：

| 级别 | 含义 | 处理 |
|------|------|------|
| 🔴 阻断 | 架构违规、安全漏洞、panic 风险 | 必须修复 |
| 🟡 警告 | 规范违反、文档不一致 | 建议修复 |
| 🔵 建议 | 可优化项 | 可选 |
| ✅ 通过 | 符合规范 | 确认通过 |

### Phase 3: 按需执行

根据变更类型决定后续步骤：

- **unit-tester** — 如有 .rs 变更，编译 + 单 crate 测试验收（编码前 test-design 用例清单同步核对）
- **doc-writer** — 如有 .rs 变更且注释不足，按 doc-comment 规范补齐 Rust 文档注释
- **/api-smoke-test** — 如有 CLI/Server 变更，编译运行测试命令
- **/db-migrate** — 如有 .sql 变更，审查迁移脚本

### Phase 4: 记忆更新

审查产生架构决策/需求结论时，按需更新：

- **architect-memory** — 架构决策以 ADR 格式追加到 `docs/memory/architect-memory.md`
- **pm-memory** — 需求上下文/文档一致性/SDD 合规记录追加到 `docs/memory/pm-memory.md`

### Phase 5: 输出报告

```markdown
## Review 最终报告

### 审查结果（reviewer 按位置路由）
- ✅ rust-syntax-review: 通过
- ✅ security-review: 通过
- ✅ docs-consistency-review: 代码与文档一致

### 综合评级
- 总体：✅ 通过
- 建议操作：可以提交

### 后续执行
- unit-tester：✅ 单 crate 测试通过
- doc-writer：✅ 注释完整
- /api-smoke-test：跳过（无 CLI/Server 变更）
```

## 与 hooks 的关系

| 机制 | 触发 | 耗时 | 用途 |
|------|------|------|------|
| hooks | Write/Edit 自动 | ~60s | 快速实时反馈（reviewer 按位置快速审查） |
| `/review` | 用户按需调用 | ~60-90s | 全面深度审查（reviewer 全维度 + 记忆更新） |