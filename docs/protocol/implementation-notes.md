# Implementation Notes

Practical notes for implementing the OpenHX protocol layer in Rust (or any other language).

---

## Minimum Required Libraries

| Requirement | Solution |
|---|---|
| USB communication | Any `libusb` binding (`libusb` 1.0+). In Rust: [`rusb`](https://crates.io/crates/rusb) or [`nusb`](https://crates.io/crates/nusb) |
| MessagePack decode | Any compliant MessagePack library. In Rust: [`rmpv`](https://crates.io/crates/rmpv) or [`rmp-serde`](https://crates.io/crates/rmp-serde) |
| Byte order | Native LE handling or manual `u32::from_le_bytes` |

---

## Portability Checklist

Use this as a pre-flight before testing a new implementation:

- [ ] Open device by VID `0x0E41` / PID `0x4253`
- [ ] Set USB configuration `1`
- [ ] Claim interface `0`
- [ ] Clear halt on endpoints `0x01` and `0x81`
- [ ] Drain stale IN data before first write (read with 50 ms timeout until timeout fires)
- [ ] Send the 5 init packets in order, reading and discarding one response after each
- [ ] Send OPEN_PRESETS (`seq=0x06`), read and discard response
- [ ] Send OPEN_STREAM (`seq=0x07`), collect `response[16..n]` as chunk #0
- [ ] Loop: send chunk requests (`seq` starting at `0x08`, `offset` starting at `0x00001138`), collect `response[16..n]`, stop once the stream holds `8 + payload_len` bytes (`payload_len` from bytes `20..24` of the first response)
- [ ] Locate `DC` followed by `preset_count` as big-endian `u16` in the reassembled buffer (`DC 00 7E` on an HX Stomp, `DC 00 80` on an HX Stomp XL / Helix)
- [ ] Parse the MessagePack `array16` of `preset_count` maps
- [ ] For each element: take its array position as the slot (the outer map key can be stale, see [data-format.md](./presets/data-format.md#preset-slots-and-indices)), look up inner map key `109` as preset name, strip the trailing `\0`
- [ ] Release USB interface on exit

---

## Known Device Quirks

| Quirk | Detail |
|---|---|
| Null-terminated strings | MessagePack string lengths include the `\0`. Strip explicitly with `trim_end_matches('\0')`. |
| Custom handshake magic | Uses `0x0028` instead of the standard Line 6 value `0x0018`. |
| 16-byte response header | All data responses (Phases 2 and 3) have a 16-byte header; payload starts at byte 16. |
| Stale session state | The device ignores init packets from a new session if the previous one wasn't closed. Drain + retry resolves this. |
| Length-delimited stream | No explicit terminator packet; the first response announces the stream length. Chunk sizes vary (272, 468 and 512 bytes seen), and header-only replies can appear mid-stream. |
