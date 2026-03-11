# Contributing to OpenHX

Thanks for your interest in contributing. OpenHX is a community project and any help is genuinely appreciated — whether that's code, protocol research, testing on hardware I don't have, or just improving the docs.

---

## Ways to Contribute

- **Protocol research** — Capturing USB traffic for unsupported devices or undocumented operations
- **Testing** — Running OpenHX on different hardware, operating systems, or device firmware versions and reporting results
- **Code** — CLI commands, GUI, device abstractions, bug fixes
- **Documentation** — Protocol docs, usage guides, translations

---

## Before You Start

For anything beyond a small fix, **open an issue first**. This avoids duplicate work and allows for alignment on approach before time is invested in a PR.

For protocol research specifically, check [`docs/protocol/`](./docs/protocol/README.md) to see what's already documented.

---

## Development Setup

```bash
git clone https://github.com/allansomensi/openhx
cd openhx
cargo build
```

**Prerequisites:** Rust stable (latest), `libusb` 1.0+. See the [README](./README.md) for platform-specific USB setup.

Run tests:

```bash
cargo test
```

Run Just for format before submitting:

```bash
just lint-fix
```

---

## Protocol Research

If you're capturing USB traffic to document new operations:

- Use Wireshark with the USBPcap plugin (Windows) or `usbmon` (Linux).
- Capture traffic from HX Edit performing the operation you want to document.
- Cross-reference with the existing docs in [`docs/protocol/`](./docs/protocol/README.md) to understand the session structure.
- Open an issue with your findings before writing a PR — raw captures are a great starting point even without a full write-up.

All reverse engineering must remain **strictly black-box**: observing USB communication only. Do not decompile, disassemble, or otherwise reverse engineer any Line 6 binary.

---

## Pull Requests

- Keep PRs focused — one feature or fix per PR.
- Reference the related issue in the PR description.
- Make sure `cargo test` and `just lint-fix` pass.
- Update or add documentation if your change affects behavior or the protocol.

---

## Commit Style

Commits follow the [Conventional Commits](https://www.conventionalcommits.org/) convention. Optionally, semantic emojis can be added.

```
feat: add preset rename command
fix: handle stale session state on reconnect
docs: document chunk request packet layout
chore: update rusb to 0.9
```

---

## Legal

By contributing, you agree that your contributions will be licensed under the [MIT License](./LICENSE).

Do not submit any code derived from Line 6's proprietary software, firmware, or SDKs.
