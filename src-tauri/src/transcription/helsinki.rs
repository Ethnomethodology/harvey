use crate::welcome::config::{CommandError};
use crate::welcome::python_env::get_python_path;
use super::TranslationEngine;
use tauri::{AppHandle, Runtime, Manager};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;
use async_trait::async_trait;
use std::path::Path;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use log::{debug, error, info, warn};
use tokio::sync::Mutex;
use std::time::Duration;
use tokio::time::sleep;

pub struct HelsinkiTranslationEngine<R: Runtime> {
    app_handle: AppHandle<R>,
}

impl<R: Runtime> HelsinkiTranslationEngine<R> {
    pub fn new(app_handle: AppHandle<R>) -> Self {
        Self { app_handle }
    }
}

#[async_trait]
impl<R: Runtime> TranslationEngine for HelsinkiTranslationEngine<R> {
    async fn translate(
        &self,
        texts: Vec<String>,
        model_path: &Path,
        job_id: &str,
        cancel_flag: Arc<AtomicBool>,
    ) -> Result<Vec<String>, CommandError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let python_path = get_python_path()
            .map_err(|e| CommandError::from(format!("Failed to get python path: {}", e)))?;
        
        let script_path = self.app_handle.path()
            .resolve("scripts/run_translation.py", tauri::path::BaseDirectory::Resource)
            .map_err(|e| CommandError::from(format!("Failed to resolve translation script path: {}", e)))?;

        let input_json = serde_json::to_string(&texts)
            .map_err(|e| CommandError::from(format!("Failed to serialize texts: {}", e)))?;

        let python_args: Vec<String> = vec![
            script_path.to_string_lossy().to_string(),
            "--model-path".to_string(),
            model_path.to_string_lossy().to_string(),
            "--text".to_string(),
            input_json,
        ];

        info!("[HelsinkiEngine][{}] Executing python script: {:?}", job_id, python_args);

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
                    warn!("[HelsinkiEngine][{}] Cancellation requested. Killing process...", job_id_clone);
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
                    debug!("[HelsinkiEngine][stdout][{}] {}", job_id, String::from_utf8_lossy(&line).trim_end());
                    python_stdout.extend_from_slice(&line);
                },
                CommandEvent::Stderr(line) => {
                    debug!("[HelsinkiEngine][stderr][{}] {}", job_id, String::from_utf8_lossy(&line).trim_end());
                    python_stderr.extend_from_slice(&line);
                },
                CommandEvent::Error(msg) => {
                    error!("[HelsinkiEngine][error][{}] {}", job_id, msg);
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
            return Err(CommandError::from("Translation cancelled by user."));
        }

        if python_error.is_some() || python_exit_code != Some(0) {
            let stderr_output = String::from_utf8_lossy(&python_stderr);
            return Err(CommandError::from(format!("Translation script failed. Code: {:?}. Error: {:?}. Stderr: {}", python_exit_code, python_error, stderr_output)));
        }

        let translated_output = String::from_utf8_lossy(&python_stdout);
        let translated_texts: Vec<String> = serde_json::from_str(&translated_output)
            .map_err(|e| CommandError::from(format!("Failed to parse translation output: {}", e)))?;

        Ok(translated_texts)
    }
}
