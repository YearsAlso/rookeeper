# rookeeper Phase 0 存储布局

## 1. 数据根目录

默认数据根目录由配置项 `storage.root_dir` 指定，建议本地部署时使用独立目录。

## 2. 目录结构

```text
data/
  wal/
    wal-0000000000000001.log
  snapshot/
    snapshot-0000000000000001.bin
  state/
    cluster.meta
  rookeeper.lock
```

## 3. 各目录职责

| 路径 | 说明 |
| --- | --- |
| `wal/` | 命令日志目录，按段滚动 |
| `snapshot/` | 快照目录，按代数命名 |
| `state/` | 本地元数据目录，后续可放恢复游标与兼容性信息 |
| `rookeeper.lock` | 单实例保护锁文件 |

## 4. 命名规则

1. WAL 段采用 `wal-{segment_id:016}.log`。
1. 快照文件采用 `snapshot-{generation:016}.bin`。
1. 所有序号均使用十进制零填充，便于排序和跨平台排查。

## 5. 恢复顺序

1. 获取 `rookeeper.lock`，确保单实例恢复。
1. 读取最新快照。
1. 从快照之后的 WAL 段继续顺序重放。
1. 状态机恢复完成后再开放服务请求。

## 6. Phase 0 边界

Phase 0 只固化布局与命名，不实现完整的 WAL/快照写入逻辑；后续 Phase 1 将在此基础上补持久化实现。
