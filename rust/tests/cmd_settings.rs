mod test_helpers;

use dualsense_mapper::engine::Engine;
use dualsense_mapper::gui::commands::{Settings, set_settings_impl, reset_settings_impl};

#[test]
fn set_settings_persists_and_rejects_invalid_ranges() {
    let cfg = test_helpers::load_example();
    let cfg_path = test_helpers::tmp_config_with(&cfg);
    let engine = Engine::spawn_with_fake_gamepad(cfg.clone()).unwrap();
    let handle = engine.handle();

    // Invalid: min >= max.
    let err = set_settings_impl(&handle, cfg_path.path(), Settings {
        deadzone: 0.4,
        trigger_threshold: 0.5,
        min_press_ms: [25, 8],
        tick_jitter_ms: [0, 3],
        log_events: true,
    }).unwrap_err();
    assert!(format!("{err:#}").to_lowercase().contains("min_press_ms"),
            "error should name the bad field; got: {err:#}");

    // Valid: deadzone change.
    set_settings_impl(&handle, cfg_path.path(), Settings {
        deadzone: 0.6,
        trigger_threshold: 0.5,
        min_press_ms: [8, 25],
        tick_jitter_ms: [0, 3],
        log_events: true,
    }).unwrap();

    let live = handle.config_read();
    assert!((live.deadzone - 0.6).abs() < 1e-4, "deadzone hot-rebound; got {}", live.deadzone);
    drop(live);
    engine.shutdown();
}

#[test]
fn reset_settings_writes_factory_defaults() {
    let mut cfg = test_helpers::load_example();
    cfg.deadzone = 0.9;
    cfg.trigger_threshold = 0.9;
    cfg.min_press_ms = [40, 60];
    cfg.tick_jitter_ms = [5, 10];
    cfg.log_events = false;
    let cfg_path = test_helpers::tmp_config_with(&cfg);
    let engine = Engine::spawn_with_fake_gamepad(cfg.clone()).unwrap();
    let handle = engine.handle();

    reset_settings_impl(&handle, cfg_path.path()).unwrap();

    let live = handle.config_read();
    assert!((live.deadzone - 0.4).abs() < 1e-4, "deadzone reset; got {}", live.deadzone);
    assert!((live.trigger_threshold - 0.5).abs() < 1e-4);
    assert_eq!(live.min_press_ms, [8, 25]);
    assert_eq!(live.tick_jitter_ms, [0, 3]);
    assert_eq!(live.log_events, true);
    drop(live);
    engine.shutdown();
}
