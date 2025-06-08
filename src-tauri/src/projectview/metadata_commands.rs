// src-tauri/src/projectview/metadata_commands.rs
use tauri::AppHandle;
use log::{debug, error};
use crate::projectview::db_handler::{
    load_asset_metadata, save_asset_metadata, FileMetadataWithCustomFieldsFromDb
};
use crate::projectview::shared_types::FileMetadata; // For the payload structure
use std::path::Path; // Added for path manipulation
use chrono::Utc;     // Added for timestamp

#[tauri::command]
pub async fn get_asset_metadata_command(
    _app_handle: AppHandle, // Use _app_handle if not directly used, but good to keep for consistency
    asset_relative_path: String,
) -> Result<Option<FileMetadataWithCustomFieldsFromDb>, String> {
    debug!("[CMD] get_asset_metadata_command for path: {}", asset_relative_path);
    load_asset_metadata(&asset_relative_path)
        .map_err(|e| {
            error!("[CMD] Error in get_asset_metadata_command for {}: {}", asset_relative_path, e);
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

    // Create a mutable copy of the metadata from the payload to sanitize it
    let mut sanitized_metadata_for_db = metadata_payload;

    // Sanitize file_name: Derive it from asset_relative_path to ensure consistency with the key
    let path_obj = Path::new(&asset_relative_path);
    sanitized_metadata_for_db.file_name = path_obj.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("") // Fallback to empty string if filename cannot be derived
        .to_string();

    // Sanitize file_path: Use asset_relative_path itself.
    // This is a defensive measure. The db_handler.save_asset_metadata's ON CONFLICT clause
    // already prevents file_path and file_name from being updated from payload if the record exists.
    // This sanitization primarily affects new records if this command is used for initial insertion.
    sanitized_metadata_for_db.file_path = asset_relative_path.clone();

    // last_modified should reflect the update time.
    sanitized_metadata_for_db.last_modified = Utc::now().to_rfc3339();

    let custom_fields_json_string: Option<String> = custom_fields_payload
        .and_then(|json_val| serde_json::to_string(&json_val).ok());

    save_asset_metadata(
        &sanitized_metadata_for_db, // Use the sanitized version
        &asset_relative_path,       // This is the key for the DB
        &asset_type,
        custom_fields_json_string.as_deref(),
    )
    .map_err(|e| {
        error!("[CMD] Error in update_asset_metadata_command for {}: {}", asset_relative_path, e);
        e.to_string()
    })
}

// --- Custom Field Definition Commands ---

use crate::projectview::shared_types::{CustomFieldDefinition, CustomFieldScope};
use crate::projectview::db_handler::{
    add_custom_field_definition,
    get_all_custom_field_definitions
    // get_custom_field_definition, // Import if individual get needed later
    // update_custom_field_definition, // Import if update command needed later
    // delete_custom_field_definition  // Import if delete command needed later
};
use log::info;


#[tauri::command]
pub async fn create_custom_field_definition_command(
    _app_handle: AppHandle,
    project_id: String,
    field_key: String,
    field_name: String,
    field_type: String,
    scope_str: String
    // default_value: Option<String> // Removed from signature
) -> Result<(), String> {
    debug!("[CMD] create_custom_field_definition_command for project_id '{}': key='{}', name='{}', type='{}', scope='{}'",
           project_id, field_key, field_name, field_type, scope_str);

    let scope = CustomFieldScope::from_db_string(&scope_str);

    let current_timestamp = Utc::now().to_rfc3339();

    let definition = CustomFieldDefinition {
        field_key: field_key.clone(),
        field_name,
        field_type,
        scope,
        default_value: None, // Explicitly set to None
        created_at: current_timestamp.clone(),
        updated_at: current_timestamp,
    };

    match add_custom_field_definition(project_id.as_str(), &definition) {
        Ok(_) => {
            info!("[CMD] Custom field definition created successfully for project_id {}: {}", project_id, field_key);
            Ok(())
        }
        Err(e) => {
            error!("[CMD] Error creating custom field definition for project_id {}: {}: {}", project_id, field_key, e);
            Err(format!("Failed to create custom field definition for project_id '{}', key '{}': {}", project_id, field_key, e))
        }
    }
}

#[tauri::command]
pub async fn get_all_custom_field_definitions_command(
    _app_handle: AppHandle,
    project_id: String,
) -> Result<Vec<CustomFieldDefinition>, String> {
    debug!("[CMD] get_all_custom_field_definitions_command called for project_id: {}", project_id);
    match get_all_custom_field_definitions(project_id.as_str()) {
        Ok(definitions) => {
            info!("[CMD] Retrieved {} custom field definitions for project_id {}.", definitions.len(), project_id);
            Ok(definitions)
        }
        Err(e) => {
            error!("[CMD] Error retrieving all custom field definitions for project_id {}: {}", project_id, e);
            Err(format!("Failed to retrieve custom field definitions for project_id {}: {}", project_id, e))
        }
    }
}
