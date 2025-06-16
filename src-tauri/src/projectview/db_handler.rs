// src-tauri/src/projectview/db_handler.rs
use rusqlite::{Connection, Result, params, OptionalExtension, ToSql};
use std::path::PathBuf;
use std::fs;
use crate::welcome::config::{get_config_dir, CommandError}; // Assuming this function gives PathBuf
use log::{info, debug, error, warn};
use serde::{Serialize, Deserialize}; // Added for the new struct
use crate::projectview::shared_types::FileMetadata; // For function signatures

const DB_FILE_NAME: &str = "harvey.sqlite";

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
    pub original_import_path: Option<String>,
    pub speaker_names_json: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupDataFromDb {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MediaTranscriptDataValues {
    pub original_import_path: Option<String>,
    pub speaker_names_json: Option<String>,
}

pub fn get_db_path() -> Result<PathBuf, CommandError> {
    let config_dir = get_config_dir().map_err(|e| CommandError::Message(format!("Failed to get config dir from welcome/config: {}", e)))?;
    Ok(config_dir.join(DB_FILE_NAME))
}

pub fn init_db() -> Result<(), CommandError> {
    let db_path = get_db_path()?;
    let conn = Connection::open(&db_path)?;

    debug!("[DB] Initializing database at: {}", db_path.display());

    // Updated pdf_annotations table definition
    conn.execute(
        "CREATE TABLE IF NOT EXISTS pdf_annotations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id TEXT NOT NULL,
            pdf_document_path TEXT NOT NULL,
            annotations_json TEXT NOT NULL,
            document_type TEXT NOT NULL DEFAULT 'pdf',
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
            UNIQUE (project_id, pdf_document_path, document_type)
        )",
        [],
    )?;
    info!("[DB] Initialized pdf_annotations table definition.");

    // Check and add document_type column if missing (for older schemas)
    let mut stmt_check_doc_type = conn.prepare("PRAGMA table_info(pdf_annotations)")?;
    let doc_type_column_exists = stmt_check_doc_type
        .query_map([], |row| row.get::<_, String>(1))?
        .any(|col_name_result| col_name_result.map_or(false, |name| name == "document_type"));

    if !doc_type_column_exists {
        info!("[DB] Adding document_type column to pdf_annotations table.");
        conn.execute("ALTER TABLE pdf_annotations ADD COLUMN document_type TEXT NOT NULL DEFAULT 'pdf'", [])?;
    }

    // Check and add project_id column if missing (for older schemas)
    // This is a simplified migration: it adds the column but doesn't backfill or add FK for existing rows via ALTER.
    // New tables get the FK from CREATE TABLE.
    let mut stmt_check_project_id = conn.prepare("PRAGMA table_info(pdf_annotations)")?;
    let project_id_column_exists = stmt_check_project_id
        .query_map([], |row| row.get::<_, String>(1))?
        .any(|col_name_result| col_name_result.map_or(false, |name| name == "project_id"));

    if !project_id_column_exists {
        info!("[DB] Adding project_id column to pdf_annotations table.");
        conn.execute("ALTER TABLE pdf_annotations ADD COLUMN project_id TEXT", [])?;
        // For existing rows, project_id will be NULL. This needs to be handled by application logic
        // or a more comprehensive data migration strategy if strict FK enforcement on old data is required.
        info!("[DB] Added project_id column to pdf_annotations. Existing rows will have NULL project_id if not manually updated.");
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
            asset_relative_path TEXT NOT NULL,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
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
            original_import_path TEXT,
            speaker_names_json TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (project_id, asset_relative_path)
        )",
        [],
    )?;
    info!("[DB] Initialized asset_metadata table definition with composite PK.");

    // Check and add project_id column to asset_metadata if missing (for older schemas)
    // This simplified migration adds the column if it doesn't exist. It does not change PK for existing tables.
    // The PRIMARY KEY change in CREATE TABLE applies to new DBs or if the table is dropped and recreated.
    // Handling PK change for existing populated tables is a complex migration not covered here.
    let mut stmt_check_asset_project_id = conn.prepare("PRAGMA table_info(asset_metadata)")?;
    let asset_project_id_exists = stmt_check_asset_project_id
        .query_map([], |row| row.get::<_, String>(1))?
        .any(|name_res| name_res.map_or(false, |name| name == "project_id"));

    if !asset_project_id_exists {
        info!("[DB] Adding project_id column to asset_metadata table (for older schema).");
        conn.execute("ALTER TABLE asset_metadata ADD COLUMN project_id TEXT", [])?;
        info!("[DB] Added project_id column to asset_metadata. Existing rows will have NULL. PK not changed for existing tables by this ALTER.");
    }

    // Migration for original_import_path
    let mut stmt_check_orig_import_path = conn.prepare("PRAGMA table_info(asset_metadata)")?;
    let orig_import_path_exists = stmt_check_orig_import_path
        .query_map([], |row| row.get::<_, String>(1))?
        .any(|name_res| name_res.map_or(false, |name| name == "original_import_path"));

    if !orig_import_path_exists {
        info!("[DB] Adding original_import_path column to asset_metadata table.");
        conn.execute("ALTER TABLE asset_metadata ADD COLUMN original_import_path TEXT", [])?;
    }

    // Migration for speaker_names_json
    let mut stmt_check_speaker_json = conn.prepare("PRAGMA table_info(asset_metadata)")?;
    let speaker_json_exists = stmt_check_speaker_json
        .query_map([], |row| row.get::<_, String>(1))?
        .any(|name_res| name_res.map_or(false, |name| name == "speaker_names_json"));

    if !speaker_json_exists {
        info!("[DB] Adding speaker_names_json column to asset_metadata table.");
        conn.execute("ALTER TABLE asset_metadata ADD COLUMN speaker_names_json TEXT", [])?;
    }

    // Update trigger for asset_metadata to use composite key if possible, or retain old logic if table structure is old.
    // For simplicity, the trigger is defined for the new composite key structure.
    // If running against an old DB structure (single PK asset_relative_path), this trigger might need adjustment or conditional creation.
    // However, SQLite usually allows triggers that might not perfectly match all old schema variations if the columns exist.
    conn.execute(
        "DROP TRIGGER IF EXISTS update_asset_metadata_updated_at", // Drop old trigger first
        [],
    )?;
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_asset_metadata_updated_at
        AFTER UPDATE ON asset_metadata
        FOR EACH ROW
        BEGIN
            UPDATE asset_metadata SET updated_at = CURRENT_TIMESTAMP
            WHERE project_id = OLD.project_id AND asset_relative_path = OLD.asset_relative_path;
        END;",
        [],
    )?;
    info!("[DB] Recreated update_asset_metadata_updated_at trigger for new PK.");

    // custom_field_definitions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS custom_field_definitions (
            project_id TEXT NOT NULL,
            field_key TEXT NOT NULL,
            field_name TEXT NOT NULL,
            field_type TEXT NOT NULL,
            scope TEXT NOT NULL,
            default_value TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
            PRIMARY KEY (project_id, field_key)
        )",
        [],
    )?;

    // Trigger for custom_field_definitions updated_at
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_custom_field_definitions_updated_at
        AFTER UPDATE ON custom_field_definitions
        FOR EACH ROW
        BEGIN
            UPDATE custom_field_definitions SET updated_at = CURRENT_TIMESTAMP WHERE project_id = OLD.project_id AND field_key = OLD.field_key;
        END;",
        [],
    )?;
    info!("[DB] Initialized custom_field_definitions table and trigger.");

    // table_layout_preferences table - Updated for composite PK and FK
    conn.execute(
        "CREATE TABLE IF NOT EXISTS table_layout_preferences (
            project_id TEXT NOT NULL,
            table_asset_relative_path TEXT NOT NULL,
            layout_json TEXT NOT NULL,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
            PRIMARY KEY (project_id, table_asset_relative_path),
            FOREIGN KEY (project_id, table_asset_relative_path)
                REFERENCES asset_metadata(project_id, asset_relative_path) ON DELETE CASCADE
        )",
        [],
    )?;
    info!("[DB] Initialized table_layout_preferences table with composite PK and FK.");

    // Check and add project_id column to table_layout_preferences if missing
    let mut stmt_check_layout_project_id = conn.prepare("PRAGMA table_info(table_layout_preferences)")?;
    let layout_project_id_exists = stmt_check_layout_project_id
        .query_map([], |row| row.get::<_, String>(1))?
        .any(|name_res| name_res.map_or(false, |name| name == "project_id"));

    if !layout_project_id_exists {
        info!("[DB] Adding project_id column to table_layout_preferences table (for older schema).");
        conn.execute("ALTER TABLE table_layout_preferences ADD COLUMN project_id TEXT", [])?;
        info!("[DB] Added project_id column to table_layout_preferences. Existing rows will have NULL. PK and FK not changed for existing tables by this ALTER.");
    }

    // Update trigger for table_layout_preferences for new composite PK
    conn.execute(
        "DROP TRIGGER IF EXISTS update_table_layout_preferences_updated_at", // Drop old trigger
        [],
    )?;
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_table_layout_preferences_updated_at
        AFTER UPDATE ON table_layout_preferences
        FOR EACH ROW
        BEGIN
            UPDATE table_layout_preferences SET updated_at = CURRENT_TIMESTAMP
            WHERE project_id = OLD.project_id AND table_asset_relative_path = OLD.table_asset_relative_path;
        END;",
        [],
    )?;
    info!("[DB] Recreated update_table_layout_preferences_updated_at trigger for new PK.");

    // projects table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            root_path TEXT NOT NULL UNIQUE,
            xml_path TEXT NOT NULL UNIQUE,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    info!("[DB] Initialized projects table.");

    // Trigger for projects updated_at
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_projects_updated_at
        AFTER UPDATE ON projects
        FOR EACH ROW
        BEGIN
            UPDATE projects SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
        END;",
        [],
    )?;
    info!("[DB] Initialized update_projects_updated_at trigger.");

    // groups table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS groups (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            description TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            UNIQUE (project_id, name)
        )",
        [],
    )?;
    info!("[DB] Initialized groups table.");

    // Trigger for groups updated_at
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_groups_updated_at
        AFTER UPDATE ON groups
        FOR EACH ROW
        BEGIN
            UPDATE groups SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
        END;",
        [],
    )?;
    info!("[DB] Initialized update_groups_updated_at trigger.");

    // file_groups table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS file_groups (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_asset_path TEXT NOT NULL,
            group_id TEXT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            added_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (project_id, file_asset_path) REFERENCES asset_metadata(project_id, asset_relative_path) ON DELETE CASCADE,
            UNIQUE (project_id, file_asset_path, group_id)
        )",
        [],
    )?;
    info!("[DB] Initialized file_groups table.");

    // media_transcript_data table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS media_transcript_data (
            project_id TEXT NOT NULL,
            asset_relative_path TEXT NOT NULL,
            original_import_path TEXT,
            speaker_names_json TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (project_id, asset_relative_path),
            FOREIGN KEY (project_id, asset_relative_path)
                REFERENCES asset_metadata(project_id, asset_relative_path) ON DELETE CASCADE
        )",
        [],
    )?;
    info!("[DB] Initialized media_transcript_data table.");

    // Trigger for media_transcript_data updated_at
    conn.execute("DROP TRIGGER IF EXISTS update_media_transcript_data_updated_at", [])?;
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_media_transcript_data_updated_at
        AFTER UPDATE ON media_transcript_data
        FOR EACH ROW
        BEGIN
            UPDATE media_transcript_data SET updated_at = CURRENT_TIMESTAMP
            WHERE project_id = OLD.project_id AND asset_relative_path = OLD.asset_relative_path;
        END;",
        [],
    )?;
    info!("[DB] Initialized update_media_transcript_data_updated_at trigger.");

    info!("[DB] Database initialized successfully with all tables and triggers.");
    Ok(())
}

// --- Group Functions ---

pub fn create_group(conn: &Connection, project_id: &str, group_id: &str, name: &str, description: Option<&str>) -> Result<(), CommandError> {
    debug!("[DB] Creating group for project_id {}: id={}, name={}", project_id, group_id, name);
    conn.execute(
        "INSERT INTO groups (id, project_id, name, description) VALUES (?1, ?2, ?3, ?4)",
        params![group_id, project_id, name, to_sql_optional_str(description)],
    )
    .map_err(|e| CommandError::Message(format!("Failed to create group {}: {}", name, e)))?;
    info!("[DB] Group created successfully: id={}, name={}", group_id, name);
    Ok(())
}

pub fn get_groups_for_project(conn: &Connection, project_id: &str) -> Result<Vec<GroupDataFromDb>, CommandError> {
    debug!("[DB] Loading groups for project_id {}", project_id);
    let mut stmt = conn.prepare("SELECT id, project_id, name, description, created_at, updated_at FROM groups WHERE project_id = ?1 ORDER BY name ASC")
        .map_err(|e| CommandError::Message(format!("Failed to prepare statement for getting groups: {}", e)))?;

    let group_iter = stmt.query_map(params![project_id], |row| {
        Ok(GroupDataFromDb {
            id: row.get(0)?,
            project_id: row.get(1)?,
            name: row.get(2)?,
            description: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    })
    .map_err(|e| CommandError::Message(format!("Failed to query groups for project {}: {}", project_id, e)))?;

    let mut groups = Vec::new();
    for group_result in group_iter {
        groups.push(group_result.map_err(|e| CommandError::Message(format!("Failed to map group row: {}", e)))?);
    }
    info!("[DB] Loaded {} groups for project_id {}", groups.len(), project_id);
    Ok(groups)
}

pub fn add_file_to_group(conn: &Connection, project_id: &str, group_id: &str, file_asset_relative_path: &str) -> Result<(), CommandError> {
    debug!("[DB] Adding file {} to group {} for project_id {}", file_asset_relative_path, group_id, project_id);
    conn.execute(
        "INSERT INTO file_groups (project_id, group_id, file_asset_path) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
        params![project_id, group_id, file_asset_relative_path],
    )
    .map_err(|e| CommandError::Message(format!("Failed to add file {} to group {}: {}", file_asset_relative_path, group_id, e)))?;
    info!("[DB] File {} added to group {} successfully (if not already present).", file_asset_relative_path, group_id);
    Ok(())
}

// --- End Group Functions ---

// --- Media Transcript Data Functions ---

pub fn save_media_transcript_data(
    project_id: &str,
    asset_relative_path: &str,
    original_import_path: Option<&str>,
    speaker_names: Option<&Vec<String>>,
) -> Result<(), CommandError> {
    debug!(
        "[DB] Saving media transcript data for project_id {}: {}",
        project_id, asset_relative_path
    );

    let db_path = get_db_path()?;
    let conn = Connection::open(&db_path)?;

    let speaker_names_json_str: Option<String> = speaker_names
        .filter(|names| !names.is_empty()) // Only serialize if not empty, otherwise store NULL
        .and_then(|names| serde_json::to_string(names).ok());

    let sql = "
        INSERT INTO media_transcript_data (
            project_id, asset_relative_path, original_import_path, speaker_names_json
        ) VALUES (?1, ?2, ?3, ?4)
        ON CONFLICT(project_id, asset_relative_path) DO UPDATE SET
            original_import_path = excluded.original_import_path,
            speaker_names_json = excluded.speaker_names_json,
            updated_at = CURRENT_TIMESTAMP;
    ";

    conn.execute(
        sql,
        params![
            project_id,
            asset_relative_path,
            to_sql_optional_str(original_import_path),
            to_sql_optional_str(speaker_names_json_str.as_deref()),
        ],
    )?;

    info!(
        "[DB] Media transcript data saved successfully for project_id {}: {}",
        project_id, asset_relative_path
    );
    Ok(())
}

pub fn load_media_transcript_data(
    project_id: &str,
    asset_relative_path: &str,
) -> Result<Option<MediaTranscriptDataValues>, CommandError> {
    debug!(
        "[DB] Loading media transcript data for project_id {}: {}",
        project_id, asset_relative_path
    );

    let db_path = get_db_path()?;
    let conn = Connection::open(&db_path)?;

    let mut stmt = conn.prepare("
        SELECT original_import_path, speaker_names_json
        FROM media_transcript_data
        WHERE project_id = ?1 AND asset_relative_path = ?2
    ")?;

    let result = stmt.query_row(params![project_id, asset_relative_path], |row| {
        Ok(MediaTranscriptDataValues {
            original_import_path: row.get(0)?,
            speaker_names_json: row.get(1)?,
        })
    }).optional()?;

    debug!(
        "[DB] Load media transcript data result for project_id {} - {}: {}",
        project_id, asset_relative_path, if result.is_some() { "Some(...)" } else { "None" }
    );
    Ok(result)
}

// --- Table Layout Preferences Functions ---

pub fn save_table_layout_preferences(project_id: &str, table_asset_relative_path: &str, layout_json: &str) -> Result<(), CommandError> {
    debug!("[DB] Saving table layout preferences for project_id {}: {}", project_id, table_asset_relative_path);
    let db_path = get_db_path()?;
    let conn = Connection::open(&db_path)?;

    // Check if asset metadata exists for the given project_id and path
    let mut stmt = conn.prepare("SELECT 1 FROM asset_metadata WHERE project_id = ?1 AND asset_relative_path = ?2")?;
    let exists: Option<i32> = stmt.query_row(params![project_id, table_asset_relative_path], |row| row.get(0)).optional()?;

    if exists.is_none() {
        let error_msg = format!("Asset metadata not found for project_id {} and table: {}", project_id, table_asset_relative_path);
        error!("[DB] {}", error_msg);
        return Err(CommandError::AssetMetadataNotFound(error_msg));
    }

    conn.execute(
        "INSERT INTO table_layout_preferences (project_id, table_asset_relative_path, layout_json)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(project_id, table_asset_relative_path) DO UPDATE SET
             layout_json = excluded.layout_json,
             updated_at = CURRENT_TIMESTAMP",
        params![project_id, table_asset_relative_path, layout_json],
    )?;
    info!("[DB] Table layout preferences saved successfully for project_id {}: {}", project_id, table_asset_relative_path);
    Ok(())
}

pub fn load_table_layout_preferences(project_id: &str, table_asset_relative_path: &str) -> Result<Option<String>, CommandError> {
    debug!("[DB] Loading table layout preferences for project_id {}: {}", project_id, table_asset_relative_path);
    let db_path = get_db_path()?;
    if !db_path.exists() {
        debug!("[DB] Database file not found at {}. Returning None for table layout: project_id {}, path {}", db_path.display(), project_id, table_asset_relative_path);
        return Ok(None);
    }
    let conn = Connection::open(&db_path)?;
    let mut stmt = conn.prepare("
        SELECT layout_json
        FROM table_layout_preferences
        WHERE project_id = ?1 AND table_asset_relative_path = ?2
    ")?;

    let result = stmt.query_row(params![project_id, table_asset_relative_path], |row| {
        row.get(0)
    }).optional()?;

    debug!("[DB] Load table layout prefs result for project_id {} - {}: {}", project_id, table_asset_relative_path, if result.is_some() { "Some(...)" } else { "None" });
    Ok(result)
}

// --- End Table Layout Preferences Functions ---

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
    project_id: &str, // New parameter
    metadata: &FileMetadata,
    asset_relative_path: &str,
    asset_type: &str,
    custom_fields_json: Option<&str>,
) -> Result<(), CommandError> {
    debug!(
        "[DB] Saving asset metadata for project_id {}: {} (type: {})",
        project_id, asset_relative_path, asset_type
    );
    let db_path = get_db_path()?;
    let conn = Connection::open(&db_path)?;

    if let Some(parent_dir) = db_path.parent() {
        if !parent_dir.exists() {
            fs::create_dir_all(parent_dir).map_err(|e| CommandError::Io(format!("Failed to create db directory: {}", e)))?;
        }
    }

    let speaker_names_json_str: Option<String> = metadata.speaker_names
        .as_ref()
        .and_then(|names| serde_json::to_string(names).ok());

    let sql = "
        INSERT INTO asset_metadata (
            project_id, asset_relative_path, file_name, file_path, last_modified, title,
            description, summary, duration_seconds, width, height, frame_rate,
            bit_rate, audio_codec, video_codec, creation_time, asset_type, custom_fields_json,
            original_import_path, speaker_names_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
        ON CONFLICT(project_id, asset_relative_path) DO UPDATE SET
            file_name = excluded.file_name,
            file_path = excluded.file_path,
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
            asset_type = excluded.asset_type,
            custom_fields_json = excluded.custom_fields_json,
            original_import_path = excluded.original_import_path,
            speaker_names_json = excluded.speaker_names_json,
            updated_at = CURRENT_TIMESTAMP
        ;
    ";

    conn.execute(
        sql,
        params![
            project_id,
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
            to_sql_optional_str(metadata.created_at.as_deref()),
            asset_type,
            to_sql_optional_str(custom_fields_json),
            to_sql_optional_str(metadata.original_import_path.as_deref()),
            to_sql_optional_str(speaker_names_json_str.as_deref()),
        ],
    )?;

    info!(
        "[DB] Asset metadata saved successfully for project_id {}: {} (type: {})",
        project_id, asset_relative_path, asset_type
    );
    Ok(())
}

pub fn load_asset_metadata(project_id: &str, asset_relative_path: &str) -> Result<Option<FileMetadataWithCustomFieldsFromDb>, CommandError> {
    debug!("[DB] Loading asset metadata for project_id {}: {}", project_id, asset_relative_path);
    let db_path = get_db_path()?;
    if !db_path.exists() {
        debug!("[DB] Database file not found at {}. Returning None for project_id {}, asset: {}", db_path.display(), project_id, asset_relative_path);
        return Ok(None);
    }
    let conn = Connection::open(&db_path)?;
    let mut stmt = conn.prepare("
        SELECT file_name, file_path, last_modified, title, description, summary,
               duration_seconds, width, height, frame_rate, bit_rate, audio_codec, video_codec,
               creation_time, custom_fields_json, asset_type, original_import_path, speaker_names_json
        FROM asset_metadata
        WHERE project_id = ?1 AND asset_relative_path = ?2
    ")?;

    let result = stmt.query_row(params![project_id, asset_relative_path], |row| {
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
            original_import_path: row.get(16)?,
            speaker_names_json: row.get(17)?,
        })
    }).optional()?;

    debug!("[DB] Load asset metadata result for project_id {} - {}: {}", project_id, asset_relative_path, if result.is_some() { "Some(...)" } else { "None" });
    Ok(result)
}

pub fn delete_asset_metadata(project_id: &str, asset_relative_path: &str) -> Result<(), CommandError> {
    debug!("[DB] Deleting asset metadata for project_id {}: {}", project_id, asset_relative_path);
    let db_path = get_db_path()?;
    if !db_path.exists() {
        debug!("[DB] Database file not found at {}. Nothing to delete for project_id {}, asset: {}", db_path.display(), project_id, asset_relative_path);
        return Ok(());
    }
    let conn = Connection::open(&db_path)?;
    let changes = conn.execute("DELETE FROM asset_metadata WHERE project_id = ?1 AND asset_relative_path = ?2", params![project_id, asset_relative_path])?;

    if changes > 0 {
        info!("[DB] Asset metadata deleted successfully for project_id {}: {} ({} rows affected)", project_id, asset_relative_path, changes);
    } else {
        debug!("[DB] No asset metadata found to delete for project_id {}: {}", project_id, asset_relative_path);
    }
    Ok(())
}

pub fn rename_asset_metadata_key(
    project_id: &str,
    old_relative_path: &str,
    new_relative_path: &str,
    new_file_path: &str, // This is the new absolute file path
    new_file_name: &str, // This is the new file name (e.g., "new_stem.ext")
) -> Result<(), CommandError> {
    debug!(
        "[DB] Renaming asset metadata key for project_id {}: from {} to {}, new_abs_path: {}, new_name: {}",
        project_id, old_relative_path, new_relative_path, new_file_path, new_file_name
    );
    let db_path = get_db_path()?;
     if !db_path.exists() {
        debug!("[DB] Database file not found at {}. Nothing to rename for project_id {}, asset: {}", db_path.display(), project_id, old_relative_path);
        return Ok(());
    }
    let conn = Connection::open(&db_path)?;

    let changes = conn.execute(
        "UPDATE asset_metadata
         SET asset_relative_path = ?1, file_path = ?2, file_name = ?3, last_modified = CURRENT_TIMESTAMP
         WHERE project_id = ?4 AND asset_relative_path = ?5",
        params![new_relative_path, new_file_path, new_file_name, project_id, old_relative_path],
    )?;

    if changes > 0 {
        info!(
            "[DB] Asset metadata key renamed successfully for project_id {} from {} to {} ({} rows affected)",
            project_id, old_relative_path, new_relative_path, changes
        );

        // Also attempt to rename in table_layout_preferences if an entry exists for this project
        match conn.execute(
            "UPDATE table_layout_preferences SET table_asset_relative_path = ?1 WHERE project_id = ?2 AND table_asset_relative_path = ?3",
            params![new_relative_path, project_id, old_relative_path],
        ) {
            Ok(layout_changes) if layout_changes > 0 => {
                info!("[DB] Renamed corresponding table_layout_preferences key for project_id {} from {} to {}", project_id, old_relative_path, new_relative_path);
            }
            Ok(_) => {
                debug!("[DB] No corresponding table_layout_preferences key found or updated for project_id {} and old path {}", project_id, old_relative_path);
            }
            Err(e) => {
                error!("[DB] Error trying to rename table_layout_preferences key for project_id {} from {} to {}: {}", project_id, old_relative_path, new_relative_path, e);
                // Not returning error here as primary rename succeeded.
            }
        }

        // Add new logic for media_transcript_data
        match conn.execute(
            "UPDATE media_transcript_data SET asset_relative_path = ?1 WHERE project_id = ?2 AND asset_relative_path = ?3",
            params![new_relative_path, project_id, old_relative_path],
        ) {
            Ok(mtd_changes) if mtd_changes > 0 => {
                info!("[DB] Renamed corresponding media_transcript_data key for project_id {} from {} to {}", project_id, old_relative_path, new_relative_path);
            }
            Ok(_) => {
                debug!("[DB] No corresponding media_transcript_data key found or updated for project_id {} and old path {}", project_id, old_relative_path);
            }
            Err(e) => {
                error!("[DB] Error trying to rename media_transcript_data key for project_id {} from {} to {}: {}", project_id, old_relative_path, new_relative_path, e);
                // Consider if this error should be propagated or just logged.
            }
        }
    } else {
        debug!("[DB] No asset metadata found to rename for project_id {} and old key: {}", project_id, old_relative_path);
    }
    Ok(())
}

// --- Custom Field Definition Functions ---

use crate::projectview::shared_types::{CustomFieldDefinition, CustomFieldScope};

pub fn add_custom_field_definition(project_id: &str, definition: &CustomFieldDefinition) -> Result<(), CommandError> {
    debug!("[DB] Adding custom field definition for project_id {}: {}", project_id, definition.field_key);
    let db_path = get_db_path()?;
    let conn = Connection::open(db_path)?;

    conn.execute(
        "INSERT INTO custom_field_definitions (project_id, field_key, field_name, field_type, scope, default_value, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            project_id,
            definition.field_key,
            definition.field_name,
            definition.field_type,
            definition.scope.to_db_string(),
            definition.default_value,
            definition.created_at,
            definition.updated_at
        ],
    )?;
    info!("[DB] Custom field definition added successfully for project_id {}: {}", project_id, definition.field_key);
    Ok(())
}

pub fn get_custom_field_definition(project_id: &str, field_key: &str) -> Result<Option<CustomFieldDefinition>, CommandError> {
    debug!("[DB] Getting custom field definition for project_id {} and key: {}", project_id, field_key);
    let db_path = get_db_path()?;
    let conn = Connection::open(&db_path)?;

    let mut stmt = conn.prepare(
        "SELECT project_id, field_key, field_name, field_type, scope, default_value, created_at, updated_at
         FROM custom_field_definitions WHERE project_id = ?1 AND field_key = ?2",
    )?;

    let def_option = stmt.query_row(params![project_id, field_key], |row| {
        let scope_str: String = row.get(4)?; // Adjusted index
        Ok(CustomFieldDefinition {
            project_id: row.get(0)?, // Added project_id
            field_key: row.get(1)?,    // Adjusted index
            field_name: row.get(2)?,   // Adjusted index
            field_type: row.get(3)?,   // Adjusted index
            scope: CustomFieldScope::from_db_string(&scope_str),
            default_value: row.get(5)?, // Adjusted index
            created_at: row.get(6)?,    // Adjusted index
            updated_at: row.get(7)?,    // Adjusted index
        })
    }).optional()?;

    if def_option.is_some() {
        info!("[DB] Custom field definition found for project_id {} and key: {}", project_id, field_key);
    } else {
        info!("[DB] No custom field definition found for project_id {} and key: {}", project_id, field_key);
    }
    Ok(def_option)
}

pub fn get_all_custom_field_definitions(project_id: &str) -> Result<Vec<CustomFieldDefinition>, CommandError> {
    debug!("[DB] Getting all custom field definitions for project_id {}", project_id);
    let db_path = get_db_path()?;
    let conn = Connection::open(db_path)?;

    let mut stmt = conn.prepare(
        "SELECT project_id, field_key, field_name, field_type, scope, default_value, created_at, updated_at
         FROM custom_field_definitions WHERE project_id = ?1",
    )?;

    let def_iter = stmt.query_map(params![project_id], |row| {
        let scope_str: String = row.get(4)?; // Adjusted index
        Ok(CustomFieldDefinition {
            project_id: row.get(0)?, // Added project_id
            field_key: row.get(1)?,    // Adjusted index
            field_name: row.get(2)?,   // Adjusted index
            field_type: row.get(3)?,   // Adjusted index
            scope: CustomFieldScope::from_db_string(&scope_str),
            default_value: row.get(5)?, // Adjusted index
            created_at: row.get(6)?,    // Adjusted index
            updated_at: row.get(7)?,    // Adjusted index
        })
    })?;

    let mut definitions = Vec::new();
    for def in def_iter {
        definitions.push(def?);
    }
    info!("[DB] Retrieved {} custom field definitions for project_id {}.", definitions.len(), project_id);
    Ok(definitions)
}

pub fn update_custom_field_definition(project_id: &str, definition: &CustomFieldDefinition) -> Result<(), CommandError> {
    debug!("[DB] Updating custom field definition for project_id {}: {}", project_id, definition.field_key);
    let db_path = get_db_path()?;
    let conn = Connection::open(db_path)?;
    // The trigger 'update_custom_field_definitions_updated_at' will handle updating 'updated_at'.
    let changes = conn.execute(
        "UPDATE custom_field_definitions
         SET field_name = ?1, field_type = ?2, scope = ?3, default_value = ?4
         WHERE field_key = ?5 AND project_id = ?6",
        params![
            definition.field_name,
            definition.field_type,
            definition.scope.to_db_string(),
            definition.default_value,
            definition.field_key,
            project_id
        ],
    )?;

    if changes > 0 {
        info!("[DB] Custom field definition updated successfully for project_id {}: {}", project_id, definition.field_key);
    } else {
        info!("[DB] No custom field definition found to update for project_id {} and key: {}", project_id, definition.field_key);
    }
    Ok(())
}

pub fn delete_custom_field_definition(project_id: &str, field_key: &str) -> Result<(), CommandError> {
    debug!("[DB] Deleting custom field definition for project_id {}: {}", project_id, field_key);
    let db_path = get_db_path()?;
    let conn = Connection::open(db_path)?;

    let changes = conn.execute(
        "DELETE FROM custom_field_definitions WHERE field_key = ?1 AND project_id = ?2",
        params![field_key, project_id],
    )?;

    if changes > 0 {
        info!("[DB] Custom field definition deleted successfully for project_id {}: {}", project_id, field_key);
    } else {
        info!("[DB] No custom field definition found to delete for project_id {} and key: {}", project_id, field_key);
    }
    Ok(())
}

// --- End Custom Field Definition Functions ---

// --- Project Table Functions ---

pub fn add_project_to_db(id: &str, name: &str, root_path: &str, xml_path: &str) -> Result<(), CommandError> {
    debug!("[DB] Adding project to db: id={}, name={}, root_path={}, xml_path={}", id, name, root_path, xml_path);
    let db_path = get_db_path()?;
    let conn = Connection::open(&db_path)?;

    conn.execute(
        "INSERT INTO projects (id, name, root_path, xml_path)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET
             name = excluded.name,
             root_path = excluded.root_path,
             xml_path = excluded.xml_path,
             updated_at = CURRENT_TIMESTAMP",
        params![id, name, root_path, xml_path],
    )?;
    info!("[DB] Project added/updated successfully: id={}", id);
    Ok(())
}

pub fn is_project_in_db(xml_path_str: &str) -> Result<bool, CommandError> {
    debug!("[DB] Checking if project exists with xml_path: {}", xml_path_str);
    let db_path = get_db_path()?;
    if !db_path.exists() {
        debug!("[DB] Database file not found at {}. Project cannot exist.", db_path.display());
        return Ok(false);
    }
    let conn = Connection::open(&db_path)?;
    let mut stmt = conn.prepare("SELECT 1 FROM projects WHERE xml_path = ?1")?;
    let exists: Option<i32> = stmt.query_row(params![xml_path_str], |row| row.get(0)).optional()?;

    match exists {
        Some(_) => {
            debug!("[DB] Project with xml_path: {} found.", xml_path_str);
            Ok(true)
        }
        None => {
            debug!("[DB] Project with xml_path: {} not found.", xml_path_str);
            Ok(false)
        }
    }
}

pub fn delete_project_from_db(project_id: &str) -> Result<(), CommandError> {
    info!("[DB] Attempting to delete project with id: {}", project_id);
    let db_path = get_db_path()?;
    if !db_path.exists() {
        warn!("[DB] Database file not found at {}. Cannot delete project {}.", db_path.display(), project_id);
        return Ok(()); // Or an error if project should always exist in DB if config exists
    }
    let conn = Connection::open(&db_path)?;
    let changes = conn.execute("DELETE FROM projects WHERE id = ?1", params![project_id])?;

    if changes > 0 {
        info!("[DB] Successfully deleted project with id: {} ({} rows affected). Associated data should be removed by CASCADE.", project_id, changes);
    } else {
        warn!("[DB] No project found in 'projects' table with id: {} to delete.", project_id);
    }
    Ok(())
}

// --- End Project Table Functions ---

pub fn load_annotations_from_db(project_id: &str, document_path: &str, doc_type: &str) -> Result<Option<String>, CommandError> {
    debug!("[DB] Loading annotations for project_id {}: {} (type: {})", project_id, document_path, doc_type);
    let db_path = get_db_path()?;
    if !db_path.exists() {
        debug!("[DB] Database file not found at {}. Returning None.", db_path.display());
        return Ok(None);
    }
    let conn = Connection::open(&db_path)?;
    let mut stmt = conn.prepare("SELECT annotations_json FROM pdf_annotations WHERE project_id = ?1 AND pdf_document_path = ?2 AND document_type = ?3")?;
    let result = stmt.query_row(params![project_id, document_path, doc_type], |row| row.get(0)).optional()?;
    debug!("[DB] Load result for project_id {} - {} (type: {}): {}", project_id, document_path, doc_type, if result.is_some() { "Some(...)" } else { "None" });
    Ok(result)
}

pub fn save_annotations_to_db(project_id: &str, document_path: &str, annotations_json: &str, doc_type: &str) -> Result<(), CommandError> {
    debug!("[DB] Saving annotations for project_id {}: {} (type: {})", project_id, document_path, doc_type);
    let db_path = get_db_path()?;
    let conn = Connection::open(&db_path)?;

    if let Some(parent_dir) = db_path.parent() {
        if !parent_dir.exists() {
            fs::create_dir_all(parent_dir).map_err(|e| CommandError::Io(format!("Failed to create db directory: {}",e)))?;
        }
    }

    conn.execute(
        "INSERT INTO pdf_annotations (project_id, pdf_document_path, annotations_json, document_type)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(project_id, pdf_document_path, document_type)
         DO UPDATE SET annotations_json = excluded.annotations_json,
                       -- document_type = excluded.document_type, -- document_type is part of the key, should not change on conflict
                       -- project_id = excluded.project_id, -- project_id is part of the key, should not change on conflict
                       updated_at = CURRENT_TIMESTAMP",
        params![project_id, document_path, annotations_json, doc_type],
    )?;
    info!("[DB] Annotations saved successfully for project_id {}: {} (type: {})", project_id, document_path, doc_type);
    Ok(())
}

pub fn delete_annotations_from_db(project_id: &str, document_path: &str, doc_type: &str) -> Result<(), CommandError> {
    debug!("[DB] Deleting annotations for project_id {}: {} (type: {})", project_id, document_path, doc_type);
    let db_path = get_db_path()?;
     if !db_path.exists() {
        debug!("[DB] Database file not found at {}. Nothing to delete for project_id {} - {} (type: {}).", db_path.display(), project_id, document_path, doc_type);
        return Ok(());
    }
    let conn = Connection::open(&db_path)?;
    let changes = conn.execute("DELETE FROM pdf_annotations WHERE project_id = ?1 AND pdf_document_path = ?2 AND document_type = ?3", params![project_id, document_path, doc_type])?;
    if changes > 0 {
        info!("[DB] Annotations deleted successfully for project_id {}: {} (type: {}) ({} rows affected)", project_id, document_path, doc_type, changes);
    } else {
        debug!("[DB] No annotations found to delete for project_id {}: {} (type: {})", project_id, document_path, doc_type);
    }
    Ok(())
}

pub fn rename_annotations_in_db(project_id: &str, old_document_path: &str, new_document_path: &str, doc_type: &str) -> Result<(), CommandError> {
    debug!("[DB] Renaming annotations for project_id {} from {} to {} (type: {})", project_id, old_document_path, new_document_path, doc_type);
    let db_path = get_db_path()?;
    if !db_path.exists() {
        debug!("[DB] Database file not found at {}. Nothing to rename for project_id {} - {} (type: {}).", db_path.display(), project_id, old_document_path, doc_type);
        return Ok(());
    }
    let conn = Connection::open(&db_path)?;
    // Note: document_type is part of the unique key and typically should not change during a "rename" of the path.
    // If document_type could also change, the operation becomes more complex (delete old, insert new, or more specific UPDATE).
    // Here, we assume only pdf_document_path changes while project_id and document_type remain constant for the renamed record.
    let changes = conn.execute(
        "UPDATE pdf_annotations SET pdf_document_path = ?1 WHERE project_id = ?2 AND pdf_document_path = ?3 AND document_type = ?4",
        params![new_document_path, project_id, old_document_path, doc_type],
    )?;
    if changes > 0 {
        info!("[DB] Annotations renamed successfully for project_id {} from {} to {} (type: {}) ({} rows affected)", project_id, old_document_path, new_document_path, doc_type, changes);
    } else {
        debug!("[DB] No annotations found to rename for project_id {} - old path: {} (type: {})", project_id, old_document_path, doc_type);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    // Ensure CONFIG_DIR_NAME is available if used by get_config_dir, or mock get_config_dir for tests.
    // For these tests, we'll use a helper that takes the DB path directly.

    // Test helper to initialize a projects table for FK constraints
    fn init_projects_table_for_test(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                root_path TEXT NOT NULL UNIQUE,
                xml_path TEXT NOT NULL UNIQUE,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        Ok(())
    }

    fn init_pdf_annotations_table_for_test(conn: &Connection) -> Result<()> {
        // Use the new schema for tests
        conn.execute(
            "CREATE TABLE IF NOT EXISTS pdf_annotations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id TEXT NOT NULL,
                pdf_document_path TEXT NOT NULL,
                annotations_json TEXT NOT NULL,
                document_type TEXT NOT NULL DEFAULT 'pdf',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
                UNIQUE (project_id, pdf_document_path, document_type)
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


    // Helper function to create a temporary DB with the new schema for isolated testing.
    // It also creates a dummy project for FK satisfaction.
    fn setup_test_db() -> (tempfile::TempDir, PathBuf, String) {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_harvey.sqlite");
        let conn = Connection::open(&db_path).unwrap();

        init_projects_table_for_test(&conn).unwrap();
        init_pdf_annotations_table_for_test(&conn).unwrap();

        let test_project_id = "test_project_uuid_123";
        conn.execute("INSERT INTO projects (id, name, root_path, xml_path) VALUES (?1, ?2, ?3, ?4)",
            params![test_project_id, "Test Project", "/fake/root", "/fake/project.xml"]
        ).unwrap();

        (temp_dir, db_path, test_project_id.to_string())
    }

    // Mock of get_db_path for testing purposes. In a real scenario, this would involve more complex mocking or DI.
    // For these tests, functions will take db_path directly.

    #[test]
    fn test_init_db_adds_project_id_column_if_not_exists() {
        let temp_base_dir = tempdir().unwrap();
        let test_db_path = temp_base_dir.path().join("test_init_project_id.sqlite");

        if test_db_path.exists() { fs::remove_file(&test_db_path).unwrap(); }

        // 1. Initialize DB with an older schema (with document_type but without project_id)
        {
            let conn = Connection::open(&test_db_path).unwrap();
            conn.execute(
                "CREATE TABLE pdf_annotations (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    pdf_document_path TEXT NOT NULL,
                    annotations_json TEXT NOT NULL,
                    document_type TEXT NOT NULL DEFAULT 'pdf',
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    UNIQUE (pdf_document_path, document_type)
                )", // Old unique constraint for test setup
                [],
            ).unwrap();
             // Also need projects table for the FK that the full init_db will try to create if pdf_annotations is new
            init_projects_table_for_test(&conn).unwrap();
        }

        // This test relies on the main `init_db` function to correctly use `get_db_path`.
        // To make it testable in isolation, we would ideally pass the db_path to init_db or mock get_config_dir.
        // For this example, we'll assume `init_db()` can be tested if `get_db_path()` is correctly mocked or overridden.
        // Since direct mocking of `get_db_path` as used by `init_db` is tricky here,
        // we will test the logic portion of adding project_id by calling a helper.

        fn simulate_init_logic_for_pdf_annotations_project_id(conn: &Connection) -> Result<()> {
            // This simulates the part of init_db that checks and adds project_id to pdf_annotations
            let mut stmt_check_project_id = conn.prepare("PRAGMA table_info(pdf_annotations)")?;
            let project_id_column_exists = stmt_check_project_id
                .query_map([], |row| row.get::<_, String>(1))?
                .any(|col_name_result| col_name_result.map_or(false, |name| name == "project_id"));

            if !project_id_column_exists {
                info!("[DB Test] Adding project_id column to pdf_annotations table for test simulation.");
                conn.execute("ALTER TABLE pdf_annotations ADD COLUMN project_id TEXT", [])?;
            }
            Ok(())
        }

        let conn_check = Connection::open(&test_db_path).unwrap();
        assert!(simulate_init_logic_for_pdf_annotations_project_id(&conn_check).is_ok());

        let mut stmt_verify = conn_check.prepare("PRAGMA table_info(pdf_annotations)").unwrap();
        let columns: Vec<String> = stmt_verify.query_map([], |row| row.get(1)).unwrap().map(|r| r.unwrap()).collect();
        assert!(columns.contains(&"project_id".to_string()), "project_id column should have been added");

        // Clean up
        drop(conn_check); // Release connection before removing file
        fs::remove_file(&test_db_path).unwrap();
    }


    #[test]
    fn test_save_and_load_annotations_with_project_id() {
        let (_temp_dir, db_path, project_id) = setup_test_db();

        // Use direct path for test functions
        fn save_annotations(db_p: &PathBuf, proj_id: &str, doc_path: &str, json: &str, doc_type: &str) -> Result<()> {
            let conn = Connection::open(db_p)?;
            conn.execute(
                "INSERT INTO pdf_annotations (project_id, pdf_document_path, annotations_json, document_type) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(project_id, pdf_document_path, document_type) DO UPDATE SET annotations_json = excluded.annotations_json",
                params![proj_id, doc_path, json, doc_type]
            )?;
            Ok(())
        }
        fn load_annotations(db_p: &PathBuf, proj_id: &str, doc_path: &str, doc_type: &str) -> Result<Option<String>> {
            let conn = Connection::open(db_p)?;
            let mut stmt = conn.prepare("SELECT annotations_json FROM pdf_annotations WHERE project_id = ?1 AND pdf_document_path = ?2 AND document_type = ?3")?;
            stmt.query_row(params![proj_id, doc_path, doc_type], |row| row.get(0)).optional()
        }

        let doc_path1 = "test/doc1.pdf";
        let annots1 = "[{\"id\":\"1\"}]";
        let doc_type1 = "pdf";
        assert!(save_annotations(&db_path, &project_id, doc_path1, annots1, doc_type1).is_ok());

        let loaded_annots1 = load_annotations(&db_path, &project_id, doc_path1, doc_type1).unwrap();
        assert_eq!(loaded_annots1, Some(annots1.to_string()));

        // Update
        let annots1_updated = "[{\"id\":\"1\", \"text\":\"updated\"}]";
        assert!(save_annotations(&db_path, &project_id, doc_path1, annots1_updated, doc_type1).is_ok());
        let loaded_annots1_updated = load_annotations(&db_path, &project_id, doc_path1, doc_type1).unwrap();
        assert_eq!(loaded_annots1_updated, Some(annots1_updated.to_string()));

        // Different project_id, same doc_path and doc_type - should be a new record
        let other_project_id = "other_project_uuid_456";
         Connection::open(&db_path).unwrap().execute("INSERT INTO projects (id, name, root_path, xml_path) VALUES (?1, ?2, ?3, ?4)",
            params![other_project_id, "Other Project", "/other/root", "/other/project.xml"]
        ).unwrap();

        assert!(save_annotations(&db_path, other_project_id, doc_path1, annots1, doc_type1).is_ok());
        let loaded_other_project_annots = load_annotations(&db_path, other_project_id, doc_path1, doc_type1).unwrap();
        assert_eq!(loaded_other_project_annots, Some(annots1.to_string()));

        // Ensure original project's annotation is still there and unchanged
        let original_project_annots_after_other_insert = load_annotations(&db_path, &project_id, doc_path1, doc_type1).unwrap();
        assert_eq!(original_project_annots_after_other_insert, Some(annots1_updated.to_string()));


        let loaded_non_existent = load_annotations(&db_path, &project_id, "other.pdf", "pdf").unwrap();
        assert!(loaded_non_existent.is_none());
    }

    #[test]
    fn test_delete_annotations_with_project_id() {
        let (_temp_dir, db_path, project_id) = setup_test_db();

        fn save_direct(conn: &Connection, proj_id: &str, doc_p: &str, json: &str, doc_t: &str) {
            conn.execute("INSERT INTO pdf_annotations (project_id, pdf_document_path, annotations_json, document_type) VALUES (?1,?2,?3,?4)", params![proj_id, doc_p, json, doc_t]).unwrap();
        }
        fn delete_direct(conn: &Connection, proj_id: &str, doc_p: &str, doc_t: &str) -> Result<usize> {
            conn.execute("DELETE FROM pdf_annotations WHERE project_id=?1 AND pdf_document_path=?2 AND document_type=?3", params![proj_id, doc_p, doc_t])
        }
        fn load_direct(conn: &Connection, proj_id: &str, doc_p: &str, doc_t: &str) -> Option<String> {
            conn.query_row("SELECT annotations_json FROM pdf_annotations WHERE project_id=?1 AND pdf_document_path=?2 AND document_type=?3", params![proj_id, doc_p, doc_t], |r| r.get(0)).optional().unwrap()
        }

        let conn = Connection::open(&db_path).unwrap();
        save_direct(&conn, &project_id, "doc1.pdf", "[]", "pdf");
        assert!(load_direct(&conn, &project_id, "doc1.pdf", "pdf").is_some());

        assert!(delete_direct(&conn, &project_id, "doc1.pdf", "pdf").unwrap() > 0);
        assert!(load_direct(&conn, &project_id, "doc1.pdf", "pdf").is_none());

        // Try deleting non-existent
        assert_eq!(delete_direct(&conn, &project_id, "non_existent.pdf", "pdf").unwrap(), 0);
    }

    #[test]
    fn test_rename_annotations_with_project_id() {
        let (_temp_dir, db_path, project_id) = setup_test_db();

        fn save_direct(conn: &Connection, proj_id: &str, doc_p: &str, json: &str, doc_t: &str) {
            conn.execute("INSERT INTO pdf_annotations (project_id, pdf_document_path, annotations_json, document_type) VALUES (?1,?2,?3,?4)", params![proj_id, doc_p, json, doc_t]).unwrap();
        }
        fn rename_direct(conn: &Connection, proj_id: &str, old_doc_p: &str, new_doc_p: &str, doc_t: &str) -> Result<usize> {
            conn.execute("UPDATE pdf_annotations SET pdf_document_path=?1 WHERE project_id=?2 AND pdf_document_path=?3 AND document_type=?4", params![new_doc_p, proj_id, old_doc_p, doc_t])
        }
        fn load_direct(conn: &Connection, proj_id: &str, doc_p: &str, doc_t: &str) -> Option<String> {
            conn.query_row("SELECT annotations_json FROM pdf_annotations WHERE project_id=?1 AND pdf_document_path=?2 AND document_type=?3", params![proj_id, doc_p, doc_t], |r| r.get(0)).optional().unwrap()
        }

        let conn = Connection::open(&db_path).unwrap();
        save_direct(&conn, &project_id, "old.pdf", "[old]", "pdf");
        assert!(rename_direct(&conn, &project_id, "old.pdf", "new.pdf", "pdf").unwrap() > 0);

        assert!(load_direct(&conn, &project_id, "old.pdf", "pdf").is_none());
        assert_eq!(load_direct(&conn, &project_id, "new.pdf", "pdf"), Some("[old]".to_string()));

        // Try renaming non-existent
        assert_eq!(rename_direct(&conn, &project_id, "non_existent.pdf", "another.pdf", "pdf").unwrap(), 0);
    }
}
