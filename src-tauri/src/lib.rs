// src-tauri/src/lib.rs
use dashmap::DashMap;
use std::sync::{Arc, atomic::AtomicBool};
use env_logger;
use log; // Added log import
use tauri::Manager; // Added Manager import
use tauri::Emitter; // For app.emit()

// use tauri::Wry; // Still needed for app_handle_clone if it's explicitly typed
use crate::projectview::db_handler::init_db as init_projectview_db;
// Removed: use crate::projectview::transcription_commands::{list_subtitle_files_command, convert_srt_to_vtt_command};

// --- Declare top-level modules ---
mod welcome;
mod projectview; 
pub mod transcription;
pub mod utils;

// Define the state for managing download cancellation flags
#[derive(Default)]
pub struct DownloadCancellationState(pub Arc<DashMap<String, Arc<AtomicBool>>>);

// Define state for managing transcription cancellation flags
#[derive(Default)]
pub struct TranscriptionCancellationState(pub Arc<DashMap<String, Arc<AtomicBool>>>);

// Define state for managing translation cancellation flags
#[derive(Default)]
pub struct TranslationCancellationState(pub Arc<DashMap<String, Arc<AtomicBool>>>);


// Define state for managing live transcription
#[derive(Default)]
pub struct LiveTranscriptionState(pub Arc<projectview::transcription_commands::LiveTranscriptionState>);


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
        .manage(TranslationCancellationState::default())
        .manage(projectview::transcription_commands::LiveTranscriptionState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init()) // Added this line
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            if id == "about_harvey" {
                // Check if about window exists
                if let Some(window) = app.get_webview_window("about") {
                    let _ = window.set_focus();
                } else {
                    // Create new about window
                    let _ = tauri::WebviewWindowBuilder::new(
                        app,
                        "about",
                        tauri::WebviewUrl::App("about".into())
                    )
                    .title("About Harvey")
                    .inner_size(600.0, 550.0)
                    .resizable(false)
                    .build();
                }
            } else if id == "configurations_harvey" {
                // Check if configurations window exists
                if let Some(window) = app.get_webview_window("configurations") {
                    let _ = window.set_focus();
                } else {
                    // Create new configurations window
                    let _ = tauri::WebviewWindowBuilder::new(
                        app,
                        "configurations",
                        tauri::WebviewUrl::App("configurations".into())
                    )
                    .title("Configurations")
                    .inner_size(800.0, 700.0)
                    .resizable(true)
                    .build();
                }
            } else if id == "file_new_project" {
                let _ = app.emit("menu:file:new-project", ());
            } else if id == "file_open_project" {
                let _ = app.emit("menu:file:open-project", ());
            } else if id == "file_import_audio" {
                let _ = app.emit("menu:file:import:audio", ());
            } else if id == "file_import_video" {
                let _ = app.emit("menu:file:import:video", ());
            } else if id == "file_import_doc" {
                let _ = app.emit("menu:file:import:document", ());
            } else if id == "file_import_image" {
                let _ = app.emit("menu:file:import:image", ());
            } else if id == "file_import_table" {
                let _ = app.emit("menu:file:import:table", ());
            } else if id == "file_import_transcript" {
                let _ = app.emit("menu:file:import:transcript", ());
            } else if id == "file_create_doc" {
                let _ = app.emit("menu:file:create:document", ());
            } else if id == "file_create_table" {
                let _ = app.emit("menu:file:create:table", ());
            } else if id == "file_create_group" {
                let _ = app.emit("menu:file:create:group", ());
            } else if id == "file_create_tag" {
                let _ = app.emit("menu:file:create:tag", ());
            } else if id == "file_create_tag_group" {
                let _ = app.emit("menu:file:create:tag-group", ());
            } else if id == "help_center" {
                let _ = app.emit("menu:help:center", ());
            } else if id == "view_license" {
                if let Some(window) = app.get_webview_window("license") {
                    let _ = window.set_focus();
                } else {
                    let _ = tauri::WebviewWindowBuilder::new(
                        app,
                        "license",
                        tauri::WebviewUrl::App("license".into())
                    )
                    .title("License")
                    .inner_size(600.0, 500.0)
                    .resizable(true)
                    .build();
                }
            } else if id == "view_credits" {
                if let Some(window) = app.get_webview_window("credits") {
                    let _ = window.set_focus();
                } else {
                    let _ = tauri::WebviewWindowBuilder::new(
                        app,
                        "credits",
                        tauri::WebviewUrl::App("credits".into())
                    )
                    .title("Credits")
                    .inner_size(600.0, 500.0)
                    .resizable(true)
                    .build();
                }
            } else if id == "view_version" {
                if let Some(window) = app.get_webview_window("version") {
                    let _ = window.set_focus();
                } else {
                    let _ = tauri::WebviewWindowBuilder::new(
                        app,
                        "version",
                        tauri::WebviewUrl::App("version".into())
                    )
                    .title("Version")
                    .inner_size(600.0, 500.0)
                    .resizable(true)
                    .build();
                }
            }
        })
        // Global shortcut plugin is now initialized in .setup
        .setup(|app_mut_ref| -> Result<(), Box<dyn std::error::Error>> {
            // log::error!("!!!!!!!!!!!!!!!!! SETUP HOOK ENTERED !!!!!!!!!!!!!!!!!"); // Line removed

            #[cfg(debug_assertions)] {
                use tauri::Manager;
                 match app_mut_ref.get_webview_window("main") {
                    Some(window) => {
                         log::debug!("Opening devtools for main window");
                         window.open_devtools();
                    },
                    None => log::warn!("Could not get main window handle to open devtools.")
                 }
            }
            #[cfg(target_os = "macos")]
            {
                use tauri::menu::{Menu, Submenu, MenuItem, PredefinedMenuItem};
                let app_handle = app_mut_ref.handle();
                
                // 1. App Menu (Harvey)
                let about_item = MenuItem::with_id(app_handle, "about_harvey", "About Harvey", true, None::<&str>)?;
                let configurations_item = MenuItem::with_id(app_handle, "configurations_harvey", "Configurations", true, Some("CmdOrCtrl+,"))?;
                let sep = PredefinedMenuItem::separator(app_handle)?;
                let quit = PredefinedMenuItem::quit(app_handle, None)?;
                
                let app_menu = Submenu::with_items(
                    app_handle,
                    "Harvey",
                    true,
                    &[&about_item, &configurations_item, &sep, &quit],
                )?;

                // 2. Edit Menu
                let undo = PredefinedMenuItem::undo(app_handle, None)?;
                let redo = PredefinedMenuItem::redo(app_handle, None)?;
                let cut = PredefinedMenuItem::cut(app_handle, None)?;
                let copy = PredefinedMenuItem::copy(app_handle, None)?;
                let paste = PredefinedMenuItem::paste(app_handle, None)?;
                let select_all = PredefinedMenuItem::select_all(app_handle, None)?;
                let sep2 = PredefinedMenuItem::separator(app_handle)?;
                
                let edit_menu = Submenu::with_items(
                    app_handle,
                    "Edit",
                    true,
                    &[&undo, &redo, &sep2, &cut, &copy, &paste, &select_all],
                )?;

                // 3. Window Menu
                let minimize = PredefinedMenuItem::minimize(app_handle, None)?;
                // let zoom = PredefinedMenuItem::zoom(app_handle, None)?; // Removed due to error
                let close = PredefinedMenuItem::close_window(app_handle, None)?;
                let sep3 = PredefinedMenuItem::separator(app_handle)?;

                let window_menu = Submenu::with_items(
                    app_handle,
                    "Window",
                    true,
                    &[&minimize, &sep3, &close],
                )?;

                // 4. Help Menu
                let help_center = MenuItem::with_id(app_handle, "help_center", "Help Center", true, None::<&str>)?;
                let license_item = MenuItem::with_id(app_handle, "view_license", "License", true, None::<&str>)?;
                let credits_item = MenuItem::with_id(app_handle, "view_credits", "Credits", true, None::<&str>)?;
                let version_item = MenuItem::with_id(app_handle, "view_version", "Version", true, None::<&str>)?;

                let help_menu = Submenu::with_items(
                    app_handle,
                    "Help",
                    true,
                    &[&help_center, &license_item, &credits_item, &version_item],
                )?;

                let menu = Menu::with_items(app_handle, &[&app_menu, &edit_menu, &window_menu, &help_menu])?;
                app_mut_ref.set_menu(menu)?;
            
            use tauri::{Emitter};
            use tauri_plugin_global_shortcut::{Shortcut, Modifiers, Code, ShortcutEvent, ShortcutState, GlobalShortcutExt};

            // log::info!("[SETUP] Preparing to set up global shortcuts..."); // Removed
            let app_handle_clone = app_mut_ref.handle().clone(); // Clone app handle for the handler

            // Define the shortcuts
            let f7_shortcut = Shortcut::new(Some(Modifiers::empty()), Code::F7);
            let f8_shortcut = Shortcut::new(Some(Modifiers::empty()), Code::F8);
            let f9_shortcut = Shortcut::new(Some(Modifiers::empty()), Code::F9);
            // log::info!("[SETUP] Defined F7, F8, F9 shortcut objects."); // Removed

            // Build the plugin with a general handler
            let global_shortcut_plugin_instance = tauri_plugin_global_shortcut::Builder::new().with_handler(
                move |_,shortcut_arg: &Shortcut,event_details: ShortcutEvent| {

                    if event_details.state == ShortcutState::Pressed {
                        // log::info!("[HANDLER] Global shortcut pressed: shortcut_arg: {:?}, state: {:?}", shortcut_arg, event_details.state); // Removed
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
            // log::info!("[SETUP] Global shortcut plugin builder created with handler."); // Removed

            // Register the plugin instance with the app
            app_mut_ref.handle().plugin(global_shortcut_plugin_instance)?;
            // log::info!("[SETUP] Global shortcut plugin registered with app handle."); // Removed

            // Explicitly register each shortcut
            app_mut_ref.global_shortcut().register(f7_shortcut)?;
            // log::info!("[SETUP] F7 shortcut registered."); // Removed
            app_mut_ref.global_shortcut().register(f8_shortcut)?;
            // log::info!("[SETUP] F8 shortcut registered."); // Removed
            app_mut_ref.global_shortcut().register(f9_shortcut)?;
            log::info!("Global media shortcuts (F7, F8, F9) registration process completed successfully."); // Added concise summary
            }

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
            welcome::commands::get_local_translation_models,
            welcome::commands::delete_model,
            welcome::commands::download_model_command,
            welcome::commands::download_faster_whisper_model_command,
            welcome::commands::download_translation_model_command,
            welcome::commands::cancel_download_command,
            welcome::commands::fetch_available_models_command,
            welcome::commands::change_download_location_and_move_models,
            welcome::commands::get_platform_info,
            welcome::commands::is_cuda_available_command,
            welcome::commands::check_python_libraries_installed,
            welcome::commands::install_python_libraries,
            welcome::python_env::is_ffmpeg_installed,
            welcome::python_env::delete_virtual_env,
            welcome::python_env::list_venv_lib_contents,
            welcome::hf_auth::check_hf_auth_status,
            welcome::hf_auth::save_hf_auth_token,
            welcome::diarization::check_diarization_model_access,
            welcome::diarization::check_gated_model_access,
            welcome::diarization::download_diarization_model,
            welcome::diarization::delete_diarization_model,
            welcome::diarization::get_diarization_cache_path,
            welcome::status::check_config_status,
            
            welcome::commands::set_selected_translation_family,
            welcome::commands::get_selected_translation_family,
            welcome::commands::set_selected_transcription_engine,
            welcome::commands::get_selected_transcription_engine,
            welcome::commands::is_ctranslate2_installed,
            welcome::commands::is_faster_whisper_dependencies_installed,
            welcome::commands::install_faster_whisper_dependencies_command,
            welcome::commands::is_whisper_cpp_installed,
            welcome::commands::install_whisper_cpp_dependencies_command,
            welcome::commands::get_dependency_check_errors,
            welcome::commands::get_theme_preference,
            welcome::commands::set_theme_preference,
            welcome::commands::get_advanced_translation_config,
            welcome::commands::set_advanced_translation_config,
            welcome::commands::get_advanced_transcription_config, // Added
            welcome::commands::set_advanced_transcription_config, // Added
            welcome::commands::set_menu_context, // Added

            // --- Project view CORE commands ---
            projectview::core_commands::load_project_data,
            projectview::core_commands::import_media,
            projectview::core_commands::rename_project_item,
            projectview::core_commands::delete_project_item,
            projectview::core_commands::reveal_in_file_explorer,
            // --- Project view GROUP commands (added) ---
            projectview::core_commands::create_new_group,
            projectview::core_commands::get_project_groups,
            projectview::core_commands::add_file_to_existing_group,
            projectview::core_commands::get_groups_for_file_asset,
            projectview::core_commands::remove_file_from_group,
            projectview::core_commands::get_group_contents,
            projectview::core_commands::update_group_details,
            projectview::core_commands::rename_project_group, // Added command
            projectview::core_commands::delete_project_group, // Added command

            // --- Project view METADATA commands (asset_metadata table) ---
            projectview::metadata_commands::get_asset_metadata_command,
            projectview::metadata_commands::update_asset_metadata_command,
            projectview::attachment_commands::upload_attachment,
            // --- Custom Field Definition Commands ---
            projectview::metadata_commands::create_custom_field_definition_command,
            projectview::metadata_commands::get_all_custom_field_definitions_command,
            projectview::metadata_commands::delete_custom_field_definition_command, // Added this line

            // --- Tag Handler Commands ---
            projectview::tag_handler::add_tag,
            projectview::tag_handler::get_all_tags,
            projectview::tag_handler::get_tag_info,
            projectview::tag_handler::update_tag,
            projectview::tag_handler::delete_tag,
            projectview::tag_handler::remove_tag_globally,
            projectview::tag_handler::rename_tag_in_highlights,
            projectview::tag_handler::remove_tag_from_highlight,
            projectview::tag_handler::create_tag_group,
            projectview::tag_handler::get_tag_groups,
            projectview::tag_handler::update_tag_group,
            projectview::tag_handler::delete_tag_group,
            projectview::tag_handler::get_tag_group_info,

            // --- Project view TRANSCRIPTION commands ---
            projectview::transcription_commands::load_transcript_json,
            projectview::transcription_commands::save_transcript_json,
            projectview::transcription_commands::trim_media,
            projectview::transcription_commands::save_speaker_config,
            projectview::transcription_commands::list_subtitle_files_command,
            projectview::transcription_commands::convert_srt_to_vtt_command,
            projectview::transcription_commands::convert_ass_to_vtt_command,
            projectview::transcription_commands::transcribe_media_command, // <--- ADD THIS LINE
            projectview::transcription_commands::start_live_transcription,
            projectview::transcription_commands::stop_live_transcription,
            projectview::transcription_commands::load_media_additional_parameters,
            projectview::transcription_commands::save_media_additional_parameters,
            
            // --- Project view TRANSLATION commands ---
            projectview::translation_commands::translate_transcript_command,
            projectview::translation_commands::translate_document_command,
            projectview::translation_commands::translate_imported_transcript_command,
            projectview::translation_commands::cancel_translation_command,


            // --- Project view DOCUMENT/DATA commands ---
            projectview::document_commands::save_note_json,
            projectview::document_commands::load_note_json,
            projectview::document_commands::load_document_metadata,
            projectview::document_commands::save_document_and_update_xml,
            projectview::document_commands::read_file_content,
            projectview::document_commands::delete_temporary_file,
            projectview::document_commands::get_unique_document_path,
            projectview::document_commands::create_new_document,
            
            // --- Project view PDF ANNOTATION commands --- ADDED
            projectview::pdf_annotation_handler::load_pdf_annotations,
            projectview::pdf_annotation_handler::save_pdf_annotations,
            // --- END ADDED ---

            // --- Project view LEXICAL HIGHLIGHT commands ---
            projectview::lexical_highlight_handler::load_lexical_highlights,
            projectview::lexical_highlight_handler::save_lexical_highlights,
            projectview::lexical_highlight_handler::delete_lexical_highlights,
            projectview::lexical_highlight_handler::save_highlight_changes,

            // --- Document Import Process Command ---
            projectview::document_handler::import_document,
            projectview::export_handler::export_transcript_to_docx,
            projectview::export_handler::export_transcript_to_srt,
            projectview::export_handler::export_transcript_to_vtt,
            projectview::export_handler::export_transcript_to_markdown,
            projectview::export_handler::export_transcript_to_ass, // Added ASS export command
            projectview::export_handler::export_document_to_docx,
            projectview::export_handler::export_document_to_markdown,
            projectview::export_handler::export_document_to_txt,

            // --- Project view transcription PROCESS commands ---
            projectview::local_handler::transcription::run_transcription,
            // projectview::local_handler::transcription::cancel_transcription, // Moved
            projectview::transcription_commands::cancel_transcription, // New location
            

            // --- Project view TABLE commands ---
            projectview::table_handler::create_new_table,
            projectview::table_handler::save_table_schema,
            projectview::table_handler::load_table_schema,
            projectview::table_handler::import_table_file,
            projectview::table_handler::load_table_data,
            projectview::table_handler::set_table_headers,
            projectview::table_handler::save_table_data,
            projectview::table_handler::rename_table_header,
            projectview::table_handler::delete_table_column,
            projectview::table_handler::save_table_styles,
            projectview::table_handler::load_table_styles,
            projectview::table_handler::export_table_to_csv, // Added
            projectview::table_handler::export_table_to_xlsx, // Added
            projectview::core_commands::save_table_layout_prefs,
            projectview::core_commands::load_table_layout_prefs,

            // --- Project view IMAGE commands ---
            projectview::image_handler::import_image_file,
            projectview::image_handler::load_image_annotations,
            projectview::image_handler::save_image_annotations,
            projectview::image_handler::save_screenshot,

            // --- Project view TRANSCRIPT IMPORT command ---
            projectview::transcription_handler::import_word_transcript,
            projectview::transcription_handler::save_imported_transcript_and_update_xml

        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}