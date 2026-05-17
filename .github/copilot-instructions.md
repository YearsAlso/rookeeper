# Copilot instructions for `rookeeper`

## Build, test, and lint

This repository is a Rust workspace. Run commands from the repository root.

### Common commands

```powershell
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace
cargo check --workspace --target x86_64-unknown-linux-gnu
```

### Run a single crate or test

```powershell
cargo test -p rookeeper-protocol
cargo test -p rookeeper-protocol model::tests::rejects_parent_segments
cargo test -p rookeeper-storage checksum_is_stable
cargo test -p rookeeper-client creates_headers_with_version
```

### Bootstrap binaries

```powershell
cargo run -p rookeeper-server -- --print-layout
cargo run -p rookeeper-cli -- status
cargo run -p rookeeper-cli -- normalize-path "\plant\\line1/robot3//config"
```

## High-level architecture

`rookeeper` is a single-node coordination service for industrial-control environments. The current codebase is still in **Phase 0**, so the focus is on stable module boundaries and shared models rather than full KV / Watcher / lock behavior.

The workspace is intentionally split so future phases can layer behavior on top of already-stable types:

| Crate | Role |
| --- | --- |
| `rookeeper-protocol` | Shared source of truth for protocol-level concepts: ACLs, path normalization, node/session models, config structs, error codes, and binary frame types |
| `rookeeper-storage` | Storage layout conventions for `data/`, `wal/`, `snapshot/`, `state/`, plus WAL/snapshot filename generation and checksum helpers |
| `rookeeper-platform` | Platform abstraction for OS detection and default IPC selection; Linux defaults to Unix domain sockets, Windows defaults to named pipes, fallback is local TCP |
| `rookeeper-client` | Client bootstrap logic that derives endpoints and request headers from shared config/protocol types |
| `rookeeper-server` | Server bootstrap layer that turns `ServiceConfig` into a runtime summary and storage layout |
| `rookeeper-cli` | Thin maintenance CLI over shared protocol/platform types |

The intended dependency direction is:

1. `rookeeper-protocol` defines the shared models.
2. `rookeeper-storage` and `rookeeper-platform` build infrastructure around those models.
3. `rookeeper-client` and `rookeeper-server` consume those shared crates instead of redefining transport, path, or config logic.
4. Future state-machine, persistence, watcher, and session logic should plug into these crates instead of bypassing them.

Important cross-file design points:

1. **Path handling is centralized in `rookeeper-protocol::model::NodePath`**. Paths accept both `\` and `/` on input, but normalize internally to Unix-style `/...` paths. New code should reuse `NodePath::parse` instead of hand-normalizing paths.
2. **Transport defaults are centralized in `rookeeper-platform`**. If endpoint selection changes, update both platform helpers and the client/server bootstrap logic together.
3. **Storage layout is centralized in `rookeeper-storage::StorageLayout`**. WAL/snapshot naming conventions should come from this type, not ad-hoc formatting.
4. **Protocol envelope details live in `rookeeper-protocol::wire`**. Request kinds, event kinds, header shape, and protocol version should stay aligned with `docs/protocol-baseline.md`.

## Key conventions

1. **Keep platform differences in `rookeeper-platform`**. Do not scatter `cfg!(target_os = ...)` checks through client/server/business code unless there is no alternative.
2. **Use shared config/model types from `rookeeper-protocol`**. Server, client, CLI, and later persistence/state-machine code are expected to depend on the same structs rather than each crate defining its own versions.
3. **Preserve deterministic semantics in protocol/state-facing code**. The architecture docs explicitly require state-machine behavior to avoid external time/randomness as decision inputs.
4. **Treat docs in `docs/` as design constraints, not background reading**. `architecture-baseline.md`, `protocol-baseline.md`, and `storage-layout.md` describe the intended module boundaries and naming/layout rules that code should follow.
5. **Phase 0 is intentionally thin**. `rookeeper-server::load_config` currently rejects parsing external config files and points callers to `config/rookeeper.default.toml`; do not assume runtime config loading is implemented yet.
6. **Workspace dependencies are pinned in the root `Cargo.toml`**. Prefer adding shared dependencies to `[workspace.dependencies]` and referencing them with `.workspace = true` in member crates.
7. **Clippy is treated as a gate**. Code should pass `cargo clippy --workspace --all-targets -- -D warnings`, so avoid patterns that trigger default Clippy warnings if there is a straightforward idiomatic alternative.
