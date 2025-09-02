// src-tauri/src/projectview/db_handler.rs
use rusqlite::{Connection, Result, params, OptionalExtension, ToSql};
use std::path::PathBuf;
use std::fs;
use crate::welcome::config::{get_config_dir, CommandError}; // Assuming this function gives PathBuf
use log::{info, debug, error, warn};
use serde::{Serialize, Deserialize}; // Added for the new struct
use crate::projectview::shared_types::{FileMetadata, FileGroupAssociationFromDb, Highlight}; // For function signatures

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
    pub waveform_data: Option<Vec<u8>>,
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
    pub language_code: Option<String>,
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
            waveform_data BLOB,
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

    // Migration for waveform_data
    let mut stmt_check_waveform_data = conn.prepare("PRAGMA table_info(asset_metadata)")?;
    let waveform_data_exists = stmt_check_waveform_data
        .query_map([], |row| row.get::<_, String>(1))?
        .any(|name_res| name_res.map_or(false, |name| name == "waveform_data"));

    if !waveform_data_exists {
        info!("[DB] Adding waveform_data column to asset_metadata table.");
        conn.execute("ALTER TABLE asset_metadata ADD COLUMN waveform_data BLOB", [])?;
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
                REFERENCES asset_metadata(project_id, asset_relative_path) ON DELETE CASCADE ON UPDATE CASCADE
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

    // table_styles table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS table_styles (
            project_id TEXT NOT NULL,
            table_path TEXT NOT NULL,
            styles TEXT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (project_id, table_path)
        )",
        [],
    )?;
    info!("[DB] Initialized table_styles table.");

    // Trigger for table_styles updated_at
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_table_styles_updated_at
        AFTER UPDATE ON table_styles
        FOR EACH ROW
        BEGIN
            UPDATE table_styles SET updated_at = CURRENT_TIMESTAMP WHERE project_id = OLD.project_id AND table_path = OLD.table_path;
        END;",
        [],
    )?;
    info!("[DB] Initialized update_table_styles_updated_at trigger.");

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
            FOREIGN KEY (project_id, file_asset_path) REFERENCES asset_metadata(project_id, asset_relative_path) ON DELETE CASCADE ON UPDATE CASCADE,
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
            language_code TEXT, -- New column for language code
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (project_id, asset_relative_path),
            FOREIGN KEY (project_id, asset_relative_path)
                REFERENCES asset_metadata(project_id, asset_relative_path) ON DELETE CASCADE ON UPDATE CASCADE
        )",
        [],
    )?;
    info!("[DB] Initialized media_transcript_data table.");

    // Migration for language_code
    let mut stmt_check_lang_code = conn.prepare("PRAGMA table_info(media_transcript_data)")?;
    let lang_code_exists = stmt_check_lang_code
        .query_map([], |row| row.get::<_, String>(1))?
        .any(|name_res| name_res.map_or(false, |name| name == "language_code"));

    if !lang_code_exists {
        info!("[DB] Adding language_code column to media_transcript_data table.");
        conn.execute("ALTER TABLE media_transcript_data ADD COLUMN language_code TEXT", [])?;
    }

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

    // tags table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id TEXT NOT NULL,
            name TEXT NOT NULL,
            color TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
            UNIQUE (project_id, name)
        )",
        [],
    )?;
    info!("[DB] Initialized tags table.");

    // Trigger for tags updated_at
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_tags_updated_at
        AFTER UPDATE ON tags
        FOR EACH ROW
        BEGIN
            UPDATE tags SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
        END;",
        [],
    )?;
    info!("[DB] Initialized update_tags_updated_at trigger.");

    // Migration for adding color column to tags table
    let mut stmt = conn.prepare("PRAGMA table_info(tags)")?;
    let column_exists = stmt.query_map([], |row| {
        let column_name: String = row.get(1)?;
        Ok(column_name)
    })?.any(|col| col.as_deref() == Ok("color"));

    if !column_exists {
        info!("[DB] Adding color column to tags table.");
        conn.execute("ALTER TABLE tags ADD COLUMN color TEXT", [])?;
    }

    // highlights table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS highlights (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            asset_id TEXT NOT NULL,
            project_id TEXT NOT NULL,
            start_offset INTEGER,
            end_offset INTEGER,
            text TEXT,
            annotation_id TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (project_id, asset_id) REFERENCES asset_metadata(project_id, asset_relative_path) ON DELETE CASCADE
        )",
        [],
    )?;
    info!("[DB] Initialized highlights table.");

    // Migration for adding columns to highlights table
    let mut stmt = conn.prepare("PRAGMA table_info(highlights)")?;
    let columns: Vec<String> = stmt.query_map([], |row| row.get(1))?.collect::<Result<Vec<_>, _>>()?;

    if !columns.contains(&"asset_id".to_string()) {
        info!("[DB] Adding asset_id column to highlights table.");
        conn.execute("ALTER TABLE highlights ADD COLUMN asset_id TEXT", [])?;
    }
    if !columns.contains(&"project_id".to_string()) {
        info!("[DB] Adding project_id column to highlights table.");
        conn.execute("ALTER TABLE highlights ADD COLUMN project_id TEXT", [])?;
    }
    if !columns.contains(&"annotation_id".to_string()) {
        info!("[DB] Adding annotation_id column to highlights table.");
        conn.execute("ALTER TABLE highlights ADD COLUMN annotation_id TEXT", [])?;
    }

    // highlight_tags table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS highlight_tags (
            highlight_id INTEGER NOT NULL,
            tag_id INTEGER NOT NULL,
            project_id TEXT NOT NULL,
            PRIMARY KEY (highlight_id, tag_id),
            FOREIGN KEY (highlight_id) REFERENCES highlights(id) ON DELETE CASCADE,
            FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE,
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        )",
        [],
    )?;
    info!("[DB] Initialized highlight_tags table.");


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

pub fn get_groups_for_file_asset(conn: &Connection, project_id: &str, file_asset_path: &str) -> Result<Vec<GroupDataFromDb>, rusqlite::Error> {
    debug!("[DB] Loading groups for file_asset_path {} in project_id {}", file_asset_path, project_id);
    let mut stmt = conn.prepare(
        "SELECT g.id, g.project_id, g.name, g.description, g.created_at, g.updated_at
         FROM groups g
         JOIN file_groups fg ON g.id = fg.group_id
         WHERE fg.project_id = ?1 AND fg.file_asset_path = ?2
         ORDER BY g.name ASC"
    )?;

    let group_iter = stmt.query_map(params![project_id, file_asset_path], |row| {
        Ok(GroupDataFromDb {
            id: row.get(0)?,
            project_id: row.get(1)?,
            name: row.get(2)?,
            description: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    })?;

    let mut groups = Vec::new();
    for group_result in group_iter {
        groups.push(group_result?);
    }
    info!("[DB] Loaded {} groups for file_asset_path {} in project_id {}", groups.len(), file_asset_path, project_id);
    Ok(groups)
}

pub fn remove_file_from_group(conn: &Connection, project_id: &str, group_id: &str, file_asset_path: &str) -> Result<usize, rusqlite::Error> {
    debug!("[DB] Removing file {} from group {} for project_id {}", file_asset_path, group_id, project_id);
    let rows_affected = conn.execute(
        "DELETE FROM file_groups
         WHERE project_id = ?1 AND group_id = ?2 AND file_asset_path = ?3",
        params![project_id, group_id, file_asset_path],
    )?;
    if rows_affected > 0 {
        info!("[DB] File {} removed from group {} successfully.", file_asset_path, group_id);
    } else {
        info!("[DB] No association found for file {} in group {} (project_id {}). Nothing removed.", file_asset_path, group_id, project_id);
    }
    Ok(rows_affected)
}

pub fn get_files_for_group(conn: &Connection, project_id: &str, group_id: &str) -> Result<Vec<FileGroupAssociationFromDb>, rusqlite::Error> {
    debug!("[DB] Loading files for group_id {} in project_id {}", group_id, project_id);
    let mut stmt = conn.prepare(
        "SELECT fg.file_asset_path FROM file_groups fg
         WHERE fg.project_id = ?1 AND fg.group_id = ?2
         ORDER BY fg.file_asset_path ASC" // Added ORDER BY for consistency
    )?;

    let rows = stmt.query_map(params![project_id, group_id], |row| {
        Ok(FileGroupAssociationFromDb {
            file_asset_path: row.get(0)?,
        })
    })?;

    let mut files = Vec::new();
    for file_result in rows {
        files.push(file_result?);
    }
    info!("[DB] Loaded {} files for group_id {} in project_id {}", files.len(), group_id, project_id);
    Ok(files)
}

pub fn update_group_details(
    conn: &Connection,
    project_id: &str,
    group_id: &str,
    new_name: &str,
    new_description: Option<&str>
) -> Result<usize, rusqlite::Error> {
    // chrono::Utc should be in scope from the top of shared_types.rs or directly here if needed.
    // For db_handler.rs, we might need to add `use chrono::Utc;` if it's not already implicitly available.
    // Assuming Utc is available for now as per its usage in FileMetadata default.
    // If not, the compiler will tell us, and we can add `use chrono::Utc;`
    let current_timestamp = chrono::Utc::now().to_rfc3339();
    debug!("[DB] Updating group details for group_id {} in project_id {}: name={}, desc_is_some={}", group_id, project_id, new_name, new_description.is_some());
    conn.execute(
        "UPDATE groups SET name = ?1, description = ?2, updated_at = ?3 WHERE project_id = ?4 AND id = ?5",
        params![new_name, new_description, current_timestamp, project_id, group_id],
    )
}

pub fn rename_group_in_db(
    conn: &Connection,
    project_id: &str,
    group_id: &str,
    new_name: &str,
    new_description: Option<&str>
) -> Result<usize, rusqlite::Error> {
    let current_timestamp = chrono::Utc::now().to_rfc3339();
    debug!(
        "[DB] Renaming group for group_id {} in project_id {}: new_name={}, new_desc_is_some={}",
        group_id, project_id, new_name, new_description.is_some()
    );
    conn.execute(
        "UPDATE groups SET name = ?1, description = ?2, updated_at = ?3 WHERE project_id = ?4 AND id = ?5",
        params![new_name, to_sql_optional_str(new_description), current_timestamp, project_id, group_id],
    )
}

pub fn delete_group_from_db(
    conn: &Connection,
    project_id: &str,
    group_id: &str
) -> Result<usize, rusqlite::Error> {
    debug!("[DB] Deleting group for group_id {} in project_id {}", group_id, project_id);

    // First, delete associations from file_groups.
    // It's important to do this first if there are foreign key constraints,
    // though in this schema, file_groups references groups, so deleting from groups
    // might cascade if ON DELETE CASCADE is set (which it is for group_id FK).
    // Explicitly deleting from file_groups first is safer and clearer.
    let file_associations_deleted = conn.execute(
        "DELETE FROM file_groups WHERE project_id = ?1 AND group_id = ?2",
        params![project_id, group_id],
    )?;
    info!(
        "[DB] Deleted {} file associations for group_id {} in project_id {}.",
        file_associations_deleted, group_id, project_id
    );

    // Then, delete the group itself.
    let group_rows_deleted = conn.execute(
        "DELETE FROM groups WHERE project_id = ?1 AND id = ?2",
        params![project_id, group_id],
    )?;

    if group_rows_deleted > 0 {
        info!("[DB] Group group_id {} in project_id {} deleted successfully.", group_id, project_id);
    } else {
        warn!("[DB] No group found with group_id {} in project_id {} to delete.", group_id, project_id);
    }
    // Return the number of rows deleted from the 'groups' table.
    // The command expects Result<(), String> so the exact number isn't directly passed up,
    // but it's good practice for a DB function to return affected rows.
    Ok(group_rows_deleted)
}

// --- End Group Functions ---

// --- Media Transcript Data Functions ---

pub fn save_media_transcript_data(
    project_id: &str,
    asset_relative_path: &str,
    original_import_path: Option<&str>,
    speaker_names: Option<&Vec<String>>,
    language_code: Option<&str>,
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
            project_id, asset_relative_path, original_import_path, speaker_names_json, language_code
        ) VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(project_id, asset_relative_path) DO UPDATE SET
            original_import_path = excluded.original_import_path,
            speaker_names_json = excluded.speaker_names_json,
            language_code = excluded.language_code,
            updated_at = CURRENT_TIMESTAMP;
    ";

    conn.execute(
        sql,
        params![
            project_id,
            asset_relative_path,
            to_sql_optional_str(original_import_path),
            to_sql_optional_str(speaker_names_json_str.as_deref()),
            to_sql_optional_str(language_code),
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
        SELECT original_import_path, speaker_names_json, language_code
        FROM media_transcript_data
        WHERE project_id = ?1 AND asset_relative_path = ?2
    ")?;

    let result = stmt.query_row(params![project_id, asset_relative_path], |row| {
        Ok(MediaTranscriptDataValues {
            original_import_path: row.get(0)?,
            speaker_names_json: row.get(1)?,
            language_code: row.get(2)?,
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

pub fn save_table_styles(project_id: &str, table_path: &str, styles: &str) -> Result<(), CommandError> {
    debug!("[DB] Saving table styles for project_id {}: {}", project_id, table_path);
    let db_path = get_db_path()?;
    let conn = Connection::open(&db_path)?;

    conn.execute(
        "INSERT INTO table_styles (project_id, table_path, styles)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(project_id, table_path) DO UPDATE SET
             styles = excluded.styles,
             updated_at = CURRENT_TIMESTAMP",
        params![project_id, table_path, styles],
    )?;
    info!("[DB] Table styles saved successfully for project_id {}: {}", project_id, table_path);
    Ok(())
}

pub fn load_table_styles(project_id: &str, table_path: &str) -> Result<Option<String>, CommandError> {
    debug!("[DB] Loading table styles for project_id {}: {}", project_id, table_path);
    let db_path = get_db_path()?;
    if !db_path.exists() {
        debug!("[DB] Database file not found at {}. Returning None for table styles: project_id {}, path {}", db_path.display(), project_id, table_path);
        return Ok(None);
    }
    let conn = Connection::open(&db_path)?;
    let mut stmt = conn.prepare("
        SELECT styles
        FROM table_styles
        WHERE project_id = ?1 AND table_path = ?2
    ")?;

    let result = stmt.query_row(params![project_id, table_path], |row| {
        row.get(0)
    }).optional()?;

    debug!("[DB] Load table styles result for project_id {} - {}: {}", project_id, table_path, if result.is_some() { "Some(...)" } else { "None" });
    Ok(result)
}

pub fn delete_table_styles(project_id: &str, table_path: &str) -> Result<(), CommandError> {
    debug!("[DB] Deleting table styles for project_id {}: {}", project_id, table_path);
    let db_path = get_db_path()?;
    if !db_path.exists() {
        debug!("[DB] Database file not found at {}. Nothing to delete for project_id {}, path {}", db_path.display(), project_id, table_path);
        return Ok(());
    }
    let conn = Connection::open(&db_path)?;
    let changes = conn.execute("DELETE FROM table_styles WHERE project_id = ?1 AND table_path = ?2", params![project_id, table_path])?;

    if changes > 0 {
        info!("[DB] Table styles deleted successfully for project_id {}: {} ({} rows affected)", project_id, table_path, changes);
    } else {
        debug!("[DB] No table styles found to delete for project_id {}: {}", project_id, table_path);
    }
    Ok(())
}

// Helper to convert Option<T> to dyn ToSql for rusqlite
pub fn to_sql_optional<T: ToSql + 'static>(opt: Option<T>) -> Box<dyn ToSql> {
    match opt {
        Some(val) => Box::new(val),
        None => Box::new(rusqlite::types::Null),
    }
}

// Helper to convert Option<&[u8]> to dyn ToSql for rusqlite
pub fn to_sql_optional_blob(opt: Option<&[u8]>) -> Box<dyn ToSql + '_> {
    match opt {
        Some(val) => Box::new(val),
        None => Box::new(rusqlite::types::Null),
    }
}
// Helper to convert Option<&str> to dyn ToSql
pub fn to_sql_optional_str(opt_str: Option<&str>) -> Box<dyn ToSql> {
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
            original_import_path, speaker_names_json, waveform_data
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)
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
            waveform_data = excluded.waveform_data,
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
            to_sql_optional_blob(metadata.waveform_data.as_deref()),
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
               creation_time, custom_fields_json, asset_type, original_import_path, speaker_names_json, waveform_data
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
            waveform_data: row.get(18)?,
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
        "[DB] Attempting to rename asset metadata key for project_id {}: from '{}' to '{}', new_abs_path: '{}', new_name: '{}'",
        project_id, old_relative_path, new_relative_path, new_file_path, new_file_name
    );

    let db_path = get_db_path()?;
    if !db_path.exists() {
        debug!("[DB] Database file not found at {}. Nothing to rename for project_id {}, asset: {}", db_path.display(), project_id, old_relative_path);
        return Ok(());
    }

    let mut conn = Connection::open(&db_path).map_err(CommandError::from)?;

    // Wrap operations in a transaction
    let tx = conn.transaction().map_err(CommandError::from)?;

    // 1. Disable foreign key constraints
    debug!("[DB TX] Disabling foreign keys for rename operation on project_id {}: from {} to {}", project_id, old_relative_path, new_relative_path);
    tx.execute("PRAGMA foreign_keys = OFF;", params![]).map_err(|e| {
        error!("[DB TX] Failed to disable foreign keys: {}. Operation aborted.", e);
        // No need to manually re-enable FKs here as the transaction will rollback.
        CommandError::from(e)
    })?;

    // 2. Update Parent Table First: asset_metadata
    debug!(
        "[DB TX] Updating parent asset_metadata for project_id {}: from '{}' to '{}'. New full path: '{}', new file name: '{}'",
        project_id, old_relative_path, new_relative_path, new_file_path, new_file_name
    );
    let parent_changes = tx.execute(
        "UPDATE asset_metadata
         SET asset_relative_path = ?1, file_path = ?2, file_name = ?3, last_modified = CURRENT_TIMESTAMP
         WHERE project_id = ?4 AND asset_relative_path = ?5",
        params![new_relative_path, new_file_path, new_file_name, project_id, old_relative_path],
    ).map_err(|e| {
        error!("[DB TX] Error updating asset_metadata for project_id {} from {} to {}: {}. Attempting to re-enable FKs and rolling back.", project_id, old_relative_path, new_relative_path, e);
        if let Err(fk_err) = tx.execute("PRAGMA foreign_keys = ON;", params![]) {
            error!("[DB TX] Failed to re-enable foreign keys during error handling for asset_metadata update: {}", fk_err);
        }
        CommandError::from(e)
    })?;

    if parent_changes > 0 {
        info!(
            "[DB TX] asset_metadata updated successfully for project_id {} from {} to {} ({} rows affected).",
            project_id, old_relative_path, new_relative_path, parent_changes
        );
    } else {
        warn!(
            "[DB TX] No rows updated in asset_metadata for project_id {} and old_relative_path: '{}'. Child table updates will still proceed.",
            project_id, old_relative_path
        );
    }

    // 3. Update Child Tables Second (to match the new parent key)

    // Update file_groups
    match tx.execute(
        "UPDATE file_groups SET file_asset_path = ?1 WHERE project_id = ?2 AND file_asset_path = ?3",
        params![new_relative_path, project_id, old_relative_path],
    ) {
        Ok(changes) if changes > 0 => {
            info!("[DB TX] Updated file_groups for project_id {} from {} to {} ({} rows affected)", project_id, old_relative_path, new_relative_path, changes);
        }
        Ok(_) => { // 0 rows affected
            debug!("[DB TX] No entries in file_groups needed update for project_id {} and old path {}", project_id, old_relative_path);
        }
        Err(e) => {
            error!("[DB TX] Error updating file_groups for project_id {} from {} to {}: {}. Attempting to re-enable FKs and rolling back.", project_id, old_relative_path, new_relative_path, e);
            if let Err(fk_err) = tx.execute("PRAGMA foreign_keys = ON;", params![]) {
                 error!("[DB TX] Failed to re-enable foreign keys during error handling: {}", fk_err);
            }
            return Err(CommandError::from(e));
        }
    }

    // Update table_layout_preferences
    match tx.execute(
        "UPDATE table_layout_preferences SET table_asset_relative_path = ?1 WHERE project_id = ?2 AND table_asset_relative_path = ?3",
        params![new_relative_path, project_id, old_relative_path],
    ) {
        Ok(changes) if changes > 0 => {
            info!("[DB TX] Updated table_layout_preferences for project_id {} from {} to {} ({} rows affected)", project_id, old_relative_path, new_relative_path, changes);
        }
        Ok(_) => {
             debug!("[DB TX] No entries in table_layout_preferences needed update for project_id {} and old path {}", project_id, old_relative_path);
        }
        Err(e) => {
            error!("[DB TX] Error updating table_layout_preferences for project_id {} from {} to {}: {}. Attempting to re-enable FKs and rolling back.", project_id, old_relative_path, new_relative_path, e);
            if let Err(fk_err) = tx.execute("PRAGMA foreign_keys = ON;", params![]) {
                 error!("[DB TX] Failed to re-enable foreign keys during error handling: {}", fk_err);
            }
            return Err(CommandError::from(e));
        }
    }

    // Update media_transcript_data
    match tx.execute(
        "UPDATE media_transcript_data SET asset_relative_path = ?1 WHERE project_id = ?2 AND asset_relative_path = ?3",
        params![new_relative_path, project_id, old_relative_path],
    ) {
        Ok(changes) if changes > 0 => {
            info!("[DB TX] Updated media_transcript_data for project_id {} from {} to {} ({} rows affected)", project_id, old_relative_path, new_relative_path, changes);
        }
        Ok(_) => {
            debug!("[DB TX] No entries in media_transcript_data needed update for project_id {} and old path {}", project_id, old_relative_path);
        }
        Err(e) => {
            error!("[DB TX] Error updating media_transcript_data for project_id {} from {} to {}: {}. Attempting to re-enable FKs and rolling back.", project_id, old_relative_path, new_relative_path, e);
            if let Err(fk_err) = tx.execute("PRAGMA foreign_keys = ON;", params![]) {
                 error!("[DB TX] Failed to re-enable foreign keys during error handling: {}", fk_err);
            }
            return Err(CommandError::from(e));
        }
    }

    // 4. Re-enable foreign key constraints before committing
    debug!("[DB TX] Re-enabling foreign keys for project_id {}: from {} to {}", project_id, old_relative_path, new_relative_path);
    tx.execute("PRAGMA foreign_keys = ON;", params![]).map_err(|e| {
        error!("[DB TX] CRITICAL: Failed to re-enable foreign keys for project_id {} after updates. Transaction will be rolled back. Error: {}", project_id, e);
        CommandError::from(e)
    })?;

    tx.commit().map_err(CommandError::from)?;
    info!(
        "[DB] Transaction committed successfully for renaming asset metadata and related child table paths for project_id {}: from {} to {}",
        project_id, old_relative_path, new_relative_path
    );

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

// --- Lexical Highlights Functions ---

pub fn save_lexical_highlights_to_db(project_id: &str, document_path: &str, highlights_json: &str) -> Result<(), CommandError> {
    save_annotations_to_db(project_id, document_path, highlights_json, "lexical")
}

pub fn load_lexical_highlights_from_db(project_id: &str, document_path: &str) -> Result<Option<String>, CommandError> {
    load_annotations_from_db(project_id, document_path, "lexical")
}

pub fn delete_lexical_highlights_from_db(project_id: &str, document_path: &str) -> Result<(), CommandError> {
    delete_annotations_from_db(project_id, document_path, "lexical")
}

// --- End Lexical Highlights Functions ---

// --- Tag Functions ---

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Tag {
    pub id: i64,
    pub project_id: String,
    pub name: String,
    pub color: Option<String>,
}

pub fn add_tag(conn: &Connection, project_id: &str, name: &str, color: Option<&str>) -> Result<i64, CommandError> {
    debug!("[DB] Adding tag to project_id {}: name={}, color={:?}", project_id, name, color);
    let mut stmt = conn.prepare("INSERT INTO tags (project_id, name, color) VALUES (?1, ?2, ?3)")?;
    let id = stmt.insert(params![project_id, name, color])?;
    info!("[DB] Tag added successfully with id {}: name={}", id, name);
    Ok(id)
}

pub fn delete_tag(conn: &Connection, project_id: &str, tag_id: i64) -> Result<(), CommandError> {
    debug!("[DB] Deleting tag with id {} from project_id {}", tag_id, project_id);
    conn.execute("DELETE FROM tags WHERE id = ?1 AND project_id = ?2", params![tag_id, project_id])?;
    info!("[DB] Tag with id {} deleted successfully.", tag_id);
    Ok(())
}

pub fn update_tag(conn: &Connection, project_id: &str, tag_id: i64, name: &str, color: Option<&str>) -> Result<(), CommandError> {
    debug!("[DB] Updating tag with id {} in project_id {}: name={}, color={:?}", tag_id, project_id, name, color);
    conn.execute(
        "UPDATE tags SET name = ?1, color = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3 AND project_id = ?4",
        params![name, color, tag_id, project_id],
    )?;
    info!("[DB] Tag with id {} updated successfully.", tag_id);
    Ok(())
}

pub fn get_all_tags(conn: &Connection, project_id: &str) -> Result<Vec<Tag>, CommandError> {
    debug!("[DB] Loading all tags for project_id {}", project_id);
    let mut stmt = conn.prepare("SELECT id, project_id, name, color FROM tags WHERE project_id = ?1 ORDER BY name ASC")?;
    let tag_iter = stmt.query_map(params![project_id], |row| {
        Ok(Tag {
            id: row.get(0)?,
            project_id: row.get(1)?,
            name: row.get(2)?,
            color: row.get(3)?,
        })
    })?;

    let mut tags = Vec::new();
    for tag_result in tag_iter {
        tags.push(tag_result?);
    }
    info!("[DB] Loaded {} tags for project_id {}", tags.len(), project_id);
    Ok(tags)
}

// --- End Tag Functions ---

// --- Highlight Functions ---

// NOTE: The following functions (add_highlight, add_tag_to_highlight, etc.) are part of a partial
// refactor to move tags to a database. However, the application currently stores highlight and
// tag information within JSON blobs in other tables (e.g., pdf_annotations). These functions
// are unused for now but are kept for future completion of the refactor.

// --- End Highlight Functions ---

pub fn get_highlights_by_tag(
    conn: &Connection,
    project_id: &str,
    tag_name: &str,
) -> Result<Vec<(Highlight, String, Vec<String>, Option<String>)>, CommandError> {
    debug!("[DB] Aggregating highlights for project_id {} with tag_name '{}'", project_id, tag_name);
    let mut all_highlights = Vec::new();

    // 1. Get highlights from pdf_annotations (covers PDFs, images, lexical docs)
    let mut stmt_pdf = conn.prepare("
        SELECT pa.pdf_document_path, pa.annotations_json, am.asset_type
        FROM pdf_annotations pa
        LEFT JOIN asset_metadata am ON pa.pdf_document_path = am.asset_relative_path AND pa.project_id = am.project_id
        WHERE pa.project_id = ?1
    ")?;
    let pdf_annotation_rows = stmt_pdf.query_map(params![project_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;

    for row in pdf_annotation_rows {
        let (doc_path, annotations_json, asset_type): (String, String, Option<String>) = row?;
        let annotations_val: serde_json::Value = match serde_json::from_str(&annotations_json) {
            Ok(val) => val,
            Err(_) => continue,
        };

        let annotations = annotations_val.as_array().map_or(Vec::new(), |arr| arr.clone());

        for annotation_val in annotations {
            if let Some(annotation_obj) = annotation_val.as_object() {
                let tags_vec: Vec<String> = annotation_obj.get("tags")
                    .and_then(|t| t.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();

                if tags_vec.contains(&tag_name.to_string()) {
                    let id = annotation_obj.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                    let text = annotation_obj.get("text").and_then(|v| v.as_str()).unwrap_or("[Image Highlight]").to_string();
                    let color = annotation_obj.get("color").and_then(|v| v.as_str())
                        .or_else(|| {
                            annotation_obj.get("body")
                                .and_then(|b| b.as_array())
                                .and_then(|bodies| bodies.get(0))
                                .and_then(|first_body| first_body.get("value"))
                                .and_then(|v| v.as_str())
                        })
                        .unwrap_or_default().to_string();

                    let highlight = Highlight {
                        id,
                        text,
                        color,
                        tags: Some(tags_vec.clone()),
                        comments: None,
                        timestamp: None,
                    };
                    all_highlights.push((highlight, doc_path.clone(), tags_vec, asset_type.clone()));
                }
            }
        }
    }

    // 2. Get highlights from table_styles (covers tables)
    let mut stmt_table = conn.prepare("
        SELECT ts.table_path, ts.styles, am.asset_type
        FROM table_styles ts
        LEFT JOIN asset_metadata am ON ts.table_path = am.asset_relative_path AND ts.project_id = am.project_id
        WHERE ts.project_id = ?1
    ")?;
    let table_style_rows = stmt_table.query_map(params![project_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;

    for row in table_style_rows {
        let (table_path, styles_json, asset_type): (String, String, Option<String>) = row?;
        let table_highlights: Vec<Highlight> = serde_json::from_str(&styles_json)
            .or_else(|_| serde_json::from_str(&styles_json).and_then(|s: String| serde_json::from_str(&s)))
            .unwrap_or_else(|_| Vec::new());

        for highlight in table_highlights {
            if let Some(tags) = &highlight.tags {
                if tags.contains(&tag_name.to_string()) {
                    all_highlights.push((highlight.clone(), table_path.clone(), tags.clone(), asset_type.clone()));
                }
            }
        }
    }

    info!("[DB] Found a total of {} highlights for project_id {} with tag_name '{}'", all_highlights.len(), project_id, tag_name);
    Ok(all_highlights)
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

        {
            let mut stmt_verify = conn_check.prepare("PRAGMA table_info(pdf_annotations)").unwrap();
            let columns: Vec<String> = stmt_verify.query_map([], |row| row.get(1)).unwrap().map(|r| r.unwrap()).collect();
            assert!(columns.contains(&"project_id".to_string()), "project_id column should have been added");
        }

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

#[cfg(test)]
pub fn init_db_for_test(conn: &Connection) -> Result<(), CommandError> {
    debug!("[DB] Initializing database for test");

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
            waveform_data BLOB,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (project_id, asset_relative_path)
        )",
        [],
    )?;

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

    // table_layout_preferences table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS table_layout_preferences (
            project_id TEXT NOT NULL,
            table_asset_relative_path TEXT NOT NULL,
            layout_json TEXT NOT NULL,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
            PRIMARY KEY (project_id, table_asset_relative_path),
            FOREIGN KEY (project_id, table_asset_relative_path)
                REFERENCES asset_metadata(project_id, asset_relative_path) ON DELETE CASCADE ON UPDATE CASCADE
        )",
        [],
    )?;

    // table_styles table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS table_styles (
            project_id TEXT NOT NULL,
            table_path TEXT NOT NULL,
            styles TEXT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (project_id, table_path)
        )",
        [],
    )?;

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

    // file_groups table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS file_groups (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_asset_path TEXT NOT NULL,
            group_id TEXT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            added_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (project_id, file_asset_path) REFERENCES asset_metadata(project_id, asset_relative_path) ON DELETE CASCADE ON UPDATE CASCADE,
            UNIQUE (project_id, file_asset_path, group_id)
        )",
        [],
    )?;

    // media_transcript_data table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS media_transcript_data (
            project_id TEXT NOT NULL,
            asset_relative_path TEXT NOT NULL,
            original_import_path TEXT,
            speaker_names_json TEXT,
            language_code TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (project_id, asset_relative_path),
            FOREIGN KEY (project_id, asset_relative_path)
                REFERENCES asset_metadata(project_id, asset_relative_path) ON DELETE CASCADE ON UPDATE CASCADE
        )",
        [],
    )?;

    // tags table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id TEXT NOT NULL,
            name TEXT NOT NULL,
            color TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
            UNIQUE (project_id, name)
        )",
        [],
    )?;

    // highlights table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS highlights (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            asset_id TEXT NOT NULL,
            project_id TEXT NOT NULL,
            start_offset INTEGER,
            end_offset INTEGER,
            text TEXT,
            annotation_id TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (project_id, asset_id) REFERENCES asset_metadata(project_id, asset_relative_path) ON DELETE CASCADE
        )",
        [],
    )?;

    // highlight_tags table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS highlight_tags (
            highlight_id INTEGER NOT NULL,
            tag_id INTEGER NOT NULL,
            project_id TEXT NOT NULL,
            PRIMARY KEY (highlight_id, tag_id),
            FOREIGN KEY (highlight_id) REFERENCES highlights(id) ON DELETE CASCADE,
            FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE,
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        )",
        [],
    )?;
    Ok(())
}

#[cfg(test)]
pub fn setup_test_db_in_memory() -> (Connection, String) {
    let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
    init_db_for_test(&conn).expect("Failed to initialize test DB schema");

    let project_id = "test_project_1".to_string();
    conn.execute(
        "INSERT INTO projects (id, name, root_path, xml_path) VALUES (?1, ?2, ?3, ?4)",
        params![&project_id, "Test Project", "/fake/path", "/fake/path/project.xml"],
    ).expect("Failed to insert test project");

    (conn, project_id)
}

#[cfg(test)]
mod get_highlights_by_tag_tests {
    use super::*;
    use rusqlite::Connection;
    use serde_json::json;

    fn setup_test_db_with_data() -> (Connection, String) {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
        init_db_for_test(&conn).expect("Failed to initialize test DB");

        let project_id = "test_project_1".to_string();
        conn.execute(
            "INSERT INTO projects (id, name, root_path, xml_path) VALUES (?1, ?2, ?3, ?4)",
            params![&project_id, "Test Project", "/fake/path", "/fake/path/project.xml"],
        ).expect("Failed to insert test project");

        // Insert mock data
        let pdf_ann_json = json!([
            {"id": "uuid-pdf-1", "text": "PDF highlight text", "tags": ["tag1", "tag2"], "color": "#FF0000"}
        ]).to_string();
        conn.execute(
            "INSERT INTO pdf_annotations (project_id, pdf_document_path, annotations_json, document_type) VALUES (?1, ?2, ?3, ?4)",
            params![&project_id, "path/to/my.pdf", &pdf_ann_json, "pdf"],
        ).unwrap();
        conn.execute("INSERT INTO asset_metadata (project_id, asset_relative_path, file_name, file_path, last_modified, asset_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![&project_id, "path/to/my.pdf", "my.pdf", "/abs/path/to/my.pdf", "0", "pdf"]
        ).unwrap();


        let lexical_ann_json = json!([
            {"id": "uuid-lex-1", "text": "Lexical highlight", "tags": ["tag2"], "color": "#00FF00"}
        ]).to_string();
        conn.execute(
            "INSERT INTO pdf_annotations (project_id, pdf_document_path, annotations_json, document_type) VALUES (?1, ?2, ?3, ?4)",
            params![&project_id, "path/to/lexical.json", &lexical_ann_json, "lexical"],
        ).unwrap();
         conn.execute("INSERT INTO asset_metadata (project_id, asset_relative_path, file_name, file_path, last_modified, asset_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![&project_id, "path/to/lexical.json", "lexical.json", "/abs/path/to/lexical.json", "0", "lexical"]
        ).unwrap();


        let image_ann_json = json!([
            {"id": "uuid-img-1", "text": "[Image Highlight]", "tags": ["tag1"], "body": [{"value": "#0000FF"}]}
        ]).to_string();
        conn.execute(
            "INSERT INTO pdf_annotations (project_id, pdf_document_path, annotations_json, document_type) VALUES (?1, ?2, ?3, ?4)",
            params![&project_id, "path/to/image.png", &image_ann_json, "image"],
        ).unwrap();
         conn.execute("INSERT INTO asset_metadata (project_id, asset_relative_path, file_name, file_path, last_modified, asset_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![&project_id, "path/to/image.png", "image.png", "/abs/path/to/image.png", "0", "image"]
        ).unwrap();

        let table_styles_inner = json!([
            {"id": "uuid-tbl-1", "text": "Table cell text", "tags": ["tag1", "tag3"], "color": "#FFFF00"}
        ]);
        let table_styles_outer = serde_json::to_string(&table_styles_inner).unwrap();
        conn.execute(
            "INSERT INTO table_styles (project_id, table_path, styles) VALUES (?1, ?2, ?3)",
            params![&project_id, "path/to/table.csv", &table_styles_outer],
        ).unwrap();
         conn.execute("INSERT INTO asset_metadata (project_id, asset_relative_path, file_name, file_path, last_modified, asset_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![&project_id, "path/to/table.csv", "table.csv", "/abs/path/to/table.csv", "0", "table"]
        ).unwrap();


        (conn, project_id)
    }

    #[test]
    fn test_get_highlights_for_tag1() {
        let (conn, project_id) = setup_test_db_with_data();
        let highlights = get_highlights_by_tag(&conn, &project_id, "tag1").unwrap();
        assert_eq!(highlights.len(), 3); // pdf, image, table

        // Check PDF highlight
        let pdf_highlight = highlights.iter().find(|h| h.0.id == "uuid-pdf-1").unwrap();
        assert_eq!(pdf_highlight.0.text, "PDF highlight text");
        assert_eq!(pdf_highlight.0.color, "#FF0000");
        assert_eq!(pdf_highlight.3.as_deref(), Some("pdf"));

        // Check Image highlight
        let img_highlight = highlights.iter().find(|h| h.0.id == "uuid-img-1").unwrap();
        assert_eq!(img_highlight.0.text, "[Image Highlight]");
        assert_eq!(img_highlight.0.color, "#0000FF");
        assert_eq!(img_highlight.3.as_deref(), Some("image"));

        // Check Table highlight
        let tbl_highlight = highlights.iter().find(|h| h.0.id == "uuid-tbl-1").unwrap();
        assert_eq!(tbl_highlight.0.text, "Table cell text");
        assert_eq!(tbl_highlight.0.color, "#FFFF00");
        assert_eq!(tbl_highlight.3.as_deref(), Some("table"));
    }

    #[test]
    fn test_get_highlights_for_tag2() {
        let (conn, project_id) = setup_test_db_with_data();
        let highlights = get_highlights_by_tag(&conn, &project_id, "tag2").unwrap();
        assert_eq!(highlights.len(), 2); // pdf, lexical

        // Check PDF highlight
        let pdf_highlight = highlights.iter().find(|h| h.0.id == "uuid-pdf-1").unwrap();
        assert_eq!(pdf_highlight.0.text, "PDF highlight text");
        assert_eq!(pdf_highlight.3.as_deref(), Some("pdf"));

        // Check Lexical highlight
        let lex_highlight = highlights.iter().find(|h| h.0.id == "uuid-lex-1").unwrap();
        assert_eq!(lex_highlight.0.text, "Lexical highlight");
        assert_eq!(lex_highlight.3.as_deref(), Some("lexical"));
    }

    #[test]
    fn test_get_highlights_for_tag3() {
        let (conn, project_id) = setup_test_db_with_data();
        let highlights = get_highlights_by_tag(&conn, &project_id, "tag3").unwrap();
        assert_eq!(highlights.len(), 1); // table only
        let tbl_highlight = &highlights[0];
        assert_eq!(tbl_highlight.0.id, "uuid-tbl-1");
        assert_eq!(tbl_highlight.3.as_deref(), Some("table"));
    }

    #[test]
    fn test_get_highlights_for_non_existent_tag() {
        let (conn, project_id) = setup_test_db_with_data();
        let highlights = get_highlights_by_tag(&conn, &project_id, "tag_that_does_not_exist").unwrap();
        assert_eq!(highlights.len(), 0);
    }
}
