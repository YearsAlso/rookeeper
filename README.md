# Rookeeper

**A deterministic, lightweight coordination service for industrial single-node deployments.**

Rookeeper provides local tree-style KV storage, watches, locks, and service registration — built for Linux and Windows on PLCs, edge gateways, robotics controllers, and industrial PCs.

---

## What It Is

Rookeeper is a **local state-machine-driven coordination kernel** deployed on a single industrial device. It gives multiple control programs, driver modules, edge agents, and ops tools on the same machine a unified coordination plane.

### Core Capabilities

| Capability | Description |
|---|---|
| **Tree KV** | Path-structured nodes with versioning and ACLs |
| **WAL + Snapshots** | Deterministic replay and < 500ms recovery after power loss |
| **Local Locks** | Multi-process / multi-thread mutual exclusion |
| **Service Discovery** | Same-machine instance registration and health tracking |
| **Persistent Watches** | Path-level and prefix-based change notifications |
| **ACL & Audit** | Access control and replay-ready audit logs |

### Target Scenarios

- Robot control programs reading / writing process parameters
- Gateway collection and protocol translation processes discovering each other
- Industrial PCs recovering to last consistent state after power loss
- Cross-platform Linux / Windows with a single SDK

### V1.0 Out of Scope

- Multi-node deployment, consensus protocols (Raft, Zab)
- Cross-host service discovery
- Forced transport encryption / mTLS
- Web consoles
- Complex SQL or full-text search

---

## Project Status

| Phase | Status | Description |
|---|---|---|
| **Phase 0** | ✅ Complete | Workspace scaffolding, protocol drafts, platform abstraction, CI baseline |
| **Phase 1** | ✅ Complete | Tree KV, WAL, snapshots, recovery, ACL checking, basic CLI |
| **Phase 2** | 🔜 Planned | Watches, local locks, service discovery |
| **Phase 3** | 📋 Future | Full IPC (UDS / Named Pipe), logging, metrics |

---

## Workspace Structure

```
rookeeper/
├── crates/
│   ├── rookeeper-protocol/   # Shared protocol, data models, ACL, error codes
│   ├── rookeeper-storage/    # Storage layout, WAL/snapshot paths and utilities
│   ├── rookeeper-platform/   # Linux / Windows platform abstraction
│   ├── rookeeper-client/     # Client SDK scaffold
│   ├── rookeeper-server/     # Server entry point and config loading
│   └── rookeeper-cli/        # CLI for local operations and devtools
├── config/
│   └── rookeeper.default.toml
├── docs/
│   ├── architecture-baseline.md   # Module responsibilities and design principles
│   ├── protocol-baseline.md       # Binary protocol header and request types
│   ├── storage-layout.md          # WAL and snapshot naming conventions
│   └── industrial-single-node-coordination-service-v1.0-prd.md  # Full PRD
├── README.md                # This file
├── README.zh-CN.md          # 中文版
└── Cargo.toml
```

---

## Quick Start

```bash
# Format, build, and test
cargo fmt --all
cargo build --workspace
cargo test --workspace

# Print default storage layout
cargo run -p rookeeper-server -- --print-layout

# Run the CLI
cargo run -p rookeeper-cli -- status
```

---

## Multi-Language Support

This README is available in:

- [English](README.md) — current page
- [简体中文](README.zh-CN.md) — 当前页面

---

## Design Principles

1. **Deterministic state machine** — business results are determined solely by command input; no randomness or wall-clock timing in the critical path.
2. **Platform differences sunk** — platform-specific logic lives only in `rookeeper-platform` and deployment assets.
3. **Protocol first** — request types, error codes, and config fields are stabilized in Phase 0 before any implementation begins.
4. **Lightweight by default** — the baseline introduces only what the first version actually needs.

---

## Protocol & Architecture

See [docs/](docs/) for detailed specifications:

- [Architecture Baseline](docs/architecture-baseline.md) — module responsibilities and 5-layer logic structure
- [Protocol Baseline](docs/protocol-baseline.md) — 24-byte binary header and request types
- [Storage Layout](docs/storage-layout.md) — WAL and snapshot file naming conventions
- [PRD](docs/industrial-single-node-coordination-service-v1.0-prd.md) — full product requirements document