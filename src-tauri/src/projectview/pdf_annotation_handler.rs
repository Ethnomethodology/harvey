use crate::welcome::config::CommandError;
use crate::projectview::shared_types::{ProjectXml, PdfAnnotationEntryXml, PDF_ANNOTATIONS_FILE_SUFFIX};
use crate::projectview::shared_utils::save_project_xml;
use std::{
    fs,
    path::{Path, PathBuf},
};
use log::{info, warn, error, debug};

/// Helper function to construct the path for a PDF's annotation file.
/// Example: For "/path/to/MyDoc.pdf", returns "/path/to/.MyDoc.annotations.json"
pub fn get_pdf_annotation_file_path(original_pdf_path: &Path) -> Result<PathBuf, CommandError> {
    let pdf_parent_dir = original_pdf_path.parent().ok_or_else(|| {
        CommandError::from(format!(
            "Could not get parent directory for PDF: {}",
            original_pdf_path.display()
        ))
    })?;

    let pdf_stem = original_pdf_path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        CommandError::from(format!(
            "Could not get file stem for PDF: {}",
            original_pdf_path.display()
        ))
    })?;

    let annotation_filename = format!(".{}.{}", pdf_stem, PDF_ANNOTATIONS_FILE_SUFFIX);
    Ok(pdf_parent_dir.join(annotation_filename))
}

#[tauri::command]
pub async fn load_pdf_annotations(
    original_pdf_abs_path_str: String,
) -> Result<Option<String>, CommandError> {
    info!("[PDF Annots] Loading for PDF: {}", original_pdf_abs_path_str);
    let original_pdf_abs_path = PathBuf::from(original_pdf_abs_path_str);

    let annotation_file_path = get_pdf_annotation_file_path(&original_pdf_abs_path)?;

    if annotation_file_path.exists() && annotation_file_path.is_file() {
        debug!("[PDF Annots] Annotation file found: {}", annotation_file_path.display());
        let content = fs::read_to_string(&annotation_file_path)
            .map_err(|e| CommandError::from(format!("Failed to read PDF annotation file: {}", e)))?;
        // Basic JSON validation (pdf-annotate.js uses a specific structure, but we just pass string)
        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(_) => Ok(Some(content)),
            Err(e) => {
                warn!("[PDF Annots] Annotation file {} content is not valid JSON: {}. Returning None.", annotation_file_path.display(), e);
                Ok(None)
            }
        }
    } else {
        debug!("[PDF Annots] Annotation file not found: {}. Returning None.", annotation_file_path.display());
        Ok(None)
    }
}

#[tauri::command]
pub async fn save_pdf_annotations(
    project_xml_path_str: String,
    original_pdf_relative_path_str: String, // Relative to project base dir
    annotations_json_content: String,
) -> Result<(), CommandError> {
    info!(
        "[PDF Annots] Saving for PDF (rel): '{}', XML: '{}'",
        original_pdf_relative_path_str, project_xml_path_str
    );

    let project_xml_path = PathBuf::from(&project_xml_path_str);
    let project_base_dir = project_xml_path.parent().ok_or_else(|| {
        CommandError::from("Could not get project base directory from XML path")
    })?;

    let original_pdf_abs_path = project_base_dir.join(&original_pdf_relative_path_str);
    if !original_pdf_abs_path.exists() || !original_pdf_abs_path.is_file() {
        return Err(CommandError::from(format!(
            "Original PDF document not found at: {}",
            original_pdf_abs_path.display()
        )));
    }

    let annotation_file_path = get_pdf_annotation_file_path(&original_pdf_abs_path)?;
    let annotation_filename = annotation_file_path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();

    debug!("[PDF Annots] Saving annotations to: {}", annotation_file_path.display());
    
    // Validate JSON structure before writing (optional, but good practice)
    if serde_json::from_str::<serde_json::Value>(&annotations_json_content).is_err() {
        warn!("[PDF Annots] Annotation JSON content appears invalid. Saving anyway.");
    }

    fs::write(&annotation_file_path, annotations_json_content)
        .map_err(|e| CommandError::from(format!("Failed to write PDF annotation file: {}", e)))?;

    // Update XML
    let xml_content = fs::read_to_string(&project_xml_path)?;
    let mut project_data: ProjectXml = quick_xml::de::from_str(&xml_content)?;

    let annotation_relative_path = annotation_file_path
        .strip_prefix(project_base_dir)?
        .to_string_lossy()
        .replace("\\", "/");

    let original_pdf_rel_path_cleaned = original_pdf_relative_path_str.replace("\\", "/");

    if let Some(existing_entry) = project_data
        .pdf_annotation_files
        .files
        .iter_mut()
        .find(|entry| entry.original_document_relative_path == original_pdf_rel_path_cleaned)
    {
        debug!("[PDF Annots] Updating existing PDF annotation entry in XML.");
        existing_entry.name = annotation_filename; // Should match the actual hidden filename
        existing_entry.relative_path = annotation_relative_path;
    } else {
        info!("[PDF Annots] Adding new PDF annotation entry to XML for original PDF: {}", original_pdf_rel_path_cleaned);
        let new_annotation_xml_entry = PdfAnnotationEntryXml {
            name: annotation_filename.clone(),
            original_document_relative_path: original_pdf_rel_path_cleaned,
            relative_path: annotation_relative_path.clone(),
        };
        project_data.pdf_annotation_files.files.push(new_annotation_xml_entry);
    }
    project_data.pdf_annotation_files.files.sort_by(|a, b| a.name.cmp(&b.name));
    save_project_xml(&project_xml_path, &project_data)?;
    info!("[PDF Annots] PDF Annotations saved and XML updated successfully.");
    Ok(())
}