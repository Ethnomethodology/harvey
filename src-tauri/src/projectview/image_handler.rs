// src-tauri/src/projectview/image_handler.rs
use super::shared_types::*;
use super::shared_utils::{save_project_xml, ensure_base_asset_dirs};
use crate::welcome::config::CommandError; // Assuming this is your custom error type
// get_image_asset_metadata_path removed
use crate::projectview::db_handler; // Added
use chrono::Utc;
use serde_json;
use log::{info, warn, error}; // debug removed
use std::{
    fs,
    // io::{Read, Write} removed as per instruction, assuming File ops below don't need direct trait import
    path::PathBuf,
};
use quick_xml;

const SUPPORTED_IMAGE_EXTENSIONS: [&str; 7] = ["jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff"];
// Constants from shared_utils, ensure they are accessible or defined here if not.
// For example:
// const HARVEY_FILES_DIR: &str = ".harvey_files";
// const IMAGES_DIR: &str = "Images";

// FileMetadata is available via shared_types::*, StandardAssetMetadata is no longer needed here.

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

    // Save metadata to DB
    let file_metadata_for_db = FileMetadata {
        file_name: final_image_name.clone(),
        file_path: final_image_path.to_string_lossy().into_owned(), // Absolute path
        last_modified: Utc::now().to_rfc3339(),
        title: String::new(),
        description: String::new(),
        summary: String::new(),
        duration_seconds: None,
        width: None, // image_handler.rs currently does not extract these on import
        height: None,
        frame_rate: None,
        bit_rate: None,
        audio_codec: None,
        video_codec: None,
        creation_time: None,
    };

    // relative_path_for_xml is already calculated and holds the image's relative path
    if let Err(e) = db_handler::save_asset_metadata(
        &file_metadata_for_db,
        &relative_path_for_xml, // This is the asset_relative_path for DB
        "image", // asset_type
        None,    // custom_fields_json is None on initial import
    ) {
        error!("[import_image_file] Failed to save image metadata to DB for {}: {}", relative_path_for_xml, e);
        return Err(CommandError::from(format!("Failed to save image metadata to DB: {}", e)));
    }
    info!("[import_image_file] Saved image metadata to DB for: {}", relative_path_for_xml);

    // Annotation JSO file creation is removed, DB will handle annotations.

    // Return the absolute path of the newly imported image
    Ok(final_image_path.to_string_lossy().to_string())
}

// --- NEW ANNOTATION COMMANDS ---

#[tauri::command]
pub async fn load_image_annotations(image_relative_path_str: String) -> Result<Option<String>, CommandError> {
    use crate::projectview::db_handler::load_annotations_from_db;
    // log macros info, error, debug are already imported at the top of the file.

    info!("[DB Image Annots] Loading for image: {}", image_relative_path_str);
    match load_annotations_from_db(&image_relative_path_str, "image") {
        Ok(Some(content)) => Ok(Some(content)),
        Ok(None) => Ok(None),
        Err(e) => {
            error!("[DB Image Annots] Error loading annotations for {}: {}", image_relative_path_str, e);
            Err(CommandError::from(format!("Failed to load image annotations from DB: {}", e)))
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
    use crate::projectview::shared_types::{ProjectXml, ImageEntryXml, FileMetadata}; // FileMetadata for type hints
    use crate::projectview::db_handler::{self as test_db_handler, FileMetadataWithCustomFieldsFromDb};
    // Assuming get_config_dir can be influenced for tests, or db_handler is refactored for testability.
    // For this example, we'll assume db_handler::init_db() can be made to work with a temp DB path.

    // Helper to create a dummy project.xml for testing
    fn create_dummy_project_xml_for_image_test(project_dir: &Path, project_name: &str) -> PathBuf {
        let project_xml_path = project_dir.join("project.xml");
        let project_data = ProjectXml {
            project_name: project_name.to_string(),
            project_uuid: "test-image-uuid".to_string(),
            project_root_is_single_file: false,
            video_files: Default::default(),
            audio_files: Default::default(),
            image_files: Default::default(), // Initialize as empty
            document_files: Default::default(),
            table_files: Default::default(),
            other_files: Default::default(),
            imported_transcript_files: Default::default(),
            document_metadata_files: Default::default(), // Should remain empty for image metadata
            chat_files: Default::default(),
            project_settings: Default::default(),
            saved_searches: Default::default(),
            project_tags: Default::default(),
            project_people: Default::default(),
            project_places: Default::default(),
            project_organizations: Default::default(),
            project_highlights_config: Default::default(),
            project_highlights_filters: Default::default(),
            project_highlights_summary_types: Default::default(),
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

    #[tokio::test]
    async fn test_import_image_file_with_db_metadata() -> Result<(), Box<dyn std::error::Error>> {
        let temp_project_dir = tempdir()?;
        let project_base_path = temp_project_dir.path();

        // Setup temp config dir for test DB
        let temp_config_dir = tempdir()?;
        let harvey_test_config_path = temp_config_dir.path().join(".harvey_test_config_image_handler"); // Unique name
        fs::create_dir_all(&harvey_test_config_path)?;
        std::env::set_var("HARVEY_TEST_CONFIG_DIR", harvey_test_config_path.to_str().unwrap());

        test_db_handler::init_db().expect("Failed to init test DB");

        // Create dummy project.xml
        let project_xml_path = create_dummy_project_xml_for_image_test(project_base_path, "TestImageProject");
        let project_xml_path_str = project_xml_path.to_string_lossy().to_string();

        // Create dummy source image file
        let source_image_dir = temp_project_dir.path().join("source_images");
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
        let expected_relative_path = final_image_abs_path.strip_prefix(project_base_path)?.to_string_lossy().replace("\\", "/");

        assert_eq!(updated_project_data.image_files.files.len(), 1, "Should be one image file in XML");
        let image_entry_xml = updated_project_data.image_files.files.get(0).unwrap();
        assert_eq!(image_entry_xml.name, expected_image_name);
        assert_eq!(image_entry_xml.relative_path, expected_relative_path);
        assert!(updated_project_data.document_metadata_files.files.is_empty(), "document_metadata_files should be empty in XML regarding this image");

        // Database Assertions
        let loaded_meta_option = test_db_handler::load_asset_metadata(&expected_relative_path)
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

#[tauri::command]
pub async fn save_image_annotations(image_relative_path_str: String, annotations_json_string: String) -> Result<(), CommandError> {
    use crate::projectview::db_handler::save_annotations_to_db;
    // log macros info, error, debug, warn are already imported at the top of the file.

    info!("[DB Image Annots] Saving for image: {}", image_relative_path_str);

    // Basic JSON validation before saving to DB (optional but good practice)
    if serde_json::from_str::<serde_json::Value>(&annotations_json_string).is_err() {
        warn!("[DB Image Annots] Annotation JSON content for {} appears invalid. Saving anyway.", image_relative_path_str);
    }

    match save_annotations_to_db(&image_relative_path_str, &annotations_json_string, "image") {
        Ok(_) => {
            info!("[DB Image Annots] Annotations saved successfully for {}.", image_relative_path_str);
            Ok(())
        },
        Err(e) => {
            error!("[DB Image Annots] Error saving annotations for {}: {}", image_relative_path_str, e);
            Err(CommandError::from(format!("Failed to save image annotations to DB: {}", e)))
        }
    }
}