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
use rust_xlsxwriter::Workbook;

#[tauri::command]
pub async fn get_xlsx_sheets(
    source_path_str: String,
) -> Result<Vec<String>, CommandError> {
    let source_path = PathBuf::from(&source_path_str);
    if !source_path.exists() || !source_path.is_file() {
        return Err(CommandError::from(format!("Source file not found: {}", source_path_str)));
    }

    let mut workbook: Xlsx<_> = open_workbook(&source_path)
        .map_err(|e| CommandError::from(format!("Failed to open XLSX '{}': {}", source_path.display(), e)))?;

    Ok(workbook.sheet_names().to_vec())
}

#[tauri::command]
pub async fn import_table_file(
    source_path_str: String,
    project_xml_path_str: String,
    sheet_name_opt: Option<String>,
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

    let stem_base = if original_source_extension == "xlsx" && sheet_name_opt.is_some() {
        let original_stem = Path::new(&original_source_filename_with_ext).file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let combined = format!("{}_{}", original_stem, sheet_name_opt.as_ref().unwrap());
        truncate_filename_stem(&format!("{}.csv", combined), MAX_FILENAME_STEM_LENGTH)
    } else {
        truncate_filename_stem(&original_source_filename_with_ext, MAX_FILENAME_STEM_LENGTH)
    };

    let table_file_stem_truncated = Path::new(&stem_base).file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            error!("[import_table_file] Could not get stem from truncated table filename: {}", stem_base);
            CommandError::from(format!("Could not get stem from truncated table filename: {}", stem_base))
        })?;
    debug!("[import_table_file] Truncated table file stem: {}", table_file_stem_truncated);

    info!("[import_table_file] Updating project XML to include table: {}", original_source_filename_with_ext);
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

    let tables_base = project_base_dir.join(HARVEY_FILES_DIR).join(TABLES_DIR);
    
    let mut folder_counter = 0;
    let (folder_path, final_table_filename) = loop {
        let folder_name = if folder_counter == 0 {
            table_file_stem_truncated.to_string()
        } else {
            format!("{}_{}", table_file_stem_truncated, folder_counter)
        };
        let candidate_folder = tables_base.join(&folder_name);
        
        // Check if this stem is already used in project_data.table_files.files
        // We want to avoid conflict even if the extension is different.
        let name_conflict = project_data.table_files.files.iter().any(|f| {
            // Check if f.name matches folder_name exactly or matches folder_name + suffix
            f.name == folder_name || f.name.starts_with(&format!("{}.", folder_name))
        });

        if !candidate_folder.exists() && !name_conflict {
            let target_extension = if original_source_extension == "xlsx" && sheet_name_opt.is_some() { "csv" } else { &original_source_extension };
            let file_name = format!("{}.{}", folder_name, target_extension);
            debug!("[import_table_file] Found unique folder path: {} and filename: {}", candidate_folder.display(), file_name);
            break (candidate_folder, file_name);
        }
        
        folder_counter += 1;
        if folder_counter > 1000 {
            error!("[import_table_file] Could not find unique folder for table base '{}' after {} attempts.", table_file_stem_truncated, folder_counter);
            return Err(CommandError::from(format!(
                "Could not find unique folder for table base '{}' after {} attempts.",
                table_file_stem_truncated, folder_counter
            )));
        }
    };

    fs::create_dir_all(&folder_path)
        .map_err(|e| {
            error!("[import_table_file] Failed to create directory {}: {}", folder_path.display(), e);
            CommandError::Io(format!("Failed to create directory {}: {}", folder_path.display(), e))
        })?;
    debug!("[import_table_file] Created unique folder: {}", folder_path.display());

    let final_table_path = folder_path.join(&final_table_filename);

    let final_table_name = final_table_path.file_name().unwrap().to_string_lossy().into_owned();
    debug!("[import_table_file] Final table name: {}", final_table_name);

    if original_source_extension == "xlsx" && sheet_name_opt.is_some() {
        let sheet_name = sheet_name_opt.as_ref().unwrap();
        info!("[import_table_file] Extracting sheet '{}' from '{}' to '{}'", sheet_name, source_path.display(), final_table_path.display());
        extract_xlsx_sheet_to_csv(&source_path, sheet_name, &final_table_path)?;
        debug!("Sheet extracted to: {}", final_table_path.display());
    } else {
        info!("[import_table_file] Copying table from '{}' to '{}'", source_path.display(), final_table_path.display());
        fs::copy(&source_path, &final_table_path).map_err(|e| {
            error!("[import_table_file] Failed to copy table file from {} to {}: {}", source_path.display(), final_table_path.display(), e);
            CommandError::from(format!("Failed to copy table file: {}", e))
        })?;
        debug!("File copied to: {}", final_table_path.display());
    }

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
        language_code: None,
        properties: None,
        file_type: "table".to_string(),
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

    // For preview, we always load WITHOUT assuming headers first
    // This allows the user to see the first row and decide if it's a header or not.
    let target_extension = final_table_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let preview_data = match target_extension {
        "csv" => load_csv_data(&final_table_path, false, Some(5)),
        "xlsx" => load_xlsx_data(&final_table_path, false, Some(5)),
        _ => {
            error!("[import_table_file] Unsupported table extension for preview: {}", target_extension);
            return Err(CommandError::from(format!("Unsupported table extension for preview: {}", target_extension)))
        },
    }?;
    debug!("[import_table_file] Preview data loaded (no headers assumed).");

    Ok(json!({
        "table_path": final_table_path.to_string_lossy(),
        "preview_data": preview_data
    }))
}

#[tauri::command]
pub async fn delete_table_column(
    table_path_str: String,
    column_name_to_delete: String,
) -> Result<(), CommandError> {
    let table_path = PathBuf::from(&table_path_str);
    let extension = table_path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();

    // Get the has_headers flag from the project XML
    let project_xml_path = get_project_xml_path_from_item(&table_path)?;
    let project_data: ProjectXml = {
        let xml_content = fs::read_to_string(&project_xml_path)?;
        quick_xml::de::from_str(&xml_content)?
    };
    let relative_path_for_xml = table_path
        .strip_prefix(project_xml_path.parent().unwrap())?
        .to_string_lossy()
        .replace("\\", "/");
    let has_headers = project_data.table_files.files.iter()
        .find(|f| f.relative_path == relative_path_for_xml)
        .and_then(|f| f.has_headers)
        .unwrap_or(true);

    let mut loaded_value = match extension.as_str() {
        "csv" => load_csv_data(&table_path, has_headers, None)?,
        "xlsx" => load_xlsx_data(&table_path, has_headers, None)?,
        _ => return Err(CommandError::from(format!("Unsupported table extension for deleting column: {}", extension))),
    };

    // Sanitize data: remove carriage returns from all cell values, mirroring frontend logic.
    if let Some(data) = loaded_value.get_mut("data").and_then(|d| d.as_array_mut()) {
        for row in data.iter_mut() {
            if let Some(row_obj) = row.as_object_mut() {
                for (_, value) in row_obj.iter_mut() {
                    if let Some(s) = value.as_str() {
                        *value = json!(s.replace('\r', ""));
                    }
                }
            }
        }
    }

    let original_headers_val = loaded_value.get("headers")
        .ok_or_else(|| CommandError::from("Loaded table data is missing 'headers' field"))?;
    let original_headers: Vec<String> = serde_json::from_value(original_headers_val.clone())?;

    let mut data_value = loaded_value.get("data")
        .ok_or_else(|| CommandError::from("Loaded table data is missing 'data' field"))?
        .clone();

    let new_headers: Vec<String> = original_headers.into_iter()
        .filter(|h| h != &column_name_to_delete)
        .collect();

    if let Some(arr) = data_value.as_array_mut() {
        for item in arr {
            if let Some(obj) = item.as_object_mut() {
                obj.remove(&column_name_to_delete);
            }
        }
    }

    let data_vec = data_value.as_array()
        .ok_or_else(|| CommandError::from("Data is not an array after processing"))?
        .to_vec();

    match extension.as_str() {
        "csv" => save_csv_data_with_headers(&table_path, data_vec, &new_headers),
        "xlsx" => save_xlsx_data_with_headers(&table_path, data_vec, &new_headers),
        _ => unreachable!(),
    }
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

    if !has_headers {
        // If the user says the file DOES NOT have headers, we generate and write them.
        // Once written, the file effectively HAS headers from our perspective.
        let extension = table_path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        let loaded_data = match extension.as_str() {
            "csv" => load_csv_data(&table_path, false, None)?,
            "xlsx" => load_xlsx_data(&table_path, false, None)?,
            _ => return Err(CommandError::from(format!("Unsupported table extension for header generation: {}", extension))),
        };

        let headers_val = loaded_data.get("headers").ok_or_else(|| CommandError::from("Missing 'headers' in loaded data"))?;
        let data_val = loaded_data.get("data").ok_or_else(|| CommandError::from("Missing 'data' in loaded data"))?;

        let headers: Vec<String> = serde_json::from_value(headers_val.clone())?;
        let data: Vec<Value> = serde_json::from_value(data_val.clone())?;

        match extension.as_str() {
            "csv" => save_csv_data_with_headers(&table_path, data, &headers)?,
            "xlsx" => save_xlsx_data_with_headers(&table_path, data, &headers)?,
            _ => unreachable!(),
        }
    } else {
        // If has_headers is true, it means the file ALREADY has headers.
        // We don't need to rewrite the file, but we should make sure the XML reflects it.
        // The load_table_data and delete_table_column already respect the has_headers flag.
    }

    // Now, update the XML. If the user said "no headers", we've added them, so the file *now* has headers.
    // So we always set has_headers to true in the XML.
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
        table_entry.has_headers = Some(true); // Always set to true now
    } else {
        return Err(CommandError::from(format!("Table not found in project XML: {}", relative_path_for_xml)));
    }

    save_project_xml(&project_xml_path, &project_data)?;
    info!("[set_table_headers] Project XML updated successfully, has_headers set to true.");

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

    if let Some(data_array) = data.get("data").and_then(|d| d.as_array()) {
        debug!("[load_table_data] Successfully loaded {} rows.", data_array.len());
    }
    Ok(data)
}

fn to_json_response(headers: Vec<String>, records: Vec<Value>) -> Result<Value, CommandError> {
    Ok(json!({
        "headers": headers,
        "data": records
    }))
}

fn load_csv_data(path: &Path, has_headers: bool, limit: Option<usize>) -> Result<Value, CommandError> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(has_headers)
        .flexible(true) // Allows records to have different lengths than the header (like Excel handles trailing commas)
        .from_path(path)
        .map_err(|e| CommandError::from(format!("Failed to open CSV '{}': {}", path.display(), e)))?;

    let headers = if has_headers {
        rdr.headers()
            .map_err(|e| CommandError::from(format!("Failed to read CSV headers '{}': {}", path.display(), e)))?
            .iter()
            .map(|h| h.to_string())
            .collect::<Vec<String>>()
    } else {
        let mut temp_rdr = csv::ReaderBuilder::new().has_headers(false).flexible(true).from_path(path)?;
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
                // Serde JSON cannot serialize NaN or Infinity. If a CSV has "NaN", "Inf", etc. we must store it as a string or null.
                if num.is_finite() {
                    json!(num)
                } else {
                    json!(value_str)
                }
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

fn extract_xlsx_sheet_to_csv(source_path: &Path, sheet_name: &str, final_table_path: &Path) -> Result<(), CommandError> {
    let mut workbook: Xlsx<_> = open_workbook(source_path)
        .map_err(|e| CommandError::from(format!("Failed to open XLSX '{}': {}", source_path.display(), e)))?;

    let range_opt = workbook.worksheet_range(sheet_name);
    let range: calamine::Range<Data> = match range_opt {
        Ok(r) => r,
        Err(calamine::XlsxError::WorksheetNotFound(msg)) => return Err(CommandError::from(format!("Sheet '{}' not found in XLSX: {}", sheet_name, msg))),
        Err(e) => return Err(CommandError::from(format!("Failed to read sheet '{}': {}", sheet_name, e))),
    };

    let mut wtr = csv::Writer::from_path(final_table_path)
        .map_err(|e| CommandError::from(format!("Failed to create CSV at '{}': {}", final_table_path.display(), e)))?;

    for row in range.rows() {
        let row_strs: Vec<String> = row.iter().map(|cell: &Data| -> String {
            match cell {
                Data::String(s) => s.clone(),
                Data::Float(f) => f.to_string(),
                Data::Int(i) => i.to_string(),
                Data::Bool(b) => b.to_string(),
                Data::DateTime(e) => e.as_f64().to_string(),
                Data::DateTimeIso(s) => s.clone(),
                Data::DurationIso(s) => s.clone(),
                Data::Error(e) => format!("Error:{:?}", e),
                Data::Empty => String::new(),
            }
        }).collect();
        wtr.write_record(&row_strs)
            .map_err(|e| CommandError::from(format!("Failed to write row to CSV: {}", e)))?;
    }
    wtr.flush()
        .map_err(|e| CommandError::from(format!("Failed to flush CSV at '{}': {}", final_table_path.display(), e)))?;
    Ok(())
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
    let all_rows: Vec<Vec<Data>> = range.rows().map(|r| r.to_vec()).collect();

    let headers: Vec<String> = if has_headers {
        if let Some(row_data) = all_rows.first() {
            row_data.iter()
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
                .collect()
        } else {
            vec![]
        }
    } else {
        if let Some(first_row) = all_rows.first() {
            let num_columns = first_row.len();
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
        } else {
            vec![]
        }
    };

    debug!("[load_xlsx_data] Headers: {:?}", headers);

    let row_iterator = all_rows.into_iter().skip(if has_headers { 1 } else { 0 });
    let data_rows_to_process = if let Some(l) = limit {
        row_iterator.take(l)
    } else {
        row_iterator.take(usize::MAX)
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

#[allow(dead_code)]
fn records_to_json(_headers: &[String], records: Vec<serde_json::Map<String, Value>>) -> Result<Value, CommandError> {
    Ok(json!(records))
}

#[tauri::command]
pub async fn save_table_data(table_path_str: String, table_data: Vec<Value>, headers: Vec<String>) -> Result<(), CommandError> {
    let table_path = Path::new(&table_path_str);
    let extension = table_path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();

    if headers.is_empty() && table_data.is_empty() {
        return Ok(()); // Nothing to save
    }

    let final_headers = if headers.is_empty() {
        if let Some(first_row) = table_data.get(0).and_then(|v| v.as_object()) {
            first_row.keys().cloned().collect::<Vec<String>>()
        } else {
            vec![]
        }
    } else {
        headers
    };

    match extension.as_str() {
        "csv" => save_csv_data_with_headers(table_path, table_data, &final_headers),
        "xlsx" => save_xlsx_data_with_headers(table_path, table_data, &final_headers),
        _ => Err(CommandError::from(format!("Unsupported table extension for saving: {}", extension))),
    }
}

fn save_xlsx_data_with_headers(path: &Path, data: Vec<Value>, headers: &[String]) -> Result<(), CommandError> {
    save_xlsx_data_with_headers_and_styles(path, data, headers, None)
}

fn save_xlsx_data_with_headers_and_styles(
    path: &Path,
    data: Vec<Value>,
    headers: &[String],
    styles: Option<serde_json::Map<String, Value>>
) -> Result<(), CommandError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    let mut header_format = rust_xlsxwriter::Format::new();
    header_format = header_format.set_bold();

    // Write headers
    for (col_num, header) in headers.iter().enumerate() {
        worksheet.write_string_with_format(0, col_num as u16, header, &header_format)?;
    }

    // Write data rows
    for (row_num, row_value) in data.iter().enumerate() {
        if let Some(row_map) = row_value.as_object() {
            for (col_num, header) in headers.iter().enumerate() {
                if let Some(cell_value) = row_map.get(header) {
                    let mut cell_format = rust_xlsxwriter::Format::new();
                    cell_format = cell_format.set_text_wrap(); // Preserve newlines

                    if let Some(ref style_map) = styles {
                        let coord_key = format!("{},{}", row_num, col_num);
                        if let Some(style_val) = style_map.get(&coord_key) {
                            if let Some(style_obj) = style_val.as_object() {
                                if let Some(is_bold) = style_obj.get("bold").and_then(|v| v.as_bool()) {
                                    if is_bold { cell_format = cell_format.set_bold(); }
                                }
                                if let Some(is_italic) = style_obj.get("italic").and_then(|v| v.as_bool()) {
                                    if is_italic { cell_format = cell_format.set_italic(); }
                                }
                                if let Some(is_underline) = style_obj.get("underline").and_then(|v| v.as_bool()) {
                                    if is_underline { cell_format = cell_format.set_underline(rust_xlsxwriter::FormatUnderline::Single); }
                                }
                                if let Some(text_color_val) = style_obj.get("textColor").and_then(|v| v.as_str()) {
                                    if !text_color_val.is_empty() {
                                        let clean_color = if text_color_val.starts_with('#') {
                                            &text_color_val[1..]
                                        } else {
                                            text_color_val
                                        };
                                        if let Ok(color_num) = u32::from_str_radix(clean_color, 16) {
                                            cell_format = cell_format.set_font_color(rust_xlsxwriter::Color::RGB(color_num));
                                        }
                                    }
                                }
                                if let Some(color_val) = style_obj.get("color").and_then(|v| v.as_str()) {
                                    if !color_val.is_empty() {
                                        // Strip # if it exists
                                        let clean_color = if color_val.starts_with('#') {
                                            &color_val[1..]
                                        } else {
                                            color_val
                                        };
                                        if let Ok(color_num) = u32::from_str_radix(clean_color, 16) {
                                            cell_format = cell_format.set_background_color(rust_xlsxwriter::Color::RGB(color_num));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    match cell_value {
                        Value::String(s) => {
                            worksheet.write_string_with_format(row_num as u32 + 1, col_num as u16, s, &cell_format)?;
                        },
                        Value::Number(n) => {
                            if let Some(float_val) = n.as_f64() {
                                worksheet.write_number_with_format(row_num as u32 + 1, col_num as u16, float_val, &cell_format)?;
                            }
                        },
                        Value::Bool(b) => {
                            worksheet.write_boolean_with_format(row_num as u32 + 1, col_num as u16, *b, &cell_format)?;
                        },
                        Value::Array(arr) => {
                            let joined = arr.iter().map(|v| match v {
                                Value::String(s) => s.clone(),
                                _ => v.to_string(),
                            }).collect::<Vec<String>>().join(", ");
                            worksheet.write_string_with_format(row_num as u32 + 1, col_num as u16, &joined, &cell_format)?;
                        },
                        _ => {
                            worksheet.write_blank(row_num as u32 + 1, col_num as u16, &cell_format)?;
                        }
                    }
                }
            }
        }
    }

    workbook.save(path).map_err(|e| CommandError::from(e.to_string()))?;
    Ok(())
}

fn save_csv_data_with_headers(path: &Path, data: Vec<Value>, headers: &[String]) -> Result<(), CommandError> {
    let mut wtr = csv::Writer::from_path(path)?;
    wtr.write_record(headers)?;
    for row_value in data {
        if let Some(row_map) = row_value.as_object() {
            let row: Vec<String> = headers.iter().map(|h| {
                row_map.get(h).map(|v| {
                    match v {
                        Value::String(s) => s.to_string(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        Value::Array(arr) => {
                            // Join array elements with a comma for better CSV presentation
                            arr.iter().map(|item| match item {
                                Value::String(s) => s.clone(),
                                _ => item.to_string(),
                            }).collect::<Vec<String>>().join(", ")
                        },
                        Value::Null => "".to_string(),
                        _ => v.to_string(),
                    }
                }).unwrap_or_else(|| "".to_string())
            }).collect();
            wtr.write_record(&row)?;
        }
    }
    wtr.flush()?;
    Ok(())
}

#[allow(dead_code)]
fn get_headers(path: &Path) -> Result<Vec<String>, CommandError> {
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    match extension.as_str() {
        "csv" => {
            let mut rdr = csv::ReaderBuilder::new().has_headers(true).from_path(path)?;
            Ok(rdr.headers()?.iter().map(|h| h.to_string()).collect())
        }
        "xlsx" => {
            let mut workbook: Xlsx<_> = open_workbook(path)?;
            let sheet_name = workbook.sheet_names().first().cloned().ok_or_else(|| CommandError::from("XLSX file contains no sheets."))?;
            let range = workbook.worksheet_range(&sheet_name)?;
            let headers = range.rows().next().map_or(Ok(vec![]), |row| {
                Ok(row.iter().map(|cell| cell.to_string()).collect())
            }) as Result<Vec<String>, CommandError>;
            let headers = headers?;
            Ok(headers)
        }
        _ => Err(CommandError::from(format!("Unsupported table extension for getting headers: {}", extension))),
    }
}

#[tauri::command]
pub async fn rename_table_header(
    table_path_str: String,
    old_header: String,
    new_header: String,
) -> Result<(), CommandError> {
    let table_path = PathBuf::from(&table_path_str);
    let extension = table_path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();

    // Get the has_headers flag from the project XML
    let project_xml_path = get_project_xml_path_from_item(&table_path)?;
    let project_base_dir = project_xml_path.parent()
        .ok_or_else(|| CommandError::from("Could not get project base directory from XML path"))?;
    let relative_path_for_xml = table_path
        .strip_prefix(project_base_dir)?
        .to_string_lossy()
        .replace("\\", "/");
    let project_data: ProjectXml = {
        let xml_content = fs::read_to_string(&project_xml_path)?;
        quick_xml::de::from_str(&xml_content)?
    };
    let has_headers = project_data.table_files.files.iter()
        .find(|f| f.relative_path == relative_path_for_xml)
        .and_then(|f| f.has_headers)
        .unwrap_or(true);

    let loaded_value = match extension.as_str() {
        "csv" => load_csv_data(&table_path, has_headers, None)?,
        "xlsx" => load_xlsx_data(&table_path, has_headers, None)?,
        _ => return Err(CommandError::from(format!("Unsupported table extension for renaming header: {}", extension))),
    };

    let original_headers_val = loaded_value.get("headers")
        .ok_or_else(|| CommandError::from("Loaded table data is missing 'headers' field"))?;
    let original_headers: Vec<String> = serde_json::from_value(original_headers_val.clone())
        .map_err(|e| CommandError::from(format!("Failed to parse headers from loaded data: {}", e)))?;

    let mut data_value = loaded_value.get("data")
        .ok_or_else(|| CommandError::from("Loaded table data is missing 'data' field"))?
        .clone();

    let new_headers: Vec<String> = original_headers.iter().map(|h| {
        if h == &old_header {
            new_header.clone()
        } else {
            h.clone()
        }
    }).collect();

    if let Some(arr) = data_value.as_array_mut() {
        for item in arr {
            if let Some(obj) = item.as_object_mut() {
                if let Some(value) = obj.remove(&old_header) {
                    obj.insert(new_header.clone(), value);
                }
            }
        }
    }

    let data_vec = data_value.as_array()
        .ok_or_else(|| CommandError::from("Data is not an array after processing"))?
        .to_vec();

    match extension.as_str() {
        "csv" => save_csv_data_with_headers(&table_path, data_vec, &new_headers),
        "xlsx" => save_xlsx_data_with_headers(&table_path, data_vec, &new_headers),
        _ => unreachable!(),
    }
}

#[tauri::command]
pub async fn save_table_styles(file_path: String, styles: Value) -> Result<(), CommandError> {
    let project_xml_path = get_project_xml_path_from_item(&PathBuf::from(&file_path))?;
    let project_data: ProjectXml = {
        let xml_content = fs::read_to_string(&project_xml_path)?;
        quick_xml::de::from_str(&xml_content)?
    };
    let project_id = project_data.project_uuid;
    let styles_string = serde_json::to_string(&styles)
        .map_err(|e| CommandError::from(format!("Failed to serialize table styles: {}", e)))?;
    db_handler::save_table_styles(&project_id, &file_path, &styles_string)
}

#[tauri::command]
pub async fn load_table_styles(file_path: String) -> Result<Value, CommandError> {
    let project_xml_path = get_project_xml_path_from_item(&PathBuf::from(&file_path))?;
    let project_data: ProjectXml = {
        let xml_content = fs::read_to_string(&project_xml_path)?;
        quick_xml::de::from_str(&xml_content)?
    };
    let project_id = project_data.project_uuid;
    let styles_string_option = db_handler::load_table_styles(&project_id, &file_path)?;

    match styles_string_option {
        Some(styles_string) => {
            let styles_value: Value = serde_json::from_str(&styles_string)
                .map_err(|e| CommandError::from(format!("Failed to parse table styles: {}", e)))?;
            Ok(styles_value)
        },
        None => Ok(json!([])), // Return an empty array if no styles are found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use crate::projectview::shared_types::{ProjectXml};
    use crate::projectview::db_handler::{self, FileMetadataWithCustomFieldsFromDb};

    fn create_dummy_project_xml(project_dir: &Path, project_name: &str, project_uuid: &str) -> PathBuf {
        let project_xml_path = project_dir.join("project.xml");
        let project_data = ProjectXml {
            name: project_name.to_string(),
            project_uuid: project_uuid.to_string(),
            media_files: Default::default(),
            document_files: Default::default(),
            table_files: Default::default(),
            image_files: Default::default(),
            imported_transcript_files: Default::default(),
            document_metadata_files: Default::default(),
        };
        let xml_string = quick_xml::se::to_string(&project_data).unwrap();
        fs::write(&project_xml_path, xml_string).unwrap();
        project_xml_path
    }

    fn setup_test_environment() -> (tempfile::TempDir, PathBuf, PathBuf, rusqlite::Connection, String) {
        let temp_dir = tempdir().unwrap();
        let project_dir = temp_dir.path().to_path_buf();
        let harvey_files_dir = project_dir.join(".harvey_files");
        let tables_dir = harvey_files_dir.join("Tables");
        fs::create_dir_all(&tables_dir).unwrap();

        let dummy_table_path = project_dir.join("dummy.csv");
        let mut f = fs::File::create(&dummy_table_path).unwrap();
        f.write_all(b"header1,header2\nval1,val2").unwrap();

        let (conn, project_id) = db_handler::setup_test_db_in_memory();

        (temp_dir, project_dir, dummy_table_path, conn, project_id)
    }

    #[tokio::test]
    #[ignore]
    async fn test_import_table_file_with_db_metadata() -> Result<(), Box<dyn std::error::Error>> {
        let (_temp_dir, project_dir, dummy_table_path, _conn, project_uuid) = setup_test_environment();
        let project_xml_path = create_dummy_project_xml(&project_dir, "TestTableProject", &project_uuid);
        let project_xml_path_str = project_xml_path.to_string_lossy().to_string();

        let source_table_dir = project_dir.join("source_tables");
        fs::create_dir_all(&source_table_dir)?;
        let dummy_table_path = source_table_dir.join("dummy_table.csv");
        let mut source_file = File::create(&dummy_table_path)?;
        writeln!(source_file, "header1,header2\nval1,val2")?;
        let dummy_table_path_str = dummy_table_path.to_string_lossy().to_string();

        let import_result = import_table_file(dummy_table_path_str.clone(), project_xml_path_str.clone()).await;
        assert!(import_result.is_ok(), "import_table_file failed: {:?}", import_result.err());
        let result_value = import_result.unwrap();
        let final_table_abs_path_str = result_value["table_path"].as_str().unwrap().to_string();
        let final_table_abs_path = PathBuf::from(&final_table_abs_path_str);

        assert!(final_table_abs_path.exists(), "Imported table file should exist at {}", final_table_abs_path_str);

        let parent_dir = final_table_abs_path.parent().unwrap();
        let stem = final_table_abs_path.file_stem().unwrap().to_str().unwrap();
        let metadata_json_path = parent_dir.join(format!(".{}.metadata.json", stem));
        assert!(!metadata_json_path.exists(), ".metadata.json file should NOT exist at {}", metadata_json_path.display());

        let updated_xml_content = fs::read_to_string(&project_xml_path)?;
        let updated_project_data: ProjectXml = quick_xml::de::from_str(&updated_xml_content)?;

        let expected_table_name = final_table_abs_path.file_name().unwrap().to_str().unwrap();
        let expected_relative_path = final_table_abs_path.strip_prefix(&project_dir)?.to_string_lossy().replace("\\", "/");

        assert_eq!(updated_project_data.table_files.files.len(), 1, "Should be one table file in XML");
        let table_entry_xml = updated_project_data.table_files.files.get(0).unwrap();
        assert_eq!(table_entry_xml.name, expected_table_name);
        assert_eq!(table_entry_xml.relative_path, expected_relative_path);
        assert!(updated_project_data.document_metadata_files.files.is_empty(), "document_metadata_files should be empty in XML");

        let loaded_meta_option: Option<FileMetadataWithCustomFieldsFromDb> = db_handler::load_asset_metadata(
            "test-uuid",
            &expected_relative_path
        ).expect("Failed to load metadata from DB for assertion");

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
#[tauri::command]
pub async fn save_table_schema(
    project_id: String,
    table_path: String,
    schema: Value,
) -> Result<(), CommandError> {
    // Normalize path: replace backslashes with forward slashes for cross-platform DB consistency
    let normalized_path = table_path.replace("\\", "/");
    info!("[save_table_schema] Saving schema for table: {} in project {}", normalized_path, project_id);
    
    let schema_json = serde_json::to_string_pretty(&schema)
        .map_err(|e| CommandError::from(format!("Failed to serialize schema: {}", e)))?;
    
    db_handler::save_table_schema(&project_id, &normalized_path, &schema_json)?;
    
    Ok(())
}

#[tauri::command]
pub async fn load_table_schema(
    project_id: String,
    table_path: String,
) -> Result<Value, CommandError> {
    // Normalize path: replace backslashes with forward slashes for cross-platform DB consistency
    let normalized_path = table_path.replace("\\", "/");
    debug!("[load_table_schema] Loading schema for table: {} in project {}", normalized_path, project_id);
    
    let schema_json_opt = db_handler::load_table_schema(&project_id, &normalized_path)?;
    
    match schema_json_opt {
        Some(schema_json) => {
            let schema: Value = serde_json::from_str(&schema_json)
                .map_err(|e| CommandError::from(format!("Failed to parse schema JSON: {}", e)))?;
            Ok(schema)
        },
        None => Ok(json!({})),
    }
}

#[tauri::command]
pub async fn create_new_table(
    project_xml_path: String, 
    headers: Vec<String>,
    schema: Option<Value>
) -> Result<String, CommandError> {
    info!("[Backend Create Table] Start: project_xml_path={}, headers={:?}, has_schema={}", project_xml_path, headers, schema.is_some());

    let xml_path = PathBuf::from(&project_xml_path);
    let project_base_dir = xml_path.parent().ok_or_else(|| CommandError::from("Could not get project base directory."))?;

    let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&xml_path)?)?;

    let tables_base = project_base_dir.join(HARVEY_FILES_DIR).join(TABLES_DIR);
    fs::create_dir_all(&tables_base)?;

    // Create a unique folder and filename
    let mut i = 1;
    let mut folder_path;
    let mut table_name;
    loop {
        let base_name = format!("Untitled_{}", i);
        folder_path = tables_base.join(&base_name);
        table_name = format!("{}.csv", base_name);
        
        let relative_path = format!("{}/{}/{}/{}", HARVEY_FILES_DIR, TABLES_DIR, base_name, table_name);
        if !folder_path.exists() && !project_data.table_files.files.iter().any(|f| f.relative_path == relative_path) {
            break;
        }
        i += 1;
    }

    fs::create_dir_all(&folder_path)?;
    let new_table_path = folder_path.join(&table_name);

    // Create the CSV file with headers and an empty row
    let mut wtr = csv::Writer::from_path(&new_table_path)?;
    wtr.write_record(&headers)?;
    wtr.write_record(headers.iter().map(|_| ""))?;
    wtr.flush()?;

    // Update project XML
    let relative_path_for_xml = new_table_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\", "/");
    project_data.table_files.files.push(TableEntryXml {
        name: table_name.clone(),
        relative_path: relative_path_for_xml.clone(),
        has_headers: Some(true),
        language_code: None,
    });
    project_data.table_files.files.sort_by(|a, b| a.name.cmp(&b.name));
    save_project_xml(&xml_path, &project_data)?;

    let project_id_for_db = project_data.project_uuid.clone();
    if project_id_for_db.is_empty() {
        return Err(CommandError::Message(format!("Project ID (UUID) is missing in the project file ({}).", xml_path.display())));
    }

    // Save schema to database if provided
    if let Some(s) = schema {
        let schema_json = serde_json::to_string(&s)
            .map_err(|e| CommandError::from(format!("Failed to serialize schema: {}", e)))?;
        let normalized_path = new_table_path.to_string_lossy().replace("\\", "/");
        db_handler::save_table_schema(&project_id_for_db, &normalized_path, &schema_json)?;
    }

    let file_metadata_for_db = FileMetadata {
        file_name: table_name.clone(),
        file_path: new_table_path.to_string_lossy().into_owned(),
        last_modified: Utc::now().to_rfc3339(),
        created_at: Some(Utc::now().to_rfc3339()),
        properties: Some(json!({"has_headers": true}).to_string()),
        ..Default::default()
    };

    db_handler::save_asset_metadata(
        &project_id_for_db,
        &file_metadata_for_db,
        &relative_path_for_xml,
        "table",
        None,
    )?;

    info!("[Backend Create Table] Success: new_table_path={}", new_table_path.display());
    Ok(new_table_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn export_formatted_table_to_csv(
    data: Vec<Value>,
    headers: Vec<String>,
    output_path_str: String,
) -> Result<String, CommandError> {
    info!("[export_formatted_table_to_csv] Exporting formatted table to: {}", output_path_str);
    
    let output_path = Path::new(&output_path_str);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    save_csv_data_with_headers(output_path, data, &headers)?;
    
    Ok(output_path_str)
}

#[tauri::command]
pub async fn export_formatted_table_to_xlsx(
    data: Vec<Value>,
    headers: Vec<String>,
    styles: Option<Value>,
    output_path_str: String,
) -> Result<String, CommandError> {
    info!("[export_formatted_table_to_xlsx] Exporting formatted table to: {}", output_path_str);
    
    let output_path = Path::new(&output_path_str);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut styles_map = None;
    if let Some(val) = styles {
        info!("[export_formatted_table_to_xlsx] Received styles parameter.");
        if let Some(map) = val.as_object() {
            info!("[export_formatted_table_to_xlsx] Successfully parsed styles as object with {} keys", map.len());
            styles_map = Some(map.clone());
        } else {
            info!("[export_formatted_table_to_xlsx] Styles parameter was not an object: {:?}", val);
        }
    } else {
        info!("[export_formatted_table_to_xlsx] Styles parameter was None.");
    }

    save_xlsx_data_with_headers_and_styles(output_path, data, &headers, styles_map)?;
    
    Ok(output_path_str)
}

#[tauri::command]
pub async fn export_table_to_csv(
    table_path_str: String,
    output_path_str: String,
) -> Result<String, CommandError> {
    info!("[export_table_to_csv] Exporting table from: {} to: {}", table_path_str, output_path_str);
    
    // Load current data
    let loaded_data = load_table_data(table_path_str).await?;
    let headers_val = loaded_data.get("headers").ok_or_else(|| CommandError::from("Missing 'headers' in table data"))?;
    let data_val = loaded_data.get("data").ok_or_else(|| CommandError::from("Missing 'data' in table data"))?;

    let headers: Vec<String> = serde_json::from_value(headers_val.clone())?;
    let data: Vec<Value> = serde_json::from_value(data_val.clone())?;

    let output_path = Path::new(&output_path_str);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    save_csv_data_with_headers(output_path, data, &headers)?;
    
    Ok(output_path_str)
}

#[tauri::command]
pub async fn export_table_to_xlsx(
    table_path_str: String,
    output_path_str: String,
) -> Result<String, CommandError> {
    info!("[export_table_to_xlsx] Exporting table from: {} to: {}", table_path_str, output_path_str);
    
    // Load current data
    let loaded_data = load_table_data(table_path_str).await?;
    let headers_val = loaded_data.get("headers").ok_or_else(|| CommandError::from("Missing 'headers' in table data"))?;
    let data_val = loaded_data.get("data").ok_or_else(|| CommandError::from("Missing 'data' in table data"))?;

    let headers: Vec<String> = serde_json::from_value(headers_val.clone())?;
    let data: Vec<Value> = serde_json::from_value(data_val.clone())?;

    let output_path = Path::new(&output_path_str);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    save_xlsx_data_with_headers(output_path, data, &headers)?;
    
    Ok(output_path_str)
}
