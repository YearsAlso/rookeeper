---
name: rookeeper-storage-consistency
description: Storage consistency and crash recovery check for Rookeeper
version: 1.0.0
---

# Rookeeper Storage Consistency

Audit WAL, snapshot, TreeKv, and recovery logic for consistency and correctness under crash scenarios.

## When to Invoke

When the user asks to:
- "check storage consistency" / "检查存储一致性"
- "audit the WAL" / "review recovery"
- "what happens if crash here"
- Any modification to `rookeeper-storage/` or `tree_kv.rs`

## Focus Areas

### WAL (write-ahead log)
- [ ] WAL entry written before state machine commit (not after)
- [ ] Flush to disk before acknowledging write to client
- [ ] WAL segment rotation: old segments can be safely truncated after snapshot

### Snapshot
- [ ] Snapshot taken at consistent point (no partial transactions)
- [ ] Snapshot checksum validates integrity on load
- [ ] WAL replay from snapshot + subsequent entries reconstructs exact state

### TreeKv
- [ ] Version conflict on concurrent writes → one wins, other returns `VersionConflict`
- [ ] Ephemeral node cleanup: when session expires, node deleted within grace period
- [ ] Path normalization: `..` and `.` in paths handled correctly

### Recovery
- [ ] On startup, replay WAL from last valid snapshot
- [ ] Corrupted WAL entry → skip and log error, don't crash
- [ ] Crash during WAL write → last entry is either complete or absent (no partial)
- [ ] Recovery time < 500ms target (verify algorithm is O(snapshot_size), not O(WAL_size))

## Invocation Pattern

```bash
cd ~/Project/Rookeeper && claude -p \
  "Audit the storage layer for consistency and crash-recovery issues.
   Focus on: WAL ordering, snapshot integrity, TreeKv version conflicts, recovery completeness.
   For each issue: file:line, severity, and concrete fix." \
  --allowedTools "Read,Grep" \
  --max-turns 10
```

## Output Format

```
[CRITICAL] <file>:<line> — WAL ordering violation
  Crash at this point would leave disk state inconsistent.
  → Fix: <concrete suggestion>

[MEDIUM] <file>:<line> — Snapshot checksum missing
  Loaded snapshot cannot be verified.
  → Fix: <concrete suggestion>
```
