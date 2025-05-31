// src-tauri/src/projectview/pdf_annotation_handler.rs
use crate::welcome::config::CommandError;
use crate::projectview::db_handler::{load_annotations_from_db, save_annotations_to_db};
use log::{info, warn, error}; // debug removed
// PathBuf removed

#[tauri::command]
pub async fn load_pdf_annotations(
    original_pdf_abs_path_str: String,
) -> Result<Option<String>, CommandError> {
    info!("[PDF Annots DB] Loading for PDF: {}", original_pdf_abs_path_str);
    // The `original_pdf_abs_path_str` is expected to be the key for the database.
    // It should be the relative path from the project base directory.
    // For now, we assume the path passed from frontend is already relative or can be made relative.
    // This might need adjustment based on how `projectService.js` constructs this path.
    // Let's assume for now it's the relative path.
    match load_annotations_from_db(&original_pdf_abs_path_str) {
        Ok(Some(content)) => Ok(Some(content)),
        Ok(None) => Ok(None),
        Err(e) => {
            error!("[PDF Annots DB] Error loading annotations for {}: {}", original_pdf_abs_path_str, e);
            Err(CommandError::from(format!("Failed to load PDF annotations from DB: {}", e)))
        },
    }
}

#[tauri::command]
pub async fn save_pdf_annotations(
    original_pdf_relative_path_str: String, // Relative to project base dir
    annotations_json_content: String,
) -> Result<(), CommandError> {
    info!(
        "[PDF Annots DB] Saving for PDF (rel path expected): '{}'",
        original_pdf_relative_path_str
    );

    // The project_xml_path_str is no longer strictly needed here for XML updates regarding annotations.
    // However, original_pdf_relative_path_str is crucial as the key.

    // Basic JSON validation before saving to DB
    if serde_json::from_str::<serde_json::Value>(&annotations_json_content).is_err() {
        warn!("[PDF Annots DB] Annotation JSON content for {} appears invalid. Saving anyway.", original_pdf_relative_path_str);
    }

    match save_annotations_to_db(&original_pdf_relative_path_str, &annotations_json_content) {
        Ok(_) => {
            info!("[PDF Annots DB] Annotations saved successfully for {}.", original_pdf_relative_path_str);
            Ok(())
        },
        Err(e) => {
            error!("[PDF Annots DB] Error saving annotations for {}: {}", original_pdf_relative_path_str, e);
            Err(CommandError::from(format!("Failed to save PDF annotations to DB: {}", e)))
        }
    }
}