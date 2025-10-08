// src-tauri/src/welcome/diarization.rs
use super::python_env::get_python_path;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_shell::ShellExt;
use dirs;

// Helper to read the HuggingFace token
fn get_hf_token<R: Runtime>(app_handle: &AppHandle<R>) -> Result<String, String> {
    let config_dir = app_handle
        .path()
        .app_config_dir()
        .expect("Failed to get app config dir");
    let token_path = config_dir.join("hf_token");
    fs::read_to_string(token_path).map_err(|e| {
        format!(
            "Failed to read token: {}. Please ensure you have saved your HuggingFace token.",
            e
        )
    })
}

// Heuristic to find the model directory within the HuggingFace cache
fn get_diarization_model_path() -> Result<PathBuf, String> {
    let cache_dir = dirs::home_dir()
        .ok_or("Could not find home directory")?
        .join(".cache")
        .join("huggingface")
        .join("hub");

    if cache_dir.exists() {
        for entry in fs::read_dir(cache_dir).map_err(|e| e.to_string())? {
            if let Ok(entry) = entry {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with("models--pyannote--speaker-diarization-3.1") {
                        return Ok(entry.path());
                    }
                }
            }
        }
    }
    Err("Model directory not found in cache.".to_string())
}

#[tauri::command]
pub fn check_diarization_model_access() -> Result<bool, String> {
    match get_diarization_model_path() {
        Ok(model_path) => Ok(model_path.join("config.yaml").exists()),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
pub async fn download_diarization_model<R: Runtime>(
    app_handle: AppHandle<R>,
) -> Result<(), String> {
    let token = get_hf_token(&app_handle)?;
    let python_path = get_python_path().map_err(|e| e.to_string())?;
    let shell = app_handle.shell();

    let script_path = app_handle
        .path()
        .resolve("scripts/download_diarization_model.py", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?;

    let output = shell
        .command(python_path.to_string_lossy().to_string())
        .args(&[script_path.to_string_lossy().to_string(), token])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Ok(())
}

#[tauri::command]
pub async fn get_diarization_cache_path<R: Runtime>(
    app_handle: AppHandle<R>,
) -> Result<String, String> {
    let python_path = get_python_path().map_err(|e| e.to_string())?;
    let shell = app_handle.shell();

    let script_path = app_handle
        .path()
        .resolve("scripts/get_diarization_cache_path.py", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?;

    let output = shell
        .command(python_path.to_string_lossy().to_string())
        .args(&[script_path.to_string_lossy().to_string()])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
