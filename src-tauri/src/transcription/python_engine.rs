use super::TranslationEngine;
use crate::welcome::config::{read_config, CommandError};
use crate::welcome::python_env::get_python_path;
use async_trait::async_trait;
use log::{debug, error, info, warn};
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

pub struct PythonTranslationEngine<R: Runtime> {
    app_handle: AppHandle<R>,
}

impl<R: Runtime> PythonTranslationEngine<R> {
    pub fn new(app_handle: AppHandle<R>) -> Self {
        Self { app_handle }
    }
}

#[async_trait]
impl<R: Runtime> TranslationEngine for PythonTranslationEngine<R> {
    async fn translate(
        &self,
        texts: Vec<String>,
        model_path: &Path,
        job_id: &str,
        cancel_flag: Arc<AtomicBool>,
        mode: &str,
        src_lang: Option<&str>,
        tgt_lang: Option<&str>,
    ) -> Result<Vec<String>, CommandError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let python_path = get_python_path()
            .map_err(|e| CommandError::from(format!("Failed to get python path: {}", e)))?;

        let script_path = self
            .app_handle
            .path()
            .resolve(
                "scripts/run_translation.py",
                tauri::path::BaseDirectory::Resource,
            )
            .map_err(|e| {
                CommandError::from(format!("Failed to resolve translation script path: {}", e))
            })?;

        let input_json = serde_json::to_string(&texts)
            .map_err(|e| CommandError::from(format!("Failed to serialize texts: {}", e)))?;

        // Create a temporary file for the input text to avoid CLI argument length limits
        let temp_dir = std::env::temp_dir();
        let temp_file_path = temp_dir.join(format!("translation_input_{}.json", job_id));
        std::fs::write(&temp_file_path, input_json).map_err(|e| {
            CommandError::from(format!(
                "Failed to write temporary translation input file: {}",
                e
            ))
        })?;

        let mut python_args: Vec<String> = vec![
            script_path.to_string_lossy().to_string(),
            "--model-path".to_string(),
            model_path.to_string_lossy().to_string(),
            "--text-file".to_string(),
            temp_file_path.to_string_lossy().to_string(),
            "--mode".to_string(),
            mode.to_string(),
        ];

        if let Some(s) = src_lang {
            python_args.push("--src-lang".to_string());
            python_args.push(s.to_string());
        }
        if let Some(t) = tgt_lang {
            python_args.push("--tgt-lang".to_string());
            python_args.push(t.to_string());
        }

        // ... (rest of the logic including cleanup of the temp file)

        // Read advanced config
        if let Ok(config) = read_config() {
            if let Some(adv) = config.advanced_translation {
                if let Some(helsinki_bs) = adv.helsinki_batch_size {
                    python_args.push("--batch-size-helsinki".to_string());
                    python_args.push(helsinki_bs.to_string());
                }
                if let Some(nllb_bs) = adv.nllb_batch_size {
                    python_args.push("--batch-size-nllb".to_string());
                    python_args.push(nllb_bs.to_string());
                }
                if let Some(threads) = adv.num_threads {
                    python_args.push("--threads".to_string());
                    python_args.push(threads.to_string());
                }
                if let Some(device) = adv.device_preference {
                    python_args.push("--device-preference".to_string());
                    python_args.push(device);
                }
            }
        }

        info!(
            "[PythonEngine][{}] Executing python script: {:?}",
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
                        "[PythonEngine][{}] Cancellation requested. Killing process...",
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
                    debug!(
                        "[PythonEngine][stdout][{}] {}",
                        job_id,
                        String::from_utf8_lossy(&line).trim_end()
                    );
                    python_stdout.extend_from_slice(&line);
                }
                CommandEvent::Stderr(line) => {
                    debug!(
                        "[PythonEngine][stderr][{}] {}",
                        job_id,
                        String::from_utf8_lossy(&line).trim_end()
                    );
                    python_stderr.extend_from_slice(&line);
                }
                CommandEvent::Error(msg) => {
                    error!("[PythonEngine][error][{}] {}", job_id, msg);
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
            return Err(CommandError::from("Translation cancelled by user."));
        }

        if python_error.is_some() || python_exit_code != Some(0) {
            let stderr_output = String::from_utf8_lossy(&python_stderr);
            // Cleanup temp file even on error
            let _ = std::fs::remove_file(&temp_file_path);
            return Err(CommandError::from(format!(
                "Translation script failed. Code: {:?}. Error: {:?}. Stderr: {}",
                python_exit_code, python_error, stderr_output
            )));
        }

        // Cleanup temp file after success
        let _ = std::fs::remove_file(&temp_file_path);

        let translated_output = String::from_utf8_lossy(&python_stdout);
        let translated_texts: Vec<String> =
            serde_json::from_str(&translated_output).map_err(|e| {
                CommandError::from(format!("Failed to parse translation output: {}", e))
            })?;

        Ok(translated_texts)
    }
}
