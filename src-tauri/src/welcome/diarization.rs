// src-tauri/src/welcome/diarization.rs
use super::python_env;
use std::fs;
use crate::welcome::config::{read_config, write_config};

use tauri::{AppHandle, Emitter, Manager, Runtime};

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

// Helper to get the custom diarization hub path
pub fn get_diarization_hub_path<R: Runtime>(app_handle: &AppHandle<R>) -> Result<std::path::PathBuf, String> {
    let config = read_config().map_err(|e| e.to_string())?;
    let base_path = std::path::PathBuf::from(config.download_location);
    Ok(base_path.join("diarization"))
}

#[tauri::command]
pub async fn check_diarization_model_access<R: Runtime>(
    app_handle: AppHandle<R>,
) -> Result<bool, String> {
    let script_path = app_handle
        .path()
        .resolve("scripts/verify_diarization_model.py", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?;

    let hf_home = get_diarization_hub_path(&app_handle)?;

    let output = python_env::get_python_command(&app_handle).map_err(|e| e.to_string())?
        .args(&[script_path.to_string_lossy().to_string()])
        .env("HF_HOME", hf_home.to_string_lossy().to_string())
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        // If it fails to run the script, it's not cached or verified
        return Ok(false);
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(stdout == "verified")
}

#[tauri::command]
pub async fn check_gated_model_access<R: Runtime>(
    _app_handle: AppHandle<R>,
    token: String,
) -> Result<bool, String> {
    log::info!("Checking HF token validity and gated model access");
    
    let client = reqwest::Client::new();
    
    // 1. Verify token validity
    let whoami_url = "https://huggingface.co/api/whoami-v2";
    let whoami_res = client.get(whoami_url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error checking HF token: {}", e))?;

    if !whoami_res.status().is_success() {
        log::warn!("HF token verification failed (Status {})", whoami_res.status());
        return Err("Invalid Hugging Face token. Please check your token and try again.".to_string());
    }

    // 2. Check model access
    let url = "https://huggingface.co/api/models/pyannote/speaker-diarization-3.1";
    let res = client.get(url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error checking HF model access: {}", e))?;

    if res.status().is_success() {
        log::info!("HF Access granted (Status 200)");
        Ok(true)
    } else {
        let status = res.status().as_u16();
        log::warn!("HF Model access denied (Status {})", status);
        if status == 403 {
            return Err("Access to pyannote/speaker-diarization-3.1 is restricted. Please ensure you have accepted the license on Hugging Face.".to_string());
        }
        Ok(false)
    }
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
    let hf_home = get_diarization_hub_path(&app_handle)?;

    // Ensure the custom directory exists
    fs::create_dir_all(&hf_home).map_err(|e| format!("Failed to create diarization directory: {}", e))?;

    let script_path = app_handle
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("scripts/download_diarization_model.py");

    app_handle.emit("diarization-installation-log", LogPayload { message: "Starting diarization model download...".into() }).unwrap();

    let (mut rx, _child) = python_env::get_python_command(&app_handle).map_err(|e| e.to_string())?
        .args(&[script_path.to_string_lossy().to_string(), token.clone()])
        .env("HF_HUB_DISABLE_PROGRESS_BARS", "0")
        .env("HF_HOME", hf_home.to_string_lossy().to_string())
        .spawn()
        .map_err(|e| e.to_string())?;

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
    let script_path = app_handle
        .path()
        .resolve("scripts/get_diarization_cache_path.py", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?;

    let hf_home = get_diarization_hub_path(&app_handle)?;

    let output = python_env::get_python_command(&app_handle).map_err(|e| e.to_string())?
        .args(&[script_path.to_string_lossy().to_string()])
        .env("HF_HOME", hf_home.to_string_lossy().to_string())
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[tauri::command]
pub async fn delete_diarization_model<R: Runtime>(
    app_handle: AppHandle<R>,
) -> Result<(), String> {
    // Use the dynamic cache path from the helper function
    let cache_path_str = get_diarization_cache_path(app_handle.clone()).await?;
    let hf_hub_path = std::path::PathBuf::from(cache_path_str);

    if !hf_hub_path.exists() {
        log::info!("HuggingFace hub directory not found at {:?}, nothing to delete.", hf_hub_path);
        return Ok(());
    }

    log::info!("Searching for pyannote models in: {:?}", hf_hub_path);

    let entries = fs::read_dir(&hf_hub_path)
        .map_err(|e| format!("Failed to read HuggingFace hub directory: {}", e))?;

    let mut model_deleted = false;
    for entry in entries {
        if let Ok(entry) = entry {
            if let Some(file_name) = entry.file_name().to_str() {
                // Delete all pyannote related models (diarization, segmentation, etc.)
                if file_name.starts_with("models--pyannote--") {
                    log::info!("Deleting model directory: {:?}", entry.path());
                    if let Err(e) = fs::remove_dir_all(entry.path()) {
                        log::error!("Failed to delete directory '{:?}': {}", file_name, e);
                    } else {
                        model_deleted = true;
                    }
                }
            }
        }
    }

    if model_deleted {
        let mut config = read_config().map_err(|e| e.to_string())?;
        config.verification_status.diarization_model_verified = false;
        write_config(&config).map_err(|e| e.to_string())?;
    }

    Ok(())
}
