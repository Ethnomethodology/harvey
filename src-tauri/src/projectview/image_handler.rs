// src-tauri/src/projectview/image_handler.rs
use super::shared_types::*;
use super::shared_utils::{save_project_xml, ensure_base_asset_dirs, truncate_filename_stem, MAX_FILENAME_STEM_LENGTH};
use crate::welcome::config::{CommandError};
use crate::projectview::db_handler;
    use chrono::Utc;
use serde_json;
use log::{info, warn, error};
use std::{
    fs,
        path::{Path, PathBuf},
};
use quick_xml;
    // TODO: Refactor to use base64::engine::general_purpose::STANDARD.decode() or similar
    // as base64::decode is deprecated.
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    // UserDirs is no longer needed as we use get_config_dir

const SUPPORTED_IMAGE_EXTENSIONS: [&str; 7] = ["jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff"];

// Placeholder for project path resolution - NEEDS PROPER IMPLEMENTATION

// Constants from shared_utils, ensure they are accessible or defined here if not.
// For example:
// const HARVEY_FILES_DIR: &str = ".harvey_files";
// const IMAGES_DIR: &str = "Images";

// FileMetadata is available via shared_types::*, StandardAssetMetadata is no longer needed here.

// Placeholder for a function that might exist in image_handler.rs or similar
// to register the image with the project (e.g., update a manifest, db, etc.)
fn register_project_image(
    project_base_dir: &Path,
    project_xml_path: &Path,
    image_filename_with_ext: &str,
    image_folder_name_for_xml: &str,
    original_media_stem_for_metadata: &str,
    timestamp: f64
) -> Result<(), CommandError> {
    info!("[register_project_image] Registering image '{}' (from media '{}', folder name for XML '{}') for project at '{}'", image_filename_with_ext, original_media_stem_for_metadata, image_folder_name_for_xml, project_base_dir.display());

    let xml_content = fs::read_to_string(project_xml_path)?;
    let mut project_data: ProjectXml = quick_xml::de::from_str(&xml_content)?;

    let images_dir_name = IMAGES_DIR;
    // let image_folder_name = media_file_name_stem; // Old logic

    let relative_path_for_xml = Path::new(HARVEY_FILES_DIR)
        .join(images_dir_name)
        .join(image_folder_name_for_xml) // Use new parameter for path
        .join(image_filename_with_ext)
        .to_string_lossy()
        .replace("\\", "/");

    let new_image_entry = ImageEntryXml {
        name: image_filename_with_ext.to_string(), // The actual file name, might have _counter
        relative_path: relative_path_for_xml.clone(),
    };

    if project_data.image_files.files.iter().any(|f| f.relative_path == relative_path_for_xml) {
        warn!("[save_screenshot] Image with relative path '{}' already exists in XML. This shouldn't happen with unique filenames.", relative_path_for_xml);
    } else {
        project_data.image_files.files.push(new_image_entry);
    }
    project_data.image_files.files.sort_by(|a, b| a.name.cmp(&b.name));
    save_project_xml(project_xml_path, &project_data)?;
    info!("[save_screenshot] Project XML updated successfully for screenshot.");

    let abs_image_path = project_base_dir.join(&relative_path_for_xml);

    let file_metadata_for_db = FileMetadata {
        file_name: image_filename_with_ext.to_string(),
        file_path: abs_image_path.to_string_lossy().into_owned(),
        last_modified: Utc::now().to_rfc3339(),
        title: format!("Screenshot from {} at {}s", original_media_stem_for_metadata, timestamp.round()), // Use new parameter
        description: format!("Screenshot captured from media '{}' at timestamp {} seconds.", original_media_stem_for_metadata, timestamp), // Use new parameter
        summary: String::new(),
        duration_seconds: None,
        width: None,
        height: None,
        frame_rate: None,
        bit_rate: None,
        audio_codec: None,
        video_codec: None,
        created_at: Some(Utc::now().to_rfc3339()),
                original_import_path: Some("screenshot".to_string()),
                speaker_names: None,
                waveform_data: None,
                language_code: None,
                properties: None,
    };

    let mut custom_fields_map = serde_json::Map::new();
    custom_fields_map.insert("_isScreenshot".to_string(), serde_json::Value::Bool(true));
    let custom_fields_json = Some(serde_json::Value::Object(custom_fields_map).to_string());

    // Read project_uuid from XML
    let project_xml_content_for_uuid = fs::read_to_string(project_xml_path) // project_xml_path is already a &Path
        .map_err(|e| CommandError::Io(format!("Failed to read project XML for UUID from {}: {}", project_xml_path.display(), e)))?;
    let project_data_for_uuid: ProjectXml = quick_xml::de::from_str(&project_xml_content_for_uuid)
        .map_err(|e| CommandError::XmlDeserialization(format!("Failed to parse project XML for UUID from {}: {}", project_xml_path.display(), e)))?;

    let project_id_for_db = project_data_for_uuid.project_uuid;
    if project_id_for_db.is_empty() {
        error!("[register_project_image] Project UUID is empty in XML file: {}. Cannot save asset metadata without project_id.", project_xml_path.display());
        return Err(CommandError::Message(format!("Project ID (UUID) is missing in the project file ({}). Asset metadata cannot be saved.", project_xml_path.display())));
    }

    if let Err(e) = db_handler::save_asset_metadata(
        &project_id_for_db, // Pass project_id
        &file_metadata_for_db,
        &relative_path_for_xml,
        "image",
        custom_fields_json.as_deref(),
    ) {
        error!("[register_project_image] Failed to save screenshot metadata to DB for {} (project_id: {}): {}", relative_path_for_xml, project_id_for_db, e);
        return Err(CommandError::from(format!("Failed to save screenshot metadata to DB: {}", e)));
    }
    info!("[register_project_image] Saved screenshot metadata to DB for: {} (project_id: {})", relative_path_for_xml, project_id_for_db);
    Ok(())
}


#[tauri::command]
pub async fn save_screenshot(
    project_xml_path_str: String, // Added
    project_id: String, // This is the UUID, used for logging or if needed for specific metadata, but not for base path
    media_file_name: String,
    timestamp: f64,
    image_data_base64: String,
) -> Result<(), String> {
    info!("[save_screenshot] Received screenshot for project_id (UUID): {}, project_xml_path: {}, media: {}, timestamp: {}", project_id, project_xml_path_str, media_file_name, timestamp);

    let project_xml_path = PathBuf::from(&project_xml_path_str);
    let project_base_dir = match project_xml_path.parent() {
        Some(p) => p.to_path_buf(),
        None => return Err(format!("Could not determine base directory from project XML path: {}", project_xml_path_str)),
    };

    // Ensure the XML file itself exists, as register_project_image will need to read/write it.
    if !project_xml_path.exists() {
        return Err(format!("Project XML file not found at the specified path: {}", project_xml_path_str));
    }

    // 1. Derive original_media_stem
    let original_media_stem = Path::new(&media_file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("media") // Fallback
        .to_string();

    // 2. Generate timestamp_str
    let secs = timestamp.trunc() as u64;
    let millis = (timestamp.fract() * 1000.0).round() as u32;
    let timestamp_str = format!("T{}_{:03}", secs, millis);

    // 3. Create screenshot_base_folder_name (this will be the folder name)
    let screenshot_base_folder_name = format!("{}_{}", original_media_stem, timestamp_str);

    // 4. Determine image_folder_for_screenshot (this is the actual directory path)
    let images_root_dir = project_base_dir.join(HARVEY_FILES_DIR).join(IMAGES_DIR);
    let image_folder_for_screenshot = images_root_dir.join(&screenshot_base_folder_name);

    // 5. Create the directory
    fs::create_dir_all(&image_folder_for_screenshot)
        .map_err(|e| format!("Failed to create Images sub-directory for screenshot {}: {}", screenshot_base_folder_name, e))?;

    // 6. Determine unique final_screenshot_filename_with_ext
    // The file name will be based on the folder name, with a counter for uniqueness if needed.
    let mut counter = 0;
    let final_screenshot_filename_with_ext: String;
    loop {
        let prospective_file_name = if counter == 0 {
            format!("{}.png", screenshot_base_folder_name)
        } else {
            format!("{}_{}.png", screenshot_base_folder_name, counter)
        };
        let candidate_path = image_folder_for_screenshot.join(&prospective_file_name);
        if !candidate_path.exists() {
            final_screenshot_filename_with_ext = prospective_file_name;
            break;
        }
        counter += 1;
        if counter > 100 { // Safety break
            return Err(format!("Could not generate a unique filename for screenshot from {} at timestamp {}", original_media_stem, timestamp_str));
        }
    }

    // 7. Set file_path
    let file_path = image_folder_for_screenshot.join(&final_screenshot_filename_with_ext);

    // 8. Save the image data
    let image_bytes = STANDARD.decode(&image_data_base64) // Updated base64 decode
        .map_err(|e| format!("Failed to decode base64 image data: {}", e))?;

    fs::write(&file_path, image_bytes)
        .map_err(|e| format!("Failed to save screenshot to '{}': {}", file_path.display(), e))?;
    info!("[save_screenshot] Screenshot saved to: {}", file_path.display());

    // 9. Call register_project_image with updated arguments
    register_project_image(
        &project_base_dir,
        &project_xml_path,
        &final_screenshot_filename_with_ext,
        &screenshot_base_folder_name, // This is image_folder_name_for_xml
        &original_media_stem,         // This is original_media_stem_for_metadata
        timestamp
    ).map_err(|e| format!("Failed to register screenshot in project: {}", e))?;

    // TODO: Emit event to frontend if UI needs to refresh image list
    // Example: app_handle.emit_all("new_screenshot_added", file_path.to_string_lossy().to_string()).unwrap_or_default();

    Ok(())
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

    let original_source_filename_with_ext = source_path.file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| CommandError::from("Could not get original image filename with extension"))?
        .to_string();

    let original_source_extension = source_path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // Check if the extension is supported
    if !SUPPORTED_IMAGE_EXTENSIONS.contains(&original_source_extension.as_str()) {
        return Err(CommandError::from(format!("Unsupported image file type: .{}", original_source_extension)));
    }

    // Truncate the original filename's stem
    let truncated_image_filename_with_ext = truncate_filename_stem(&original_source_filename_with_ext, MAX_FILENAME_STEM_LENGTH);
    info!("[import_image_file] Original filename: '{}', Truncated filename for project: '{}'", original_source_filename_with_ext, truncated_image_filename_with_ext);

    let image_file_stem_truncated = Path::new(&truncated_image_filename_with_ext).file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| CommandError::from(format!("Could not get stem from truncated image filename: {}", truncated_image_filename_with_ext)))?;

    // Create folder under Images named after the truncated file stem
    let images_base = project_base_dir.join(HARVEY_FILES_DIR).join(IMAGES_DIR);
    let folder_path = images_base.join(image_file_stem_truncated); // Folder uses truncated stem
    if !folder_path.exists() {
        fs::create_dir_all(&folder_path)
            .map_err(|e| CommandError::from(format!("Failed to create image folder {}: {}", folder_path.display(), e)))?;
    }

    // Determine unique filename *inside* the (truncated stem) folder
    let mut counter = 0;
    let final_image_path = loop {
        let file_name_to_try = if counter == 0 {
            truncated_image_filename_with_ext.clone()
        } else {
            // If collision, append suffix to the *truncated stem* part, then add original extension
            format!("{}_{}.{}", image_file_stem_truncated, counter, original_source_extension)
        };
        let candidate = folder_path.join(&file_name_to_try);
        if !candidate.exists() {
            break candidate;
        }
        counter += 1;
        if counter > 1000 { // Safety break
            return Err(CommandError::from(format!(
                "Could not find unique filename for image base '{}' (derived from truncated name) after {} attempts.",
                image_file_stem_truncated, counter
            )));
        }
    };

    // final_image_name is the name of the file as it will be saved (e.g., truncated_stem.png or truncated_stem_1.png)
    let final_image_name = final_image_path.file_name().unwrap().to_string_lossy().into_owned();

    // Copy the file
    info!("[import_image_file] Copying image from '{}' to '{}'", source_path.display(), final_image_path.display());
    fs::copy(&source_path, &final_image_path).map_err(|e| CommandError::from(format!("Failed to copy image file: {}", e)))?;

    // Update the project XML
    info!("[import_image_file] Updating project XML to include image: {}", final_image_name);
    let xml_content = fs::read_to_string(&project_xml_path)?;
    let mut project_data: ProjectXml = quick_xml::de::from_str(&xml_content)?;

    // Calculate the relative path for XML storage
    let relative_path_for_xml = final_image_path // Path uses (potentially suffixed) truncated name
        .strip_prefix(project_base_dir)?
        .to_string_lossy()
        .replace("\\", "/");

    let new_image_entry = ImageEntryXml {
        name: final_image_name.clone(), // XML name is the final (potentially suffixed) truncated filename
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

    // Save metadata to DB
    let file_metadata_for_db = FileMetadata {
        file_name: final_image_name.clone(), // Use final (potentially suffixed) truncated filename
        file_path: final_image_path.to_string_lossy().into_owned(), // Absolute path uses final name
        last_modified: Utc::now().to_rfc3339(),
        title: String::new(), // Init empty
        description: String::new(), // Init empty
        summary: String::new(),
        duration_seconds: None,
        width: None, // image_handler.rs currently does not extract these on import
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
    };

    // relative_path_for_xml is already calculated and holds the image's relative path
    // Read project_uuid from XML
    let project_xml_content_for_uuid = fs::read_to_string(&project_xml_path) // project_xml_path is already a PathBuf
        .map_err(|e| CommandError::Io(format!("Failed to read project XML for UUID from {}: {}", project_xml_path.display(), e)))?;
    let project_data_for_uuid: ProjectXml = quick_xml::de::from_str(&project_xml_content_for_uuid)
        .map_err(|e| CommandError::XmlDeserialization(format!("Failed to parse project XML for UUID from {}: {}", project_xml_path.display(), e)))?;

    let project_id_for_db = project_data_for_uuid.project_uuid;
    if project_id_for_db.is_empty() {
        error!("[import_image_file] Project UUID is empty in XML file: {}. Cannot save asset metadata without project_id.", project_xml_path.display());
        return Err(CommandError::Message(format!("Project ID (UUID) is missing in the project file ({}). Asset metadata cannot be saved.", project_xml_path.display())));
    }

    if let Err(e) = db_handler::save_asset_metadata(
        &project_id_for_db, // Pass project_id
        &file_metadata_for_db,
        &relative_path_for_xml, // DB key uses final (potentially suffixed) truncated name based path
        "image", // asset_type
        None, // custom_fields_json
    ) {
        error!("[import_image_file] Failed to save image metadata to DB for {} (project_id: {}): {}", relative_path_for_xml, project_id_for_db, e);
        // Attempt to clean up copied file if DB save fails?
        return Err(CommandError::from(format!("Failed to save image metadata to DB: {}", e)));
    }
    info!("[import_image_file] Saved image metadata to DB for: {} (project_id: {})", relative_path_for_xml, project_id_for_db);

    // Annotation JSO file creation is removed, DB will handle annotations.

    // Return the absolute path of the newly imported image
    Ok(final_image_path.to_string_lossy().to_string()) // Return absolute path to the imported (potentially renamed) file
}

// --- NEW ANNOTATION COMMANDS ---

#[tauri::command]
pub async fn load_image_annotations(project_id: String, image_relative_path_str: String) -> Result<Option<String>, CommandError> {
    use crate::projectview::db_handler::load_annotations_from_db;
    // log macros info, error, debug are already imported at the top of the file.

    info!("[DB Image Annots] Loading for project_id {}: image key '{}'", project_id, image_relative_path_str);
    match load_annotations_from_db(&project_id, &image_relative_path_str, "image") {
        Ok(Some(content)) => Ok(Some(content)),
        Ok(None) => Ok(None),
        Err(e) => {
            error!("[DB Image Annots] Error loading annotations for project_id {} - {}: {}", project_id, image_relative_path_str, e);
            Err(CommandError::from(format!("Failed to load image annotations from DB: {}", e)))
        }
    }
}

#[tauri::command]
pub async fn save_image_annotations(project_id: String, image_relative_path_str: String, annotations_json_string: String) -> Result<(), CommandError> {
    use crate::projectview::db_handler::save_annotations_to_db;
    // log macros info, error, debug, warn are already imported at the top of the file.

    info!("[DB Image Annots] Saving for project_id {}: image key '{}'", project_id, image_relative_path_str);

    // Basic JSON validation before saving to DB (optional but good practice)
    if serde_json::from_str::<serde_json::Value>(&annotations_json_string).is_err() {
        warn!("[DB Image Annots] Annotation JSON content for project_id {} - {} appears invalid. Saving anyway.", project_id, image_relative_path_str);
    }

    match save_annotations_to_db(&project_id, &image_relative_path_str, &annotations_json_string, "image") {
        Ok(_) => {
            info!("[DB Image Annots] Annotations saved successfully for project_id {} - {}.", project_id, image_relative_path_str);
            Ok(())
        },
        Err(e) => {
            error!("[DB Image Annots] Error saving annotations for project_id {} - {}: {}", project_id, image_relative_path_str, e);
            Err(CommandError::from(format!("Failed to save image annotations to DB: {}", e)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;
    use crate::projectview::shared_types::{ProjectXml};
    use crate::projectview::db_handler::{self as test_db_handler};
    use rusqlite::Connection;
    // Assuming get_config_dir can be influenced for tests, or db_handler is refactored for testability.
    // For this example, we'll assume db_handler::init_db() can be made to work with a temp DB path.

    // Helper to create a dummy project.xml for testing
    fn create_dummy_project_xml_for_image_test(project_dir: &Path, project_name: &str, project_uuid: &str) -> PathBuf {
        let project_xml_path = project_dir.join("project.xml");
        let project_data = ProjectXml {
            name: project_name.to_string(),
            project_uuid: project_uuid.to_string(),
            media_files: Default::default(),
            image_files: Default::default(), // Initialize as empty
            document_files: Default::default(),
            table_files: Default::default(),
            imported_transcript_files: Default::default(),
            document_metadata_files: Default::default(), // Should remain empty for image metadata
        };
        let xml_string = quick_xml::se::to_string(&project_data).unwrap();
        fs::write(&project_xml_path, xml_string).unwrap();
        project_xml_path
    }

    // Helper to create a tiny dummy PNG file
    fn create_dummy_png_file(dir: &Path, filename: &str) -> PathBuf {
        let file_path = dir.join(filename);
        // PNG magic bytes
        let png_magic: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let mut file = File::create(&file_path).expect("Failed to create dummy png");
        file.write_all(&png_magic).expect("Failed to write dummy png content");
        file_path
    }

    fn setup_test_environment() -> (tempfile::TempDir, PathBuf, PathBuf, Connection, String) {
        let temp_dir = tempdir().unwrap();
        let project_dir = temp_dir.path().to_path_buf();
        let harvey_files_dir = project_dir.join(".harvey_files");
        let images_dir = harvey_files_dir.join("Images");
        fs::create_dir_all(&images_dir).unwrap();

        let dummy_image_path = project_dir.join("dummy.jpg");
        let mut f = fs::File::create(&dummy_image_path).unwrap();
        f.write_all(b"fake image data").unwrap();

        let (conn, project_id) = test_db_handler::setup_test_db_in_memory();

        (temp_dir, project_dir, dummy_image_path, conn, project_id)
    }

    #[tokio::test]
    #[ignore]
    async fn test_import_image_file_with_db_metadata() -> Result<(), Box<dyn std::error::Error>> {
        let (_temp_dir, project_dir, _dummy_image_path, _conn, project_uuid) = setup_test_environment();
        let project_xml_path = create_dummy_project_xml_for_image_test(&project_dir, "TestImageProject", &project_uuid);
        let project_xml_path_str = project_xml_path.to_string_lossy().to_string();

        // Create dummy source image file
        let source_image_dir = project_dir.join("source_images");
        fs::create_dir_all(&source_image_dir)?;
        let dummy_image_path = create_dummy_png_file(&source_image_dir, "dummy_image.png");
        let dummy_image_path_str = dummy_image_path.to_string_lossy().to_string();

        // Execute import_image_file
        let import_result = import_image_file(dummy_image_path_str.clone(), project_xml_path_str.clone()).await;
        assert!(import_result.is_ok(), "import_image_file failed: {:?}", import_result.err());
        let final_image_abs_path_str = import_result.unwrap();
        let final_image_abs_path = PathBuf::from(&final_image_abs_path_str);

        // File System Assertions
        assert!(final_image_abs_path.exists(), "Imported image file should exist at {}", final_image_abs_path_str);

        let parent_dir = final_image_abs_path.parent().unwrap();
        let stem = final_image_abs_path.file_stem().unwrap().to_str().unwrap();
        let metadata_json_path = parent_dir.join(format!(".{}.metadata.json", stem)); // Path where old metadata would have been
        assert!(!metadata_json_path.exists(), ".metadata.json file should NOT exist at {}", metadata_json_path.display());

        // XML Assertions
        let updated_xml_content = fs::read_to_string(&project_xml_path)?;
        let updated_project_data: ProjectXml = quick_xml::de::from_str(&updated_xml_content)?;

        let expected_image_name = final_image_abs_path.file_name().unwrap().to_str().unwrap();
        let expected_relative_path = final_image_abs_path.strip_prefix(&project_dir)?.to_string_lossy().replace("\\", "/");

        assert_eq!(updated_project_data.image_files.files.len(), 1, "Should be one image file in XML");
        let image_entry_xml = updated_project_data.image_files.files.get(0).unwrap();
        assert_eq!(image_entry_xml.name, expected_image_name);
        assert_eq!(image_entry_xml.relative_path, expected_relative_path);
        assert!(updated_project_data.document_metadata_files.files.is_empty(), "document_metadata_files should be empty in XML regarding this image");

        // Database Assertions
        let loaded_meta_option = test_db_handler::load_asset_metadata("test-image-uuid", &expected_relative_path)
            .expect("Failed to load metadata from DB for assertion");

        assert!(loaded_meta_option.is_some(), "Metadata should be found in DB for relative path: {}", expected_relative_path);
        if let Some(loaded_meta) = loaded_meta_option {
            assert_eq!(loaded_meta.file_name, expected_image_name);
            assert_eq!(loaded_meta.file_path, final_image_abs_path_str); // DB stores absolute path
            assert_eq!(loaded_meta.asset_type, "image");
            assert_eq!(loaded_meta.title.unwrap_or_default(), "");
            assert!(loaded_meta.custom_fields_json.is_none());
            assert!(loaded_meta.width.is_none(), "Width should be None as image_handler does not extract it");
            assert!(loaded_meta.height.is_none(), "Height should be None as image_handler does not extract it");
        }

        // Cleanup
        std::env::remove_var("HARVEY_TEST_CONFIG_DIR");
        // temp_project_dir and temp_config_dir will be cleaned up when they go out of scope.
        Ok(())
    }
}
