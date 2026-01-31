// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use is_elevated::is_elevated;
use std::env;
use std::process;

fn main() {
    if !is_elevated() {
        if let Ok(exe) = env::current_exe() {
            let _ = runas::Command::new(exe)
                .args(&env::args().skip(1).collect::<Vec<_>>())
                .status();
        }
        process::exit(0);
    }
    zust_lib::run();
}
