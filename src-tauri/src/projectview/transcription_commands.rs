// src/projectview/transcription_commands.rs
use super::shared_types::*;
use super::shared_utils::*;
use crate::welcome::config::CommandError;
use log::{debug, error, info, warn};
use serde_json;
use std::{
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
    // Removed unused imports:
    // collections::HashMap,
    // cmp::Ordering as CmpOrdering,
};
use tauri::{AppHandle};
use tauri_plugin_shell::ShellExt;
use quick_xml;


// --- trim_media Command --- (Unchanged)
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

// --- save_speaker_config Command --- (Unchanged)
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

// --- load_transcript_json Command --- (Unchanged)
#[tauri::command]
pub async fn load_transcript_json(transcript_path: String) -> Result<Vec<TranscriptSegment>, CommandError> {
    info!("[Backend Load JSON] Path: {}", transcript_path);
    let file_path = PathBuf::from(&transcript_path);

    if !file_path.exists() || !file_path.is_file() {
        return Err(CommandError::from(format!("Transcript file not found: {}", transcript_path)));
    }
    if file_path.extension().and_then(|e| e.to_str()) != Some("json") {
        return Err(CommandError::from("Only .json transcripts are supported for loading.".to_string()));
    }

    let file = File::open(&file_path)?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|e| CommandError::from(format!("Failed to parse transcript JSON: {}", e)))
}

// --- save_transcript_json Command --- (Unchanged)
#[tauri::command]
pub async fn save_transcript_json( project_xml_path: String, transcript_path: String, segments: Vec<TranscriptSegment>) -> Result<(), CommandError> {
    info!("[Backend Save JSON] Transcript Path: {}", transcript_path);
    info!("[Backend Save JSON] Project XML Path: {}", project_xml_path);
    let transcript_path_buf = PathBuf::from(&transcript_path);
    let project_xml_path_buf = PathBuf::from(&project_xml_path);

    let project_base_dir = project_xml_path_buf.parent().ok_or_else(|| CommandError::from("Could not get project base dir from XML path"))?;

    if let Some(parent) = transcript_path_buf.parent() {
        fs::create_dir_all(parent)?;
    } else {
        return Err(CommandError::from(format!("Invalid transcript path (no parent directory): {}", transcript_path)));
    }

    let file = File::create(&transcript_path_buf)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &segments).map_err(|e| CommandError::from(format!("Failed to write transcript JSON: {}", e)))?;
    info!("[Backend Save JSON] Saved {} segments to disk.", segments.len());

    let transcript_filename = transcript_path_buf.file_name().and_then(|n| n.to_str()).ok_or_else(|| CommandError::from("Could not get transcript filename"))?.to_string();

    let (_item_type, media_identifier_opt, transcript_relative_path_buf) = get_item_details(&transcript_path_buf, project_base_dir)?;
    let media_identifier = media_identifier_opt.ok_or_else(|| CommandError::from(format!("Could not determine media identifier for transcript path: {}", transcript_path)))?;
    let transcript_relative_path = transcript_relative_path_buf.to_string_lossy().replace("\\", "/");

    info!("[Backend Save JSON] Media ID: '{}', Transcript Filename: '{}', Transcript Rel Path: '{}'", media_identifier, transcript_filename, transcript_relative_path);

    let xml_content = fs::read_to_string(&project_xml_path_buf)?;
    let mut project_data: ProjectXml = quick_xml::de::from_str(&xml_content)?;
    let mut found_media = false;

    if let Some(media_entry) = project_data.media_files.files.iter_mut().find(|f| f.name == media_identifier) {
        found_media = true;
        debug!("[Backend Save JSON] Found media entry '{}' in XML.", media_identifier);

        let mut found_transcript = false;
        for transcript_entry in media_entry.transcripts.iter_mut() {
            if transcript_entry.relative_path == transcript_relative_path {
                debug!("[Backend Save JSON] Found existing transcript entry for '{}'. Updating name (if needed).", transcript_relative_path);
                 if transcript_entry.name != transcript_filename {
                     warn!("[Backend Save JSON] Updating transcript name in XML from '{}' to '{}' for path '{}'", transcript_entry.name, transcript_filename, transcript_relative_path);
                    transcript_entry.name = transcript_filename.clone();
                 }
                found_transcript = true;
                break;
            }
        }

        if !found_transcript {
            debug!("[Backend Save JSON] Adding new transcript entry for '{}'.", transcript_relative_path);
            media_entry.transcripts.push(TranscriptEntryXml {
                name: transcript_filename.clone(),
                relative_path: transcript_relative_path.clone(),
            });
             media_entry.transcripts.sort_by(|a,b| a.name.cmp(&b.name));
        }
    }

    if !found_media {
        warn!("[Backend Save JSON] Media identifier '{}' not found in XML. XML not updated.", media_identifier);
         return Err(CommandError::from(format!("Media identifier '{}' not found in XML. Could not link saved transcript.", media_identifier)));
    }

    save_project_xml(&project_xml_path_buf, &project_data)?;
    info!("[Backend Save JSON] Project XML updated.");
    Ok(())
}

// --- prepare_output_paths Helper Function --- (Unchanged)
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

    let output_base_in_transcripts = transcripts_dir.join(&media_filename_stem);
    let output_path_base_str = output_base_in_transcripts.to_string_lossy().to_string();

    let expected_whisper_output_path = output_base_in_transcripts.with_extension("json");
    let expected_rttm_path = output_base_in_transcripts.with_extension("rttm");
    let final_transcript_path = expected_whisper_output_path.clone();

    debug!("[prepare_output_paths][{}] Base: '{}', Whisper JSON: '{}', RTTM: '{}', Final JSON: '{}'", job_id, output_path_base_str, expected_whisper_output_path.display(), expected_rttm_path.display(), final_transcript_path.display());

    Ok((output_path_base_str, expected_whisper_output_path, expected_rttm_path, final_transcript_path))
}


// --- Map Speaker IDs to User Names --- (Unchanged)
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
                .map(|index_0| index_0)
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
             if original_speaker != "Unknown" {
                 debug!("[Name Map] Speaker ID '{}' is not in a recognized generic format ('SPEAKER_XX' or 'speaker_X'). Skipping mapping for this segment.", original_speaker);
             }
        }
    }
    info!("[Name Map] Finished speaker name mapping process.");
}