---
name: rookeeper-cross-platform
description: Cross-platform abstraction review for Rookeeper
version: 1.0.0
---

# Rookeeper Cross-Platform Review

Ensure platform-specific code is properly abstracted and both Linux and Windows behave identically at the API level.

## When to Invoke

When the user asks to:
- "check platform code"
- "review cross-platform"
- "does this work on Windows"
- Any modification to `rookeeper-platform/` or files with `#[cfg(...)]`

## Checklist

### Path Handling
- [ ] All internal paths normalized to Unix style (`/`) before storage
- [ ] No hardcoded `/` in path construction — use `PathBuf` or join helpers
- [ ] Config file paths respect platform conventions (`~/.config/rookeeper` on Linux, `%APPDATA%/rookeeper` on Windows)

### IPC Abstraction
- [ ] Linux uses UDS (Unix Domain Sockets)
- [ ] Windows uses Named Pipes
- [ ] Both exposed via same `PlatformTransport` trait in `rookeeper-platform`

### File Operations
- [ ] File locking uses platform abstraction (not `flock` directly)
- [ ] Directory creation uses recursive platform API
- [ ] No `fork()`, `syscall`, or other Unix-only primitives

### Behavioral Parity
- [ ] WAL fsync semantics equivalent on both platforms (no silent data loss)
- [ ] Ephemeral node cleanup timing consistent across platforms
- [ ] Watch event delivery order consistent across platforms

## Invocation Pattern

```bash
cd ~/Project/Rookeeper && claude -p \
  "Audit <file_or_module> for cross-platform correctness.
   Check: path handling, IPC abstraction, file operations, behavioral parity.
   Report any platform-specific leakage or inconsistency." \
  --allowedTools "Read,Grep" \
  --max-turns 8
```

## Output Format

```
## Platform Audit: <file>

### Issues Found
- ❌ <issue>: <platform-specific code or behavior>
  → Fix: <suggestion>

### Verified Safe
- ✅ <pattern>: <why it's safe>
```
