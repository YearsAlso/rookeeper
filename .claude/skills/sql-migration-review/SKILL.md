---
name: sql-migration-review
description: 数据库迁移脚本审查 Skill — 审查迁移脚本的命名规范、幂等性、回滚约定与 docs/ 一致性，供 reviewer 在 .sql 变更时调用
---

# SQL Migration Review — 迁移脚本审查

审查 rookeeper 项目的迁移脚本，确保迁移机制安全、可复现、与文档一致。reviewer 在 `**/*.sql` 变更时调用本 skill。

## 硬性约定

### 命名规范
- 文件名格式：`NNN_描述.sql`（三位序号前缀 + 下划线 + 简短描述），如 `016_AddOperationLogsFields.sql`
- 序号必须递增且全局唯一；多个并行迁移不共用同一序号

### 回滚约定（关键）
- **禁止**在 `scripts/migrations/` 放置任何独立 rollback 脚本（如 `016_xxx_rollback.sql`）——可能被误当成正向迁移自动执行，undo 已应用的结构变更
- 回滚信息写在**同文件**内的 `-- Down` 注释块中（仅作文档参考，不会自动执行）
- 需要真正回滚时：编写新的正向迁移脚本（新序号）逆操作，或人工执行 `-- Down` 块内容并手动清理记录

### 幂等性
- `CREATE TABLE` 加 `IF NOT EXISTS`
- `CREATE TYPE` / `ALTER TYPE` 用 `DO $$ ... EXCEPTION WHEN duplicate_object THEN ...` 保护
- `ALTER TABLE ... ADD COLUMN` 用 `IF NOT EXISTS`；`DROP COLUMN` 用 `IF EXISTS`

### docs/ 一致性
- 变更前先比对 `docs/storage-layout.md` 和相关数据库设计文档，结构变更后同步更新该文档（由 software-engineer 编码时同步）

## 审核流程

```
1. 读取变更文件（目标 .sql）
2. 逐项核对：命名规范 → 幂等性 → 回滚约定 → docs/ 一致性
3. 输出审查结论：✅ 通过 / 🔴 必须修复（文件:行号 + 修复建议）
4. 如发现 scripts/migrations/ 存在 _rollback 命名文件：标记 🔴 并说明可能被误执行的后果
```

## 输出格式

```
## 迁移审查报告

### 结论
- ✅ 通过 / 🔴 必须修复

### 检查项
1. 命名规范：✅ / 🔴 ...
2. 幂等性：✅ / 🔴 ...
3. 回滚约定：✅ / 🔴（禁止独立 rollback 文件，-- Down 注释块是否存在）
4. docs/ 一致性：✅ / 🔴 ...

### 详细发现
| 文件:行号 | 风险等级 | 问题描述 | 修复建议 |
|-----------|---------|---------|---------|
```

## 工作约束

- 审查时不直接修改迁移文件，只输出审查报告（修改动作由主 Agent 执行）
- 生成新迁移时输出完整脚本草稿，供主 Agent 写入 `scripts/migrations/`
- 不确定的迁移影响（如已有数据回填）标记"需人工确认"