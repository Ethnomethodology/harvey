// src-tauri/src/projectview/transcription_commands.rs

use super::shared_types::*;
use super::shared_utils::*;
use crate::welcome::config::{CommandError, read_config, get_default_download_location};
use log::{debug, error, info, warn};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, Runtime}; // Removed ShellExt from here
use tauri_plugin_shell::ShellExt; // Added specific import for ShellExt
use serde_json::Value as JsonValue;
use crate::projectview::utils::{get_ffmpeg_path, get_ffprobe_path};
use serde::Deserialize; // Added for FFProbeOutput
use chrono::Utc; // Added for timestamps
// use uuid::Uuid; // Removed unused import
use crate::projectview::db_handler; // Added for database operations
use crate::projectview::waveform_utils;
use tokio::sync::Mutex;
use tauri_plugin_shell::process::CommandChild;

use std::{
    fs::{self, File},
    io::{BufWriter, Write, BufRead},
    path::{Path, PathBuf},
    sync::{Arc, atomic::{AtomicBool, Ordering as AtomicOrdering}},
};
use dashmap::DashMap;
use tauri_plugin_shell::{process::CommandEvent};
use tokio::time::{sleep, Duration};
use quick_xml;
use crate::welcome::python_env::get_python_path;

// Helper to read the HuggingFace token
fn get_hf_token<R: Runtime>(app_handle: &AppHandle<R>) -> Result<String, String> {
    let config_dir = app_handle
        .path()
        .app_config_dir()
        .expect("Failed to get app config dir");
    let token_path = config_dir.join("hf_token");
    fs::read_to_string(token_path).map_err(|e| {
        format!(
            "Failed to read token: {}. Please ensure you have saved your HuggingFace token.",
            e
        )
    })
}


// --- State for Live Transcription ---
pub struct LiveTranscriptionState {
    pub whisper_child: Mutex<Option<CommandChild>>,
    pub is_running: Arc<AtomicBool>,
    pub start_time: Mutex<Option<chrono::DateTime<chrono::Utc>>>,
    pub active_document_path: Mutex<Option<String>>,
    pub project_uuid: Mutex<Option<String>>,
    pub project_base_dir: Mutex<Option<PathBuf>>,
}

impl Default for LiveTranscriptionState {
    fn default() -> Self {
        Self {
            whisper_child: Mutex::new(None),
            is_running: Arc::new(AtomicBool::new(false)),
            start_time: Mutex::new(None),
            active_document_path: Mutex::new(None),
            project_uuid: Mutex::new(None),
            project_base_dir: Mutex::new(None),
        }
    }
}

#[derive(serde::Serialize, Clone)]
pub struct LiveTranscriptionResult {
    pub text: String,
    pub is_final: bool,
    pub start_time: f64,
    pub end_time: f64,
}

// --- FFProbe Helper Structs (copied from core_commands.rs) ---
#[derive(Deserialize, Debug, Default, Clone)]
struct FFProbeStreamTags {
    #[serde(rename = "DURATION")]
    duration: Option<String>,
}

#[derive(Deserialize, Debug, Default, Clone)]
struct FFProbeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
    avg_frame_rate: Option<String>,
    r_frame_rate: Option<String>,
    bit_rate: Option<String>,
    #[serde(default)]
    tags: FFProbeStreamTags,
}

#[derive(Deserialize, Debug, Default, Clone)]
struct FFProbeFormatTags {
    
    #[serde(rename = "DURATION")]
    duration: Option<String>,
}

#[derive(Deserialize, Debug, Default, Clone)]
struct FFProbeFormat {
    duration: Option<String>,
    bit_rate: Option<String>,
    #[serde(default)]
    tags: Option<FFProbeFormatTags>,
    
}

#[derive(Deserialize, Debug, Default, Clone)]
struct FFProbeOutput {
    #[serde(default)]
    streams: Vec<FFProbeStream>,
    #[serde(default)]
    format: FFProbeFormat,
}

// --- Helper Functions for FFProbe Data Parsing (copied from core_commands.rs) ---
fn parse_duration_str_to_seconds(s_opt: Option<String>) -> Option<f64> {
    s_opt.as_deref().and_then(|s| {
        if s.contains(':') {
            let parts: Vec<&str> = s.split(':').collect();
            if parts.len() == 3 { // HH:MM:SS.mmm
                let hours = parts[0].parse::<f64>().ok()?;
                let minutes = parts[1].parse::<f64>().ok()?;
                let seconds_ms = parts[2].parse::<f64>().ok()?;
                Some(hours * 3600.0 + minutes * 60.0 + seconds_ms)
            } else { None }
        } else { // Seconds only
            s.parse::<f64>().ok()
        }
    })
}

fn parse_frame_rate_str(s_opt: Option<String>) -> Option<f32> {
    s_opt.as_deref().and_then(|s| {
        if s.contains('/') {
            let parts: Vec<&str> = s.split('/').collect();
            if parts.len() == 2 {
                let num = parts[0].parse::<f32>().ok()?;
                let den = parts[1].parse::<f32>().ok()?;
                if den.abs() > f32::EPSILON { Some(num / den) } else { None }
            } else { None }
        } else {
            s.parse::<f32>().ok()
        }
    })
}


// --- CancelGuard for managing transcription cancellation ---
struct CancelGuard {
    job_id: String,
    state: Arc<DashMap<String, Arc<AtomicBool>>>,
}

impl Drop for CancelGuard {
    fn drop(&mut self) {
        if self.state.remove(&self.job_id).is_some() {
            debug!("[CancelGuard] Removed cancel flag for job '{}' on drop.", self.job_id);
        } else {
            warn!("[CancelGuard] Attempted to remove flag for job '{}' on drop, but it was already gone.", self.job_id);
        }
    }
}


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
pub async fn trim_media<R: Runtime>( app_handle: AppHandle<R>, original_media_path: String, start_time: f64, end_time: f64) -> Result<Vec<FileEntry>, CommandError> {
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

    let ffmpeg_path = get_ffmpeg_path(&app_handle)?;

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

    info!("[Trim Backend] FFmpeg Cmd: {:?} {}", ffmpeg_path, args.join(" "));
    let shell_scope = app_handle.shell();
    let mut command = shell_scope.command(ffmpeg_path).args(args);

    if cfg!(target_os = "macos") {
        if let Ok(resource_dir) = app_handle.path().resource_dir() {
            let sidecars_path = resource_dir.join("sidecars");
            if sidecars_path.exists() {
                let sidecars_path_str = sidecars_path.to_string_lossy();
                if let Ok(existing_path) = std::env::var("DYLD_LIBRARY_PATH") {
                    command = command.env("DYLD_LIBRARY_PATH", format!("{}:{}", sidecars_path_str, existing_path));
                } else {
                    command = command.env("DYLD_LIBRARY_PATH", sidecars_path_str.to_string());
                }
            }
        }
    }

    let (mut rx, _child) = command.spawn()?;

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
        relative_path: new_relative_path_for_xml.clone(), // Clone here
        speakers: original_speakers.clone().or_else(|| Some(SpeakersXml::default())), // Clone here
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
    let reloaded_project_data_result = super::core_commands::load_project_data(project_xml_path_str.clone()).await;

    let project_uuid_for_db = match &reloaded_project_data_result {
        Ok(data) => data.project_uuid.clone(),
        Err(e) => {
            error!("[Trim Backend] Failed to reload project data to get UUID after trim: {}. Cannot save metadata to DB.", e);
            // Depending on strictness, you might return Err here or just log and skip DB operations.
            // For now, returning the file list as before if reload failed, but DB ops will be skipped.
            return reloaded_project_data_result.map(|data| data.files);
        }
    };

    if project_uuid_for_db.is_empty() {
        error!("[Trim Backend] Project UUID is empty after reloading project data. Cannot save metadata to DB.");
    } else {
        info!("[Trim Backend] Project UUID for DB operations: {}", project_uuid_for_db);
        // --- Start: Add metadata to SQLite database for the new trimmed media ---
        let mut duration_seconds_meta: Option<f64> = None;
        let mut width_meta: Option<i32> = None;
        let mut height_meta: Option<i32> = None;
        let mut frame_rate_meta: Option<f32> = None;
        let mut bit_rate_overall_meta: Option<i64> = None;
        let mut audio_codec_meta: Option<String> = None;
        let mut video_codec_meta: Option<String> = None;

        let ffprobe_args = vec![
            "-v".to_string(), "quiet".to_string(),
            "-print_format".to_string(), "json".to_string(),
            "-show_format".to_string(),
            "-show_streams".to_string(),
            output_media_path.to_string_lossy().to_string(),
        ];

        info!("[Trim Backend] Running ffprobe for new trimmed media: {}", output_media_path.display());
    let ffprobe_path = get_ffprobe_path(&app_handle)?;
    match app_handle.shell().command(ffprobe_path).args(ffprobe_args).output().await {
            Ok(output) => {
                if output.status.success() {
                    let ffprobe_json_str = String::from_utf8_lossy(&output.stdout).to_string();
                    debug!("[Trim Backend] ffprobe output for {}: {}", output_media_path.display(), ffprobe_json_str);
                    match serde_json::from_str::<FFProbeOutput>(&ffprobe_json_str) {
                        Ok(parsed_ffprobe_output) => {
                            duration_seconds_meta = parse_duration_str_to_seconds(parsed_ffprobe_output.format.duration.clone())
                                .or_else(|| parse_duration_str_to_seconds(parsed_ffprobe_output.format.tags.as_ref().and_then(|t| t.duration.clone())));
                            bit_rate_overall_meta = parsed_ffprobe_output.format.bit_rate.as_deref().and_then(|s| s.parse().ok());

                            for stream in parsed_ffprobe_output.streams {
                                if duration_seconds_meta.is_none() {
                                     duration_seconds_meta = parse_duration_str_to_seconds(stream.tags.duration.clone());
                                }
                                match stream.codec_type.as_deref() {
                                    Some("video") if width_meta.is_none() => {
                                        width_meta = stream.width;
                                        height_meta = stream.height;
                                        video_codec_meta = stream.codec_name;
                                        frame_rate_meta = parse_frame_rate_str(stream.avg_frame_rate.clone())
                                            .or_else(|| parse_frame_rate_str(stream.r_frame_rate.clone()));
                                        if bit_rate_overall_meta.is_none() {
                                            bit_rate_overall_meta = stream.bit_rate.as_deref().and_then(|s| s.parse().ok());
                                        }
                                    }
                                    Some("audio") if audio_codec_meta.is_none() => {
                                        audio_codec_meta = stream.codec_name;
                                        if bit_rate_overall_meta.is_none() && stream.bit_rate.is_some() {
                                             bit_rate_overall_meta = stream.bit_rate.as_deref().and_then(|s| s.parse().ok());
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            info!("[Trim Backend] Successfully parsed ffprobe output for {}", output_media_path.display());
                        }
                        Err(e) => {
                            error!("[Trim Backend] Failed to parse ffprobe JSON for {}: {}. JSON: '{}'", output_media_path.display(), e, ffprobe_json_str);
                        }
                    }
                } else {
                    let stderr_str = String::from_utf8_lossy(&output.stderr);
                    error!("[Trim Backend] ffprobe failed for {}. Code: {:?}, Stderr: {}", output_media_path.display(), output.status.code(), stderr_str);
                }
            }
            Err(e) => {
                error!("[Trim Backend] ffprobe execution error for {}: {}", output_media_path.display(), e);
            }
        }

        let trimmed_media_file_metadata = FileMetadata {
            file_name: output_filename.clone(), // Filename of the trimmed media
            file_path: output_media_path.to_string_lossy().into_owned(), // Absolute path
            last_modified: Utc::now().to_rfc3339(),
            title: String::new(), // Initialize as empty
            description: String::new(), // Initialize as empty
            summary: String::new(), // Initialize as empty
            duration_seconds: duration_seconds_meta,
            width: width_meta,
            height: height_meta,
            frame_rate: frame_rate_meta,
            bit_rate: bit_rate_overall_meta,
            audio_codec: audio_codec_meta.clone(),
            video_codec: video_codec_meta.clone(),
            created_at: Some(Utc::now().to_rfc3339()),
            original_import_path: Some(original_media_path.clone()), // Store original path as import path
            speaker_names: None, // Speaker names come from XML, not directly stored here.
            waveform_data: None,
        };

        let asset_type = if video_codec_meta.is_some() { "video" } else if audio_codec_meta.is_some() { "audio" } else { "media" }.to_string();

        // The relative path used as key for DB is the same as the one stored in XML for the new media entry
        let db_key_relative_path_trimmed = new_relative_path_for_xml.clone();

        let waveform_peaks = match fs::read(&output_media_path) {
            Ok(audio_data) => {
                match waveform_utils::generate_audio_peaks(&audio_data, 512) {
                    Ok(peaks) => {
                        let mut u8_peaks = Vec::with_capacity(peaks.len() * 4);
                        for peak in peaks {
                            u8_peaks.extend_from_slice(&peak.to_le_bytes());
                        }
                        Some(u8_peaks)
                    }
                    Err(e) => {
                        warn!("[Trim Backend] Failed to generate waveform peaks: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                warn!("[Trim Backend] Failed to read media file for waveform generation: {}", e);
                None
            }
        };

        let mut trimmed_media_file_metadata_with_waveform = trimmed_media_file_metadata;
        trimmed_media_file_metadata_with_waveform.waveform_data = waveform_peaks;

        match db_handler::save_asset_metadata(
            &project_uuid_for_db,
            &trimmed_media_file_metadata_with_waveform,
            &db_key_relative_path_trimmed,
            &asset_type,
            None, // custom_fields_json
        ) {
            Ok(_) => info!("[Trim Backend] Successfully saved asset metadata to DB for trimmed media: {}", db_key_relative_path_trimmed),
            Err(e) => warn!("[Trim Backend] Failed to save asset metadata to DB for trimmed media {}: {}.", db_key_relative_path_trimmed, e),
        }

        // Also save initial media_transcript_data
        if let Err(e) = db_handler::save_media_transcript_data(
            &project_uuid_for_db,
            &db_key_relative_path_trimmed,
            Some(&original_media_path), // Original media path as source reference
            original_speakers.as_ref().map(|s_xml| &s_xml.names),
            None, // language_code: Option<&str> - Not known at initial import
        ) {
            warn!("[Trim Backend] Failed to save media_transcript_data for trimmed media {}: {}", db_key_relative_path_trimmed, e);
        } else {
            info!("[Trim Backend] Successfully saved media_transcript_data for trimmed media: {}", db_key_relative_path_trimmed);
        }

        // --- Start: Copy group associations from original media to trimmed media ---
        let original_media_relative_path = original_path.strip_prefix(project_base_dir)
            .map(|p| p.to_string_lossy().replace("\\", "/"))
            .map_err(|e| CommandError::from(format!("Failed to get relative path for original media: {}", e)));

        if let Ok(orig_rel_path) = original_media_relative_path {
            info!("[Trim Backend] Attempting to copy group associations from original media '{}' to new media '{}'", orig_rel_path, db_key_relative_path_trimmed);
            match db_handler::get_db_path() {
                Ok(db_path_for_groups) => {
                    match rusqlite::Connection::open(&db_path_for_groups) {
                        Ok(conn) => {
                            match db_handler::get_groups_for_file_asset(&conn, &project_uuid_for_db, &orig_rel_path) {
                                Ok(groups) => {
                                    if groups.is_empty() {
                                        info!("[Trim Backend] Original media '{}' belongs to no groups. No associations to copy.", orig_rel_path);
                                    } else {
                                        info!("[Trim Backend] Original media '{}' belongs to {} group(s). Copying to new media '{}'.", orig_rel_path, groups.len(), db_key_relative_path_trimmed);
                                        for group in groups {
                                            match db_handler::add_file_to_group(&conn, &project_uuid_for_db, &group.id, &db_key_relative_path_trimmed) {
                                                Ok(_) => info!("[Trim Backend] Successfully added trimmed media '{}' to group '{}' (ID: {})", db_key_relative_path_trimmed, group.name, group.id),
                                                Err(e) => warn!("[Trim Backend] Failed to add trimmed media '{}' to group '{}' (ID: {}): {}. It might already be associated.", db_key_relative_path_trimmed, group.name, group.id, e),
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("[Trim Backend] Failed to get groups for original media '{}': {}. Skipping group association copy.", orig_rel_path, e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("[Trim Backend] Failed to open DB connection to copy group associations: {}. Skipping.", e);
                        }
                    }
                }
                Err(e) => {
                     warn!("[Trim Backend] Failed to get DB path to copy group associations: {}. Skipping.", e);
                }
            }
        } else {
            warn!("[Trim Backend] Could not determine original media relative path. Skipping group association copy.");
        }
        // --- End: Copy group associations ---

        // --- End: Add metadata to SQLite database ---
    }


    reloaded_project_data_result.map(|data| data.files)
}


#[derive(serde::Deserialize, Debug)]
pub struct SaveSpeakerConfigPayload {
    project_xml_path: String,
    media_identifier: String,
    count: usize,
    names: Vec<String>,
    translated_names: Option<Vec<String>>,
}

#[tauri::command]
pub async fn save_speaker_config(payload: SaveSpeakerConfigPayload) -> Result<(), CommandError> {
    info!("[Backend SaveSpeakers] Request: Project='{}', MediaID='{}', Count={}, Names={:?}, TranslatedNames={:?}",
        payload.project_xml_path, payload.media_identifier, payload.count, payload.names, payload.translated_names);

    let xml_path = PathBuf::from(&payload.project_xml_path);
    if !xml_path.exists() || !xml_path.is_file() {
        return Err(CommandError::from(format!("Project file not found: {}", payload.project_xml_path)));
    }

    let xml_content = fs::read_to_string(&xml_path)?;
    let mut project_data: ProjectXml = quick_xml::de::from_str(&xml_content)?;
    let mut found_and_updated = false;

    if let Some(media_file) = project_data.media_files.files.iter_mut().find(|f| f.name == payload.media_identifier) {
        info!("[Backend SaveSpeakers] Found entry '{}'. Updating speakers.", payload.media_identifier);

        let mut validated_count = payload.count;
        let mut validated_names = payload.names.clone();
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

        // Handle translated_names, defaulting to Some(Vec::new()) if None is provided from payload
        let validated_translated_names = payload.translated_names.clone().unwrap_or_else(Vec::new);
        // Further validation for translated_names (e.g., trimming, ensuring uniqueness if needed) can be added here
        // For now, directly use the provided or defaulted Vec.

        let speakers_data = SpeakersXml {
            count: validated_count,
            names: validated_names,
            translated_names: Some(validated_translated_names), // Store as Some(Vec)
        };
        info!("[Backend SaveSpeakers] Saving validated config: {:?}", speakers_data);
        media_file.speakers = Some(speakers_data);
        found_and_updated = true;
    }

    if !found_and_updated {
        return Err(CommandError::from(format!("Media ID '{}' not found in XML.", payload.media_identifier)));
    }

    save_project_xml(&xml_path, &project_data)?;
    info!("[Backend SaveSpeakers] Success for '{}'.", payload.media_identifier);
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
    lexical_table_json_string: String,
    language_code: Option<String>, // Added language_code parameter
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
            if !(json_value.get("root").is_some() && json_value.get("root").unwrap().is_object() &&
                 json_value.get("root").unwrap().get("children").is_some() && json_value.get("root").unwrap().get("children").unwrap().is_array()) {
                 return Err(CommandError::from("Provided string is not a valid Lexical JSON structure (missing root object or children array)."));
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
                // Always update the language code, even for existing entries.
                transcript_xml_entry_instance.language_code = language_code.clone();

                found_transcript_xml_entry = true;
                break;
            }
        }

        if !found_transcript_xml_entry {
            debug!("[Backend Save Full Transcript JSON] Adding new transcript entry for '{}'.", transcript_relative_path);
            media_entry.transcripts.push(TranscriptEntryXml {
                name: transcript_filename.clone(),
                relative_path: transcript_relative_path.clone(),
                language_code: language_code.clone(), // Add language_code here
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
) -> Result<(String, PathBuf, PathBuf, PathBuf, Option<String>, Option<PathBuf>, Option<PathBuf>), CommandError> { // Return signature matches new var names
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

    // --- Paths for original transcript ---
    // Use a short, generic temporary base name for whisper's direct output.
    let temp_whisper_output_base_orig = transcripts_dir.join(format!("whisper_temp_{}_orig", job_id));
    let temp_whisper_output_base_orig_str = temp_whisper_output_base_orig.to_string_lossy().to_string();
    // This is the path whisper-cli will actually write its JSON to.
    let expected_whisper_temp_json_path_orig = temp_whisper_output_base_orig.with_extension("json");

    // The final path for the transcript uses the (potentially truncated at import) media_filename_stem.
    let mut final_transcript_path_orig = transcripts_dir.join(format!("{}_1.json", media_filename_stem));
    let mut counter = 2;
    while final_transcript_path_orig.exists() {
        final_transcript_path_orig = transcripts_dir.join(format!("{}_{}.json", media_filename_stem, counter));
        counter += 1;
    }
    
    // Path for RTTM (common for original transcript diarization) - can also use a short temp name.
    let temp_rttm_base = transcripts_dir.join(format!("rttm_temp_{}", job_id)); // Generic base for RTTM
    let expected_rttm_temp_path = temp_rttm_base.with_extension("rttm");

    debug!("[prepare_output_paths][{}] Orig Temp Whisper Base: '{}', Orig Whisper JSON (temp): '{}', RTTM (temp): '{}', Orig Final JSON: '{}'",
        job_id, temp_whisper_output_base_orig_str, expected_whisper_temp_json_path_orig.display(), expected_rttm_temp_path.display(), final_transcript_path_orig.display());

    // --- Paths for translated transcript (if requested) ---
    let mut temp_whisper_output_base_en_str_opt: Option<String> = None; // Use _opt suffix for clarity
    let mut expected_whisper_temp_json_path_en_opt: Option<PathBuf> = None; // Use _opt suffix
    let mut final_transcript_path_en_opt: Option<PathBuf> = None; // Use _opt suffix

    if translate_to_english {
        let temp_whisper_output_base_en = transcripts_dir.join(format!("whisper_temp_{}_en", job_id));
        temp_whisper_output_base_en_str_opt = Some(temp_whisper_output_base_en.to_string_lossy().to_string());
        expected_whisper_temp_json_path_en_opt = Some(temp_whisper_output_base_en.with_extension("json"));

        // Final path for translated transcript also uses the media_filename_stem.
        let mut final_transcript_path_en = transcripts_dir.join(format!("{}.en.json", media_filename_stem));
        let mut counter = 1;
        while final_transcript_path_en.exists() {
            final_transcript_path_en = transcripts_dir.join(format!("{}_{}.en.json", media_filename_stem, counter));
            counter += 1;
        }
        final_transcript_path_en_opt = Some(final_transcript_path_en);

        debug!("[prepare_output_paths][{}] EN Temp Whisper Base: '{:?}', EN Whisper JSON (temp): '{:?}', EN Final JSON: '{:?}'",
            job_id, temp_whisper_output_base_en_str_opt, expected_whisper_temp_json_path_en_opt, final_transcript_path_en_opt);
    }

    Ok((
        temp_whisper_output_base_orig_str, // This is correct as it's the first element
        expected_whisper_temp_json_path_orig,
        expected_rttm_temp_path,
        final_transcript_path_orig,
        temp_whisper_output_base_en_str_opt, // Use the new Option suffixed name
        expected_whisper_temp_json_path_en_opt, // Use the new Option suffixed name
        final_transcript_path_en_opt, // Use the new Option suffixed name
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
    speaker_names: Vec<String>, // For original transcript
    translated_speaker_names: Option<Vec<String>>, // For translated transcript
}

#[derive(serde::Serialize, Clone)]
pub struct TranscriptionResultPayload {
    original_transcript_path: String,
    translated_transcript_path: Option<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct TranscriptionInitiatedPayload {
    job_id: String,
}

// --- Payload for transcription job completion event (redefined for this context) ---
// This version is simplified for the events emitted directly by transcribe_media_command
// in case of early errors or successful initiation that then hands over to another process.
// The local_handler/transcription.rs will use its own more detailed version for its specific events.
#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionJobCompletedPayload { // Made pub
    job_id: String,
    status: String, // "done" (by local_handler), "cancelled" (by local_handler), "error" (by local_handler), "failed_initiation" (by this command)
    job_finished_path: String, // Path of the media that was processed or attempted
    transcript_file_path: Option<String>, // Path to the main (original) transcript (None if error before creation)
    translated_transcript_file_path: Option<String>,
    error_message: Option<String>,
}

#[tauri::command]
pub async fn transcribe_media_command<R: Runtime>(
    app_handle: AppHandle<R>,
    payload: TranscribeMediaPayload,
    cancel_state: tauri::State<'_, crate::TranscriptionCancellationState>, // Added
) -> Result<TranscriptionInitiatedPayload, CommandError> {
    let job_id = uuid::Uuid::new_v4().to_string();
    info!("[Transcribe Command][{}] Received request: {:?}", job_id, payload);

    // Create and register the cancellation flag
    let cancel_flag = Arc::new(AtomicBool::new(false)); // cancel_flag is not used in this command directly but passed to run_transcription
    cancel_state.0.insert(job_id.clone(), Arc::clone(&cancel_flag));
    info!("[Transcribe Command][{}] Registered with cancellation state.", job_id);

    // Setup the guard to ensure cleanup from the DashMap
    let _cancel_guard = CancelGuard {
        job_id: job_id.clone(),
        state: Arc::clone(&cancel_state.0),
    };

    // 1. Initial Cancellation Check
    if cancel_flag.load(AtomicOrdering::Relaxed) {
        warn!("[Transcribe Command][{}] Cancelled immediately after job registration.", job_id);
        let _ = app_handle.emit("custom_transcription_job_completed", TranscriptionJobCompletedPayload {
            job_id: job_id.clone(),
            status: "cancelled".to_string(),
            job_finished_path: payload.media_path_str.clone(),
            transcript_file_path: None,
            translated_transcript_file_path: None,
            error_message: Some("Transcription cancelled by user immediately.".to_string()),
        });
        return Err(CommandError::from("Transcription cancelled by user immediately."));
    }

    let media_path_for_filename = std::path::PathBuf::from(&payload.media_path_str);
    let media_filename_for_progress = media_path_for_filename.file_name()
        .map_or_else(
            || payload.media_path_str.clone(),
            |os_str| os_str.to_string_lossy().into_owned()
        );

    let app_handle_clone = app_handle.clone(); // app_handle is used later for final emit, so clone for helpers

    let (
        temp_whisper_output_base_orig_str, // Changed variable name
        expected_whisper_temp_json_path_orig,
        expected_rttm_temp_path,
        final_transcript_path_orig,
        temp_whisper_output_base_en_str_opt, // Changed variable name
        expected_whisper_temp_json_path_en_opt, // Changed variable name
        final_transcript_path_en_opt, // Changed variable name
    ) = prepare_output_paths(&payload.media_path_str, &job_id, payload.translate_to_english)?;

    // Use final_transcript_path_en_opt for the payload, it's already an Option<PathBuf>
    let final_transcript_path_en_for_payload = final_transcript_path_en_opt.clone();

    emit_progress_cmd(&app_handle_clone, &job_id, 0.0, &format!("Processing {}...", media_filename_for_progress))?;

    let wav_media_path = match convert_to_wav_if_needed_cmd(
        &app_handle_clone,
        &payload.media_path_str,
        &job_id,
        &media_filename_for_progress,
        cancel_flag.clone(),
    )
    .await
    {
        Ok(path) => path,
        Err(e) => {
            let error_message = e.to_string();
            if error_message.to_lowercase().contains("cancel") {
                warn!("[Transcribe Command][{}] WAV conversion cancelled by helper. Emitting event.", job_id);
                let _ = app_handle.emit("custom_transcription_job_completed", TranscriptionJobCompletedPayload {
                    job_id: job_id.clone(),
                    status: "cancelled".to_string(),
                    job_finished_path: payload.media_path_str.clone(),
                    transcript_file_path: None,
                    translated_transcript_file_path: None,
                    error_message: Some(error_message.clone()), // Use error message from helper
                });
            } else {
                error!("[Transcribe Command][{}] WAV conversion failed: {}. Emitting error event.", job_id, error_message);
                let _ = app_handle.emit("custom_transcription_job_completed", TranscriptionJobCompletedPayload {
                    job_id: job_id.clone(),
                    status: "error".to_string(),
                    job_finished_path: payload.media_path_str.clone(),
                    transcript_file_path: None,
                    translated_transcript_file_path: None,
                    error_message: Some(format!("WAV conversion failed: {}", error_message)),
                });
            }
            return Err(e); // Propagate the original error from the helper
        }
    };

    emit_progress_cmd(&app_handle_clone, &job_id, 5.0, &format!("Audio for {} prepared.", media_filename_for_progress))?;

    let whisper_model_path_str = resolve_whisper_model_path_cmd(&payload.model_name, &job_id)?;

    // Pre-execute_transcription_pass (Original Language) Cancellation Check
    if cancel_flag.load(AtomicOrdering::Relaxed) {
        warn!("[Transcribe Command][{}] Cancelled before main processing pass.", job_id);
        if wav_media_path.to_string_lossy() != payload.media_path_str {
            let _ = fs::remove_file(&wav_media_path).map_err(|e| warn!("[Transcribe Command][{}] Failed to clean up temp WAV during pre-original cancel: {:?}", job_id, e));
        }
        let _ = app_handle.emit("custom_transcription_job_completed", TranscriptionJobCompletedPayload {
            job_id: job_id.clone(),
            status: "cancelled".to_string(),
            job_finished_path: payload.media_path_str.clone(),
            transcript_file_path: None,
            translated_transcript_file_path: None,
            error_message: Some("Transcription cancelled by user before main processing pass.".to_string()),
        });
        return Err(CommandError::from("Transcription cancelled before main processing pass."));
    }

    // --- First Pass: Original Language Transcription ---
    emit_progress_cmd(&app_handle_clone, &job_id, 10.0, &format!("Transcribing {}...", media_filename_for_progress))?;

    let mut original_segments = match execute_transcription_pass(
        &app_handle_clone,
        &wav_media_path.to_string_lossy(),
        &whisper_model_path_str,
        &payload.language_code.clone().unwrap_or_else(|| "auto".to_string()),
        &job_id,
        &temp_whisper_output_base_orig_str, // Use changed variable name
        &expected_whisper_temp_json_path_orig,
        payload.num_speakers,
        &expected_rttm_temp_path,
        false, // is_translation_pass
        &payload.speaker_names, // Pass as slice
        &media_filename_for_progress,
        cancel_flag.clone(),
    ).await {
        Ok(segments) => segments,
        Err(e) => {
            let error_message = e.to_string();
            warn!("[Transcribe Command][{}] Original transcription pass failed: {}", job_id, error_message);
            // Cleanup temporary files from this pass
            if wav_media_path.to_string_lossy() != payload.media_path_str { // Check if WAV was temporary
                let _ = fs::remove_file(&wav_media_path).map_err(|e_del| warn!("[Transcribe Command][{}] Failed to delete temp WAV file during original pass error: {:?}", job_id, e_del));
            }
            let _ = fs::remove_file(&expected_whisper_temp_json_path_orig).map_err(|e_del| warn!("[Transcribe Command][{}] Failed to delete temp Whisper JSON during original pass error: {:?}", job_id, e_del));
            if payload.num_speakers > 0 {
                let _ = fs::remove_file(&expected_rttm_temp_path).map_err(|e_del| warn!("[Transcribe Command][{}] Failed to delete temp RTTM file during original pass error: {:?}", job_id, e_del));
            }

            if error_message.to_lowercase().contains("cancel") {
                let _ = app_handle.emit("custom_transcription_job_completed", TranscriptionJobCompletedPayload {
                    job_id: job_id.clone(),
                    status: "cancelled".to_string(),
                    job_finished_path: payload.media_path_str.clone(),
                    transcript_file_path: None,
                    translated_transcript_file_path: None,
                    error_message: Some(error_message.clone()),
                });
            } else {
                 let _ = app_handle.emit("custom_transcription_job_completed", TranscriptionJobCompletedPayload {
                    job_id: job_id.clone(),
                    status: "error".to_string(),
                     job_finished_path: payload.media_path_str.clone(),
                    transcript_file_path: None,
                    translated_transcript_file_path: None,
                    error_message: Some(format!("Original transcription pass failed: {}", error_message)),
                });
            }
            return Err(CommandError::from(format!("Original transcription pass failed: {}", error_message)));
        }
    };

    map_speaker_ids_to_names(&mut original_segments, &payload.speaker_names);

    emit_progress_cmd(&app_handle_clone, &job_id, 50.0, &format!("Saving transcript for {}...", media_filename_for_progress))?;
    let lexical_json_orig = create_lexical_table_from_segments(&original_segments);
    let lexical_json_orig_str = serde_json::to_string_pretty(&lexical_json_orig)
        .map_err(|e| CommandError::from(format!("Failed to serialize original Lexical Table JSON: {}", e)))?;

    // Determine the language code to save for the original transcript.
    let original_lang_code_to_save = match payload.language_code.as_deref() {
        None => Some("original".to_string()),
        Some(lang) if lang.is_empty() || lang == "auto" => Some("original".to_string()),
        Some(lang) => Some(lang.to_string()),
    };

    save_transcript_json(
        payload.project_xml_path.clone(),
        final_transcript_path_orig.to_string_lossy().to_string(),
        lexical_json_orig_str,
        original_lang_code_to_save,
    ).await?;
    info!("[Transcribe Command][{}] Original transcript saved to: {:?}", job_id, final_transcript_path_orig);
    emit_progress_cmd(&app_handle_clone, &job_id, 55.0, &format!("Original transcript for {} saved.", media_filename_for_progress))?;

    info!("[Transcribe Command][{}] Attempting to clean up temporary files for original pass...", job_id);
    info!("[Transcribe Command][{}] Targeting temp original whisper JSON for deletion: {:?}", job_id, expected_whisper_temp_json_path_orig);
    if expected_whisper_temp_json_path_orig.exists() {
        if let Err(e) = fs::remove_file(&expected_whisper_temp_json_path_orig) {
            warn!("[Transcribe Command][{}] Failed to delete temp original whisper JSON {:?}: {}", job_id, expected_whisper_temp_json_path_orig, e);
        } else {
            info!("[Transcribe Command][{}] Successfully deleted temp original whisper JSON: {:?}", job_id, expected_whisper_temp_json_path_orig);
        }
    } else {
        warn!("[Transcribe Command][{}] Temp original whisper JSON not found for deletion: {:?}", job_id, expected_whisper_temp_json_path_orig);
    }

    if payload.num_speakers > 0 {
        info!("[Transcribe Command][{}] Targeting temp RTTM file for deletion: {:?}", job_id, expected_rttm_temp_path);
        if expected_rttm_temp_path.exists() {
            if let Err(e) = fs::remove_file(&expected_rttm_temp_path) {
                warn!("[Transcribe Command][{}] Failed to delete temp RTTM file {:?}: {}", job_id, expected_rttm_temp_path, e);
            } else {
                info!("[Transcribe Command][{}] Successfully deleted temp RTTM file: {:?}", job_id, expected_rttm_temp_path);
            }
        } else {
            // This case is fine, as RTTM might not be created if diarization fails or num_speakers is 0.
            debug!("[Transcribe Command][{}] Temp RTTM file not found (or num_speakers was 0), skipping deletion: {:?}", job_id, expected_rttm_temp_path);
        }
    }

    // --- Second Pass: English Translation (if requested) ---
    if payload.translate_to_english {
        // Pre-execute_transcription_pass (Translation) Cancellation Check
        if cancel_flag.load(AtomicOrdering::Relaxed) {
            warn!("[Transcribe Command][{}] Cancelled before translation pass.", job_id);
            if wav_media_path.to_string_lossy() != payload.media_path_str {
                let _ = fs::remove_file(&wav_media_path).map_err(|e| warn!("[Transcribe Command][{}] Failed to clean up temp WAV during pre-translation cancel: {:?}", job_id, e));
            }
            // Original transcript might have been saved, attempt to remove it if cancellation occurs here.
            // Or, decide if it should be kept. For now, let's assume it might be partial or unwanted.
            // However, `final_transcript_path_orig` is typically a final destination, not temporary.
            // Let's reconsider removing `final_transcript_path_orig`. Usually, we only remove temp files.
            // The `expected_whisper_temp_json_path_orig` and `expected_rttm_temp_path` should have been cleaned up by successful original pass, or its error handling.

            let _ = app_handle.emit("custom_transcription_job_completed", TranscriptionJobCompletedPayload {
                job_id: job_id.clone(),
                status: "cancelled".to_string(),
                job_finished_path: payload.media_path_str.clone(),
                transcript_file_path: Some(final_transcript_path_orig.to_string_lossy().into_owned()),
                translated_transcript_file_path: None,
                error_message: Some("Transcription cancelled by user before translation pass.".to_string()),
            });
            return Err(CommandError::from("Transcription cancelled before translation pass."));
        }

        info!("[Transcribe Command][{}] DEBUG: Entered 'translate_to_english' block. translate_to_english flag is true.", job_id);
        info!("[Transcribe Command][{}] DEBUG: Pre-translation pass paths: temp_base_en: {:?}, temp_json_en: {:?}, final_en: {:?}", job_id, temp_whisper_output_base_en_str_opt, expected_whisper_temp_json_path_en_opt, final_transcript_path_en_for_payload);

        if let (Some(base_en_str), Some(json_path_en), Some(final_path_en_pb)) = (
            temp_whisper_output_base_en_str_opt, // Use changed variable name
            expected_whisper_temp_json_path_en_opt, // Use changed variable name
            final_transcript_path_en_opt, // Use changed variable name (already an Option<PathBuf>)
        ) {
            emit_progress_cmd(&app_handle_clone, &job_id, 60.0, &format!("Translating {}...", media_filename_for_progress))?;

            info!("[Transcribe Command][{}] DEBUG: Preparing for English translation pass call.", job_id);
            info!("[Transcribe Command][{}]   WAV Path: {:?}", job_id, wav_media_path.to_string_lossy());
            info!("[Transcribe Command][{}]   Model Path: {}", job_id, whisper_model_path_str);
            info!("[Transcribe Command][{}]   Lang Code for EN pass: en", job_id);
            info!("[Transcribe Command][{}]   Output Base for EN Whisper: {}", job_id, base_en_str);
            info!("[Transcribe Command][{}]   Expected EN Whisper JSON: {:?}", job_id, json_path_en);
            info!("[Transcribe Command][{}]   Is Translation Pass: true", job_id);
            info!("[Transcribe Command][{}]   Num Speakers for EN pass: 0", job_id);

            // Safely get owned strings for paths needed by execute_transcription_pass
            let base_en_str_owned = base_en_str.clone(); // already a String from Option<String>
            let json_path_en_owned = json_path_en.clone(); // already a PathBuf from Option<PathBuf>

            // Determine the source language for the translation process
            let source_language_for_translation = payload.language_code.clone().unwrap_or_else(|| "auto".to_string());
            info!("[Transcribe Command][{}]   Source Lang for Translation: {}", job_id, source_language_for_translation);


            // 6. Handle execute_transcription_pass (Translation) Result
            let translation_result = execute_transcription_pass(
                &app_handle_clone,
                &wav_media_path.to_string_lossy(),
                &whisper_model_path_str,
                &source_language_for_translation, // Use determined source language
                &job_id,
                &base_en_str_owned, // Use owned string
                &json_path_en_owned,  // Use owned PathBuf
                0, // No diarization for translation pass
                &PathBuf::new(), // Empty RTTM path
                true, // is_translation_pass
                &payload.speaker_names,
                &media_filename_for_progress,
                cancel_flag.clone(),
            ).await;

            let mut translated_segments = match translation_result {
                Ok(segments) => {
                    info!("[Transcribe Command][{}] DEBUG: English translation pass call completed. Number of segments: {}", job_id, segments.len());
                    segments
                }
                Err(e) => {
                    let error_message = e.to_string();
                    warn!("[Transcribe Command][{}] Translation pass failed: {}", job_id, error_message);
                    // Cleanup temporary files from this pass
                    if wav_media_path.to_string_lossy() != payload.media_path_str {
                        let _ = fs::remove_file(&wav_media_path).map_err(|e_del| warn!("[Transcribe Command][{}] Failed to delete temp WAV file during translation pass error: {:?}", job_id, e_del));
                    }
                    // expected_whisper_temp_json_path_orig and expected_rttm_temp_path should have been cleaned by original pass.
                    // Only clean up translation-specific temp files here.
                    let _ = fs::remove_file(&json_path_en).map_err(|e_del| warn!("[Transcribe Command][{}] Failed to delete temp EN Whisper JSON during translation pass error: {:?}", job_id, e_del));


                    if error_message.to_lowercase().contains("cancel") {
                        let _ = app_handle.emit("custom_transcription_job_completed", TranscriptionJobCompletedPayload {
                            job_id: job_id.clone(),
                            status: "cancelled".to_string(),
                            job_finished_path: payload.media_path_str.clone(),
                            transcript_file_path: Some(final_transcript_path_orig.to_string_lossy().into_owned()), // Original is kept
                            translated_transcript_file_path: None,
                            error_message: Some(error_message.clone()),
                        });
                    } else {
                        let _ = app_handle.emit("custom_transcription_job_completed", TranscriptionJobCompletedPayload {
                            job_id: job_id.clone(),
                            status: "error".to_string(),
                            job_finished_path: payload.media_path_str.clone(),
                           transcript_file_path: Some(final_transcript_path_orig.to_string_lossy().into_owned()), // Original is kept
                           translated_transcript_file_path: None,
                            error_message: Some(format!("Translation pass failed: {}", error_message)),
                        });
                    }
                    return Err(CommandError::from(format!("Translation pass failed: {}", error_message)));
                }
            };

            // Speaker mapping for translated segments can be complex.
            // A simple approach: if segment counts are similar, try to reuse original speakers.
            // This might need refinement based on actual whisper output for translations.
            // For now, let's apply the same mapping, or default if counts differ significantly.
            // A more robust solution might involve aligning segments by time.
            // if translated_segments.len() == original_segments.len() {
            //      map_speaker_ids_to_names(&mut translated_segments, &payload.speaker_names);
            // } else {
            //     warn!("[Transcribe Command][{}] Segment count mismatch after translation (orig: {}, trans: {}). Speaker names might be less accurate for translated version.", job_id, original_segments.len(), translated_segments.len());
            //     // Optionally, apply a default "SPEAKER_XX" or clear speakers for translated version
            //     // For now, try mapping anyway or let map_speaker_ids_to_names handle it based on its logic.
            //      map_speaker_ids_to_names(&mut translated_segments, &payload.speaker_names);
            // }
            info!("[Transcribe Command][{}] Aligning speakers for translated segments based on original diarization and original speaker names list...", job_id);
            align_speakers_to_translated_segments(&original_segments, &mut translated_segments, &payload.speaker_names, &job_id);

            // Apply translated_speaker_names if provided
            if let Some(ref translated_names) = payload.translated_speaker_names {
                let contains_actual_names = translated_names.iter().any(|name| !name.trim().is_empty());
                if !translated_names.is_empty() && contains_actual_names {
                    info!("[Transcribe Command][{}] Applying user-defined translated speaker names to translated segments.", job_id);
                    map_speaker_ids_to_names(&mut translated_segments, translated_names);
                } else if !translated_names.is_empty() && !contains_actual_names {
                    info!("[Transcribe Command][{}] Translated speaker names list provided, but all names are empty. Using aligned speaker IDs for translated segments.", job_id);
                } else {
                    info!("[Transcribe Command][{}] Translated speaker names list is empty. Using aligned speaker IDs for translated segments.", job_id);
                }
            } else {
                info!("[Transcribe Command][{}] No translated speaker names list provided (Option is None). Using aligned speaker IDs for translated segments.", job_id);
            }

            emit_progress_cmd(&app_handle_clone, &job_id, 90.0, &format!("Saving translation for {}...", media_filename_for_progress))?;
            info!("[Transcribe Command][{}] DEBUG: Attempting to save translated transcript to: {:?}", job_id, final_path_en_pb);
            let lexical_json_en = create_lexical_table_from_segments(&translated_segments);
            let lexical_json_en_str = serde_json::to_string_pretty(&lexical_json_en)
                .map_err(|e| CommandError::from(format!("Failed to serialize translated Lexical Table JSON: {}", e)))?;

            save_transcript_json(
                payload.project_xml_path.clone(),
                final_path_en_pb.to_string_lossy().to_string(),
                lexical_json_en_str,
                Some("en".to_string()), // Always use "en" for translated transcripts
            ).await?;
            info!("[Transcribe Command][{}] Translated transcript saved to: {:?}", job_id, final_path_en_pb);
            emit_progress_cmd(&app_handle_clone, &job_id, 95.0, &format!("Translation for {} saved.", media_filename_for_progress))?;

            info!("[Transcribe Command][{}] Attempting to clean up temporary files for translation pass...", job_id);
            info!("[Transcribe Command][{}] Targeting temp translated whisper JSON for deletion: {:?}", job_id, json_path_en_owned);
            if json_path_en_owned.exists() {
                if let Err(e) = fs::remove_file(&json_path_en_owned) {
                    warn!("[Transcribe Command][{}] Failed to delete temp translated whisper JSON {:?}: {}", job_id, json_path_en_owned, e);
                } else {
                    info!("[Transcribe Command][{}] Successfully deleted temp translated whisper JSON: {:?}", job_id, json_path_en_owned);
                }
            } else {
                warn!("[Transcribe Command][{}] Temp translated whisper JSON not found for deletion: {:?}", job_id, json_path_en_owned);
            }
            } else { // This 'else' corresponds to if let (Some(base_en_str)...
            warn!("[Transcribe Command][{}] Translation requested, but English output paths are not available from prepare_output_paths. Skipping translation.", job_id);
        }
        } else { // This 'else' corresponds to if payload.translate_to_english
            // If no translation, add a "Finalizing" step before 100%
            emit_progress_cmd(&app_handle_clone, &job_id, 95.0, &format!("Finalizing {}...", media_filename_for_progress))?;
    }
        // Final message
    emit_progress_cmd(&app_handle_clone, &job_id, 100.0, &format!("Successfully processed {}.", media_filename_for_progress))?;
    info!("[Transcribe Command][{}] Processing complete.", job_id);

    let final_status_message = if payload.translate_to_english && final_transcript_path_en_for_payload.is_some() {
        "Transcription and translation complete."
    } else if payload.translate_to_english && final_transcript_path_en_for_payload.is_none() {
        "Transcription complete; translation was skipped or failed to produce a final path."
    } else {
        "Transcription complete."
    };
    info!("[Transcribe Command][{}] {}", job_id, final_status_message);

    let completion_payload = TranscriptionJobCompletedPayload {
        job_id: job_id.clone(),
        status: "done".to_string(),
        job_finished_path: payload.media_path_str.clone(),
        transcript_file_path: Some(final_transcript_path_orig.to_string_lossy().into_owned()),
        translated_transcript_file_path: final_transcript_path_en_for_payload.map(|p| p.to_string_lossy().into_owned()),
        error_message: None,
    };

    if let Err(e) = app_handle.emit("custom_transcription_job_completed", completion_payload) {
        error!("[Transcribe Command][{}] Failed to emit custom_transcription_job_completed event: {}", job_id, e);
    }

    // Cleanup temporary WAV file if it was created
    if wav_media_path.to_string_lossy() != payload.media_path_str {
        info!("[Transcribe Command][{}] Cleaning up temporary WAV file: {:?}", job_id, wav_media_path);
        if let Err(e) = fs::remove_file(&wav_media_path) {
            warn!("[Transcribe Command][{}] Failed to delete temporary WAV file {:?}: {}", job_id, wav_media_path, e);
        } else {
            info!("[Transcribe Command][{}] Successfully deleted temporary WAV file: {:?}", job_id, wav_media_path);
        }
    }

    Ok(TranscriptionInitiatedPayload { job_id })
}

// --- Implemented Helper Functions ---

fn align_speakers_to_translated_segments(
    original_segments: &[TranscriptSegment], // These should have correct speaker info from diarization
    translated_segments: &mut Vec<TranscriptSegment>, // These have speakers from Whisper, likely "Unknown"
    original_speaker_names: &[String], // New parameter: list of original speaker names
    job_id: &str, // For logging
) {
    if original_segments.is_empty() {
        warn!("[Align Speakers][{}] Original segments list is empty. Cannot align speakers for translation.", job_id);
        return;
    }
    if translated_segments.is_empty() {
        info!("[Align Speakers][{}] Translated segments list is empty. No speakers to align.", job_id);
        return;
    }

    info!("[Align Speakers][{}] Aligning speakers for {} translated segments based on {} original segments.", job_id, translated_segments.len(), original_segments.len());

    for t_seg in translated_segments.iter_mut() {
        let t_start = t_seg.start_time;
        let t_end = t_seg.end_time;
        // Ensure t_start is not greater than t_end to prevent panic in midpoint calculation or negative overlap.
        if t_start > t_end {
            warn!("[Align Speakers][{}] Skipping translated segment with invalid times: start {:.3} > end {:.3}", job_id, t_start, t_end);
            continue;
        }
        let t_mid = t_start + (t_end - t_start) / 2.0;

        let mut best_match_speaker = "Unknown".to_string();
        let mut max_overlap = 0.0f64;
        let mut best_overlap_tiebreak_priority = -1.0f64; // Lower is better (e.g., distance from midpoint)

        for o_seg in original_segments {
            let o_start = o_seg.start_time;
            let o_end = o_seg.end_time;
            if o_start > o_end { // Skip invalid original segments
                continue;
            }

            // Calculate overlap
            let overlap_start = t_start.max(o_start);
            let overlap_end = t_end.min(o_end);
            let current_overlap = (overlap_end - overlap_start).max(0.0);

            if current_overlap > 0.0 {
                if current_overlap > max_overlap {
                    max_overlap = current_overlap;
                    best_match_speaker = o_seg.speaker.clone();
                    // Prioritize segments containing the translated segment's midpoint
                    if t_mid >= o_start && t_mid < o_end {
                        best_overlap_tiebreak_priority = 0.0; // Highest priority
                    } else {
                        // Secondary tie-break: smallest distance between midpoints
                        let o_mid = o_start + (o_end - o_start) / 2.0;
                        best_overlap_tiebreak_priority = (t_mid - o_mid).abs();
                    }
                } else if current_overlap == max_overlap {
                    // Tie-breaking logic
                    let current_priority;
                    if t_mid >= o_start && t_mid < o_end {
                        current_priority = 0.0;
                    } else {
                        let o_mid = o_start + (o_end - o_start) / 2.0;
                        current_priority = (t_mid - o_mid).abs();
                    }

                    if current_priority < best_overlap_tiebreak_priority {
                        best_overlap_tiebreak_priority = current_priority;
                        best_match_speaker = o_seg.speaker.clone();
                    }
                }
            }
        }

        if max_overlap > 0.0 { // Only assign if there was any overlap
            debug!("[Align Speakers][{}] Assigning speaker '{}' to translated segment {:.3}-{:.3} (Max Overlap: {:.3})",
                job_id, best_match_speaker, t_start, t_end, max_overlap);
            // Found the speaker name from the original segment. Now find its index in original_speaker_names.
            if let Some(index) = original_speaker_names.iter().position(|name| name == &best_match_speaker) {
                t_seg.speaker = format!("SPEAKER_{:02}", index);
                debug!("[Align Speakers][{}] Mapped original speaker '{}' to generic ID 'SPEAKER_{:02}' for translated segment {:.3}-{:.3}",
                    job_id, best_match_speaker, index, t_start, t_end);
            } else {
                // If best_match_speaker is not in original_speaker_names (e.g., it's "Unknown" or an unmapped RTTM ID)
                warn!("[Align Speakers][{}] Speaker '{}' from original segment not found in original_speaker_names. Setting translated speaker to 'Unknown' for segment {:.3}-{:.3}.",
                    job_id, best_match_speaker, t_start, t_end);
                t_seg.speaker = "Unknown".to_string();
            }
        } else {
            // No overlap found.
            // The previous logic tried to find the *closest* original segment's speaker.
            // For this new logic, if there's no overlap, we should probably assign "Unknown".
            // Replicating a closest-match might lead to incorrect indexing if that closest speaker isn't truly related.
            warn!("[Align Speakers][{}] No overlapping original segment found for translated segment {:.3}-{:.3}. Speaker set to 'Unknown'.",
                job_id, t_start, t_end);
            t_seg.speaker = "Unknown".to_string();
        }
    }
    info!("[Align Speakers][{}] Finished aligning speakers for translated segments, mapping to generic IDs.", job_id);
}

// Adapted from local_handler/transcription.rs
// Omitting cancel_flag for now
pub(crate) async fn convert_to_wav_if_needed_cmd<R: Runtime>(
    app_handle: &AppHandle<R>,
    input_path_str: &str,
    job_id: &str,
    media_filename_for_progress: &str,
    cancel_flag: Arc<AtomicBool>, // New argument
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
    let _ = emit_progress_cmd(app_handle, job_id, 2.0, &format!("Converting {} to WAV...", media_filename_for_progress))?;

    let ffmpeg_path = get_ffmpeg_path(app_handle)?;

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
    let mut command = shell_scope.command(ffmpeg_path).args(args);

    if cfg!(target_os = "macos") {
        if let Ok(resource_dir) = app_handle.path().resource_dir() {
            let sidecars_path = resource_dir.join("sidecars");
            if sidecars_path.exists() {
                let sidecars_path_str = sidecars_path.to_string_lossy();
                if let Ok(existing_path) = std::env::var("DYLD_LIBRARY_PATH") {
                    command = command.env("DYLD_LIBRARY_PATH", format!("{}:{}", sidecars_path_str, existing_path));
                } else {
                    command = command.env("DYLD_LIBRARY_PATH", sidecars_path_str.to_string());
                }
            }
        }
    }

    let (mut rx, child) = command.spawn()?;
    debug!("[FFmpeg CMD][{}] Spawned FFmpeg process (PID: {:?})", job_id, child.pid());

    let mut ffmpeg_stderr: Vec<String> = Vec::new();
    let mut ffmpeg_exit_code: Option<i32> = None;
    let mut ffmpeg_error: Option<String> = None;

    loop {
        if cancel_flag.load(AtomicOrdering::Relaxed) {
            warn!("[FFmpeg CMD][{}] Cancellation requested. Killing FFmpeg process...", job_id);
            let _ = child.kill();
            if output_wav_path.exists() {
                if let Err(e) = fs::remove_file(&output_wav_path) {
                    warn!("[FFmpeg CMD][{}] Failed to remove partial WAV file {:?}: {}", job_id, output_wav_path, e);
                } else {
                    info!("[FFmpeg CMD][{}] Removed partial WAV file {:?}", job_id, output_wav_path);
                }
            }
            return Err(CommandError::from(format!("Audio conversion cancelled for job {}.", job_id)));
        }

        tokio::select! {
            biased;
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
                            ffmpeg_exit_code = Some(-1);
                        }
                        break;
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(50)) => { // Check flag periodically
                // This branch exists to ensure the loop continues and periodically checks the cancel_flag
                // if no CommandEvent is immediately available.
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
async fn run_whisper_cpp_sidecar_cmd<R: Runtime>(
    app_handle: &AppHandle<R>,
    media_path: &str, // Should be wav_media_path.to_string_lossy().to_string()
    whisper_model_path_str: &str,
    language: &str,
    job_id: &str,
    output_base_for_whisper: &str,
    expected_whisper_json_output_path: &Path,
    is_translation_pass: bool,
    cancel_flag: Arc<AtomicBool>, // New argument
) -> Result<PathBuf, CommandError> {
    let sidecar_name = "whisper-cli";
    let lang_arg = if language.trim().is_empty() || language == "auto" { "auto" } else { language.trim() };
    // debug!("[Whisper CPP CMD][{}] Using language: '{}', Translate: {}", job_id, lang_arg, is_translation_pass); // Original debug

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

    info!("[Whisper CPP CMD][{}] DEBUG: Executing whisper-cli. Language: '{}', Translate: {}. Full Args: {:?}", job_id, lang_arg, is_translation_pass, args);
    // debug!("[Whisper CPP CMD][{}] Running sidecar '{}' with args: {:?}", job_id, sidecar_name, args); // Original debug

    let shell_scope = app_handle.shell();
    let (mut rx, child) = shell_scope.sidecar(sidecar_name)?.args(args).spawn()
     .map_err(|e| {
         error!("Failed to spawn whisper-cli: {}. Check tauri.conf.json, binary paths, and permissions.", e);
         CommandError::from(format!("Failed to execute whisper-cli sidecar: {}. Ensure it's bundled and executable.", e))
     })?;
    info!("[Whisper CPP CMD][{}] Spawned sidecar '{}' (PID: {:?})", job_id, sidecar_name, child.pid());

    // Simplified event handling (omitting cancellation for now)
    let mut process_error: Option<String> = None;
    let mut exit_code: Option<i32> = None;

    let shared_child = Arc::new(Mutex::new(Some(child)));
    let cancel_flag_clone = cancel_flag.clone();
    let shared_child_clone_for_cancel = shared_child.clone();
    let job_id_clone = job_id.to_string();

    tokio::spawn(async move {
        loop {
            if cancel_flag_clone.load(AtomicOrdering::Relaxed) {
                warn!("[Whisper CPP CMD][{}] Cancellation requested. Killing process from spawned task...", job_id_clone);
                if let Some(child_to_kill) = shared_child_clone_for_cancel.lock().await.take() {
                    let _ = child_to_kill.kill();
                }
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }
    });

    loop {
        tokio::select! {
            biased;
            maybe_event = rx.recv() => {
                match maybe_event {
                    Some(event) => match event {
                        CommandEvent::Stdout(line) => { debug!("[{}][stdout][{}] {}", sidecar_name, job_id, String::from_utf8_lossy(&line).trim_end()); },
                        CommandEvent::Stderr(line) => { debug!("[{}][stderr][{}] {}", sidecar_name, job_id, String::from_utf8_lossy(&line).trim_end()); },
                        CommandEvent::Error(msg) => { process_error = Some(msg); break; },
                        CommandEvent::Terminated(payload) => { exit_code = payload.code; break; },
                        _ => {}
                    },
                    None => {
                        if exit_code.is_none() && process_error.is_none() {
                             warn!("[{}][{}] Event channel closed unexpectedly.", sidecar_name, job_id);
                             exit_code = Some(-1); // Treat as error
                        }
                        break;
                    }
                }
            }
        }
    }

    // Check cancel_flag one last time after loop exits to ensure consistent error reporting
    if cancel_flag.load(AtomicOrdering::Relaxed) {
        warn!("[Whisper CPP CMD][{}] Process terminated due to cancellation. Returning cancellation error.", job_id);
        if expected_whisper_json_output_path.exists() {
            if let Err(e) = fs::remove_file(expected_whisper_json_output_path) {
                warn!("[Whisper CPP CMD][{}] Failed to remove partial JSON output {:?}: {}", job_id, expected_whisper_json_output_path, e);
            } else {
                info!("[Whisper CPP CMD][{}] Removed partial JSON output {:?}", job_id, expected_whisper_json_output_path);
            }
        }
        return Err(CommandError::from(format!("Whisper C++ process cancelled for job {}.", job_id)));
    }

    if process_error.is_some() || exit_code != Some(0) {
        // Cleanup even on non-cancellation error, as Whisper might leave partial files.
        if expected_whisper_json_output_path.exists() { let _ = fs::remove_file(expected_whisper_json_output_path); }
        return Err(CommandError::from(format!("Sidecar '{}' failed. Exit: {:?}, Err: {:?}", sidecar_name, exit_code, process_error)));
    }

    // Wait for file to appear, with cancellation check
    let mut attempts = 0;
    while !expected_whisper_json_output_path.exists() && attempts < 10 { // Increased attempts slightly
        if cancel_flag.load(AtomicOrdering::Relaxed) {
            warn!("[Whisper CPP CMD][{}] Cancelled while waiting for output file.", job_id);
            // No need to remove expected_whisper_json_output_path as it doesn't exist yet
            return Err(CommandError::from(format!("Whisper C++ process cancelled (waiting for file) for job {}.", job_id)));
        }
        sleep(Duration::from_millis(300)).await;
        attempts += 1;
    }

    if !expected_whisper_json_output_path.exists() {
        error!("[Whisper CPP CMD][{}] Output JSON file NOT found after whisper-cli execution and wait: {:?}", job_id, expected_whisper_json_output_path);
        return Err(CommandError::from(format!("Whisper output JSON missing: {:?}", expected_whisper_json_output_path)));
    }

    // Validate file size (optional, but good practice)
    match expected_whisper_json_output_path.metadata() {
        Ok(meta) if meta.len() == 0 => {
            warn!("[Whisper CPP CMD][{}] Output JSON file is empty: {:?}", job_id, expected_whisper_json_output_path);
            let _ = fs::remove_file(expected_whisper_json_output_path); // Clean up empty file
            return Err(CommandError::from(format!("Whisper output JSON is empty: {:?}", expected_whisper_json_output_path)));
        }
        Err(e) => {
             warn!("[Whisper CPP CMD][{}] Could not get metadata for output file {:?}: {}", job_id, expected_whisper_json_output_path, e);
             // Potentially return error or proceed if metadata check is not critical
        }
        _ => {} // File exists and is not empty
    }

    info!("[Whisper CPP CMD][{}] DEBUG: Output JSON file FOUND after whisper-cli execution: {:?}", job_id, expected_whisper_json_output_path);
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
async fn run_diarization_script<R: Runtime>(
    app_handle: &AppHandle<R>,
    media_path: &str,
    num_speakers: usize,
    output_rttm_path: &Path,
    job_id: &str,
    cancel_flag: Arc<AtomicBool>,
) -> Result<PathBuf, CommandError> {
    info!("[Diarization Script][{}] Starting for: {}, num_speakers: {}", job_id, media_path, num_speakers);
    if let Some(parent_dir) = output_rttm_path.parent() { fs::create_dir_all(parent_dir)?; }

    let python_path = get_python_path().map_err(|e| CommandError::from(e.to_string()))?;

    let script_path = app_handle
        .path()
        .resolve("scripts/run_diarization.py", tauri::path::BaseDirectory::Resource)
        .map_err(|e| CommandError::from(e.to_string()))?;

    let token = get_hf_token(app_handle).map_err(|e| CommandError::from(e.to_string()))?;

    let args = vec![
        script_path.to_string_lossy().to_string(),
        media_path.to_string(),
        num_speakers.to_string(),
        token,
    ];

    let shell_scope = app_handle.shell();
    let mut command = shell_scope.command(python_path.to_string_lossy().to_string()).args(args);

    if cfg!(target_os = "macos") {
        if let Ok(resource_dir) = app_handle.path().resource_dir() {
            let sidecars_path = resource_dir.join("sidecars");
            if sidecars_path.exists() {
                let sidecars_path_str = sidecars_path.to_string_lossy();
                if let Ok(existing_path) = std::env::var("DYLD_LIBRARY_PATH") {
                    command = command.env("DYLD_LIBRARY_PATH", format!("{}:{}", sidecars_path_str, existing_path));
                } else {
                    command = command.env("DYLD_LIBRARY_PATH", sidecars_path_str.to_string());
                }
            }
        }
    }

    let (mut rx, child) = command.spawn()
        .map_err(|e| CommandError::from(format!("Failed to execute diarization script: {}", e)))?;
    info!("[Diarization Script][{}] Spawned python script (PID: {:?})", job_id, child.pid());

    let mut process_error: Option<String> = None;
    let mut exit_code: Option<i32> = None;
    let mut stderr_lines = Vec::new();
    let mut stdout_lines = Vec::new();

    loop {
        if cancel_flag.load(AtomicOrdering::Relaxed) {
            warn!("[Diarization Script][{}] Cancellation requested. Killing process...", job_id);
            let _ = child.kill();
            return Err(CommandError::from(format!("Diarization process cancelled for job {}.", job_id)));
        }

        tokio::select! {
            biased;
            maybe_event = rx.recv() => {
                match maybe_event {
                    Some(event) => match event {
                        CommandEvent::Stdout(line) => {
                            let line_str = String::from_utf8_lossy(&line).to_string();
                            debug!("[Diarization Script][stdout][{}] {}", job_id, line_str.trim_end());
                            stdout_lines.push(line_str);
                        },
                        CommandEvent::Stderr(line) => {
                            let line_str = String::from_utf8_lossy(&line).to_string();
                            debug!("[Diarization Script][stderr][{}] {}", job_id, line_str.trim_end());
                            stderr_lines.push(line_str);
                        },
                        CommandEvent::Error(msg) => { process_error = Some(msg); break; },
                        CommandEvent::Terminated(payload) => { exit_code = payload.code; break; },
                        _ => {}
                    },
                    None => {
                        if exit_code.is_none() && process_error.is_none() {
                            warn!("[Diarization Script][{}] Event channel closed unexpectedly.", job_id);
                            exit_code = Some(-1);
                        }
                        break;
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(50)) => {}
        }
    }

    if process_error.is_some() || exit_code != Some(0) {
        let stderr_output = stderr_lines.join("\n");
        let error_message = format!("Diarization script failed. Exit: {:?}, Err: {:?}\nStderr:\n{}", exit_code, process_error, stderr_output);
        error!("{}", error_message);
        return Err(CommandError::from(error_message));
    }

    let stdout_output = stdout_lines.join("");
    fs::write(output_rttm_path, stdout_output)?;

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

pub(crate) async fn execute_transcription_pass<R: Runtime>(
    app_handle: &AppHandle<R>,
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
    media_filename_for_progress: &str,
    cancel_flag: Arc<AtomicBool>, // New argument
) -> Result<Vec<TranscriptSegment>, CommandError> {
    info!("[Exec Pass][{}] DEBUG: Entered. Lang: {}, Translate: {}, NumSpeakers: {}, output_base_for_whisper: {}, expected_json: {:?}",
        job_id, language_code, is_translation_pass, num_speakers, output_base_for_whisper, expected_whisper_json_output_path);
    // Original info log:
    // ... (kept for reference)

    info!("[Exec Pass][{}] DEBUG: About to call run_whisper_cpp_sidecar_cmd. is_translation_pass: {}", job_id, is_translation_pass);
    // `expected_whisper_json_output_path` is the short temporary path whisper will write to.
    // `output_base_for_whisper` is its base name (without .json extension).
    let temp_whisper_json_output_path = run_whisper_cpp_sidecar_cmd(
        app_handle,
        wav_media_path_str,
        model_path,
        language_code,
        job_id,
        output_base_for_whisper, // This is the short temp base name
        expected_whisper_json_output_path, // This is the short temp full path with .json
        is_translation_pass,
        cancel_flag.clone(),
    ).await?;

    // Determine the final destination path for this pass's transcript
    let final_transcript_destination_path = if is_translation_pass {
        // This requires prepare_output_paths to provide the final EN path
        // Let's assume it's passed in or reconstructed correctly
        // For now, we need to ensure `prepare_output_paths` returns it and it's passed here.
        // This part of the logic needs `final_transcript_path_en` to be available here.
        // We'll assume `final_transcript_path_orig` is for original, and a similar var for EN.
        // Let's refine this: expected_whisper_json_output_path is temp, we need a *final* path argument.
        // This function should take the *final_destination_path* as an argument.
        // For now, let's construct it based on is_translation_pass:
        let media_path = PathBuf::from(wav_media_path_str);
        let media_filename_stem = media_path.file_stem().and_then(|s| s.to_str()).unwrap_or("transcript");
        let transcripts_dir = expected_whisper_json_output_path.parent().unwrap(); // get transcripts dir from temp path
        if is_translation_pass {
            transcripts_dir.join(format!("{}.en.json", media_filename_stem))
        } else {
            transcripts_dir.join(format!("{}.json", media_filename_stem))
        }
    } else {
        // This is the original pass, use the final_transcript_path_orig from prepare_output_paths
        // This needs to be passed into execute_transcription_pass
        // For now, reconstructing it based on the temp path's parent and media_filename_stem.
        let media_path = PathBuf::from(wav_media_path_str);
        let media_filename_stem = media_path.file_stem().and_then(|s| s.to_str()).unwrap_or("transcript");
        let transcripts_dir = expected_whisper_json_output_path.parent().unwrap();
        transcripts_dir.join(format!("{}.json", media_filename_stem))
    };


    info!("[Exec Pass][{}] Moving temporary whisper output from {:?} to {:?}", job_id, temp_whisper_json_output_path, final_transcript_destination_path);
    fs::rename(&temp_whisper_json_output_path, &final_transcript_destination_path)
        .map_err(|e| CommandError::from(format!("Failed to move whisper output to final destination: {}", e)))?;

    // Now parse from the final destination
    let mut segments = parse_whisper_json_cmd(&final_transcript_destination_path)?;

    if num_speakers > 0 && !is_translation_pass {
        emit_progress_cmd(app_handle, job_id, 30.0, &format!("Diarizing {}...", media_filename_for_progress))?;

        // RTTM path is already temporary and short based on prepare_output_paths change.
        let rttm_path = run_diarization_script(
            app_handle,
            wav_media_path_str,
            num_speakers,
            expected_rttm_output_path, // This is the temp RTTM path
            job_id,
            cancel_flag.clone(),
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
        // Clean up temp RTTM file after use
        if rttm_path.exists() {
            if let Err(e_rttm_del) = fs::remove_file(&rttm_path) {
                warn!("[Exec Pass][{}] Failed to delete temporary RTTM file {:?}: {}", job_id, rttm_path, e_rttm_del);
            }
        }

    } else {
        info!("[Exec Pass][{}] Skipping diarization.", job_id);
    }

    map_speaker_ids_to_names(&mut segments, speaker_names);

    info!("[Exec Pass][{}] Pass complete. Segments: {}. Final transcript at: {:?}", job_id, segments.len(), final_transcript_destination_path);
    Ok(segments)
}


// Helper to emit progress
pub(crate) fn emit_progress_cmd<R: Runtime>(
    app_handle: &AppHandle<R>,
    job_id: &str,
    percent: f32,
    message: &str,
) -> Result<(), CommandError> {
    let clamped_percent = percent.max(0.0).min(100.0);
    debug!("[Progress Emit CMD][{}] {:.1}% - {}", job_id, clamped_percent, message);
    app_handle.emit("TRANSCRIPTION_PROGRESS", ProgressPayload {
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

// --- cancel_transcription Command (moved from local_handler/transcription.rs) ---
#[tauri::command]
pub async fn cancel_transcription(
    job_id: String,
    cancel_state: tauri::State<'_, crate::TranscriptionCancellationState> // Ensure crate::TranscriptionCancellationState is in use
) -> Result<(), CommandError> { // Ensure CommandError is in use
    info!("[Transcribe Command][Cancel] Received cancellation request for job: {}", job_id);
    if let Some(flag_entry) = cancel_state.0.get(&job_id) {
        let cancel_flag = flag_entry.value();
        // Use AtomicOrdering alias if std::sync::atomic::Ordering is aliased, otherwise direct path.
        match cancel_flag.compare_exchange(false, true, AtomicOrdering::SeqCst, AtomicOrdering::SeqCst) {
             Ok(_) => { info!("[Transcribe Command][Cancel] Cancellation flag successfully SET for job: {}", job_id); }
             Err(_) => { info!("[Transcribe Command][Cancel] Cancellation flag was already SET for job: {}", job_id); }
        }
    } else {
        warn!("[Transcribe Command][Cancel] Cancellation request for unknown or already completed job ID: {}", job_id);
    }
    Ok(())
}

#[tauri::command]
pub async fn start_live_transcription(
    app_handle: AppHandle,
    model_name: String,
    language: String,
    save_audio: bool,
    active_document_path: String,
    project_uuid: String,
    project_base_dir: String,
    state: tauri::State<'_, LiveTranscriptionState>,
) -> Result<bool, String> {
    if state.is_running.load(std::sync::atomic::Ordering::SeqCst) {
        return Err("Live transcription is already running.".to_string());
    }

    let model_path = resolve_whisper_model_path_cmd(&model_name, "live")
        .map_err(|e| e.to_string())?;

    let mut args = vec![
        "-m".to_string(),
        model_path.clone(),
        "-l".to_string(),
        language.clone(),
        "--step".to_string(), "5000".to_string(),
        "--length".to_string(), "5000".to_string(),
        "-c".to_string(), "0".to_string(),
        "-t".to_string(), "8".to_string(),
        "--max-tokens".to_string(), "32".to_string(),
        "--audio-ctx".to_string(), "768".to_string(),
    ];

    let mut command = app_handle
        .shell()
        .sidecar("whisper-stream")
        .expect("failed to create `whisper-stream` command");

    if save_audio {
        let active_doc_path = PathBuf::from(&active_document_path);
        let attachments_dir = active_doc_path.parent().unwrap().join("attachments");
        fs::create_dir_all(&attachments_dir).map_err(|e| e.to_string())?;

        args.push("--save-audio".to_string());
        command = command.current_dir(attachments_dir);
    }

    let (mut rx, whisper_child) = command.args(args).spawn().map_err(|e| e.to_string())?;

    *state.whisper_child.lock().await = Some(whisper_child);
    state.is_running.store(true, std::sync::atomic::Ordering::SeqCst);
    *state.start_time.lock().await = Some(chrono::Utc::now());
    *state.active_document_path.lock().await = Some(active_document_path);
    *state.project_uuid.lock().await = Some(project_uuid);
    *state.project_base_dir.lock().await = Some(PathBuf::from(project_base_dir));

    let is_running_clone = state.is_running.clone();
    let app_handle_clone = app_handle.clone();
    let start_time_clone = state.start_time.lock().await.clone();

    tokio::spawn(async move {
        info!("[Live Transcription] Started listening to whisper-stream sidecar.");
        let mut last_text = String::new();
        let mut segment_start_time = 0.0;

        while let Some(event) = rx.recv().await {
            if !is_running_clone.load(std::sync::atomic::Ordering::SeqCst) {
                info!("[Live Transcription] Loop broken due to is_running flag being false.");
                break;
            }
            match event {
                CommandEvent::Stdout(line) => {
                    let text = String::from_utf8_lossy(&line).to_string();
                    let cleaned_text = text
                        .replace("[Start speaking]", "")
                        .replace("[BLANK_AUDIO]", "")
                        .replace("[ Silence ]", "")
                        .replace("\u{1b}[2K", "")
                        .replace("\r", "")
                        .trim()
                        .to_string();

                    if !cleaned_text.is_empty() && cleaned_text != last_text {
                        let is_final = !cleaned_text.ends_with("...");
                        let end_time = if let Some(start_time) = start_time_clone {
                            (chrono::Utc::now() - start_time).num_milliseconds() as f64 / 1000.0
                        } else {
                            0.0
                        };
                        let _ = app_handle_clone.emit("live_transcription_result", LiveTranscriptionResult { text: cleaned_text.clone(), is_final, start_time: segment_start_time, end_time });
                        info!("[Live Transcription] Emitted live_transcription_result with text: '{}', is_final: {}", cleaned_text, is_final);
                        if is_final {
                            last_text = cleaned_text;
                            segment_start_time = end_time;
                        }
                    }
                }
                CommandEvent::Stderr(line) => {
                    error!("[Live Transcription][whisper-stream stderr]: {}", String::from_utf8_lossy(&line));
                }
                CommandEvent::Error(err) => {
                    error!("[Live Transcription][whisper-stream error]: {}", err);
                }
                CommandEvent::Terminated(payload) => {
                    info!("[Live Transcription] Whisper-stream process terminated with payload: {:?}", payload);
                }
                _ => {}
            }
        }
        info!("[Live Transcription] Stopped listening to whisper-stream sidecar.");
    });

    Ok(true)
}

#[tauri::command]
pub async fn stop_live_transcription(
    app_handle: AppHandle,
    state: tauri::State<'_, LiveTranscriptionState>
) -> Result<bool, String> {
    info!("[Live Transcription] Stop command received.");
    if !state.is_running.load(std::sync::atomic::Ordering::SeqCst) {
        return Err("Live transcription is not running.".to_string());
    }

    if let Some(child) = state.whisper_child.lock().await.take() {
        child.kill().map_err(|e| e.to_string())?;
    }

    state.is_running.store(false, std::sync::atomic::Ordering::SeqCst);

    let active_doc_path_opt = state.active_document_path.lock().await.take();
    let project_uuid_opt = state.project_uuid.lock().await.take();
    let project_base_dir_opt = state.project_base_dir.lock().await.take();

    if let (Some(active_doc_path_str), Some(project_uuid), Some(project_base_dir)) = (active_doc_path_opt, project_uuid_opt, project_base_dir_opt) {
        info!("[Live Transcription] Processing saved audio for doc: {}", active_doc_path_str);

        let active_doc_path = PathBuf::from(&active_doc_path_str);
        let attachments_dir = active_doc_path.parent().unwrap().join("attachments");

        if attachments_dir.exists() && attachments_dir.is_dir() {
            let mut audio_files: Vec<String> = Vec::new();
            match fs::read_dir(&attachments_dir) {
                Ok(entries) => {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            let path = entry.path();
                            if path.is_file() {
                                if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
                                    if extension.eq_ignore_ascii_case("wav") {
                                         audio_files.push(path.to_string_lossy().to_string());
                                    }
                                }
                            }
                        }
                    }
                },
                Err(e) => {
                    error!("[Live Transcription] Failed to read attachments directory: {}", e);
                }
            }

            if !audio_files.is_empty() {
                info!("[Live Transcription] Found {} audio files to attach.", audio_files.len());
                let relative_doc_path = match active_doc_path.strip_prefix(&project_base_dir) {
                    Ok(p) => p.to_string_lossy().replace("\\", "/"),
                    Err(_) => {
                        error!("[Live Transcription] Failed to create relative path for document.");
                        return Ok(true);
                    }
                };

                match db_handler::load_asset_metadata(&project_uuid, &relative_doc_path) {
                    Ok(Some(metadata_from_db)) => {
                        let mut custom_fields: Vec<serde_json::Value> = metadata_from_db.custom_fields_json
                            .as_deref()
                            .and_then(|json| serde_json::from_str(json).ok())
                            .unwrap_or_else(Vec::new);

                        let attachments_json_string = json!(audio_files).to_string();

                        if let Some(existing_field) = custom_fields.iter_mut().find(|f| f.get("key").and_then(|k| k.as_str()) == Some("attachments")) {
                            if let Some(obj) = existing_field.as_object_mut() {
                                obj.insert("value".to_string(), json!(attachments_json_string));
                            }
                        } else {
                            let new_field = json!({
                                "key": "attachments",
                                "value": attachments_json_string
                            });
                            custom_fields.push(new_field);
                        }

                        let updated_custom_fields_json_str = serde_json::to_string(&custom_fields).unwrap_or_else(|_| "[]".to_string());

                        let file_metadata = FileMetadata {
                            file_name: metadata_from_db.file_name,
                            file_path: metadata_from_db.file_path,
                            last_modified: Utc::now().to_rfc3339(),
                            title: metadata_from_db.title.unwrap_or_default(),
                            description: metadata_from_db.description.unwrap_or_default(),
                            summary: metadata_from_db.summary.unwrap_or_default(),
                            duration_seconds: metadata_from_db.duration_seconds,
                            width: metadata_from_db.width,
                            height: metadata_from_db.height,
                            frame_rate: metadata_from_db.frame_rate,
                            bit_rate: metadata_from_db.bit_rate,
                            audio_codec: metadata_from_db.audio_codec,
                            video_codec: metadata_from_db.video_codec,
                            created_at: metadata_from_db.creation_time,
                            original_import_path: metadata_from_db.original_import_path,
                            speaker_names: metadata_from_db.speaker_names_json.and_then(|s| serde_json::from_str(&s).ok()),
                            waveform_data: metadata_from_db.waveform_data,
                        };

                        if let Err(e) = db_handler::save_asset_metadata(&project_uuid, &file_metadata, &relative_doc_path, &metadata_from_db.asset_type, Some(&updated_custom_fields_json_str)) {
                            error!("[Live Transcription] Failed to update metadata with attachments: {}", e);
                        } else {
                            info!("[Live Transcription] Successfully updated metadata with attachments.");
                            if let Err(e) = app_handle.emit("metadata_updated", &active_doc_path_str) {
                                error!("Failed to emit metadata_updated event: {}", e);
                            }
                        }
                    },
                    Ok(None) => {
                        info!("[Live Transcription] No existing metadata for '{}'. Creating new entry.", relative_doc_path);
                        let file_metadata = FileMetadata {
                            file_name: active_doc_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                            file_path: active_doc_path_str.clone(),
                            last_modified: Utc::now().to_rfc3339(),
                            title: String::new(),
                            description: String::new(),
                            summary: String::new(),
                            duration_seconds: None,
                            width: None,
                            height: None,
                            frame_rate: None,
                            bit_rate: None,
                            audio_codec: None,
                            video_codec: None,
                            created_at: Some(Utc::now().to_rfc3339()),
                            original_import_path: None,
                            speaker_names: None,
                            waveform_data: None,
                        };

                        let attachments_json_string = json!(audio_files).to_string();
                        let custom_fields = vec![json!({
                            "key": "attachments",
                            "value": attachments_json_string
                        })];
                        let updated_custom_fields_json_str = serde_json::to_string(&custom_fields).unwrap_or_else(|_| "[]".to_string());

                        if let Err(e) = db_handler::save_asset_metadata(&project_uuid, &file_metadata, &relative_doc_path, "doc", Some(&updated_custom_fields_json_str)) {
                            error!("[Live Transcription] Failed to save new metadata with attachments: {}", e);
                        } else {
                            info!("[Live Transcription] Successfully saved new metadata with attachments.");
                            if let Err(e) = app_handle.emit("metadata_updated", &active_doc_path_str) {
                                error!("Failed to emit metadata_updated event: {}", e);
                            }
                        }
                    },
                    Err(e) => error!("[Live Transcription] Failed to get metadata for document '{}': {}", relative_doc_path, e),
                }
            }
        }
    } else {
        warn!("[Live Transcription] Could not update attachments metadata because path or project info was missing from state.");
    }

    Ok(true)
}
