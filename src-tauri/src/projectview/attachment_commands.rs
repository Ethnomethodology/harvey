// src-tauri/src/projectview/attachment_commands.rs
use tauri::AppHandle;
use log::{debug, error, info, warn};
use std::path::{Path, PathBuf};
use std::fs;
use crate::projectview::db_handler;
use crate::projectview::shared_types::{ProjectXml, FileMetadata};
use quick_xml;
use chrono::Utc;
use serde_json::json;

#[tauri::command]
pub async fn upload_attachment(
    _app_handle: AppHandle,
    project_xml_path_str: String,
    asset_relative_path: String,
    source_file_path_str: String,
) -> Result<String, String> {
    debug!(
        "[CMD] upload_attachment: asset_relative_path={}, source_file_path_str={}",
        asset_relative_path, source_file_path_str
    );

    let project_xml_path = PathBuf::from(&project_xml_path_str);
    let project_base_dir = project_xml_path.parent().ok_or_else(|| "Could not get project base directory.".to_string())?;

    let project_xml_content = fs::read_to_string(&project_xml_path)
        .map_err(|e| format!("Failed to read project XML: {}", e))?;
    let project_data: ProjectXml = quick_xml::de::from_str(&project_xml_content)
        .map_err(|e| format!("Failed to parse project XML: {}", e))?;

    let project_id = project_data.project_uuid;
    if project_id.is_empty() {
        return Err("Project UUID is missing in the project file.".to_string());
    }

    let source_path = PathBuf::from(&source_file_path_str);
    if !source_path.exists() || !source_path.is_file() {
        return Err(format!("Source file not found: {}", source_file_path_str));
    }

    let file_name = source_path.file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Could not get filename from source path.".to_string())?;

    // Determine target directory: asset_parent_dir/attachments
    let asset_abs_path = project_base_dir.join(&asset_relative_path);
    let asset_parent_dir = asset_abs_path.parent().ok_or_else(|| "Could not get asset parent directory.".to_string())?;
    let attachments_dir = asset_parent_dir.join("attachments");

    if !attachments_dir.exists() {
        fs::create_dir_all(&attachments_dir).map_err(|e| format!("Failed to create attachments directory: {}", e))?;
    }

    let target_path = attachments_dir.join(file_name);
    
    // If file already exists, we might want to append a suffix, but for now let's just copy (overwrite)
    fs::copy(&source_path, &target_path).map_err(|e| format!("Failed to copy file to attachments: {}", e))?;

    let target_path_str = target_path.to_string_lossy().to_string();

    // Update DB
    match db_handler::load_asset_metadata(&project_id, &asset_relative_path) {
        Ok(Some(metadata_from_db)) => {
            let mut custom_fields: Vec<serde_json::Value> = metadata_from_db.custom_fields_json
                .as_deref()
                .and_then(|json| serde_json::from_str(json).ok())
                .unwrap_or_else(Vec::new);

            let mut attachments: Vec<String> = custom_fields.iter()
                .find(|f| f.get("key").and_then(|k| k.as_str()) == Some("attachments"))
                .and_then(|f| f.get("value").and_then(|v| v.as_str()))
                .and_then(|v| serde_json::from_str(v).ok())
                .unwrap_or_else(Vec::new);

            if !attachments.contains(&target_path_str) {
                attachments.push(target_path_str.clone());
            }

            let attachments_json_string = json!(attachments).to_string();

            if let Some(existing_field) = custom_fields.iter_mut().find(|f| f.get("key").and_then(|k| k.as_str()) == Some("attachments")) {
                if let Some(obj) = existing_field.as_object_mut() {
                    obj.insert("value".to_string(), json!(attachments_json_string));
                }
            } else {
                let new_field = json!({
                    "key": "attachments",
                    "value": attachments_json_string
                });
                custom_fields.push(new_field);
            }

            let updated_custom_fields_json_str = serde_json::to_string(&custom_fields).unwrap_or_else(|_| "[]".to_string());

            let file_metadata = FileMetadata {
                file_name: metadata_from_db.file_name,
                file_path: metadata_from_db.file_path,
                last_modified: Utc::now().to_rfc3339(),
                title: metadata_from_db.title.unwrap_or_default(),
                description: metadata_from_db.description.unwrap_or_default(),
                summary: metadata_from_db.summary.unwrap_or_default(),
                duration_seconds: metadata_from_db.duration_seconds,
                width: metadata_from_db.width,
                height: metadata_from_db.height,
                frame_rate: metadata_from_db.frame_rate,
                bit_rate: metadata_from_db.bit_rate,
                audio_codec: metadata_from_db.audio_codec,
                video_codec: metadata_from_db.video_codec,
                created_at: metadata_from_db.creation_time,
                original_import_path: metadata_from_db.original_import_path,
                speaker_names: metadata_from_db.speaker_names_json.and_then(|s| serde_json::from_str(&s).ok()),
                waveform_data: metadata_from_db.waveform_data,
                language_code: metadata_from_db.language_code,
                properties: metadata_from_db.properties,
            };

            db_handler::save_asset_metadata(
                &project_id,
                &file_metadata,
                &asset_relative_path,
                &metadata_from_db.asset_type,
                Some(&updated_custom_fields_json_str)
            ).map_err(|e| e.to_string())?;

            info!("[CMD] Attachment uploaded and metadata updated for {}", asset_relative_path);
            Ok(target_path_str)
        }
        Ok(None) => Err(format!("Asset metadata not found for path: {}", asset_relative_path)),
        Err(e) => Err(format!("Error loading asset metadata: {}", e)),
    }
}
