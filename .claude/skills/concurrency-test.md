---
name: rookeeper-concurrency-test
description: Concurrency test generation for Rookeeper primitives
version: 1.0.0
---

# Rookeeper Concurrency Test Design

Generate and review concurrency tests for Rookeeper's distributed primitives: locks, sessions, watches, and service registry.

## When to Invoke

When the user asks to:
- "write concurrency tests"
- "test the lock fairness"
- "generate test cases"
- Any request to write or review tests for `lock.rs`, `session.rs`, `registry.rs`, or `watch.rs`

## Required Test Scenarios

### lock.rs
- [ ] Two sessions acquire same lock → only one succeeds, other becomes waiter
- [ ] Lock holder disconnects → lock auto-released, next waiter gets it (FIFO verified)
- [ ] Three sessions request same lock simultaneously → sequence number determines winner
- [ ] Non-blocking acquire on held lock → returns LockBusy immediately
- [ ] Timeout lock → returns LockTimeout after deadline

### session.rs
- [ ] Session heartbeat extends lease
- [ ] Session without heartbeat expires → ephemeral nodes deleted
- [ ] Concurrent heartbeats from same session don't corrupt lease
- [ ] Session close triggers full cleanup of associated locks and ephemeral nodes

### registry.rs
- [ ] Register service → can be listed
- [ ] Same instance_id re-registered → update, not duplicate
- [ ] Deregister non-existent → no error (idempotent)
- [ ] Service offline → ttl-based expiration or explicit deregister

### watch.rs / backpressure.rs
- [ ] Two sessions watch same path → both receive events
- [ ] Session closes → receives WatchExpired, no further events
- [ ] Slow consumer → Drop-Newest behavior: newer events overwrite older
- [ ] Event delivery doesn't block the main watch dispatch loop
- [ ] Concurrent watch on overlapping paths → correct pattern matching

## Invocation Pattern

```bash
cd ~/Project/Rookeeper && claude -p \
  "Generate concurrency test cases for <module>.
   Cover all scenarios in the checklist above.
   Use tokio::spawn for parallel sessions, assert exact outcomes.
   Output: runnable Rust test code." \
  --allowedTools "Read,Edit" \
  --max-turns 15
```

## Output Format

```rust
#[tokio::test]
async fn <test_name>(){
    // Setup: create test storage + server
    // Arrange: spawn N sessions
    // Act: trigger the scenario
    // Assert: verify exact outcomes
}
```
