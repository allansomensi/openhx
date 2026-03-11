# Session Handshake (Phase 0)

Every operation requires a **5-packet initialization sequence** before the device will accept any application commands. These packets establish the logical communication channel at the application level.

> **Prerequisites:** The USB interface must be open, configured, and [stale data drained](./transport.md#stale-data-drain) before sending Packet 1.

All packets in this phase target the **primary channel** with source `0x1001` and destination `0x03EF`.

---

## Packet Sequence

### Packet 1 — HANDSHAKE

Establishes the logical channel. Uses the magic value `0x0028` (distinct from the standard Line 6 value `0x0018`).

```
0C 00 00 28 01 10 EF 03 00 00 00 02 00 01 00 21
00 10 00 00
```

**Length:** 20 bytes

---

### Packet 2 — SESSION_OPEN_1

Opens the first session resource (`cmd=0x04`, `seq=0x02`).

```
11 00 00 18 01 10 EF 03 00 02 00 04 00 10 00 00
01 00 02 00 01 00 00 00 02 00 00 00
```

**Length:** 28 bytes

---

### Packet 3 — SESSION_CHUNK_1

First session chunk request (`cmd=0x08`, `seq=0x03`).

```
08 00 00 18 01 10 EF 03 00 03 00 08 09 10 00 00
```

**Length:** 16 bytes

---

### Packet 4 — SESSION_OPEN_2

Opens the second session resource (`cmd=0x04`, `seq=0x04`).

```
1A 00 00 18 01 10 EF 03 00 04 00 04 09 10 00 00
01 00 02 00 0A 00 00 00 83 66 CD 03 E8 64 CC FE
65 80 00 00
```

**Length:** 36 bytes

---

### Packet 5 — SESSION_CHUNK_2

Second session chunk request (`cmd=0x08`, `seq=0x05`).

```
08 00 00 18 01 10 EF 03 00 05 00 08 1A 10 00 00
```

**Length:** 16 bytes

---

## Response Handling

For each of the 5 packets: send the packet, then perform one bulk read. **The response content is ignored.** The read is mandatory to keep the device's request/response state machine synchronized.

```
for packet in [HANDSHAKE, SESSION_OPEN_1, SESSION_CHUNK_1, SESSION_OPEN_2, SESSION_CHUNK_2]:
    write_bulk(EP_OUT, packet)
    read_bulk(EP_IN, buf[512])   // discard response
```

---

## Sequence Numbers

After the handshake completes, the next sequence number available for application commands is `0x06`. Each subsequent packet must increment `seq` by 1 (wrapping `0xFF → 0x00`).

---

## Sequence Diagram

```
Host                                       HX Stomp
 │                                              │
 │──── drain reads until timeout ─────────────► │
 │                                              │
 │──── HANDSHAKE (20 bytes) ──────────────────► │
 │◄─── ack (ignored) ────────────────────────── │
 │──── SESSION_OPEN_1 (28 bytes) ─────────────► │
 │◄─── ack (ignored) ────────────────────────── │
 │──── SESSION_CHUNK_1 (16 bytes) ────────────► │
 │◄─── ack (ignored) ────────────────────────── │
 │──── SESSION_OPEN_2 (36 bytes) ─────────────► │
 │◄─── ack (ignored) ────────────────────────── │
 │──── SESSION_CHUNK_2 (16 bytes) ────────────► │
 │◄─── ack (ignored) ────────────────────────── │
 │                                              │
 │     [device ready for application commands]  │
```
