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
}

pub type SharedKeyState = Arc<Mutex<KeyState>>;

pub fn shared() -> SharedKeyState {
    Arc::new(Mutex::new(KeyState::new()))
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
