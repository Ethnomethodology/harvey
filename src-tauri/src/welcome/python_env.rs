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
    let packages = ["pyannote.audio", "transformers", "sacremoses"];
    for package in &packages {
        log::info!("Checking for package: {}", package);
        let output = shell.command(python_path.to_str().unwrap())
            .args(&["-c", &format!("import {}", package)])
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

pub async fn install_python_libraries<R: Runtime>(app: &AppHandle<R>, shell: &Shell<R>) -> Result<(), CommandError> {
    let venv_path = get_venv_path()?;
    let python3_command = if cfg!(windows) { "python" } else { "python3" };

    if !venv_path.exists() {
        app.emit("installation-log", LogPayload { message: "Creating virtual environment...".into() }).unwrap();
        let output = shell.command(python3_command)
            .args(&["-m", "venv", venv_path.to_str().unwrap()])
            .output()
            .await?;
        if !output.status.success() {
            let error_message = format!("Failed to create virtual environment: {}", String::from_utf8_lossy(&output.stderr));
            app.emit("installation-log", LogPayload { message: error_message.clone() }).unwrap();
            return Err(CommandError::Message(error_message));
        }
         app.emit("installation-log", LogPayload { message: "Virtual environment created successfully.".into() }).unwrap();
    }

    let python_path = get_python_path()?;
    let packages = ["pyannote.audio", "transformers", "sacremoses"];
    for package in &packages {
        app.emit("installation-log", LogPayload { message: format!("Installing {}...", package) }).unwrap();
        let output = shell.command(python_path.to_str().unwrap())
            .args(&["-m", "pip", "install", package])
            .output()
            .await?;
        if !output.status.success() {
            let error_message = format!("Failed to install {}: {}", package, String::from_utf8_lossy(&output.stderr));
            app.emit("installation-log", LogPayload { message: error_message.clone() }).unwrap();
            return Err(CommandError::Message(error_message));
        }
        app.emit("installation-log", LogPayload { message: format!("Successfully installed {}.", package) }).unwrap();
    }

    app.emit("installation-log", LogPayload { message: "Installation complete.".into() }).unwrap();
    Ok(())
}
