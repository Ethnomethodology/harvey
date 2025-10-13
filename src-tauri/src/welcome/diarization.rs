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



    let mut command = shell.command(python_path.to_string_lossy().to_string());

    command = command.args(&[script_path.to_string_lossy().to_string()]);



    // On macOS, we need to set the `DYLD_LIBRARY_PATH` to include our bundled ffmpeg libs

        if cfg!(target_os = "macos") {

        if let Ok(resource_dir) = app_handle.path().resource_dir() {

                let ffmpeg_lib_path = resource_dir.join("sidecars");

                if ffmpeg_lib_path.exists() {

                    let ffmpeg_path_str = ffmpeg_lib_path.to_string_lossy();

                    if let Some(existing_path) = std::env::var("DYLD_LIBRARY_PATH").ok() {

                        command = command.env("DYLD_LIBRARY_PATH", format!("{}:{}", ffmpeg_path_str, existing_path));

                    } else {

                        command = command.env("DYLD_LIBRARY_PATH", ffmpeg_path_str.to_string());

                    }

                }

            }

        }



    let output = command.output().await.map_err(|e| e.to_string())?;



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
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("scripts/download_diarization_model.py");

    app_handle.emit("diarization-installation-log", LogPayload { message: "Starting diarization model download...".into() }).unwrap();

    let mut command = shell.command(python_path.to_string_lossy().to_string());
    command = command.args(&[script_path.to_string_lossy().to_string(), token.clone()]);
    command = command.env("HF_HUB_DISABLE_PROGRESS_BARS", "1");

    // On macOS, we need to set the `DYLD_LIBRARY_PATH` to include our bundled ffmpeg libs
    // and the python venv libs, so torchcodec can find everything.
    if cfg!(target_os = "macos") {
        let mut new_paths = Vec::new();

        // 1. Add bundled ffmpeg path
        if let Ok(resource_dir) = app_handle.path().resource_dir() {
            let ffmpeg_lib_path = resource_dir.join("sidecars");
            if ffmpeg_lib_path.exists() {
                new_paths.push(ffmpeg_lib_path.to_string_lossy().to_string());
            }
        }

        // 2. Add venv lib path
        if let Some(venv_dir) = python_path.parent().and_then(|p| p.parent()) {
            new_paths.push(venv_dir.join("lib").to_string_lossy().to_string());
        }

        // 3. Prepend to existing path if it exists
        if let Ok(existing_path) = std::env::var("DYLD_LIBRARY_PATH") {
            if !existing_path.is_empty() {
                new_paths.push(existing_path);
            }
        }
        
        if !new_paths.is_empty() {
            command = command.env("DYLD_LIBRARY_PATH", new_paths.join(":"));
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

#[tauri::command]
pub async fn delete_diarization_model() -> Result<(), String> {
    let hf_hub_path = dirs::home_dir()
        .ok_or_else(|| "Could not find home directory".to_string())?
        .join(".cache").join("huggingface").join("hub");

    if !hf_hub_path.exists() {
        log::info!("HuggingFace hub directory not found, nothing to delete.");
        return Ok(());
    }

    log::info!("Searching for pyannote models in: {:?}", hf_hub_path);

    let entries = fs::read_dir(&hf_hub_path)
        .map_err(|e| format!("Failed to read HuggingFace hub directory: {}", e))?;

    for entry in entries {
        if let Ok(entry) = entry {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.starts_with("models--pyannote--") {
                    log::info!("Deleting model directory: {:?}", entry.path());
                    if let Err(e) = fs::remove_dir_all(entry.path()) {
                        log::error!("Failed to delete directory '{:?}': {}", file_name, e);
                        // Don't return early, try to delete other matches
                    }
                }
            }
        }
    }

    Ok(())
}
