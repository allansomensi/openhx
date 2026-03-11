# Transport Layer

All communication uses **USB bulk transfers** on the vendor-specific interface (`0`).

---

## Request / Response Pattern

The protocol is strictly request/response. The host sends one packet and then reads one response before proceeding to the next step. **Never send two packets back-to-back without reading the response in between.**

---

## Buffer Sizes and Timeouts

| Parameter | Value | Notes |
|---|---|---|
| Read buffer | `512 bytes` | The device never sends more than 512 bytes in a single bulk transfer during this protocol |
| Operation timeout | `2000 ms` | Sufficient for all operations under normal conditions |
| Drain timeout | `50 ms` | Short timeout used during stale data drain (see below) |

---

## Byte Order

Multi-byte integers in all payloads are **little-endian**.

---

## Stale Data Drain

Before starting any session, drain any data the device may have buffered from a previous session that was not cleanly closed.

```
LOOP:
    read_bulk(EP_IN, buf[512], timeout=50ms)
    if timeout / error → STOP
```

Repeat until the read returns a timeout error. This prevents residual IN packets from misaligning the request/response state machine.

> **When to drain:** Always drain immediately after claiming the interface, before sending the first packet of the [session handshake](./session-handshake.md).
