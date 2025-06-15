// src-tauri/src/projectview/pdf_annotation_handler.rs
use crate::welcome::config::CommandError;
use crate::projectview::db_handler::{load_annotations_from_db, save_annotations_to_db};
use log::{info, warn, error}; // debug removed
// PathBuf removed

#[tauri::command]
pub async fn load_pdf_annotations(
    project_id: String,
    original_pdf_relative_path_str: String, // Assuming this is the relative path used as DB key
) -> Result<Option<String>, CommandError> {
    info!("[PDF Annots DB] Loading for project_id {}: PDF key '{}'", project_id, original_pdf_relative_path_str);

    match load_annotations_from_db(&project_id, &original_pdf_relative_path_str, "pdf") {
        Ok(Some(content)) => Ok(Some(content)),
        Ok(None) => Ok(None),
        Err(e) => {
            error!("[PDF Annots DB] Error loading annotations for project_id {} - {}: {}", project_id, original_pdf_relative_path_str, e);
            Err(CommandError::from(format!("Failed to load PDF annotations from DB: {}", e)))
        },
    }
}

#[tauri::command]
pub async fn save_pdf_annotations(
    project_id: String,
    original_pdf_relative_path_str: String, // Relative to project base dir
    annotations_json_content: String,
) -> Result<(), CommandError> {
    info!(
        "[PDF Annots DB] Saving for project_id {}: PDF key '{}'",
        project_id, original_pdf_relative_path_str
    );

    // Basic JSON validation before saving to DB
    if serde_json::from_str::<serde_json::Value>(&annotations_json_content).is_err() {
        warn!("[PDF Annots DB] Annotation JSON content for project_id {} - {} appears invalid. Saving anyway.", project_id, original_pdf_relative_path_str);
    }

    match save_annotations_to_db(&project_id, &original_pdf_relative_path_str, &annotations_json_content, "pdf") {
        Ok(_) => {
            info!("[PDF Annots DB] Annotations saved successfully for project_id {} - {}.", project_id, original_pdf_relative_path_str);
            Ok(())
        },
        Err(e) => {
            error!("[PDF Annots DB] Error saving annotations for project_id {} - {}: {}", project_id, original_pdf_relative_path_str, e);
            Err(CommandError::from(format!("Failed to save PDF annotations to DB: {}", e)))
        }
    }
}