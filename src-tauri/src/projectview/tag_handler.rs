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

        let mut final_asset_type = asset_type_opt.clone().unwrap_or_else(|| "unknown".to_string());

		if final_asset_type == "unknown" || final_asset_type == "lexical" {
			// This is likely a highlight on a transcript. The icon should reflect the parent media type.
			if file_path.contains("/transcripts/") {
				let path = Path::new(&file_path);
				if let Some(transcript_dir) = path.parent() { // e.g., assets/transcripts/my-video
					if let Some(stem) = transcript_dir.file_name().and_then(|s| s.to_str()) { // e.g., "my-video"
						// Now find the corresponding media asset. The relative path would be like "assets/media/my-video.mp4"
						let media_path_pattern = format!("assets/media/{}%", stem);
						match conn.query_row(
							"SELECT asset_type FROM asset_metadata WHERE project_id = ?1 AND asset_relative_path LIKE ?2 LIMIT 1",
							rusqlite::params![project_id, media_path_pattern],
							|row| row.get(0),
						) {
							Ok(parent_asset_type) => {
								info!("Found parent media asset type '{}' for transcript {}", &parent_asset_type, file_path);
								final_asset_type = parent_asset_type
							},
							Err(e) => warn!("Could not find parent media asset for transcript {}: {}. Searched for pattern: {}", file_path, e, media_path_pattern),
						}
					}
				}
			} else if final_asset_type == "unknown" {
				// Fallback for non-transcript files that are missing metadata for some reason. Guess from extension.
				warn!("Asset metadata type is missing for {}. Falling back to file extension.", file_path);
				let extension = Path::new(&file_path).extension().and_then(|s| s.to_str()).unwrap_or("");
				final_asset_type = match extension {
					"pdf" => "pdf".to_string(),
                    "png" | "jpg" | "jpeg" | "gif" => "image".to_string(),
					"mp4" | "mov" | "avi" => "video".to_string(),
                    "mp3" | "wav" | "m4a" => "audio".to_string(),
					"csv" | "tsv" => "table".to_string(),
                    _ => "document".to_string(),
				};
			}
		}

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
