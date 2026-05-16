//! Engine: long-running mapper loop with a thread-safe handle for the GUI.
//!
//! v0.1.x ran the loop directly from `app::run`. The GUI needs to mutate
//! config while the loop runs, pause it, suppress synth during key
//! capture, and shut it down. This module wraps that into:
//!
//!   * `Engine`      — owns the thread.
//!   * `Handle`      — what the GUI holds; clones cheaply (`Arc` inside).
//!   * `EngineEvent` — what the loop emits for the activity log.

use crate::config::Config;
use crate::gamepad::GamepadSource;
use crate::keyboard::KeyboardSink;
use crate::macro_engine::MacroEngine;
use crate::mapper::{KeyAction, Mapper};
use crate::safety::{self, SharedKeyState};
use anyhow::Result;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread::{self, JoinHandle};
use std::time::Duration;

// ─── Public event type ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum EngineEvent {
    ControllerConnected { name: String, transport: String },
    ControllerDisconnected,
    ButtonDown { id: u32 },
    ButtonUp   { id: u32 },
    KeyEmit    { ts_ms: u64, key: String, action: &'static str /* "down"|"up" */ },
    MacroStart { ts_ms: u64, name: String },
    MacroEnd   { ts_ms: u64, name: String, completed: bool },
}

/// Snapshot of the controller's current connection state. Polled by the GUI
/// on first load (since Tauri events don't replay missed emissions) and
/// mutated by the run loop on every Connected/Disconnected.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ControllerStatus {
    pub name: String,
    pub transport: String,
}

// ─── ConfigWriteGuard — bumps generation on drop ─────────────────────────────

/// An RAII wrapper around `RwLockWriteGuard<Config>` that increments
/// `config_generation` when dropped, so the run loop knows to rebuild
/// the `Mapper` on the next tick.
pub struct ConfigWriteGuard<'a> {
    guard: RwLockWriteGuard<'a, Config>,
    generation: &'a AtomicU64,
}

impl<'a> std::ops::Deref for ConfigWriteGuard<'a> {
    type Target = Config;
    fn deref(&self) -> &Config { &self.guard }
}

impl<'a> std::ops::DerefMut for ConfigWriteGuard<'a> {
    fn deref_mut(&mut self) -> &mut Config { &mut self.guard }
}

impl<'a> Drop for ConfigWriteGuard<'a> {
    fn drop(&mut self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }
}

// ─── Handle internals ────────────────────────────────────────────────────────

struct HandleInner {
    config: RwLock<Config>,
    /// Incremented by ConfigWriteGuard::drop; the run loop tracks the last
    /// seen value and rebuilds the Mapper when they differ.
    config_generation: AtomicU64,
    paused: AtomicBool,
    capture_active: AtomicBool,
    shutdown: AtomicBool,
    event_tx: Sender<EngineEvent>,
    event_rx: Receiver<EngineEvent>,
    key_state: SharedKeyState,
    /// Current controller connection state. Set/cleared by the run loop on
    /// every Connected/Disconnected event. Exposed via `Handle::current_status`
    /// so the frontend can synchronously query "is a controller already
    /// connected?" on init without waiting to miss a startup-race event.
    current_status: RwLock<Option<ControllerStatus>>,
    /// Test-only: sender for the fake gamepad source. `None` in production.
    #[doc(hidden)]
    fake_tx: Mutex<Option<crossbeam_channel::Sender<crate::gamepad::GamepadEvent>>>,
}

// ─── Handle (GUI-side) ───────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Handle {
    inner: Arc<HandleInner>,
}

impl Handle {
    pub fn set_paused(&self, v: bool) {
        self.inner.paused.store(v, Ordering::SeqCst);
    }
    pub fn is_paused(&self) -> bool {
        self.inner.paused.load(Ordering::SeqCst)
    }
    pub fn set_capture_active(&self, v: bool) {
        self.inner.capture_active.store(v, Ordering::SeqCst);
    }
    pub fn is_capture_active(&self) -> bool {
        self.inner.capture_active.load(Ordering::SeqCst)
    }

    pub fn config_read(&self) -> RwLockReadGuard<'_, Config> {
        self.inner.config.read().unwrap()
    }

    pub fn config_write(&self) -> ConfigWriteGuard<'_> {
        ConfigWriteGuard {
            guard: self.inner.config.write().unwrap(),
            generation: &self.inner.config_generation,
        }
    }

    /// Current controller connection state — `Some` if a gamepad is plugged in
    /// (or was at startup), `None` otherwise. Frontend uses this on first load
    /// to avoid the Tauri-event startup race (events don't replay).
    pub fn current_status(&self) -> Option<ControllerStatus> {
        self.inner.current_status.read().unwrap().clone()
    }

    pub fn drain_events(&self) -> Vec<EngineEvent> {
        let mut v = Vec::new();
        while let Ok(e) = self.inner.event_rx.try_recv() {
            v.push(e);
        }
        v
    }

    pub fn held_keys_count(&self) -> usize {
        self.inner.key_state.lock().unwrap_or_else(|p| p.into_inner()).len_held()
    }

    /// Return a clone of the engine's key-state Arc. Used by `app::run` to
    /// register the global panic-hook state after the engine is spawned.
    pub fn key_state(&self) -> crate::safety::SharedKeyState {
        self.inner.key_state.clone()
    }

    /// Inject a fake ButtonDown event. Works only when spawned via
    /// `Engine::spawn_with_fake_gamepad`; no-op otherwise.
    #[doc(hidden)]
    pub fn fake_button_down(&self, id: u32) {
        if let Ok(guard) = self.inner.fake_tx.lock() {
            if let Some(ref tx) = *guard {
                let _ = tx.send(crate::gamepad::GamepadEvent::ButtonDown(id));
            }
        }
    }

    /// Inject a fake ButtonUp event. Works only when spawned via
    /// `Engine::spawn_with_fake_gamepad`; no-op otherwise.
    #[doc(hidden)]
    pub fn fake_button_up(&self, id: u32) {
        if let Ok(guard) = self.inner.fake_tx.lock() {
            if let Some(ref tx) = *guard {
                let _ = tx.send(crate::gamepad::GamepadEvent::ButtonUp(id));
            }
        }
    }
}

// ─── Engine (owns the thread) ────────────────────────────────────────────────

pub struct Engine {
    thread: Option<JoinHandle<Result<()>>>,
    handle: Handle,
}

impl Engine {
    /// Spawn with a real gamepad source.
    pub fn spawn(cfg: Config, dry_run: bool) -> Result<Self> {
        let src = GamepadSource::new()?;
        Self::spawn_inner(cfg, dry_run, src)
    }

    /// Spawn with a fake gamepad source. Events are injected via
    /// `Handle::fake_button_down` / `Handle::fake_button_up`.
    /// Not part of the stable public API — used by integration tests.
    #[doc(hidden)]
    pub fn spawn_with_fake_gamepad(cfg: Config) -> Result<Self> {
        let (fake_tx, fake_rx) = unbounded::<crate::gamepad::GamepadEvent>();
        let src = GamepadSource::fake(fake_rx);
        let engine = Self::spawn_inner(cfg, /*dry_run=*/true, src)?;
        // Install the sender into HandleInner.
        *engine.handle.inner.fake_tx.lock().unwrap() = Some(fake_tx);
        Ok(engine)
    }

    fn spawn_inner(cfg: Config, dry_run: bool, src: GamepadSource) -> Result<Self> {
        let key_state = safety::shared();
        let (event_tx, event_rx) = unbounded::<EngineEvent>();

        let inner = Arc::new(HandleInner {
            config: RwLock::new(cfg),
            config_generation: AtomicU64::new(0),
            paused: AtomicBool::new(false),
            capture_active: AtomicBool::new(false),
            shutdown: AtomicBool::new(false),
            event_tx,
            event_rx,
            key_state: key_state.clone(),
            fake_tx: Mutex::new(None),
            current_status: RwLock::new(None),
        });

        let handle = Handle { inner: inner.clone() };

        let thread = thread::spawn(move || run_loop(inner, src, dry_run));
        Ok(Engine { thread: Some(thread), handle })
    }

    pub fn handle(&self) -> Handle {
        self.handle.clone()
    }

    /// Signal shutdown and block until the engine thread exits.
    pub fn shutdown(mut self) {
        self.handle.inner.shutdown.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

// ─── Run loop (engine-thread) ─────────────────────────────────────────────────

fn run_loop(h: Arc<HandleInner>, mut source: GamepadSource, dry_run: bool) -> Result<()> {
    // Read initial config values before the loop.
    let (tick_jitter_ms, min_press_ms) = {
        let cfg = h.config.read().unwrap();
        (cfg.tick_jitter_ms, cfg.min_press_ms)
    };

    let mut sink = KeyboardSink::new(h.key_state.clone(), tick_jitter_ms, min_press_ms, dry_run)
        .expect("KeyboardSink::new failed");

    // Set up macro → keyboard channel (std::sync::mpsc, matching MacroEngine).
    let (macro_tx, macro_rx) = std::sync::mpsc::channel::<KeyAction>();
    let mut macros = MacroEngine::new(macro_tx);

    // Build the initial mapper.
    let mut mapper = {
        let cfg = h.config.read().unwrap();
        Mapper::new(cfg.clone())
    };
    let mut last_config_gen = h.config_generation.load(Ordering::SeqCst);

    let mut last_paused = false;
    let mut gp_events: Vec<crate::gamepad::GamepadEvent> = Vec::with_capacity(32);

    while !h.shutdown.load(Ordering::SeqCst) {
        // ── Pause / unpause edge ─────────────────────────────────────────────
        let paused = h.paused.load(Ordering::SeqCst);
        if paused && !last_paused {
            // Drain all macros and held keys when pausing.
            macros.stop_all();
            // Drain macro-emitted releases so sink's refcounts are balanced.
            while let Ok(action) = macro_rx.try_recv() {
                execute_action(&action, &mut sink, &mut macros, mapper.config(), &h.event_tx);
            }
            sink.release_all_held();
        }
        last_paused = paused;

        // ── Config change: rebuild mapper ────────────────────────────────────
        let current_gen = h.config_generation.load(Ordering::SeqCst);
        if current_gen != last_config_gen {
            let cfg = h.config.read().unwrap();
            mapper = Mapper::new(cfg.clone());
            last_config_gen = current_gen;
        }

        if !paused {
            // ── Poll gamepad ─────────────────────────────────────────────────
            gp_events.clear();
            source.poll(&mut gp_events);

            for ev in gp_events.drain(..) {
                // Always emit raw gamepad events (even when capture_active).
                match &ev {
                    crate::gamepad::GamepadEvent::ButtonDown(id) => {
                        let _ = h.event_tx.send(EngineEvent::ButtonDown { id: *id });
                    }
                    crate::gamepad::GamepadEvent::ButtonUp(id) => {
                        let _ = h.event_tx.send(EngineEvent::ButtonUp { id: *id });
                    }
                    crate::gamepad::GamepadEvent::Connected => {
                        let status = ControllerStatus {
                            name: "DualSense".to_string(),
                            transport: "USB/BT".to_string(),
                        };
                        *h.current_status.write().unwrap() = Some(status.clone());
                        let _ = h.event_tx.send(EngineEvent::ControllerConnected {
                            name: status.name,
                            transport: status.transport,
                        });
                    }
                    crate::gamepad::GamepadEvent::Disconnected => {
                        *h.current_status.write().unwrap() = None;
                        let _ = h.event_tx.send(EngineEvent::ControllerDisconnected);
                    }
                    _ => {}
                }

                // Gate synth on capture_active.
                if h.capture_active.load(Ordering::SeqCst) {
                    continue;
                }

                let cfg = h.config.read().unwrap();
                let actions = mapper.handle(ev);
                drop(cfg); // release lock before execute calls into sink
                for (id, pressed) in mapper.take_visual_transitions() {
                    let _ = if pressed {
                        h.event_tx.send(EngineEvent::ButtonDown { id })
                    } else {
                        h.event_tx.send(EngineEvent::ButtonUp { id })
                    };
                }
                for action in &actions {
                    execute_action(action, &mut sink, &mut macros, mapper.config(), &h.event_tx);
                }
            }

            // ── Drain macro-emitted key actions ──────────────────────────────
            while let Ok(action) = macro_rx.try_recv() {
                execute_action(&action, &mut sink, &mut macros, mapper.config(), &h.event_tx);
            }
        }

        thread::sleep(Duration::from_millis(8));
    }

    // ── Graceful shutdown ────────────────────────────────────────────────────
    macros.stop_all();
    // Drain any final macro releases.
    while let Ok(action) = macro_rx.try_recv() {
        execute_action(&action, &mut sink, &mut macros, mapper.config(), &h.event_tx);
    }
    // sink.drop() will call release_all_held() — that is the iron-rule guarantee.
    // We explicitly drop to make the intent clear.
    drop(sink);
    Ok(())
}

fn execute_action(
    action: &KeyAction,
    sink: &mut KeyboardSink,
    macros: &mut MacroEngine,
    cfg: &Config,
    event_tx: &Sender<EngineEvent>,
) {
    let now_ms = || {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    };
    match action {
        KeyAction::Press(k) => {
            let _ = sink.press(k);
            let _ = event_tx.send(EngineEvent::KeyEmit {
                ts_ms: now_ms(),
                key: k.clone(),
                action: "down",
            });
        }
        KeyAction::Release(k) => {
            let _ = sink.release(k);
            let _ = event_tx.send(EngineEvent::KeyEmit {
                ts_ms: now_ms(),
                key: k.clone(),
                action: "up",
            });
        }
        KeyAction::MacroStart { name, source_id } => {
            if let Some(def) = cfg.macros.get(name) {
                macros.start(*source_id, def.clone());
                let _ = event_tx.send(EngineEvent::MacroStart {
                    ts_ms: now_ms(),
                    name: name.clone(),
                });
            } else {
                tracing::error!(name = %name, "macro not found in config");
            }
        }
        KeyAction::MacroStop { source_id } => {
            macros.stop(*source_id);
            // MacroEnd with completed=false (interrupted by button release).
            // We don't track name per source_id here; emit a best-effort event.
        }
    }
}
