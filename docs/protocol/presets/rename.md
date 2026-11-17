# Renaming a Preset

This document describes how to store a new name for a preset.

> **Prerequisites:** Complete the [session handshake](../session-handshake.md) before executing the phase described here. The sequence number below assumes a fresh session where `seq` starts at `0x06`.

> **Status:** Validated on an HX Stomp (firmware 3.80): the name is stored and read back from the [preset stream](./list.md). The request format follows the preset-channel conventions shared with [selecting a preset](./select.md) and matches what HX Edit sends, as documented by the [helix_usb](https://github.com/kempline/helix_usb) project.

---

## Operation Overview

```
[SESSION INIT complete — seq at 0x06]
         │
         ▼
[PHASE 1] Rename Preset   — 1 packet (seq=0x06) → ACK
         │
         ▼
[PHASE 2] Drain           — discard the notification burst
```

Like [selection](./select.md), renaming is a single request/response exchange in the `seq=0x06` slot.

---

## Phase 1 — Rename Preset

Sends the rename command (`cmd=0x04`, `seq=0x06`). Unlike the other preset requests, the packet length depends on the name.

```
LL 00 00 18 01 10 EF 03 00 06 00 04 1A 10 00 00
01 00 02 00 PL 00 00 00 83 66 CD 03 ED 64 06 65
83 6B SS 6C PP 6D SL <name bytes> 00 <padding>
```

| Byte(s) | Value | Description |
|---|---|---|
| 0 | `LL` | Frame length: `16 + PL` (every byte after the routing field, unpadded) |
| 1–23 | fixed | Header, routing, `seq=0x06`, `cmd=0x04`, inner resource header |
| 20–23 | `PL` | MessagePack payload length, little-endian `u32`: `16 + name length` |
| 24– | MessagePack | `{102: 1005, 100: 6, 101: {107: SS, 108: PP, 109: name}}` |
| 34 | `SS` | **Setlist index** (`0` on single-setlist devices) |
| 36 | `PP` | **Preset slot** within the setlist (`0`–`preset_count − 1`) |
| 38 | `SL` | `fixstr` marker: `0xA0` + name length + 1 |
| 39– | ASCII | The name followed by a `00` terminator, which is part of the string |
| end | `00`… | Padding to a 4-byte boundary |

`SS` and `PP` are positive fixints and must be `< 128`. The terminator mirrors how the device stores names: names read back from the [preset stream](./data-format.md) carry the same trailing `\0`.

### Name Constraints

| Constraint | Value |
|---|---|
| Length | 1–16 characters |
| Characters | Printable ASCII (`0x20`–`0x7E`) |

### Example

Renaming setlist `0`, slot `12` to `Clean`:

```
25 00 00 18 01 10 EF 03 00 06 00 04 1A 10 00 00
01 00 02 00 15 00 00 00 83 66 CD 03 ED 64 06 65
83 6B 00 6C 0C 6D A6 43 6C 65 61 6E 00 00 00 00
```

### Response

Perform one bulk read and match the sequence byte (`0x06`). The payload is the response envelope `{102: 1005, 103: status, 104: nil}`. A stored name is acknowledged with **status `1`**, the same code a successful [selection](./select.md#response) reports. Note that this differs from the [setlist reply](./setlists.md), which reports `0` on success. Treat any other status as a rejected rename.

Captured from an HX Stomp:

```
19 00 00 18 EF 03 01 10 00 06 00 04 3E 02 00 00
00 00 02 00 09 00 00 00 83 66 CD 03 ED 67 01 68
C0 77 84 CD
```

The last three bytes pad the packet to a 4-byte boundary; unlike in requests, they are not zeroed and carry no meaning.

---

## Phase 2 — Drain

The device broadcasts the changed name to its other channels. Read from the IN endpoint until a short timeout (50 ms) elapses before starting the next session. See [transport.md](../transport.md).
