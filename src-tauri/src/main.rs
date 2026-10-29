//! The desktops' start: the app itself is in src/lib.rs, shared with the
//! phones.

// A release build on Windows opens no console window beside the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    app_native_lib::run();
}
