# Changelog

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.2] - 2026-05-17

### Fixed

- **D-pad pentagon apex angles are now strictly 45° and gaps are
  uniform.** v1.1.1 picked R_shoulder and half_w independently so the
  apex sides came out at arbitrary slopes and the gap between
  adjacent pentagons varied. The parametrisation now enforces
  `R_shoulder = R_inner + half_w`, which makes the apex side of
  every pentagon lie on a line of slope −1 (or its 90°-rotated
  equivalent), so adjacent pentagons' apex sides are parallel and
  the gap between every pair is uniformly `R_inner * √2`.
- **Stick wedges are trapezoids with 45° sides instead of arcs.**
  The arc-based quarters from v1.1.1 produced visually non-parallel
  boundaries between adjacent quarters (each boundary was a radius,
  which converged to the centre). They now share the same
  parallel-gap geometry as the d-pad pentagons: flat outer base,
  flat inner base, two 45° side edges. Adjacent trapezoids' diagonal
  sides lie on parallel lines so the gap is uniform along the
  whole shared boundary.
- **Unbound wedges show a dashed outline.** v1.1.1 hid the unbound
  hit zones entirely (`stroke: none`); on the four-stick-directions
  case where nothing is bound the user couldn't see there were
  buttons at all. Unbound wedges now carry a thin dashed `--muted`
  stroke so the hit zone is discoverable while still reading as
  "not bound" (no fill).

[1.1.2]: https://github.com/Luotee/dualsense-mac-mapper/releases/tag/v1.1.2

## [1.1.1] - 2026-05-17

### Fixed

- **Status no longer shows "Connected" before any pad is plugged in.**
  v1.1.0 enumerated every gilrs gamepad entry on first poll and
  unconditionally emitted `Connected` for each — but some Windows
  drivers leave stale phantom entries in `gilrs.gamepads()` from
  previous sessions. The startup scan now filters on
  `Gamepad::is_connected()` so only real, physically-attached pads
  trigger the connected status.
- **Stick wedge outlines.** A stick with all four directions bound to
  the same colour used to render as a single ring of solid colour in
  v1.1.0 — `.wedge { stroke: none; fill-opacity: 0.4 }` left no
  visual separator between adjacent quarters. Each wedge now carries
  a thin stroke in its binding colour (`--accent` for key,
  `--macro` for macro) and the arc span shrinks from 90° to 84° so
  there's a small gap between adjacent quarters. Four bound
  directions read as four separate buttons.

### Changed

- **D-pad: four label pentagons, apex inward.** v1.1.0's outward-
  arrow pentagons + underlying cross sprite read as one combined
  glyph. The new design drops the cross sprite entirely and uses
  inward-apex label shapes (flat outer base, tapered tip pointing
  toward the d-pad centre), sized closer to the face-button cluster.
  Reads as four independent targets and the press-ring animation
  flashes one label outline per direction.
- **L2 / R2 / L1 / R1 evenly spaced.** v1.1.0 had a 6 px gap between
  the trigger row and the shoulder row but only 2 px between
  shoulders and the body. Triggers move down by 4 px (ry 10 → 14)
  so trigger-shoulder, shoulder-body, and body-top edges are
  uniformly 2 px apart.

[1.1.1]: https://github.com/Luotee/dualsense-mac-mapper/releases/tag/v1.1.1

## [1.1.0] - 2026-05-17

### Changed

- **Palette: Gruvbox Dark.** `rust/web/solarized.css` is renamed to
  `palette.css` and every CSS variable is swapped to Gruvbox Dark
  hex values (bg `#282828`, card `#3c3836`, accent `#83a598`, macro
  `#fe8019`, success `#b8bb26`, …). All UI surfaces — toolbar,
  tabs, chip rows, bind popup, macro editor, settings, activity
  drawer, controller fill / hit zones — pick the new colours up
  automatically through the existing `var(--…)` references. ICO
  assets are regenerated from the same source palette so the app
  icon and tray icons stay in lockstep.
- **D-pad hit zones are pentagon arrows.** Each direction (Up /
  Down / Left / Right) now has its own outward-pointing pentagon
  outline sized to its arm of the cross sprite, instead of the
  v1.0.x shared triangle wedge. The press-ring animation follows
  the same arrow silhouette on physical press, so each direction
  flashes its own shape — matching the face-button per-direction
  behaviour the user expected.
- **Stick hit zones are donut quarters.** Each of the 4 virtual
  stick directions (Up / Down / Left / Right) is now a quarter
  arc of an annulus around the stick well, instead of a triangle
  pointing into the centre. The L3 / R3 inner circle (id 7 / 8)
  stays concentric on top, so each stick has five distinct,
  individually-flashing hit zones (4 quarters + 1 centre).
- **L2 / R2 match L1 / R1 size.** Triggers used to be 26×11 over
  the body's top edge — visibly chunkier than the 22×6 shoulders.
  Both pairs are now 22×6 with L2 / R2 sitting six pixels above
  L1 / R1, producing a balanced top edge.

### Build

- `scripts/build_icons.py` palette constants point at the new
  Gruvbox values; existing `python3 scripts/build_icons.py` flow
  is unchanged.
- New `scripts/palette_mockup.py` renders the GUI under several
  palettes side-by-side; used during brainstorming before this
  release picked Gruvbox Dark.

[1.1.0]: https://github.com/Luotee/dualsense-mac-mapper/releases/tag/v1.1.0

## [1.0.6] - 2026-05-17

### Added

- **Bind any OEM punctuation key** (`-` `=` `[` `]` `\` `;` `'` `,`
  `.` `/` backtick). The frontend capture box used to reject anything
  outside letters / digits / named keys; the backend already
  routed punctuation through `Key::Unicode`, which on Windows
  inserts a character but does not register as a held key. The
  punctuation chars now resolve to the matching `VK_OEM_*` virtual
  key on Windows (US layout) so games hooked at the virtual-key
  level see them as real held keys.
- **Left / right modifier names**: `LShift`, `RShift`, `LControl`
  (alias `LCtrl`), `RControl` (alias `RCtrl`), `LAlt`, `RAlt`. Bind
  them by pressing the left or right modifier in the capture box;
  the frontend distinguishes via `KeyboardEvent.code`. Generic
  `Shift` / `Control` / `Alt` names still work for binders that
  don't care about the side.

### Build

- `_keyboard_keys` cheat-sheet in the bundled
  `dualsense-mapper.json` now lists the new `modifiers_lr` and
  `punctuation` sections so a user editing the file in Notepad
  sees the full set of valid names inline.
- Two new `parse_key` tests cover punctuation round-trip and the
  six L/R modifier aliases. Test count: 49 → 51.

[1.0.6]: https://github.com/Luotee/dualsense-mac-mapper/releases/tag/v1.0.6

## [1.0.5] - 2026-05-17

### Changed

- **Icon assets fill the canvas.** v1.0.4's ICOs left big empty bands
  above and below the controller because the source SVG viewBox is
  240×130 (wide-aspect 1.85) and was being fitted by width into the
  square ICO. `scripts/build_icons.py` now tight-crops the rendered
  silhouette's alpha bounding box and rescales the result to fill
  ~94% of the target square, so the controller occupies the icon at
  every resolution — same trick common Windows app icons use.
- **Drop "Esc cancels" hint from the bind popup's key-capture box.**
  Esc didn't actually cancel in v1.0.2..v1.0.4 (the popup-root
  handler doesn't fire from inside the focused capture box on every
  WebView2 build), and the user has Unbound as an explicit cancel
  path anyway. The hint text was misleading — removed for now until
  Esc handling is repaired.

[1.0.5]: https://github.com/Luotee/dualsense-mac-mapper/releases/tag/v1.0.5

## [1.0.4] - 2026-05-17

### Changed

- **Icon redesign — matches the in-app SVG controller.** All three
  ICO assets (`rust/icons/icon.ico`, `tray-connected.ico`,
  `tray-disconnected.ico`) are now generated from the same geometry
  as `rust/web/controller.js`: body silhouette, touchpad notch,
  d-pad cross, four face-button dots, two stick wells, and the PS /
  Share / Options markers cut out as negative space. L1 / R1 / L2 /
  R2 (the parts that protrude above the body) are dropped from the
  icon — they were noise at icon resolution. Each ICO carries
  hand-tuned 16 / 32 / 48 / 256 layers (16 is a pure silhouette;
  32 keeps stick wells + d-pad; 48 adds face buttons + touchpad;
  256 has the full detail set). Solarized palette: accent blue for
  the app icon, success green when a controller is connected,
  muted grey when disconnected.

### Build

- New `scripts/build_icons.py` generator that produces all three
  ICOs from the same parametrised silhouette. PIL only (no SVG
  renderer dependency); each ICO is assembled as a manual
  multi-resolution container so the smaller layers stay
  hand-tuned instead of resampled from the 256 master.

[1.0.4]: https://github.com/Luotee/dualsense-mac-mapper/releases/tag/v1.0.4

## [1.0.3] - 2026-05-16

### Changed

- **Closing the window (✕) now fully exits the process.** v1.0.0–v1.0.2
  followed the original spec §10 design where ✕ hid the window and the
  mapper kept running in the tray; users reported the process
  lingering in Task Manager and hitting unexpected behaviour because
  Windows convention is "✕ closes the app." The window close handler
  now calls `app.exit(0)` directly — `engine.shutdown()` still runs on
  the way out, so held keys release cleanly (Iron rule #3). The tray's
  `Quit` entry is unchanged; it becomes a convenience duplicate of ✕
  rather than the only exit path. Users who want background mapping
  while the window is hidden should minimise to the taskbar instead.

[1.0.3]: https://github.com/Luotee/dualsense-mac-mapper/releases/tag/v1.0.3

## [1.0.2] - 2026-05-16

### Fixed

- **GUI press-ring lights up for stick directions and analog triggers.**
  In v1.0.0 / v1.0.1, pushing the L-stick / R-stick past the deadzone
  (ids 15–22) and pulling L2 / R2 past the trigger threshold (ids
  23–24) correctly synthesised the bound keystroke, but the SVG hit
  zone on the controller never highlighted. Root cause: those virtual
  presses were happening inside `Mapper::transition_virtual`, while the
  Engine-to-GUI event bridge only forwarded real gilrs `ButtonPressed`
  events. The mapper now buffers each virtual flip and the engine
  drains it via `Mapper::take_visual_transitions`, re-emitting
  `EngineEvent::ButtonDown` / `ButtonUp` so the existing frontend
  press-ring path lights up exactly the same way it does for physical
  face buttons.
- **D-pad and stick wedges now tint with their binding colour.** The
  triangular hit zones that overlay the d-pad cross and stick wells
  carried a `hit-invisible` class so the sprite beneath stayed
  readable, but that hid the binding state entirely. They now carry
  the `binding-key` / `binding-macro` class with a `fill-opacity: 0.4`
  modifier so a bound direction tints visibly while still letting the
  cross / well sprite read through. Unbound wedges stay fully
  transparent — same as v1.0.0.
- **Bind popup key capture stays active after the first keystroke.**
  Previously each capture required clicking the capture box, pressing
  one key, then clicking again to change it. The capture box now
  auto-focuses when the Key segment opens, the listener stays attached,
  and each subsequent keypress overwrites the captured value in place.
  Escape still cancels via the popup-root handler.

### Changed

- **Default `config.example.json` ships a MapleStory-friendly profile.**
  Cross → `Alt`, Circle → `z`, Square → `Shift`, Triangle → `a`,
  R2 → `Shift`, PS → `Space`, Options → `Enter`, D-pad and L-stick
  → arrow keys, R-stick / L1 / R1 / L2 / L3 / R3 / Share unbound by
  default. The sample `macro_A` definition stays in the `macros`
  section so users can see the macro schema even though it isn't
  bound out of the box. `examples/maple_artale.json` is kept in sync
  with the default for parity.

[1.0.2]: https://github.com/Luotee/dualsense-mac-mapper/releases/tag/v1.0.2

## [1.0.1] - 2026-05-16

Cosmetic patch on top of v1.0.0. No code or behaviour change.

### Fixed

- Default `dualsense-mapper.json` written on first run: drop the
  misleading "觸發巨集" suffix from the L2 (id 23) label. The
  suffix described the default *binding* (a macro) rather than the
  button itself, so anyone who later remapped L2 saw a stale label.
  Existing users with their own config are unaffected.

[1.0.1]: https://github.com/Luotee/dualsense-mac-mapper/releases/tag/v1.0.1

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
