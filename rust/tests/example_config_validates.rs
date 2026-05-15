use dualsense_mapper::config::Config;
use std::path::PathBuf;

#[test]
fn shipped_example_config_validates() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config.example.json");
    let cfg = Config::load_from_path(&path).expect("load+validate");
    assert_eq!(cfg.version, 1);
    assert_eq!(cfg.buttons.len(), 25);
    // L2 must be a macro
    assert!(matches!(
        cfg.buttons["23"].binding,
        dualsense_mapper::config::Binding::Macro(_)
    ));
}

#[test]
fn shipped_maple_profile_validates() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/maple_artale.json");
    Config::load_from_path(&path).expect("maple profile load+validate");
}
