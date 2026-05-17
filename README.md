# DualSense Mapper

Map a PS5 DualSense controller to keyboard keys for use on a laptop. Originally written in Python in May 2025 to let my wife play MapleStory Worlds Artale comfortably on a MacBook; rewritten in Rust to ship as a single Windows executable.

## Implementations

| Folder | Status | Audience |
|---|---|---|
| `legacy-python/` | Functional, frozen for reference | Developers comfortable with `pip install` |
| `rust/` | Phase 1 (Windows), Phase 2 (macOS) in progress | End users — single `.exe` |

The Rust build is the recommended path. Python is kept for blame history and because it remains usable on macOS until Phase 2 lands.

## Supported hardware

- **DualSense PS5 controller** (`054c:0ce6`) over **Bluetooth**.

Not supported in v2.0:

- DualSense USB transport (deferred to v2.0.1).
- DualSense Edge (`054c:0df2`, deferred to v2.0.1).
- Xbox / 8BitDo / generic XInput pads — v1.2.0 is the last release with the gilrs-based generic-pad path; v2.0.0 moves to a DualSense-specific raw HID reader so the connection state, touchpad, IMU and battery can be read directly from the 78-byte HID report.

## Why this exists

Existing mapper tools dropped key-release events under load, leaving keys "stuck." The Python prototype solved this with a three-layer release-on-exit defense and a macro engine with randomized delays so that scripted-feeling input patterns don't get flagged by online games. The Rust rewrite preserves both, fixes two latent bugs from the Python version (trigger idle-value mismatch across platforms; shared-key release collision), and packages it for non-technical users.

See:

- `rust/README.md` — build, run, button reference
- `legacy-python/README.md` — original Python notes
- `docs/superpowers/specs/2026-05-16-rust-rewrite-design.md` — design spec (not committed; local working state)
