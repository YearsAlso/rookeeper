---
name: rookeeper-code-review
description: Code audit and review for Rookeeper distributed coordination service
version: 1.1.0
---

# Rookeeper Code Review

Use Claude Code to perform thorough code audits on Rookeeper codebase.

## Codebase Overview

```
rookeeper/
├── crates/
│   ├── rookeeper-server/src/
│   │   ├── lock.rs        # Distributed lock (ephemeral+sequential nodes)
│   │   ├── session.rs     # Session lease & ephemeral binding
│   │   ├── registry.rs   # Service registry & discovery
│   │   ├── backpressure.rs# Watch delivery backpressure (Drop-Newest)
│   │   ├── tree_kv.rs    # In-memory tree + WAL + Snapshot
│   │   ├── watch.rs      # Subscription manager + broadcast
│   │   └── server.rs     # Server main loop
│   ├── rookeeper-protocol/src/
│   │   ├── wire.rs       # RequestKind, EventKind, FrameHeader
│   │   ├── error.rs      # ErrorCode (Phase 1 + Phase 2)
│   │   └── model.rs      # NodePath, SessionId, NodeKind
│   └── rookeeper-client/src/
│       └── sdk.rs        # Async Rust SDK
└── docs/
    └── protocol-baseline.md
```

## When to Invoke

When the user asks to:
- "review the code" / "代码审查" / "审计"
- "check for bugs" / "look for issues"
- "audit the code"
- Any request involving reviewing, checking, or auditing Rookeeper code

## Review Focus Areas

### 1. Correctness
- [ ] Logic errors in lock acquisition/release (FIFO, deadlock potential) → `lock.rs`
- [ ] Session cleanup ordering (lock waiters before sequence check) → `lock.rs::cleanup_session`
- [ ] TreeKv version conflict handling → `tree_kv.rs`
- [ ] Watch event delivery and backpressure correctness → `backpressure.rs`, `watch.rs`
- [ ] Ephemeral node cleanup on session expiry → `session.rs`, `tree_kv.rs`

### 2. Concurrency & Safety
- [ ] `RwLock`/`Mutex`: no held locks across `.await` points → all `*.rs`
- [ ] `Arc` cloning: cloned for async tasks, not shared across sync boundaries → `lock.rs`, `backpressure.rs`
- [ ] `spawn`ed tasks: dropped/cancelled properly on session close → `session.rs`
- [ ] `broadcast` channel: slow consumers don't cause memory buildup → `watch.rs`

### 3. Protocol Compliance
- [ ] Frame header size matches `FRAME_HEADER_LEN` → `wire.rs`, `sdk.rs`
- [ ] Phase 2 request types (13-20, see `wire.rs`) handled in server handler → `server.rs`
- [ ] Phase 2 event types (9-15, see `wire.rs`) propagated correctly → `server.rs`, `watch.rs`
- [ ] `from_u16` covers all Phase 1 + Phase 2 error codes → `error.rs`

### 4. Error Handling
- [ ] All `Result` types handled or propagated with meaningful context
- [ ] No `unwrap()` on paths that could fail (user input, I/O)
- [ ] Error messages include path/key context for debugging

### 5. Performance
- [ ] O(n) scans in hot paths → `watch.rs::matching_subscriptions`
- [ ] Lock contention: heavy RwLock under load
- [ ] Memory: unbounded channel sizes

### 6. Security
- [ ] No secrets in logs
- [ ] Path traversal: `..` validated? → `model.rs`
- [ ] ACL enforcement on mutating operations → `acl.rs`

## Invocation Pattern

```bash
cd ~/Project/Rookeeper && claude -p \
  "Review <file_or_module> for bugs, race conditions, and protocol compliance.
   Focus on: <top 3 issues from the checklist above>.
   List each issue with file:line and severity (Critical/High/Medium/Low).
   End with: Reviewed: N files | Critical: N | High: N | Medium: N | Low: N" \
  --allowedTools "Read,Edit,Bash,Grep,Glob" \
  --max-turns 15
```

## Output Format

```
## Issues Found

[CRITICAL] <file>:<line>
  <description>
  → <recommendation>

[HIGH] <file>:<line>
  ...

## Summary
Reviewed: <N> files
Critical: N | High: N | Medium: N | Low: N
```
