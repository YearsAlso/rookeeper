---
name: rookeeper-documentation
description: Documentation and comment generation for Rookeeper
version: 1.1.0
---

# Rookeeper Documentation

Use Claude Code to generate and improve documentation for the Rookeeper codebase.

## When to Invoke

When the user asks to:
- "写文档" / "write documentation"
- "add comments" / "添加注释"
- "生成 API 文档" / "document this module"
- "improve the docs" / "改进文档"
- Any documentation request

## Documentation Standards

### Language Rule
Triggered in Chinese → output Chinese comments + Chinese doc blocks.
Triggered in English → output English comments + English doc blocks.
Default: English (code is English, consistency matters).

### Rust Doc Comments

All public items must have doc comments:

```rust
/// Creates a new lock with the given name.
///
/// The lock is lazily created on first access. Multiple sessions
/// can wait on the same lock concurrently; they are served in FIFO order.
///
/// # Errors
/// Returns `ErrorCode::InvalidPath` if `lock_name` contains '/' or is empty.
///
/// # Example
/// ```ignore
/// let (path, seq) = lock_manager.acquire("my-lock", 1, "nonblocking").await?;
/// ```
pub async fn acquire(&self, lock_name: &str, ...) -> Result<...> { ... }
```

> **Note:** Use `//!` for module-level docs (inside the file), and `///` for item-level docs.

### Module-Level Documentation

Every `.rs` file must start with a module doc block:

```rust
//! <module_name>.rs
//!
//! <One-line description>.
//!
//! ## Design Principles
//! - <principle 1>
//! - <principle 2>
//!
//! ## Data Structures
//! - `LockMeta`: tracks lock holder and waiter queue
//! - `Waiter`: single session waiting on a lock
//!
//! ## Concurrency Model
//! - All state protected by `Arc<RwLock<HashMap<...>>>`
//! - No locks held across `.await` points
```

### Protocol Documentation

Each new request/event type must be documented in `wire.rs`:

```rust
/// Acquire a distributed lock (Phase 2).
///
/// Payload: `AcquireLockPayload` (JSON)
/// Response: `AcquireLockResponse` (JSON) or `ErrorCode`
///
/// | Field | Type | Description |
/// |-------|------|-------------|
/// | lock_name | string | Name of the lock to acquire |
/// | session_id | u64 | Session requesting the lock |
/// | mode | string | "nonblocking", "blocking", or "timeout:<secs>" |
AcquireLock,
```

### Design Decision Records (DDR)

Use `docs/ddr/` for significant architectural decisions:

```markdown
# DDR-001: Lock Implementation Choice
Date: 2026-06-16
Status: Accepted

## Context
We needed to implement distributed locks on top of TreeKv + Watch.

## Decision
Use ephemeral + sequential nodes. Lock holder = lowest sequence number holder.

## Consequences
- **Pro:** No additional consensus needed beyond TreeKv
- **Con:** Polling-based wait (inefficient vs. notify)
- **Con:** Single-node TreeKv limits distribution
```

## Invocation Pattern

```bash
# Add module-level doc comments to a file
cd ~/Project/Rookeeper && claude -p \
  "Add comprehensive Rust doc comments to <file>. Include: module overview,
   function docs with # Errors and # Example sections, struct field comments.
   Output language: <English or Chinese>" \
  --allowedTools "Read,Edit" \
  --max-turns 15

# Document the protocol additions
cd ~/Project/Rookeeper && claude -p \
  "Document the Phase 2 protocol additions in wire.rs as a protocol spec.
   Use ASCII tables for frame formats. Each variant: payload format, when triggered." \
  --allowedTools "Read,Edit" \
  --max-turns 15

# Add inline comments for concurrency model
cd ~/Project/Rookeeper && claude -p \
  "Add inline comments to <file> explaining the concurrency model,
   any non-obvious logic, and the reasoning behind key design choices." \
  --allowedTools "Read,Edit" \
  --max-turns 10
```

## File Checklist (apply to every `.rs` file)

For each `.rs` file in `rookeeper-server/src/` and `rookeeper-protocol/src/`:

**Module doc block:**
- [ ] Module name + one-line description
- [ ] Design Principles (2-3 bullet points)
- [ ] Data Structures (key structs with role)
- [ ] Concurrency Model (locks, channels, async patterns)

**Public functions:**
- [ ] `# Errors` section listing all failure modes
- [ ] `# Example` section with `ignore` tag
- [ ] Inline comments for non-obvious logic

**Structs/Enums:**
- [ ] Every field documented
- [ ] Important invariants noted

## Output Format

```
## Documentation Added
Files: <N>
- <file1>: <summary of changes>
- <file2>: <summary of changes>

## TODO (needs domain expert review)
- <item requiring manual verification>
```
