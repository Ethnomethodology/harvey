// src-tauri/src/projectview/table_handler.rs
use super::shared_types::*;
use super::shared_utils::{save_project_xml, ensure_base_asset_dirs};
use crate::welcome::config::CommandError;
use crate::projectview::core_commands::get_table_asset_metadata_path;
use chrono::Utc;
use serde_json; // Already used for Value, json, but good to ensure it's available for to_string_pretty
use serde::{Serialize, Deserialize};
use log::{info, warn, debug};
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
    highlights: Vec<String>, // Assuming highlights are strings
}

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

    let source_filename_stem = source_path.file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| CommandError::from("Could not get table filename stem"))?;

    let source_extension = source_path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    if source_extension != "csv" && source_extension != "xlsx" {
        return Err(CommandError::from(format!("Unsupported table file type: .{}", source_extension)));
    }

    // Create a folder under Tables named after the file stem
    let tables_base = project_base_dir.join(HARVEY_FILES_DIR).join(TABLES_DIR);
    let folder_path = tables_base.join(source_filename_stem);
    if !folder_path.exists() {
        fs::create_dir_all(&folder_path)?;
    }
    // Pick unique filename inside that folder
    let mut counter = 0;
    let final_table_path = loop {
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
                "Could not find unique filename for table base '{}' after {} attempts.",
                source_filename_stem, counter
            )));
        }
    };
    let final_table_name = final_table_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    info!("[import_table_file] Copying table from '{}' to '{}'", source_path.display(), final_table_path.display());
    fs::copy(&source_path, &final_table_path).map_err(|e| CommandError::from(format!("Failed to copy table file: {}", e)))?;

    info!("[import_table_file] Updating project XML to include table: {}", final_table_name);
    let xml_content = fs::read_to_string(&project_xml_path)?;
    let mut project_data: ProjectXml = quick_xml::de::from_str(&xml_content)?;

    let relative_path_for_xml = final_table_path
        .strip_prefix(project_base_dir)?
        .to_string_lossy()
        .replace("\\", "/");

    let new_table_entry = TableEntryXml {
        name: final_table_name.clone(),
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

    info!("[import_table_file] Creating standard asset metadata for table: {}", final_table_path.display());
    match get_table_asset_metadata_path(&final_table_path) {
        Ok(asset_metadata_path) => {
            // final_table_name is already defined in this function
            let asset_metadata_content = StandardAssetMetadata {
                metadata: FileMetadata {
                    file_name: final_table_name.clone(), // Use final_table_name
                    file_path: final_table_path.to_string_lossy().into_owned(), // Absolute path
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
                        warn!("[import_table_file] Failed to write asset metadata file {}: {}", asset_metadata_path.display(), e);
                    } else {
                        info!("[import_table_file] Created asset metadata file: {}", asset_metadata_path.display());
                    }
                }
                Err(e) => {
                    warn!("[import_table_file] Failed to serialize asset metadata for {}: {}", asset_metadata_path.display(), e);
                }
            }
        }
        Err(e) => {
            warn!("[import_table_file] Failed to get asset metadata path for {}: {:?}", final_table_path.display(), e);
        }
    }

    Ok(final_table_path.to_string_lossy().to_string())
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