use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_shell::{Shell, ShellExt};
use crate::welcome::config::{CommandError, get_config_dir, read_config, write_config};
use std::path::{PathBuf};
use std::fs::{self};
// use std::io;
// use reqwest;


const ENV_DIR: &str = "harvey_env";

// --- Helper Functions ---

pub fn get_env_path() -> Result<PathBuf, CommandError> {
    get_config_dir().map(|path| path.join(ENV_DIR))
}



pub fn get_python_path() -> Result<PathBuf, CommandError> {
    let env_path = get_env_path()?;
    if cfg!(windows) {
        // Micromamba on windows
        Ok(env_path.join("python.exe"))
    } else {
        // Micromamba on unix
        Ok(env_path.join("bin").join("python"))
    }
}

#[derive(PartialEq, Debug)]
enum PyTorchInstallStrategy {
    Gpu,
    Cpu,
}

async fn get_pytorch_install_strategy<R: Runtime>(shell: &Shell<R>) -> PyTorchInstallStrategy {
    let target_triple = tauri::utils::platform::target_triple().unwrap_or_default();

    if target_triple.contains("apple-darwin") {
        if target_triple.contains("aarch64") {
            log::info!("Detected Apple Silicon, enabling GPU (MPS) support for PyTorch.");
            return PyTorchInstallStrategy::Gpu;
        }
    } else if target_triple.contains("pc-windows-msvc") {
        log::info!("Checking for NVIDIA GPU with command: nvidia-smi.exe");
        if let Ok(output) = shell.command("nvidia-smi.exe").output().await {
            if output.status.success() {
                log::info!("nvidia-smi found, enabling GPU (CUDA) support for PyTorch.");
                log::debug!("nvidia-smi stdout: {}", String::from_utf8_lossy(&output.stdout));
                return PyTorchInstallStrategy::Gpu;
            }
            log::info!("nvidia-smi not found or failed, defaulting to CPU. Stderr: {}", String::from_utf8_lossy(&output.stderr));
        } else {
            log::info!("Error executing nvidia-smi, defaulting to CPU.");
        }
    } else if target_triple.contains("unknown-linux-gnu") {
        log::info!("Checking for NVIDIA GPU with command: nvidia-smi");
        if let Ok(output) = shell.command("nvidia-smi").output().await {
            if output.status.success() {
                log::info!("nvidia-smi found, enabling GPU (CUDA) support for PyTorch.");
                log::debug!("nvidia-smi stdout: {}", String::from_utf8_lossy(&output.stdout));
                return PyTorchInstallStrategy::Gpu;
            }
            log::info!("nvidia-smi not found or failed, defaulting to CPU. Stderr: {}", String::from_utf8_lossy(&output.stderr));
        } else {
            log::info!("Error executing nvidia-smi, defaulting to CPU.");
        }
    }

    log::info!("Defaulting to CPU-only PyTorch installation.");
    PyTorchInstallStrategy::Cpu
}



// --- Main Installation Logic ---

pub async fn install_python_libraries<R: Runtime>(app: &AppHandle<R>, shell: &Shell<R>) -> Result<(), CommandError> {
    install_python_libraries_micromamba(app, shell).await
}



#[allow(unused_variables)]
async fn install_python_libraries_micromamba<R: Runtime>(app: &AppHandle<R>, shell: &Shell<R>) -> Result<(), CommandError> {
    let emitter = app.clone();
    emitter.emit("installation-log", LogPayload { message: "Starting installation... Cleaning up previous attempts.".into() }).unwrap();

    let env_path = get_env_path()?;
    if env_path.exists() {
        log::info!("Removing existing environment at: {:?}", env_path);
        emitter.emit("installation-log", LogPayload { message: "Removing existing environment...".into() }).unwrap();
        std::fs::remove_dir_all(&env_path).map_err(|e| CommandError::Message(format!("Failed to remove existing env: {}", e)))?;
    }

    let config_dir = get_config_dir()?;
    fs::create_dir_all(&config_dir)
        .map_err(|e| CommandError::Message(format!("Failed to create config directory: {}", e)))?;

    emitter.emit("installation-log", LogPayload { message: "Found environment manager.".into() }).unwrap();

    // Clear micromamba cache to prevent issues with corrupted downloads
    emitter.emit("installation-log", LogPayload { message: "Clearing micromamba cache...".into() }).unwrap();
    let mut clean_command = shell.sidecar("micromamba")?;
    clean_command = clean_command.args(&["clean", "--all", "-y"])
        .env("MAMBA_ROOT_PREFIX", config_dir.to_str().unwrap());
    
    if cfg!(target_os = "windows") {
        clean_command = clean_command.env("MAMBA_SSL_NO_REVOKE", "true");
    }

    let (mut rx_clean, _child_clean) = clean_command.spawn()?;

    while let Some(event) = rx_clean.recv().await {
        match event {
            tauri_plugin_shell::process::CommandEvent::Stdout(line) | tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                let line_str = String::from_utf8_lossy(&line).to_string();
                emitter.emit("installation-log", LogPayload { message: line_str }).unwrap();
            },
            tauri_plugin_shell::process::CommandEvent::Terminated(payload) => {
                if payload.code != Some(0) {
                    return Err(CommandError::Message(format!("Failed to clear micromamba cache: {:?}", payload.code)));
                }
                break;
            }
            _ => {}
        }
    }
    emitter.emit("installation-log", LogPayload { message: "Micromamba cache cleared.".into() }).unwrap();

    std::thread::sleep(std::time::Duration::from_secs(2));

    // Step 1: Create environment with Python and pip
    emitter.emit("installation-log", LogPayload { message: "Creating Python environment...".into() }).unwrap();

    let create_args = vec![
        "create".to_string(),
        "-p".to_string(),
        env_path.to_str().unwrap().to_string(),
        "python=3.12".to_string(),
        "pip".to_string(),
        "ffmpeg".to_string(), // Always include ffmpeg
        "--override-channels".to_string(),
        "-c".to_string(),
        "conda-forge".to_string(),
        "-y".to_string(),
        "--verbose".to_string(),
    ];

    let mut attempts = 0;
    let max_attempts = 3;
    let mut success = false;

    while attempts < max_attempts {
        attempts += 1;
        emitter.emit("installation-log", LogPayload { message: format!("Attempt {} of {}: Creating Python environment...", attempts, max_attempts) }).unwrap();

        let mut create_command = shell.sidecar("micromamba")?;
        create_command = create_command.args(&create_args)
            .env("PYTHONUNBUFFERED", "1")
            .env("MAMBA_ROOT_PREFIX", config_dir.to_str().unwrap());
        
        if cfg!(target_os = "windows") {
            create_command = create_command.env("MAMBA_SSL_NO_REVOKE", "true");
        }

        let (mut rx, _child) = create_command.spawn()?;

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
                        if attempts == max_attempts {
                            return Err(CommandError::Message(error_message));
                        }
                        log::warn!("Micromamba create failed, retrying...");
                        break;
                    } else {
                        success = true;
                        break;
                    }
                }
                _ => {}
            }
        }
        if success { break; }
        std::thread::sleep(std::time::Duration::from_secs(5));
    }

    if !success {
        return Err(CommandError::Message("Failed to create conda environment after multiple attempts.".to_string()));
    }

    // Step 2: Install packages using pip
    emitter.emit("installation-log", LogPayload { message: "Installing Python libraries...".into() }).unwrap();

    let strategy = get_pytorch_install_strategy(shell).await;
    let mut pip_packages = vec![
        "torch~=2.9.0", "torchaudio~=2.9.0",
        "pyannote.audio~=4.0.1", "pypandoc~=1.15",
        "transformers~=4.57.1", "sacremoses~=0.1.1", "sentencepiece~=0.2.1", "torchcodec~=0.8.0"
    ];
    if strategy == PyTorchInstallStrategy::Cpu {
        pip_packages.extend(vec!["--extra-index-url", "https://download.pytorch.org/whl/cpu"]);
    }

    let mut pip_args = vec!["run", "-p", env_path.to_str().unwrap(), "pip", "install", "--no-cache-dir"];
    pip_args.extend(pip_packages.iter().map(|s| *s));

    let mut pip_command = shell.sidecar("micromamba")?;
    pip_command = pip_command.args(&pip_args)
        .env("PYTHONUNBUFFERED", "1")
        .env("MAMBA_ROOT_PREFIX", config_dir.to_str().unwrap());

    if cfg!(target_os = "windows") {
        if let Ok(resource_dir) = app.path().resource_dir() {
            let sidecars_path = resource_dir.join("sidecars");
            if sidecars_path.exists() {
                let cleaned_sidecars_path = dunce::canonicalize(&sidecars_path)
                    .map_err(|e| CommandError::Message(format!("Failed to canonicalize sidecars path: {}", e)))?;
                let existing_path = std::env::var("PATH").unwrap_or_default();
                let new_path = format!("{};{}", cleaned_sidecars_path.to_string_lossy(), existing_path);
                pip_command = pip_command.env("PATH", new_path);
            }
        }
    }

    let (mut rx_pip, _child_pip) = pip_command.spawn()?;

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

    // Step 3: Download pandoc binaries
    emitter.emit("installation-log", LogPayload { message: "Downloading Pandoc binaries...".into() }).unwrap();
    let python_path = get_python_path()?;
    let pandoc_args = ["-c", "import pypandoc; pypandoc.download_pandoc()"];
    let output = shell.command(python_path.to_str().unwrap())
        .args(&pandoc_args)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        emitter.emit("installation-log", LogPayload { message: format!("Failed to download pandoc: {}", stderr) }).unwrap();
    } else {
        emitter.emit("installation-log", LogPayload { message: "Pandoc downloaded successfully.".into() }).unwrap();
    }

    emitter.emit("installation-log", LogPayload { message: "Installation complete.".into() }).unwrap();
    emitter.emit("installation-finished", ()).unwrap();
    Ok(())
}


#[tauri::command]
pub async fn is_ffmpeg_installed<R: Runtime>(app: AppHandle<R>) -> Result<bool, CommandError> {
    log::info!("Checking for FFmpeg...");

    // Step 1: Check for ffmpeg in the sidecar directory.
    if let Ok(resource_dir) = app.path().resource_dir() {
        let ffmpeg_exe_name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
        let sidecar_ffmpeg_path = resource_dir.join("sidecars").join(ffmpeg_exe_name);

        if sidecar_ffmpeg_path.exists() {
            log::info!("Found FFmpeg in sidecar directory: {:?}", sidecar_ffmpeg_path);
            return Ok(true);
        }
        log::info!("FFmpeg not found in sidecar directory. Will check system PATH.");
    } else {
        log::warn!("Could not resolve resource directory. Will check system PATH.");
    }

    // Step 2: Fallback to checking the system PATH.
    log::info!("Checking for FFmpeg in system PATH.");
    let shell = app.shell();
    let command = if cfg!(windows) { "where" } else { "which" };
    let output = shell.command(command).arg("ffmpeg").output().await?;

    if output.status.success() {
        log::info!("Found FFmpeg in system PATH.");
    } else {
        log::info!("FFmpeg not found in system PATH.");
    }

    Ok(output.status.success())
}

pub async fn check_python_libraries_installed<R: Runtime>(
    _app: &AppHandle<R>,
    shell: &Shell<R>,
) -> Result<bool, CommandError> {
    let result: Result<bool, CommandError> = async {
        let env_path = get_env_path()?;
        if !env_path.exists() {
            log::info!("Python env does not exist at {:?}", env_path);
            return Ok(false);
        }

        let python_path = get_python_path()?;

        let packages_to_check = vec![
            ("torch", "torch"),
            ("torchaudio", "torchaudio"),
            ("torchcodec", "torchcodec"),
            ("pyannote.audio", "pyannote.audio"),
            ("transformers", "transformers"),
            ("sacremoses", "sacremoses"),
            ("sentencepiece", "sentencepiece"),
            ("pypandoc", "pypandoc"),
        ];

        // Prepare Windows PATH once, before the loop
        let windows_path_env: Option<String> = if cfg!(target_os = "windows") {
            let env_bin_path = env_path.join("Library").join("bin");
            if env_bin_path.exists() {
                let existing_path = std::env::var("PATH").unwrap_or_default();
                let new_path = format!("{};{}", env_bin_path.to_string_lossy(), existing_path);
                log::info!("Prepared PATH for verification: {}", new_path);
                Some(new_path)
            } else {
                None
            }
        } else {
            None
        };

        for (package_name, import_name) in &packages_to_check {
            log::info!("Checking for package: {}", package_name);

            let mut command = shell.command(python_path.to_str().unwrap());

            // Apply the pre-calculated PATH for Windows
            if let Some(path_val) = &windows_path_env {
                command = command.env("PATH", path_val.clone());
            } else if cfg!(target_os = "macos") {
                let env_lib_path = env_path.join("lib");
                if env_lib_path.exists() {
                    let existing_path = std::env::var("DYLD_LIBRARY_PATH").unwrap_or_default();
                    let new_path = format!("{}:{}", env_lib_path.to_string_lossy(), existing_path);
                    command = command.env("DYLD_LIBRARY_PATH", new_path);
                }
            } else if cfg!(target_os = "linux") {
                let env_lib_path = env_path.join("lib");
                if env_lib_path.exists() {
                    let existing_path = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
                    let new_path = format!("{}:{}", env_lib_path.to_string_lossy(), existing_path);
                    command = command.env("LD_LIBRARY_PATH", new_path);
                }
            }

            let output = command
                .args(&["-c", &format!("import {}", import_name)])
                .output()
                .await?;

            if !output.status.success() {
                log::warn!("Package '{}' not found.", package_name);
                log::warn!("Import check stdout: {}", String::from_utf8_lossy(&output.stdout));
                log::warn!("Import check stderr: {}", String::from_utf8_lossy(&output.stderr));
                return Ok(false);
            }
        }

        log::info!("All required Python libraries are installed.");
        Ok(true)
    }.await;

    match result {
        Ok(status) => Ok(status),
        Err(e) => {
            log::error!("Error checking python libraries: {}", e);
            Err(CommandError::Message("Required libraries are not installed.".to_string()))
        }
    }
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

        let mut config = read_config().map_err(|e| e.to_string())?;
        config.verification_status.python_libraries_verified = false;
        write_config(&config).map_err(|e| e.to_string())?;
    }
    Ok(())
}