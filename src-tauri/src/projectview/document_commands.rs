// src-tauri/src/projectview/document_commands.rs
use super::db_handler::{self, FileMetadataWithCustomFieldsFromDb};
use super::shared_types::*;
use super::shared_utils::*;
use crate::utils::canonicalize_path;
use crate::welcome::config::CommandError;
use log::{debug, error, info, warn};
use serde_json;
use std::{
    fs,
    path::{Path, PathBuf},
};

// --- save_note_json Command ---
#[tauri::command]
pub async fn save_note_json(
    target_path: String,
    json_content: String,
    highlights_json: Option<String>,
) -> Result<(), String> {
    info!("Saving JSON content to: {}", target_path);
    let path = PathBuf::from(&target_path);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed create parent dir: {}", e))?;
    } else {
        return Err(format!(
            "Invalid target path (no parent directory): {}",
            path.display()
        ));
    }

    fs::write(&path, json_content).map_err(|e| format!("Failed write JSON file: {}", e))?;
    info!("Successfully saved JSON file: {}", path.display());

    if let Some(highlights) = highlights_json {
        info!("Highlights provided, attempting to save...");
        let project_xml_path = get_project_xml_path_from_item(&path)
            .map_err(|e| format!("Failed to get project xml path: {}", e))?;
        let project_data: ProjectXml = {
            let xml_content = fs::read_to_string(&project_xml_path)
                .map_err(|e| format!("Failed to read project xml: {}", e))?;
            serde_json::from_str(&xml_content)
                .map_err(|e| format!("Failed to parse project xml: {}", e))?
        };
        let project_id = project_data.project_uuid;

        db_handler::save_lexical_highlights_to_db(&project_id, target_path.as_str(), &highlights)
            .map_err(|e| format!("Failed to save highlights: {}", e))?;
        info!("Successfully saved highlights for: {}", target_path);
    }

    Ok(())
}

// --- load_note_json Command ---
#[tauri::command]
pub async fn load_note_json(file_path: String) -> Result<String, String> {
    info!("Loading JSON content from: {}", file_path);
    let path = PathBuf::from(&file_path);

    if !path.exists() {
        return Err(format!("JSON file not found: {}", file_path));
    }
    if !path.is_file() {
        return Err(format!("Path is not a file: {}", file_path));
    }

    fs::read_to_string(&path).map_err(|e| format!("Failed read JSON file: {}", e))
}

// --- save_document_and_update_xml Command ---
#[tauri::command]
pub async fn save_document_and_update_xml(
    project_xml_path: String,
    target_path: String,
    document_name: String,
    json_content: String,
) -> Result<(), CommandError> {
    info!("[Backend Save Doc] Target Path: {}", target_path);
    info!("[Backend Save Doc] Project XML: {}", project_xml_path);
    info!("[Backend Save Doc] Document Name: {}", document_name);

    // Canonicalize paths to strip Windows prefixes for consistent comparison
    let target_path_buf =
        canonicalize_path(&target_path).unwrap_or_else(|_| PathBuf::from(&target_path));
    let project_xml_path_buf =
        canonicalize_path(&project_xml_path).unwrap_or_else(|_| PathBuf::from(&project_xml_path));

    if !project_xml_path_buf.exists() || !project_xml_path_buf.is_file() {
        return Err(CommandError::from(format!(
            "Project XML not found: {}",
            project_xml_path
        )));
    }
    let project_base_dir = project_xml_path_buf
        .parent()
        .ok_or_else(|| CommandError::from("Could not get project base dir from XML path"))?;

    if let Some(parent) = target_path_buf.parent() {
        let docs_root = project_base_dir.join(HARVEY_FILES_DIR).join(DOCS_DIR);
        if !parent.starts_with(&docs_root) {
            return Err(CommandError::from(format!(
                "Document save path must be inside '{}' or its subfolders, got: {}",
                docs_root.display(),
                target_path_buf.display()
            )));
        }
        // Ensure the document’s folder exists (allows nested per-document folders)
        fs::create_dir_all(parent).map_err(|e| {
            CommandError::from(format!("Failed to create parent dir for doc: {}", e))
        })?;
    } else {
        return Err(CommandError::from(format!(
            "Invalid target document path (no parent directory): {}",
            target_path
        )));
    }

    match serde_json::from_str::<serde_json::Value>(&json_content) {
        Ok(_) => debug!("[Backend Save Doc] JSON content appears valid."),
        Err(e) => {
            warn!(
                "[Backend Save Doc] JSON content validation failed: {}. Saving anyway.",
                e
            );
        }
    }

    fs::write(&target_path_buf, json_content)
        .map_err(|e| CommandError::from(format!("Failed write document file: {}", e)))?;
    info!(
        "[Backend Save Doc] Saved document content to: {}",
        target_path_buf.display()
    );

    let xml_content = fs::read_to_string(&project_xml_path_buf)?;
    let mut project_data: ProjectXml = serde_json::from_str(&xml_content)?;

    let relative_path_for_doc_xml = target_path_buf
        .strip_prefix(project_base_dir)?
        .to_string_lossy()
        .replace("\\", "/");

    let new_doc_entry = DocumentEntryXml {
        name: document_name.clone(),
        relative_path: relative_path_for_doc_xml.clone(),
        language_code: None,
    };

    let mut is_new_document_entry = false;
    if let Some(index) = project_data
        .document_files
        .files
        .iter()
        .position(|doc| doc.relative_path == relative_path_for_doc_xml)
    {
        warn!("[Backend Save Doc] Document entry with relative path '{}' already exists in XML. Updating name if different.", relative_path_for_doc_xml);
        if project_data.document_files.files[index].name != document_name {
            info!("[Backend Save Doc] Updating existing XML entry name from '{}' to '{}' for RelPath '{}'", project_data.document_files.files[index].name, document_name, relative_path_for_doc_xml);
            project_data.document_files.files[index].name = document_name.clone();
            project_data
                .document_files
                .files
                .sort_by(|a, b| a.name.cmp(&b.name));
        } else {
            info!("[Backend Save Doc] Existing XML entry found, name is unchanged.");
        }
    } else {
        project_data.document_files.files.push(new_doc_entry);
        project_data
            .document_files
            .files
            .sort_by(|a, b| a.name.cmp(&b.name));
        info!(
            "[Backend Save Doc] Added document entry to XML: Name='{}', RelPath='{}'",
            document_name, relative_path_for_doc_xml
        );
        is_new_document_entry = true;
    }

    let _metadata_path = get_document_metadata_path(&target_path_buf)?; // Changed here
    let metadata_exists_in_xml = project_data
        .document_metadata_files
        .files
        .iter()
        .any(|m| m.original_document_relative_path == relative_path_for_doc_xml);

    if is_new_document_entry || !metadata_exists_in_xml {
        // --- Block for creating .metadata.json file REMOVED ---
        // The following code block was removed:
        // if !_metadata_path.exists() {
        //     info!("[Backend Save Doc] Creating empty metadata file for new/untracked document at: {}", _metadata_path.display());
        //     // Create default DocumentHighlightData with populated file_name and last_modified
        //     let mut default_metadata_content = DocumentHighlightData::default(); // metadata is now FileMetadata::default()
        //     default_metadata_content.metadata.file_name = document_name.clone(); // Use the name of the document being saved
        //     default_metadata_content.metadata.file_path = target_path_buf.to_string_lossy().into_owned(); // Set the file_path
        //     default_metadata_content.metadata.last_modified = Utc::now().to_rfc3339();
        //
        //     let metadata_json_content = serde_json::to_string_pretty(&default_metadata_content)
        //         .map_err(|e| CommandError::from(format!("Failed to serialize default metadata to JSON: {}", e)))?;
        //     fs::write(&_metadata_path, metadata_json_content)
        //         .map_err(|e| CommandError::from(format!("Failed to write initial empty metadata file: {}", e)))?;
        // }

        // --- Block for adding DocumentMetadataEntryXml to project.xml REMOVED ---
        // The following code block was removed:
        // if !metadata_exists_in_xml {
        //     let metadata_filename = _metadata_path.file_name().unwrap_or_default().to_string_lossy().to_string();
        //     let metadata_relative_path = _metadata_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\", "/");
        //     let new_metadata_xml_entry = DocumentMetadataEntryXml {
        //         name: metadata_filename,
        //         original_document_relative_path: relative_path_for_doc_xml.clone(),
        //         relative_path: metadata_relative_path,
        //     };
        //     project_data.document_metadata_files.files.push(new_metadata_xml_entry);
        //     project_data.document_metadata_files.files.sort_by(|a,b| a.name.cmp(&b.name));
        //     info!("[Backend Save Doc] Added metadata entry to XML for document: {}", relative_path_for_doc_xml);
        // }
        debug!("[Backend Save Doc] .metadata.json file creation and XML entry addition are now disabled for document: {}", relative_path_for_doc_xml);
    }

    save_project_xml(&project_xml_path_buf, &project_data)?;
    info!("[Backend Save Doc] Project XML updated successfully.");

    // Sync metadata to SQLite to ensure Foreign Keys like `file_groups` don't fail
    let document_file_metadata = crate::projectview::shared_types::FileMetadata {
        file_name: document_name.clone(),
        file_path: target_path_buf.to_string_lossy().into_owned(),
        last_modified: chrono::Utc::now().to_rfc3339(),
        title: String::new(),
        description: String::new(),
        summary: String::new(),
        duration_seconds: None,
        width: None,
        height: None,
        frame_rate: None,
        bit_rate: None,
        audio_codec: None,
        video_codec: None,
        created_at: Some(chrono::Utc::now().to_rfc3339()),
        original_import_path: None,
        speaker_names: None,
        waveform_data: None,
        language_code: None,
        properties: None,
        file_type: "document".to_string(),
        thumbnail: None,
    };

    if let Err(e) = crate::projectview::db_handler::save_asset_metadata(
        &project_data.project_uuid,
        &document_file_metadata,
        &relative_path_for_doc_xml,
        "doc",
        None,
    ) {
        log::warn!(
            "[Backend Save Doc] Failed to save document metadata to DB: {}",
            e
        );
    } else {
        log::info!(
            "[Backend Save Doc] Successfully saved document metadata to DB for: {}",
            relative_path_for_doc_xml
        );
    }

    Ok(())
}

#[derive(serde::Serialize)]
pub struct DocumentMetadata {
    metadata: Option<FileMetadataWithCustomFieldsFromDb>,
    highlights: Option<String>,
}

#[tauri::command]
pub async fn load_document_metadata(
    project_xml_path_str: String,
    original_document_relative_path_str: String,
) -> Result<Option<DocumentMetadata>, CommandError> {
    let project_xml_path = PathBuf::from(&project_xml_path_str);
    let project_data: ProjectXml = {
        let xml_content = fs::read_to_string(&project_xml_path)?;
        serde_json::from_str(&xml_content)?
    };
    let project_id = project_data.project_uuid;
    let project_base_dir = project_xml_path.parent().unwrap();
    let original_document_abs_path = project_base_dir.join(&original_document_relative_path_str);

    let metadata =
        db_handler::load_asset_metadata(&project_id, &original_document_relative_path_str)?;

    let highlights = db_handler::load_lexical_highlights_from_db(
        &project_id,
        original_document_abs_path.to_str().unwrap(),
    )?;

    if metadata.is_none() && highlights.is_none() {
        return Ok(None);
    }

    Ok(Some(DocumentMetadata {
        metadata,
        highlights,
    }))
}

#[tauri::command]
pub async fn read_file_content(path: String) -> Result<String, CommandError> {
    info!("[read_file_content] Reading content from: {}", path);
    let file_path = PathBuf::from(&path);
    if !file_path.exists() {
        return Err(CommandError::from(format!("File not found: {}", path)));
    }
    if !file_path.is_file() {
        return Err(CommandError::from(format!("Path is not a file: {}", path)));
    }
    fs::read_to_string(&file_path)
        .map_err(|e| CommandError::from(format!("Failed to read file content: {}", e)))
}

#[tauri::command]
pub async fn delete_temporary_file(path: String) -> Result<(), CommandError> {
    info!("[delete_temporary_file] Attempting to delete: {}", path);
    let file_path = PathBuf::from(&path);

    if !file_path.exists() {
        warn!(
            "[delete_temporary_file] File not found, assuming already deleted: {}",
            path
        );
        return Ok(());
    }
    if !file_path.is_file() {
        return Err(CommandError::from(format!(
            "Path is not a file, cannot delete: {}",
            path
        )));
    }

    let is_in_tmp_dir = file_path.parent().map_or(false, |p| {
        p.file_name().map_or(false, |n| n == TEMP_SUBDIR_DOCS)
    });
    let is_in_harvey_files = file_path
        .components()
        .any(|comp| comp.as_os_str() == HARVEY_FILES_DIR);

    if !is_in_tmp_dir || !is_in_harvey_files {
        error!("[delete_temporary_file] Refusing to delete file outside a designated '.tmp' directory within '{}': {}", HARVEY_FILES_DIR, path);
        return Err(CommandError::from(format!(
            "Attempted to delete file outside a '{}/{}' directory.",
            HARVEY_FILES_DIR, TEMP_SUBDIR_DOCS
        )));
    }

    fs::remove_file(&file_path).map_err(|e| {
        CommandError::from(format!("Failed to delete temporary file {}: {}", path, e))
    })?;
    info!("[delete_temporary_file] Successfully deleted: {}", path);
    // Remove the parent '.tmp' folder if it is now empty
    if let Some(parent_dir) = file_path.parent() {
        if parent_dir
            .file_name()
            .map_or(false, |n| n == TEMP_SUBDIR_DOCS)
        {
            if let Ok(mut entries) = fs::read_dir(&parent_dir) {
                if entries.next().is_none() {
                    // Directory is empty, remove it
                    if let Err(e) = fs::remove_dir(&parent_dir) {
                        warn!(
                            "[delete_temporary_file] Failed to remove tmp directory {}: {}",
                            parent_dir.display(),
                            e
                        );
                    } else {
                        info!(
                            "[delete_temporary_file] Removed empty tmp directory: {}",
                            parent_dir.display()
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn get_unique_document_path(
    target_dir_str: String,
    base_name: String,
    extension: String,
) -> Result<String, CommandError> {
    let target_dir = PathBuf::from(target_dir_str);

    if !target_dir.exists() {
        warn!(
            "Target documents directory {} not found. Attempting to create.",
            target_dir.display()
        );
        fs::create_dir_all(&target_dir)?;
        info!("Created documents directory: {}", target_dir.display());
    }

    let mut counter = 0;
    loop {
        let file_name = if counter == 0 {
            format!("{}.{}", base_name, extension)
        } else {
            format!("{}_{}.{}", base_name, counter, extension)
        };
        let target_path = target_dir.join(&file_name);

        if !target_path.exists() {
            debug!("Found unique path: {}", target_path.display());
            return Ok(target_path.to_string_lossy().to_string());
        }

        counter += 1;
        if counter > 1000 {
            return Err(CommandError::from(format!(
                "Could not find unique filename for base '{}' after {} attempts.",
                base_name, counter
            )));
        }
    }
}

pub fn get_document_metadata_path(original_doc_path: &Path) -> Result<PathBuf, CommandError> {
    let doc_parent_dir = original_doc_path.parent().ok_or_else(|| {
        CommandError::from(format!(
            "Could not get parent directory for document: {}",
            original_doc_path.display()
        ))
    })?;

    let doc_stem = original_doc_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            CommandError::from(format!(
                "Could not get file stem for document: {}",
                original_doc_path.display()
            ))
        })?;

    let metadata_filename = format!(".{}.{}", doc_stem, METADATA_FILE_SUFFIX);
    Ok(doc_parent_dir.join(metadata_filename))
}

#[tauri::command]
pub async fn create_new_document(
    project_xml_path: String,
    document_name: String,
) -> Result<String, CommandError> {
    info!(
        "[create_new_document] Creating new document named: {}",
        document_name
    );
    let project_xml_path_buf = PathBuf::from(&project_xml_path);
    let project_base_dir = project_xml_path_buf
        .parent()
        .ok_or_else(|| CommandError::from("Could not get project base dir from XML path"))?;

    // Derive a safe folder name from the document_name
    let document_stem = Path::new(&document_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();

    let docs_root = project_base_dir.join(HARVEY_FILES_DIR).join(DOCS_DIR);

    // Find a unique folder name first
    let mut counter = 0;
    let mut unique_stem = document_stem.clone();
    let mut doc_folder = docs_root.join(&unique_stem);

    while doc_folder.exists() {
        counter += 1;
        unique_stem = format!("{}_{}", document_stem, counter);
        doc_folder = docs_root.join(&unique_stem);
    }

    // Create the unique directory
    fs::create_dir_all(&doc_folder)
        .map_err(|e| CommandError::from(format!("Failed to create doc folder: {}", e)))?;

    // Construct the file path inside the unique folder, matching the folder stem
    let file_name = format!("{}.json", unique_stem);
    let unique_path_buf = doc_folder.join(&file_name);
    let unique_path_str = unique_path_buf.to_string_lossy().to_string();

    let initial_content = "{\"root\":{\"children\":[{\"children\":[],\"direction\":null,\"format\":\"\",\"indent\":0,\"type\":\"paragraph\",\"version\":1}],\"direction\":null,\"format\":\"\",\"indent\":0,\"type\":\"root\",\"version\":1}}";

    save_document_and_update_xml(
        project_xml_path.clone(),
        unique_path_str.clone(),
        file_name,
        initial_content.to_string(),
    )
    .await?;

    info!(
        "[create_new_document] Successfully created new document at: {}",
        unique_path_str
    );
    Ok(unique_path_str)
}
