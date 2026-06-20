// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // VelopackApp must run before anything else: on install/update/uninstall it may
    // perform hook tasks and terminate/restart the process. No-op for normal launches.
    velopack::VelopackApp::build().run();

    translator_desktop_lib::run()
}
