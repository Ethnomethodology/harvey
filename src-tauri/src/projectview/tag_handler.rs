// src-tauri/src/projectview/tag_handler.rs

use crate::projectview::db_handler;
use crate::projectview::shared_types::{HighlightInfo, HighlightSource};
use crate::welcome::config::CommandError;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path; // Added PathBuf // Added OptionalExtension

use log::{error, info, warn};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TagInfo {
    pub name: String,
    pub description: String,
    pub highlight_count: usize,
    pub highlights: Vec<HighlightInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TagGroupInfo {
    pub name: String,
    pub description: String,
    pub highlight_count: usize,
    pub highlights: Vec<HighlightInfo>,
}

use rusqlite::Connection;

#[tauri::command]
pub fn add_tag(
    project_id: &str,
    name: String,
    color: Option<String>,
    description: Option<String>,
    tag_group_id: Option<String>,
) -> Result<i64, CommandError> {
    info!(
        "[Tags] Adding tag '{}' to project_id: {}, group: {:?}",
        name, project_id, tag_group_id
    );
    let db_path = db_handler::get_db_path()?;
    let conn = Connection::open(&db_path)?;
    db_handler::add_tag(
        &conn,
        project_id,
        &name,
        color.as_deref(),
        description.as_deref(),
        tag_group_id.as_deref(),
    )
}

#[tauri::command]
pub fn get_tag_group_info(
    project_id: &str,
    group_id: String,
) -> Result<TagGroupInfo, CommandError> {
    info!(
        "[Tags] Getting info for tag group '{}' in project_id: {}",
        group_id, project_id
    );
    let db_path = db_handler::get_db_path()?;
    let conn = Connection::open(&db_path)?;

    // Get group details
    let mut stmt =
        conn.prepare("SELECT name, description FROM tag_groups WHERE id = ?1 AND project_id = ?2")?;
    let (group_name, group_desc): (String, Option<String>) = stmt
        .query_row(params![group_id, project_id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;

    // Get all tags in this group
    let all_tags = db_handler::get_all_tags(&conn, project_id)?;
    let child_tag_names: HashSet<String> = all_tags
        .into_iter()
        .filter(|t| t.tag_group_id.as_deref() == Some(&group_id))
        .map(|t| t.name)
        .collect();

    let mut all_highlight_infos = Vec::new();
    let mut seen_highlight_ids = HashSet::new();

    for tag_name in &child_tag_names {
        let highlights_with_tag = db_handler::get_highlights_by_tag(&conn, project_id, tag_name)?;

        for (mut highlight, file_path, tags, asset_type_opt) in highlights_with_tag {
            if !seen_highlight_ids.insert(highlight.id.clone()) {
                continue;
            }

            highlight.tags = Some(tags);

            let source_file_path = Path::new(&file_path);
            let file_name = source_file_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            // other_tags should exclude any tag that is IN the group.
            let other_tags = highlight
                .tags
                .as_ref()
                .map(|tags| {
                    tags.iter()
                        .filter(|t| !child_tag_names.contains(*t))
                        .cloned()
                        .collect()
                })
                .unwrap_or_else(Vec::new);

            let final_asset_type =
                determine_asset_type(&asset_type_opt, &file_path, &conn, project_id);

            let source = HighlightSource {
                file_name,
                file_path: file_path.clone(),
                file_type: map_asset_type_to_icon_type(&final_asset_type).to_string(),
                original_doc_type: final_asset_type,
            };

            all_highlight_infos.push(HighlightInfo {
                source,
                highlight,
                other_tags,
            });
        }
    }

    Ok(TagGroupInfo {
        name: group_name,
        description: group_desc.unwrap_or_default(),
        highlight_count: all_highlight_infos.len(),
        highlights: all_highlight_infos,
    })
}

#[tauri::command]
pub fn get_all_tags(project_id: &str) -> Result<Vec<db_handler::Tag>, CommandError> {
    info!("[Tags] Getting all tags for project_id: {}", project_id);
    let db_path = db_handler::get_db_path()?;
    let conn = Connection::open(&db_path)?;
    db_handler::get_all_tags(&conn, project_id)
}

#[tauri::command]
pub fn create_tag_group(
    project_id: &str,
    group_id: String,
    name: String,
    description: Option<String>,
) -> Result<(), CommandError> {
    info!(
        "[Tags] Creating tag group '{}' ({}) for project_id: {}",
        name, group_id, project_id
    );
    let db_path = db_handler::get_db_path()?;
    let conn = Connection::open(&db_path)?;
    db_handler::create_tag_group(&conn, project_id, &group_id, &name, description.as_deref())
}

#[tauri::command]
pub fn get_tag_groups(project_id: &str) -> Result<Vec<db_handler::TagGroup>, CommandError> {
    info!(
        "[Tags] Getting all tag groups for project_id: {}",
        project_id
    );
    let db_path = db_handler::get_db_path()?;
    let conn = Connection::open(&db_path)?;
    db_handler::get_tag_groups(&conn, project_id)
}

#[tauri::command]
pub fn update_tag_group(
    project_id: &str,
    group_id: String,
    name: String,
    description: Option<String>,
) -> Result<(), CommandError> {
    info!(
        "[Tags] Updating tag group '{}' for project_id: {}",
        group_id, project_id
    );
    let db_path = db_handler::get_db_path()?;
    let conn = Connection::open(&db_path)?;
    db_handler::update_tag_group(&conn, project_id, &group_id, &name, description.as_deref())
}

#[tauri::command]
pub fn delete_tag_group(project_id: &str, group_id: String) -> Result<(), CommandError> {
    info!(
        "[Tags] Deleting tag group '{}' from project_id: {}",
        group_id, project_id
    );
    let db_path = db_handler::get_db_path()?;
    let conn = Connection::open(&db_path)?;
    db_handler::delete_tag_group(&conn, project_id, &group_id)
}

fn map_asset_type_to_icon_type(asset_type: &str) -> &str {
    match asset_type {
        "video" => "video",
        "audio" => "audio",
        "image" => "image",
        "document" => "document",
        "pdf" => "document",
        "table" => "table",
        "standalone_transcript" => "standalone_transcript",
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
    info!(
        "[Tags] determine_asset_type file_path_str: {}",
        file_path_str
    );
    let path = Path::new(file_path_str);

    // 1. Check for standalone_transcript (standalone)
    if let Some(db_type) = asset_type_opt {
        if db_type == "standalone_transcript" {
            return db_type.clone();
        }
    }

    // 2. Path-based check for standalone_transcript (fallback if DB type is missing/wrong)
    if path
        .to_str()
        .unwrap_or_default()
        .contains("harvey_files/Transcripts")
    {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext == "json" {
                return "standalone_transcript".to_string();
            }
        }
    }

    // 3. Check for media-associated transcripts (JSON files within harvey_files/Media|Audios|Videos/STEM/transcripts/)
    let path_str_lower = file_path_str.to_lowercase();
    let is_media_associated = (path_str_lower.contains("harvey_files/media/")
        || path_str_lower.contains("harvey_files/audios/")
        || path_str_lower.contains("harvey_files/videos/"))
        && path_str_lower.contains("/transcripts/")
        && path_str_lower.ends_with(".json");

    if is_media_associated {
        // Extract the media stem from the path
        let parts: Vec<&str> = file_path_str.split('/').collect();
        if let Some(media_stem_index) = parts.iter().position(|&p| {
            p.eq_ignore_ascii_case("media")
                || p.eq_ignore_ascii_case("audios")
                || p.eq_ignore_ascii_case("videos")
        }) {
            if parts.len() > media_stem_index + 1 {
                let folder_name = parts[media_stem_index];
                let media_stem = parts[media_stem_index + 1];

                let media_file_relative_path_prefix =
                    format!("harvey_files/{}/{}/media/", folder_name, media_stem);

                // Query the asset_metadata table for assets whose relative path starts with this prefix
                // and whose asset_type is 'media'. Then check the extension of that media file.
                let mut stmt = conn
                    .prepare(
                        "SELECT asset_relative_path FROM asset_metadata
                     WHERE project_id = ?1 AND asset_relative_path LIKE ?2 || '%'
                     AND asset_type = 'media' LIMIT 1",
                    )
                    .unwrap();

                let result = stmt
                    .query_row(
                        rusqlite::params![project_id, media_file_relative_path_prefix],
                        |row| {
                            let media_asset_relative_path: String = row.get(0)?;
                            Ok(media_asset_relative_path)
                        },
                    )
                    .optional()
                    .unwrap();

                if let Some(media_asset_relative_path) = result {
                    let media_path = Path::new(&media_asset_relative_path);
                    let extension = media_path
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    return match extension.to_lowercase().as_str() {
                        "mp3" | "wav" | "m4a" | "flac" | "ogg" => "audio_transcript".to_string(),
                        "mp4" | "mov" | "avi" | "mkv" | "webm" => "video_transcript".to_string(),
                        _ => "audio_transcript".to_string(),
                    };
                } else {
                    warn!("[Tags] Could not find associated media asset metadata for transcript: {}. Falling back to 'audio_transcript'.", file_path_str);
                    return "audio_transcript".to_string();
                }
            }
        }
    }

    // 4. Fallback to DB asset_type (if not standalone_transcript)
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
pub fn get_tag_info(
    project_id: &str,
    _tag_id: i64,
    tag_name: String,
) -> Result<TagInfo, CommandError> {
    info!("[Tags] get_tag_info called for tag_name: {}", tag_name);
    info!(
        "[Tags] Getting info for tag '{}' in project_id: {}",
        tag_name, project_id
    );
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

        let other_tags = highlight
            .tags
            .as_ref()
            .map(|tags| tags.iter().filter(|t| **t != tag_name).cloned().collect())
            .unwrap_or_else(Vec::new);

        let final_asset_type = determine_asset_type(&asset_type_opt, &file_path, &conn, project_id);
        info!(
            "[Tags] Determined asset type for file '{}' is '{}'",
            file_path, final_asset_type
        );

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

    // Fetch description from DB if available
    // We already have 'tags' (which are strings) for each highlight, but we need the description of the *queried* tag.
    // We can fetch it separately or pass it if available.
    // Since get_all_tags returns description, the frontend likely already knows it, but TagInfo is returned here.
    // Let's fetch the tag definition to get the description.
    let mut stmt =
        conn.prepare("SELECT description FROM tags WHERE project_id = ?1 AND name = ?2")?;
    let description: String = stmt
        .query_row(params![project_id, tag_name], |row| {
            Ok(row.get(0).unwrap_or_default())
        })
        .unwrap_or_default();

    let tag_info = TagInfo {
        name: tag_name.clone(),
        description,
        highlight_count: highlight_infos.len(),
        highlights: highlight_infos,
    };

    Ok(tag_info)
}

#[tauri::command]
pub fn update_tag(
    project_id: &str,
    tag_id: i64,
    new_name: String,
    color: Option<String>,
    description: Option<String>,
    tag_group_id: Option<String>,
) -> Result<(), CommandError> {
    info!("[Tags] Updating info for tag '{}'", tag_id);
    let db_path = db_handler::get_db_path()?;
    let conn = Connection::open(&db_path)?;
    db_handler::update_tag(
        &conn,
        project_id,
        tag_id,
        &new_name,
        color.as_deref(),
        description.as_deref(),
        tag_group_id.as_deref(),
    )
}

#[tauri::command]
pub fn delete_tag(project_id: &str, tag_id: i64) -> Result<(), CommandError> {
    info!(
        "[Tags] Deleting tag '{}' from project_id: {}",
        tag_id, project_id
    );
    let db_path = db_handler::get_db_path()?;
    let conn = Connection::open(&db_path)?;
    db_handler::delete_tag(&conn, project_id, tag_id)
}

#[tauri::command]
pub fn remove_tag_globally(project_id: &str, tag_name: String) -> Result<(), CommandError> {
    info!(
        "[Tags] Removing tag globally '{}' for project_id: {}",
        tag_name, project_id
    );
    let db_path = db_handler::get_db_path()?;
    let conn = Connection::open(&db_path)?;

    let all_annotations = db_handler::get_all_annotations_for_project(&conn, project_id)?;

    for (path, json_str, doc_type) in all_annotations {
        let mut highlights: Vec<serde_json::Value> = if doc_type == "table" {
            serde_json::from_str(&json_str)
                .or_else(|_| {
                    serde_json::from_str::<String>(&json_str).and_then(|s| serde_json::from_str(&s))
                })
                .unwrap_or_else(|_| Vec::new())
        } else {
            serde_json::from_str(&json_str).unwrap_or_else(|_| Vec::new())
        };

        if highlights.is_empty() {
            continue;
        }

        let mut was_modified = false;
        for highlight in highlights.iter_mut() {
            if let Some(h_obj) = highlight.as_object_mut() {
                if let Some(tags_val) = h_obj.get_mut("tags") {
                    if let Some(tags_arr) = tags_val.as_array_mut() {
                        let initial_len = tags_arr.len();
                        tags_arr.retain(|t| t.as_str() != Some(&tag_name));
                        if tags_arr.len() < initial_len {
                            was_modified = true;
                        }
                    }
                }
            }
        }

        if was_modified {
            if doc_type == "table" {
                let inner_json = serde_json::to_string(&highlights)?;
                let outer_json = serde_json::to_string(&inner_json)?;
                db_handler::save_table_styles(project_id, &path, &outer_json)?;
            } else {
                let new_json_string = serde_json::to_string(&highlights)?;
                db_handler::save_annotations_to_db(project_id, &path, &new_json_string, &doc_type)?;
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub fn manage_highlight_comment(
    project_id: &str,
    highlight_id: String,
    action: String, // "add", "update", "delete"
    comment_id: Option<String>,
    comment: Option<serde_json::Value>,
    text: Option<String>,
    file_path: String,
    doc_type: String,
) -> Result<(), CommandError> {
    info!(
        "[Tags] Managing comment '{}' for highlight '{}' in file '{}' of type '{}'",
        action, highlight_id, file_path, doc_type
    );

    let db_path = db_handler::get_db_path()?;
    let conn = Connection::open(&db_path)?;

    // Determine the table and column to update based on doc_type
    let (table_name, json_column, path_column) = match doc_type.as_str() {
        "document"
        | "pdf"
        | "image"
        | "lexical"
        | "standalone_transcript"
        | "audio_transcript"
        | "video_transcript" => ("pdf_annotations", "annotations_json", "pdf_document_path"),
        "table" => ("table_styles", "styles", "table_path"),
        _ => {
            let err_msg = format!(
                "Unsupported document type for comment management: {}",
                doc_type
            );
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
            serde_json::from_str(&json_str)
                .or_else(|_| {
                    serde_json::from_str::<String>(&json_str).and_then(|s| serde_json::from_str(&s))
                })
                .map_err(|e| {
                    CommandError::Message(format!("Failed to parse table styles JSON: {}", e))
                })?
        } else {
            serde_json::from_str(&json_str)
                .map_err(|e| CommandError::Message(format!("Failed to parse JSON: {}", e)))?
        };

        let mut was_modified = false;
        for highlight in highlights.iter_mut() {
            if let Some(h_obj) = highlight.as_object_mut() {
                if h_obj.get("id").and_then(|v| v.as_str()) == Some(&highlight_id) {
                    let comments_arr = h_obj
                        .entry("comments")
                        .or_insert_with(|| serde_json::json!([]));

                    if let Some(arr) = comments_arr.as_array_mut() {
                        if action == "add" {
                            if let Some(new_comment) = comment.clone() {
                                arr.push(new_comment);
                                was_modified = true;
                            }
                        } else if action == "delete" {
                            if let Some(c_id) = &comment_id {
                                let initial_len = arr.len();
                                arr.retain(|c| {
                                    c.get("id").and_then(|v| v.as_str()) != Some(c_id)
                                        && c.get("parentId").and_then(|v| v.as_str()) != Some(c_id)
                                });
                                if arr.len() < initial_len {
                                    was_modified = true;
                                }
                            }
                        } else if action == "update" {
                            if let (Some(c_id), Some(new_text)) = (&comment_id, &text) {
                                for c in arr.iter_mut() {
                                    if c.get("id").and_then(|v| v.as_str()) == Some(c_id) {
                                        if let Some(c_obj) = c.as_object_mut() {
                                            c_obj.insert(
                                                "text".to_string(),
                                                serde_json::Value::String(new_text.clone()),
                                            );
                                            // also update editedAt if needed, but standard update is fine
                                            was_modified = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    break;
                }
            }
        }

        // 3. Save the modified JSON back to the DB
        if was_modified {
            let new_json_string = if doc_type == "table" {
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
                "[Tags] Successfully updated comments for file: {}",
                file_path
            );
        }
    }

    Ok(())
}

#[tauri::command]
pub fn rename_tag_in_highlights(
    project_id: &str,
    old_name: String,
    new_name: String,
) -> Result<(), CommandError> {
    info!(
        "[Tags] Renaming tag globally from '{}' to '{}' for project_id: {}",
        old_name, new_name, project_id
    );
    let db_path = db_handler::get_db_path()?;
    let conn = Connection::open(&db_path)?;

    let all_annotations = db_handler::get_all_annotations_for_project(&conn, project_id)?;

    for (path, json_str, doc_type) in all_annotations {
        let mut highlights: Vec<serde_json::Value> = if doc_type == "table" {
            serde_json::from_str(&json_str)
                .or_else(|_| {
                    serde_json::from_str::<String>(&json_str).and_then(|s| serde_json::from_str(&s))
                })
                .unwrap_or_else(|_| Vec::new())
        } else {
            serde_json::from_str(&json_str).unwrap_or_else(|_| Vec::new())
        };

        if highlights.is_empty() {
            continue;
        }

        let mut was_modified = false;
        for highlight in highlights.iter_mut() {
            if let Some(h_obj) = highlight.as_object_mut() {
                if let Some(tags_val) = h_obj.get_mut("tags") {
                    if let Some(tags_arr) = tags_val.as_array_mut() {
                        for tag in tags_arr.iter_mut() {
                            if tag.as_str() == Some(&old_name) {
                                *tag = serde_json::Value::String(new_name.clone());
                                was_modified = true;
                            }
                        }
                    }
                }
            }
        }

        if was_modified {
            if doc_type == "table" {
                let inner_json = serde_json::to_string(&highlights)?;
                let outer_json = serde_json::to_string(&inner_json)?;
                db_handler::save_table_styles(project_id, &path, &outer_json)?;
            } else {
                let new_json_string = serde_json::to_string(&highlights)?;
                db_handler::save_annotations_to_db(project_id, &path, &new_json_string, &doc_type)?;
            }
        }
    }

    Ok(())
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
        "document"
        | "pdf"
        | "image"
        | "lexical"
        | "standalone_transcript"
        | "audio_transcript"
        | "video_transcript" => ("pdf_annotations", "annotations_json", "pdf_document_path"),
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
                .or_else(|_| {
                    serde_json::from_str::<String>(&json_str).and_then(|s| serde_json::from_str(&s))
                })
                .map_err(|e| {
                    let err_msg = format!(
                        "Failed to parse table styles JSON from DB for file {}: {}",
                        file_path, e
                    );
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
