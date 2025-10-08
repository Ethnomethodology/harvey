// src-tauri/src/welcome/diarization.rs
use super::python_env::get_python_path;
use std::fs;

use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_shell::ShellExt;

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

#[tauri::command]
pub async fn check_diarization_model_access<R: Runtime>(
    app_handle: AppHandle<R>,
) -> Result<bool, String> {
    let python_path = get_python_path().map_err(|e| e.to_string())?;
    let shell = app_handle.shell();

    let script_path = app_handle
        .path()
        .resolve("scripts/check_model_cached.py", tauri::path::BaseDirectory::Resource)
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

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(stdout == "cached")
}

#[derive(Clone, serde::Serialize)]
struct LogPayload {
  message: String,
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

    app_handle.emit("diarization-installation-log", LogPayload { message: "Starting diarization model download...".into() }).unwrap();

    let mut command = shell.command(python_path.to_string_lossy().to_string());
    command = command.args(&[script_path.to_string_lossy().to_string(), token.clone()]);

    // On macOS, we need to set the `DYLD_LIBRARY_PATH` to include the venv's `lib` directory.
    // This is because `torchcodec` and other libraries with native extensions may not be able to
    // find their dependencies otherwise, especially when the app is bundled.
    if cfg!(target_os = "macos") {
        if let Some(venv_dir) = python_path.parent().and_then(|p| p.parent()) {
            let lib_path = venv_dir.join("lib");
            if let Some(existing_path) = std::env::var("DYLD_LIBRARY_PATH").ok() {
                command = command.env("DYLD_LIBRARY_PATH", format!("{}:{}", lib_path.to_string_lossy(), existing_path));
            } else {
                command = command.env("DYLD_LIBRARY_PATH", lib_path.to_string_lossy().to_string());
            }
        }
    }

    let (mut rx, _child) = command.spawn().map_err(|e| e.to_string())?;

    let mut success = false;
    while let Some(event) = rx.recv().await {
        match event {
            tauri_plugin_shell::process::CommandEvent::Stdout(line) => {
                app_handle.emit("diarization-installation-log", LogPayload { message: String::from_utf8_lossy(&line).to_string() }).unwrap();
            }
            tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                app_handle.emit("diarization-installation-log", LogPayload { message: String::from_utf8_lossy(&line).to_string() }).unwrap();
            }
            tauri_plugin_shell::process::CommandEvent::Terminated(payload) => {
                if payload.code == Some(0) {
                    app_handle.emit("diarization-installation-log", LogPayload { message: "Diarization model downloaded successfully.".into() }).unwrap();
                    success = true;
                } else {
                    app_handle.emit("diarization-installation-log", LogPayload { message: "Diarization model download failed.".into() }).unwrap();
                }
                break;
            }
            _ => {}
        }
    }

    app_handle.emit("diarization-installation-finished", ()).unwrap();

    if success {
        Ok(())
    } else {
        Err("Diarization model download failed.".to_string())
    }
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
