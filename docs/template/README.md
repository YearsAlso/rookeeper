# 文档模板中心

本目录是 SIDU-LATS 项目的**标准文档模板目录**（doc-template skill 的强制检查位置）。

> **兼容说明**：`docs/templates/`（复数）为历史模板目录，包含 `template-开发规范文档.md`、`template-设计方案文档.md`、`template-审查结果文档.md`、`template-数据契约文档.md`。doc-template skill 检查顺序：**先查 `docs/template/`（单数，标准），无匹配时再查 `docs/templates/`（复数，历史）**；新模板一律创建在单数目录。

## 使用规则

1. 编写/更新任何文档前，先检查本目录是否有匹配模板
2. 有模板 → 按模板章节结构填写，占位符 `{...}` 全部替换为实际内容
3. 无模板 → 提示并建议创建，参考 `docs/` 下现有同类文档结构编写
4. 新建模板后 → 同步更新下方映射表

## 文档类型 → 模板映射表

| 文档类型 | 模板文件 | 适用产出 |
|---------|---------|---------|
| 接口文档 | `接口文档模板.md` | API 路径、请求/响应、认证、错误码 |
| 数据库设计 | `数据库设计模板.md` | 表结构、字段、索引、FK、RULE |
| 技术架构/方案 | `技术架构模板.md` | 分层、依赖、关键模式、合规路径 |
| Spec 规范 | `Spec模板.md` | `specs/<name>/spec.md` + `design.md` |
| 开发规范（历史） | `../templates/template-开发规范文档.md` | 编码/命名/约束规范 |
| 设计方案（历史） | `../templates/template-设计方案文档.md` | 详细设计文档（模块级） |
| 审查结果（历史） | `../templates/template-审查结果文档.md` | 审查报告 |
| 审计报告 | `../templates/template-审计报告.md`（复数目录，audit-report skill 专用） | `docs/audit-reports/` 审计报告（reviewer 落盘） |
| 数据契约（历史） | `../templates/template-数据契约文档.md` | DTO/契约定义 |

## 模板编写规范

- 模板使用 `{占位符}` 标注需填写位置
- 章节结构可复用 `docs/` 现有文档的成熟结构
- 每个模板必须包含：用途说明、章节结构、填写示例
- 模板只定义结构与填写要点，不承载实际内容
