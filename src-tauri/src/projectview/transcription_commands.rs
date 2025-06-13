// src-tauri/src/projectview/transcription_commands.rs

use super::shared_types::*;
use super::shared_utils::*;
use crate::welcome::config::CommandError;
use log::{debug, error, info, warn};
use serde_json::json;
use serde_json::Value as JsonValue;

use std::{
    fs::{self, File},
    io::{BufWriter, Write}, 
    path::{Path, PathBuf},
};
use tauri::{AppHandle};
use tauri_plugin_shell::ShellExt; 
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
pub(crate) fn prepare_output_paths( media_path_str: &str, job_id: &str) -> Result<(String, PathBuf, PathBuf, PathBuf), CommandError> {
    debug!("[prepare_output_paths][{}] Media path: {}", job_id, media_path_str);
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

    let temp_output_base_in_transcripts = transcripts_dir.join(format!("{}_temp_{}", media_filename_stem, job_id));
    let temp_output_path_base_str = temp_output_base_in_transcripts.to_string_lossy().to_string();

    // EXPECTED path for Whisper's JSON output, assuming "-of <base>" and "-oj" results in "<base>.json"
    let expected_whisper_temp_output_path = temp_output_base_in_transcripts.with_extension("json"); // CORRECTED
    
    let expected_rttm_temp_path = temp_output_base_in_transcripts.with_extension("rttm");
    
    let final_transcript_path = transcripts_dir.join(format!("{}.json", media_filename_stem));

    debug!("[prepare_output_paths][{}] Temp Base: '{}', Whisper JSON (temp): '{}', RTTM (temp): '{}', Final Transcript JSON: '{}'", 
        job_id, temp_output_path_base_str, expected_whisper_temp_output_path.display(), expected_rttm_temp_path.display(), final_transcript_path.display());

    Ok((temp_output_path_base_str, expected_whisper_temp_output_path, expected_rttm_temp_path, final_transcript_path))
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