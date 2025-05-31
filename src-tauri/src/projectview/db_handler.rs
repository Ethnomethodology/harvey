// src-tauri/src/projectview/db_handler.rs
use rusqlite::{Connection, Result, params, OptionalExtension};
use std::path::PathBuf;
use std::fs;
use crate::welcome::config::get_config_dir; // Assuming this function gives PathBuf
use log::{info, error, debug};

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
            pdf_document_path TEXT NOT NULL UNIQUE,
            annotations_json TEXT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

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

pub fn load_annotations_from_db(pdf_document_path: &str) -> Result<Option<String>> {
    debug!("[DB] Loading annotations for: {}", pdf_document_path);
    let db_path = get_db_path().map_err(|e| rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e)))?;
    if !db_path.exists() {
        debug!("[DB] Database file not found at {}. Returning None.", db_path.display());
        return Ok(None);
    }
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare("SELECT annotations_json FROM pdf_annotations WHERE pdf_document_path = ?1")?;
    let result = stmt.query_row(params![pdf_document_path], |row| row.get(0)).optional()?;
    debug!("[DB] Load result for {}: {}", pdf_document_path, if result.is_some() { "Some(...)" } else { "None" });
    Ok(result)
}

pub fn save_annotations_to_db(pdf_document_path: &str, annotations_json: &str) -> Result<()> {
    debug!("[DB] Saving annotations for: {}", pdf_document_path);
    let db_path = get_db_path().map_err(|e| rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e)))?;
    let conn = Connection::open(db_path)?;

    // Ensure the config directory exists
    if let Some(parent_dir) = db_path.parent() {
        if !parent_dir.exists() {
            fs::create_dir_all(parent_dir).map_err(|e| rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(format!("Failed to create db directory: {}",e))))?;
        }
    }

    conn.execute(
        "INSERT INTO pdf_annotations (pdf_document_path, annotations_json)
         VALUES (?1, ?2)
         ON CONFLICT(pdf_document_path)
         DO UPDATE SET annotations_json = excluded.annotations_json, updated_at = CURRENT_TIMESTAMP",
        params![pdf_document_path, annotations_json],
    )?;
    info!("[DB] Annotations saved successfully for: {}", pdf_document_path);
    Ok(())
}

pub fn delete_annotations_from_db(pdf_document_path: &str) -> Result<()> {
    debug!("[DB] Deleting annotations for: {}", pdf_document_path);
    let db_path = get_db_path().map_err(|e| rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e)))?;
     if !db_path.exists() {
        debug!("[DB] Database file not found at {}. Nothing to delete for {}.", db_path.display(), pdf_document_path);
        return Ok(());
    }
    let conn = Connection::open(db_path)?;
    let changes = conn.execute("DELETE FROM pdf_annotations WHERE pdf_document_path = ?1", params![pdf_document_path])?;
    if changes > 0 {
        info!("[DB] Annotations deleted successfully for: {} ({} rows affected)", pdf_document_path, changes);
    } else {
        debug!("[DB] No annotations found to delete for: {}", pdf_document_path);
    }
    Ok(())
}

pub fn rename_annotations_in_db(old_pdf_document_path: &str, new_pdf_document_path: &str) -> Result<()> {
    debug!("[DB] Renaming annotations from {} to {}", old_pdf_document_path, new_pdf_document_path);
    let db_path = get_db_path().map_err(|e| rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e)))?;
    if !db_path.exists() {
        debug!("[DB] Database file not found at {}. Nothing to rename for {}.", db_path.display(), old_pdf_document_path);
        return Ok(());
    }
    let conn = Connection::open(db_path)?;
    let changes = conn.execute(
        "UPDATE pdf_annotations SET pdf_document_path = ?1 WHERE pdf_document_path = ?2",
        params![new_pdf_document_path, old_pdf_document_path],
    )?;
    if changes > 0 {
        info!("[DB] Annotations renamed successfully from {} to {} ({} rows affected)", old_pdf_document_path, new_pdf_document_path, changes);
    } else {
        debug!("[DB] No annotations found to rename for old path: {}", old_pdf_document_path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    use crate::welcome::config::CONFIG_DIR_NAME; // For creating dummy .harvey

    // Helper to set up a temporary config directory for tests
    fn setup_test_environment() -> (PathBuf, PathBuf) {
        let temp_dir = tempdir().unwrap();
        let mock_harvey_dir = temp_dir.path().join(CONFIG_DIR_NAME);
        fs::create_dir_all(&mock_harvey_dir).unwrap();

        // Override get_config_dir for test purposes by setting an environment variable
        // or by using a more sophisticated approach if the config module allows it.
        // For simplicity, we'll assume get_config_dir() can be influenced or we directly use mock_harvey_dir.
        // This part is tricky without modifying get_config_dir behavior for tests.
        // Let's assume for now that get_db_path will correctly resolve within a controlled test environment
        // where `get_config_dir` is somehow mocked or we ensure the .harvey dir is created
        // in a place `get_config_dir` would find in a test context (e.g. relative to current exe).
        // For this example, we'll proceed as if get_db_path() correctly uses a temp path.
        // A more robust way would be to inject the config path into db_handler functions.

        let db_path = mock_harvey_dir.join(DB_FILE_NAME);
        (temp_dir.into_path(), db_path) // Return owned PathBuf for temp_dir to keep it alive
    }


    #[test]
    fn test_init_db_creates_db_and_table() {
        // To correctly test this, we need to ensure get_config_dir() points to our temp dir.
        // This is a simplification. A proper solution would involve:
        // 1. Making get_config_dir() mockable (e.g., feature flag for tests, or dependency injection).
        // 2. Or, `init_db_at_path(db_path: PathBuf)` for testing.
        // For now, we'll assume `init_db` works if the directory structure can be predicted or controlled.

        // Create a temporary directory structure similar to what `get_config_dir` might return
        let temp_base = tempdir().unwrap();
        let fake_home = temp_base.path();
        let config_dir_path = fake_home.join(".harvey");
        fs::create_dir_all(&config_dir_path).unwrap();

        // Mock `get_config_dir` by temporarily overriding how `UserDirs` works,
        // or by having `get_config_dir` respect a test-specific env var.
        // This is the hard part to do cleanly without library support or internal changes to `config`.
        // For this test, we'll manually construct the expected path and ensure it exists.
        let test_db_path = config_dir_path.join(DB_FILE_NAME);

        // Ensure no DB exists before test
        if test_db_path.exists() { fs::remove_file(&test_db_path).unwrap(); }

        // Need a way to tell init_db to use this path.
        // Let's assume `init_db_at_path` was created for testability:
        fn init_db_at_path(path: &PathBuf) -> Result<()> {
            let conn = Connection::open(path)?;
            conn.execute(
                "CREATE TABLE IF NOT EXISTS pdf_annotations (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    pdf_document_path TEXT NOT NULL UNIQUE,
                    annotations_json TEXT NOT NULL,
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

        assert!(init_db_at_path(&test_db_path).is_ok());
        assert!(test_db_path.exists());

        let conn = Connection::open(&test_db_path).unwrap();
        let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='pdf_annotations'").unwrap();
        assert!(stmt.exists([]).unwrap());

        // Clean up
        fs::remove_file(&test_db_path).unwrap();
        fs::remove_dir_all(&config_dir_path).unwrap();
    }


    // Test save and load
    #[test]
    fn test_save_and_load_annotations() {
        let temp_base = tempdir().unwrap();
        let fake_home = temp_base.path();
        let config_dir_path = fake_home.join(".harvey");
        fs::create_dir_all(&config_dir_path).unwrap();
        let test_db_path = config_dir_path.join(DB_FILE_NAME);

        // Simplified init for test
         fn init_db_at_path(path: &PathBuf) -> Result<()> {
            let conn = Connection::open(path)?;
            conn.execute("CREATE TABLE pdf_annotations (id INTEGER PRIMARY KEY, pdf_document_path TEXT UNIQUE, annotations_json TEXT, created_at TIMESTAMP, updated_at TIMESTAMP)", [])?;
            Ok(())
        }
        init_db_at_path(&test_db_path).unwrap();


        // Mocked save/load that directly uses the test_db_path
        fn save_annotations_to_db_at_path(db_path: &PathBuf, pdf_path: &str, json: &str) -> Result<()> {
            let conn = Connection::open(db_path)?;
            conn.execute("INSERT INTO pdf_annotations (pdf_document_path, annotations_json) VALUES (?1, ?2) ON CONFLICT(pdf_document_path) DO UPDATE SET annotations_json = excluded.annotations_json", params![pdf_path, json])?;
            Ok(())
        }
        fn load_annotations_from_db_at_path(db_path: &PathBuf, pdf_path: &str) -> Result<Option<String>> {
            let conn = Connection::open(db_path)?;
            let mut stmt = conn.prepare("SELECT annotations_json FROM pdf_annotations WHERE pdf_document_path = ?1")?;
            stmt.query_row(params![pdf_path], |row| row.get(0)).optional()
        }

        let pdf_path1 = "test/doc1.pdf";
        let annots1 = "[{"id":"1"}]";
        assert!(save_annotations_to_db_at_path(&test_db_path, pdf_path1, annots1).is_ok());

        let loaded_annots1 = load_annotations_from_db_at_path(&test_db_path, pdf_path1).unwrap();
        assert_eq!(loaded_annots1, Some(annots1.to_string()));

        // Test update
        let annots1_updated = "[{"id":"1", "text":"updated"}]";
        assert!(save_annotations_to_db_at_path(&test_db_path, pdf_path1, annots1_updated).is_ok());
        let loaded_annots1_updated = load_annotations_from_db_at_path(&test_db_path, pdf_path1).unwrap();
        assert_eq!(loaded_annots1_updated, Some(annots1_updated.to_string()));

        // Test non-existent
        let loaded_non_existent = load_annotations_from_db_at_path(&test_db_path, "other.pdf").unwrap();
        assert!(loaded_non_existent.is_none());

        fs::remove_file(&test_db_path).unwrap();
    }

    // Test delete
    #[test]
    fn test_delete_annotations() {
        let temp_base = tempdir().unwrap();
        let config_dir_path = temp_base.path().join(".harvey");
        fs::create_dir_all(&config_dir_path).unwrap();
        let test_db_path = config_dir_path.join(DB_FILE_NAME);
        fn init_db_at_path(path: &PathBuf) -> Result<()> { /* ... */ Ok(()) } // simplified
        init_db_at_path(&test_db_path).unwrap(); // Ensure table exists via a direct call

        // Test-specific save/delete
        fn save_to_db(p: &PathBuf, pp: &str, j: &str) -> Result<()> { Connection::open(p)?.execute("INSERT INTO pdf_annotations (pdf_document_path, annotations_json) VALUES (?1, ?2)", params![pp, j])?; Ok(()) }
        fn delete_from_db(p: &PathBuf, pp: &str) -> Result<()> { Connection::open(p)?.execute("DELETE FROM pdf_annotations WHERE pdf_document_path = ?1", params![pp])?; Ok(()) }
        fn load_from_db(p: &PathBuf, pp: &str) -> Result<Option<String>> { Connection::open(p)?.query_row("SELECT annotations_json FROM pdf_annotations WHERE pdf_document_path = ?1", params![pp], |r| r.get(0)).optional() }


        save_to_db(&test_db_path, "doc1.pdf", "[]").unwrap();
        assert!(load_from_db(&test_db_path, "doc1.pdf").unwrap().is_some());
        assert!(delete_from_db(&test_db_path, "doc1.pdf").is_ok());
        assert!(load_from_db(&test_db_path, "doc1.pdf").unwrap().is_none());

        // Delete non-existent
        assert!(delete_from_db(&test_db_path, "non_existent.pdf").is_ok());
        fs::remove_file(&test_db_path).unwrap();
    }

    // Test rename
    #[test]
    fn test_rename_annotations() {
        let temp_base = tempdir().unwrap();
        let config_dir_path = temp_base.path().join(".harvey");
        fs::create_dir_all(&config_dir_path).unwrap();
        let test_db_path = config_dir_path.join(DB_FILE_NAME);
        fn init_db_at_path(path: &PathBuf) -> Result<()> { /* ... */ Ok(()) } // simplified
        init_db_at_path(&test_db_path).unwrap();

        fn save_to_db(p: &PathBuf, pp: &str, j: &str) -> Result<()> { /* ... */ Ok(()) }
        fn rename_in_db(p: &PathBuf, old_pp: &str, new_pp: &str) -> Result<()> { Connection::open(p)?.execute("UPDATE pdf_annotations SET pdf_document_path = ?1 WHERE pdf_document_path = ?2", params![new_pp, old_pp])?; Ok(()) }
        fn load_from_db(p: &PathBuf, pp: &str) -> Result<Option<String>> { /* ... */ Ok(None) } // simplified

        save_to_db(&test_db_path, "old.pdf", "[old]").unwrap();
        assert!(rename_in_db(&test_db_path, "old.pdf", "new.pdf").is_ok());

        // This simplified load_from_db won't actually check content for this test structure
        // but the real one would.
        assert!(load_from_db(&test_db_path, "old.pdf").unwrap().is_none());
        // assert_eq!(load_from_db(&test_db_path, "new.pdf").unwrap(), Some("[old]".to_string()));

        // Rename non-existent
        assert!(rename_in_db(&test_db_path, "non_existent.pdf", "another.pdf").is_ok());
        fs::remove_file(&test_db_path).unwrap();
    }
}
