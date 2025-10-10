use tauri::{command, AppHandle, Manager};
use tauri_plugin_shell::ShellExt;
use std::fs;
use serde_json::{Value, json};
use log::{info, error};
use super::transcription_commands::save_transcript_json;
use crate::welcome::config::{read_config, get_default_download_location};
use crate::welcome::python_env::get_python_path;
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

#[tauri::command]
pub async fn translate_transcript_command(
    app_handle: AppHandle,
    window: tauri::Window,
    project_xml_path: String,
    transcript_path: String,
    model_name: String,
    target_language: String,
) -> Result<String, String> {
    info!("[Translate] Starting translation for transcript: {}", transcript_path);

    let config = read_config().map_err(|e| e.to_string())?;
    let download_location = if !config.download_location.trim().is_empty() {
        config.download_location.clone()
    } else {
        get_default_download_location().map_err(|e| e.to_string())?
    };
    // Correctly construct the path to the Hugging Face model snapshot directory.
    // The hf-hub library downloads models into a specific structure.
    let model_cache_dir_name = format!("models--{}", model_name.replace('/', "--"));
    let model_base_path = std::path::Path::new(&download_location).join(model_cache_dir_name);

    let refs_path = model_base_path.join("refs/main");
    let commit_hash = fs::read_to_string(refs_path)
        .map_err(|e| format!("Failed to read commit hash for model '{}': {}", model_name, e))?
        .trim()
        .to_string();

    let model_path = model_base_path.join("snapshots").join(commit_hash);
    info!("[Translate] Using model path: {}", model_path.display());

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
        info!("[Translate] No text found to translate.");
        return Ok(transcript_path);
    }

    let python_path = get_python_path().map_err(|e| e.to_string())?;
    info!("[Translate] Using Python path: {}", python_path.display());
    let script_path = app_handle.path().resolve("scripts/run_translation.py", tauri::path::BaseDirectory::Resource)
        .map_err(|e| format!("Failed to resolve script path: {}", e))?;

    let input_text = texts_to_translate.join("\n");
    info!("[Translate] Input text to Python script: \n{}", input_text);

    let output = app_handle.shell().command(python_path.to_string_lossy().to_string())
        .args(&[
            script_path.to_string_lossy().to_string(),
            "--model-path".to_string(),
            model_path.to_string_lossy().to_string(),
            "--text".to_string(),
            input_text,
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to execute Python script: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("[Translate Python Stderr] {}", stderr);
        return Err(format!("Translation script failed with exit code {:?}: {}", output.status.code(), stderr));
    }

    let translated_output = String::from_utf8_lossy(&output.stdout);
    info!("[Translate] Translated output from Python script: \n{}", translated_output);

    let translated_texts: Vec<String> = translated_output.lines().map(|s| s.to_string()).collect();

    if translated_texts.len() != texts_to_translate.len() {
        return Err(format!("Translation count mismatch. Expected {}, got {}.", texts_to_translate.len(), translated_texts.len()));
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
    let new_content = serde_json::to_string_pretty(&lexical_json).map_err(|e| e.to_string())?;
    fs::write(&new_path, &new_content).map_err(|e| e.to_string())?;

    info!("[Translate] New translated transcript path: {}", new_path);

    save_transcript_json(project_xml_path, new_path.clone(), new_content, Some(target_lang.to_string()))
        .await
        .map_err(|e| format!("[Translate] Failed to register translated transcript: {}", e))?;

    Ok(new_path)
}
