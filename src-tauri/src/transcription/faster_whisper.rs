use super::{TranscriptionEngine, TranscriptionOptions};
use crate::projectview::shared_types::TranscriptSegment;
use crate::welcome::config::{read_config, CommandError};
use crate::welcome::python_env::get_python_path;
use async_trait::async_trait;
use log::{debug, error, info, warn};
use serde::Deserialize;
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;
use tokio::sync::Mutex;
use tokio::time::sleep;

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
    #[serde(default)]
    words: Vec<crate::projectview::shared_types::Word>,
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

        let script_path = self
            .app_handle
            .path()
            .resolve(
                "scripts/run_transcription.py",
                tauri::path::BaseDirectory::Resource,
            )
            .map_err(|e| {
                CommandError::from(format!(
                    "Failed to resolve transcription script path: {}",
                    e
                ))
            })?;

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

        if let Some(prompt) = &options.initial_prompt {
            if !prompt.trim().is_empty() {
                python_args.push("--prompt".to_string());
                python_args.push(prompt.clone());
            }
        }

        if let Some(hotwords) = &options.hotwords {
            if !hotwords.trim().is_empty() {
                python_args.push("--hotwords".to_string());
                python_args.push(hotwords.clone());
            }
        }

        // Read advanced config
        if let Ok(config) = read_config() {
            // General device/threads settings (reusing from translation config or moving to transcription)
            // Note: advanced_translation contains general CPU threads and Device prefs which are likely system-wide intent.
            let mut device_preference_set = false;

            // Prioritize transcription-specific device preference if available
            if let Some(trans_conf) = &config.advanced_transcription {
                if let Some(device) = &trans_conf.device_preference {
                    python_args.push("--device".to_string());
                    python_args.push(device.clone());
                    device_preference_set = true;
                }
            }

            if let Some(adv) = config.advanced_translation {
                if let Some(threads) = adv.num_threads {
                    python_args.push("--threads".to_string());
                    python_args.push(threads.to_string());
                }
                if !device_preference_set {
                    if let Some(device) = adv.device_preference {
                        python_args.push("--device".to_string());
                        python_args.push(device);
                    }
                }
            }

            // Faster-Whisper specific settings
            if let Some(trans_conf) = config.advanced_transcription {
                // If specific num_threads is set for transcription, use it.
                // This overrides the global one if both are present (which logic implies by appending later)
                if let Some(threads) = trans_conf.num_threads {
                    // We need to remove previous --threads arg if it was added from translation config
                    // Ideally we should have prioritized transcription config first or used a unified logic.
                    // Given the vector append nature, we can just push it again and hope python script takes the last one,
                    // OR better, we check before pushing the global one.
                    // But simpler: just push it. argparse usually takes the last value for non-append actions.
                    python_args.push("--threads".to_string());
                    python_args.push(threads.to_string());
                }

                if let Some(compute_type) = trans_conf.faster_whisper_compute_type {
                    python_args.push("--compute_type".to_string());
                    python_args.push(compute_type);
                }
                if let Some(beam_size) = trans_conf.faster_whisper_beam_size {
                    python_args.push("--beam_size".to_string());
                    python_args.push(beam_size.to_string());
                }
            }
            
            if let Some(engine) = config.selected_transcription_engine {
                // No engine-specific flags needed. crisper-whisper will use standard faster-whisper defaults.
            }
        }

        info!(
            "[FasterWhisperEngine][{}] Executing python script: {:?}",
            job_id, python_args
        );

        let shell_scope = self.app_handle.shell();
        let (mut rx, child) = shell_scope
            .command(python_path.to_string_lossy().to_string())
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
                    warn!(
                        "[FasterWhisperEngine][{}] Cancellation requested. Killing process...",
                        job_id_clone
                    );
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
                }
                CommandEvent::Stderr(line) => {
                    let l = String::from_utf8_lossy(&line).to_string();
                    debug!("[FasterWhisperEngine][stderr][{}] {}", job_id, l.trim_end());
                    python_stderr.extend_from_slice(&line);
                }
                CommandEvent::Error(msg) => {
                    error!("[FasterWhisperEngine][error][{}] {}", job_id, msg);
                    python_error = Some(msg);
                    break;
                }
                CommandEvent::Terminated(payload) => {
                    python_exit_code = payload.code;
                    if payload.signal.is_some() && python_exit_code.is_none() {
                        python_exit_code = Some(-1);
                    }
                    break;
                }
                _ => {}
            }
        }

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::from("Transcription cancelled by user."));
        }

        if python_error.is_some() || python_exit_code != Some(0) {
            let stderr_output = String::from_utf8_lossy(&python_stderr);
            return Err(CommandError::from(format!(
                "Transcription script failed. Code: {:?}. Error: {:?}. Stderr: {}",
                python_exit_code, python_error, stderr_output
            )));
        }

        let output_str = String::from_utf8_lossy(&python_stdout);
        let parsed_output: FasterWhisperOutput =
            serde_json::from_str(&output_str).map_err(|e| {
                CommandError::from(format!(
                    "Failed to parse transcription output: {}. Output was: {}",
                    e, output_str
                ))
            })?;

        if let Some(err) = parsed_output.error {
            return Err(CommandError::from(format!("Faster-whisper error: {}", err)));
        }

        let segments = parsed_output
            .segments
            .unwrap_or_default()
            .into_iter()
            .map(|s| TranscriptSegment {
                start_time: s.start,
                end_time: s.end,
                text: s.text,
                speaker: s.speaker,
                words: Some(s.words),
            })
            .collect();

        Ok(segments)
    }
}
