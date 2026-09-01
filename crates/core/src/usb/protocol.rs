pub const EP_OUT: u8 = 0x01;
pub const EP_IN: u8 = 0x81;
pub const INTERFACE: u8 = 0;

pub const TIMEOUT_MS: u64 = 2_000;
pub const DRAIN_TIMEOUT_MS: u64 = 50;

pub const OFFSET_STEP: u32 = 0x0100;
pub const STREAM_INITIAL_OFFSET: u32 = 0x0000_1138;
pub const PAGINATION_INITIAL_SEQ: u8 = 0x08;

/// A response shorter than this value signals end-of-stream.
pub const MAX_CHUNK_SIZE: usize = 272;

/// Number of leading header bytes to skip before the MessagePack payload.
pub const RESPONSE_HEADER_SIZE: usize = 16;

// Common header layout for Line 6 proprietary USB packets:
//   [0..4]   – framing / length prefix (low byte at [0])
//   [4..8]   – protocol routing field; host→device emits `01 10 EF 03`,
//              device→host echoes it byte-pair-swapped as `EF 03 01 10`
//   [8..10]  – sequence number, big-endian u16. The high byte at [8] is
//              always 0 in observed traffic, so [9] is the effective u8
//              seq (see `SEQ_BYTE_OFFSET` in `client.rs`).
//   [10..12] – command, big-endian u16. [10] is always 0; [11] is the
//              effective cmd byte (e.g. 0x04 = open, 0x08 = chunk req).
//   [12..]   – payload

/// Channel handshake (`magic = 0x0028`).
/// Re-synchronises the device's session state.
pub const HANDSHAKE: &[u8] = &[
    0x0C, 0x00, 0x00, 0x28, 0x01, 0x10, 0xEF, 0x03, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0x00, 0x21,
    0x00, 0x10, 0x00, 0x00,
];

/// Open first session resource (`cmd = 0x04`, `seq = 0x02`).
pub const SESSION_OPEN_1: &[u8] = &[
    0x11, 0x00, 0x00, 0x18, 0x01, 0x10, 0xEF, 0x03, 0x00, 0x02, 0x00, 0x04, 0x00, 0x10, 0x00, 0x00,
    0x01, 0x00, 0x02, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
];

/// First session chunk request (`cmd = 0x08`, `seq = 0x03`).
pub const SESSION_CHUNK_1: &[u8] = &[
    0x08, 0x00, 0x00, 0x18, 0x01, 0x10, 0xEF, 0x03, 0x00, 0x03, 0x00, 0x08, 0x09, 0x10, 0x00, 0x00,
];

/// Open second session resource (`cmd = 0x04`, `seq = 0x04`).
pub const SESSION_OPEN_2: &[u8] = &[
    0x1A, 0x00, 0x00, 0x18, 0x01, 0x10, 0xEF, 0x03, 0x00, 0x04, 0x00, 0x04, 0x09, 0x10, 0x00, 0x00,
    0x01, 0x00, 0x02, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x83, 0x66, 0xCD, 0x03, 0xE8, 0x64, 0xCC, 0xFE,
    0x65, 0x80, 0x00, 0x00,
];

/// Second session chunk request (`cmd = 0x08`, `seq = 0x05`).
pub const SESSION_CHUNK_2: &[u8] = &[
    0x08, 0x00, 0x00, 0x18, 0x01, 0x10, 0xEF, 0x03, 0x00, 0x05, 0x00, 0x08, 0x1A, 0x10, 0x00, 0x00,
];

pub const SESSION_INIT_SEQUENCE: &[&[u8]] = &[
    HANDSHAKE,
    SESSION_OPEN_1,
    SESSION_CHUNK_1,
    SESSION_OPEN_2,
    SESSION_CHUNK_2,
];

/// Open the presets resource (`cmd = 0x04`, `seq = 0x06`).
pub const OPEN_PRESETS: &[u8] = &[
    0x19, 0x00, 0x00, 0x18, 0x01, 0x10, 0xEF, 0x03, 0x00, 0x06, 0x00, 0x04, 0x1A, 0x10, 0x00, 0x00,
    0x01, 0x00, 0x02, 0x00, 0x09, 0x00, 0x00, 0x00, 0x83, 0x66, 0xCD, 0x03, 0xE9, 0x64, 0x00, 0x65,
    0xC0, 0x00, 0x00, 0x00,
];

/// Sequence number of the start-paged-stream request in a fresh session.
pub const OPEN_STREAM_SEQ: u8 = 0x07;

/// Builds the start-paged-stream request (`cmd = 0x0C`) for one setlist.
///
/// The payload is a MsgPack structure:
/// `{102: 1002, 100: 1, 101: {107: setlist, 101: 2}}`
///
/// Key 107 selects which setlist the device streams. Single-setlist devices
/// always use `0`, which reproduces the packet captured from an HX Stomp XL.
/// `setlist` must be `< 128` so it encodes as a positive fixint.
#[inline]
pub fn build_open_stream_request(seq: u8, setlist: u8) -> [u8; 40] {
    debug_assert!(setlist < 0x80, "setlist must encode as a positive fixint");
    [
        0x1D, 0x00, 0x00, 0x18, // Magic header with length for 40-byte packet
        0x01, 0x10, 0xEF, 0x03, // Standard Line 6 routing
        0x00, seq, 0x00, 0x0C, // Sequence and Command (0x0C = start paged stream)
        // Inner resource header
        0x38, 0x10, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x0D, 0x00, 0x00,
        0x00, // MsgPack payload length (13 bytes)
        // MsgPack Payload
        0x83, // fixmap(3 items)
        0x66, 0xCD, 0x03, 0xEA, // Key 102 (Transaction ID): 1002
        0x64, 0x01, // Key 100 (Command Type): 1 (Start stream)
        0x65, 0x82, // Key 101 (Args): map(2 items)
        0x6B, setlist, // Key 107 (Setlist Index): setlist
        0x65, 0x02, // Key 101: 2
        // Padding to align to 4-byte boundary
        0x00, 0x00, 0x00,
    ]
}

/// Builds a pagination request for the given sequence number and stream offset.
#[inline]
pub fn build_pagination_request(seq: u8, offset: u32) -> [u8; 16] {
    let [o0, o1, o2, o3] = offset.to_le_bytes();
    [
        0x08, 0x00, 0x00, 0x18, 0x01, 0x10, 0xEF, 0x03, 0x00, seq, 0x00, 0x08, o0, o1, o2, o3,
    ]
}

/// Builds a preset selection request (`cmd = 0x04`, `seq = 0x06`).
///
/// The payload is a MsgPack structure:
/// `{102: 1010, 100: 20, 101: {107: setlist, 108: preset}}`
///
/// `preset` is the slot within `setlist` (`0..preset_count`). Both values
/// must be `< 128` so they encode as positive fixints.
#[inline]
pub fn build_select_preset_request(seq: u8, setlist: u8, preset: u8) -> [u8; 40] {
    debug_assert!(
        setlist < 0x80 && preset < 0x80,
        "args must encode as positive fixints"
    );
    [
        0x1D, 0x00, 0x00, 0x18, // Magic header with length for 40-byte packet
        0x01, 0x10, 0xEF, 0x03, // Standard Line 6 routing
        0x00, seq, 0x00, 0x04, // Sequence and Command (0x04)
        // Inner resource header
        0x1A, 0x10, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x0D, 0x00, 0x00,
        0x00, // MsgPack payload length (13 bytes)
        // MsgPack Payload
        0x83, // fixmap(3 items)
        0x66, 0xCD, 0x03, 0xF2, // Key 102 (Transaction ID): 1010
        0x64, 0x14, // Key 100 (Command Type): 20 (Select)
        0x65, 0x82, // Key 101 (Args): map(2 items)
        0x6B, setlist, // Key 107 (Setlist Index): setlist
        0x6C, preset, // Key 108 (Preset Index): preset
        // Padding to align to 4-byte boundary
        0x00, 0x00, 0x00,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    // Byte-offset constants
    const SEQ_OFFSET: usize = 9;
    const CMD_OFFSET: usize = 11;

    // Builders

    #[test]
    fn build_pagination_request_formats_correctly() {
        let pkt = build_pagination_request(0x08, STREAM_INITIAL_OFFSET);

        assert_eq!(pkt.len(), 16);
        assert_eq!(pkt[SEQ_OFFSET], 0x08);
        assert_eq!(pkt[CMD_OFFSET], 0x08); // command: chunk request

        // Offset should be Little-Endian (0x0000_1138 -> 38 11 00 00)
        assert_eq!(&pkt[12..16], &[0x38, 0x11, 0x00, 0x00]);
    }

    /// Start-paged-stream packet as captured from an HX Stomp XL
    /// (`cmd = 0x0C`, `seq = 0x07`, setlist 0).
    const OPEN_STREAM_CAPTURED: &[u8] = &[
        0x1D, 0x00, 0x00, 0x18, 0x01, 0x10, 0xEF, 0x03, 0x00, 0x07, 0x00, 0x0C, 0x38, 0x10, 0x00,
        0x00, 0x01, 0x00, 0x02, 0x00, 0x0D, 0x00, 0x00, 0x00, 0x83, 0x66, 0xCD, 0x03, 0xEA, 0x64,
        0x01, 0x65, 0x82, 0x6B, 0x00, 0x65, 0x02, 0x00, 0x00, 0x00,
    ];

    #[test]
    fn build_open_stream_request_for_setlist_zero_matches_captured_packet() {
        let pkt = build_open_stream_request(OPEN_STREAM_SEQ, 0);
        assert_eq!(&pkt[..], OPEN_STREAM_CAPTURED);
    }

    #[test]
    fn build_open_stream_request_places_setlist_correctly() {
        let pkt = build_open_stream_request(OPEN_STREAM_SEQ, 5);

        assert_eq!(pkt.len(), 40);
        assert_eq!(pkt[SEQ_OFFSET], OPEN_STREAM_SEQ);
        assert_eq!(pkt[CMD_OFFSET], 0x0C); // command: start paged stream
        assert_eq!(pkt[34], 5, "setlist index mismatch");
    }

    #[test]
    fn build_select_preset_request_places_args_correctly() {
        let setlist_idx = 3;
        let preset_idx = 15;
        let pkt = build_select_preset_request(0x06, setlist_idx, preset_idx);

        assert_eq!(pkt.len(), 40);
        assert_eq!(pkt[SEQ_OFFSET], 0x06);
        assert_eq!(pkt[CMD_OFFSET], 0x04); // command: open

        // Validate dynamic payload injection points
        assert_eq!(pkt[34], setlist_idx, "setlist index mismatch");
        assert_eq!(pkt[36], preset_idx, "preset index mismatch");
    }

    // Static Packets & Sequences

    #[test]
    fn session_init_sequence_is_ordered_correctly() {
        let expected_seqs = [0x00u8, 0x02, 0x03, 0x04, 0x05];

        assert_eq!(SESSION_INIT_SEQUENCE.len(), 5);
        for (packet, &expected) in SESSION_INIT_SEQUENCE.iter().zip(&expected_seqs) {
            assert_eq!(
                packet[SEQ_OFFSET], expected,
                "unexpected seq in session init packet"
            );
        }
    }

    #[test]
    fn all_packets_use_standard_routing_field() {
        let all_packets: &[&[u8]] = &[
            HANDSHAKE,
            SESSION_OPEN_1,
            SESSION_CHUNK_1,
            SESSION_OPEN_2,
            SESSION_CHUNK_2,
            OPEN_PRESETS,
            &build_open_stream_request(OPEN_STREAM_SEQ, 0),
            &build_select_preset_request(0x06, 0, 0),
        ];

        for pkt in all_packets {
            assert_eq!(
                &pkt[4..8],
                &[0x01, 0x10, 0xEF, 0x03],
                "packet does not carry standard routing field"
            );
        }
    }
}
