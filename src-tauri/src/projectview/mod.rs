// src-tauri/src/projectview/mod.rs

// Declare the new modules for shared code and categorized commands
// --- Project View Main Module ---
pub mod core_commands;
pub mod transcription_commands;
pub mod document_commands;
pub mod metadata_commands;
pub mod attachment_commands;
pub mod document_handler;
pub mod table_handler;
pub mod image_handler;
pub mod transcription_handler; // For imported transcripts
pub mod pdf_annotation_handler; // ADDED
pub mod translation_commands;
pub mod local_handler;


pub mod export_handler;
pub mod shared_types;
pub mod shared_utils;
pub mod db_handler;
pub mod waveform_utils;
pub mod lexical_highlight_handler;
pub mod tag_handler;
pub mod utils;

// Re-export specific commands if needed, for now, direct paths are used in lib.rs