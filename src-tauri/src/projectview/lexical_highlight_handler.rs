// src-tauri/src/projectview/lexical_highlight_handler.rs
use super::db_handler;
use crate::welcome::config::CommandError;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use log::{info, error};

#[derive(Deserialize)]
pub struct SaveHighlightsArgs<'a> {
    #[serde(rename = "projectId")]
    project_id: &'a str,
    #[serde(rename = "documentPath")]
    document_path: &'a str,
    #[serde(rename = "highlightsJson")]
    highlights_json: &'a str,
}

#[tauri::command]
pub fn save_lexical_highlights(
    app_handle: AppHandle,
    file_path: String,
    doc_type: String,
    highlights: Vec<LexicalHighlight>,
) -> Result<(), String> {
    let project_id = get_project_id_from_app_handle(&app_handle)?;
    let db_path = get_project_db_path(&app_handle, &project_id)?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

    let mut doc_type_for_db = doc_type.clone();
    if doc_type == "transcript" || doc_type == "imported_transcript" {
        doc_type_for_db = "lexical".to_string();
    }

    // If we are migrating, we need to delete the old record.
    if doc_type != doc_type_for_db {
        conn.execute(
            "DELETE FROM lexical_highlights WHERE file_path = ?1 AND doc_type = ?2",
            params![&file_path, &doc_type],
        ).optional().map_err(|e| e.to_string())?; // optional in case it doesn't exist
    }

    let highlights_json = serde_json::to_string(&highlights).map_err(|e| e.to_string())?;

    conn.execute(
        "REPLACE INTO lexical_highlights (file_path, doc_type, project_id, highlights) VALUES (?1, ?2, ?3, ?4)",
        params![file_path, doc_type_for_db, project_id, highlights_json],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[derive(Deserialize)]
pub struct LoadHighlightsArgs<'a> {
    #[serde(rename = "projectId")]
    project_id: &'a str,
    #[serde(rename = "documentPath")]
    document_path: &'a str,
}

#[tauri::command]
pub fn load_lexical_highlights(args: LoadHighlightsArgs) -> Result<Option<String>, CommandError> {
    db_handler::load_lexical_highlights_from_db(args.project_id, args.document_path)
}

#[derive(Deserialize)]
pub struct DeleteHighlightsArgs<'a> {
    #[serde(rename = "projectId")]
    project_id: &'a str,
    #[serde(rename = "documentPath")]
    document_path: &'a str,
}

#[tauri::command]
pub fn delete_lexical_highlights(args: DeleteHighlightsArgs) -> Result<(), CommandError> {
    db_handler::delete_lexical_highlights_from_db(args.project_id, args.document_path)
}

use rusqlite::{Connection, OptionalExtension};
use crate::file_tree::get_project_id_from_app_handle;
use tauri::AppHandle;

#[tauri::command]
pub fn save_highlight_changes(
    app_handle: AppHandle,
    updated_highlight: LexicalHighlight,
    file_path: String,
    doc_type: String,
) -> Result<(), String> {
    let project_id = get_project_id_from_app_handle(&app_handle)?;
    let db_path = get_project_db_path(&app_handle, &project_id)?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

    let mut doc_type_for_db = doc_type.clone();
    if doc_type == "transcript" || doc_type == "imported_transcript" {
        doc_type_for_db = "lexical".to_string();
    }

    // Try to get existing highlights, preferring the normalized doc_type
    let row: Option<(String, String)> = conn.query_row(
        "SELECT highlights, doc_type FROM lexical_highlights WHERE file_path = ?1 AND doc_type IN (?2, ?3) ORDER BY doc_type = ?2 DESC",
        params![&file_path, &doc_type_for_db, &doc_type],
        |row| Ok((row.get(0)?, row.get(1)?))
    ).optional().map_err(|e| e.to_string())?;

    let mut highlights: Vec<LexicalHighlight> = if let Some((json, _)) = &row {
        serde_json::from_str(json).map_err(|e| e.to_string())?
    } else {
        Vec::new()
    };

    if let Some(pos) = highlights.iter().position(|h| h.id == updated_highlight.id) {
        highlights[pos] = updated_highlight;
    } else {
        highlights.push(updated_highlight);
    }

    let updated_highlights_json = serde_json::to_string(&highlights).map_err(|e| e.to_string())?;

    // If we found a record with the old, non-normalized doc_type, delete it to prepare for migration.
    if let Some((_, found_doc_type)) = row {
        if found_doc_type != doc_type_for_db {
            conn.execute(
                "DELETE FROM lexical_highlights WHERE file_path = ?1 AND doc_type = ?2",
                params![&file_path, &found_doc_type],
            ).map_err(|e| e.to_string())?;
        }
    }

    // Use REPLACE to either insert a new record or update an existing one with the normalized doc_type.
    conn.execute(
        "REPLACE INTO lexical_highlights (file_path, doc_type, project_id, highlights) VALUES (?1, ?2, ?3, ?4)",
        params![file_path, doc_type_for_db, project_id, updated_highlights_json],
    ).map_err(|e| e.to_string())?;

    Ok(())
}
