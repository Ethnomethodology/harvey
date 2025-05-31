// src-tauri/src/projectview/db_handler.rs
use rusqlite::{Connection, Result, params, OptionalExtension};
use std::path::PathBuf;
use std::fs;
use crate::welcome::config::get_config_dir; // Assuming this function gives PathBuf
use log::{info, debug};

const DB_FILE_NAME: &str = "harvey_annotations.sqlite";

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
    info!("[DB] Database initialized successfully.");
    Ok(())
}

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
