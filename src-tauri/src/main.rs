// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(windows)]
    if let Ok(exe) = std::env::current_exe() {
        let old = exe.with_extension("old");
        let _ = std::fs::remove_file(old);
    }

    create_client_lib::run()
}
