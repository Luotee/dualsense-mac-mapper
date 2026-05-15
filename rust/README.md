# DualSense Mapper (Rust)

PS5 DualSense → keyboard mapper. Single executable, JSON config, supports macros with randomized delays.

## Status

Phase 1: Windows build. macOS support comes in Phase 2.

## Quick start

1. Plug in a DualSense controller (USB).
2. Run `dualsense-mapper.exe`. First run writes a default `config.json` to
   `%APPDATA%\dualsense-mapper\config.json` and exits.
3. Edit `config.json` — keep all 25 button ids; change `type` / `value` to your taste.
4. Run again. Press buttons; the mapped keys are sent to the focused window.
5. Ctrl-C to quit. Every held key is released on exit, on panic, and on disconnect.

## CLI

| Flag | Purpose |
|---|---|
| `--config PATH` | Use a specific config file. |
| `--validate` | Parse and validate the config, then exit. |
| `--dry-run` | Print every synthesized event instead of typing real keys. Use this before going live in a game. |
| `--list-buttons` | Print every gamepad event with its label. Use this to discover which id each button has on your system. |
| `--verbose` | Debug-level logs. |

## Button id reference

The config requires every id 0..=24 to be present, even if `"type": "unbound"`. This is so the user can see the full surface without guessing. The on-device readout from `--list-buttons` is authoritative — gilrs may map ids differently across USB vs Bluetooth or across OSes.

| Id | Source | Default label |
|---:|---|---|
| 0 | Physical | Cross (X) |
| 1 | Physical | Circle (O) |
| 2 | Physical | Square ([]) |
| 3 | Physical | Triangle (^) |
| 4 | Physical | Share |
| 5 | Physical | PS Button |
| 6 | Physical | Options |
| 7 | Physical | L3 (stick click) |
| 8 | Physical | R3 (stick click) |
| 9 | Physical | L1 |
| 10 | Physical | R1 |
| 11 | Physical | D-pad Up |
| 12 | Physical | D-pad Down |
| 13 | Physical | D-pad Left |
| 14 | Physical | D-pad Right |
| 15 | Virtual (L-stick Y ≤ −deadzone) | L-stick Up |
| 16 | Virtual (L-stick Y ≥ deadzone) | L-stick Down |
| 17 | Virtual (L-stick X ≤ −deadzone) | L-stick Left |
| 18 | Virtual (L-stick X ≥ deadzone) | L-stick Right |
| 19 | Virtual (R-stick Y ≤ −deadzone) | R-stick Up |
| 20 | Virtual (R-stick Y ≥ deadzone) | R-stick Down |
| 21 | Virtual (R-stick X ≤ −deadzone) | R-stick Left |
| 22 | Virtual (R-stick X ≥ deadzone) | R-stick Right |
| 23 | Virtual (L2 normalized ≥ trigger_threshold) | L2 trigger |
| 24 | Virtual (R2 normalized ≥ trigger_threshold) | R2 trigger |

## Anti-cheat self-discipline

This tool is not a cheat — it remaps physical inputs to keyboard inputs. To avoid being misclassified as one, the synthesis pipeline avoids fixed-period patterns:

- Macro step delays are always `[min, max]` ranges, never constants.
- Synthesized keys honor `min_press_ms` (randomized minimum down/up gap).
- Simultaneous events on the same gamepad tick are spread by `tick_jitter_ms`.
- No process hooking, DLL injection, or driver. User-mode `SendInput` (Windows) / `CGEvent` (macOS, Phase 2) only.
- `log_events: true` writes every synthesized event with a timestamp so you can self-audit.

## Build

```
cd rust
cargo build --release
```

Binary lands at `target/release/dualsense-mapper.exe`. Strip + LTO are enabled in release; expect ~3–5 MB.

## Tests

```
cargo test
```
