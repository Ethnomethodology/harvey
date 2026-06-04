import re

with open('src-tauri/src/welcome/commands.rs', 'r') as f:
    content = f.read()

# Add cancellation_state to parameter lists
content = re.sub(
    r'(pub async fn download_faster_whisper_model_command\(\n\s*app: AppHandle,)',
    r'\1\n    cancellation_state: tauri::State<\'_, DownloadCancellationState>,',
    content
)
content = re.sub(
    r'(pub async fn download_crisper_whisper_model_command\(\n\s*app: AppHandle,)',
    r'\1\n    cancellation_state: tauri::State<\'_, DownloadCancellationState>,',
    content
)
content = re.sub(
    r'(pub async fn download_translation_model_command\(\n\s*app: AppHandle,)',
    r'\1\n    cancellation_state: tauri::State<\'_, DownloadCancellationState>,',
    content
)

# Replace the blocking rx.recv loop with a timeout loop
# We'll do this carefully. Since the loop body is similar, we can match the whole loop.

def replace_loop(match):
    prefix = match.group(1)
    return prefix + """
    let mut success = false;
    loop {
        if let Some(flag_entry) = cancellation_state.0.get(&model_name) {
            if flag_entry.value().load(std::sync::atomic::Ordering::Relaxed) {
                log::info!("Cancellation detected for {}, killing process.", &model_name);
                let _ = _child.kill();
                
                // Construct target path to delete (we know it's models--<model_name_replaced>)
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
                        
                        // Capture PROGRESS for translation
                        if target_dir_str.contains("translation") && line_str.starts_with("PROGRESS:") {
                            let parts: Vec<&str> = line_str.trim().split(':').collect();
                            if parts.len() == 3 {
                                if let Ok(percent) = parts[1].parse::<u64>() {
                                    let _ = window.emit(
                                        "translation-download-progress",
                                        serde_json::json!({
                                            "model_name": &model_name,
                                            "downloaded_bytes": percent,
                                            "total_bytes": 100
                                        }),
                                    );
                                }
                            }
                        }
                    }
                    tauri_plugin_shell::process::CommandEvent::Terminated(payload) => {
                        log::info!(
                            "Download process for '{}' terminated with code: {:?}",
                            &model_name,
                            payload.code
                        );
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
            Err(_) => {
                // Timeout, continue loop to check cancellation flag
            }
        }
    }
"""

# The translation loop differs slightly because it handles PROGRESS.
# Using the above loop is safe enough because we check if target_dir_str contains "translation".

content = re.sub(
    r'(\s*)let mut success = false;\s*while let Some\(event\) = rx\.recv\(\)\.await \{\s*match event \{[\s\S]*?\}\s*\}',
    replace_loop,
    content
)

with open('src-tauri/src/welcome/commands.rs', 'w') as f:
    f.write(content)
