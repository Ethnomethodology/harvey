use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_shell::Shell;
use crate::welcome::config::{CommandError, get_config_dir};

const VENV_DIR: &str = "harvey_env";

fn get_venv_path() -> Result<PathBuf, CommandError> {
    get_config_dir().map(|path| path.join(VENV_DIR))
}

pub fn get_python_path() -> Result<PathBuf, CommandError> {
    let venv_path = get_venv_path()?;
    if cfg!(windows) {
        Ok(venv_path.join("Scripts").join("python.exe"))
    } else {
        Ok(venv_path.join("bin").join("python"))
    }
}

pub async fn check_python_libraries_installed<R: Runtime>(shell: &Shell<R>) -> Result<bool, CommandError> {
    let venv_path = get_venv_path()?;
    if !venv_path.exists() {
        log::info!("Python venv does not exist at {:?}", venv_path);
        return Ok(false);
    }

    let python_path = get_python_path()?;
    let packages = ["pyannote.audio", "transformers", "sacremoses", "sentencepiece"];
    for package in &packages {
        log::info!("Checking for package: {}", package);
        let import_name = if package == &"pyannote.audio" { "pyannote" } else { package };
        let output = shell.command(python_path.to_str().unwrap())
            .args(&["-c", &format!("import {}", import_name)])
            .output()
            .await?;
        if !output.status.success() {
            log::warn!("Package '{}' not found.", package);
            return Ok(false);
        }
    }
    log::info!("All required Python libraries are installed.");
    Ok(true)
}

#[derive(Clone, serde::Serialize)]
struct LogPayload {
  message: String,
}

use std::fs;

// Temporary command for diagnostics
#[tauri::command]
pub async fn list_venv_lib_contents() -> Result<Vec<String>, String> {
    let venv_path = get_venv_path().map_err(|e| e.to_string())?;
    let lib_path = venv_path.join("lib");

    if !lib_path.exists() {
        return Err("Virtual environment 'lib' directory not found.".to_string());
    }

    let entries = fs::read_dir(lib_path)
        .map_err(|e| format!("Failed to read 'lib' directory: {}", e))?
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                e.file_name().into_string().ok()
            })
        })
        .collect();

    Ok(entries)
}


pub async fn install_python_libraries<R: Runtime>(app: &AppHandle<R>, shell: &Shell<R>) -> Result<(), CommandError> {
    let venv_path = get_venv_path()?;
    let python3_command = if cfg!(windows) { "python" } else { "python3" };

    if !venv_path.exists() {
        app.emit("installation-log", LogPayload { message: "Creating virtual environment...".into() }).unwrap();
        let (mut rx, _child) = shell.command(python3_command)
            .args(&["-m", "venv", venv_path.to_str().unwrap()])
            .env("PYTHONUNBUFFERED", "1")
            .spawn()?;

        while let Some(event) = rx.recv().await {
            match event {
                tauri_plugin_shell::process::CommandEvent::Stdout(line) => {
                    app.emit("installation-log", LogPayload { message: String::from_utf8_lossy(&line).to_string() }).unwrap();
                }
                tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                    app.emit("installation-log", LogPayload { message: String::from_utf8_lossy(&line).to_string() }).unwrap();
                }
                tauri_plugin_shell::process::CommandEvent::Terminated(payload) => {
                    if payload.code != Some(0) {
                        let error_message = format!("Failed to create virtual environment.");
                        app.emit("installation-log", LogPayload { message: error_message.clone() }).unwrap();
                        return Err(CommandError::Message(error_message));
                    }
                    break;
                }
                _ => {}
            }
        }
        app.emit("installation-log", LogPayload { message: "Virtual environment created successfully.".into() }).unwrap();
    }

    let python_path = get_python_path()?;

    // Consolidate all pip installations into a single command.
    // This addresses the UI hang and potential `@rpath` issues on macOS by installing `ffmpeg-python`
    // in the same transaction as the libraries that depend on it (like `torchcodec`).
    let packages = [
        "--upgrade", "pip",
        "ffmpeg-python",
        "torch",
        "torchcodec",
        "pyannote.audio",
        "transformers",
        "sacremoses",
        "sentencepiece",
        "pypandoc",
    ];
    app.emit("installation-log", LogPayload { message: "Installing Python libraries...".into() }).unwrap();

    let mut pip_args = vec!["-m", "pip", "install"];
    pip_args.extend_from_slice(&packages);

    // --- Add cache directory to pip command ---
    let pip_cache_dir: Option<PathBuf> = if cfg!(target_os = "windows") {
        std::env::var("LOCALAPPDATA").ok().map(|p| PathBuf::from(p).join("pip").join("cache"))
    } else if cfg!(target_os = "macos") {
        std::env::var("HOME").ok().map(|p| PathBuf::from(p).join("Library").join("Caches").join("pip"))
    } else { // Assuming Linux-like
        std::env::var("HOME").ok().map(|p| PathBuf::from(p).join(".cache").join("pip"))
    };

    let cache_dir_arg_storage;
    if let Some(cache_dir) = pip_cache_dir {
        if cache_dir.exists() {
            app.emit("installation-log", LogPayload { message: format!("Using pip cache: {:?}", cache_dir).into() }).unwrap();
            cache_dir_arg_storage = cache_dir.to_string_lossy().into_owned();
            pip_args.push("--cache-dir");
            pip_args.push(&cache_dir_arg_storage);
        }
    }

    let (mut rx, _child) = shell.command(python_path.to_str().unwrap())
        .args(&pip_args)
        .env("PYTHONUNBUFFERED", "1")
        .spawn()?;

    while let Some(event) = rx.recv().await {
        match event {
            tauri_plugin_shell::process::CommandEvent::Stdout(line) => {
                app.emit("installation-log", LogPayload { message: String::from_utf8_lossy(&line).to_string() }).unwrap();
            }
            tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                app.emit("installation-log", LogPayload { message: String::from_utf8_lossy(&line).to_string() }).unwrap();
            }
            tauri_plugin_shell::process::CommandEvent::Terminated(payload) => {
                if payload.code != Some(0) {
                    let error_message = "Failed to install Python libraries: pip install failed".to_string();
                    app.emit("installation-log", LogPayload { message: error_message.clone() }).unwrap();
                    return Err(CommandError::Message(error_message));
                }
                break;
            }
            _ => {}
        }
    }
    app.emit("installation-log", LogPayload { message: "Successfully installed Python libraries.".into() }).unwrap();

    app.emit("installation-log", LogPayload { message: "Installation complete.".into() }).unwrap();
    app.emit("installation-finished", ()).unwrap();
    Ok(())
}

#[tauri::command]
pub async fn delete_virtual_env() -> Result<(), String> {
    let venv_path = get_venv_path().map_err(|e| e.to_string())?;
    if venv_path.exists() {
        log::info!("Deleting virtual environment at: {:?}", venv_path);
        std::fs::remove_dir_all(&venv_path)
            .map_err(|e| format!("Failed to delete virtual environment: {}", e))?;
        log::info!("Virtual environment deleted successfully.");
    }
    Ok(())
}