// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(feature = "desktop")]
fn main() {
    fh6_tel_lib::run()
}

#[cfg(not(feature = "desktop"))]
fn main() {
    eprintln!("This binary was built without the `desktop` feature. Use fh6-tel-serve instead.");
}
