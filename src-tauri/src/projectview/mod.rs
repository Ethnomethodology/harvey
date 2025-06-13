// src-tauri/src/projectview/mod.rs

// Declare the new modules for shared code and categorized commands
// --- Project View Main Module ---
pub mod core_commands;
pub mod transcription_commands;
pub mod document_commands;
pub mod metadata_commands;
pub mod document_handler;
pub mod table_handler;
pub mod image_handler;
pub mod transcription_handler; // For imported transcripts
pub mod pdf_annotation_handler; // ADDED
pub mod local_handler;
pub mod cloud_handler;
pub mod export_handler;
pub mod shared_types;
pub mod shared_utils;
pub mod db_handler;

// Re-export specific commands to make them available directly under `projectview::`
pub use transcription_commands::{list_subtitle_files_command, convert_srt_to_vtt_command};