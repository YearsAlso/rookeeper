---
name: chain-verify
description: 校验和验证 — 验证数据完整性校验和，生成验证报告，比对与 docs/ 架构文档的一致性
---

# /chain-verify — 校验和完整性验证

需要验证数据完整性校验和或审计哈希链完整性时使用。

## 工作流

### Step 1: 确定验证范围

指定要验证的数据段或文件路径，或选择验证所有数据目录。

### Step 2: 执行验证

#### 2.1 通过代码验证
调用 `rookeeper_storage::checksum()` 验证数据完整性：
- 计算数据的 CRC32 校验和
- 与存储的校验和比对
- 返回校验结果（是否匹配）

#### 2.2 验证 WAL 条目
调用 `rookeeper_storage::prelude::WalWriter` 相关的验证方法：
- 检查 WAL 条目顺序
- 验证每个条目的校验和
- 检查 WAL 段文件命名连续性

### Step 3: 比对 docs/ 架构文档

打开 `docs/storage-layout.md`，比对：

| 检查项 | 文档描述 | 代码实现 | 是否一致 |
|--------|---------|---------|---------|
| 校验和算法 | CRC32 | `crc32fast::hash()` | ✅ |
| WAL 命名 | wal-{segment_id:016}.log | `wal_segment_path()` | ✅ |
| 快照命名 | snapshot-{generation:016}.bin | `snapshot_path()` | ✅ |
| 目录布局 | data/wal/, data/snapshot/, data/state/ | `StorageLayout::from_root()` | ✅ |

### Step 4: 输出报告

```
## 校验和验证报告

### 验证信息
- 验证时间：{timestamp}
- 验证方式：代码 / 离线

### 数据完整性
- ✅ 总数据段：N
- ✅ 校验通过：N
- ✅ 全部匹配 / ❌ 存在不匹配

### 各数据段详情
| 段 | 原始校验和 | 计算校验和 | 匹配 |
|---|-----------|-----------|------|
| segment_001 | abc... | abc... | ✅ |

### docs/ 一致性比对
- ✅ 存储布局一致
- ✅ 校验算法一致
- ✅ 命名规范一致

### 结论
- ✅ 数据完整，与设计一致 / ❌ 数据损坏，需修复
```