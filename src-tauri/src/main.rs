// src-tauri/src/main.rs
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

// Entry point for the desktop application
fn main() {
    harvey_1_lib::run(); // Correctly calls the function in lib.rs
}