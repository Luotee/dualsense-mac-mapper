use crate::config::{MacroDef, StepAction};
use crate::mapper::KeyAction;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct MacroEngine {
    /// Per source_id (button id that started the macro), the cancel flag.
    active: HashMap<u32, Arc<AtomicBool>>,
    tx: Sender<KeyAction>,
}

impl MacroEngine {
    pub fn new(tx: Sender<KeyAction>) -> Self {
        Self { active: HashMap::new(), tx }
    }

    pub fn start(&mut self, source_id: u32, def: MacroDef) {
        // If already running for this source, stop first.
        self.stop(source_id);
        let cancel = Arc::new(AtomicBool::new(false));
        self.active.insert(source_id, cancel.clone());
        let tx = self.tx.clone();
        thread::spawn(move || run_macro(def, cancel, tx));
    }

    pub fn stop(&mut self, source_id: u32) {
        if let Some(flag) = self.active.remove(&source_id) {
            flag.store(true, Ordering::SeqCst);
        }
    }

    pub fn stop_all(&mut self) {
        for (_, flag) in self.active.drain() {
            flag.store(true, Ordering::SeqCst);
        }
    }
}

fn run_macro(def: MacroDef, cancel: Arc<AtomicBool>, tx: Sender<KeyAction>) {
    loop {
        for step in &def.steps {
            if cancel.load(Ordering::SeqCst) { return; }
            let action = match step.action {
                StepAction::Down => KeyAction::Press(step.key.clone()),
                StepAction::Up   => KeyAction::Release(step.key.clone()),
            };
            if tx.send(action).is_err() { return; } // receiver gone
            let [lo, hi] = step.delay_ms;
            // Config validation guarantees lo < hi, so the inclusive range is non-empty.
            let ms = fastrand::u32(lo..=hi);
            interruptible_sleep(Duration::from_millis(ms as u64), &cancel);
            if cancel.load(Ordering::SeqCst) { return; }
        }
        if !def.repeat { return; }
    }
}

fn interruptible_sleep(total: Duration, cancel: &AtomicBool) {
    let step = Duration::from_millis(5);
    let mut slept = Duration::ZERO;
    while slept < total {
        if cancel.load(Ordering::SeqCst) { return; }
        let s = std::cmp::min(step, total - slept);
        thread::sleep(s);
        slept += s;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{MacroDef, MacroStep, StepAction};
    use std::sync::mpsc;
    use std::time::Instant;

    fn one_step_loop(key: &str, range: [u32; 2]) -> MacroDef {
        MacroDef {
            repeat: true,
            steps: vec![MacroStep { key: key.into(), action: StepAction::Down, delay_ms: range }],
        }
    }

    #[test]
    fn start_then_stop_terminates_quickly() {
        let (tx, rx) = mpsc::channel::<KeyAction>();
        let mut eng = MacroEngine::new(tx);
        eng.start(23, one_step_loop("Left", [5, 10]));

        // wait for at least one action
        let first = rx.recv_timeout(Duration::from_millis(500)).expect("first action");
        assert!(matches!(first, KeyAction::Press(ref k) if k == "Left"));

        let stop_at = Instant::now();
        eng.stop(23);

        // After stop, channel must go quiet within a bounded window.
        let mut last = Instant::now();
        while rx.recv_timeout(Duration::from_millis(50)).is_ok() {
            last = Instant::now();
        }
        let drain_ms = (last - stop_at).as_millis();
        assert!(drain_ms < 200, "macro drained too slowly: {drain_ms}ms");
    }

    #[test]
    fn non_repeat_macro_runs_once() {
        let (tx, rx) = mpsc::channel::<KeyAction>();
        let mut eng = MacroEngine::new(tx);
        let def = MacroDef {
            repeat: false,
            steps: vec![
                MacroStep { key: "Left".into(),  action: StepAction::Down, delay_ms: [1, 5] },
                MacroStep { key: "Right".into(), action: StepAction::Down, delay_ms: [1, 5] },
            ],
        };
        eng.start(24, def);

        let mut got = Vec::new();
        while let Ok(a) = rx.recv_timeout(Duration::from_millis(200)) {
            got.push(a);
        }
        assert_eq!(got.len(), 2, "got {got:?}");
        assert!(matches!(got[0], KeyAction::Press(ref k) if k == "Left"));
        assert!(matches!(got[1], KeyAction::Press(ref k) if k == "Right"));
    }
}
