use super::shared_types::*;
use super::shared_utils::{save_project_xml, ensure_base_asset_dirs, truncate_filename_stem, MAX_FILENAME_STEM_LENGTH, get_project_xml_path_from_item};
use crate::welcome::config::CommandError;
use crate::projectview::db_handler;
use chrono::Utc;
use serde_json;
use log::{info, debug, error};
use std::{
    fs,
    path::{Path, PathBuf},
};
use quick_xml;
use serde_json::{Value, json};
use csv;
use calamine::{Reader, Xlsx, open_workbook, Data};

#[tauri::command]
pub async fn import_table_file(
    source_path_str: String,
    project_xml_path_str: String,
) -> Result<Value, CommandError> {
    info!("[import_table_file] Importing table from: {}, Project XML Path: {}", source_path_str, project_xml_path_str);
    let source_path = PathBuf::from(&source_path_str);
    let project_xml_path = PathBuf::from(&project_xml_path_str);

    if !source_path.exists() || !source_path.is_file() {
        error!("[import_table_file] Source table file not found: {}", source_path_str);
        return Err(CommandError::from(format!("Source table file not found: {}", source_path_str)));
    }
    debug!("[import_table_file] Source file exists: {}", source_path_str);

    let project_base_dir = project_xml_path.parent()
        .ok_or_else(|| {
            error!("[import_table_file] Could not get project base directory from XML path: {}", project_xml_path.display());
            CommandError::from("Could not get project base directory from XML path")
        })?;
    debug!("[import_table_file] Project base directory: {}", project_base_dir.display());

    ensure_base_asset_dirs(project_base_dir)?;
    debug!("[import_table_file] Base asset directories ensured.");

    let project_xml_content_for_uuid = fs::read_to_string(&project_xml_path)
        .map_err(|e| {
            error!("[import_table_file] Failed to read project XML for UUID from {}: {}", project_xml_path.display(), e);
            CommandError::Io(format!("Failed to read project XML for UUID from {}: {}", project_xml_path.display(), e))
        })?;
    let project_data_for_uuid: ProjectXml = quick_xml::de::from_str(&project_xml_content_for_uuid)
        .map_err(|e| {
            error!("[import_table_file] Failed to parse project XML for UUID from {}: {}", project_xml_path.display(), e);
            CommandError::XmlDeserialization(format!("Failed to parse project XML for UUID from {}: {}", project_xml_path.display(), e))
        })?;
    debug!("[import_table_file] Project XML parsed for UUID.");

    let project_id_for_db = project_data_for_uuid.project_uuid;
    if project_id_for_db.is_empty() {
        error!("[import_table_file] Project UUID is empty in XML file: {}. Cannot import table without project_id.", project_xml_path.display());
        return Err(CommandError::Message(format!("Project ID (UUID) is missing in the project file ({}). Table import cannot proceed.", project_xml_path.display())))
    }
    debug!("[import_table_file] Project ID for DB: {}", project_id_for_db);

    let original_source_filename_with_ext = match source_path.file_name().and_then(|s| s.to_str()) {
        Some(s) => s.to_string(),
        None => {
            error!("[import_table_file] Could not get original table filename with extension from: {}", source_path.display());
            return Err(CommandError::from("Could not get original table filename with extension"));
        }
    };
    debug!("[import_table_file] Original source filename: {}", original_source_filename_with_ext);

    let original_source_extension = source_path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    debug!("[import_table_file] Original source extension: {}", original_source_extension);

    if original_source_extension != "csv" && original_source_extension != "xlsx" {
        error!("[import_table_file] Unsupported table file type: .{}", original_source_extension);
        return Err(CommandError::from(format!("Unsupported table file type: .{}", original_source_extension)));
    }

    let truncated_table_filename_with_ext = truncate_filename_stem(&original_source_filename_with_ext, MAX_FILENAME_STEM_LENGTH);
    info!("[import_table_file] Original filename: '{}', Truncated filename for project: '{}'", original_source_filename_with_ext, truncated_table_filename_with_ext);

    let table_file_stem_truncated = Path::new(&truncated_table_filename_with_ext).file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            error!("[import_table_file] Could not get stem from truncated table filename: {}", truncated_table_filename_with_ext);
            CommandError::from(format!("Could not get stem from truncated table filename: {}", truncated_table_filename_with_ext))
        })?;
    debug!("[import_table_file] Truncated table file stem: {}", table_file_stem_truncated);

    let tables_base = project_base_dir.join(HARVEY_FILES_DIR).join(TABLES_DIR);
    let folder_path = tables_base.join(table_file_stem_truncated);
    debug!("[import_table_file] Target folder path: {}", folder_path.display());

    if !folder_path.exists() {
        fs::create_dir_all(&folder_path)
            .map_err(|e| {
                error!("[import_table_file] Failed to create directory {}: {}", folder_path.display(), e);
                CommandError::Io(format!("Failed to create directory {}: {}", folder_path.display(), e))
            })?;
        debug!("[import_table_file] Created folder: {}", folder_path.display());
    }

    let mut counter = 0;
    let final_table_path = loop {
        let file_name_to_try = if counter == 0 {
            truncated_table_filename_with_ext.clone()
        } else {
            format!("{}_{}.{}", table_file_stem_truncated, counter, original_source_extension)
        };
        let candidate = folder_path.join(&file_name_to_try);
        if !candidate.exists() {
            debug!("[import_table_file] Found unique filename: {}", candidate.display());
            break candidate;
        }
        counter += 1;
        if counter > 1000 {
            error!("[import_table_file] Could not find unique filename for table base '{}' after {} attempts.", table_file_stem_truncated, counter);
            return Err(CommandError::from(format!(
                "Could not find unique filename for table base '{}' (derived from truncated name) after {} attempts.",
                table_file_stem_truncated, counter
            )));
        }
    };

    let final_table_name = final_table_path.file_name().unwrap().to_string_lossy().into_owned();
    debug!("[import_table_file] Final table name: {}", final_table_name);

    info!("[import_table_file] Copying table from '{}' to '{}'", source_path.display(), final_table_path.display());
    fs::copy(&source_path, &final_table_path).map_err(|e| {
        error!("[import_table_file] Failed to copy table file from {} to {}: {}", source_path.display(), final_table_path.display(), e);
        CommandError::from(format!("Failed to copy table file: {}", e))
    })?;
    debug!("File copied to: {}", final_table_path.display());

    info!("[import_table_file] Updating project XML to include table: {}", final_table_name);
    let xml_content = fs::read_to_string(&project_xml_path)
        .map_err(|e| {
            error!("[import_table_file] Failed to read project XML for update from {}: {}", project_xml_path.display(), e);
            CommandError::Io(format!("Failed to read project XML for update from {}: {}", project_xml_path.display(), e))
        })?;
    let mut project_data: ProjectXml = quick_xml::de::from_str(&xml_content)
        .map_err(|e| {
            error!("[import_table_file] Failed to parse project XML for update from {}: {}", project_xml_path.display(), e);
            CommandError::XmlDeserialization(format!("Failed to parse project XML for update from {}: {}", project_xml_path.display(), e))
        })?;
    debug!("[import_table_file] Project XML parsed for update.");

    let relative_path_for_xml = final_table_path
        .strip_prefix(project_base_dir)
        .map_err(|e| {
            error!("[import_table_file] Failed to strip prefix {} from {}: {}", project_base_dir.display(), final_table_path.display(), e);
            CommandError::Path(format!("Failed to get relative path for XML: {}", e))
        })?
        .to_string_lossy()
        .replace("\\", "/");
    debug!("[import_table_file] Relative path for XML: {}", relative_path_for_xml);

    let new_table_entry = TableEntryXml {
        name: final_table_name.clone(),
        relative_path: relative_path_for_xml.clone(),
        language_code: None,
        has_headers: None, // Initially unknown
    };
    debug!("[import_table_file] New table entry created.");

    if project_data.table_files.files.iter().any(|f| f.relative_path == new_table_entry.relative_path) {
        log::warn!("[import_table_file] Table with relative path '{}' already exists in XML. Overwriting name if different.", new_table_entry.relative_path);
        if let Some(existing_entry) = project_data.table_files.files.iter_mut().find(|f| f.relative_path == new_table_entry.relative_path) {
            existing_entry.name = new_table_entry.name.clone();
            existing_entry.has_headers = new_table_entry.has_headers;
        }
    } else {
        project_data.table_files.files.push(new_table_entry);
    }
    project_data.table_files.files.sort_by(|a, b| a.name.cmp(&b.name));
    debug!("[import_table_file] Project data updated with new table entry.");

    save_project_xml(&project_xml_path, &project_data)
        .map_err(|e| {
            error!("[import_table_file] Failed to save project XML {}: {}", project_xml_path.display(), e);
            e
        })?;
    info!("[import_table_file] Project XML updated successfully for table.");

    let file_metadata_for_db = FileMetadata {
        file_name: final_table_name.clone(),
        file_path: final_table_path.to_string_lossy().into_owned(),
        last_modified: Utc::now().to_rfc3339(),
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
        created_at: Some(Utc::now().to_rfc3339()),
        original_import_path: Some(source_path_str.clone()),
        speaker_names: None,
        waveform_data: None,
    };
    debug!("[import_table_file] File metadata for DB created.");

    if let Err(e) = db_handler::save_asset_metadata(
        &project_id_for_db,
        &file_metadata_for_db,
        &relative_path_for_xml,
        "table",
        None,
    ) {
        error!("[import_table_file] Failed to save table metadata to DB for table '{}' (path: {}, project_id: {}): {}", final_table_name, relative_path_for_xml, project_id_for_db, e);
        return Err(e);
    }
    info!("[import_table_file] Saved table metadata to DB for: {} (project_id: {})", relative_path_for_xml, project_id_for_db);

    // Always assume headers for the preview, the user will confirm.
    let preview_data = match original_source_extension.as_str() {
        "csv" => load_csv_data(&final_table_path, true, Some(5)),
        "xlsx" => load_xlsx_data(&final_table_path, true, Some(5)),
        _ => {
            error!("[import_table_file] Unsupported table extension for preview: {}", original_source_extension);
            return Err(CommandError::from(format!("Unsupported table extension for preview: {}", original_source_extension)))
        },
    }?;
    debug!("[import_table_file] Preview data loaded.");

    Ok(json!({
        "table_path": final_table_path.to_string_lossy(),
        "preview_data": preview_data
    }))
}

#[tauri::command]
pub async fn set_table_headers(
    table_path_str: String,
    has_headers: bool,
) -> Result<(), CommandError> {
    info!("[set_table_headers] Setting has_headers={} for table: {}", has_headers, table_path_str);
    let table_path = PathBuf::from(&table_path_str);
    let project_xml_path = get_project_xml_path_from_item(&table_path)?;

    if !table_path.exists() {
        error!("[set_table_headers] Table file does not exist at: {}", table_path.display());
        return Err(CommandError::from(format!("Table file not found: {}", table_path_str)));
    }
    if !project_xml_path.exists() {
        error!("[set_table_headers] Project XML file does not exist at: {}", project_xml_path.display());
        return Err(CommandError::from(format!("Project XML file not found: {}", project_xml_path.to_string_lossy())));
    }

    let project_base_dir = project_xml_path.parent()
        .ok_or_else(|| CommandError::from("Could not get project base directory from XML path"))?;

    let relative_path_for_xml = table_path
        .strip_prefix(project_base_dir)?
        .to_string_lossy()
        .replace("\\", "/");

    let mut project_data: ProjectXml = {
        let xml_content = fs::read_to_string(&project_xml_path)?;
        quick_xml::de::from_str(&xml_content)?
    };

    if let Some(table_entry) = project_data.table_files.files.iter_mut()
        .find(|f| f.relative_path == relative_path_for_xml) {
        table_entry.has_headers = Some(has_headers);
    } else {
        return Err(CommandError::from(format!("Table not found in project XML: {}", relative_path_for_xml)));
    }

    save_project_xml(&project_xml_path, &project_data)?;
    info!("[set_table_headers] Project XML updated successfully.");

    Ok(())
}

#[tauri::command]
pub async fn load_table_data(table_path_str: String) -> Result<Value, CommandError> {
    info!("[load_table_data] Loading data from: {}", table_path_str);
    let table_path = PathBuf::from(&table_path_str);

    if !table_path.exists() || !table_path.is_file() {
        error!("[load_table_data] Table file not found or is not a file: {}", table_path.display());
        return Err(CommandError::from(format!("Table file not found: {}", table_path_str)));
    }

    let project_xml_path = get_project_xml_path_from_item(&table_path)?;
    debug!("[load_table_data] Derived project_xml_path: {}", project_xml_path.display());

    if !project_xml_path.exists() || !project_xml_path.is_file() {
        error!("[import_table_file] Project XML file not found at: {}", project_xml_path.display());
        return Err(CommandError::from(format!("Project XML file not found: {}", project_xml_path.to_string_lossy())));
    }

    let relative_path_for_xml = table_path
        .strip_prefix(project_xml_path.parent().ok_or_else(|| CommandError::from("Could not get project base directory from project XML path"))?)?
        .to_string_lossy()
        .replace("\\", "/");

    let project_data: ProjectXml = {
        let xml_content = fs::read_to_string(&project_xml_path)?;
        quick_xml::de::from_str(&xml_content)?
    };

    let has_headers = project_data.table_files.files.iter()
        .find(|f| f.relative_path == relative_path_for_xml)
        .and_then(|f| f.has_headers)
        .unwrap_or(true); // Default to true if not specified

    let extension = table_path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();

    let data = match extension.as_str() {
        "csv" => load_csv_data(&table_path, has_headers, None),
        "xlsx" => load_xlsx_data(&table_path, has_headers, None),
        _ => Err(CommandError::from(format!("Unsupported table extension for loading: {}", extension))),
    }?;

    debug!("[load_table_data] Successfully loaded {} rows.", data.as_array().map_or(0, |a| a.len()));
    Ok(data)
}

fn to_json_response(headers: Vec<String>, records: Vec<Value>) -> Result<Value, CommandError> {
    Ok(json!(records))
}

fn load_csv_data(path: &Path, has_headers: bool, limit: Option<usize>) -> Result<Value, CommandError> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(has_headers)
        .from_path(path)
        .map_err(|e| CommandError::from(format!("Failed to open CSV '{}': {}", path.display(), e)))?;

    let headers = if has_headers {
        rdr.headers()
            .map_err(|e| CommandError::from(format!("Failed to read CSV headers '{}': {}", path.display(), e)))?
            .iter()
            .map(|h| h.to_string())
            .collect::<Vec<String>>()
    } else {
        let mut temp_rdr = csv::ReaderBuilder::new().has_headers(false).from_path(path)?;
        let first_record = temp_rdr.records().next().transpose()?.unwrap_or_default();
        let num_columns = first_record.len();
        (0..num_columns).map(|i| {
            let mut col_name = String::new();
            let mut n = i;
            loop {
                col_name.insert(0, (b'A' + (n % 26) as u8) as char);
                if n < 26 { break; }
                n = n / 26 - 1;
            }
            col_name
        }).collect()
    };

    let mut records = Vec::new();
    let records_iterator = rdr.records();
    let records_to_process = if let Some(l) = limit {
        records_iterator.take(l)
    } else {
        records_iterator.take(usize::MAX) // Effectively no limit
    };

    for result in records_to_process {
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

    to_json_response(headers, records)
}

fn load_xlsx_data(path: &Path, has_headers: bool, limit: Option<usize>) -> Result<Value, CommandError> {
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
    let mut data_rows = range.rows();

    if has_headers {
        if let Some(row) = data_rows.next() {
            headers = row.iter()
                .enumerate()
                .map(|(col_idx, cell)| {
                    match cell {
                        Data::String(s) => s.trim().to_string(),
                        Data::Float(f) => f.to_string(),
                        Data::Int(i) => i.to_string(),
                        Data::Bool(b) => b.to_string(),
                        Data::DateTime(excel_dt_struct) => {
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
        }
    } else {
        if let Some(first_row) = range.rows().next() {
            let num_columns = first_row.len();
            headers = (0..num_columns).map(|i| {
                let mut col_name = String::new();
                let mut n = i;
                loop {
                    col_name.insert(0, (b'A' + (n % 26) as u8) as char);
                    if n < 26 { break; }
                    n = n / 26 - 1;
                }
                col_name
            }).collect();
        }
    }

    let data_rows_iterator = range.rows();
    let data_rows_to_process = if let Some(l) = limit {
        data_rows_iterator.skip(if has_headers { 1 } else { 0 }).take(l)
    } else {
        data_rows_iterator.skip(if has_headers { 1 } else { 0 }).take(usize::MAX)
    };

    for row in data_rows_to_process {
        let mut map = serde_json::Map::new();
        let mut row_has_data = false;
        for (col_idx, cell) in row.iter().enumerate() {
            if let Some(header) = headers.get(col_idx) {
                let value_json = match cell {
                    Data::String(s) => { row_has_data = true; json!(s.trim()) },
                    Data::Float(f) => { row_has_data = true; json!(f) },
                    Data::Int(i) => { row_has_data = true; json!(i) },
                    Data::Bool(b) => { row_has_data = true; json!(b) },
                    Data::DateTime(excel_dt_struct) => {
                        row_has_data = true;
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
                map.insert(header.clone(), value_json);
            }
        }
        if row_has_data {
             records.push(Value::Object(map));
        }
    }
    to_json_response(headers, records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use crate::projectview::shared_types::{ProjectXml, TableEntryXml, FileMetadata};
    use crate::projectview::db_handler::{self, FileMetadataWithCustomFieldsFromDb};
    use crate::welcome::config;

    fn create_dummy_project_xml(project_dir: &Path, project_name: &str) -> PathBuf {
        let project_xml_path = project_dir.join("project.xml");
        let project_data = ProjectXml {
            project_name: project_name.to_string(),
            project_uuid: "test-uuid".to_string(),
            project_root_is_single_file: false,
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

        let temp_config_dir = tempdir()?;
        let temp_db_dir_for_test = temp_config_dir.path().join(".harvey");
        fs::create_dir_all(&temp_db_dir_for_test)?;

        std::env::set_var("HARVEY_TEST_CONFIG_DIR", temp_config_dir.path().to_str().unwrap());

        db_handler::init_db().expect("Failed to initialize test DB");

        let project_xml_path = create_dummy_project_xml(project_base_path, "TestTableProject");
        let project_xml_path_str = project_xml_path.to_string_lossy().to_string();

        let source_table_dir = temp_project_dir.path().join("source_tables");
        fs::create_dir_all(&source_table_dir)?;
        let dummy_table_path = source_table_dir.join("dummy_table.csv");
        let mut source_file = File::create(&dummy_table_path)?;
        writeln!(source_file, "header1,header2\nval1,val2")?;
        let dummy_table_path_str = dummy_table_path.to_string_lossy().to_string();

        let import_result = import_table_file(dummy_table_path_str.clone()).await;
        assert!(import_result.is_ok(), "import_table_file failed: {:?}", import_result.err());
        let final_table_abs_path_str = import_result.unwrap();
        let final_table_abs_path = PathBuf::from(&final_table_abs_path_str);

        assert!(final_table_abs_path.exists(), "Imported table file should exist at {}", final_table_abs_path_str);

        let parent_dir = final_table_abs_path.parent().unwrap();
        let stem = final_table_abs_path.file_stem().unwrap().to_str().unwrap();
        let metadata_json_path = parent_dir.join(format!(".{}.metadata.json", stem));
        assert!(!metadata_json_path.exists(), ".metadata.json file should NOT exist at {}", metadata_json_path.display());

        let updated_xml_content = fs::read_to_string(&project_xml_path)?;
        let updated_project_data: ProjectXml = quick_xml::de::from_str(&updated_xml_content)?;

        let expected_table_name = final_table_abs_path.file_name().unwrap().to_str().unwrap();
        let expected_relative_path = final_table_abs_path.strip_prefix(project_base_path)?.to_string_lossy().replace("\\", "/");

        assert_eq!(updated_project_data.table_files.files.len(), 1, "Should be one table file in XML");
        let table_entry_xml = updated_project_data.table_files.files.get(0).unwrap();
        assert_eq!(table_entry_xml.name, expected_table_name);
        assert_eq!(table_entry_xml.relative_path, expected_relative_path);
        assert!(updated_project_data.document_metadata_files.files.is_empty(), "document_metadata_files should be empty in XML");

        let loaded_meta_option = db_handler::load_asset_metadata(&expected_relative_path)
            .expect("Failed to load metadata from DB for assertion");

        assert!(loaded_meta_option.is_some(), "Metadata should be found in DB for relative path: {}", expected_relative_path);
        if let Some(loaded_meta) = loaded_meta_option {
            assert_eq!(loaded_meta.file_name, expected_table_name);
            assert_eq!(loaded_meta.file_path, final_table_abs_path_str);
            assert_eq!(loaded_meta.asset_type, "table");
            assert_eq!(loaded_meta.title.unwrap_or_default(), "");
            assert!(loaded_meta.custom_fields_json.is_none());
        }

        std::env::remove_var("HARVEY_TEST_CONFIG_DIR");
        Ok(())
    }
}