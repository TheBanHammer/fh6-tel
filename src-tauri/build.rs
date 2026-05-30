fn main() {
    // Run tauri's build-time codegen only for the desktop binary.
    if std::env::var_os("CARGO_FEATURE_DESKTOP").is_some() {
        tauri_build::build();
    }
}
