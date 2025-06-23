// src-tauri/src/projectview/table_handler.rs
use super::shared_types::*;
use super::shared_utils::{save_project_xml, ensure_base_asset_dirs, truncate_filename_stem, MAX_FILENAME_STEM_LENGTH};
use crate::welcome::config::CommandError;
use crate::projectview::db_handler;
use chrono::Utc;
use serde_json;
use log::{info, warn, debug, error}; // Added error
use std::{
    fs,
    path::{Path, PathBuf},
};
use quick_xml;
use serde_json::{Value, json};
use csv;
// Import Data and ExcelDateTime explicitly.
// Error as CalamineError might be needed if there's ambiguity, but usually not if not aliased.
use calamine::{Reader, Xlsx, open_workbook, Data};

// StandardAssetMetadata import removed, FileMetadata is already in shared_types::*
// Helper to get a unique path in the Tables directory (Unchanged)
fn get_unique_table_path(
    project_base_dir: &Path,
    base_name: &str,
    extension: &str,
) -> Result<PathBuf, CommandError> {
    let target_dir = project_base_dir.join(HARVEY_FILES_DIR).join(TABLES_DIR);

    if !target_dir.exists() {
        warn!("Target tables directory {} not found. Attempting to create.", target_dir.display());
        fs::create_dir_all(&target_dir)?;
        info!("Created tables directory: {}", target_dir.display());
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
            debug!("Found unique table path: {}", target_path.display());
            return Ok(target_path);
        }
        counter += 1;
        if counter > 1000 {
            return Err(CommandError::from(format!("Could not find unique filename for table base '{}' after {} attempts.", base_name, counter)));
        }
    }
}


// Import command (Modified)
#[tauri::command]
pub async fn import_table_file(
    source_path_str: String,
    project_xml_path_str: String,
) -> Result<String, CommandError> {
    info!("[import_table_file] Importing table from: {}", source_path_str);
    let source_path = PathBuf::from(&source_path_str);
    let project_xml_path = PathBuf::from(&project_xml_path_str);

    if !source_path.exists() || !source_path.is_file() {
        return Err(CommandError::from(format!("Source table file not found: {}", source_path_str)));
    }

    let project_base_dir = project_xml_path.parent()
        .ok_or_else(|| CommandError::from("Could not get project base directory from XML path"))?;

    ensure_base_asset_dirs(project_base_dir)?;

    // Read project_uuid from XML
    let project_xml_content_for_uuid = fs::read_to_string(&project_xml_path)
        .map_err(|e| CommandError::Io(format!("Failed to read project XML for UUID from {}: {}", project_xml_path.display(), e)))?;
    let project_data_for_uuid: ProjectXml = quick_xml::de::from_str(&project_xml_content_for_uuid)
        .map_err(|e| CommandError::XmlDeserialization(format!("Failed to parse project XML for UUID from {}: {}", project_xml_path.display(), e)))?;

    let project_id_for_db = project_data_for_uuid.project_uuid;
    if project_id_for_db.is_empty() {
        error!("[import_table_file] Project UUID is empty in XML file: {}. Cannot import table without project_id.", project_xml_path.display());
        return Err(CommandError::Message(format!("Project ID (UUID) is missing in the project file ({}). Table import cannot proceed.", project_xml_path.display())));
    }

    let original_source_filename_with_ext = source_path.file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| CommandError::from("Could not get original table filename with extension"))?
        .to_string();

    let original_source_extension = source_path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    if original_source_extension != "csv" && original_source_extension != "xlsx" {
        return Err(CommandError::from(format!("Unsupported table file type: .{}", original_source_extension)));
    }

    // Truncate the original filename's stem
    let truncated_table_filename_with_ext = truncate_filename_stem(&original_source_filename_with_ext, MAX_FILENAME_STEM_LENGTH);
    info!("[import_table_file] Original filename: '{}', Truncated filename for project: '{}'", original_source_filename_with_ext, truncated_table_filename_with_ext);

    let table_file_stem_truncated = Path::new(&truncated_table_filename_with_ext).file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| CommandError::from(format!("Could not get stem from truncated table filename: {}", truncated_table_filename_with_ext)))?;

    // Create a folder under Tables named after the truncated file stem
    let tables_base = project_base_dir.join(HARVEY_FILES_DIR).join(TABLES_DIR);
    let folder_path = tables_base.join(table_file_stem_truncated); // Folder uses truncated stem

    // Check for existing folder based on truncated stem. If it exists and is for a different original file, this is a collision.
    // The current logic of `get_unique_table_path` or the loop below might need adjustment if strict collision on folder name is desired.
    // For now, if folder exists, we'll try to place the file inside, potentially with a numeric suffix if the exact truncated filename exists.
    if !folder_path.exists() {
        fs::create_dir_all(&folder_path)?;
    }

    // Determine unique filename *inside* the (potentially truncated stem) folder, using the truncated filename as base.
    let mut counter = 0;
    let final_table_path = loop {
        let file_name_to_try = if counter == 0 {
            truncated_table_filename_with_ext.clone()
        } else {
            // If collision, append suffix to the *truncated stem* part, then add extension
            format!("{}_{}.{}", table_file_stem_truncated, counter, original_source_extension)
        };
        let candidate = folder_path.join(&file_name_to_try);
        if !candidate.exists() {
            break candidate;
        }
        counter += 1;
        if counter > 1000 { // Safety break
            return Err(CommandError::from(format!(
                "Could not find unique filename for table base '{}' (derived from truncated name) after {} attempts.",
                table_file_stem_truncated, counter
            )));
        }
    };

    // final_table_name is the name of the file as it will be saved (e.g., truncated_stem.csv or truncated_stem_1.csv)
    let final_table_name = final_table_path.file_name().unwrap().to_string_lossy().into_owned();

    info!("[import_table_file] Copying table from '{}' to '{}'", source_path.display(), final_table_path.display());
    fs::copy(&source_path, &final_table_path).map_err(|e| CommandError::from(format!("Failed to copy table file: {}", e)))?;

    info!("[import_table_file] Updating project XML to include table: {}", final_table_name);
    let xml_content = fs::read_to_string(&project_xml_path)?;
    let mut project_data: ProjectXml = quick_xml::de::from_str(&xml_content)?;

    let relative_path_for_xml = final_table_path // Path uses (potentially suffixed) truncated name
        .strip_prefix(project_base_dir)?
        .to_string_lossy()
        .replace("\\", "/");

    let new_table_entry = TableEntryXml {
        name: final_table_name.clone(), // XML name is the final (potentially suffixed) truncated filename
        relative_path: relative_path_for_xml.clone(),
    };

    if project_data.table_files.files.iter().any(|f| f.relative_path == relative_path_for_xml) {
        warn!("[import_table_file] Table with relative path '{}' already exists in XML. Overwriting name if different.", relative_path_for_xml);
        if let Some(existing_entry) = project_data.table_files.files.iter_mut().find(|f| f.relative_path == relative_path_for_xml) {
            existing_entry.name = final_table_name.clone();
        }
    } else {
        project_data.table_files.files.push(new_table_entry);
    }
    project_data.table_files.files.sort_by(|a, b| a.name.cmp(&b.name));

    save_project_xml(&project_xml_path, &project_data)?;
    info!("[import_table_file] Project XML updated successfully for table.");

    // Save metadata to DB
    let file_metadata_for_db = FileMetadata {
        file_name: final_table_name.clone(), // Use final (potentially suffixed) truncated filename
        file_path: final_table_path.to_string_lossy().into_owned(), // Absolute path uses final name
        last_modified: Utc::now().to_rfc3339(),
        title: String::new(), // Init empty
        description: String::new(), // Init empty
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
    };

    // relative_path_for_xml is already calculated and is the correct key for the DB
    if let Err(e) = db_handler::save_asset_metadata(
        &project_id_for_db, // Pass project_id
        &file_metadata_for_db,
        &relative_path_for_xml, // DB key uses final (potentially suffixed) truncated name based path
        "table", // asset_type
        None, // custom_fields_json
    ) {
        error!("[import_table_file] Failed to save table metadata to DB for table '{}' (path: {}, project_id: {}): {}", final_table_name, relative_path_for_xml, project_id_for_db, e);
        // Attempt to clean up copied file if DB save fails? Or let it be and user can delete.
        // For now, return error.
        return Err(e);
    }
    info!("[import_table_file] Saved table metadata to DB for: {} (project_id: {})", relative_path_for_xml, project_id_for_db);

    Ok(final_table_path.to_string_lossy().to_string()) // Return absolute path to the imported (potentially renamed) file
}


#[tauri::command]
pub async fn load_table_data(table_path_str: String) -> Result<Value, CommandError> {
    info!("[load_table_data] Loading data from: {}", table_path_str);
    let table_path = PathBuf::from(&table_path_str);

    if !table_path.exists() || !table_path.is_file() {
        return Err(CommandError::from(format!("Table file not found: {}", table_path_str)));
    }

    let extension = table_path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();

    let data = match extension.as_str() {
        "csv" => load_csv_data(&table_path),
        "xlsx" => load_xlsx_data(&table_path),
        _ => Err(CommandError::from(format!("Unsupported table extension for loading: {}", extension))),
    }?;

    debug!("[load_table_data] Successfully loaded {} rows.", data.as_array().map_or(0, |a| a.len()));
    Ok(data)
}

fn load_csv_data(path: &Path) -> Result<Value, CommandError> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .map_err(|e| CommandError::from(format!("Failed to open CSV '{}': {}", path.display(), e)))?;

    let headers = rdr.headers()
        .map_err(|e| CommandError::from(format!("Failed to read CSV headers '{}': {}", path.display(), e)))?
        .iter()
        .map(|h| h.to_string())
        .collect::<Vec<String>>();

    let mut records = Vec::new();
    for result in rdr.records() {
        let record = result.map_err(|e| CommandError::from(format!("Failed to read CSV record '{}': {}", path.display(), e)))?;
        let mut map = serde_json::Map::new();
        for (i, header) in headers.iter().enumerate() {
            let value_str = record.get(i).unwrap_or("").trim();
            let value_json = if let Ok(num) = value_str.parse::<f64>() {
                json!(num)
            } else if let Ok(b) = value_str.parse::<bool>() {
                 json!(b)
            } else {
                json!(value_str)
            };
            map.insert(header.clone(), value_json);
        }
        records.push(Value::Object(map));
    }

    Ok(Value::Array(records))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use crate::projectview::shared_types::{ProjectXml, TableEntryXml, FileMetadata}; // Added FileMetadata
    use crate::projectview::db_handler::{self, FileMetadataWithCustomFieldsFromDb}; // Added db_handler itself and the specific struct for loading
    use crate::welcome::config; // To potentially influence config dir

    // Helper to create a dummy project.xml for testing
    fn create_dummy_project_xml(project_dir: &Path, project_name: &str) -> PathBuf {
        let project_xml_path = project_dir.join("project.xml");
        let project_data = ProjectXml {
            project_name: project_name.to_string(),
            project_uuid: "test-uuid".to_string(),
            project_root_is_single_file: false, // Default for this kind of project
            video_files: Default::default(),
            audio_files: Default::default(),
            image_files: Default::default(),
            document_files: Default::default(),
            table_files: Default::default(),
            other_files: Default::default(),
            imported_transcript_files: Default::default(),
            document_metadata_files: Default::default(),
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

    #[tokio::test]
    async fn test_import_table_file_with_db_metadata() -> Result<(), Box<dyn std::error::Error>> {
        let temp_project_dir = tempdir()?;
        let project_base_path = temp_project_dir.path();

        // 1. Setup: Create a temporary config directory for the test DB
        let temp_config_dir = tempdir()?;
        let temp_db_dir_for_test = temp_config_dir.path().join(".harvey"); // Mimic structure if get_config_dir expects .harvey
        fs::create_dir_all(&temp_db_dir_for_test)?;

        // THIS IS THE CRITICAL PART: Forcing get_config_dir to use our temp path.
        // This needs support from get_config_dir, e.g., checking an env var.
        // For this test, we'll set an env var that `get_config_dir` in `welcome/config.rs`
        // would need to be modified to respect for testing purposes.
        // Example: `HARVEY_TEST_CONFIG_DIR`
        std::env::set_var("HARVEY_TEST_CONFIG_DIR", temp_config_dir.path().to_str().unwrap());

        // Initialize the test DB using the actual db_handler::init_db()
        // This will now create the DB in our temp_config_dir if the env var is respected.
        db_handler::init_db().expect("Failed to initialize test DB");


        // Create dummy project.xml
        let project_xml_path = create_dummy_project_xml(project_base_path, "TestTableProject");
        let project_xml_path_str = project_xml_path.to_string_lossy().to_string();

        // Create dummy source table file
        let source_table_dir = temp_project_dir.path().join("source_tables");
        fs::create_dir_all(&source_table_dir)?;
        let dummy_table_path = source_table_dir.join("dummy_table.csv");
        let mut source_file = File::create(&dummy_table_path)?;
        writeln!(source_file, "header1,header2\nval1,val2")?;
        let dummy_table_path_str = dummy_table_path.to_string_lossy().to_string();

        // 2. Execute import_table_file
        let import_result = import_table_file(dummy_table_path_str.clone(), project_xml_path_str.clone()).await;
        assert!(import_result.is_ok(), "import_table_file failed: {:?}", import_result.err());
        let final_table_abs_path_str = import_result.unwrap();
        let final_table_abs_path = PathBuf::from(&final_table_abs_path_str);

        // 3. File System Assertions
        assert!(final_table_abs_path.exists(), "Imported table file should exist at {}", final_table_abs_path_str);

        // Assert that .metadata.json file does NOT exist
        let parent_dir = final_table_abs_path.parent().unwrap();
        let stem = final_table_abs_path.file_stem().unwrap().to_str().unwrap();
        let metadata_json_path = parent_dir.join(format!(".{}.metadata.json", stem));
        assert!(!metadata_json_path.exists(), ".metadata.json file should NOT exist at {}", metadata_json_path.display());

        // 4. XML Assertions
        let updated_xml_content = fs::read_to_string(&project_xml_path)?;
        let updated_project_data: ProjectXml = quick_xml::de::from_str(&updated_xml_content)?;

        let expected_table_name = final_table_abs_path.file_name().unwrap().to_str().unwrap();
        let expected_relative_path = final_table_abs_path.strip_prefix(project_base_path)?.to_string_lossy().replace("\\", "/");

        assert_eq!(updated_project_data.table_files.files.len(), 1, "Should be one table file in XML");
        let table_entry_xml = updated_project_data.table_files.files.get(0).unwrap();
        assert_eq!(table_entry_xml.name, expected_table_name);
        assert_eq!(table_entry_xml.relative_path, expected_relative_path);
        assert!(updated_project_data.document_metadata_files.files.is_empty(), "document_metadata_files should be empty in XML");

        // 5. Database Assertions
        let loaded_meta_option = db_handler::load_asset_metadata(&expected_relative_path)
            .expect("Failed to load metadata from DB for assertion");

        assert!(loaded_meta_option.is_some(), "Metadata should be found in DB for relative path: {}", expected_relative_path);
        if let Some(loaded_meta) = loaded_meta_option {
            assert_eq!(loaded_meta.file_name, expected_table_name);
            assert_eq!(loaded_meta.file_path, final_table_abs_path_str); // DB stores absolute path here
            assert_eq!(loaded_meta.asset_type, "table");
            assert_eq!(loaded_meta.title.unwrap_or_default(), "");
            assert!(loaded_meta.custom_fields_json.is_none());
        }

        // Cleanup the env var
        std::env::remove_var("HARVEY_TEST_CONFIG_DIR");
        Ok(())
    }
}

fn load_xlsx_data(path: &Path) -> Result<Value, CommandError> {
    let mut workbook: Xlsx<_> = open_workbook(path)
        .map_err(|e| CommandError::from(format!("Failed to open XLSX '{}': {}", path.display(), e)))?;

    let sheet_name = workbook.sheet_names().first().cloned()
        .ok_or_else(|| CommandError::from("XLSX file contains no sheets."))?;

    info!("[load_xlsx_data] Reading sheet: {}", sheet_name);

    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|e| CommandError::from(format!("Calamine error reading sheet '{}': {}", sheet_name, e)))?;

    let mut records = Vec::new();
    let mut headers: Vec<String> = Vec::new();

    for (row_idx, row) in range.rows().enumerate() {
        if row_idx == 0 { // Assume first row is header
            headers = row.iter()
                .enumerate()
                .map(|(col_idx, cell)| {
                    match cell {
                        Data::String(s) => s.trim().to_string(),
                        Data::Float(f) => f.to_string(),
                        Data::Int(i) => i.to_string(),
                        Data::Bool(b) => b.to_string(),
                        Data::DateTime(excel_dt_struct) => {
                            // Try full datetime first, then fallback to raw serial number
                            if let Some(dt) = excel_dt_struct.as_datetime() {
                                dt.to_string()
                            } else {
                                excel_dt_struct.as_f64().to_string()
                            }
                        }
                        Data::DateTimeIso(s) => s.clone(),
                        Data::DurationIso(s) => s.clone(),
                        Data::Error(e) => format!("Error:{:?}", e),
                        Data::Empty => format!("Column_{}", col_idx + 1),
                    }
                })
                .collect();
            debug!("[load_xlsx_data] Headers: {:?}", headers);
        } else { // Data rows
            let mut map = serde_json::Map::new();
            let mut row_has_data = false;
            for (col_idx, cell) in row.iter().enumerate() {
                let header = headers.get(col_idx).cloned().unwrap_or_else(|| format!("Column_{}", col_idx + 1));
                let value_json = match cell {
                    Data::String(s) => { row_has_data = true; json!(s.trim()) },
                    Data::Float(f) => { row_has_data = true; json!(f) },
                    Data::Int(i) => { row_has_data = true; json!(i) },
                    Data::Bool(b) => { row_has_data = true; json!(b) },
                    Data::DateTime(excel_dt_struct) => {
                        row_has_data = true;
                        // Try full datetime first, then fallback to raw serial number
                        if let Some(dt) = excel_dt_struct.as_datetime() {
                            json!(dt.to_string())
                        } else {
                            json!(excel_dt_struct.as_f64())
                        }
                    }
                    Data::DateTimeIso(s) => { row_has_data = true; json!(s) },
                    Data::DurationIso(s) => { row_has_data = true; json!(s) },
                    Data::Error(_) => json!(null),
                    Data::Empty => json!(null),
                };
                map.insert(header, value_json);
            }
            if row_has_data {
                 records.push(Value::Object(map));
            }
        }
    }
    Ok(Value::Array(records))
}