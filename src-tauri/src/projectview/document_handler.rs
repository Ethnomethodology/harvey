// src-tauri/src/projectview/document_handler.rs
use crate::welcome::config::CommandError;
use crate::projectview::shared_types::{
    ProjectXml, DocumentEntryXml, DocumentMetadataEntryXml,
    HARVEY_FILES_DIR, DOCS_DIR, TEMP_SUBDIR_DOCS,
    StandardAssetMetadata, FileMetadata // Added imports
};
use crate::projectview::shared_utils::{save_project_xml, ensure_base_asset_dirs};
use crate::projectview::document_commands::{get_document_metadata_path};

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH}
};
use log::{info, warn, error, debug};
use tauri::{AppHandle};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;
use uuid::Uuid;
use quick_xml;
// use serde::{Serialize, Deserialize}; // Removed as per instruction (if unused)
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
pub async fn import_document(
    app_handle: AppHandle,
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

    let source_filename_stem = source_path.file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| CommandError::from("Could not get filename stem"))?;
    
    // Create per-document folder under Documents
    let docs_base = project_base_dir.join(HARVEY_FILES_DIR).join(DOCS_DIR);
    let doc_folder = docs_base.join(source_filename_stem);
    fs::create_dir_all(&doc_folder)?;
    
    let source_filename_with_ext = source_path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();


    let source_extension = source_path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    let final_or_temp_path: String;

    match source_extension.as_str() {
        "pdf" => {
            info!("[import_document] Handling PDF direct import for .pdf file");

            // Copy PDF into its own document folder
            let final_pdf_path = doc_folder.join(&source_filename_with_ext);
            let final_pdf_name = source_filename_with_ext.clone();
            let final_pdf_path_str = final_pdf_path.to_string_lossy().to_string();

            info!("[import_document] Copying PDF from '{}' to '{}'", source_path.display(), final_pdf_path.display());
            fs::copy(&source_path, &final_pdf_path).map_err(|e| CommandError::from(format!("Failed to copy PDF: {}", e)))?;

            info!("[import_document] Updating project XML to include PDF: {}", final_pdf_name);
            let xml_content = fs::read_to_string(&project_xml_path)?;
            let mut project_data: ProjectXml = quick_xml::de::from_str(&xml_content)?;

            let relative_path_for_pdf_xml = final_pdf_path
                .strip_prefix(project_base_dir)?
                .to_string_lossy()
                .replace("\\", "/");

            let new_doc_entry = DocumentEntryXml {
                name: final_pdf_name.clone(),
                relative_path: relative_path_for_pdf_xml.clone(),
            };

            let mut main_doc_xml_changed = false;
            if !project_data.document_files.files.iter().any(|doc| doc.relative_path == relative_path_for_pdf_xml) {
                project_data.document_files.files.push(new_doc_entry);
                project_data.document_files.files.sort_by(|a, b| a.name.cmp(&b.name));
                main_doc_xml_changed = true;
            }

            // --- Handle .metadata.json (our app's general metadata) ---
            let app_metadata_path = get_document_metadata_path(&final_pdf_path)?;
            if !app_metadata_path.exists() {
                // Create new StandardAssetMetadata instead of DocumentHighlightData
                let new_standard_metadata = StandardAssetMetadata {
                    metadata: FileMetadata {
                        file_name: final_pdf_name.clone(), // final_pdf_name is from the existing PDF logic
                        file_path: final_pdf_path.to_string_lossy().into_owned(), // final_pdf_path is existing
                        last_modified: Utc::now().to_rfc3339(),
                        title: "".to_string(),
                        description: "".to_string(),
                        summary: "".to_string(),
                        duration_seconds: None,
                        width: None,
                        height: None,
                        frame_rate: None,
                        bit_rate: None,
                        audio_codec: None,
                        video_codec: None,
                        creation_time: None,
                    },
                    highlights: Vec::new(), // Standardized, starts empty
                };
                let app_metadata_json_content = serde_json::to_string_pretty(&new_standard_metadata)
                    .map_err(|e| CommandError::from(format!("Failed to serialize standard asset metadata for PDF: {}",e)))?;
                fs::write(&app_metadata_path, app_metadata_json_content)
                    .map_err(|e| CommandError::from(format!("Failed to write standard asset metadata for PDF: {}", e)))?;
                info!("[import_document] Created standard asset metadata file for PDF: {}", app_metadata_path.display());
            }
            let app_metadata_filename_xml = app_metadata_path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let app_metadata_relative_path_xml = app_metadata_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\", "/");
            let mut app_metadata_xml_changed = false;
            if !project_data.document_metadata_files.files.iter().any(|m| m.original_document_relative_path == relative_path_for_pdf_xml) {
                project_data.document_metadata_files.files.push(DocumentMetadataEntryXml {
                    name: app_metadata_filename_xml,
                    original_document_relative_path: relative_path_for_pdf_xml.clone(),
                    relative_path: app_metadata_relative_path_xml,
                });
                project_data.document_metadata_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                info!("[import_document] Added app metadata entry to XML for PDF: {}", relative_path_for_pdf_xml);
                app_metadata_xml_changed = true;
            }

            // --- PDF Annotations are now handled by the database, no file creation or XML entry needed here ---
            info!("[import_document] PDF annotation file/XML entry is no longer created for PDF: {}", relative_path_for_pdf_xml);
            let _pdf_annotation_xml_changed = false; // Ensure this doesn't interfere with save_project_xml logic

            if main_doc_xml_changed || app_metadata_xml_changed { // Removed pdf_annotation_xml_changed
                save_project_xml(&project_xml_path, &project_data)?;
                info!("[import_document] Project XML updated successfully for PDF and/or its metadata/annotations.");
            } else {
                info!("[import_document] PDF and its metadata/annotations entries already existed in XML. No XML changes made.");
            }

            final_or_temp_path = final_pdf_path_str;
            info!("[import_document] PDF import successful. Final path: {}", final_or_temp_path);
        }

        "txt" | "md" | "rtf" | "docx" => {
            let conversion_type = if source_extension == "docx" { "docx" } else { "text/markdown/rtf" };
            info!("[import_document] Handling Pandoc conversion for .{} file ({}) -> HTML", source_extension, conversion_type);
            let temp_html_path: PathBuf = get_unique_temp_path_for_conversion(&doc_folder, source_filename_stem, "html")?;

            let source_format_arg = match source_extension.as_str() {
                "txt" => "plain",
                "md" => "markdown",
                "rtf" => "rtf",
                "docx" => "docx",
                _ => unreachable!(),
            };

            let mut pandoc_args = vec![
                source_path.to_string_lossy().to_string(),
                "-f".to_string(), source_format_arg.to_string(),
                "-t".to_string(), "html".to_string(),
                "--standalone".to_string(),
                "-o".to_string(), temp_html_path.to_string_lossy().to_string(),
            ];

            let mut media_extract_path_option: Option<PathBuf> = None;
            if source_extension == "docx" {
                pandoc_args.push("--wrap=none".to_string());
                let unique_media_extract_folder_name = format!("media_extracted_{}_{}", source_filename_stem, Uuid::new_v4().to_string().split('-').next().unwrap_or("rand"));
                let media_extract_path = temp_html_path.parent().unwrap_or_else(|| Path::new(".")).join(unique_media_extract_folder_name);
                fs::create_dir_all(&media_extract_path).map_err(|e| CommandError::from(format!("Failed to create media extract dir {}: {}", media_extract_path.display(), e)))?;
                pandoc_args.push("--extract-media=".to_string() + &media_extract_path.to_string_lossy());
                media_extract_path_option = Some(media_extract_path);
            }

            info!("[import_document] Running Pandoc sidecar: pandoc {}", pandoc_args.join(" "));

            let (mut rx, _child) = app_handle.shell().sidecar("pandoc")?.args(&pandoc_args).spawn()
                .map_err(|e| CommandError::from(format!("Failed Pandoc spawn for {}: {}", conversion_type, e)))?;

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
            
            if let Some(media_extract_path) = media_extract_path_option {
                 if media_extract_path.exists() && media_extract_path.is_dir() {
                    debug!("[import_document] Cleaning up temporary extracted media folder: {}", media_extract_path.display());
                    if let Err(e) = fs::remove_dir_all(&media_extract_path) {
                        warn!("[import_document] Failed to cleanup media extraction folder {}: {}", media_extract_path.display(), e);
                    }
                }
            }

            if exit_code != Some(0) {
                 error!("[import_document] Pandoc {} failed. Code:{:?}\nStderr:{}", conversion_type, exit_code, tool_stderr);
                 let _ = fs::remove_file(&temp_html_path);
                 return Err(CommandError::from(format!("Pandoc {} failed (Code:{:?}):\n{}", conversion_type, exit_code.unwrap_or(-1), tool_stderr.chars().take(300).collect::<String>())));
            }
            if !temp_html_path.exists() || fs::metadata(&temp_html_path)?.len() == 0 {
                 warn!("[import_document] Pandoc {} success but output missing/empty: {}", conversion_type, temp_html_path.display());
                 return Err(CommandError::from(format!("Pandoc {} success but output empty/missing.", conversion_type)));
            }

            info!("[import_document] Pandoc {} successful to temp HTML: {}", conversion_type, temp_html_path.display());
            final_or_temp_path = format!("{}|original_filename:{}", temp_html_path.to_string_lossy(), source_filename_with_ext);
        }

        _ => {
            error!("[import_document] Unsupported file type for import: .{}", source_extension);
            return Err(CommandError::from(format!("Unsupported file type for import: .{}", source_extension)));
        }
    }

    info!("[import_document] Import process finished. Returning path (and original filename if applicable): {}", final_or_temp_path);
    Ok(final_or_temp_path)
}