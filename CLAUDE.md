# Rookeeper — Distributed Coordination Service

## Project Overview

Rookeeper is a distributed coordination service (similar to etcd/ZooKeeper) built in Rust. Currently in **Phase 2** development.

**Repository:** `~/Project/Rookeeper/`

## Document Conventions

| File | Role |
|------|------|
| `docs/` | Project documentation library (specs, ADRs, protocol docs) |
| `TODO.md` | Task list and progress tracker |
| `openspec/` | OpenSpec spec-driven development root |

## OpenSpec / SDD Requirements

Rookeeper follows **OpenSpec Spec-Driven Development (SDD)** for all features and changes.

### Core Principles
```
fluid not rigid         — 无阶段门，按需工作
iterative not waterfall — 迭代式，边做边学
easy not complex       — 轻量级，最小仪式感
brownfield-first       — 兼容现有代码库
```

### OpenSpec Directory Structure
```
openspec/
├── specs/                     # 规范源头（系统当前行为）
│   └── <domain>/
│       └── spec.md
├── changes/                   # 变更提案
│   └── <change-name>/
│       ├── proposal.md        # 为什么做 + 做什么
│       ├── design.md          # 怎么做（技术方案）
│       ├── tasks.md           # 实现清单
│       └── specs/             # Delta spec
└── config.yaml
```

### Workflow
Any feature or change MUST go through the OpenSpec workflow:
```
/opsx:propose → /opsx:apply → /opsx:sync → /opsx:archive
```
1. **propose**: 创建 `changes/<name>/proposal.md` + `design.md` + `tasks.md`
2. **apply**: 编码实现，遵循 spec
3. **sync**: 将变更同步到 `specs/<domain>/spec.md`
4. **archive**: 归档已完成变更

### Spec Format
- Requirements: 使用 RFC 2119 关键词 (`SHALL`/`MUST`/`SHOULD`/`MAY`)
- Scenarios: 使用 `GIVEN/WHEN/THEN` 格式
- 每个新模块/功能必须先有 spec，再实现

## Architecture

```
rookeeper-protocol/   # Wire protocol types, error codes, frame formats
rookeeper-platform/   # OS abstraction, config defaults
rookeeper-storage/    # TreeKv + WAL + Snapshot persistence layer
rookeeper-server/     # Server implementation (main crate)
rookeeper-client/     # Client SDK + CLI support
rookeeper-cli/        # Command-line tool
```

## Current Phase: Phase 2 (Implementation in Progress)

Phase 2 adds distributed coordination primitives:
- **p2-1** ✅ `lock.rs` — Distributed lock (ephemeral + sequential nodes, FIFO)
- **p2-2** ✅ `session.rs` — Session management (lease, heartbeat, ephemeral binding)
- **p2-3** ✅ `registry.rs` — Service registry & discovery
- **p2-4** ✅ `backpressure.rs` — Watch event delivery backpressure (Drop-Newest)
- **p2-5** ✅ `sdk.rs` — High-level async Rust SDK
- **p2-6** ✅ CLI — Session/lock/service/watch subcommands
- **p2-8** 🔄 Integration tests

## Key Design Decisions

### Lock Design
- Lock nodes stored at `/locks/{lock_name}/{session_id}-{sequence}`
- Sequence number provides FIFO ordering (lowest sequence = lock holder)
- No separate coordination protocol — uses TreeKv + Watch only
- Session disconnect → `cleanup_session()` auto-releases held locks

### Watch Design
- Phase 1: `broadcast` channel for event dispatch
- Phase 2: Per-session bounded `mpsc` queue with Drop-Newest backpressure
- Pattern matching: exact path or glob (`*` and `**`)

### Storage Design
- TreeKv: in-memory tree with path hierarchy
- WAL: append-only log for crash recovery
- Snapshot: periodic full-state snapshots

## Phase 1 Protocol Types

| Request (u8) | Event (u8) | Error (u8) |
|---|---|---|
| 1: Get | 1: NodeCreated | 1: PathNotFound |
| 2: Set | 2: NodeUpdated | 2: PathAlreadyExists |
| 3: Create | 3: NodeDeleted | 3: VersionConflict |
| 4: Delete | 4: ChildrenChanged | 4: PermissionDenied |
| 5: List | 5: NodeExpired | 5: InvalidPath |
| 6: Watch | | 6: SessionExpired |
| 7: SessionCreate | | 7: LockBusy |
| 8: SessionHeartbeat | | 8: ResourceExhausted |
| 9: SessionClose | | 9: StorageCorruption |
| 10: RegisterService | | 10: TransportUnavailable |
| 11: ListServices | | 11: UnsupportedVersion |
| 12: DeregisterService | | |

### Phase 2 Protocol Extensions

| Request (u8) | Event (u8) | Error (u8) |
|---|---|---|
| 13: CancelWatch | 9: WatchExpired | 12: LockTimeout |
| 14: AcquireLock | 10: LockAcquired | 13: SessionNotFound |
| 15: ReleaseLock | 11: LockReleased | 14: ServiceAlreadyRegistered |
| 16: CreateSession | 12: SessionExpired | 15: LockNotHeld |
| 17: Heartbeat | 13: HeartbeatAck | 16: WatchNotFound |
| 18: RegisterServiceV2 | 14: ServiceChanged | |
| 19: DeregisterServiceV2 | 15: LockWaitTimeout | |
| 20: ListServicesV2 | | |

## Key Commands

```bash
# Build
cargo build --workspace

# Run tests
cargo test --workspace --lib

# Test specific module
cargo test -p rookeeper-server --lib -- lock

# Build CLI
cargo build -p rookeeper-cli
```

## Code Standards

- All public items require Rust doc comments (`///`)
- Module doc block at top of every `.rs` file
- Error propagation with `ErrorCode` (no `unwrap()` on user/controlled paths)
- No locks held across `.await` points
- All async tasks must be properly cancelled on session close
