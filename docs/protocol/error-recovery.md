# Error Handling and Session Recovery

---

## Stale Session State

If a previous session was not cleanly closed (e.g. the host process was killed), the device **ignores new init packets** until the application-level session state is resynchronized. The HANDSHAKE packet (step 1 of [session-handshake.md](./session-handshake.md)) resets this state.

**Recovery procedure:**

1. Drain stale IN data (see [transport.md](./transport.md#stale-data-drain)).
2. Repeat the full 5-packet init sequence.
3. Apply exponential backoff between attempts: `300 × 2^attempt` ms → (600, 1200, 2400, 4800 ms).
4. Give up after 5 failed attempts.

---

## Timeout Behavior

| Situation | Action |
|---|---|
| Timeout during init sequence | Drain + retry (stale session state) |
| Timeout during chunk loop | Fatal — do not retry; restart the full session |

USB timeouts during the init sequence indicate the device is in an inconsistent state. Drain and retry as described above. Timeouts during data streaming are fatal and must not be retried without a full session restart.

---

## Stream Offset Overflow

The stream offset is a `u32`. Although overflow is practically impossible given the data size (~128 presets), implementations should guard against it:

```
new_offset = offset + 0x0100
if new_offset overflows u32 → abort with error
```

---

## Session Teardown

There is no explicit session-close packet. To avoid leaving the device in an open state, the host must at minimum **release the USB interface** on exit. The drain + retry mechanism handles recovery from missed teardowns on the next session start.
