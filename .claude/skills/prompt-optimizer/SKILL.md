---
name: prompt-optimizer
description: 提示词优化 — 调用本地 Ollama 分析并优化 prompt 结构，提升 LLM 回答质量
---

# Prompt Optimizer — 提示词优化

调用本地 Ollama（`ornith:9b`）分析并优化 prompt 结构，使其更清晰、结构化、易于 LLM 理解。

## 适用场景

| 场景 | 说明 | 推荐模式 |
|------|------|---------|
| 构建复杂 prompt | 多步任务、多约束的 prompt 需要结构化 | `--stdin` |
| prompt 质量不佳时 | Claude 回答偏离预期时优化 prompt | `--preflight` 先分析 |
| 团队分享 prompt | 优化后结构清晰，便于他人理解 | `--stdin` |
| 快速检查 | 看看当前 prompt 有什么问题 | `--preflight` |

## 调用方式

### 1. 预检分析（仅评分，不修改）

```bash
echo "你的 prompt" | python ~/.claude/scripts/prompt-optimizer.py --stdin --preflight
```

输出：质量评分 + 改进建议列表 + 统计数据

评分说明：
- **80-100**：A 级，结构良好
- **65-79**：B 级，有小问题
- **50-64**：C 级，建议优化
- **<50**：D 级，强烈建议优化

### 2. 完整优化（调 Ollama 做语义优化）

```bash
echo "你的 prompt" | python ~/.claude/scripts/prompt-optimizer.py --stdin
```

先做本地格式标准化，再调 Ollama 做语义优化（去冗余、重组结构、明确输出）。

### 3. 仅本地变换（不调 Ollama）

```bash
echo "你的 prompt" | python ~/.claude/scripts/prompt-optimizer.py --stdin --dry-run
```

仅做格式标准化（换行/缩进/BOM/行尾空格），不消耗 Ollama 算力。

### 4. 从文件读取

```bash
python ~/.claude/scripts/prompt-optimizer.py --input prompt.txt --preflight
python ~/.claude/scripts/prompt-optimizer.py --input prompt.txt
```

### 5. JSON 输出（供程序消费）

```bash
echo "你的 prompt" | python ~/.claude/scripts/prompt-optimizer.py --stdin --preflight --json
echo "你的 prompt" | python ~/.claude/scripts/prompt-optimizer.py --stdin --json
```

## 执行流程

```
用户输入 prompt
  |
  |-- Step 0: 检查 Ollama 可用性
  |     curl -s --connect-timeout 3 --max-time 5 http://localhost:11434/api/tags
  |     |- 200 -> 可用，执行完整优化
  |     |- 其他 -> 回退到 dry-run（仅本地变换）
  |
  |-- Step 1: 本地格式标准化
  |     CRLF->LF、去行尾空格、合并空行、去 BOM
  |
  |-- Step 2: 结构分析（--preflight 在此输出并退出）
  |     评分（0-100）+ 改进建议 + 统计数据
  |
  |-- Step 3: Ollama 语义优化（--dry-run 跳过此步）
  |     调 ornith:9b 做：添加角色定义、拆分结构、消除冗余、
  |     明确输出格式、补充约束
  |
  |-- Step 4: 输出优化结果
        优化后 prompt + 改动列表 + 字符数对比
```

## 质量门禁

| 检查项 | 门禁规则 |
|--------|---------|
| Ollama 可用性 | 不可用回退到 dry-run，不终止 |
| JSON 解析 | 无法解析时返回原始 Ollama 输出文本 |
| 输入长度 | 超过 12000 字符截断 |
| 模型超时 | 120s 超时，超时回退到 dry-run |

## 限制与回退

### 硬限制
- **Ollama 调用超时**：120s（首次加载模型可能较慢）
- **输入截断**：超过 12000 字符时截断
- **探测超时**：5s（`--connect-timeout 3 --max-time 5`）

### 回退条件
- Ollama 不可用 -> dry-run（仅本地变换）
- JSON 解析失败 -> 返回原始 Ollama 输出文本
- 模型返回空内容 -> dry-run

### 不可替代的场景
以下场景建议跳过优化直接发送：
- prompt 已经很短（<20 字符），优化收益不大
- prompt 包含高度专业化的格式要求（优化可能引入偏差）
- 对实时性要求高，不能等待 10-60s 的优化耗时

## Agent 集成

### 集成 1：slash command 手动调用

用户在对话中输入 `/prompt-optimizer` 后跟上 prompt 内容，即可触发优化流程。

```
用户：/prompt-optimizer 请帮我写一个 Rust 程序...
  |
  |- 步骤 1：echo "请帮我写..." | python prompt-optimizer.py --stdin --preflight
  |    输出质量评分 + 改进建议
  |
  |- 步骤 2（可选）：echo "请帮我写..." | python prompt-optimizer.py --stdin
  |    输出优化后 prompt + 改动列表
  |
  |- 步骤 3：展示优化前后对比，供用户确认
```

### 集成 2：主 Agent 自动调用

当主 Agent 检测到用户要发送复杂 prompt 时，可以自动调用：

```
主 Agent 决策：用户输入了复杂请求
  |
  |- 1. Skill: prompt-optimizer（preflight 模式）
  |    评分 >= 60 -> 直接发送，不做优化
  |    评分 < 60  -> 执行完整优化
  |
  |- 2. Skill: prompt-optimizer（完整优化）
  |    将优化后 prompt 用于后续任务
```

### 管道模式

```bash
cat prompt.txt | python ~/.claude/scripts/prompt-optimizer.py --stdin
```

## 工作约束

- 不改写用户的业务需求，只优化结构表达
- 如果 prompt 质量评分 >= 80，可跳过完整优化
- 优化结果需先展示给用户确认，不擅自替换用户的原始 prompt
- Ollama 不可用时回退到预检分析 + 本地格式标准化（dry-run）
