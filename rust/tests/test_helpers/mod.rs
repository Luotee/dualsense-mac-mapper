//! Shared helpers for integration tests under `rust/tests/`.
//!
//! Each integration test crate that needs these imports `mod test_helpers;`
//! at the top. Cargo wires the `mod.rs` automatically when it finds a
//! `test_helpers/` directory alongside the test files.

use dualsense_mapper::config::Config;
use tempfile::NamedTempFile;

pub fn load_example() -> Config {
    // examples/maple_artale.json is a hand-crafted full-25-button config
    // shipped in the v0.1.x release. Tests use it as a known-good baseline.
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/maple_artale.json");
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("examples/maple_artale.json must exist at {}", path.display()));
    Config::load_from_str(&json).expect("example config must parse + validate")
}

pub fn tmp_config_with(cfg: &Config) -> NamedTempFile {
    let f = NamedTempFile::new().expect("temp file");
    // Write the typed config as pretty JSON. ConfigDoc::load expects valid JSON
    // with all required fields; serde_json::to_string_pretty(cfg) yields that.
    let json = serde_json::to_string_pretty(cfg).expect("serialise");
    std::fs::write(f.path(), json).expect("write");
    f
}
