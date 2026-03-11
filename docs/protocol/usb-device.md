# USB Device Identification

This document covers USB-level identification for Line 6 HX series devices.

---

## HX Stomp XL

| Property | Value |
|---|---|
| Vendor ID | `0x0E41` |
| Product ID | `0x4253` |
| Configuration | `1` |
| Interface | `0` (Vendor-Specific) |
| Endpoint OUT | `0x01` (Bulk, Host → Device) |
| Endpoint IN | `0x81` (Bulk, Device → Host) |

---

## Opening the Device

Use standard USB enumeration to open the device by VID/PID, then:

1. Set configuration `1`.
2. Claim interface `0`.
3. Clear any halted endpoints on `0x01` and `0x81`.

Only after these steps should any packets be sent. See [transport.md](./transport.md) for the stale data drain that must happen before the first write.

---

## Notes

- The interface is **vendor-specific** — no standard USB class driver applies.
- The device exposes a single bulk OUT and a single bulk IN endpoint for all application-level communication.
- On Linux, `libusb` may require a kernel driver detach (`libusb_detach_kernel_driver`) if a generic driver has claimed the interface.
