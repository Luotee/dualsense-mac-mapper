use crate::config::parse_key;
use crate::safety::{Edge, SharedKeyState};
use anyhow::Result;
use enigo::{Direction, Enigo, Keyboard, Settings};
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct KeyboardSink {
    /// `None` when `dry_run` is true — defer real Enigo construction so dry-run
    /// works on hosts without a display (CI, WSL without WSLg, etc.).
    enigo: Option<Enigo>,
    state: SharedKeyState,
    tick_jitter_ms: [u32; 2],
    min_press_ms: [u32; 2],
    /// When each currently-pressed key was emitted. Cleared on actual release.
    press_times: HashMap<String, Instant>,
    dry_run: bool,
}

impl KeyboardSink {
    pub fn new(
        state: SharedKeyState,
        tick_jitter_ms: [u32; 2],
        min_press_ms: [u32; 2],
        dry_run: bool,
    ) -> Result<Self> {
        let enigo = if dry_run { None } else { Some(Enigo::new(&Settings::default())?) };
        Ok(Self {
            enigo,
            state,
            tick_jitter_ms,
            min_press_ms,
            press_times: HashMap::new(),
            dry_run,
        })
    }

    pub fn press(&mut self, key_name: &str) -> Result<()> {
        let edge = {
            let mut s = self.state.lock().unwrap_or_else(|p| p.into_inner());
            s.press(key_name)
        };
        if edge == Edge::Press {
            self.jittered_pause();
            self.emit(key_name, Direction::Press)?;
            self.press_times.insert(key_name.to_string(), Instant::now());
        }
        Ok(())
    }

    pub fn release(&mut self, key_name: &str) -> Result<()> {
        let edge = {
            let mut s = self.state.lock().unwrap_or_else(|p| p.into_inner());
            s.release(key_name)
        };
        if edge == Edge::Release {
            self.wait_min_press(key_name);
            self.jittered_pause();
            self.emit(key_name, Direction::Release)?;
            self.press_times.remove(key_name);
        }
        Ok(())
    }

    /// Release every currently held key. Called by Drop, panic hook, signal handler.
    pub fn release_all_held(&mut self) {
        let held: Vec<String> = {
            let mut s = self.state.lock().unwrap_or_else(|p| p.into_inner());
            s.drain_held()
        };
        // Shutdown / panic / disconnect path: release every held key immediately.
        // min_press_ms is deliberately NOT honored here — getting keys back up is
        // more important than maintaining the anti-cheat timing profile.
        for k in held {
            tracing::info!(key = %k, "releasing held key on shutdown");
            let _ = self.emit(&k, Direction::Release);
        }
        self.press_times.clear();
    }

    fn wait_min_press(&self, key_name: &str) {
        let Some(pressed_at) = self.press_times.get(key_name) else { return; };
        let [lo, hi] = self.min_press_ms;
        // Config validation already enforces lo < hi.
        let target_ms = fastrand::u32(lo..=hi) as u128;
        let elapsed_ms = pressed_at.elapsed().as_millis();
        if elapsed_ms < target_ms {
            std::thread::sleep(Duration::from_millis((target_ms - elapsed_ms) as u64));
        }
    }

    fn emit(&mut self, key_name: &str, dir: Direction) -> Result<()> {
        tracing::debug!(key = %key_name, ?dir, "synth");
        if self.dry_run {
            println!("[dry-run] {dir:?} {key_name}");
            return Ok(());
        }
        let k = parse_key(key_name)?;
        let enigo = self.enigo.as_mut()
            .expect("non-dry-run KeyboardSink must hold an Enigo (constructed in new())");
        enigo.key(k, dir)?;
        Ok(())
    }

    fn jittered_pause(&self) {
        let [lo, hi] = self.tick_jitter_ms;
        if hi == 0 { return; }
        let ms = fastrand::u32(lo..=hi);
        if ms > 0 { std::thread::sleep(Duration::from_millis(ms as u64)); }
    }
}

impl Drop for KeyboardSink {
    fn drop(&mut self) {
        self.release_all_held();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::safety;

    #[test]
    fn dry_run_press_release_balances_state() {
        let state = safety::shared();
        let mut sink = KeyboardSink::new(state.clone(), [0, 0], [0, 1], true).unwrap();
        sink.press("Up").unwrap();
        sink.press("Up").unwrap();
        sink.release("Up").unwrap();
        sink.release("Up").unwrap();
        let held = state.lock().unwrap().drain_held();
        assert!(held.is_empty(), "expected no held keys, got {held:?}");
    }

    #[test]
    fn min_press_ms_enforces_floor_on_fast_release() {
        let state = safety::shared();
        let mut sink = KeyboardSink::new(state.clone(), [0, 0], [30, 50], true).unwrap();
        sink.press("Up").unwrap();
        let start = std::time::Instant::now();
        sink.release("Up").unwrap();
        let elapsed = start.elapsed().as_millis();
        assert!(elapsed >= 30, "release returned in {elapsed}ms, expected >= 30");
    }

    #[test]
    fn min_press_ms_skipped_on_release_all_held() {
        let state = safety::shared();
        let mut sink = KeyboardSink::new(state.clone(), [0, 0], [200, 300], true).unwrap();
        sink.press("Up").unwrap();
        let start = std::time::Instant::now();
        sink.release_all_held();
        let elapsed = start.elapsed().as_millis();
        assert!(elapsed < 100, "release_all_held should skip min_press_ms; took {elapsed}ms");
    }
}
