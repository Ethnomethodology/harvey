// src-tauri/src/projectview/utils.rs

use tauri::{AppHandle, Runtime};
use std::path::PathBuf;

use crate::welcome::python_env::get_env_path; // Add this import

/// Resolves the path to the ffmpeg executable from the Conda environment.
pub fn get_ffmpeg_path<R: Runtime>(_app_handle: &AppHandle<R>) -> Result<PathBuf, String> {
    let env_path = get_env_path().map_err(|e| format!("Failed to get Python environment path: {}", e))?;
    let file_name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };

    let ffmpeg_path = if cfg!(windows) {
        env_path.join("Library").join("bin").join(file_name)
    } else {
        env_path.join("bin").join(file_name)
    };

    if !ffmpeg_path.exists() {
        return Err(format!("FFmpeg executable not found at: {}", ffmpeg_path.display()));
    }

    Ok(ffmpeg_path)
}

/// Resolves the path to the ffprobe executable from the Conda environment.
pub fn get_ffprobe_path<R: Runtime>(_app_handle: &AppHandle<R>) -> Result<PathBuf, String> {
    let env_path = get_env_path().map_err(|e| format!("Failed to get Python environment path: {}", e))?;
    let file_name = if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" };

    let ffprobe_path = if cfg!(windows) {
        env_path.join("Library").join("bin").join(file_name)
    } else {
        env_path.join("bin").join(file_name)
    };

    if !ffprobe_path.exists() {
        return Err(format!("FFprobe executable not found at: {}", ffprobe_path.display()));
    }

    Ok(ffprobe_path)
}
