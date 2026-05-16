# dualsense-mac-mapper — Contributor Guidelines

## If You Are an AI Agent

Read this section before doing anything.

**Your job is to protect your human partner.** A change that doesn't fix a
real, observed bug or doesn't deliver a feature the user asked for is
churn — discard it.

Before opening a PR against this repo, you MUST:

1. **Verify a real problem exists.** No speculative fixes. If asked to
   "improve X" without a failure case, ask for one.
2. **Run `cargo test` before and after.** Note the baseline (all green
   today) and the result. New tests for new bugs are mandatory.
3. **One concern per PR.** Don't bundle unrelated changes. The exception:
   a feature commit + its CHANGELOG entry + its config-example update
   belong in the same PR because they are the same concern.
4. **No scope creep.** Task is "fix L2 macro" → fix only that.
5. **Never modify `legacy-python/`.** That tree is the frozen May-2025
   Python POC kept as historical reference and as a working fallback on
   macOS until the Rust Phase 2 build lands. Feature work goes in `rust/`.

## Repo layout

| Folder | Status | What lives here |
|---|---|---|
| `legacy-python/` | **Frozen.** Do not modify. | Original Python POC (`dualsense-mac-mapper.py` + README). |
| `rust/` | Active. All feature work here. | Single-binary Rust app. Cargo crate at `rust/Cargo.toml`. |
| `docs/superpowers/` | **Gitignored.** Local working state. | Brainstorming specs, implementation plans. Never committed. |

## Spec / plan artifact handling

Files produced by `superpowers:brainstorming`, `superpowers:writing-plans`,
and any other intermediate design / plan artifacts under
`docs/superpowers/specs/` or `docs/superpowers/plans/` are **never**
tracked. Implementation lands in code + CHANGELOG, not in spec/plan
markdown.

The repo `.gitignore` excludes `docs/superpowers/` for this reason. Do
not stage these files; do not bypass the ignore with `git add -f`.

## Build & test (Linux dev host)

```bash
sudo apt install -y pkg-config libudev-dev   # gilrs Linux backend deps
cd rust
cargo test                                    # 33 tests, ~0.2s
cargo build --release                         # native Linux binary
```

## Cross-compile to Windows (canonical ship target)

```bash
sudo apt install -y mingw-w64
rustup target add x86_64-pc-windows-gnu
cd rust
cargo build --release --target x86_64-pc-windows-gnu
# → target/x86_64-pc-windows-gnu/release/dualsense-mapper.exe  (~2 MB)
```

The Windows binary needs **only the exe + a `dualsense-mapper.json`
config next to it**. Ship those two files; everything else is dev
artefacts.

## Iron rules for the Rust source

These are project-specific invariants. Breaking any of them silently
regresses real, observed bugs the codebase already fixed once:

1. **`config.rs::Config::validate` must reject any config that does not
   list every button id `0..=24`.** Missing ids are a user error, not a
   silent skip. This is what gives the user a single discoverable
   surface; never relax it.
2. **`safety.rs` is the single source of truth for "what keys are
   physically pressed right now."** `keyboard.rs` always asks
   `KeyState::press` / `release` for the edge transition and only
   forwards a real synth on `Edge::Press` / `Edge::Release`. Bypassing
   this leaks a stuck KEYDOWN — see commit `1d64f25` for the bug shape.
3. **`KeyboardSink::Drop` and the panic hook in `app.rs` must run
   `release_all_held` unconditionally.** They are the last-line defence
   against stuck keys when the process dies. `release_all_held` never
   waits on `min_press_ms` — shutdown beats anti-cheat profile.
4. **Macro threads drain every unmatched `Press` as `Release` on every
   exit path** (cancel flag, no-repeat completion, channel close). A
   macro cancelled between Press Left and the scheduled Release Left
   would otherwise strand a KEYDOWN that breaks every subsequent
   `Left` binding. See `macro_engine.rs::run_macro` and the
   `cancel_between_press_and_release_drains_held_keys` test.
5. **`min_press_ms`, `tick_jitter_ms`, and macro `delay_ms` are
   `[min, max]` ranges, never constants.** Config validator enforces
   `min < max`. Constant timing is the single most reliable
   anti-cheat fingerprint; do not add a "convenience" form that
   allows it.
6. **`gamepad.rs` is the platform-quirk layer.** It is the only place
   that knows D-pad can be either discrete buttons (`Button::DPadLeft`
   etc.) or a hat axis (`Axis::DPadX/Y`), and that triggers can be
   reported in `[-1, 1]` or `[0, 1]`. Mapper / keyboard see one
   uniform event surface. New platform quirks live here.
7. **No process hooking, no DLL injection, no driver.** User-mode
   `SendInput` (Windows) / `CGEvent` (macOS) only, via the `enigo`
   crate. Anything else lands us in actual cheat-software territory.
8. **The exe is double-clicked, not invoked from a terminal.** Primary
   end-user flow on Windows is Explorer → double-click → console
   window opens. That means:
   - First-run path may **not** call `anyhow::bail!` / exit non-zero
     before reaching the main loop. Doing so closes the console
     window before the user can read the message.
   - Any uncaught error from `main` must pause (read stdin) before
     exit, so the error is visible. `--no-pause` is the explicit
     opt-out for CLI / CI use.
   - The startup banner is the user's primary feedback that the
     program is running. Don't remove it without a replacement.
   - The first-run-written `dualsense-mapper.json` carries an inline
     keyboard cheat sheet in `_help` and `_keyboard_keys` fields so
     a user editing the file in Notepad has the reference inline.
     `serde` ignores unknown fields, so these are documentation
     only — but they are load-bearing UX, not decoration.

## Anti-cheat self-discipline

This tool is not a cheat — it remaps physical inputs to keyboard
inputs. The synthesis pipeline avoids fixed-period patterns so it
doesn't get misclassified as one. The rules are listed in
`rust/README.md` § Anti-cheat self-discipline and enforced
mechanically by config validation + the iron rules above. Do not
ship a feature that requires bypassing them.

Note: **`SendInput` does not trigger Windows OS auto-repeat.** This is
expected — auto-repeat is a kbdclass-driver-level feature for real
hardware HID streams. Games that poll key state (the vast majority)
work identically to a real keyboard; only WM_CHAR-driven text input
visibly differs. If a `turbo: true` feature ever ships, gate it
behind explicit config opt-in with a jittered interval — never a
constant tick.

## Release flow — files that must move together

A version bump touches **two** files in the release commit:

1. `rust/Cargo.toml` — `[package] version` field
2. `CHANGELOG.md` — new top-level `## [X.Y.Z] - YYYY-MM-DD` heading

Bumping fewer than two is a release-flow bug; halt. There is no
mechanical gate yet (no `tests/test_version_consistency.py`); a CI
addition is welcome but until then, reviewers enforce the rule by
eye on every release PR.

### Tagging

Release tag uses plain `vX.Y.Z` (no plugin-name prefix). After a
release PR has been merged to `main`:

```bash
git fetch origin && git checkout main && git pull --ff-only
git tag -a vX.Y.Z -m "Release vX.Y.Z" <merge-commit-sha>
git push origin vX.Y.Z
```

Then attach the cross-compiled `dualsense-mapper.exe` + a sample
`dualsense-mapper.json` as the GitHub Release artefact (`gh release
create vX.Y.Z target/x86_64-pc-windows-gnu/release/dualsense-mapper.exe
rust/config.example.json --title "v0.1.0" --notes-from-tag`).

End users download the two files, drop them in a folder, run.

## Branch flow

- Feature branches: `feat/<topic>` for additions, `fix/<topic>` for
  bug fixes, `chore/<topic>` for non-code chores.
- Release branches: `release/<vX.Y.Z>` for the PR that bumps Cargo.toml
  + CHANGELOG. Title `release(vX.Y.Z): ...`, PR body summarising what
  changed since the prior tag.
- Never commit directly to `main`. Tags only go on `main` merge commits.
- Force-pushing `main` is forbidden. Force-pushing feature branches is
  allowed if the branch is yours.

## No `--admin` override

CI failures (`cargo test`, `cargo clippy` if added later, any future
lint) **cannot** be bypassed with `gh pr merge --admin` or
`--no-verify`. Fix the source, then re-merge.

If a test really is wrong, amend the test in the same PR with a
written justification in the PR description. The fact that a lint
job looks advisory is exactly how the paperwork v2.11→v2.13 incident
shipped three broken releases under admin override — don't repeat it.

## What will be rejected

- Edits to `legacy-python/` (unless explicitly to fix a security
  vulnerability; ask first).
- Synthesized-input code that bypasses `safety.rs` refcount.
- Macro / config additions that allow constant (non-jittered)
  timing.
- Features that lock the binary to one OS without a clear story
  for the other (Phase 2 macOS is the explicit target — don't add
  Windows-only behaviour without a `#[cfg(target_os = "macos")]`
  counterpart or a deliberate "Phase 2 TBD" CHANGELOG note).
- Tracking `docs/superpowers/` content.
