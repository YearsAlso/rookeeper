---
name: rookeeper-async-correctness
description: Async/await correctness audit for Rookeeper
version: 1.0.0
---

# Rookeeper Async Correctness

Audit async code in Rookeeper for correctness issues: deadlocks, held-locks-across-await, task cancellation leaks, and unfair select branches.

## When to Invoke

When the user asks to:
- "check async code" / "异步代码审查"
- "review the await points" / "检查 await"
- "is there a deadlock risk"
- Any request modifying `tokio::spawn`, `Mutex`, `RwLock`, `select!`, `join!`, or any `.await`

## Focus Areas

### 1. Held-Locks-Across-Await (CRITICAL — project hard rule)
- No `RwLock`/`Mutex` held across any `.await` point
- Pattern to find: `RwLock::read().await` followed by `.await` on different line before `drop()`
- Common error: acquiring read lock → calling `.await` → releasing after another await

### 2. Task Cancellation Leaks
- `spawn`ed tasks must be dropped/cancelled when their session closes
- `tokio::spawn` inside `session.rs::cleanup_session` — verify proper abort or cancellation
- Background tasks started in `Watch::subscribe` — verify they hold a weak reference to session

### 3. Deadlock Risk
- Multi-lock acquisition: check lock order consistency (always acquire A before B if both needed)
- `select!` with branches that can both be ready simultaneously — fairness issue

### 4. Arc/RwLock Cloning
- `Arc<RwLock<T>>` must be cloned for each `spawn` task, not shared directly
- Check that `Arc::clone(&arc)` is used for task spawning, not `arc.clone()` (which is a deep clone for Arc)

### 5. Channel Backpressure
- `broadcast::channel(capacity)` — is capacity bounded? Is overflow handled?
- `mpsc::channel(capacity)` — slow receiver can cause sender to wait; is this intentional?

## Invocation Pattern

```bash
cd ~/Project/Rookeeper && claude -p \
  "Audit <file_or_module> for async/await correctness.
   Focus on: held-locks-across-await, task cancellation leaks, deadlock risk.
   List each issue with file:line, severity, and fix recommendation." \
  --allowedTools "Read,Grep,Glob" \
  --max-turns 10
```

## Output Format

```
[CRITICAL] <file>:<line> — Held lock across await
  ...
  → Fix: <concrete suggestion>

[HIGH] <file>:<line> — Unchecked task spawn cancellation
  ...
  → Fix: <concrete suggestion>
```
