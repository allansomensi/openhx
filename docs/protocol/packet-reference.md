# Packet Reference

Master table of all known packets in the HX protocol. Packets are listed in the order they appear in a session.

---

## Session Initialization

> Documented in: [session-handshake.md](./session-handshake.md)

| Step | Name | Direction | Length | seq | cmd | Notes |
|---|---|---|---|---|---|---|
| 1 | HANDSHAKE | OUT | 20 | — | — | Magic=`0x0028` (not the standard `0x0018`) |
| 2 | SESSION_OPEN_1 | OUT | 28 | `0x02` | `0x04` | Opens first session resource |
| 3 | SESSION_CHUNK_1 | OUT | 16 | `0x03` | `0x08` | First session chunk request |
| 4 | SESSION_OPEN_2 | OUT | 36 | `0x04` | `0x04` | Opens second session resource |
| 5 | SESSION_CHUNK_2 | OUT | 16 | `0x05` | `0x08` | Second session chunk request |

All 5 packets are followed by a bulk IN read. Responses are discarded.

---

## Preset Listing

> Documented in: [presets/list.md](./presets/list.md)

| Step | Name | Direction | Length | seq | cmd | Notes |
|---|---|---|---|---|---|---|
| 6 | OPEN_PRESETS | OUT | 36 | `0x06` | `0x04` | Prepares preset resource for streaming; the reply carries the setlist table (see [presets/setlists.md](./presets/setlists.md)) |
| 7 | OPEN_STREAM | OUT | 40 | `0x07` | `0x0C` | Dynamic: byte 34 = setlist index. Starts paged stream; response contains chunk #0 |
| 8+ | CHUNK_REQUEST | OUT | 16 | `0x08`+ | `0x08` | Dynamic: `seq` and `offset` increment per chunk |

All OUT packets are immediately followed by a bulk IN read.

### Chunk Request Fields

| Field | Start offset | Initial value | Increment |
|---|---|---|---|
| `seq` | byte 9 | `0x08` | `+1` (wraps at `0xFF`) |
| `offset` | bytes 12–15 | `0x00001138` (LE) | `+0x0100` per full chunk |

---

## Preset Selection

> Documented in: [presets/select.md](./presets/select.md)

| Step | Name | Direction | Length | seq | cmd | Notes |
|---|---|---|---|---|---|---|
| 6 | SELECT_PRESET | OUT | 40 | `0x06` | `0x04` | Dynamic: byte 34 = setlist, byte 36 = preset slot. Replaces OPEN_PRESETS in a select session |

The ACK is followed by an unsolicited burst of notification packets that must be drained.

---

## MessagePack Command Envelope

Every command packet that carries a payload (`SESSION_OPEN_2`, `OPEN_PRESETS`, `OPEN_STREAM`, `SELECT_PRESET`) wraps a small MessagePack map with the same three keys:

| Key | Meaning | Observed values |
|---|---|---|
| `102` | Transaction ID | `1000`, `1001`, `1002`, `1010` — echoed by the device |
| `100` | Opcode | `254` (session open), `0` (open presets), `1` (start stream), `20` (select preset) |
| `101` | Arguments | `nil`, `{}`, or a map (`107` = setlist, `108` = preset slot, `101` = `2` for the preset stream) |

Replies use the same envelope shape with different keys:

| Key | Meaning |
|---|---|
| `102` | Transaction ID, echoed from the request |
| `103` | Status — `0` on success |
| `104` | Result value (`nil` for commands with no return value) |

The payload sits at bytes 24.. of the packet; bytes 20–23 carry its length, and the packet is padded to a 4-byte boundary.

---

## Response Payload Layout

Responses in the preset stream have a fixed 16-byte header followed by payload data:

```
[0..15]   header (ignored)
[16..n]   payload data
```

A response where `n < 272` signals end of stream. See [presets/list.md](./presets/list.md#phase-4--end-of-stream-detection).
