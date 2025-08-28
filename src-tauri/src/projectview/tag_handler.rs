// src-tauri/src/projectview/tag_handler.rs

use serde::{Serialize, Deserialize};
use crate::welcome::config::CommandError;
use crate::projectview::shared_types::{Highlight, HighlightInfo, HighlightSource};
use crate::projectview::db_handler;
use std::collections::HashSet;
use std::path::Path;

fn get_file_type_from_path(path_str: &str) -> String {
    let path = Path::new(path_str);
    match path.extension().and_then(|s| s.to_str()) {
        Some("mp3") | Some("wav") | Some("m4a") | Some("ogg") => "audio".to_string(),
        Some("mp4") | Some("mov") | Some("mkv") | Some("avi") => "video".to_string(),
        Some("pdf") => "document".to_string(),
        Some("jpg") | Some("jpeg") | Some("png") | Some("gif") => "image".to_string(),
        Some("csv") | Some("tsv") => "table".to_string(),
        Some("txt") | Some("md") | Some("rtf") => "document".to_string(),
        _ => "unknown".to_string(),
    }
}
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
    info!("[Tags] Getting info for tag '{}' in project_id: {}", tag_name, project_id);
    let db_path = db_handler::get_db_path()?;
    let conn = Connection::open(&db_path)?;
    let highlights_with_tag = db_handler::get_highlights_by_tag(&conn, project_id, tag_id)?;

    let mut highlight_infos = Vec::new();
    let mut seen_highlight_ids = HashSet::new();

    for (mut highlight, file_path, tags) in highlights_with_tag {
        if !seen_highlight_ids.insert(highlight.id) {
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

        let source = HighlightSource {
            file_name,
            file_path: file_path.clone(),
            file_type: get_file_type_from_path(&file_path),
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
