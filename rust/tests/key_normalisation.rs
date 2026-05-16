//! Parity test: every key name the frontend's `key_capture.js` emits MUST
//! parse cleanly via Rust's `config::parse_key`. If you add a new key to the
//! frontend's normalisation table, add it here too — and if `parse_key`
//! rejects it, fix `parse_key`, not the test.
//!
//! Spec §7.4. The frontend table is `rust/web/key_capture.js::normaliseKeyEvent`.

use dualsense_mapper::config::parse_key;

const ACCEPTED_BY_FRONTEND: &[&str] = &[
    // Letters and digits (the frontend lowercases letters before sending)
    "a", "z", "0", "9",
    // Arrows
    "Up", "Down", "Left", "Right",
    // Modifiers
    "Shift", "Control", "Alt", "Meta",
    // Function keys
    "F1", "F2", "F11", "F12",
    // Named keys
    "Space", "Enter", "Return", "Tab", "Escape", "Esc",
    "Backspace", "Delete", "Del",
    "Home", "End", "PageUp", "PageDown",
];

#[test]
fn parse_key_accepts_every_frontend_emitted_name() {
    for name in ACCEPTED_BY_FRONTEND {
        parse_key(name).unwrap_or_else(|e| {
            panic!("parse_key rejected frontend-accepted name '{name}': {e:#}")
        });
    }
}
