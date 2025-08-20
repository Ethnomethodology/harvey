// src-tauri/src/projectview/lexical_highlight_handler.rs
use super::db_handler;
use crate::welcome::config::CommandError;
use serde::Deserialize;

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
