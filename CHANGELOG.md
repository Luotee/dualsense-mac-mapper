# Changelog

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-05-16

First GUI release. Version jumps from `0.1.x` straight to `1.0.0` to
mark the project as feature-complete for Phase 1 (Windows GUI mapper);
the underlying changeset is the same one previously tracked as `0.2.0`
during development. The exe now opens a real window with a controller
diagram, click-to-capture remap, step-list macro editor, Solarized Light
theme, and a tray-resident background mapper. The v0.1.x console flow
stays available via the new `--cli` flag.

### Added

- **Tauri 2.x GUI shell** (`rust/src/gui/`). Window opens within ~1 s of
  double-click; close-on-X hides to tray, Quit on the tray menu is the
  only way to exit the process.
- **System tray** with two icon states (connected / disconnected) and a
  3-item menu (Open / Pause mapper / Quit). Tray icon swaps green ↔ grey
  on controller connect / disconnect.
- **Mappings tab** with a full SVG DualSense diagram, all 25 hit zones
  (face buttons + D-pad + L1/R1 + L2/R2 + L3/R3 + stick virtual
  directions). Live highlight on physical press; click any button to
  open a bind popup with Key / Macro / Unbound segmented control.
- **Click-to-capture** key binding: in Key mode, the popup shows
  "Press the key to bind…", normalises the next `KeyboardEvent` via the
  spec §7.4 table, and writes it. No more typing key names by hand.
- **Macros tab** with a left-pane macro list (showing which buttons each
  macro is bound to) and a right-pane step-list editor. `+ Step`, `+
  Quick tap…` (sugar for down+up pair), drag-to-reorder, Loop toggle,
  inline `min < max` validation, Save / Discard. Rename / Duplicate /
  Delete via right-click; Delete blocked-with-confirmation when the
  macro is bound to any button.
- **Settings tab** with the 5 top-level config fields (deadzone,
  trigger_threshold, min_press_ms[min,max], tick_jitter_ms[min,max],
  log_events), a "Reset to defaults" button, and an "Open config file
  in editor" button (Notepad on Windows, `open -t` on macOS Phase 2).
- **Activity log drawer** (📊 toggle in the toolbar): live stream of
  gamepad events + synthesised key emits + macro lifecycle. Throttled
  to ≤1 paint per `requestAnimationFrame`, capped at 200 DOM rows.
  Drawer-open state persists across restarts in `dualsense-mapper.ui.json`.
- **Solarized Light theme** pinned to Ethan Schoonover's spec —
  10 CSS variables drive every UI element, no invented colours.
- **File watcher** (`notify` crate) for external edits to the config:
  user opens `dualsense-mapper.json` in Notepad, saves, and the live
  engine hot-rebinds within 250 ms. Validation failure surfaces the
  Rust error verbatim instead of silently dropping the change.
- **`Engine` + `Handle`** abstraction (`rust/src/engine.rs`): the v0.1.x
  blocking mapper loop is now wrapped in a thread-safe handle that the
  GUI mutates while it runs. Atomic flags for pause + capture-active +
  shutdown; `RwLock<Config>` for hot rebinding; channel of
  `EngineEvent`s for the GUI bridge.
- **`ConfigDoc`** raw-JSON-preserving reader/writer
  (`rust/src/config_io.rs`): GUI writes through it so the `_help` and
  `_keyboard_keys` inline cheat sheet (and any other `_*` doc fields)
  round-trip byte-for-byte. Atomic write via `*.tmp` + rename.
- **Iron rule #9 (new)**: the GUI is a chrome layer. No mapping
  decision, no key synth, no macro scheduling lives in JavaScript.
  Every runtime-state mutation routes through `#[tauri::command]` in
  `rust/src/gui/commands.rs`. JS that calls `SendInput` is a rejection.
- **`--cli` flag** in `main.rs`: opt-in to the v0.1.x console mode.
  Useful for `--validate`, `--list-buttons`, headless dry-runs, and CI.

### Changed

- **Iron rule #8 reframed** for the GUI-first world: window must be
  visible within ~1 s of double-click; CLI mode is the explicit
  opt-in legacy path (rule still applies there). See `CLAUDE.md`.
- **Default mode is GUI, not CLI**. v0.1.x users who scripted the exe
  with no flags will need to add `--cli` to keep the previous
  behaviour. The 1-line migration is documented in `rust/README.md`.

### Fixed

- **Iron rule #3 panic hook actually works now.** v0.1.x installed a
  panic hook that captured a freshly-allocated `safety::shared()` Arc
  — not the live engine's Arc. On panic, the hook drained an empty
  map, leaving real held keys stuck at the OS level. Fixed by routing
  through a `OnceLock<SharedKeyState>` that the engine binds via
  `safety::register_global` after spawn. The hook now drains the
  actual engine state. Latent in v0.1.0 and v0.1.1.

### Build

- New cargo feature `gui` gates the Tauri dependency tree so
  `cargo test` and `cargo build` on a Linux dev host without
  webkit2gtk dev libs still work. Production builds use
  `cargo build --release --target x86_64-pc-windows-gnu --features gui`.

[1.0.0]: https://github.com/Luotee/dualsense-mac-mapper/releases/tag/v1.0.0

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
