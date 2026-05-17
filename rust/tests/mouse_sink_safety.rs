//! Mouse-button refcount discipline at the safety layer.
//!
//! Mirrors the keyboard side (`safety::KeyState::press` / `release` /
//! `drain_held`) so Iron rule #3 (release on shutdown / panic) covers
//! both halves of the synthesis pipeline.

use dualsense_mapper::config::MouseButton;
use dualsense_mapper::safety;

#[test]
fn mouse_press_release_balances_via_shared_state() {
    let state = safety::shared();
    {
        let mut s = state.lock().unwrap();
        s.press_mouse(MouseButton::Left);
        assert!(s.is_mouse_held(MouseButton::Left));
        s.release_mouse(MouseButton::Left);
        assert!(!s.is_mouse_held(MouseButton::Left));
    }
}

#[test]
fn drain_held_mouse_collects_multi_buttons_and_clears() {
    let state = safety::shared();
    let drained = {
        let mut s = state.lock().unwrap();
        s.press_mouse(MouseButton::Left);
        s.press_mouse(MouseButton::Right);
        s.press_mouse(MouseButton::Middle);
        s.drain_held_mouse()
    };
    assert_eq!(drained.len(), 3);
    assert!(drained.contains(&MouseButton::Left));
    assert!(drained.contains(&MouseButton::Right));
    assert!(drained.contains(&MouseButton::Middle));

    let s = state.lock().unwrap();
    assert_eq!(s.len_held_mouse(), 0);
    assert!(!s.is_mouse_held(MouseButton::Left));
}

#[test]
fn mouse_refcount_double_press_only_releases_at_zero() {
    let state = safety::shared();
    let mut s = state.lock().unwrap();
    s.press_mouse(MouseButton::Left);
    s.press_mouse(MouseButton::Left);
    s.release_mouse(MouseButton::Left);
    assert!(s.is_mouse_held(MouseButton::Left), "still held — refcount > 0");
    s.release_mouse(MouseButton::Left);
    assert!(!s.is_mouse_held(MouseButton::Left));
}
