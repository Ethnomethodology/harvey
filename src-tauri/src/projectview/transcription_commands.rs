// src-tauri/src/projectview/transcription_commands.rs

use super::shared_types::*;
use super::shared_utils::*;
use crate::welcome::config::{CommandError, read_config, get_default_download_location};
use log::{debug, error, info, warn};
use serde_json::json;
use tauri::Emitter; // Added Emitter
use serde_json::Value as JsonValue;

use std::{
    fs::{self, File},
    io::{BufWriter, Write, BufRead},
    path::{Path, PathBuf},
};
use tauri::{AppHandle};
use tauri_plugin_shell::{ShellExt, process::CommandEvent};
use tokio::time::{sleep, Duration};
use quick_xml;


/// Creates a Lexical JSON structure for a single paragraph containing the given text.
/// This is suitable for the content of a single cell, using ExtendedTextNode.
pub fn create_lexical_paragraph_json_value(text: &str) -> JsonValue {
    json!({
        "type": "paragraph",
        "version": 1,
        "children": [{
            "detail": 0,
            "format": 0,
            "mode": "normal",
            "style": "",
            "text": text,
            "type": "extended-text",
            "version": 1,
            "highlightId": null
        }],
        "direction": "ltr",
        "format": "",
        "indent": 0
    })
}

/// Helper to format timestamp precisely for display in the table.
fn format_timestamp_for_table(seconds: f64) -> String {
    if seconds.is_nan() || seconds.is_infinite() || seconds < 0.0 {
        return "00:00.000".to_string(); 
    }
    let total_millis = (seconds * 1000.0).round() as i64;
    let millis_part = total_millis % 1000;
    let total_seconds_part = total_millis / 1000;
    let secs_part = total_seconds_part % 60;
    let total_minutes_part = total_seconds_part / 60;
    format!("{:02}:{:02}.{:03}", total_minutes_part, secs_part, millis_part)
}


/// Creates a Lexical Table JSON Value from an array of `TranscriptSegment`s.
/// The `text` field in each input `TranscriptSegment` is assumed to be plain text.
pub fn create_lexical_table_from_segments(segments: &[TranscriptSegment]) -> JsonValue {
    let mut table_rows_json: Vec<JsonValue> = Vec::new();

    let col_widths_json: Vec<JsonValue> = vec![
        json!(50),  
        json!(140), 
        json!(120), 
        json!(450)  
    ];

    let header_texts = ["#", "Timestamp", "Speaker", "Text"];
    let mut header_cells_json: Vec<JsonValue> = Vec::new();
    for (idx, header_text) in header_texts.iter().enumerate() {
        header_cells_json.push(json!({
            "type": "tablecell",
            "version": 1,
            "headerState": 2,
            "width": col_widths_json.get(idx).cloned().unwrap_or(JsonValue::Null),
            "children": [
                create_lexical_paragraph_json_value(header_text)
            ]
        }));
    }
    table_rows_json.push(json!({
        "type": "tablerow",
        "version": 1,
        "children": header_cells_json
    }));

    for (index, segment) in segments.iter().enumerate() {
        let mut data_cells_json: Vec<JsonValue> = Vec::new();

        data_cells_json.push(json!({
            "type": "tablecell",
            "version": 1,
            "headerState": 0,
            "width": col_widths_json.get(0).cloned().unwrap_or(JsonValue::Null),
            "children": [
                create_lexical_paragraph_json_value(&format!("{}", index + 1))
            ]
        }));

        let timestamp_str = format!(
            "{} - {}",
            format_timestamp_for_table(segment.start_time),
            format_timestamp_for_table(segment.end_time)
        );
        data_cells_json.push(json!({
            "type": "tablecell",
            "version": 1,
            "headerState": 0,
            "width": col_widths_json.get(1).cloned().unwrap_or(JsonValue::Null),
            "children": [
                create_lexical_paragraph_json_value(&timestamp_str)
            ]
        }));

        data_cells_json.push(json!({
            "type": "tablecell",
            "version": 1,
            "headerState": 0,
            "width": col_widths_json.get(2).cloned().unwrap_or(JsonValue::Null),
            "children": [
                create_lexical_paragraph_json_value(&segment.speaker)
            ]
        }));

        data_cells_json.push(json!({
            "type": "tablecell",
            "version": 1,
            "headerState": 0,
            "width": col_widths_json.get(3).cloned().unwrap_or(JsonValue::Null),
            "children": [
                create_lexical_paragraph_json_value(&segment.text)
            ]
        }));

        table_rows_json.push(json!({
            "type": "tablerow",
            "version": 1,
            "children": data_cells_json
        }));
    }

    json!({
        "root": {
            "children": [{
                "type": "table",
                "version": 1,
                "children": table_rows_json,
            }, {
                "type": "paragraph", 
                "version": 1,
                "children": [],
                "direction": "ltr",
                "format": "",
                "indent": 0
            }],
            "direction": "ltr",
            "format": "",
            "indent": 0,
            "type": "root",
            "version": 1
        }
    })
}


#[tauri::command]
pub async fn trim_media( app_handle: AppHandle, original_media_path: String, start_time: f64, end_time: f64) -> Result<Vec<FileEntry>, CommandError> {
    info!("[Trim Backend] Start: Path='{}', Start={:.3}, End={:.3}", original_media_path, start_time, end_time);
    let original_path = PathBuf::from(&original_media_path);

    if !original_path.exists() || !original_path.is_file() {
        return Err(CommandError::from(format!("Original media not found: {}", original_media_path)));
    }
    if start_time < 0.0 || end_time <= start_time {
        return Err(CommandError::from("Invalid trim times"));
    }

    let media_subdir = original_path.parent().ok_or_else(|| CommandError::from("Could not get media parent dir"))?;
    if media_subdir.file_name().and_then(|n| n.to_str()) != Some(MEDIA_SUBDIR) {
        return Err(CommandError::from(format!("Media not in '{}' subdir", MEDIA_SUBDIR)));
    }
    let media_stem_dir = media_subdir.parent().ok_or_else(|| CommandError::from("Could not get media stem dir"))?;
    let original_media_identifier = media_stem_dir.file_name().and_then(|n| n.to_str()).ok_or_else(|| CommandError::from("Could not get media identifier"))?;
    let media_asset_dir = media_stem_dir.parent().ok_or_else(|| CommandError::from("Could not get Media asset dir"))?;
    if media_asset_dir.file_name().and_then(|n| n.to_str()) != Some(MEDIA_DIR) {
        return Err(CommandError::from(format!("Media stem not in '{}' dir", MEDIA_DIR)));
    }
    let harvey_files_dir = media_asset_dir.parent().ok_or_else(|| CommandError::from(format!("Could not get '{}' dir", HARVEY_FILES_DIR)))?;
    if harvey_files_dir.file_name().and_then(|n| n.to_str()) != Some(HARVEY_FILES_DIR) {
        return Err(CommandError::from(format!("Media asset dir not in '{}' dir", HARVEY_FILES_DIR)));
    }
    let project_base_dir = harvey_files_dir.parent().ok_or_else(|| CommandError::from("Could not get project base dir"))?;
    let project_base_dir_name = project_base_dir.file_name().and_then(|n| n.to_str()).ok_or_else(|| CommandError::from("Could not get project dir name"))?;
    let project_xml_path = project_base_dir.join(format!("{}.harvey.xml", project_base_dir_name));
    let project_xml_path_str = project_xml_path.to_string_lossy().to_string();

    if !project_xml_path.exists() {
        return Err(CommandError::from(format!("Project XML not found: {:?}", project_xml_path)));
    }

    let mut trim_counter = 1;
    let output_stem_dir_name = loop {
        let name = format!("{}_trimmed_{}", original_media_identifier, trim_counter);
        let potential_path = media_asset_dir.join(&name);
        if !potential_path.exists() {
            break name;
        }
        trim_counter += 1;
        if trim_counter > 999 {
            return Err(CommandError::from("Could not find unique trim directory name after 999 attempts."));
        }
    };

    let output_stem_base_path = media_asset_dir.join(&output_stem_dir_name);
    let output_media_subdir = output_stem_base_path.join(MEDIA_SUBDIR);
    let output_transcripts_subdir = output_stem_base_path.join(TRANSCRIPTS_SUBDIR);

    fs::create_dir_all(&output_media_subdir)?;
    fs::create_dir_all(&output_transcripts_subdir)?;

    let original_extension = original_path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let output_filename = format!("{}.{}", output_stem_dir_name, original_extension);
    let output_media_path = output_media_subdir.join(&output_filename);
    let output_media_path_str = output_media_path.to_string_lossy().to_string();

    let args: Vec<String> = vec![
        "-i".into(),
        original_media_path.clone(),
        "-ss".into(),
        format!("{:.6}", start_time),
        "-to".into(),
        format!("{:.6}", end_time),
        "-c".into(),
        "copy".into(),
        "-map".into(),
        "0".into(),
        "-avoid_negative_ts".into(),
        "make_zero".to_string(),
        "-y".into(),
        output_media_path_str.clone(),
    ];

    info!("[Trim Backend] FFmpeg Cmd: ffmpeg {}", args.join(" "));
    let shell_scope = app_handle.shell();
    let (mut rx, _child) = shell_scope
        .sidecar("ffmpeg")?
        .args(args)
        .spawn()?;

    let mut ffmpeg_stderr: Vec<String> = Vec::new();
    let mut ffmpeg_exit_code: Option<i32> = None;
    let mut ffmpeg_error: Option<String> = None;

    while let Some(event) = rx.recv().await {
         match event {
             tauri_plugin_shell::process::CommandEvent::Stdout(line) => { debug!("[FFmpeg Trim][stdout] {}", String::from_utf8_lossy(&line).trim_end()); }
             tauri_plugin_shell::process::CommandEvent::Stderr(line) => { let l = String::from_utf8_lossy(&line).to_string(); debug!("[FFmpeg Trim][stderr] {}", l.trim_end()); ffmpeg_stderr.push(l); }
             tauri_plugin_shell::process::CommandEvent::Error(msg) => { error!("[FFmpeg Trim][error] {}", msg); ffmpeg_error = Some(msg); break; }
             tauri_plugin_shell::process::CommandEvent::Terminated(payload) => { info!("[FFmpeg Trim][term] Code:{:?}, Sig:{:?}", payload.code, payload.signal); ffmpeg_exit_code = payload.code; break; }
             _ => {}
         }
    }

    let stderr_output = ffmpeg_stderr.join("\n");

    if ffmpeg_error.is_some() || ffmpeg_exit_code != Some(0) {
        error!("[Trim Backend] FFmpeg fail. Code:{:?}, Err:{:?}\nStderr:\n{}", ffmpeg_exit_code, ffmpeg_error, stderr_output);
        let _ = fs::remove_dir_all(&output_stem_base_path);
        return Err(CommandError::from(format!("ffmpeg trim failed. Code:{:?}. Error:{:?}", ffmpeg_exit_code, ffmpeg_error)));
    }
    if !output_media_path.exists() || output_media_path.metadata()?.len() == 0 {
        error!("[Trim Backend] FFmpeg output missing or empty: {}", output_media_path.display());
        let _ = fs::remove_dir_all(&output_stem_base_path);
        return Err(CommandError::from(format!("ffmpeg produced an empty or missing output file: {}", output_media_path.display())));
    }

    info!("[Trim Backend] FFmpeg trim success: {}", output_media_path.display());

    info!("[Trim Backend] Updating XML: {}", project_xml_path.display());
    let xml_content = fs::read_to_string(&project_xml_path)?;
    let mut project_data: ProjectXml = quick_xml::de::from_str(&xml_content)?;

    let original_entry = project_data.media_files.files.iter().find(|f| f.name == original_media_identifier).cloned();
    let (original_speakers, _original_transcripts) = match original_entry {
        Some(entry) => (entry.speakers, entry.transcripts),
        None => {
            warn!("[Trim Backend] Original XML entry '{}' not found when trying to copy metadata.", original_media_identifier);
            (None, Vec::new())
        }
    };

    let new_relative_path_for_xml = Path::new(HARVEY_FILES_DIR)
        .join(MEDIA_DIR)
        .join(&output_stem_dir_name)
        .join(MEDIA_SUBDIR)
        .join(&output_filename)
        .to_string_lossy()
        .replace("\\", "/");

    let new_media_entry = MediaFileEntryXml {
        name: output_stem_dir_name.clone(),
        original_path: None,
        relative_path: new_relative_path_for_xml,
        speakers: original_speakers.or_else(|| Some(SpeakersXml::default())),
        transcripts: Vec::new(),
    };

    if !project_data.media_files.files.iter().any(|f| f.name == new_media_entry.name) {
        info!("[Trim Backend] Adding new media entry to XML: {}", new_media_entry.name);
        project_data.media_files.files.push(new_media_entry);
        project_data.media_files.files.sort_by(|a,b| a.name.cmp(&b.name));
        save_project_xml(&project_xml_path, &project_data)?;
        log::info!("[Trim Backend] XML updated.");
    } else {
        warn!("[Trim Backend] Trimmed media ID '{}' already exists in XML. Skipping XML update.", new_media_entry.name);
    }

    info!("[Trim Backend] Reloading project data...");
    super::core_commands::load_project_data(project_xml_path_str).await.map(|data| data.files)
}


#[tauri::command]
pub async fn save_speaker_config( project_xml_path: String, media_identifier: String, count: usize, names: Vec<String>) -> Result<(), CommandError> {
    info!("[Backend SaveSpeakers] Request: Project='{}', MediaID='{}', Count={}, Names={:?}", project_xml_path, media_identifier, count, names);

    let xml_path = PathBuf::from(&project_xml_path);
    if !xml_path.exists() || !xml_path.is_file() {
        return Err(CommandError::from(format!("Project file not found: {}", project_xml_path)));
    }

    let xml_content = fs::read_to_string(&xml_path)?;
    let mut project_data: ProjectXml = quick_xml::de::from_str(&xml_content)?;
    let mut found_and_updated = false;

    if let Some(media_file) = project_data.media_files.files.iter_mut().find(|f| f.name == media_identifier) {
        info!("[Backend SaveSpeakers] Found entry '{}'. Updating speakers.", media_identifier);

        let mut validated_count = count;
        let mut validated_names = names.clone();
        if validated_names.len() != validated_count {
             warn!("Speaker count ({}) and number of names ({}) mismatch. Adjusting count to match names.", validated_count, validated_names.len());
             validated_count = validated_names.len();
        }
         validated_names = validated_names.iter().enumerate().map(|(i, name)| {
            let trimmed = name.trim();
            if trimmed.is_empty() { format!("Speaker {}", i + 1) } else { trimmed.to_string() }
        }).collect();
         let mut unique_names = std::collections::HashSet::new();
         for name in &validated_names {
             if !unique_names.insert(name.clone()) {
                 warn!("Duplicate speaker name detected: '{}'. Frontend should ideally prevent this.", name);
             }
         }

        let speakers_data = SpeakersXml { count: validated_count, names: validated_names };
        info!("[Backend SaveSpeakers] Saving validated config: {:?}", speakers_data);
        media_file.speakers = Some(speakers_data);
        found_and_updated = true;
    }

    if !found_and_updated {
        return Err(CommandError::from(format!("Media ID '{}' not found in XML.", media_identifier)));
    }

    save_project_xml(&xml_path, &project_data)?;
    info!("[Backend SaveSpeakers] Success for '{}'.", media_identifier);
    Ok(())
}


#[tauri::command]
pub async fn load_transcript_json(transcript_path: String) -> Result<String, CommandError> {
    info!("[Backend Load Full Transcript JSON] Path: {}", transcript_path);
    let file_path = PathBuf::from(&transcript_path);

    if !file_path.exists() || !file_path.is_file() {
        return Err(CommandError::from(format!("Transcript file not found: {}", transcript_path)));
    }
    if file_path.extension().and_then(|e| e.to_str()) != Some("json") {
        return Err(CommandError::from("Only .json transcripts are supported for loading.".to_string()));
    }

    let content = fs::read_to_string(&file_path)
        .map_err(|e| CommandError::from(format!("Failed to read transcript file {}: {}", transcript_path, e)))?;
    
    match serde_json::from_str::<JsonValue>(&content) {
        Ok(json_value) => {
            if json_value.get("root").is_some() && json_value.get("root").unwrap().is_object() {
                Ok(content)
            } else {
                Err(CommandError::from("Transcript file content is not a valid Lexical JSON structure (missing root object)."))
            }
        }
        Err(e) => Err(CommandError::from(format!("Failed to parse transcript JSON: {}. File: {}", e, transcript_path))),
    }
}



#[tauri::command]
pub async fn save_transcript_json(
    project_xml_path: String,
    transcript_path: String,
    lexical_table_json_string: String 
) -> Result<(), CommandError> {
    info!("[Backend Save Full Transcript JSON] Transcript Path: {}", transcript_path);
    info!("[Backend Save Full Transcript JSON] Project XML Path: {}", project_xml_path);
    let transcript_path_buf = PathBuf::from(&transcript_path);
    let project_xml_path_buf = PathBuf::from(&project_xml_path);

    let project_base_dir = project_xml_path_buf.parent().ok_or_else(|| CommandError::from("Could not get project base dir from XML path"))?;

    if let Some(parent) = transcript_path_buf.parent() {
        fs::create_dir_all(parent)?;
    } else {
        return Err(CommandError::from(format!("Invalid transcript path (no parent directory): {}", transcript_path)));
    }

    match serde_json::from_str::<JsonValue>(&lexical_table_json_string) {
        Ok(json_value) => {
            if !(json_value.get("root").is_some() && json_value.get("root").unwrap().is_object()) {
                 return Err(CommandError::from("Provided string is not a valid Lexical JSON structure (missing root object)."));
            }
            if let Some(root_children) = json_value.get("root").and_then(|r| r.get("children")).and_then(|c| c.as_array()) {
                if root_children.is_empty() || root_children.first().and_then(|n| n.get("type")).and_then(|t| t.as_str()) != Some("table") {
                     warn!("[Backend Save Full Transcript JSON] Lexical JSON root does not start with a table node. Saving anyway.");
                }
            } else {
                 warn!("[Backend Save Full Transcript JSON] Lexical JSON root has no children or invalid children structure. Saving anyway.");
            }
        }
        Err(e) => return Err(CommandError::from(format!("Provided string is not valid JSON: {}", e))),
    }

    let file = File::create(&transcript_path_buf)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(lexical_table_json_string.as_bytes())
        .map_err(|e| CommandError::from(format!("Failed to write transcript JSON: {}", e)))?;
    writer.flush()?; 
    info!("[Backend Save Full Transcript JSON] Saved Lexical Table JSON to disk: {}", transcript_path_buf.display());


    let transcript_filename = transcript_path_buf.file_name().and_then(|n| n.to_str()).ok_or_else(|| CommandError::from("Could not get transcript filename"))?.to_string();

    let (_item_type, media_identifier_opt, transcript_relative_path_buf) = get_item_details(&transcript_path_buf, project_base_dir)?;
    let media_identifier = media_identifier_opt.ok_or_else(|| CommandError::from(format!("Could not determine media identifier for transcript path: {}", transcript_path)))?;
    let transcript_relative_path = transcript_relative_path_buf.to_string_lossy().replace("\\", "/");

    info!("[Backend Save Full Transcript JSON] Media ID: '{}', Transcript Filename: '{}', Transcript Rel Path: '{}'", media_identifier, transcript_filename, transcript_relative_path);

    let xml_content = fs::read_to_string(&project_xml_path_buf)?;
    let mut project_data: ProjectXml = quick_xml::de::from_str(&xml_content)?;
    let mut found_media = false;

    if let Some(media_entry) = project_data.media_files.files.iter_mut().find(|f| f.name == media_identifier) {
        found_media = true;
        debug!("[Backend Save Full Transcript JSON] Found media entry '{}' in XML.", media_identifier);

        let mut found_transcript_xml_entry = false;
        for transcript_xml_entry_instance in media_entry.transcripts.iter_mut() {
            if transcript_xml_entry_instance.relative_path == transcript_relative_path {
                debug!("[Backend Save Full Transcript JSON] Found existing transcript entry for '{}'. Updating name (if needed).", transcript_relative_path);
                 if transcript_xml_entry_instance.name != transcript_filename {
                     warn!("[Backend Save Full Transcript JSON] Updating transcript name in XML from '{}' to '{}' for path '{}'", transcript_xml_entry_instance.name, transcript_filename, transcript_relative_path);
                    transcript_xml_entry_instance.name = transcript_filename.clone();
                 }
                found_transcript_xml_entry = true;
                break;
            }
        }

        if !found_transcript_xml_entry {
            debug!("[Backend Save Full Transcript JSON] Adding new transcript entry for '{}'.", transcript_relative_path);
            media_entry.transcripts.push(TranscriptEntryXml {
                name: transcript_filename.clone(),
                relative_path: transcript_relative_path.clone(),
            });
             media_entry.transcripts.sort_by(|a,b| a.name.cmp(&b.name));
        }
    }

    if !found_media {
        warn!("[Backend Save Full Transcript JSON] Media identifier '{}' not found in XML. XML not updated.", media_identifier);
         return Err(CommandError::from(format!("Media identifier '{}' not found in XML. Could not link saved transcript.", media_identifier)));
    }

    save_project_xml(&project_xml_path_buf, &project_data)?;
    info!("[Backend Save Full Transcript JSON] Project XML updated.");
    Ok(())
}

// --- prepare_output_paths Helper Function ---
/// Prepares paths for transcription and translation outputs.
///
/// Returns a tuple containing:
/// - `temp_transcript_output_base_orig_str`: Base path for temporary original transcript files (without extension).
/// - `expected_whisper_temp_json_path_orig`: Path to the temporary JSON output from Whisper for the original transcript.
/// - `expected_rttm_temp_path`: Path to the temporary RTTM file for diarization.
/// - `final_transcript_path_orig`: Final path for the original transcript JSON file.
/// - `temp_transcript_output_base_en_str`: Base path for temporary translated transcript files (without extension).
/// - `expected_whisper_temp_json_path_en`: Path to the temporary JSON output from Whisper for the translated transcript.
/// - `final_transcript_path_en`: Final path for the translated transcript JSON file.
pub(crate) fn prepare_output_paths(
    media_path_str: &str,
    job_id: &str,
    translate_to_english: bool,
) -> Result<(String, PathBuf, PathBuf, PathBuf, Option<String>, Option<PathBuf>, Option<PathBuf>), CommandError> {
    debug!("[prepare_output_paths][{}] Media path: {}, Translate: {}", job_id, media_path_str, translate_to_english);
    let media_path = PathBuf::from(media_path_str);

    let media_filename_stem = media_path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| CommandError::from(format!("Invalid media filename: {}", media_path_str)))?.to_string();

    let media_subdir = media_path.parent().ok_or_else(|| CommandError::from("Cannot get parent dir of media file"))?;
    if media_subdir.file_name().and_then(|n| n.to_str()) != Some(MEDIA_SUBDIR) {
        return Err(CommandError::from(format!("Media file not in expected '{}' subdir.", MEDIA_SUBDIR)));
    }
    let media_stem_dir = media_subdir.parent().ok_or_else(|| CommandError::from("Cannot get media stem dir"))?;
    let transcripts_dir = media_stem_dir.join(TRANSCRIPTS_SUBDIR);

    fs::create_dir_all(&transcripts_dir)?;
    debug!("[prepare_output_paths][{}] Transcripts dir ensured: {:?}", job_id, transcripts_dir);

    // Paths for original transcript
    let temp_transcript_output_base_orig = transcripts_dir.join(format!("{}_temp_{}_orig", media_filename_stem, job_id));
    let temp_transcript_output_base_orig_str = temp_transcript_output_base_orig.to_string_lossy().to_string();
    let expected_whisper_temp_json_path_orig = temp_transcript_output_base_orig.with_extension("json");
    let final_transcript_path_orig = transcripts_dir.join(format!("{}.json", media_filename_stem));
    
    // Path for RTTM (common for original transcript diarization)
    let expected_rttm_temp_path = temp_transcript_output_base_orig.with_extension("rttm"); // Use original's temp base for RTTM

    debug!("[prepare_output_paths][{}] Orig Temp Base: '{}', Orig Whisper JSON (temp): '{}', RTTM (temp): '{}', Orig Final JSON: '{}'",
        job_id, temp_transcript_output_base_orig_str, expected_whisper_temp_json_path_orig.display(), expected_rttm_temp_path.display(), final_transcript_path_orig.display());

    // Paths for translated transcript (if requested)
    let mut temp_transcript_output_base_en_str: Option<String> = None;
    let mut expected_whisper_temp_json_path_en: Option<PathBuf> = None;
    let mut final_transcript_path_en: Option<PathBuf> = None;

    if translate_to_english {
        let temp_transcript_output_base_en = transcripts_dir.join(format!("{}_temp_{}_en", media_filename_stem, job_id));
        temp_transcript_output_base_en_str = Some(temp_transcript_output_base_en.to_string_lossy().to_string());
        expected_whisper_temp_json_path_en = Some(temp_transcript_output_base_en.with_extension("json"));
        final_transcript_path_en = Some(transcripts_dir.join(format!("{}.en.json", media_filename_stem)));

        debug!("[prepare_output_paths][{}] EN Temp Base: '{:?}', EN Whisper JSON (temp): '{:?}', EN Final JSON: '{:?}'",
            job_id, temp_transcript_output_base_en_str, expected_whisper_temp_json_path_en, final_transcript_path_en);
    }

    Ok((
        temp_transcript_output_base_orig_str,
        expected_whisper_temp_json_path_orig,
        expected_rttm_temp_path,
        final_transcript_path_orig,
        temp_transcript_output_base_en_str,
        expected_whisper_temp_json_path_en,
        final_transcript_path_en,
    ))
}


// --- Map Speaker IDs to User Names ---
pub fn map_speaker_ids_to_names(
    segments: &mut Vec<TranscriptSegment>,
    user_names: &[String])
{
    if user_names.is_empty() {
        info!("[Name Map] No user-defined speaker names provided. Skipping mapping.");
        return;
    }
    info!("[Name Map] Attempting to map generic speaker IDs to User Names: {:?}", user_names);

    for segment in segments.iter_mut() {
        let original_speaker = segment.speaker.trim();

        let number_part_opt = if original_speaker.starts_with("SPEAKER_") {
            original_speaker.get("SPEAKER_".len()..)
                .and_then(|num_str| num_str.parse::<usize>().ok())
        } else if original_speaker.starts_with("speaker_") {
            original_speaker.get("speaker_".len()..)
                .and_then(|num_str| num_str.parse::<usize>().ok())
                .filter(|&index_1| index_1 > 0)
                .map(|index_1| index_1 - 1) 
        } else {
            None
        };

        if let Some(user_name_index) = number_part_opt {
            if let Some(mapped_name) = user_names.get(user_name_index) {
                 if !mapped_name.trim().is_empty() {
                    debug!("[Name Map] Mapping '{}' -> '{}' (index {}) for segment at {:.3}s",
                           original_speaker, mapped_name, user_name_index, segment.start_time);
                    segment.speaker = mapped_name.clone();
                 } else {
                     warn!("[Name Map] User name at index {} is empty. Keeping original ID '{}'.", user_name_index, original_speaker);
                 }
            } else {
                warn!("[Name Map] Speaker index {} derived from '{}' is out of bounds for user names list (length {}). Keeping original ID.",
                      user_name_index, original_speaker, user_names.len());
            }
        } else {
             if original_speaker != "Unknown" && !user_names.contains(&original_speaker.to_string()) {
                 debug!("[Name Map] Speaker ID '{}' not in generic format & not in user_names. Skipping mapping for this segment.", original_speaker);
             }
        }
    }
    info!("[Name Map] Finished speaker name mapping process.");
}

// --- Main Transcription Command ---
#[derive(serde::Deserialize, Debug)]
pub struct TranscribeMediaPayload {
    project_xml_path: String,
    media_path_str: String,
    num_speakers: usize,
    language_code: Option<String>,
    model_name: String,
    translate_to_english: bool,
    speaker_names: Vec<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct TranscriptionResultPayload {
    original_transcript_path: String,
    translated_transcript_path: Option<String>,
}

#[tauri::command]
pub async fn transcribe_media_command(
    app_handle: AppHandle,
    payload: TranscribeMediaPayload,
) -> Result<TranscriptionResultPayload, CommandError> {
    let job_id = uuid::Uuid::new_v4().to_string();
    info!("[Transcribe Command][{}] Received request: {:?}", job_id, payload);

    let app_handle_clone = app_handle.clone();

    let (
        temp_transcript_output_base_orig_str,
        expected_whisper_temp_json_path_orig,
        expected_rttm_temp_path,
        final_transcript_path_orig,
        temp_transcript_output_base_en_str,
        expected_whisper_temp_json_path_en,
        final_transcript_path_en,
    ) = prepare_output_paths(&payload.media_path_str, &job_id, payload.translate_to_english)?;

    let final_transcript_path_en_for_payload = final_transcript_path_en.clone();

    emit_progress_cmd(&app_handle_clone, &job_id, 1.0, "Preparing audio...")?;
    let wav_media_path = convert_to_wav_if_needed_cmd(&app_handle_clone, &payload.media_path_str, &job_id).await?;
    emit_progress_cmd(&app_handle_clone, &job_id, 5.0, "Audio ready.")?;

    let whisper_model_path_str = resolve_whisper_model_path_cmd(&payload.model_name, &job_id)?;

    // --- First Pass: Original Language Transcription ---
    emit_progress_cmd(&app_handle_clone, &job_id, 10.0, "Transcribing original language...")?;
    let mut original_segments = execute_transcription_pass(
        &app_handle_clone,
        &wav_media_path.to_string_lossy(), // Pass as &str
        &whisper_model_path_str,
        &payload.language_code.clone().unwrap_or_else(|| "auto".to_string()),
        &job_id,
        &temp_transcript_output_base_orig_str,
        &expected_whisper_temp_json_path_orig,
        payload.num_speakers,
        &expected_rttm_temp_path,
        false, // is_translation_pass
        &payload.speaker_names, // Pass as slice
    ).await?;

    map_speaker_ids_to_names(&mut original_segments, &payload.speaker_names);

    emit_progress_cmd(&app_handle_clone, &job_id, 45.0, "Saving original transcript...")?;
    let lexical_json_orig = create_lexical_table_from_segments(&original_segments);
    let lexical_json_orig_str = serde_json::to_string_pretty(&lexical_json_orig)
        .map_err(|e| CommandError::from(format!("Failed to serialize original Lexical Table JSON: {}", e)))?;

    save_transcript_json(
        payload.project_xml_path.clone(),
        final_transcript_path_orig.to_string_lossy().to_string(),
        lexical_json_orig_str,
    ).await?;
    info!("[Transcribe Command][{}] Original transcript saved to: {:?}", job_id, final_transcript_path_orig);

    // --- Second Pass: English Translation (if requested) ---
    if payload.translate_to_english {
        if let (Some(base_en_str), Some(json_path_en), Some(final_path_en_pb)) = (
            temp_transcript_output_base_en_str,
            expected_whisper_temp_json_path_en,
            final_transcript_path_en, // This is an Option<PathBuf>
        ) {
            emit_progress_cmd(&app_handle_clone, &job_id, 55.0, "Translating to English...")?;

            // Safely get owned strings for paths needed by execute_transcription_pass
            let base_en_str_owned = base_en_str.clone();
            let json_path_en_owned = json_path_en.clone();

            let mut translated_segments = execute_transcription_pass(
                &app_handle_clone,
                &wav_media_path.to_string_lossy(), // Pass as &str
                &whisper_model_path_str,
                "en", // Target language for translation is English
                &job_id,
                &base_en_str_owned, // Use owned string
                &json_path_en_owned,  // Use owned PathBuf
                0, // No diarization for translation pass
                &PathBuf::new(), // Empty RTTM path
                true, // is_translation_pass
                &payload.speaker_names, // Pass as slice
            ).await?;

            // Speaker mapping for translated segments can be complex.
            // A simple approach: if segment counts are similar, try to reuse original speakers.
            // This might need refinement based on actual whisper output for translations.
            // For now, let's apply the same mapping, or default if counts differ significantly.
            // A more robust solution might involve aligning segments by time.
            if translated_segments.len() == original_segments.len() {
                 map_speaker_ids_to_names(&mut translated_segments, &payload.speaker_names);
            } else {
                warn!("[Transcribe Command][{}] Segment count mismatch after translation (orig: {}, trans: {}). Speaker names might be less accurate for translated version.", job_id, original_segments.len(), translated_segments.len());
                // Optionally, apply a default "SPEAKER_XX" or clear speakers for translated version
                // For now, try mapping anyway or let map_speaker_ids_to_names handle it based on its logic.
                 map_speaker_ids_to_names(&mut translated_segments, &payload.speaker_names);
            }


            emit_progress_cmd(&app_handle_clone, &job_id, 90.0, "Saving translated transcript...")?;
            let lexical_json_en = create_lexical_table_from_segments(&translated_segments);
            let lexical_json_en_str = serde_json::to_string_pretty(&lexical_json_en)
                .map_err(|e| CommandError::from(format!("Failed to serialize translated Lexical Table JSON: {}", e)))?;

            save_transcript_json(
                payload.project_xml_path.clone(),
                final_path_en_pb.to_string_lossy().to_string(),
                lexical_json_en_str,
            ).await?;
            info!("[Transcribe Command][{}] Translated transcript saved to: {:?}", job_id, final_path_en_pb);
        } else {
            warn!("[Transcribe Command][{}] Translation requested, but English output paths are not available. Skipping translation.", job_id);
        }
    }

    emit_progress_cmd(&app_handle_clone, &job_id, 100.0, "Transcription complete.")?;
    info!("[Transcribe Command][{}] Processing complete.", job_id);

    Ok(TranscriptionResultPayload {
        original_transcript_path: final_transcript_path_orig.to_string_lossy().to_string(),
        translated_transcript_path: if payload.translate_to_english {
            final_transcript_path_en_for_payload.map(|p| p.to_string_lossy().into_owned())
        } else {
            None
        },
    })
}

// --- Implemented Helper Functions ---

// Adapted from local_handler/transcription.rs
// Omitting cancel_flag for now
pub(crate) async fn convert_to_wav_if_needed_cmd(
    app_handle: &AppHandle,
    input_path_str: &str,
    job_id: &str,
) -> Result<PathBuf, CommandError> {
    info!("[FFmpeg CMD][{}] Checking audio file: {}", job_id, input_path_str);
    let input_path = PathBuf::from(input_path_str);
    let extension = input_path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();

    if extension == "wav" {
        info!("[FFmpeg CMD][{}] Input is already WAV. Skipping conversion.", job_id);
        return Ok(input_path);
    }

    let output_wav_path = input_path.with_extension("wav");
    info!("[FFmpeg CMD][{}] Target WAV path: {}", job_id, output_wav_path.display());

    if output_wav_path.exists() {
        match output_wav_path.metadata() {
            Ok(m) if m.len() > 0 => {
                info!("[FFmpeg CMD][{}] Target WAV file already exists and is not empty. Reusing.", job_id);
                return Ok(output_wav_path);
            },
            _ => {
                warn!("[FFmpeg CMD][{}] Target WAV file exists but is empty or metadata error. Overwriting.", job_id);
            }
        }
    }

    info!("[FFmpeg CMD][{}] Starting FFmpeg conversion...", job_id);
    // Using emit_progress_cmd from this file
    let _ = emit_progress_cmd(app_handle, job_id, 2.0, "Converting audio to WAV...")?;

    let args: Vec<String> = vec![
        "-i".into(), input_path_str.to_string(),
        "-vn".into(),
        "-acodec".into(), "pcm_s16le".into(),
        "-ar".into(), "16000".into(),
        "-ac".into(), "1".into(),
        "-y".into(),
        output_wav_path.to_string_lossy().to_string(),
    ];
    debug!("[FFmpeg CMD][{}] Command arguments: {:?}", job_id, args);

    let shell_scope = app_handle.shell();
    let (mut rx, child) = shell_scope
        .sidecar("ffmpeg")?
        .args(args)
        .spawn()?;
    debug!("[FFmpeg CMD][{}] Spawned FFmpeg process (PID: {:?})", job_id, child.pid());

    let mut ffmpeg_stderr: Vec<String> = Vec::new();
    let mut ffmpeg_exit_code: Option<i32> = None;
    let mut ffmpeg_error: Option<String> = None;

    loop {
        // Cancellation logic omitted for this adaptation
        // if cancel_flag.load(Ordering::Relaxed) {
        //     warn!("[FFmpeg CMD][{}] Cancellation requested. Killing FFmpeg process...", job_id);
        //     let _ = child.kill();
        //     if output_wav_path.exists() { let _ = fs::remove_file(&output_wav_path); }
        //     return Err(CommandError::from("Audio conversion cancelled."));
        // }

        tokio::select! {
            biased; // Ensure cancellation check (if added later) is prioritized
            maybe_event = rx.recv() => {
                match maybe_event {
                    Some(event) => match event {
                        CommandEvent::Stdout(line) => { debug!("[FFmpeg CMD][stdout][{}] {}", job_id, String::from_utf8_lossy(&line).trim_end()); },
                        CommandEvent::Stderr(line) => { let l = String::from_utf8_lossy(&line).to_string(); debug!("[FFmpeg CMD][stderr][{}] {}", job_id, l.trim_end()); ffmpeg_stderr.push(l); },
                        CommandEvent::Error(msg) => { error!("[FFmpeg CMD][error][{}] {}", job_id, msg); ffmpeg_error = Some(msg); break; },
                        CommandEvent::Terminated(payload) => { info!("[FFmpeg CMD][term][{}] Process terminated. Code: {:?}, Signal: {:?}", job_id, payload.code, payload.signal); ffmpeg_exit_code = payload.code; if payload.signal.is_some() && ffmpeg_exit_code.is_none() { ffmpeg_exit_code = Some(-1); } break; }
                        _ => {}
                    },
                    None => {
                        if ffmpeg_exit_code.is_none() && ffmpeg_error.is_none() {
                            warn!("[FFmpeg CMD][{}] Event channel closed unexpectedly before termination signal.", job_id);
                            ffmpeg_exit_code = Some(-1); // Treat as an error
                        }
                        break;
                    }
                }
            }
            // Minimal sleep if cancellation is omitted, otherwise select! might behave unexpectedly without multiple branches.
            // If cancellation was present:
            // _ = sleep(Duration::from_millis(50)) => { continue; }
            // Since it's omitted, we might not need this sleep, but keeping it for safety during select! usage.
            // If recv() is the only branch, select! is not really needed.
            // For now, let's assume recv() might not always be ready immediately.
             _ = sleep(Duration::from_millis(10)) => {
                 // This branch is mainly to ensure select! doesn't block indefinitely if recv() is slow
                 // and to allow future re-integration of cancellation checks.
                 // If there are no other branches, this sleep isn't strictly necessary
                 // but also doesn't harm significantly for a short duration.
                 // If this were the only branch, a simple loop with rx.recv().await would suffice.
             }
        }
    }

    let stderr_output = ffmpeg_stderr.join("\n");
    if ffmpeg_error.is_some() || ffmpeg_exit_code != Some(0) {
        error!("[FFmpeg CMD][{}] FFmpeg process failed. Code: {:?}, Error: {:?}\nStderr:\n{}", job_id, ffmpeg_exit_code, ffmpeg_error, stderr_output);
        if output_wav_path.exists() { let _ = fs::remove_file(&output_wav_path); }
        return Err(CommandError::from(format!("FFmpeg conversion failed. Code: {:?}. Error: {}", ffmpeg_exit_code, ffmpeg_error.unwrap_or_default())));
    }

    if !output_wav_path.exists() {
        error!("[FFmpeg CMD][{}] FFmpeg reported success, but output file is missing: {}", job_id, output_wav_path.display());
        return Err(CommandError::from(format!("FFmpeg conversion failed: output file missing ({})", output_wav_path.display())));
    }
    match output_wav_path.metadata() {
        Ok(m) if m.len() == 0 => {
            error!("[FFmpeg CMD][{}] FFmpeg reported success, but output file is empty: {}", job_id, output_wav_path.display());
            let _ = fs::remove_file(&output_wav_path);
            return Err(CommandError::from(format!("FFmpeg conversion failed: output file is empty ({})", output_wav_path.display())));
        },
        Err(e) => {
            error!("[FFmpeg CMD][{}] FFmpeg reported success, but failed to get metadata for {}: {}", job_id, output_wav_path.display(), e);
            let _ = fs::remove_file(&output_wav_path);
            return Err(CommandError::from(format!("FFmpeg conversion failed: output metadata error ({})", e)));
        },
        Ok(_) => {}
    }

    info!("[FFmpeg CMD][{}] Successfully converted '{}' to WAV: {}", job_id, input_path_str, output_wav_path.display());
    Ok(output_wav_path)
}

// Adapted from local_handler/transcription.rs
// Renamed to avoid conflict and made pub(crate)
pub(crate) fn find_model_file_cmd(model_dir: &Path) -> Result<PathBuf, CommandError> {
    debug!("[Helper CMD] Searching for model file in directory: {:?}", model_dir);
    if !model_dir.exists() || !model_dir.is_dir() {
        return Err(CommandError::from(format!("Model directory not found or is not a directory: {}", model_dir.display())));
    }

    for entry_result in fs::read_dir(model_dir)? {
        match entry_result {
            Ok(entry) => {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                        let lower_ext = ext.to_lowercase();
                        // Common model file extensions
                        if lower_ext == "bin" || lower_ext == "gguf" || lower_ext == "pt" {
                            info!("[Helper CMD] Found potential model file: {:?}", path);
                            return Ok(path);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("[Helper CMD] Failed to read directory entry in '{}': {}", model_dir.display(), e);
            }
        }
    }

    Err(CommandError::from(format!("No model file (.bin, .gguf, .pt) found within directory: {}", model_dir.display())))
}


// Adapted from local_handler/transcription.rs
// Made synchronous as read_config and find_model_file_cmd are sync.

// --- Structs specific to parsing whisper output (can be kept local to where they are used) ---
#[derive(serde::Deserialize, Debug)]
struct WhisperJsonOutput {
    transcription: Option<Vec<WhisperJsonSegment>>,
}
#[derive(serde::Deserialize, Debug)]
struct WhisperJsonSegment {
    timestamps: WhisperJsonTimestamps,
    text: String,
}
#[derive(serde::Deserialize, Debug)]
struct WhisperJsonTimestamps {
    from: String,
    to: String,
}

// --- Struct specific to parsing RTTM output (can be kept local) ---
#[derive(Debug, Clone)]
struct RttmRecord {
    start_time: f64,
    duration: f64,
    speaker_id: String,
}


pub(crate) fn resolve_whisper_model_path_cmd(
    model_name: &str,
    job_id: &str, // Kept for logging consistency, though not strictly needed by logic
) -> Result<String, CommandError> {
    let config = read_config()?; // This is synchronous
    let base_model_dir_str = if !config.download_location.trim().is_empty() {
        config.download_location
    } else {
        get_default_download_location()? // This is synchronous
    };
    let model_dir_path = PathBuf::from(&base_model_dir_str).join(model_name);

    if !model_dir_path.exists() || !model_dir_path.is_dir() {
        let e_msg = format!("Model directory not found: '{}'. Please download the model first.", model_dir_path.display());
        error!("[Transcription CMD][{}] Error resolving model path: {}", job_id, e_msg);
        return Err(CommandError::from(e_msg));
    }
    // Call the adapted find_model_file_cmd
    let model_file_path = find_model_file_cmd(&model_dir_path)?;
    Ok(model_file_path.to_string_lossy().to_string())
}

// --- START: Adapted Helper Functions for execute_transcription_pass ---

// Adapted from local_handler/transcription.rs
async fn run_whisper_cpp_sidecar_cmd(
    app_handle: &AppHandle,
    media_path: &str, // Should be wav_media_path.to_string_lossy().to_string()
    whisper_model_path_str: &str,
    language: &str,
    job_id: &str,
    output_base_for_whisper: &str,
    expected_whisper_json_output_path: &Path,
    is_translation_pass: bool,
    // cancel_flag: &Arc<AtomicBool>, // Omitted for now
) -> Result<PathBuf, CommandError> {
    let sidecar_name = "whisper-cpp";
    let lang_arg = if language.trim().is_empty() || language == "auto" { "auto" } else { language.trim() };
    debug!("[Whisper CPP CMD][{}] Using language: '{}', Translate: {}", job_id, lang_arg, is_translation_pass);

    let mut args: Vec<String> = vec![
        "-m".into(), whisper_model_path_str.to_string(),
        "-f".into(), media_path.to_string(),
        "-l".into(), lang_arg.to_string(),
        "-oj".into(), // Output JSON
        "-of".into(), output_base_for_whisper.to_string(),
    ];

    if is_translation_pass {
        args.push("--translate".into());
    }

    debug!("[Whisper CPP CMD][{}] Running sidecar '{}' with args: {:?}", job_id, sidecar_name, args);

    let shell_scope = app_handle.shell();
    let (mut rx, child) = shell_scope.sidecar(sidecar_name)?.args(args).spawn()
     .map_err(|e| {
         error!("Failed to spawn whisper-cpp: {}. Check tauri.conf.json, binary paths, and permissions.", e);
         CommandError::from(format!("Failed to execute whisper-cpp sidecar: {}. Ensure it's bundled and executable.", e))
     })?;
    info!("[Whisper CPP CMD][{}] Spawned sidecar '{}' (PID: {:?})", job_id, sidecar_name, child.pid());

    // Simplified event handling (omitting cancellation for now)
    let mut process_error: Option<String> = None;
    let mut exit_code: Option<i32> = None;
    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(line) => { debug!("[{}][stdout][{}] {}", sidecar_name, job_id, String::from_utf8_lossy(&line).trim_end()); },
            CommandEvent::Stderr(line) => { debug!("[{}][stderr][{}] {}", sidecar_name, job_id, String::from_utf8_lossy(&line).trim_end()); },
            CommandEvent::Error(msg) => { process_error = Some(msg); break; },
            CommandEvent::Terminated(payload) => { exit_code = payload.code; break; },
            _ => {}
        }
    }

    if process_error.is_some() || exit_code != Some(0) {
        return Err(CommandError::from(format!("Sidecar '{}' failed. Exit: {:?}, Err: {:?}", sidecar_name, exit_code, process_error)));
    }
    if !expected_whisper_json_output_path.exists() {
        // Add a small delay and check again, as file system operations might not be instantaneous
        sleep(Duration::from_millis(300)).await;
        if !expected_whisper_json_output_path.exists() {
            return Err(CommandError::from(format!("Whisper output JSON missing: {:?}", expected_whisper_json_output_path)));
        }
    }
    Ok(expected_whisper_json_output_path.to_path_buf())
}

// Adapted from local_handler/transcription.rs
fn parse_whisper_json_cmd(json_path: &Path) -> Result<Vec<TranscriptSegment>, CommandError> {
    debug!("[JSON Parse CMD] Reading whisper output: {:?}", json_path);
    let file = File::open(json_path)?;
    let reader = std::io::BufReader::new(file); // Ensure BufReader is in scope
    let output: WhisperJsonOutput = serde_json::from_reader(reader)
        .map_err(|e| CommandError::from(format!("Failed to parse whisper JSON from '{}': {}", json_path.display(), e)))?;

    let mut segments = Vec::new();
    if let Some(transcription) = output.transcription {
        for (idx, w_seg) in transcription.iter().enumerate() {
            // Simplified timestamp parsing for this adaptation, assuming format is always "00:00:00,000" or "00:00:00.000"
            // A more robust parser like in local_handler might be needed if whisper's output varies.
            let parse_ts = |ts_str: &str| -> Result<f64, String> {
                let parts: Vec<&str> = ts_str.split(|c| c == ':' || c == ',' || c == '.').collect();
                if parts.len() == 4 { // hh:mm:ss:ms
                    let h: f64 = parts[0].parse().map_err(|_| "h".to_string())?;
                    let m: f64 = parts[1].parse().map_err(|_| "m".to_string())?;
                    let s: f64 = parts[2].parse().map_err(|_| "s".to_string())?;
                    let ms: f64 = parts[3].parse().map_err(|_| "ms".to_string())?;
                    Ok(h * 3600.0 + m * 60.0 + s + ms / 1000.0)
                } else if parts.len() == 3 && ts_str.contains(':') { // mm:ss.ms
                     let m: f64 = parts[0].parse().map_err(|_| "m2".to_string())?;
                     let s: f64 = parts[1].parse().map_err(|_| "s2".to_string())?;
                     let ms: f64 = parts[2].parse().map_err(|_| "ms2".to_string())?;
                     Ok(m * 60.0 + s + ms / 1000.0)
                }
                 else { Err(format!("Invalid timestamp format: {}", ts_str)) }
            };

            let start_time = parse_ts(&w_seg.timestamps.from)
                .map_err(|e_msg| CommandError::from(format!("Segment {}: Invalid start time '{}': {}", idx, w_seg.timestamps.from, e_msg)))?;
            let end_time = parse_ts(&w_seg.timestamps.to)
                 .map_err(|e_msg| CommandError::from(format!("Segment {}: Invalid end time '{}': {}", idx, w_seg.timestamps.to, e_msg)))?;

            if end_time < start_time {
                 warn!("[JSON Parse CMD] Skipping segment {} due to end time < start time.", idx);
                 continue;
            }
            segments.push(TranscriptSegment {
                start_time,
                end_time,
                speaker: "Unknown".to_string(), // Default speaker
                text: w_seg.text.trim().to_string(),
            });
        }
    }
    info!("[JSON Parse CMD] Parsed {} segments from {}", segments.len(), json_path.display());
    Ok(segments)
}

// Adapted from local_handler/transcription.rs
async fn run_diarize_cli_sidecar_cmd(
    app_handle: &AppHandle,
    media_path: &str, // wav_media_path.to_string_lossy().to_string()
    num_speakers: usize,
    output_rttm_path: &Path,
    job_id: &str,
    // cancel_flag: &Arc<AtomicBool>, // Omitted
) -> Result<PathBuf, CommandError> {
    let sidecar_name = "diarize-cli";
    info!("[DiarizeCLI CMD][{}] Starting for: {}, num_speakers: {}", job_id, media_path, num_speakers);
    if let Some(parent_dir) = output_rttm_path.parent() { fs::create_dir_all(parent_dir)?; }

    let mut args = vec![
        "--audio".into(), media_path.to_string(),
        "--output".into(), output_rttm_path.to_string_lossy().to_string(),
    ];
    if num_speakers > 0 { // diarize-cli might handle num_speakers=0 as auto, but explicit is safer
        args.push("--num_speakers".into()); args.push(num_speakers.to_string());
        // Add min/max if your diarize-cli version supports/requires them
        args.push("--min_speakers".into()); args.push(1.to_string()); // Example
        args.push("--max_speakers".into()); args.push(num_speakers.max(1).to_string()); // Example
    }

    let shell_scope = app_handle.shell();
    let (mut rx, child) = shell_scope.sidecar(sidecar_name)?.args(args).spawn()
        .map_err(|e| CommandError::from(format!("Failed to execute {} sidecar: {}", sidecar_name, e)))?;
    info!("[DiarizeCLI CMD][{}] Spawned '{}' (PID: {:?})", job_id, sidecar_name, child.pid());

    let mut process_error: Option<String> = None;
    let mut exit_code: Option<i32> = None;
    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(line) => { debug!("[{}][stdout][{}] {}", sidecar_name, job_id, String::from_utf8_lossy(&line).trim_end()); },
            CommandEvent::Stderr(line) => { debug!("[{}][stderr][{}] {}", sidecar_name, job_id, String::from_utf8_lossy(&line).trim_end()); },
            CommandEvent::Error(msg) => { process_error = Some(msg); break; },
            CommandEvent::Terminated(payload) => { exit_code = payload.code; break; },
            _ => {}
        }
    }

    if process_error.is_some() || exit_code != Some(0) {
        return Err(CommandError::from(format!("Sidecar '{}' failed. Exit: {:?}, Err: {:?}", sidecar_name, exit_code, process_error)));
    }
    if !output_rttm_path.exists() {
        sleep(Duration::from_millis(300)).await;
        if !output_rttm_path.exists() {
             return Err(CommandError::from(format!("Diarization RTTM output missing: {:?}", output_rttm_path)));
        }
    }
    Ok(output_rttm_path.to_path_buf())
}

// Adapted from local_handler/transcription.rs
fn parse_rttm_file_cmd(rttm_path: &Path) -> Result<Vec<RttmRecord>, CommandError> {
    debug!("[RTTM Parse CMD] Reading RTTM file: {:?}", rttm_path);
    let file = File::open(rttm_path)?;
    let reader = std::io::BufReader::new(file);
    let mut records = Vec::new();

    for line_result in reader.lines() {
        let line = line_result?;
        if line.trim().is_empty() || line.starts_with(';') { continue; }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 8 || parts[0] != "SPEAKER" { continue; }
        let start_time: f64 = parts[3].parse().map_err(|_| CommandError::from("Invalid RTTM start time"))?;
        let duration: f64 = parts[4].parse().map_err(|_| CommandError::from("Invalid RTTM duration"))?;
        if start_time < 0.0 || duration <= 0.0 { continue; } // Basic validation
        records.push(RttmRecord { start_time, duration, speaker_id: parts[7].to_string() });
    }
    info!("[RTTM Parse CMD] Parsed {} records from {}", records.len(), rttm_path.display());
    Ok(records)
}

// Adapted from local_handler/transcription.rs
fn merge_diarization_results_cmd(whisper_segments: &mut Vec<TranscriptSegment>, rttm_records: &[RttmRecord]) {
    if rttm_records.is_empty() { info!("[Merge CMD] No RTTM records for merging."); return; }
    if whisper_segments.is_empty() { info!("[Merge CMD] No whisper segments for merging."); return; }

    let mut sorted_rttm = rttm_records.to_vec();
    sorted_rttm.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap_or(std::cmp::Ordering::Equal));

    for seg in whisper_segments.iter_mut() {
        let seg_mid_point = seg.start_time + (seg.end_time - seg.start_time) / 2.0;
        let mut best_speaker = "Unknown".to_string();
        let mut max_overlap = 0.0f64;

        for rttm_rec in &sorted_rttm {
            let rttm_start = rttm_rec.start_time;
            let rttm_end = rttm_rec.start_time + rttm_rec.duration;

            let overlap_start = seg.start_time.max(rttm_start);
            let overlap_end = seg.end_time.min(rttm_end);
            let current_overlap = (overlap_end - overlap_start).max(0.0);

            if current_overlap > max_overlap {
                max_overlap = current_overlap;
                best_speaker = rttm_rec.speaker_id.clone();
            } else if current_overlap > 0.0 && current_overlap == max_overlap {
                // Tie-breaking: prefer speaker whose turn contains the segment's midpoint
                if seg_mid_point >= rttm_start && seg_mid_point < rttm_end {
                    best_speaker = rttm_rec.speaker_id.clone();
                }
            }
        }
        if max_overlap > 0.0 { // Only assign if there was any overlap
            seg.speaker = best_speaker;
        }
    }
    info!("[Merge CMD] Finished merging diarization results.");
}

// --- END: Adapted Helper Functions ---

pub(crate) async fn execute_transcription_pass(
    app_handle: &AppHandle,
    wav_media_path_str: &str, // Changed to &str to match caller
    model_path: &str,
    language_code: &str,
    job_id: &str,
    output_base_for_whisper: &str,
    expected_whisper_json_output_path: &PathBuf,
    num_speakers: usize,
    expected_rttm_output_path: &PathBuf,
    is_translation_pass: bool,
    speaker_names: &[String],
) -> Result<Vec<TranscriptSegment>, CommandError> {
    info!(
        "[Exec Pass][{}] Start. Lang: {}, Translate: {}, NumSpeakers: {}",
        job_id, language_code, is_translation_pass, num_speakers
    );

    let whisper_json_path = run_whisper_cpp_sidecar_cmd(
        app_handle,
        wav_media_path_str,
        model_path,
        language_code,
        job_id,
        output_base_for_whisper,
        expected_whisper_json_output_path,
        is_translation_pass,
    ).await?;

    let mut segments = parse_whisper_json_cmd(&whisper_json_path)?;

    if num_speakers > 0 && !is_translation_pass {
        emit_progress_cmd(app_handle, job_id,segments.len() as f32 * 0.1 + 20.0, "Running diarization...")?; // Example progress update

        let rttm_path = run_diarize_cli_sidecar_cmd(
            app_handle,
            wav_media_path_str,
            num_speakers,
            expected_rttm_output_path,
            job_id,
        ).await?;

        match parse_rttm_file_cmd(&rttm_path) {
            Ok(rttm_records) => {
                if !rttm_records.is_empty() {
                    merge_diarization_results_cmd(&mut segments, &rttm_records);
                } else {
                    warn!("[Exec Pass][{}] Diarization produced no RTTM records.", job_id);
                }
            }
            Err(e) => {
                warn!("[Exec Pass][{}] Failed to parse RTTM file: {}. Proceeding without merged diarization.", job_id, e);
            }
        }
    } else {
        info!("[Exec Pass][{}] Skipping diarization.", job_id);
    }

    map_speaker_ids_to_names(&mut segments, speaker_names);

    info!("[Exec Pass][{}] Pass complete. Segments: {}", job_id, segments.len());
    Ok(segments)
}


// Helper to emit progress
pub(crate) fn emit_progress_cmd(
    app_handle: &AppHandle,
    job_id: &str,
    percent: f32,
    message: &str,
) -> Result<(), CommandError> {
    let clamped_percent = percent.max(0.0).min(100.0);
    debug!("[Progress Emit CMD][{}] {:.1}% - {}", job_id, clamped_percent, message);
    app_handle.emit("PROGRESS", ProgressPayload {
        job_id: job_id.to_string(),
        percent: clamped_percent,
        message: message.to_string(),
    }).map_err(|e| CommandError::from(format!("Failed to emit progress: {}", e)))
}


#[tauri::command]
pub async fn list_subtitle_files_command(media_path_str: String) -> Result<Vec<SubtitleFileEntry>, CommandError> {
    info!("[list_subtitle_files_command] Listing subtitles for: {}", media_path_str);
    let media_path = PathBuf::from(media_path_str);

    // Expecting subtitles in a 'transcripts' subdirectory relative to the media file's stem directory
    // e.g., if media is .../Media/MyVideo/media/MyVideo.mp4
    // then subtitles are in .../Media/MyVideo/transcripts/
    let media_file_parent_dir = media_path.parent().ok_or_else(|| CommandError::from("Could not get media file parent dir"))?;
    if media_file_parent_dir.file_name().and_then(|n| n.to_str()) != Some(MEDIA_SUBDIR) {
        return Err(CommandError::from(format!("Media file not in expected '{}' subdirectory.", MEDIA_SUBDIR)));
    }
    let media_stem_dir = media_file_parent_dir.parent().ok_or_else(|| CommandError::from("Could not get media stem directory"))?;
    let transcripts_dir = media_stem_dir.join(TRANSCRIPTS_SUBDIR);

    let mut subtitle_files = Vec::new();
    if transcripts_dir.exists() && transcripts_dir.is_dir() {
        for entry in fs::read_dir(transcripts_dir).map_err(|e| CommandError::from(format!("Failed to read transcripts dir: {}", e)))? {
            let entry = entry.map_err(|e| CommandError::from(format!("Error reading directory entry: {}", e)))?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext == "srt" || ext == "vtt" {
                        subtitle_files.push(SubtitleFileEntry {
                            name: path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
                            path: path.to_string_lossy().into_owned(),
                        });
                    }
                }
            }
        }
    }
    subtitle_files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(subtitle_files)
}

#[tauri::command]
pub async fn convert_srt_to_vtt_command(srt_path_str: String) -> Result<String, CommandError> {
    info!("[convert_srt_to_vtt_command] Converting SRT: {}", srt_path_str);
    // Placeholder: Implement actual SRT to VTT conversion logic here.
    // For now, let's assume it just returns the path or an error if not an SRT.
    let srt_path = PathBuf::from(&srt_path_str);
    if srt_path.extension().and_then(|e| e.to_str()) != Some("srt") {
        return Err(CommandError::from("Not an SRT file.".to_string()));
    }
    // This should return the content of the VTT file or path to a new VTT file.
    // For this stub, we'll just return a message.
    Ok(format!("Successfully processed (stubbed) SRT file: {}", srt_path_str))
}