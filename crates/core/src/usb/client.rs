use super::{
    parser::{parse_msgpack_stream, parse_setlists, response_payload, response_status, stream_len},
    preset_data::{parse_active_preset, parse_active_preset_info},
    protocol::*,
};
use crate::{
    blocks::PresetChain,
    client::DeviceClient,
    device::{KnownDevice, catalog::DEVICE_CATALOG, profile::DeviceProfile},
    error::HxError,
    models::{ActivePreset, Preset, Setlist, validate_preset_name},
};
use openhx_i18n::fl;
use rusb::{Context, DeviceHandle, UsbContext};
use std::time::Duration;

/// Maximum number of attempts to initialize the USB session before giving up.
const MAX_INIT_RETRIES: u32 = 5;

/// Base back-off duration in milliseconds.
///
/// The actual wait on attempt `n` (1-indexed) is `BASE_BACKOFF_MS × 2ⁿ`,
/// yielding 600 ms, 1 200 ms, 2 400 ms, 4 800 ms for attempts 1–4.
const BASE_BACKOFF_MS: u64 = 300;

/// Offset of the per-packet sequence byte within both request and response
/// headers.
///
/// The wire field is a big-endian `u16` at bytes 8–9; in observed traffic
/// the high byte is always zero, so byte 9 is the effective `u8` seq. See
/// the header layout in `protocol.rs`.
const SEQ_BYTE_OFFSET: usize = 9;

/// Offset of the effective per-packet command byte within both request and
/// response headers.
///
/// The wire field is a big-endian `u16` at bytes 10–11; in observed traffic
/// the high byte is always zero, so byte 11 carries the effective cmd.
const CMD_BYTE_OFFSET: usize = 11;

/// Offset of the 4-byte routing field, which names the channel a packet
/// belongs to.
const ROUTING_OFFSET: usize = 4;

/// Maximum number of stale packets discarded while waiting for a matching
/// sequence byte before declaring the session wedged.
///
/// Sized purely as a runaway guard. A healthy device emits at most a few
/// out-of-band packets between transactions; the per-discard `tracing`
/// log surfaces excessive drops long before this ceiling.
const MAX_STALE_DROPS: usize = 64;

/// Per-attempt timeout for the channel-handshake read.
///
/// A successful handshake responds in tens of milliseconds; anything
/// longer means the device dropped the write and a resend is cheaper
/// than waiting out the full session timeout.
const HANDSHAKE_RETRY_TIMEOUT_MS: u64 = 400;

/// Maximum number of handshake resends before surfacing a timeout to the
/// outer retry loop.
const HANDSHAKE_INNER_RETRIES: u32 = 4;

/// Renders the byte at `offset` as `0xNN`, or `<short>` if the read was
/// truncated below that offset. Used by the diagnostic logging paths.
fn fmt_byte_at(buf: &[u8], n: usize, offset: usize) -> String {
    if n > offset {
        format!("0x{:02X}", buf[offset])
    } else {
        "<short>".to_string()
    }
}

/// One of the device's two request channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Channel {
    /// Preset names, selection and renaming.
    Preset,
    /// The active preset's contents.
    Edit,
}

impl Channel {
    /// Routing of host→device packets on this channel.
    fn routing_out(self) -> [u8; 4] {
        match self {
            Self::Preset => PRESET_ROUTING_OUT,
            Self::Edit => EDIT_ROUTING_OUT,
        }
    }

    /// The handshake that opens this channel.
    fn handshake(self) -> &'static [u8] {
        match self {
            Self::Preset => HANDSHAKE,
            Self::Edit => EDIT_HANDSHAKE,
        }
    }

    /// Whether a reply of `n` bytes in `buf` belongs to this channel.
    ///
    /// Edit-channel replies are checked by routing, so a stale preset-channel
    /// packet with the same sequence byte is not mistaken for the reply.
    fn owns_reply(self, buf: &[u8], n: usize) -> bool {
        match self {
            Self::Preset => true,
            Self::Edit => {
                buf[..n].get(ROUTING_OFFSET..ROUTING_OFFSET + 4) == Some(&EDIT_ROUTING_IN)
            }
        }
    }
}

/// A USB client that communicates with any supported device
/// over bulk endpoints using the device's proprietary
/// MessagePack-over-USB protocol.
pub struct Client {
    handle: DeviceHandle<Context>,
    profile: &'static DeviceProfile,
}

impl Client {
    /// Attempts to establish a USB connection with a specific device model.
    pub fn connect(device: KnownDevice) -> Result<Self, HxError> {
        Self::connect_profile(device.profile())
    }

    /// Scans the USB bus to auto-detect and connect to the first supported device.
    ///
    /// This method iterates through the internal [`DEVICE_CATALOG`]. It is ideal for
    /// generic initialization where the exact hardware model isn't known ahead of time.
    pub fn detect() -> Result<Self, HxError> {
        for profile in DEVICE_CATALOG {
            match Self::connect_profile(profile) {
                Ok(client) => {
                    eprintln!("{}", fl!("usb-detected", device = client.profile.name));
                    return Ok(client);
                }
                Err(HxError::DeviceNotFound) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(HxError::DeviceNotFound)
    }

    /// Internal routine to handle the low-level `libusb` setup for a matched profile.
    ///
    /// This handles detaching default OS kernel drivers, setting the active configuration,
    /// claiming the proprietary interface, and clearing any endpoint halts.
    fn connect_profile(profile: &'static DeviceProfile) -> Result<Self, HxError> {
        let context = Context::new()?;

        let handle = context
            .open_device_with_vid_pid(profile.vendor_id, profile.product_id)
            .ok_or(HxError::DeviceNotFound)?;

        // Ask libusb to detach any kernel driver bound to the interface so we
        // can claim it exclusively.
        handle.set_auto_detach_kernel_driver(true).map_err(|e| {
            HxError::protocol(fl!("usb-kernel-detach-failed", error = e.to_string()))
        })?;

        // Ensure configuration 1 — the only configuration that exposes the
        // vendor-specific bulk endpoints.
        if handle.active_configuration()? != 1 {
            handle.set_active_configuration(1)?;
        }

        handle.claim_interface(INTERFACE)?;
        handle.clear_halt(EP_OUT)?;
        handle.clear_halt(EP_IN)?;

        let client = Self { handle, profile };

        // Discard data buffered by a prior unclosed session so it cannot
        // misalign the subsequent request/response pairing.
        client.drain_stale_data();

        Ok(client)
    }

    /// Runs `session` until it succeeds, retrying with exponential back-off
    /// while the device times out.
    ///
    /// The IN endpoint is drained before every attempt, not just retries:
    /// the previous transaction's trailing packets persist in the buffer
    /// across calls on a cached client.
    fn with_retries<T>(
        &self,
        mut session: impl FnMut(Duration) -> Result<T, HxError>,
    ) -> Result<T, HxError> {
        let timeout = Duration::from_millis(TIMEOUT_MS);

        for attempt in 0..MAX_INIT_RETRIES {
            if attempt > 0 {
                self.wait_with_backoff(attempt);
            }
            self.drain_stale_data();

            match session(timeout) {
                Err(HxError::Usb(rusb::Error::Timeout)) if attempt + 1 < MAX_INIT_RETRIES => {}
                result => return result,
            }
        }

        Err(HxError::protocol(fl!(
            "usb-device-unresponsive",
            device = self.profile.name,
            attempts = MAX_INIT_RETRIES
        )))
    }

    /// Opens the presets resource and parses the setlist names from its reply.
    ///
    /// The device answers the open-presets command with its setlist table, so
    /// this needs no packet beyond the ones a preset listing already sends.
    fn run_setlists_session(&self, timeout: Duration) -> Result<Vec<Setlist>, HxError> {
        let mut buf = vec![0u8; 512];

        self.session_init(&mut buf, timeout)?;

        self.handle.write_bulk(EP_OUT, OPEN_PRESETS, timeout)?;
        let n = self.read_reply(
            Channel::Preset,
            OPEN_PRESETS[SEQ_BYTE_OFFSET],
            &mut buf,
            timeout,
        )?;

        let setlists = parse_setlists(response_payload(&buf, n)?)?;

        self.drain_stale_data();

        Ok(setlists)
    }

    /// Internal routine to handle the session cycle specifically for changing a preset.
    fn run_select_session(
        &self,
        setlist: u8,
        preset: u8,
        timeout: Duration,
    ) -> Result<(), HxError> {
        let mut buf = vec![0u8; 512];

        self.session_init(&mut buf, timeout)?;

        // Send the Select Preset command (after handshake, the next seq is 0x06)
        let seq = 0x06;
        let request = build_select_preset_request(seq, setlist, preset);

        self.handle.write_bulk(EP_OUT, &request, timeout)?;
        self.read_reply(Channel::Preset, seq, &mut buf, timeout)?;

        // Consume the burst of notifications triggered by the device after
        // the preset change before releasing the endpoint for the next operation.
        self.drain_stale_data();

        Ok(())
    }

    /// Internal routine to handle the session cycle for renaming a preset.
    ///
    /// Retrying is safe: storing the same name twice leaves the device in
    /// the same state as storing it once.
    fn run_rename_session(
        &self,
        setlist: u8,
        preset: u8,
        name: &str,
        timeout: Duration,
    ) -> Result<(), HxError> {
        let mut buf = vec![0u8; 512];

        self.session_init(&mut buf, timeout)?;

        // Like a selection, the rename is the first command after session
        // init, so it takes seq 0x06.
        let seq = 0x06;
        let request = build_rename_preset_request(seq, setlist, preset, name);

        self.handle.write_bulk(EP_OUT, &request, timeout)?;
        let n = self.read_reply(Channel::Preset, seq, &mut buf, timeout)?;

        // A write command's ACK reports `ACK_STATUS_OK` once the device has
        // stored the change; anything else means the name was not stored.
        let status = response_status(response_payload(&buf, n)?)?;
        tracing::debug!(setlist, preset, ?status, "rename ACK");
        if status != Some(ACK_STATUS_OK) {
            return Err(HxError::protocol(fl!(
                "usb-write-rejected",
                device = self.profile.name,
                status = status.map_or_else(|| "none".to_owned(), |s| s.to_string())
            )));
        }

        // The device broadcasts the changed name to its other channels; clear
        // those notifications before the next operation.
        self.drain_stale_data();

        Ok(())
    }

    /// Executes a single, complete preset extraction session for `setlist`.
    fn run_session(&self, setlist: u8, timeout: Duration) -> Result<Vec<Preset>, HxError> {
        let mut buf = vec![0u8; STREAM_BUFFER_SIZE];

        self.session_init(&mut buf, timeout)?;

        // Phase 1: open presets resource
        self.handle.write_bulk(EP_OUT, OPEN_PRESETS, timeout)?;
        self.read_reply(
            Channel::Preset,
            OPEN_PRESETS[SEQ_BYTE_OFFSET],
            &mut buf,
            timeout,
        )?;

        // Phase 2: start stream for the requested setlist
        let open_stream = build_open_stream_request(OPEN_STREAM_SEQ, setlist);
        self.handle.write_bulk(EP_OUT, &open_stream, timeout)?;
        let n = self.read_reply(Channel::Preset, OPEN_STREAM_SEQ, &mut buf, timeout)?;

        // Phase 3: paginate until the announced length has arrived
        let raw_stream = self.read_stream(
            Channel::Preset,
            &mut buf,
            n,
            PAGINATION_INITIAL_SEQ,
            timeout,
        )?;

        let mut presets = parse_msgpack_stream(&raw_stream, self.profile.preset_count, setlist)?;
        presets.sort_unstable_by_key(|p| p.index);
        Ok(presets)
    }

    /// Asks the edit channel which preset is active; a single request/reply.
    fn run_active_preset_info_session(&self, timeout: Duration) -> Result<ActivePreset, HxError> {
        let mut buf = vec![0u8; STREAM_BUFFER_SIZE];

        self.open_edit_session(&mut buf, timeout)?;

        self.handle
            .write_bulk(EP_OUT, READ_ACTIVE_PRESET_INFO, timeout)?;
        let n = self.read_reply(Channel::Edit, READ_ACTIVE_PRESET_SEQ, &mut buf, timeout)?;
        tracing::debug!(hex = format!("{:02X?}", &buf[..n]), "active preset info");
        let active =
            parse_active_preset_info(response_payload(&buf, n)?, self.profile.preset_count)?;

        self.drain_stale_data();

        Ok(active)
    }

    /// Opens resource 6 on the edit channel.
    fn open_edit_session(&self, buf: &mut [u8], timeout: Duration) -> Result<(), HxError> {
        self.handshake(Channel::Edit, buf, timeout)?;
        self.handle.write_bulk(EP_OUT, EDIT_SESSION_OPEN, timeout)?;
        self.read_reply(
            Channel::Edit,
            EDIT_SESSION_OPEN[SEQ_BYTE_OFFSET],
            buf,
            timeout,
        )?;
        Ok(())
    }

    /// Reads the active preset's raw stream over the edit channel.
    fn run_active_preset_session(&self, timeout: Duration) -> Result<Vec<u8>, HxError> {
        let mut buf = vec![0u8; STREAM_BUFFER_SIZE];

        self.open_edit_session(&mut buf, timeout)?;

        self.handle
            .write_bulk(EP_OUT, READ_ACTIVE_PRESET, timeout)?;
        let n = self.read_reply(Channel::Edit, READ_ACTIVE_PRESET_SEQ, &mut buf, timeout)?;

        let raw_stream = self.read_stream(
            Channel::Edit,
            &mut buf,
            n,
            EDIT_PAGINATION_INITIAL_SEQ,
            timeout,
        )?;

        self.drain_stale_data();

        Ok(raw_stream)
    }

    /// Reads a length-delimited stream whose start request was answered with
    /// the `first_len` bytes in `buf`, requesting page after page from
    /// `initial_seq` on until the announced length has arrived.
    ///
    /// The first reply that carries data announces the stream length. Chunk
    /// sizes vary between devices and even between sessions, so the length,
    /// not the size of a chunk, tells when the stream is complete. A
    /// header-only reply (`cmd = 0x08`) carries no data: the device had no
    /// chunk ready yet (e.g. right after another stream), and the next
    /// request picks the stream up.
    fn read_stream(
        &self,
        channel: Channel,
        buf: &mut [u8],
        first_len: usize,
        initial_seq: u8,
        timeout: Duration,
    ) -> Result<Vec<u8>, HxError> {
        let mut raw_stream = Vec::with_capacity(8_192);
        let mut expected_len = None;
        self.collect_chunk(&mut raw_stream, &mut expected_len, buf, first_len)?;

        let mut seq = initial_seq;
        let mut offset = STREAM_INITIAL_OFFSET;

        for _ in 0..MAX_STREAM_REQUESTS {
            if expected_len.is_some_and(|len| raw_stream.len() >= len) {
                break;
            }

            let request = build_pagination_request(channel.routing_out(), seq, offset);
            self.handle.write_bulk(EP_OUT, &request, timeout)?;
            let n = self.read_reply(channel, seq, buf, timeout)?;
            tracing::debug!(
                ?channel,
                seq = format!("0x{seq:02X}"),
                len = n,
                received = raw_stream.len(),
                expected = ?expected_len,
                "stream chunk",
            );
            self.collect_chunk(&mut raw_stream, &mut expected_len, buf, n)?;

            offset = offset
                .checked_add(OFFSET_STEP)
                .ok_or_else(|| HxError::protocol(fl!("usb-stream-offset-overflow")))?;
            seq = seq.wrapping_add(1);
        }

        let expected_len = expected_len.unwrap_or(usize::MAX);
        if raw_stream.len() < expected_len {
            return Err(HxError::protocol(fl!(
                "usb-stream-incomplete",
                got = raw_stream.len(),
                expected = expected_len
            )));
        }

        Ok(raw_stream)
    }

    /// Appends one stream reply to `raw_stream`, taking the stream length
    /// from the first reply that carries data. Header-only replies carry
    /// nothing and are skipped.
    fn collect_chunk(
        &self,
        raw_stream: &mut Vec<u8>,
        expected_len: &mut Option<usize>,
        buf: &[u8],
        n: usize,
    ) -> Result<(), HxError> {
        if n <= RESPONSE_HEADER_SIZE {
            return Ok(());
        }
        if expected_len.is_none() {
            *expected_len = Some(stream_len(buf, n)?);
        }
        raw_stream.extend_from_slice(&buf[RESPONSE_HEADER_SIZE..n]);
        Ok(())
    }

    /// Flushes the USB IN endpoint of any unread data.
    ///
    /// Every transaction is followed by a short, unsolicited "epilogue":
    /// a handful of header-only frames (one with `cmd = 0x08`, then two
    /// or three with `cmd = 0x10` — a code the host never emits) carrying
    /// the just-completed transaction's ID and seq numbers continuing
    /// past the last ACK. They land in the IN buffer between calls.
    /// Consuming them here keeps the next session's request/response
    /// pairing aligned and also clears any leftover from a crashed
    /// session. The [`DRAIN_TIMEOUT_MS`] window assumes the tail finishes
    /// within ~50 ms of the last packet; [`Self::read_reply`] guards
    /// against the case where it doesn't.
    fn drain_stale_data(&self) {
        let timeout = Duration::from_millis(DRAIN_TIMEOUT_MS);
        let mut buf = vec![0u8; 512];
        while let Ok(n) = self.handle.read_bulk(EP_IN, &mut buf, timeout) {
            tracing::debug!(
                seq = fmt_byte_at(&buf, n, SEQ_BYTE_OFFSET),
                cmd = fmt_byte_at(&buf, n, CMD_BYTE_OFFSET),
                len = n,
                hex = format!("{:02X?}", &buf[..n.min(16)]),
                "drained stale pkt",
            );
        }
    }

    /// Reads from the IN endpoint until a reply on `channel` with the
    /// sequence byte `expected` arrives, discarding stale or out-of-band
    /// packets along the way.
    ///
    /// Out-of-band packets that arrive between transactions can land in
    /// the IN buffer ahead of the ACK and shift every subsequent read by
    /// one. Validating the sequence byte at [`SEQ_BYTE_OFFSET`] lets the
    /// caller discard such packets inline as a safety net behind
    /// [`Self::drain_stale_data`]. Each re-read uses the caller's full
    /// timeout: a genuine reply queued behind a stale packet returns
    /// immediately from the OS buffer regardless, and a shorter window
    /// would only risk misclassifying a slow-but-valid read as stale.
    fn read_reply(
        &self,
        channel: Channel,
        expected: u8,
        buf: &mut [u8],
        timeout: Duration,
    ) -> Result<usize, HxError> {
        for drops in 0..=MAX_STALE_DROPS {
            let n = self.handle.read_bulk(EP_IN, buf, timeout)?;
            if n > SEQ_BYTE_OFFSET && buf[SEQ_BYTE_OFFSET] == expected && channel.owns_reply(buf, n)
            {
                return Ok(n);
            }
            tracing::debug!(
                ?channel,
                expected = format!("0x{expected:02X}"),
                got = fmt_byte_at(buf, n, SEQ_BYTE_OFFSET),
                cmd = fmt_byte_at(buf, n, CMD_BYTE_OFFSET),
                len = n,
                hex = format!("{:02X?}", &buf[..n.min(16)]),
                drop = drops + 1,
                cap = MAX_STALE_DROPS,
                "discarding out-of-band USB pkt",
            );
        }
        Err(HxError::Usb(rusb::Error::Timeout))
    }

    /// Runs the full five-packet session-init exchange.
    ///
    /// The channel handshake is delegated to [`Self::handshake`], which
    /// retries internally on a dropped first write. The remaining four
    /// packets respond reliably once the channel is up and use the
    /// standard write-then-[`Self::read_reply`] path.
    fn session_init(&self, buf: &mut [u8], timeout: Duration) -> Result<(), HxError> {
        self.handshake(Channel::Preset, buf, timeout)?;
        for packet in &SESSION_INIT_SEQUENCE[1..] {
            self.handle.write_bulk(EP_OUT, packet, timeout)?;
            self.read_reply(Channel::Preset, packet[SEQ_BYTE_OFFSET], buf, timeout)?;
        }
        Ok(())
    }

    /// Performs the handshake of `channel`, retrying internally on timeout.
    ///
    /// The first handshake of a session is occasionally dropped by the
    /// device while it finishes settling from the previous session. A
    /// short-timeout resend handled here recovers in roughly
    /// [`HANDSHAKE_RETRY_TIMEOUT_MS`] milliseconds, avoiding the outer
    /// retry's full read timeout plus exponential back-off per occurrence.
    fn handshake(
        &self,
        channel: Channel,
        buf: &mut [u8],
        outer_timeout: Duration,
    ) -> Result<(), HxError> {
        let short = Duration::from_millis(HANDSHAKE_RETRY_TIMEOUT_MS);
        let packet = channel.handshake();
        let expected = packet[SEQ_BYTE_OFFSET];

        for attempt in 0..HANDSHAKE_INNER_RETRIES {
            self.handle.write_bulk(EP_OUT, packet, outer_timeout)?;
            match self.read_reply(channel, expected, buf, short) {
                Ok(_) => return Ok(()),
                Err(HxError::Usb(rusb::Error::Timeout))
                    if attempt + 1 < HANDSHAKE_INNER_RETRIES => {}
                Err(e) => return Err(e),
            }
        }
        Err(HxError::Usb(rusb::Error::Timeout))
    }

    /// Suspends the current thread according to the exponential backoff curve.
    ///
    /// Called when entering retry iteration `attempt` (≥ 1), so `attempt`
    /// failures have already happened — that's the value to report.
    fn wait_with_backoff(&self, attempt: u32) {
        let wait_ms = BASE_BACKOFF_MS * (1 << attempt);

        eprintln!(
            "{}",
            fl!(
                "usb-retry-attempt",
                device = self.profile.name,
                current = attempt,
                total = MAX_INIT_RETRIES,
                wait_ms = wait_ms
            )
        );
        std::thread::sleep(Duration::from_millis(wait_ms));
    }
}

impl DeviceClient for Client {
    #[inline]
    fn profile(&self) -> &'static DeviceProfile {
        self.profile
    }

    #[inline]
    fn read_presets(&self) -> Result<Vec<Preset>, HxError> {
        self.read_setlist_presets(0)
    }

    #[inline]
    fn read_setlist_presets(&self, setlist: u8) -> Result<Vec<Preset>, HxError> {
        self.profile.validate_setlist(setlist)?;
        self.with_retries(|timeout| self.run_session(setlist, timeout))
    }

    #[inline]
    fn list_setlists(&self) -> Result<Vec<Setlist>, HxError> {
        self.with_retries(|timeout| self.run_setlists_session(timeout))
    }

    #[inline]
    fn select_preset(&self, setlist: u8, preset: u8) -> Result<(), HxError> {
        self.profile.validate_setlist(setlist)?;
        self.with_retries(|timeout| self.run_select_session(setlist, preset, timeout))
    }

    #[inline]
    fn rename_preset(&self, setlist: u8, preset: u8, name: &str) -> Result<(), HxError> {
        self.profile.validate_setlist(setlist)?;
        self.profile.validate_preset(preset)?;
        validate_preset_name(name)?;
        self.with_retries(|timeout| self.run_rename_session(setlist, preset, name, timeout))
    }

    #[inline]
    fn read_active_preset(&self) -> Result<ActivePreset, HxError> {
        self.profile.ensure_edit_channel()?;
        self.with_retries(|timeout| self.run_active_preset_info_session(timeout))
    }

    #[inline]
    fn read_active_preset_blocks(&self) -> Result<PresetChain, HxError> {
        self.profile.ensure_edit_channel()?;
        let stream = self.with_retries(|timeout| self.run_active_preset_session(timeout))?;
        parse_active_preset(&stream)
    }
}

impl Drop for Client {
    /// Ensures the USB interface is cleanly released back to the OS when the
    /// client is dropped.
    fn drop(&mut self) {
        let _ = self.handle.release_interface(INTERFACE);
    }
}
