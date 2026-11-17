# Reading the Active Preset's Blocks

This document describes how to read the blocks of the device's active preset — which model sits in each position of its signal paths, whether it is on, and its parameter values.

> **Status:** Read-only, HX Stomp only. Validated on an HX Stomp (firmware 3.71); other models have not been tested, and openhx refuses to read their blocks until they are. The block layout follows the structure documented by the [helix_usb](https://github.com/kempline/helix_usb) project; how to *change* a parameter has not been captured yet.

---

## The Edit Channel

Preset names, selection and renaming use the preset channel (routing `01 10 EF 03`, resource `2`). The active preset's contents come from a second channel:

| Direction | Routing field (bytes 4–7) |
|---|---|
| Host → device | `80 10 ED 03` |
| Device → host | `ED 03 80 10` |

It uses resource `6` and keeps its own sequence numbers. Replies from the two channels can be told apart only by the routing field, so match it in addition to the sequence byte.

---

## Operation Overview

```
[PHASE 1] Handshake      — EDIT_HANDSHAKE (seq=0x00) → ACK
[PHASE 2] Open resource  — EDIT_SESSION_OPEN (seq=0x02) → ACK
[PHASE 3] Start stream   — READ_ACTIVE_PRESET (seq=0x03) → chunk #0
[PHASE 4] Paginate       — chunk requests (seq=0x04, 0x05, …) until the stream is complete
[PHASE 5] Drain
```

No preset-channel session is needed first.

---

## Phases 1–2 — Handshake and Open

```
EDIT_HANDSHAKE (20 bytes):
0C 00 00 28 80 10 ED 03 00 00 00 02 00 01 00 21 00 10 00 00

EDIT_SESSION_OPEN (28 bytes):
11 00 00 18 80 10 ED 03 00 02 00 04 00 10 00 00 01 00 06 00 01 00 00 00 06 00 00 00
```

Both mirror their preset-channel counterparts with the edit routing and resource `6`. Like on the preset channel, the device occasionally drops the first handshake of a new session; resend it after a short timeout (~400 ms).

---

## Phase 3 — Start Stream

```
19 00 00 18 80 10 ED 03 00 03 00 0C 09 10 00 00
01 00 06 00 09 00 00 00 83 66 CD 03 F4 64 16 65
C0 00 00 00
```

`cmd = 0x0C` (start paged stream) with the payload `{102: 1012, 100: 22, 101: nil}`.

---

## Phase 4 — Paginate

Chunk requests have the same layout as on the preset channel ([list.md](./list.md#phase-3--paged-chunk-loop)), with the edit routing and `seq` continuing from `0x04`:

```
08 00 00 18 80 10 ED 03 00 SS 00 08 O0 O1 O2 O3
```

The stream is length-delimited like the preset list: the first reply that carries data announces the length in bytes `20..24`, and the stream is complete once it holds `8 + payload_len` bytes.

> **The start request may be answered header-only.** Right after a preset-list stream, the device answered `READ_ACTIVE_PRESET` with a 16-byte reply carrying no data; the length then arrived with the first chunk. Take the length from the first reply with data, not from the reply to the start request.

---

## Active Preset Info

To learn *which* preset is active, send this instead of `READ_ACTIVE_PRESET` after Phase 2. It is a single request/reply, with no stream:

```
19 00 00 18 80 10 ED 03 00 03 00 04 09 10 00 00
01 00 06 00 09 00 00 00 83 66 CD 04 04 64 17 65
C0 00 00 00
```

`cmd = 0x04` with the payload `{102: 1028, 100: 23, 101: nil}`. The reply is a response envelope whose result starts with the preset's address and name:

```
{102: 1028, 103: 0, 104: {107: setlist, 108: preset, 109: name, …}}
```

Captured from an HX Stomp with slot `031` ("New Preset") active:

```
38 00 00 18 ED 03 80 10 00 03 00 04 1A 02 00 00
00 00 06 00 28 00 00 00 83 66 CD 04 04 67 00 68
86 6B CD 00 00 6C CD 00 1F 6D AB 4E 65 77 20 50
72 65 73 65 74 00 75 C2 53 92 CD 1E F8 00 5C 00
```

`107` and `108` are encoded as `uint16`. The remaining keys (`117`, `83`) are not decoded.

---

## Payload

The assembled stream holds the 8-byte inner header followed by a response envelope:

```
{102: 1012, 103: 0, 104: <blob>}
```

`104` is a `str16` whose bytes are a sequence of MessagePack values:

```
"l6-helix\0"            magic
<str, 48 bytes>         offset table (12 × u32 LE); not needed to decode the rest
{                       preset body
  0:  { 21: …, 22: [20 slots] },        signal flow
  7:  { 37: "v3.71-…", … },             firmware version
  …                                     snapshots, footswitch and controller assignments
}
```

### Slots

`0.22` is an array of 20 slots, the two signal paths in order. In each path, slot `0` is the input, slots `1`–`8` are the block positions, and slot `9` is the output:

| Array index | Meaning |
|---|---|
| 0 | Path 1 input |
| 1–8 | Path 1, positions 1–8 |
| 9 | Path 1 output |
| 10 | Path 2 input (split) |
| 11–18 | Path 2, positions 1–8 |
| 19 | Path 2 output (merge) |

Each slot is `{19: type, 20: body}`:

| Type (`19`) | Meaning | Body (`20`) |
|---|---|---|
| `0`–`3` | Path input / output | Input, output, split or merge settings |
| `6` | Block | See below |
| `7` | Looper (not decoded yet) | — |
| `8` | Empty position | `nil` |

### Split and Join

Path 2's input slot (type `2`) holds the split under key `15`, and its output slot (type `3`) holds the join under key `17`:

```
{ 8: model, 13: slot, 10: enabled, 7: params }
```

`13` is the index of the path 1 slot that the split or join sits *before*. Path 2 carries the signal for the positions in between. The factory preset "Cathedral" on an HX Stomp has a split with `13: 3` and a join with `13: 5`. Path 1 branches off between positions 2 and 3, and path 2 merges back before position 5, where path 1's reverb sits. The split model was `257` and the join model `151` (a mixer with six parameters) in both captured presets; the other split types have not been captured yet.

### Block Body

```
{
  24: { 23: dual, 25: model_a, 26: model_b },
   9: …,
  10: enabled,
  11: params_a,
  12: params_b,
}
```

| Key | Type | Description |
|---|---|---|
| `24.23` | bool | `true` for an amp+cab block |
| `24.25` | uint | Model ID; the amp of an amp+cab block |
| `24.26` | int | The cab of an amp+cab block; `-1` otherwise |
| `10` | bool | Block is on |
| `11` | map | Parameters of model A |
| `12` | map | Parameters of model B (the cab); empty otherwise |

A parameter map is `{2: count, 3: …, 4: [values]}`. Values are in the model's parameter order and are not named in the stream:

| Encoding | Meaning |
|---|---|
| `float32` | Knob value; most knobs are `0.0`–`1.0` (a knob showing `65%` stores `0.65`) |
| uint | Selector (mic type, note value, …) |
| bool | Switch |
| `str` | Reference, e.g. an IR's hash |

Model IDs map to model names, categories and parameter definitions (names, types, ranges, display units); openhx ships that table in `crates/core/src/blocks/catalog.rs`. The value list holds a model's regular parameters in the order HX Edit lists them, followed by a cab's mic selector or a delay's or reverb's trails switch. Where a tempo sync switch (`TempoSync1`, bool) and its note value (`SyncSelect1`, uint) sit differs between models: Pitch Echo stores the switch first, Simple Delay the note value. The enabled state is key `10`, and the stereo variant is a separate model; some parameters exist in the stereo variant only. This is verified against HX Stomp captures for a distortion, an amp, a cab, a dynamic, an EQ, two delays and a reverb. openhx uses a definition only when the device sent exactly one value of the defined type for each parameter.

### Example

A drive block in path 1, position 1:

```
82 13 06 14 85                       {19: 6, 20: {
   18 83 17 C2 19 CD 02 46 1A FF        24: {23: false, 25: 582, 26: -1},
   09 01                                9: 1,
   0A C3                                10: true,
   0B 83 02 06 03 06 04 96 …            11: {2: 6, 3: 6, 4: [6 values]},
   0C 83 02 00 03 00 04 90              12: {2: 0, 3: 0, 4: []}}}
```

Model `582` is the Horizon Drive (mono).
