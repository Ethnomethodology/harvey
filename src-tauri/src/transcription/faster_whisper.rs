use crate::welcome::config::{CommandError, read_config};
use crate::welcome::python_env::get_python_path;
use super::{TranscriptionEngine, TranscriptionOptions};
use crate::projectview::shared_types::TranscriptSegment;
use tauri::{AppHandle, Runtime, Manager};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use log::{debug, error, info, warn};
use tokio::sync::Mutex;
use std::time::Duration;
use tokio::time::sleep;
use serde::Deserialize;

pub struct FasterWhisperEngine<R: Runtime> {
    app_handle: AppHandle<R>,
}

impl<R: Runtime> FasterWhisperEngine<R> {
    pub fn new(app_handle: AppHandle<R>) -> Self {
        Self { app_handle }
    }
}

#[derive(Deserialize, Debug)]
struct FasterWhisperOutput {
    segments: Option<Vec<FasterWhisperSegment>>,
    error: Option<String>,
}

#[derive(Deserialize, Debug)]
struct FasterWhisperSegment {
    start: f64,
    end: f64,
    text: String,
    speaker: String,
}

#[async_trait]
impl<R: Runtime> TranscriptionEngine for FasterWhisperEngine<R> {
    async fn transcribe(
        &self,
        audio_path: &Path,
        options: &TranscriptionOptions,
        job_id: &str,
        cancel_flag: Arc<AtomicBool>,
    ) -> Result<Vec<TranscriptSegment>, CommandError> {
        let python_path = get_python_path()
            .map_err(|e| CommandError::from(format!("Failed to get python path: {}", e)))?;

        let script_path = self.app_handle.path()
            .resolve("scripts/run_transcription.py", tauri::path::BaseDirectory::Resource)
            .map_err(|e| CommandError::from(format!("Failed to resolve transcription script path: {}", e)))?;

        let lang_arg = options.language_code.as_deref().unwrap_or("auto");

        let mut python_args: Vec<String> = vec![
            script_path.to_string_lossy().to_string(),
            "--audio".to_string(),
            audio_path.to_string_lossy().to_string(),
            "--model".to_string(),
            options.model_path.clone(),
            "--language".to_string(),
            lang_arg.to_string(),
        ];

        if options.translate {
            python_args.push("--task".to_string());
            python_args.push("translate".to_string());
        }

        // Read advanced config
        if let Ok(config) = read_config() {
            if let Some(adv) = config.advanced_translation {
                if let Some(threads) = adv.num_threads {
                    python_args.push("--threads".to_string());
                    python_args.push(threads.to_string());
                }
                if let Some(device) = adv.device_preference {
                    python_args.push("--device".to_string());
                    python_args.push(device);
                }
            }
        }

        info!("[FasterWhisperEngine][{}] Executing python script: {:?}", job_id, python_args);

        let shell_scope = self.app_handle.shell();
        let (mut rx, child) = shell_scope.command(python_path.to_string_lossy().to_string())
            .args(python_args)
            .spawn()
            .map_err(|e| CommandError::from(format!("Failed to spawn Python script: {}", e)))?;

        let shared_child = Arc::new(Mutex::new(Some(child)));
        let cancel_flag_clone = cancel_flag.clone();
        let shared_child_clone = shared_child.clone();
        let job_id_clone = job_id.to_string();

        // Cancellation monitor
        tokio::spawn(async move {
            loop {
                if cancel_flag_clone.load(Ordering::Relaxed) {
                    warn!("[FasterWhisperEngine][{}] Cancellation requested. Killing process...", job_id_clone);
                    if let Some(child) = shared_child_clone.lock().await.take() {
                        let _ = child.kill();
                    }
                    break;
                }
                sleep(Duration::from_millis(100)).await;
            }
        });

        let mut python_stdout = Vec::new();
        let mut python_stderr = Vec::new();
        let mut python_exit_code: Option<i32> = None;
        let mut python_error: Option<String> = None;

        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    // Accumulate JSON output
                    python_stdout.extend_from_slice(&line);
                },
                CommandEvent::Stderr(line) => {
                    let l = String::from_utf8_lossy(&line).to_string();
                    debug!("[FasterWhisperEngine][stderr][{}] {}", job_id, l.trim_end());
                    python_stderr.extend_from_slice(&line);
                },
                CommandEvent::Error(msg) => {
                    error!("[FasterWhisperEngine][error][{}] {}", job_id, msg);
                    python_error = Some(msg);
                    break;
                },
                CommandEvent::Terminated(payload) => {
                    python_exit_code = payload.code;
                    if payload.signal.is_some() && python_exit_code.is_none() {
                        python_exit_code = Some(-1);
                    }
                    break;
                },
                _ => {}
            }
        }

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::from("Transcription cancelled by user."));
        }

        if python_error.is_some() || python_exit_code != Some(0) {
            let stderr_output = String::from_utf8_lossy(&python_stderr);
            return Err(CommandError::from(format!("Transcription script failed. Code: {:?}. Error: {:?}. Stderr: {}", python_exit_code, python_error, stderr_output)));
        }

        let output_str = String::from_utf8_lossy(&python_stdout);
        let parsed_output: FasterWhisperOutput = serde_json::from_str(&output_str)
            .map_err(|e| CommandError::from(format!("Failed to parse transcription output: {}. Output was: {}", e, output_str)))?;

        if let Some(err) = parsed_output.error {
            return Err(CommandError::from(format!("Faster-whisper error: {}", err)));
        }

        let segments = parsed_output.segments.unwrap_or_default().into_iter().map(|s| TranscriptSegment {
            start_time: s.start,
            end_time: s.end,
            text: s.text,
            speaker: s.speaker,
        }).collect();

        Ok(segments)
    }
}
