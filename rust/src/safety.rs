use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edge {
    /// Refcount went 0 → 1: caller should physically press.
    Press,
    /// Refcount went 1 → 0: caller should physically release.
    Release,
    /// No physical action required.
    None,
}

/// Process-wide singleton used exclusively by the panic hook and by the
/// `emergency_release_all` / `press_for_test` helpers.
///
/// This is intentionally separate from `shared()` (which returns a fresh Arc
/// each call) so that integration tests that spawn multiple Engine instances
/// get independent refcount tables and do not interfere with each other.
/// In production there is only one Engine; that engine passes its own
/// `SharedKeyState` clone to `register_global` immediately after construction,
/// binding the global to the real engine state.
static GLOBAL: std::sync::OnceLock<SharedKeyState> = std::sync::OnceLock::new();

#[derive(Default, Debug)]
pub struct KeyState {
    counts: HashMap<String, u32>,
}

impl KeyState {
    pub fn new() -> Self {
        Self { counts: HashMap::new() }
    }

    pub fn press(&mut self, key: &str) -> Edge {
        let entry = self.counts.entry(key.to_string()).or_insert(0);
        *entry += 1;
        if *entry == 1 { Edge::Press } else { Edge::None }
    }

    pub fn release(&mut self, key: &str) -> Edge {
        let entry = self.counts.entry(key.to_string()).or_insert(0);
        if *entry == 0 {
            return Edge::None;
        }
        *entry -= 1;
        if *entry == 0 { Edge::Release } else { Edge::None }
    }

    pub fn drain_held(&mut self) -> Vec<String> {
        let held: Vec<String> = self.counts.iter()
            .filter_map(|(k, c)| if *c > 0 { Some(k.clone()) } else { None })
            .collect();
        self.counts.clear();
        held
    }

    /// Number of keys with a positive refcount (i.e., currently pressed).
    pub fn len_held(&self) -> usize {
        self.counts.values().filter(|&&c| c > 0).count()
    }
}

pub type SharedKeyState = Arc<Mutex<KeyState>>;

/// Returns a **new** independent `SharedKeyState`.
///
/// Each call returns a distinct `Arc<Mutex<KeyState>>`. This is the function
/// used by the Engine and KeyboardSink — integration tests that spawn multiple
/// Engine instances each get their own isolated refcount table and do not
/// interfere with each other.
///
/// In production there is one Engine; after spawning, call
/// `register_global(state.clone())` so that `emergency_release_all` (invoked
/// by the panic hook) can drain the real held-key map.
pub fn shared() -> SharedKeyState {
    Arc::new(Mutex::new(KeyState::new()))
}

/// Bind the process-wide panic-hook state to a specific `SharedKeyState`.
///
/// Call this once, immediately after `Engine::spawn`, passing a clone of the
/// engine's key state. The panic hook installed in `main.rs` will then drain
/// the real engine state on panic.
///
/// Calling this more than once is a no-op (the `OnceLock` keeps the first
/// value) — there should only ever be one Engine in production.
pub fn register_global(state: SharedKeyState) {
    let _ = GLOBAL.set(state);
}

/// OS-level emergency key release: drains the held-key refcount table and
/// synthesises a `Direction::Release` for each held key via a fresh Enigo.
///
/// Called from the panic hook in `main.rs` (Iron Rule #3).
///
/// The refcount map is **always** drained (so callers can assert `len_held()==0`
/// even on a headless machine where Enigo has no display). The OS-level synth
/// is best-effort: if Enigo fails to initialise (e.g. no display server in CI),
/// the error is logged to stderr and `Ok(())` is returned so the panic hook
/// does not itself panic.
///
/// If `register_global` was never called (e.g. the binary panicked before
/// reaching `Engine::spawn`), this is a safe no-op.
pub fn emergency_release_all() -> anyhow::Result<()> {
    let Some(state) = GLOBAL.get() else { return Ok(()); };
    let held = state
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .drain_held();
    if held.is_empty() {
        return Ok(());
    }
    eprintln!("[emergency_release_all] releasing held keys: {held:?}");
    // Best-effort OS synth — may fail on headless runners; that is acceptable.
    match enigo::Enigo::new(&enigo::Settings::default()) {
        Ok(mut enigo) => {
            use enigo::{Direction, Keyboard};
            for name in &held {
                if let Ok(k) = crate::config::parse_key(name) {
                    let _ = enigo.key(k, Direction::Release);
                }
            }
        }
        Err(e) => {
            eprintln!("[emergency_release_all] Enigo init failed (best-effort, continuing): {e}");
        }
    }
    Ok(())
}

/// Simulate a key press in the global panic-hook state. For integration tests only.
///
/// Initialises the global singleton if it has not been registered yet, so
/// the test does not depend on `register_global` being called first.
#[doc(hidden)]
pub fn press_for_test(key: &str) {
    let state = GLOBAL.get_or_init(|| Arc::new(Mutex::new(KeyState::new())));
    state.lock().unwrap().press(key);
}

/// Return the number of keys with a positive refcount in the global state.
/// For integration tests only — allows tests to assert the count without
/// exposing a raw `SharedKeyState` clone of the global.
#[doc(hidden)]
pub fn global_len_held() -> usize {
    GLOBAL
        .get()
        .map(|s| s.lock().unwrap_or_else(|p| p.into_inner()).len_held())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_press_release_edges() {
        let mut s = KeyState::new();
        assert_eq!(s.press("x"), Edge::Press);
        assert_eq!(s.release("x"), Edge::Release);
    }

    #[test]
    fn double_press_only_first_edges() {
        let mut s = KeyState::new();
        assert_eq!(s.press("Up"), Edge::Press);
        assert_eq!(s.press("Up"), Edge::None);
        assert_eq!(s.release("Up"), Edge::None);
        assert_eq!(s.release("Up"), Edge::Release);
    }

    #[test]
    fn unbalanced_release_is_safe_noop() {
        let mut s = KeyState::new();
        assert_eq!(s.release("Up"), Edge::None);
    }

    #[test]
    fn drain_returns_held_keys_and_clears() {
        let mut s = KeyState::new();
        s.press("x");
        s.press("Up");
        s.press("Up");
        let held = s.drain_held();
        assert!(held.contains(&"x".to_string()));
        assert!(held.contains(&"Up".to_string()));
        assert_eq!(held.len(), 2);
        assert_eq!(s.release("x"), Edge::None); // table cleared
    }
}
