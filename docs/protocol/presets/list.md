# Listing Presets

This document describes how to enumerate all 128 preset names from the device.

> **Prerequisites:** Complete the [session handshake](../session-handshake.md) before executing any phase described here. The sequence numbers below assume a fresh session where `seq` starts at `0x06`.

---

## Operation Overview

```
[SESSION INIT complete — seq at 0x06]
         │
         ▼
[PHASE 1] Open Preset Resource   — 1 packet (seq=0x06)
         │
         ▼
[PHASE 2] Start Paged Stream     — 1 packet (seq=0x07) → first data chunk
         │
         ▼
[PHASE 3] Paged Chunk Loop       — N packets (seq=0x08+, until short read)
         │
         ▼
[PHASE 4] End of Stream          — reassemble buffer, parse MessagePack
```

---

## Phase 1 — Open Preset Resource

Instructs the device to prepare the preset resource for streaming (`cmd=0x04`, `seq=0x06`).

```
19 00 00 18 01 10 EF 03 00 06 00 04 1A 10 00 00
01 00 02 00 09 00 00 00 83 66 CD 03 E9 64 00 65
C0 00 00 00
```

**Length:** 36 bytes

Send this packet and perform one bulk read. **The response is not parsed.**

---

## Phase 2 — Start Paged Stream

Instructs the device to begin sending preset data in pages (`cmd=0x0C`, `seq=0x07`).

```
1D 00 00 18 01 10 EF 03 00 07 00 0C 38 10 00 00
01 00 02 00 0D 00 00 00 83 66 CD 03 EA 64 01 65
82 6B 00 65 02 00 00 00
```

**Length:** 40 bytes

### Response

The device responds with the **first data chunk**. Strip the **16-byte header**:

```
useful_data = response_bytes[16..n]   // where n = number of bytes read
```

Append `useful_data` to the reassembly buffer if `n > 16`.

---

## Phase 3 — Paged Chunk Loop

After the first chunk, continue requesting subsequent chunks until the stream is exhausted.

### Chunk Request Packet (dynamic)

Each request is a **16-byte packet** with two variable fields: a sequence number (`seq`) and a 4-byte little-endian stream offset (`offset`).

**Packet layout:**

| Byte(s) | Value | Description |
|---|---|---|
| 0 | `0x08` | Fixed |
| 1 | `0x00` | Fixed |
| 2 | `0x00` | Fixed |
| 3 | `0x18` | Fixed |
| 4 | `0x01` | Fixed |
| 5 | `0x10` | Fixed |
| 6 | `0xEF` | Fixed |
| 7 | `0x03` | Fixed |
| 8 | `0x00` | Fixed |
| 9 | `seq` | **Sequence number** (see below) |
| 10 | `0x00` | Fixed |
| 11 | `0x08` | Fixed |
| 12–15 | `offset[0..3]` | **4-byte LE stream offset** |

### Sequence Number

- Starts at `0x08` for the first chunk request after Phase 2.
- Increments by `1` (wraps `0xFF → 0x00`) with each subsequent request.

### Stream Offset

- Starts at `0x00001138`.
- Increments by `0x0100` (256) after each full chunk received.

### Assembling a Chunk Request

```rust
fn build_chunk_request(seq: u8, offset: u32) -> [u8; 16] {
    let [o0, o1, o2, o3] = offset.to_le_bytes();
    [0x08, 0x00, 0x00, 0x18, 0x01, 0x10, 0xEF, 0x03,
     0x00, seq,  0x00, 0x08, o0,   o1,   o2,   o3  ]
}
```

### Loop Pseudocode

```
seq    = 0x08
offset = 0x00001138

LOOP:
    packet = build_chunk_request(seq, offset)
    write_bulk(EP_OUT, packet)
    n = read_bulk(EP_IN, buf[512])

    if n > 16:
        raw_stream.append(buf[16..n])

    if n < 272:          // MAX_CHUNK_SIZE — end of stream
        STOP

    offset = offset + 0x0100
    seq    = (seq + 1) & 0xFF
```

---

## Phase 4 — End of Stream Detection

A chunk response is considered the **final chunk** when the number of bytes read is **less than 272** (`MAX_CHUNK_SIZE`). The device sends no explicit end-of-stream packet.

| Condition | Meaning |
|---|---|
| `n == 272` | Full chunk — more data follows |
| `n < 272` | Short (final) chunk — stream exhausted |
| `n <= 16` | Empty final packet — no payload bytes |

After the loop ends, the reassembly buffer contains the complete raw MessagePack stream. Parse it using [presets/data-format.md](./data-format.md).

---

## Full Sequence Diagram

```
Host                                       HX Stomp
 │                                              │
 │  [Phase 1 — Open Preset Resource]            │
 │──── OPEN_PRESETS (36 bytes) ───────────────► │
 │◄─── ack (ignored) ────────────────────────── │
 │                                              │
 │  [Phase 2 — Start Stream]                    │
 │──── OPEN_STREAM (40 bytes) ────────────────► │
 │◄─── response[16:n] → chunk #0 ────────────── │
 │                                              │
 │  [Phase 3 — Paginate]  seq=0x08              │
 │──── chunk_request(seq, offset=0x1138) ─────► │
 │◄─── response[16:n] → chunk #1 ────────────── │
 │     (repeat, incrementing seq and offset)    │
 │──── chunk_request(seq, offset) ────────────► │
 │◄─── response[16:n] (n < 272) → chunk #N ──── │
 │                                              │
 │  [Phase 4 — Parse]                           │
 │     locate DC 00 80 in raw_stream            │
 │     decode MessagePack array[128]            │
 │     extract key=109 from each preset map     │
 │     strip trailing \0 from each name         │
```
