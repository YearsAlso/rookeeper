---
name: rookeeper-protocol-compliance
description: Protocol consistency check for Rookeeper wire types
version: 1.0.0
---

# Rookeeper Protocol Compliance

Verify that protocol types (`wire.rs`, `error.rs`) stay in sync with the protocol baseline doc and the CLAUDE.md protocol table. Prevent protocol drift.

## When to Invoke

When the user asks to:
- "check protocol" / "check the wire types"
- "add a new request type"
- "is the protocol in sync"
- Any modification to `rookeeper-protocol/src/wire.rs` or `error.rs`

## Checklist

### Request/Event Types
- [ ] New request ID is the next available number (check `wire.rs` for current max)
- [ ] New event ID is the next available number
- [ ] New variant documented with payload format table
- [ ] Server handler handles the new type (or returns `UnsupportedVersion`)
- [ ] `CLAUDE.md` protocol table updated

### Error Codes
- [ ] New error code is the next available number (current max: 16)
- [ ] New `from_u16` branch added for the new code
- [ ] Error code documented in `CLAUDE.md` error table

### Frame Format
- [ ] New frame header size matches `FRAME_HEADER_LEN` (24 bytes)
- [ ] Body length field correctly represents payload bytes
- [ ] CRC32 checksum computed over header + body

### Backward Compatibility
- [ ] New optional fields don't change existing field positions
- [ ] New enum variants don't affect existing variant discriminants

## Invocation Pattern

```bash
cd ~/Project/Rookeeper && claude -p \
  "Verify that the protocol in wire.rs and error.rs is in sync with CLAUDE.md.
   Check: request IDs, event IDs, error codes, frame format, from_u16 completeness.
   Report any drift found and suggest fixes." \
  --allowedTools "Read,Grep" \
  --max-turns 8
```

## Output Format

```
## Protocol Drift Report

### Requests
- ✅ / ❌ <N> RequestKind variants — last ID: <max>
- Drift: <description or "None">

### Events
- ✅ / ❌ <N> EventKind variants — last ID: <max>
- Drift: <description or "None">

### Error Codes
- ✅ / ❌ ErrorCode range <min>..=<max> — last ID: <max>
- Missing from_u16 branches: <list>
- Drift: <description or "None">

### Frame Format
- ✅ / ❌ Header length: <N> bytes (expected 24)
```
