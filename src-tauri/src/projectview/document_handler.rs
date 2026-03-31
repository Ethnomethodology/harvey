// src-tauri/src/projectview/document_handler.rs
use crate::welcome::config::CommandError;
use crate::projectview::shared_types::{
    ProjectXml, DocumentEntryXml,
    HARVEY_FILES_DIR, DOCS_DIR, TEMP_SUBDIR_DOCS,
    FileMetadata
};
use crate::projectview::shared_utils::{save_project_xml, ensure_base_asset_dirs, truncate_filename_stem, MAX_FILENAME_STEM_LENGTH};
use crate::projectview::db_handler;

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH}
};
use log::{info, warn, error, debug};
use tauri::{AppHandle, Runtime};
use tauri::Manager;
use tauri_plugin_shell::process::CommandEvent;
use crate::welcome::python_env::get_python_command;
use uuid::Uuid;
use quick_xml;
// use serde::{Serialize, Deserialize};
use chrono::Utc;
// Local FileMetadata and StandardAssetMetadata structs removed, shared_types versions will be used.

fn get_unique_temp_path_for_conversion(base_dir: &Path, prefix: &str, extension: &str) -> Result<PathBuf, CommandError> {
    let temp_dir = base_dir.join(TEMP_SUBDIR_DOCS);
    fs::create_dir_all(&temp_dir)?;

    let unique_id = Uuid::new_v4().to_string();
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let sanitized_prefix = prefix.replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
    let file_name = format!("{}_{}_{}.{}", sanitized_prefix, timestamp, unique_id, extension);
    debug!("Generated unique temp path: {}", temp_dir.join(&file_name).display());
    Ok(temp_dir.join(file_name))
}


#[tauri::command]
pub async fn import_document<R: Runtime>(
    app_handle: AppHandle<R>,
    source_path_str: String,
    project_xml_path_str: String,
) -> Result<String, CommandError> {
    info!("[import_document] Starting import for: {}", source_path_str);
    let source_path = PathBuf::from(&source_path_str);
    let project_xml_path = PathBuf::from(&project_xml_path_str);

    if !source_path.exists() || !source_path.is_file() {
        return Err(CommandError::from(format!("Source file not found: {}", source_path_str)));
    }

    let project_base_dir = project_xml_path.parent()
        .ok_or_else(|| CommandError::from("Could not get project base directory"))?;

    ensure_base_asset_dirs(project_base_dir)?;

    // Read project_uuid from XML once at the beginning
    let project_xml_content_for_uuid = fs::read_to_string(&project_xml_path)
        .map_err(|e| CommandError::Io(format!("Failed to read project XML for UUID from {}: {}", project_xml_path.display(), e)))?;
    let project_data_for_uuid: ProjectXml = serde_json::from_str(&project_xml_content_for_uuid)
        .map_err(|e| CommandError::XmlDeserialization(format!("Failed to parse project XML for UUID from {}: {}", project_xml_path.display(), e)))?;

    let project_id_for_db = project_data_for_uuid.project_uuid;
    if project_id_for_db.is_empty() {
        error!("[import_document] Project UUID is empty in XML file: {}. Cannot proceed with import without project_id.", project_xml_path.display());
        return Err(CommandError::Message(format!("Project ID (UUID) is missing in the project file ({}). Asset import cannot proceed.", project_xml_path.display())));
    }

    let original_source_filename_with_ext = source_path.file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| CommandError::from("Could not get original filename with extension"))?
        .to_string();

    // Truncate the original filename's stem
    let new_document_filename_with_ext = truncate_filename_stem(&original_source_filename_with_ext, MAX_FILENAME_STEM_LENGTH);
    info!("[import_document] Original filename: '{}', Truncated filename for project: '{}'", original_source_filename_with_ext, new_document_filename_with_ext);

    let new_document_filename_stem_truncated = Path::new(&new_document_filename_with_ext).file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| CommandError::from(format!("Could not get stem from truncated filename: {}", new_document_filename_with_ext)))?;
    
    // Read project_data to check for name conflicts in XML
    let xml_content = fs::read_to_string(&project_xml_path)?;
    let project_data: ProjectXml = serde_json::from_str(&xml_content)?;

    // Create per-document folder under Documents, using the truncated stem
    let docs_base = project_base_dir.join(HARVEY_FILES_DIR).join(DOCS_DIR);

    let mut folder_counter = 0;
    let (doc_folder, new_document_filename_with_ext, new_document_filename_stem) = loop {
        let current_stem = if folder_counter == 0 {
            new_document_filename_stem_truncated.to_string()
        } else {
            format!("{}_{}", new_document_filename_stem_truncated, folder_counter)
        };

        let candidate_folder = docs_base.join(&current_stem);

        // Check if this stem is already used in project_data.document_files.files
        let name_conflict = project_data.document_files.files.iter().any(|f| {
            f.name == current_stem || f.name.starts_with(&format!("{}.", current_stem))
        });

        if !candidate_folder.exists() && !name_conflict {
            let file_name = format!("{}.{}", current_stem, source_path.extension().and_then(|e| e.to_str()).unwrap_or(""));
            debug!("[import_document] Found unique document folder: {} and filename: {}", candidate_folder.display(), file_name);
            break (candidate_folder, file_name, current_stem);
        }

        folder_counter += 1;
        if folder_counter > 1000 {
            return Err(CommandError::from(format!("Could not find unique folder for document stem '{}' after 1000 attempts.", new_document_filename_stem_truncated)));
        }
    };

    fs::create_dir_all(&doc_folder)?;
    
    let source_extension_lower = source_path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    let final_or_temp_path: String;

    match source_extension_lower.as_str() {
        "pdf" => {
            info!("[import_document] Handling PDF direct import for .pdf file");

            // Copy PDF into its own document folder, using the (potentially truncated) new filename
            let final_pdf_path = doc_folder.join(&new_document_filename_with_ext);
            // final_pdf_name is the new, potentially truncated, filename with extension
            let final_pdf_name = new_document_filename_with_ext.clone();
            let final_pdf_path_str = final_pdf_path.to_string_lossy().to_string();

            info!("[import_document] Copying PDF from '{}' to '{}'", source_path.display(), final_pdf_path.display());
            fs::copy(&source_path, &final_pdf_path).map_err(|e| CommandError::from(format!("Failed to copy PDF: {}", e)))?;

            info!("[import_document] Updating project XML to include PDF: {}", final_pdf_name);
            let xml_content = fs::read_to_string(&project_xml_path)?;
            let mut project_data: ProjectXml = serde_json::from_str(&xml_content)?;

            let relative_path_for_pdf_xml = final_pdf_path // Path uses truncated name
                .strip_prefix(project_base_dir)?
                .to_string_lossy()
                .replace("\\", "/");

            let new_doc_entry = DocumentEntryXml {
                name: final_pdf_name.clone(), // XML name is the truncated filename
                relative_path: relative_path_for_pdf_xml.clone(),
                language_code: None,
            };

            let mut main_doc_xml_changed = false;
            if !project_data.document_files.files.iter().any(|doc| doc.relative_path == relative_path_for_pdf_xml) {
                project_data.document_files.files.push(new_doc_entry);
                project_data.document_files.files.sort_by(|a, b| a.name.cmp(&b.name));
                main_doc_xml_changed = true;
            }

            // --- .metadata.json handling removed ---

            // --- PDF Annotations are now handled by the database, no file creation or XML entry needed here ---
            info!("[import_document] PDF annotation file/XML entry is no longer created for PDF: {}", relative_path_for_pdf_xml);

            if main_doc_xml_changed {
                save_project_xml(&project_xml_path, &project_data)?;
                info!("[import_document] Project XML updated successfully for PDF document entry.");
            } else {
                info!("[import_document] PDF document entry already existed in XML or no changes needed. No XML changes made for document entry.");
            }

            // Construct FileMetadata for SQLite
            let pdf_file_metadata = FileMetadata {
                file_name: final_pdf_name.clone(), // Use truncated filename
                file_path: final_pdf_path.to_string_lossy().into_owned(), // Path uses truncated filename
                last_modified: Utc::now().to_rfc3339(),
                title: String::new(), // Initialize empty, user can fill later
                description: String::new(), // Initialize empty
                summary: String::new(),
                duration_seconds: None,
                width: None,
                height: None,
                frame_rate: None,
                bit_rate: None,
                audio_codec: None,
                video_codec: None,
                created_at: Some(Utc::now().to_rfc3339()),
                original_import_path: Some(source_path_str.clone()),
                speaker_names: None,
                waveform_data: None,
                language_code: None,
                properties: None,
                file_type: "document".to_string(),
                thumbnail: None,
            };

            // Save metadata to SQLite database
            info!("[import_document] PDF FileMetadata before save: created_at={:?}", pdf_file_metadata.created_at);
            match db_handler::save_asset_metadata(
                &project_id_for_db,           // Pass project_id
                &pdf_file_metadata,
                &relative_path_for_pdf_xml, // Path for DB key uses truncated name
                "pdf", // asset_type
                None, // custom_fields_json
            ) {
                Ok(_) => info!("[import_document] Successfully saved PDF metadata to DB for: {} with project_id {}", relative_path_for_pdf_xml, project_id_for_db),
                Err(e) => {
                    warn!("[import_document] Failed to save PDF metadata to DB for {} (project_id {}): {}. The PDF was imported, but its metadata might be missing from the database.", relative_path_for_pdf_xml, project_id_for_db, e);
                }
            }

            final_or_temp_path = final_pdf_path_str; // This is the absolute path to the (truncated) copied file
            info!("[import_document] PDF import successful. Final path: {}", final_or_temp_path);
        }

        "txt" | "md" | "rtf" | "docx" => {
            // For Pandoc conversion, the source_filename_stem (truncated) is used for temp file prefix.
            // The final document entry in XML and DB will still use new_document_filename_with_ext.
            let conversion_type = if source_extension_lower == "docx" { "docx" } else { "text/markdown/rtf" };
            info!("[import_document] Handling Pandoc conversion for .{} file ({}) -> HTML", source_extension_lower, conversion_type);
            // Use new_document_filename_stem (truncated) for the temp file name prefix
            let temp_html_path: PathBuf = get_unique_temp_path_for_conversion(&doc_folder, &new_document_filename_stem, "html")?;

            let source_format_arg = match source_extension_lower.as_str() {
                "txt" => "plain",
                "md" => "markdown",
                "rtf" => "rtf",
                "docx" => "docx",
                _ => unreachable!(), // Should be caught by outer match
            };

            let script_path = app_handle.path()
                .resolve("scripts/convert_with_pandoc.py", tauri::path::BaseDirectory::Resource)
                .map_err(|e| CommandError::from(format!("Failed to resolve pandoc script path: {}", e)))?;

            let pandoc_args = vec![
                source_path.to_string_lossy().to_string(),
                temp_html_path.to_string_lossy().to_string(),
                "html".to_string(),
                "--standalone".to_string(),
                "--katex".to_string(),
                format!("--from={}", source_format_arg),
            ];
            
            info!("[import_document] Running pandoc script: {} {}", script_path.display(), pandoc_args.join(" "));

            let (mut rx, _child) = get_python_command(&app_handle)?
                .args(&[script_path.to_string_lossy().to_string()])
                .args(&pandoc_args)
                .spawn()
                .map_err(|e| CommandError::from(format!("Failed to spawn pandoc script for {}: {}", conversion_type, e)))?;

            let mut tool_stderr = String::new(); let mut exit_code = None;
            while let Some(event) = rx.recv().await {
                 match event {
                    CommandEvent::Stderr(line) => { let line_str = String::from_utf8_lossy(&line); warn!("[Pandoc {} Stderr]: {}", conversion_type, line_str.trim_end()); tool_stderr.push_str(&line_str); tool_stderr.push('\n'); }
                    CommandEvent::Stdout(line) => { info!("[Pandoc {} Stdout]: {}", conversion_type, String::from_utf8_lossy(&line).trim_end()); }
                    CommandEvent::Error(msg) => { error!("[Pandoc {} Error]: {}", conversion_type, msg); tool_stderr.push_str(&format!("Exec Error: {}\n", msg)); exit_code = Some(-1); break; }
                    CommandEvent::Terminated(payload) => { info!("[Pandoc {} Term]: {:?}", conversion_type, payload); exit_code = payload.code; if payload.signal.is_some() && exit_code.is_none() { exit_code = Some(-1); } break; }
                    _ => {}
                }
            }
            

            if exit_code != Some(0) {
                 error!("[import_document] Pandoc {} failed. Code:{:?}\nStderr:{}", conversion_type, exit_code, tool_stderr);
                 let _ = fs::remove_file(&temp_html_path); // Clean up temp HTML on failure
                 return Err(CommandError::from(format!("Pandoc {} failed (Code:{:?}):\n{}", conversion_type, exit_code.unwrap_or(-1), tool_stderr.chars().take(300).collect::<String>())));
            }
            if !temp_html_path.exists() || fs::metadata(&temp_html_path)?.len() == 0 {
                 warn!("[import_document] Pandoc {} success but output missing/empty: {}", conversion_type, temp_html_path.display());
                 return Err(CommandError::from(format!("Pandoc {} success but output empty/missing.", conversion_type)));
            }

            // Metadata saving logic for non-PDF documents
            // The conceptual path for the original document type (e.g., .docx) uses the new_document_filename_with_ext (truncated)
            let conceptual_original_doc_path = doc_folder.join(&new_document_filename_with_ext);

            let asset_relative_path_for_db = conceptual_original_doc_path // Path uses truncated name
                .strip_prefix(project_base_dir)
                .map_err(|_| CommandError::from(format!("Failed to strip prefix for DB relative path of document: {}", conceptual_original_doc_path.display())))?
                .to_string_lossy()
                .replace("\\", "/")
                .to_string();

            // Get last modified from original source path
            let last_modified_timestamp = match fs::metadata(&source_path) {
                Ok(metadata) => {
                    match metadata.modified() {
                        Ok(time) => {
                            let secs = time.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
                            chrono::DateTime::from_timestamp(secs as i64, 0)
                                .map(|dt| dt.to_rfc3339())
                                .unwrap_or_else(|| Utc::now().to_rfc3339())
                        }
                        Err(_) => Utc::now().to_rfc3339(), // Fallback if modified time fails
                    }
                }
                Err(_) => Utc::now().to_rfc3339(), // Fallback if metadata fails
            };

            let doc_file_metadata = FileMetadata {
                file_name: new_document_filename_with_ext.clone(), // Use truncated filename
                file_path: conceptual_original_doc_path.to_string_lossy().into_owned(), // Path uses truncated filename
                last_modified: last_modified_timestamp,
                title: String::new(), // Initialize empty
                description: String::new(), // Initialize empty
                summary: String::new(),
                duration_seconds: None,
                width: None,
                height: None,
                frame_rate: None,
                bit_rate: None,
                audio_codec: None,
                video_codec: None,
                created_at: Some(Utc::now().to_rfc3339()), // Could also attempt to get from source_path metadata if needed
                original_import_path: Some(source_path_str.clone()),
                speaker_names: None,
                waveform_data: None,
                language_code: None,
                properties: None,
                file_type: "document".to_string(),
                thumbnail: None,
            };

            info!("[import_document] DOC FileMetadata before save: created_at={:?}", doc_file_metadata.created_at);
            match db_handler::save_asset_metadata(
                &project_id_for_db,           // Pass project_id
                &doc_file_metadata,
                &asset_relative_path_for_db, // DB key uses truncated name based path
                &source_extension_lower,    // asset_type is the original extension
                None, // custom_fields_json
            ) {
                Ok(_) => info!("[import_document] Successfully saved document metadata to DB for: {} (type: {}, project_id: {})", asset_relative_path_for_db, source_extension_lower, project_id_for_db),
                Err(e) => {
                    warn!("[import_document] Failed to save document metadata to DB for {} (project_id {}): {}. Proceeding with import.", asset_relative_path_for_db, project_id_for_db, e);
                }
            }

            info!("[import_document] Pandoc {} successful to temp HTML: {}", conversion_type, temp_html_path.display());
            // Return temp HTML path, but also pass the *new* (truncated) document filename for XML creation later by frontend/service
            final_or_temp_path = format!("{}|original_filename:{}", temp_html_path.to_string_lossy(), new_document_filename_with_ext);
        }

        _ => {
            error!("[import_document] Unsupported file type for import: .{}", source_extension_lower);
            return Err(CommandError::from(format!("Unsupported file type for import: .{}", source_extension_lower)));
        }
    }

    info!("[import_document] Import process finished. Returning path (and potentially truncated original filename if applicable): {}", final_or_temp_path);
    Ok(final_or_temp_path)
}