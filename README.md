# 🎛️ OpenHX

> An unofficial, open-source alternative to HX Edit — built in Rust, runs everywhere.

OpenHX is a community-driven tool for managing Line 6 HX series devices (HX Stomp, HX Stomp XL, and Helix family) from any operating system. It ships as both a **command-line interface** and a **graphical interface**, with no dependency on HX Edit or any official Line 6 software.

---

## 🎯 Motivation

HX Edit — the official companion software for Line 6 HX devices — is only supported on Windows and macOS. Linux users, BSD users, and anyone running an unsupported OS are left without a way to manage their presets, update settings, or interact with their hardware programmatically.

OpenHX was built to close that gap.

---

## ✨ Features

> ⚠️ OpenHX is in early development. Features marked 🚧 are planned but not yet implemented.

- ✅ List all 128 preset names from the device
- 🚧 Rename presets
- 🚧 Import and export presets (`.hlx` files)
- 🚧 Trigger footswitches programmatically
- 🚧 Full GUI with preset browser and editor
- 🚧 Support for multiple HX devices

---

## ✅ Supported Devices

| Device | Status |
|---|---|
| HX Stomp | 🚧 Planned |
| HX Stomp XL | ✅ Validated |
| HX Effects | 🚧 Planned |
| Helix Floor | 🚧 Planned |
| Helix LT | 🚧 Planned |

---

## 📥 Installation

> Pre-built binaries are not yet available. Build from source for now.

**Prerequisites:** Rust toolchain (`rustup`), `libusb` 1.0+.

```bash
git clone https://github.com/allansomensi/openhx
cd openhx
cargo build --release
```

The binary will be at `target/release/openhx`.

---

## 💻 Usage

### CLI

```bash
# List all presets
openhx-cli list-presets
```

### GUI

```bash
openhx
```

---

## 🔬 How It Works

OpenHX communicates with HX devices over **USB bulk transfers** using the same vendor-specific protocol as HX Edit.

The protocol was reverse-engineered entirely **black-box** — by capturing and analyzing USB traffic between an HX Stomp XL and the official HX Edit software. No Line 6 source code, firmware, or proprietary SDKs were accessed or used at any point. The reverse engineering was done **strictly for the purpose of interoperability** with a platform not officially supported by Line 6.

The full protocol is documented in [`docs/protocol/`](./docs/protocol/README.md).

---

## 🤝 Contributing

Contributions are very welcome, especially:

- Protocol research for additional devices or operations
- Testing on different hardware and operating systems
- GUI development
- Documentation improvements

Please open an issue before starting work on a large feature so we can coordinate.

---

## ⚖️ Legal

**OpenHX is an unofficial, community-driven, open-source project. It is not affiliated with, endorsed by, sponsored by, or associated with Line 6 or Yamaha Guitar Group, Inc. in any way.**

"Line 6", "HX Stomp", "HX Effects", "Helix", and "HX Edit" are trademarks of Yamaha Guitar Group, Inc. All trademarks are the property of their respective owners and are used here strictly for descriptive and nominative purposes.

The reverse engineering performed in this project was conducted entirely black-box — by observing USB communication between the device and the official software — and is intended solely to enable interoperability on platforms not officially supported. No proprietary code, firmware, or trade secrets were accessed or incorporated.

OpenHX is released under the [MIT License](./LICENSE).
