use crate::welcome::config::{get_config_dir, read_config, write_config, CommandError};
use std::fs::{self};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_shell::{Shell, ShellExt};
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

pub fn get_env_command<R: Runtime>(
    app: &AppHandle<R>,
    binary_path: &str,
) -> Result<tauri_plugin_shell::process::Command, CommandError> {
    let env_path = get_env_path()?;
    let shell = app.shell();
    let mut command = shell.command(binary_path);

    // Add environment bin folder to PATH so subprocesses (like pandoc) can be found
    let mut bin_paths = Vec::new();

    #[cfg(target_os = "windows")]
    {
        bin_paths.push(env_path.join("Library").join("bin"));
        bin_paths.push(env_path.join("Scripts")); // Some python tools on windows
    }
    #[cfg(not(target_os = "windows"))]
    {
        bin_paths.push(env_path.join("bin"));
    }

    let existing_path = std::env::var("PATH").unwrap_or_default();
    let separator = if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    };

    let mut new_path_str = String::new();
    for path in bin_paths {
        if path.exists() {
            if !new_path_str.is_empty() {
                new_path_str.push_str(separator);
            }
            new_path_str.push_str(&path.to_string_lossy());
        }
    }

    if !new_path_str.is_empty() {
        new_path_str.push_str(separator);
        new_path_str.push_str(&existing_path);
        command = command.env("PATH", new_path_str);
    }

    Ok(command)
}

pub fn get_python_command<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<tauri_plugin_shell::process::Command, CommandError> {
    let python_path = get_python_path()?;
    get_env_command(app, python_path.to_str().unwrap())
}

#[derive(PartialEq, Debug)]
pub enum PyTorchInstallStrategy {
    Gpu,
    Cpu,
}

pub async fn get_pytorch_install_strategy<R: Runtime>(shell: &Shell<R>) -> PyTorchInstallStrategy {
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
                log::debug!(
                    "nvidia-smi stdout: {}",
                    String::from_utf8_lossy(&output.stdout)
                );
                return PyTorchInstallStrategy::Gpu;
            }
            log::info!(
                "nvidia-smi not found or failed, defaulting to CPU. Stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        } else {
            log::info!("Error executing nvidia-smi, defaulting to CPU.");
        }
    } else if target_triple.contains("unknown-linux-gnu") {
        log::info!("Checking for NVIDIA GPU with command: nvidia-smi");
        if let Ok(output) = shell.command("nvidia-smi").output().await {
            if output.status.success() {
                log::info!("nvidia-smi found, enabling GPU (CUDA) support for PyTorch.");
                log::debug!(
                    "nvidia-smi stdout: {}",
                    String::from_utf8_lossy(&output.stdout)
                );
                return PyTorchInstallStrategy::Gpu;
            }
            log::info!(
                "nvidia-smi not found or failed, defaulting to CPU. Stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        } else {
            log::info!("Error executing nvidia-smi, defaulting to CPU.");
        }
    }

    log::info!("Defaulting to CPU-only PyTorch installation.");
    PyTorchInstallStrategy::Cpu
}

fn emit_log<R: Runtime>(
    app: &AppHandle<R>,
    event_name: &str,
    message: String,
    model_name: Option<&str>,
) {
    if event_name == "transcription-download-log" || event_name == "translation-download-log" {
        let payload = serde_json::json!({
            "model_name": model_name.unwrap_or("System"),
            "log_line": message
        });
        let _ = app.emit(event_name, payload);
    } else {
        let _ = app.emit(event_name, LogPayload { message });
    }
}

// --- Main Installation Logic ---

pub async fn install_python_libraries<R: Runtime>(
    app: &AppHandle<R>,
    shell: &Shell<R>,
) -> Result<(), CommandError> {
    install_python_libraries_micromamba(app, shell).await
}

#[allow(unused_variables)]
async fn install_python_libraries_micromamba<R: Runtime>(
    app: &AppHandle<R>,
    shell: &Shell<R>,
) -> Result<(), CommandError> {
    let emitter = app.clone();
    emit_log(
        &emitter,
        "installation-log",
        "Starting installation... Cleaning up previous attempts.".into(),
        None,
    );

    let env_path = get_env_path()?;
    if env_path.exists() {
        log::info!("Removing existing environment at: {:?}", env_path);
        emit_log(
            &emitter,
            "installation-log",
            "Removing existing environment...".into(),
            None,
        );
        std::fs::remove_dir_all(&env_path)
            .map_err(|e| CommandError::Message(format!("Failed to remove existing env: {}", e)))?;
    }

    let config_dir = get_config_dir()?;
    fs::create_dir_all(&config_dir)
        .map_err(|e| CommandError::Message(format!("Failed to create config directory: {}", e)))?;

    emit_log(
        &emitter,
        "installation-log",
        "Found environment manager.".into(),
        None,
    );

    // Clear micromamba cache to prevent issues with corrupted downloads
    emit_log(
        &emitter,
        "installation-log",
        "Clearing micromamba cache...".into(),
        None,
    );
    let mut clean_command = shell.sidecar("micromamba")?;
    clean_command = clean_command
        .args(["clean", "--all", "-y"])
        .env("MAMBA_ROOT_PREFIX", config_dir.to_str().unwrap());

    if cfg!(target_os = "windows") {
        clean_command = clean_command.env("MAMBA_SSL_NO_REVOKE", "true");
    }

    let (mut rx_clean, _child_clean) = clean_command.spawn()?;

    while let Some(event) = rx_clean.recv().await {
        match event {
            tauri_plugin_shell::process::CommandEvent::Stdout(line)
            | tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                let line_str = String::from_utf8_lossy(&line).to_string();
                emit_log(&emitter, "installation-log", line_str, None);
            }
            tauri_plugin_shell::process::CommandEvent::Terminated(payload) => {
                if payload.code != Some(0) {
                    return Err(CommandError::Message(format!(
                        "Failed to clear micromamba cache: {:?}",
                        payload.code
                    )));
                }
                break;
            }
            _ => {}
        }
    }
    emit_log(
        &emitter,
        "installation-log",
        "Micromamba cache cleared.".into(),
        None,
    );

    std::thread::sleep(std::time::Duration::from_secs(2));

    // Step 1: Create environment with Python and pip
    emit_log(
        &emitter,
        "installation-log",
        "Creating Python environment...".into(),
        None,
    );

    let create_args = vec![
        "create".to_string(),
        "-p".to_string(),
        env_path.to_str().unwrap().to_string(),
        "python=3.12".to_string(),
        "pip".to_string(),
        "ffmpeg".to_string(), // Always include ffmpeg
        "pandoc".to_string(), // Include pandoc from conda-forge
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
        emit_log(
            &emitter,
            "installation-log",
            format!(
                "Attempt {} of {}: Creating Python environment...",
                attempts, max_attempts
            ),
            None,
        );

        let mut create_command = shell.sidecar("micromamba")?;
        create_command = create_command
            .args(&create_args)
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
                    emit_log(&emitter, "installation-log", line_str.clone(), None);
                    output_lines.push(line_str);
                }
                tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                    let line_str = String::from_utf8_lossy(&line).to_string();
                    emit_log(&emitter, "installation-log", line_str.clone(), None);
                    output_lines.push(line_str);
                }
                tauri_plugin_shell::process::CommandEvent::Terminated(payload) => {
                    if payload.code != Some(0) {
                        let full_log = output_lines.join("\n");
                        let error_message = format!(
                            "Failed to create conda environment. Full log:\n{}",
                            full_log
                        );
                        emit_log(&emitter, "installation-log", error_message.clone(), None);
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
        if success {
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
    }

    if !success {
        return Err(CommandError::Message(
            "Failed to create conda environment after multiple attempts.".to_string(),
        ));
    }

    // Step 2: Install packages using pip
    emit_log(
        &emitter,
        "installation-log",
        "Installing Python libraries...".into(),
        None,
    );

    let strategy = get_pytorch_install_strategy(shell).await;
    let mut pip_packages = vec![
        "torch~=2.9.0",
        "torchaudio~=2.9.0",
        "pyannote.audio~=4.0.1",
        "pypandoc~=1.15",
        "transformers~=4.57.1",
        "sacremoses~=0.1.1",
        "sentencepiece~=0.2.1",
        "torchcodec~=0.8.0",
    ];
    if strategy == PyTorchInstallStrategy::Cpu {
        pip_packages.extend(vec![
            "--extra-index-url",
            "https://download.pytorch.org/whl/cpu",
        ]);
    }

    let mut pip_args = vec![
        "run",
        "-p",
        env_path.to_str().unwrap(),
        "pip",
        "install",
        "--no-cache-dir",
    ];
    pip_args.extend(pip_packages.iter().copied());

    let mut pip_command = shell.sidecar("micromamba")?;
    pip_command = pip_command
        .args(&pip_args)
        .env("PYTHONUNBUFFERED", "1")
        .env("MAMBA_ROOT_PREFIX", config_dir.to_str().unwrap());

    if cfg!(target_os = "windows") {
        if let Ok(resource_dir) = app.path().resource_dir() {
            let sidecars_path = resource_dir.join("sidecars");
            if sidecars_path.exists() {
                let cleaned_sidecars_path = dunce::canonicalize(&sidecars_path).map_err(|e| {
                    CommandError::Message(format!("Failed to canonicalize sidecars path: {}", e))
                })?;
                let existing_path = std::env::var("PATH").unwrap_or_default();
                let new_path = format!(
                    "{};{}",
                    cleaned_sidecars_path.to_string_lossy(),
                    existing_path
                );
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
                emit_log(&emitter, "installation-log", line_str.clone(), None);
                pip_output_lines.push(line_str);
            }
            tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                let line_str = String::from_utf8_lossy(&line).to_string();
                emit_log(&emitter, "installation-log", line_str.clone(), None);
                pip_output_lines.push(line_str);
            }
            tauri_plugin_shell::process::CommandEvent::Terminated(payload) => {
                if payload.code != Some(0) {
                    let full_log = pip_output_lines.join("\n");
                    let error_message =
                        format!("Failed to install pip packages. Full log:\n{}", full_log);
                    emit_log(&emitter, "installation-log", error_message.clone(), None);
                    return Err(CommandError::Message(error_message));
                }
                break;
            }
            _ => {}
        }
    }

    emit_log(
        &emitter,
        "installation-log",
        "Successfully installed Python libraries.".into(),
        None,
    );

    emit_log(
        &emitter,
        "installation-log",
        "Installation complete.".into(),
        None,
    );
    emitter.emit("installation-finished", ()).unwrap();
    Ok(())
}

#[tauri::command]
pub async fn is_ffmpeg_installed<R: Runtime>(app: AppHandle<R>) -> Result<bool, CommandError> {
    log::info!("Checking for FFmpeg...");

    // Step 1: Check for ffmpeg in the sidecar directory.
    if let Ok(resource_dir) = app.path().resource_dir() {
        let ffmpeg_exe_name = if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        };
        let sidecar_ffmpeg_path = resource_dir.join("sidecars").join(ffmpeg_exe_name);

        if sidecar_ffmpeg_path.exists() {
            log::info!(
                "Found FFmpeg in sidecar directory: {:?}",
                sidecar_ffmpeg_path
            );
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
    app: &AppHandle<R>,
    _shell: &Shell<R>,
) -> Result<bool, CommandError> {
    let result: Result<bool, CommandError> = async {
        let env_path = get_env_path()?;
        if !env_path.exists() {
            log::info!("Python env does not exist at {:?}", env_path);
            return Ok(false);
        }

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

        for (package_name, import_name) in &packages_to_check {
            if !check_package_installed(app, import_name).await? {
                log::warn!("Package '{}' not found.", package_name);
                return Ok(false);
            }
        }

        log::info!("All required Python libraries are installed.");
        Ok(true)
    }
    .await;

    match result {
        Ok(status) => Ok(status),
        Err(e) => {
            log::error!("Error checking python libraries: {}", e);
            Err(CommandError::Message(
                "Required libraries are not installed.".to_string(),
            ))
        }
    }
}

pub async fn check_package_installed<R: Runtime>(
    app: &AppHandle<R>,
    import_name: &str,
) -> Result<bool, CommandError> {
    let output = run_python_import_check(app, import_name).await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::warn!("Import check failed for '{}': {}", import_name, stderr);
    }
    Ok(output.status.success())
}

pub async fn get_package_import_error<R: Runtime>(
    app: &AppHandle<R>,
    import_name: &str,
) -> Result<Option<String>, CommandError> {
    let output = run_python_import_check(app, import_name).await?;
    if !output.status.success() {
        Ok(Some(String::from_utf8_lossy(&output.stderr).to_string()))
    } else {
        Ok(None)
    }
}

async fn run_python_import_check<R: Runtime>(
    app: &AppHandle<R>,
    import_name: &str,
) -> Result<tauri_plugin_shell::process::Output, CommandError> {
    let command = get_python_command(app)?;

    let output = command
        .args(["-c", &format!("import {}", import_name)])
        .output()
        .await?;

    Ok(output)
}

pub async fn install_pip_packages<R: Runtime>(
    app: &AppHandle<R>,
    shell: &Shell<R>,
    packages: Vec<&str>,
    log_event_name: &str,
    model_name: Option<&str>,
) -> Result<(), CommandError> {
    let emitter = app.clone();
    let env_path = get_env_path()?;
    let config_dir = get_config_dir()?;

    let mut pip_args = vec![
        "run",
        "-p",
        env_path.to_str().unwrap(),
        "pip",
        "install",
        "--no-cache-dir",
    ];
    pip_args.extend(packages.iter().copied());

    let mut pip_command = shell.sidecar("micromamba")?;
    pip_command = pip_command
        .args(&pip_args)
        .env("PYTHONUNBUFFERED", "1")
        .env("MAMBA_ROOT_PREFIX", config_dir.to_str().unwrap());

    if cfg!(target_os = "windows") {
        if let Ok(resource_dir) = app.path().resource_dir() {
            let sidecars_path = resource_dir.join("sidecars");
            if sidecars_path.exists() {
                let cleaned_sidecars_path = dunce::canonicalize(&sidecars_path).map_err(|e| {
                    CommandError::Message(format!("Failed to canonicalize sidecars path: {}", e))
                })?;
                let existing_path = std::env::var("PATH").unwrap_or_default();
                let new_path = format!(
                    "{};{}",
                    cleaned_sidecars_path.to_string_lossy(),
                    existing_path
                );
                pip_command = pip_command.env("PATH", new_path);
            }
        }
    }

    let (mut rx_pip, _child_pip) = pip_command.spawn()?;

    while let Some(event) = rx_pip.recv().await {
        match event {
            tauri_plugin_shell::process::CommandEvent::Stdout(line)
            | tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                let line_str = String::from_utf8_lossy(&line).to_string();
                emit_log(&emitter, log_event_name, line_str, model_name);
            }
            tauri_plugin_shell::process::CommandEvent::Terminated(payload) => {
                if payload.code != Some(0) {
                    return Err(CommandError::Message(format!(
                        "Failed to install pip packages: {:?}",
                        payload.code
                    )));
                }
                break;
            }
            _ => {}
        }
    }
    Ok(())
}

use tauri_plugin_shell::process::CommandEvent;

pub async fn install_whisper_cpp_dependencies<R: Runtime>(
    app: &AppHandle<R>,
    shell: &Shell<R>,
    log_event_name: &str,
    model_name: Option<&str>,
) -> Result<(), CommandError> {
    let emitter = app.clone();
    let env_path = get_env_path()?;
    let config_dir = get_config_dir()?;

    let package_name = if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "whisper.cpp=*=*mkl*"
    } else {
        "whisper.cpp"
    };

    let conda_args = vec![
        "install",
        "-p",
        env_path.to_str().unwrap(),
        package_name,
        "-c",
        "conda-forge",
        "-y",
    ];

    emit_log(
        &emitter,
        log_event_name,
        "Installing whisper.cpp via micromamba...".to_string(),
        model_name,
    );
    log::info!("Executing: micromamba {}", conda_args.join(" "));

    let mut conda_command = shell.sidecar("micromamba")?;
    conda_command = conda_command
        .args(&conda_args)
        .env("MAMBA_ROOT_PREFIX", config_dir.to_str().unwrap());

    if cfg!(target_os = "windows") {
        conda_command = conda_command.env("MAMBA_SSL_NO_REVOKE", "true");
    }

    let (mut rx, _child) = conda_command.spawn().map_err(|e| {
        CommandError::from(format!("Failed to start micromamba for whisper.cpp: {}", e))
    })?;

    let mut success = false;
    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(line) | CommandEvent::Stderr(line) => {
                emit_log(
                    &emitter,
                    log_event_name,
                    String::from_utf8_lossy(&line).to_string(),
                    model_name,
                );
            }
            CommandEvent::Terminated(payload) => {
                if payload.code == Some(0) {
                    success = true;
                } else {
                    emit_log(
                        &emitter,
                        log_event_name,
                        format!("whisper.cpp install failed with code: {:?}", payload.code),
                        model_name,
                    );
                }
                break;
            }
            _ => {}
        }
    }

    if !success {
        return Err(CommandError::from(
            "Failed to install whisper.cpp via micromamba.",
        ));
    }

    // Verify it was installed
    #[cfg(target_os = "windows")]
    let binary_path = env_path.join("Library").join("bin").join("whisper-cli.exe");
    #[cfg(not(target_os = "windows"))]
    let binary_path = env_path.join("bin").join("whisper-cli");

    if !binary_path.exists() {
        emit_log(
            &emitter,
            log_event_name,
            format!(
                "Installation reported success, but binary not found at {:?}",
                binary_path
            ),
            model_name,
        );
        return Err(CommandError::from(
            "whisper-cli binary not found after installation.",
        ));
    }

    emit_log(
        &emitter,
        log_event_name,
        "whisper.cpp installed successfully.".to_string(),
        model_name,
    );
    Ok(())
}

pub async fn install_faster_whisper_dependencies<R: Runtime>(
    app: &AppHandle<R>,
    shell: &Shell<R>,
    log_event_name: &str,
    model_name: Option<&str>,
) -> Result<(), CommandError> {
    let emitter = app.clone();
    let env_path = get_env_path()?;
    let config_dir = get_config_dir()?;

    // Step 1: Install portaudio via micromamba
    emit_log(
        &emitter,
        log_event_name,
        "Installing system audio libraries (portaudio)...".into(),
        model_name,
    );
    let conda_args = vec![
        "install",
        "-p",
        env_path.to_str().unwrap(),
        "portaudio",
        "-c",
        "conda-forge",
        "-y",
    ];
    let mut conda_command = shell.sidecar("micromamba")?;
    conda_command = conda_command
        .args(&conda_args)
        .env("MAMBA_ROOT_PREFIX", config_dir.to_str().unwrap());

    if cfg!(target_os = "windows") {
        conda_command = conda_command.env("MAMBA_SSL_NO_REVOKE", "true");
    }

    let (mut rx_conda, _child_conda) = conda_command.spawn()?;
    while let Some(event) = rx_conda.recv().await {
        match event {
            tauri_plugin_shell::process::CommandEvent::Stdout(line)
            | tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                let line_str = String::from_utf8_lossy(&line).to_string();
                emit_log(&emitter, log_event_name, line_str, model_name);
            }
            tauri_plugin_shell::process::CommandEvent::Terminated(payload) => {
                if payload.code != Some(0) {
                    return Err(CommandError::Message(format!(
                        "Failed to install portaudio: {:?}",
                        payload.code
                    )));
                }
                break;
            }
            _ => {}
        }
    }

    // Step 2: Install pip packages
    emit_log(
        &emitter,
        log_event_name,
        "Installing Python transcription libraries...".into(),
        model_name,
    );
    let pip_packages = vec!["faster-whisper", "ctranslate2", "sounddevice"];
    install_pip_packages(app, shell, pip_packages, log_event_name, model_name).await?;

    emit_log(
        &emitter,
        log_event_name,
        "Transcription libraries installed successfully.".into(),
        model_name,
    );
    Ok(())
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
        return Err(format!(
            "Conda environment 'site-packages' directory not found at {:?}",
            lib_path
        ));
    }

    let entries = fs::read_dir(&lib_path)
        .map_err(|e| format!("Failed to read 'site-packages' directory: {}", e))?
        .filter_map(|entry| entry.ok().and_then(|e| e.file_name().into_string().ok()))
        .collect();

    Ok(entries)
}

#[tauri::command]
pub async fn delete_virtual_env(app_handle: AppHandle) -> Result<(), String> {
    let env_path = get_env_path().map_err(|e| e.to_string())?;
    if env_path.exists() {
        log::info!("Deleting environment at: {:?}", env_path);
        std::fs::remove_dir_all(&env_path)
            .map_err(|e| format!("Failed to delete environment: {}", e))?;
        log::info!("Environment deleted successfully.");

        let mut config = read_config().map_err(|e| e.to_string())?;
        config.verification_status.python_libraries_verified = false;

        // Also delete downloaded models
        let base_model_dir_str = if !config.download_location.trim().is_empty() {
            config.download_location.clone()
        } else {
            crate::welcome::config::get_default_download_location().map_err(|e| e.to_string())?
        };

        // Paths where models and engines are downloaded
        let transcription_models_dir = PathBuf::from(&base_model_dir_str).join("transcription");
        let translation_models_dir = PathBuf::from(&base_model_dir_str).join("translation");

        if transcription_models_dir.exists() {
            log::info!(
                "Deleting transcription models at: {:?}",
                transcription_models_dir
            );
            let _ = std::fs::remove_dir_all(&transcription_models_dir);
        }

        if translation_models_dir.exists() {
            log::info!(
                "Deleting translation models at: {:?}",
                translation_models_dir
            );
            let _ = std::fs::remove_dir_all(&translation_models_dir);
        }

        // Also delete diarization models
        if let Ok(diarization_models_dir) =
            crate::welcome::diarization::get_diarization_hub_path(&app_handle)
        {
            if diarization_models_dir.exists() {
                log::info!(
                    "Deleting diarization models at: {:?}",
                    diarization_models_dir
                );
                let _ = std::fs::remove_dir_all(&diarization_models_dir);
            }
        }

        // Old legacy path for whisper.cpp models
        let legacy_model_dir = PathBuf::from(&base_model_dir_str);
        if legacy_model_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&legacy_model_dir) {
                for entry in entries.flatten() {
                    if let Ok(file_name) = entry.file_name().into_string() {
                        if file_name.starts_with("ggml-") && file_name.ends_with(".bin") {
                            log::info!("Deleting legacy model file: {:?}", entry.path());
                            let _ = std::fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }

        write_config(&config).map_err(|e| e.to_string())?;
    }
    Ok(())
}
