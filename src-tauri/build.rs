fn main() {
    // tauri-build is an optional build-dep enabled only by the `desktop` feature.
    if std::env::var_os("CARGO_FEATURE_DESKTOP").is_some() {
        tauri_build::build();
    }
}
