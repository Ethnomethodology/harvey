use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_shell::Shell;
use crate::welcome::config::{CommandError, get_config_dir};
use std::path::PathBuf;
use std::fs;

const ENV_DIR: &str = "harvey_env";

// --- Helper Functions ---

fn get_env_path() -> Result<PathBuf, CommandError> {
    get_config_dir().map(|path| path.join(ENV_DIR))
}

fn get_micromamba_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, CommandError> {
    let target_triple = tauri::utils::platform::target_triple().unwrap();
    let exe_suffix = if target_triple.contains("windows") { ".exe" } else { "" };
    let binary_name = format!("micromamba-{}{}", target_triple, exe_suffix);
    let resource_path = PathBuf::from("binaries").join(&binary_name);

    let resource_micromamba_path = app.path()
        .resolve(&resource_path, tauri::path::BaseDirectory::Resource)
        .map_err(|e| CommandError::Message(format!("Failed to resolve micromamba resource path: {}", e)))?;

    let app_cache_dir = app.path().app_cache_dir()
        .map_err(|e| CommandError::Message(format!("App cache directory not found: {}", e)))?;

    if !app_cache_dir.exists() {
        fs::create_dir_all(&app_cache_dir)
            .map_err(|e| CommandError::Message(format!("Failed to create app cache directory: {}", e)))?;
    }

    let dest_binary_name = format!("micromamba{}", exe_suffix);
    let dest_path = app_cache_dir.join(dest_binary_name);

    if !dest_path.exists() {
        log::info!("Copying micromamba from {:?} to {:?}", &resource_micromamba_path, &dest_path);
        fs::copy(&resource_micromamba_path, &dest_path)
            .map_err(|e| CommandError::Message(format!("Failed to copy micromamba: {}", e)))?;
    }

    Ok(dest_path)
}

pub fn get_python_path() -> Result<PathBuf, CommandError> {
    let env_path = get_env_path()?;
    if cfg!(windows) {
        Ok(env_path.join("Scripts").join("python.exe"))
    } else {
        Ok(env_path.join("bin").join("python"))
    }
}

// --- Main Installation Logic ---

pub async fn install_python_libraries<R: Runtime>(app: &AppHandle<R>, shell: &Shell<R>) -> Result<(), CommandError> {
    let emitter = app.clone();
    emitter.emit("installation-log", LogPayload { message: "Starting installation... Cleaning up previous attempts.".into() }).unwrap();

    let env_path = get_env_path()?;
    if env_path.exists() {
        log::info!("Removing existing environment at: {:?}", env_path);
        emitter.emit("installation-log", LogPayload { message: "Removing existing environment...".into() }).unwrap();
        std::fs::remove_dir_all(&env_path).map_err(|e| CommandError::Message(format!("Failed to remove existing env: {}", e)))?;
    }

    let micromamba_path = get_micromamba_path(app)?;
    log::info!("Using bundled micromamba at: {:?}", micromamba_path);
    emitter.emit("installation-log", LogPayload { message: "Found environment manager.".into() }).unwrap();

    // Step 1: Create environment with Python and pip
    emitter.emit("installation-log", LogPayload { message: "Creating Python environment...".into() }).unwrap();

    let create_args = vec![
        "create", "-p", env_path.to_str().unwrap(),
        "python=3.12", "pip", "pandoc", "ffmpeg", "--override-channels", "-c", "conda-forge", "-y",
    ];

    let (mut rx, _child) = shell.command(micromamba_path.to_str().unwrap())
        .args(&create_args)
        .env("PYTHONUNBUFFERED", "1")
        .spawn()?;
    
    let mut output_lines = Vec::new();
    while let Some(event) = rx.recv().await {
        match event {
            tauri_plugin_shell::process::CommandEvent::Stdout(line) => {
                let line_str = String::from_utf8_lossy(&line).to_string();
                emitter.emit("installation-log", LogPayload { message: line_str.clone() }).unwrap();
                output_lines.push(line_str);
            }
            tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                let line_str = String::from_utf8_lossy(&line).to_string();
                emitter.emit("installation-log", LogPayload { message: line_str.clone() }).unwrap();
                output_lines.push(line_str);
            }
            tauri_plugin_shell::process::CommandEvent::Terminated(payload) => {
                if payload.code != Some(0) {
                    let full_log = output_lines.join("\n");
                    let error_message = format!("Failed to create conda environment. Full log:\n{}", full_log);
                    emitter.emit("installation-log", LogPayload { message: error_message.clone() }).unwrap();
                    return Err(CommandError::Message(error_message));
                }
                break;
            }
            _ => {}
        }
    }

    // Step 2: Install packages using pip
    emitter.emit("installation-log", LogPayload { message: "Installing Python libraries...".into() }).unwrap();

    let pip_packages = [
        "torch==2.9.0", "torchvision==0.24.0", "torchaudio==2.9.0", "--extra-index-url", "https://download.pytorch.org/whl/cpu",
        "pyannote.audio==4.0.1", "pypandoc==1.15", "ffmpeg-python==0.2.0", "transformers==4.57.1", "sacremoses==0.1.1", "sentencepiece==0.2.1", "torchcodec==0.8.0"
    ];
    let mut pip_args = vec!["run", "-p", env_path.to_str().unwrap(), "pip", "install", "--no-cache-dir"];
    pip_args.extend(pip_packages.iter().map(|s| *s));

    let (mut rx_pip, _child_pip) = shell.command(micromamba_path.to_str().unwrap())
        .args(&pip_args)
        .env("PYTHONUNBUFFERED", "1")
        .spawn()?;

    let mut pip_output_lines = Vec::new();
    while let Some(event) = rx_pip.recv().await {
        match event {
            tauri_plugin_shell::process::CommandEvent::Stdout(line) => {
                let line_str = String::from_utf8_lossy(&line).to_string();
                emitter.emit("installation-log", LogPayload { message: line_str.clone() }).unwrap();
                pip_output_lines.push(line_str);
            }
            tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                let line_str = String::from_utf8_lossy(&line).to_string();
                emitter.emit("installation-log", LogPayload { message: line_str.clone() }).unwrap();
                pip_output_lines.push(line_str);
            }
            tauri_plugin_shell::process::CommandEvent::Terminated(payload) => {
                if payload.code != Some(0) {
                    let full_log = pip_output_lines.join("\n");
                    let error_message = format!("Failed to install pip packages. Full log:\n{}", full_log);
                    emitter.emit("installation-log", LogPayload { message: error_message.clone() }).unwrap();
                    return Err(CommandError::Message(error_message));
                }
                break;
            }
            _ => {}
        }
    }

    emitter.emit("installation-log", LogPayload { message: "Successfully installed Python libraries.".into() }).unwrap();
    emitter.emit("installation-log", LogPayload { message: "Installation complete.".into() }).unwrap();
    emitter.emit("installation-finished", ()).unwrap();
    Ok(())
}

pub async fn check_python_libraries_installed<R: Runtime>(
    app: &AppHandle<R>,
    shell: &Shell<R>,
) -> Result<bool, CommandError> {
    let env_path = get_env_path()?;
    if !env_path.exists() {
        log::info!("Python env does not exist at {:?}", env_path);
        return Ok(false);
    }

    let python_path = get_python_path()?;
    let packages = ["pyannote.audio", "transformers", "sacremoses", "sentencepiece", "torchcodec", "pypandoc"];

    for package in &packages {
        log::info!("Checking for package: {}", package);
        let import_name = if package == &"pyannote.audio" { "pyannote" } else { package };

        let mut command = shell.command(python_path.to_str().unwrap());

        if cfg!(target_os = "macos") {
            let resource_dir = app.path().resource_dir().map_err(|e| CommandError::Message(format!("Resource dir not found: {}", e)))?;
            let sidecars_path = resource_dir.join("sidecars");
            
            let mut dyld_paths = Vec::new();
            if sidecars_path.exists() {
                dyld_paths.push(sidecars_path.to_string_lossy().to_string());
            }

            let env_lib_path = env_path.join("lib");
            if env_lib_path.exists() {
                dyld_paths.push(env_lib_path.to_string_lossy().to_string());
            }

            if !dyld_paths.is_empty() {
                let dyld_path_str = dyld_paths.join(":");
                log::info!("Setting DYLD_LIBRARY_PATH to: {}", dyld_path_str);
                command = command.env("DYLD_LIBRARY_PATH", dyld_path_str);
            }
        } else if cfg!(target_os = "linux") {
            let resource_dir = app.path().resource_dir().map_err(|e| CommandError::Message(format!("Resource dir not found: {}", e)))?;
            let sidecars_path = resource_dir.join("sidecars");

            let mut ld_paths = Vec::new();
            if sidecars_path.exists() {
                ld_paths.push(sidecars_path.to_string_lossy().to_string());
            }

            let env_lib_path = env_path.join("lib");
            if env_lib_path.exists() {
                ld_paths.push(env_lib_path.to_string_lossy().to_string());
            }

            if !ld_paths.is_empty() {
                let ld_path_str = ld_paths.join(":
");
                log::info!("Setting LD_LIBRARY_PATH to: {}", ld_path_str);
                command = command.env("LD_LIBRARY_PATH", ld_path_str);
            }
        }
        
        let output = command
            .args(&["-c", &format!("import {}", import_name)])
            .output()
            .await?;
        
        if !output.status.success() {
            log::warn!("Package '{}' not found.", package);
            log::warn!("Import check stdout: {}", String::from_utf8_lossy(&output.stdout));
            log::warn!("Import check stderr: {}", String::from_utf8_lossy(&output.stderr));
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

// Temporary command for diagnostics
#[tauri::command]
pub async fn list_venv_lib_contents() -> Result<Vec<String>, String> {
    let env_path = get_env_path().map_err(|e| e.to_string())?;
    let lib_path = if cfg!(windows) {
        env_path.join("Lib").join("site-packages")
    } else {
        let python_version_dir = fs::read_dir(env_path.join("lib"))
            .map_err(|e| format!("Failed to read env/lib directory: {}", e))?
            .filter_map(Result::ok)
            .find(|entry| entry.file_name().to_string_lossy().starts_with("python"));
        
        if let Some(dir) = python_version_dir {
            dir.path().join("site-packages")
        } else {
            return Err("Could not find python version directory in env/lib".to_string());
        }
    };

    if !lib_path.exists() {
        return Err(format!("Conda environment 'site-packages' directory not found at {:?}", lib_path));
    }

    let entries = fs::read_dir(&lib_path)
        .map_err(|e| format!("Failed to read 'site-packages' directory: {}", e))?
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                e.file_name().into_string().ok()
            })
        })
        .collect();

    Ok(entries)
}

#[tauri::command]
pub async fn delete_virtual_env() -> Result<(), String> {
    let env_path = get_env_path().map_err(|e| e.to_string())?;
    if env_path.exists() {
        log::info!("Deleting environment at: {:?}", env_path);
        std::fs::remove_dir_all(&env_path)
            .map_err(|e| format!("Failed to delete environment: {}", e))?;
        log::info!("Environment deleted successfully.");
    }
    Ok(())
}