// src-tauri/src/projectview/core_commands.rs
use super::shared_types::{*, TABLES_DIR, IMAGES_DIR};
use super::shared_utils::*;
use crate::welcome::config::CommandError;
use log::{debug, error, info, warn};
use std::{
    fs,
    path::{Path, PathBuf},
};
use quick_xml;
use super::pdf_annotation_handler::get_pdf_annotation_file_path; // ADDED for delete/rename


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
            info!("[Backend Delete] Deleting standalone image file: {}", item_path_buf.display());
            // 1. Delete the image file itself
            fs::remove_file(&item_path_buf)
                .map_err(|e| CommandError::from(format!("Failed to delete image file {}: {}", item_path_buf.display(), e)))?;

            // 2. Delete containing folder if empty
            if let Some(folder) = item_path_buf.parent() {
                if folder.exists() {
                    match fs::remove_dir(folder) {
                        Ok(_) => (),
                        Err(err) if err.kind() == std::io::ErrorKind::DirectoryNotEmpty => (),
                        Err(err) => return Err(CommandError::from(format!("Failed to delete image folder: {}", err))),
                    }
                }
            }

            // 3. Delete annotation metadata file, if present
            if let Ok(metadata_path) = get_annotation_metadata_path_for_image(&item_path_buf) {
                if metadata_path.exists() {
                    info!("[Backend Delete] Deleting image annotation metadata file: {}", metadata_path.display());
                    if let Err(e) = fs::remove_file(&metadata_path) {
                        warn!("[Backend Delete] Failed to delete image annotation metadata file {}: {}", metadata_path.display(), e);
                    }
                }
            }

            // 4. Update project XML to remove image entry
            info!("[Backend Delete] Updating XML to remove image entry '{}'", item_relative_path);
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            let initial_len = project_data.image_files.files.len();
            project_data.image_files.files.retain(|i| i.relative_path != item_relative_path);

            if project_data.image_files.files.len() < initial_len {
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Delete] XML updated for image.");
            } else {
                warn!("[Backend Delete] Deleted image file, but no matching entry found in XML for path '{}'.", item_relative_path);
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
pub async fn rename_project_item( item_path: String, new_name: String, project_xml_path: String) -> Result<(), CommandError> {
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
                entry.relative_path = primary_media_new_relative_path;

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
            }
        },
        "imported_transcript" => {
            if contains_invalid_chars(new_name_trimmed) { return Err(CommandError::from("New transcript name contains invalid characters.")); }
            if new_name_trimmed.starts_with('.') { return Err(CommandError::from("Transcript name cannot start with a dot.")); }

            let final_new_filename_with_ext = format!("{}.json", new_name_trimmed);
            let new_path = parent_dir.join(&final_new_filename_with_ext);

            if item_path_buf == new_path { info!("[Backend Rename] New path for imported transcript is same. No action needed."); return Ok(()); }

            if new_path.exists() {
                 let canon_old = fs::canonicalize(&item_path_buf).ok();
                 let canon_new = fs::canonicalize(&new_path).ok();
                 if canon_old.is_some() && canon_new.is_some() && canon_old != canon_new {
                     return Err(CommandError::from(format!("File named '{}' already exists in Transcripts folder.", final_new_filename_with_ext)));
                 }
            }

            let old_metadata_path_result = get_document_metadata_path_for_doc(&item_path_buf); 


            info!("[Backend Rename] Renaming imported transcript file {} -> {}", item_path_buf.display(), new_path.display());
            fs::rename(&item_path_buf, &new_path).map_err(|e| CommandError::from(format!("Failed to rename imported transcript file: {}", e)))?;

            if let Ok(old_metadata_path) = old_metadata_path_result {
                if old_metadata_path.exists() {
                    match get_document_metadata_path_for_doc(&new_path) { 
                        Ok(new_metadata_path) => {
                            info!("[Backend Rename] Renaming metadata for imported transcript: {} -> {}", old_metadata_path.display(), new_metadata_path.display());
                             if new_metadata_path.exists() && new_metadata_path != old_metadata_path {
                                warn!("[Backend Rename] Target metadata file {} already exists. Skipping rename of old metadata {}.", new_metadata_path.display(), old_metadata_path.display());
                            } else if old_metadata_path != new_metadata_path {
                                if let Err(e) = fs::rename(&old_metadata_path, &new_metadata_path) {
                                    warn!("[Backend Rename] Failed to rename metadata for imported transcript: {}", e);
                                }
                            }
                        },
                        Err(e) => warn!("[Backend Rename] Could not determine new metadata path for imported transcript {}: {:?}", new_path.display(), e),
                    }
                }
            }


            let new_relative_path_buf = new_path.strip_prefix(project_base_dir)?;
            let new_relative_path_for_transcript = new_relative_path_buf.to_string_lossy().replace("\\", "/");

            info!("[Backend Rename] Updating XML for imported transcript: OldRelPath '{}' -> NewRelPath '{}', Name -> '{}'", item_relative_path, new_relative_path_for_transcript, final_new_filename_with_ext);
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            let mut updated_xml = false;

            if let Some(entry) = project_data.imported_transcript_files.files.iter_mut().find(|t| t.relative_path == item_relative_path) {
                entry.name = final_new_filename_with_ext.clone();
                entry.relative_path = new_relative_path_for_transcript.clone();
                project_data.imported_transcript_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                updated_xml = true;
                info!("[Backend Rename] XML imported transcript entry updated.");
            } else {
                warn!("[Backend Rename] Renamed imported transcript file, but could not find matching path '{}' in XML.", item_relative_path);
            }

            if let Ok(new_metadata_path_for_xml) = get_document_metadata_path_for_doc(&new_path) { 
                 if let Some(metadata_entry) = project_data.document_metadata_files.files.iter_mut().find(|m| m.original_document_relative_path == item_relative_path) {
                    let new_metadata_relative_path = new_metadata_path_for_xml.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\", "/");
                    metadata_entry.name = new_metadata_path_for_xml.file_name().unwrap_or_default().to_string_lossy().to_string();
                    metadata_entry.original_document_relative_path = new_relative_path_for_transcript.clone();
                    metadata_entry.relative_path = new_metadata_relative_path;
                    project_data.document_metadata_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                    updated_xml = true;
                    info!("[Backend Rename] XML document metadata entry updated for imported transcript.");
                }
            }


            if updated_xml {
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Rename] XML saved for imported transcript rename.");
            }
        },
        "doc" => { // Handles .json, .pdf, .md, .txt
            let new_filename_with_ext = new_name_trimmed;
            let new_path = parent_dir.join(new_filename_with_ext);

            if contains_invalid_chars(new_filename_with_ext) { return Err(CommandError::from("New filename contains invalid chars.")); }
            let allowed_extensions = ["json", "md", "txt", "pdf"];
            let new_ext = new_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if !allowed_extensions.contains(&new_ext.as_str()) {
                 return Err(CommandError::from(format!("Invalid extension '.{}'. Allowed extensions for documents are: {:?}", new_ext, allowed_extensions)));
            }
            let old_ext = item_path_buf.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
             if old_ext != new_ext {
                  return Err(CommandError::from(format!("Changing document file extension from '.{}' to '.{}' is not allowed.", old_ext, new_ext)));
             }
            if new_filename_with_ext.starts_with('.') && 
               !new_filename_with_ext.ends_with(METADATA_FILE_SUFFIX) &&
               !new_filename_with_ext.ends_with(PDF_ANNOTATIONS_FILE_SUFFIX) {
                return Err(CommandError::from("Document filename cannot start with a dot unless it's a designated metadata or annotation file."));
            }


            if item_path_buf == new_path { info!("[Backend Rename] New path is same as old path. No action needed."); return Ok(()); }

            if new_path.exists() {
                 let canon_old = fs::canonicalize(&item_path_buf).ok();
                 let canon_new = fs::canonicalize(&new_path).ok();
                 if canon_old.is_some() && canon_new.is_some() && canon_old != canon_new {
                     return Err(CommandError::from(format!("File named '{}' already exists.", new_filename_with_ext)));
                 }
            }

            // Rename main document file
            info!("[Backend Rename] Renaming document file {} -> {}", item_path_buf.display(), new_path.display());
            fs::rename(&item_path_buf, &new_path).map_err(|e| CommandError::from(format!("Failed rename file: {}", e)))?;

            // Rename associated app metadata file (.filename.metadata.json)
            if let Ok(old_app_metadata_path) = get_document_metadata_path_for_doc(&item_path_buf) {
                if old_app_metadata_path.exists() {
                    if let Ok(new_app_metadata_path) = get_document_metadata_path_for_doc(&new_path) {
                        info!("[Backend Rename] Renaming app metadata: {} -> {}", old_app_metadata_path.display(), new_app_metadata_path.display());
                        if new_app_metadata_path.exists() && new_app_metadata_path != old_app_metadata_path {
                            warn!("[Backend Rename] Target app metadata {} already exists. Skipping rename.", new_app_metadata_path.display());
                        } else if old_app_metadata_path != new_app_metadata_path {
                            if let Err(e) = fs::rename(&old_app_metadata_path, &new_app_metadata_path) {
                                warn!("[Backend Rename] Failed to rename app metadata: {}", e);
                            }
                        }
                    }
                }
            }
            
            // If it's a PDF, also rename its PDF annotation file (.filename.annotations.json)
            let mut new_pdf_annotation_relative_path_for_xml: Option<String> = None;
            if old_ext == "pdf" {
                if let Ok(old_pdf_annot_path) = get_pdf_annotation_file_path(&item_path_buf) {
                    if old_pdf_annot_path.exists() {
                        if let Ok(new_pdf_annot_path) = get_pdf_annotation_file_path(&new_path) {
                            info!("[Backend Rename] Renaming PDF annotation: {} -> {}", old_pdf_annot_path.display(), new_pdf_annot_path.display());
                             if new_pdf_annot_path.exists() && new_pdf_annot_path != old_pdf_annot_path {
                                warn!("[Backend Rename] Target PDF annotation {} already exists. Skipping rename.", new_pdf_annot_path.display());
                            } else if old_pdf_annot_path != new_pdf_annot_path {
                                if let Err(e) = fs::rename(&old_pdf_annot_path, &new_pdf_annot_path) {
                                    warn!("[Backend Rename] Failed to rename PDF annotation: {}", e);
                                } else {
                                     new_pdf_annotation_relative_path_for_xml = Some(new_pdf_annot_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\","/"));
                                }
                            }
                        }
                    }
                }
            }


            let new_relative_path_for_doc = new_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\", "/");

            info!("[Backend Rename] Updating XML for document: Path '{}' -> '{}', name -> '{}'", item_relative_path, new_relative_path_for_doc, new_filename_with_ext);
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            let mut updated_xml = false;

            if let Some(doc_entry) = project_data.document_files.files.iter_mut().find(|d| d.relative_path == item_relative_path) {
                doc_entry.name = new_filename_with_ext.to_string();
                doc_entry.relative_path = new_relative_path_for_doc.clone();
                project_data.document_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                updated_xml = true;
                info!("[Backend Rename] XML document entry updated.");
            } else {
                warn!("[Backend Rename] Renamed document file, but could not find matching path '{}' in XML.", item_relative_path);
            }

            // Update app metadata XML entry
            if let Ok(new_app_metadata_path_for_xml) = get_document_metadata_path_for_doc(&new_path) {
                 if let Some(metadata_entry) = project_data.document_metadata_files.files.iter_mut().find(|m| m.original_document_relative_path == item_relative_path) {
                    let new_metadata_relative_path = new_app_metadata_path_for_xml.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\", "/");
                    metadata_entry.name = new_app_metadata_path_for_xml.file_name().unwrap_or_default().to_string_lossy().to_string();
                    metadata_entry.original_document_relative_path = new_relative_path_for_doc.clone();
                    metadata_entry.relative_path = new_metadata_relative_path;
                    project_data.document_metadata_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                    updated_xml = true;
                    info!("[Backend Rename] XML document app metadata entry updated.");
                }
            }
            
            // Update PDF annotation XML entry if it's a PDF and was renamed
            if old_ext == "pdf" {
                if let Some(new_rel_path) = new_pdf_annotation_relative_path_for_xml {
                    if let Some(pdf_annot_entry) = project_data.pdf_annotation_files.files.iter_mut().find(|pa| pa.original_document_relative_path == item_relative_path) {
                        let new_pdf_annot_filename = PathBuf::from(&new_rel_path).file_name().unwrap_or_default().to_string_lossy().to_string();
                        pdf_annot_entry.name = new_pdf_annot_filename;
                        pdf_annot_entry.original_document_relative_path = new_relative_path_for_doc.clone();
                        pdf_annot_entry.relative_path = new_rel_path;
                        project_data.pdf_annotation_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                        updated_xml = true;
                        info!("[Backend Rename] XML PDF annotation entry updated.");
                    }
                }
            }


            if updated_xml {
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Rename] XML saved for document and its associated files.");
            }
        },
        "table" => {
            let new_filename_with_ext = new_name_trimmed;
            let new_path = parent_dir.join(new_filename_with_ext);

            if contains_invalid_chars(new_filename_with_ext) { return Err(CommandError::from("New table filename contains invalid characters.")); }
            let allowed_extensions = ["csv", "xlsx"];
            let new_ext = new_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if !allowed_extensions.contains(&new_ext.as_str()) {
                return Err(CommandError::from(format!("Invalid extension '.{}'. Allowed extensions for tables are: {:?}", new_ext, allowed_extensions)));
            }
            let old_ext = item_path_buf.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if old_ext != new_ext {
                return Err(CommandError::from(format!("Changing table file extension from '.{}' to '.{}' is not allowed.", old_ext, new_ext)));
            }
            if new_filename_with_ext.starts_with('.') { return Err(CommandError::from("Table filename cannot start with a dot.")); }

            if item_path_buf == new_path { info!("[Backend Rename] New table path is same as old path. No action needed."); return Ok(()); }

            if new_path.exists() {
                let canon_old = fs::canonicalize(&item_path_buf).ok();
                let canon_new = fs::canonicalize(&new_path).ok();
                if canon_old.is_some() && canon_new.is_some() && canon_old != canon_new {
                    return Err(CommandError::from(format!("Table file named '{}' already exists.", new_filename_with_ext)));
                }
            }

            info!("[Backend Rename] Renaming table file {} -> {}", item_path_buf.display(), new_path.display());
            fs::rename(&item_path_buf, &new_path).map_err(|e| CommandError::from(format!("Failed to rename table file: {}", e)))?;

            let new_relative_path_buf = new_path.strip_prefix(project_base_dir)?;
            let new_relative_path = new_relative_path_buf.to_string_lossy().replace("\\", "/");

            info!("[Backend Rename] Updating XML for table: Path '{}' -> '{}', name -> '{}'", item_relative_path, new_relative_path, new_filename_with_ext);
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            let mut updated_xml = false;

            if let Some(table_entry) = project_data.table_files.files.iter_mut().find(|t| t.relative_path == item_relative_path) {
                table_entry.name = new_filename_with_ext.to_string();
                table_entry.relative_path = new_relative_path;
                project_data.table_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                updated_xml = true;
                info!("[Backend Rename] XML table entry updated.");
            } else {
                warn!("[Backend Rename] Renamed table file, but could not find matching path '{}' in XML.", item_relative_path);
            }

            if updated_xml {
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Rename] XML saved for table rename.");
            }
        },
        "image" => {
            let new_filename_with_ext = new_name_trimmed;
            let new_path = parent_dir.join(new_filename_with_ext);

            if contains_invalid_chars(new_filename_with_ext) { return Err(CommandError::from("New image filename contains invalid characters.")); }
            let allowed_extensions = ["jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff"];
            let new_ext = new_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if !allowed_extensions.contains(&new_ext.as_str()) {
                return Err(CommandError::from(format!("Invalid extension '.{}'. Allowed extensions for images are: {:?}", new_ext, allowed_extensions)));
            }
            let old_ext = item_path_buf.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if old_ext != new_ext {
                return Err(CommandError::from(format!("Changing image file extension from '.{}' to '.{}' is not allowed.", old_ext, new_ext)));
            }
            if new_filename_with_ext.starts_with('.') { return Err(CommandError::from("Image filename cannot start with a dot.")); }

            if item_path_buf == new_path { info!("[Backend Rename] New image path is same as old path. No action needed."); return Ok(()); }

            if new_path.exists() {
                let canon_old = fs::canonicalize(&item_path_buf).ok();
                let canon_new = fs::canonicalize(&new_path).ok();
                if canon_old.is_some() && canon_new.is_some() && canon_old != canon_new {
                    return Err(CommandError::from(format!("Image file named '{}' already exists.", new_filename_with_ext)));
                }
            }

            let old_metadata_path_result = get_annotation_metadata_path_for_image(&item_path_buf);

            info!("[Backend Rename] Renaming image file {} -> {}", item_path_buf.display(), new_path.display());
            fs::rename(&item_path_buf, &new_path).map_err(|e| CommandError::from(format!("Failed to rename image file: {}", e)))?;

            if let Ok(old_metadata_path) = old_metadata_path_result {
                if old_metadata_path.exists() {
                    match get_annotation_metadata_path_for_image(&new_path) {
                        Ok(new_metadata_path) => {
                            info!("[Backend Rename] Attempting to rename image annotation metadata file: {} -> {}", old_metadata_path.display(), new_metadata_path.display());
                            if new_metadata_path.exists() {
                                warn!("[Backend Rename] Target annotation metadata file {} already exists. Skipping rename of old metadata file {} to avoid overwrite.", new_metadata_path.display(), old_metadata_path.display());
                            } else {
                                if let Err(e) = fs::rename(&old_metadata_path, &new_metadata_path) {
                                    warn!("[Backend Rename] Failed to rename image annotation metadata file {} to {}: {}", old_metadata_path.display(), new_metadata_path.display(), e);
                                } else {
                                    info!("[Backend Rename] Successfully renamed image annotation metadata file.");
                                }
                            }
                        }
                        Err(e) => {
                            warn!("[Backend Rename] Could not determine new image annotation metadata path for {}: {:?}. Skipping metadata rename.", new_path.display(), e);
                        }
                    }
                } else {
                    debug!("[Backend Rename] Old image annotation metadata file not found, no need to rename: {}", old_metadata_path.display());
                }
            } else {
                 warn!("[Backend Rename] Could not determine old image annotation metadata path for {}: {:?}. Skipping metadata rename.", item_path_buf.display(), old_metadata_path_result.err());
            }


            let new_relative_path_buf = new_path.strip_prefix(project_base_dir)?;
            let new_relative_path = new_relative_path_buf.to_string_lossy().replace("\\", "/");

            info!("[Backend Rename] Updating XML for image: Path '{}' -> '{}', name -> '{}'", item_relative_path, new_relative_path, new_filename_with_ext);
            let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path_buf)?)?;
            let mut updated_xml = false;

            if let Some(image_entry) = project_data.image_files.files.iter_mut().find(|i| i.relative_path == item_relative_path) {
                image_entry.name = new_filename_with_ext.to_string();
                image_entry.relative_path = new_relative_path;
                project_data.image_files.files.sort_by(|a,b| a.name.cmp(&b.name));
                updated_xml = true;
                info!("[Backend Rename] XML image entry updated.");
            } else {
                warn!("[Backend Rename] Renamed image file, but could not find matching path '{}' in XML.", item_relative_path);
            }

            if updated_xml {
                save_project_xml(&xml_path_buf, &project_data)?;
                info!("[Backend Rename] XML saved for image rename.");
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
