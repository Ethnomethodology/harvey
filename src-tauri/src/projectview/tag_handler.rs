// src-tauri/src/projectview/tag_handler.rs

use serde::{Serialize, Deserialize};
use crate::projectview::shared_types::{CommandError, Highlight};
use crate::projectview::db_handler;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use log::{debug, error, info, warn};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TagInfo {
    pub name: String,
    pub description: String,
    pub highlight_count: usize,
    pub highlights: Vec<Highlight>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct TagMetadata {
    pub description: String,
}

const TAG_METADATA_FOLDER: &str = ".harvey_metadata/tags";

fn get_tag_metadata_path(project_root_path: &Path, tag_name: &str) -> PathBuf {
    project_root_path.join(TAG_METADATA_FOLDER).join(format!("{}.json", tag_name))
}

fn load_tag_metadata(project_root_path: &Path, tag_name: &str) -> Result<TagMetadata, CommandError> {
    let metadata_path = get_tag_metadata_path(project_root_path, tag_name);
    if !metadata_path.exists() {
        return Ok(TagMetadata::default());
    }
    let content = fs::read_to_string(&metadata_path)
        .map_err(|e| CommandError::Io(format!("Failed to read tag metadata for {}: {}", tag_name, e)))?;

    serde_json::from_str(&content)
        .map_err(|e| CommandError::Json(format!("Failed to parse tag metadata for {}: {}", tag_name, e)))
}

fn save_tag_metadata(project_root_path: &Path, tag_name: &str, metadata: &TagMetadata) -> Result<(), CommandError> {
    let metadata_dir = project_root_path.join(TAG_METADATA_FOLDER);
    fs::create_dir_all(&metadata_dir)
        .map_err(|e| CommandError::Io(format!("Failed to create tag metadata directory: {}", e)))?;

    let metadata_path = get_tag_metadata_path(project_root_path, tag_name);
    let content = serde_json::to_string_pretty(metadata)
        .map_err(|e| CommandError::Json(format!("Failed to serialize tag metadata for {}: {}", tag_name, e)))?;

    fs::write(&metadata_path, content)
        .map_err(|e| CommandError::Io(format!("Failed to write tag metadata for {}: {}", tag_name, e)))
}

#[tauri::command]
pub fn get_all_tags(project_root_path_str: &str, project_id: &str) -> Result<Vec<String>, CommandError> {
    info!("[Tags] Getting all tags for project_id: {}", project_id);
    let mut all_tags = HashSet::new();

    // 1. Get tags from the database (PDF and image annotations)
    match db_handler::get_all_tags_for_project(project_id) {
        Ok(db_tags) => {
            for tag in db_tags {
                all_tags.insert(tag);
            }
        }
        Err(e) => {
            error!("[Tags] Failed to get tags from database for project_id {}: {:?}", project_id, e);
            return Err(e);
        }
    }

    // 2. Get tags from .json files in the project's Documents directory
    let docs_path = Path::new(project_root_path_str).join("harvey_files").join("Documents");
    if docs_path.is_dir() {
        for entry in WalkDir::new(docs_path).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() && entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(highlights_val) = json_value.get("highlights") {
                             if let Ok(highlights) = serde_json::from_value::<Vec<Highlight>>(highlights_val.clone()) {
                                for highlight in highlights {
                                    if let Some(tags) = highlight.tags {
                                        for tag in tags {
                                            all_tags.insert(tag);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut sorted_tags: Vec<String> = all_tags.into_iter().collect();
    sorted_tags.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));

    info!("[Tags] Found {} unique tags for project_id {}", sorted_tags.len(), project_id);
    Ok(sorted_tags)
}


#[tauri::command]
pub fn get_tag_info(project_root_path_str: &str, project_id: &str, tag_name: String) -> Result<TagInfo, CommandError> {
    info!("[Tags] Getting info for tag '{}' in project_id: {}", tag_name, project_id);
    let project_root_path = Path::new(project_root_path_str);
    let mut highlights_with_tag = Vec::new();

    // 1. Scan DB annotations
    match db_handler::get_all_highlights_for_project(project_id) {
        Ok(db_highlights) => {
            for highlight in db_highlights {
                if let Some(tags) = &highlight.tags {
                    if tags.contains(&tag_name) {
                        highlights_with_tag.push(highlight);
                    }
                }
            }
        }
        Err(e) => {
            error!("[Tags] Failed to get highlights from database for project_id {}: {:?}", project_id, e);
            return Err(e);
        }
    }


    // 2. Scan file-based annotations
    let docs_path = Path::new(project_root_path_str).join("harvey_files").join("Documents");
    if docs_path.is_dir() {
        for entry in WalkDir::new(docs_path).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() && entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                     if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(highlights_val) = json_value.get("highlights") {
                             if let Ok(highlights) = serde_json::from_value::<Vec<Highlight>>(highlights_val.clone()) {
                                for highlight in highlights {
                                    if let Some(tags) = &highlight.tags {
                                        if tags.contains(&tag_name) {
                                            highlights_with_tag.push(highlight);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Load tag metadata
    let metadata = load_tag_metadata(project_root_path, &tag_name)?;

    let tag_info = TagInfo {
        name: tag_name.clone(),
        description: metadata.description,
        highlight_count: highlights_with_tag.len(),
        highlights: highlights_with_tag,
    };

    Ok(tag_info)
}

#[tauri::command]
pub fn update_tag_info(project_root_path_str: &str, tag_name: String, new_description: String) -> Result<(), CommandError> {
    info!("[Tags] Updating info for tag '{}'", tag_name);
    let project_root_path = Path::new(project_root_path_str);

    let mut metadata = load_tag_metadata(project_root_path, &tag_name)?;
    metadata.description = new_description;

    save_tag_metadata(project_root_path, &tag_name, &metadata)
}

#[tauri::command]
pub fn delete_tag(project_root_path_str: &str, project_id: &str, tag_name: String) -> Result<(), CommandError> {
    info!("[Tags] Deleting tag '{}' from project_id: {}", tag_name, project_id);
    let project_root_path = Path::new(project_root_path_str);

    // 1. Remove tag from DB annotations
    db_handler::remove_tag_from_all_annotations(project_id, &tag_name)?;

    // 2. Remove tag from file-based annotations
    let docs_path = project_root_path.join("harvey_files").join("Documents");
    if docs_path.is_dir() {
        for entry in WalkDir::new(docs_path).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() && entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(mut json_value) = serde_json::from_str::<serde_json::Value>(&content) {
                        let mut modified = false;
                        if let Some(highlights) = json_value.get_mut("highlights").and_then(|h| h.as_array_mut()) {
                            for highlight_val in highlights.iter_mut() {
                                if let Ok(mut highlight) = serde_json::from_value::<Highlight>(highlight_val.clone()) {
                                    if let Some(tags) = &mut highlight.tags {
                                        if tags.contains(&tag_name) {
                                            tags.retain(|t| t != &tag_name);
                                            modified = true;
                                            *highlight_val = serde_json::to_value(&highlight).unwrap();
                                        }
                                    }
                                }
                            }
                        }
                        if modified {
                            if let Ok(new_content) = serde_json::to_string_pretty(&json_value) {
                                if let Err(e) = fs::write(entry.path(), new_content) {
                                    error!("Failed to write updated json file {}: {}", entry.path().display(), e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Delete the tag's metadata file
    let metadata_path = get_tag_metadata_path(project_root_path, &tag_name);
    if metadata_path.exists() {
        if let Err(e) = fs::remove_file(&metadata_path) {
            warn!("Failed to delete tag metadata file for {}: {}", tag_name, e);
        }
    }

    Ok(())
}
