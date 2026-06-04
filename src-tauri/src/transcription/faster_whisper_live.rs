use crate::projectview::transcription_commands::{LiveTranscriptionResult, LiveTranscriptionState};
use crate::welcome::python_env::get_python_path;
use log::{error, info, warn};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

pub async fn start_faster_whisper_live<R: Runtime>(
    app_handle: AppHandle<R>,
    model_path: String,
    language: String,
    save_audio: bool,
    active_document_path: String,
    project_uuid: String,
    project_base_dir: String,
    engine: Option<String>,
    state: tauri::State<'_, LiveTranscriptionState>,
) -> Result<bool, String> {
    let python_path = get_python_path().map_err(|e| e.to_string())?;

    let script_path = app_handle
        .path()
        .resolve(
            "scripts/run_live_transcription.py",
            tauri::path::BaseDirectory::Resource,
        )
        .map_err(|e| format!("Failed to resolve live transcription script path: {}", e))?;

    let mut args = vec![
        script_path.to_string_lossy().to_string(),
        "--model".to_string(),
        model_path,
        "--language".to_string(),
        language,
        "--step".to_string(),
        "5000".to_string(),
        "--length".to_string(),
        "5000".to_string(),
    ];

    if let Some(eng) = engine {
        if eng == "crisper-whisper" {
            args.push("--without_timestamps".to_string());
            args.push("--beam_size".to_string());
            args.push("1".to_string());
        }
    }

    if save_audio {
        let active_doc_path = std::path::PathBuf::from(&active_document_path);
        let attachments_dir = active_doc_path.parent().unwrap().join("attachments");
        std::fs::create_dir_all(&attachments_dir).map_err(|e| e.to_string())?;
        let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
        let wav_path = attachments_dir.join(format!("{}.wav", timestamp));
        args.push("--save-audio".to_string());
        args.push(wav_path.to_string_lossy().to_string());
    }

    info!(
        "[Faster-Whisper Live] Spawning Python script: {:?} {:?}",
        python_path, args
    );

    let (mut rx, child) = app_handle
        .shell()
        .command(python_path.to_string_lossy().to_string())
        .args(args)
        .spawn()
        .map_err(|e| e.to_string())?;

    *state.whisper_child.lock().await = Some(child);
    state.is_running.store(true, Ordering::SeqCst);
    *state.start_time.lock().await = Some(chrono::Utc::now());
    *state.active_document_path.lock().await = Some(active_document_path);
    *state.project_uuid.lock().await = Some(project_uuid);
    *state.project_base_dir.lock().await = Some(PathBuf::from(project_base_dir));

    let is_running_clone = state.is_running.clone();
    let app_handle_clone = app_handle.clone();
    let start_time_clone = *state.start_time.lock().await;

    tokio::spawn(async move {
        info!("[Faster-Whisper Live] Started listening to python stdout.");
        let mut last_text = String::new();
        let mut segment_start_time = 0.0;

        while let Some(event) = rx.recv().await {
            if !is_running_clone.load(Ordering::SeqCst) {
                break;
            }
            match event {
                CommandEvent::Stdout(line) => {
                    let text = String::from_utf8_lossy(&line).to_string();
                    let trimmed = text.trim();

                    // Detect JSON error objects emitted by the Python script
                    // (e.g. microphone permission denied, model load failure).
                    if trimmed.starts_with('{') {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
                            if let Some(err_msg) = v.get("error").and_then(|e| e.as_str()) {
                                error!("[Faster-Whisper Live] Script error: {}", err_msg);
                                let _ = app_handle_clone
                                    .emit("live_transcription_error", err_msg.to_string());
                                is_running_clone.store(false, Ordering::SeqCst);
                                break;
                            }
                        }
                    }

                    if trimmed.contains("[Start speaking]") {
                        let _ = app_handle_clone.emit("live_transcription_ready", ());
                    }

                    let cleaned_text = trimmed.replace("[Start speaking]", "").trim().to_string();

                    if !cleaned_text.is_empty() && cleaned_text != last_text {
                        let is_final = !cleaned_text.ends_with("...");
                        let end_time = if let Some(start_time) = start_time_clone {
                            (chrono::Utc::now() - start_time).num_milliseconds() as f64 / 1000.0
                        } else {
                            0.0
                        };
                        let _ = app_handle_clone.emit(
                            "live_transcription_result",
                            LiveTranscriptionResult {
                                text: cleaned_text.clone(),
                                is_final,
                                start_time: segment_start_time,
                                end_time,
                            },
                        );
                        if is_final {
                            last_text = cleaned_text;
                            segment_start_time = end_time;
                        }
                    }
                }
                CommandEvent::Stderr(line) => {
                    let msg = String::from_utf8_lossy(&line).to_string();
                    // faster-whisper logs progress to stderr — only surface real errors
                    if msg.to_lowercase().contains("error") || msg.to_lowercase().contains("failed") {
                        error!("[Faster-Whisper Live][stderr]: {}", msg.trim());
                    } else {
                        warn!("[Faster-Whisper Live][stderr]: {}", msg.trim());
                    }
                }
                CommandEvent::Error(err) => {
                    error!("[Faster-Whisper Live][error]: {}", err);
                }
                CommandEvent::Terminated(payload) => {
                    info!("[Faster-Whisper Live] Process terminated: {:?}", payload);
                    is_running_clone.store(false, Ordering::SeqCst);
                }
                _ => {}
            }
        }
        info!("[Faster-Whisper Live] Stopped listening.");
    });

    Ok(true)
}
