# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository. It follows the **OpenSpec SDD** (Specification-Driven Development) principles.

## OpenSpec Core Principles

- **fluid not rigid** — no phase gates, work on-demand
- **iterative not waterfall** — iterate, learn, and adjust as you go
- **easy not complex** — lightweight, minimal ceremony
- **brownfield-first** — always work with the existing codebase, never against it

## Project Status

**Phase 1 — Tree KV, WAL, Recovery, ACL (~100% complete)**
- Tree KV（BTreeMap 内存树）
- WAL 预写日志（checksum 支持）
- 快照（generation 生成）
- 断电恢复（RecoveryManager）
- ACL 权限系统
- 24-byte 二进制协议
- 平台抽象（Linux UDS / Windows named pipe / TCP fallback）
- 基础 CLI
- 23 个测试全部通过

**Phase 0 — Baseline ✓ (已存档)**
- Crate 结构、协议、存储布局已完成

## Build, test, and lint

```powershell
cargo fmt --all --check        # Format check
cargo clippy --workspace --all-targets -- -D warnings  # Lint (gate)
cargo check --workspace        # Compilation check
cargo test --workspace         # All tests
```

Run a single crate or test:
```powershell
cargo test -p rookeeper-protocol
cargo test -p rookeeper-protocol model::tests::rejects_parent_segments
cargo test -p rookeeper-storage checksum_is_stable
cargo test -p rookeeper-client creates_headers_with_version
```

Bootstrap binaries:
```powershell
cargo run -p rookeeper-server -- --print-layout
cargo run -p rookeeper-cli -- status
cargo run -p rookeeper-cli -- normalize-path "\plant\\line1/robot3//config"
```

Toolchain: stable, rust-version 1.82, with clippy and rustfmt components (see `rust-toolchain.toml`).

## Architecture

`rookeeper` is a single-node coordination service for industrial-control environments. The workspace is split so future phases can layer behavior on top of already-stable types.

### Crate responsibilities

| Crate | Role |
| --- | --- |
| `rookeeper-protocol` | Shared source of truth: ACLs, path normalization, node/session models, config structs, error codes, binary frame types. Other crates import from here, not re-define. |
| `rookeeper-storage` | Storage layout conventions for `data/`, `wal/`, `snapshot/`, `state/`, plus WAL/snapshot filename generation and checksum helpers. |
| `rookeeper-platform` | Platform abstraction for OS detection and default IPC selection. Linux → Unix domain socket, Windows → named pipe, fallback → local TCP. |
| `rookeeper-client` | Client bootstrap that derives endpoints and request headers from shared config/protocol types. |
| `rookeeper-server` | Server bootstrap that turns `ServiceConfig` into a runtime summary and storage layout. |
| `rookeeper-cli` | Thin maintenance CLI over shared protocol/platform types. |

### Intended dependency direction

1. `rookeeper-protocol` defines shared models.
2. `rookeeper-storage` and `rookeeper-platform` build infrastructure around those models.
3. `rookeeper-client` and `rookeeper-server` consume those crates instead of redefining transport, path, or config logic.
4. Future state-machine, persistence, watcher, and session logic plug into these crates rather than bypassing them.

## Key cross-file design points

1. **Path handling is centralized in `rookeeper-protocol::model::NodePath`**. Paths accept both `\` and `/` on input, normalize internally to Unix-style `/...`, reject `..` parent segments and `.` references. Reuse `NodePath::parse` instead of hand-normalizing.

2. **Transport defaults are centralized in `rookeeper-platform`**. OS detection, IPC transport selection, and default endpoint construction live here. If endpoint selection changes, update platform helpers and client/server bootstrap together.

3. **Storage layout is centralized in `rookeeper-storage::StorageLayout`**. WAL/snapshot naming conventions come from this type. Layout:
   ```
   data/
     wal/wal-{segment_id:016}.log
     snapshot/snapshot-{generation:016}.bin
     state/cluster.meta
     rookeeper.lock
   ```

4. **Protocol frame details live in `rookeeper-protocol::wire`**. Request kinds, event kinds, header shape, and protocol version stay aligned with `docs/protocol-baseline.md`. Frame is a 24-byte fixed header: version (u16), request_kind (u16), flags (u16), reserved (u16), request_id (u32), session_id (u64), payload_len (u32).

## Design constraints

- **Keep platform differences in `rookeeper-platform`** only. Do not scatter `cfg!(target_os = ...)` through business code.
- **Use shared config/model types from `rookeeper-protocol`**. Server, client, CLI, and later persistence code depend on the same structs rather than redefining them.
- **State-machine behavior must be deterministic**. Architecture docs explicitly require avoiding external time/randomness as decision inputs.
- **Treat `docs/` as design constraints**. `architecture-baseline.md`, `protocol-baseline.md`, and `storage-layout.md` describe intended module boundaries and naming/layout rules — not background reading.
- **Phase 0 is intentionally thin**. `rookeeper-server::load_config` currently returns `ServiceConfig::default()` and rejects external config files, pointing callers to `config/rookeeper.default.toml`. Do not assume runtime config loading is implemented.
- **Workspace dependencies are pinned in root `Cargo.toml`**. Add shared dependencies to `[workspace.dependencies]` and reference with `.workspace = true` in member crates.

## Phase Boundaries

### Phase 1 — Tree KV, WAL, Recovery, ACL ✓
**Status: ~100% complete — 23 tests passing**

| Deliverable | Status | Notes |
| --- | --- | --- |
| Tree KV | ✓ Done | BTreeMap 内存树，支持 parent 路径 |
| WAL | ✓ Done | 预写日志，checksum 支持，分段写入 |
| 快照 | ✓ Done | generation 生成，定期压缩 |
| 断电恢复 | ✓ Done | RecoveryManager，支持增量恢复 |
| ACL 权限 | ✓ Done | 空 ACL = allow-all，SessionMeta 追踪 |
| 二进制协议 | ✓ Done | 24-byte 头，请求/响应帧 |
| 平台抽象 | ✓ Done | Linux UDS / Windows named pipe / TCP |
| 基础 CLI | ✓ Done | normalize-path, status, print-layout |

**Phase 1 边界**: Watch 持久订阅、分布式锁、服务发现不在当前范围。

### Phase 2 — 下一个迭代
**范围**: Watch 持久订阅 / 分布式锁 / 服务发现

### Phase 0 — Baseline ✓ (已存档)
**范围**: 项目初始化、crate 结构、协议定义、存储布局

## Working with Specs (Iterative + Fluid Approach)

1. **Start by reading `docs/`** — `architecture-baseline.md`, `protocol-baseline.md`, `storage-layout.md` define your constraints
2. **When uncertain, check the spec first** — don't guess, consult the existing design
3. **Iterate in small steps** — no big upfront design, evolve incrementally
4. **If spec is unclear, ask** — treat this document as the single source of truth

## Spec Files (Design Constraints)

| File | Role |
| --- | --- |
| `docs/architecture-baseline.md` | Module boundaries, crate responsibilities |
| `docs/protocol-baseline.md` | Wire protocol, frame format, request/event kinds |
| `docs/storage-layout.md` | Directory layout, WAL/snapshot naming conventions |

> **Note**: `docs/` files are **constraints**, not suggestions. Code must comply with them.