---
name: ollama-call
description: 调用本地 Ollama 模型执行轻量级任务，减少 Claude API token 消耗。适用于 auto-committer 消息生成、reviewer 第一遍扫描、doc-writer 注释生成等场景
---

# Ollama Call — 本地模型调用

调用本地 Ollama 的 `ornith:9b` 模型执行轻量级任务，减少 Claude API token 消耗。

## 适用场景（按推荐度排序）

| 场景 | 模板 | 预估节省 | 说明 |
|------|------|---------|------|
| 生成 commit 消息 | `commit-msg` | 2-5K token | 替代 auto-committer agent 的消息生成部分 |
| 代码审查第一遍扫描 | `code-review` | 10-20K token | 扫描明显 bug/反模式，reviewer 复审 |
| 快速安全预审 | `security-scan` | 8-12K token | 机械安全检查，reviewer 复审 |
| 文档与注释生成 | `docs` | 5-15K token | Rust 文档注释、README、变更说明、PR 描述 |
| 通用问答 | `generic` | 不定 | 任何不需要深度代码理解的轻量问答 |

## 调用方式

### 1. 确认 Ollama 可用

```bash
curl -s --connect-timeout 3 --max-time 5 -o /dev/null -w "%{http_code}" http://localhost:11434/api/tags
```
- 返回 `200` → 可用
- 返回 `000`（连接拒绝/超时）或其他 → Ollama 未启动，**回退到 Claude 执行**
- `--connect-timeout 3`：TCP 连接超时 3 秒，本地无 Ollama 时立即失败
- `--max-time 5`：总超时 5 秒，防止 hang 住

### 2. 调用模型

```bash
curl -s --connect-timeout 5 --max-time 120 http://localhost:11434/api/chat -d @- <<'EOF'
{
  "model": "ornith:9b",
  "stream": false,
  "options": {
    "temperature": 0.2,
    "num_ctx": 2048,
    "num_predict": 300
  },
  "messages": [
    {"role": "system", "content": "<系统提示词>"},
    {"role": "user", "content": "<用户输入>"}
  ]
}
EOF
```
- `--connect-timeout 5`：TCP 连接超时 5 秒
- `--max-time 120`：总超时 120 秒（含模型推理时间），防止大输入时 hang 住

返回 JSON 格式：`{"message":{"content":"..."},"total_duration":...,"eval_count":...}`

### 3. 解析结果

```bash
# 提取 content 字段（优先 python，回退 python3）
curl -s ... | python -c "import sys,json; print(json.load(sys.stdin)['message']['content'])"
```

## 场景模板

### 模板 A：commit-msg — 生成 Conventional Commits 消息

**使用时机**：替代 auto-committer agent 的消息生成阶段（校验仍由 auto-committer 执行）

**系统提示词**：
```
You are a commit message generator. Given a git diff, generate a single Conventional Commits message.
Format: {前缀}({模块}): {中文描述}
Prefixes: feat|fix|refactor|perf|test|docs|chore|style|revert
Modules: protocol|storage|platform|client|server|cli|config|docs|scripts

Return ONLY the commit message, no explanation.
```

**用户输入**：`git diff --cached` 的输出（限制 200 行以内）

**后处理**：取返回的第一行，校验 prefix 是否合法，不合法则丢弃并用 Claude 重新生成。

### 模板 B：code-review — 第一遍扫描

**使用时机**：reviewer agent 执行完整审查前的快速扫描。发现明显问题作为线索，reviewer 逐条验证 + 深度审查。

**系统提示词**：
```
You are a Rust code reviewer. Scan the following diff for common issues:

Naming:
1. function/variable not using snake_case → WARNING (e.g. "myVar" not "my_var")
2. type/trait/enum not using PascalCase → WARNING (e.g. "node_path" not "NodePath")
3. constant not using SCREAMING_CASE → WARNING (e.g. "maxSize" not "MAX_SIZE")
4. module name not using snake_case → WARNING

Error Handling:
5. .unwrap() or .expect() used in production code (not tests) → HIGH
6. panic!() or unreachable!() used → HIGH
7. Error not propagated with ? operator → WARNING (manual match+return)
8. Fallible function returning () instead of Result → WARNING

Safety:
9. unsafe block without // SAFETY: comment → HIGH
10. Potential null pointer dereference → HIGH
11. Unnecessary clone() on hot path → MEDIUM

Async:
12. sync blocking call inside async fn (.blocking_recv, block_on) → HIGH

Output format (one finding per line, or "NO_ISSUES"):
SEVERITY|CATEGORY|FILE:LINE_SHORT|DESCRIPTION
SEVERITY: HIGH|MEDIUM|WARNING

Only report issues visible in the NEW code. Be conservative — if unsure, don't report.
```

**用户输入**：`git diff` 输出（限制 500 行以内）

**后处理**：本地模型报告的问题作为线索，reviewer 逐条验证。本地模型没报告的，reviewer 仍需独立审查。

### 模板 C：security-scan — 快速安全预审

**使用时机**：reviewer（security-review）执行前的机械安全检查，过滤明显安全风险

**系统提示词**：
```
You are a Rust security auditor for a coordination service in industrial-control environments. Scan the diff for:

1. Hardcoded secrets (keys, tokens, passwords) → RED FLAG
2. Path traversal risk (NodePath not normalizing .. segments) → RED FLAG
3. unwrap()/expect() on external input → RED FLAG
4. Sensitive data leaked in log output → RED FLAG
5. unsafe block without // SAFETY: comment → RED FLAG
6. TOCTOU race condition (check-then-use pattern on files) → RED FLAG
7. Missing #[serde(deny_unknown_fields)] on config structs → WARNING
8. Unvalidated payload_len in binary protocol → RED FLAG

Output format (one finding per line, or "CLEAN"):
SEVERITY|CATEGORY|FILE:LINE_SHORT|DESCRIPTION
SEVERITY: RED_FLAG|WARNING

Only report NEW vulnerabilities in the diff. Ignore pre-existing code.
```

**用户输入**：`git diff` 输出（限制 300 行以内）

**后处理**：Ollama 报告的 RED_FLAG 作为 reviewer（security-review）的必查项；WARNING 作为线索。reviewer 独立验证每条发现，过滤误报。

### 模板 D：docs — 文档与注释生成

**使用时机**：生成 Rust 文档注释、README 段落、变更说明、PR 描述等文档型任务，doc-writer agent 复审

**系统提示词**：
```
You are a Rust technical documentation writer for a coordination service project. Write clear, concise documentation in Chinese.

Project conventions:
- Workspace: rookeeper (6 crates: protocol, storage, platform, client, server, cli)
- Key concepts: NodePath (节点路径), WAL (预写日志), snapshot (快照), checksum (CRC32 校验), StorageLayout (存储布局), binary protocol frame (24-byte header)
- IPC: Unix Domain Socket (Linux), Named Pipe (Windows), local TCP (fallback)

Rules:
- Type/function names in backticks: `NodePath`, `parse`, `StorageLayout`
- Active voice, no redundant descriptions
- Rust doc format: /// on its own line, //! for module-level, # Examples for examples
- For README/changelog: use markdown format
- Describe business purpose, don't repeat code logic
```

**分场景子模板**：

| 子场景 | 用户输入 | 输出格式 | 后处理 |
|--------|---------|---------|--------|
| Rust 文档注释 | 方法签名 + 上下文（50 行） | `///` 文档注释 | doc-writer 复审术语准确性 |
| README 段落 | 功能描述 + 要点 | Markdown 段落 | doc-writer 复审技术准确性 |
| 变更说明 | `git log --oneline -10` | 分类 changelog | 人工确认 |
| PR 描述 | `git diff --stat` + 意图 | PR body | reviewer 复审 |

**后处理**：
- Rust 文档注释：doc-writer 抽查注释规范是否符合项目强制规则（/// 独占行、模块级 //! 注释）
- README/变更说明：doc-writer 复审技术事实是否正确
- 任何 "UNCERTAIN" → 丢弃，Claude 重写

### 模板 E：generic — 通用轻量问答

**系统提示词**：
```
You are a helpful coding assistant for a Rust coordination service project (rookeeper).
Technologies: Rust 1.82, tokio async runtime, serde serialization, CRC32 checksum, WAL + snapshot persistence, binary protocol (24-byte frame header), anyhow/thiserror error handling.

Answer the following question concisely.
If you don't know or are unsure, say "UNCERTAIN" and the question will be escalated to a more capable model.
```

**用户输入**：任意问题

## 执行流程

```
1. 接收任务 → 判断是否匹配上述 5 个模板
2. Bash: curl 探测 Ollama 是否可用
   ├─ 不可用 → 回退到 Claude 直接执行
   └─ 可用 → 继续
3. Bash: 准备 diff/输入（限制长度），调用 curl 发送到 Ollama
4. 解析返回结果
5. 质量门禁：
   ├─ commit-msg: 校验 prefix 合法性 → 不合法则丢弃
   ├─ code-review: 只作为线索，Claude 仍需独立复审
   ├─ security-scan: RED_FLAG 转入必查项，WARNING 为线索
   ├─ docs: 校验 "UNCERTAIN" → 丢弃，doc-writer 重写
   └─ generic: 包含 "UNCERTAIN" → 丢弃，Claude 重新回答
6. 将有效结果传递给下一步 Agent 处理
```

## 限制与回退

### 硬限制
- **输入长度**：diff 不超过 500 行，超过则截断（优先保留 `.rs` 文件变更）
- **连接超时**：探测 `--connect-timeout 3 --max-time 5`；API 调用 `--connect-timeout 5 --max-time 120`
- **输出长度**：`num_predict` 上限 300 token（轻量任务足够，减少生成耗时）
- **上下文窗口**：`num_ctx` 限制 2048 token（控制输入+输出总量，避免小模型 OOM）
- **温度**：temperature=0.2，保证输出稳定性

### 回退条件
以下任一情况发生，立即回退到 Claude 执行：
- Ollama 不可用（连接拒绝/超时）
- 模型返回空内容
- 输出格式不符合模板要求
- commit-msg 的 prefix 不合法
- 任何任务返回 "UNCERTAIN"

### 不可回退的场景
以下任务**禁止**使用本地模型，必须用 Claude 执行：
- 涉及 WAL 写入完整性校验逻辑
- 涉及 CRC32 校验和计算/验证
- 涉及确定性状态机转换（节点生命周期守卫条件）
- 涉及路径安全性分析（NodePath 的 `..` 防护）
- 涉及并发安全（Mutex/RwLock 的正确性分析）
- 任何需要 `file:line` 精确引用的技术断言
- 涉及架构分层/crate 依赖方向判断

## 行为约束

1. **先探测再调用** — 每次使用前先 `curl --connect-timeout 3 --max-time 5 http://localhost:11434/api/tags` 确认可用
2. **所有 curl 必带超时** — 探测 3s/5s，API 调用 5s/120s，禁止无超时裸调
3. **输入截断** — diff > 500 行时取前 500 行，并在 prompt 末尾加 `[TRUNCATED]`
4. **结果必校验** — 不直接信任本地模型输出，必须经过质量门禁
5. **失败不重试** — Ollama 调用失败一次即回退，不做无意义重试
6. **记录使用量** — 每次调用后输出：`[Ollama] 本次节省 ~N token | 耗时 Tms`

## Agent 集成工作流

### 集成 1：auto-committer — 本地生成 + Claude 校验

```
主 Agent 决策：需要提交代码
  │
  ├─ 1. Skill: ollama-call（模板 commit-msg）
  │     ├─ Ollama 可用 → 生成 commit 消息
  │     │   ├─ 质量门禁通过 → 消息存入变量 OLLAMA_MSG
  │     │   └─ 质量门禁失败 → OLLAMA_MSG 置空
  │     └─ Ollama 不可用 → OLLAMA_MSG 置空
  │
  └─ 2. Agent: auto-committer
        ├─ OLLAMA_MSG 非空 → 使用预生成消息，跳过 Claude 生成阶段
        │   └─ 仅执行：校验消息格式 → 过滤禁止提交文件 → git commit
        └─ OLLAMA_MSG 为空 → 完整 auto-committer 流程（Claude 生成）
```

**预期节省**：Ollama 可用时，auto-committer 仅消耗 ~1K token（校验 + git 操作），节省 ~2-4K token。

### 集成 2：reviewer — 本地第一遍 + Claude 复核

```
主 Agent 决策：需要代码审查
  │
  ├─ 1. Skill: ollama-call（模板 code-review）
  │     ├─ 返回 "NO_ISSUES" → 提示主 Agent 可跳过 reviewer（低风险变更）
  │     └─ 返回 N 条问题 → 存入变量 OLLAMA_ISSUES
  │
  └─ 2. Agent: reviewer（仅当 OLLAMA_ISSUES 非空或有高风险文件时）
        └─ 以 OLLAMA_ISSUES 为线索，reviewer 逐条验证 + 深度审查（rust-syntax-review + security-review 全覆盖）
```

**预期节省**：低风险变更直接跳过 reviewer 的深度审查，节省 ~10-20K token。

### 集成 3：reviewer（security-review）— 机械安全预审 + Claude 深度审查

```
主 Agent 决策：需要安全审查
  │
  ├─ 1. Skill: ollama-call（模板 security-scan）
  │     ├─ Ollama 可用 → 列出疑似安全违规项
  │     │   ├─ 返回 "CLEAN" → 跳过机械安全检查
  │     │   └─ 返回 N 条违规 → 存入变量 OLLAMA_SEC_FINDINGS
  │     └─ Ollama 不可用 → OLLAMA_SEC_FINDINGS 置空
  │
  └─ 2. Agent: reviewer（security-review skill）
        ├─ OLLAMA_SEC_FINDINGS 非空 → 逐条验证 RED_FLAG，WARNING 做参考
        │   ├─ 验证通过 → 纳入安全报告，标记 [Ollama]
        │   └─ 验证失败 → 过滤为误报
        └─ OLLAMA_SEC_FINDINGS 为空 → 完整安全维度审查
```

**预期节省**：Ollama 可用时，reviewer 跳过机械扫描，仅做语义验证，节省 ~8-12K token。

### 集成 4：doc-writer — 文档生成 + Claude 复审

```
主 Agent 决策：需要编写文档/注释
  │
  ├─ 判断文档类型：
  │   ├─ Rust 文档注释（单个方法/类）→ Skill: ollama-call（模板 docs）
  │   │   └─ doc-writer 仅抽查注释规范合规性
  │   ├─ README / 设计文档 → Skill: ollama-call（模板 docs）
  │   │   └─ doc-writer 复审技术事实 + 术语准确性
  │   ├─ 变更说明 / Changelog → Skill: ollama-call（模板 docs）
  │   │   └─ 人工确认即可
  │   └─ PR 描述 → Skill: ollama-call（模板 docs）
  │       └─ reviewer 或主 Agent 补充技术细节
  │
  └─ Ollama 不可用 → 回退到 Claude 直接编写
```

**预期节省**：Rust 文档注释节省 ~5-8K token/类，README 段落节省 ~10-15K token，变更说明节省 ~3-5K token。

### 主 Agent 判断流程

在调用 auto-committer / reviewer / doc-writer 之前，主 Agent 应：

1. **判断是否适合用 Ollama**：
   - ✅ 变更仅涉及增删文件、配置修改、简单重构 → 优先 Ollama
   - ✅ diff < 500 行 → 优先 Ollama
   - ✅ 文档/注释编写（Rust 文档注释、README、变更说明）→ 优先 Ollama
   - ✅ 机械性安全检查（unwrap/panic、命名违规、硬编码密钥）→ 优先 Ollama
   - ❌ 变更涉及 WAL 完整性 / CRC32 校验 / 确定性状态机 / 并发安全 / 架构分层判断 → 跳过 Ollama，直接 Claude
2. **执行 Ollama 预调用**：按上述模板调用
3. **传递结果给 Agent**：将 Ollama 输出作为 prompt 的一部分传给 Agent