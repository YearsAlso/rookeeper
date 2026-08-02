---
name: db-migrate
description: 数据库迁移脚本创建 — 创建新的迁移脚本模板，与 docs/ 存储布局文档比对一致性，验证幂等性和回滚方案
---

# /db-migrate — 创建数据库迁移脚本

需要修改数据库结构时使用。创建新的迁移脚本、比对文档一致性、验证幂等性。

## 工作流

### Step 1: 确定变更内容

明确本次迁移要做什么：

- [ ] 新增表
- [ ] 新增字段
- [ ] 修改字段类型/长度
- [ ] 新增索引
- [ ] 新增枚举类型
- [ ] 种子数据

### Step 2: 比对 docs/ 文档

打开 `docs/storage-layout.md` 和相关数据库设计文档，确认变更与文档一致：

| 检查项 | 说明 |
|--------|------|
| 表名 | 是否按文档命名规范 |
| 字段 | 类型、长度、默认值、NOT NULL 是否一致 |
| 索引 | 索引名、字段、类型是否一致 |
| 约束 | FK、UK、CHECK 是否实现 |
| 枚举 | 枚举值是否完整 |

如果文档与需求不符，先更新文档再创建迁移。

### Step 3: 创建迁移脚本

```bash
# 确定下一个序号（取现有最大序号前缀 +1；不数文件个数，避免重复前缀）
NEXT=$(ls scripts/migrations/*.sql | sed 's/.*\///; s/_.*//' | sort -n | tail -1)
NEXT=$((NEXT + 1))
DESC="简短描述"
touch "scripts/migrations/$(printf '%03d' $NEXT)_${DESC}.sql"
```

**模板**（幂等 + 可回滚，`-- Down` 为同文件注释块）：

```sql
-- Down: 撤销本脚本变更（DROP TABLE IF EXISTS ... / ALTER TABLE ... DROP COLUMN ...）

BEGIN;

-- 枚举：EXCEPTION 保护
DO $$ BEGIN
  CREATE TYPE example_status AS ENUM ('draft', 'final');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

-- 表：IF NOT EXISTS
CREATE TABLE IF NOT EXISTS example (
  id BIGSERIAL PRIMARY KEY,
  status example_status NOT NULL DEFAULT 'draft',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 索引：IF NOT EXISTS
CREATE INDEX IF NOT EXISTS idx_example_status ON example(status);

COMMIT;
```

**强制规范（幂等性）**：
- `CREATE TYPE` 用 `DO $$ BEGIN ... EXCEPTION WHEN duplicate_object THEN null; END $$;`
- `CREATE TABLE` 必须包含 `IF NOT EXISTS`
- `CREATE INDEX` 必须包含 `IF NOT EXISTS`
- `ADD COLUMN` 必须用 `DO $$ ... IF NOT EXISTS (information_schema) ... END $$;` 包裹
- `DROP COLUMN` 必须用 `DO $$ ... IF EXISTS (information_schema) ... END $$;`
- `ALTER COLUMN` 必须用 `DO $$ ... IF EXISTS (information_schema) ... END $$;`
- 索引命名：`idx_{表名}_{字段}`
- 必须包含 `-- Down` 回滚
- 必须包裹在 `BEGIN; ... COMMIT;` 事务块内

### Step 4: 验证迁移脚本（幂等性检查）

- [ ] 序号是否连续（不跳号、不重复）
- [ ] 是否幂等（重复执行不报错）— 按 Step 3 强制规范逐项核对
- [ ] 同文件是否包含 `-- Down` 回滚注释块
- [ ] 是否与 docs/ 文档一致

### Step 5: 输出报告

```
## 迁移脚本创建报告

### 新建脚本
- 文件名：003_xxx.sql
- 变更内容：...

### docs/ 一致性
- ✅ 表结构一致
- ✅ 索引完整
- ✅ 约束完整

### 回滚方案
```sql
DROP TABLE IF EXISTS ...;
```

### 风险评估
- 🔴 破坏性变更：无
- 🟡 数据迁移风险：无
- ✅ 向前兼容：是
```

## 工作约束

- 不修改已发布的迁移脚本（通过新脚本修正）
- 回滚写在同文件 `-- Down` 注释块中且可逆
- 生产环境迁移必须经过安全审查
- 序号递增不跳号、不重复