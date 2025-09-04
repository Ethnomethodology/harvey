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
pub fn save_lexical_highlights(args: SaveHighlightsArgs) -> Result<(), CommandError> {
    db_handler::save_lexical_highlights_to_db(args.project_id, args.document_path, args.highlights_json)
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveHighlightChangesArgs {
    project_id: String,
    file_path: String,
    doc_type: String,
    highlight: JsonValue,
}

#[tauri::command]
pub fn save_highlight_changes(args: SaveHighlightChangesArgs) -> Result<(), CommandError> {
    info!(
        "Received request to save highlight changes for project_id: {}, path: {}, doc_type: {}",
        args.project_id, args.file_path, args.doc_type
    );

    // 1. Load existing highlights
    let existing_highlights_json =
        db_handler::load_annotations_from_db(&args.project_id, &args.file_path, &args.doc_type)?;

    let mut highlights: Vec<JsonValue> = match existing_highlights_json {
        Some(json_str) => serde_json::from_str(&json_str).unwrap_or_else(|e| {
            error!("Failed to parse existing highlights JSON: {}. Starting with a new list.", e);
            Vec::new()
        }),
        None => Vec::new(),
    };

    // 2. Find and update the highlight
    let highlight_id_to_update = args.highlight.get("id").and_then(|id| id.as_str());

    if let Some(id_to_update) = highlight_id_to_update {
        let mut found = false;
        for highlight in highlights.iter_mut() {
            if let Some(id) = highlight.get("id").and_then(|id| id.as_str()) {
                if id == id_to_update {
                    *highlight = args.highlight.clone();
                    found = true;
                    break;
                }
            }
        }
        if !found {
            // If not found, it's a new highlight, so add it.
            highlights.push(args.highlight);
        }
    } else {
        return Err(CommandError::from("Highlight data is missing an 'id' field."));
    }


    // 3. Serialize the updated highlights list back to JSON
    let updated_highlights_json = serde_json::to_string(&highlights)
        .map_err(|e| CommandError::from(format!("Failed to serialize updated highlights: {}", e)))?;

    // 4. Save the updated JSON back to the database
    db_handler::save_annotations_to_db(
        &args.project_id,
        &args.file_path,
        &updated_highlights_json,
        &args.doc_type,
    )
}
