use crate::config::parse_key;
use crate::safety::{Edge, SharedKeyState};
use anyhow::Result;
use enigo::{Direction, Enigo, Keyboard, Settings};
use std::time::Duration;

pub struct KeyboardSink {
    /// `None` when `dry_run` is true — defer real Enigo construction so dry-run
    /// works on hosts without a display (CI, WSL without WSLg, etc.).
    enigo: Option<Enigo>,
    state: SharedKeyState,
    tick_jitter_ms: [u32; 2],
    dry_run: bool,
}

impl KeyboardSink {
    pub fn new(state: SharedKeyState, tick_jitter_ms: [u32; 2], dry_run: bool) -> Result<Self> {
        let enigo = if dry_run { None } else { Some(Enigo::new(&Settings::default())?) };
        Ok(Self { enigo, state, tick_jitter_ms, dry_run })
    }

    pub fn press(&mut self, key_name: &str) -> Result<()> {
        let edge = {
            let mut s = self.state.lock().unwrap();
            s.press(key_name)
        };
        if edge == Edge::Press {
            self.jittered_pause();
            self.emit(key_name, Direction::Press)?;
        }
        Ok(())
    }

    pub fn release(&mut self, key_name: &str) -> Result<()> {
        let edge = {
            let mut s = self.state.lock().unwrap();
            s.release(key_name)
        };
        if edge == Edge::Release {
            self.jittered_pause();
            self.emit(key_name, Direction::Release)?;
        }
        Ok(())
    }

    /// Release every currently held key. Called by Drop, panic hook, signal handler.
    pub fn release_all_held(&mut self) {
        let held: Vec<String> = {
            let mut s = self.state.lock().unwrap();
            s.drain_held()
        };
        for k in held {
            tracing::info!(key = %k, "releasing held key on shutdown");
            let _ = self.emit(&k, Direction::Release);
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
        let mut sink = KeyboardSink::new(state.clone(), [0, 0], true).unwrap();
        sink.press("Up").unwrap();
        sink.press("Up").unwrap();
        sink.release("Up").unwrap();
        sink.release("Up").unwrap();
        let held = state.lock().unwrap().drain_held();
        assert!(held.is_empty(), "expected no held keys, got {held:?}");
    }
}
