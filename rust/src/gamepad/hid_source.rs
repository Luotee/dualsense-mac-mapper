//! DualSense BT raw HID source.
//!
//! Spawns a worker thread that owns the hidapi `HidDevice`, drives the
//! `Searching → Handshaking → Streaming` state machine, decodes 78-byte
//! 0x31 reports, diffs against the previous snapshot, and pushes
//! `GamepadEvent`s to a channel the engine drains via `poll()`.
//!
//! Three constructors:
//! - `new()`             — production hidapi worker
//! - `fake(rx)`          — direct GamepadEvent injection (engine tests)
//! - `new_from_byte_stream(rx)` — raw 78-byte injection (state-machine tests)

use super::cursor_params::CursorParams;
use super::ds_protocol::{build_handshake_buffer, decode_31, DsState, REPORT_LEN_BT};
use super::events::GamepadEvent;
use anyhow::{anyhow, Context, Result};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Drop sub-2-pixel motion so a resting finger doesn't synthesise a
/// drift. Anything strictly greater than this gets emitted.
const CURSOR_JITTER_FLOOR: i32 = 1;
/// Per-frame raw-coordinate delta upper bound. A real finger at 250 Hz
/// frame rate over the 1920-wide pad cannot move more than ~150 raw px
/// in one frame. Larger jumps come from the DualSense reporting one
/// stale (pre-touch) frame at touch-down — the very first contact
/// frame can carry the x/y of the previous touch even though `active`
/// has flipped to true. We re-anchor on these without emitting so the
/// cursor stays still rather than teleporting halfway across the screen.
const CURSOR_TELEPORT_GUARD: i32 = 250;
/// Button ids 25..=28 = Touchpad TL/TR/BL/BR. Matches
/// `config::TOUCHPAD_QUADRANT_IDS`; duplicated here to avoid pulling
/// the config crate into the gamepad layer.
const QUAD_TL: u32 = 25;
const QUAD_TR: u32 = 26;
const QUAD_BL: u32 = 27;
const QUAD_BR: u32 = 28;

/// Per-worker touchpad tracking. Reset on each new device connect.
#[derive(Default)]
struct TouchpadState {
    last_finger_pos: Option<(u16, u16)>,
    /// Position recorded at the moment finger 0 first became active in
    /// the current contact. The quadrant for the touchpad click uses
    /// this — the user's *intent* is where they put their finger, not
    /// where the finger has drifted to by the time the physical click
    /// switch fires (the press motion shifts the contact point).
    touchdown_pos: Option<(u16, u16)>,
    last_click_quadrant: Option<u32>,
    prev_touchpad_btn: bool,
}

fn quadrant_for(x: u16, y: u16, mid_x: u16, mid_y: u16) -> u32 {
    match (x < mid_x, y < mid_y) {
        (true,  true)  => QUAD_TL,
        (false, true)  => QUAD_TR,
        (true,  false) => QUAD_BL,
        (false, false) => QUAD_BR,
    }
}

/// Cursor delta + touchpad click → 4-quadrant button events. Mutates
/// `state` across frames. Called per decoded `DsState`.
fn process_touchpad(
    state: &mut TouchpadState,
    cur: &DsState,
    params: &CursorParams,
    tx: &Sender<GamepadEvent>,
) {
    // Cursor: relative motion from the previous finger 0 position.
    if cur.finger0_active {
        match state.last_finger_pos {
            None => {
                // Touch-down — record the start position without emitting.
                state.last_finger_pos = Some((cur.finger0_x, cur.finger0_y));
                state.touchdown_pos = Some((cur.finger0_x, cur.finger0_y));
            }
            Some((lx, ly)) => {
                let dx_raw = cur.finger0_x as i32 - lx as i32;
                let dy_raw = cur.finger0_y as i32 - ly as i32;
                if dx_raw.abs() > CURSOR_TELEPORT_GUARD
                    || dy_raw.abs() > CURSOR_TELEPORT_GUARD
                {
                    // Stale touch-down frame — re-anchor silently. The
                    // previous touchdown was garbage; replace it with the
                    // new real coordinates so the next click sees the
                    // settled position.
                    state.last_finger_pos = Some((cur.finger0_x, cur.finger0_y));
                    state.touchdown_pos = Some((cur.finger0_x, cur.finger0_y));
                } else {
                    let moved = dx_raw.abs() > CURSOR_JITTER_FLOOR
                        || dy_raw.abs() > CURSOR_JITTER_FLOOR;
                    if moved {
                        if params.enabled() {
                            let s = params.sensitivity();
                            let dx = (dx_raw as f32 * s) as i32;
                            let dy = (dy_raw as f32 * s) as i32;
                            let _ = tx.send(GamepadEvent::MouseDelta { dx, dy });
                        }
                        state.last_finger_pos = Some((cur.finger0_x, cur.finger0_y));
                        // touchdown_pos intentionally NOT updated on
                        // motion — it represents the user's intent at
                        // touch-down, not the drifted current position.
                    }
                }
            }
        }
    } else {
        // Finger lifted — clear references so the next touch-down
        // doesn't synthesise a jump from the old position.
        state.last_finger_pos = None;
        state.touchdown_pos = None;
    }

    // Click: rising edge captures the quadrant using the touch-down
    // position (the user's intent), not the click-frame instantaneous
    // position. Falling edge releases the same id (so a drag across
    // quadrant boundaries does not re-emit).
    if cur.touchpad_btn && !state.prev_touchpad_btn {
        let click_pos = state.touchdown_pos
            .or_else(|| if cur.finger0_active {
                Some((cur.finger0_x, cur.finger0_y))
            } else {
                None
            });
        let mid_x = params.midpoint_x();
        let mid_y = params.midpoint_y();
        let (raw_x, raw_y) = click_pos.unwrap_or((mid_x, mid_y));
        let q = quadrant_for(raw_x, raw_y, mid_x, mid_y);
        tracing::info!(
            x = raw_x,
            y = raw_y,
            mid_x,
            mid_y,
            quadrant = q,
            "touchpad click captured"
        );
        state.last_click_quadrant = Some(q);
        let _ = tx.send(GamepadEvent::TouchpadClick { raw_x, raw_y, quadrant: q });
        let _ = tx.send(GamepadEvent::ButtonDown(q));
    } else if !cur.touchpad_btn && state.prev_touchpad_btn {
        if let Some(q) = state.last_click_quadrant.take() {
            let _ = tx.send(GamepadEvent::ButtonUp(q));
        }
    }
    state.prev_touchpad_btn = cur.touchpad_btn;
}

/// Sony Computer Entertainment.
const DS_VID: u16 = 0x054c;
/// DualSense (standard PS5 pad).
const DS_PID: u16 = 0x0ce6;

/// Sleep between enumeration attempts in Searching state.
const ENUM_RETRY_INTERVAL: Duration = Duration::from_millis(1000);

/// Blocking read timeout per HID frame. 4 ms matches BT 250 Hz cadence.
const READ_TIMEOUT_MS: i32 = 4;
/// After this many consecutive read-timeouts (each `READ_TIMEOUT_MS`),
/// declare the pad disconnected. 50 × 4 ms = 200 ms.
const DISCONNECT_AFTER_TIMEOUTS: u32 = 50;

pub struct HidSource {
    rx: Receiver<GamepadEvent>,
    stop: Arc<AtomicBool>,
}

impl HidSource {
    /// Spawn the production hidapi worker. The worker thread starts in
    /// `Searching` and emits no events until the first 0x31 frame is
    /// decoded. `params` carries the live touchpad cursor sensitivity
    /// and on/off flag; the engine mutates them through `set_settings`.
    pub fn new(params: CursorParams) -> Result<Self> {
        let (tx, rx) = unbounded::<GamepadEvent>();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_thread = stop.clone();
        thread::Builder::new()
            .name("dualsense-hid".into())
            .spawn(move || worker_real(tx, stop_for_thread, params))
            .map_err(|e| anyhow!("spawning HID worker thread: {e}"))?;
        Ok(Self { rx, stop })
    }

    /// Test-only constructor for engine integration tests. Drains
    /// `GamepadEvent`s directly from the supplied channel — no HID
    /// pipeline, no state machine.
    #[doc(hidden)]
    pub fn fake(rx: Receiver<GamepadEvent>) -> Self {
        Self { rx, stop: Arc::new(AtomicBool::new(false)) }
    }

    /// Test-only constructor for state-machine tests. Decodes
    /// `Vec<u8>` frames as if they came from hidapi and runs the full
    /// diff pipeline. Uses default cursor params (1.5, enabled=true).
    #[doc(hidden)]
    pub fn new_from_byte_stream(byte_rx: Receiver<Vec<u8>>) -> Self {
        Self::new_from_byte_stream_with_params(byte_rx, CursorParams::default())
    }

    /// Same as `new_from_byte_stream` but with explicit cursor params.
    #[doc(hidden)]
    pub fn new_from_byte_stream_with_params(
        byte_rx: Receiver<Vec<u8>>,
        params: CursorParams,
    ) -> Self {
        let (tx, rx) = unbounded::<GamepadEvent>();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_thread = stop.clone();
        thread::Builder::new()
            .name("dualsense-hid-fake".into())
            .spawn(move || worker_byte_stream(byte_rx, tx, stop_for_thread, params))
            .expect("spawning fake HID worker");
        Self { rx, stop }
    }

    /// Drain pending events into `out`. Non-blocking.
    pub fn poll(&mut self, out: &mut Vec<GamepadEvent>) {
        while let Ok(ev) = self.rx.try_recv() {
            out.push(ev);
        }
    }
}

impl Drop for HidSource {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

fn worker_real(tx: Sender<GamepadEvent>, stop: Arc<AtomicBool>, params: CursorParams) {
    let mut api = match hidapi::HidApi::new() {
        Ok(a) => a,
        Err(e) => {
            tracing::error!(error = %e, "hidapi init failed");
            return;
        }
    };
    while !stop.load(Ordering::SeqCst) {
        // device_list() returns a cached snapshot — must refresh
        // explicitly each iteration or a pad turned on after app
        // startup is invisible forever.
        if let Err(e) = api.refresh_devices() {
            tracing::warn!(error = %e, "hidapi refresh_devices failed");
        }
        let device = api
            .device_list()
            .find(|info| info.vendor_id() == DS_VID && info.product_id() == DS_PID)
            .and_then(|info| info.open_device(&api).ok());
        match device {
            Some(d) => {
                tracing::info!("DualSense opened, attempting 0x31 handshake");
                let _ = try_handshake(&d);
                let mut prev_buttons = [false; 25];
                let mut last_state: Option<DsState> = None;
                let mut touchpad = TouchpadState::default();
                let outcome = read_loop(
                    d, &tx, &mut last_state, &mut prev_buttons, &mut touchpad, &params, &stop,
                );
                tracing::info!(?outcome, "read loop exited; back to Searching");
                let _ = tx.send(GamepadEvent::Disconnected);
            }
            None => {
                thread::sleep(ENUM_RETRY_INTERVAL);
            }
        }
    }
}

fn try_handshake(d: &hidapi::HidDevice) -> Result<()> {
    let mut buf = build_handshake_buffer();
    d.get_feature_report(&mut buf)
        .context("requesting calibration feature 0x05 to unlock 0x31 mode")?;
    Ok(())
}

fn read_loop(
    d: hidapi::HidDevice,
    tx: &Sender<GamepadEvent>,
    last_state: &mut Option<DsState>,
    prev_buttons: &mut [bool; 25],
    touchpad: &mut TouchpadState,
    params: &CursorParams,
    stop: &Arc<AtomicBool>,
) -> &'static str {
    let mut buf = [0u8; REPORT_LEN_BT];
    let mut consecutive_timeouts = 0u32;
    let mut emitted_connected = false;
    while !stop.load(Ordering::SeqCst) {
        match d.read_timeout(&mut buf, READ_TIMEOUT_MS) {
            Ok(0) => {
                consecutive_timeouts += 1;
                if consecutive_timeouts >= DISCONNECT_AFTER_TIMEOUTS {
                    return "timeout";
                }
            }
            Ok(_n) => {
                consecutive_timeouts = 0;
                if let Some(s) = decode_31(&buf) {
                    if !emitted_connected {
                        let _ = tx.send(GamepadEvent::Connected);
                        emitted_connected = true;
                    }
                    diff_emit(last_state.as_ref(), &s, prev_buttons, tx);
                    process_touchpad(touchpad, &s, params, tx);
                    *last_state = Some(s);
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "HID read error");
                return "read-error";
            }
        }
    }
    "stop-signal"
}

fn worker_byte_stream(
    byte_rx: Receiver<Vec<u8>>,
    tx: Sender<GamepadEvent>,
    stop: Arc<AtomicBool>,
    params: CursorParams,
) {
    let mut last_state: Option<DsState> = None;
    let mut prev_buttons = [false; 25];
    let mut touchpad = TouchpadState::default();
    let mut emitted_connected = false;
    while !stop.load(Ordering::SeqCst) {
        match byte_rx.recv() {
            Ok(buf) => {
                if let Some(s) = decode_31(&buf) {
                    if !emitted_connected {
                        let _ = tx.send(GamepadEvent::Connected);
                        emitted_connected = true;
                    }
                    diff_emit(last_state.as_ref(), &s, &mut prev_buttons, &tx);
                    process_touchpad(&mut touchpad, &s, &params, &tx);
                    last_state = Some(s);
                }
            }
            Err(_) => {
                let _ = tx.send(GamepadEvent::Disconnected);
                return;
            }
        }
    }
}

fn diff_emit(
    prev: Option<&DsState>,
    cur: &DsState,
    prev_buttons: &mut [bool; 25],
    tx: &Sender<GamepadEvent>,
) {
    if prev.map_or(true, |p| p.stick_lx != cur.stick_lx) {
        let _ = tx.send(GamepadEvent::Stick { axis: 0, value: cur.stick_lx });
    }
    if prev.map_or(true, |p| p.stick_ly != cur.stick_ly) {
        let _ = tx.send(GamepadEvent::Stick { axis: 1, value: cur.stick_ly });
    }
    if prev.map_or(true, |p| p.stick_rx != cur.stick_rx) {
        let _ = tx.send(GamepadEvent::Stick { axis: 2, value: cur.stick_rx });
    }
    if prev.map_or(true, |p| p.stick_ry != cur.stick_ry) {
        let _ = tx.send(GamepadEvent::Stick { axis: 3, value: cur.stick_ry });
    }
    if prev.map_or(true, |p| p.trigger_l2 != cur.trigger_l2) {
        let _ = tx.send(GamepadEvent::Trigger { axis: 4, value: cur.trigger_l2 });
    }
    if prev.map_or(true, |p| p.trigger_r2 != cur.trigger_r2) {
        let _ = tx.send(GamepadEvent::Trigger { axis: 5, value: cur.trigger_r2 });
    }
    for (i, &down) in cur.buttons.iter().enumerate() {
        if down != prev_buttons[i] {
            let id = i as u32;
            let _ = tx.send(if down {
                GamepadEvent::ButtonDown(id)
            } else {
                GamepadEvent::ButtonUp(id)
            });
        }
    }
    prev_buttons.copy_from_slice(&cur.buttons);
}
