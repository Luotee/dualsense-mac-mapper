/// Integration test for Iron Rule #3: emergency_release_all clears held-key
/// refcount state. We don't assert OS-level synth (that would interfere with
/// the test runner's keyboard and requires a display server), but we do verify
/// that the in-memory refcount map is drained — which is the necessary
/// precondition for the panic hook not to leak stuck keys.
///
/// Uses `press_for_test` (doc(hidden) pub in safety.rs) to populate the
/// process-wide global panic-hook state, then calls `emergency_release_all`
/// and verifies via `global_len_held()` that the map is empty.
///
/// `press_for_test` initialises the global singleton if it hasn't been set yet
/// (via `GLOBAL.get_or_init`), so this test does not need `register_global`.

use dualsense_mapper::safety;

#[test]
fn emergency_release_clears_state() {
    // Simulate a held key in the global panic-hook state.
    safety::press_for_test("Left");
    assert!(
        safety::global_len_held() > 0,
        "expected at least one held key after press_for_test"
    );

    // emergency_release_all drains the global refcount map and attempts
    // best-effort OS synth (Enigo init failure is logged, not propagated,
    // so the call succeeds even on headless CI runners).
    safety::emergency_release_all().unwrap();

    assert_eq!(
        safety::global_len_held(),
        0,
        "expected no held keys after emergency_release_all"
    );
}
