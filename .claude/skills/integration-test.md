---
name: rookeeper-integration-test
description: Integration test design and review for Rookeeper
version: 1.0.0
---

# Rookeeper Integration Test

Design and review integration tests for Rookeeper's end-to-end behavior across all modules.

## When to Invoke

When the user asks to:
- "write integration tests"
- "design e2e tests"
- "review the test coverage"
- Any request to create or audit `tests/` directory integration tests

## Coverage Checklist

### Phase 1 Core
- [ ] Get/Set/Create/Delete on existing and non-existing paths
- [ ] Version conflict on concurrent Set to same path
- [ ] List returns correct children, sorted order
- [ ] Watch fires NodeCreated/NodeUpdated/NodeDeleted events

### Phase 2 Primitives
- [ ] Lock: two processes compete for same lock → one wins, one waits
- [ ] Lock: holder exits → waiter acquires lock (event + observed behavior)
- [ ] Session: heartbeat extends lease; no heartbeat → expiration
- [ ] Ephemeral: session disconnect → ephemeral nodes auto-deleted
- [ ] Registry: register → discover → deregister → no longer discoverable
- [ ] Watch backpressure: slow consumer does not block event producer

### Non-Functional
- [ ] Crash during write → WAL replay recovers exact state
- [ ] Restart server → sessions do NOT survive (by design — ephemeral)
- [ ] Concurrent requests on same path → no data corruption

## Test Structure

```rust
// Use this structure for all integration tests:
#[tokio::test]
async fn integration_<scenario>(){
    // 1. Bootstrap: start in-process server on random port
    // 2. Connect: create client, establish session
    // 3. Arrange: prepare state or spawn competitor tasks
    // 4. Act: perform the operation
    // 5. Assert: verify observable outcomes (not internal state)
    // 6. Cleanup: drop client, stop server
}
```

## Invocation Pattern

```bash
cd ~/Project/Rookeeper && claude -p \
  "Design integration tests for <scenario>.
   Follow the test structure above.
   Use in-process server bootstrap.
   Verify observable behavior only (no internal state peeking).
   Output: complete runnable Rust test file." \
  --allowedTools "Read,Edit" \
  --max-turns 15
```
