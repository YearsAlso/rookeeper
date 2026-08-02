# Workflow: doc-tidy — 文档整理

本文件是该 workflow 的唯一规范源。路由规则与场景信号见 `.claude/agents/leader.md`，主对话按本文件编排阶段序列执行。

## 适用场景

整理 / 清理 / 归档 / 合并 / 目录 / 格式（弱信号：文档 / 更新文档 / 重命名）。只处理文档与目录，不处理代码。

## 阶段序列

| 阶段 | 负责人 agent | 输入 | 产物 |
|------|-------------|------|------|
| 1. 基线确认（可选） | pm | 文档清单 | 文档基线 |
| 2. 文档整理 | software-engineer / doc-writer | 基线 | 模板对齐（doc-template）+ 内容更新 |
| 3. 验证 | 主对话 | 变更文档 | check-assets 通过 |
| 4. 归档 | pm | 全部产物 | 索引/映射更新（CLAUDE.md 索引 + pm-memory） |

## 裁剪规则

- pm 基线确认可选（变更范围明确时可跳过）；doc-tidy 属于轻量场景，收敛阶段

## 边界与升级

- 歧义词分流：命中歧义词（重命名 / 整理 / 合并 / 迁移）先判对象类型——代码文件（.rs / Cargo.toml）→ refactor-mechanical；文档/目录（.md / docs）→ 本 workflow
- 文档整理涉及技术内容变更（架构/接口描述改动）→ 需经 reviewer 确认文档一致性（docs-consistency-review）
