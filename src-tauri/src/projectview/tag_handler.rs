// src-tauri/src/projectview/tag_handler.rs

use serde::{Serialize, Deserialize};
use crate::welcome::config::CommandError;
use crate::projectview::shared_types::{HighlightInfo, HighlightSource};
use crate::projectview::db_handler;
use std::collections::HashSet;
use std::path::{Path, PathBuf}; // Added PathBuf
use rusqlite::{params, OptionalExtension}; // Added OptionalExtension

use log::{info, warn, error};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TagInfo {
    pub name: String,
    pub description: String,
    pub highlight_count: usize,
    pub highlights: Vec<HighlightInfo>,
}

use rusqlite::Connection;


#[tauri::command]
pub fn add_tag(project_id: &str, name: String, color: Option<String>) -> Result<i64, CommandError> {
    info!("[Tags] Adding tag '{}' to project_id: {}", name, project_id);
    let db_path = db_handler::get_db_path()?;
    let conn = Connection::open(&db_path)?;
    db_handler::add_tag(&conn, project_id, &name, color.as_deref())
}

#[tauri::command]
pub fn get_all_tags(project_id: &str) -> Result<Vec<db_handler::Tag>, CommandError> {
    info!("[Tags] Getting all tags for project_id: {}", project_id);
    let db_path = db_handler::get_db_path()?;
    let conn = Connection::open(&db_path)?;
    db_handler::get_all_tags(&conn, project_id)
}

fn map_asset_type_to_icon_type(asset_type: &str) -> &str {
    match asset_type {
        "video" => "video",
        "audio" => "audio",
        "image" => "image",
        "document" => "document",
        "pdf" => "document",
        "table" => "table",
        "imported_transcript" => "transcript",
        "audio_transcript" => "audio_transcript",
        "video_transcript" => "video_transcript",
        _ => "unknown",
    }
}

fn determine_asset_type(
    asset_type_opt: &Option<String>,
    file_path_str: &str,
    conn: &Connection,
    project_id: &str,
) -> String {
    info!("[Tags] determine_asset_type file_path_str: {}", file_path_str);
    let path = Path::new(file_path_str);

    // 1. Check for imported_transcript (standalone)
    if let Some(db_type) = asset_type_opt {
        if db_type == "imported_transcript" {
            return db_type.clone();
        }
    }

    // 2. Path-based check for imported_transcript (fallback if DB type is missing/wrong)
    if path.to_str().unwrap_or_default().contains("harvey_files/Transcripts") {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext == "json" {
                return "imported_transcript".to_string();
            }
        }
    }

    // 3. Check for media-associated transcripts (JSON files within harvey_files/Media/STEM/transcripts/)
    let path_str_lower = file_path_str.to_lowercase();
    if path_str_lower.contains("harvey_files/media/") && path_str_lower.contains("/transcripts/") && path_str_lower.ends_with(".json") {
        // Extract the media stem from the path
        // Example: /Users/dipanjan/Documents/Test Project/harvey_files/Media/20130922/transcripts/20130922_1.json
        // We need "20130922"
        let parts: Vec<&str> = file_path_str.split('/').collect();
        if let Some(media_stem_index) = parts.iter().position(|&p| p.eq_ignore_ascii_case("media")) {
            if parts.len() > media_stem_index + 1 {
                let media_stem = parts[media_stem_index + 1];
                // Construct the likely relative path to the actual media file
                // This assumes a structure like harvey_files/Media/STEM/media/STEM.ext
                // We need to find the actual media file within the media stem directory
                let _media_stem_dir_path = PathBuf::from(file_path_str)
                    .parent().and_then(|p| p.parent()) // Go up from transcripts/ to STEM/
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();

                // Construct the relative path to the media file within the project
                // This is tricky because the media file name might not be the same as the stem.
                // We need to query asset_metadata for assets within this media stem directory.
                // A more robust way would be to pass the media_xml_identifier from the frontend.
                // For now, let's try to find the media file based on the stem.

                // This is a simplified approach. A more robust solution would involve
                // querying the asset_metadata table for the media file associated with this stem.
                // For now, let's assume the media file is named after the stem and is in the 'media' subdirectory.
                let media_file_relative_path_prefix = format!("harvey_files/Media/{}/media/", media_stem);

                // Query the asset_metadata table for assets whose relative path starts with this prefix
                // and whose asset_type is 'media'. Then check the extension of that media file.
                let mut stmt = conn.prepare(
                    "SELECT asset_relative_path FROM asset_metadata
                     WHERE project_id = ?1 AND asset_relative_path LIKE ?2 || '%'
                     AND asset_type = 'media' LIMIT 1" // Changed asset_type to 'media'
                ).unwrap(); // TODO: Handle unwrap gracefully

                let result = stmt.query_row(
                    rusqlite::params![project_id, media_file_relative_path_prefix],
                    |row| {
                        let media_asset_relative_path: String = row.get(0)?;
                        Ok(media_asset_relative_path)
                    }
                ).optional().unwrap(); // TODO: Handle unwrap gracefully

                if let Some(media_asset_relative_path) = result {
                    let media_path = Path::new(&media_asset_relative_path);
                    let extension = media_path.extension().and_then(|s| s.to_str()).unwrap_or("");
                    return match extension {
                        "mp3" | "wav" | "m4a" => "audio_transcript".to_string(),
                        "mp4" | "mov" | "avi" => "video_transcript".to_string(),
                        _ => "transcript".to_string(), // Fallback if media extension is unexpected
                    };
                } else {
                    warn!("[Tags] Could not find associated media asset metadata for transcript: {}. Falling back to generic 'transcript'.", file_path_str);
                    return "transcript".to_string();
                }
            }
        }
    }


    // 4. Fallback to DB asset_type (if not imported_transcript)
    if let Some(db_type) = asset_type_opt {
        if !db_type.is_empty() && db_type != "unknown" && db_type != "lexical" {
            return db_type.clone();
        }
    }

    // 5. Fallback to extension-based detection
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    match extension {
        "pdf" => "document".to_string(),
        "png" | "jpg" | "jpeg" | "gif" => "image".to_string(),
        "mp4" | "mov" | "avi" => "video".to_string(),
        "mp3" | "wav" | "m4a" => "audio".to_string(),
        "csv" | "xlsx" => "table".to_string(),
        _ => "document".to_string(), // Default for unknown JSONs or other files
    }
}


#[tauri::command]
pub fn get_tag_info(project_id: &str, _tag_id: i64, tag_name: String) -> Result<TagInfo, CommandError> {
    info!("[Tags] get_tag_info called for tag_name: {}", tag_name);
    info!("[Tags] Getting info for tag '{}' in project_id: {}", tag_name, project_id);
    let db_path = db_handler::get_db_path()?;
    let conn = Connection::open(&db_path)?;
    let highlights_with_tag = db_handler::get_highlights_by_tag(&conn, project_id, &tag_name)?;

    let mut highlight_infos = Vec::new();
    let mut seen_highlight_ids = HashSet::new();

    for (mut highlight, file_path, tags, asset_type_opt) in highlights_with_tag {
        if !seen_highlight_ids.insert(highlight.id.clone()) {
            continue; // Skip if this highlight ID has already been processed
        }

        highlight.tags = Some(tags);

        let source_file_path = Path::new(&file_path);
        let file_name = source_file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let other_tags = highlight.tags.as_ref()
            .map(|tags| tags.iter().filter(|t| **t != tag_name).cloned().collect())
            .unwrap_or_else(Vec::new);

        let final_asset_type = determine_asset_type(&asset_type_opt, &file_path, &conn, project_id);
        info!("[Tags] Determined asset type for file '{}' is '{}'", file_path, final_asset_type);

        let source = HighlightSource {
            file_name,
            file_path: file_path.clone(),
            file_type: map_asset_type_to_icon_type(&final_asset_type).to_string(),
            original_doc_type: final_asset_type,
        };

        highlight_infos.push(HighlightInfo {
            source,
            highlight,
            other_tags,
        });
    }

    let tag_info = TagInfo {
        name: tag_name.clone(),
        description: "".to_string(), // Description is not stored in the db yet
        highlight_count: highlight_infos.len(),
        highlights: highlight_infos,
    };

    

    Ok(tag_info)
}

#[tauri::command]
pub fn update_tag(project_id: &str, tag_id: i64, new_name: String, color: Option<String>) -> Result<(), CommandError> {
    info!("[Tags] Updating info for tag '{}'", tag_id);
    let db_path = db_handler::get_db_path()?;
    let conn = Connection::open(&db_path)?;
    db_handler::update_tag(&conn, project_id, tag_id, &new_name, color.as_deref())
}

#[tauri::command]
pub fn delete_tag(project_id: &str, tag_id: i64) -> Result<(), CommandError> {
    info!("[Tags] Deleting tag '{}' from project_id: {}", tag_id, project_id);
    let db_path = db_handler::get_db_path()?;
    let conn = Connection::open(&db_path)?;
    db_handler::delete_tag(&conn, project_id, tag_id)
}

#[tauri::command]
pub fn remove_tag_from_highlight(
    project_id: &str,
    highlight_id: String,
    tag_to_remove: String,
    file_path: String,
    doc_type: String,
) -> Result<(), CommandError> {
    info!(
        "[Tags] Removing tag '{}' from highlight '{}' in file '{}' of type '{}'",
        tag_to_remove, highlight_id, file_path, doc_type
    );

    let db_path = db_handler::get_db_path()?;
    let conn = Connection::open(&db_path)?;

    // Determine the table and column to update based on doc_type
    let (table_name, json_column, path_column) = match doc_type.as_str() {
        // pdf_annotations stores highlights for multiple "types"
        "document" | "pdf" | "image" | "lexical" | "imported_transcript" | "audio_transcript" | "video_transcript" => ("pdf_annotations", "annotations_json", "pdf_document_path"),
        "table" => ("table_styles", "styles", "table_path"),
        _ => {
            let err_msg = format!("Unsupported document type for tag removal: {}", doc_type);
            error!("[Tags] {}", err_msg);
            return Err(CommandError::Message(err_msg));
        }
    };

    // 1. Load the existing JSON blob
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM {} WHERE project_id = ?1 AND {} = ?2",
        json_column, table_name, path_column
    ))?;

    let json_string_opt: Option<String> = stmt
        .query_row(params![project_id, file_path], |row| row.get(0))
        .optional()?;

    if let Some(json_str) = json_string_opt {
        // 2. Parse and modify the JSON
        let mut highlights: Vec<serde_json::Value> = if doc_type == "table" {
            // Table styles can be double-encoded JSON. Try parsing twice.
            serde_json::from_str(&json_str)
                .or_else(|_| serde_json::from_str::<String>(&json_str).and_then(|s| serde_json::from_str(&s)))
                .map_err(|e| {
                    let err_msg = format!("Failed to parse table styles JSON from DB for file {}: {}", file_path, e);
                    error!("[Tags] {}", err_msg);
                    CommandError::Message(err_msg)
                })?
        } else {
            // Other document types are single-encoded.
            serde_json::from_str(&json_str).map_err(|e| {
                let err_msg = format!("Failed to parse JSON from DB for file {}: {}", file_path, e);
                error!("[Tags] {}", err_msg);
                CommandError::Message(err_msg)
            })?
        };

        let mut was_modified = false;
        for highlight in highlights.iter_mut() {
            if let Some(h_obj) = highlight.as_object_mut() {
                if h_obj.get("id").and_then(|v| v.as_str()) == Some(&highlight_id) {
                    if let Some(tags_val) = h_obj.get_mut("tags") {
                        if let Some(tags_arr) = tags_val.as_array_mut() {
                            let initial_len = tags_arr.len();
                            tags_arr.retain(|t| t.as_str() != Some(&tag_to_remove));
                            if tags_arr.len() < initial_len {
                                was_modified = true;
                            }
                        }
                    }
                    break; // Found and processed the highlight, exit loop
                }
            }
        }

        // 3. Save the modified JSON back to the DB
        if was_modified {
            let new_json_string = if doc_type == "table" {
                // For tables, re-serialize the inner Vec<Value>, then serialize that string again
                // to maintain the double-encoding.
                let inner_json = serde_json::to_string(&highlights)?;
                serde_json::to_string(&inner_json)?
            } else {
                serde_json::to_string(&highlights)?
            };

            conn.execute(
                &format!(
                    "UPDATE {} SET {} = ?1 WHERE project_id = ?2 AND {} = ?3",
                    table_name, json_column, path_column
                ),
                params![new_json_string, project_id, file_path],
            )?;
            info!(
                "[Tags] Successfully removed tag and updated annotations for file: {}",
                file_path
            );
        } else {
            warn!(
                "[Tags] Tag '{}' not found on highlight '{}' in file '{}'. No changes made.",
                tag_to_remove, highlight_id, file_path
            );
        }
    } else {
        warn!(
            "[Tags] No annotation/style entry found for file '{}'. Cannot remove tag.",
            file_path
        );
    }

    Ok(())
}
