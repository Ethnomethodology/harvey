import re

with open('src-tauri/src/welcome/commands.rs', 'r') as f:
    text = f.read()

# Add cancellation parameter
text = re.sub(
    r'(pub async fn download_faster_whisper_model_command\(\n\s*app: AppHandle,)',
    r"\1\n    cancellation_state: tauri::State<'_, DownloadCancellationState>,",
    text
)
text = re.sub(
    r'(pub async fn download_crisper_whisper_model_command\(\n\s*app: AppHandle,)',
    r"\1\n    cancellation_state: tauri::State<'_, DownloadCancellationState>,",
    text
)
text = re.sub(
    r'(pub async fn download_translation_model_command\(\n\s*app: AppHandle,)',
    r"\1\n    cancellation_state: tauri::State<'_, DownloadCancellationState>,",
    text
)

# Replace loops
def replace_loop(match):
    prefix = match.group(1) # should be whitespace
    target_dir_str = "target_dir_str.contains(\"translation\")"
    
    return prefix + """let mut success = false;
    loop {
        if let Some(flag_entry) = cancellation_state.0.get(&model_name) {
            if flag_entry.value().load(std::sync::atomic::Ordering::Relaxed) {
                log::info!("Cancellation detected for {}, killing process.", &model_name);
                let _ = _child.kill();
                
                let model_folder_name = format!("models--{}", model_name.replace("/", "--"));
                let model_folder_path = target_dir.join(&model_folder_name);
                
                if model_folder_path.exists() {
                    log::info!("Deleting incomplete download folder: {:?}", model_folder_path);
                    let _ = std::fs::remove_dir_all(&model_folder_path);
                }
                
                window.emit("transcription-download-finished", ()).unwrap();
                return Err(CommandError::Message("Download cancelled by user.".to_string()));
            }
        }
        
        match tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv()).await {
            Ok(Some(event)) => {
                match event {
                    tauri_plugin_shell::process::CommandEvent::Stdout(line) => {
                        let line_str = String::from_utf8_lossy(&line).to_string();
                        log::info!("[Python] {}", &line_str);
                        let _ = window.emit(
                            if target_dir_str.contains("translation") { "translation-download-log" } else { "transcription-download-log" },
                            serde_json::json!({ "model_name": &model_name, "log_line": &line_str }),
                        );
                    }
                    tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                        let line_str = String::from_utf8_lossy(&line).to_string();
                        log::error!("[Python] {}", &line_str);
                        let _ = window.emit(
                            if target_dir_str.contains("translation") { "translation-download-log" } else { "transcription-download-log" },
                            serde_json::json!({ "model_name": &model_name, "log_line": &line_str }),
                        );
                        
                        if target_dir_str.contains("translation") && line_str.starts_with("PROGRESS:") {
                            let parts: Vec<&str> = line_str.trim().split(':').collect();
                            if parts.len() >= 3 {
                                if let Ok(percent) = parts[1].parse::<u32>() {
                                    let file_name = parts[2].trim();
                                    let _ = window.emit(
                                        "translation-download-progress",
                                        serde_json::json!({
                                            "model_name": &model_name,
                                            "percent": percent,
                                            "file_name": file_name
                                        }),
                                    );
                                }
                            }
                        }
                    }
                    tauri_plugin_shell::process::CommandEvent::Terminated(payload) => {
                        log::info!("Download process for '{}' terminated with code: {:?}", &model_name, payload.code);
                        if payload.code == Some(0) {
                            success = true;
                        } else {
                            let _ = window.emit(
                                if target_dir_str.contains("translation") { "translation-download-error" } else { "transcription-download-error" },
                                serde_json::json!({ "model_name": &model_name, "error_message": "Download script failed" })
                            );
                        }
                        break;
                    }
                    _ => {}
                }
            }
            Ok(None) => break,
            Err(_) => {}
        }
    }

    if !success {"""

# Replace each occurrence of the loop up to "if !success {"
text = re.sub(
    r'(\s*)let mut success = false;\s*while let Some\(event\) = rx\.recv\(\)\.await \{\s*match event \{[\s\S]*?\}\s*\}\s*if !success \{',
    replace_loop,
    text
)

with open('src-tauri/src/welcome/commands.rs', 'w') as f:
    f.write(text)
