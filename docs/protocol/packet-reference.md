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
| 6 | OPEN_PRESETS | OUT | 36 | `0x06` | `0x04` | Prepares preset resource for streaming |
| 7 | OPEN_STREAM | OUT | 40 | `0x07` | `0x0C` | Starts paged stream; response contains chunk #0 |
| 8+ | CHUNK_REQUEST | OUT | 16 | `0x08`+ | `0x08` | Dynamic: `seq` and `offset` increment per chunk |

All OUT packets are immediately followed by a bulk IN read.

### Chunk Request Fields

| Field | Start offset | Initial value | Increment |
|---|---|---|---|
| `seq` | byte 9 | `0x08` | `+1` (wraps at `0xFF`) |
| `offset` | bytes 12–15 | `0x00001138` (LE) | `+0x0100` per full chunk |

---

## Response Payload Layout

Responses in the preset stream have a fixed 16-byte header followed by payload data:

```
[0..15]   header (ignored)
[16..n]   payload data
```

A response where `n < 272` signals end of stream. See [presets/list.md](./presets/list.md#phase-4--end-of-stream-detection).
