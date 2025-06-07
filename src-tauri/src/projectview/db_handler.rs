// src-tauri/src/projectview/db_handler.rs
use rusqlite::{Connection, Result, params, OptionalExtension, ToSql};
use std::path::PathBuf;
use std::fs;
use crate::welcome::config::get_config_dir; // Assuming this function gives PathBuf
use log::{info, debug, error}; // Added error
use serde::{Serialize, Deserialize}; // Added for the new struct
use crate::projectview::shared_types::FileMetadata; // For function signatures

const DB_FILE_NAME: &str = "harvey_annotations.sqlite";

#[derive(Debug, Serialize, Deserialize)]
pub struct FileMetadataWithCustomFieldsFromDb {
    pub file_name: String,
    pub file_path: String,
    pub last_modified: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub summary: Option<String>,
    pub duration_seconds: Option<f64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub frame_rate: Option<f32>,
    pub bit_rate: Option<i64>,
    pub audio_codec: Option<String>,
    pub video_codec: Option<String>,
    pub creation_time: Option<String>,
    pub custom_fields_json: Option<String>,
    pub asset_type: String,
}

fn get_db_path() -> Result<PathBuf, String> {
    let config_dir = get_config_dir().map_err(|e| format!("Failed to get config dir: {}", e.message))?;
    Ok(config_dir.join(DB_FILE_NAME))
}

pub fn init_db() -> Result<()> {
    let db_path = get_db_path().map_err(|e| rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e)))?;
    let conn = Connection::open(&db_path)?;

    debug!("[DB] Initializing database at: {}", db_path.display());

    conn.execute(
        "CREATE TABLE IF NOT EXISTS pdf_annotations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            pdf_document_path TEXT NOT NULL UNIQUE, -- Conceptually 'document_path'
            annotations_json TEXT NOT NULL,
            document_type TEXT NOT NULL DEFAULT 'pdf', -- New column
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Check if document_type column exists
    let mut stmt = conn.prepare("PRAGMA table_info(pdf_annotations)")?;
    let column_exists = stmt.query_map([], |row| row.get::<_, String>(1))?
                            .any(|col_name_result| col_name_result.map_or(false, |name| name == "document_type"));

    if !column_exists {
        info!("[DB] Adding document_type column to pdf_annotations table.");
        conn.execute("ALTER TABLE pdf_annotations ADD COLUMN document_type TEXT NOT NULL DEFAULT 'pdf'", [])?;
    }

    // Create a trigger to update `updated_at` timestamp
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_pdf_annotations_updated_at
        AFTER UPDATE ON pdf_annotations
        FOR EACH ROW
        BEGIN
            UPDATE pdf_annotations SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
        END;",
        [],
    )?;

    // asset_metadata table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS asset_metadata (
            asset_relative_path TEXT PRIMARY KEY,
            file_name TEXT NOT NULL,
            file_path TEXT NOT NULL,
            last_modified TEXT NOT NULL,
            title TEXT,
            description TEXT,
            summary TEXT,
            duration_seconds REAL,
            width INTEGER,
            height INTEGER,
            frame_rate REAL,
            bit_rate INTEGER,
            audio_codec TEXT,
            video_codec TEXT,
            creation_time TEXT,
            asset_type TEXT NOT NULL,
            custom_fields_json TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_asset_metadata_updated_at
        AFTER UPDATE ON asset_metadata
        FOR EACH ROW
        BEGIN
            UPDATE asset_metadata SET updated_at = CURRENT_TIMESTAMP WHERE asset_relative_path = OLD.asset_relative_path;
        END;",
        [],
    )?;

    // custom_field_definitions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS custom_field_definitions (
            field_key TEXT PRIMARY KEY NOT NULL,
            field_name TEXT NOT NULL,
            field_type TEXT NOT NULL,
            scope TEXT NOT NULL,
            default_value TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
        )",
        [],
    )?;

    // Trigger for custom_field_definitions updated_at
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_custom_field_definitions_updated_at
        AFTER UPDATE ON custom_field_definitions
        FOR EACH ROW
        BEGIN
            UPDATE custom_field_definitions SET updated_at = CURRENT_TIMESTAMP WHERE field_key = OLD.field_key;
        END;",
        [],
    )?;
    info!("[DB] Initialized custom_field_definitions table and trigger.");

    info!("[DB] Database initialized successfully with all tables and triggers.");
    Ok(())
}

// Helper to convert Option<T> to dyn ToSql for rusqlite
fn to_sql_optional<T: ToSql + 'static>(opt: Option<T>) -> Box<dyn ToSql> {
    match opt {
        Some(val) => Box::new(val),
        None => Box::new(rusqlite::types::Null),
    }
}
// Helper to convert Option<&str> to dyn ToSql
fn to_sql_optional_str(opt_str: Option<&str>) -> Box<dyn ToSql> {
    match opt_str {
        Some(s) => Box::new(s.to_string()), // Convert &str to String before boxing
        None => Box::new(rusqlite::types::Null),
    }
}


pub fn save_asset_metadata(
    metadata: &FileMetadata,
    asset_relative_path: &str,
    asset_type: &str,
    custom_fields_json: Option<&str>,
) -> Result<()> {
    debug!(
        "[DB] Saving asset metadata for: {} (type: {})",
        asset_relative_path, asset_type
    );
    let db_path = get_db_path().map_err(|e| {
        error!("[DB] Failed to get DB path: {}", e);
        rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e))
    })?;
    let conn = Connection::open(&db_path)?;

    if let Some(parent_dir) = db_path.parent() {
        if !parent_dir.exists() {
            fs::create_dir_all(parent_dir).map_err(|e| {
                error!("[DB] Failed to create db directory: {}", e);
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(1),
                    Some(format!("Failed to create db directory: {}", e)),
                )
            })?;
        }
    }

    let sql = "
        INSERT INTO asset_metadata (
            asset_relative_path, file_name, file_path, last_modified, title,
            description, summary, duration_seconds, width, height, frame_rate,
            bit_rate, audio_codec, video_codec, creation_time, asset_type, custom_fields_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
        ON CONFLICT(asset_relative_path) DO UPDATE SET
            last_modified = excluded.last_modified,
            title = excluded.title,
            description = excluded.description,
            summary = excluded.summary,
            duration_seconds = excluded.duration_seconds,
            width = excluded.width,
            height = excluded.height,
            frame_rate = excluded.frame_rate,
            bit_rate = excluded.bit_rate,
            audio_codec = excluded.audio_codec,
            video_codec = excluded.video_codec,
            creation_time = excluded.creation_time,
            custom_fields_json = excluded.custom_fields_json,
            updated_at = CURRENT_TIMESTAMP
        -- file_name, file_path, and asset_type are NOT updated from 'excluded' during an update.
        -- asset_type was already not being updated, which is correct.
        ;
    ";

    conn.execute(
        sql,
        params![
            asset_relative_path,
            metadata.file_name,
            metadata.file_path,
            metadata.last_modified,
            &metadata.title,
            &metadata.description,
            &metadata.summary,
            to_sql_optional(metadata.duration_seconds),
            to_sql_optional(metadata.width),
            to_sql_optional(metadata.height),
            to_sql_optional(metadata.frame_rate),
            to_sql_optional(metadata.bit_rate),
            to_sql_optional_str(metadata.audio_codec.as_deref()),
            to_sql_optional_str(metadata.video_codec.as_deref()),
            to_sql_optional_str(metadata.creation_time.as_deref()),
            asset_type,
            to_sql_optional_str(custom_fields_json),
        ],
    )?;

    info!(
        "[DB] Asset metadata saved successfully for: {} (type: {})",
        asset_relative_path, asset_type
    );
    Ok(())
}

pub fn load_asset_metadata(asset_relative_path: &str) -> Result<Option<FileMetadataWithCustomFieldsFromDb>> {
    debug!("[DB] Loading asset metadata for: {}", asset_relative_path);
    let db_path = get_db_path().map_err(|e| {
        error!("[DB] Failed to get DB path: {}", e);
        rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e))
    })?;
    if !db_path.exists() {
        debug!("[DB] Database file not found at {}. Returning None for asset: {}", db_path.display(), asset_relative_path);
        return Ok(None);
    }
    let conn = Connection::open(&db_path)?;
    let mut stmt = conn.prepare("
        SELECT file_name, file_path, last_modified, title, description, summary,
               duration_seconds, width, height, frame_rate, bit_rate, audio_codec, video_codec,
               creation_time, custom_fields_json, asset_type
        FROM asset_metadata
        WHERE asset_relative_path = ?1
    ")?;

    let result = stmt.query_row(params![asset_relative_path], |row| {
        Ok(FileMetadataWithCustomFieldsFromDb {
            file_name: row.get(0)?,
            file_path: row.get(1)?,
            last_modified: row.get(2)?,
            title: row.get(3)?,
            description: row.get(4)?,
            summary: row.get(5)?,
            duration_seconds: row.get(6)?,
            width: row.get(7)?,
            height: row.get(8)?,
            frame_rate: row.get(9)?,
            bit_rate: row.get(10)?,
            audio_codec: row.get(11)?,
            video_codec: row.get(12)?,
            creation_time: row.get(13)?,
            custom_fields_json: row.get(14)?,
            asset_type: row.get(15)?,
        })
    }).optional()?;

    debug!("[DB] Load asset metadata result for {}: {}", asset_relative_path, if result.is_some() { "Some(...)" } else { "None" });
    Ok(result)
}

pub fn delete_asset_metadata(asset_relative_path: &str) -> Result<()> {
    debug!("[DB] Deleting asset metadata for: {}", asset_relative_path);
    let db_path = get_db_path().map_err(|e| {
        error!("[DB] Failed to get DB path: {}", e);
        rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e))
    })?;
    if !db_path.exists() {
        debug!("[DB] Database file not found at {}. Nothing to delete for asset: {}", db_path.display(), asset_relative_path);
        return Ok(());
    }
    let conn = Connection::open(&db_path)?;
    let changes = conn.execute("DELETE FROM asset_metadata WHERE asset_relative_path = ?1", params![asset_relative_path])?;

    if changes > 0 {
        info!("[DB] Asset metadata deleted successfully for: {} ({} rows affected)", asset_relative_path, changes);
    } else {
        debug!("[DB] No asset metadata found to delete for: {}", asset_relative_path);
    }
    Ok(())
}

pub fn rename_asset_metadata_key(
    old_relative_path: &str,
    new_relative_path: &str,
    new_file_path: &str,
    new_file_name: &str,
) -> Result<()> {
    debug!(
        "[DB] Renaming asset metadata key from {} to {}, new_path: {}, new_name: {}",
        old_relative_path, new_relative_path, new_file_path, new_file_name
    );
    let db_path = get_db_path().map_err(|e| {
        error!("[DB] Failed to get DB path: {}", e);
        rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e))
    })?;
     if !db_path.exists() {
        debug!("[DB] Database file not found at {}. Nothing to rename for asset: {}", db_path.display(), old_relative_path);
        return Ok(());
    }
    let conn = Connection::open(&db_path)?;
    // Note: last_modified is updated to reflect the change in the metadata record itself (key change),
    // while updated_at will be handled by the trigger.
    let changes = conn.execute(
        "UPDATE asset_metadata
         SET asset_relative_path = ?1, file_path = ?2, file_name = ?3, last_modified = CURRENT_TIMESTAMP
         WHERE asset_relative_path = ?4",
        params![new_relative_path, new_file_path, new_file_name, old_relative_path],
    )?;

    if changes > 0 {
        info!(
            "[DB] Asset metadata key renamed successfully from {} to {} ({} rows affected)",
            old_relative_path, new_relative_path, changes
        );
    } else {
        debug!("[DB] No asset metadata found to rename for old key: {}", old_relative_path);
    }
    Ok(())
}

// --- Custom Field Definition Functions ---

use crate::projectview::shared_types::{CustomFieldDefinition, CustomFieldScope};

pub fn add_custom_field_definition(definition: &CustomFieldDefinition) -> Result<()> {
    debug!("[DB] Adding custom field definition: {}", definition.field_key);
    let db_path = get_db_path().map_err(|e| rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e)))?;
    let conn = Connection::open(db_path)?;

    conn.execute(
        "INSERT INTO custom_field_definitions (field_key, field_name, field_type, scope, default_value, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            definition.field_key,
            definition.field_name,
            definition.field_type,
            definition.scope.to_db_string(),
            definition.default_value,
            definition.created_at,
            definition.updated_at
        ],
    )?;
    info!("[DB] Custom field definition added successfully: {}", definition.field_key);
    Ok(())
}

pub fn get_custom_field_definition(field_key: &str) -> Result<Option<CustomFieldDefinition>> {
    debug!("[DB] Getting custom field definition for key: {}", field_key);
    let db_path = get_db_path().map_err(|e| rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e)))?;
    let conn = Connection::open(&db_path)?;

    let mut stmt = conn.prepare(
        "SELECT field_key, field_name, field_type, scope, default_value, created_at, updated_at
         FROM custom_field_definitions WHERE field_key = ?1",
    )?;

    let def_option = stmt.query_row(params![field_key], |row| {
        let scope_str: String = row.get(3)?;
        Ok(CustomFieldDefinition {
            field_key: row.get(0)?,
            field_name: row.get(1)?,
            field_type: row.get(2)?,
            scope: CustomFieldScope::from_db_string(&scope_str),
            default_value: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    }).optional()?;

    if def_option.is_some() {
        info!("[DB] Custom field definition found for key: {}", field_key);
    } else {
        info!("[DB] No custom field definition found for key: {}", field_key);
    }
    Ok(def_option)
}

pub fn get_all_custom_field_definitions() -> Result<Vec<CustomFieldDefinition>> {
    debug!("[DB] Getting all custom field definitions");
    let db_path = get_db_path().map_err(|e| rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e)))?;
    let conn = Connection::open(db_path)?;

    let mut stmt = conn.prepare(
        "SELECT field_key, field_name, field_type, scope, default_value, created_at, updated_at
         FROM custom_field_definitions",
    )?;

    let def_iter = stmt.query_map([], |row| {
        let scope_str: String = row.get(3)?;
        Ok(CustomFieldDefinition {
            field_key: row.get(0)?,
            field_name: row.get(1)?,
            field_type: row.get(2)?,
            scope: CustomFieldScope::from_db_string(&scope_str),
            default_value: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    })?;

    let mut definitions = Vec::new();
    for def in def_iter {
        definitions.push(def?);
    }
    info!("[DB] Retrieved {} custom field definitions.", definitions.len());
    Ok(definitions)
}

pub fn update_custom_field_definition(definition: &CustomFieldDefinition) -> Result<()> {
    debug!("[DB] Updating custom field definition: {}", definition.field_key);
    let db_path = get_db_path().map_err(|e| rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e)))?;
    let conn = Connection::open(db_path)?;
    // The trigger 'update_custom_field_definitions_updated_at' will handle updating 'updated_at'.
    let changes = conn.execute(
        "UPDATE custom_field_definitions
         SET field_name = ?1, field_type = ?2, scope = ?3, default_value = ?4
         WHERE field_key = ?5",
        params![
            definition.field_name,
            definition.field_type,
            definition.scope.to_db_string(),
            definition.default_value,
            definition.field_key
        ],
    )?;

    if changes > 0 {
        info!("[DB] Custom field definition updated successfully: {}", definition.field_key);
    } else {
        info!("[DB] No custom field definition found to update for key: {}", definition.field_key);
    }
    Ok(())
}

pub fn delete_custom_field_definition(field_key: &str) -> Result<()> {
    debug!("[DB] Deleting custom field definition: {}", field_key);
    let db_path = get_db_path().map_err(|e| rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e)))?;
    let conn = Connection::open(db_path)?;

    let changes = conn.execute(
        "DELETE FROM custom_field_definitions WHERE field_key = ?1",
        params![field_key],
    )?;

    if changes > 0 {
        info!("[DB] Custom field definition deleted successfully: {}", field_key);
    } else {
        info!("[DB] No custom field definition found to delete for key: {}", field_key);
    }
    Ok(())
}

// --- End Custom Field Definition Functions ---


pub fn load_annotations_from_db(document_path: &str, doc_type: &str) -> Result<Option<String>> {
    debug!("[DB] Loading annotations for: {} (type: {})", document_path, doc_type);
    let db_path = get_db_path().map_err(|e| rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e)))?;
    if !db_path.exists() {
        debug!("[DB] Database file not found at {}. Returning None.", db_path.display());
        return Ok(None);
    }
    let conn = Connection::open(&db_path)?;
    let mut stmt = conn.prepare("SELECT annotations_json FROM pdf_annotations WHERE pdf_document_path = ?1 AND document_type = ?2")?;
    let result = stmt.query_row(params![document_path, doc_type], |row| row.get(0)).optional()?;
    debug!("[DB] Load result for {} (type: {}): {}", document_path, doc_type, if result.is_some() { "Some(...)" } else { "None" });
    Ok(result)
}

pub fn save_annotations_to_db(document_path: &str, annotations_json: &str, doc_type: &str) -> Result<()> {
    debug!("[DB] Saving annotations for: {} (type: {})", document_path, doc_type);
    let db_path = get_db_path().map_err(|e| rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e)))?;
    let conn = Connection::open(&db_path)?;

    if let Some(parent_dir) = db_path.parent() {
        if !parent_dir.exists() {
            fs::create_dir_all(parent_dir).map_err(|e| rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(format!("Failed to create db directory: {}",e))))?;
        }
    }

    conn.execute(
        "INSERT INTO pdf_annotations (pdf_document_path, annotations_json, document_type)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(pdf_document_path)
         DO UPDATE SET annotations_json = excluded.annotations_json,
                       document_type = excluded.document_type,
                       updated_at = CURRENT_TIMESTAMP",
        params![document_path, annotations_json, doc_type],
    )?;
    info!("[DB] Annotations saved successfully for: {} (type: {})", document_path, doc_type);
    Ok(())
}

pub fn delete_annotations_from_db(document_path: &str, doc_type: &str) -> Result<()> {
    debug!("[DB] Deleting annotations for: {} (type: {})", document_path, doc_type);
    let db_path = get_db_path().map_err(|e| rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e)))?;
     if !db_path.exists() {
        debug!("[DB] Database file not found at {}. Nothing to delete for {} (type: {}).", db_path.display(), document_path, doc_type);
        return Ok(());
    }
    let conn = Connection::open(&db_path)?;
    let changes = conn.execute("DELETE FROM pdf_annotations WHERE pdf_document_path = ?1 AND document_type = ?2", params![document_path, doc_type])?;
    if changes > 0 {
        info!("[DB] Annotations deleted successfully for: {} (type: {}) ({} rows affected)", document_path, doc_type, changes);
    } else {
        debug!("[DB] No annotations found to delete for: {} (type: {})", document_path, doc_type);
    }
    Ok(())
}

pub fn rename_annotations_in_db(old_document_path: &str, new_document_path: &str, doc_type: &str) -> Result<()> {
    debug!("[DB] Renaming annotations from {} to {} (type: {})", old_document_path, new_document_path, doc_type);
    let db_path = get_db_path().map_err(|e| rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e)))?;
    if !db_path.exists() {
        debug!("[DB] Database file not found at {}. Nothing to rename for {} (type: {}).", db_path.display(), old_document_path, doc_type);
        return Ok(());
    }
    let conn = Connection::open(&db_path)?;
    let changes = conn.execute(
        "UPDATE pdf_annotations SET pdf_document_path = ?1 WHERE pdf_document_path = ?2 AND document_type = ?3",
        params![new_document_path, old_document_path, doc_type],
    )?;
    if changes > 0 {
        info!("[DB] Annotations renamed successfully from {} to {} (type: {}) ({} rows affected)", old_document_path, new_document_path, doc_type, changes);
    } else {
        debug!("[DB] No annotations found to rename for old path: {} (type: {})", old_document_path, doc_type);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    use crate::welcome::config::CONFIG_DIR_NAME;

    fn init_db_at_path_for_test(path: &PathBuf) -> Result<()> {
        let conn = Connection::open(path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS pdf_annotations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                pdf_document_path TEXT NOT NULL UNIQUE,
                annotations_json TEXT NOT NULL,
                document_type TEXT NOT NULL DEFAULT 'pdf',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS update_pdf_annotations_updated_at
            AFTER UPDATE ON pdf_annotations FOR EACH ROW BEGIN
                UPDATE pdf_annotations SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
            END;",
            [],
        )?;
        Ok(())
    }

    #[test]
    fn test_init_db_adds_column_if_not_exists() {
        let temp_base = tempdir().unwrap();
        let config_dir_path = temp_base.path().join(".harvey");
        fs::create_dir_all(&config_dir_path).unwrap();
        let test_db_path = config_dir_path.join(DB_FILE_NAME);

        if test_db_path.exists() { fs::remove_file(&test_db_path).unwrap(); }

        // 1. Initialize DB with old schema (without document_type)
        {
            let conn = Connection::open(&test_db_path).unwrap();
            conn.execute(
                "CREATE TABLE pdf_annotations (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    pdf_document_path TEXT NOT NULL UNIQUE,
                    annotations_json TEXT NOT NULL,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )",
                [],
            ).unwrap();
        } // Connection closed here

        // Mock get_config_dir to point to our temp_base for this test scope
        // This is a conceptual mock. In a real scenario, you might use a crate like `config_rs`
        // with environment variable overrides, or feature flags for test-specific config paths.
        // For this test, we'll assume that `get_db_path` inside `init_db` will somehow resolve to `test_db_path`.
        // This typically requires `get_config_dir` to be mockable or the test environment
        // to be set up such that `directories::UserDirs::new()` (if used by get_config_dir)
        // would point to a controlled location.
        // For simplicity, we'll assume `init_db` called below will use `test_db_path`
        // because of how `get_db_path` is constructed (relative to some base dir).
        // This is the most complex part of testing `init_db` directly without DI for path.

        // To make this test work reliably without actual mocking of `get_config_dir` (which is hard in this setup):
        // We will call a modified init_db that takes a path.
        fn init_db_at_specific_path(db_path_param: &PathBuf) -> Result<()> {
            let conn = Connection::open(db_path_param)?;
            conn.execute( // Original create table (as if it's an old DB)
                "CREATE TABLE IF NOT EXISTS pdf_annotations (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    pdf_document_path TEXT NOT NULL UNIQUE,
                    annotations_json TEXT NOT NULL,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )",
                [],
            )?;
             // Now, run the ALTER TABLE logic from the main init_db function
            let mut stmt_check = conn.prepare("PRAGMA table_info(pdf_annotations)")?;
            let column_exists = stmt_check.query_map([], |row| row.get::<_, String>(1))?
                                    .any(|col_name_result| col_name_result.map_or(false, |name| name == "document_type"));
            if !column_exists {
                conn.execute("ALTER TABLE pdf_annotations ADD COLUMN document_type TEXT NOT NULL DEFAULT 'pdf'", [])?;
            }
            // Trigger part from main init_db
            conn.execute(
                "CREATE TRIGGER IF NOT EXISTS update_pdf_annotations_updated_at
                AFTER UPDATE ON pdf_annotations FOR EACH ROW BEGIN
                    UPDATE pdf_annotations SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
                END;",
                [],
            )?;
            Ok(())
        }

        assert!(init_db_at_specific_path(&test_db_path).is_ok());

        // Verify column exists
        let conn_check = Connection::open(&test_db_path).unwrap();
        let mut stmt_verify = conn_check.prepare("PRAGMA table_info(pdf_annotations)").unwrap();
        let columns: Vec<String> = stmt_verify.query_map([], |row| row.get(1)).unwrap().map(|r| r.unwrap()).collect();
        assert!(columns.contains(&"document_type".to_string()));

        fs::remove_file(&test_db_path).unwrap();
    }


    #[test]
    fn test_save_and_load_annotations() {
        let temp_base = tempdir().unwrap();
        let config_dir_path = temp_base.path().join(".harvey");
        fs::create_dir_all(&config_dir_path).unwrap();
        let test_db_path = config_dir_path.join(DB_FILE_NAME);

        init_db_at_path_for_test(&test_db_path).unwrap();

        fn save_annotations_to_db_at_path(db_path: &PathBuf, doc_path: &str, json: &str, doc_type: &str) -> Result<()> {
            let conn = Connection::open(db_path)?;
            conn.execute(
                "INSERT INTO pdf_annotations (pdf_document_path, annotations_json, document_type) VALUES (?1, ?2, ?3)
                 ON CONFLICT(pdf_document_path) DO UPDATE SET annotations_json = excluded.annotations_json, document_type = excluded.document_type",
                params![doc_path, json, doc_type]
            )?;
            Ok(())
        }
        fn load_annotations_from_db_at_path(db_path: &PathBuf, doc_path: &str, doc_type: &str) -> Result<Option<String>> {
            let conn = Connection::open(db_path)?;
            let mut stmt = conn.prepare("SELECT annotations_json FROM pdf_annotations WHERE pdf_document_path = ?1 AND document_type = ?2")?;
            stmt.query_row(params![doc_path, doc_type], |row| row.get(0)).optional()
        }

        let doc_path1 = "test/doc1.pdf";
        let annots1 = "[{\"id\":\"1\"}]";
        let doc_type1 = "pdf";
        assert!(save_annotations_to_db_at_path(&test_db_path, doc_path1, annots1, doc_type1).is_ok());

        let loaded_annots1 = load_annotations_from_db_at_path(&test_db_path, doc_path1, doc_type1).unwrap();
        assert_eq!(loaded_annots1, Some(annots1.to_string()));

        let annots1_updated = "[{\"id\":\"1\", \"text\":\"updated\"}]";
        assert!(save_annotations_to_db_at_path(&test_db_path, doc_path1, annots1_updated, doc_type1).is_ok());
        let loaded_annots1_updated = load_annotations_from_db_at_path(&test_db_path, doc_path1, doc_type1).unwrap();
        assert_eq!(loaded_annots1_updated, Some(annots1_updated.to_string()));

        // Test with a different doc_type for the same path - this should fail due to UNIQUE constraint on pdf_document_path
        // if we were not using ON CONFLICT DO UPDATE. With ON CONFLICT, it will update.
        let doc_path2_img = "test/doc1.pdf"; // Same path
        let annots2_img = "[{\"id\":\"img1\"}]";
        let doc_type2_img = "image";
        // This will update the existing row for "test/doc1.pdf", changing its type and annotations.
        assert!(save_annotations_to_db_at_path(&test_db_path, doc_path2_img, annots2_img, doc_type2_img).is_ok());

        // Try to load the original "pdf" type - should be None now
        let loaded_original_pdf_type = load_annotations_from_db_at_path(&test_db_path, doc_path1, doc_type1).unwrap();
        assert!(loaded_original_pdf_type.is_none());

        // Load the "image" type - should be Some
        let loaded_image_type = load_annotations_from_db_at_path(&test_db_path, doc_path2_img, doc_type2_img).unwrap();
        assert_eq!(loaded_image_type, Some(annots2_img.to_string()));


        let loaded_non_existent = load_annotations_from_db_at_path(&test_db_path, "other.pdf", "pdf").unwrap();
        assert!(loaded_non_existent.is_none());

        fs::remove_file(&test_db_path).unwrap();
    }

    #[test]
    fn test_delete_annotations() {
        let temp_base = tempdir().unwrap();
        let config_dir_path = temp_base.path().join(".harvey");
        fs::create_dir_all(&config_dir_path).unwrap();
        let test_db_path = config_dir_path.join(DB_FILE_NAME);
        init_db_at_path_for_test(&test_db_path).unwrap();

        fn save_to_db(p: &PathBuf, pp: &str, j: &str, dt: &str) -> Result<()> { Connection::open(p)?.execute("INSERT INTO pdf_annotations (pdf_document_path, annotations_json, document_type) VALUES (?1, ?2, ?3)", params![pp, j, dt])?; Ok(()) }
        fn delete_from_db(p: &PathBuf, pp: &str, dt: &str) -> Result<()> { Connection::open(p)?.execute("DELETE FROM pdf_annotations WHERE pdf_document_path = ?1 AND document_type = ?2", params![pp, dt])?; Ok(()) }
        fn load_from_db(p: &PathBuf, pp: &str, dt: &str) -> Result<Option<String>> { Connection::open(p)?.query_row("SELECT annotations_json FROM pdf_annotations WHERE pdf_document_path = ?1 AND document_type = ?2", params![pp, dt], |r| r.get(0)).optional() }

        save_to_db(&test_db_path, "doc1.pdf", "[]", "pdf").unwrap();
        assert!(load_from_db(&test_db_path, "doc1.pdf", "pdf").unwrap().is_some());
        assert!(delete_from_db(&test_db_path, "doc1.pdf", "pdf").is_ok());
        assert!(load_from_db(&test_db_path, "doc1.pdf", "pdf").unwrap().is_none());

        assert!(delete_from_db(&test_db_path, "non_existent.pdf", "pdf").is_ok());
        fs::remove_file(&test_db_path).unwrap();
    }

    #[test]
    fn test_rename_annotations() {
        let temp_base = tempdir().unwrap();
        let config_dir_path = temp_base.path().join(".harvey");
        fs::create_dir_all(&config_dir_path).unwrap();
        let test_db_path = config_dir_path.join(DB_FILE_NAME);
        init_db_at_path_for_test(&test_db_path).unwrap();

        fn save_to_db(p: &PathBuf, pp: &str, j: &str, dt: &str) -> Result<()> { Connection::open(p)?.execute("INSERT INTO pdf_annotations (pdf_document_path, annotations_json, document_type) VALUES (?1, ?2, ?3)", params![pp, j, dt])?; Ok(()) }
        fn rename_in_db(p: &PathBuf, old_pp: &str, new_pp: &str, dt: &str) -> Result<()> { Connection::open(p)?.execute("UPDATE pdf_annotations SET pdf_document_path = ?1 WHERE pdf_document_path = ?2 AND document_type = ?3", params![new_pp, old_pp, dt])?; Ok(()) }
        fn load_from_db(p: &PathBuf, pp: &str, dt: &str) -> Result<Option<String>> { Connection::open(p)?.query_row("SELECT annotations_json FROM pdf_annotations WHERE pdf_document_path = ?1 AND document_type = ?2", params![pp, dt], |r| r.get(0)).optional() }

        save_to_db(&test_db_path, "old.pdf", "[old]", "pdf").unwrap();
        assert!(rename_in_db(&test_db_path, "old.pdf", "new.pdf", "pdf").is_ok());

        assert!(load_from_db(&test_db_path, "old.pdf", "pdf").unwrap().is_none());
        assert_eq!(load_from_db(&test_db_path, "new.pdf", "pdf").unwrap(), Some("[old]".to_string()));

        assert!(rename_in_db(&test_db_path, "non_existent.pdf", "another.pdf", "pdf").is_ok());
        fs::remove_file(&test_db_path).unwrap();
    }
}
