# Selecting Presets

This document covers the protocol flows for **switching the active preset** on the device. There are two distinct flows depending on whether the change is initiated by the host software (via USB bulk endpoints) or physically on the device (via footswitches or MIDI IN).

> **Prerequisites:** Complete the [session handshake](../session-handshake.md) before executing any phase described here. The sequence numbers below assume a fresh session where `seq` starts at `0x06`.

---

## Index Conventions

| Reference point | Numbering | Example |
|---|---|---|
| Device display | 0-indexed | `00`, `01`, `02` … |
| HX Edit UI | 1-indexed | `01`, `02`, `03` … |
| Wire protocol | 0-indexed | `0x00`, `0x01`, `0x02` … |

**Always use 0-indexed values in the protocol.** HX Edit's 1-indexed display is purely cosmetic.

---

## Packet Layout

All packets share the same 16-byte header followed by an 8-byte sub-header and a MessagePack payload:

```
Offset   Size   Field
──────   ────   ─────
 0–1      2     Payload length, little-endian (total_bytes − 11)
 2–3      2     Magic: always 00 18
 4–7      4     Channel direction marker (see below)
 8–9      2     Sequence number, little-endian (increments per packet)
10–11     2     Command type: 00 04 = data, 00 0C = stream-open
12–15     4     Channel / resource ID
16–23     8     Sub-header (see below)
24–end    n     MessagePack payload + zero padding to align
```

### Channel Direction Marker (bytes 4–7)

| Direction | Value |
|---|---|
| Host → Device | `01 10 EF 03` |
| Device → Host | `EF 03 01 10` |

### Sub-header (bytes 16–23)

```
Offset   Value   Notes
──────   ─────   ─────
  0       01      Always 0x01
  1       00
  2       06      Resource operation type (0x06 = write/set)
  3       00
  4       09      Resource handle
  5–7     00 00 00
```

### MessagePack Payload Schema

Each command payload is a **3-entry MessagePack map** (`fixmap(3)`):

| Key (decimal) | Key (hex) | Meaning | Type |
|---|---|---|---|
| 102 | `0x66` | Opcode / transaction counter | `uint16` |
| 100 | `0x64` | Command type | `uint8` / `fixint` |
| 101 | `0x65` | Command arguments | varies |

**Known command type values:**

| Value | Meaning |
|---|---|
| `0x00` | List presets |
| `0x01` | Stream start (for list pagination) |
| `0x14` | **Change preset** (host-initiated) |
| `0x16` | **Fetch preset data** (after a preset change) |
| `0x17` | Sync/ack (device-initiated flow only) |

---

## Opcode Counter

The `0x66` key holds a `uint16` opcode counter that increments by 1 with each new command sent in a session. The counter starts at **1000 (`0x03E8`)** when `SESSION_OPEN_2` is sent (sequence number 4 of the session init), so:

| Session step | Sequence number | Opcode value |
|---|---|---|
| `SESSION_OPEN_2` | 4 | 1000 (`0x03E8`) |
| `OPEN_PRESETS` (list) | 6 | 1001 (`0x03E9`) |
| `OPEN_STREAM` (list) | 7 | 1002 (`0x03EA`) |
| `CHANGE_PRESET` (select) | 6 | 1001 (`0x03E9`) |
| `FETCH_PRESET_DATA` (select) | 7 | 1002 (`0x03EA`) |

> The opcode counter for "select" starts at 1001 because "select" skips the list commands and uses the same post-init sequence numbers.

---

## Host-Initiated Flow (CLI / GUI → Device)

This is the flow used when the host application requests a preset change programmatically.

```
Host                                       HX Stomp
 │                                              │
 │──── SESSION INIT (5 packets) ─────────────► │
 │◄─── acks (discarded) ─────────────────────── │
 │                                              │
 │──── CHANGE_PRESET ─────────────────────────► │
 │◄─── CHANGE_PRESET_ACK ────────────────────── │
 │                                              │
 │──── FETCH_PRESET_DATA ─────────────────────► │
 │◄─── data chunk #0 ────────────────────────── │
 │──── CHUNK_REQUEST ─────────────────────────► │
 │◄─── data chunk #1 ────────────────────────── │
 │    (repeat until chunk n < 272 bytes)        │
```

### Step 1 — CHANGE_PRESET (Host → Device, 40 bytes)

Selects bank `BB` and preset `PP` (both 0-indexed).

```
1D 00  00 18  01 10 EF 03  00 06  00 04  1A 10 00 00
01 00 06 00 09 00 00 00
83
   66 CD 03 E9      ; key=102, value=uint16(1001)
   64 14            ; key=100, value=20 (change preset command)
   65 82            ; key=101, value=fixmap(2)
      6B BB         ;   key=107, bank index (0-indexed)
      6C PP         ;   key=108, preset index (0-indexed)
00 00 00            ; zero padding
```

**Total:** 40 bytes. Length field = `0x1D`.

For a single-bank device such as the HX Stomp XL, always use `BB = 0x00`.

### Step 2 — CHANGE_PRESET_ACK (Device → Host, 60 bytes)

The device confirms the change and returns the preset metadata.

```
3D 00  00 18  EF 03 01 10  00 06  00 04  1A 10 00 00
00 00 06 00 2C 00 00 00
83
   66 CD 03 E9      ; opcode echo
   67 00            ; status: 0 = success
   68 86            ; key=104, value=fixmap(6) ← preset info map
      6B CD 00 BB   ;   key=107, bank (uint16)
      6C CD 00 PP   ;   key=108, preset index (uint16)
      6D [str]      ;   key=109, preset name (null-terminated string)
      [3 more keys] ;   additional fields (ignored)
```

**Key fields to extract:**

| MessagePack key | Decimal | Content |
|---|---|---|
| `0x6B` | 107 | Bank index (`uint16`) |
| `0x6C` | 108 | Preset index (`uint16`) |
| `0x6D` | 109 | Preset name (`str`, null-terminated) |

Strip the trailing `\0` from the name string.

### Step 3 — FETCH_PRESET_DATA (Host → Device, 36 bytes)

Requests the full preset data to complete synchronisation.

```
19 00  00 18  01 10 EF 03  00 07  00 0C  38 10 00 00
01 00 06 00 09 00 00 00
83
   66 CD 03 EA      ; key=102, value=uint16(1002)
   64 16            ; key=100, value=22 (fetch preset data)
   65 C0            ; key=101, nil
00 00 00
```

**Total:** 36 bytes. Length field = `0x19`. Command type = `00 0C` (stream-open).

### Step 4 — Preset Data Transfer (Device → Host, multiple chunks)

The device streams the full binary preset data identically to the preset list stream:

- Each chunk is up to **272 bytes**.
- After receiving each full-size chunk (`n == 272`), send a pagination request (same `build_pagination_request(seq, offset)` as the list protocol).
- When `n < 272`, the stream has ended — no more pagination requests are sent.
- Initial `seq = 0x08`, initial `offset = 0x00001138`, `offset` increments by `0x0100`.

This data may be discarded if only confirmation of the change is needed (e.g., the CLI only uses the ACK from Step 2).

---

## Device-Initiated Flow (Pedal footswitch → Host)

When the user switches preset on the physical device, the device proactively notifies the host. This flow is relevant for a persistent monitor process (GUI) that must keep its state in sync.

```
Host                                       HX Stomp
 │                                              │
 │    [user presses footswitch on device]       │
 │◄─── PRESET_CHANGED_NOTIFICATION ──────────── │
 │                                              │
 │──── SYNC_ACK ──────────────────────────────► │
 │◄─── SYNC_CONFIRM ─────────────────────────── │
 │                                              │
 │──── FETCH_PRESET_DATA ─────────────────────► │
 │◄─── data chunks (same as host-init Step 4) ── │
```

### PRESET_CHANGED_NOTIFICATION (Device → Host, 44 bytes)

Sent by the device on every preset change. Key field: `0x6C PP` where PP is the new 0-indexed preset.

```
21 00 00 18 EF 03 01 10 00 NN 00 04 ...
  ... 6B 00  ; bank index
  ... 6C PP  ; new preset index
  ...
```

### SYNC_ACK (Host → Device, 36 bytes)

The host acknowledges the notification and requests sync:

```
19 00 00 18 01 10 EF 03 00 NN 00 04 ...
83
   66 CD 03 fX
   64 17            ; cmd=0x17 (sync/ack)
   65 C0
00 00 00
```

### SYNC_CONFIRM (Device → Host, 60 bytes)

Same structure as `CHANGE_PRESET_ACK`. Extract bank, index, and name.

### FETCH_PRESET_DATA + data drain

Identical to the host-initiated Steps 3 and 4.

---

## Reference Byte Sequences

### CHANGE_PRESET — Static preamble (bank=0x00, preset=0x00)

```
1D 00 00 18 01 10 EF 03 00 06 00 04 1A 10 00 00
01 00 06 00 09 00 00 00
83 66 CD 03 E9 64 14 65 82 6B 00 6C 00
00 00 00
```

### FETCH_PRESET_DATA — Complete static packet

```
19 00 00 18 01 10 EF 03 00 07 00 0C 38 10 00 00
01 00 06 00 09 00 00 00
83 66 CD 03 EA 64 16 65 C0
00 00 00
```

---

## Implementation Checklist

- [ ] After the 5-packet session init, sequence counter is at 5 → first new packet uses `seq = 0x06`
- [ ] `CHANGE_PRESET` uses command type `00 04`; `FETCH_PRESET_DATA` uses `00 0C`
- [ ] MessagePack `uint16` values are encoded **big-endian** (`CD hi lo`), unlike the USB header fields which are little-endian
- [ ] Extract bank, preset index, and name from the `0x68` sub-map of the ACK
- [ ] Strip trailing `\0` from the preset name string
- [ ] Always drain the full preset data stream after `FETCH_PRESET_DATA` (loop until `n < 272`)
- [ ] The device-initiated flow uses command `0x17` (sync/ack) instead of `0x14` (change preset)
