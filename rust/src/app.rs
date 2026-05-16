use crate::config::Config;
use crate::engine::Engine;
use crate::safety::{self, SharedKeyState};
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct RunOptions {
    pub dry_run: bool,
    pub shutdown: Arc<AtomicBool>,
}

pub fn run(cfg: Config, opts: RunOptions) -> Result<()> {
    // Install the panic hook before spawning the engine so that if the engine
    // thread panics, held keys are released via a best-effort fresh Enigo
    // instance. Engine::spawn already guarantees KeyboardSink::Drop runs on the
    // normal shutdown path (Iron Rule #3); the panic hook is the last-line
    // defence for the abnormal path.
    let state = safety::shared();
    install_panic_hook(state);

    let engine = Engine::spawn(cfg, opts.dry_run)?;
    while !opts.shutdown.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(50));
    }
    // shutdown() signals the engine thread and joins it; the engine thread's
    // graceful-shutdown path drains macros and then drops KeyboardSink, which
    // calls release_all_held() — Iron Rule #3 guaranteed.
    engine.shutdown();
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
