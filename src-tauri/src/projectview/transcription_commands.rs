// src-tauri/src/projectview/transcription_commands.rs

use super::shared_types::*;
use super::shared_utils::*;
use crate::welcome::config::{CommandError, read_config, get_default_download_location};
use log::{debug, error, info, warn};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_shell::ShellExt;
use serde_json::Value as JsonValue;
use crate::projectview::utils::{get_ffmpeg_path, get_ffprobe_path};
use serde::Deserialize;
use chrono::Utc;
use uuid::Uuid;
use crate::projectview::db_handler;
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
use tokio::time::{Duration};
use quick_xml;
use regex::Regex;
use crate::welcome::python_env::{get_python_command, get_env_path};
use crate::transcription::{TranscriptionEngine, TranscriptionOptions};
use crate::transcription::whisper_cpp::WhisperCppEngine;
use crate::transcription::faster_whisper::FasterWhisperEngine;

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
        json!(40),
        json!(120),
        json!(120),
        json!(520)
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
                "colWidths": col_widths_json.clone(),
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
    let project_xml_path = project_base_dir.join(format!("{}.harvey", project_base_dir_name));
    let project_xml_path_str = project_xml_path.to_string_lossy().to_string();

    if !project_xml_path.exists() {
        return Err(CommandError::from(format!("Project Manifest not found: {:?}", project_xml_path)));
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
    let mut project_data: ProjectXml = serde_json::from_str(&xml_content)?;

    let original_entry = project_data.find_media(&original_media_identifier).cloned();
    let (original_speakers, _original_transcripts, is_video_source) = match original_entry {
        Some(entry) => {
            let path_lower = entry.relative_path.to_lowercase();
            let is_video = path_lower.contains("/videos/") || 
                           path_lower.ends_with(".mp4") || 
                           path_lower.ends_with(".mov") || 
                           path_lower.ends_with(".avi");
            (entry.speakers, entry.transcripts, is_video)
        },
        None => {
            warn!("[Trim Backend] Original XML entry '{}' not found when trying to copy metadata.", original_media_identifier);
            (None, Vec::new(), false)
        }
    };

    let target_dir = if is_video_source { VIDEOS_DIR } else { AUDIOS_DIR };

    let new_relative_path_for_xml = Path::new(HARVEY_FILES_DIR)
        .join(target_dir)
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

    if project_data.find_media(&new_media_entry.name).is_none() {
        info!("[Trim Backend] Adding new media entry to XML: {}", new_media_entry.name);
        if is_video_source {
            project_data.video_files.files.push(new_media_entry);
            project_data.video_files.files.sort_by(|a,b| a.name.cmp(&b.name));
        } else {
            project_data.audio_files.files.push(new_media_entry);
            project_data.audio_files.files.sort_by(|a,b| a.name.cmp(&b.name));
        }
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
            language_code: None,
            properties: None,
            file_type: String::new(),
            thumbnail: None,
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
        trimmed_media_file_metadata_with_waveform.file_type = asset_type.clone();

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
            None, // initial_prompt
            None, // hotwords
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
    media_relative_path: String,
    count: usize,
    names: Vec<String>,
    translated_names: Option<Vec<String>>,
}

#[tauri::command]
pub async fn save_speaker_config(payload: SaveSpeakerConfigPayload) -> Result<(), CommandError> {
    info!("[Backend SaveSpeakers] Request: Project='{}', MediaRelPath='{}', Count={}, Names={:?}, TranslatedNames={:?}",
        payload.project_xml_path, payload.media_relative_path, payload.count, payload.names, payload.translated_names);

    let xml_path = PathBuf::from(&payload.project_xml_path);
    if !xml_path.exists() || !xml_path.is_file() {
        return Err(CommandError::from(format!("Project file not found: {}", payload.project_xml_path)));
    }

    let xml_content = fs::read_to_string(&xml_path)?;
    let mut project_data: ProjectXml = serde_json::from_str(&xml_content)?;
    let mut found_and_updated = false;

    if let Some(media_file) = project_data.find_media_by_relative_path_mut(&payload.media_relative_path) {
        info!("[Backend SaveSpeakers] Found entry '{}'. Updating speakers.", payload.media_relative_path);

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
        return Err(CommandError::from(format!("Media Rel Path '{}' not found in XML.", payload.media_relative_path)));
    }

    save_project_xml(&xml_path, &project_data)?;
    info!("[Backend SaveSpeakers] Success for '{}'.", payload.media_relative_path);
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
    use super::shared_utils;
    info!("[Backend Save Full Transcript JSON] Transcript Path: {}", transcript_path);
    info!("[Backend Save Full Transcript JSON] Project XML Path: {}", project_xml_path);
    let normalized_transcript_path_buf = shared_utils::normalize_path_for_comparison(&PathBuf::from(&transcript_path));
    let normalized_project_xml_path_buf = shared_utils::normalize_path_for_comparison(&PathBuf::from(&project_xml_path));

    let project_base_dir = normalized_project_xml_path_buf.parent().ok_or_else(|| CommandError::from("Could not get project base dir from XML path"))?;

    if let Some(parent) = normalized_transcript_path_buf.parent() {
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

    let file = File::create(&normalized_transcript_path_buf)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(lexical_table_json_string.as_bytes())
        .map_err(|e| CommandError::from(format!("Failed to write transcript JSON: {}", e)))?;
    writer.flush()?; 
    info!("[Backend Save Full Transcript JSON] Saved Lexical Table JSON to disk: {}", normalized_transcript_path_buf.display());


    let transcript_filename = normalized_transcript_path_buf.file_name().and_then(|n| n.to_str()).ok_or_else(|| CommandError::from("Could not get transcript filename"))?.to_string();

    let (_item_type, media_identifier_opt, transcript_relative_path_buf) = shared_utils::get_item_details(&normalized_transcript_path_buf, project_base_dir)?;
    let media_identifier = media_identifier_opt.ok_or_else(|| CommandError::from(format!("Could not determine media identifier for transcript path: {}", transcript_path)))?;
    let transcript_relative_path = transcript_relative_path_buf.to_string_lossy().replace("\\", "/");
    let stem_rel_path = transcript_relative_path_buf.parent().and_then(|p| p.parent()).map(|p| p.to_string_lossy().replace("\\", "/"))
        .unwrap_or_else(|| String::new());

    info!("[Backend Save Full Transcript JSON] Media ID: '{}', Transcript Filename: '{}', Transcript Rel Path: '{}', Stem Rel Path: '{}'", media_identifier, transcript_filename, transcript_relative_path, stem_rel_path);

    let xml_content = fs::read_to_string(&normalized_project_xml_path_buf)?;
    let mut project_data: ProjectXml = serde_json::from_str(&xml_content)?;
    let mut found_media = false;

    if let Some(media_entry) = project_data.find_media_by_stem_dir_mut(&stem_rel_path) {
        found_media = true;
        debug!("[Backend Save Full Transcript JSON] Found media entry for stem '{}' in XML.", stem_rel_path);

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

    // Get media entry relative path before closing the mutable borrow if needed, 
    // but we need it for metadata lookup.
    let media_entry_relative_path = project_data.find_media(&media_identifier).unwrap().relative_path.clone();
    let project_uuid_for_db = project_data.project_uuid.clone();

    save_project_xml(&normalized_project_xml_path_buf, &project_data)?;
    info!("[Backend Save Full Transcript JSON] Project XML updated.");

    // Update asset_metadata for the saved transcript
    let media_metadata = db_handler::load_asset_metadata(&project_uuid_for_db, &media_entry_relative_path)?;
    let file_type = if let Some(meta) = media_metadata {
        let is_video = meta.asset_type == "video" || meta.file_type.as_deref() == Some("video");
        if is_video {
            "video-transcript".to_string()
        } else {
            "audio-transcript".to_string()
        }
    } else {
        "audio-transcript".to_string()
    };

    let transcript_abs_path = project_base_dir.join(&transcript_relative_path);
    let transcript_metadata = FileMetadata {
        file_name: transcript_filename.clone(),
        file_path: transcript_abs_path.to_string_lossy().to_string(),
        last_modified: chrono::Utc::now().to_rfc3339(),
        file_type: file_type.clone(),
        ..Default::default()
    };

    db_handler::save_asset_metadata(
        &project_uuid_for_db,
        &transcript_metadata,
        &transcript_relative_path,
        &file_type,
        None
    )?;

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
    let media_path = PathBuf::from(media_path_str.replace("/", &std::path::MAIN_SEPARATOR.to_string()));

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
    let temp_whisper_output_base_orig = transcripts_dir.join(format!("whisper_temp_{}_orig", job_id));
    let temp_whisper_output_base_orig_str = temp_whisper_output_base_orig.to_string_lossy().to_string();
    let expected_whisper_temp_json_path_orig = temp_whisper_output_base_orig.with_extension("json");

    let mut final_transcript_path_orig = transcripts_dir.join(format!("{}_1.json", media_filename_stem));
    let mut counter = 2;
    while final_transcript_path_orig.exists() {
        final_transcript_path_orig = transcripts_dir.join(format!("{}_{}.json", media_filename_stem, counter));
        counter += 1;
    }
    
    let temp_rttm_base = transcripts_dir.join(format!("rttm_temp_{}", job_id));
    let expected_rttm_temp_path = temp_rttm_base.with_extension("rttm");

    debug!("[prepare_output_paths][{}] Orig Temp Whisper Base: '{}', Orig Whisper JSON (temp): '{}', RTTM (temp): '{}', Orig Final JSON: '{}'",
        job_id, temp_whisper_output_base_orig_str, expected_whisper_temp_json_path_orig.display(), expected_rttm_temp_path.display(), final_transcript_path_orig.display());

    // --- Paths for translated transcript (if requested) ---
    let mut temp_whisper_output_base_en_str_opt: Option<String> = None;
    let mut expected_whisper_temp_json_path_en_opt: Option<PathBuf> = None;
    let mut final_transcript_path_en_opt: Option<PathBuf> = None;

    if translate_to_english {
        let temp_whisper_output_base_en = transcripts_dir.join(format!("whisper_temp_{}_en", job_id));
        temp_whisper_output_base_en_str_opt = Some(temp_whisper_output_base_en.to_string_lossy().to_string());
        expected_whisper_temp_json_path_en_opt = Some(temp_whisper_output_base_en.with_extension("json"));

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
        temp_whisper_output_base_orig_str,
        expected_whisper_temp_json_path_orig,
        expected_rttm_temp_path,
        final_transcript_path_orig,
        temp_whisper_output_base_en_str_opt,
        expected_whisper_temp_json_path_en_opt,
        final_transcript_path_en_opt,
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
        // Strip trailing colon if present (e.g. "SPEAKER_00:" -> "SPEAKER_00")
        let speaker_id = original_speaker.trim_end_matches(':');

        let number_part_opt = if speaker_id.starts_with("SPEAKER_") {
            speaker_id.get("SPEAKER_".len()..)
                .and_then(|num_str| num_str.parse::<usize>().ok())
        } else if speaker_id.starts_with("speaker_") {
            speaker_id.get("speaker_".len()..)
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
                      user_name_index, speaker_id, user_names.len());
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
    #[serde(default)]
    transcription_engine: Option<String>,
    #[serde(default)]
    initial_prompt: Option<String>,
    #[serde(default)]
    hotwords: Option<String>,
}

#[allow(dead_code)]
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
    let job_id = Uuid::new_v4().to_string();
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

    let whisper_model_path_str = resolve_model_path_cmd(&payload.model_name, &job_id, payload.transcription_engine.as_deref())?;

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

    // Clean up hotwords (strip whitespace around commas)
    let clean_hotwords = payload.hotwords.as_ref().map(|vocab| {
        vocab.split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>()
            .join(", ")
    });

    let mut original_segments = match execute_transcription_pass(
        &app_handle_clone,
        &wav_media_path.to_string_lossy(),
        &whisper_model_path_str,
        &payload.language_code.clone().unwrap_or_else(|| "auto".to_string()),
        &job_id,
        &temp_whisper_output_base_orig_str,
        &expected_whisper_temp_json_path_orig,
        &final_transcript_path_orig, 
        payload.num_speakers,
        &expected_rttm_temp_path,
        false, // is_translation_pass
        None, // Delay speaker mapping
        &media_filename_for_progress,
        cancel_flag.clone(),
        payload.transcription_engine.clone(),
        payload.initial_prompt.clone(),
        clean_hotwords.clone(),
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

    // --- Second Pass: English Translation (if requested) ---
    if payload.translate_to_english {
        // ... (pre-pass check remains same) ...
        if cancel_flag.load(AtomicOrdering::Relaxed) {
            warn!("[Transcribe Command][{}] Cancelled before translation pass.", job_id);
            if wav_media_path.to_string_lossy() != payload.media_path_str {
                let _ = fs::remove_file(&wav_media_path).map_err(|e| warn!("[Transcribe Command][{}] Failed to clean up temp WAV during pre-translation cancel: {:?}", job_id, e));
            }
            let _ = app_handle.emit("custom_transcription_job_completed", TranscriptionJobCompletedPayload {
                job_id: job_id.clone(),
                status: "cancelled".to_string(),
                job_finished_path: payload.media_path_str.clone(),
                transcript_file_path: None, // No original saved yet in this new flow
                translated_transcript_file_path: None,
                error_message: Some("Transcription cancelled by user before translation pass.".to_string()),
            });
            return Err(CommandError::from("Transcription cancelled before translation pass."));
        }

        if let (Some(base_en_str), Some(json_path_en), Some(final_path_en_pb)) = (
            temp_whisper_output_base_en_str_opt,
            expected_whisper_temp_json_path_en_opt,
            final_transcript_path_en_opt.clone(),
        ) {
            emit_progress_cmd(&app_handle_clone, &job_id, 60.0, &format!("Translating {}...", media_filename_for_progress))?;

            let source_language_for_translation = payload.language_code.clone().unwrap_or_else(|| "auto".to_string());

            let translation_result = execute_transcription_pass(
                &app_handle_clone,
                &wav_media_path.to_string_lossy(),
                &whisper_model_path_str,
                &source_language_for_translation,
                &job_id,
                &base_en_str,
                &json_path_en,
                &final_path_en_pb,
                0, // No diarization for translation pass
                &PathBuf::new(), 
                true, // is_translation_pass
                None, // Delay speaker mapping
                &media_filename_for_progress,
                cancel_flag.clone(),
                payload.transcription_engine.clone(),
                payload.initial_prompt.clone(),
                clean_hotwords.clone(),
            ).await;

            let mut translated_segments = match translation_result {
                Ok(segments) => segments,
                Err(e) => {
                    let error_message = e.to_string();
                    warn!("[Transcribe Command][{}] Translation pass failed: {}", job_id, error_message);
                    if wav_media_path.to_string_lossy() != payload.media_path_str {
                        let _ = fs::remove_file(&wav_media_path).map_err(|e_del| warn!("[Transcribe Command][{}] Failed to delete temp WAV file during translation pass error: {:?}", job_id, e_del));
                    }
                    let _ = fs::remove_file(&json_path_en).map_err(|e_del| warn!("[Transcribe Command][{}] Failed to delete temp EN Whisper JSON during translation pass error: {:?}", job_id, e_del));

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
                            error_message: Some(format!("Translation pass failed: {}", error_message)),
                        });
                    }
                    return Err(CommandError::from(format!("Translation pass failed: {}", error_message)));
                }
            };

            info!("[Transcribe Command][{}] Aligning speakers for translated segments using generic original pass IDs...", job_id);
            align_speakers_to_translated_segments(&original_segments, &mut translated_segments, &job_id);

            // --- Apply Speaker Names to Translated Transcript ---
            // Fix: If translated_speaker_names provided, use them.
            if let Some(ref translated_names) = payload.translated_speaker_names {
                let contains_actual_names = translated_names.iter().any(|name| !name.trim().is_empty());
                if !translated_names.is_empty() && contains_actual_names {
                    info!("[Transcribe Command][{}] Applying user-defined translated speaker names to translated segments.", job_id);
                    map_speaker_ids_to_names(&mut translated_segments, translated_names);
                } else {
                    info!("[Transcribe Command][{}] Translated speaker names provided but effectively empty. Falling back to original names for translation pass.", job_id);
                    map_speaker_ids_to_names(&mut translated_segments, &payload.speaker_names);
                }
            } else {
                info!("[Transcribe Command][{}] No translated speaker names provided. Falling back to original names for translation pass.", job_id);
                map_speaker_ids_to_names(&mut translated_segments, &payload.speaker_names);
            }

            emit_progress_cmd(&app_handle_clone, &job_id, 90.0, &format!("Saving translation for {}...", media_filename_for_progress))?;
            let lexical_json_en = create_lexical_table_from_segments(&translated_segments);
            let lexical_json_en_str = serde_json::to_string_pretty(&lexical_json_en)
                .map_err(|e| CommandError::from(format!("Failed to serialize translated Lexical Table JSON: {}", e)))?;

            save_transcript_json(
                payload.project_xml_path.clone(),
                final_path_en_pb.to_string_lossy().to_string(),
                lexical_json_en_str,
                Some("en".to_string()),
            ).await?;
            info!("[Transcribe Command][{}] Translated transcript saved.", job_id);

            if json_path_en.exists() { let _ = fs::remove_file(&json_path_en); }
        }
    }

    // --- Finalize Original Transcript ---
    // Apply Original Speaker Names
    map_speaker_ids_to_names(&mut original_segments, &payload.speaker_names);

    emit_progress_cmd(&app_handle_clone, &job_id, 95.0, &format!("Saving original transcript for {}...", media_filename_for_progress))?;
    let lexical_json_orig = create_lexical_table_from_segments(&original_segments);
    let lexical_json_orig_str = serde_json::to_string_pretty(&lexical_json_orig)
        .map_err(|e| CommandError::from(format!("Failed to serialize original Lexical Table JSON: {}", e)))?;

    let original_lang_code_to_save = match payload.language_code.as_deref() {
        None | Some("") | Some("auto") => Some("auto".to_string()),
        Some(lang) => Some(lang.to_string()),
    };

    save_transcript_json(
        payload.project_xml_path.clone(),
        final_transcript_path_orig.to_string_lossy().to_string(),
        lexical_json_orig_str,
        original_lang_code_to_save,
    ).await?;

    // --- Database Parameter Sync (same as before) ---
    let normalized_xml_path = crate::projectview::shared_utils::normalize_path_for_comparison(&PathBuf::from(&payload.project_xml_path));
    if let Some(project_base_dir) = normalized_xml_path.parent() {
         let media_path_buf = PathBuf::from(&payload.media_path_str);
         if let Ok(relative_path) = media_path_buf.strip_prefix(project_base_dir) {
             let asset_relative_path = relative_path.to_string_lossy().replace("\\", "/");
             if let Ok(project_data) = crate::projectview::core_commands::load_project_data(payload.project_xml_path.clone()).await {
                 let project_uuid = project_data.project_uuid;
                 let mut speaker_names = None;
                 if let Some(media_entry) = project_data.files.iter().find(|f| f.path == payload.media_path_str) {
                     if let Some(speakers) = &media_entry.speakers {
                         speaker_names = Some(&speakers.names);
                     }
                 }
                 let _ = db_handler::save_media_transcript_data(
                     &project_uuid, &asset_relative_path, Some(&payload.media_path_str),
                     speaker_names, payload.language_code.as_deref(), payload.initial_prompt.as_deref(), clean_hotwords.as_deref()
                 );
             }
         }
    }

    emit_progress_cmd(&app_handle_clone, &job_id, 100.0, &format!("Successfully processed {}.", media_filename_for_progress))?;
    
    let completion_payload = TranscriptionJobCompletedPayload {
        job_id: job_id.clone(),
        status: "done".to_string(),
        job_finished_path: payload.media_path_str.clone(),
        transcript_file_path: Some(final_transcript_path_orig.to_string_lossy().into_owned()),
        translated_transcript_file_path: final_transcript_path_en_for_payload.map(|p| p.to_string_lossy().into_owned()),
        error_message: None,
    };
    let _ = app_handle.emit("custom_transcription_job_completed", completion_payload);

    if expected_whisper_temp_json_path_orig.exists() { let _ = fs::remove_file(&expected_whisper_temp_json_path_orig); }
    if expected_rttm_temp_path.exists() { let _ = fs::remove_file(&expected_rttm_temp_path); }
    if wav_media_path.to_string_lossy() != payload.media_path_str { let _ = fs::remove_file(&wav_media_path); }

    Ok(TranscriptionInitiatedPayload { job_id })
}

// --- Helper Functions ---

fn align_speakers_to_translated_segments(
    original_segments: &[TranscriptSegment], // Should have SPEAKER_XX IDs
    translated_segments: &mut Vec<TranscriptSegment>, // Should have "Unknown"
    job_id: &str,
) {
    if original_segments.is_empty() || translated_segments.is_empty() { return; }
    info!("[Align Speakers][{}] Aligning {} translated segments with {} original segments.", job_id, translated_segments.len(), original_segments.len());

    for t_seg in translated_segments.iter_mut() {
        let t_start = t_seg.start_time;
        let t_end = t_seg.end_time;
        if t_start > t_end { continue; }
        let t_mid = t_start + (t_end - t_start) / 2.0;

        let mut best_match_speaker = "Unknown".to_string();
        let mut max_overlap = 0.0f64;
        let mut best_overlap_priority = f64::MAX;

        for o_seg in original_segments {
            let o_start = o_seg.start_time;
            let o_end = o_seg.end_time;
            if o_start > o_end { continue; }

            let overlap_start = t_start.max(o_start);
            let overlap_end = t_end.min(o_end);
            let current_overlap = (overlap_end - overlap_start).max(0.0);

            if current_overlap > 0.0 {
                let o_mid = o_start + (o_end - o_start) / 2.0;
                let current_priority = if t_mid >= o_start && t_mid < o_end { 0.0 } else { (t_mid - o_mid).abs() };

                if current_overlap > max_overlap || (current_overlap == max_overlap && current_priority < best_overlap_priority) {
                    max_overlap = current_overlap;
                    best_overlap_priority = current_priority;
                    best_match_speaker = o_seg.speaker.clone();
                }
            }
        }
        if max_overlap > 0.0 {
            debug!("[Align Speakers][{}] Assigning speaker '{}' to translated segment {:.3}-{:.3} (Max Overlap: {:.3})",
                job_id, best_match_speaker, t_start, t_end, max_overlap);
            t_seg.speaker = best_match_speaker;
        }
    }
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



// --- Struct specific to parsing RTTM output (can be kept local) ---
#[derive(Debug, Clone)]
struct RttmRecord {
    start_time: f64,
    duration: f64,
    speaker_id: String,
}


pub(crate) fn resolve_model_path_cmd(
    model_name: &str,
    job_id: &str, // Kept for logging consistency, though not strictly needed by logic
    engine: Option<&str>,
) -> Result<String, CommandError> {
    let config = read_config()?; // This is synchronous
    let base_model_dir_str = if !config.download_location.trim().is_empty() {
        config.download_location
    } else {
        get_default_download_location()? // This is synchronous
    };

    if engine == Some("faster-whisper") {
        let sub_dir = PathBuf::from("transcription").join("faster-whisper");
        // Matches the folder name constructed in python script and expected by delete_model
        let folder_name = format!("models--{}", model_name.replace('/', "--"));
        let model_dir_path = PathBuf::from(&base_model_dir_str).join(sub_dir).join(&folder_name);

        if !model_dir_path.exists() || !model_dir_path.is_dir() {
             let e_msg = format!("Faster-whisper model directory not found: '{}'. Please download the model first.", model_dir_path.display());
             error!("[Transcription CMD][{}] Error resolving model path: {}", job_id, e_msg);
             return Err(CommandError::from(e_msg));
        }
        // faster-whisper takes the directory path
        Ok(model_dir_path.to_string_lossy().to_string())
    } else {
        // New directory structure: transcription/whisper-cpp/model_name
        let sub_dir = PathBuf::from("transcription").join("whisper-cpp");
        let model_dir_path = PathBuf::from(&base_model_dir_str).join(sub_dir).join(model_name);

        // Fallback logic
        let model_dir_path = if model_dir_path.exists() {
            model_dir_path
        } else {
            let legacy_model_dir_path = PathBuf::from(&base_model_dir_str).join(model_name);
            if legacy_model_dir_path.exists() {
                 legacy_model_dir_path
            } else {
                 // If neither exists, default to new structure so error message reflects preferred path
                 model_dir_path
            }
        };

        if !model_dir_path.exists() || !model_dir_path.is_dir() {
            let e_msg = format!("Model directory not found: '{}'. Please download the model first.", model_dir_path.display());
            error!("[Transcription CMD][{}] Error resolving model path: {}", job_id, e_msg);
            return Err(CommandError::from(e_msg));
        }
        // Call the adapted find_model_file_cmd
        let model_file_path = find_model_file_cmd(&model_dir_path)?;
        Ok(model_file_path.to_string_lossy().to_string())
    }
}

// --- START: Adapted Helper Functions for execute_transcription_pass ---





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

    let script_path = app_handle
        .path()
        .resolve("scripts/run_diarization.py", tauri::path::BaseDirectory::Resource)
        .map_err(|e| CommandError::from(e.to_string()))?;

    let token = get_hf_token(app_handle).map_err(|e| CommandError::from(e.to_string()))?;

    let mut args = vec![
        script_path.to_string_lossy().to_string(),
        media_path.to_string(),
        num_speakers.to_string(),
        token,
    ];

    // Read advanced configuration for Diarization
    if let Ok(config) = read_config() {
        if let Some(adv) = config.advanced_translation {
            if let Some(device) = adv.diarization_device {
                args.push("--device".to_string());
                args.push(device);
            }
            if let Some(threads) = adv.diarization_threads {
                args.push("--threads".to_string());
                args.push(threads.to_string());
            }
        }
    }

    let mut command = get_python_command(app_handle).map_err(|e| CommandError::from(e.to_string()))?;
    
    if let Ok(hf_home) = crate::welcome::diarization::get_diarization_hub_path(app_handle) {
        command = command.env("HF_HOME", hf_home.to_string_lossy().to_string());
    }

    command = command.args(args);

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
    wav_media_path_str: &str,
    model_path: &str,
    language_code: &str,
    job_id: &str,
    _output_base_for_whisper: &str,
    expected_whisper_json_output_path: &PathBuf,
    _final_transcript_destination_path: &PathBuf, // Not used directly here anymore
    num_speakers: usize,
    expected_rttm_output_path: &PathBuf,
    is_translation_pass: bool,
    speaker_names: Option<&[String]>,
    media_filename_for_progress: &str,
    cancel_flag: Arc<AtomicBool>,
    transcription_engine: Option<String>,
    initial_prompt: Option<String>,
    hotwords: Option<String>,
) -> Result<Vec<TranscriptSegment>, CommandError> {
    info!("[Exec Pass][{}] DEBUG: Entered. Lang: {}, Translate: {}, NumSpeakers: {}, Engine: {:?}", job_id, language_code, is_translation_pass, num_speakers, transcription_engine);

    let output_dir = expected_whisper_json_output_path.parent()
        .ok_or_else(|| CommandError::from("Invalid output path"))?
        .to_path_buf();

    let options = TranscriptionOptions {
        language_code: Some(language_code.to_string()),
        model_path: model_path.to_string(),
        output_dir: output_dir,
        translate: is_translation_pass,
        initial_prompt,
        hotwords,
    };

    let engine: Box<dyn TranscriptionEngine> = if transcription_engine.as_deref() == Some("faster-whisper") {
        Box::new(FasterWhisperEngine::new(app_handle.clone()))
    } else {
        Box::new(WhisperCppEngine::new(app_handle.clone()))
    };
    
    info!("[Exec Pass][{}] Calling TranscriptionEngine...", job_id);
    let mut segments = engine.transcribe(
        Path::new(wav_media_path_str),
        &options,
        job_id,
        cancel_flag.clone()
    ).await?;

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

    if let Some(names) = speaker_names {
        map_speaker_ids_to_names(&mut segments, names);
    }

    info!("[Exec Pass][{}] Pass complete. Segments: {}.", job_id, segments.len());
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
    let srt_path = PathBuf::from(&srt_path_str);
    if srt_path.extension().and_then(|e| e.to_str()) != Some("srt") {
        return Err(CommandError::from("Not an SRT file.".to_string()));
    }

    let srt_content = fs::read_to_string(&srt_path)
        .map_err(|e| CommandError::from(format!("Failed to read SRT file: {}", e)))?;

    let mut vtt_content = String::from("WEBVTT\n\n");
    // Add STYLE block for strikethrough support in modern players/browsers
    vtt_content.push_str("STYLE\n");
    vtt_content.push_str("::cue(s) { text-decoration: line-through; }\n");
    vtt_content.push_str("::cue(strike) { text-decoration: line-through; }\n");
    vtt_content.push_str("::cue { color: #FFFFFF; }\n\n");

    // Simple SRT to VTT conversion: 
    // 1. Prepend WEBVTT
    // 2. Change time separator from ',' to '.'
    
    for line in srt_content.lines() {
        if line.contains(" --> ") {
            // Replace ',' with '.' in timecode line
            vtt_content.push_str(&line.replace(',', "."));
        } else {
            vtt_content.push_str(line);
        }
        vtt_content.push('\n');
    }

    Ok(vtt_content)
}

fn convert_ass_time_to_vtt(t: &str) -> String {
    let parts: Vec<&str> = t.split('.').collect();
    if parts.len() >= 2 {
        let hms = parts[0];
        let cs = parts[1]; // centiseconds
        let hms_parts: Vec<&str> = hms.split(':').collect();
        let normalized_hms = if hms_parts.len() == 3 {
            format!("{:0>2}:{:0>2}:{:0>2}", hms_parts[0], hms_parts[1], hms_parts[2])
        } else if hms_parts.len() == 2 {
            format!("00:{:0>2}:{:0>2}", hms_parts[0], hms_parts[1])
        } else {
             format!("00:00:{:0>2}", hms)
        };
        // VTT uses milliseconds (3 digits), ASS often has 2 (centiseconds).
        // If cs is "55", that's 550ms.
        let ms_str = if cs.len() == 1 {
            format!("{}00", cs)
        } else if cs.len() == 2 {
             format!("{}0", cs)
        } else {
             cs.chars().take(3).collect::<String>()
        };
        format!("{}.{}", normalized_hms, ms_str)
    } else {
        t.to_string()
    }
}

// Helper to convert ASS color (&HBBGGRR or &HAABBGGRR) to CSS Hex (#RRGGBB)
// Ignores Alpha for now or assumes opaque if simple hex.
fn ass_color_to_css(ass_color: &str) -> Option<String> {
    // Strip &H (case insensitive) and optional trailing &
    let clean = ass_color.trim().replace("&H", "").replace("&h", "").replace("&", "");
    
    // Parse hex string
    if let Ok(val) = u32::from_str_radix(&clean, 16) {
        // ASS format: AABBGGRR (or just BBGGRR)
        // We need RGB.
        let (r, g, b) = if clean.len() > 6 {
            // Has Alpha: AA BB GG RR
            let r = val & 0xFF;
            let g = (val >> 8) & 0xFF;
            let b = (val >> 16) & 0xFF;
            // let a = (val >> 24) & 0xFF; // ASS Alpha: 00=Opaque, FF=Transparent. CSS is opposite.
            (r, g, b)
        } else {
            // No Alpha: BB GG RR
            let r = val & 0xFF;
            let g = (val >> 8) & 0xFF;
            let b = (val >> 16) & 0xFF;
            (r, g, b)
        };
        Some(format!("#{:02X}{:02X}{:02X}", r, g, b))
    } else {
        None
    }
}

#[derive(Debug, Default, Clone)]
struct AssStyle {
    name: String,
    primary_colour: Option<String>,
    bold: bool,
    italic: bool,
    underline: bool,
    strike_out: bool,
}

#[tauri::command]
pub async fn convert_ass_to_vtt_command(ass_path_str: String) -> Result<String, CommandError> {
    info!("[convert_ass_to_vtt_command] Converting ASS with robust state machine: {}", ass_path_str);
    let ass_path = PathBuf::from(&ass_path_str);
    
    let ass_content = fs::read_to_string(&ass_path)
        .map_err(|e| CommandError::from(format!("Failed to read ASS file: {}", e)))?;

    // --- Data Structures ---
    #[derive(Clone)]
    struct DialogueEvent {
        start: String,
        end: String,
        style: String,
        text: String,
    }

    let mut styles: std::collections::HashMap<String, AssStyle> = std::collections::HashMap::new();
    let mut events: Vec<DialogueEvent> = Vec::new();
    // Set of unique hex colors found in inline tags (CSS format #RRGGBB)
    let mut inline_colors: std::collections::HashSet<String> = std::collections::HashSet::new();

    let mut section = "";
    let mut format_cols_styles: Vec<String> = Vec::new();
    let mut format_cols_events: Vec<String> = Vec::new();

    // Regexes for parsing
    // Matches { ... } blocks
    let re_tag_block = Regex::new(r"\{.*?\}").unwrap();
    // Matches specific commands within a block
    let re_bold = Regex::new(r"\\b(\d+)").unwrap(); // \b1, \b0, \b100
    let re_italic = Regex::new(r"\\i(\d)").unwrap(); // \i1, \i0
    let re_underline = Regex::new(r"\\u(\d)").unwrap(); // \u1, \u0
    let re_strike = Regex::new(r"\\s(\d)").unwrap(); // \s1, \s0
    // Matches \c&HBBGGRR& or \1c&HBBGGRR& etc. 
    // Capture group 1: Optional number (1-4)
    // Capture group 2: the hex code part (e.g. &HFFFFFF&)
    let re_color = Regex::new(r"\\([1-4])?c\s*(&[hH][0-9a-fA-F]+&?)").unwrap(); 

    // --- Pass 1: Parse File, Collect Styles & Events, Identify Inline Colors ---
    for line in ass_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed;
            continue;
        }

        if section == "[V4+ Styles]" || section == "[V4 Styles]" {
            if trimmed.starts_with("Format:") {
                let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
                if parts.len() == 2 {
                    format_cols_styles = parts[1].split(',').map(|s| s.trim().to_lowercase()).collect();
                }
            } else if trimmed.starts_with("Style:") {
                 let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
                 if parts.len() == 2 && !format_cols_styles.is_empty() {
                     let data_part = parts[1];
                     let col_values: Vec<&str> = data_part.split(',').map(|s| s.trim()).collect();
                     
                     if col_values.len() == format_cols_styles.len() {
                         let mut style = AssStyle::default();
                         if let Some(idx) = format_cols_styles.iter().position(|c| c == "name") {
                             style.name = col_values[idx].to_string();
                         }
                         if let Some(idx) = format_cols_styles.iter().position(|c| c == "primarycolour") {
                             style.primary_colour = ass_color_to_css(col_values[idx]);
                         }
                         if let Some(idx) = format_cols_styles.iter().position(|c| c == "bold") {
                             // ASS: -1 = true, 0 = false. Sometimes 1 = true.
                             style.bold = col_values[idx] == "-1" || col_values[idx] == "1";
                         }
                         if let Some(idx) = format_cols_styles.iter().position(|c| c == "italic") {
                             style.italic = col_values[idx] == "-1" || col_values[idx] == "1";
                         }
                         if let Some(idx) = format_cols_styles.iter().position(|c| c == "underline") {
                             style.underline = col_values[idx] == "-1" || col_values[idx] == "1";
                         }
                         if let Some(idx) = format_cols_styles.iter().position(|c| c == "strikeout") {
                             style.strike_out = col_values[idx] == "-1" || col_values[idx] == "1";
                         }
                         if !style.name.is_empty() {
                             styles.insert(style.name.clone(), style);
                         }
                     }
                 }
            }
        } else if section == "[Events]" {
            if trimmed.starts_with("Format:") {
                 let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
                if parts.len() == 2 {
                    format_cols_events = parts[1].split(',').map(|s| s.trim().to_lowercase()).collect();
                }
            } else if trimmed.starts_with("Dialogue:") {
                let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
                if parts.len() == 2 && !format_cols_events.is_empty() {
                    let data_part = parts[1];
                    let num_cols = format_cols_events.len();
                    let col_values: Vec<&str> = data_part.splitn(num_cols, ',').map(|s| s.trim()).collect();

                    if col_values.len() == num_cols {
                        let start_idx = format_cols_events.iter().position(|c| c == "start").unwrap_or(1);
                        let end_idx = format_cols_events.iter().position(|c| c == "end").unwrap_or(2);
                        let style_idx = format_cols_events.iter().position(|c| c == "style").unwrap_or(3);
                        let text_idx = format_cols_events.iter().position(|c| c == "text").unwrap_or(9);

                        if start_idx < col_values.len() && end_idx < col_values.len() && text_idx < col_values.len() {
                            let raw_text = col_values[text_idx].to_string();
                            
                            // Scan for inline colors to register them
                            for caps in re_color.captures_iter(&raw_text) {
                                // Group 1 is optional prefix, Group 2 is hex
                                let type_prefix = caps.get(1).map(|m| m.as_str());
                                let hex_match = caps.get(2);

                                if type_prefix.is_none() || type_prefix == Some("1") {
                                    if let Some(m) = hex_match {
                                        if let Some(css_hex) = ass_color_to_css(m.as_str()) {
                                            inline_colors.insert(css_hex);
                                        }
                                    }
                                }
                            }

                            events.push(DialogueEvent {
                                start: col_values[start_idx].to_string(),
                                end: col_values[end_idx].to_string(),
                                style: if style_idx < col_values.len() { col_values[style_idx].to_string() } else { "Default".to_string() },
                                text: raw_text,
                            });
                        }
                    }
                }
            }
        }
    }

    // --- Pass 2: Generate VTT Content ---
    let mut cue_lines = Vec::new();
    let mut used_classes = std::collections::HashMap::new();

    // 2. Events Processing
    for event in events {
        let vtt_start = convert_ass_time_to_vtt(&event.start);
        let vtt_end = convert_ass_time_to_vtt(&event.end);
        
        // Initial State based on the line's Style
        let base_style = styles.get(&event.style);
        let mut state_bold = base_style.map(|s| s.bold).unwrap_or(false);
        let mut state_italic = base_style.map(|s| s.italic).unwrap_or(false);
        let mut state_underline = base_style.map(|s| s.underline).unwrap_or(false);
        let mut state_strike = base_style.map(|s| s.strike_out).unwrap_or(false);
        let mut state_color: Option<String> = None; // None means use base style color

        // Split text by tags: { ... }
        // We use finding matches to iterate content
        let mut last_idx = 0;
        let mut processed_line = String::new();

        // Safe style name for the outer cue
        let _safe_style_name = event.style.replace(" ", "_").replace(".", "-");
        
        // We will build segments. Each segment of text needs to be wrapped according to CURRENT state.
        // Helper to append text with current wrappers
        let append_text = |out: &mut String, text: &str, bold: bool, italic: bool, underline: bool, strike: bool, color: &Option<String>, used_classes: &mut std::collections::HashMap<String, String>| {
            if text.is_empty() { return; }
            
            // Clean text escapes
            let clean = text.replace("\\h", "\u{00A0}")
                            .replace("\\N", "\n")
                            .replace("\\n", "\n");
            
            // Wrap
            let mut prefix = String::new();
            let mut suffix = String::new();

            // VTT classes for color and strike.
            if let Some(c) = color {
                let is_white = c.eq_ignore_ascii_case("#FFFFFF") || c.eq_ignore_ascii_case("#FFF") || c.eq_ignore_ascii_case("white");
                if !is_white {
                    let safe_suffix = c.trim_start_matches('#').chars()
                        .map(|ch| if ch.is_alphanumeric() { ch } else { '_' })
                        .collect::<String>();
                    let class_name = format!("c_{}", safe_suffix);
                    
                    prefix.push_str(&format!("<c.{}>", class_name));
                    suffix.insert_str(0, "</c>");
                    used_classes.insert(class_name, c.clone());
                }
            }

            if bold { prefix.push_str("<b>"); suffix.insert_str(0, "</b>"); }
            if italic { prefix.push_str("<i>"); suffix.insert_str(0, "</i>"); }
            if underline { prefix.push_str("<u>"); suffix.insert_str(0, "</u>"); }
            if strike { 
                prefix.push_str("<c.s>"); 
                suffix.insert_str(0, "</c>"); 
                used_classes.insert("s".to_string(), "line-through".to_string());
            }

            out.push_str(&prefix);
            out.push_str(&clean);
            out.push_str(&suffix);
        };

        for cap in re_tag_block.find_iter(&event.text) {
            // Text before the tag
            if cap.start() > last_idx {
                let text_segment = &event.text[last_idx..cap.start()];
                append_text(&mut processed_line, text_segment, state_bold, state_italic, state_underline, state_strike, &state_color, &mut used_classes);
            }

            // Process the tag content
            let tag_content = cap.as_str();
            
            // Update State
            // Bold
            if let Some(b_match) = re_bold.captures(tag_content) {
                if let Some(val_str) = b_match.get(1) {
                    if let Ok(val) = val_str.as_str().parse::<u32>() {
                        // \b0 = false, \b1 = true, \b100+ = true
                        state_bold = val >= 1; // Simplification: any weight >= 1 is bold
                    }
                }
            }
            // Italic
            if let Some(i_match) = re_italic.captures(tag_content) {
                if let Some(val_str) = i_match.get(1) {
                     state_italic = val_str.as_str() == "1";
                }
            }
            // Underline
            if let Some(u_match) = re_underline.captures(tag_content) {
                if let Some(val_str) = u_match.get(1) {
                     state_underline = val_str.as_str() == "1";
                }
            }
            // Strikethrough
            if let Some(s_match) = re_strike.captures(tag_content) {
                if let Some(val_str) = s_match.get(1) {
                     state_strike = val_str.as_str() == "1";
                }
            }
            // Color
            for c_match in re_color.captures_iter(tag_content) {
                // Group 1: Optional number (1, 2, 3, 4)
                // Group 2: The hex string
                let type_prefix = c_match.get(1).map(|m| m.as_str());
                let hex_str_match = c_match.get(2);

                // Only apply primary color (\c or \1c). Ignore \2c, \3c, \4c for text color state.
                if type_prefix.is_none() || type_prefix == Some("1") {
                    if let Some(hex_match) = hex_str_match {
                        if let Some(css) = ass_color_to_css(hex_match.as_str()) {
                            state_color = Some(css);
                        }
                    }
                }
            }
            // Reset Color (\r or \rDefault usually resets everything, but let's just assume \c w/o arg might reset? ASS is tricky. 
            // Often \r resets style. Let's handle \r roughly.)
            if tag_content.contains("\\r") {
                // Reset to base style state
                state_bold = base_style.map(|s| s.bold).unwrap_or(false);
                state_italic = base_style.map(|s| s.italic).unwrap_or(false);
                state_underline = base_style.map(|s| s.underline).unwrap_or(false);
                state_strike = base_style.map(|s| s.strike_out).unwrap_or(false);
                state_color = None;
            }

            last_idx = cap.end();
        }

        // Remaining text after last tag
        if last_idx < event.text.len() {
            let text_segment = &event.text[last_idx..];
            append_text(&mut processed_line, text_segment, state_bold, state_italic, state_underline, state_strike, &state_color, &mut used_classes);
        }

        cue_lines.push(format!("{} --> {}\n{}\n\n", vtt_start, vtt_end, processed_line));
    }

    let mut vtt_content = String::from("WEBVTT\n\n");

    if !used_classes.is_empty() {
        vtt_content.push_str("STYLE\n");
        let mut sorted_keys: Vec<_> = used_classes.keys().collect();
        sorted_keys.sort();

        for class in sorted_keys {
            if class == "s" {
                vtt_content.push_str("::cue(.s) { text-decoration: line-through; }\n");
            } else if class.starts_with("c_") {
                if let Some(css_color) = used_classes.get(class) {
                    vtt_content.push_str(&format!("::cue(.{}) {{ color: {}; }}\n", class, css_color));
                }
            }
        }
        vtt_content.push_str("::cue { color: #FFFFFF; }\n");
        vtt_content.push('\n');
    }

    for cue in cue_lines {
        vtt_content.push_str(&cue);
    }

    Ok(vtt_content)
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

#[derive(serde::Serialize, Clone)]
pub struct MediaAdditionalParameters {
    pub initial_prompt: Option<String>,
    pub hotwords: Option<String>,
}

#[tauri::command]
pub async fn load_media_additional_parameters(
    project_id: String,
    asset_relative_path: String
) -> Result<Option<MediaAdditionalParameters>, CommandError> {
    match db_handler::load_media_transcript_data(&project_id, &asset_relative_path) {
        Ok(Some(data)) => Ok(Some(MediaAdditionalParameters {
            initial_prompt: data.initial_prompt,
            hotwords: data.hotwords,
        })),
        Ok(None) => Ok(None),
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub async fn save_media_additional_parameters(
    project_id: String,
    asset_relative_path: String,
    initial_prompt: Option<String>,
    hotwords: Option<String>
) -> Result<(), CommandError> {
    db_handler::update_media_additional_parameters(
        &project_id,
        &asset_relative_path,
        initial_prompt.as_deref(),
        hotwords.as_deref()
    )
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
    engine: Option<String>,
    state: tauri::State<'_, LiveTranscriptionState>,
) -> Result<bool, String> {
    if state.is_running.load(std::sync::atomic::Ordering::SeqCst) {
        return Err("Live transcription is already running.".to_string());
    }

    let model_path = resolve_model_path_cmd(&model_name, "live", engine.as_deref())
        .map_err(|e| e.to_string())?;

    if engine.as_deref() == Some("faster-whisper") {
        return crate::transcription::faster_whisper_live::start_faster_whisper_live(
            app_handle,
            model_path,
            language,
            save_audio,
            active_document_path,
            project_uuid,
            project_base_dir,
            state
        ).await;
    }

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

    let env_path = get_env_path().map_err(|e| e.to_string())?;
    #[cfg(target_os = "windows")]
    let binary_path = env_path.join("Library").join("bin").join("whisper-stream.exe");
    #[cfg(not(target_os = "windows"))]
    let binary_path = env_path.join("bin").join("whisper-stream");

    if !binary_path.exists() {
        return Err("whisper-stream binary not found in the environment. Please ensure whisper.cpp is fully installed.".to_string());
    }

    let binary_path_str = binary_path.to_string_lossy().to_string();

    let mut command = app_handle
        .shell()
        .command(binary_path_str.clone());

    // Set environment variables for dependencies
    if cfg!(target_os = "windows") {
        let env_bin_path = env_path.join("Library").join("bin");
        if env_bin_path.exists() {
            if let Ok(cleaned_env_path) = dunce::canonicalize(&env_bin_path) {
                let env_path_str = cleaned_env_path.to_string_lossy();
                if let Ok(existing_path) = std::env::var("PATH") {
                    command = command.env("PATH", format!("{};{}", env_path_str, existing_path));
                } else {
                    command = command.env("PATH", env_path_str.to_string());
                }
            }
        }
    } else if cfg!(target_os = "macos") {
        let env_lib_path = env_path.join("lib");
        if env_lib_path.exists() {
            let env_lib_path_str = env_lib_path.to_string_lossy();
            if let Ok(existing_path) = std::env::var("DYLD_LIBRARY_PATH") {
                command = command.env("DYLD_LIBRARY_PATH", format!("{}:{}", env_lib_path_str, existing_path));
            } else {
                command = command.env("DYLD_LIBRARY_PATH", env_lib_path_str.to_string());
            }
        }
    }

    if save_audio {
        let active_doc_path = PathBuf::from(&active_document_path);
        let attachments_dir = active_doc_path.parent().unwrap().join("attachments");
        fs::create_dir_all(&attachments_dir).map_err(|e| e.to_string())?;

        args.push("--save-audio".to_string());
        command = command.current_dir(attachments_dir);
    }

    info!("[Live Transcription] Executing command '{}' with args: {:?}", binary_path_str, args);
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
            
            // First collect all candidate files synchronously
            let mut candidate_paths: Vec<PathBuf> = Vec::new();
            match fs::read_dir(&attachments_dir) {
                Ok(entries) => {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            let path = entry.path();
                            if path.is_file() {
                                if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
                                    if extension.eq_ignore_ascii_case("wav") {
                                         candidate_paths.push(path);
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

            // Now process them (trim silence)
            for path in candidate_paths {
                let path_str = path.to_string_lossy().to_string();
                // Attempt to trim silence
                if let Err(e) = trim_silence_from_wav(&app_handle, &path_str).await {
                    error!("[Live Transcription] Failed to trim silence from {}: {}", path_str, e);
                }
                audio_files.push(path_str);
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

                        // Load existing attachments
                        let mut existing_attachments: Vec<String> = custom_fields.iter()
                            .find(|f| f.get("key").and_then(|k| k.as_str()) == Some("attachments"))
                            .and_then(|f| f.get("value").and_then(|v| v.as_str()))
                            .and_then(|v| serde_json::from_str(v).ok())
                            .unwrap_or_else(Vec::new);

                        // Append new audio files to existing attachments
                        existing_attachments.extend(audio_files.clone());
                        existing_attachments.sort();
                        existing_attachments.dedup();

                        let attachments_json_string = json!(existing_attachments).to_string();

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
                            language_code: metadata_from_db.language_code,
                            properties: metadata_from_db.properties,
                            file_type: metadata_from_db.file_type.unwrap_or_else(|| "document".to_string()),
                            thumbnail: metadata_from_db.thumbnail,
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
                            language_code: None,
                            properties: None,
                            file_type: "document".to_string(),
                            thumbnail: None,
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

// Helper to trim silence from the beginning of a WAV file using FFmpeg
async fn trim_silence_from_wav(app_handle: &AppHandle, file_path_str: &str) -> Result<(), String> {
    let input_path = PathBuf::from(file_path_str);
    if !input_path.exists() {
        return Err(format!("Input file not found: {}", file_path_str));
    }

    let output_path = input_path.with_extension("trimmed.wav");
    let ffmpeg_path = get_ffmpeg_path(app_handle).map_err(|e| e.to_string())?;

    // Use silenceremove filter to remove silence from the start
    // start_periods=1: remove silence from the beginning only
    // start_duration=0.1: silence must be at least 0.1s long to be detected
    // start_threshold=-50dB: consider anything below -50dB as silence
    let args = vec![
        "-i".to_string(),
        file_path_str.to_string(),
        "-af".to_string(),
        "silenceremove=start_periods=1:start_duration=0.1:start_threshold=-50dB".to_string(),
        "-y".to_string(),
        output_path.to_string_lossy().to_string(),
    ];

    info!("[Live Transcription] Trimming silence: {:?} {}", ffmpeg_path, args.join(" "));

    let output = app_handle
        .shell()
        .command(ffmpeg_path.to_string_lossy().to_string())
        .args(args)
        .output()
        .await
        .map_err(|e| format!("Failed to execute ffmpeg: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("FFmpeg failed: {}", stderr));
    }

    // Replace original file with trimmed file if successful and not empty
    if output_path.exists() && output_path.metadata().map(|m| m.len() > 0).unwrap_or(false) {
        fs::remove_file(&input_path).map_err(|e| format!("Failed to remove original file: {}", e))?;
        fs::rename(&output_path, &input_path).map_err(|e| format!("Failed to rename trimmed file: {}", e))?;
        info!("[Live Transcription] Successfully trimmed silence from {}", file_path_str);
    } else {
        return Err(format!("Trimmed output file missing or empty for {}", file_path_str));
    }

    Ok(())
}
