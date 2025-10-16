// src-tauri/src/projectview/utils.rs

use tauri::{AppHandle, Manager, Runtime};
use std::path::PathBuf;

/// Resolves the path to the bundled ffmpeg executable.
pub fn get_ffmpeg_path<R: Runtime>(app_handle: &AppHandle<R>) -> Result<PathBuf, String> {
    let file_name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    let resource_path = PathBuf::from("sidecars").join(file_name);

    app_handle
        .path()
        .resolve(&resource_path, tauri::path::BaseDirectory::Resource)
        .map_err(|e| format!("Failed to resolve ffmpeg resource path: {}", e))
}

/// Resolves the path to the bundled ffprobe executable.
pub fn get_ffprobe_path<R: Runtime>(app_handle: &AppHandle<R>) -> Result<PathBuf, String> {
    let file_name = if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" };
    let resource_path = PathBuf::from("sidecars").join(file_name);

    app_handle
        .path()
        .resolve(&resource_path, tauri::path::BaseDirectory::Resource)
        .map_err(|e| format!("Failed to resolve ffprobe resource path: {}", e))
}
