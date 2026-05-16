# Changelog

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-05-16

End-user double-click UX pass — Windows users open the exe by double
clicking, not from a terminal. v0.1.0 first-run flow exited with code 1
which closed the console window before they could read anything.

### Changed

- **First-run no longer exits.** When the bundled default
  `dualsense-mapper.json` is written next to the exe, the program keeps
  running with that default. The user can edit the file later and
  restart to customize. (Previous behaviour: write default + exit code
  1, which made the console window vanish for double-click users.)
- **Errors pause for "Press Enter to close".** Any uncaught error from
  `main` prints the chain, then waits on stdin, so the console window
  stays visible long enough to read what went wrong. `--no-pause` flag
  added for CLI / CI users who want immediate exit.
- **Startup banner.** On normal start the exe prints program name,
  version, config path, and "Press Ctrl-C or close window to quit".
  First-run users also get a "Wrote default config — edit it in
  Notepad" note.
- First-run-written `dualsense-mapper.json` now embeds an inline
  keyboard cheat sheet (`_help` + `_keyboard_keys` fields) so end users
  discover valid key names directly in the file they are editing,
  without having to consult the README. Both fields start with `_` and
  are silently ignored by the config loader (serde drops unknown keys),
  so the file validates and round-trips normally.

### Added

- `--no-pause` CLI flag.

### Removed

- GitHub Release no longer bundles a separate `config.example.json`.
  The exe writes the same content on first run, so shipping both was
  redundant.

## [0.1.0] - 2026-05-16

First Rust rewrite ship. Single-binary Windows `.exe` portable bundle.
Legacy Python (`legacy-python/`) remains in repo as frozen reference.

### Added

- Rust crate at `rust/` producing a single `dualsense-mapper(.exe)` binary.
- JSON config schema (`config.example.json` shipped alongside binary):
  - Every button id `0..=24` must be present; `"type": "unbound"` for unused.
  - Bindings: `key` (single key by name), `macro` (named macro), `unbound`.
  - Macros: ordered steps with `[min, max]` random delays.
- Three discoverability layers for button ids:
  1. Exhaustive `config.example.json` listing all 25 ids with labels.
  2. README cheat-sheet table.
  3. `--list-buttons` CLI live readout (authoritative for the current OS / driver).
- CLI flags: `--config PATH`, `--validate`, `--dry-run`, `--list-buttons`, `--verbose`.
- Default config path: **next to the executable** as `dualsense-mapper.json`.
  Portable — copy the folder to a USB stick, runs anywhere.
- Stuck-key prevention (four layers):
  - Refcounted key state in `safety.rs`.
  - `Drop` on `KeyboardSink` releases everything held.
  - Panic hook releases keys before unwind exits the process.
  - Ctrl-C handler drains the loop cleanly.
- Anti-cheat self-discipline (carries the Python POC's intent forward):
  - `min_press_ms` floor enforces a randomized minimum KEYDOWN→KEYUP gap on
    every synthesized press, so transient bot-shaped patterns get smoothed.
  - `tick_jitter_ms` adds ±jitter when multiple keys fire on the same tick.
  - Macro step delays are always `[min, max]` ranges; constant delays are
    rejected by the config validator.
- Macro engine on dedicated `std::thread`s with cancellable `AtomicBool` flag.
  Cancellation or natural exit **drains every unmatched Press** as a Release
  before the thread returns, so mid-macro release of the source button can
  never strand a KEYDOWN at the OS level.
- D-pad hat-axis handling — gilrs reports many controllers' D-pad as
  `Axis::DPadX` / `Axis::DPadY` (-1 / 0 / +1) instead of discrete
  `DPadLeft/Right/Up/Down` buttons. `gamepad.rs` watches both paths and
  synthesises `ButtonDown/Up(11..=14)` from axis crossings so the mapper
  sees a single, uniform event surface.
- Trigger normalization detects both `[-1, 1]` (Linux gilrs) and `[0, 1]`
  (Windows XInput) conventions by sign, so idle-trigger value never sits
  on the activation threshold.
- Verbose pipeline tracing — every gilrs event, mapper decision, and
  enigo emit logs at `info` level; debug fills in dropped events and
  parse details. `--verbose 2> log.txt` is the canonical bug-report dump.
- 33 unit + integration tests covering schema, validation, mapper,
  refcount, macro drain on cancel, shipped configs.

### Cross-platform notes

- Build target: `x86_64-pc-windows-gnu` from a Linux dev host requires
  `mingw-w64`. Linux test/dev build requires `pkg-config + libudev-dev`
  (gilrs's Linux backend).
- macOS support is Phase 2 (uses `enigo` 0.6's `CGEvent` backend, no
  code change expected in `keyboard.rs`).

[0.1.1]: https://github.com/Luotee/dualsense-mac-mapper/releases/tag/v0.1.1
[0.1.0]: https://github.com/Luotee/dualsense-mac-mapper/releases/tag/v0.1.0
