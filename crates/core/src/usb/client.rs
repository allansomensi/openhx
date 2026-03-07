use super::{parser::parse_msgpack_stream, protocol::*};
use crate::{
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

    /// Returns a reference to the static metadata profile of the connected device.
    #[inline]
    #[must_use]
    pub fn profile(&self) -> &'static DeviceProfile {
        self.profile
    }

    /// Executes the full proprietary preset-read sequence over the USB bulk endpoints.
    ///
    /// This method includes a retry mechanism with exponential backoff to handle
    /// transient device unresponsiveness (e.g., when the hardware is busy processing UI
    /// events or just finishing its boot sequence). Upon success, the presets are
    /// returned sorted by their hardware index.
    pub fn read_presets(&self) -> Result<Vec<Preset>, HxError> {
        let timeout = Duration::from_millis(TIMEOUT_MS);

        for attempt in 0..MAX_INIT_RETRIES {
            if attempt > 0 {
                self.wait_with_backoff(attempt);
                self.drain_stale_data();
            }

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

    /// Executes a single, complete preset extraction session.
    fn run_session(&self, timeout: Duration) -> Result<Vec<Preset>, HxError> {
        let mut buf = vec![0u8; 512];
        let mut raw_stream: Vec<u8> = Vec::with_capacity(4_096);

        // Session init (5 packets, one ACK each)
        for packet in SESSION_INIT_SEQUENCE {
            self.handle.write_bulk(EP_OUT, packet, timeout)?;
            self.handle.read_bulk(EP_IN, &mut buf, timeout)?;
        }

        // Phase 1: open presets resourced
        self.handle.write_bulk(EP_OUT, OPEN_PRESETS, timeout)?;
        self.handle.read_bulk(EP_IN, &mut buf, timeout)?;

        // Phase 2: start stream (first response carries payload
        self.handle.write_bulk(EP_OUT, OPEN_STREAM, timeout)?;
        let n = self.handle.read_bulk(EP_IN, &mut buf, timeout)?;
        self.collect_payload(&mut raw_stream, &buf, n);

        // Phase 3: paginate until end-of-stream
        let mut seq: u8 = PAGINATION_INITIAL_SEQ;
        let mut offset: u32 = STREAM_INITIAL_OFFSET;

        loop {
            let request = build_pagination_request(seq, offset);
            self.handle.write_bulk(EP_OUT, &request, timeout)?;
            let n = self.handle.read_bulk(EP_IN, &mut buf, timeout)?;
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
    /// This prevents out-of-sync responses if a previous session panicked or
    /// was interrupted before consuming all device output.
    fn drain_stale_data(&self) {
        let timeout = Duration::from_millis(DRAIN_TIMEOUT_MS);
        let mut buf = vec![0u8; 512];
        while self.handle.read_bulk(EP_IN, &mut buf, timeout).is_ok() {}
    }

    /// Suspends the current thread according to the exponential backoff curve.
    fn wait_with_backoff(&self, attempt: u32) {
        let wait_ms = BASE_BACKOFF_MS * (1 << attempt);
        let current_attempt = attempt + 1;

        eprintln!(
            "{}",
            fl!(
                "usb-retry-attempt",
                device = self.profile.name,
                current = current_attempt,
                total = MAX_INIT_RETRIES,
                wait_ms = wait_ms
            )
        );
        std::thread::sleep(Duration::from_millis(wait_ms));
    }
}

impl Drop for Client {
    /// Ensures the USB interface is cleanly released back to the OS when the
    /// client is dropped.
    fn drop(&mut self) {
        let _ = self.handle.release_interface(INTERFACE);
    }
}
