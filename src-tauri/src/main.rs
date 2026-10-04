#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

fn main() {
    if agenthub_core::git_auth::dispatch_helper() {
        return;
    }
    agenthub_desktop_lib::run();
}
