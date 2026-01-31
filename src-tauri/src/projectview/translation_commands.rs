// src-tauri/src/projectview/translation_commands.rs
use std::path::PathBuf;
use tauri::{AppHandle, Runtime, Emitter};
use std::fs;
use std::sync::{Arc, atomic::{AtomicBool, Ordering as AtomicOrdering}};
use serde_json::{Value, json};
use log::{info, error, debug, warn};
use super::transcription_commands::save_transcript_json;
use super::document_commands::save_document_and_update_xml;
use crate::welcome::config::{read_config, get_default_download_location, CommandError};
use dashmap::DashMap;
use crate::TranslationCancellationState;
use crate::transcription::TranslationEngine;
use crate::transcription::python_engine::PythonTranslationEngine;


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

#[derive(Debug, Clone, Copy, PartialEq)]
enum TranslationMode {
    Document,
    Transcript,
    ImportedTranscript,
}

impl std::fmt::Display for TranslationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TranslationMode::Document => write!(f, "document"),
            TranslationMode::Transcript => write!(f, "transcript"),
            TranslationMode::ImportedTranscript => write!(f, "imported_transcript"),
        }
    }
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

// Recursive helper for documents
fn extract_texts_recursive(node: &Value, texts: &mut Vec<String>) {
    if let Some(obj) = node.as_object() {
        if let Some(type_) = obj.get("type").and_then(|t| t.as_str()) {
            if matches!(type_, "paragraph" | "heading" | "quote" | "listitem") {
                texts.push(extract_plain_text_from_lexical(node));
                return;
            }
        }
    }
    if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
        for child in children {
            extract_texts_recursive(child, texts);
        }
    } else if let Some(root) = node.get("root") {
        extract_texts_recursive(root, texts);
    }
}

// Recursive helper for documents
fn apply_translations_recursive(node: &mut Value, translations: &mut std::vec::IntoIter<String>) {
    if let Some(obj) = node.as_object() {
        if let Some(type_) = obj.get("type").and_then(|t| t.as_str()) {
            if matches!(type_, "paragraph" | "heading" | "quote" | "listitem") {
                if let Some(new_text) = translations.next() {
                     if let Some(children) = node.get_mut("children") {
                         *children = json!([{ 
                            "type": "text",
                            "text": new_text,
                            "detail": 0,
                            "format": 0,
                            "mode": "normal",
                            "style": "",
                            "version": 1
                         }]);
                     }
                }
                return;
            }
        }
    }
    
    if let Some(children) = node.get_mut("children").and_then(|c| c.as_array_mut()) {
        for child in children {
            apply_translations_recursive(child, translations);
        }
    } else if let Some(root) = node.get_mut("root") {
        apply_translations_recursive(root, translations);
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
    translate_file_command(app_handle, project_xml_path, transcript_path, model_name, target_language, cancel_state, TranslationMode::Transcript).await
}

#[tauri::command]
pub async fn translate_document_command<R: Runtime>(
    app_handle: AppHandle<R>,
    project_xml_path: String,
    document_path: String,
    model_name: String,
    target_language: String,
    cancel_state: tauri::State<'_, TranslationCancellationState>,
) -> Result<TranslationInitiatedPayload, String> {
    translate_file_command(app_handle, project_xml_path, document_path, model_name, target_language, cancel_state, TranslationMode::Document).await
}

#[tauri::command]
pub async fn translate_imported_transcript_command<R: Runtime>(
    app_handle: AppHandle<R>,
    project_xml_path: String,
    transcript_path: String,
    model_name: String,
    target_language: String,
    cancel_state: tauri::State<'_, TranslationCancellationState>,
) -> Result<TranslationInitiatedPayload, String> {
    translate_file_command(app_handle, project_xml_path, transcript_path, model_name, target_language, cancel_state, TranslationMode::ImportedTranscript).await
}

async fn translate_file_command<R: Runtime>(
    app_handle: AppHandle<R>,
    project_xml_path: String,
    file_path: String,
    model_name: String,
    target_language: String,
    cancel_state: tauri::State<'_, TranslationCancellationState>,
    mode: TranslationMode,
) -> Result<TranslationInitiatedPayload, String> {
    let job_id = uuid::Uuid::new_v4().to_string();
    info!("[Translate Command][{}] Received request for file: {} (mode={})", job_id, file_path, mode);

    let cancel_flag = Arc::new(AtomicBool::new(false));
    cancel_state.0.insert(job_id.clone(), Arc::clone(&cancel_flag));

    let app_handle_clone = app_handle.clone();
    let job_id_clone = job_id.clone();
    let cancel_state_for_spawn = cancel_state.inner().0.clone();

    tokio::spawn(async move {
        let _cancel_guard = CancelGuard {
            job_id: job_id_clone.clone(),
            state: cancel_state_for_spawn,
        };
        let completion_payload = match run_translation_process(
            app_handle_clone,
            job_id_clone.clone(),
            project_xml_path,
            file_path,
            model_name,
            target_language,
            cancel_flag,
            mode,
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
    file_path: String,
    model_name: String,
    target_language: String,
    cancel_flag: Arc<AtomicBool>,
    mode: TranslationMode,
) -> Result<String, CommandError> {
    use crate::projectview::shared_utils;

    let normalized_project_xml_path = shared_utils::normalize_path_for_comparison(&PathBuf::from(&project_xml_path)).to_string_lossy().to_string();
    let normalized_file_path = shared_utils::normalize_path_for_comparison(&PathBuf::from(&file_path)).to_string_lossy().to_string();

    emit_translation_progress(&app_handle, &job_id, 5.0, "Preparing for translation...");

    let config = read_config()?;
    let download_location = if !config.download_location.trim().is_empty() {
        config.download_location.clone()
    } else {
        get_default_download_location()? 
    };

    // Determine family and source language
    let is_nllb = model_name.to_lowercase().contains("nllb");
    let family = if is_nllb { "nllb" } else { "helsinki" };
    let org_dir = if family == "nllb" { "facebook" } else { "helsinki-nlp" };

    // Extract source language from file if possible
    let mut source_lang = None;
    let content = fs::read_to_string(&normalized_file_path)?;
    let mut lexical_json: Value = serde_json::from_str(&content)?;

    // Try to find language code in the file metadata if it's a transcript
    if let Some(metadata_path) = shared_utils::get_metadata_path(&PathBuf::from(&normalized_file_path)) {
        if metadata_path.exists() {
            if let Ok(metadata_content) = fs::read_to_string(metadata_path) {
                if let Ok(metadata_json) = serde_json::from_str::<Value>(&metadata_content) {
                    if let Some(lang) = metadata_json.get("metadata").and_then(|m| m.get("language_code")).and_then(|l| l.as_str()) {
                        source_lang = Some(lang.to_string());
                    }
                }
            }
        }
    }

    // Fallback for source language if not found in metadata
    if source_lang.is_none() && family == "helsinki" {
        let parts: Vec<&str> = model_name.split('/').collect();
        let model_id = parts.last().unwrap_or(&"");
        let lang_parts: Vec<&str> = model_id.split('-').collect();
        source_lang = lang_parts.get(2).map(|s| s.to_string());
    }

    let model_cache_dir_name = format!("models--{}", model_name.replace('/', "--"));
    let sub_dir = PathBuf::from("translation").join(org_dir);
    let model_base_path = std::path::Path::new(&download_location).join(&sub_dir).join(&model_cache_dir_name);
    
    // Fallback: check legacy path if new path doesn't exist (only for helsinki)
    let model_base_path = if model_base_path.exists() {
        model_base_path
    } else if family == "helsinki" {
        let legacy_path = std::path::Path::new(&download_location).join(&model_cache_dir_name);
        if legacy_path.exists() {
             legacy_path
        } else {
             model_base_path
        }
    } else {
        model_base_path
    };

    let refs_path = model_base_path.join("refs/main");
    let commit_hash = fs::read_to_string(refs_path).map_err(|e| CommandError::from(format!("Failed to read commit hash for model '{}': {}", model_name, e)))?.trim().to_string();
    let model_path = model_base_path.join("snapshots").join(commit_hash);

    info!("[Translate][{}] Using model path: {}", job_id, model_path.display());

    if cancel_flag.load(AtomicOrdering::Relaxed) { return Err(CommandError::from("Translation cancelled by user.")); }

    let mut texts_to_translate: Vec<String> = Vec::new();

    if mode == TranslationMode::Document {
        // Generic document: Extract all paragraphs/headings/quotes/listitems
        extract_texts_recursive(&lexical_json, &mut texts_to_translate);
    } else if mode == TranslationMode::Transcript {
        // Transcript: Extract only from table -> 4th column (Text)
        if let Some(table_node) = lexical_json.get("root").and_then(|r| r.get("children")).and_then(|c| c.as_array()).and_then(|c| c.iter().find(|n| n.get("type").and_then(|t| t.as_str()) == Some("table"))) {
            if let Some(rows) = table_node.get("children").and_then(|c| c.as_array()) {
                texts_to_translate = rows.iter().skip(1).filter_map(|row| {
                    row.get("children").and_then(|c| c.as_array()).and_then(|cells| cells.get(3)).map(|cell| extract_plain_text_from_lexical(cell))
                }).collect();
            }
        }
    } else if mode == TranslationMode::ImportedTranscript {
        // Imported Transcript: Extract Col 3 (Speaker) and Col 4 (Text)
        if let Some(table_node) = lexical_json.get("root").and_then(|r| r.get("children")).and_then(|c| c.as_array()).and_then(|c| c.iter().find(|n| n.get("type").and_then(|t| t.as_str()) == Some("table"))) {
            if let Some(rows) = table_node.get("children").and_then(|c| c.as_array()) {
                for row in rows.iter().skip(1) {
                    if let Some(cells) = row.get("children").and_then(|c| c.as_array()) {
                        if let Some(speaker_cell) = cells.get(2) {
                            texts_to_translate.push(extract_plain_text_from_lexical(speaker_cell));
                        }
                        if let Some(text_cell) = cells.get(3) {
                            texts_to_translate.push(extract_plain_text_from_lexical(text_cell));
                        }
                    }
                }
            }
        }
    }

    if texts_to_translate.is_empty() {
        info!("[Translate][{}] No text found to translate.", job_id);
        return Ok(normalized_file_path);
    }

    emit_translation_progress(&app_handle, &job_id, 20.0, "Running translation model...");

    let engine = PythonTranslationEngine::new(app_handle.clone());
    let mode_str = if mode == TranslationMode::Document { "document" } else { "transcript" };
    let translated_texts = engine.translate(
        texts_to_translate.clone(),
        &model_path,
        &job_id,
        cancel_flag.clone(),
        mode_str,
        source_lang.as_deref(),
        Some(&target_language),
    ).await?;

    if cancel_flag.load(AtomicOrdering::Relaxed) { return Err(CommandError::from("Translation cancelled by user.")); }

    emit_translation_progress(&app_handle, &job_id, 80.0, "Processing results...");

    if translated_texts.len() != texts_to_translate.len() {
        return Err(CommandError::from("Translation count mismatch."));
    }

    let mut translations_iter = translated_texts.into_iter();

    if mode == TranslationMode::Document {
        apply_translations_recursive(&mut lexical_json, &mut translations_iter);
    } else if mode == TranslationMode::Transcript {
        // Transcript update logic (Col 4 only)
        if let Some(table_node) = lexical_json.get_mut("root").and_then(|r| r.get_mut("children")).and_then(|c| c.as_array_mut()).and_then(|c| c.iter_mut().find(|n| n.get("type").and_then(|t| t.as_str()) == Some("table"))) {
            if let Some(rows) = table_node.get_mut("children").and_then(|c| c.as_array_mut()) {
                for row in rows.iter_mut().skip(1) {
                    if let Some(cells) = row.get_mut("children").and_then(|c| c.as_array_mut()) {
                        if cells.len() > 3 {
                            if let Some(text_cell) = cells.get_mut(3) {
                                if let Some(translated_text) = translations_iter.next() {
                                    let new_lexical_text = create_lexical_with_text(&translated_text);
                                    if let Some(new_children) = new_lexical_text.pointer("/root/children") {
                                        text_cell["children"] = new_children.clone();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    } else if mode == TranslationMode::ImportedTranscript {
        // Imported Transcript update logic (Col 3 & 4)
        if let Some(table_node) = lexical_json.get_mut("root").and_then(|r| r.get_mut("children")).and_then(|c| c.as_array_mut()).and_then(|c| c.iter_mut().find(|n| n.get("type").and_then(|t| t.as_str()) == Some("table"))) {
            if let Some(rows) = table_node.get_mut("children").and_then(|c| c.as_array_mut()) {
                for row in rows.iter_mut().skip(1) {
                    if let Some(cells) = row.get_mut("children").and_then(|c| c.as_array_mut()) {
                        if cells.len() > 3 {
                            // Update Speaker (Col 3)
                            if let Some(speaker_cell) = cells.get_mut(2) {
                                if let Some(translated_speaker) = translations_iter.next() {
                                    let new_lexical_text = create_lexical_with_text(&translated_speaker);
                                    if let Some(new_children) = new_lexical_text.pointer("/root/children") {
                                        speaker_cell["children"] = new_children.clone();
                                    }
                                }
                            }
                            // Update Text (Col 4)
                            if let Some(text_cell) = cells.get_mut(3) {
                                if let Some(translated_text) = translations_iter.next() {
                                    let new_lexical_text = create_lexical_with_text(&translated_text);
                                    if let Some(new_children) = new_lexical_text.pointer("/root/children") {
                                        text_cell["children"] = new_children.clone();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let source_lang_code = source_lang.unwrap_or_else(|| "unk".to_string());
    let target_lang_code = target_language;

    let original_file_stem = std::path::Path::new(&normalized_file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");

    let base_new_filename_stem = format!("{}-{}-{}", original_file_stem, source_lang_code, target_lang_code);
    let mut new_filename = format!("{}.json", base_new_filename_stem);
    let mut counter = 0;
    let mut new_path_buf = std::path::PathBuf::from(&normalized_file_path);
    new_path_buf.set_file_name(&new_filename);

    while new_path_buf.exists() {
        counter += 1;
        new_filename = format!("{}-{}.json", base_new_filename_stem, counter);
        new_path_buf.set_file_name(&new_filename);
    }
    let new_path = new_path_buf.to_string_lossy().to_string();

    let new_content = serde_json::to_string_pretty(&lexical_json)?;
    fs::write(&new_path, &new_content)?;

    emit_translation_progress(&app_handle, &job_id, 95.0, "Saving translated file...");

    // Determine saving logic
    match mode {
        TranslationMode::Document => {
            save_document_and_update_xml(
                normalized_project_xml_path,
                new_path.clone(),
                new_filename,
                new_content
            ).await?;
        },
        TranslationMode::Transcript => {
            save_transcript_json(
                normalized_project_xml_path,
                new_path.clone(),
                new_content,
                Some(format!("{}-{}", source_lang_code, target_lang_code))
            ).await?;
        },
        TranslationMode::ImportedTranscript => {
            use crate::projectview::transcription_handler::save_imported_transcript_and_update_xml;
            save_imported_transcript_and_update_xml(
                normalized_project_xml_path,
                new_path.clone(),
                new_filename,
                new_content
            ).await?;
        }
    }

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

