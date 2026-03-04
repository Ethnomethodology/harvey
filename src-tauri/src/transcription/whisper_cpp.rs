use crate::welcome::config::{CommandError, read_config};
use crate::welcome::python_env::get_env_path;
use super::{TranscriptionEngine, TranscriptionOptions};
use crate::projectview::shared_types::TranscriptSegment;
use tauri::{AppHandle, Runtime};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;
use async_trait::async_trait;
use std::path::{Path};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::Duration;
use tokio::time::sleep;
use log::{debug, info, warn};
use std::fs::{self, File};
use std::io::BufReader;
use serde::Deserialize;
use tokio::sync::Mutex;
use dunce;

pub struct WhisperCppEngine<R: Runtime> {
    app_handle: AppHandle<R>,
}

impl<R: Runtime> WhisperCppEngine<R> {
    pub fn new(app_handle: AppHandle<R>) -> Self {
        Self { app_handle }
    }
}

#[derive(Deserialize, Debug)]
struct WhisperJsonOutput {
    transcription: Option<Vec<WhisperJsonSegment>>,
}
#[derive(Deserialize, Debug)]
struct WhisperJsonSegment {
    timestamps: WhisperJsonTimestamps,
    text: String,
}
#[derive(Deserialize, Debug)]
struct WhisperJsonTimestamps {
    from: String,
    to: String,
}

// Helper function to normalize paths for the CLI, inspired by shared_utils
fn normalize_path_for_cli(path_str: &str) -> String {
    if cfg!(target_os = "windows") {
        // Strip the UNC prefix and convert to forward slashes
        let stripped_path = path_str.strip_prefix("\\\\?\\").unwrap_or(path_str);
        stripped_path.replace('\\', "/")
    } else {
        path_str.to_string()
    }
}

#[async_trait]
impl<R: Runtime> TranscriptionEngine for WhisperCppEngine<R> {
    async fn transcribe(
        &self,
        audio_path: &Path,
        options: &TranscriptionOptions,
        job_id: &str,
        cancel_flag: Arc<AtomicBool>,
    ) -> Result<Vec<TranscriptSegment>, CommandError> {
        let env_path = get_env_path()?;
        #[cfg(target_os = "windows")]
        let binary_path = env_path.join("Library").join("bin").join("whisper-cli.exe");
        #[cfg(not(target_os = "windows"))]
        let binary_path = env_path.join("bin").join("whisper-cli");

        if !binary_path.exists() {
            return Err(CommandError::from("whisper.cpp binary not found. Please install it from settings."));
        }

        let binary_path_str = binary_path.to_string_lossy().to_string();
        let lang_arg = options.language_code.as_deref().unwrap_or("auto");
        
        // Prepare output path and normalize it for the CLI
        let temp_base_name = format!("whisper_{}_temp", job_id);
        let output_base_path = options.output_dir.join(&temp_base_name);
        let output_base_path_str = normalize_path_for_cli(&output_base_path.to_string_lossy());
        let expected_json_path = output_base_path.with_extension("json");

        // Normalize all paths for the CLI
        let model_path_str = normalize_path_for_cli(&options.model_path);
        let audio_path_str = normalize_path_for_cli(&audio_path.to_string_lossy());

        let mut args: Vec<String> = vec![
            "-m".into(), model_path_str,
            "-f".into(), audio_path_str,
            "-l".into(), lang_arg.to_string(),
            "-oj".into(), // Output JSON
            "-of".into(), output_base_path_str,
        ];

        if options.translate {
            args.push("--translate".into());
        }

        if let Some(prompt) = &options.initial_prompt {
            if !prompt.trim().is_empty() {
                args.push("--prompt".into());
                args.push(prompt.clone());
            }
        }

        if let Ok(config) = read_config() {
            let mut device_preference_set = false;
            if let Some(trans_conf) = &config.advanced_transcription {
                if let Some(device) = &trans_conf.device_preference {
                    if device == "cpu" {
                        args.push("-ng".into());
                    }
                    device_preference_set = true;
                }
            }
            if !device_preference_set {
                if let Some(adv) = &config.advanced_translation {
                    if let Some(device) = &adv.device_preference {
                        if device == "cpu" {
                            args.push("-ng".into());
                        }
                    }
                }
            }
        }

        info!("[WhisperCppEngine][{}] Executing command '{}' with args: {:?}", job_id, binary_path_str, args);

        let shell_scope = self.app_handle.shell();
        let mut command = shell_scope.command(binary_path_str.clone())
            .args(args.clone());

        // Set the library path for the child process so it can find dependencies (like ffmpeg if needed)
        if cfg!(target_os = "windows") {
            let env_bin_path = env_path.join("Library").join("bin");
            if env_bin_path.exists() {
                if let Ok(cleaned_env_path) = dunce::canonicalize(&env_bin_path) {
                    let env_path_str = cleaned_env_path.to_string_lossy();
                    info!("[WhisperCppEngine][{}] Setting PATH to include conda bin: {}", job_id, env_path_str);
                    if let Ok(existing_path) = std::env::var("PATH") {
                        command = command.env("PATH", format!("{};{}", env_path_str, existing_path));
                    } else {
                        command = command.env("PATH", env_path_str.to_string());
                    }
                }
            }
        } else if cfg!(target_os = "macos") {
            let env_lib_path = env_path.join("lib");
            if env_lib_path.exists() {
                let env_lib_path_str = env_lib_path.to_string_lossy();
                info!("[WhisperCppEngine][{}] Setting DYLD_LIBRARY_PATH for macOS: {}", job_id, env_lib_path_str);
                if let Ok(existing_path) = std::env::var("DYLD_LIBRARY_PATH") {
                    command = command.env("DYLD_LIBRARY_PATH", format!("{}:{}", env_lib_path_str, existing_path));
                } else {
                    command = command.env("DYLD_LIBRARY_PATH", env_lib_path_str.to_string());
                }
            }
        }

        let (mut rx, child) = command.spawn()
            .map_err(|e| CommandError::from(format!("Failed to spawn whisper-cli: {}", e)))?;

        info!("[WhisperCppEngine][{}] Spawned process (PID: {:?})", job_id, child.pid());

        let shared_child = Arc::new(Mutex::new(Some(child)));
        let cancel_flag_clone = cancel_flag.clone();
        let shared_child_clone = shared_child.clone();
        let job_id_clone = job_id.to_string();

        // Cancellation monitor task
        tokio::spawn(async move {
            loop {
                if cancel_flag_clone.load(Ordering::Relaxed) {
                    warn!("[WhisperCppEngine][{}] Cancellation requested. Killing process...", job_id_clone);
                    if let Some(child) = shared_child_clone.lock().await.take() {
                        let _ = child.kill();
                    }
                    break;
                }
                sleep(Duration::from_millis(100)).await;
            }
        });

        let mut process_error: Option<String> = None;
        let mut exit_code: Option<i32> = None;

        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => { debug!("[WhisperCppEngine][stdout][{}] {}", job_id, String::from_utf8_lossy(&line).trim_end()); },
                CommandEvent::Stderr(line) => { debug!("[WhisperCppEngine][stderr][{}] {}", job_id, String::from_utf8_lossy(&line).trim_end()); },
                CommandEvent::Error(msg) => { process_error = Some(msg); break; },
                CommandEvent::Terminated(payload) => { exit_code = payload.code; break; },
                _ => {{}}
            }
        }

        if cancel_flag.load(Ordering::Relaxed) {
            if expected_json_path.exists() {{ let _ = fs::remove_file(&expected_json_path); }}
            return Err(CommandError::from(format!("Transcription cancelled for job {}.", job_id)));
        }

        if process_error.is_some() || exit_code != Some(0) {
            if expected_json_path.exists() {{ let _ = fs::remove_file(&expected_json_path); }}
            return Err(CommandError::from(format!("Whisper process failed. Exit: {:?}, Err: {:?}", exit_code, process_error)));
        }

        // Wait for file
        let mut attempts = 0;
        while !expected_json_path.exists() && attempts < 20 {
            if cancel_flag.load(Ordering::Relaxed) {{ return Err(CommandError::from("Cancelled while waiting for output file.")); }}
            sleep(Duration::from_millis(200)).await;
            attempts += 1;
        }

        if !expected_json_path.exists() {
            return Err(CommandError::from(format!("Output JSON not found at {:?}", expected_json_path)));
        }

        // Parse JSON
        let segments = parse_whisper_json(&expected_json_path)?;
        
        // Cleanup temp file
        let _ = fs::remove_file(&expected_json_path);

        Ok(segments)
    }
}

fn parse_whisper_json(json_path: &Path) -> Result<Vec<TranscriptSegment>, CommandError> {
    let file = File::open(json_path).map_err(|e| CommandError::from(format!("Failed to open JSON: {}", e)))?;
    let reader = BufReader::new(file);
    let output: WhisperJsonOutput = serde_json::from_reader(reader)
        .map_err(|e| CommandError::from(format!("Failed to parse JSON: {}", e)))?;

    let mut segments = Vec::new();
    if let Some(transcription) = output.transcription {
        for (idx, w_seg) in transcription.iter().enumerate() {
            let start_time = parse_ts(&w_seg.timestamps.from)
                .map_err(|e| CommandError::from(format!("Segment {}: Invalid start time: {}", idx, e)))?;
            let end_time = parse_ts(&w_seg.timestamps.to)
                .map_err(|e| CommandError::from(format!("Segment {}: Invalid end time: {}", idx, e)))?;
            
            if end_time < start_time {{ continue; }}

            segments.push(TranscriptSegment {
                start_time,
                end_time,
                speaker: "Unknown".to_string(),
                text: w_seg.text.trim().to_string(),
            });
        }
    }
    Ok(segments)
}

fn parse_ts(ts_str: &str) -> Result<f64, String> {
    let parts: Vec<&str> = ts_str.split(|c| c == ':' || c == ',' || c == '.').collect();
    if parts.len() == 4 { // hh:mm:ss:ms
        let h: f64 = parts[0].parse().map_err(|_| "h".to_string())?;
        let m: f64 = parts[1].parse().map_err(|_| "m".to_string())?;
        let s: f64 = parts[2].parse().map_err(|_| "s".to_string())?;
        let ms: f64 = parts[3].parse().map_err(|_| "ms".to_string())?;
        Ok(h * 3600.0 + m * 60.0 + s + ms / 1000.0)
    } else if parts.len() == 3 && ts_str.contains(':') { // mm:ss.ms
         let m: f64 = parts[0].parse().map_err(|_| "m2".to_string())?;
         let s: f64 = parts[1].parse().map_err(|_| "s2".to_string())?;
         let ms: f64 = parts[2].parse().map_err(|_| "ms2".to_string())?;
         Ok(m * 60.0 + s + ms / 1000.0)
    } else { Err(format!("Invalid timestamp format: {}", ts_str)) }
}