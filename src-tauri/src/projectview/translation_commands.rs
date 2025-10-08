
use tauri::{command, AppHandle};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;
use std::fs;
use serde_json::{Value, json};
use log::{info, error};
use super::transcription_commands::save_transcript_json;
use crate::welcome::config::{read_config, get_default_download_location};

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



#[command]
pub async fn translate_transcript_command(
    app_handle: AppHandle,
    project_xml_path: String,
    transcript_path: String,
    model_name: String,
    target_lang: String
) -> Result<String, String> {
    info!("[Translate JS] Starting translation for transcript: {}", transcript_path);

    // Get custom model download location from config
    let config = read_config().map_err(|e| e.to_string())?;
    let download_location = if !config.download_location.trim().is_empty() {
        config.download_location.clone()
    } else {
        get_default_download_location().map_err(|e| e.to_string())? 
    };
    info!("[Translate JS] Using model cache directory: {}", download_location);

    // 1. Read transcript and extract text segments
    let content = fs::read_to_string(&transcript_path).map_err(|e| e.to_string())?;
    let mut lexical_json: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let texts_to_translate: Vec<String> = if let Some(table_node) = lexical_json.get("root").and_then(|r| r.get("children")).and_then(|c| c.as_array()).and_then(|c| c.iter().find(|n| n.get("type").and_then(|t| t.as_str()) == Some("table"))) {
        if let Some(rows) = table_node.get("children").and_then(|c| c.as_array()) {
            rows.iter().skip(1).filter_map(|row| {
                row.get("children").and_then(|c| c.as_array()).and_then(|cells| cells.get(3)).map(|cell| extract_plain_text_from_lexical(cell))
            }).collect()
        } else { Vec::new() }
    } else { Vec::new() };

    if texts_to_translate.is_empty() {
        info!("[Translate JS] No text found to translate.");
        return Ok(transcript_path);
    }

    // 2. Execute sidecar script for translation
    info!("[Translate JS] Using model: {}", &model_name);

    let (mut rx, mut child) = app_handle.shell().sidecar("translator-sidecar")
        .map_err(|e| format!("Failed to create sidecar command: {}. This is a packaging issue.", e))? 
        .args([&model_name, &download_location])
        .spawn()
        .map_err(|e| format!("Failed to spawn sidecar: {}. The sidecar executable may be missing or have permission issues.", e))?;

    let input_text = texts_to_translate.join("\n");
    child.write(input_text.as_bytes()).map_err(|e| e.to_string())?;

    // 3. Read translated texts from sidecar stdout
    let mut translated_texts = Vec::new();
    let mut buffer = String::new();
    let mut exit_code = None;

    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(line_bytes) => {
                let line = String::from_utf8_lossy(&line_bytes).to_string();
                if line == "---END_OF_TRANSLATION---" {
                    translated_texts.push(buffer.trim_end().to_string());
                    buffer.clear();
                } else {
                    buffer.push_str(&line);
                    buffer.push('\n');
                }
            }
            CommandEvent::Stderr(line) => {
                error!("[Translate JS Sidecar Stderr] {}", String::from_utf8_lossy(&line));
            }
            CommandEvent::Terminated(payload) => {
                info!("[Translate JS] Sidecar process terminated with payload: {:?}", payload);
                exit_code = payload.code;
                break;
            }
            _ => {}
        }
    }

    if exit_code.is_none() || exit_code != Some(0) {
        error!("[Translate JS] Sidecar process exited with non-zero status or was terminated.");
        return Err("Sidecar process failed. Check application logs for errors from the sidecar.".to_string());
    }

    if translated_texts.len() != texts_to_translate.len() {
        return Err(format!("Translation count mismatch. Expected {}, got {}. Instead, got {}.", texts_to_translate.len(), translated_texts.len(), translated_texts.join(", ")));
    }

    // 4. Reconstruct the new transcript JSON
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

    // 5. Save the new transcript file
    let new_path = transcript_path.replace(".json", &format!(".{}.json", target_lang));
    let new_content = serde_json::to_string_pretty(&lexical_json).map_err(|e| e.to_string())?;
    fs::write(&new_path, &new_content).map_err(|e| e.to_string())?;

    save_transcript_json(project_xml_path, new_path.clone(), new_content, Some(target_lang))
        .await
        .map_err(|e| format!("[Translate] Failed to register translated transcript: {}", e))?;

    Ok(new_path)
}
