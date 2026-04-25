use super::{parser::parse_msgpack_stream, protocol::*};
use crate::{
    client::DeviceClient,
    device::{KnownDevice, catalog::DEVICE_CATALOG, profile::DeviceProfile},
    error::HxError,
    models::Preset,
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

    /// Core retry loop for reading presets, extracted to avoid name collision
    /// with the [`DeviceClient`] trait method of the same name.
    fn read_presets_impl(&self) -> Result<Vec<Preset>, HxError> {
        let timeout = Duration::from_millis(TIMEOUT_MS);

        for attempt in 0..MAX_INIT_RETRIES {
            if attempt > 0 {
                self.wait_with_backoff(attempt);
            }
            // Drain runs on every attempt, not just retries: the previous
            // transaction's trailing packets persist in the IN buffer
            // across calls on a cached client.
            self.drain_stale_data();

            match self.run_session(timeout) {
                Ok(mut presets) => {
                    presets.sort_unstable_by_key(|p| p.index);
                    return Ok(presets);
                }
                Err(HxError::Usb(rusb::Error::Timeout)) if attempt + 1 < MAX_INIT_RETRIES => {
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        Err(HxError::protocol(fl!(
            "usb-device-unresponsive",
            device = self.profile.name,
            attempts = MAX_INIT_RETRIES
        )))
    }

    /// Core retry loop for selecting a preset, extracted to avoid name collision
    /// with the [`DeviceClient`] trait method of the same name.
    fn select_preset_impl(&self, bank: u8, preset: u8) -> Result<(), HxError> {
        let timeout = Duration::from_millis(TIMEOUT_MS);

        for attempt in 0..MAX_INIT_RETRIES {
            if attempt > 0 {
                self.wait_with_backoff(attempt);
            }
            self.drain_stale_data();

            match self.run_select_session(bank, preset, timeout) {
                Ok(_) => return Ok(()),
                Err(HxError::Usb(rusb::Error::Timeout)) if attempt + 1 < MAX_INIT_RETRIES => {
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        Err(HxError::protocol(fl!(
            "usb-device-unresponsive",
            device = self.profile.name,
            attempts = MAX_INIT_RETRIES
        )))
    }

    /// Internal routine to handle the session cycle specifically for changing a preset.
    fn run_select_session(&self, bank: u8, preset: u8, timeout: Duration) -> Result<(), HxError> {
        let mut buf = vec![0u8; 512];

        self.session_init(&mut buf, timeout)?;

        // Send the Select Preset command (after handshake, the next seq is 0x06)
        let seq = 0x06;
        let request = build_select_preset_request(seq, bank, preset);

        self.handle.write_bulk(EP_OUT, &request, timeout)?;
        self.read_for_seq(seq, &mut buf, timeout)?;

        Ok(())
    }

    /// Executes a single, complete preset extraction session.
    fn run_session(&self, timeout: Duration) -> Result<Vec<Preset>, HxError> {
        let mut buf = vec![0u8; 512];
        let mut raw_stream: Vec<u8> = Vec::with_capacity(4_096);

        self.session_init(&mut buf, timeout)?;

        // Phase 1: open presets resource
        self.handle.write_bulk(EP_OUT, OPEN_PRESETS, timeout)?;
        self.read_for_seq(OPEN_PRESETS[SEQ_BYTE_OFFSET], &mut buf, timeout)?;

        // Phase 2: start stream (first response carries payload)
        self.handle.write_bulk(EP_OUT, OPEN_STREAM, timeout)?;
        let n = self.read_for_seq(OPEN_STREAM[SEQ_BYTE_OFFSET], &mut buf, timeout)?;
        self.collect_payload(&mut raw_stream, &buf, n);

        // Phase 3: paginate until end-of-stream
        let mut seq: u8 = PAGINATION_INITIAL_SEQ;
        let mut offset: u32 = STREAM_INITIAL_OFFSET;

        loop {
            let request = build_pagination_request(seq, offset);
            self.handle.write_bulk(EP_OUT, &request, timeout)?;
            let n = self.read_for_seq(seq, &mut buf, timeout)?;
            self.collect_payload(&mut raw_stream, &buf, n);

            // Phase 4: a response shorter than MAX_CHUNK_SIZE signals
            // end-of-stream.
            if n < MAX_CHUNK_SIZE {
                break;
            }

            offset = offset
                .checked_add(OFFSET_STEP)
                .ok_or_else(|| HxError::protocol(fl!("usb-stream-offset-overflow")))?;
            seq = seq.wrapping_add(1);
        }

        parse_msgpack_stream(&raw_stream, self.profile.preset_count)
    }

    /// Appends the MessagePack payload portion of a bulk IN response to `dest`.
    ///
    /// Each response is prefixed by [`RESPONSE_HEADER_SIZE`] bytes that are
    /// not part of the MessagePack stream and must be skipped.
    #[inline]
    fn collect_payload(&self, dest: &mut Vec<u8>, buf: &[u8], n: usize) {
        if n > RESPONSE_HEADER_SIZE {
            dest.extend_from_slice(&buf[RESPONSE_HEADER_SIZE..n]);
        }
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
    /// within ~50 ms of the last packet; [`Self::read_for_seq`] guards
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

    /// Reads from the IN endpoint until the response's sequence byte
    /// matches `expected`, discarding stale or out-of-band packets along
    /// the way.
    ///
    /// Out-of-band packets that arrive between transactions can land in
    /// the IN buffer ahead of the ACK and shift every subsequent read by
    /// one. Validating the sequence byte at [`SEQ_BYTE_OFFSET`] lets the
    /// caller discard such packets inline as a safety net behind
    /// [`Self::drain_stale_data`]. Each re-read uses the caller's full
    /// timeout: a genuine reply queued behind a stale packet returns
    /// immediately from the OS buffer regardless, and a shorter window
    /// would only risk misclassifying a slow-but-valid read as stale.
    fn read_for_seq(
        &self,
        expected: u8,
        buf: &mut [u8],
        timeout: Duration,
    ) -> Result<usize, HxError> {
        for drops in 0..=MAX_STALE_DROPS {
            let n = self.handle.read_bulk(EP_IN, buf, timeout)?;
            if n > SEQ_BYTE_OFFSET && buf[SEQ_BYTE_OFFSET] == expected {
                return Ok(n);
            }
            tracing::debug!(
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
    /// standard write-then-[`Self::read_for_seq`] path.
    fn session_init(&self, buf: &mut [u8], timeout: Duration) -> Result<(), HxError> {
        self.handshake(buf, timeout)?;
        for packet in &SESSION_INIT_SEQUENCE[1..] {
            self.handle.write_bulk(EP_OUT, packet, timeout)?;
            self.read_for_seq(packet[SEQ_BYTE_OFFSET], buf, timeout)?;
        }
        Ok(())
    }

    /// Performs the channel handshake, retrying internally on timeout.
    ///
    /// The first [`HANDSHAKE`] write of a session is occasionally dropped
    /// by the device while it finishes settling from the previous session.
    /// A short-timeout resend handled here recovers in roughly
    /// [`HANDSHAKE_RETRY_TIMEOUT_MS`] milliseconds, avoiding the outer
    /// retry's full read timeout plus exponential back-off per occurrence.
    fn handshake(&self, buf: &mut [u8], outer_timeout: Duration) -> Result<(), HxError> {
        let short = Duration::from_millis(HANDSHAKE_RETRY_TIMEOUT_MS);
        let expected = HANDSHAKE[SEQ_BYTE_OFFSET];

        for attempt in 0..HANDSHAKE_INNER_RETRIES {
            self.handle.write_bulk(EP_OUT, HANDSHAKE, outer_timeout)?;
            match self.read_for_seq(expected, buf, short) {
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
        self.read_presets_impl()
    }

    #[inline]
    fn select_preset(&self, bank: u8, preset: u8) -> Result<(), HxError> {
        self.select_preset_impl(bank, preset)
    }
}

impl Drop for Client {
    /// Ensures the USB interface is cleanly released back to the OS when the
    /// client is dropped.
    fn drop(&mut self) {
        let _ = self.handle.release_interface(INTERFACE);
    }
}
