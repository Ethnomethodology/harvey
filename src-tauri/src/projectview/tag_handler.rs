// src-tauri/src/projectview/tag_handler.rs

use serde::{Serialize, Deserialize};
use crate::welcome::config::CommandError;
use crate::projectview::shared_types::{HighlightInfo, HighlightSource};
use crate::projectview::db_handler;
use std::collections::HashSet;
use std::path::Path;

use log::info;

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


#[tauri::command]
pub fn get_tag_info(project_id: &str, tag_id: i64, tag_name: String) -> Result<TagInfo, CommandError> {
    info!("[Tags] Getting info for tag '{}' with id '{}' in project_id: {}", tag_name, tag_id, project_id);
    let db_path = db_handler::get_db_path()?;
    let conn = Connection::open(&db_path)?;
    // The new implementation of get_highlights_by_tag uses tag_name, not tag_id.
    let highlights_with_tag = db_handler::get_highlights_by_tag(&conn, project_id, &tag_name)?;

fn map_asset_type_to_icon_type(asset_type: &str) -> String {
    match asset_type {
        "audio" => "audio".to_string(),
        "video" => "video".to_string(),
        "pdf" | "document" | "lexical" | "doc" => "document".to_string(),
        "image" => "image".to_string(),
        "table" => "table".to_string(),
        "transcript" | "imported_transcript" => "imported_transcript".to_string(),
        _ => "unknown".to_string(),
    }
}

    let mut highlight_infos = Vec::new();
    let mut seen_highlight_ids = HashSet::new();

    for (mut highlight, file_path, tags, asset_type) in highlights_with_tag {
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

        let file_type = map_asset_type_to_icon_type(&asset_type.unwrap_or_default());

        let source = HighlightSource {
            file_name,
            file_path: file_path.clone(),
            file_type,
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
