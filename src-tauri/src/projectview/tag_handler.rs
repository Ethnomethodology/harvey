// src-tauri/src/projectview/tag_handler.rs

use serde::{Serialize, Deserialize};
use crate::welcome::config::CommandError;
use crate::projectview::shared_types::{HighlightInfo, HighlightSource};
use crate::projectview::db_handler;
use std::collections::HashSet;
use std::path::Path;

use log::{info, warn};

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

fn map_asset_type_to_icon_type(asset_type: &str) -> String {
    match asset_type {
        "audio" => "audio".to_string(),
        "video" => "video".to_string(),
        "pdf" | "document" | "lexical" | "doc" | "txt" | "md" => "document".to_string(),
        "image" => "image".to_string(),
        "table" => "table".to_string(),
        "imported_transcript" => "imported_transcript".to_string(),
        _ => "unknown".to_string(),
    }
}

fn determine_asset_type(
    asset_type_opt: &Option<String>,
    file_path_str: &str,
    conn: &Connection,
    project_id: &str,
) -> String {
    let path = Path::new(file_path_str);
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    // 1. Highest priority: Robust, case-insensitive path component checking
    let mut is_transcript = false;
    for component in path.components() {
        if let Some(part) = component.as_os_str().to_str() {
            match part.to_lowercase().as_str() {
                "tables" => return "table".to_string(),
                "documents" => return "document".to_string(),
                "images" => return "image".to_string(),
                "transcripts" => {
                    is_transcript = true;
                    break; // Found transcript folder, proceed to parent media check
                }
                _ => (),
            }
        }
    }

    // Handle transcripts (both generated and imported)
    if is_transcript {
        if let Some(transcript_dir) = path.parent() {
            if let Some(stem) = transcript_dir.file_name().and_then(|s| s.to_str()) {
                for media_folder in ["media", "Media"].iter() {
                    let media_path_pattern = format!("assets/{}/{}%", media_folder, stem);
                    if let Ok(parent_asset_type) = conn.query_row(
                        "SELECT asset_type FROM asset_metadata WHERE project_id = ?1 AND asset_relative_path LIKE ?2 LIMIT 1",
                        rusqlite::params![project_id, media_path_pattern],
                        |row| row.get::<_, String>(0),
                    ) {
                        // Found parent media, so it's a generated transcript
                        return parent_asset_type;
                    }
                }
            }
        }
        // If we confirmed it's in a transcript path but found no parent, it's a standalone imported transcript
        return "imported_transcript".to_string();
    }

    // Override for table file extensions if path check missed it
    if extension == "csv" || extension == "tsv" {
        return "table".to_string();
    }

    // 2. Trust the database if it has a specific type
    if let Some(db_type) = asset_type_opt {
        if !db_type.is_empty() && db_type != "unknown" && db_type != "lexical" {
            return db_type.clone();
        }
    }

    // 3. Fallback to extension for any remaining cases
    warn!("Could not determine asset type from path or DB for {}. Falling back to file extension.", file_path_str);
    match extension {
        "pdf" => "pdf".to_string(),
        "png" | "jpg" | "jpeg" | "gif" => "image".to_string(),
        "mp4" | "mov" | "avi" => "video".to_string(),
        "mp3" | "wav" | "m4a" => "audio".to_string(),
        _ => "document".to_string(),
    }
}


#[tauri::command]
pub fn get_tag_info(project_id: &str, _tag_id: i64, tag_name: String) -> Result<TagInfo, CommandError> {
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

        let source = HighlightSource {
            file_name,
            file_path: file_path.clone(),
            file_type: map_asset_type_to_icon_type(&final_asset_type),
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
