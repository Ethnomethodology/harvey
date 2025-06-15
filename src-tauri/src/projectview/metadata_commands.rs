// src-tauri/src/projectview/metadata_commands.rs
use tauri::AppHandle;
use log::{debug, error, info}; // Added info
use std::path::{Path, PathBuf}; // Added PathBuf here
use std::fs; // Added fs
use chrono::Utc; // Added for timestamp
use quick_xml; // Added for parsing XML
use crate::projectview::shared_types::ProjectXml; // Ensure ProjectXml is imported at the top
use crate::projectview::db_handler::{
    load_asset_metadata, save_asset_metadata, FileMetadataWithCustomFieldsFromDb
};
use crate::projectview::shared_types::FileMetadata; // For the payload structure
// Path and Utc are already imported above

#[tauri::command]
pub async fn get_asset_metadata_command(
    _app_handle: AppHandle,
    project_id: String,
    asset_relative_path: String,
) -> Result<Option<FileMetadataWithCustomFieldsFromDb>, String> {
    info!("[CMD] get_asset_metadata_command for project_id {}: path: {}", project_id, asset_relative_path);
    load_asset_metadata(&project_id, &asset_relative_path)
        .map_err(|e| {
            error!("[CMD] Error in get_asset_metadata_command for project_id {}, path {}: {}", project_id, asset_relative_path, e);
            e.to_string()
        })
}

#[tauri::command]
pub async fn update_asset_metadata_command(
    _app_handle: AppHandle,
    project_xml_path_str: String, // Added: Path to the project's XML file
    asset_relative_path: String,
    metadata_payload: FileMetadata,
    custom_fields_payload: Option<serde_json::Value>,
    asset_type: String,
) -> Result<(), String> {
    debug!("[CMD] update_asset_metadata_command for project_xml: {}, asset_path: {}, type: {}", project_xml_path_str, asset_relative_path, asset_type);

    // Read project_uuid from XML
    let project_xml_path = PathBuf::from(project_xml_path_str); // PathBuf is now directly available
    let project_xml_content_for_uuid = fs::read_to_string(&project_xml_path) // fs is now directly available
        .map_err(|e| format!("Failed to read project XML for UUID from {}: {}", project_xml_path.display(), e))?;
    let project_data_for_uuid: ProjectXml = quick_xml::de::from_str(&project_xml_content_for_uuid) // ProjectXml is now directly available
        .map_err(|e| format!("Failed to parse project XML for UUID from {}: {}", project_xml_path.display(), e))?;

    let project_id_for_db = project_data_for_uuid.project_uuid;
    if project_id_for_db.is_empty() {
        let err_msg = format!("Project UUID is empty in XML file: {}. Cannot update asset metadata without project_id.", project_xml_path.display());
        error!("[CMD] {}", err_msg);
        return Err(err_msg);
    }

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
        &project_id_for_db,         // Pass project_id
        &sanitized_metadata_for_db,
        &asset_relative_path,
        &asset_type,
        custom_fields_json_string.as_deref(),
    )
    .map_err(|e| {
        error!("[CMD] Error in update_asset_metadata_command for {}: {}", asset_relative_path, e);
        e.to_string()
    })
}

// --- Custom Field Definition Commands ---

use crate::projectview::shared_types::{CustomFieldDefinition, CustomFieldScope}; // ProjectXml already imported at top
use crate::projectview::db_handler::{
    add_custom_field_definition,
    get_all_custom_field_definitions
    // get_custom_field_definition, // Import if individual get needed later
    // update_custom_field_definition, // Import if update command needed later
    // delete_custom_field_definition  // Import if delete command needed later
};
// info and quick_xml already imported at top


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
        project_id: project_id.clone(), // Added project_id field
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
pub async fn delete_custom_field_definition_command(
    _app_handle: AppHandle, // Or app_handle if you plan to use it
    project_id: String,
    field_key: String,
) -> Result<(), String> {
    debug!(
        "[CMD] delete_custom_field_definition_command for project_id: {}, field_key: {}",
        project_id, field_key
    );

    match crate::projectview::db_handler::delete_custom_field_definition(&project_id, &field_key) {
        Ok(_) => {
            info!(
                "[CMD] Custom field definition deleted successfully for project_id: {}, field_key: {}",
                project_id, field_key
            );
            Ok(())
        }
        Err(e) => {
            error!(
                "[CMD] Error deleting custom field definition for project_id: {}, field_key: {}: {}",
                project_id, field_key, e
            );
            Err(format!(
                "Failed to delete custom field definition '{}' for project '{}': {}",
                field_key, project_id, e
            ))
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
