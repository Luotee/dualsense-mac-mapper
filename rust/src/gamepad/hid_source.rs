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

use super::ds_protocol::{build_handshake_buffer, decode_31, DsState, REPORT_LEN_BT};
use super::events::GamepadEvent;
use anyhow::{anyhow, Context, Result};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

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
    /// decoded.
    pub fn new() -> Result<Self> {
        let (tx, rx) = unbounded::<GamepadEvent>();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_thread = stop.clone();
        thread::Builder::new()
            .name("dualsense-hid".into())
            .spawn(move || worker_real(tx, stop_for_thread))
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
    /// diff pipeline.
    #[doc(hidden)]
    pub fn new_from_byte_stream(byte_rx: Receiver<Vec<u8>>) -> Self {
        let (tx, rx) = unbounded::<GamepadEvent>();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_thread = stop.clone();
        thread::Builder::new()
            .name("dualsense-hid-fake".into())
            .spawn(move || worker_byte_stream(byte_rx, tx, stop_for_thread))
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

fn worker_real(tx: Sender<GamepadEvent>, stop: Arc<AtomicBool>) {
    let api = match hidapi::HidApi::new() {
        Ok(a) => a,
        Err(e) => {
            tracing::error!(error = %e, "hidapi init failed");
            return;
        }
    };
    while !stop.load(Ordering::SeqCst) {
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
                let outcome = read_loop(d, &tx, &mut last_state, &mut prev_buttons, &stop);
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
) {
    let mut last_state: Option<DsState> = None;
    let mut prev_buttons = [false; 25];
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
