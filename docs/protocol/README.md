# Protocol Documentation

This directory documents the USB bulk transfer protocol used to communicate with **Line 6 HX series devices** (reverse-engineered from live USB captures). All sequences have been validated against a fully working implementation.

> **Scope:** This documentation covers the application-level protocol layered on top of USB bulk transfers. It is device-agnostic where possible, but all byte sequences were captured from an **HX Stomp XL**.

---

## Document Index

### Foundation (shared across all operations)

| Document | Description |
|---|---|
| [usb-device.md](./usb-device.md) | USB identification: VID/PID, endpoints, configuration |
| [transport.md](./transport.md) | Transport layer: bulk transfers, timeouts, stale data drain |
| [session-handshake.md](./session-handshake.md) | Session initialization: 5-packet handshake sequence |
| [error-recovery.md](./error-recovery.md) | Error handling, session recovery, retry strategy |

### Preset Operations

| Document | Description |
|---|---|
| [presets/list.md](./presets/list.md) | Enumerate all 128 preset names
| [presets/data-format.md](./presets/data-format.md) | MessagePack payload format and preset data structure |

### Reference

| Document | Description |
|---|---|
| [packet-reference.md](./packet-reference.md) | Master table of all known packets |
| [implementation-notes.md](./implementation-notes.md) | Portability checklist, known device quirks, required libraries |

---

## Session Cycle Overview

Every operation with the device follows this top-level flow:

```
[CONNECT & DRAIN STALE DATA]
         │
         ▼
[SESSION INIT] — 5-packet handshake
         │
         ▼
[OPERATION] — e.g. list presets, rename preset, trigger footswitch
         │
         ▼
[RELEASE USB INTERFACE]
```

Each packet exchange follows a strict request/response pattern:

```
write_bulk(EP_OUT, packet)
read_bulk(EP_IN, response_buffer)
```

The device will not accept new commands until the previous response has been read.

---

## Tested Devices

| Device | VID | PID | Status |
|---|---|---|---|
| HX Stomp XL | `0x0E41` | `0x4253` | ✅ Validated |
