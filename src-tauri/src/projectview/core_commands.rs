// src-tauri/src/projectview/core_commands.rs
use super::shared_types::{*, TABLES_DIR, IMAGES_DIR, FileMetadata};
use super::shared_utils::*;
use crate::utils::canonicalize_path;
use crate::welcome::config::CommandError;
use log::{debug, error, info, warn};
use serde::Deserialize;
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;
#[cfg(not(target_os = "windows"))]
use tauri_plugin_opener::OpenerExt;
use serde_json::{self, json};
use serde::Serialize;
use std::{
    fs::{self},
    path::{Path, PathBuf},
};
use quick_xml;
use chrono::Utc;
use super::db_handler::{self, delete_annotations_from_db};
use crate::projectview::chart_handler;
use super::shared_types::GroupData; // Added for group commands
use rusqlite::{Connection, params}; // Added for opening DB connection in commands
use tauri::Emitter;
use uuid::Uuid; // Added for UUID generation

// --- Table Layout Preferences Commands ---
#[tauri::command]
pub async fn save_table_layout_prefs(project_id: String, table_path: String, layout_json: serde_json::Value) -> Result<(), String> {
    let layout_json_string = serde_json::to_string(&layout_json)
        .map_err(|e| format!("Failed to serialize layout JSON: {}", e))?;
    db_handler::save_table_layout_preferences(&project_id, &table_path, &layout_json_string)
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

// --- Group Commands ---
#[tauri::command]
pub async fn create_new_group(
    project_id: String,
    name: String,
    description: Option<String>,
    file_asset_relative_path: Option<String> // New parameter
) -> Result<GroupData, String> {
    if project_id.is_empty() || project_id == "null" {
        error!("[CMD] create_new_group - Project ID is missing or invalid.");
        return Err("Project ID is missing. Cannot create group.".to_string());
    }
    if name.trim().is_empty() {
        error!("[CMD] create_new_group - Group name cannot be empty.");
        return Err("Group name cannot be empty.".to_string());
    }

    let group_id = Uuid::new_v4().to_string();
    let trimmed_name = name.trim().to_string(); // Trim the name once
    info!(
        "[CMD] create_new_group: id={}, project_id={}, name={}, file_to_add_rel_path: {:?}",
        group_id, project_id, trimmed_name, file_asset_relative_path
    );

    let db_path = match db_handler::get_db_path() {
        Ok(path) => path,
        Err(e) => {
            error!("[CMD] create_new_group - Failed to get DB path: {}", e);
            return Err(format!("Failed to get database path: {}", e.to_string()));
        }
    };

    let conn = match Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            error!("[CMD] create_new_group - Failed to open DB: {}", e);
            return Err(format!("Failed to open database: {}", e.to_string()));
        }
    };

    // Create the group first
    match db_handler::create_group(&conn, &project_id, &group_id, &trimmed_name, description.as_deref()) {
        Ok(_) => {
            info!("[CMD] create_new_group - Group details saved successfully to 'groups' table: {}", group_id);

            // If a file path is provided, try to associate it with the new group
            if let Some(path_str) = file_asset_relative_path {
                if !path_str.is_empty() {
                    info!("[CMD] create_new_group - Attempting to add file '{}' to new group '{}'", path_str, group_id);
                    match db_handler::add_file_to_group(&conn, &project_id, &group_id, &path_str) {
                        Ok(_) => info!("[CMD] create_new_group - File '{}' successfully associated with new group '{}'.", path_str, group_id),
                        Err(e) => {
                            // Log a warning but don't fail the whole command, as the group was created.
                            warn!("[CMD] create_new_group - Group '{}' created, but failed to associate file '{}': {}", group_id, path_str, e);
                        }
                    }
                }
            }
            // Return success with GroupData regardless of file association outcome (as group was created)
            Ok(GroupData {
                id: group_id,
                project_id,
                name: trimmed_name, // Use the trimmed name
                description,
            })
        }
        Err(e) => {
            error!("[CMD] create_new_group - Failed to save group details to 'groups' table: {}", e);
             if e.to_string().contains("UNIQUE constraint failed: groups.project_id, groups.name") {
                 error!("[CMD] create_new_group - Unique constraint violation for group name '{}' in project '{}'.", trimmed_name, project_id);
                 return Err(format!("A group with the name \"{}\" already exists in this project.", trimmed_name));
            }
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn rename_project_group(
    project_id: String,
    group_id: String,
    new_name: String,
    new_description: Option<String>,
) -> Result<GroupData, String> {
    info!(
        "[CMD] rename_project_group: project_id={}, group_id={}, new_name={}",
        project_id, group_id, new_name
    );

    if project_id.trim().is_empty() {
        error!("[CMD] rename_project_group - Project ID cannot be empty.");
        return Err("Project ID cannot be empty.".to_string());
    }
    if group_id.trim().is_empty() {
        error!("[CMD] rename_project_group - Group ID cannot be empty.");
        return Err("Group ID cannot be empty.".to_string());
    }
    let trimmed_new_name = new_name.trim();
    if trimmed_new_name.is_empty() {
        error!("[CMD] rename_project_group - New group name cannot be empty.");
        return Err("New group name cannot be empty.".to_string());
    }

    let db_path = match db_handler::get_db_path() {
        Ok(path) => path,
        Err(e) => {
            error!("[CMD] rename_project_group - Failed to get DB path: {}", e);
            return Err(format!("Failed to get database path: {}", e));
        }
    };
    let conn = match Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            error!("[CMD] rename_project_group - Failed to open DB: {}", e);
            return Err(format!("Failed to open database: {}", e));
        }
    };

    match db_handler::rename_group_in_db(&conn, &project_id, &group_id, trimmed_new_name, new_description.as_deref()) {
        Ok(rows_affected) => {
            if rows_affected > 0 {
                info!("[CMD] rename_project_group - Group {} renamed successfully.", group_id);
                Ok(GroupData {
                    id: group_id,
                    project_id,
                    name: trimmed_new_name.to_string(),
                    description: new_description,
                })
            } else {
                error!("[CMD] rename_project_group - Group with ID {} not found or not updated.", group_id);
                Err(format!("Group with ID {} not found or no changes made.", group_id))
            }
        }
        Err(e) => {
            error!("[CMD] rename_project_group - Failed for group {}: {}", group_id, e);
            if e.to_string().contains("UNIQUE constraint failed: groups.project_id, groups.name") {
                 error!("[CMD] rename_project_group - Unique constraint violation for group name '{}' in project '{}'.", trimmed_new_name, project_id);
                 return Err(format!("A group with the name \"{}\" already exists in this project.", trimmed_new_name));
            }
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn delete_project_group(project_id: String, group_id: String) -> Result<(), String> {
    info!("[CMD] delete_project_group: project_id={}, group_id={}", project_id, group_id);

    if project_id.trim().is_empty() {
        error!("[CMD] delete_project_group - Project ID cannot be empty.");
        return Err("Project ID cannot be empty.".to_string());
    }
    if group_id.trim().is_empty() {
        error!("[CMD] delete_project_group - Group ID cannot be empty.");
        return Err("Group ID cannot be empty.".to_string());
    }

    let db_path = match db_handler::get_db_path() {
        Ok(path) => path,
        Err(e) => {
            error!("[CMD] delete_project_group - Failed to get DB path: {}", e);
            return Err(format!("Failed to get database path: {}", e));
        }
    };
    let conn = match Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            error!("[CMD] delete_project_group - Failed to open DB: {}", e);
            return Err(format!("Failed to open database: {}", e));
        }
    };

    match db_handler::delete_group_from_db(&conn, &project_id, &group_id) {
        Ok(rows_affected) => {
            if rows_affected > 0 {
                info!("[CMD] delete_project_group - Group {} deleted successfully.", group_id);
            } else {
                // This case might mean the group was already deleted or never existed.
                // For a delete operation, not finding the item is often not an error for the frontend.
                info!("[CMD] delete_project_group - Group with ID {} not found or already deleted.", group_id);
            }
            Ok(())
        }
        Err(e) => {
            error!("[CMD] delete_project_group - Failed for group {}: {}", group_id, e);
            Err(e.to_string())
        }
    }
}



#[tauri::command]
pub async fn update_group_details(
    project_id: String,
    group_id: String,
    name: String,
    description: Option<String> // Option<String> from frontend
) -> Result<GroupData, String> {
    info!("[CMD] update_group_details for group_id: {} in project_id: {}", group_id, project_id);

    if name.trim().is_empty() {
        error!("[CMD] update_group_details - Group name cannot be empty.");
        return Err("Group name cannot be empty.".to_string());
    }
    if group_id.trim().is_empty() {
        error!("[CMD] update_group_details - Group ID cannot be empty.");
        return Err("Group ID cannot be empty.".to_string());
    }
    if project_id.trim().is_empty() {
        error!("[CMD] update_group_details - Project ID cannot be empty.");
        return Err("Project ID cannot be empty.".to_string());
    }

    let db_path = match db_handler::get_db_path() {
        Ok(path) => path,
        Err(e) => {
            error!("[CMD] update_group_details - Failed to get DB path: {}", e);
            return Err(format!("Failed to get database path: {}", e));
        }
    };
    let conn = match Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            error!("[CMD] update_group_details - Failed to open DB: {}", e);
            return Err(format!("Failed to open database: {}", e));
        }
    };

    // Convert Option<String> to Option<&str> for db_handler
    let description_ref = description.as_deref();

    match db_handler::update_group_details(&conn, &project_id, &group_id, &name.trim(), description_ref) {
        Ok(rows_affected) => {
            if rows_affected > 0 {
                info!("[CMD] update_group_details - Group {} updated successfully.", group_id);
                Ok(GroupData {
                    id: group_id,
                    project_id,
                    name: name.trim().to_string(), // Use trimmed name
                    description, // This is Option<String>
                })
            } else {
                error!("[CMD] update_group_details - Group with ID {} not found or not updated.", group_id);
                Err(format!("Group with ID {} not found or no changes made.", group_id))
            }
        }
        Err(e) => {
            error!("[CMD] update_group_details - Failed for group {}: {}", group_id, e);
            // Check for unique constraint violation (name already exists for this project_id)
            if e.to_string().contains("UNIQUE constraint failed: groups.project_id, groups.name") {
                 error!("[CMD] update_group_details - Unique constraint violation for group name '{}' in project '{}'.", name.trim(), project_id);
                 return Err(format!("A group with the name \"{}\" already exists in this project.", name.trim()));
            }
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn get_groups_for_file_asset(project_id: String, file_asset_relative_path: String) -> Result<Vec<GroupData>, String> {
    if project_id.is_empty() || project_id == "null" {
        error!("[CMD] get_groups_for_file_asset - Project ID is missing or invalid.");
        return Err("Project ID is missing. Cannot get groups for file asset.".to_string());
    }
    if file_asset_relative_path.is_empty() {
        error!("[CMD] get_groups_for_file_asset - File asset relative path is missing.");
        return Err("File asset relative path is missing. Cannot get groups for file asset.".to_string());
    }
    info!("[CMD] get_groups_for_file_asset for project_id: {}, file_asset_relative_path: {}", project_id, file_asset_relative_path);

    let db_path = match db_handler::get_db_path() {
        Ok(path) => path,
        Err(e) => {
            error!("[CMD] get_groups_for_file_asset - Failed to get DB path: {}", e);
            return Err(format!("Failed to get database path: {}", e.to_string()));
        }
    };

    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(e) => {
            error!("[CMD] get_groups_for_file_asset - Failed to open DB: {}", e);
            return Err(format!("Failed to open database: {}", e.to_string()));
        }
    };

    match db_handler::get_groups_for_file_asset(&conn, &project_id, &file_asset_relative_path) {
        Ok(groups_from_db) => {
            info!("[CMD] get_groups_for_file_asset - Found {} groups for project_id: {}, file_asset_relative_path: {}", groups_from_db.len(), project_id, file_asset_relative_path);
            let groups_for_frontend: Vec<GroupData> = groups_from_db
                .into_iter()
                .map(|g_db| GroupData {
                    id: g_db.id,
                    project_id: g_db.project_id,
                    name: g_db.name,
                    description: g_db.description,
                })
                .collect();
            Ok(groups_for_frontend)
        }
        Err(e) => {
            error!("[CMD] get_groups_for_file_asset - Failed for project_id {}, file_asset_relative_path {}: {}", project_id, file_asset_relative_path, e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn remove_file_from_group(project_id: String, group_id: String, file_asset_relative_path: String) -> Result<(), String> {
    if project_id.is_empty() || project_id == "null" {
        error!("[CMD] remove_file_from_group - Project ID is missing or invalid.");
        return Err("Project ID is missing. Cannot remove file from group.".to_string());
    }
    if group_id.is_empty() || group_id == "null" {
        error!("[CMD] remove_file_from_group - Group ID is missing or invalid.");
        return Err("Group ID is missing. Cannot remove file from group.".to_string());
    }
    if file_asset_relative_path.is_empty() {
        error!("[CMD] remove_file_from_group - File asset relative path is missing.");
        return Err("File asset relative path is missing. Cannot remove file from group.".to_string());
    }
    info!("[CMD] remove_file_from_group: project_id={}, group_id={}, file_path={}", project_id, group_id, file_asset_relative_path);

    let db_path = match db_handler::get_db_path() {
        Ok(path) => path,
        Err(e) => {
            error!("[CMD] remove_file_from_group - Failed to get DB path: {}", e);
            return Err(format!("Failed to get database path: {}", e.to_string()));
        }
    };

    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(e) => {
            error!("[CMD] remove_file_from_group - Failed to open DB: {}", e);
            return Err(format!("Failed to open database: {}", e.to_string()));
        }
    };

    match db_handler::remove_file_from_group(&conn, &project_id, &group_id, &file_asset_relative_path) {
        Ok(rows_affected) => {
            if rows_affected > 0 {
                info!("[CMD] remove_file_from_group - File {} removed from group {} successfully.", file_asset_relative_path, group_id);
            } else {
                info!("[CMD] remove_file_from_group - No association found for file {} in group {}. Nothing removed.", file_asset_relative_path, group_id);
            }
            Ok(())
        }
        Err(e) => {
            error!("[CMD] remove_file_from_group - Failed for project_id {}, group_id {}, file {}: {}", project_id, group_id, file_asset_relative_path, e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn get_group_contents(project_xml_path_str: String, group_id: String) -> Result<Vec<AssociatedFile>, String> {
    info!("[CMD] get_group_contents for group_id: {}", group_id);

    let project_xml_path = PathBuf::from(&project_xml_path_str);
    let project_base_dir = project_xml_path.parent().ok_or_else(|| "Could not get project base directory.".to_string())?;

    let project_data_for_uuid: ProjectXml = match fs::read_to_string(&project_xml_path){
        Ok(content) => match serde_json::from_str(&content) {
            Ok(data) => data,
            Err(e) => return Err(format!("Failed to parse project XML for UUID: {}", e)),
        },
        Err(e) => return Err(format!("Failed to read project XML for UUID: {}", e)),
    };

    let project_id_for_db = project_data_for_uuid.project_uuid;
    if project_id_for_db.is_empty() {
        return Err("Project ID (UUID) is missing in the project file.".to_string());
    }

    if group_id.is_empty() || group_id == "null" {
        error!("[CMD] get_group_contents - Group ID is missing or invalid.");
        return Err("Group ID is missing. Cannot get group contents.".to_string());
    }

    let db_path = match db_handler::get_db_path() {
        Ok(path) => path,
        Err(e) => return Err(format!("Failed to get database path: {}", e)),
    };
    let conn = Connection::open(db_path).map_err(|e| format!("Failed to open database: {}", e))?;

    let file_associations_from_db = db_handler::get_files_for_group(&conn, &project_id_for_db, &group_id)
        .map_err(|e| format!("Failed to get files for group from DB: {}", e))?;

    let mut associated_files: Vec<AssociatedFile> = Vec::new();

    for assoc in file_associations_from_db {
        let relative_path_str = assoc.file_asset_path.clone();
        let full_path = project_base_dir.join(&relative_path_str);
        let file_name = full_path.file_name().unwrap_or_default().to_string_lossy().into_owned();

        let mut file_type = "other".to_string();
        let mut media_xml_identifier: Option<String> = None;

        if relative_path_str.contains(&format!("{}/", MEDIA_DIR)) 
            || relative_path_str.contains(&format!("{}/", AUDIOS_DIR))
            || relative_path_str.contains(&format!("{}/", VIDEOS_DIR)) 
        {
            let path_parts: Vec<&str> = relative_path_str.split('/').collect();
            if path_parts.len() >= 4 && path_parts[0] == HARVEY_FILES_DIR && (path_parts[1] == MEDIA_DIR || path_parts[1] == AUDIOS_DIR || path_parts[1] == VIDEOS_DIR) {
                // Example: harvey_files/Audios/STEM_NAME/media/file.mp3 -> STEM_NAME
                media_xml_identifier = Some(path_parts[2].to_string());
                let ext = PathBuf::from(&file_name).extension().unwrap_or_default().to_string_lossy().to_lowercase();
                if ["mp4", "mov", "avi", "mkv", "webm"].contains(&ext.as_str()) {
                    file_type = "video".to_string();
                } else if ["mp3", "wav", "m4a", "ogg", "aac", "flac"].contains(&ext.as_str()) {
                    file_type = "audio".to_string();
                } else {
                    file_type = "media_other".to_string();
                }
            }
        } else if relative_path_str.contains(&format!("{}/", DOCS_DIR)) {
            file_type = "document".to_string();
        } else if relative_path_str.contains(&format!("{}/", IMAGES_DIR)) {
            file_type = "image".to_string();
        } else if relative_path_str.contains(&format!("{}/", TABLES_DIR)) {
            file_type = "table".to_string();
        } else if relative_path_str.contains(&format!("{}/", TRANSCRIPTS_DIR)) {
            // This might need refinement if TRANSCRIPTS_DIR is for media-associated transcripts vs imported ones
            // Assuming TRANSCRIPTS_DIR implies it's an imported transcript if not under a media stem.
            // The logic in load_project_data for FileEntry might be more robust here.
            // For now, following the provided snippet's logic.
            file_type = "standalone_transcript".to_string();
        }
        // TODO: Consider using a JOIN with asset_metadata to get the definitive asset_type
        // or use/enhance shared_utils::determine_asset_type if applicable.

        let meta_opt = db_handler::load_asset_metadata(&project_id_for_db, &relative_path_str).ok().flatten();
        let last_modified = meta_opt.as_ref().map(|meta| meta.last_modified.clone());
        let created_at = meta_opt.as_ref().and_then(|meta| meta.creation_time.clone());
        let title = meta_opt.as_ref().and_then(|meta| meta.title.clone());
        let description = meta_opt.as_ref().and_then(|meta| meta.description.clone());

        associated_files.push(AssociatedFile {
            name: file_name,
            relative_path: relative_path_str,
            full_path: full_path.to_string_lossy().into_owned(),
            file_type,
            media_xml_identifier,
            last_modified,
            created_at,
            title,
            description,
            waveform_data: meta_opt.as_ref().and_then(|meta| meta.waveform_data.clone()),
            duration_seconds: meta_opt.as_ref().and_then(|meta| meta.duration_seconds),
            page_count: meta_opt.as_ref().and_then(|meta| meta.page_count),
            thumbnail_data: meta_opt.as_ref().and_then(|meta| meta.thumbnail.clone()),
        });
    }
    Ok(associated_files)
}

#[tauri::command]
pub async fn get_project_groups(project_id: String) -> Result<Vec<GroupData>, String> {
    if project_id.is_empty() || project_id == "null" {
        error!("[CMD] get_project_groups - Project ID is missing or invalid.");
        return Err("Project ID is missing. Cannot get groups.".to_string());
    }
    info!("[CMD] get_project_groups for project_id: {}", project_id);

    let db_path = match db_handler::get_db_path() {
        Ok(path) => path,
        Err(e) => {
            error!("[CMD] get_project_groups - Failed to get DB path: {}", e);
            return Err(format!("Failed to get database path: {}", e.to_string()));
        }
    };

    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(e) => {
            error!("[CMD] get_project_groups - Failed to open DB: {}", e);
            return Err(format!("Failed to open database: {}", e.to_string()));
        }
    };

    match db_handler::get_groups_for_project(&conn, &project_id) {
        Ok(groups_from_db) => {
            info!("[CMD] get_project_groups - Found {} groups for project_id: {}", groups_from_db.len(), project_id);
            let groups_for_frontend: Vec<GroupData> = groups_from_db
                .into_iter()
                .map(|g_db| GroupData {
                    id: g_db.id,
                    project_id: g_db.project_id,
                    name: g_db.name,
                    description: g_db.description,
                    // created_at and updated_at from GroupDataFromDb are not included in GroupData for now
                })
                .collect();
            Ok(groups_for_frontend)
        }
        Err(e) => {
            error!("[CMD] get_project_groups - Failed for project_id {}: {}", project_id, e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn add_file_to_existing_group(project_id: String, group_id: String, file_asset_relative_path: String) -> Result<(), String> {
    if project_id.is_empty() || project_id == "null" {
        error!("[CMD] add_file_to_existing_group - Project ID is missing or invalid.");
        return Err("Project ID is missing. Cannot add file to group.".to_string());
    }
    if group_id.is_empty() || group_id == "null" {
        error!("[CMD] add_file_to_existing_group - Group ID is missing or invalid.");
        return Err("Group ID is missing. Cannot add file to group.".to_string());
    }
    info!("[CMD] add_file_to_existing_group: project_id={}, group_id={}, file_path={}", project_id, group_id, file_asset_relative_path);

    let db_path = match db_handler::get_db_path() {
        Ok(path) => path,
        Err(e) => {
            error!("[CMD] add_file_to_existing_group - Failed to get DB path: {}", e);
            return Err(format!("Failed to get database path: {}", e.to_string()));
        }
    };

    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(e) => {
            error!("[CMD] add_file_to_existing_group - Failed to open DB: {}", e);
            return Err(format!("Failed to open database: {}", e.to_string()));
        }
    };

    match db_handler::add_file_to_group(&conn, &project_id, &group_id, &file_asset_relative_path) {
        Ok(_) => {
            info!("[CMD] add_file_to_existing_group - File added successfully to group {}", group_id);
            Ok(())
        }
        Err(e) => {
            error!("[CMD] add_file_to_existing_group - Failed for group_id {}: {}", group_id, e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn save_pdf_metadata(
    project_id: String,
    asset_relative_path: String,
    page_count: i32,
    thumbnail: Vec<u8>,
) -> Result<(), String> {
    info!("[CMD] save_pdf_metadata for project_id: {}, path: {}", project_id, asset_relative_path);
    db_handler::save_pdf_metadata_to_db(&project_id, &asset_relative_path, page_count, &thumbnail)
        .map_err(|e| format!("Failed to save PDF metadata to DB: {}", e))
}
// --- End Group Commands ---

#[allow(dead_code)]
#[derive(Clone, serde::Serialize)]
struct MediaRenamedPayload {
    old_media_stem: String,
    new_media_stem: String,
    new_media_file_relative_path: String,
    new_absolute_path: String,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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
#[allow(dead_code)]
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
    let mut project_data: ProjectXml = serde_json::from_str(&project_xml_content).map_err(|e| CommandError::from(format!("Failed to parse XML {}: {}", xml_path.display(), e)))?;

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

    // --- Parse Project Assets ---
    // Register this project in the SQLite DB. This handles cases where:
    // 1. It's a fresh import (DB didn't know about it).
    // 2. The project folder was moved (DB has old path).
    // 3. The Manifest was overwritten (DB needs to align with new UUID).
    if let Err(e) = db_handler::add_project_to_db(
        &project_data.project_uuid,
        &project_name,
        &base_directory,
        &xml_path.to_string_lossy()
    ) {
        error!("[Backend Load Manifest] Failed to register project identity in DB: {}", e);
    }

    // Performance optimization check: Do we have assets in the database?
    let db_path = db_handler::get_db_path().map_err(|e| CommandError::Message(e.to_string()))?;
    let conn = rusqlite::Connection::open(&db_path).map_err(|e| CommandError::Message(e.to_string()))?;
    let asset_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM asset_metadata WHERE project_id = ?",
        [&project_data.project_uuid],
        |row| row.get(0)
    ).unwrap_or(0);

    let is_new_project_in_db = asset_count == 0;
    if is_new_project_in_db {
        info!("[Backend Load Manifest] Project is new to SQLite. Running deep sync/auto-healing.");
    } else {
        info!("[Backend Load Manifest] Project already exists in SQLite ({} assets). Skipping heavy database sync.", asset_count);
    }

    let mut file_entries: Vec<FileEntry> = Vec::new();
    let mut was_xml_healed = false;

    // Helper closure to process media entries from different lists
    let mut process_media_list = |entries: &mut Vec<MediaFileEntryXml>, dir_name: &str, was_healed: &mut bool, is_new_project: bool| -> Result<(), CommandError> {
        let dir_rel_path = format!("{}/{}", HARVEY_FILES_DIR, dir_name);
        for media_entry in entries {
            // Ignore legacy Media folder entries
            if dir_name == MEDIA_DIR && media_entry.relative_path.contains(&format!("{}/", MEDIA_DIR)) {
                warn!("[Backend Load XML] Ignoring legacy Media folder entry: {}", media_entry.name);
                continue;
            }

            let media_stem = &media_entry.name;
            let stem_rel_path = format!("{}/{}", dir_rel_path, media_stem);
            let stem_abs_path = project_base_dir.join(&stem_rel_path);

            if !stem_abs_path.exists() || !stem_abs_path.is_dir() {
                warn!("[Backend Load XML] Media stem directory '{}' listed in XML does not exist on disk, skipping.", stem_abs_path.display());
                continue;
            }

            let mut media_children: Vec<FileEntry> = Vec::new();
            let media_file_rel_path = &media_entry.relative_path;
            let media_file_abs_path = project_base_dir.join(media_file_rel_path);

            if media_file_abs_path.exists() && media_file_abs_path.is_file() {
                let media_file_name = media_file_abs_path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                let media_file_canonical = canonicalize_path(&media_file_abs_path)
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
                }
            }

            let transcripts_dir_abs = project_base_dir.join(&stem_rel_path).join(TRANSCRIPTS_SUBDIR);
            
            // Auto-Healing: 1. Remove corrupted/cross-pollinated transcripts
            let mut valid_transcripts = Vec::new();
            for transcript_xml_entry in media_entry.transcripts.drain(..) {
                if transcript_xml_entry.relative_path.starts_with(&stem_rel_path) {
                    valid_transcripts.push(transcript_xml_entry);
                } else {
                    warn!("[Auto-Heal] Purging mismatched transcript '{}' from media stem '{}'", transcript_xml_entry.relative_path, stem_rel_path);
                    *was_healed = true;
                }
            }
            media_entry.transcripts = valid_transcripts;

            // Auto-Healing: 2. Re-discover orphaned transcripts physically present on disk
            if transcripts_dir_abs.exists() && transcripts_dir_abs.is_dir() {
                if let Ok(entries) = fs::read_dir(&transcripts_dir_abs) {
                    for entry in entries.filter_map(Result::ok) {
                        let path = entry.path();
                        if path.is_file() && path.extension().unwrap_or_default() == "json" {
                            if let Ok(rel_path_buf) = path.strip_prefix(&project_base_dir) {
                                let rel_path_str = rel_path_buf.to_string_lossy().replace("\\", "/");
                                if !media_entry.transcripts.iter().any(|t| t.relative_path == rel_path_str) {
                                    info!("[Auto-Heal] Adopting orphaned transcript '{}' into media stem '{}'", rel_path_str, stem_rel_path);
                                    
                                    let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                                    let mut lang_code = None;
                                    let parts: Vec<&str> = file_stem.split('.').collect();
                                    if parts.len() > 1 {
                                        let code = parts.last().unwrap().to_string();
                                        if code.len() == 2 {
                                            lang_code = Some(code);
                                        }
                                    }

                                    media_entry.transcripts.push(TranscriptEntryXml {
                                        name: path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string(),
                                        relative_path: rel_path_str,
                                        language_code: lang_code,
                                    });
                                    *was_healed = true;
                                }
                            }
                        }
                    }
                }
            }

            for transcript_xml_entry in &mut media_entry.transcripts {
                let transcript_rel_path = &transcript_xml_entry.relative_path;
                let transcript_abs_path = project_base_dir.join(transcript_rel_path);

                if transcript_abs_path.exists() && transcript_abs_path.is_file() {
                    let transcript_file_name = transcript_abs_path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                    transcript_xml_entry.name = transcript_file_name.clone();

                    let file_stem = transcript_abs_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    let parts: Vec<&str> = file_stem.split('.').collect();
                    if parts.len() > 1 {
                        let lang_code = parts.last().unwrap().to_string();
                        if lang_code.len() == 2 {
                            transcript_xml_entry.language_code = Some(lang_code);
                        }
                    }

                    let transcript_file_canonical = canonicalize_path(&transcript_abs_path)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| transcript_abs_path.to_string_lossy().to_string());

                    let media_asset_relative_path = media_entry.relative_path.clone().replace("\\", "/");
                    let media_asset_metadata = db_handler::load_asset_metadata(&project_data.project_uuid, &media_asset_relative_path)?;
                    let transcript_file_type = if let Some(metadata) = media_asset_metadata {
                        match metadata.asset_type.as_str() {
                            "audio" => "audio_transcript".to_string(),
                            "video" => "video_transcript".to_string(),
                            _ => "audio_transcript".to_string(),
                        }
                    } else {
                        "audio_transcript".to_string()
                    };

                    media_children.push(FileEntry {
                        name: transcript_file_name,
                        path: transcript_file_canonical,
                        relative_path: transcript_rel_path.clone().replace("\\", "/"),
                        file_type: transcript_file_type,
                        is_directory: false,
                        parent_relative_path: format!("{}/{}", stem_rel_path, TRANSCRIPTS_SUBDIR).replace("\\", "/"),
                        depth: 5,
                        speakers: None,
                        media_xml_identifier: Some(media_stem.clone()),
                        associated_transcripts: Vec::new(),
                        children: Vec::new(),
                    });
                }
            }

            file_entries.push(FileEntry {
                name: media_stem.clone(),
                path: stem_abs_path.to_string_lossy().to_string(),
                relative_path: stem_rel_path.replace("\\", "/"),
                file_type: "directory_media_stem".to_string(),
                is_directory: true,
                parent_relative_path: dir_rel_path.replace("\\", "/"),
                depth: 4,
                speakers: None,
                media_xml_identifier: Some(media_stem.clone()),
                associated_transcripts: Vec::new(),
                children: media_children,
            });
        }
        Ok(())
    };

    process_media_list(&mut project_data.audio_files.files, AUDIOS_DIR, &mut was_xml_healed, is_new_project_in_db)?;
    process_media_list(&mut project_data.video_files.files, VIDEOS_DIR, &mut was_xml_healed, is_new_project_in_db)?;
    process_media_list(&mut project_data.media_files.files, MEDIA_DIR, &mut was_xml_healed, is_new_project_in_db)?; // Legacy folder

    // Add imported transcript files to the main file_entries tree
    for standalone_transcript_entry in project_data.standalone_transcript_files.files.iter() {
        let transcript_abs_path = project_base_dir.join(&standalone_transcript_entry.relative_path);

        if transcript_abs_path.exists() && transcript_abs_path.is_file() {
            let transcript_file_name = transcript_abs_path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let transcript_file_canonical = canonicalize_path(&transcript_abs_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| transcript_abs_path.to_string_lossy().to_string());

            file_entries.push(FileEntry {
                name: transcript_file_name,
                path: transcript_file_canonical,
                relative_path: standalone_transcript_entry.relative_path.clone().replace("\\", "/"),
                file_type: "standalone_transcript".to_string(), // Explicitly set to standalone_transcript
                is_directory: false,
                parent_relative_path: format!("{}/{}", HARVEY_FILES_DIR, TRANSCRIPTS_DIR).replace("\\", "/"), // Assuming direct child of Transcripts folder
                depth: 3, // Assuming it's at the same level as media stems
                speakers: None,
                media_xml_identifier: None,
                associated_transcripts: Vec::new(),
                children: Vec::new(),
            });
        } else {
            warn!("[Backend Load XML] Imported transcript file listed in XML does not exist on disk: '{}'", transcript_abs_path.display());
        }
    }
    // Add imported transcript files to the main file_entries tree
    for standalone_transcript_entry in project_data.standalone_transcript_files.files.iter() {
        let transcript_abs_path = project_base_dir.join(&standalone_transcript_entry.relative_path);

        if transcript_abs_path.exists() && transcript_abs_path.is_file() {
            let transcript_file_name = transcript_abs_path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let transcript_file_canonical = canonicalize_path(&transcript_abs_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| transcript_abs_path.to_string_lossy().to_string());

            file_entries.push(FileEntry {
                name: transcript_file_name,
                path: transcript_file_canonical,
                relative_path: standalone_transcript_entry.relative_path.clone().replace("\\", "/"),
                file_type: "standalone_transcript".to_string(), // Explicitly set to standalone_transcript
                is_directory: false,
                parent_relative_path: format!("{}/{}", HARVEY_FILES_DIR, TRANSCRIPTS_DIR).replace("\\", "/"), // Assuming direct child of Transcripts folder
                depth: 3, // Assuming it's at the same level as media stems
                speakers: None,
                media_xml_identifier: None,
                associated_transcripts: Vec::new(),
                children: Vec::new(),
            });
        } else {
            warn!("[Backend Load XML] Imported transcript file listed in XML does not exist on disk: '{}'", transcript_abs_path.display());
        }
    }

    // Add document files to the main file_entries tree
    for doc_entry in project_data.document_files.files.iter() {
        let doc_abs_path = project_base_dir.join(&doc_entry.relative_path);

        if doc_abs_path.exists() && doc_abs_path.is_file() {
            let doc_file_name = doc_abs_path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let doc_file_canonical = canonicalize_path(&doc_abs_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| doc_abs_path.to_string_lossy().to_string());

            let file_type = if doc_file_name.to_lowercase().ends_with(".pdf") {
                "pdf".to_string()
            } else {
                "document".to_string()
            };

            file_entries.push(FileEntry {
                name: doc_file_name,
                path: doc_file_canonical,
                relative_path: doc_entry.relative_path.clone().replace("\\", "/"),
                file_type: file_type,
                is_directory: false,
                parent_relative_path: format!("{}/{}", HARVEY_FILES_DIR, DOCS_DIR).replace("\\", "/"),
                depth: 3,
                speakers: None,
                media_xml_identifier: None,
                associated_transcripts: Vec::new(),
                children: Vec::new(),
            });
        } else {
            warn!("[Backend Load XML] Document file listed in XML does not exist on disk: '{}'", doc_abs_path.display());
        }
    }

    // Add table files to the main file_entries tree
    for table_entry in project_data.table_files.files.iter() {
        let table_abs_path = project_base_dir.join(&table_entry.relative_path);

        if table_abs_path.exists() && table_abs_path.is_file() {
            let table_file_name = table_abs_path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let table_file_canonical = canonicalize_path(&table_abs_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| table_abs_path.to_string_lossy().to_string());

            file_entries.push(FileEntry {
                name: table_file_name,
                path: table_file_canonical,
                relative_path: table_entry.relative_path.clone().replace("\\", "/"),
                file_type: "table".to_string(),
                is_directory: false,
                parent_relative_path: format!("{}/{}", HARVEY_FILES_DIR, TABLES_DIR).replace("\\", "/"),
                depth: 3,
                speakers: None,
                media_xml_identifier: None,
                associated_transcripts: Vec::new(),
                children: Vec::new(),
            });
        } else {
            warn!("[Backend Load XML] Table file listed in XML does not exist on disk: '{}'", table_abs_path.display());
        }
    }

    // Add image files to the main file_entries tree
    for image_entry in project_data.image_files.files.iter() {
        let image_abs_path = project_base_dir.join(&image_entry.relative_path);

        if image_abs_path.exists() && image_abs_path.is_file() {
            let image_file_name = image_abs_path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let image_file_canonical = canonicalize_path(&image_abs_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| image_abs_path.to_string_lossy().to_string());

            file_entries.push(FileEntry {
                name: image_file_name,
                path: image_file_canonical,
                relative_path: image_entry.relative_path.clone().replace("\\", "/"),
                file_type: "image".to_string(),
                is_directory: false,
                parent_relative_path: format!("{}/{}", HARVEY_FILES_DIR, IMAGES_DIR).replace("\\", "/"),
                depth: 3,
                speakers: None,
                media_xml_identifier: None,
                associated_transcripts: Vec::new(),
                children: Vec::new(),
            });
        } else {
            warn!("[Backend Load XML] Image file listed in XML does not exist on disk: '{}'", image_abs_path.display());
        }
    }
    file_entries.sort_by(|a, b| a.name.cmp(&b.name));

    log::debug!(
        "[Backend Load XML] Assets: {}, Documents: {}, Tables: {}, Images: {}, Imported Transcripts: {}",
        file_entries.len(),
        project_data.document_files.files.len(),
        project_data.table_files.files.len(),
        project_data.image_files.files.len(),
        project_data.standalone_transcript_files.files.len()
    );

    // --- SYNC & SELF-HEALING: Ensure DB has metadata for all XML assets ---
    if is_new_project_in_db {
        let project_id_sync = project_data.project_uuid.clone();
        let mut current_xml_relative_paths = std::collections::HashSet::new();

    // Helper closure to sync media entries
    let mut sync_media_list = |entries: &Vec<MediaFileEntryXml>| -> Result<(), CommandError> {
        for media_entry in entries {
            let rel_path = media_entry.relative_path.clone().replace("\\", "/");
            current_xml_relative_paths.insert(rel_path.clone());

            if let Ok(None) = db_handler::load_asset_metadata(&project_id_sync, &rel_path) {
                info!("[Backend Sync] Creating missing metadata for media: {}", rel_path);
                let abs_path = project_base_dir.join(&rel_path).to_string_lossy().to_string();
                let file_name = Path::new(&abs_path).file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                
                let ext = Path::new(&file_name).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                let (asset_type, file_type) = match ext.as_str() {
                    "mp4" | "mov" | "avi" | "mkv" | "webm" => ("video", "video"),
                    _ => ("audio", "audio"),
                };

                let file_meta = FileMetadata {
                    file_name,
                    file_path: abs_path,
                    last_modified: Utc::now().to_rfc3339(),
                    created_at: Some(Utc::now().to_rfc3339()),
                    file_type: file_type.to_string(),
                    ..FileMetadata::default()
                };
                let _ = db_handler::save_asset_metadata(&project_id_sync, &file_meta, &rel_path, asset_type, None);
            }

            for transcript_entry in &media_entry.transcripts {
                let t_rel_path = transcript_entry.relative_path.clone().replace("\\", "/");
                current_xml_relative_paths.insert(t_rel_path.clone());

                if let Ok(None) = db_handler::load_asset_metadata(&project_id_sync, &t_rel_path) {
                    info!("[Backend Sync] Creating missing metadata for generated transcript: {}", t_rel_path);
                    let t_abs_path = project_base_dir.join(&t_rel_path).to_string_lossy().to_string();
                    let t_file_name = Path::new(&t_abs_path).file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                    
                    let media_rel_path = media_entry.relative_path.clone().replace("\\", "/");
                    let t_file_type = if let Ok(Some(m_meta)) = db_handler::load_asset_metadata(&project_id_sync, &media_rel_path) {
                        if m_meta.asset_type == "video" { "video-transcript" } else { "audio-transcript" }
                    } else { "audio-transcript" };

                    let t_file_meta = FileMetadata {
                        file_name: t_file_name,
                        file_path: t_abs_path,
                        last_modified: Utc::now().to_rfc3339(),
                        created_at: Some(Utc::now().to_rfc3339()),
                        language_code: transcript_entry.language_code.clone(),
                        file_type: t_file_type.to_string(),
                        ..FileMetadata::default()
                    };
                    let _ = db_handler::save_asset_metadata(&project_id_sync, &t_file_meta, &t_rel_path, &t_file_type, None);
                }
            }
        }
        Ok(())
    };

        sync_media_list(&project_data.audio_files.files)?;
        sync_media_list(&project_data.video_files.files)?;
        sync_media_list(&project_data.media_files.files)?;
        info!("[DB SYNC] Finished syncing media items.");

        // 1. Sync Documents
        for doc in &project_data.document_files.files {
        let rel_path = doc.relative_path.clone().replace("\\", "/");
        current_xml_relative_paths.insert(rel_path.clone());
        let existing_meta = db_handler::load_asset_metadata(&project_id_sync, &rel_path).ok().flatten();
        
        if existing_meta.is_none() {
            info!("[Backend Sync] Creating missing metadata for document: {}", rel_path);
            let abs_path = project_base_dir.join(&rel_path).to_string_lossy().to_string();
            let file_name = Path::new(&abs_path).file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            
            let file_meta = FileMetadata {
                file_name,
                file_path: abs_path,
                last_modified: Utc::now().to_rfc3339(),
                title: "".to_string(),
                description: "".to_string(),
                summary: "".to_string(),
                duration_seconds: None,
                width: None, height: None, frame_rate: None, bit_rate: None,
                audio_codec: None, video_codec: None,
                created_at: Some(Utc::now().to_rfc3339()),
                original_import_path: None,
                speaker_names: None,
                waveform_data: None,
                language_code: doc.language_code.clone(),
                properties: None,
                file_type: "document".to_string(),
                page_count: None,
                thumbnail: None,
            };
            let _ = db_handler::save_asset_metadata(&project_id_sync, &file_meta, &rel_path, "document", None);
        } else if let Some(mut meta_db) = existing_meta {
            // Update language_code if it differs
            if meta_db.language_code != doc.language_code {
                meta_db.language_code = doc.language_code.clone();
                let file_meta_to_save = FileMetadata {
                    file_name: meta_db.file_name,
                    file_path: meta_db.file_path,
                    last_modified: meta_db.last_modified,
                    title: meta_db.title.unwrap_or_default(),
                    description: meta_db.description.unwrap_or_default(),
                    summary: meta_db.summary.unwrap_or_default(),
                    duration_seconds: meta_db.duration_seconds,
                    width: meta_db.width,
                    height: meta_db.height,
                    frame_rate: meta_db.frame_rate,
                    bit_rate: meta_db.bit_rate,
                    audio_codec: meta_db.audio_codec,
                    video_codec: meta_db.video_codec,
                    created_at: meta_db.creation_time,
                    original_import_path: meta_db.original_import_path,
                    speaker_names: meta_db.speaker_names_json.and_then(|s| serde_json::from_str(&s).ok()),
                    waveform_data: meta_db.waveform_data,
                    language_code: meta_db.language_code,
                    properties: meta_db.properties,
                    file_type: meta_db.file_type.unwrap_or("document".to_string()),
                    page_count: meta_db.page_count,
                    thumbnail: meta_db.thumbnail,
                };
                let _ = db_handler::save_asset_metadata(&project_id_sync, &file_meta_to_save, &rel_path, &meta_db.asset_type, meta_db.custom_fields_json.as_deref());
            }
        }
    }

        // 2. Sync Table Files
        for table in &project_data.table_files.files {
        let rel_path = table.relative_path.clone().replace("\\", "/");
        current_xml_relative_paths.insert(rel_path.clone());
        let existing_meta = db_handler::load_asset_metadata(&project_id_sync, &rel_path).ok().flatten();

        if existing_meta.is_none() {
            info!("[Backend Sync] Creating missing metadata for table: {}", rel_path);
            let abs_path = project_base_dir.join(&rel_path).to_string_lossy().to_string();
            let file_name = Path::new(&abs_path).file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            
            let mut props_map = serde_json::Map::new();
            if let Some(xml_val) = table.has_headers {
                props_map.insert("has_headers".to_string(), json!(xml_val));
            }

            let file_meta = FileMetadata {
                file_name,
                file_path: abs_path,
                last_modified: Utc::now().to_rfc3339(),
                title: "".to_string(),
                description: "".to_string(),
                summary: "".to_string(),
                duration_seconds: None,
                width: None, height: None, frame_rate: None, bit_rate: None,
                audio_codec: None, video_codec: None,
                created_at: Some(Utc::now().to_rfc3339()),
                original_import_path: None,
                speaker_names: None,
                waveform_data: None,
                language_code: table.language_code.clone(),
                properties: Some(serde_json::to_string(&props_map).unwrap()),
                file_type: "table".to_string(),
                page_count: None,
                thumbnail: None,
            };
            let _ = db_handler::save_asset_metadata(&project_id_sync, &file_meta, &rel_path, "table", None);
        } else if let Some(mut meta_db) = existing_meta {
            let mut changed = false;
            
            if meta_db.language_code != table.language_code {
                 meta_db.language_code = table.language_code.clone();
                 changed = true;
            }

            let props_json = meta_db.properties.clone();
            let mut props_map: serde_json::Map<String, serde_json::Value> = props_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            
            let db_has_headers = props_map.get("has_headers").and_then(|v| v.as_bool());
            
            if let Some(xml_val) = table.has_headers {
                 if db_has_headers != Some(xml_val) {
                     props_map.insert("has_headers".to_string(), json!(xml_val));
                     meta_db.properties = Some(serde_json::to_string(&props_map).unwrap());
                     changed = true;
                 }
            }

            if changed {
                let file_meta_to_save = FileMetadata {
                    file_name: meta_db.file_name,
                    file_path: meta_db.file_path,
                    last_modified: meta_db.last_modified,
                    title: meta_db.title.unwrap_or_default(),
                    description: meta_db.description.unwrap_or_default(),
                    summary: meta_db.summary.unwrap_or_default(),
                    duration_seconds: meta_db.duration_seconds,
                    width: meta_db.width,
                    height: meta_db.height,
                    frame_rate: meta_db.frame_rate,
                    bit_rate: meta_db.bit_rate,
                    audio_codec: meta_db.audio_codec,
                    video_codec: meta_db.video_codec,
                    created_at: meta_db.creation_time,
                    original_import_path: meta_db.original_import_path,
                    speaker_names: meta_db.speaker_names_json.and_then(|s| serde_json::from_str(&s).ok()),
                    waveform_data: meta_db.waveform_data,
                    language_code: meta_db.language_code,
                    properties: meta_db.properties,
                    file_type: meta_db.file_type.unwrap_or("table".to_string()),
                    page_count: meta_db.page_count,
                    thumbnail: meta_db.thumbnail,
                };
                let _ = db_handler::save_asset_metadata(&project_id_sync, &file_meta_to_save, &rel_path, &meta_db.asset_type, meta_db.custom_fields_json.as_deref());
            }
        }
    }

        // 3. Sync Imported Transcripts
        for transcript in &project_data.standalone_transcript_files.files {
        let rel_path = transcript.relative_path.clone().replace("\\", "/");
        current_xml_relative_paths.insert(rel_path.clone());
        if let Ok(None) = db_handler::load_asset_metadata(&project_id_sync, &rel_path) {
            info!("[Backend Sync] Creating missing metadata for imported transcript: {}", rel_path);
            let abs_path = project_base_dir.join(&rel_path).to_string_lossy().to_string();
            let file_name = Path::new(&abs_path).file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            
            let file_meta = FileMetadata {
                file_name,
                file_path: abs_path,
                last_modified: Utc::now().to_rfc3339(),
                title: "".to_string(),
                description: "".to_string(),
                summary: "".to_string(),
                duration_seconds: None,
                width: None, height: None, frame_rate: None, bit_rate: None,
                audio_codec: None, video_codec: None,
                created_at: Some(Utc::now().to_rfc3339()),
                original_import_path: None,
                speaker_names: None,
                waveform_data: None,
                language_code: None,
                properties: None,
                file_type: "standalone_transcript".to_string(),
                page_count: None,
                thumbnail: None,
            };
            let _ = db_handler::save_asset_metadata(&project_id_sync, &file_meta, &rel_path, "standalone_transcript", None);
        }
    }

        // 4. Sync Images
        for image in &project_data.image_files.files {
        let rel_path = image.relative_path.clone().replace("\\", "/");
        current_xml_relative_paths.insert(rel_path.clone());
        if let Ok(None) = db_handler::load_asset_metadata(&project_id_sync, &rel_path) {
            info!("[Backend Sync] Creating missing metadata for image: {}", rel_path);
            let abs_path = project_base_dir.join(&rel_path).to_string_lossy().to_string();
            let file_name = Path::new(&abs_path).file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            
            let file_meta = FileMetadata {
                file_name,
                file_path: abs_path,
                last_modified: Utc::now().to_rfc3339(),
                title: "".to_string(),
                description: "".to_string(),
                summary: "".to_string(),
                duration_seconds: None,
                width: None, height: None, frame_rate: None, bit_rate: None,
                audio_codec: None, video_codec: None,
                created_at: Some(Utc::now().to_rfc3339()),
                original_import_path: None,
                speaker_names: None,
                waveform_data: None,
                language_code: None,
                properties: None,
                file_type: "image".to_string(),
                page_count: None,
                thumbnail: None,
            };
            let _ = db_handler::save_asset_metadata(&project_id_sync, &file_meta, &rel_path, "image", None);
        }
    }

        // 5. PRUNE: Remove metadata for files no longer in XML
        // This handles the "ghost" items issue in the dropdown.
        if let Ok(db_path) = db_handler::get_db_path() {
            if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                let mut stmt = conn.prepare("SELECT asset_relative_path FROM asset_metadata WHERE project_id = ?1")?;
                let db_paths: Vec<String> = stmt.query_map(params![project_id_sync], |row| row.get(0))?
                    .filter_map(|r| r.ok())
                    .collect();

                for db_path in db_paths {
                    // If it's an attachment, we don't prune it yet as they aren't explicitly in the main XML lists
                    if db_path.contains("/attachments/") { continue; }

                    if !current_xml_relative_paths.contains(&db_path) {
                        info!("[Backend Prune] Removing stale metadata for: {} (project_id: {})", db_path, project_id_sync);
                        let _ = db_handler::delete_asset_metadata(&project_id_sync, &db_path);
                    }
                }
            }
        }
    } // End of if is_new_project_in_db

    if was_uuid_generated || was_xml_healed {
        let save_reason = if was_xml_healed { "healed data" } else { "new UUID" };
        match save_project_xml(&xml_path, &project_data) {
            Ok(_) => info!("[Backend Load Manifest] Successfully saved updated project manifest with {} to {}", save_reason, xml_path.display()),
            Err(e) => warn!("[Backend Load Manifest] Failed to save updated project manifest with {} to {}: {}", save_reason, xml_path.display(), e),
        }
    }

    Ok(ProjectViewData {
        project_name,
        project_xml_path: project_xml_path.to_string(),
        base_directory,
        project_uuid: project_data.project_uuid.clone(),
        files: file_entries,
        document_files: project_data.document_files.files,
        table_files: project_data.table_files.files,
        image_files: project_data.image_files.files,
        standalone_transcript_files: project_data.standalone_transcript_files.files,
        document_metadata_files: project_data.document_metadata_files.files,
    })
}


#[tauri::command]
pub async fn import_media(
    app_handle: AppHandle, 
    source_file_path_str: String, 
    project_xml_path_str: String,
    import_type: Option<String>
) -> Result<FileEntry, CommandError> {
    info!("[Backend Import] Source: '{}', Project XML: '{}', Type Hint: '{:?}'", source_file_path_str, project_xml_path_str, import_type);
    let source_path = canonicalize_path(&source_file_path_str).unwrap_or_else(|_| PathBuf::from(&source_file_path_str));
    let project_xml_path = canonicalize_path(&project_xml_path_str).unwrap_or_else(|_| PathBuf::from(&project_xml_path_str));

    if !source_path.exists() || !source_path.is_file() {
        return Err(CommandError::from(format!("Source file not found: {}", source_file_path_str)));
    }
    let project_base_dir = project_xml_path.parent().ok_or_else(|| CommandError::from("Could not get project base directory"))?;
    if !project_base_dir.exists() || !project_base_dir.is_dir() {
        return Err(CommandError::from(format!("Project base directory not found: {}", project_base_dir.display())));
    }

    let original_source_filename_os = source_path.file_name().ok_or_else(|| CommandError::from("Could not get filename"))?;
    let original_source_filename = original_source_filename_os.to_string_lossy().to_string();

    // Truncate the source filename's stem for use as the actual filename in the project
    let truncated_source_filename = truncate_filename_stem(&original_source_filename, MAX_FILENAME_STEM_LENGTH);
    info!("[Backend Import] Original source filename: '{}', Truncated for use in project: '{}'", original_source_filename, truncated_source_filename);

    // Generate media_stem_identifier from the original source filename's stem, then truncate it.
    let original_media_stem = source_path.file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| CommandError::from("Invalid source filename stem."))?;
    let media_stem_truncated = Path::new(&truncate_filename_stem(&format!("{}.tmp", original_media_stem), MAX_FILENAME_STEM_LENGTH))
        .file_stem().unwrap_or_default().to_string_lossy().into_owned();
    info!("[Backend Import] Original media stem: '{}', Truncated media stem: '{}'", original_media_stem, media_stem_truncated);

    // --- Detect file type early using ffprobe on source ---
    let ffprobe_args = vec![
        "-v".to_string(), "quiet".to_string(),
        "-print_format".to_string(), "json".to_string(),
        "-show_format".to_string(),
        "-show_streams".to_string(),
        source_path.to_string_lossy().to_string(),
    ];

    info!("[Backend Import] Running ffprobe early for: {}", source_path.display());
    let mut duration_seconds: Option<f64> = None;
    let mut width: Option<i32> = None;
    let mut height: Option<i32> = None;
    let mut frame_rate: Option<f32> = None;
    let mut bit_rate_overall: Option<i64> = None;
    let mut audio_codec: Option<String> = None;
    let mut video_codec: Option<String> = None;

    match app_handle.shell().sidecar("ffprobe").expect("ffprobe sidecar not configured in tauri.conf.json").args(ffprobe_args).output().await {
        Ok(output) => {
            if output.status.success() {
                let ffprobe_json_str = String::from_utf8_lossy(&output.stdout).to_string();
                if let Ok(parsed_ffprobe_output) = serde_json::from_str::<FFProbeOutput>(&ffprobe_json_str) {
                    duration_seconds = parse_duration_str_to_seconds(parsed_ffprobe_output.format.duration.clone())
                        .or_else(|| parse_duration_str_to_seconds(parsed_ffprobe_output.format.tags.as_ref().and_then(|t| t.duration.clone())));
                    bit_rate_overall = parsed_ffprobe_output.format.bit_rate.as_deref().and_then(|s| s.parse().ok());
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
                                if bit_rate_overall.is_none() { bit_rate_overall = stream.bit_rate.as_deref().and_then(|s| s.parse().ok()); }
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
                }
            }
        }
        Err(e) => { error!("[Backend Import] Early ffprobe execution error: {}", e); }
    }

    let final_asset_type: String;
    let media_dir_name: &str;

    // Determine target directory using import_type hint if provide, otherwise fallback to ffprobe
    if let Some(ref hint) = import_type {
        match hint.to_lowercase().as_str() {
            "video" => {
                final_asset_type = "video".to_string();
                media_dir_name = VIDEOS_DIR;
            },
            "audio" => {
                final_asset_type = "audio".to_string();
                media_dir_name = AUDIOS_DIR;
            },
            _ => {
                // If "auto" or unknown hint, use ffprobe detection
                if video_codec.is_some() {
                    final_asset_type = "video".to_string();
                    media_dir_name = VIDEOS_DIR;
                } else if audio_codec.is_some() {
                    final_asset_type = "audio".to_string();
                    media_dir_name = AUDIOS_DIR;
                } else {
                    final_asset_type = source_path.extension()
                        .and_then(|s| s.to_str())
                        .map_or_else(|| "media".to_string(), |ext| ext.to_lowercase());
                    media_dir_name = AUDIOS_DIR;
                }
            }
        }
    } else {
        // Legacy behavior if no import_type is passed
        if video_codec.is_some() {
            final_asset_type = "video".to_string();
            media_dir_name = VIDEOS_DIR;
        } else if audio_codec.is_some() {
            final_asset_type = "audio".to_string();
            media_dir_name = AUDIOS_DIR;
        } else {
            final_asset_type = source_path.extension()
                .and_then(|s| s.to_str())
                .map_or_else(|| "media".to_string(), |ext| ext.to_lowercase());
            media_dir_name = AUDIOS_DIR;
        }
    }

    // Load project XML to check for conflicts
    let xml_content = fs::read_to_string(&project_xml_path)?;
    let mut project_data: ProjectXml = serde_json::from_str(&xml_content)?;

    let media_asset_dir = project_base_dir.join(HARVEY_FILES_DIR).join(media_dir_name);
    fs::create_dir_all(&media_asset_dir)?;

    let mut folder_counter = 0;
    let (media_stem_identifier, truncated_source_filename) = loop {
        let current_stem = if folder_counter == 0 { media_stem_truncated.clone() } else { format!("{}_{}", media_stem_truncated, folder_counter) };
        let candidate_folder = media_asset_dir.join(&current_stem);
        
        let name_conflict = project_data.find_media(&current_stem).is_some();

        if !candidate_folder.exists() && !name_conflict {
            let file_name = format!("{}.{}", current_stem, source_path.extension().and_then(|e| e.to_str()).unwrap_or(""));
            break (current_stem, file_name);
        }
        folder_counter += 1;
        if folder_counter > 1000 {
            return Err(CommandError::from(format!("Could not find unique folder for media stem '{}' after 1000 attempts.", media_stem_truncated)));
        }
    };

    let media_stem_base_path = media_asset_dir.join(&media_stem_identifier);
    let media_subfolder_path = media_stem_base_path.join(MEDIA_SUBDIR);
    let transcripts_subfolder_path = media_stem_base_path.join(TRANSCRIPTS_SUBDIR);
    let destination_media_path = media_subfolder_path.join(&truncated_source_filename);

    fs::create_dir_all(&media_subfolder_path)?;
    fs::create_dir_all(&transcripts_subfolder_path)?;

    fs::copy(&source_path, &destination_media_path)?;
    info!("[Backend Import] File copied to {}", destination_media_path.display());

    let canonical_dest_path = canonicalize_path(&destination_media_path)
        .map_err(|e| CommandError::Io(format!("Failed to canonicalize destination media path {}: {:?}", destination_media_path.display(), e)))?;

    // Prepare and save metadata to SQLite database
    let file_metadata_for_db = FileMetadata {
        file_name: truncated_source_filename.clone(),
        file_path: destination_media_path.to_string_lossy().into_owned(),
        last_modified: Utc::now().to_rfc3339(),
        title: String::new(),
        description: String::new(),
        summary: String::new(),
        duration_seconds,
        width,
        height,
        frame_rate,
        bit_rate: bit_rate_overall,
        audio_codec: audio_codec.clone(),
        video_codec: video_codec.clone(),
        created_at: Some(Utc::now().to_rfc3339()),
        original_import_path: Some(source_file_path_str.clone()),
        speaker_names: None,
        waveform_data: None,
        language_code: None,
        properties: None,
        file_type: final_asset_type.clone(),
        page_count: None,
        thumbnail: None,
    };

    let db_key_relative_path = Path::new(HARVEY_FILES_DIR)
        .join(media_dir_name)
        .join(&media_stem_identifier)
        .join(MEDIA_SUBDIR)
        .join(&truncated_source_filename)
        .to_string_lossy()
        .replace("\\", "/");

    match db_handler::save_asset_metadata(
        &project_data.project_uuid,
        &file_metadata_for_db,
        &db_key_relative_path,
        &final_asset_type,
        None,
    ) {
        Ok(_) => info!("[Backend Import] Successfully saved media metadata to DB for: {}", db_key_relative_path),
        Err(e) => warn!("[Backend Import] Failed to save media metadata to DB: {}", e),
    }

    if let Err(e) = db_handler::save_media_transcript_data(
        &project_data.project_uuid,
        &db_key_relative_path,
        Some(source_file_path_str.as_str()),
        None,
        None,
        None,
        None,
    ) {
        warn!("[Backend Import] Failed to save media_transcript_data: {}", e);
    }

    let new_media_entry = MediaFileEntryXml {
        name: media_stem_identifier.clone(),
        original_path: Some(source_file_path_str.clone()),
        relative_path: db_key_relative_path.clone(),
        speakers: Some(SpeakersXml::default()),
        transcripts: Vec::new(),
    };

    if final_asset_type == "video" {
        project_data.video_files.files.push(new_media_entry);
        project_data.video_files.files.sort_by(|a, b| a.name.cmp(&b.name));
    } else {
        project_data.audio_files.files.push(new_media_entry);
        project_data.audio_files.files.sort_by(|a, b| a.name.cmp(&b.name));
    }

    save_project_xml(&project_xml_path, &project_data)?;
    info!("[Backend Import] XML updated with entry '{}'.", media_stem_identifier);

    // Construct the FileEntry for the newly imported media
    let final_file_entry = FileEntry {
        name: truncated_source_filename.clone(),
        path: canonical_dest_path.to_string_lossy().to_string(),
        relative_path: db_key_relative_path.clone(),
        file_type: "media".to_string(),
        is_directory: false,
        parent_relative_path: Path::new(HARVEY_FILES_DIR)
            .join(media_dir_name)
            .join(&media_stem_identifier)
            .join(MEDIA_SUBDIR)
            .to_string_lossy()
            .replace("\\", "/"),
        depth: 5,
        speakers: Some(SpeakersXml::default()),
        media_xml_identifier: Some(media_stem_identifier.clone()),
        associated_transcripts: Vec::new(),
        children: Vec::new(),
    };

    Ok(final_file_entry)
}


#[tauri::command]
pub async fn delete_project_item( item_path: String, project_xml_path: String) -> Result<(), CommandError> {
    info!("[Backend Delete] Request for: {} in project_xml: {}", item_path, project_xml_path);
    let item_path_buf = canonicalize_path(&item_path).unwrap_or_else(|_| PathBuf::from(&item_path));
    let xml_path_buf = canonicalize_path(&project_xml_path).unwrap_or_else(|_| PathBuf::from(&project_xml_path));

    if !xml_path_buf.exists() || !xml_path_buf.is_file() {
        return Err(CommandError::from(format!("Project XML not found: {}", project_xml_path)));
    }
    let project_base_dir = xml_path_buf.parent().ok_or_else(|| CommandError::from("Could not get project base dir"))?;

    // Get project_id for DB operations
    let project_xml_content_for_uuid = fs::read_to_string(&xml_path_buf)
        .map_err(|e| CommandError::Io(format!("Failed to read project XML for UUID from {}: {}", xml_path_buf.display(), e)))?;
    let project_data_for_uuid: ProjectXml = serde_json::from_str(&project_xml_content_for_uuid)
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
                "standalone_transcript".to_string()
            } else if item_type_guess == "other" && path_lower.contains(&tables_folder) && (ext == "csv" || ext == "xlsx") {
                "table".to_string()
            } else {
                item_type_guess
            }
        };
        let mut project_data: ProjectXml = serde_json::from_str(&fs::read_to_string(&xml_path_buf)?)?;
        let mut xml_changed = false;

        match item_type_guess.as_str() {
            "media" | "directory_media_stem" => {
                let stem_dir_rel_path = std::path::Path::new(&item_relative_path_guess).parent().and_then(|p| p.parent()).map(|p| p.to_string_lossy().replace("\\", "/")).unwrap_or_default();
                if !stem_dir_rel_path.is_empty() {
                    if project_data.remove_media_by_stem_dir(&stem_dir_rel_path) {
                        info!("[Backend Delete] Cleaned up XML media entry for non-existent stem dir '{}'.", stem_dir_rel_path);
                        xml_changed = true;
                    }
                }
            },
            "audio_transcript" | "video_transcript" => {
                let stem_dir_rel_path = std::path::Path::new(&item_relative_path_guess).parent().and_then(|p| p.parent()).map(|p| p.to_string_lossy().replace("\\", "/")).unwrap_or_default();
                if !stem_dir_rel_path.is_empty() {
                    if let Some(media_entry) = project_data.find_media_by_stem_dir_mut(&stem_dir_rel_path) {
                        let initial_transcript_len = media_entry.transcripts.len();
                        media_entry.transcripts.retain(|t| t.relative_path != item_relative_path_guess);
                        if media_entry.transcripts.len() < initial_transcript_len {
                            info!("[Backend Delete] Cleaned up XML media-associated transcript entry '{}'.", item_relative_path_guess);
                            xml_changed = true;
                        }
                    }
                }
            },
            "standalone_transcript" => {
                let initial_len = project_data.standalone_transcript_files.files.len();
                project_data.standalone_transcript_files.files.retain(|t| t.relative_path != item_relative_path_guess);
                if project_data.standalone_transcript_files.files.len() < initial_len {
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
            "standalone_transcript".to_string()
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
                let (media_stem_dir_path, media_entry_rel_path) = {
                    let mut path = None;
                    let mut rel = None;
                    if let Ok(xml_content) = fs::read_to_string(&xml_path_buf) {
                        if let Ok(project_data) = serde_json::from_str::<ProjectXml>(&xml_content) {
                            if let Some(entry) = project_data.find_media(&media_stem) {
                                // Extract the folder from relative_path, e.g. "harvey_files/Audios/Stem/media/file.wav" -> "harvey_files/Audios/Stem"
                                let rel_path = Path::new(&entry.relative_path);
                                if let Some(parent) = rel_path.parent().and_then(|p| p.parent()) {
                                    path = Some(project_base_dir.join(parent));
                                    rel = Some(entry.relative_path.clone());
                                }
                            }
                        }
                    }
                    (path.unwrap_or_else(|| project_base_dir.join(HARVEY_FILES_DIR).join(MEDIA_DIR).join(media_stem)), rel)
                };

                // Cleanup highlights for all associated transcripts before deleting
                if let Ok(project_data) = serde_json::from_str::<ProjectXml>(&fs::read_to_string(&xml_path_buf).unwrap_or_default()) {
                    if let Some(media_entry) = project_data.find_media(&media_stem) {
                        for transcript in &media_entry.transcripts {
                            if let Err(e) = delete_annotations_from_db(&project_id_for_db, &transcript.relative_path, "lexical") {
                                warn!("[Backend Delete] Failed to delete highlights for transcript {} during media deletion: {}", transcript.relative_path, e);
                            } else {
                                info!("[Backend Delete] Deleted highlights for transcript {} during media deletion.", transcript.relative_path);
                            }
                            // Delete transcript metadata from DB
                            let _ = db_handler::delete_asset_metadata(&project_id_for_db, &transcript.relative_path);
                        }
                        // Delete media file metadata from DB
                        let _ = db_handler::delete_asset_metadata(&project_id_for_db, &media_entry.relative_path);
                    }
                }

                if media_stem_dir_path.exists() && media_stem_dir_path.is_dir() {
                    info!("[Backend Delete] Deleting media stem directory: {}", media_stem_dir_path.display());
                    fs::remove_dir_all(&media_stem_dir_path).map_err(|e| CommandError::from(format!("Failed to delete directory {}: {}", media_stem_dir_path.display(), e)))?;

                    info!("[Backend Delete] Updating XML to remove entry for '{}'", media_stem);
                    let mut project_data: ProjectXml = serde_json::from_str(&fs::read_to_string(&xml_path_buf)?)?;
                    if project_data.remove_media(media_stem) {
                        save_project_xml(&xml_path_buf, &project_data)?;
                        info!("[Backend Delete] XML media entry removed.");
                    } else {
                        warn!("[Backend Delete] Deleted directory but no XML entry found for '{}'.", media_stem);
                    }
                } else {
                    warn!("[Backend Delete] Media stem directory {} not found. Assuming already deleted. Cleaning up XML.", media_stem_dir_path.display());
                     let mut project_data: ProjectXml = serde_json::from_str(&fs::read_to_string(&xml_path_buf)?)?;
                     if project_data.remove_media(&media_stem) {
                         save_project_xml(&xml_path_buf, &project_data)?;
                         info!("[Backend Delete] XML media entry removed during cleanup.");
                     }
                }
            } else {
                return Err(CommandError::from(format!("Could not determine media stem for media file deletion: {}", item_path)));
            }
        },
        "audio_transcript" | "video_transcript" => {
             if let Some(media_stem) = media_stem_opt.as_deref() {
                info!("[Backend Delete] Deleting media-associated transcript file: {}", item_path_buf.display());
                fs::remove_file(&item_path_buf).map_err(|e| CommandError::from(format!("Failed to delete file {}: {}", item_path_buf.display(), e)))?;

                if let Err(e) = delete_annotations_from_db(&project_id_for_db, &item_relative_path, "lexical") {
                    warn!("[Backend Delete] Failed to delete highlights for transcript {} during transcript deletion: {}", item_relative_path, e);
                } else {
                    info!("[Backend Delete] Deleted highlights for transcript {} during transcript deletion.", item_relative_path);
                }

                // Delete transcript metadata from DB
                if let Err(e) = db_handler::delete_asset_metadata(&project_id_for_db, &item_relative_path) {
                    warn!("[Backend Delete] Failed to delete asset metadata from DB for project_id {}, transcript {}: {}", project_id_for_db, item_relative_path, e);
                } else {
                    info!("[Backend Delete] Deleted asset metadata from DB for project_id {}, transcript {}", project_id_for_db, item_relative_path);
                }

                info!("[Backend Delete] Updating XML to remove transcript link for stem name '{}' with path '{}'", media_stem, item_relative_path);
                let mut project_data: ProjectXml = serde_json::from_str(&fs::read_to_string(&xml_path_buf)?)?;
                let mut xml_changed = false;
                let stem_dir_rel_path = std::path::Path::new(&item_relative_path).parent().and_then(|p| p.parent()).map(|p| p.to_string_lossy().replace("\\", "/")).unwrap_or_default();
                
                if !stem_dir_rel_path.is_empty() {
                    if let Some(media_entry) = project_data.find_media_by_stem_dir_mut(&stem_dir_rel_path) {
                        let initial_transcript_len = media_entry.transcripts.len();
                        media_entry.transcripts.retain(|t| t.relative_path != item_relative_path);
                        if media_entry.transcripts.len() < initial_transcript_len {
                            info!("[Backend Delete] Transcript entry removed from XML for stem dir '{}'.", stem_dir_rel_path);
                            xml_changed = true;
                        } else {
                            warn!("[Backend Delete] Deleted transcript file, but no matching entry found in XML for path '{}' under stem dir '{}'.", item_relative_path, stem_dir_rel_path);
                        }
                    } else {
                        warn!("[Backend Delete] Deleted transcript file, but stem dir '{}' not found in XML.", stem_dir_rel_path);
                    }
                }
                
                if xml_changed {
                    save_project_xml(&xml_path_buf, &project_data)?;
                    info!("[Backend Delete] XML updated.");
                }
            } else {
                return Err(CommandError::from(format!("Could not determine media stem for transcript: {}", item_path)));
            }
        },
        "standalone_transcript" => {
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

            // Delete highlights from DB
            if let Err(e) = delete_annotations_from_db(&project_id_for_db, &item_relative_path, "lexical") {
                warn!("[Backend Delete] Failed to delete highlights for imported transcript {} during deletion: {}", item_relative_path, e);
            } else {
                info!("[Backend Delete] Deleted highlights for imported transcript {} during deletion.", item_relative_path);
            }

            info!("[Backend Delete] Updating XML to remove imported transcript entry '{}'", item_relative_path);
            let mut project_data: ProjectXml = serde_json::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            let initial_entries = project_data.standalone_transcript_files.files.len();
            project_data.standalone_transcript_files.files.retain(|t| t.relative_path != item_relative_path);

            // document_metadata_files list in XML is no longer managed for imported transcript metadata

            if project_data.standalone_transcript_files.files.len() < initial_entries {
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
            let mut project_data: ProjectXml = serde_json::from_str(&fs::read_to_string(&xml_path_buf)?)?;
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

            // Also delete table styles from the database
            if let Err(e) = db_handler::delete_table_styles(&project_id_for_db, &item_path) {
                warn!("[Backend Delete Table] Failed to delete table styles from DB for project_id {}, table {}: {}", project_id_for_db, item_path, e);
            } else {
                info!("[Backend Delete Table] Deleted table styles from DB for project_id {}, table {}", project_id_for_db, item_path);
            }

            // Also delete any table schema configs associated with this table
            if let Err(e) = db_handler::delete_table_schema(&project_id_for_db, &item_relative_path) {
                warn!("[Backend Delete Table] Failed to delete table schema configs from DB for project_id {}, table {}: {}", project_id_for_db, item_relative_path, e);
            } else {
                info!("[Backend Delete Table] Deleted table schema configs from DB for project_id {}, table {}", project_id_for_db, item_relative_path);
            }

            // Also delete any chart configs associated with this table
            if let Err(e) = chart_handler::delete_all_charts_for_table(&project_id_for_db, &item_relative_path) {
                warn!("[Backend Delete Table] Failed to delete chart configs from DB for project_id {}, table {}: {}", project_id_for_db, item_relative_path, e);
            } else {
                info!("[Backend Delete Table] Deleted chart configs from DB for project_id {}, table {}", project_id_for_db, item_relative_path);
            }

            info!("[Backend Delete] Updating XML to remove table link with path '{}'", item_relative_path);
            let mut project_data: ProjectXml = serde_json::from_str(&fs::read_to_string(&xml_path_buf)?)?;
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
            let mut project_data: ProjectXml = serde_json::from_str(&fs::read_to_string(&xml_path_buf)?)?;
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


fn rename_asset_with_folder(
    app_handle: &tauri::AppHandle,
    item_path: &Path, // This is the path to the actual file, e.g., .../STEM/media/file.mp4
    new_name_input: &str, // This is the new stem name, e.g., "new_video_stem" - RENAMED PARAMETER
    project_xml_path: &Path,
    project_base_dir: &Path,
    project_id_for_db: &str,
    item_type: &str,
) -> Result<PathBuf, CommandError> {
    let old_stem_name;
    let new_item_path;
    let old_relative_path;
    let new_relative_path;
    let new_filename;
    let mut category_dir_name = String::new(); // Declare here for use in multiple blocks

    // Apply truncation to the new name input for consistency with import
    let new_name = truncate_filename_stem(new_name_input, MAX_FILENAME_STEM_LENGTH); // NEW LINE

    // Determine old_stem_name and construct new_item_path based on item_type
    match item_type {
        "media" => {
            // For media, item_path is like .../HARVEY_FILES_DIR/CATEGORY_DIR/OLD_STEM/MEDIA_SUBDIR/file.ext
            // Resolve the category directory (e.g., Audios, Videos, or Media) from the item_path
            category_dir_name = item_path
                .parent() // .../OLD_STEM/MEDIA_SUBDIR
                .and_then(|p| p.parent()) // .../OLD_STEM
                .and_then(|p| p.parent()) // .../Audios or .../Videos or .../Media
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
                .ok_or_else(|| CommandError::from("Could not get category directory name from item path"))?
                .to_string();

            old_stem_name = item_path.parent() // .../OLD_STEM/MEDIA_SUBDIR
                                .and_then(|p| p.parent()) // .../OLD_STEM
                                .and_then(|p| p.file_name())
                                .and_then(|s| s.to_str())
                                .ok_or_else(|| CommandError::from("Could not get old media stem directory name"))?;

            let media_sub_dir_name = item_path.parent()
                                        .and_then(|p| p.file_name())
                                        .and_then(|s| s.to_str())
                                        .ok_or_else(|| CommandError::from("Could not get media sub directory name"))?; // This should be "media"

            let old_filename_os = item_path.file_name().ok_or_else(|| CommandError::from("Could not get old filename"))?;
            let extension = item_path.extension().and_then(|s| s.to_str()).unwrap_or("");
            new_filename = format!("{}.{}", new_name, extension); // Use the now truncated 'new_name'

            let new_stem_base_path = project_base_dir.join(HARVEY_FILES_DIR).join(&category_dir_name).join(&new_name);
            let new_media_subfolder_path = new_stem_base_path.join(media_sub_dir_name); // Re-use "media"
            new_item_path = new_media_subfolder_path.join(&new_filename);

            // Construct old and new relative paths for DB and XML updates
            old_relative_path = item_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\", "/");
            new_relative_path = new_item_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\", "/");

            // Perform folder rename (renaming the STEM directory)
            let old_stem_dir_path = project_base_dir.join(HARVEY_FILES_DIR).join(&category_dir_name).join(old_stem_name);
            let new_stem_dir_path = project_base_dir.join(HARVEY_FILES_DIR).join(&category_dir_name).join(&new_name);

            if old_stem_dir_path == new_stem_dir_path {
                info!("[Backend Rename] Old and new media stem paths are identical. No folder rename needed.");
            } else {
                if new_stem_dir_path.exists() {
                    return Err(CommandError::from(format!("A folder named '{}' already exists for media.", new_name)));
                }
                fs::rename(&old_stem_dir_path, &new_stem_dir_path)?;
                info!("[Backend Rename] Renamed media stem directory from {} to {}", old_stem_dir_path.display(), new_stem_dir_path.display());
            }

            // Rename the actual media file inside the newly moved folder
            let old_media_file_path_in_new_folder = new_media_subfolder_path.join(old_filename_os);
            if old_media_file_path_in_new_folder.exists() {
                fs::rename(&old_media_file_path_in_new_folder, &new_item_path)?;
                info!("[Backend Rename] Renamed media file from {} to {}", old_media_file_path_in_new_folder.display(), new_item_path.display());
            } else {
                warn!("[Backend Rename] Old media file path in new folder not found: {}. Skipping file rename.", old_media_file_path_in_new_folder.display());
            }
        },
        _ => {
            // Existing logic for doc, image, table, etc.
            // For these, new_name is already the stem, and it's used for folder and filename.
            // If these also need truncation, similar logic would apply here.
            // Based on the problem description, the issue is with media files.
            let parent_dir = item_path.parent().ok_or_else(|| CommandError::from("Could not get parent directory"))?;
            old_stem_name = parent_dir.file_name().and_then(|s| s.to_str()).ok_or_else(|| CommandError::from("Could not get old stem"))?;

            let asset_dir = parent_dir.parent().ok_or_else(|| CommandError::from("Could not get asset directory"))?;
            let new_folder_path = asset_dir.join(&new_name); // Use the now truncated 'new_name'

            if new_folder_path.exists() {
                return Err(CommandError::from(format!("A folder named '{}' already exists.", new_name))); // Use the now truncated 'new_name'
            }
            fs::rename(parent_dir, &new_folder_path)?;

            let old_filename = item_path.file_name().and_then(|s| s.to_str()).ok_or_else(|| CommandError::from("Could not get old filename"))?;
            let extension = item_path.extension().and_then(|s| s.to_str()).unwrap_or("");
            new_filename = format!("{}.{}", new_name, extension); // Use the now truncated 'new_name'
            new_item_path = new_folder_path.join(&new_filename);

            let old_item_path_in_new_folder = new_folder_path.join(old_filename);
            if old_item_path_in_new_folder.exists() {
                fs::rename(old_item_path_in_new_folder, &new_item_path)?;
            }

            old_relative_path = item_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\", "/");
            new_relative_path = new_item_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\", "/");
        }
    }

    // Update database entry
    db_handler::rename_asset_metadata_key(
        project_id_for_db,
        &old_relative_path,
        &new_relative_path,
        &new_item_path.to_string_lossy(),
        &new_filename, // Pass the new filename for the file_name field in DB
    )?;

    let mut project_data: ProjectXml = serde_json::from_str(&fs::read_to_string(project_xml_path)?)?;

    match item_type {
        "doc" => {
            if let Some(entry) = project_data.document_files.files.iter_mut().find(|f| f.relative_path == old_relative_path) {
                entry.name = new_filename.clone();
                entry.relative_path = new_relative_path.clone();
            }
        },
        "image" => {
            if let Some(entry) = project_data.image_files.files.iter_mut().find(|f| f.relative_path == old_relative_path) {
                entry.name = new_filename.clone();
                entry.relative_path = new_relative_path.clone();
            }
            // Also rename annotations in the database if they exist
            if let Err(e) = db_handler::rename_annotations_in_db(
                project_id_for_db,
                &old_relative_path,
                &new_relative_path,
                "image",
            ) {
                warn!("[Backend Rename] Failed to rename image annotations in DB for project_id {} from {} to {}: {}", project_id_for_db, old_relative_path, new_relative_path, e);
            } else {
                info!("[Backend Rename] Successfully renamed image annotations in DB for project_id {} from {} to {}", project_id_for_db, old_relative_path, new_relative_path);
            }
        },
        "table" => {
            if let Some(entry) = project_data.table_files.files.iter_mut().find(|f| f.relative_path == old_relative_path) {
                entry.name = new_filename.clone();
                entry.relative_path = new_relative_path.clone();
            }
        },
        "media" => {
            if let Some(entry) = project_data.find_media_by_relative_path_mut(&old_relative_path) {
                entry.name = new_name.to_string(); // Update XML entry name to new stem (truncated)
                entry.relative_path = new_relative_path.clone(); // Update XML entry relative_path to new media file path (truncated)

                // Update associated transcripts' relative paths and names
                for transcript_entry in entry.transcripts.iter_mut() {
                    let old_transcript_relative_path = transcript_entry.relative_path.clone();
                    let transcript_filename = PathBuf::from(&old_transcript_relative_path)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string();
                    
                    // Construct new relative path for transcript using the detected category directory
                    let new_transcript_relative_path = Path::new(HARVEY_FILES_DIR)
                        .join(&category_dir_name)
                        .join(&new_name) // Use new media stem (truncated)
                        .join(TRANSCRIPTS_SUBDIR)
                        .join(&transcript_filename)
                        .to_string_lossy()
                        .replace("\\", "/");

                    transcript_entry.relative_path = new_transcript_relative_path.clone();
                    // No need to change transcript_entry.name as it's already the filename

                    // Rename transcript metadata in DB
                    if let Err(e) = db_handler::rename_asset_metadata_key(
                        project_id_for_db,
                        &old_transcript_relative_path,
                        &new_transcript_relative_path,
                        &project_base_dir.join(&new_transcript_relative_path).to_string_lossy(), // new full path
                        &transcript_filename, // new filename
                    ) {
                        warn!("[Backend Rename] Failed to rename transcript metadata in DB for project_id {} from {} to {}: {}", project_id_for_db, old_transcript_relative_path, new_transcript_relative_path, e);
                    } else {
                        info!("[Backend Rename] Successfully renamed transcript metadata in DB for project_id {} from {} to {}", project_id_for_db, old_transcript_relative_path, new_transcript_relative_path);
                    }
                }
            }
        },
        _ => return Err(CommandError::from(format!("Unsupported item type for rename: {}", item_type))),
    };

    save_project_xml(project_xml_path, &project_data)?;

    let payload = ItemRenamedPayload {
        old_path: item_path.to_string_lossy().into_owned(),
        new_path: new_item_path.to_string_lossy().into_owned(),
        new_name: new_filename,
        item_type: item_type.to_string(),
        project_xml_path: project_xml_path.to_string_lossy().into_owned(),
        base_directory: project_base_dir.to_string_lossy().into_owned(),
    };
    app_handle.emit("item_renamed", payload).map_err(|e| CommandError::from(format!("Failed to emit event: {}", e)))?;

    Ok(new_item_path)
}

#[tauri::command]
pub async fn rename_project_item( app_handle: tauri::AppHandle, item_path: String, new_name: String, project_xml_path: String) -> Result<String, CommandError> {
    info!("[Backend Rename] Request: Item='{}', NewNameParam='{}'", item_path, new_name);
    let item_path_buf = canonicalize_path(&item_path).unwrap_or_else(|_| PathBuf::from(&item_path));
    let xml_path_buf = canonicalize_path(&project_xml_path).unwrap_or_else(|_| PathBuf::from(&project_xml_path));
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
    let project_data_for_uuid: ProjectXml = serde_json::from_str(&project_xml_content_for_uuid)
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

    let (item_type, media_stem_opt, item_relative_path_buf) = get_item_details(&item_path_buf, project_base_dir)?;
    let item_relative_path = item_relative_path_buf.to_string_lossy().replace("\\", "/");
    info!("[Backend Rename] Item type: '{}', Media Stem: {:?}, Rel Path: '{}'", item_type, media_stem_opt, item_relative_path);

    let parent_dir = item_path_buf.parent().ok_or_else(|| CommandError::from(format!("Could not get parent directory for {}", item_path_buf.display())))?;

    let final_new_path: PathBuf;

    match item_type.as_str() {
        "media" | "doc" | "image" | "table" => {
            let new_name_path_buf = PathBuf::from(new_name_trimmed);
            let new_stem_from_input = new_name_path_buf
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| CommandError::from("Could not get stem from new name."))?;
            // For these types, rename_asset_with_folder expects the new_name to be the stem.
            // The original extension will be re-applied by rename_asset_with_folder.
            final_new_path = rename_asset_with_folder(&app_handle, &item_path_buf, new_stem_from_input, &xml_path_buf, project_base_dir, &project_id_for_db, &item_type)?;
        },
        "audio_transcript" | "video_transcript" => {
            let new_filename_with_ext = new_name_trimmed;
            let new_path = parent_dir.join(new_filename_with_ext);
            final_new_path = new_path.clone();

            if new_filename_with_ext.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) { return Err(CommandError::from("New filename contains invalid characters.")); }
            if !new_filename_with_ext.ends_with(".json") { return Err(CommandError::from("Transcript filename must end with .json")); }
            if new_filename_with_ext.starts_with('.') { return Err(CommandError::from("Filename cannot start with a dot.")); }

            if item_path_buf == new_path { info!("[Backend Rename] New path is same as old path. No action needed."); return Ok(item_path); }

            if new_path.exists() {
                 let canon_old = canonicalize_path(&item_path_buf).ok();
                 let canon_new = canonicalize_path(&new_path).ok();
                 if canon_old.is_some() && canon_new.is_some() && canon_old != canon_new {
                     return Err(CommandError::from(format!("File named '{}' already exists.", new_filename_with_ext)));
                 } else {
                     debug!("[Backend Rename] Target path exists but might be same file (case change?). Allowing rename attempt.");
                 }
            }

            info!("[Backend Rename] Renaming transcript file {} -> {}", item_path_buf.display(), new_path.display());
            fs::rename(&item_path_buf, &new_path).map_err(|e| CommandError::from(format!("Failed to rename file: {}", e)))?;

            let new_relative_path_buf = new_path.strip_prefix(project_base_dir)?;
            let new_relative_path = new_relative_path_buf.to_string_lossy().replace("\\", "/");
            let stem_dir_rel_path = std::path::Path::new(&item_relative_path).parent().and_then(|p| p.parent()).map(|p| p.to_string_lossy().replace("\\", "/")).unwrap_or_default();

            info!("[Backend Rename] Updating XML for stem '{}': Path '{}' -> '{}', name -> '{}'", stem_dir_rel_path, item_relative_path, new_relative_path, new_filename_with_ext);
            let mut project_data: ProjectXml = serde_json::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            let mut xml_changed = false;

            if !stem_dir_rel_path.is_empty() {
                if let Some(media_entry) = project_data.find_media_by_stem_dir_mut(&stem_dir_rel_path) {
                    if let Some(transcript_entry) = media_entry.transcripts.iter_mut().find(|t| t.relative_path == item_relative_path) {
                        transcript_entry.name = new_filename_with_ext.to_string();
                        transcript_entry.relative_path = new_relative_path;
                        media_entry.transcripts.sort_by(|a,b| a.name.cmp(&b.name));
                        xml_changed = true;
                        info!("[Backend Rename] XML transcript entry updated.");
                    } else {
                        warn!("[Backend Rename] Renamed transcript file, but could not find matching path '{}' in XML under stem '{}'.", item_relative_path, stem_dir_rel_path);
                    }
                } else {
                    warn!("[Backend Rename] Renamed transcript file, but could not find stem dir '{}' in XML.", stem_dir_rel_path);
                }
            }

            if xml_changed {
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Rename] XML saved.");

                let payload = ItemRenamedPayload {
                    old_path: item_path_buf.to_string_lossy().into_owned(),
                    new_path: new_path.to_string_lossy().into_owned(),
                    new_name: new_filename_with_ext.to_string(),
                    item_type: item_type.to_string(),
                    project_xml_path: xml_path_buf.to_string_lossy().into_owned(),
                    base_directory: project_base_dir.to_string_lossy().into_owned(),
                };
                if let Err(e) = app_handle.emit("item_renamed", payload) {
                    warn!("[Backend Rename] Failed to emit item_renamed event for media-associated transcript: {}", e);
                }
            }
        },
        "standalone_transcript" => {
            let old_transcript_file_abs_path = &item_path_buf;
            let old_transcript_folder_abs_path = parent_dir;
            let old_transcript_relative_path = &item_relative_path; // This is key for DB

            let new_transcript_filename_with_ext_str = new_name_trimmed; // Use the full name from input
            if new_transcript_filename_with_ext_str.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) { return Err(CommandError::from("New transcript name contains invalid characters.")); }
            if !new_transcript_filename_with_ext_str.ends_with(".json") { return Err(CommandError::from("Imported transcript filename must end with .json")); }
            if new_transcript_filename_with_ext_str.starts_with('.') { return Err(CommandError::from("Filename cannot start with a dot.")); }

            let new_transcript_filename_path_buf = PathBuf::from(new_transcript_filename_with_ext_str);
            let new_transcript_stem_str = new_transcript_filename_path_buf
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| CommandError::from("Could not get stem from new imported transcript name."))?;
            let new_transcript_filename_pathbuf = PathBuf::from(&new_transcript_filename_with_ext_str);

            let new_transcript_file_path_in_old_folder = old_transcript_folder_abs_path.join(&new_transcript_filename_pathbuf);

            let transcripts_root_abs_path = old_transcript_folder_abs_path.parent()
                .ok_or_else(|| CommandError::from(format!("Could not get Transcripts root from {}", old_transcript_folder_abs_path.display())))?;

            let new_transcript_folder_abs_path = transcripts_root_abs_path.join(new_transcript_stem_str);

            // Check if no effective change
            if *old_transcript_file_abs_path == new_transcript_file_path_in_old_folder && old_transcript_folder_abs_path == &new_transcript_folder_abs_path {
                info!("[Backend Rename] Imported transcript name and folder name are effectively unchanged. No action needed.");
                return Ok(item_path);
            }

            // Check for conflicts
            if old_transcript_folder_abs_path != &new_transcript_folder_abs_path && new_transcript_folder_abs_path.exists() {
                return Err(CommandError::from(format!("A folder named '{}' already exists for imported transcripts. Cannot rename folder.", new_transcript_stem_str)));
            }
            let final_new_transcript_file_abs_path = new_transcript_folder_abs_path.join(&new_transcript_filename_pathbuf);
            final_new_path = final_new_transcript_file_abs_path.clone();
            if final_new_transcript_file_abs_path.exists() {
                let canon_old_abs = canonicalize_path(old_transcript_file_abs_path).map_err(|e| CommandError::from(format!("Cannot canonicalize old transcript path {}: {:?}", old_transcript_file_abs_path.display(), e)))?;
                let canon_final_target_abs = canonicalize_path(&final_new_transcript_file_abs_path).map_err(|e| CommandError::from(format!("Cannot canonicalize final target transcript path {}: {:?}", final_new_transcript_file_abs_path.display(), e)))?;
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
            let mut project_data: ProjectXml = serde_json::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            // Removed: let mut xml_actually_changed_for_standalone_transcript = false;

            if let Some(entry) = project_data.standalone_transcript_files.files.iter_mut().find(|t| t.relative_path == *old_transcript_relative_path) {
                entry.name = new_transcript_filename_with_ext_str.to_string();
                entry.relative_path = new_relative_path_for_xml_and_db.clone();
                project_data.standalone_transcript_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                // xml_actually_changed_for_standalone_transcript = true; // Variable removed
                info!("[Backend Rename] XML imported transcript entry updated. Saving XML.");
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Rename] XML saved for imported transcript rename.");

                let payload = ItemRenamedPayload {
                    old_path: item_path_buf.to_string_lossy().into_owned(),
                    new_path: final_new_transcript_file_abs_path.to_string_lossy().into_owned(),
                    new_name: new_transcript_filename_with_ext_str.to_string(),
                    item_type: "standalone_transcript".to_string(),
                    project_xml_path: xml_path_buf.to_string_lossy().into_owned(),
                    base_directory: project_base_dir.to_string_lossy().into_owned(),
                };
                if let Err(e) = app_handle.emit("item_renamed", payload) {
                    warn!("[Backend Rename] Failed to emit item_renamed event for standalone_transcript: {}", e);
                }
            } else {
                // This should ideally not happen if DB update was successful, as it means XML was out of sync.
                warn!("[Backend Rename] Renamed imported transcript (FS & DB), but could not find matching old relative path '{}' in XML. XML not saved.", old_transcript_relative_path);
            }
            // Logic for updating project_data.document_metadata_files.files is REMOVED.
            // The conditional save based on the flag is removed; save now happens inside the 'if let Some(entry)' block.
        },
        _ => {
            error!("[Backend Rename] Renaming items of type '{}' is not supported directly: {}", item_type, item_path);
            return Err(CommandError::from(format!("Renaming not supported for item type '{}'. Rename the primary associated asset.", item_type)));
        }
    }

    info!("[Backend Rename] Success for: {}", item_path);
    Ok(final_new_path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

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
                <standaloneTranscriptFiles/>
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
#[allow(unused_variables)]
pub async fn reveal_in_file_explorer(app: AppHandle, file_path_str: String) -> Result<(), CommandError> {
    info!("[CMD] Revealed in Explorer: {}", file_path_str);

    let mut cleaned_path_str = file_path_str;

    // On Windows, remove the \\?\ prefix if present
    if cfg!(windows) && cleaned_path_str.starts_with(r"\\\\?\\") {
        cleaned_path_str = cleaned_path_str.trim_start_matches(r"\\\\?\\").to_string();
        info!(r"reveal_in_file_explorer: Removed \\?\\ prefix. Cleaned path: '{}'", cleaned_path_str);
    }

    // Normalize all slashes to forward slashes for consistent PathBuf operations
    let normalized_path_str = cleaned_path_str.replace('\\', "/");

    let path = PathBuf::from(&normalized_path_str);

    if !path.exists() {
        error!("reveal_in_file_explorer: Error - File or directory not found: {:?}", path);
        return Err(CommandError::from(format!("File or directory not found: {}", path.display())));
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        // On Windows, use `explorer.exe /select,` to open the parent directory and select the file
        // Or, if it's a directory, just open the directory
        let final_path_for_explorer = path.to_string_lossy().replace('/', "\\"); // Ensure backslashes for explorer.exe

        if path.is_file() {
            info!("[CMD] Executing: explorer.exe /select, {}", final_path_for_explorer);
            Command::new("explorer.exe")
                .arg("/select,")
                .arg(final_path_for_explorer)
                .spawn()
                .map_err(|e| CommandError::from(format!("Failed to open explorer: {}", e)))?;
        } else {
            info!("[CMD] Executing: explorer.exe {}", final_path_for_explorer);
            Command::new("explorer.exe")
                .arg(final_path_for_explorer)
                .spawn()
                .map_err(|e| CommandError::from(format!("Failed to open explorer: {}", e)))?;
        }
    }

    #[cfg(target_os = "macos")]
    {
        app.opener().reveal_item_in_dir(&path).map_err(|e| {
            error!("reveal_in_file_explorer: Error opening URL: {}", e);
            CommandError::from(format!("Failed to open project location: {}", e))
        })?;
    }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]

        {

            // For other OS, use app.opener().open_url

            let dir_url = format!("file://{}", path.to_string_lossy().replace('\\', "/"));

            info!("[CMD] Executing: app.opener().open_url {}", dir_url);

            app.opener().open_url(dir_url, None::<String>).map_err(|e| {

                error!("reveal_in_file_explorer: Error opening URL: {}", e);

                CommandError::from(format!("Failed to open project location: {}", e))

            })?;

        }

    

        Ok(())
    }
#[tauri::command]
pub async fn export_project_manifest(project_id: String, manifest_path: String) -> Result<(), CommandError> {
    info!("[Backend Export Manifest] Exporting SQLite data to JSON manifest for project: {}", project_id);

    // In a fully developed CQRS architecture, this function will query `asset_metadata` and build the `ProjectXml` struct.
    // For now, it simply touches the manifest to ensure its modified timestamp updates,
    // since the current synchronous logic already keeps the manifest updated.
    // This allows the Svelte frontend to start hooking into the command for debounced "save" logic seamlessly.

    let path = std::path::PathBuf::from(&manifest_path);
    if path.exists() {
        let _ = std::fs::OpenOptions::new().append(true).open(&path); // Touch the file to update modify timestamp
    }

    Ok(())
}
