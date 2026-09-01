# Selecting a Preset

This document describes how to make a preset the device's active preset.

> **Prerequisites:** Complete the [session handshake](../session-handshake.md) before executing the phase described here. The sequence number below assumes a fresh session where `seq` starts at `0x06`.

---

## Operation Overview

```
[SESSION INIT complete — seq at 0x06]
         │
         ▼
[PHASE 1] Select Preset   — 1 packet (seq=0x06) → ACK
         │
         ▼
[PHASE 2] Drain           — discard the notification burst
```

Unlike [listing presets](./list.md), selection is a single request/response exchange. The select packet occupies the `seq=0x06` slot that `OPEN_PRESETS` uses in a listing session; the two operations are never combined in one session.

---

## Phase 1 — Select Preset

Sends the selection command (`cmd=0x04`, `seq=0x06`).

```
1D 00 00 18 01 10 EF 03 00 06 00 04 1A 10 00 00
01 00 02 00 0D 00 00 00 83 66 CD 03 F2 64 14 65
82 6B SS 6C PP 00 00 00
```

**Length:** 40 bytes

| Byte(s) | Value | Description |
|---|---|---|
| 0–23 | fixed | Header, routing, `seq=0x06`, `cmd=0x04`, inner resource header (payload length `0x0D`) |
| 24–37 | MessagePack | `{102: 1010, 100: 20, 101: {107: SS, 108: PP}}` |
| 34 | `SS` | **Setlist index** (`0` on single-setlist devices) |
| 36 | `PP` | **Preset slot** within the setlist (`0`–`preset_count − 1`) |
| 38–39 | `00 00` | Padding to a 4-byte boundary |

Both `SS` and `PP` are encoded as MessagePack positive fixints and must therefore be `< 128`.

```rust
fn build_select_preset_request(seq: u8, setlist: u8, preset: u8) -> [u8; 40] {
    [0x1D, 0x00, 0x00, 0x18, 0x01, 0x10, 0xEF, 0x03, 0x00, seq,  0x00, 0x04,
     0x1A, 0x10, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x0D, 0x00, 0x00, 0x00,
     0x83, 0x66, 0xCD, 0x03, 0xF2, 0x64, 0x14, 0x65, 0x82, 0x6B, setlist,
     0x6C, preset, 0x00, 0x00, 0x00]
}
```

### Response

Perform one bulk read and match the sequence byte (`0x06`). The ACK content is not parsed.

---

## Phase 2 — Drain

The preset change triggers an unsolicited burst of notification packets from the device. Read from the IN endpoint until a short timeout (50 ms) elapses before starting the next session; otherwise those packets misalign the next request/response pairing. See [transport.md](../transport.md).

---

## Addressing

| Device family | Setlist | Preset slot |
|---|---|---|
| HX Stomp | `0` | `0`–`125` |
| HX Stomp XL | `0` | `0`–`127` |
| Helix Floor | `0`–`7` | `0`–`127` |

Slots are numbered sequentially across the device's banks: on a Helix, slot `3` is preset `01D`, slot `4` is `02A`.

Validated on an HX Stomp XL (setlist `0`) and a Helix Floor (setlist `2`).
