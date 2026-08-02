---
name: rookeeper-architecture-reflection
description: Architecture review and design reflection for Rookeeper
version: 1.1.0
---

# Rookeeper Architecture Reflection

Use Claude Code to reflect on Rookeeper's design decisions, challenge assumptions, and propose concrete improvements. **This skill is for system-level design decisions. For code-level bugs, use `code-review.md`.**

## When to Invoke

When the user asks to:
- "反思设计" / "review the architecture"
- "这个设计有什么问题" / "what are the design problems"
- "方案评审" / "design review"
- "consider alternatives for" / "is this approach correct for"
- Any request to challenge, challenge, or improve Rookeeper's architecture

## Reflection Framework

### 1. Correctness Challenges
- Does the lock implementation handle all crash scenarios correctly?
- Can a session hold a lock but fail to clean up (panic mid-function)?
- Is the FIFO ordering truly guaranteed or only best-effort?
- What happens if Watch events are delivered out of order?

### 2. Scalability Challenges
- Watch manager iterates all sessions on every event: O(n) scan — acceptable for now, but when does it break?
- All sessions in-memory: session state lost on server crash? (Intentional per spec, but is recovery needed?)
- Lock metadata in-memory: lost on restart — acceptable for Phase 2?

### 3. Consistency Challenges
- TreeKv is single-node: no Raft/Paxos — acceptable for Phase 2's scope?
- Lock waiting uses polling (check every 100ms): causes thundering herd
- No lease extension API: long-running operations holding locks need explicit renewal

### 4. Protocol Challenges
- Request IDs: local to session or global?
- Event ordering: multiple sessions watching same path — in what order?
- Backpressure Drop-Newest: clients can miss events — acceptable for watch?

## Invocation Pattern

```bash
# For quick questions (simple opinions)
cd ~/Project/Rookeeper && claude -p \
  "<question>" \
  --allowedTools "Read,Bash" \
  --max-turns 10

# For formal design reviews (DDR output)
cd ~/Project/Rookeeper && claude -p \
  "Reflect on <module_or_decision>. Challenge the design: what breaks at scale,
   what edge cases are unhandled, what alternatives exist? Output in DDR format." \
  --allowedTools "Read,Edit,Bash" \
  --max-turns 20
```

## Output Format

**Simple questions** — free-form analysis is acceptable.

**Formal DDR format** (for design reviews):
```
## Design Decision: <what is being questioned>

| Field | Value |
|-------|-------|
| Current | <current approach> |
| Risk | <what could go wrong> |
| Alternative | <possible better approach> |
| Verdict | keep / conditional-accept / change / revisit / deprecate-over-time |

**Evidence:** <why this verdict>
**Action:** <if change needed, what specific change>
```

**Convergence rule:** If 3+ Critical risks are found, stop and present findings rather than continuing indefinitely.
