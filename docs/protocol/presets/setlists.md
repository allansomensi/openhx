# Listing Setlists

This document describes how to read the device's setlist table — the names shown on the device itself (`FACTORY 1`, `USER 1`, …).

> **Prerequisites:** Complete the [session handshake](../session-handshake.md) before executing the phase described here. The sequence number below assumes a fresh session where `seq` starts at `0x06`.

---

## Operation Overview

The setlist table is the **reply to the open-presets command** (`OPEN_PRESETS`, opcode `0`). No additional packet is needed: a client that lists presets already sends this request and can read the setlist names from the response it would otherwise discard.

```
[SESSION INIT complete — seq at 0x06]
         │
         ▼
[Open Preset Resource]  — OPEN_PRESETS (seq=0x06) → setlist table
         │
         ▼
[optionally continue with the preset stream — see list.md]
```

---

## Request

The unmodified `OPEN_PRESETS` packet documented in [list.md](./list.md#phase-1--open-preset-resource) (`cmd=0x04`, `seq=0x06`, 36 bytes, payload `{102: 1001, 100: 0, 101: nil}`).

---

## Response

A single packet — 140 bytes on a Helix Floor, shorter on single-setlist devices. Its layout is the standard response envelope:

| Byte(s) | Description |
|---|---|
| 0–15 | Transport header |
| 16–19 | Inner header |
| 20–23 | MessagePack payload length, little-endian `u32` |
| 24… | MessagePack payload |

The payload is the response envelope `{102: transaction_id, 103: status, 104: result}`, where `103` is `0` on success and `104` holds an array of one-entry maps, `{setlist_index: name}`.

Captured from a Helix Floor:

```
83 66 CD 03 E9 67 00 68 98
 81 CD 00 00 AA "FACTORY 1\0"  81 CD 00 01 AA "FACTORY 2\0"
 81 CD 00 02 A7 "USER 1\0"     81 CD 00 03 A7 "USER 2\0"
 81 CD 00 04 A7 "USER 3\0"     81 CD 00 05 A7 "USER 4\0"
 81 CD 00 06 A7 "USER 5\0"     81 CD 00 07 AA "TEMPLATES\0"
```

Names are null-terminated in the same way as preset names — the encoded string length includes the trailing `\0`, so strip it before use. See [data-format.md](./data-format.md#known-device-quirks).

---

## Response Envelope

Every command reply uses the same three keys, which is worth knowing when adding new operations:

| Key | Meaning |
|---|---|
| `102` | Transaction id, echoed from the request |
| `103` | Status — `0` on success |
| `104` | Result value (`nil` for commands with no return value) |

---

Validated on a Helix Floor (8 setlists). Single-setlist devices are expected to return one entry; untested.
