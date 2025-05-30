// src-tauri/src/projectview/image_handler.rs
use super::shared_types::*;
use super::shared_utils::{save_project_xml, ensure_base_asset_dirs};
use crate::welcome::config::CommandError; // Assuming this is your custom error type
use crate::projectview::core_commands::get_image_asset_metadata_path;
use chrono::Utc;
use serde_json;
use serde::{Serialize, Deserialize};
use log::{info, warn, debug, error}; // Added error
use std::{
    fs,
    io::{Read, Write}, // Added for file operations
    path::{Path, PathBuf},
};
use quick_xml;

const SUPPORTED_IMAGE_EXTENSIONS: [&str; 7] = ["jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff"];
// Constants from shared_utils, ensure they are accessible or defined here if not.
// For example:
// const HARVEY_FILES_DIR: &str = ".harvey_files";
// const IMAGES_DIR: &str = "Images";

// Duplicated struct definitions (to be refactored to shared_types later)
#[derive(Serialize, Deserialize, Debug)]
struct FileMetadata {
    file_name: String,
    file_path: String,
    last_modified: String,
    title: String,
    description: String,
    summary: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct StandardAssetMetadata {
    metadata: FileMetadata,
    highlights: Vec<String>,
}

// Helper to get a unique path in the Images directory
// Removed get_unique_image_path function as it is no longer used

// Import command for image files
#[tauri::command]
pub async fn import_image_file(
    source_path_str: String,
    project_xml_path_str: String,
) -> Result<String, CommandError> {
    info!("[import_image_file] Importing image from: {}", source_path_str);
    let source_path = PathBuf::from(&source_path_str);
    let project_xml_path = PathBuf::from(&project_xml_path_str);

    if !source_path.exists() || !source_path.is_file() {
        return Err(CommandError::from(format!("Source image file not found: {}", source_path_str)));
    }

    let project_base_dir = project_xml_path.parent()
        .ok_or_else(|| CommandError::from("Could not get project base directory from XML path"))?;

    // Ensure base asset directories (including Images) exist
    ensure_base_asset_dirs(project_base_dir)?;

    let source_filename_stem = source_path.file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| CommandError::from("Could not get image filename stem"))?;

    let source_extension = source_path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // Check if the extension is supported
    if !SUPPORTED_IMAGE_EXTENSIONS.contains(&source_extension.as_str()) {
        return Err(CommandError::from(format!("Unsupported image file type: .{}", source_extension)));
    }

    // Create folder under Images named after file stem
    let images_base = project_base_dir.join(HARVEY_FILES_DIR).join(IMAGES_DIR);
    let folder_path = images_base.join(source_filename_stem);
    if !folder_path.exists() {
        fs::create_dir_all(&folder_path)
            .map_err(|e| CommandError::from(format!("Failed to create image folder {}: {}", folder_path.display(), e)))?;
    }
    // Choose unique filename inside folder
    let mut counter = 0;
    let final_image_path = loop {
        let file_name = if counter == 0 {
            format!("{}.{}", source_filename_stem, source_extension)
        } else {
            format!("{}_{}.{}", source_filename_stem, counter, source_extension)
        };
        let candidate = folder_path.join(&file_name);
        if !candidate.exists() {
            break candidate;
        }
        counter += 1;
        if counter > 1000 {
            return Err(CommandError::from(format!(
                "Could not find unique filename for image base '{}' after {} attempts.",
                source_filename_stem, counter
            )));
        }
    };
    let final_image_name = final_image_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    // Copy the file
    info!("[import_image_file] Copying image from '{}' to '{}'", source_path.display(), final_image_path.display());
    fs::copy(&source_path, &final_image_path).map_err(|e| CommandError::from(format!("Failed to copy image file: {}", e)))?;

    // Update the project XML
    info!("[import_image_file] Updating project XML to include image: {}", final_image_name);
    let xml_content = fs::read_to_string(&project_xml_path)?;
    let mut project_data: ProjectXml = quick_xml::de::from_str(&xml_content)?;

    // Calculate the relative path for XML storage
    let relative_path_for_xml = final_image_path
        .strip_prefix(project_base_dir)?
        .to_string_lossy()
        .replace("\\", "/");

    let new_image_entry = ImageEntryXml {
        name: final_image_name.clone(),
        relative_path: relative_path_for_xml.clone(),
    };

    // Add the new entry to the image files list if it doesn't exist, or update name if path exists
    if project_data.image_files.files.iter().any(|f| f.relative_path == relative_path_for_xml) {
        warn!("[import_image_file] Image with relative path '{}' already exists in XML. Overwriting name if different.", relative_path_for_xml);
        if let Some(existing_entry) = project_data.image_files.files.iter_mut().find(|f| f.relative_path == relative_path_for_xml) {
            existing_entry.name = final_image_name.clone();
        }
    } else {
        project_data.image_files.files.push(new_image_entry);
    }
    project_data.image_files.files.sort_by(|a, b| a.name.cmp(&b.name)); // Keep the list sorted

    // Save the modified XML
    save_project_xml(&project_xml_path, &project_data)?;
    info!("[import_image_file] Project XML updated successfully for image.");

    info!("[import_image_file] Creating standard asset metadata for image: {}", final_image_path.display());
    match get_image_asset_metadata_path(&final_image_path) {
        Ok(asset_metadata_path) => {
            let image_file_name = final_image_path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("")
                .to_string();

            let asset_metadata_content = StandardAssetMetadata {
                metadata: FileMetadata {
                    file_name: image_file_name,
                    file_path: final_image_path.to_string_lossy().into_owned(), // Absolute path
                    last_modified: Utc::now().to_rfc3339(),
                    title: "".to_string(),
                    description: "".to_string(),
                    summary: "".to_string(),
                },
                highlights: Vec::new(),
            };

            match serde_json::to_string_pretty(&asset_metadata_content) {
                Ok(json_string) => {
                    if let Err(e) = fs::write(&asset_metadata_path, json_string) {
                        warn!("[import_image_file] Failed to write asset metadata file {}: {}", asset_metadata_path.display(), e);
                    } else {
                        info!("[import_image_file] Created asset metadata file: {}", asset_metadata_path.display());
                    }
                }
                Err(e) => {
                    warn!("[import_image_file] Failed to serialize asset metadata for {}: {}", asset_metadata_path.display(), e);
                }
            }
        }
        Err(e) => {
            warn!("[import_image_file] Failed to get asset metadata path for {}: {:?}", final_image_path.display(), e);
            // Do not block import if metadata path generation fails
        }
    }

    // Create an empty annotations file for this image
    let annotations_file_name = format!(".{}.annotations.json", source_filename_stem);
    let annotations_path = folder_path.join(&annotations_file_name);
    if !annotations_path.exists() {
        info!("[import_image_file] Creating empty annotations file: {}", annotations_path.display());
        fs::write(&annotations_path, "").map_err(|e| CommandError::from(format!("Failed to create annotations file {}: {}", annotations_path.display(), e)))?;
    }

    // Return the absolute path of the newly imported image
    Ok(final_image_path.to_string_lossy().to_string())
}


// --- NEW ANNOTATION COMMANDS ---

#[tauri::command]
pub async fn load_image_annotations(metadata_path_str: String) -> Result<Option<String>, CommandError> {
    debug!("[load_image_annotations] Attempting to load annotations from: {}", metadata_path_str);
    let metadata_path = PathBuf::from(metadata_path_str);

    if !metadata_path.exists() {
        info!("[load_image_annotations] Metadata file not found at {}. No annotations to load.", metadata_path.display());
        return Ok(None); // Not an error, just no file
    }

    if !metadata_path.is_file() {
        warn!("[load_image_annotations] Metadata path {} is not a file.", metadata_path.display());
        return Err(CommandError::from(format!("Metadata path {} is not a file.", metadata_path.display())));
    }

    match fs::File::open(&metadata_path) {
        Ok(mut file) => {
            let mut contents = String::new();
            match file.read_to_string(&mut contents) {
                Ok(_) => {
                    info!("[load_image_annotations] Successfully loaded annotations from {}.", metadata_path.display());
                    Ok(Some(contents))
                }
                Err(e) => {
                    error!("[load_image_annotations] Failed to read annotations from {}: {}", metadata_path.display(), e);
                    Err(CommandError::from(format!("Failed to read annotations file {}: {}", metadata_path.display(), e)))
                }
            }
        }
        Err(e) => {
            error!("[load_image_annotations] Failed to open annotations file {}: {}", metadata_path.display(), e);
            // Distinguish between "not found" (already handled) and other errors
            Err(CommandError::from(format!("Failed to open annotations file {}: {}", metadata_path.display(), e)))
        }
    }
}

#[tauri::command]
pub async fn save_image_annotations(metadata_path_str: String, annotations_json_string: String) -> Result<(), CommandError> {
    debug!("[save_image_annotations] Attempting to save annotations to: {}", metadata_path_str);
    let metadata_path = PathBuf::from(metadata_path_str);

    // Ensure parent directory exists if the path is not just a filename
    if let Some(parent_dir) = metadata_path.parent() {
        if !parent_dir.exists() {
            info!("[save_image_annotations] Parent directory {} does not exist, creating.", parent_dir.display());
            fs::create_dir_all(parent_dir).map_err(|e| CommandError::from(format!("Failed to create parent directory {}: {}", parent_dir.display(), e)))?;
        }
    }

    match fs::File::create(&metadata_path) {
        Ok(mut file) => {
            match file.write_all(annotations_json_string.as_bytes()) {
                Ok(_) => {
                    info!("[save_image_annotations] Successfully saved annotations to {}.", metadata_path.display());
                    Ok(())
                }
                Err(e) => {
                    error!("[save_image_annotations] Failed to write annotations to {}: {}", metadata_path.display(), e);
                    Err(CommandError::from(format!("Failed to write annotations to file {}: {}", metadata_path.display(), e)))
                }
            }
        }
        Err(e) => {
            error!("[save_image_annotations] Failed to create/truncate annotations file {}: {}", metadata_path.display(), e);
            Err(CommandError::from(format!("Failed to create/truncate annotations file {}: {}", metadata_path.display(), e)))
        }
    }
}