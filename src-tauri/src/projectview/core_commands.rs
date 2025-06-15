// src-tauri/src/projectview/core_commands.rs
use super::shared_types::{*, TABLES_DIR, IMAGES_DIR, FileMetadata, StandardAssetMetadata}; // StandardAssetMetadata might be removable if not used by media/docs
use super::shared_utils::*;
use crate::welcome::config::CommandError;
use log::{debug, error, info, warn};
use serde::Deserialize;
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;
use std::{
    fs::{self},
    path::{Path, PathBuf},
    process::Command, // To run external commands
};
use quick_xml;
use tauri_plugin_os::platform; // For OS detection
use chrono::Utc;
use serde_json;
use serde::Serialize;
// Ensure db_handler is appropriately used or imported.
// If db_handler is already imported as `super::db_handler`, then specific functions might need to be brought into scope if not covered by `self`.
// However, the new commands will call `db_handler::function_name`, so a general `use crate::projectview::db_handler;` or `use super::db_handler;` is sufficient.
// The existing line `use super::db_handler::{self, delete_annotations_from_db, rename_annotations_in_db};` should be fine.
use super::db_handler::{self, delete_annotations_from_db, rename_annotations_in_db};
use tauri::Emitter;
use uuid::Uuid; // Added for UUID generation

// --- Table Layout Preferences Commands ---
#[tauri::command]
pub async fn save_table_layout_prefs(project_id: String, table_path: String, layout_json: String) -> Result<(), String> {
    db_handler::save_table_layout_preferences(&project_id, &table_path, &layout_json)
        .map_err(|e| {
            log::error!("Failed to save table layout prefs for project_id {} table {}: {}", project_id, table_path, e);
            e.to_string()
        })
}

#[tauri::command]
pub async fn load_table_layout_prefs(project_id: String, table_path: String) -> Result<Option<String>, String> {
    db_handler::load_table_layout_preferences(&project_id, &table_path)
        .map_err(|e| {
            log::error!("Failed to load table layout prefs for project_id {} table {}: {}", project_id, table_path, e);
            e.to_string()
        })
}
// --- End Table Layout Preferences Commands ---

#[derive(Clone, serde::Serialize)]
struct MediaRenamedPayload {
    old_media_stem: String,
    new_media_stem: String,
    new_media_file_relative_path: String,
    new_absolute_path: String,
}

#[derive(Clone, Serialize)]
struct ItemRenamedPayload {
    old_path: String,
    new_path: String,
    new_name: String,
    item_type: String,
    project_xml_path: String,
    base_directory: String,
}

// --- FFProbe Helper Structs ---
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
    #[serde(rename = "creation_time")]
    creation_time: Option<String>,
    #[serde(rename = "DURATION")]
    duration: Option<String>,
}

#[derive(Deserialize, Debug, Default, Clone)]
struct FFProbeFormat {
    duration: Option<String>,
    bit_rate: Option<String>,
    #[serde(default)]
    tags: Option<FFProbeFormatTags>,
    format_name: Option<String>,
}

#[derive(Deserialize, Debug, Default, Clone)]
struct FFProbeOutput {
    #[serde(default)]
    streams: Vec<FFProbeStream>,
    #[serde(default)]
    format: FFProbeFormat,
}

// --- Helper Functions for FFProbe Data Parsing ---
fn parse_duration_str_to_seconds(s_opt: Option<String>) -> Option<f64> {
    s_opt.as_deref().and_then(|s| {
        if s.contains(':') {
            let parts: Vec<&str> = s.split(':').collect();
            if parts.len() == 3 {
                let hours = parts[0].parse::<f64>().ok()?;
                let minutes = parts[1].parse::<f64>().ok()?;
                let seconds_ms = parts[2].parse::<f64>().ok()?;
                Some(hours * 3600.0 + minutes * 60.0 + seconds_ms)
            } else { None }
        } else {
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

// Helper function to get document metadata path (ONLY for .harvey_metadata.json files, not SQLite based metadata)
fn get_document_metadata_path_for_doc(doc_path: &Path) -> Result<PathBuf, CommandError> {
    let doc_parent_dir = doc_path.parent().ok_or_else(|| {
        CommandError::from(format!(
            "Could not get parent directory for document: {}",
            doc_path.display()
        ))
    })?;
    let doc_stem = doc_path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        CommandError::from(format!(
            "Could not get file stem for document: {}",
            doc_path.display()
        ))
    })?;
    let metadata_filename = format!(".{}.{}", doc_stem, METADATA_FILE_SUFFIX);
    Ok(doc_parent_dir.join(metadata_filename))
}

// Helper function to get media metadata path (for .metadata.json, specific to media assets if they still use it)
pub fn get_media_metadata_path(media_path: &Path) -> Result<PathBuf, CommandError> {
    let parent_dir = media_path.parent().ok_or_else(|| {
        CommandError::from(format!(
            "Could not get parent directory for media file: {}",
            media_path.display()
        ))
    })?;
    let media_stem = media_path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        CommandError::from(format!(
            "Could not get file stem for media file: {}",
            media_path.display()
        ))
    })?;

    let metadata_filename = format!(".{}.metadata.json", media_stem);
    Ok(parent_dir.join(metadata_filename))
}

// get_image_asset_metadata_path and get_table_asset_metadata_path are removed as image and table metadata are now in DB.
// If any other part of the codebase was using them, those parts would need updating.
// For now, they are removed from core_commands.rs as per the refactoring direction.

#[tauri::command]
pub async fn load_project_data(project_xml_path: String) -> Result<ProjectViewData, CommandError> {
    info!("[Backend Load XML] Start: {}", project_xml_path);
    let xml_path = PathBuf::from(&project_xml_path);
    if !xml_path.exists() || !xml_path.is_file() {
        return Err(CommandError::from(format!("Project file not found: {}", project_xml_path)));
    }
    let project_base_dir = xml_path.parent().ok_or_else(|| CommandError::from("Could not get project base directory."))?;
    let base_directory = project_base_dir.to_string_lossy().to_string();
    if base_directory.is_empty() {
        return Err(CommandError::from("Base directory path is empty."));
    }

    ensure_base_asset_dirs(project_base_dir)?;

    let project_xml_content = fs::read_to_string(&xml_path).map_err(|e| CommandError::from(format!("Failed to read XML {}: {}", xml_path.display(), e)))?;
    let mut project_data: ProjectXml = quick_xml::de::from_str(&project_xml_content).map_err(|e| CommandError::from(format!("Failed to parse XML {}: {}", xml_path.display(), e)))?;

    let mut was_uuid_generated = false;
    if project_data.project_uuid.is_empty() {
        let new_uuid = Uuid::new_v4().to_string();
        info!("[Backend Load XML] Project UUID was missing or empty. Generated new UUID: {}", new_uuid);
        project_data.project_uuid = new_uuid;
        was_uuid_generated = true;
    }

    let project_name = project_data.name.clone();
    info!("[Backend Load XML] Project Name: {}", project_name);
    info!("[Backend Load XML] Project UUID: {}", project_data.project_uuid); // Log the UUID being used

    let media_dir_rel_path = format!("{}/{}", HARVEY_FILES_DIR, MEDIA_DIR);
    let mut file_entries: Vec<FileEntry> = Vec::new();

    for media_entry in &project_data.media_files.files {
        let media_stem = &media_entry.name;
        let stem_rel_path = format!("{}/{}", media_dir_rel_path, media_stem);
        let stem_abs_path = project_base_dir.join(&stem_rel_path);

        if !stem_abs_path.exists() || !stem_abs_path.is_dir() {
            warn!("[Backend Load XML] Media stem directory listed in XML does not exist on disk (or is not a dir), skipping entry: '{}'", stem_abs_path.display());
            continue;
        }

        let mut media_children: Vec<FileEntry> = Vec::new();
        let mut transcript_children: Vec<FileEntry> = Vec::new();

        let media_file_rel_path = &media_entry.relative_path;
        let media_file_abs_path = project_base_dir.join(media_file_rel_path);

        if media_file_abs_path.exists() && media_file_abs_path.is_file() {
            let media_file_name = media_file_abs_path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let media_file_canonical = fs::canonicalize(&media_file_abs_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| media_file_abs_path.to_string_lossy().to_string());

            if !media_file_name.is_empty() {
                media_children.push(FileEntry {
                    name: media_file_name,
                    path: media_file_canonical,
                    relative_path: media_file_rel_path.clone().replace("\\", "/"),
                    file_type: "media".to_string(),
                    is_directory: false,
                    parent_relative_path: format!("{}/{}", stem_rel_path, MEDIA_SUBDIR).replace("\\", "/"),
                    depth: 5,
                    speakers: media_entry.speakers.clone(),
                    media_xml_identifier: Some(media_stem.clone()),
                    associated_transcripts: media_entry.transcripts.clone(),
                    children: Vec::new(),
                });
            } else {
                warn!("[Backend Load XML] Could not determine media filename from relative path: {}", media_file_rel_path);
            }
        } else {
            warn!("[Backend Load XML] Media file listed in XML does not exist on disk: '{}'", media_file_abs_path.display());
        }

        for transcript_xml_entry in &media_entry.transcripts {
            let transcript_rel_path = &transcript_xml_entry.relative_path;
            let transcript_abs_path = project_base_dir.join(transcript_rel_path);

            if transcript_abs_path.exists() && transcript_abs_path.is_file() {
                let transcript_file_name = transcript_xml_entry.name.clone();
                 let transcript_file_canonical = fs::canonicalize(&transcript_abs_path)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| transcript_abs_path.to_string_lossy().to_string());

                transcript_children.push(FileEntry {
                    name: transcript_file_name,
                    path: transcript_file_canonical,
                    relative_path: transcript_rel_path.clone().replace("\\", "/"),
                    file_type: "transcript".to_string(),
                    is_directory: false,
                    parent_relative_path: format!("{}/{}", stem_rel_path, TRANSCRIPTS_SUBDIR).replace("\\", "/"),
                    depth: 5,
                    speakers: None,
                    media_xml_identifier: Some(media_stem.clone()),
                    associated_transcripts: Vec::new(),
                    children: Vec::new(),
                });
            } else {
                warn!("[Backend Load XML] Transcript file listed in XML does not exist on disk: '{}'", transcript_abs_path.display());
            }
        }

        media_children.sort_by(|a, b| a.name.cmp(&b.name));
        transcript_children.sort_by(|a, b| a.name.cmp(&b.name));

        let mut sub_folders: Vec<FileEntry> = Vec::new();
        let media_subdir_rel_path = format!("{}/{}", stem_rel_path, MEDIA_SUBDIR).replace("\\", "/");
        sub_folders.push(FileEntry {
            name: MEDIA_SUBDIR.to_string(),
            path: project_base_dir.join(&media_subdir_rel_path).to_string_lossy().to_string(),
            relative_path: media_subdir_rel_path,
            file_type: "directory".to_string(),
            is_directory: true,
            parent_relative_path: stem_rel_path.clone().replace("\\", "/"),
            depth: 4,
            speakers: None,
            media_xml_identifier: Some(media_stem.clone()),
            associated_transcripts: Vec::new(),
            children: media_children,
        });
        let transcripts_subdir_rel_path = format!("{}/{}", stem_rel_path, TRANSCRIPTS_SUBDIR).replace("\\", "/");
        sub_folders.push(FileEntry {
            name: TRANSCRIPTS_SUBDIR.to_string(),
            path: project_base_dir.join(&transcripts_subdir_rel_path).to_string_lossy().to_string(),
            relative_path: transcripts_subdir_rel_path,
            file_type: "directory".to_string(),
            is_directory: true,
            parent_relative_path: stem_rel_path.clone().replace("\\", "/"),
            depth: 4,
            speakers: None,
            media_xml_identifier: Some(media_stem.clone()),
            associated_transcripts: Vec::new(),
            children: transcript_children,
        });

        file_entries.push(FileEntry {
            name: media_stem.clone(),
            path: stem_abs_path.to_string_lossy().to_string(),
            relative_path: stem_rel_path.clone().replace("\\", "/"),
            file_type: "directory_media_stem".to_string(),
            is_directory: true,
            parent_relative_path: media_dir_rel_path.clone().replace("\\", "/"),
            depth: 3,
            speakers: media_entry.speakers.clone(),
            media_xml_identifier: Some(media_stem.clone()),
            associated_transcripts: Vec::new(),
            children: sub_folders,
        });
    }
    file_entries.sort_by(|a, b| a.name.cmp(&b.name));

    log::debug!(
        "[Backend Load XML] Media stems: {}, Documents: {}, Tables: {}, Images: {}, Imported Transcripts: {}, App Metadata Files: {}",
            file_entries.len(),
        project_data.document_files.files.len(),
        project_data.table_files.files.len(),
        project_data.image_files.files.len(),
        project_data.imported_transcript_files.files.len(),
        project_data.document_metadata_files.files.len() // This list is now only for .harvey_metadata.json from imported "doc" types.
    );

    if was_uuid_generated {
        match save_project_xml(&xml_path, &project_data) {
            Ok(_) => info!("[Backend Load XML] Successfully saved updated project XML with new UUID to {}", xml_path.display()),
            Err(e) => warn!("[Backend Load XML] Failed to save updated project XML with new UUID to {}: {}. The new UUID will be used for this session, but not persisted.", xml_path.display(), e),
        }
    }

    Ok(ProjectViewData {
        project_name,
        project_xml_path,
        base_directory,
        project_uuid: project_data.project_uuid.clone(),
        files: file_entries,
        document_files: project_data.document_files.files,
        table_files: project_data.table_files.files,
        image_files: project_data.image_files.files,
        imported_transcript_files: project_data.imported_transcript_files.files,
        document_metadata_files: project_data.document_metadata_files.files,
    })
}


#[tauri::command]
pub async fn import_media(app_handle: AppHandle, source_file_path_str: String, project_xml_path_str: String) -> Result<Vec<FileEntry>, CommandError> {
    info!("[Backend Import] Source: '{}', Project XML: '{}'", source_file_path_str, project_xml_path_str);
    let source_path = PathBuf::from(&source_file_path_str);
    let project_xml_path = PathBuf::from(&project_xml_path_str);

    if !source_path.exists() || !source_path.is_file() {
        return Err(CommandError::from(format!("Source file not found: {}", source_file_path_str)));
    }
    let project_base_dir = project_xml_path.parent().ok_or_else(|| CommandError::from("Could not get project base directory"))?;
    if !project_base_dir.exists() || !project_base_dir.is_dir() {
        return Err(CommandError::from(format!("Project base directory not found: {}", project_base_dir.display())));
    }

    let source_filename_os = source_path.file_name().ok_or_else(|| CommandError::from("Could not get filename"))?;
    let source_filename = source_filename_os.to_string_lossy().to_string();

    let media_stem_identifier = source_path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| CommandError::from("Invalid source filename stem."))?;

    let media_asset_dir = project_base_dir.join(HARVEY_FILES_DIR).join(MEDIA_DIR);
    let media_stem_base_path = media_asset_dir.join(media_stem_identifier);
    let media_subfolder_path = media_stem_base_path.join(MEDIA_SUBDIR);
    let transcripts_subfolder_path = media_stem_base_path.join(TRANSCRIPTS_SUBDIR);
    let destination_media_path = media_subfolder_path.join(&source_filename);

    let xml_content_check = fs::read_to_string(&project_xml_path)?;
    let project_data_check: ProjectXml = quick_xml::de::from_str(&xml_content_check)?;
    if project_data_check.media_files.files.iter().any(|f| f.name == media_stem_identifier) {
        return Err(CommandError::from(format!("Media identifier '{}' already exists.", media_stem_identifier)));
    }

    if media_stem_base_path.exists() {
        warn!("[Backend Import] Target media stem directory exists: {}. Files may be overwritten or structure reused.", media_stem_base_path.display());
    }

    fs::create_dir_all(&media_subfolder_path)?;
    fs::create_dir_all(&transcripts_subfolder_path)?;

    fs::copy(&source_path, &destination_media_path)?;
    info!("[Backend Import] File copied to {}", destination_media_path.display());

    let mut duration_seconds: Option<f64> = None;
    let mut width: Option<i32> = None;
    let mut height: Option<i32> = None;
    let mut frame_rate: Option<f32> = None;
    let mut bit_rate_overall: Option<i64> = None;
    let mut audio_codec: Option<String> = None;
    let mut video_codec: Option<String> = None;
    // let mut creation_time_tag: Option<String> = None; // Removed

    let ffprobe_args = vec![
        "-v".to_string(), "quiet".to_string(),
        "-print_format".to_string(), "json".to_string(),
        "-show_format".to_string(),
        "-show_streams".to_string(),
        destination_media_path.to_string_lossy().to_string(),
    ];

    info!("[Backend Import] Running ffprobe for: {}", destination_media_path.display());
    match app_handle.shell().sidecar("ffprobe").expect("ffprobe sidecar not configured in tauri.conf.json").args(ffprobe_args).output().await {
        Ok(output) => {
            if output.status.success() {
                let ffprobe_json_str = String::from_utf8_lossy(&output.stdout).to_string();
                debug!("[Backend Import] ffprobe output JSON for {}: {}", destination_media_path.display(), ffprobe_json_str);
                match serde_json::from_str::<FFProbeOutput>(&ffprobe_json_str) {
                    Ok(parsed_ffprobe_output) => {
                        duration_seconds = parse_duration_str_to_seconds(parsed_ffprobe_output.format.duration.clone())
                            .or_else(|| parse_duration_str_to_seconds(parsed_ffprobe_output.format.tags.as_ref().and_then(|t| t.duration.clone())));

                        bit_rate_overall = parsed_ffprobe_output.format.bit_rate.as_deref().and_then(|s| s.parse().ok());
                        // if let Some(tags) = parsed_ffprobe_output.format.tags { // Removed
                        //     creation_time_tag = tags.creation_time; // Removed
                        // } // Removed

                        for stream in parsed_ffprobe_output.streams {
                            if duration_seconds.is_none() {
                                 duration_seconds = parse_duration_str_to_seconds(stream.tags.duration.clone());
                            }
                            match stream.codec_type.as_deref() {
                                Some("video") if width.is_none() => {
                                    width = stream.width;
                                    height = stream.height;
                                    video_codec = stream.codec_name;
                                    frame_rate = parse_frame_rate_str(stream.avg_frame_rate.clone())
                                        .or_else(|| parse_frame_rate_str(stream.r_frame_rate.clone()));
                                    if bit_rate_overall.is_none() {
                                        bit_rate_overall = stream.bit_rate.as_deref().and_then(|s| s.parse().ok());
                                    }
                                }
                                Some("audio") if audio_codec.is_none() => {
                                    audio_codec = stream.codec_name;
                                    if bit_rate_overall.is_none() && stream.bit_rate.is_some() {
                                         bit_rate_overall = stream.bit_rate.as_deref().and_then(|s| s.parse().ok());
                                    }
                                }
                                _ => {}
                            }
                        }
                        info!("[Backend Import] Successfully parsed ffprobe output for {}", destination_media_path.display());
                    }
                    Err(e) => {
                        error!("[Backend Import] Failed to parse ffprobe JSON for {}: {}. JSON: '{}'", destination_media_path.display(), e, ffprobe_json_str);
                    }
                }
            } else {
                let stderr_str = String::from_utf8_lossy(&output.stderr);
                error!("[Backend Import] ffprobe failed for {}. Code: {:?}, Stderr: {}", destination_media_path.display(), output.status.code(), stderr_str);
            }
        }
        Err(e) => {
            error!("[Backend Import] ffprobe execution error for {}: {}", destination_media_path.display(), e);
        }
    }

    // --- Remove old .metadata.json file creation logic ---
    // The entire 'match get_media_metadata_path(...){...}' block has been removed.

    // --- Prepare and save metadata to SQLite database ---
    let file_metadata_for_db = FileMetadata {
        file_name: source_filename.clone(), // source_filename is available from earlier
        file_path: destination_media_path.to_string_lossy().into_owned(), // Absolute path
        last_modified: Utc::now().to_rfc3339(), // For new assets, set current time
        title: String::new(),
        description: String::new(),
        summary: String::new(),
        duration_seconds, // From ffprobe
        width,            // From ffprobe
        height,           // From ffprobe
        frame_rate,       // From ffprobe
        bit_rate: bit_rate_overall, // From ffprobe
        audio_codec: audio_codec.clone(), // From ffprobe (ensure cloned if Option<String>)
        video_codec: video_codec.clone(), // From ffprobe (ensure cloned if Option<String>)
        created_at: Some(Utc::now().to_rfc3339()), // Set to current time on import
    };

    let final_asset_type: String;
    if video_codec.is_some() {
        final_asset_type = "video".to_string();
    } else if audio_codec.is_some() {
        final_asset_type = "audio".to_string();
    } else {
        final_asset_type = source_path.extension()
            .and_then(|s| s.to_str())
            .map_or_else(|| "media".to_string(), |ext| ext.to_lowercase());
    }

    // destination_relative_path_for_xml is calculated before this block for XML update, use it as DB key
    let destination_relative_path_for_xml_calc = Path::new(HARVEY_FILES_DIR)
        .join(MEDIA_DIR)
        .join(media_stem_identifier) // media_stem_identifier is from source_path.file_stem()
        .join(MEDIA_SUBDIR)
        .join(&source_filename) // source_filename is from source_path.file_name()
        .to_string_lossy()
        .replace("\\", "/");
    let db_key_relative_path = destination_relative_path_for_xml_calc;

    // project_id_for_db is project_data_check.project_uuid, parsed earlier
    info!("[Backend Import] Media FileMetadata before save: created_at={:?}", file_metadata_for_db.created_at);
    match db_handler::save_asset_metadata(
        &project_data_check.project_uuid, // Added: project_id (UUID of the project)
        &file_metadata_for_db,
        &db_key_relative_path,
        &final_asset_type,
        None, // custom_fields_json (None for initial import)
    ) {
        Ok(_) => info!("[Backend Import] Successfully saved media metadata to DB for: {} with project_id {}", db_key_relative_path, project_data_check.project_uuid),
        Err(e) => {
            warn!("[Backend Import] Failed to save media metadata to DB for {} (project_id {}): {}. Proceeding with XML update.", db_key_relative_path, project_data_check.project_uuid, e);
        }
    }

    let xml_content = fs::read_to_string(&project_xml_path)?;
    let mut project_data: ProjectXml = quick_xml::de::from_str(&xml_content)?;

    let destination_relative_path_for_xml = Path::new(HARVEY_FILES_DIR)
        .join(MEDIA_DIR)
        .join(media_stem_identifier)
        .join(MEDIA_SUBDIR)
        .join(&source_filename)
        .to_string_lossy()
        .replace("\\", "/");

    let new_media_entry = MediaFileEntryXml {
        name: media_stem_identifier.to_string(),
        original_path: Some(source_file_path_str.clone()),
        relative_path: destination_relative_path_for_xml,
        speakers: Some(SpeakersXml::default()),
        transcripts: Vec::new(),
    };

    project_data.media_files.files.push(new_media_entry);
    project_data.media_files.files.sort_by(|a,b| a.name.cmp(&b.name));

    save_project_xml(&project_xml_path, &project_data)?;
    info!("[Backend Import] XML updated with entry '{}'.", media_stem_identifier);

    load_project_data(project_xml_path_str).await.map(|data| data.files)
}


#[tauri::command]
pub async fn delete_project_item( item_path: String, project_xml_path: String) -> Result<(), CommandError> {
    info!("[Backend Delete] Request for: {} in project_xml: {}", item_path, project_xml_path);
    let item_path_buf = PathBuf::from(&item_path);
    let xml_path_buf = PathBuf::from(&project_xml_path);

    if !xml_path_buf.exists() || !xml_path_buf.is_file() {
        return Err(CommandError::from(format!("Project XML not found: {}", project_xml_path)));
    }
    let project_base_dir = xml_path_buf.parent().ok_or_else(|| CommandError::from("Could not get project base dir"))?;

    // Get project_id for DB operations
    let project_xml_content_for_uuid = fs::read_to_string(&xml_path_buf)
        .map_err(|e| CommandError::Io(format!("Failed to read project XML for UUID from {}: {}", xml_path_buf.display(), e)))?;
    let project_data_for_uuid: ProjectXml = quick_xml::de::from_str(&project_xml_content_for_uuid)
        .map_err(|e| CommandError::XmlDeserialization(format!("Failed to parse project XML for UUID from {}: {}", xml_path_buf.display(), e)))?;
    let project_id_for_db = project_data_for_uuid.project_uuid;
    if project_id_for_db.is_empty() {
        error!("[Backend Delete] Project UUID is empty in XML file: {}. Cannot proceed with DB operations.", xml_path_buf.display());
        return Err(CommandError::Message(format!("Project ID (UUID) is missing in the project file ({}). DB operations cannot proceed.", xml_path_buf.display())));
    }
    info!("[Backend Delete] Operating with project_id: {}", project_id_for_db);

    if !item_path_buf.exists() {
        warn!("[Backend Delete] Item '{}' (project_id: {}) not found. Assuming already deleted or invalid path. Attempting XML cleanup...", item_path, project_id_for_db);
        let (item_type_guess, media_stem_opt_guess, item_relative_path_buf_guess) = match get_item_details(&item_path_buf, project_base_dir) {
            Ok(details) => details,
            Err(_) => {
                warn!("[Backend Delete] Could not determine item details for non-existent path '{}'. Skipping XML cleanup.", item_path);
                return Ok(());
            }
        };
        let item_relative_path_guess = item_relative_path_buf_guess.to_string_lossy().replace("\\", "/");
        let item_type_guess = {
            let path_lower = item_relative_path_guess.to_lowercase();
            let transcripts_folder = format!("{}/", TRANSCRIPTS_SUBDIR.to_lowercase());
            let tables_folder = format!("{}/", TABLES_DIR.to_lowercase());
            let ext = item_path_buf.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if item_type_guess == "other" && path_lower.contains(&transcripts_folder) && ext == "json" {
                "imported_transcript".to_string()
            } else if item_type_guess == "other" && path_lower.contains(&tables_folder) && (ext == "csv" || ext == "xlsx") {
                "table".to_string()
            } else {
                item_type_guess
            }
        };
        let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
        let mut xml_changed = false;

        match item_type_guess.as_str() {
            "media" | "directory_media_stem" => {
                if let Some(media_stem) = media_stem_opt_guess {
                    let initial_len = project_data.media_files.files.len();
                    project_data.media_files.files.retain(|entry| entry.name != media_stem);
                    if project_data.media_files.files.len() < initial_len {
                        info!("[Backend Delete] Cleaned up XML media entry for non-existent '{}'.", media_stem);
                        xml_changed = true;
                    }
                }
            },
            "transcript" => {
                if let Some(media_stem) = media_stem_opt_guess {
                    if let Some(media_entry) = project_data.media_files.files.iter_mut().find(|f| f.name == media_stem) {
                        let initial_transcript_len = media_entry.transcripts.len();
                        media_entry.transcripts.retain(|t| t.relative_path != item_relative_path_guess);
                        if media_entry.transcripts.len() < initial_transcript_len {
                            info!("[Backend Delete] Cleaned up XML media-associated transcript entry '{}'.", item_relative_path_guess);
                            xml_changed = true;
                        }
                    }
                }
            },
            "imported_transcript" => {
                let initial_len = project_data.imported_transcript_files.files.len();
                project_data.imported_transcript_files.files.retain(|t| t.relative_path != item_relative_path_guess);
                if project_data.imported_transcript_files.files.len() < initial_len {
                    info!("[Backend Delete] Cleaned up XML imported transcript entry '{}'.", item_relative_path_guess);
                    xml_changed = true;
                }
                // Metadata is in DB, attempt to delete it as well during cleanup
                if xml_changed { // Only if the main transcript entry was found and removed from XML
                    if let Err(e) = db_handler::delete_asset_metadata(&project_id_for_db, &item_relative_path_guess) {
                        warn!("[Backend Delete] Failed to delete asset metadata from DB (project_id: {}) during cleanup for non-existent path {}: {}", project_id_for_db, item_relative_path_guess, e);
                    } else {
                        info!("[Backend Delete] Deleted asset metadata from DB (project_id: {}) during cleanup for non-existent path {}", project_id_for_db, item_relative_path_guess);
                    }
                }
                // The document_metadata_files list is no longer updated for imported transcript metadata.
            },
            "doc" => {
                let initial_doc_len = project_data.document_files.files.len();
                project_data.document_files.files.retain(|d| d.relative_path != item_relative_path_guess);
                if project_data.document_files.files.len() < initial_doc_len {
                    info!("[Backend Delete] Cleaned up XML document entry '{}'.", item_relative_path_guess);
                    xml_changed = true;
                }
                let initial_meta_len = project_data.document_metadata_files.files.len();
                project_data.document_metadata_files.files.retain(|m| m.original_document_relative_path != item_relative_path_guess);
                if project_data.document_metadata_files.files.len() < initial_meta_len {
                    info!("[Backend Delete] Cleaned up XML document (app) metadata entry for original doc '{}'.", item_relative_path_guess);
                    xml_changed = true;
                }
            },
            "table" => {
                let initial_table_len = project_data.table_files.files.len();
                project_data.table_files.files.retain(|t| t.relative_path != item_relative_path_guess);
                if project_data.table_files.files.len() < initial_table_len {
                    info!("[Backend Delete] Cleaned up XML table entry '{}'.", item_relative_path_guess);
                    xml_changed = true;
                    if let Err(e) = db_handler::delete_asset_metadata(&project_id_for_db, &item_relative_path_guess) {
                        warn!("[Backend Delete] Failed to delete asset metadata from DB (project_id: {}) during cleanup for table {}: {}", project_id_for_db, item_relative_path_guess, e);
                    } else {
                        info!("[Backend Delete] Deleted asset metadata from DB (project_id: {}) during cleanup for table {}", project_id_for_db, item_relative_path_guess);
                    }
                }
            },
            "image" => {
                let initial_image_len = project_data.image_files.files.len();
                project_data.image_files.files.retain(|i| i.relative_path != item_relative_path_guess);
                if project_data.image_files.files.len() < initial_image_len {
                    info!("[Backend Delete] Cleaned up XML image entry '{}'.", item_relative_path_guess);
                    xml_changed = true;
                    if let Err(e) = db_handler::delete_asset_metadata(&project_id_for_db, &item_relative_path_guess) {
                        warn!("[Backend Delete] Failed to delete asset metadata from DB (project_id: {}) during cleanup for non-existent image {}: {}", project_id_for_db, item_relative_path_guess, e);
                    } else {
                        info!("[Backend Delete] Deleted asset metadata from DB (project_id: {}) during cleanup for non-existent image {}", project_id_for_db, item_relative_path_guess);
                    }
                }
            },
            _ => {
                warn!("[Backend Delete] Unknown item type '{}' for XML cleanup of non-existent path '{}'.", item_type_guess, item_path);
            }
        }

        if xml_changed { save_project_xml(&xml_path_buf, &project_data)?; }
        return Ok(());
    }

    if item_path_buf.is_dir() {
         let (item_type, _, _) = get_item_details(&item_path_buf, project_base_dir)?;
         if item_type != "directory_media_stem" {
            return Err(CommandError::from(format!("Deleting arbitrary directories ('{}') is not supported via this function. Delete the associated media file or asset instead.", item_type)));
         }
         warn!("[Backend Delete] Request path '{}' is a directory, but rename should be triggered by logic for its primary media file.", item_path);
    }

    let (item_type, media_stem_opt, item_relative_path_buf) = get_item_details(&item_path_buf, project_base_dir)?;
    let item_relative_path = item_relative_path_buf.to_string_lossy().replace("\\", "/");
    let item_type = {
        let path_lower = item_relative_path.to_lowercase();
        let transcripts_folder = format!("{}/", TRANSCRIPTS_SUBDIR.to_lowercase());
        let tables_folder = format!("{}/", TABLES_DIR.to_lowercase());
        let images_folder = format!("{}/", IMAGES_DIR.to_lowercase());
        let ext = item_path_buf.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if item_type == "other" && path_lower.contains(&transcripts_folder) && ext == "json" {
            "imported_transcript".to_string()
        } else if item_type == "other" && path_lower.contains(&tables_folder) && (ext == "csv" || ext == "xlsx") {
            "table".to_string()
        } else if item_type == "other" && path_lower.contains(&images_folder)
            && matches!(ext.as_str(), "jpg"|"jpeg"|"png"|"gif"|"bmp"|"webp"|"tiff")
        {
            "image".to_string()
        } else {
            item_type
        }
    };
    info!("[Backend Delete] Item type: '{}', Media Stem: {:?}, Rel Path: '{}'", item_type, media_stem_opt, item_relative_path);

    match item_type.as_str() {
        "media" => {
             if let Some(media_stem) = media_stem_opt.as_deref() {
                let media_stem_dir_path = project_base_dir.join(HARVEY_FILES_DIR).join(MEDIA_DIR).join(media_stem);
                if media_stem_dir_path.exists() && media_stem_dir_path.is_dir() {
                    info!("[Backend Delete] Deleting media stem directory: {}", media_stem_dir_path.display());
                    fs::remove_dir_all(&media_stem_dir_path).map_err(|e| CommandError::from(format!("Failed to delete directory {}: {}", media_stem_dir_path.display(), e)))?;

                    info!("[Backend Delete] Updating XML to remove entry for '{}'", media_stem);
                    let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
                    let initial_len = project_data.media_files.files.len();
                    project_data.media_files.files.retain(|entry| entry.name != media_stem);
                    if project_data.media_files.files.len() < initial_len {
                        save_project_xml(&xml_path_buf, &project_data)?;
                        info!("[Backend Delete] XML media entry removed.");
                    } else {
                        warn!("[Backend Delete] Deleted directory but no XML entry found for '{}'.", media_stem);
                    }
                } else {
                    warn!("[Backend Delete] Media stem directory {} not found. Assuming already deleted. Cleaning up XML.", media_stem_dir_path.display());
                     let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
                     let initial_len = project_data.media_files.files.len();
                     project_data.media_files.files.retain(|entry| entry.name != media_stem);
                     if project_data.media_files.files.len() < initial_len {
                         save_project_xml(&xml_path_buf, &project_data)?;
                         info!("[Backend Delete] XML media entry removed during cleanup.");
                     }
                }
            } else {
                return Err(CommandError::from(format!("Could not determine media stem for media file deletion: {}", item_path)));
            }
        },
        "transcript" => {
             if let Some(media_stem) = media_stem_opt.as_deref() {
                info!("[Backend Delete] Deleting media-associated transcript file: {}", item_path_buf.display());
                fs::remove_file(&item_path_buf).map_err(|e| CommandError::from(format!("Failed to delete file {}: {}", item_path_buf.display(), e)))?;

                info!("[Backend Delete] Updating XML to remove transcript link for '{}' with path '{}'", media_stem, item_relative_path);
                let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
                let mut xml_changed = false;
                if let Some(media_entry) = project_data.media_files.files.iter_mut().find(|f| f.name == media_stem) {
                    let initial_transcript_len = media_entry.transcripts.len();
                    media_entry.transcripts.retain(|t| t.relative_path != item_relative_path);
                    if media_entry.transcripts.len() < initial_transcript_len {
                        info!("[Backend Delete] Transcript entry removed from XML for media '{}'.", media_stem);
                        xml_changed = true;
                    } else {
                        warn!("[Backend Delete] Deleted transcript file, but no matching entry found in XML for path '{}' under media '{}'.", item_relative_path, media_stem);
                    }
                } else {
                    warn!("[Backend Delete] Deleted transcript file, but media identifier '{}' not found in XML.", media_stem);
                }
                if xml_changed {
                    save_project_xml(&xml_path_buf, &project_data)?;
                    info!("[Backend Delete] XML updated.");
                }
            } else {
                return Err(CommandError::from(format!("Could not determine media stem for transcript: {}", item_path)));
            }
        },
        "imported_transcript" => {
            info!("[Backend Delete] Deleting standalone imported transcript file: {}", item_path_buf.display());
            fs::remove_file(&item_path_buf)
                .map_err(|e| CommandError::from(format!("Failed to delete imported transcript file {}: {}", item_path_buf.display(), e)))?;

            if let Some(folder) = item_path_buf.parent() {
                if folder.exists() {
                    match fs::remove_dir(folder) {
                        Ok(_) => (),
                        Err(err) if err.kind() == std::io::ErrorKind::DirectoryNotEmpty => (), // Ok if not empty (e.g. other files exist)
                        Err(err) => warn!("[Backend Delete] Failed to delete transcript folder {}: {}. Continuing.", folder.display(), err), // Log and continue
                    }
                }
            }

            // Delete metadata from DB
            if let Err(e) = db_handler::delete_asset_metadata(&project_id_for_db, &item_relative_path) {
                warn!("[Backend Delete] Failed to delete asset metadata from DB for project_id {}, path {}: {}. Main file was deleted.", project_id_for_db, item_relative_path, e);
            } else {
                info!("[Backend Delete] Deleted asset metadata from DB for project_id {}, path {}", project_id_for_db, item_relative_path);
            }

            info!("[Backend Delete] Updating XML to remove imported transcript entry '{}'", item_relative_path);
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            let initial_entries = project_data.imported_transcript_files.files.len();
            project_data.imported_transcript_files.files.retain(|t| t.relative_path != item_relative_path);

            // document_metadata_files list in XML is no longer managed for imported transcript metadata

            if project_data.imported_transcript_files.files.len() < initial_entries {
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Delete] XML updated for imported transcript.");
            } else {
                warn!("[Backend Delete] Deleted imported transcript file, but no matching entry found in XML for path '{}'.", item_relative_path);
            }
        },
        "doc" => {
            let stem = item_path_buf.file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| CommandError::from(format!("Could not get document stem: {}", item_path_buf.display())))?;
            let docs_root = project_base_dir.join(HARVEY_FILES_DIR).join(DOCS_DIR);
            let doc_folder = docs_root.join(stem);
            if doc_folder.exists() && doc_folder.is_dir() {
                info!("[Backend Delete] Deleting document folder: {}", doc_folder.display());
                fs::remove_dir_all(&doc_folder)
                    .map_err(|e| CommandError::from(format!("Failed to delete document folder {}: {}", doc_folder.display(), e)))?;

                if item_relative_path.to_lowercase().ends_with(".pdf") {
                    if let Err(db_err) = delete_annotations_from_db(&project_id_for_db, &item_relative_path, "pdf") {
                        warn!("[Backend Delete] Failed to delete PDF annotations from DB for project_id {}, path {}: {}", project_id_for_db, item_relative_path, db_err);
                    }
                }
            } else {
                info!("[Backend Delete] Document folder not found for project_id {}, path {}. Deleting single file: {}", project_id_for_db, doc_folder.display(), item_path_buf.display());
                fs::remove_file(&item_path_buf)
                    .map_err(|e| CommandError::from(format!("Failed to delete document file {}: {}", item_path_buf.display(), e)))?;
                 if item_relative_path.to_lowercase().ends_with(".pdf") {
                    if let Err(db_err) = delete_annotations_from_db(&project_id_for_db, &item_relative_path, "pdf") {
                        warn!("[Backend Delete] Failed to delete PDF annotations from DB for single file (project_id {}), path {}: {}", project_id_for_db, item_relative_path, db_err);
                    }
                }
            }
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            let prefix = format!("{}/{}/{}", HARVEY_FILES_DIR, DOCS_DIR, stem);
            project_data.document_files.files.retain(|d| !d.relative_path.starts_with(&prefix) && d.relative_path != item_relative_path);
            project_data.document_metadata_files.files
                .retain(|m| !m.original_document_relative_path.starts_with(&prefix) && m.original_document_relative_path != item_relative_path);
            save_project_xml(&xml_path_buf, &project_data)?;
            info!("[Backend Delete] XML entries removed for document '{}'", stem);
        },
        "table" => {
            info!("[Backend Delete] Deleting table file: {}", item_path_buf.display());
            let file_stem = item_path_buf.file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| CommandError::from(format!("Could not get table filename stem for deletion: {}", item_path_buf.display())))?;

            let tables_dir = project_base_dir.join(HARVEY_FILES_DIR).join(TABLES_DIR);
            let folder_path = tables_dir.join(file_stem);

            if folder_path.exists() && folder_path.is_dir() {
                info!("[Backend Delete] Deleting table folder: {}", folder_path.display());
                fs::remove_dir_all(&folder_path).map_err(|e| CommandError::from(format!("Failed to delete table folder {}: {}", folder_path.display(), e)))?;
            } else {
                warn!("[Backend Delete] Table folder {} not found for project_id {}. Assuming already deleted.", folder_path.display(), project_id_for_db);
            }

            if let Err(e) = db_handler::delete_asset_metadata(&project_id_for_db, &item_relative_path) {
                warn!("[Backend Delete Table] Failed to delete asset metadata from DB for project_id {}, table {}: {}", project_id_for_db, item_relative_path, e);
            } else {
                info!("[Backend Delete Table] Deleted asset metadata from DB for project_id {}, table {}", project_id_for_db, item_relative_path);
            }

            info!("[Backend Delete] Updating XML to remove table link with path '{}'", item_relative_path);
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            let initial_table_len = project_data.table_files.files.len();
            project_data.table_files.files.retain(|t| t.relative_path != item_relative_path);
            if project_data.table_files.files.len() < initial_table_len {
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Delete] Table entry removed from XML.");
            } else {
                warn!("[Backend Delete] Deleted table file/folder, but no matching entry found in XML for path '{}'.", item_relative_path);
            }
        },
        "image" => {
            info!("[Backend Delete] Request to delete image and its folder for: {}", item_path_buf.display());
            let image_folder_to_delete = item_path_buf.parent().ok_or_else(|| {
                CommandError::from(format!(
                    "Could not get parent directory for image file: {}",
                    item_path_buf.display()
                ))
            })?;

            if image_folder_to_delete.exists() && image_folder_to_delete.is_dir() {
                info!("[Backend Delete] Deleting image folder: {}", image_folder_to_delete.display());
                fs::remove_dir_all(image_folder_to_delete).map_err(|e| {
                    CommandError::from(format!(
                        "Failed to delete image folder {}: {}",
                        image_folder_to_delete.display(),
                        e
                    ))
                })?;
            } else {
                warn!(
                    "[Backend Delete] Image folder {} not found. Assuming already deleted or structure is unexpected. Proceeding with XML cleanup.",
                    image_folder_to_delete.display()
                );
            }

            if let Err(db_err) = delete_annotations_from_db(&project_id_for_db, &item_relative_path, "image") {
                warn!("[Backend Delete] Failed to delete image annotations from DB for project_id {}, image {}: {}. File deletion proceeded.", project_id_for_db, item_relative_path, db_err);
            }

            if let Err(e) = db_handler::delete_asset_metadata(&project_id_for_db, &item_relative_path) {
                warn!("[Backend Delete Image] Failed to delete asset metadata from DB for project_id {}, image {}: {}", project_id_for_db, item_relative_path, e);
            } else {
                info!("[Backend Delete Image] Deleted asset metadata from DB for project_id {}, image {}", project_id_for_db, item_relative_path);
            }

            info!("[Backend Delete] Updating XML to remove image entry '{}'", item_relative_path);
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            let initial_len = project_data.image_files.files.len();
            project_data.image_files.files.retain(|i| i.relative_path != item_relative_path);

            if project_data.image_files.files.len() < initial_len {
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Delete] XML updated for image.");
            } else {
                warn!("[Backend Delete] Deleted image folder (or it was already gone), but no matching entry found in XML for path '{}'.", item_relative_path);
            }
        },
        _ => {
            error!("[Backend Delete] Deleting items of type '{}' is not supported directly: {}", item_type, item_path);
            return Err(CommandError::from(format!("Deletion not supported for item type '{}'. Delete the primary associated asset.", item_type)));
        }
    }

    info!("[Backend Delete] Success for: {}", item_path);
    Ok(())
}


#[tauri::command]
pub async fn rename_project_item( app_handle: tauri::AppHandle, item_path: String, new_name: String, project_xml_path: String) -> Result<(), CommandError> {
    info!("[Backend Rename] Request: Item='{}', NewNameParam='{}'", item_path, new_name);
    let item_path_buf = PathBuf::from(&item_path);
    let xml_path_buf = PathBuf::from(&project_xml_path);
    let new_name_trimmed = new_name.trim();

    if !item_path_buf.exists() {
        return Err(CommandError::from(format!("Item not found: {}", item_path)));
    }
    if new_name_trimmed.is_empty() {
        return Err(CommandError::from("New name cannot be empty."));
    }
    if !xml_path_buf.exists() || !xml_path_buf.is_file() {
        return Err(CommandError::from(format!("Project XML not found: {}", project_xml_path)));
    }
    let project_base_dir = xml_path_buf.parent().ok_or_else(|| CommandError::from("Could not get project base dir"))?;

    // Get project_id for DB operations
    let project_xml_content_for_uuid = fs::read_to_string(&xml_path_buf)
        .map_err(|e| CommandError::Io(format!("Failed to read project XML for UUID from {}: {}", xml_path_buf.display(), e)))?;
    let project_data_for_uuid: ProjectXml = quick_xml::de::from_str(&project_xml_content_for_uuid)
        .map_err(|e| CommandError::XmlDeserialization(format!("Failed to parse project XML for UUID from {}: {}", xml_path_buf.display(), e)))?;
    let project_id_for_db = project_data_for_uuid.project_uuid;
    if project_id_for_db.is_empty() {
        error!("[Backend Rename] Project UUID is empty in XML file: {}. Cannot proceed with DB operations.", xml_path_buf.display());
        return Err(CommandError::Message(format!("Project ID (UUID) is missing in the project file ({}). DB operations cannot proceed.", xml_path_buf.display())));
    }
    info!("[Backend Rename] Operating with project_id: {}", project_id_for_db);

    if item_path_buf.is_dir() {
        let (item_type, _, _) = get_item_details(&item_path_buf, project_base_dir)?;
        if item_type != "directory_media_stem" {
             return Err(CommandError::from(format!("Renaming arbitrary directories ('{}') is not supported via this function. Rename the associated asset file instead.", item_type)));
        }
         warn!("[Backend Rename] Request path '{}' is a directory, but rename should be triggered by media file. Proceeding with media logic.", item_path);
    }

    let contains_invalid_chars = |name: &str| name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']);
    let (item_type, media_stem_opt, item_relative_path_buf) = get_item_details(&item_path_buf, project_base_dir)?;
    let item_relative_path = item_relative_path_buf.to_string_lossy().replace("\\", "/");
    info!("[Backend Rename] Item type: '{}', Media Stem: {:?}, Rel Path: '{}'", item_type, media_stem_opt, item_relative_path);

    let parent_dir = item_path_buf.parent().ok_or_else(|| CommandError::from(format!("Could not get parent directory for {}", item_path_buf.display())))?;

    match item_type.as_str() {
        "media" => {
             let old_stem = media_stem_opt.ok_or_else(|| CommandError::from("Could not get media stem identifier for rename."))?;
            let original_extension = item_path_buf.extension().and_then(|s| s.to_str()).unwrap_or("");
            let new_stem = if new_name_trimmed.contains('.') {
                Path::new(new_name_trimmed).file_stem().and_then(|s| s.to_str()).unwrap_or(new_name_trimmed).to_string()
            } else {
                new_name_trimmed.to_string()
            };

            info!("[Backend Rename] Media Rename: OldStem='{}', NewStem='{}'", old_stem, new_stem);

            if contains_invalid_chars(&new_stem) {
                return Err(CommandError::from("New media name contains invalid characters."));
            }
            if new_stem == old_stem {
                 info!("[Backend Rename] New name is same as old name. No action needed.");
                 return Ok(());
            }

            let media_asset_dir = project_base_dir.join(HARVEY_FILES_DIR).join(MEDIA_DIR);
            let old_stem_dir_path = media_asset_dir.join(&old_stem);
            let new_stem_dir_path = media_asset_dir.join(&new_stem);

            if new_stem_dir_path.exists() {
                return Err(CommandError::from(format!("A media project named '{}' already exists.", new_stem)));
            }

            info!("[Backend Rename] Renaming dir {} -> {}", old_stem_dir_path.display(), new_stem_dir_path.display());
            fs::rename(&old_stem_dir_path, &new_stem_dir_path).map_err(|e| CommandError::from(format!("Failed to rename media directory: {}", e)))?;

            let new_media_subdir = new_stem_dir_path.join(MEDIA_SUBDIR);
            let old_filename_in_new_dir = format!("{}.{}", old_stem, original_extension);
            let new_filename = format!("{}.{}", new_stem, original_extension);
            let old_media_path_in_new_dir = new_media_subdir.join(old_filename_in_new_dir);
            let new_media_path = new_media_subdir.join(&new_filename);
            let primary_media_new_relative_path;

            if old_media_path_in_new_dir.exists() {
                info!("[Backend Rename] Renaming media file {} -> {}", old_media_path_in_new_dir.display(), new_media_path.display());
                if let Err(e) = fs::rename(&old_media_path_in_new_dir, &new_media_path) {
                    warn!("Failed rename media file: {}. Reverting directory rename.", e);
                    let _ = fs::rename(&new_stem_dir_path, &old_stem_dir_path);
                    return Err(CommandError::from(format!("Failed to rename internal media file: {}", e)));
                }
                primary_media_new_relative_path = Path::new(HARVEY_FILES_DIR).join(MEDIA_DIR).join(&new_stem).join(MEDIA_SUBDIR).join(&new_filename).to_string_lossy().replace("\\", "/");
            } else {
                warn!("[Backend Rename] Media file not found at expected path {} inside renamed directory {}. Reverting directory rename.", old_media_path_in_new_dir.display(), new_stem_dir_path.display());
                let _ = fs::rename(&new_stem_dir_path, &old_stem_dir_path);
                return Err(CommandError::from(format!("Original media file structure inconsistent after directory rename. Expected file at {}", old_media_path_in_new_dir.display())));
            }

            let old_metadata_path_result = get_media_metadata_path(&old_media_path_in_new_dir);
            let new_metadata_path_result = get_media_metadata_path(&new_media_path);

            match (old_metadata_path_result, new_metadata_path_result) {
                (Ok(old_metadata_path), Ok(new_metadata_path)) => {
                    let mut metadata_content: Option<StandardAssetMetadata> = None;

                    if old_metadata_path.exists() {
                        info!("[Backend Rename] Attempting to read old media metadata file: {}", old_metadata_path.display());
                        match fs::read_to_string(&old_metadata_path) {
                            Ok(old_json_content) => {
                                match serde_json::from_str::<StandardAssetMetadata>(&old_json_content) {
                                    Ok(mut parsed_metadata) => {
                                        parsed_metadata.metadata.file_name = new_media_path.file_name()
                                            .and_then(|s| s.to_str()).unwrap_or("").to_string();
                                        parsed_metadata.metadata.file_path = new_media_path.to_string_lossy().into_owned();
                                        parsed_metadata.metadata.last_modified = Utc::now().to_rfc3339();
                                        metadata_content = Some(parsed_metadata);
                                        info!("[Backend Rename] Successfully parsed and updated old media metadata.");
                                    }
                                    Err(e) => {
                                        warn!("[Backend Rename] Failed to parse old media metadata file {}: {}. A new one will be created.", old_metadata_path.display(), e);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("[Backend Rename] Failed to read old media metadata file {}: {}. A new one will be created.", old_metadata_path.display(), e);
                            }
                        }
                        if let Err(e) = fs::remove_file(&old_metadata_path) {
                            warn!("[Backend Rename] Failed to remove old media metadata file {}: {}", old_metadata_path.display(), e);
                        }
                    } else {
                        info!("[Backend Rename] Old media metadata file {} not found. A new one will be created.", old_metadata_path.display());
                    }

                    let final_metadata_to_write = metadata_content.unwrap_or_else(|| {
                        info!("[Backend Rename] Creating new media metadata content for {}.", new_media_path.display());
                        StandardAssetMetadata {
                            metadata: FileMetadata {
                                file_name: new_media_path.file_name()
                                    .and_then(|s| s.to_str()).unwrap_or("").to_string(),
                                file_path: new_media_path.to_string_lossy().into_owned(),
                                last_modified: Utc::now().to_rfc3339(),
                                title: "".to_string(),
                                description: "".to_string(),
                                summary: "".to_string(),
                                duration_seconds: None,
                                width: None,
                                height: None,
                                frame_rate: None,
                                bit_rate: None,
                                audio_codec: None,
                                video_codec: None,
                                created_at: None,
                            },
                            highlights: Vec::new(),
                        }
                    });

                    match serde_json::to_string_pretty(&final_metadata_to_write) {
                        Ok(json_string) => {
                            if let Err(e) = fs::write(&new_metadata_path, json_string) {
                                warn!("[Backend Rename] Failed to write media metadata file {}: {}", new_metadata_path.display(), e);
                            } else {
                                info!("[Backend Rename] Successfully wrote media metadata to {}", new_metadata_path.display());
                            }
                        }
                        Err(e) => {
                            warn!("[Backend Rename] Failed to serialize media metadata for {}: {}", new_metadata_path.display(), e);
                        }
                    }
                }
                (Err(e_old), _) => {
                    warn!("[Backend Rename] Could not determine old media metadata path for {}: {:?}", old_media_path_in_new_dir.display(), e_old);
                }
                (_, Err(e_new)) => {
                    warn!("[Backend Rename] Could not determine new media metadata path for {}: {:?}", new_media_path.display(), e_new);
                }
            }

            let old_transcript_filename = format!("{}.json", old_stem);
            let new_transcript_filename = format!("{}.json", new_stem);
            let transcript_subdir_in_new_stem = new_stem_dir_path.join(TRANSCRIPTS_SUBDIR);
            let old_transcript_path_in_new_dir = transcript_subdir_in_new_stem.join(&old_transcript_filename);
            let new_transcript_path = transcript_subdir_in_new_stem.join(&new_transcript_filename);

            if old_transcript_path_in_new_dir.exists() {
                info!("[Backend Rename] Renaming primary transcript {} -> {}", old_transcript_path_in_new_dir.display(), new_transcript_path.display());
                 if let Err(e) = fs::rename(&old_transcript_path_in_new_dir, &new_transcript_path) {
                     warn!("Failed rename primary transcript: {}. Reverting directory and media file renames.", e);
                     let _ = fs::rename(&new_media_path, &old_media_path_in_new_dir);
                     let _ = fs::rename(&new_stem_dir_path, &old_stem_dir_path);
                     return Err(CommandError::from(format!("Failed to rename primary transcript file: {}", e)));
                 }
            } else {
                info!("[Backend Rename] Primary transcript {} not found, skipping transcript rename.", old_transcript_path_in_new_dir.display());
            }

            info!("[Backend Rename] Updating XML: ID '{}' -> '{}', Path -> '{}'", old_stem, new_stem, primary_media_new_relative_path);
            let xml_content = fs::read_to_string(&xml_path_buf)?;
            let mut project_data: ProjectXml = quick_xml::de::from_str(&xml_content)?;
            if let Some(entry) = project_data.media_files.files.iter_mut().find(|f| f.name == old_stem) {
                entry.name = new_stem.clone();
                entry.relative_path = primary_media_new_relative_path.clone();

                for transcript_entry in entry.transcripts.iter_mut() {
                    let old_t_path = PathBuf::from(&transcript_entry.relative_path);
                    if let Some(t_filename) = old_t_path.file_name().and_then(|n| n.to_str()) {
                        let new_t_filename = if t_filename == old_transcript_filename {
                            new_transcript_filename.clone()
                        } else {
                            t_filename.to_string()
                        };
                         let new_t_relative_path = Path::new(HARVEY_FILES_DIR).join(MEDIA_DIR).join(&new_stem).join(TRANSCRIPTS_SUBDIR).join(&new_t_filename).to_string_lossy().replace("\\", "/");

                        debug!("[Backend Rename XML] Updating transcript path from '{}' to '{}'", transcript_entry.relative_path, new_t_relative_path);
                        transcript_entry.relative_path = new_t_relative_path;
                        if t_filename == old_transcript_filename {
                            transcript_entry.name = new_transcript_filename.clone();
                        }
                    } else {
                        warn!("[Backend Rename XML] Could not parse filename from transcript relative path: {}", transcript_entry.relative_path);
                    }
                }
                entry.transcripts.sort_by(|a,b| a.name.cmp(&b.name));

                info!("[Backend Rename] XML entry updated.");
                project_data.media_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Rename] XML saved.");

                let payload = MediaRenamedPayload {
                    old_media_stem: old_stem.clone(),
                    new_media_stem: new_stem.clone(),
                    new_media_file_relative_path: primary_media_new_relative_path.clone(),
                    new_absolute_path: new_media_path.to_string_lossy().into_owned(),
                };
                if let Err(e) = app_handle.emit("media_renamed", payload) {
                    warn!("[Backend Rename] Failed to emit media_renamed event for new stem {}: {}", new_stem, e);
                }

            } else {
                error!("[Backend Rename] CRITICAL: Failed find XML entry for '{}' after file operations. File system may be inconsistent.", old_stem);
                return Err(CommandError::from(format!("XML entry for '{}' not found after successful file renames. Project state potentially inconsistent.", old_stem)));
            }
        },
        "transcript" => {
            let new_filename_with_ext = new_name_trimmed;
            let new_path = parent_dir.join(new_filename_with_ext);

            if contains_invalid_chars(new_filename_with_ext) { return Err(CommandError::from("New filename contains invalid characters.")); }
            if !new_filename_with_ext.ends_with(".json") { return Err(CommandError::from("Transcript filename must end with .json")); }
            if new_filename_with_ext.starts_with('.') { return Err(CommandError::from("Filename cannot start with a dot.")); }

            if item_path_buf == new_path { info!("[Backend Rename] New path is same as old path. No action needed."); return Ok(()); }

            if new_path.exists() {
                 let canon_old = fs::canonicalize(&item_path_buf).ok();
                 let canon_new = fs::canonicalize(&new_path).ok();
                 if canon_old.is_some() && canon_new.is_some() && canon_old != canon_new {
                     return Err(CommandError::from(format!("File named '{}' already exists.", new_filename_with_ext)));
                 } else {
                     debug!("[Backend Rename] Target path exists but might be same file (case change?). Allowing rename attempt.");
                 }
            }

            info!("[Backend Rename] Renaming transcript file {} -> {}", item_path_buf.display(), new_path.display());
            fs::rename(&item_path_buf, &new_path).map_err(|e| CommandError::from(format!("Failed to rename file: {}", e)))?;

            let media_identifier = media_stem_opt.ok_or_else(|| CommandError::from("Could not determine media stem for transcript rename."))?;
            let new_relative_path_buf = new_path.strip_prefix(project_base_dir)?;
            let new_relative_path = new_relative_path_buf.to_string_lossy().replace("\\", "/");

            info!("[Backend Rename] Updating XML for media '{}': Path '{}' -> '{}', name -> '{}'", media_identifier, item_relative_path, new_relative_path, new_filename_with_ext);
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            let mut xml_changed = false;

            if let Some(media_entry) = project_data.media_files.files.iter_mut().find(|f| f.name == media_identifier) {
                if let Some(transcript_entry) = media_entry.transcripts.iter_mut().find(|t| t.relative_path == item_relative_path) {
                    transcript_entry.name = new_filename_with_ext.to_string();
                    transcript_entry.relative_path = new_relative_path;
                    media_entry.transcripts.sort_by(|a,b| a.name.cmp(&b.name));
                    xml_changed = true;
                    info!("[Backend Rename] XML transcript entry updated.");
                } else {
                    warn!("[Backend Rename] Renamed transcript file, but could not find matching path '{}' in XML under media '{}'.", item_relative_path, media_identifier);
                }
            } else {
                warn!("[Backend Rename] Renamed transcript file, but could not find media ID '{}' in XML.", media_identifier);
            }

            if xml_changed {
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Rename] XML saved.");

                let payload = ItemRenamedPayload {
                    old_path: item_path_buf.to_string_lossy().into_owned(),
                    new_path: new_path.to_string_lossy().into_owned(),
                    new_name: new_filename_with_ext.to_string(),
                    item_type: "transcript".to_string(),
                    project_xml_path: xml_path_buf.to_string_lossy().into_owned(),
                    base_directory: project_base_dir.to_string_lossy().into_owned(),
                };
                if let Err(e) = app_handle.emit("item_renamed", payload) {
                    warn!("[Backend Rename] Failed to emit item_renamed event for transcript: {}", e);
                }
            }
        },
        "imported_transcript" => {
            let old_transcript_file_abs_path = &item_path_buf;
            let old_transcript_folder_abs_path = parent_dir;
            let old_transcript_relative_path = &item_relative_path; // This is key for DB

            let new_transcript_stem_str = new_name_trimmed;
            if contains_invalid_chars(new_transcript_stem_str) { return Err(CommandError::from("New transcript name contains invalid characters.")); }
            if new_transcript_stem_str.starts_with('.') { return Err(CommandError::from("Transcript name cannot start with a dot.")); }

            let new_transcript_filename_with_ext_str = format!("{}.json", new_transcript_stem_str);
            let new_transcript_filename_pathbuf = PathBuf::from(&new_transcript_filename_with_ext_str);

            let new_transcript_file_path_in_old_folder = old_transcript_folder_abs_path.join(&new_transcript_filename_pathbuf);

            let transcripts_root_abs_path = old_transcript_folder_abs_path.parent()
                .ok_or_else(|| CommandError::from(format!("Could not get Transcripts root from {}", old_transcript_folder_abs_path.display())))?;
            
            let new_transcript_folder_abs_path = transcripts_root_abs_path.join(new_transcript_stem_str);

            // Check if no effective change
            if *old_transcript_file_abs_path == new_transcript_file_path_in_old_folder && old_transcript_folder_abs_path == &new_transcript_folder_abs_path {
                info!("[Backend Rename] Imported transcript name and folder name are effectively unchanged. No action needed.");
                return Ok(());
            }

            // Check for conflicts
            if old_transcript_folder_abs_path != &new_transcript_folder_abs_path && new_transcript_folder_abs_path.exists() {
                return Err(CommandError::from(format!("A folder named '{}' already exists for imported transcripts. Cannot rename folder.", new_transcript_stem_str)));
            }
            let final_new_transcript_file_abs_path = new_transcript_folder_abs_path.join(&new_transcript_filename_pathbuf);
            if final_new_transcript_file_abs_path.exists() {
                let canon_old_abs = fs::canonicalize(old_transcript_file_abs_path).map_err(|e| CommandError::from(format!("Cannot canonicalize old transcript path {}: {}", old_transcript_file_abs_path.display(), e)))?;
                let canon_final_target_abs = fs::canonicalize(&final_new_transcript_file_abs_path).map_err(|e| CommandError::from(format!("Cannot canonicalize final target transcript path {}: {}", final_new_transcript_file_abs_path.display(), e)))?;
                if canon_final_target_abs != canon_old_abs {
                    return Err(CommandError::from(format!("An imported transcript file named '{}' already exists in the target location '{}'.", new_transcript_filename_with_ext_str, new_transcript_folder_abs_path.display())));
                 }
            }

            // 1. Rename the main transcript file (if its name changes within the folder)
            if old_transcript_file_abs_path != &new_transcript_file_path_in_old_folder {
                info!("[Backend Rename] Renaming imported transcript file {} -> {}", old_transcript_file_abs_path.display(), new_transcript_file_path_in_old_folder.display());
                fs::rename(old_transcript_file_abs_path, &new_transcript_file_path_in_old_folder)
                    .map_err(|e| CommandError::from(format!("Failed to rename imported transcript file: {}", e)))?;
            }

            // Current path of the transcript file after potential rename, still in old folder if folder name changes
            let current_transcript_path_before_folder_rename = new_transcript_file_path_in_old_folder.clone();

            // 2. Rename the folder (if stem changes)
            if old_transcript_folder_abs_path != &new_transcript_folder_abs_path {
                info!("[Backend Rename] Renaming imported transcript folder {} -> {}", old_transcript_folder_abs_path.display(), new_transcript_folder_abs_path.display());
                if let Err(e) = fs::rename(old_transcript_folder_abs_path, &new_transcript_folder_abs_path) {
                    warn!("[Backend Rename] Failed to rename imported transcript folder: {}. Attempting to revert file rename.", e);
                    if old_transcript_file_abs_path != &current_transcript_path_before_folder_rename && current_transcript_path_before_folder_rename.exists() {
                        let _ = fs::rename(&current_transcript_path_before_folder_rename, old_transcript_file_abs_path);
                    }
                    return Err(CommandError::from(format!("Failed to rename imported transcript folder: {}", e)));
                }
            }

            // final_new_transcript_file_abs_path is the ultimate new absolute path
            // new_transcript_filename_with_ext_str is the new filename "new_stem.json"
            let new_relative_path_for_xml_and_db = final_new_transcript_file_abs_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\", "/");

            // 3. Update database entry
            if let Err(e) = db_handler::rename_asset_metadata_key(
                &project_id_for_db,
                old_transcript_relative_path, // old key
                &new_relative_path_for_xml_and_db, // new key
                &final_new_transcript_file_abs_path.to_string_lossy(), // new full file_path field value
                &new_transcript_filename_with_ext_str, // new file_name field value
            ) {
                warn!("[Backend Rename] Failed to rename/update asset metadata in DB for project_id {}, imported transcript {} -> {}: {}. File system changes were successful. Attempting to revert FS changes.", project_id_for_db, old_transcript_relative_path, new_relative_path_for_xml_and_db, e);
                // Attempt to revert FS operations (best effort)
                if old_transcript_folder_abs_path != &new_transcript_folder_abs_path && new_transcript_folder_abs_path.exists() { // if folder was renamed
                    let _ = fs::rename(&new_transcript_folder_abs_path, old_transcript_folder_abs_path); // revert folder rename
                     // After folder revert, the file is at old_transcript_folder_abs_path.join(new_transcript_filename_pathbuf) if it was renamed
                    let path_after_folder_revert = old_transcript_folder_abs_path.join(new_transcript_filename_pathbuf);
                    if path_after_folder_revert.exists() && path_after_folder_revert != *old_transcript_file_abs_path {
                         let _ = fs::rename(path_after_folder_revert, old_transcript_file_abs_path); // revert file rename
                    }
                } else if old_transcript_file_abs_path != &current_transcript_path_before_folder_rename && current_transcript_path_before_folder_rename.exists() { // if only file was renamed
                     let _ = fs::rename(&current_transcript_path_before_folder_rename, old_transcript_file_abs_path); // revert file rename
                }
                return Err(CommandError::from(format!("Failed to update transcript metadata in DB: {}. File system changes attempted to be reverted.", e)));
            } else {
                info!("[Backend Rename] Successfully renamed/updated asset metadata in DB for imported transcript {} -> {}", old_transcript_relative_path, new_relative_path_for_xml_and_db);
            }

            // 4. Update Project XML
            // The .metadata.json file is no longer managed in XML, so no need to update DocumentMetadataEntryXml.
            info!("[Backend Rename] Updating XML for imported transcript: OldRelPath '{}', NewRelPath '{}', NewName '{}'", item_relative_path, new_relative_path_for_xml_and_db, new_transcript_filename_with_ext_str);
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            // Removed: let mut xml_actually_changed_for_imported_transcript = false;

            if let Some(entry) = project_data.imported_transcript_files.files.iter_mut().find(|t| t.relative_path == *old_transcript_relative_path) {
                entry.name = new_transcript_filename_with_ext_str.clone();
                entry.relative_path = new_relative_path_for_xml_and_db.clone();
                project_data.imported_transcript_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                // xml_actually_changed_for_imported_transcript = true; // Variable removed
                info!("[Backend Rename] XML imported transcript entry updated. Saving XML.");
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Rename] XML saved for imported transcript rename.");

                let payload = ItemRenamedPayload {
                    old_path: item_path_buf.to_string_lossy().into_owned(),
                    new_path: final_new_transcript_file_abs_path.to_string_lossy().into_owned(),
                    new_name: new_transcript_filename_with_ext_str.clone(),
                    item_type: "imported_transcript".to_string(),
                    project_xml_path: xml_path_buf.to_string_lossy().into_owned(),
                    base_directory: project_base_dir.to_string_lossy().into_owned(),
                };
                if let Err(e) = app_handle.emit("item_renamed", payload) {
                    warn!("[Backend Rename] Failed to emit item_renamed event for imported_transcript: {}", e);
                }
            } else {
                // This should ideally not happen if DB update was successful, as it means XML was out of sync.
                warn!("[Backend Rename] Renamed imported transcript (FS & DB), but could not find matching old relative path '{}' in XML. XML not saved.", old_transcript_relative_path);
            }
            // Logic for updating project_data.document_metadata_files.files is REMOVED.
            // The conditional save based on the flag is removed; save now happens inside the 'if let Some(entry)' block.
        },
        "doc" => {
            let new_filename_with_ext_str = new_name_trimmed;
            let new_filename_pathbuf = PathBuf::from(new_filename_with_ext_str);

            if contains_invalid_chars(new_filename_with_ext_str) { return Err(CommandError::from("New filename contains invalid chars.")); }
            let allowed_extensions = ["json", "md", "txt", "pdf"];
            let new_ext = new_filename_pathbuf.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if !allowed_extensions.contains(&new_ext.as_str()) {
                 return Err(CommandError::from(format!("Invalid extension '.{}'. Allowed extensions for documents are: {:?}", new_ext, allowed_extensions)));
            }
            let old_ext = item_path_buf.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
             if old_ext != new_ext {
                  return Err(CommandError::from(format!("Changing document file extension from '.{}' to '.{}' is not allowed.", old_ext, new_ext)));
             }
            if new_filename_with_ext_str.starts_with('.') &&
               !new_filename_with_ext_str.ends_with(METADATA_FILE_SUFFIX) {
                return Err(CommandError::from("Document filename cannot start with a dot unless it's a designated metadata or annotation file."));
            }

            let old_doc_file_path = &item_path_buf;
            let old_doc_folder_path = parent_dir;
            
            let new_doc_file_path_in_old_folder = old_doc_folder_path.join(&new_filename_pathbuf);
            
            let new_doc_filename_stem = new_filename_pathbuf.file_stem().and_then(|s| s.to_str())
                .ok_or_else(|| CommandError::from(format!("Could not get new document file stem from {}", new_filename_pathbuf.display())))?;

            let documents_root_path = old_doc_folder_path.parent()
                .ok_or_else(|| CommandError::from(format!("Could not get documents root from {}", old_doc_folder_path.display())))?;
            
            let new_doc_folder_path = documents_root_path.join(new_doc_filename_stem);

            if *old_doc_file_path == new_doc_file_path_in_old_folder && old_doc_folder_path == &new_doc_folder_path {
                info!("[Backend Rename] Document name and folder name are effectively unchanged. No action needed.");
                return Ok(());
            }

            if old_doc_folder_path != &new_doc_folder_path && new_doc_folder_path.exists() {
                return Err(CommandError::from(format!("A folder named '{}' already exists. Cannot rename document folder.", new_doc_filename_stem)));
            }
            
            let final_target_doc_file_path = new_doc_folder_path.join(&new_filename_pathbuf);
            if final_target_doc_file_path.exists() {
                 let canon_old_abs = fs::canonicalize(old_doc_file_path).map_err(|e| CommandError::from(format!("Cannot canonicalize old path {}: {}", old_doc_file_path.display(),e)))?;
                 let canon_final_target_abs = fs::canonicalize(&final_target_doc_file_path).map_err(|e| CommandError::from(format!("Cannot canonicalize final target path {}: {}", final_target_doc_file_path.display(),e)))?;
                 if canon_final_target_abs != canon_old_abs {
                     return Err(CommandError::from(format!("A file named '{}' already exists in the target location '{}'.", new_filename_with_ext_str, new_doc_folder_path.display())));
                 }
            }

            if old_doc_file_path != &new_doc_file_path_in_old_folder {
                info!("[Backend Rename] Renaming document file {} -> {}", old_doc_file_path.display(), new_doc_file_path_in_old_folder.display());
                fs::rename(old_doc_file_path, &new_doc_file_path_in_old_folder)
                    .map_err(|e| CommandError::from(format!("Failed to rename document file: {}", e)))?;
            }

            if let Ok(old_app_metadata_path) = get_document_metadata_path_for_doc(old_doc_file_path) {
                if old_app_metadata_path.exists() {
                    if let Ok(new_app_metadata_path_in_old_folder) = get_document_metadata_path_for_doc(&new_doc_file_path_in_old_folder) { 
                        if old_app_metadata_path != new_app_metadata_path_in_old_folder {
                            info!("[Backend Rename] Renaming app metadata: {} -> {}", old_app_metadata_path.display(), new_app_metadata_path_in_old_folder.display());
                            if new_app_metadata_path_in_old_folder.exists() {
                                warn!("[Backend Rename] Target app metadata {} already exists. Skipping rename of {}.", new_app_metadata_path_in_old_folder.display(), old_app_metadata_path.display());
                            } else {
                                if let Err(e) = fs::rename(&old_app_metadata_path, &new_app_metadata_path_in_old_folder) {
                                    warn!("[Backend Rename] Failed to rename app metadata: {}. Attempting to revert main doc rename.", e);
                                    if old_doc_file_path != &new_doc_file_path_in_old_folder {
                                        let _ = fs::rename(&new_doc_file_path_in_old_folder, old_doc_file_path);
                                    }
                                    return Err(CommandError::from(format!("Failed to rename app metadata: {}", e)));
                                }
                            }
                        }
                    }
                }
            }
            
            // PDF Annotation file system rename is no longer handled here. DB call handles the rename.

            if old_ext == "pdf" {
                let temp_final_new_doc_file_abs_path = new_doc_folder_path.join(&new_filename_pathbuf);
                let temp_new_relative_path_for_doc = temp_final_new_doc_file_abs_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\", "/");
                if let Err(db_err) = rename_annotations_in_db(&project_id_for_db, &item_relative_path, &temp_new_relative_path_for_doc, "pdf") {
                    warn!("[Backend Rename] Failed to rename PDF annotations in DB for project_id {} from {} to {}: {}. File operations will proceed, but DB might be inconsistent.", project_id_for_db, item_relative_path, temp_new_relative_path_for_doc, db_err);
                }
            }

            let mut current_doc_folder_path_for_xml_update = old_doc_folder_path.clone();
            if old_doc_folder_path != &new_doc_folder_path {
                info!("[Backend Rename] Renaming document folder {} -> {}", old_doc_folder_path.display(), new_doc_folder_path.display());
                if let Err(e) = fs::rename(old_doc_folder_path, &new_doc_folder_path) {
                    warn!("[Backend Rename] Failed to rename document folder for project_id {}: {}. Attempting to revert file renames.", project_id_for_db, e);
                    if old_ext == "pdf" {
                        let temp_final_new_doc_file_abs_path_for_revert = new_doc_folder_path.join(&new_filename_pathbuf);
                        let temp_new_relative_path_for_doc_for_revert = temp_final_new_doc_file_abs_path_for_revert.strip_prefix(project_base_dir).map_err(|_| CommandError::from("Path stripping error during revert calc"))?.to_string_lossy().replace("\\", "/");
                        if rename_annotations_in_db(&project_id_for_db, &temp_new_relative_path_for_doc_for_revert, &item_relative_path, "pdf").is_ok() {
                             warn!("[Backend Rename] Successfully reverted PDF annotation rename in DB (project_id {}) during folder rename failure.", project_id_for_db);
                        } else {
                             warn!("[Backend Rename] Failed to revert PDF annotation rename in DB (project_id {}) during folder rename failure. DB might be inconsistent.", project_id_for_db);
                        }
                    }
                    if let Ok(old_app_meta_p) = get_document_metadata_path_for_doc(old_doc_file_path) {
                        if let Ok(new_app_meta_p_temp) = get_document_metadata_path_for_doc(&new_doc_file_path_in_old_folder) {
                            if old_app_meta_p != new_app_meta_p_temp && new_app_meta_p_temp.exists() { let _ = fs::rename(&new_app_meta_p_temp, &old_app_meta_p); }
                        }
                    }
                    if old_doc_file_path != &new_doc_file_path_in_old_folder && new_doc_file_path_in_old_folder.exists() {
                        let _ = fs::rename(&new_doc_file_path_in_old_folder, old_doc_file_path);
                    }
                    return Err(CommandError::from(format!("Failed to rename document folder: {}", e)));
                }
                current_doc_folder_path_for_xml_update = &new_doc_folder_path;
            }

            let final_new_doc_file_abs_path = current_doc_folder_path_for_xml_update.join(&new_filename_pathbuf);
            let new_relative_path_for_doc = final_new_doc_file_abs_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\", "/");

            let mut new_app_metadata_relative_path_for_xml: Option<String> = None;
            if let Ok(final_new_app_metadata_abs_path) = get_document_metadata_path_for_doc(&final_new_doc_file_abs_path) {
                if final_new_app_metadata_abs_path.exists() {
                     new_app_metadata_relative_path_for_xml = Some(final_new_app_metadata_abs_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\","/"));
                }
            }

            info!("[Backend Rename] Updating XML for document: OldRelPath '{}', NewRelPath '{}', NewName '{}'", item_relative_path, new_relative_path_for_doc, new_filename_with_ext_str);
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            let mut actual_changes_made_to_doc_xml = false;

            if let Some(doc_entry) = project_data.document_files.files.iter_mut().find(|d| d.relative_path == item_relative_path) {
                doc_entry.name = new_filename_with_ext_str.to_string();
                doc_entry.relative_path = new_relative_path_for_doc.clone();
                actual_changes_made_to_doc_xml = true;
                info!("[Backend Rename] XML document entry updated.");
            } else {
                warn!("[Backend Rename] Renamed document, but could not find matching old relative path '{}' in XML for main doc.", item_relative_path);
            }

            if let Some(new_rel_meta_path) = new_app_metadata_relative_path_for_xml {
                 if let Some(metadata_entry) = project_data.document_metadata_files.files.iter_mut().find(|m| m.original_document_relative_path == item_relative_path) {
                    let new_meta_filename = PathBuf::from(&new_rel_meta_path).file_name().unwrap_or_default().to_string_lossy().to_string();
                    metadata_entry.name = new_meta_filename;
                    metadata_entry.original_document_relative_path = new_relative_path_for_doc.clone();
                    metadata_entry.relative_path = new_rel_meta_path;
                    actual_changes_made_to_doc_xml = true;
                    info!("[Backend Rename] XML document app metadata entry updated.");
                } else {
                     warn!("[Backend Rename] App metadata file renamed/moved, but could not find matching old original_document_relative_path '{}' in XML for metadata.", item_relative_path);
                }
            }
            
            if actual_changes_made_to_doc_xml {
                project_data.document_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                project_data.document_metadata_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Rename] XML saved for document and its associated files.");

                let payload = ItemRenamedPayload {
                    old_path: item_path_buf.to_string_lossy().into_owned(),
                    new_path: final_new_doc_file_abs_path.to_string_lossy().into_owned(),
                    new_name: new_filename_with_ext_str.to_string(),
                    item_type: "doc".to_string(),
                    project_xml_path: xml_path_buf.to_string_lossy().into_owned(),
                    base_directory: project_base_dir.to_string_lossy().into_owned(),
                };
                if let Err(e) = app_handle.emit("item_renamed", payload) {
                    warn!("[Backend Rename] Failed to emit item_renamed event for doc: {}", e);
                }
            }
        },
        "table" => {
            let old_table_file_abs_path = item_path_buf.clone();
            let old_table_folder_abs_path = parent_dir.to_path_buf();

            let old_table_filename_str = old_table_file_abs_path.file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| CommandError::from(format!("Could not get old table filename string from {}", old_table_file_abs_path.display())))?
                .to_string();
            let _old_table_stem_str = old_table_file_abs_path.file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| CommandError::from(format!("Could not get old table stem string from {}", old_table_file_abs_path.display())))?
                .to_string();

            let new_table_filename_str = new_name_trimmed.to_string();
            let new_table_filename_pathbuf = PathBuf::from(&new_table_filename_str);
            let new_table_stem_str = new_table_filename_pathbuf.file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| CommandError::from(format!("Could not get new table stem string from {}", new_table_filename_str)))?
                .to_string();

            let tables_root_abs_path = old_table_folder_abs_path.parent()
                .ok_or_else(|| CommandError::from(format!("Could not get tables root directory from {}", old_table_folder_abs_path.display())))?;

            let new_table_folder_abs_path = tables_root_abs_path.join(&new_table_stem_str);
            let final_new_table_file_abs_path = new_table_folder_abs_path.join(&new_table_filename_str);

            // JSON metadata file logic for tables is removed. DB will be updated instead.
            // let old_asset_metadata_path = get_table_asset_metadata_path(&old_table_file_abs_path)?;
            // let new_asset_metadata_path = get_table_asset_metadata_path(&final_new_table_file_abs_path)?;

            if contains_invalid_chars(&new_table_filename_str) { return Err(CommandError::from("New table filename contains invalid characters.")); }
            let allowed_extensions = ["csv", "xlsx"];
            let new_ext = final_new_table_file_abs_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if !allowed_extensions.contains(&new_ext.as_str()) {
                return Err(CommandError::from(format!("Invalid extension '.{}'. Allowed extensions for tables are: {:?}", new_ext, allowed_extensions)));
            }
            let old_ext = old_table_file_abs_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if old_ext != new_ext {
                return Err(CommandError::from(format!("Changing table file extension from '.{}' to '.{}' is not allowed.", old_ext, new_ext)));
            }
            if new_table_filename_str.starts_with('.') { return Err(CommandError::from("Table filename cannot start with a dot.")); }

            if old_table_file_abs_path == final_new_table_file_abs_path {
                info!("[Backend Rename Table] New table path is the same as the old path. No action needed.");
                return Ok(());
            }

            if old_table_folder_abs_path != new_table_folder_abs_path && new_table_folder_abs_path.exists() {
                return Err(CommandError::from(format!("Target folder '{}' already exists. Cannot rename table folder.", new_table_folder_abs_path.display())));
            }

            if final_new_table_file_abs_path.exists() {
                let canon_old = fs::canonicalize(&old_table_file_abs_path).ok();
                let canon_target = fs::canonicalize(&final_new_table_file_abs_path).ok();
                if canon_old.is_some() && canon_target.is_some() && canon_old != canon_target {
                     return Err(CommandError::from(format!("Target table file '{}' already exists and is different from the source.", final_new_table_file_abs_path.display())));
                } else if canon_old.is_none() && canon_target.is_some() {
                     return Err(CommandError::from(format!("Target table file '{}' already exists.", final_new_table_file_abs_path.display())));
                }
                 info!("[Backend Rename Table] Target file path {} exists, but might be the same file due to case change or prior operations. Proceeding carefully.", final_new_table_file_abs_path.display());
            }

            if old_table_folder_abs_path != new_table_folder_abs_path {
                info!("[Backend Rename Table] Renaming folder {} -> {}", old_table_folder_abs_path.display(), new_table_folder_abs_path.display());
                fs::rename(&old_table_folder_abs_path, &new_table_folder_abs_path)
                    .map_err(|e| CommandError::from(format!("Failed to rename table folder: {}", e)))?;

                let current_table_file_path_after_folder_rename = new_table_folder_abs_path.join(&old_table_filename_str);

                if old_table_filename_str != new_table_filename_str {
                    info!("[Backend Rename Table] Renaming table file (post folder rename) {} -> {}", current_table_file_path_after_folder_rename.display(), final_new_table_file_abs_path.display());
                    if let Err(e) = fs::rename(&current_table_file_path_after_folder_rename, &final_new_table_file_abs_path) {
                        warn!("[Backend Rename Table] Failed to rename table file after folder rename: {}. Reverting folder rename.", e);
                        let _ = fs::rename(&new_table_folder_abs_path, &old_table_folder_abs_path);
                        return Err(CommandError::from(format!("Failed to rename table file after folder rename: {}", e)));
                    }
                }

                // File system operations for main table file and its folder
                // ... (existing logic for renaming folder and file) ...
                // This part is assumed to be complex and error-prone to fully replicate in diff,
                // the key is that `final_new_table_file_abs_path` and `new_table_filename_str` are correctly determined.
                // The old JSON metadata file logic (using get_table_asset_metadata_path, reading/writing StandardAssetMetadata)
                // is removed.
            }
            // After FS operations are successful:
            let new_relative_path_for_xml = final_new_table_file_abs_path.strip_prefix(project_base_dir)?
                .to_string_lossy().replace("\\", "/");

            // Update metadata in DB
            if let Err(e) = db_handler::rename_asset_metadata_key(
                &project_id_for_db,
                &item_relative_path, // old_relative_path (old DB key)
                &new_relative_path_for_xml, // new_relative_path (new DB key)
                &final_new_table_file_abs_path.to_string_lossy(), // new full file_path field value
                &new_table_filename_str, // new file_name field value
            ) {
                warn!("[Backend Rename Table] Failed to rename/update asset metadata in DB for project_id {}, table {} -> {}: {}. File system changes were successful and will not be reverted.", project_id_for_db, item_relative_path, new_relative_path_for_xml, e);
                // Not attempting to revert FS changes here as it's complex and might fail further.
            } else {
                info!("[Backend Rename Table] Successfully renamed/updated asset metadata in DB for project_id {}, table {} -> {}", project_id_for_db, item_relative_path, new_relative_path_for_xml);
            }

            info!("[Backend Rename Table] Updating XML: OldRelPath '{}', NewRelPath '{}', NewName '{}'", item_relative_path, new_relative_path_for_xml, new_table_filename_str);
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;

            if let Some(table_entry) = project_data.table_files.files.iter_mut().find(|t| t.relative_path == item_relative_path) {
                table_entry.name = new_table_filename_str.clone();
                table_entry.relative_path = new_relative_path_for_xml.clone();
                project_data.table_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                info!("[Backend Rename Table] XML table entry updated.");
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Rename Table] XML saved.");

                let payload = ItemRenamedPayload {
                    old_path: item_path_buf.to_string_lossy().into_owned(),
                    new_path: final_new_table_file_abs_path.to_string_lossy().into_owned(),
                    new_name: new_table_filename_str.clone(),
                    item_type: "table".to_string(),
                    project_xml_path: xml_path_buf.to_string_lossy().into_owned(),
                    base_directory: project_base_dir.to_string_lossy().into_owned(),
                };
                if let Err(e) = app_handle.emit("item_renamed", payload) {
                    warn!("[Backend Rename] Failed to emit item_renamed event for table: {}", e);
                }
            } else {
                error!("[Backend Rename Table] CRITICAL: File system operations for table rename succeeded, but could not find matching old relative path '{}' in XML. Project XML might be inconsistent.", item_relative_path);
                 return Err(CommandError::from(format!("Failed to update XML as old table entry for {} was not found after file operations. Project state may be inconsistent.", item_relative_path)));
            }
        },
        "image" => {
            let new_image_filename_with_ext_str = new_name_trimmed;
            let new_image_filename_pathbuf = PathBuf::from(new_image_filename_with_ext_str);

            if contains_invalid_chars(new_image_filename_with_ext_str) { return Err(CommandError::from("New image filename contains invalid characters.")); }
            let allowed_extensions = ["jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff"];
            let new_ext = new_image_filename_pathbuf.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if !allowed_extensions.contains(&new_ext.as_str()) {
                return Err(CommandError::from(format!("Invalid extension '.{}'. Allowed extensions for images are: {:?}", new_ext, allowed_extensions)));
            }
            let old_ext = item_path_buf.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if old_ext != new_ext {
                return Err(CommandError::from(format!("Changing image file extension from '.{}' to '.{}' is not allowed.", old_ext, new_ext)));
            }
            if new_image_filename_with_ext_str.starts_with('.') { return Err(CommandError::from("Image filename cannot start with a dot.")); }

            let old_image_file_abs_path = item_path_buf.clone();
            let old_image_folder_abs_path = parent_dir.to_path_buf();

            #[allow(unused_variables)]
            let old_image_stem_str = old_image_file_abs_path.file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| CommandError::from(format!("Could not get old image stem from {}", old_image_file_abs_path.display())))?
                .to_string();

            let new_image_stem_str = new_image_filename_pathbuf.file_stem().and_then(|s| s.to_str())
                .ok_or_else(|| CommandError::from(format!("Could not get new image file stem from {}", new_image_filename_pathbuf.display())))?
                .to_string();

            let images_root_abs_path = old_image_folder_abs_path.parent()
                .ok_or_else(|| CommandError::from(format!("Could not get images root from {}", old_image_folder_abs_path.display())))?;

            let new_image_folder_abs_path = images_root_abs_path.join(&new_image_stem_str);
            let final_new_image_file_abs_path = new_image_folder_abs_path.join(&new_image_filename_pathbuf);
            
            let new_image_file_path_in_old_folder = old_image_folder_abs_path.join(&new_image_filename_pathbuf);

            // JSON metadata file logic for images is removed. DB will be updated instead.
            // let old_asset_metadata_abs_path = get_image_asset_metadata_path(&old_image_file_abs_path)?;
            // let new_asset_metadata_abs_path = get_image_asset_metadata_path(&final_new_image_file_abs_path)?;

            if old_image_file_abs_path == final_new_image_file_abs_path {
                if old_image_folder_abs_path == new_image_folder_abs_path {
                    info!("[Backend Rename] Image name and folder name are effectively unchanged. No action needed.");
                    return Ok(());
                }
            }

            if old_image_folder_abs_path != new_image_folder_abs_path && new_image_folder_abs_path.exists() {
                return Err(CommandError::from(format!("A folder named '{}' already exists for images. Cannot rename image folder.", new_image_stem_str)));
            }

            if final_new_image_file_abs_path.exists() {
                let canon_old_abs = fs::canonicalize(&old_image_file_abs_path).map_err(|e| CommandError::from(format!("Cannot canonicalize old image path {}: {}", old_image_file_abs_path.display(), e)))?;
                let canon_final_target_abs = fs::canonicalize(&final_new_image_file_abs_path).map_err(|e| CommandError::from(format!("Cannot canonicalize final target image path {}: {}", final_new_image_file_abs_path.display(), e)))?;
                if canon_final_target_abs != canon_old_abs {
                    return Err(CommandError::from(format!("An image file named '{}' already exists in the target location '{}'.", new_image_filename_with_ext_str, new_image_folder_abs_path.display())));
                }
            }

            if old_image_file_abs_path != new_image_file_path_in_old_folder {
                info!("[Backend Rename Image] Renaming image file {} -> {}", old_image_file_abs_path.display(), new_image_file_path_in_old_folder.display());
                fs::rename(&old_image_file_abs_path, &new_image_file_path_in_old_folder)
                    .map_err(|e| CommandError::from(format!("Failed to rename image file (pre-folder op): {}", e)))?;
            }

            // Physical JSO annotation file rename is removed. DB call will handle annotation rename.

            let mut folder_renamed = false;
            if old_image_folder_abs_path != new_image_folder_abs_path {
                info!("[Backend Rename Image] Renaming image folder {} -> {}", old_image_folder_abs_path.display(), new_image_folder_abs_path.display());
                if let Err(e) = fs::rename(&old_image_folder_abs_path, &new_image_folder_abs_path) {
                    warn!("[Backend Rename Image] Failed to rename image folder: {}. Attempting to revert file renames.", e);
                    if old_image_file_abs_path != new_image_file_path_in_old_folder && new_image_file_path_in_old_folder.exists() {
                        let _ = fs::rename(&new_image_file_path_in_old_folder, &old_image_file_abs_path);
                    }
                    return Err(CommandError::from(format!("Failed to rename image folder: {}", e)));
                }
                folder_renamed = true;
            }

            let current_image_path_before_final_rename = if folder_renamed {
                new_image_folder_abs_path.join(new_image_file_path_in_old_folder.file_name().unwrap_or_default())
            } else {
                new_image_file_path_in_old_folder.clone()
            };

            if current_image_path_before_final_rename != final_new_image_file_abs_path && current_image_path_before_final_rename.exists() {
                 info!("[Backend Rename Image] Renaming image file (post-folder op) {} -> {}", current_image_path_before_final_rename.display(), final_new_image_file_abs_path.display());
                 if let Err(e) = fs::rename(&current_image_path_before_final_rename, &final_new_image_file_abs_path) {
                    warn!("[Backend Rename Image] Failed to rename image file to final path: {}. Attempting to revert operations.", e);
                    if folder_renamed {
                        let _ = fs::rename(&new_image_folder_abs_path, &old_image_folder_abs_path);
                    }
                    if old_image_file_abs_path != new_image_file_path_in_old_folder {
                        let path_to_revert_from = if folder_renamed { &new_image_file_path_in_old_folder } else { &current_image_path_before_final_rename };
                        if path_to_revert_from.exists() {
                             let _ = fs::rename(path_to_revert_from, &old_image_file_abs_path);
                        }
                    }
                    return Err(CommandError::from(format!("Failed to rename image file to final path: {}", e)));
                 }
            }

            let new_relative_path_for_image_xml = final_new_image_file_abs_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\", "/");

            // After FS operations for image file and folder, and after PDF annotation DB rename (if applicable for images)
            if let Err(db_err) = rename_annotations_in_db(&project_id_for_db, &item_relative_path, &new_relative_path_for_image_xml, "image") {
                 warn!("[Backend Rename Image] Failed to rename image annotations in DB for project_id {} from {} to {}: {}. Main file operations succeeded.", project_id_for_db, item_relative_path, new_relative_path_for_image_xml, db_err);
            }

            // Update metadata in DB for the image asset itself
            if let Err(e) = db_handler::rename_asset_metadata_key(
                &project_id_for_db,
                &item_relative_path, // old_relative_path (old DB key)
                &new_relative_path_for_image_xml, // new_relative_path (new DB key)
                &final_new_image_file_abs_path.to_string_lossy(), // new full file_path field value for DB
                &new_image_filename_with_ext_str, // new file_name field value for DB
            ) {
                warn!("[Backend Rename Image] Failed to rename/update asset metadata in DB for project_id {}, image {} -> {}: {}. File system and annotation DB changes were successful.", project_id_for_db, item_relative_path, new_relative_path_for_image_xml, e);
            } else {
                info!("[Backend Rename Image] Successfully renamed/updated asset metadata in DB for project_id {}, image {} -> {}", project_id_for_db, item_relative_path, new_relative_path_for_image_xml);
            }

            info!("[Backend Rename Image] Updating XML for image: OldRelPath '{}', NewRelPath '{}', NewName '{}'", item_relative_path, new_relative_path_for_image_xml, new_image_filename_with_ext_str);
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;

            if let Some(image_entry) = project_data.image_files.files.iter_mut().find(|i| i.relative_path == item_relative_path) {
                image_entry.name = new_image_filename_with_ext_str.to_string();
                image_entry.relative_path = new_relative_path_for_image_xml.clone();
                project_data.image_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                info!("[Backend Rename Image] XML image entry updated.");
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Rename Image] XML saved for image rename.");

                let payload = ItemRenamedPayload {
                    old_path: item_path_buf.to_string_lossy().into_owned(),
                    new_path: final_new_image_file_abs_path.to_string_lossy().into_owned(),
                    new_name: new_image_filename_with_ext_str.to_string(),
                    item_type: "image".to_string(),
                    project_xml_path: xml_path_buf.to_string_lossy().into_owned(),
                    base_directory: project_base_dir.to_string_lossy().into_owned(),
                };
                if let Err(e) = app_handle.emit("item_renamed", payload) {
                    warn!("[Backend Rename] Failed to emit item_renamed event for image: {}", e);
                }
            } else {
                 error!("[Backend Rename Image] CRITICAL: File system operations for image rename succeeded, but could not find matching old relative path '{}' in XML. Project XML might be inconsistent.", item_relative_path);
                 return Err(CommandError::from(format!("Failed to update XML as old image entry for {} was not found after file operations. Project state may be inconsistent.", item_relative_path)));
            }
        },
        _ => {
            error!("[Backend Rename] Renaming items of type '{}' is not supported directly: {}", item_type, item_path);
            return Err(CommandError::from(format!("Renaming not supported for item type '{}'. Rename the primary associated asset.", item_type)));
        }
    }

    info!("[Backend Rename] Success for: {}", item_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use crate::projectview::shared_types::ProjectXml; // Ensure ProjectXml is in scope if needed for direct construction, though here we rely on its deserialization.

    #[tokio::test]
    async fn test_load_project_data_includes_uuid() {
        let test_uuid = "test-uuid-123-abc";
        let project_name_test = "Test Project for UUID";

        let xml_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
            <project>
                <name>{}</name>
                <project_uuid>{}</project_uuid>
                <mediaFiles/>
                <documentFiles/>
                <tableFiles/>
                <imageFiles/>
                <importedTranscriptFiles/>
                <documentMetadataFiles/>
            </project>"#,
            project_name_test, test_uuid
        );

        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file.write_all(xml_content.as_bytes()).expect("Failed to write to temp file");
        let temp_file_path_str = temp_file.path().to_str().unwrap().to_string();

        // Create the harvey_files directory structure as ensure_base_asset_dirs expects it
        let temp_dir = temp_file.path().parent().expect("Temp file has no parent");
        let harvey_files_dir = temp_dir.join(HARVEY_FILES_DIR);
        fs::create_dir_all(&harvey_files_dir.join(MEDIA_DIR)).expect("Failed to create test media dir");
        fs::create_dir_all(&harvey_files_dir.join(DOCS_DIR)).expect("Failed to create test docs dir");
        fs::create_dir_all(&harvey_files_dir.join(TABLES_DIR)).expect("Failed to create test tables dir");
        fs::create_dir_all(&harvey_files_dir.join(IMAGES_DIR)).expect("Failed to create test images dir");
        fs::create_dir_all(&harvey_files_dir.join(TRANSCRIPTS_DIR)).expect("Failed to create test transcripts dir");


        match load_project_data(temp_file_path_str.clone()).await {
            Ok(project_view_data) => {
                assert_eq!(project_view_data.project_uuid, test_uuid, "ProjectViewData.project_uuid should match the UUID in the XML.");
                assert_eq!(project_view_data.project_name, project_name_test, "ProjectViewData.project_name should match the name in the XML.");
                assert_eq!(project_view_data.project_xml_path, temp_file_path_str, "ProjectViewData.project_xml_path should match the temp file path.");
            }
            Err(e) => {
                panic!("load_project_data failed: {:?}", e);
            }
        }

        // temp_file is automatically deleted when it goes out of scope.
        // However, we need to manually clean up directories created for ensure_base_asset_dirs
        if harvey_files_dir.exists() {
            fs::remove_dir_all(&harvey_files_dir).expect("Failed to remove test harvey_files dir");
        }
    }
}

#[tauri::command]
pub async fn reveal_in_file_explorer(file_path_str: String) -> Result<(), String> {
    info!("[CMD] reveal_in_file_explorer for path: {}", file_path_str);
    let path = PathBuf::from(file_path_str);

    if !path.exists() {
        let err_msg = format!("File or directory not found: {}", path.display());
        error!("[CMD] {}", err_msg);
        return Err(err_msg);
    }

    let os_type = platform(); // Get OS type from tauri::api::os

    match os_type {
        "macos" => {
            let status = Command::new("open")
                .arg("-R") // Reveals the file in Finder
                .arg(&path)
                .status()
                .map_err(|e| format!("Failed to execute 'open -R': {}", e))?;
            if status.success() {
                info!("[CMD] Revealed in Finder: {}", path.display());
                Ok(())
            } else {
                Err(format!("'open -R' command failed for {}: {:?}", path.display(), status.code()))
            }
        }
        "windows" => {
            // Ensure the path is properly quoted for explorer.exe /select
            let abs_path_str = path.to_string_lossy().into_owned();
            let arg_str = format!("/select,\"{}\"", abs_path_str);

            let status = Command::new("explorer.exe")
                .arg(arg_str)
                .status()
                .map_err(|e| format!("Failed to execute 'explorer.exe': {}", e))?;
            if status.success() {
                info!("[CMD] Revealed in Explorer: {}", path.display());
                Ok(())
            } else {
                Err(format!("'explorer.exe /select' command failed for {}: {:?}", path.display(), status.code()))
            }
        }
        "linux" | _ => { // Default to xdg-open for Linux and other Unix-like systems
            // xdg-open typically opens the directory if it's a file path,
            // or the file itself with its default application.
            // For revealing in file manager, we need the parent directory.
            let target_to_open = if path.is_file() {
                path.parent().unwrap_or(&path).to_path_buf()
            } else {
                path.clone()
            };

            let status = Command::new("xdg-open")
                .arg(&target_to_open)
                .status()
                .map_err(|e| format!("Failed to execute 'xdg-open': {}", e))?;

            if status.success() {
                info!("[CMD] Opened directory with xdg-open: {}", target_to_open.display());
                Ok(())
            } else {
                Err(format!("'xdg-open' command failed for {}: {:?}", target_to_open.display(), status.code()))
            }
        }
    }
}
