//! State-machine integration tests for `HidSource`. Uses the byte-stream
//! injection constructor so no real DualSense is required.

use crossbeam_channel::unbounded;
use dualsense_mapper::gamepad::events::GamepadEvent;
use dualsense_mapper::gamepad::hid_source::HidSource;

fn neutral_report() -> Vec<u8> {
    let mut buf = vec![0u8; 78];
    buf[0] = 0x31;
    buf[2] = 128;
    buf[3] = 128;
    buf[4] = 128;
    buf[5] = 128;
    buf[9] = 0x08; // hat = released
    buf
}

#[test]
fn first_streaming_frame_emits_connected() {
    let (tx, rx) = unbounded::<Vec<u8>>();
    let mut src = HidSource::new_from_byte_stream(rx);

    tx.send(neutral_report()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));

    let mut out = Vec::new();
    src.poll(&mut out);
    assert_eq!(out.first(), Some(&GamepadEvent::Connected),
        "expected Connected first, got {out:?}");
}

#[test]
fn cross_press_then_release_emits_button_events() {
    let (tx, rx) = unbounded::<Vec<u8>>();
    let mut src = HidSource::new_from_byte_stream(rx);

    let mut press = neutral_report();
    press[9] |= 1 << 5; // Cross
    tx.send(neutral_report()).unwrap();
    tx.send(press).unwrap();
    tx.send(neutral_report()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));

    let mut out = Vec::new();
    src.poll(&mut out);
    assert!(out.iter().any(|e| matches!(e, GamepadEvent::ButtonDown(0))),
        "missing ButtonDown(0): {out:?}");
    assert!(out.iter().any(|e| matches!(e, GamepadEvent::ButtonUp(0))),
        "missing ButtonUp(0): {out:?}");
}

#[test]
fn channel_close_emits_disconnected() {
    let (tx, rx) = unbounded::<Vec<u8>>();
    let mut src = HidSource::new_from_byte_stream(rx);

    tx.send(neutral_report()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));
    drop(tx);
    std::thread::sleep(std::time::Duration::from_millis(100));

    let mut out = Vec::new();
    src.poll(&mut out);
    assert!(out.iter().any(|e| matches!(e, GamepadEvent::Disconnected)),
        "expected Disconnected after channel close: {out:?}");
}
