// src-tauri/src/lib.rs
use dashmap::DashMap;
use std::sync::{Arc, atomic::AtomicBool};
use env_logger;
use tauri::Manager;

// --- Declare top-level modules ---
mod welcome;
mod projectview; 

// Define the state for managing download cancellation flags
#[derive(Default)]
pub struct DownloadCancellationState(pub Arc<DashMap<String, Arc<AtomicBool>>>);

// Define state for managing transcription cancellation flags
#[derive(Default)]
pub struct TranscriptionCancellationState(pub Arc<DashMap<String, Arc<AtomicBool>>>);


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(e) = crate::welcome::config::ensure_config_dir_exists() {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
        log::error!("Fatal Error: Failed to ensure config directory exists: {}", e.message);
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    log::info!("Starting Harvey application...");

    tauri::Builder::default()
        .manage(DownloadCancellationState::default())
        .manage(TranscriptionCancellationState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(debug_assertions)] {
                 match app.get_webview_window("main") {
                    Some(window) => {
                         log::debug!("Opening devtools for main window");
                         window.open_devtools();
                    },
                    None => log::warn!("Could not get main window handle to open devtools.")
                 }
            }
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Regular);

            Ok(())
         })
        .invoke_handler(tauri::generate_handler![
            // --- Welcome screen commands ---
            welcome::commands::load_recent_projects,
            welcome::commands::create_project,
            welcome::commands::locate_in_finder,
            welcome::commands::rename_project,
            welcome::commands::remove_project_from_list,
            welcome::commands::open_project,
            welcome::commands::import_project,
            welcome::commands::delete_project,

            // --- Configuration commands ---
            welcome::commands::ensure_directory,
            welcome::commands::save_download_location,
            welcome::commands::get_download_location,
            welcome::commands::get_downloaded_models,
            welcome::commands::delete_model,
            welcome::commands::download_model_command,
            welcome::commands::cancel_download_command,
            welcome::commands::change_download_location_and_move_models,
            welcome::commands::get_cloud_config,
            welcome::commands::save_cloud_config,
            welcome::commands::get_theme_preference,
            welcome::commands::set_theme_preference,

            // --- Project view CORE commands ---
            projectview::core_commands::load_project_data,
            projectview::core_commands::import_media,
            projectview::core_commands::rename_project_item,
            projectview::core_commands::delete_project_item,

            // --- Project view TRANSCRIPTION commands ---
            projectview::transcription_commands::load_transcript_json,
            projectview::transcription_commands::save_transcript_json,
            projectview::transcription_commands::trim_media,
            projectview::transcription_commands::save_speaker_config,

            // --- Project view DOCUMENT/NOTES commands ---
            projectview::document_commands::save_note_json,
            projectview::document_commands::load_note_json,
            projectview::document_commands::save_document_and_update_xml,
            projectview::document_commands::read_file_content,
            projectview::document_commands::delete_temporary_file,
            projectview::document_commands::get_unique_document_path,
            projectview::document_commands::save_document_metadata,
            projectview::document_commands::load_document_metadata,
            
            // --- Project view PDF ANNOTATION commands --- ADDED
            projectview::pdf_annotation_handler::load_pdf_annotations,
            projectview::pdf_annotation_handler::save_pdf_annotations,
            // --- END ADDED ---

            // --- Document Import Process Command ---
            projectview::document_handler::import_document,
            projectview::export_handler::export_transcript_to_docx,

            // --- Project view transcription PROCESS commands ---
            projectview::local_handler::transcription::run_transcription,
            projectview::local_handler::transcription::cancel_transcription,
            projectview::cloud_handler::cloud_transcribe::run_cloud_transcription,
            projectview::cloud_handler::cloud_transcribe::cancel_cloud_transcription,

            // --- Project view TABLE commands ---
            projectview::table_handler::import_table_file,
            projectview::table_handler::load_table_data,

            // --- Project view IMAGE commands ---
            projectview::image_handler::import_image_file,
            projectview::image_handler::load_image_annotations,
            projectview::image_handler::save_image_annotations,

            // --- Project view TRANSCRIPT IMPORT command ---
            projectview::transcription_handler::import_word_transcript

        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}