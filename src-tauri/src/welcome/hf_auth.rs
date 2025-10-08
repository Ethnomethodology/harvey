// src-tauri/src/welcome/hf_auth.rs
use tauri::Manager;
use std::fs;
use std::path::PathBuf;

// Function to get the path to the token file
fn get_token_path(app_handle: &tauri::AppHandle) -> PathBuf {
    let config_dir = app_handle.path().app_config_dir().unwrap();
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).unwrap();
    }
    config_dir.join("hf_token")
}

#[tauri::command]
pub fn check_hf_auth_status(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let token_path = get_token_path(&app_handle);
    Ok(token_path.exists() && fs::read_to_string(token_path).map_or(false, |s| !s.is_empty()))
}

#[tauri::command]
pub fn save_hf_auth_token(app_handle: tauri::AppHandle, token: String) -> Result<(), String> {
    if token.is_empty() {
        return Err("Token cannot be empty".into());
    }
    let token_path = get_token_path(&app_handle);
    fs::write(token_path, token).map_err(|e| e.to_string())
}
