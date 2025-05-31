// src-tauri/src/projectview/core_commands.rs
use super::shared_types::{*, TABLES_DIR, IMAGES_DIR, FileMetadata, StandardAssetMetadata}; // Added FileMetadata, StandardAssetMetadata
use super::shared_utils::*;
use crate::welcome::config::CommandError;
use log::{debug, error, info, warn};
use std::{
    fs,
    path::{Path, PathBuf},
};
use quick_xml;
use super::pdf_annotation_handler::get_pdf_annotation_file_path; // ADDED for delete/rename
use chrono::Utc;
use serde_json;
use serde::{Serialize, Deserialize};
use tauri::Manager; // Added for app_handle.emit
use tauri::Emitter; // Added for app_handle.emit_to (if needed for specific window)

#[derive(Clone, serde::Serialize)]
struct MediaRenamedPayload {
    old_media_stem: String,
    new_media_stem: String,
    new_media_file_relative_path: String,
    new_absolute_path: String,
}

#[derive(Clone, Serialize)]
struct ItemRenamedPayload {
    old_path: String,
    new_path: String,
    new_name: String,
    item_type: String,
    project_xml_path: String,
    base_directory: String,
}

// Helper function to get annotation metadata path for an image (from existing code)
fn get_annotation_metadata_path_for_image(image_path: &Path) -> Result<PathBuf, CommandError> {
    let parent_dir = image_path.parent().ok_or_else(|| {
        CommandError::from(format!(
            "Could not get parent directory for image: {}",
            image_path.display()
        ))
    })?;
    let image_stem = image_path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        CommandError::from(format!(
            "Could not get file stem for image: {}",
            image_path.display()
        ))
    })?;

    let metadata_filename = format!(".{}.annotations.json", image_stem);
    Ok(parent_dir.join(metadata_filename))
}

// Helper function to get document metadata path
fn get_document_metadata_path_for_doc(doc_path: &Path) -> Result<PathBuf, CommandError> {
    let doc_parent_dir = doc_path.parent().ok_or_else(|| {
        CommandError::from(format!(
            "Could not get parent directory for document: {}",
            doc_path.display()
        ))
    })?;
    let doc_stem = doc_path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        CommandError::from(format!(
            "Could not get file stem for document: {}",
            doc_path.display()
        ))
    })?;
    let metadata_filename = format!(".{}.{}", doc_stem, METADATA_FILE_SUFFIX);
    Ok(doc_parent_dir.join(metadata_filename))
}

// Helper function to get media metadata path
pub fn get_media_metadata_path(media_path: &Path) -> Result<PathBuf, CommandError> {
    let parent_dir = media_path.parent().ok_or_else(|| {
        CommandError::from(format!(
            "Could not get parent directory for media file: {}",
            media_path.display()
        ))
    })?;
    let media_stem = media_path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        CommandError::from(format!(
            "Could not get file stem for media file: {}",
            media_path.display()
        ))
    })?;

    let metadata_filename = format!(".{}.metadata.json", media_stem);
    Ok(parent_dir.join(metadata_filename))
}

// Helper function to get asset metadata path for an image
pub fn get_image_asset_metadata_path(image_path: &Path) -> Result<PathBuf, CommandError> {
    let parent_dir = image_path.parent().ok_or_else(|| {
        CommandError::from(format!(
            "Could not get parent directory for image asset: {}",
            image_path.display()
        ))
    })?;
    let image_stem = image_path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        CommandError::from(format!(
            "Could not get file stem for image asset: {}",
            image_path.display()
        ))
    })?;

    let metadata_filename = format!(".{}.metadata.json", image_stem); // Hidden file
    Ok(parent_dir.join(metadata_filename))
}

// Helper function to get asset metadata path for a table
pub fn get_table_asset_metadata_path(table_path: &Path) -> Result<PathBuf, CommandError> {
    let parent_dir = table_path.parent().ok_or_else(|| {
        CommandError::from(format!(
            "Could not get parent directory for table asset: {}",
            table_path.display()
        ))
    })?;
    let table_stem = table_path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        CommandError::from(format!(
            "Could not get file stem for table asset: {}",
            table_path.display()
        ))
    })?;

    let metadata_filename = format!(".{}.metadata.json", table_stem); // Hidden file
    Ok(parent_dir.join(metadata_filename))
}


#[tauri::command]
pub async fn load_project_data(project_xml_path: String) -> Result<ProjectViewData, CommandError> {
    info!("[Backend Load XML] Start: {}", project_xml_path);
    let xml_path = PathBuf::from(&project_xml_path);
    if !xml_path.exists() || !xml_path.is_file() {
        return Err(CommandError::from(format!("Project file not found: {}", project_xml_path)));
    }
    let project_base_dir = xml_path.parent().ok_or_else(|| CommandError::from("Could not get project base directory."))?;
    let base_directory = project_base_dir.to_string_lossy().to_string();
    if base_directory.is_empty() {
        return Err(CommandError::from("Base directory path is empty."));
    }

    ensure_base_asset_dirs(project_base_dir)?;

    let project_xml_content = fs::read_to_string(&xml_path).map_err(|e| CommandError::from(format!("Failed to read XML {}: {}", xml_path.display(), e)))?;
    let project_data: ProjectXml = quick_xml::de::from_str(&project_xml_content).map_err(|e| CommandError::from(format!("Failed to parse XML {}: {}", xml_path.display(), e)))?;
    let project_name = project_data.name.clone();
    info!("[Backend Load XML] Project Name: {}", project_name);

    let media_dir_rel_path = format!("{}/{}", HARVEY_FILES_DIR, MEDIA_DIR);
    let mut file_entries: Vec<FileEntry> = Vec::new();

    for media_entry in &project_data.media_files.files {
        let media_stem = &media_entry.name;
        let stem_rel_path = format!("{}/{}", media_dir_rel_path, media_stem);
        let stem_abs_path = project_base_dir.join(&stem_rel_path);

        if !stem_abs_path.exists() || !stem_abs_path.is_dir() {
            warn!("[Backend Load XML] Media stem directory listed in XML does not exist on disk (or is not a dir), skipping entry: '{}'", stem_abs_path.display());
            continue;
        }

        let mut media_children: Vec<FileEntry> = Vec::new();
        let mut transcript_children: Vec<FileEntry> = Vec::new();

        let media_file_rel_path = &media_entry.relative_path;
        let media_file_abs_path = project_base_dir.join(media_file_rel_path);

        if media_file_abs_path.exists() && media_file_abs_path.is_file() {
            let media_file_name = media_file_abs_path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let media_file_canonical = fs::canonicalize(&media_file_abs_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| media_file_abs_path.to_string_lossy().to_string());

            if !media_file_name.is_empty() {
                media_children.push(FileEntry {
                    name: media_file_name,
                    path: media_file_canonical,
                    relative_path: media_file_rel_path.clone().replace("\\", "/"),
                    file_type: "media".to_string(),
                    is_directory: false,
                    parent_relative_path: format!("{}/{}", stem_rel_path, MEDIA_SUBDIR).replace("\\", "/"),
                    depth: 5,
                    speakers: media_entry.speakers.clone(),
                    media_xml_identifier: Some(media_stem.clone()),
                    associated_transcripts: media_entry.transcripts.clone(),
                    children: Vec::new(),
                });
            } else {
                warn!("[Backend Load XML] Could not determine media filename from relative path: {}", media_file_rel_path);
            }
        } else {
            warn!("[Backend Load XML] Media file listed in XML does not exist on disk: '{}'", media_file_abs_path.display());
        }

        for transcript_xml_entry in &media_entry.transcripts {
            let transcript_rel_path = &transcript_xml_entry.relative_path;
            let transcript_abs_path = project_base_dir.join(transcript_rel_path);

            if transcript_abs_path.exists() && transcript_abs_path.is_file() {
                let transcript_file_name = transcript_xml_entry.name.clone();
                 let transcript_file_canonical = fs::canonicalize(&transcript_abs_path)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| transcript_abs_path.to_string_lossy().to_string());

                transcript_children.push(FileEntry {
                    name: transcript_file_name,
                    path: transcript_file_canonical,
                    relative_path: transcript_rel_path.clone().replace("\\", "/"),
                    file_type: "transcript".to_string(),
                    is_directory: false,
                    parent_relative_path: format!("{}/{}", stem_rel_path, TRANSCRIPTS_SUBDIR).replace("\\", "/"),
                    depth: 5,
                    speakers: None,
                    media_xml_identifier: Some(media_stem.clone()),
                    associated_transcripts: Vec::new(),
                    children: Vec::new(),
                });
            } else {
                warn!("[Backend Load XML] Transcript file listed in XML does not exist on disk: '{}'", transcript_abs_path.display());
            }
        }

        media_children.sort_by(|a, b| a.name.cmp(&b.name));
        transcript_children.sort_by(|a, b| a.name.cmp(&b.name));

        let mut sub_folders: Vec<FileEntry> = Vec::new();
        let media_subdir_rel_path = format!("{}/{}", stem_rel_path, MEDIA_SUBDIR).replace("\\", "/");
        sub_folders.push(FileEntry {
            name: MEDIA_SUBDIR.to_string(),
            path: project_base_dir.join(&media_subdir_rel_path).to_string_lossy().to_string(),
            relative_path: media_subdir_rel_path,
            file_type: "directory".to_string(),
            is_directory: true,
            parent_relative_path: stem_rel_path.clone().replace("\\", "/"),
            depth: 4,
            speakers: None,
            media_xml_identifier: Some(media_stem.clone()),
            associated_transcripts: Vec::new(),
            children: media_children,
        });
        let transcripts_subdir_rel_path = format!("{}/{}", stem_rel_path, TRANSCRIPTS_SUBDIR).replace("\\", "/");
        sub_folders.push(FileEntry {
            name: TRANSCRIPTS_SUBDIR.to_string(),
            path: project_base_dir.join(&transcripts_subdir_rel_path).to_string_lossy().to_string(),
            relative_path: transcripts_subdir_rel_path,
            file_type: "directory".to_string(),
            is_directory: true,
            parent_relative_path: stem_rel_path.clone().replace("\\", "/"),
            depth: 4,
            speakers: None,
            media_xml_identifier: Some(media_stem.clone()),
            associated_transcripts: Vec::new(),
            children: transcript_children,
        });

        file_entries.push(FileEntry {
            name: media_stem.clone(),
            path: stem_abs_path.to_string_lossy().to_string(),
            relative_path: stem_rel_path.clone().replace("\\", "/"),
            file_type: "directory_media_stem".to_string(),
            is_directory: true,
            parent_relative_path: media_dir_rel_path.clone().replace("\\", "/"),
            depth: 3,
            speakers: media_entry.speakers.clone(),
            media_xml_identifier: Some(media_stem.clone()),
            associated_transcripts: Vec::new(),
            children: sub_folders,
        });
    }
    file_entries.sort_by(|a, b| a.name.cmp(&b.name));

    log::debug!(
        "[Backend Load XML] Media stems: {}, Documents: {}, Tables: {}, Images: {}, Imported Transcripts: {}, App Metadata Files: {}, PDF Annotation Files: {}",
        file_entries.len(),
        project_data.document_files.files.len(),
        project_data.table_files.files.len(),
        project_data.image_files.files.len(),
        project_data.imported_transcript_files.files.len(),
        project_data.document_metadata_files.files.len(),
        project_data.pdf_annotation_files.files.len() // ADDED
    );

    Ok(ProjectViewData {
        project_name,
        project_xml_path,
        base_directory,
        files: file_entries,
        document_files: project_data.document_files.files,
        table_files: project_data.table_files.files,
        image_files: project_data.image_files.files,
        imported_transcript_files: project_data.imported_transcript_files.files,
        document_metadata_files: project_data.document_metadata_files.files,
        pdf_annotation_files: project_data.pdf_annotation_files.files, // ADDED
    })
}


#[tauri::command]
pub async fn import_media( source_file_path_str: String, project_xml_path_str: String) -> Result<Vec<FileEntry>, CommandError> {
    info!("[Backend Import] Source: '{}', Project XML: '{}'", source_file_path_str, project_xml_path_str);
    let source_path = PathBuf::from(&source_file_path_str);
    let project_xml_path = PathBuf::from(&project_xml_path_str);

    if !source_path.exists() || !source_path.is_file() {
        return Err(CommandError::from(format!("Source file not found: {}", source_file_path_str)));
    }
    let project_base_dir = project_xml_path.parent().ok_or_else(|| CommandError::from("Could not get project base directory"))?;
    if !project_base_dir.exists() || !project_base_dir.is_dir() {
        return Err(CommandError::from(format!("Project base directory not found: {}", project_base_dir.display())));
    }

    let source_filename_os = source_path.file_name().ok_or_else(|| CommandError::from("Could not get filename"))?;
    let source_filename = source_filename_os.to_string_lossy().to_string();

    let media_stem_identifier = source_path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| CommandError::from("Invalid source filename stem."))?;

    let media_asset_dir = project_base_dir.join(HARVEY_FILES_DIR).join(MEDIA_DIR);
    let media_stem_base_path = media_asset_dir.join(media_stem_identifier);
    let media_subfolder_path = media_stem_base_path.join(MEDIA_SUBDIR);
    let transcripts_subfolder_path = media_stem_base_path.join(TRANSCRIPTS_SUBDIR);
    let destination_media_path = media_subfolder_path.join(&source_filename);

    let xml_content_check = fs::read_to_string(&project_xml_path)?;
    let project_data_check: ProjectXml = quick_xml::de::from_str(&xml_content_check)?;
    if project_data_check.media_files.files.iter().any(|f| f.name == media_stem_identifier) {
        return Err(CommandError::from(format!("Media identifier '{}' already exists.", media_stem_identifier)));
    }

    if media_stem_base_path.exists() {
        warn!("[Backend Import] Target media stem directory exists: {}. Files may be overwritten or structure reused.", media_stem_base_path.display());
    }

    fs::create_dir_all(&media_subfolder_path)?;
    fs::create_dir_all(&transcripts_subfolder_path)?;

    fs::copy(&source_path, &destination_media_path)?;
    info!("[Backend Import] File copied to {}", destination_media_path.display());

    // Create a structured metadata file
    match get_media_metadata_path(&destination_media_path) {
        Ok(metadata_path) => {
            let file_name = destination_media_path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("")
                .to_string();

            let metadata_content = StandardAssetMetadata {
                metadata: FileMetadata {
                    file_name,
                    file_path: destination_media_path.to_string_lossy().into_owned(),
                    last_modified: Utc::now().to_rfc3339(),
                    title: "".to_string(),
                    description: "".to_string(),
                    summary: "".to_string(),
                },
                highlights: Vec::new(),
            };

            match serde_json::to_string_pretty(&metadata_content) {
                Ok(json_string) => {
                    if let Err(e) = fs::write(&metadata_path, json_string) {
                        error!("[Backend Import] Failed to write metadata file {}: {}", metadata_path.display(), e);
                    } else {
                        info!("[Backend Import] Created metadata file: {}", metadata_path.display());
                    }
                }
                Err(e) => {
                    error!("[Backend Import] Failed to serialize metadata for {}: {}", metadata_path.display(), e);
                }
            }
        }
        Err(e) => {
            error!("[Backend Import] Failed to get media metadata path for {}: {:?}", destination_media_path.display(), e);
            // Do not block import if metadata path generation fails
        }
    }

    let xml_content = fs::read_to_string(&project_xml_path)?;
    let mut project_data: ProjectXml = quick_xml::de::from_str(&xml_content)?;

    let destination_relative_path_for_xml = Path::new(HARVEY_FILES_DIR)
        .join(MEDIA_DIR)
        .join(media_stem_identifier)
        .join(MEDIA_SUBDIR)
        .join(&source_filename)
        .to_string_lossy()
        .replace("\\", "/");

    let new_media_entry = MediaFileEntryXml {
        name: media_stem_identifier.to_string(),
        original_path: Some(source_file_path_str.clone()),
        relative_path: destination_relative_path_for_xml,
        speakers: Some(SpeakersXml::default()),
        transcripts: Vec::new(),
    };

    project_data.media_files.files.push(new_media_entry);
    project_data.media_files.files.sort_by(|a,b| a.name.cmp(&b.name));

    save_project_xml(&project_xml_path, &project_data)?;
    info!("[Backend Import] XML updated with entry '{}'.", media_stem_identifier);

    load_project_data(project_xml_path_str).await.map(|data| data.files)
}


#[tauri::command]
pub async fn delete_project_item( item_path: String, project_xml_path: String) -> Result<(), CommandError> {
    info!("[Backend Delete] Request for: {}", item_path);
    let item_path_buf = PathBuf::from(&item_path);
    let xml_path_buf = PathBuf::from(&project_xml_path);

    if !xml_path_buf.exists() || !xml_path_buf.is_file() {
        return Err(CommandError::from(format!("Project XML not found: {}", project_xml_path)));
    }
    let project_base_dir = xml_path_buf.parent().ok_or_else(|| CommandError::from("Could not get project base dir"))?;

    if !item_path_buf.exists() {
        warn!("[Backend Delete] Item '{}' not found. Assuming already deleted or invalid path. Attempting XML cleanup...", item_path);
        let (item_type_guess, media_stem_opt_guess, item_relative_path_buf_guess) = match get_item_details(&item_path_buf, project_base_dir) {
            Ok(details) => details,
            Err(_) => {
                warn!("[Backend Delete] Could not determine item details for non-existent path '{}'. Skipping XML cleanup.", item_path);
                return Ok(());
            }
        };
        let item_relative_path_guess = item_relative_path_buf_guess.to_string_lossy().replace("\\", "/");
        // Enhanced detection for imported transcripts and tables in non-existent path cleanup
        let item_type_guess = {
            let path_lower = item_relative_path_guess.to_lowercase();
            let transcripts_folder = format!("{}/", TRANSCRIPTS_SUBDIR.to_lowercase());
            let tables_folder = format!("{}/", TABLES_DIR.to_lowercase());
            let ext = item_path_buf.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if item_type_guess == "other" && path_lower.contains(&transcripts_folder) && ext == "json" {
                "imported_transcript".to_string()
            } else if item_type_guess == "other" && path_lower.contains(&tables_folder) && (ext == "csv" || ext == "xlsx") {
                "table".to_string()
            } else {
                item_type_guess
            }
        };
        let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
        let mut xml_changed = false;

        match item_type_guess.as_str() {
            "media" | "directory_media_stem" => {
                if let Some(media_stem) = media_stem_opt_guess {
                    let initial_len = project_data.media_files.files.len();
                    project_data.media_files.files.retain(|entry| entry.name != media_stem);
                    if project_data.media_files.files.len() < initial_len {
                        info!("[Backend Delete] Cleaned up XML media entry for non-existent '{}'.", media_stem);
                        xml_changed = true;
                    }
                }
            },
            "transcript" => {
                if let Some(media_stem) = media_stem_opt_guess {
                    if let Some(media_entry) = project_data.media_files.files.iter_mut().find(|f| f.name == media_stem) {
                        let initial_transcript_len = media_entry.transcripts.len();
                        media_entry.transcripts.retain(|t| t.relative_path != item_relative_path_guess);
                        if media_entry.transcripts.len() < initial_transcript_len {
                            info!("[Backend Delete] Cleaned up XML media-associated transcript entry '{}'.", item_relative_path_guess);
                            xml_changed = true;
                        }
                    }
                }
            },
            "imported_transcript" => {
                let initial_len = project_data.imported_transcript_files.files.len();
                project_data.imported_transcript_files.files.retain(|t| t.relative_path != item_relative_path_guess);
                if project_data.imported_transcript_files.files.len() < initial_len {
                    info!("[Backend Delete] Cleaned up XML imported transcript entry '{}'.", item_relative_path_guess);
                    xml_changed = true;
                }
                let initial_meta_len = project_data.document_metadata_files.files.len();
                project_data.document_metadata_files.files.retain(|m| m.original_document_relative_path != item_relative_path_guess);
                if project_data.document_metadata_files.files.len() < initial_meta_len {
                    info!("[Backend Delete] Cleaned up XML document metadata entry for original imported transcript '{}'.", item_relative_path_guess);
                    xml_changed = true;
                }
            },
            "doc" => { // This handles .json, .pdf, .md, .txt documents
                let initial_doc_len = project_data.document_files.files.len();
                project_data.document_files.files.retain(|d| d.relative_path != item_relative_path_guess);
                if project_data.document_files.files.len() < initial_doc_len {
                    info!("[Backend Delete] Cleaned up XML document entry '{}'.", item_relative_path_guess);
                    xml_changed = true;
                }
                // Clean up app metadata
                let initial_meta_len = project_data.document_metadata_files.files.len();
                project_data.document_metadata_files.files.retain(|m| m.original_document_relative_path != item_relative_path_guess);
                if project_data.document_metadata_files.files.len() < initial_meta_len {
                    info!("[Backend Delete] Cleaned up XML document (app) metadata entry for original doc '{}'.", item_relative_path_guess);
                    xml_changed = true;
                }
                // Clean up PDF annotations if it was a PDF
                if item_relative_path_guess.to_lowercase().ends_with(".pdf") {
                    let initial_pdf_annot_len = project_data.pdf_annotation_files.files.len();
                    project_data.pdf_annotation_files.files.retain(|pa| pa.original_document_relative_path != item_relative_path_guess);
                    if project_data.pdf_annotation_files.files.len() < initial_pdf_annot_len {
                        info!("[Backend Delete] Cleaned up XML PDF annotation entry for original PDF '{}'.", item_relative_path_guess);
                        xml_changed = true;
                    }
                }
            },
            "table" => {
                let initial_table_len = project_data.table_files.files.len();
                project_data.table_files.files.retain(|t| t.relative_path != item_relative_path_guess);
                if project_data.table_files.files.len() < initial_table_len {
                    info!("[Backend Delete] Cleaned up XML table entry '{}'.", item_relative_path_guess);
                    xml_changed = true;
                }
            },
            "image" => {
                let initial_image_len = project_data.image_files.files.len();
                project_data.image_files.files.retain(|i| i.relative_path != item_relative_path_guess);
                if project_data.image_files.files.len() < initial_image_len {
                    info!("[Backend Delete] Cleaned up XML image entry '{}'.", item_relative_path_guess);
                    xml_changed = true;
                }
            },
            _ => {
                warn!("[Backend Delete] Unknown item type '{}' for XML cleanup of non-existent path '{}'.", item_type_guess, item_path);
            }
        }

        if xml_changed { save_project_xml(&xml_path_buf, &project_data)?; }
        return Ok(());
    }

    if item_path_buf.is_dir() {
         let (item_type, _, _) = get_item_details(&item_path_buf, project_base_dir)?;
         if item_type != "directory_media_stem" {
            return Err(CommandError::from(format!("Deleting arbitrary directories ('{}') is not supported via this function. Delete the associated media file or asset instead.", item_type)));
         }
         warn!("[Backend Delete] Request path '{}' is a media stem directory. Deletion will be handled by logic for its primary media file.", item_path);
    }

    let (item_type, media_stem_opt, item_relative_path_buf) = get_item_details(&item_path_buf, project_base_dir)?;
    let item_relative_path = item_relative_path_buf.to_string_lossy().replace("\\", "/");
    // Enhanced detection for imported transcripts, tables, and images
    let item_type = {
        let path_lower = item_relative_path.to_lowercase();
        let transcripts_folder = format!("{}/", TRANSCRIPTS_SUBDIR.to_lowercase());
        let tables_folder = format!("{}/", TABLES_DIR.to_lowercase());
        let images_folder = format!("{}/", IMAGES_DIR.to_lowercase());
        let ext = item_path_buf.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if item_type == "other" && path_lower.contains(&transcripts_folder) && ext == "json" {
            "imported_transcript".to_string()
        } else if item_type == "other" && path_lower.contains(&tables_folder) && (ext == "csv" || ext == "xlsx") {
            "table".to_string()
        } else if item_type == "other" && path_lower.contains(&images_folder)
            && matches!(ext.as_str(), "jpg"|"jpeg"|"png"|"gif"|"bmp"|"webp"|"tiff")
        {
            "image".to_string()
        } else {
            item_type
        }
    };
    info!("[Backend Delete] Item type: '{}', Media Stem: {:?}, Rel Path: '{}'", item_type, media_stem_opt, item_relative_path);

    match item_type.as_str() {
        "media" => {
             if let Some(media_stem) = media_stem_opt.as_deref() {
                let media_stem_dir_path = project_base_dir.join(HARVEY_FILES_DIR).join(MEDIA_DIR).join(media_stem);
                if media_stem_dir_path.exists() && media_stem_dir_path.is_dir() {
                    info!("[Backend Delete] Deleting media stem directory: {}", media_stem_dir_path.display());
                    fs::remove_dir_all(&media_stem_dir_path).map_err(|e| CommandError::from(format!("Failed to delete directory {}: {}", media_stem_dir_path.display(), e)))?;

                    info!("[Backend Delete] Updating XML to remove entry for '{}'", media_stem);
                    let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
                    let initial_len = project_data.media_files.files.len();
                    project_data.media_files.files.retain(|entry| entry.name != media_stem);
                    if project_data.media_files.files.len() < initial_len {
                        save_project_xml(&xml_path_buf, &project_data)?;
                        info!("[Backend Delete] XML media entry removed.");
                    } else {
                        warn!("[Backend Delete] Deleted directory but no XML entry found for '{}'.", media_stem);
                    }
                } else {
                    warn!("[Backend Delete] Media stem directory {} not found. Assuming already deleted. Cleaning up XML.", media_stem_dir_path.display());
                     let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
                     let initial_len = project_data.media_files.files.len();
                     project_data.media_files.files.retain(|entry| entry.name != media_stem);
                     if project_data.media_files.files.len() < initial_len {
                         save_project_xml(&xml_path_buf, &project_data)?;
                         info!("[Backend Delete] XML media entry removed during cleanup.");
                     }
                }
            } else {
                return Err(CommandError::from(format!("Could not determine media stem for media file deletion: {}", item_path)));
            }
        },
        "transcript" => {
             if let Some(media_stem) = media_stem_opt.as_deref() {
                info!("[Backend Delete] Deleting media-associated transcript file: {}", item_path_buf.display());
                fs::remove_file(&item_path_buf).map_err(|e| CommandError::from(format!("Failed to delete file {}: {}", item_path_buf.display(), e)))?;

                info!("[Backend Delete] Updating XML to remove transcript link for '{}' with path '{}'", media_stem, item_relative_path);
                let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
                let mut xml_changed = false;
                if let Some(media_entry) = project_data.media_files.files.iter_mut().find(|f| f.name == media_stem) {
                    let initial_transcript_len = media_entry.transcripts.len();
                    media_entry.transcripts.retain(|t| t.relative_path != item_relative_path);
                    if media_entry.transcripts.len() < initial_transcript_len {
                        info!("[Backend Delete] Transcript entry removed from XML for media '{}'.", media_stem);
                        xml_changed = true;
                    } else {
                        warn!("[Backend Delete] Deleted transcript file, but no matching entry found in XML for path '{}' under media '{}'.", item_relative_path, media_stem);
                    }
                } else {
                    warn!("[Backend Delete] Deleted transcript file, but media identifier '{}' not found in XML.", media_stem);
                }
                if xml_changed {
                    save_project_xml(&xml_path_buf, &project_data)?;
                    info!("[Backend Delete] XML updated.");
                }
            } else {
                return Err(CommandError::from(format!("Could not determine media stem for transcript: {}", item_path)));
            }
        },
        "imported_transcript" => {
            info!("[Backend Delete] Deleting standalone imported transcript file: {}", item_path_buf.display());
            fs::remove_file(&item_path_buf)
                .map_err(|e| CommandError::from(format!("Failed to delete imported transcript file {}: {}", item_path_buf.display(), e)))?;

            // 1. Delete containing folder if empty
            if let Some(folder) = item_path_buf.parent() {
                if folder.exists() {
                    match fs::remove_dir(folder) {
                        Ok(_) => (),
                        Err(err) if err.kind() == std::io::ErrorKind::DirectoryNotEmpty => (),
                        Err(err) => return Err(CommandError::from(format!("Failed to delete transcript folder: {}", err))),
                    }
                }
            }

            // 2. Delete its metadata file, if present
            if let Ok(metadata_path) = get_document_metadata_path_for_doc(&item_path_buf) {
                if metadata_path.exists() {
                    info!("[Backend Delete] Deleting metadata file for imported transcript: {}", metadata_path.display());
                    if let Err(e) = fs::remove_file(&metadata_path) {
                        warn!("[Backend Delete] Failed to delete metadata file for imported transcript {}: {}", metadata_path.display(), e);
                    }
                }
            }

            // 3. Update project XML to remove the transcript and metadata entries
            info!("[Backend Delete] Updating XML to remove imported transcript entry '{}'", item_relative_path);
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            let initial_entries = project_data.imported_transcript_files.files.len();
            project_data.imported_transcript_files.files.retain(|t| t.relative_path != item_relative_path);
            let initial_meta = project_data.document_metadata_files.files.len();
            project_data.document_metadata_files.files.retain(|m| m.original_document_relative_path != item_relative_path);

            if project_data.imported_transcript_files.files.len() < initial_entries
                || project_data.document_metadata_files.files.len() < initial_meta
            {
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Delete] XML updated for imported transcript and its metadata.");
            }
        },
        "doc" => {
            // Delete entire document folder (file, metadata, annotations, .tmp)
            let stem = item_path_buf.file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| CommandError::from(format!("Could not get document stem: {}", item_path_buf.display())))?;
            let docs_root = project_base_dir.join(HARVEY_FILES_DIR).join(DOCS_DIR);
            let doc_folder = docs_root.join(stem);
            if doc_folder.exists() && doc_folder.is_dir() {
                info!("[Backend Delete] Deleting document folder: {}", doc_folder.display());
                fs::remove_dir_all(&doc_folder)
                    .map_err(|e| CommandError::from(format!("Failed to delete document folder {}: {}", doc_folder.display(), e)))?;
            } else {
                info!("[Backend Delete] Document folder not found, deleting single file: {}", item_path_buf.display());
                fs::remove_file(&item_path_buf)
                    .map_err(|e| CommandError::from(format!("Failed to delete document file {}: {}", item_path_buf.display(), e)))?;
            }
            // Prune XML entries for this document
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            // Match XML entries which include the full "harvey_files/Documents/<stem>" path
            let prefix = format!("{}/{}/{}", HARVEY_FILES_DIR, DOCS_DIR, stem);
            project_data.document_files.files.retain(|d| !d.relative_path.starts_with(&prefix));
            project_data.document_metadata_files.files
                .retain(|m| !m.original_document_relative_path.starts_with(&prefix));
            project_data.pdf_annotation_files.files
                .retain(|p| !p.original_document_relative_path.starts_with(&prefix));
            save_project_xml(&xml_path_buf, &project_data)?;
            info!("[Backend Delete] XML entries removed for document '{}'", stem);
        },
        "table" => {
            info!("[Backend Delete] Deleting table file: {}", item_path_buf.display());

            // Construct folder path
            let file_stem = item_path_buf.file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| CommandError::from(format!("Could not get table filename stem for deletion: {}", item_path_buf.display())))?;

            let tables_dir = project_base_dir.join(HARVEY_FILES_DIR).join(TABLES_DIR);
            let folder_path = tables_dir.join(file_stem);

            // Delete folder and its contents
            if folder_path.exists() && folder_path.is_dir() {
                info!("[Backend Delete] Deleting table folder: {}", folder_path.display());
                fs::remove_dir_all(&folder_path).map_err(|e| CommandError::from(format!("Failed to delete table folder {}: {}", folder_path.display(), e)))?;
            } else {
                warn!("[Backend Delete] Table folder {} not found. Assuming already deleted.", folder_path.display());
            }

            // XML update (no need to remove file separately, folder deletion suffices)
            info!("[Backend Delete] Updating XML to remove table link with path '{}'", item_relative_path);
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            let initial_table_len = project_data.table_files.files.len();
            project_data.table_files.files.retain(|t| t.relative_path != item_relative_path);
            if project_data.table_files.files.len() < initial_table_len {
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Delete] Table entry removed from XML.");
            } else {
                warn!("[Backend Delete] Deleted table file, but no matching entry found in XML for path '{}'.", item_relative_path);
            }
        },
        "image" => {
            info!("[Backend Delete] Request to delete image and its folder for: {}", item_path_buf.display());

            // Get the parent folder of the image file. This is the folder to be deleted.
            let image_folder_to_delete = item_path_buf.parent().ok_or_else(|| {
                CommandError::from(format!(
                    "Could not get parent directory for image file: {}",
                    item_path_buf.display()
                ))
            })?;

            if image_folder_to_delete.exists() && image_folder_to_delete.is_dir() {
                info!("[Backend Delete] Deleting image folder: {}", image_folder_to_delete.display());
                fs::remove_dir_all(image_folder_to_delete).map_err(|e| {
                    CommandError::from(format!(
                        "Failed to delete image folder {}: {}",
                        image_folder_to_delete.display(),
                        e
                    ))
                })?;
            } else {
                warn!(
                    "[Backend Delete] Image folder {} not found. Assuming already deleted or structure is unexpected. Proceeding with XML cleanup.",
                    image_folder_to_delete.display()
                );
            }

            // Update project XML to remove image entry
            info!("[Backend Delete] Updating XML to remove image entry '{}'", item_relative_path);
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            let initial_len = project_data.image_files.files.len();
            project_data.image_files.files.retain(|i| i.relative_path != item_relative_path);

            if project_data.image_files.files.len() < initial_len {
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Delete] XML updated for image.");
            } else {
                warn!("[Backend Delete] Deleted image folder (or it was already gone), but no matching entry found in XML for path '{}'.", item_relative_path);
            }
        },
        _ => {
            error!("[Backend Delete] Deleting items of type '{}' is not supported directly: {}", item_type, item_path);
            return Err(CommandError::from(format!("Deletion not supported for item type '{}'. Delete the primary associated asset.", item_type)));
        }
    }

    info!("[Backend Delete] Success for: {}", item_path);
    Ok(())
}


#[tauri::command]
pub async fn rename_project_item( app_handle: tauri::AppHandle, item_path: String, new_name: String, project_xml_path: String) -> Result<(), CommandError> {
    info!("[Backend Rename] Request: Item='{}', NewNameParam='{}'", item_path, new_name);
    let item_path_buf = PathBuf::from(&item_path);
    let xml_path_buf = PathBuf::from(&project_xml_path);
    let new_name_trimmed = new_name.trim();

    if !item_path_buf.exists() {
        return Err(CommandError::from(format!("Item not found: {}", item_path)));
    }
    if new_name_trimmed.is_empty() {
        return Err(CommandError::from("New name cannot be empty."));
    }
    if !xml_path_buf.exists() || !xml_path_buf.is_file() {
        return Err(CommandError::from(format!("Project XML not found: {}", project_xml_path)));
    }
    let project_base_dir = xml_path_buf.parent().ok_or_else(|| CommandError::from("Could not get project base dir"))?;

    if item_path_buf.is_dir() {
        let (item_type, _, _) = get_item_details(&item_path_buf, project_base_dir)?;
        if item_type != "directory_media_stem" {
             return Err(CommandError::from(format!("Renaming arbitrary directories ('{}') is not supported via this function. Rename the associated asset file instead.", item_type)));
        }
         warn!("[Backend Rename] Request path '{}' is a directory, but rename should be triggered by media file. Proceeding with media logic.", item_path);
    }

    let contains_invalid_chars = |name: &str| name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']);
    let (item_type, media_stem_opt, item_relative_path_buf) = get_item_details(&item_path_buf, project_base_dir)?;
    let item_relative_path = item_relative_path_buf.to_string_lossy().replace("\\", "/");
    info!("[Backend Rename] Item type: '{}', Media Stem: {:?}, Rel Path: '{}'", item_type, media_stem_opt, item_relative_path);

    let parent_dir = item_path_buf.parent().ok_or_else(|| CommandError::from(format!("Could not get parent directory for {}", item_path_buf.display())))?;

    match item_type.as_str() {
        "media" => {
             let old_stem = media_stem_opt.ok_or_else(|| CommandError::from("Could not get media stem identifier for rename."))?;
            let original_extension = item_path_buf.extension().and_then(|s| s.to_str()).unwrap_or("");
            let new_stem = if new_name_trimmed.contains('.') {
                Path::new(new_name_trimmed).file_stem().and_then(|s| s.to_str()).unwrap_or(new_name_trimmed).to_string()
            } else {
                new_name_trimmed.to_string()
            };

            info!("[Backend Rename] Media Rename: OldStem='{}', NewStem='{}'", old_stem, new_stem);

            if contains_invalid_chars(&new_stem) {
                return Err(CommandError::from("New media name contains invalid characters."));
            }
            if new_stem == old_stem {
                 info!("[Backend Rename] New name is same as old name. No action needed.");
                 return Ok(());
            }

            let media_asset_dir = project_base_dir.join(HARVEY_FILES_DIR).join(MEDIA_DIR);
            let old_stem_dir_path = media_asset_dir.join(&old_stem);
            let new_stem_dir_path = media_asset_dir.join(&new_stem);

            if new_stem_dir_path.exists() {
                return Err(CommandError::from(format!("A media project named '{}' already exists.", new_stem)));
            }

            info!("[Backend Rename] Renaming dir {} -> {}", old_stem_dir_path.display(), new_stem_dir_path.display());
            fs::rename(&old_stem_dir_path, &new_stem_dir_path).map_err(|e| CommandError::from(format!("Failed to rename media directory: {}", e)))?;

            let new_media_subdir = new_stem_dir_path.join(MEDIA_SUBDIR);
            let old_filename_in_new_dir = format!("{}.{}", old_stem, original_extension);
            let new_filename = format!("{}.{}", new_stem, original_extension);
            let old_media_path_in_new_dir = new_media_subdir.join(old_filename_in_new_dir);
            let new_media_path = new_media_subdir.join(&new_filename);
            let primary_media_new_relative_path;

            if old_media_path_in_new_dir.exists() {
                info!("[Backend Rename] Renaming media file {} -> {}", old_media_path_in_new_dir.display(), new_media_path.display());
                if let Err(e) = fs::rename(&old_media_path_in_new_dir, &new_media_path) {
                    warn!("Failed rename media file: {}. Reverting directory rename.", e);
                    let _ = fs::rename(&new_stem_dir_path, &old_stem_dir_path);
                    return Err(CommandError::from(format!("Failed to rename internal media file: {}", e)));
                }
                primary_media_new_relative_path = Path::new(HARVEY_FILES_DIR).join(MEDIA_DIR).join(&new_stem).join(MEDIA_SUBDIR).join(&new_filename).to_string_lossy().replace("\\", "/");
            } else {
                warn!("[Backend Rename] Media file not found at expected path {} inside renamed directory {}. Reverting directory rename.", old_media_path_in_new_dir.display(), new_stem_dir_path.display());
                let _ = fs::rename(&new_stem_dir_path, &old_stem_dir_path);
                return Err(CommandError::from(format!("Original media file structure inconsistent after directory rename. Expected file at {}", old_media_path_in_new_dir.display())));
            }

            // Enhanced media metadata handling: read, update, and rewrite, or create new.
            let old_metadata_path_result = get_media_metadata_path(&old_media_path_in_new_dir);
            let new_metadata_path_result = get_media_metadata_path(&new_media_path);

            match (old_metadata_path_result, new_metadata_path_result) {
                (Ok(old_metadata_path), Ok(new_metadata_path)) => {
                    let mut metadata_content: Option<StandardAssetMetadata> = None;

                    if old_metadata_path.exists() {
                        info!("[Backend Rename] Attempting to read old media metadata file: {}", old_metadata_path.display());
                        match fs::read_to_string(&old_metadata_path) {
                            Ok(old_json_content) => {
                                match serde_json::from_str::<StandardAssetMetadata>(&old_json_content) {
                                    Ok(mut parsed_metadata) => {
                                        // Successfully parsed, now update fields
                                        parsed_metadata.metadata.file_name = new_media_path.file_name()
                                            .and_then(|s| s.to_str()).unwrap_or("").to_string();
                                        parsed_metadata.metadata.file_path = new_media_path.to_string_lossy().into_owned();
                                        parsed_metadata.metadata.last_modified = Utc::now().to_rfc3339();
                                        metadata_content = Some(parsed_metadata);
                                        info!("[Backend Rename] Successfully parsed and updated old media metadata.");
                                    }
                                    Err(e) => {
                                        warn!("[Backend Rename] Failed to parse old media metadata file {}: {}. A new one will be created.", old_metadata_path.display(), e);
                                        // Proceed to create new metadata below
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("[Backend Rename] Failed to read old media metadata file {}: {}. A new one will be created.", old_metadata_path.display(), e);
                                // Proceed to create new metadata below
                            }
                        }
                        // Attempt to remove the old metadata file if it was read or if parsing failed (to replace it)
                        if let Err(e) = fs::remove_file(&old_metadata_path) {
                            warn!("[Backend Rename] Failed to remove old media metadata file {}: {}", old_metadata_path.display(), e);
                        }
                    } else {
                        info!("[Backend Rename] Old media metadata file {} not found. A new one will be created.", old_metadata_path.display());
                    }

                    // If metadata wasn't loaded and updated, create new default metadata
                    let final_metadata_to_write = metadata_content.unwrap_or_else(|| {
                        info!("[Backend Rename] Creating new media metadata content for {}.", new_media_path.display());
                        StandardAssetMetadata {
                            metadata: FileMetadata {
                                file_name: new_media_path.file_name()
                                    .and_then(|s| s.to_str()).unwrap_or("").to_string(),
                                file_path: new_media_path.to_string_lossy().into_owned(),
                                last_modified: Utc::now().to_rfc3339(),
                                title: "".to_string(),
                                description: "".to_string(),
                                summary: "".to_string(),
                            },
                            highlights: Vec::new(),
                        }
                    });

                    // Write the (potentially updated or new) metadata to the new path
                    match serde_json::to_string_pretty(&final_metadata_to_write) {
                        Ok(json_string) => {
                            if let Err(e) = fs::write(&new_metadata_path, json_string) {
                                warn!("[Backend Rename] Failed to write media metadata file {}: {}", new_metadata_path.display(), e);
                            } else {
                                info!("[Backend Rename] Successfully wrote media metadata to {}", new_metadata_path.display());
                            }
                        }
                        Err(e) => {
                            warn!("[Backend Rename] Failed to serialize media metadata for {}: {}", new_metadata_path.display(), e);
                        }
                    }
                }
                (Err(e_old), _) => {
                    warn!("[Backend Rename] Could not determine old media metadata path for {}: {:?}", old_media_path_in_new_dir.display(), e_old);
                }
                (_, Err(e_new)) => {
                    warn!("[Backend Rename] Could not determine new media metadata path for {}: {:?}", new_media_path.display(), e_new);
                }
            }

            let old_transcript_filename = format!("{}.json", old_stem);
            let new_transcript_filename = format!("{}.json", new_stem);
            let transcript_subdir_in_new_stem = new_stem_dir_path.join(TRANSCRIPTS_SUBDIR);
            let old_transcript_path_in_new_dir = transcript_subdir_in_new_stem.join(&old_transcript_filename);
            let new_transcript_path = transcript_subdir_in_new_stem.join(&new_transcript_filename);

            if old_transcript_path_in_new_dir.exists() {
                info!("[Backend Rename] Renaming primary transcript {} -> {}", old_transcript_path_in_new_dir.display(), new_transcript_path.display());
                 if let Err(e) = fs::rename(&old_transcript_path_in_new_dir, &new_transcript_path) {
                     warn!("Failed rename primary transcript: {}. Reverting directory and media file renames.", e);
                     let _ = fs::rename(&new_media_path, &old_media_path_in_new_dir);
                     let _ = fs::rename(&new_stem_dir_path, &old_stem_dir_path);
                     return Err(CommandError::from(format!("Failed to rename primary transcript file: {}", e)));
                 }
            } else {
                info!("[Backend Rename] Primary transcript {} not found, skipping transcript rename.", old_transcript_path_in_new_dir.display());
            }

            info!("[Backend Rename] Updating XML: ID '{}' -> '{}', Path -> '{}'", old_stem, new_stem, primary_media_new_relative_path);
            let xml_content = fs::read_to_string(&xml_path_buf)?;
            let mut project_data: ProjectXml = quick_xml::de::from_str(&xml_content)?;
            if let Some(entry) = project_data.media_files.files.iter_mut().find(|f| f.name == old_stem) {
                entry.name = new_stem.clone();
                entry.relative_path = primary_media_new_relative_path.clone();

                for transcript_entry in entry.transcripts.iter_mut() {
                    let old_t_path = PathBuf::from(&transcript_entry.relative_path);
                    if let Some(t_filename) = old_t_path.file_name().and_then(|n| n.to_str()) {
                        let new_t_filename = if t_filename == old_transcript_filename {
                            new_transcript_filename.clone()
                        } else {
                            t_filename.to_string()
                        };
                         let new_t_relative_path = Path::new(HARVEY_FILES_DIR).join(MEDIA_DIR).join(&new_stem).join(TRANSCRIPTS_SUBDIR).join(&new_t_filename).to_string_lossy().replace("\\", "/");

                        debug!("[Backend Rename XML] Updating transcript path from '{}' to '{}'", transcript_entry.relative_path, new_t_relative_path);
                        transcript_entry.relative_path = new_t_relative_path;
                        if t_filename == old_transcript_filename {
                            transcript_entry.name = new_transcript_filename.clone();
                        }
                    } else {
                        warn!("[Backend Rename XML] Could not parse filename from transcript relative path: {}", transcript_entry.relative_path);
                    }
                }
                entry.transcripts.sort_by(|a,b| a.name.cmp(&b.name));

                info!("[Backend Rename] XML entry updated.");
                project_data.media_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Rename] XML saved.");

                // Emit the media_renamed event
                let payload = MediaRenamedPayload {
                    old_media_stem: old_stem.clone(),
                    new_media_stem: new_stem.clone(),
                    new_media_file_relative_path: primary_media_new_relative_path.clone(),
                    new_absolute_path: new_media_path.to_string_lossy().into_owned(),
                };
                if let Err(e) = app_handle.emit("media_renamed", payload) {
                    warn!("[Backend Rename] Failed to emit media_renamed event for new stem {}: {}", new_stem, e);
                }

            } else {
                error!("[Backend Rename] CRITICAL: Failed find XML entry for '{}' after file operations. File system may be inconsistent.", old_stem);
                return Err(CommandError::from(format!("XML entry for '{}' not found after successful file renames. Project state potentially inconsistent.", old_stem)));
            }
        },
        "transcript" => {
            let new_filename_with_ext = new_name_trimmed;
            let new_path = parent_dir.join(new_filename_with_ext);

            if contains_invalid_chars(new_filename_with_ext) { return Err(CommandError::from("New filename contains invalid characters.")); }
            if !new_filename_with_ext.ends_with(".json") { return Err(CommandError::from("Transcript filename must end with .json")); }
            if new_filename_with_ext.starts_with('.') { return Err(CommandError::from("Filename cannot start with a dot.")); }

            if item_path_buf == new_path { info!("[Backend Rename] New path is same as old path. No action needed."); return Ok(()); }

            if new_path.exists() {
                 let canon_old = fs::canonicalize(&item_path_buf).ok();
                 let canon_new = fs::canonicalize(&new_path).ok();
                 if canon_old.is_some() && canon_new.is_some() && canon_old != canon_new {
                     return Err(CommandError::from(format!("File named '{}' already exists.", new_filename_with_ext)));
                 } else {
                     debug!("[Backend Rename] Target path exists but might be same file (case change?). Allowing rename attempt.");
                 }
            }

            info!("[Backend Rename] Renaming transcript file {} -> {}", item_path_buf.display(), new_path.display());
            fs::rename(&item_path_buf, &new_path).map_err(|e| CommandError::from(format!("Failed to rename file: {}", e)))?;

            let media_identifier = media_stem_opt.ok_or_else(|| CommandError::from("Could not determine media stem for transcript rename."))?;
            let new_relative_path_buf = new_path.strip_prefix(project_base_dir)?;
            let new_relative_path = new_relative_path_buf.to_string_lossy().replace("\\", "/");

            info!("[Backend Rename] Updating XML for media '{}': Path '{}' -> '{}', name -> '{}'", media_identifier, item_relative_path, new_relative_path, new_filename_with_ext);
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            let mut xml_changed = false;

            if let Some(media_entry) = project_data.media_files.files.iter_mut().find(|f| f.name == media_identifier) {
                if let Some(transcript_entry) = media_entry.transcripts.iter_mut().find(|t| t.relative_path == item_relative_path) {
                    transcript_entry.name = new_filename_with_ext.to_string();
                    transcript_entry.relative_path = new_relative_path;
                    media_entry.transcripts.sort_by(|a,b| a.name.cmp(&b.name));
                    xml_changed = true;
                    info!("[Backend Rename] XML transcript entry updated.");
                } else {
                    warn!("[Backend Rename] Renamed transcript file, but could not find matching path '{}' in XML under media '{}'.", item_relative_path, media_identifier);
                }
            } else {
                warn!("[Backend Rename] Renamed transcript file, but could not find media ID '{}' in XML.", media_identifier);
            }

            if xml_changed {
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Rename] XML saved.");

                let payload = ItemRenamedPayload {
                    old_path: item_path_buf.to_string_lossy().into_owned(),
                    new_path: new_path.to_string_lossy().into_owned(),
                    new_name: new_filename_with_ext.to_string(),
                    item_type: "transcript".to_string(),
                    project_xml_path: xml_path_buf.to_string_lossy().into_owned(),
                    base_directory: project_base_dir.to_string_lossy().into_owned(),
                };
                if let Err(e) = app_handle.emit("item_renamed", payload) {
                    warn!("[Backend Rename] Failed to emit item_renamed event for transcript: {}", e);
                }
            }
        },
        "imported_transcript" => {
            // new_name_trimmed is expected to be the new stem (e.g., "NewTranscriptName")
            let new_transcript_stem_str = new_name_trimmed;
            if contains_invalid_chars(new_transcript_stem_str) { return Err(CommandError::from("New transcript name contains invalid characters.")); }
            if new_transcript_stem_str.starts_with('.') { return Err(CommandError::from("Transcript name cannot start with a dot.")); }

            let new_transcript_filename_with_ext_str = format!("{}.json", new_transcript_stem_str);
            let new_transcript_filename_pathbuf = PathBuf::from(&new_transcript_filename_with_ext_str);

            // --- Path Definitions ---
            let old_transcript_file_path = &item_path_buf; // e.g., .../OldTranscriptStem/OldTranscriptStem.json
            let old_transcript_folder_path = parent_dir;   // e.g., .../OldTranscriptStem/

            let new_transcript_file_path_in_old_folder = old_transcript_folder_path.join(&new_transcript_filename_pathbuf);

            // new_transcript_stem_str is already the stem

            let transcripts_root_path = old_transcript_folder_path.parent()
                .ok_or_else(|| CommandError::from(format!("Could not get Transcripts root from {}", old_transcript_folder_path.display())))?;
            
            let new_transcript_folder_path = transcripts_root_path.join(new_transcript_stem_str);

            // --- Pre-checks for conflicts ---
            if *old_transcript_file_path == new_transcript_file_path_in_old_folder && old_transcript_folder_path == &new_transcript_folder_path {
                info!("[Backend Rename] Imported transcript name and folder name are effectively unchanged. No action needed.");
                return Ok(());
            }
            if old_transcript_folder_path != &new_transcript_folder_path && new_transcript_folder_path.exists() {
                return Err(CommandError::from(format!("A folder named '{}' already exists for imported transcripts. Cannot rename folder.", new_transcript_stem_str)));
            }
            let final_target_transcript_file_path = new_transcript_folder_path.join(&new_transcript_filename_pathbuf);
            if final_target_transcript_file_path.exists() {
                let canon_old_abs = fs::canonicalize(old_transcript_file_path).map_err(|e| CommandError::from(format!("Cannot canonicalize old transcript path {}: {}", old_transcript_file_path.display(), e)))?;
                let canon_final_target_abs = fs::canonicalize(&final_target_transcript_file_path).map_err(|e| CommandError::from(format!("Cannot canonicalize final target transcript path {}: {}", final_target_transcript_file_path.display(), e)))?;
                if canon_final_target_abs != canon_old_abs {
                    return Err(CommandError::from(format!("An imported transcript file named '{}' already exists in the target location '{}'.", new_transcript_filename_with_ext_str, new_transcript_folder_path.display())));
                 }
            }

            // --- File System Operations ---
            // 1. Rename main transcript file (within its current/old folder)
            if old_transcript_file_path != &new_transcript_file_path_in_old_folder {
                info!("[Backend Rename] Renaming imported transcript file {} -> {}", old_transcript_file_path.display(), new_transcript_file_path_in_old_folder.display());
                fs::rename(old_transcript_file_path, &new_transcript_file_path_in_old_folder)
                    .map_err(|e| CommandError::from(format!("Failed to rename imported transcript file: {}", e)))?;
            }

            // 2. Rename associated app metadata file (within its current/old folder)
            // Imported transcripts use the same metadata file naming as documents.
            if let Ok(old_metadata_path) = get_document_metadata_path_for_doc(old_transcript_file_path) {
                if old_metadata_path.exists() {
                    if let Ok(new_metadata_path_in_old_folder) = get_document_metadata_path_for_doc(&new_transcript_file_path_in_old_folder) {
                        if old_metadata_path != new_metadata_path_in_old_folder {
                            info!("[Backend Rename] Renaming metadata for imported transcript: {} -> {}", old_metadata_path.display(), new_metadata_path_in_old_folder.display());
                            if new_metadata_path_in_old_folder.exists() {
                                warn!("[Backend Rename] Target metadata file {} already exists. Skipping rename of old metadata {}.", new_metadata_path_in_old_folder.display(), old_metadata_path.display());
                            } else {
                                if let Err(e) = fs::rename(&old_metadata_path, &new_metadata_path_in_old_folder) {
                                    warn!("[Backend Rename] Failed to rename metadata for imported transcript: {}. Reverting main transcript rename.", e);
                                    if old_transcript_file_path != &new_transcript_file_path_in_old_folder {
                                        let _ = fs::rename(&new_transcript_file_path_in_old_folder, old_transcript_file_path);
                                    }
                                    return Err(CommandError::from(format!("Failed to rename metadata for imported transcript: {}", e)));
                                }
                            }
                        }
                    }
                }
            }

            // 3. Rename the folder if its name (derived from stem) has changed
            let mut current_transcript_folder_path_for_xml_update = old_transcript_folder_path.clone();
            if old_transcript_folder_path != &new_transcript_folder_path {
                info!("[Backend Rename] Renaming imported transcript folder {} -> {}", old_transcript_folder_path.display(), new_transcript_folder_path.display());
                if let Err(e) = fs::rename(old_transcript_folder_path, &new_transcript_folder_path) {
                    warn!("[Backend Rename] Failed to rename imported transcript folder: {}. Attempting to revert file renames.", e);
                    // Revert metadata rename
                    if let Ok(old_meta_p) = get_document_metadata_path_for_doc(old_transcript_file_path) {
                        if let Ok(new_meta_p_temp) = get_document_metadata_path_for_doc(&new_transcript_file_path_in_old_folder) {
                            if old_meta_p != new_meta_p_temp && new_meta_p_temp.exists() { let _ = fs::rename(&new_meta_p_temp, &old_meta_p); }
                        }
                    }
                    // Revert main transcript rename
                    if old_transcript_file_path != &new_transcript_file_path_in_old_folder && new_transcript_file_path_in_old_folder.exists() {
                        let _ = fs::rename(&new_transcript_file_path_in_old_folder, old_transcript_file_path);
                    }
                    return Err(CommandError::from(format!("Failed to rename imported transcript folder: {}", e)));
                }
                current_transcript_folder_path_for_xml_update = &new_transcript_folder_path;
            }

            // --- XML Update ---
            let final_new_transcript_file_abs_path = current_transcript_folder_path_for_xml_update.join(&new_transcript_filename_pathbuf);
            let new_relative_path_for_transcript_xml = final_new_transcript_file_abs_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\", "/");

            // Standardize or update the .metadata.json file (previously app-specific, now StandardAssetMetadata)
            // Note: old_transcript_file_path is the original absolute path of the transcript before any renames.
            // final_new_transcript_file_abs_path is the new absolute path after all renames.
            let old_asset_metadata_path_result = get_document_metadata_path_for_doc(old_transcript_file_path);
            let new_asset_metadata_path_result = get_document_metadata_path_for_doc(&final_new_transcript_file_abs_path);

            match (old_asset_metadata_path_result, new_asset_metadata_path_result) {
                (Ok(old_asset_meta_path), Ok(new_asset_meta_path)) => {
                    let mut asset_metadata_to_write: Option<StandardAssetMetadata> = None;

                    if old_asset_meta_path.exists() && old_asset_meta_path != new_asset_meta_path {
                        info!("[Backend Rename] Attempting to read old imported transcript asset metadata: {}", old_asset_meta_path.display());
                        match fs::read_to_string(&old_asset_meta_path) {
                            Ok(json_content) => {
                                match serde_json::from_str::<StandardAssetMetadata>(&json_content) {
                                    Ok(mut parsed_meta) => {
                                        parsed_meta.metadata.file_name = new_transcript_filename_with_ext_str.clone();
                                        parsed_meta.metadata.file_path = final_new_transcript_file_abs_path.to_string_lossy().into_owned();
                                        parsed_meta.metadata.last_modified = Utc::now().to_rfc3339();
                                        asset_metadata_to_write = Some(parsed_meta);
                                        info!("[Backend Rename] Parsed and updated old imported transcript asset metadata.");
                                    }
                                    Err(e) => warn!("[Backend Rename] Failed to parse old imported transcript asset metadata as StandardAssetMetadata {}: {}. New one will be created.", old_asset_meta_path.display(), e),
                                }
                            }
                            Err(e) => warn!("[Backend Rename] Failed to read old imported transcript asset metadata {}: {}. New one will be created.", old_asset_meta_path.display(), e),
                        }
                        if old_asset_meta_path != new_asset_meta_path { // Only remove if paths are different
                            if let Err(e) = fs::remove_file(&old_asset_meta_path) {
                                warn!("[Backend Rename] Failed to remove old imported transcript asset metadata {}: {}", old_asset_meta_path.display(), e);
                            }
                        }
                    } else if new_asset_meta_path.exists() { // Handles case where old and new path are same (e.g. case change of main file)
                         info!("[Backend Rename] Attempting to read existing imported transcript asset metadata for in-place update: {}", new_asset_meta_path.display());
                         match fs::read_to_string(&new_asset_meta_path) {
                            Ok(json_content) => {
                                match serde_json::from_str::<StandardAssetMetadata>(&json_content) {
                                    Ok(mut parsed_meta) => {
                                        parsed_meta.metadata.file_name = new_transcript_filename_with_ext_str.clone();
                                        parsed_meta.metadata.file_path = final_new_transcript_file_abs_path.to_string_lossy().into_owned();
                                        parsed_meta.metadata.last_modified = Utc::now().to_rfc3339();
                                        asset_metadata_to_write = Some(parsed_meta);
                                        info!("[Backend Rename] Parsed and updated existing imported transcript asset metadata for in-place update.");
                                    }
                                    Err(e) => warn!("[Backend Rename] Failed to parse existing imported transcript asset metadata as StandardAssetMetadata {}: {}. It will be overwritten.", new_asset_meta_path.display(), e),
                                }
                            }
                            Err(e) => warn!("[Backend Rename] Failed to read existing imported transcript asset metadata {}: {}. It will be overwritten.", new_asset_meta_path.display(), e),
                        }
                    } else {
                         info!("[Backend Rename] Old imported transcript asset metadata {} not found or same as new path. New one will be created/overwritten at {}.", old_asset_meta_path.display(), new_asset_meta_path.display());
                    }

                    let final_asset_metadata = asset_metadata_to_write.unwrap_or_else(|| {
                        StandardAssetMetadata {
                            metadata: FileMetadata {
                                file_name: new_transcript_filename_with_ext_str.clone(),
                                file_path: final_new_transcript_file_abs_path.to_string_lossy().into_owned(),
                                last_modified: Utc::now().to_rfc3339(),
                                title: "".to_string(),
                                description: "".to_string(),
                                summary: "".to_string(),
                            },
                            highlights: Vec::new(),
                        }
                    });

                    match serde_json::to_string_pretty(&final_asset_metadata) {
                        Ok(json_string) => {
                            if let Err(e) = fs::write(&new_asset_meta_path, json_string) {
                                warn!("[Backend Rename] Failed to write imported transcript asset metadata to {}: {}", new_asset_meta_path.display(), e);
                            } else {
                                info!("[Backend Rename] Wrote imported transcript asset metadata to {}", new_asset_meta_path.display());
                            }
                        }
                        Err(e) => warn!("[Backend Rename] Failed to serialize imported transcript asset metadata for {}: {}", new_asset_meta_path.display(), e),
                    }
                }
                (Err(e), _) => warn!("[Backend Rename] Could not determine old imported transcript asset metadata path: {:?}", e),
                (_, Err(e)) => warn!("[Backend Rename] Could not determine new imported transcript asset metadata path: {:?}", e),
            }

            // This is where the XML path for the .<name>.metadata.json (DocumentMetadataEntry) is determined
            let mut new_app_metadata_relative_path_for_xml: Option<String> = None;
            if let Ok(final_new_app_metadata_abs_path) = get_document_metadata_path_for_doc(&final_new_transcript_file_abs_path) {
                if final_new_app_metadata_abs_path.exists() { // Check if it exists after our write attempt
                    new_app_metadata_relative_path_for_xml = Some(final_new_app_metadata_abs_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\","/"));
                }
            }

            info!("[Backend Rename] Updating XML for imported transcript: OldRelPath '{}', NewRelPath '{}', NewName '{}'", item_relative_path, new_relative_path_for_transcript_xml, new_transcript_filename_with_ext_str);
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            let mut updated_xml = false;

            if let Some(entry) = project_data.imported_transcript_files.files.iter_mut().find(|t| t.relative_path == item_relative_path) {
                entry.name = new_transcript_filename_with_ext_str.clone();
                entry.relative_path = new_relative_path_for_transcript_xml.clone();
                project_data.imported_transcript_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                updated_xml = true;
                info!("[Backend Rename] XML imported transcript entry updated.");
            } else {
                warn!("[Backend Rename] Renamed imported transcript and/or folder, but could not find matching old relative path '{}' in XML for main transcript.", item_relative_path);
            }

            // Update associated DocumentMetadataEntryXml
            if let Some(new_rel_meta_path) = new_app_metadata_relative_path_for_xml {
                if let Some(metadata_entry) = project_data.document_metadata_files.files.iter_mut().find(|m| m.original_document_relative_path == item_relative_path) {
                    let new_meta_filename = PathBuf::from(&new_rel_meta_path).file_name().unwrap_or_default().to_string_lossy().to_string();
                    metadata_entry.name = new_meta_filename;
                    metadata_entry.original_document_relative_path = new_relative_path_for_transcript_xml.clone(); // Link to new transcript path
                    metadata_entry.relative_path = new_rel_meta_path; // New path of the metadata file itself
                    project_data.document_metadata_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                    updated_xml = true;
                    info!("[Backend Rename] XML document metadata entry updated for imported transcript.");
                } else {
                    warn!("[Backend Rename] Imported transcript metadata file renamed/moved, but could not find matching old original_document_relative_path '{}' in XML for metadata.", item_relative_path);
                }
            }

            if updated_xml {
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Rename] XML saved for imported transcript rename.");

                let payload = ItemRenamedPayload {
                    old_path: item_path_buf.to_string_lossy().into_owned(),
                    new_path: final_new_transcript_file_abs_path.to_string_lossy().into_owned(),
                    new_name: new_transcript_filename_with_ext_str.clone(),
                    item_type: "imported_transcript".to_string(),
                    project_xml_path: xml_path_buf.to_string_lossy().into_owned(),
                    base_directory: project_base_dir.to_string_lossy().into_owned(),
                };
                if let Err(e) = app_handle.emit("item_renamed", payload) {
                    warn!("[Backend Rename] Failed to emit item_renamed event for imported_transcript: {}", e);
                }
            }

        },
        "doc" => { // Handles .json, .pdf, .md, .txt
            let new_filename_with_ext_str = new_name_trimmed; // e.g., "NewDocName.pdf"
            let new_filename_pathbuf = PathBuf::from(new_filename_with_ext_str);

            if contains_invalid_chars(new_filename_with_ext_str) { return Err(CommandError::from("New filename contains invalid chars.")); }
            let allowed_extensions = ["json", "md", "txt", "pdf"];
            let new_ext = new_filename_pathbuf.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if !allowed_extensions.contains(&new_ext.as_str()) {
                 return Err(CommandError::from(format!("Invalid extension '.{}'. Allowed extensions for documents are: {:?}", new_ext, allowed_extensions)));
            }
            let old_ext = item_path_buf.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
             if old_ext != new_ext {
                  return Err(CommandError::from(format!("Changing document file extension from '.{}' to '.{}' is not allowed.", old_ext, new_ext)));
             }
            if new_filename_with_ext_str.starts_with('.') &&
               !new_filename_with_ext_str.ends_with(METADATA_FILE_SUFFIX) &&
               !new_filename_with_ext_str.ends_with(PDF_ANNOTATIONS_FILE_SUFFIX) {
                return Err(CommandError::from("Document filename cannot start with a dot unless it's a designated metadata or annotation file."));
            }

            // --- Path Definitions ---
            let old_doc_file_path = &item_path_buf; // e.g., .../OldDocStem/OldDocName.pdf
            let old_doc_folder_path = parent_dir;   // e.g., .../OldDocStem/ (parent_dir is item_path_buf.parent())
            
            // Path for the document file *after* filename rename but *before* folder rename
            let new_doc_file_path_in_old_folder = old_doc_folder_path.join(&new_filename_pathbuf); // e.g., .../OldDocStem/NewDocName.pdf
            
            let new_doc_filename_stem = new_filename_pathbuf.file_stem().and_then(|s| s.to_str())
                .ok_or_else(|| CommandError::from(format!("Could not get new document file stem from {}", new_filename_pathbuf.display())))?;

            let documents_root_path = old_doc_folder_path.parent()
                .ok_or_else(|| CommandError::from(format!("Could not get documents root from {}", old_doc_folder_path.display())))?;
            
            let new_doc_folder_path = documents_root_path.join(new_doc_filename_stem); // e.g., .../Documents/NewDocStem/

            // --- Pre-checks for conflicts ---
            if *old_doc_file_path == new_doc_file_path_in_old_folder && old_doc_folder_path == &new_doc_folder_path {
                info!("[Backend Rename] Document name and folder name are effectively unchanged. No action needed.");
                return Ok(());
            }

            // Check if the target *folder* (if different) already exists
            if old_doc_folder_path != &new_doc_folder_path && new_doc_folder_path.exists() {
                return Err(CommandError::from(format!("A folder named '{}' already exists. Cannot rename document folder.", new_doc_filename_stem)));
            }
            
            // Check if the final target *file* would conflict (e.g. .../NewDocStem/NewDocName.pdf)
            let final_target_doc_file_path = new_doc_folder_path.join(&new_filename_pathbuf);
            if final_target_doc_file_path.exists() {
                 let canon_old_abs = fs::canonicalize(old_doc_file_path).map_err(|e| CommandError::from(format!("Cannot canonicalize old path {}: {}", old_doc_file_path.display(),e)))?;
                 let canon_final_target_abs = fs::canonicalize(&final_target_doc_file_path).map_err(|e| CommandError::from(format!("Cannot canonicalize final target path {}: {}", final_target_doc_file_path.display(),e)))?;
                 if canon_final_target_abs != canon_old_abs {
                     return Err(CommandError::from(format!("A file named '{}' already exists in the target location '{}'.", new_filename_with_ext_str, new_doc_folder_path.display())));
                 }
            }

            // --- File System Operations ---
            // 1. Rename main document file (within its current/old folder)
            if old_doc_file_path != &new_doc_file_path_in_old_folder {
                info!("[Backend Rename] Renaming document file {} -> {}", old_doc_file_path.display(), new_doc_file_path_in_old_folder.display());
                fs::rename(old_doc_file_path, &new_doc_file_path_in_old_folder)
                    .map_err(|e| CommandError::from(format!("Failed to rename document file: {}", e)))?;
            }

            // 2. Rename associated app metadata file (within its current/old folder)
            if let Ok(old_app_metadata_path) = get_document_metadata_path_for_doc(old_doc_file_path) { // Use original path to find old metadata
                if old_app_metadata_path.exists() {
                    // New metadata name, still in old folder (based on new_doc_file_path_in_old_folder)
                    if let Ok(new_app_metadata_path_in_old_folder) = get_document_metadata_path_for_doc(&new_doc_file_path_in_old_folder) { 
                        if old_app_metadata_path != new_app_metadata_path_in_old_folder {
                            info!("[Backend Rename] Renaming app metadata: {} -> {}", old_app_metadata_path.display(), new_app_metadata_path_in_old_folder.display());
                            if new_app_metadata_path_in_old_folder.exists() {
                                warn!("[Backend Rename] Target app metadata {} already exists. Skipping rename of {}.", new_app_metadata_path_in_old_folder.display(), old_app_metadata_path.display());
                            } else {
                                if let Err(e) = fs::rename(&old_app_metadata_path, &new_app_metadata_path_in_old_folder) {
                                    warn!("[Backend Rename] Failed to rename app metadata: {}. Attempting to revert main doc rename.", e);
                                    if old_doc_file_path != &new_doc_file_path_in_old_folder { // only revert if it was actually renamed
                                        let _ = fs::rename(&new_doc_file_path_in_old_folder, old_doc_file_path);
                                    }
                                    return Err(CommandError::from(format!("Failed to rename app metadata: {}", e)));
                                }
                            }
                        }
                    }
                }
            }
            
            // 3. Rename PDF annotation file (if PDF) (within its current/old folder)
            if old_ext == "pdf" {
                if let Ok(old_pdf_annot_path) = get_pdf_annotation_file_path(old_doc_file_path) { // Use original path
                    if old_pdf_annot_path.exists() {
                        // New annotation name, still in old folder (based on new_doc_file_path_in_old_folder)
                        if let Ok(new_pdf_annot_path_in_old_folder) = get_pdf_annotation_file_path(&new_doc_file_path_in_old_folder) { 
                            if old_pdf_annot_path != new_pdf_annot_path_in_old_folder {
                                info!("[Backend Rename] Renaming PDF annotation: {} -> {}", old_pdf_annot_path.display(), new_pdf_annot_path_in_old_folder.display());
                                if new_pdf_annot_path_in_old_folder.exists() {
                                     warn!("[Backend Rename] Target PDF annotation {} already exists. Skipping rename of {}.", new_pdf_annot_path_in_old_folder.display(), old_pdf_annot_path.display());
                                } else {
                                    if let Err(e) = fs::rename(&old_pdf_annot_path, &new_pdf_annot_path_in_old_folder) {
                                        warn!("[Backend Rename] Failed to rename PDF annotation: {}. Attempting to revert renames.", e);
                                        // Revert app metadata
                                        if let Ok(old_app_meta_p) = get_document_metadata_path_for_doc(old_doc_file_path) {
                                            if let Ok(new_app_meta_p_temp) = get_document_metadata_path_for_doc(&new_doc_file_path_in_old_folder) {
                                                if old_app_meta_p != new_app_meta_p_temp && new_app_meta_p_temp.exists() { let _ = fs::rename(&new_app_meta_p_temp, &old_app_meta_p); }
                                            }
                                        }
                                        // Revert main doc
                                        if old_doc_file_path != &new_doc_file_path_in_old_folder { let _ = fs::rename(&new_doc_file_path_in_old_folder, old_doc_file_path); }
                                        return Err(CommandError::from(format!("Failed to rename PDF annotation: {}", e)));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 4. Rename the folder if its name (derived from stem) has changed
            let mut current_doc_folder_path_for_xml_update = old_doc_folder_path.clone(); // Start with old, update if renamed
            if old_doc_folder_path != &new_doc_folder_path {
                info!("[Backend Rename] Renaming document folder {} -> {}", old_doc_folder_path.display(), new_doc_folder_path.display());
                if let Err(e) = fs::rename(old_doc_folder_path, &new_doc_folder_path) {
                    warn!("[Backend Rename] Failed to rename document folder: {}. Attempting to revert file renames.", e);
                    // Revert PDF annotation rename (if any)
                    if old_ext == "pdf" {
                        if let Ok(old_pdf_annot_p) = get_pdf_annotation_file_path(old_doc_file_path) {
                           if let Ok(new_pdf_annot_p_temp) = get_pdf_annotation_file_path(&new_doc_file_path_in_old_folder) {
                                if old_pdf_annot_p != new_pdf_annot_p_temp && new_pdf_annot_p_temp.exists() { let _ = fs::rename(&new_pdf_annot_p_temp, &old_pdf_annot_p); }
                           }
                        }
                    }
                    // Revert app metadata rename (if any)
                    if let Ok(old_app_meta_p) = get_document_metadata_path_for_doc(old_doc_file_path) {
                        if let Ok(new_app_meta_p_temp) = get_document_metadata_path_for_doc(&new_doc_file_path_in_old_folder) {
                            if old_app_meta_p != new_app_meta_p_temp && new_app_meta_p_temp.exists() { let _ = fs::rename(&new_app_meta_p_temp, &old_app_meta_p); }
                        }
                    }
                    // Revert main doc rename
                    if old_doc_file_path != &new_doc_file_path_in_old_folder && new_doc_file_path_in_old_folder.exists() {
                        let _ = fs::rename(&new_doc_file_path_in_old_folder, old_doc_file_path);
                    }
                    return Err(CommandError::from(format!("Failed to rename document folder: {}", e)));
                }
                current_doc_folder_path_for_xml_update = &new_doc_folder_path; // Folder was renamed, use new path for XML
            }

            // --- XML Update ---
            // The main document file is now at: current_doc_folder_path_for_xml_update.join(&new_filename_pathbuf)
            let final_new_doc_file_abs_path = current_doc_folder_path_for_xml_update.join(&new_filename_pathbuf);
            let new_relative_path_for_doc = final_new_doc_file_abs_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\", "/");

            let mut new_app_metadata_relative_path_for_xml: Option<String> = None;
            if let Ok(final_new_app_metadata_abs_path) = get_document_metadata_path_for_doc(&final_new_doc_file_abs_path) {
                if final_new_app_metadata_abs_path.exists() {
                     new_app_metadata_relative_path_for_xml = Some(final_new_app_metadata_abs_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\","/"));
                }
            }
            let mut new_pdf_annotation_relative_path_for_xml: Option<String> = None;
            if old_ext == "pdf" {
                if let Ok(final_new_pdf_annot_abs_path) = get_pdf_annotation_file_path(&final_new_doc_file_abs_path) {
                    if final_new_pdf_annot_abs_path.exists() {
                        new_pdf_annotation_relative_path_for_xml = Some(final_new_pdf_annot_abs_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\","/"));
                    }
                }
            }

            info!("[Backend Rename] Updating XML for document: OldRelPath '{}', NewRelPath '{}', NewName '{}'", item_relative_path, new_relative_path_for_doc, new_filename_with_ext_str);
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            let mut updated_xml = false;

            if let Some(doc_entry) = project_data.document_files.files.iter_mut().find(|d| d.relative_path == item_relative_path) {
                doc_entry.name = new_filename_with_ext_str.to_string();
                doc_entry.relative_path = new_relative_path_for_doc.clone();
                updated_xml = true;
                info!("[Backend Rename] XML document entry updated.");
            } else {
                warn!("[Backend Rename] Renamed document, but could not find matching old relative path '{}' in XML for main doc.", item_relative_path);
            }

            // Update app metadata XML entry
            if let Some(new_rel_meta_path) = new_app_metadata_relative_path_for_xml {
                 if let Some(metadata_entry) = project_data.document_metadata_files.files.iter_mut().find(|m| m.original_document_relative_path == item_relative_path) {
                    let new_meta_filename = PathBuf::from(&new_rel_meta_path).file_name().unwrap_or_default().to_string_lossy().to_string();
                    metadata_entry.name = new_meta_filename;
                    metadata_entry.original_document_relative_path = new_relative_path_for_doc.clone();
                    metadata_entry.relative_path = new_rel_meta_path;
                    updated_xml = true;
                    info!("[Backend Rename] XML document app metadata entry updated.");
                } else {
                     warn!("[Backend Rename] App metadata file renamed/moved, but could not find matching old original_document_relative_path '{}' in XML for metadata.", item_relative_path);
                }
            }
            
            // Update PDF annotation XML entry if it's a PDF and was renamed
            if old_ext == "pdf" {
                if let Some(new_rel_annot_path) = new_pdf_annotation_relative_path_for_xml {
                    if let Some(pdf_annot_entry) = project_data.pdf_annotation_files.files.iter_mut().find(|pa| pa.original_document_relative_path == item_relative_path) {
                        let new_pdf_annot_filename = PathBuf::from(&new_rel_annot_path).file_name().unwrap_or_default().to_string_lossy().to_string();
                        pdf_annot_entry.name = new_pdf_annot_filename;
                        pdf_annot_entry.original_document_relative_path = new_relative_path_for_doc.clone();
                        pdf_annot_entry.relative_path = new_rel_annot_path;
                        updated_xml = true;
                        info!("[Backend Rename] XML PDF annotation entry updated.");
                    } else {
                        warn!("[Backend Rename] PDF annotation file renamed/moved, but could not find matching old original_document_relative_path '{}' in XML for PDF annotation.", item_relative_path);
                    }
                }
            }

            if updated_xml {
                project_data.document_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                project_data.document_metadata_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                project_data.pdf_annotation_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Rename] XML saved for document and its associated files.");

                let payload = ItemRenamedPayload {
                    old_path: item_path_buf.to_string_lossy().into_owned(),
                    new_path: final_new_doc_file_abs_path.to_string_lossy().into_owned(),
                    new_name: new_filename_with_ext_str.to_string(),
                    item_type: "doc".to_string(),
                    project_xml_path: xml_path_buf.to_string_lossy().into_owned(),
                    base_directory: project_base_dir.to_string_lossy().into_owned(),
                };
                if let Err(e) = app_handle.emit("item_renamed", payload) {
                    warn!("[Backend Rename] Failed to emit item_renamed event for doc: {}", e);
                }
            }
        },
        "table" => {
            // --- Path Definitions ---
            let old_table_file_abs_path = item_path_buf.clone();
            let old_table_folder_abs_path = parent_dir.to_path_buf(); // parent_dir is item_path_buf.parent().unwrap()

            let old_table_filename_str = old_table_file_abs_path.file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| CommandError::from(format!("Could not get old table filename string from {}", old_table_file_abs_path.display())))?
                .to_string();
            let old_table_stem_str = old_table_file_abs_path.file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| CommandError::from(format!("Could not get old table stem string from {}", old_table_file_abs_path.display())))?
                .to_string();

            let new_table_filename_str = new_name_trimmed.to_string();
            let new_table_filename_pathbuf = PathBuf::from(&new_table_filename_str);
            let new_table_stem_str = new_table_filename_pathbuf.file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| CommandError::from(format!("Could not get new table stem string from {}", new_table_filename_str)))?
                .to_string();

            let tables_root_abs_path = old_table_folder_abs_path.parent()
                .ok_or_else(|| CommandError::from(format!("Could not get tables root directory from {}", old_table_folder_abs_path.display())))?;

            let new_table_folder_abs_path = tables_root_abs_path.join(&new_table_stem_str);
            let final_new_table_file_abs_path = new_table_folder_abs_path.join(&new_table_filename_str);

            let old_asset_metadata_path = get_table_asset_metadata_path(&old_table_file_abs_path)?;
            let new_asset_metadata_path = get_table_asset_metadata_path(&final_new_table_file_abs_path)?;

            // --- Validations ---
            if contains_invalid_chars(&new_table_filename_str) { return Err(CommandError::from("New table filename contains invalid characters.")); }
            let allowed_extensions = ["csv", "xlsx"];
            let new_ext = final_new_table_file_abs_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if !allowed_extensions.contains(&new_ext.as_str()) {
                return Err(CommandError::from(format!("Invalid extension '.{}'. Allowed extensions for tables are: {:?}", new_ext, allowed_extensions)));
            }
            let old_ext = old_table_file_abs_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if old_ext != new_ext {
                return Err(CommandError::from(format!("Changing table file extension from '.{}' to '.{}' is not allowed.", old_ext, new_ext)));
            }
            if new_table_filename_str.starts_with('.') { return Err(CommandError::from("Table filename cannot start with a dot.")); }

            if old_table_file_abs_path == final_new_table_file_abs_path {
                info!("[Backend Rename Table] New table path is the same as the old path. No action needed.");
                return Ok(());
            }

            if old_table_folder_abs_path != new_table_folder_abs_path && new_table_folder_abs_path.exists() {
                return Err(CommandError::from(format!("Target folder '{}' already exists. Cannot rename table folder.", new_table_folder_abs_path.display())));
            }

            if final_new_table_file_abs_path.exists() {
                // Check if it's the same file (e.g. case change on case-insensitive FS)
                // Canonicalize both paths and compare. If they are different, then it's a true conflict.
                let canon_old = fs::canonicalize(&old_table_file_abs_path).ok();
                let canon_target = fs::canonicalize(&final_new_table_file_abs_path).ok();
                if canon_old.is_some() && canon_target.is_some() && canon_old != canon_target {
                     return Err(CommandError::from(format!("Target table file '{}' already exists and is different from the source.", final_new_table_file_abs_path.display())));
                } else if canon_old.is_none() && canon_target.is_some() { // old path might not exist if folder was renamed first, then error occurred
                     return Err(CommandError::from(format!("Target table file '{}' already exists.", final_new_table_file_abs_path.display())));
                }
                 info!("[Backend Rename Table] Target file path {} exists, but might be the same file due to case change or prior operations. Proceeding carefully.", final_new_table_file_abs_path.display());
            }

            // --- File System Operations ---
            if old_table_folder_abs_path != new_table_folder_abs_path {
                // Scenario 1: Folder name changes
                info!("[Backend Rename Table] Renaming folder {} -> {}", old_table_folder_abs_path.display(), new_table_folder_abs_path.display());
                fs::rename(&old_table_folder_abs_path, &new_table_folder_abs_path)
                    .map_err(|e| CommandError::from(format!("Failed to rename table folder: {}", e)))?;

                // The table file is now at new_table_folder_abs_path.join(&old_table_filename_str)
                let current_table_file_path_after_folder_rename = new_table_folder_abs_path.join(&old_table_filename_str);

                if old_table_filename_str != new_table_filename_str {
                    info!("[Backend Rename Table] Renaming table file (post folder rename) {} -> {}", current_table_file_path_after_folder_rename.display(), final_new_table_file_abs_path.display());
                    if let Err(e) = fs::rename(&current_table_file_path_after_folder_rename, &final_new_table_file_abs_path) {
                        warn!("[Backend Rename Table] Failed to rename table file after folder rename: {}. Reverting folder rename.", e);
                        let _ = fs::rename(&new_table_folder_abs_path, &old_table_folder_abs_path); // Attempt to revert folder rename
                        return Err(CommandError::from(format!("Failed to rename table file after folder rename: {}", e)));
                    }
                }

                let current_old_metadata_filename = format!(".{}.metadata.json", old_table_stem_str);
                let current_old_metadata_path_after_folder_rename = new_table_folder_abs_path.join(current_old_metadata_filename);

                if current_old_metadata_path_after_folder_rename.exists() {
                    info!("[Backend Rename Table] Reading old metadata from moved location: {}", current_old_metadata_path_after_folder_rename.display());
                    let old_json_content = match fs::read_to_string(&current_old_metadata_path_after_folder_rename) {
                        Ok(content) => content,
                        Err(e) => {
                            warn!("[Backend Rename Table] Failed to read old metadata (post folder rename): {}. Reverting operations.", e);
                            if old_table_filename_str != new_table_filename_str { // Revert file rename if done
                                let _ = fs::rename(&final_new_table_file_abs_path, &current_table_file_path_after_folder_rename);
                            }
                            let _ = fs::rename(&new_table_folder_abs_path, &old_table_folder_abs_path); // Revert folder rename
                            return Err(CommandError::from(format!("Failed to read old metadata after folder rename: {}", e)));
                        }
                    };

                    let mut parsed_metadata: StandardAssetMetadata = match serde_json::from_str(&old_json_content) {
                        Ok(meta) => meta,
                        Err(e) => {
                             warn!("[Backend Rename Table] Failed to parse old metadata (post folder rename): {}. Reverting operations.", e);
                            if old_table_filename_str != new_table_filename_str { // Revert file rename if done
                                let _ = fs::rename(&final_new_table_file_abs_path, &current_table_file_path_after_folder_rename);
                            }
                            let _ = fs::rename(&new_table_folder_abs_path, &old_table_folder_abs_path); // Revert folder rename
                            return Err(CommandError::from(format!("Failed to parse old metadata: {}", e)));
                        }
                    };

                    parsed_metadata.metadata.file_name = new_table_filename_str.clone();
                    parsed_metadata.metadata.file_path = final_new_table_file_abs_path.to_string_lossy().into_owned();
                    parsed_metadata.metadata.last_modified = Utc::now().to_rfc3339();

                    let updated_json_string = match serde_json::to_string_pretty(&parsed_metadata) {
                        Ok(s) => s,
                        Err(e) => {
                            warn!("[Backend Rename Table] Failed to serialize updated metadata: {}. Reverting operations.", e);
                            if old_table_filename_str != new_table_filename_str { let _ = fs::rename(&final_new_table_file_abs_path, &current_table_file_path_after_folder_rename); }
                            let _ = fs::rename(&new_table_folder_abs_path, &old_table_folder_abs_path);
                            return Err(CommandError::from(format!("Failed to serialize updated metadata: {}", e)));
                        }
                    };

                    info!("[Backend Rename Table] Writing updated metadata to {}", new_asset_metadata_path.display());
                    if let Err(e) = fs::write(&new_asset_metadata_path, updated_json_string) {
                        warn!("[Backend Rename Table] Failed to write new metadata: {}. Reverting operations.", e);
                         if old_table_filename_str != new_table_filename_str { let _ = fs::rename(&final_new_table_file_abs_path, &current_table_file_path_after_folder_rename); }
                         let _ = fs::rename(&new_table_folder_abs_path, &old_table_folder_abs_path);
                        return Err(CommandError::from(format!("Failed to write new metadata: {}", e)));
                    }

                    if current_old_metadata_path_after_folder_rename != new_asset_metadata_path {
                        info!("[Backend Rename Table] Removing old metadata from moved location: {}", current_old_metadata_path_after_folder_rename.display());
                        if let Err(e) = fs::remove_file(&current_old_metadata_path_after_folder_rename) {
                            warn!("[Backend Rename Table] Failed to remove old metadata file (post folder rename) {}: {}", current_old_metadata_path_after_folder_rename.display(), e);
                        }
                    }
                } else if !new_asset_metadata_path.exists() { // Old metadata (even after folder move) does not exist, AND new one also doesn't. Create new.
                    info!("[Backend Rename Table] Old metadata not found at {}. Creating new metadata at {}.", current_old_metadata_path_after_folder_rename.display(), new_asset_metadata_path.display());
                    let default_metadata = StandardAssetMetadata {
                        metadata: FileMetadata {
                            file_name: new_table_filename_str.clone(),
                            file_path: final_new_table_file_abs_path.to_string_lossy().into_owned(),
                            last_modified: Utc::now().to_rfc3339(),
                            title: "".to_string(), description: "".to_string(), summary: "".to_string(),
                        },
                        highlights: Vec::new(),
                    };
                    let json_string = serde_json::to_string_pretty(&default_metadata)
                        .map_err(|e| CommandError::from(format!("Failed to serialize new default metadata: {}", e)))?; // Basic error, no revert needed yet for this specific failure
                    fs::write(&new_asset_metadata_path, json_string)
                        .map_err(|e| CommandError::from(format!("Failed to write new default metadata to {}: {}", new_asset_metadata_path.display(), e)))?;
                }

            } else {
                // Scenario 2: Folder name does NOT change (so filename must be different, checked by initial validation)
                info!("[Backend Rename Table] Renaming table file (folder same) {} -> {}", old_table_file_abs_path.display(), final_new_table_file_abs_path.display());
                fs::rename(&old_table_file_abs_path, &final_new_table_file_abs_path)
                    .map_err(|e| CommandError::from(format!("Failed to rename table file (folder same): {}", e)))?;

                if old_asset_metadata_path.exists() {
                    info!("[Backend Rename Table] Reading old metadata from original location: {}", old_asset_metadata_path.display());
                    let old_json_content = match fs::read_to_string(&old_asset_metadata_path) {
                        Ok(content) => content,
                        Err(e) => {
                            warn!("[Backend Rename Table] Failed to read old metadata: {}. Reverting table file rename.", e);
                            let _ = fs::rename(&final_new_table_file_abs_path, &old_table_file_abs_path);
                            return Err(CommandError::from(format!("Failed to read old metadata: {}", e)));
                        }
                    };
                    let mut parsed_metadata: StandardAssetMetadata = match serde_json::from_str(&old_json_content) {
                         Ok(meta) => meta,
                         Err(e) => {
                            warn!("[Backend Rename Table] Failed to parse old metadata: {}. Reverting table file rename.", e);
                            let _ = fs::rename(&final_new_table_file_abs_path, &old_table_file_abs_path);
                            return Err(CommandError::from(format!("Failed to parse old metadata: {}", e)));
                         }
                    };

                    parsed_metadata.metadata.file_name = new_table_filename_str.clone();
                    parsed_metadata.metadata.file_path = final_new_table_file_abs_path.to_string_lossy().into_owned();
                    parsed_metadata.metadata.last_modified = Utc::now().to_rfc3339();

                    let updated_json_string = match serde_json::to_string_pretty(&parsed_metadata) {
                        Ok(s) => s,
                        Err(e) => {
                            warn!("[Backend Rename Table] Failed to serialize updated metadata: {}. Reverting table file rename.", e);
                            let _ = fs::rename(&final_new_table_file_abs_path, &old_table_file_abs_path);
                            return Err(CommandError::from(format!("Failed to serialize updated metadata: {}", e)));
                        }
                    };

                    info!("[Backend Rename Table] Writing updated metadata to {}", new_asset_metadata_path.display());
                    if let Err(e) = fs::write(&new_asset_metadata_path, updated_json_string) {
                        warn!("[Backend Rename Table] Failed to write new metadata: {}. Reverting table file rename.", e);
                        let _ = fs::rename(&final_new_table_file_abs_path, &old_table_file_abs_path);
                        return Err(CommandError::from(format!("Failed to write new metadata: {}", e)));
                    }

                    if old_asset_metadata_path != new_asset_metadata_path { // Only remove if paths are different (e.g. stem changed)
                        info!("[Backend Rename Table] Removing old metadata from original location: {}", old_asset_metadata_path.display());
                        if let Err(e) = fs::remove_file(&old_asset_metadata_path) {
                            warn!("[Backend Rename Table] Failed to remove old metadata file {}: {}", old_asset_metadata_path.display(), e);
                        }
                    }
                } else if !new_asset_metadata_path.exists() { // Old metadata does not exist, and new one also doesn't. Create new.
                    info!("[Backend Rename Table] Old metadata not found at {}. Creating new metadata at {}.", old_asset_metadata_path.display(), new_asset_metadata_path.display());
                     let default_metadata = StandardAssetMetadata {
                        metadata: FileMetadata {
                            file_name: new_table_filename_str.clone(),
                            file_path: final_new_table_file_abs_path.to_string_lossy().into_owned(),
                            last_modified: Utc::now().to_rfc3339(),
                            title: "".to_string(), description: "".to_string(), summary: "".to_string(),
                        },
                        highlights: Vec::new(),
                    };
                    let json_string = serde_json::to_string_pretty(&default_metadata)
                        .map_err(|e| CommandError::from(format!("Failed to serialize new default metadata: {}", e)))?;
                    fs::write(&new_asset_metadata_path, json_string)
                        .map_err(|e| CommandError::from(format!("Failed to write new default metadata to {}: {}", new_asset_metadata_path.display(), e)))?;
                }
            }

            // --- XML Update ---
            let new_relative_path_for_xml = final_new_table_file_abs_path.strip_prefix(project_base_dir)?
                .to_string_lossy().replace("\\", "/");

            info!("[Backend Rename Table] Updating XML: OldRelPath '{}', NewRelPath '{}', NewName '{}'", item_relative_path, new_relative_path_for_xml, new_table_filename_str);
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            let mut updated_xml = false;

            if let Some(table_entry) = project_data.table_files.files.iter_mut().find(|t| t.relative_path == item_relative_path) {
                table_entry.name = new_table_filename_str.clone();
                table_entry.relative_path = new_relative_path_for_xml;
                project_data.table_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                updated_xml = true;
                info!("[Backend Rename Table] XML table entry updated.");
            } else {
                // This is a more critical error after file operations have succeeded.
                // Attempting complex rollbacks here could be risky. Log an error.
                error!("[Backend Rename Table] CRITICAL: File system operations for table rename succeeded, but could not find matching old relative path '{}' in XML. Project XML might be inconsistent.", item_relative_path);
                // Depending on desired behavior, could return an error here to signal inconsistency.
                // For now, we'll save if other changes were made, but this is a problem.
                 return Err(CommandError::from(format!("Failed to update XML as old table entry for {} was not found after file operations. Project state may be inconsistent.", item_relative_path)));
            }

            if updated_xml {
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Rename Table] XML saved.");

                let payload = ItemRenamedPayload {
                    old_path: item_path_buf.to_string_lossy().into_owned(),
                    new_path: final_new_table_file_abs_path.to_string_lossy().into_owned(),
                    new_name: new_table_filename_str.clone(),
                    item_type: "table".to_string(),
                    project_xml_path: xml_path_buf.to_string_lossy().into_owned(),
                    base_directory: project_base_dir.to_string_lossy().into_owned(),
                };
                if let Err(e) = app_handle.emit("item_renamed", payload) {
                    warn!("[Backend Rename] Failed to emit item_renamed event for table: {}", e);
                }
            }
        },
        "image" => {
            let new_image_filename_with_ext_str = new_name_trimmed; // e.g., "NewImage.png"
            let new_image_filename_pathbuf = PathBuf::from(new_image_filename_with_ext_str);

            if contains_invalid_chars(new_image_filename_with_ext_str) { return Err(CommandError::from("New image filename contains invalid characters.")); }
            let allowed_extensions = ["jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff"];
            let new_ext = new_image_filename_pathbuf.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if !allowed_extensions.contains(&new_ext.as_str()) {
                return Err(CommandError::from(format!("Invalid extension '.{}'. Allowed extensions for images are: {:?}", new_ext, allowed_extensions)));
            }
            let old_ext = item_path_buf.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if old_ext != new_ext {
                return Err(CommandError::from(format!("Changing image file extension from '.{}' to '.{}' is not allowed.", old_ext, new_ext)));
            }
            if new_image_filename_with_ext_str.starts_with('.') { return Err(CommandError::from("Image filename cannot start with a dot.")); }

            // --- Path Definitions ---
            let old_image_file_abs_path = item_path_buf.clone(); // e.g., .../OldImageStem/OldImage.png
            let old_image_folder_abs_path = parent_dir.to_path_buf();   // e.g., .../OldImageStem/

            #[allow(unused_variables)] // old_image_stem_str might be unused if no metadata/annotation exists
            let old_image_stem_str = old_image_file_abs_path.file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| CommandError::from(format!("Could not get old image stem from {}", old_image_file_abs_path.display())))?
                .to_string();

            // new_image_filename_with_ext_str is new_name_trimmed
            // new_image_filename_pathbuf is PathBuf::from(new_image_filename_with_ext_str)
            let new_image_stem_str = new_image_filename_pathbuf.file_stem().and_then(|s| s.to_str())
                .ok_or_else(|| CommandError::from(format!("Could not get new image file stem from {}", new_image_filename_pathbuf.display())))?
                .to_string();

            let images_root_abs_path = old_image_folder_abs_path.parent()
                .ok_or_else(|| CommandError::from(format!("Could not get images root from {}", old_image_folder_abs_path.display())))?;

            let new_image_folder_abs_path = images_root_abs_path.join(&new_image_stem_str); // e.g., .../Images/NewImageStem/
            let final_new_image_file_abs_path = new_image_folder_abs_path.join(&new_image_filename_pathbuf); // e.g., .../NewImageStem/NewImage.png
            
            // Path for the image file *after* filename rename but *before* potential folder rename
            let new_image_file_path_in_old_folder = old_image_folder_abs_path.join(&new_image_filename_pathbuf);

            // Asset Metadata Paths
            let old_asset_metadata_abs_path = get_image_asset_metadata_path(&old_image_file_abs_path)?;
            let new_asset_metadata_abs_path = get_image_asset_metadata_path(&final_new_image_file_abs_path)?;

            // --- Pre-checks for conflicts ---
            if old_image_file_abs_path == final_new_image_file_abs_path { // Covers case where only folder might change due to stem, but filename itself is same.
                 // Also covers case where item_path_buf (old_image_file_abs_path) is identical to final_new_image_file_abs_path
                if old_image_folder_abs_path == new_image_folder_abs_path {
                    info!("[Backend Rename] Image name and folder name are effectively unchanged. No action needed.");
                    return Ok(());
                }
            }

            if old_image_folder_abs_path != new_image_folder_abs_path && new_image_folder_abs_path.exists() {
                return Err(CommandError::from(format!("A folder named '{}' already exists for images. Cannot rename image folder.", new_image_stem_str)));
            }

            if final_new_image_file_abs_path.exists() {
                let canon_old_abs = fs::canonicalize(&old_image_file_abs_path).map_err(|e| CommandError::from(format!("Cannot canonicalize old image path {}: {}", old_image_file_abs_path.display(), e)))?;
                let canon_final_target_abs = fs::canonicalize(&final_new_image_file_abs_path).map_err(|e| CommandError::from(format!("Cannot canonicalize final target image path {}: {}", final_new_image_file_abs_path.display(), e)))?;
                if canon_final_target_abs != canon_old_abs {
                    return Err(CommandError::from(format!("An image file named '{}' already exists in the target location '{}'.", new_image_filename_with_ext_str, new_image_folder_abs_path.display())));
                }
            }

            // --- Read Old Metadata (Before Any Renames) ---
            let mut parsed_old_metadata_content: Option<StandardAssetMetadata> = None;
            if old_asset_metadata_abs_path.exists() {
                info!("[Backend Rename Image] Attempting to read old asset metadata from: {}", old_asset_metadata_abs_path.display());
                match fs::read_to_string(&old_asset_metadata_abs_path) {
                    Ok(json_content) => {
                        match serde_json::from_str::<StandardAssetMetadata>(&json_content) {
                            Ok(parsed_meta) => {
                                parsed_old_metadata_content = Some(parsed_meta);
                                info!("[Backend Rename Image] Successfully parsed old asset metadata.");
                            }
                            Err(e) => {
                                warn!("[Backend Rename Image] Failed to parse old asset metadata from {}: {}. Will create new metadata if needed.", old_asset_metadata_abs_path.display(), e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("[Backend Rename Image] Failed to read old asset metadata from {}: {}. Will create new metadata if needed.", old_asset_metadata_abs_path.display(), e);
                    }
                }
            }


            // --- File System Operations ---
            // 1. Rename main image file (within its current/old folder first if filename changes)
            // This new_image_file_path_in_old_folder is (.../OldImageStem/NewImage.png)
            if old_image_file_abs_path != new_image_file_path_in_old_folder {
                info!("[Backend Rename Image] Renaming image file {} -> {}", old_image_file_abs_path.display(), new_image_file_path_in_old_folder.display());
                fs::rename(&old_image_file_abs_path, &new_image_file_path_in_old_folder)
                    .map_err(|e| CommandError::from(format!("Failed to rename image file (pre-folder op): {}", e)))?;
            }

            // 2. Rename associated annotation file (within its current/old folder)
            // old_image_file_abs_path is the original path to the image (e.g. .../OldStem/OldImage.png)
            // new_image_file_path_in_old_folder is the path if image is renamed but still in old folder (e.g. .../OldStem/NewImage.png)
            if let Ok(old_annotation_path) = get_annotation_metadata_path_for_image(&old_image_file_abs_path) {
                if old_annotation_path.exists() {
                    // new_annotation_path_in_old_folder is path for annotation if image renamed in old folder. (e.g. .../OldStem/.NewImage.annotations.json)
                    if let Ok(new_annotation_path_in_old_folder) = get_annotation_metadata_path_for_image(&new_image_file_path_in_old_folder) {
                        if old_annotation_path != new_annotation_path_in_old_folder { // Check if annotation name needs to change
                            info!("[Backend Rename Image] Renaming image annotation (pre-folder op): {} -> {}", old_annotation_path.display(), new_annotation_path_in_old_folder.display());
                            if new_annotation_path_in_old_folder.exists() { // Conflict for annotation file
                                warn!("[Backend Rename Image] Target image annotation {} already exists. Skipping rename of {}.", new_annotation_path_in_old_folder.display(), old_annotation_path.display());
                            } else {
                                if let Err(e) = fs::rename(&old_annotation_path, &new_annotation_path_in_old_folder) {
                                    warn!("[Backend Rename Image] Failed to rename image annotation: {}. Attempting to revert main image rename (pre-folder op).", e);
                                    if old_image_file_abs_path != new_image_file_path_in_old_folder { // only revert if it was actually renamed
                                        let _ = fs::rename(&new_image_file_path_in_old_folder, &old_image_file_abs_path);
                                    }
                                    return Err(CommandError::from(format!("Failed to rename image annotation file: {}", e)));
                                }
                            }
                        }
                    }
                }
            }

            let mut folder_renamed = false;
            // 3. Rename the main image folder if its name (derived from stem) has changed
            if old_image_folder_abs_path != new_image_folder_abs_path {
                info!("[Backend Rename Image] Renaming image folder {} -> {}", old_image_folder_abs_path.display(), new_image_folder_abs_path.display());
                if let Err(e) = fs::rename(&old_image_folder_abs_path, &new_image_folder_abs_path) {
                    warn!("[Backend Rename Image] Failed to rename image folder: {}. Attempting to revert file renames.", e);
                    // Revert annotation rename (if any)
                    if let Ok(old_annot_p) = get_annotation_metadata_path_for_image(&old_image_file_abs_path) {
                        if let Ok(new_annot_p_temp) = get_annotation_metadata_path_for_image(&new_image_file_path_in_old_folder) {
                            if old_annot_p != new_annot_p_temp && new_annot_p_temp.exists() { // if annotation was renamed
                                 // new_annot_p_temp would be something like .../OldStem/.NewImageStem.annotations.json
                                 // old_annot_p would be .../OldStem/.OldImageStem.annotations.json
                                let _ = fs::rename(&new_annot_p_temp, &old_annot_p);
                            }
                        }
                    }
                    // Revert main image rename (if it was done within the old folder)
                    if old_image_file_abs_path != new_image_file_path_in_old_folder && new_image_file_path_in_old_folder.exists() {
                        let _ = fs::rename(&new_image_file_path_in_old_folder, &old_image_file_abs_path);
                    }
                    return Err(CommandError::from(format!("Failed to rename image folder: {}", e)));
                }
                folder_renamed = true;
            }

            // 4. If folder name changed, the image file (which might have been renamed, e.g. OldImage.png -> NewImage.png)
            // is now in new_image_folder_abs_path but with its *original filename* if only stem changed folder,
            // or its *new filename* if filename also changed.
            // It should be `final_new_image_file_abs_path`.
            // Current path of image file:
            // - If folder renamed: new_image_folder_abs_path.join(new_image_file_path_in_old_folder.file_name().unwrap())
            // - If folder NOT renamed: new_image_file_path_in_old_folder
            let current_image_path_before_final_rename = if folder_renamed {
                new_image_folder_abs_path.join(new_image_file_path_in_old_folder.file_name().unwrap_or_default())
            } else {
                new_image_file_path_in_old_folder.clone() // Already .../OldStem/NewImage.png if name changed
            };

            if current_image_path_before_final_rename != final_new_image_file_abs_path && current_image_path_before_final_rename.exists() {
                 info!("[Backend Rename Image] Renaming image file (post-folder op) {} -> {}", current_image_path_before_final_rename.display(), final_new_image_file_abs_path.display());
                 if let Err(e) = fs::rename(&current_image_path_before_final_rename, &final_new_image_file_abs_path) {
                    warn!("[Backend Rename Image] Failed to rename image file to final path: {}. Attempting to revert operations.", e);
                    // Complex revert: folder, then annotations, then initial image rename
                    if folder_renamed {
                        let _ = fs::rename(&new_image_folder_abs_path, &old_image_folder_abs_path);
                        // After folder revert, annotation and image file are back in old_image_folder_abs_path
                        // Revert annotation rename (e.g. from .../OldStem/.NewImage.annotations.json back to .../OldStem/.OldImage.annotations.json)
                        if let Ok(old_annot_p_orig) = get_annotation_metadata_path_for_image(&old_image_file_abs_path) { // .../OldStem/.OldImage.annotations.json
                           if let Ok(new_annot_p_in_old_folder_orig) = get_annotation_metadata_path_for_image(&new_image_file_path_in_old_folder) { // .../OldStem/.NewImage.annotations.json
                                if new_annot_p_in_old_folder_orig.exists() && old_annot_p_orig != new_annot_p_in_old_folder_orig {
                                    let _ = fs::rename(&new_annot_p_in_old_folder_orig, &old_annot_p_orig);
                                }
                           }
                        }
                    }
                     // Revert initial image rename (e.g. from .../OldStem/NewImage.png back to .../OldStem/OldImage.png)
                    if old_image_file_abs_path != new_image_file_path_in_old_folder {
                        let path_to_revert_from = if folder_renamed { &new_image_file_path_in_old_folder } else { &current_image_path_before_final_rename };
                        if path_to_revert_from.exists() {
                             let _ = fs::rename(path_to_revert_from, &old_image_file_abs_path);
                        }
                    }
                    return Err(CommandError::from(format!("Failed to rename image file to final path: {}", e)));
                 }
            }


            // --- Prepare and Write New/Updated Metadata (After All Other Renames) ---
            let final_metadata_to_write: StandardAssetMetadata;
            if let Some(mut metadata) = parsed_old_metadata_content.take() { // Use .take() to get ownership
                info!("[Backend Rename Image] Updating existing asset metadata for {}", new_asset_metadata_abs_path.display());
                metadata.metadata.file_name = new_image_filename_with_ext_str.to_string();
                metadata.metadata.file_path = final_new_image_file_abs_path.to_string_lossy().into_owned();
                metadata.metadata.last_modified = Utc::now().to_rfc3339();
                final_metadata_to_write = metadata;

                // Cleanup old metadata file
                if old_asset_metadata_abs_path != new_asset_metadata_abs_path {
                    let path_of_old_meta_to_remove = if folder_renamed {
                        // If folder was renamed, the original old_asset_metadata_abs_path is stale.
                        // The file would have moved with the folder.
                        new_image_folder_abs_path.join(old_asset_metadata_abs_path.file_name().unwrap_or_default())
                    } else {
                        old_asset_metadata_abs_path.clone()
                    };
                    if path_of_old_meta_to_remove.exists() && path_of_old_meta_to_remove != new_asset_metadata_abs_path {
                        info!("[Backend Rename Image] Removing old/moved asset metadata file from: {}", path_of_old_meta_to_remove.display());
                        if let Err(e) = fs::remove_file(&path_of_old_meta_to_remove) {
                            warn!("[Backend Rename Image] Failed to remove old/moved asset metadata {}: {}", path_of_old_meta_to_remove.display(), e);
                        }
                    } else if !path_of_old_meta_to_remove.exists() && old_asset_metadata_abs_path.exists() && old_asset_metadata_abs_path != new_asset_metadata_abs_path {
                        // This case handles if folder was NOT renamed, but stem changed, so old metadata file is at original path.
                        info!("[Backend Rename Image] Removing original asset metadata file from: {}", old_asset_metadata_abs_path.display());
                         if let Err(e) = fs::remove_file(&old_asset_metadata_abs_path) {
                            warn!("[Backend Rename Image] Failed to remove original asset metadata {}: {}", old_asset_metadata_abs_path.display(), e);
                        }
                    }
                }
            } else {
                info!("[Backend Rename Image] Creating new default asset metadata for {}", new_asset_metadata_abs_path.display());
                final_metadata_to_write = StandardAssetMetadata {
                    metadata: FileMetadata {
                        file_name: new_image_filename_with_ext_str.to_string(),
                        file_path: final_new_image_file_abs_path.to_string_lossy().into_owned(),
                        last_modified: Utc::now().to_rfc3339(),
                        title: "".to_string(),
                        description: "".to_string(),
                        summary: "".to_string(),
                    },
                    highlights: Vec::new(),
                };
            }

            match serde_json::to_string_pretty(&final_metadata_to_write) {
                Ok(json_string) => {
                    info!("[Backend Rename Image] Writing asset metadata to {}", new_asset_metadata_abs_path.display());
                    if let Err(e) = fs::write(&new_asset_metadata_abs_path, json_string) {
                        warn!("[Backend Rename Image] Failed to write asset metadata to {}: {}. Attempting full rollback.", new_asset_metadata_abs_path.display(), e);
                        // Attempt to roll back: final image file, folder, annotation, initial image file
                        if final_new_image_file_abs_path.exists() && final_new_image_file_abs_path != current_image_path_before_final_rename { // Revert final image rename
                           let _ = fs::rename(&final_new_image_file_abs_path, &current_image_path_before_final_rename);
                        }
                        if folder_renamed { // Revert folder rename
                            let _ = fs::rename(&new_image_folder_abs_path, &old_image_folder_abs_path);
                             // After folder revert, annotation and image file are back in old_image_folder_abs_path
                            // Revert annotation rename (e.g. from .../OldStem/.NewImage.annotations.json back to .../OldStem/.OldImage.annotations.json)
                            if let Ok(old_annot_p_orig) = get_annotation_metadata_path_for_image(&old_image_file_abs_path) {
                               if let Ok(new_annot_p_in_old_folder_orig) = get_annotation_metadata_path_for_image(&new_image_file_path_in_old_folder) {
                                    if new_annot_p_in_old_folder_orig.exists() && old_annot_p_orig != new_annot_p_in_old_folder_orig {
                                        let _ = fs::rename(&new_annot_p_in_old_folder_orig, &old_annot_p_orig);
                                    }
                               }
                            }
                        }
                        // Revert initial image rename (e.g. from .../OldStem/NewImage.png back to .../OldStem/OldImage.png)
                        if old_image_file_abs_path != new_image_file_path_in_old_folder {
                             let path_to_revert_from = if folder_renamed { old_image_folder_abs_path.join(new_image_file_path_in_old_folder.file_name().unwrap_or_default()) } else { new_image_file_path_in_old_folder.clone() };
                             if path_to_revert_from.exists() {
                                 let _ = fs::rename(path_to_revert_from, &old_image_file_abs_path);
                             }
                        }
                        return Err(CommandError::from(format!("Failed to write asset metadata: {}", e)));
                    }
                }
                Err(e) => {
                     warn!("[Backend Rename Image] Failed to serialize asset metadata for {}: {}", new_asset_metadata_abs_path.display(), e);
                    // No file system changes made yet for metadata, so no specific rollback for this, but it's an error.
                    return Err(CommandError::from(format!("Failed to serialize asset metadata: {}",e)));
                }
            }

            // --- XML Update ---
            let new_relative_path_for_image_xml = final_new_image_file_abs_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\", "/");
            info!("[Backend Rename Image] Updating XML for image: OldRelPath '{}', NewRelPath '{}', NewName '{}'", item_relative_path, new_relative_path_for_image_xml, new_image_filename_with_ext_str);
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            let mut updated_xml = false;

            if let Some(image_entry) = project_data.image_files.files.iter_mut().find(|i| i.relative_path == item_relative_path) {
                image_entry.name = new_image_filename_with_ext_str.to_string();
                image_entry.relative_path = new_relative_path_for_image_xml;
                project_data.image_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                updated_xml = true;
                info!("[Backend Rename Image] XML image entry updated.");
            } else {
                 error!("[Backend Rename Image] CRITICAL: File system operations for image rename succeeded, but could not find matching old relative path '{}' in XML. Project XML might be inconsistent.", item_relative_path);
                 return Err(CommandError::from(format!("Failed to update XML as old image entry for {} was not found after file operations. Project state may be inconsistent.", item_relative_path)));
            }

            if updated_xml {
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Rename Image] XML saved for image rename.");

                let payload = ItemRenamedPayload {
                    old_path: item_path_buf.to_string_lossy().into_owned(),
                    new_path: final_new_image_file_abs_path.to_string_lossy().into_owned(),
                    new_name: new_image_filename_with_ext_str.to_string(),
                    item_type: "image".to_string(),
                    project_xml_path: xml_path_buf.to_string_lossy().into_owned(),
                    base_directory: project_base_dir.to_string_lossy().into_owned(),
                };
                if let Err(e) = app_handle.emit("item_renamed", payload) {
                    warn!("[Backend Rename] Failed to emit item_renamed event for image: {}", e);
                }
            }
        },
        _ => {
            error!("[Backend Rename] Renaming items of type '{}' is not supported directly: {}", item_type, item_path);
            return Err(CommandError::from(format!("Renaming not supported for item type '{}'. Rename the primary associated asset.", item_type)));
        }
    }

    info!("[Backend Rename] Success for: {}", item_path);
    Ok(())
}
