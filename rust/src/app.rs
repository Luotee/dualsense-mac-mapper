use crate::config::Config;
use crate::gamepad::GamepadSource;
use crate::keyboard::KeyboardSink;
use crate::macro_engine::MacroEngine;
use crate::mapper::{KeyAction, Mapper};
use crate::safety::{self, SharedKeyState};
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

pub struct RunOptions {
    pub dry_run: bool,
    pub shutdown: Arc<AtomicBool>,
}

pub fn run(cfg: Config, opts: RunOptions) -> Result<()> {
    let state: SharedKeyState = safety::shared();
    let tick_jitter = cfg.tick_jitter_ms;
    let min_press = cfg.min_press_ms;
    install_panic_hook(state.clone());

    let mut mapper = Mapper::new(cfg);
    let mut source = GamepadSource::new()?;
    let mut sink = KeyboardSink::new(state.clone(), tick_jitter, min_press, opts.dry_run)?;

    let (tx, rx) = mpsc::channel::<KeyAction>();
    let mut macros = MacroEngine::new(tx.clone());

    let mut gp_events = Vec::with_capacity(32);
    while !opts.shutdown.load(Ordering::SeqCst) {
        gp_events.clear();
        source.poll(&mut gp_events);
        for ev in gp_events.drain(..) {
            for action in mapper.handle(ev) {
                execute(&action, &mut sink, &mut macros, mapper.config())?;
            }
        }
        // Drain macro-emitted actions until the channel is momentarily empty.
        while let Ok(action) = rx.try_recv() {
            execute(&action, &mut sink, &mut macros, mapper.config())?;
        }
        std::thread::sleep(Duration::from_millis(8));
    }

    macros.stop_all();
    // Sink's Drop releases anything still held.
    Ok(())
}

fn execute(
    action: &KeyAction,
    sink: &mut KeyboardSink,
    macros: &mut MacroEngine,
    cfg: &Config,
) -> Result<()> {
    match action {
        KeyAction::Press(k)   => sink.press(k)?,
        KeyAction::Release(k) => sink.release(k)?,
        KeyAction::MacroStart { name, source_id } => {
            if let Some(def) = cfg.macros.get(name) {
                macros.start(*source_id, def.clone());
            } else {
                tracing::error!(name = %name, "macro not found in config");
            }
        }
        KeyAction::MacroStop { source_id } => {
            macros.stop(*source_id);
        }
    }
    Ok(())
}

fn install_panic_hook(state: SharedKeyState) {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Ok(mut s) = state.lock() {
            let held = s.drain_held();
            if !held.is_empty() {
                eprintln!("[panic] releasing held keys: {held:?}");
                // Best-effort synth: a fresh Enigo to release each held key.
                if let Ok(mut enigo) = enigo::Enigo::new(&enigo::Settings::default()) {
                    use enigo::{Direction, Keyboard};
                    for name in held {
                        if let Ok(k) = crate::config::parse_key(&name) {
                            let _ = enigo.key(k, Direction::Release);
                        }
                    }
                }
            }
        }
        prev(info);
    }));
}
