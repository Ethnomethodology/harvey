// src-tauri/src/lib.rs
use dashmap::DashMap;
use std::sync::{Arc, atomic::AtomicBool};
use env_logger;
use log; // Added log import
use tauri::Manager; // Ensure Manager is used for app.handle()
use tauri_plugin_global_shortcut::{
    self, // Keep or remove 'self' based on preference for qualification
    Code, // For Shortcut::new(..., Code::F7)
    GlobalShortcutExt,
    Modifiers,
    Shortcut,
    ShortcutEvent,
    ShortcutState,
};
// use tauri::Wry; // Still needed for app_handle_clone if it's explicitly typed
use tauri::Emitter; // For app.emit()
use crate::projectview::db_handler::init_db as init_projectview_db;
// Removed: use crate::projectview::transcription_commands::{list_subtitle_files_command, convert_srt_to_vtt_command};

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
        log::error!("Fatal Error: Failed to ensure config directory exists: {}", e);
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    // Initialize ProjectView Database
    if let Err(e) = init_projectview_db() {
        log::error!("Failed to initialize project view database: {}", e);
        // Depending on severity, you might want to panic or show a dialog to the user.
        // For now, just logging.
    }

    log::info!("Starting Harvey application...");

    tauri::Builder::default()
        .manage(DownloadCancellationState::default())
        .manage(TranscriptionCancellationState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        // Global shortcut plugin is now initialized in .setup
        .setup(|app_mut_ref| -> Result<(), Box<dyn std::error::Error>> {
            #[cfg(debug_assertions)] {
                 match app_mut_ref.get_webview_window("main") {
                    Some(window) => {
                         log::debug!("Opening devtools for main window");
                         window.open_devtools();
                    },
                    None => log::warn!("Could not get main window handle to open devtools.")
                 }
            }
            #[cfg(target_os = "macos")]
            app_mut_ref.set_activation_policy(tauri::ActivationPolicy::Regular);

            log::info!("[SETUP] Preparing to set up global shortcuts...");
            let app_handle_clone = app_mut_ref.handle().clone(); // Clone app handle for the handler

            // Define the shortcuts
            let f7_shortcut = Shortcut::new(Some(Modifiers::empty()), Code::F7);
            let f8_shortcut = Shortcut::new(Some(Modifiers::empty()), Code::F8);
            let f9_shortcut = Shortcut::new(Some(Modifiers::empty()), Code::F9);
            log::info!("[SETUP] Defined F7, F8, F9 shortcut objects.");

            // Build the plugin with a general handler
            let global_shortcut_plugin_instance = tauri_plugin_global_shortcut::Builder::with_handler(
                move |_, // First parameter from handler (likely an AppHandle from plugin) is ignored
                      shortcut_arg: &Shortcut,
                      event_details: &ShortcutEvent| {

                    if event_details.state == ShortcutState::Pressed { // ShortcutState can now be used directly
                        log::info!("[HANDLER] Global shortcut pressed: shortcut_arg: {:?}, state: {:?}", shortcut_arg, event_details.state);
                        if shortcut_arg == &f7_shortcut {
                            log::info!("[HANDLER] F7 shortcut matched.");
                            app_handle_clone.emit("shortcut-event", "rewind").unwrap_or_else(|e| {
                                log::error!("[HANDLER] Failed to emit rewind event for F7: {}", e);
                            });
                        } else if shortcut_arg == &f8_shortcut {
                            log::info!("[HANDLER] F8 shortcut matched.");
                            app_handle_clone.emit("shortcut-event", "play-pause").unwrap_or_else(|e| {
                                log::error!("[HANDLER] Failed to emit play-pause event for F8: {}", e);
                            });
                        } else if shortcut_arg == &f9_shortcut {
                            log::info!("[HANDLER] F9 shortcut matched.");
                            app_handle_clone.emit("shortcut-event", "forward").unwrap_or_else(|e| {
                                log::error!("[HANDLER] Failed to emit forward event for F9: {}", e);
                            });
                        }
                    }
                },
            )
            .build();
            log::info!("[SETUP] Global shortcut plugin builder created with handler.");

            // Register the plugin instance with the app
            app_mut_ref.handle().plugin(global_shortcut_plugin_instance)?;
            log::info!("[SETUP] Global shortcut plugin registered with app handle.");

            // Explicitly register each shortcut
            app_mut_ref.global_shortcut().register(f7_shortcut)?;
            log::info!("[SETUP] F7 shortcut registered.");
            app_mut_ref.global_shortcut().register(f8_shortcut)?;
            log::info!("[SETUP] F8 shortcut registered.");
            app_mut_ref.global_shortcut().register(f9_shortcut)?;
            log::info!("[SETUP] F9 shortcut registered. All shortcuts registration process completed.");

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

            // --- Project view METADATA commands (asset_metadata table) ---
            projectview::metadata_commands::get_asset_metadata_command,
            projectview::metadata_commands::update_asset_metadata_command,
            // --- Custom Field Definition Commands ---
            projectview::metadata_commands::create_custom_field_definition_command,
            projectview::metadata_commands::get_all_custom_field_definitions_command,
            projectview::metadata_commands::delete_custom_field_definition_command, // Added this line

            // --- Project view TRANSCRIPTION commands ---
            projectview::transcription_commands::load_transcript_json,
            projectview::transcription_commands::save_transcript_json,
            projectview::transcription_commands::trim_media,
            projectview::transcription_commands::save_speaker_config,
            projectview::transcription_commands::list_subtitle_files_command,
            projectview::transcription_commands::convert_srt_to_vtt_command,

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
            projectview::core_commands::save_table_layout_prefs,
            projectview::core_commands::load_table_layout_prefs,

            // --- Project view IMAGE commands ---
            projectview::image_handler::import_image_file,
            projectview::image_handler::load_image_annotations,
            projectview::image_handler::save_image_annotations,
            projectview::image_handler::save_screenshot,

            // --- Project view TRANSCRIPT IMPORT command ---
            projectview::transcription_handler::import_word_transcript

        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}