# Preset Data Format

This document describes the MessagePack payload returned by the device when reading presets. It applies to all operations that stream preset data (e.g. [listing presets](./list.md)).

---

## Raw Stream Layout

The reassembled buffer is a **continuous stream of MessagePack bytes**. It does not start at offset `0` — there is a variable-length preamble that must be skipped.

---

## Locating the Preset Array

Search the buffer for the 3-byte marker sequence:

```
DC 00 80
```

This is the MessagePack **`array16` format byte** (`0xDC`) followed by the count `0x0080` = **128**, indicating an array of 128 presets.

```
start_index = first occurrence of [0xDC, 0x00, 0x80] in raw_stream
msgpack_data = raw_stream[start_index..]
```

Parse the MessagePack value at `msgpack_data` using any standard MessagePack library. The root value will be an **array of 128 elements**.

---

## Preset Array Structure

```
Array[128]
└── [i] fixmap(1)                          // outer map: 1 entry
      └── key:   integer (preset index)    // u16, device-global (see below)
          value: map                       // inner map: preset fields
                └── key 109: str           // preset name (null-terminated)
```

---

## Preset Indices

The outer-map key is the **device-global** preset index, not the position within the setlist:

```
index = setlist × preset_count + slot
```

| Setlist | Index range (128-preset device) | MessagePack encoding |
|---|---|---|
| 0 | `0`–`127` | positive fixint |
| 1 | `128`–`255` | `uint8` (`CC xx`) |
| 2–7 | `256`–`1023` | `uint16` (`CD xx xx`) |

Single-setlist devices only ever return `0`–`preset_count − 1`. Decode the key as an unsigned integer of any width and subtract `setlist × preset_count` to obtain the slot; treat an index outside the requested setlist's range as a protocol error.

---

## Field Reference

| Key (integer) | Type | Description |
|---|---|---|
| `109` | string | Preset name. **Includes a trailing null byte (`\0`)**; strip it before use. |

Other keys may be present in the inner map and should be ignored for the purposes of name enumeration. Their meaning is not yet fully documented.

---

## Extracting Preset Names

```
for each item in root_array:
    outer_map  = item as map
    index      = outer_map[0].key as u16          // device-global
    slot       = index - setlist * preset_count    // 0..preset_count
    inner_map  = outer_map[0].value as map
    raw_name   = inner_map[key=109] as string
    clean_name = raw_name.trim_end_matches('\0')
    presets.push({ slot, name: clean_name })
```

---

## Known Device Quirks

### Null-Terminated Strings

The string length encoded in the MessagePack header **includes the null terminator**. A standard MessagePack decoder will include the `\0` as part of the string value. Always strip it explicitly:

```rust
let name = raw_name.trim_end_matches('\0');
```

> This is a Line 6 implementation detail and is not standard MessagePack behavior.
