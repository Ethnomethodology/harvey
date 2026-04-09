// src-tauri/src/projectview/lexical_highlight_handler.rs
use super::db_handler;
use crate::projectview::shared_utils::normalize_path_for_comparison;
use crate::welcome::config::CommandError;
use log::{error, info};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::path::Path;

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
    db_handler::save_lexical_highlights_to_db(
        args.project_id,
        args.document_path,
        args.highlights_json,
    )
}

#[tauri::command]
pub fn save_highlight_changes(
    project_id: String,
    file_path: String,
    doc_type: String,
    highlight: JsonValue,
) -> Result<(), CommandError> {
    let normalized_file_path = normalize_path_for_comparison(Path::new(&file_path))
        .to_string_lossy()
        .to_string();

    info!(
        "Received request to save highlight changes for project_id: {}, path: {}, doc_type: {} [Normalized: {}]",
        project_id, file_path, &doc_type, normalized_file_path
    );

    let normalized_doc_type = if doc_type == "audio_transcript"
        || doc_type == "standalone_transcript"
        || doc_type == "video_transcript"
    {
        "lexical".to_string()
    } else {
        doc_type.clone()
    };

    let original_doc_type = doc_type;
    let was_migrated = original_doc_type != normalized_doc_type;

    // 1. Load existing highlights, trying normalized first, then original if migration is possible
    let existing_highlights_json =
        db_handler::load_annotations_from_db(&project_id, &normalized_file_path, &normalized_doc_type)?
            .or_else(|| {
                if was_migrated {
                    // Use a blocking Result to handle potential errors inside or_else
                    match db_handler::load_annotations_from_db(&project_id, &normalized_file_path, &original_doc_type) {
                        Ok(Some(json)) => Some(json),
                        Ok(None) => None,
                        Err(e) => {
                            error!("Error loading annotations with original doc_type during migration check: {}", e);
                            None
                        }
                    }
                } else {
                    None
                }
            });

    let mut highlights: Vec<JsonValue> = match existing_highlights_json {
        Some(json_str) => serde_json::from_str(&json_str).unwrap_or_else(|e| {
            error!(
                "Failed to parse existing highlights JSON: {}. Starting with a new list.",
                e
            );
            Vec::new()
        }),
        None => Vec::new(),
    };

    // 2. Find and update the highlight
    let highlight_id_to_update = highlight.get("id").and_then(|id| id.as_str());

    if let Some(id_to_update) = highlight_id_to_update {
        let mut found = false;
        for h in highlights.iter_mut() {
            if let Some(id) = h.get("id").and_then(|id| id.as_str()) {
                if id == id_to_update {
                    *h = highlight.clone();
                    found = true;
                    break;
                }
            }
        }
        if !found {
            highlights.push(highlight);
        }
    } else {
        return Err(CommandError::from(
            "Highlight data is missing an 'id' field.",
        ));
    }

    // 3. Serialize the updated highlights list back to JSON
    let updated_highlights_json = serde_json::to_string(&highlights).map_err(|e| {
        CommandError::from(format!("Failed to serialize updated highlights: {}", e))
    })?;

    // 4. Save the updated JSON back to the database with the normalized doc_type
    db_handler::save_annotations_to_db(
        &project_id,
        &normalized_file_path,
        &updated_highlights_json,
        &normalized_doc_type,
    )?;

    // 5. If migration occurred, delete the old record
    if was_migrated {
        info!(
            "Migrating highlights from doc_type '{}' to '{}' for file: {}",
            &original_doc_type, &normalized_doc_type, &normalized_file_path
        );
        db_handler::delete_annotations_from_db(&project_id, &normalized_file_path, &original_doc_type)?;
    }

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
