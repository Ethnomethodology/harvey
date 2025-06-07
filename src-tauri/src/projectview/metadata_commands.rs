// src-tauri/src/projectview/metadata_commands.rs
use tauri::AppHandle;
use log::{debug, error};
use crate::projectview::db_handler::{
    load_asset_metadata, save_asset_metadata, FileMetadataWithCustomFieldsFromDb
};
use crate::projectview::shared_types::FileMetadata; // For the payload structure

#[tauri::command]
pub async fn get_asset_metadata_command(
    _app_handle: AppHandle, // Use _app_handle if not directly used, but good to keep for consistency
    asset_relative_path: String,
) -> Result<Option<FileMetadataWithCustomFieldsFromDb>, String> {
    debug!("[CMD] get_asset_metadata_command for path: {}", asset_relative_path);
    load_asset_metadata(&asset_relative_path)
        .map_err(|e| {
            error!("Failed to load asset metadata for {}: {}", asset_relative_path, e);
            e.to_string()
        })
}

#[tauri::command]
pub async fn update_asset_metadata_command(
    _app_handle: AppHandle,
    asset_relative_path: String,
    metadata_payload: FileMetadata, // This is shared_types::FileMetadata
    custom_fields_payload: Option<serde_json::Value>,
    asset_type: String, // Need to know the asset type
) -> Result<(), String> {
    debug!("[CMD] update_asset_metadata_command for path: {}, type: {}", asset_relative_path, asset_type);

    let custom_fields_json_string: Option<String> = custom_fields_payload
        .and_then(|json_val| serde_json::to_string(&json_val).ok());

    save_asset_metadata(
        &metadata_payload,
        &asset_relative_path,
        &asset_type,
        custom_fields_json_string.as_deref(),
    )
    .map_err(|e| {
        error!("Failed to save asset metadata for {}: {}", asset_relative_path, e);
        e.to_string()
    })
}
