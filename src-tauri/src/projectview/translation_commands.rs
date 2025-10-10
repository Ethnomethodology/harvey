use tauri::{AppHandle, Manager, Runtime, Emitter};
use tauri_plugin_shell::ShellExt;
use std::fs;
use std::sync::{Arc, atomic::{AtomicBool, Ordering as AtomicOrdering}};
use serde_json::{Value, json};
use log::{info, error, debug, warn};
use super::transcription_commands::save_transcript_json;
use crate::welcome::config::{read_config, get_default_download_location, CommandError};
use crate::welcome::python_env::get_python_path;
use dashmap::DashMap;
use crate::TranslationCancellationState;


// --- CancelGuard for ensuring cleanup ---
struct CancelGuard {
    job_id: String,
    state: Arc<DashMap<String, Arc<AtomicBool>>>,
}

impl Drop for CancelGuard {
    fn drop(&mut self) {
        self.state.remove(&self.job_id);
        debug!("[Translate CancelGuard] Removed cancel flag for job '{}' on drop.", self.job_id);
    }
}

// --- Payloads for frontend communication ---
#[derive(serde::Serialize, Clone)]
pub struct TranslationInitiatedPayload {
    job_id: String,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TranslationJobCompletedPayload {
    job_id: String,
    status: String, // "done", "cancelled", "error"
    original_transcript_path: String,
    new_transcript_path: Option<String>,
    error_message: Option<String>,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TranslationProgressPayload {
    job_id: String,
    percent: f32,
    message: String,
}


// Helper to extract plain text from a Lexical JSON structure.
fn extract_plain_text_from_lexical(node: &Value) -> String {
    let mut text = String::new();
    if let Some(node_text) = node.get("text").and_then(|v| v.as_str()) {
        text.push_str(node_text);
    }
    if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
        for child in children {
            text.push_str(&extract_plain_text_from_lexical(child));
        }
    }
    text
}

// Helper to create a simple Lexical JSON structure from plain text.
fn create_lexical_with_text(text: &str) -> Value {
    json!({
        "root": {
            "type": "root",
            "children": [{
                "type": "paragraph",
                "children": [{
                    "type": "text",
                    "text": text,
                    "detail": 0,
                    "format": 0,
                    "mode": "normal",
                    "style": "",
                    "version": 1
                }],
                "direction": "ltr",
                "format": "",
                "indent": 0,
                "version": 1
            }],
            "direction": "ltr",
            "format": "",
            "indent": 0,
            "version": 1
        }
    })
}

// Helper to emit progress updates
fn emit_translation_progress<R: Runtime>(
    app_handle: &AppHandle<R>,
    job_id: &str,
    percent: f32,
    message: &str,
) {
    let payload = TranslationProgressPayload {
        job_id: job_id.to_string(),
        percent,
        message: message.to_string(),
    };
    if let Err(e) = app_handle.emit("TRANSLATION_PROGRESS", payload) {
        error!("[Translate Progress][{}] Failed to emit progress event: {}", job_id, e);
    }
}


#[tauri::command]
pub async fn translate_transcript_command<R: Runtime>(
    app_handle: AppHandle<R>,
    project_xml_path: String,
    transcript_path: String,
    model_name: String,
    target_language: String,
    cancel_state: tauri::State<'_, TranslationCancellationState>,
) -> Result<TranslationInitiatedPayload, String> {
    let job_id = uuid::Uuid::new_v4().to_string();
    info!("[Translate Command][{}] Received request for transcript: {}", job_id, transcript_path);

    let cancel_flag = Arc::new(AtomicBool::new(false));
    cancel_state.0.insert(job_id.clone(), Arc::clone(&cancel_flag));

    let _cancel_guard = CancelGuard {
        job_id: job_id.clone(),
        state: Arc::clone(&cancel_state.0),
    };

    let app_handle_clone = app_handle.clone();
    let job_id_clone = job_id.clone();

    tokio::spawn(async move {
        let completion_payload = match run_translation_process(
            app_handle_clone,
            job_id_clone.clone(),
            project_xml_path,
            transcript_path,
            model_name,
            target_language,
            cancel_flag,
        ).await {
            Ok(new_path) => {
                info!("[Translate Task][{}] Translation process completed successfully.", job_id_clone);
                TranslationJobCompletedPayload {
                    job_id: job_id_clone,
                    status: "done".to_string(),
                    original_transcript_path: new_path.clone(),
                    new_transcript_path: Some(new_path),
                    error_message: None,
                }
            }
            Err(e) => {
                error!("[Translate Task][{}] Translation process failed: {}", job_id_clone, e);
                let (status, err_msg, path) = if e.to_string().to_lowercase().contains("cancel") {
                    ("cancelled".to_string(), e.to_string(), e.to_string())
                } else {
                    ("error".to_string(), e.to_string(), e.to_string())
                };

                TranslationJobCompletedPayload {
                    job_id: job_id_clone,
                    status: status,
                    original_transcript_path: path,
                    new_transcript_path: None,
                    error_message: Some(err_msg),
                }
            }
        };

        if let Err(e) = app_handle.emit("translation_job_completed", &completion_payload) {
            error!("[Translate Task][{}] Failed to emit completion event: {}", completion_payload.job_id, e);
        }
    });

    Ok(TranslationInitiatedPayload { job_id })
}


async fn run_translation_process<R: Runtime>(
    app_handle: AppHandle<R>,
    job_id: String,
    project_xml_path: String,
    transcript_path: String,
    model_name: String,
    _target_language: String,
    cancel_flag: Arc<AtomicBool>,
) -> Result<String, CommandError> {
    emit_translation_progress(&app_handle, &job_id, 5.0, "Preparing for translation...");

    let config = read_config()?;
    let download_location = if !config.download_location.trim().is_empty() {
        config.download_location.clone()
    } else {
        get_default_download_location()?
    };

    let model_cache_dir_name = format!("models--{}", model_name.replace('/', "--"));
    let model_base_path = std::path::Path::new(&download_location).join(model_cache_dir_name);
    let refs_path = model_base_path.join("refs/main");
    let commit_hash = fs::read_to_string(refs_path).map_err(|e| CommandError::from(format!("Failed to read commit hash for model '{}': {}", model_name, e)))?.trim().to_string();
    let model_path = model_base_path.join("snapshots").join(commit_hash);

    info!("[Translate][{}] Using model path: {}", job_id, model_path.display());

    if cancel_flag.load(AtomicOrdering::Relaxed) { return Err(CommandError::from("Translation cancelled by user.")); }

    let content = fs::read_to_string(&transcript_path)?;
    let mut lexical_json: Value = serde_json::from_str(&content)?;

    let texts_to_translate: Vec<String> = if let Some(table_node) = lexical_json.get("root").and_then(|r| r.get("children")).and_then(|c| c.as_array()).and_then(|c| c.iter().find(|n| n.get("type").and_then(|t| t.as_str()) == Some("table"))) {
        if let Some(rows) = table_node.get("children").and_then(|c| c.as_array()) {
            rows.iter().skip(1).filter_map(|row| {
                row.get("children").and_then(|c| c.as_array()).and_then(|cells| cells.get(3)).map(|cell| extract_plain_text_from_lexical(cell))
            }).collect()
        } else { Vec::new() }
    } else { Vec::new() };

    if texts_to_translate.is_empty() {
        info!("[Translate][{}] No text found to translate.", job_id);
        return Ok(transcript_path);
    }

    emit_translation_progress(&app_handle, &job_id, 20.0, "Running translation model...");

    let python_path = get_python_path()?;
    let script_path = app_handle.path().resolve("scripts/run_translation.py", tauri::path::BaseDirectory::Resource)
        .map_err(|e| CommandError::from(format!("Failed to resolve script path: {}", e)))?;
    let input_json = serde_json::to_string(&texts_to_translate)?;

    let mut python_args: Vec<String> = vec![
        script_path.to_string_lossy().to_string(),
        "--model-path".to_string(),
        model_path.to_string_lossy().to_string(),
        "--text".to_string(),
        input_json,
    ];

    let shell_scope = app_handle.shell();
    let (mut rx, child_process) = shell_scope.command(python_path.to_string_lossy().to_string())
        .args(python_args)
        .spawn()
        .map_err(|e| CommandError::from(format!("Failed to spawn Python script: {}", e)))?;

    let shared_child = Arc::new(tokio::sync::Mutex::new(Some(child_process)));
    let child_pid = shared_child.lock().await.as_ref().map(|c| c.pid());
    info!("[Translate Python CMD][{}] Spawned Python process (PID: {:?})", job_id, child_pid);

    let mut python_stdout = Vec::new();
    let mut python_stderr = Vec::new();
    let mut python_exit_code: Option<i32> = None;
    let mut python_error: Option<String> = None;
    let mut cancellation_initiated = false;

    loop {
        tokio::select! {
            biased;
            maybe_event = rx.recv() => {
                match maybe_event {
                    Some(event) => match event {
                        tauri_plugin_shell::process::CommandEvent::Stdout(line) => {
                            debug!("[Translate Python CMD][stdout][{}] {}", job_id, String::from_utf8_lossy(&line).trim_end());
                            python_stdout.extend_from_slice(&line);
                        },
                        tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                            debug!("[Translate Python CMD][stderr][{}] {}", job_id, String::from_utf8_lossy(&line).trim_end());
                            python_stderr.extend_from_slice(&line);
                        },
                        tauri_plugin_shell::process::CommandEvent::Error(msg) => {
                            error!("[Translate Python CMD][error][{}] {}", job_id, msg);
                            python_error = Some(msg);
                            break;
                        },
                        tauri_plugin_shell::process::CommandEvent::Terminated(payload) => {
                            info!("[Translate Python CMD][term][{}] Process terminated. Code: {:?}, Signal: {:?}", job_id, payload.code, payload.signal);
                            python_exit_code = payload.code;
                            if payload.signal.is_some() && python_exit_code.is_none() {
                                python_exit_code = Some(-1); // Indicate abnormal termination
                            }
                            break;
                        },
                        _ => {}
                    },
                    None => {
                        if python_exit_code.is_none() && python_error.is_none() {
                            warn!("[Translate Python CMD][{}] Event channel closed unexpectedly before termination signal.", job_id);
                            python_exit_code = Some(-1);
                        }
                        break;
                    }
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {
                if cancel_flag.load(AtomicOrdering::Relaxed) && !cancellation_initiated {
                    warn!("[Translate Python CMD][{}] Cancellation requested. Killing Python process...", job_id);
                    if let Some(child_to_kill) = shared_child.lock().await.take() {
                        let _ = child_to_kill.kill();
                    }
                    cancellation_initiated = true;
                    // Do not return here, continue the loop to wait for Terminated event
                }
            }
        }
    }

    if cancellation_initiated {
        warn!("[Translate Python CMD][{}] Process terminated due to cancellation. Returning cancellation error.", job_id);
        return Err(CommandError::from(format!("Translation cancelled for job {}.", job_id)));
    }

    if python_error.is_some() || python_exit_code != Some(0) {
        let stderr_output = String::from_utf8_lossy(&python_stderr);
        error!("[Translate Python CMD][{}] Python script failed. Code: {:?}, Error: {:?}\nStderr:\n{}", job_id, python_exit_code, python_error, stderr_output);
        return Err(CommandError::from(format!("Translation script failed. Code: {:?}. Error: {}. Stderr: {}", python_exit_code, python_error.unwrap_or_default(), stderr_output)));
    }

    let translated_output = String::from_utf8_lossy(&python_stdout);

    if cancel_flag.load(AtomicOrdering::Relaxed) { return Err(CommandError::from("Translation cancelled by user.")); }

    emit_translation_progress(&app_handle, &job_id, 80.0, "Processing results...");

    let translated_texts: Vec<String> = serde_json::from_str(&translated_output)?;

    if translated_texts.len() != texts_to_translate.len() {
        return Err(CommandError::from("Translation count mismatch."));
    }

    if let Some(table_node) = lexical_json.get_mut("root").and_then(|r| r.get_mut("children")).and_then(|c| c.as_array_mut()).and_then(|c| c.iter_mut().find(|n| n.get("type").and_then(|t| t.as_str()) == Some("table"))) {
        if let Some(rows) = table_node.get_mut("children").and_then(|c| c.as_array_mut()) {
            for (row, translated_text) in rows.iter_mut().skip(1).zip(translated_texts.iter()) {
                if let Some(cells) = row.get_mut("children").and_then(|c| c.as_array_mut()) {
                    if cells.len() > 3 {
                        if let Some(text_cell) = cells.get_mut(3) {
                            let new_lexical_text = create_lexical_with_text(translated_text);
                            if let Some(new_children) = new_lexical_text.pointer("/root/children") {
                                text_cell["children"] = new_children.clone();
                            }
                        }
                    }
                }
            }
        }
    }

    let target_lang = model_name.split('-').last().unwrap_or("trans");
    let new_path = transcript_path.replace(".json", &format!(".{}.json", target_lang));
    let new_content = serde_json::to_string_pretty(&lexical_json)?;
    fs::write(&new_path, &new_content)?;

    emit_translation_progress(&app_handle, &job_id, 95.0, "Saving translated transcript...");

    save_transcript_json(project_xml_path, new_path.clone(), new_content, Some(target_lang.to_string())).await?;

    emit_translation_progress(&app_handle, &job_id, 100.0, "Translation complete.");

    Ok(new_path)
}

#[tauri::command]
pub async fn cancel_translation_command(
    job_id: String,
    cancel_state: tauri::State<'_, TranslationCancellationState>,
) -> Result<(), String> {
    info!("[Translate Cancel][{}] Received cancellation request.", job_id);
    if let Some(flag_entry) = cancel_state.0.get(&job_id) {
        flag_entry.value().store(true, AtomicOrdering::Relaxed);
        info!("[Translate Cancel][{}] Cancellation flag set.", job_id);
    } else {
        warn!("[Translate Cancel][{}] Job ID not found in cancellation state.", job_id);
    }
    Ok(())
}
