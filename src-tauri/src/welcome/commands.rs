// src-tauri/src/welcome/commands.rs

use crate::welcome::config::{
    ModelInfo, ProjectInfo, add_or_update_project_in_config, read_config, write_config, // Keep these config functions
    PROJECT_FILE_EXTENSION, CommandError, get_default_download_location,
};
use crate::utils::canonicalize_path;
use crate::DownloadCancellationState;
use chrono::{Utc, DateTime};
use log; // Use log crate
use quick_xml::{Reader, Writer, events::{Event, BytesText}};
use quick_xml::de::from_str;
use serde::{Deserialize};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{BufReader, Cursor, Write},
    path::{PathBuf, Path},
    sync::{Arc, atomic::{AtomicBool, Ordering}},
};
use tauri::{AppHandle, command, Emitter, State, Manager, Runtime};
use tauri_plugin_shell::ShellExt;
use uuid::Uuid; // Added for UUID generation
use crate::projectview::db_handler; // Added for DB operations
use crate::projectview::shared_types::ProjectXml; // For parsing project_uuid
#[cfg(not(target_os = "windows"))]
use tauri_plugin_opener::OpenerExt;
use reqwest;
use futures_util::StreamExt;

use crate::welcome::python_env;
use walkdir::WalkDir;

// --- Structs for Translation Model Download ---
#[derive(Clone, serde::Serialize)]
struct TranslationDownloadProgress {
  model_name: String,
  file_name: String,
  downloaded_bytes: u64,
  total_bytes: Option<u64>,
}

#[derive(Clone, serde::Serialize)]
struct TranslationErrorPayload {
  model_name: String,
  error_message: String,
}

#[command]
pub async fn is_ctranslate2_installed<R: Runtime>(app: AppHandle<R>) -> Result<bool, CommandError> {
    let shell = app.shell();
    let python_path = python_env::get_python_path()?;
    let env_path = python_env::get_env_path()?;
    
    // Prepare Windows PATH once
    let windows_path_env: Option<String> = if cfg!(target_os = "windows") {
        let env_bin_path = env_path.join("Library").join("bin");
        if env_bin_path.exists() {
            let existing_path = std::env::var("PATH").unwrap_or_default();
            Some(format!("{};{}", env_bin_path.to_string_lossy(), existing_path))
        } else {
            None
        }
    } else {
        None
    };

    python_env::check_package_installed(&shell, &python_path, "ctranslate2", &windows_path_env, &env_path).await
}

// --- Translation Model Download Command ---
#[command]
pub async fn download_translation_model_command(
    app: AppHandle,
    model_info: ModelInfo, // Re-using ModelInfo, `download_url` is the repo URL
    download_location: String,
) -> Result<(), CommandError> {
    log::info!("CMD: download_translation_model: {} (family: {:?}) -> {}", model_info.name, model_info.family, download_location);
    let model_name = model_info.name.clone();
    
    // Determine target directory based on family
    let family = model_info.family.as_deref().unwrap_or("helsinki");
    let org_dir = if family == "nllb" { "facebook" } else { "helsinki-nlp" };
    
    let target_dir = PathBuf::from(&download_location).join("translation").join(org_dir);
    let target_dir_str = target_dir.to_string_lossy().to_string();

    if download_location.trim().is_empty() {
        return Err(CommandError::from(format!("Download location is empty for '{}'.", model_name)));
    }
    if !target_dir.exists() {
        log::info!("Target directory {:?} does not exist. Creating...", target_dir);
        fs::create_dir_all(&target_dir)?;
    } else if !target_dir.is_dir() {
        return Err(CommandError::from(format!("Target path {:?} is not a directory.", target_dir)));
    }

    let python_path = python_env::get_python_path()?;
    let window = app.get_webview_window("main").unwrap();

    // 1. Check/Install CTranslate2 if it's a Helsinki model (optimization candidate)
    // Actually we want to optimize ALL models eventually, but user specifically asked for Helsinki first.
    // For now let's do it for Helsinki as requested.
    if family == "helsinki" {
        let ct2_installed = is_ctranslate2_installed(app.clone()).await.unwrap_or(false);
        if !ct2_installed {
            log::info!("CTranslate2 not found. Installing...");
            window.emit("translation-download-log", serde_json::json!({ "model_name": &model_name, "log_line": "CTranslate2 is missing. Installing it now for faster translations..." })).unwrap();
            python_env::install_pip_packages(&app, &app.shell(), vec!["ctranslate2~=4.5.0"], "translation-download-log").await?;
            window.emit("translation-download-log", serde_json::json!({ "model_name": &model_name, "log_line": "CTranslate2 installed successfully." })).unwrap();
        }
    }

    // 2. Download model weights
    let script_path = app.path().resource_dir().unwrap().join("scripts/download_translation_model.py");

    let token_path = app.path().app_config_dir().unwrap().join("hf_token");
    let token = if token_path.exists() {
        fs::read_to_string(token_path).unwrap_or_default()
    } else {
        String::new()
    };

    let (mut rx, _child) = app.shell()
        .command(python_path.to_str().unwrap())
        .args(&[script_path.to_str().unwrap(), &model_name, &target_dir_str, &token])
        .env("HF_HUB_DISABLE_PROGRESS_BARS", "1")
        .spawn()
        .map_err(|e| format!("Failed to spawn python script: {}", e))?;

    window.emit("translation-download-start", &model_name).unwrap();

    let mut success = false;
    while let Some(event) = rx.recv().await {
        match event {
            tauri_plugin_shell::process::CommandEvent::Stdout(line) => {
                let line_str = String::from_utf8_lossy(&line).to_string();
                log::info!("[Python] {}", &line_str);
                window.emit("translation-download-log", serde_json::json!({ "model_name": &model_name, "log_line": &line_str })).unwrap();
            }
            tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                let line_str = String::from_utf8_lossy(&line).to_string();
                log::error!("[Python] {}", &line_str);
                window.emit("translation-download-log", serde_json::json!({ "model_name": &model_name, "log_line": &line_str })).unwrap();
            }
            tauri_plugin_shell::process::CommandEvent::Terminated(payload) => {
                log::info!("Translation download process for '{}' terminated with code: {:?}", &model_name, payload.code);
                if payload.code == Some(0) {
                    success = true;
                } else {
                    window.emit("translation-download-error", serde_json::json!({ "model_name": &model_name, "error_message": "Download script failed" })).unwrap();
                }
                break;
            }
            _ => {}
        }
    }

    if !success {
        window.emit("translation-download-finished", ()).unwrap();
        return Err(CommandError::Message("Translation model download failed.".to_string()));
    }

    // 3. Optimize model for CTranslate2
    window.emit("translation-download-log", serde_json::json!({ "model_name": &model_name, "log_line": "Optimizing model for faster CPU inference..." })).unwrap();
    
    let optimize_script = app.path().resource_dir().unwrap().join("scripts/optimize_translation_model.py");
    let folder_name = format!("models--{}", model_name.replace('/', "--"));
    let model_path = target_dir.join(&folder_name);
    let output_path = model_path.join("ct2_optimized");

    let output = app.shell()
        .command(python_path.to_str().unwrap())
        .args(&[optimize_script.to_str().unwrap(), model_path.to_str().unwrap(), output_path.to_str().unwrap()])
        .output()
        .await?;

    if output.status.success() {
        window.emit("translation-download-log", serde_json::json!({ "model_name": &model_name, "log_line": "Optimization complete." })).unwrap();
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::error!("Optimization failed: {}", stderr);
        window.emit("translation-download-log", serde_json::json!({ "model_name": &model_name, "log_line": format!("Optimization failed (non-critical): {}", stderr) })).unwrap();
    }

    window.emit("translation-download-complete", &model_name).unwrap();
    window.emit("translation-download-finished", ()).unwrap();

    if success {
        log::info!("Translation model '{}' downloaded successfully. Updating config.", &model_name);
        match || -> Result<(), CommandError> {
            let mut config = read_config()?;
            let mut downloaded_model_info = model_info.clone();
            // Store the BASE download location in config, consistent with other models
            downloaded_model_info.download_location = Some(download_location.clone());

            if let Some(idx) = config.downloaded_models.iter().position(|m| m.name == downloaded_model_info.name) {
                log::info!("Model '{}' already in config. Updating.", &model_name);
                config.downloaded_models[idx] = downloaded_model_info;
            } else {
                log::info!("Adding new model '{}' to config.", &model_name);
                config.downloaded_models.push(downloaded_model_info);
            }
            write_config(&config)?;
            log::info!("Config updated successfully for '{}'.", &model_name);
            Ok(())
        }() {
            Ok(_) => Ok(()),
            Err(e) => {
                log::error!("Failed to update config for translation model '{}': {}", &model_name, e);
                Err(CommandError::from(format!("Model downloaded but failed to save configuration: {}", e)))
            }
        }
    } else {
        Err(CommandError::Message("Translation model download failed.".to_string()))
    }
}

#[command]
pub async fn get_local_translation_models() -> Result<Vec<ModelInfo>, CommandError> {
    log::info!("CMD: get_local_translation_models");
    let base_download_dir = PathBuf::from(get_download_location().await?);
    let mut models = Vec::new();

    let families = [
        ("helsinki", "helsinki-nlp"),
        ("nllb", "facebook"),
    ];

    for (family_id, sub_dir) in families {
        let download_dir = base_download_dir.join("translation").join(sub_dir);
        if !download_dir.exists() {
            continue;
        }

        for entry in fs::read_dir(download_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(folder_name) = path.file_name().and_then(|n| n.to_str()) {
                    if folder_name.starts_with("models--") {
                        // The folder name is like `models--Helsinki-NLP--opus-mt-ja-en`
                        // We need to convert it back to `Helsinki-NLP/opus-mt-ja-en`
                        let model_name = folder_name
                            .strip_prefix("models--")
                            .unwrap_or(folder_name)
                            .replace("--", "/");

                        // Calculate size
                        let size_bytes: u64 = WalkDir::new(&path)
                            .into_iter()
                            .filter_map(|e| e.ok())
                            .filter_map(|e| e.metadata().ok())
                            .filter(|m| m.is_file())
                            .map(|m| m.len())
                            .sum();

                        let size_str = if size_bytes > 0 {
                            const KB: u64 = 1024;
                            const MB: u64 = KB * 1024;
                            const GB: u64 = MB * 1024;
                            if size_bytes >= GB {
                                Some(format!("{:.1} GiB", size_bytes as f64 / GB as f64))
                            } else if size_bytes >= MB {
                                Some(format!("{:.1} MiB", size_bytes as f64 / MB as f64))
                            } else {
                                Some(format!("{:.1} KiB", size_bytes as f64 / KB as f64))
                            }
                        } else {
                            None
                        };

                        models.push(ModelInfo {
                            name: model_name,
                            family: Some(family_id.to_string()),
                            language: None,
                            size: size_str,
                            description: None,
                            download_location: Some(path.to_string_lossy().into_owned()),
                            download_url: None,
                        });
                    }
                }
            }
        }
    }

    log::info!("Found {} local translation models.", models.len());
    Ok(models)
}

#[command]
pub async fn set_selected_translation_family(family: String) -> Result<(), CommandError> {
    log::info!("CMD: set_selected_translation_family: {}", family);
    let mut config = read_config()?;
    config.selected_translation_family = Some(family);
    write_config(&config)?;
    Ok(())
}

#[command]
pub async fn get_selected_translation_family() -> Result<Option<String>, CommandError> {
    let config = read_config()?;
    Ok(config.selected_translation_family)
}

#[command]
pub async fn get_platform_info() -> Result<String, CommandError> {
    Ok(tauri::utils::platform::target_triple()
        .unwrap_or_else(|_| "unknown".to_string()))
}

#[derive(Deserialize)]
struct HuggingFaceApiResponse {
    siblings: Vec<HuggingFaceApiFile>,
}

#[derive(Deserialize)]
struct HuggingFaceApiFile {
    rfilename: String,
}

// --- Structs (DownloadProgress, ErrorPayload) - Unchanged ---
#[derive(Clone, serde::Serialize)]
struct DownloadProgress {
  model_name: String,
  downloaded_bytes: u64,
  total_bytes: Option<u64>,
}
#[derive(Clone, serde::Serialize)]
struct ErrorPayload {
  model_name: String,
  error_message: String,
}


// --- Project Commands (Unchanged - Omitted for brevity) ---
#[command] pub async fn load_recent_projects() -> Result<Vec<ProjectInfo>, CommandError> { /* ... */ log::info!("---- load_recent_projects: Start ----"); let mut config = read_config()?; let original_project_count = config.projects.len(); let existing_paths: HashSet<String> = config.projects .iter() .filter(|p| PathBuf::from(&p.path).exists()) .map(|p| p.path.clone()) .collect(); let mut updated_config = false; if existing_paths.len() < original_project_count { config.projects.retain(|p| existing_paths.contains(&p.path)); log::info!("load_recent_projects: Removed {} missing projects.", original_project_count - existing_paths.len()); updated_config = true; } log::info!("load_recent_projects: Found {} valid projects.", config.projects.len()); if updated_config { write_config(&config)?; log::info!("load_recent_projects: Updated config.xml after removing missing projects."); } config.projects.sort_by(|a, b| b.last_opened_ts.cmp(&a.last_opened_ts)); log::info!("---- load_recent_projects: End ----"); Ok(config.projects) }
#[command]
pub async fn create_project(name: String, parent_location: String, overwrite: Option<bool>) -> Result<String, CommandError> {
    let should_overwrite = overwrite.unwrap_or(false);
    log::info!("---- create_project: Start. Name='{}', Location='{}', Overwrite={} ----", name, parent_location, should_overwrite);

    let project_uuid = Uuid::new_v4().to_string(); // Generate UUID
    log::info!("create_project: Generated Project UUID: {}", project_uuid);

    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        log::error!("create_project: Error - Empty name.");
        return Err("Project name cannot be empty.".into());
    }

    let parent_path = PathBuf::from(&parent_location);
    let project_dir_path = parent_path.join(trimmed_name);
    log::info!("create_project: Target project dir: {:?}", project_dir_path);

    if project_dir_path.exists() {
        if should_overwrite {
            log::warn!("create_project: Target directory exists and overwrite is true. Attempting deletion...");
            delete_project_folder_internal(&project_dir_path)?;
            log::info!("create_project: Existing directory deleted successfully.");
            let old_xml_path_str = project_dir_path.join(format!("{}.{}", trimmed_name, PROJECT_FILE_EXTENSION)).to_string_lossy().to_string();
            // Note: If the project existed in the new DB, its record is not explicitly deleted here.
            // `add_project_to_db` uses ON CONFLICT for ID, but root_path/xml_path are UNIQUE.
            // If a new UUID is generated, but paths conflict with an *old* entry not yet cleaned from DB,
            // the db_handler::add_project_to_db call later might fail.
            // This is a potential edge case if overwriting projects that were previously in the new DB.
            remove_project_from_config_internal(&old_xml_path_str)?;
        } else {
            let error_msg = format!("Directory '{}' already exists in the selected location.", trimmed_name);
            log::error!("create_project: Error - {}", error_msg);
            return Err(CommandError::Message(format!("E_DIR_EXISTS:{}", error_msg)));
        }
    } else if project_dir_path.is_file() {
        log::error!("create_project: Error - Target path is a file.");
        return Err(CommandError::from(format!("A file named '{}' already exists in the selected location.", trimmed_name)));
    }

    fs::create_dir_all(&project_dir_path)?;
    log::info!("create_project: Created directory: {:?}", project_dir_path);

    let xml_file_name = format!("{}.{}", trimmed_name, PROJECT_FILE_EXTENSION);
    let xml_path = project_dir_path.join(xml_file_name);
    log::info!("create_project: Creating project file: {:?}", xml_path);

    let escaped_name = quick_xml::escape::escape(trimmed_name);
    // Add project_uuid to the XML content
    let project_xml_content = format!(
        "<project>\n  <name>{}</name>\n  <project_uuid>{}</project_uuid>\n  <mediaFiles></mediaFiles>\n</project>",
        escaped_name, project_uuid
    );

    fs::write(&xml_path, project_xml_content)?;
    log::info!("create_project: Wrote project XML content with UUID.");

    // Canonicalize paths *after* file/dir creation
    let absolute_xml_path = canonicalize_path(&xml_path)?
        .to_str()
        .ok_or("Failed to convert project XML path to string")?
        .to_string();
    log::info!("create_project: Canonicalized XML path: {}", absolute_xml_path);

    let absolute_root_path = canonicalize_path(&project_dir_path)?
        .to_str()
        .ok_or("Failed to convert project root path to string")?
        .to_string();
    log::info!("create_project: Canonicalized root path: {}", absolute_root_path);

    // Call add_project_to_db BEFORE add_or_update_project_in_config
    // This ensures the project is in our primary DB before potentially failing on legacy config.
    // Error handling: if DB call fails, we log it but proceed to update config.xml.
    // This could be changed to a hard error if DB persistence is paramount.
    match db_handler::add_project_to_db(&project_uuid, trimmed_name, &absolute_root_path, &absolute_xml_path) {
        Ok(_) => log::info!("create_project: Added project to DB successfully. UUID: {}", project_uuid),
        Err(e) => {
            log::error!("create_project: CRITICAL - Failed to add project to DB: {}. Project files were created but metadata might be missing.", e);
            // Depending on product requirements, this might be a hard error:
            // return Err(CommandError::Message(format!("Failed to save critical project metadata: {}. Please try creating the project again.", e)));
        }
    }

    let now = Utc::now();
    let project_info = ProjectInfo { // This struct is for the legacy config.xml
        name: trimmed_name.to_string(),
        path: absolute_xml_path.clone(), // XML path
        created_ts: now,
        last_opened_ts: now,
    };

    add_or_update_project_in_config(project_info)?;
    log::info!("create_project: Added/Updated project in config.xml");

    log::info!("---- create_project: End ----");
    Ok(absolute_xml_path)
}
#[command]
#[allow(unused_variables)]
pub async fn locate_in_finder(app: AppHandle, project_xml_path: String) -> Result<(), CommandError> {
    log::info!("---- locate_in_finder: Start. Path='{}' ----", project_xml_path);

    let path = PathBuf::from(project_xml_path);
    if !path.exists() {
        log::error!("locate_in_finder: Error - Project file not found.");
        return Err("Project file not found.".into());
    }
    let dir_to_open = path.parent().ok_or("Could not get parent directory from path.")?;
    log::info!("locate_in_finder: Directory to open: {:?}", dir_to_open);
    let absolute_dir_path = dunce::canonicalize(dir_to_open)?;

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        log::info!("locate_in_finder: Attempting to open directory with explorer.exe: {:?}", absolute_dir_path);
        Command::new("explorer.exe")
            .arg(absolute_dir_path.to_string_lossy().as_ref())
            .spawn()
            .map_err(|e| CommandError::from(format!("Failed to open explorer: {}", e)))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        // For non-Windows (e.g., macOS), use file:// URL. Ensure forward slashes.
        let dir_url = format!("file://{}", absolute_dir_path.to_string_lossy().replace('\\', "/"));
        log::info!("locate_in_finder: Attempting to open URL: {}", dir_url);
        app.opener().open_url(dir_url, None::<String>).map_err(|e| {
            log::error!("locate_in_finder: Error opening URL: {}", e);
            CommandError::from(format!("Failed to open project location: {}", e))
        })?;
    }
    log::info!("---- locate_in_finder: End ----");
    Ok(())
}
#[command]
pub async fn rename_project(project_xml_path: String, new_name: String) -> Result<(), CommandError> {
    log::info!("---- rename_project: Start. Path='{}', NewName='{}' ----", project_xml_path, new_name);
    let trimmed_new_name = new_name.trim();
    if trimmed_new_name.is_empty() {
        log::error!("rename_project: Error - New name is empty.");
        return Err("New project name cannot be empty.".into());
    }
    if trimmed_new_name.contains('/') || trimmed_new_name.contains('\\') || trimmed_new_name.contains(':') {
        log::error!("rename_project: Error - New name contains invalid chars.");
        return Err("New project name cannot contain path separators or colons.".into());
    }
    log::info!("rename_project: Validating old path...");
    let old_xml_path = PathBuf::from(&project_xml_path);
    if !old_xml_path.exists() {
        log::error!("rename_project: Error - Original XML file not found at '{}'.", project_xml_path);
        return Err("Original project XML file not found.".into());
    }
    log::info!("rename_project: Old path exists: {:?}", old_xml_path);
    let old_project_dir = old_xml_path.parent().ok_or("Could not get parent directory from old path.")?;
    log::info!("rename_project: Old project dir: {:?}", old_project_dir);
    let base_dir = old_project_dir.parent().ok_or("Could not get base directory.")?;
    log::info!("rename_project: Base dir: {:?}", base_dir);
    let new_project_dir = base_dir.join(trimmed_new_name);
    log::info!("rename_project: New project dir target: {:?}", new_project_dir);
    let new_xml_filename = format!("{}.{}", trimmed_new_name, PROJECT_FILE_EXTENSION);
    let new_xml_path = new_project_dir.join(&new_xml_filename);
    log::info!("rename_project: New XML path target: {:?}", new_xml_path);
    let new_xml_path_str = new_xml_path.to_str().ok_or("Failed to convert new project XML path to string")?.to_string();
    log::info!("rename_project: New XML path string: {}", new_xml_path_str);
    log::info!("rename_project: Checking if target directory exists and differs...");
    if new_project_dir.exists() && new_project_dir != old_project_dir {
        log::info!("rename_project: Target directory '{}' exists and is different. Checking if same entry...", new_project_dir.display());
        let are_same_entry = || -> std::io::Result<bool> {
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                log::info!("rename_project: Comparing inodes (Unix)...");
                let meta1 = fs::metadata(&old_project_dir)?;
                let meta2 = fs::metadata(&new_project_dir)?;
                Ok(meta1.dev() == meta2.dev() && meta1.ino() == meta2.ino())
            }
            #[cfg(windows)]
            {
                log::info!("rename_project: Comparing canonical paths (Windows)...");
                let canon1 = canonicalize_path(&old_project_dir).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
                let canon2 = canonicalize_path(&new_project_dir).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
                Ok(canon1 == canon2)
            }
            #[cfg(not(any(unix, windows)))]
            {
                log::info!("rename_project: Comparing canonical paths (Other OS)...");
                let canon1 = canonicalize_path(&old_project_dir).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
                let canon2 = canonicalize_path(&new_project_dir).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
                Ok(canon1 == canon2)
            }
        };
        match are_same_entry() {
            Ok(true) => {
                log::info!("rename_project: Target is the same directory (case change?). Proceeding.");
            }
            Ok(false) => {
                log::error!("rename_project: Error - A different directory with name '{}' already exists.", trimmed_new_name);
                return Err(format!("A different directory with name '{}' exists.", trimmed_new_name).into());
            }
            Err(e) => {
                log::error!("rename_project: Error - Could not compare existing directory effectively: {}", e);
                return Err(format!("Could not reliably check if '{}' is the same directory.", trimmed_new_name).into());
            }
        }
    } else {
        log::info!("rename_project: Target directory doesn't exist or is the same as old one.");
    }
    if old_project_dir != new_project_dir {
        log::info!("rename_project: === Attempting FOLDER rename: {:?} -> {:?} ===", old_project_dir, new_project_dir);
        fs::rename(&old_project_dir, &new_project_dir).map_err(|e| {
            log::error!("rename_project: *** FOLDER RENAME FAILED: {} ***", e);
            CommandError::from(format!("Failed to rename project folder: {}", e))
        })?;
        log::info!("rename_project: === FOLDER rename successful. ===");
    } else {
        log::info!("rename_project: Folder name same, skipping folder rename.");
    }
    let original_xml_filename_osstr = old_xml_path.file_name().ok_or("Could not get old XML filename.")?;
    let xml_path_in_potentially_new_dir = new_project_dir.join(original_xml_filename_osstr);
    log::info!("rename_project: Path of XML inside new/current dir: {:?}", xml_path_in_potentially_new_dir);
    if xml_path_in_potentially_new_dir.exists() && xml_path_in_potentially_new_dir != new_xml_path {
        log::info!("rename_project: === Attempting XML FILE rename: {:?} -> {:?} ===", xml_path_in_potentially_new_dir, new_xml_path);
        fs::rename(&xml_path_in_potentially_new_dir, &new_xml_path).map_err(|e| {
            log::error!("rename_project: *** XML FILE RENAME FAILED: {} ***", e);
            CommandError::from(format!("Failed to rename project XML file: {}", e))
        })?;
        log::info!("rename_project: === XML FILE rename successful. ===");
    } else if !xml_path_in_potentially_new_dir.exists() {
        log::warn!("rename_project: Expected XML {:?} not found after folder rename. Checking target {:?}...", xml_path_in_potentially_new_dir, new_xml_path);
        if !new_xml_path.exists() {
            log::error!("rename_project: XML file missing after rename attempt. Target {:?} does not exist.", new_xml_path);
            return Err("Project XML file missing after rename.".into());
        } else {
            log::info!("rename_project: XML file already exists at target {:?}. No file rename needed.", new_xml_path);
        }
    } else {
        log::info!("rename_project: XML filename same or matches target. Skipping file rename.");
    }
    log::info!("rename_project: === Attempting read XML from final path: {:?} ===", new_xml_path);
    let original_xml_content = fs::read(&new_xml_path).map_err(|e| {
        log::error!("rename_project: *** FAILED READ XML from {:?}: {} ***", new_xml_path, e);
        CommandError::from(format!("Failed read XML after rename: {}", e))
    })?;
    log::info!("rename_project: Read {} bytes from XML.", original_xml_content.len());
    log::info!("rename_project: === Parsing and updating XML content ===");
    let mut reader = Reader::from_reader(BufReader::new(Cursor::new(original_xml_content)));
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    let mut in_name_tag = false;
    let mut name_updated = false;
    let mut depth = 0;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag_name = e.name();
                writer.write_event(Event::Start(e.clone()))?;
                if depth == 1 && tag_name.as_ref() == b"name" {
                    in_name_tag = true;
                    writer.write_event(Event::Text(BytesText::new(trimmed_new_name)))?;
                    name_updated = true;
                }
                depth += 1;
            }
            Ok(Event::End(ref e)) => {
                depth -= 1;
                if depth == 1 && e.name().as_ref() == b"name" {
                    in_name_tag = false;
                }
                writer.write_event(Event::End(e.clone()))?;
            }
            Ok(Event::Text(_)) if in_name_tag => {}
            Ok(Event::Eof) => {
                log::info!("XML Parse - Reached EOF.");
                break;
            }
            Ok(event) => {
                writer.write_event(event)?;
            }
            Err(e) => {
                log::error!("rename_project: *** Error parsing XML: {} at pos {} ***", e, reader.buffer_position());
                return Err(format!("Error parsing XML: {}", e).into());
            }
        }
        buf.clear();
    }
    if !name_updated {
        log::error!("rename_project: Error - Did not find top-level <name> tag in XML."); /* Consider if this should be a hard error */
    }
    let updated_xml_bytes = writer.into_inner().into_inner();
    log::info!("rename_project: === XML content updated ({} bytes). Writing back to {:?} ===", updated_xml_bytes.len(), new_xml_path);
    fs::write(&new_xml_path, updated_xml_bytes).map_err(|e| {
        log::error!("rename_project: *** FAILED WRITE updated XML to {:?}: {} ***", new_xml_path, e);
        CommandError::from(format!("Failed write updated XML: {}", e))
    })?;
    log::info!("rename_project: Successfully wrote updated XML content.");
    log::info!("rename_project: === Updating config.xml entry ===");
    let mut config = read_config().map_err(|e| {
        log::error!("rename_project: *** FAILED READ config.xml: {} ***", e);
        e
    })?;
    log::info!("rename_project: Config read. Searching for project path '{}'.", project_xml_path);
    let now = Utc::now();
    let mut project_updated_in_config = false;
    if let Some(project) = config.projects.iter_mut().find(|p| p.path == project_xml_path) {
        log::info!("rename_project: Found project by original path. Updating name and path.");
        project.name = trimmed_new_name.to_string();
        project.path = new_xml_path_str.clone();
        project.last_opened_ts = now;
        project_updated_in_config = true;
    } else {
        log::error!("rename_project: Error - Could not find project in config to update using path '{}'. Config might be out of sync.", project_xml_path); /* Consider recovery or just warning */
    }
    if project_updated_in_config {
        log::info!("rename_project: Project updated in config struct. Writing config.xml...");
        write_config(&config).map_err(|e| {
            log::error!("rename_project: *** FAILED WRITE config.xml: {} ***", e);
            e
        })?;
        log::info!("rename_project: Config.xml written successfully.");
    } else {
        log::warn!("rename_project: No project entry updated in config. Skipping config write.");
    }
    log::info!("---- rename_project: End Successfully ----");
    Ok(())
}
#[command] pub async fn remove_project_from_list(project_xml_path: String) -> Result<(), CommandError> { /* ... */ log::info!("---- remove_project_from_list: Start Command. Path='{}' ----", project_xml_path); let result = remove_project_from_config_internal(&project_xml_path); log::info!("---- remove_project_from_list: End Command ----"); result }
fn remove_project_from_config_internal(project_xml_path: &str) -> Result<(), CommandError> { /* ... */ log::info!("---- remove_project_from_config_internal: Start. Path='{}' ----", project_xml_path); let mut config = read_config()?; let initial_len = config.projects.len(); config.projects.retain(|p| p.path != project_xml_path); if config.projects.len() < initial_len { log::info!("remove_project_from_config_internal: Removed project path {} from config.", project_xml_path); write_config(&config)?; log::info!("remove_project_from_config_internal: Config.xml updated."); } else { log::warn!("remove_project_from_config_internal: Project path {} not found in config. No changes.", project_xml_path); } log::info!("---- remove_project_from_config_internal: End ----"); Ok(()) }
#[command] pub async fn open_project(project_xml_path: String) -> Result<ProjectInfo, CommandError> { /* ... */ log::info!("---- open_project: Start. Path='{}' ----", project_xml_path); let path_buf = PathBuf::from(&project_xml_path); if !path_buf.exists() || !path_buf.is_file() { log::error!("open_project: Error - XML file not found: {}", project_xml_path); return Err(CommandError::from(format!("Project XML file not found: {}", project_xml_path))); } log::info!("open_project: File exists."); let mut config = read_config()?; log::info!("open_project: Config read."); let project_index = config.projects.iter().position(|p| p.path == project_xml_path); log::info!("open_project: Project index in config: {:?}", project_index); let final_project_info: ProjectInfo; let mut config_needs_write = false; if let Some(index) = project_index { log::info!("open_project: Found project in config ('{}'). Updating timestamp.", config.projects[index].name); let now = Utc::now(); if config.projects[index].last_opened_ts != now { config.projects[index].last_opened_ts = now; config_needs_write = true; log::info!("open_project: Updated last_opened_ts for '{}'.", config.projects[index].name); } else { log::info!("open_project: Timestamp current for '{}'.", config.projects[index].name); } final_project_info = config.projects[index].clone(); } else { log::info!("open_project: Project not in config, importing..."); match import_project_internal(&project_xml_path) { Ok(imported_info) => { log::info!("open_project: Import successful."); final_project_info = imported_info; config_needs_write = true; /* Config was updated by import */ }, Err(e) => { log::error!("open_project: Import failed: {}", e); return Err(e); } } } if config_needs_write { log::info!("open_project: Config needs write. Writing..."); config.projects.sort_by(|a, b| b.last_opened_ts.cmp(&a.last_opened_ts)); write_config(&config)?; log::info!("open_project: Config.xml written."); } else { log::info!("open_project: Config up-to-date."); } log::info!("---- open_project: End Successfully ----"); Ok(final_project_info) }
fn import_project_internal(project_xml_path: &str) -> Result<ProjectInfo, CommandError> { /* ... */ log::info!("---- import_project_internal: Start. Path='{}' ----", project_xml_path); let path_buf = PathBuf::from(project_xml_path); if !path_buf.exists() || !path_buf.is_file() { return Err(CommandError::from(format!("Import failed: File not found: {}", project_xml_path))); } let canonical_path = canonicalize_path(&path_buf)?; log::info!("import_project_internal: Canonical path: {:?}", canonical_path); let canonical_path_str = canonical_path.to_str().ok_or("Failed to convert path to string")?.to_string(); let xml_content = fs::read_to_string(&canonical_path)?; log::info!("import_project_internal: Read XML ({} bytes).", xml_content.len()); #[derive(Deserialize, Debug)] struct MinimalProject { name: String } let imported: MinimalProject = from_str(&xml_content).map_err(|e| CommandError::from(format!("XML deserialize error for '{}': {}. Content: '{}'", project_xml_path, e, xml_content.chars().take(100).collect::<String>() )))?; log::info!("import_project_internal: Deserialized name: {}", imported.name); let now = Utc::now(); let created_time = fs::metadata(project_xml_path)?.created().map(DateTime::<Utc>::from).unwrap_or(now); log::info!("import_project_internal: Metadata (created: {:?}).", created_time); let project_info = ProjectInfo { name: imported.name, path: canonical_path_str, created_ts: created_time, last_opened_ts: now }; log::info!("import_project_internal: Created ProjectInfo."); add_or_update_project_in_config(project_info.clone())?; log::info!("import_project_internal: Added/Updated project in config."); log::info!("---- import_project_internal: End ----"); Ok(project_info) }
#[command] pub async fn import_project(project_xml_path: String) -> Result<ProjectInfo, CommandError> { /* ... */ log::info!("---- import_project: Start Command Wrapper. Path='{}' ----", project_xml_path); let result = import_project_internal(&project_xml_path); log::info!("---- import_project: End Command Wrapper ----"); result }
#[command]
pub async fn delete_project(project_xml_path: String) -> Result<(), CommandError> {
    log::info!("---- delete_project: Start Command. Path='{}' ----", project_xml_path);
    let xml_path = PathBuf::from(&project_xml_path);

    let mut project_uuid_for_db_deletion: Option<String> = None;
    if xml_path.exists() && xml_path.is_file() {
        match fs::read_to_string(&xml_path) {
            Ok(xml_content) => {
                match quick_xml::de::from_str::<ProjectXml>(&xml_content) {
                    Ok(project_data) => {
                        if !project_data.project_uuid.is_empty() {
                            project_uuid_for_db_deletion = Some(project_data.project_uuid);
                            log::info!("delete_project: Extracted project_uuid {} for DB deletion.", project_uuid_for_db_deletion.as_ref().unwrap());
                        } else {
                            log::warn!("delete_project: project_uuid is empty in XML file {}. Cannot delete from DB by UUID.", project_xml_path);
                        }
                    }
                    Err(e) => {
                        log::warn!("delete_project: Failed to parse ProjectXml from {} to get UUID: {}. DB record might not be deleted by UUID.", project_xml_path, e);
                    }
                }
            }
            Err(e) => {
                log::warn!("delete_project: Failed to read XML file {} to get UUID: {}. DB record might not be deleted by UUID.", project_xml_path, e);
            }
        }
    } else {
        log::warn!("delete_project: Project XML file {} not found before attempting folder deletion. Cannot extract UUID for DB deletion.", project_xml_path);
    }

    // Get project directory before deleting it (if possible)
    let project_dir = match xml_path.parent() {
        Some(p) => p.to_path_buf(),
        None => {
            // This case should ideally not be reached if xml_path is valid and exists.
            // If xml_path itself is a directory (which it shouldn't be), this might also be an issue.
            log::error!("delete_project: Could not determine parent directory from project_xml_path: {}. Skipping folder deletion.", project_xml_path);
            // We might still want to proceed with DB and config removal if UUID was found.
            // For now, let's return an error as folder deletion is a key part.
            return Err(CommandError::from(format!("Could not determine project directory for {}.", project_xml_path)));
        }
    };

    // Attempt to delete the project folder
    // delete_project_folder_internal logs its own errors/warnings if path doesn't exist.
    if let Err(e) = delete_project_folder_internal(&project_dir) {
        log::error!("delete_project: Failed to delete project folder for {}: {}. Proceeding with DB and config cleanup attempts.", project_xml_path, e);
        // Continue to attempt DB and config cleanup even if folder deletion fails,
        // as the project might be in an inconsistent state.
    }

    // Attempt DB deletion if UUID was found
    if let Some(uuid) = project_uuid_for_db_deletion {
        match db_handler::delete_project_from_db(&uuid) {
            Ok(_) => log::info!("delete_project: Successfully requested deletion of project_id {} from database.", uuid),
            Err(e) => log::error!("delete_project: Failed to delete project_id {} from database: {}. This might leave an orphaned DB record.", uuid, e),
        }
    } else {
        log::warn!("delete_project: No project_uuid available, skipping database record deletion for project defined by {}.", project_xml_path);
    }

    // Remove from legacy config.xml
    // This function also logs if the project is not found, which is fine.
    if let Err(e) = remove_project_from_config_internal(&project_xml_path) {
         log::error!("delete_project: Failed to remove project from config.xml for {}: {}. The project might reappear in recent projects if config wasn't cleaned.", project_xml_path, e);
    }

    log::info!("---- delete_project: End Command ----");
    Ok(())
}
fn delete_project_folder_internal(project_dir_path: &Path) -> Result<(), CommandError> { /* ... */ log::info!("---- delete_project_folder_internal: Start. Path='{:?}' ----", project_dir_path); if project_dir_path.exists() { if project_dir_path.is_dir() { log::info!("delete_project_folder_internal: === Attempting FOLDER delete: {:?} ===", project_dir_path); fs::remove_dir_all(project_dir_path).map_err(|e| { log::error!("delete_project_folder_internal: *** FOLDER delete FAILED: {} ***", e); CommandError::from(format!("Failed delete folder '{:?}': {}", project_dir_path.display(), e)) })?; log::info!("delete_project_folder_internal: Folder deleted successfully."); } else { log::error!("delete_project_folder_internal: Path is not a directory: {:?}", project_dir_path); return Err(CommandError::from(format!("Path '{}' is not a directory.", project_dir_path.display()))); } } else { log::warn!("delete_project_folder_internal: Directory {:?} already missing.", project_dir_path); } log::info!("---- delete_project_folder_internal: End ----"); Ok(()) }

// --- Configuration Commands (Unchanged - Omitted for brevity) ---
#[command] pub async fn ensure_directory(path: String) -> Result<(), CommandError> { /* ... */ log::info!("Backend: Ensuring directory: {}", path); fs::create_dir_all(&path)?; Ok(()) }
#[command] pub async fn save_download_location(new_location: String) -> Result<(), CommandError> { /* ... */ log::info!("Backend: save_download_location: {}", new_location); let mut config = read_config()?; let trimmed = new_location.trim(); if trimmed.is_empty() { return Err("Download location empty.".into()); } config.download_location = trimmed.to_string(); write_config(&config)?; log::info!("Config: Saved download location: {}", trimmed); Ok(()) }
#[command] pub async fn get_download_location() -> Result<String, CommandError> { /* ... */ let mut config = read_config()?; let mut needs_save = false; let mut final_location = config.download_location.clone(); if final_location.trim().is_empty() { log::info!("Config: Download location empty, computing default."); final_location = get_default_download_location()?; config.download_location = final_location.clone(); needs_save = true; } let dir_path = PathBuf::from(&final_location); if !dir_path.exists() { log::info!("Config: Download directory '{}' missing. Creating...", final_location); fs::create_dir_all(&dir_path)?; } else if !dir_path.is_dir() { return Err(format!("Config Error: Download path '{}' is not a directory.", final_location).into()); } if needs_save { write_config(&config)?; log::info!("Config: Saved default download location: {}", final_location); } else { log::info!("Config: Using download location: {}", final_location); } Ok(final_location) }
#[command]
pub async fn get_downloaded_models() -> Result<Vec<ModelInfo>, CommandError> {
    let mut config = read_config()?;
    let initial_len = config.downloaded_models.len();
    
    // Automatically clean up paraphrase models if they exist in config
    config.downloaded_models.retain(|m| !m.name.contains("paraphrase"));
    
    if config.downloaded_models.len() < initial_len {
        log::info!("Cleaned up {} legacy paraphrase model(s) from config.", initial_len - config.downloaded_models.len());
        write_config(&config)?;
    }

    log::info!("Config: Returning {} downloaded models.", config.downloaded_models.len());
    Ok(config.downloaded_models)
}

#[command]
pub async fn delete_model(model_to_delete: ModelInfo) -> Result<(), CommandError> {
    log::info!("CMD: delete_model: Attempting to delete '{}' (family: {:?})", model_to_delete.name, model_to_delete.family);
    let mut config = read_config()?;
    let initial_len = config.downloaded_models.len();

    let base_location = if !config.download_location.trim().is_empty() {
        config.download_location.clone()
    } else {
        log::warn!("Config/Delete: Download location is empty, using default.");
        get_default_download_location()?
    };
    if base_location.trim().is_empty() {
        return Err(CommandError::from(format!("Cannot determine download location to delete model '{}'.", model_to_delete.name)));
    }

    // Handle the case where the model name in the config (e.g., "Helsinki-NLP/opus-mt-ja-en")
    // is different from the folder name on disk (e.g., "models--Helsinki-NLP--opus-mt-ja-en").
    let is_translation = model_to_delete.name.contains('/') || model_to_delete.family.is_some();
    
    let folder_name = if is_translation && model_to_delete.name.contains('/') {
        let transformed = format!("models--{}", model_to_delete.name.replace('/', "--"));
        log::info!("Transforming translation model name '{}' to folder name '{}' for deletion.", &model_to_delete.name, &transformed);
        transformed
    } else {
        model_to_delete.name.clone()
    };

    let sub_dir = if is_translation {
         let family = model_to_delete.family.as_deref().unwrap_or("helsinki");
         let org_dir = if family == "nllb" { "facebook" } else { "helsinki-nlp" };
         PathBuf::from("translation").join(org_dir)
    } else {
         PathBuf::from("transcription").join("whisper-cpp")
    };

    let model_path = PathBuf::from(&base_location).join(sub_dir).join(&folder_name);

    if model_path.exists() {
        log::info!("Deleting model from filesystem at path: {:?}", model_path);
        if model_path.is_dir() {
            fs::remove_dir_all(&model_path)?;
            log::info!("Successfully deleted directory.");
        } else if model_path.is_file() {
            log::warn!("Expected model path {:?} to be a directory, but it's a file. Deleting file.", model_path);
            fs::remove_file(&model_path)?;
        } else {
            log::warn!("Model path {:?} is not a file or directory. Skipping filesystem delete.", model_path);
        }
    } else {
        log::warn!("Model path {:?} not found on disk. Skipping filesystem delete.", model_path);
    }

    // Remove from config using the original name
    config.downloaded_models.retain(|m| m.name != model_to_delete.name);

    if config.downloaded_models.len() < initial_len {
        let remaining_models = &config.downloaded_models;
        let has_transcription_models = remaining_models.iter().any(|m| m.family.is_none() && !m.name.contains('/'));
        let has_translation_models = remaining_models.iter().any(|m| m.family.is_some() || m.name.contains('/'));

        if !has_transcription_models {
            config.verification_status.transcription_models_verified = false;
        }
        if !has_translation_models {
            config.verification_status.translation_models_verified = false;
        }

        write_config(&config)?;
        log::info!("Removed entry '{}' from config.", model_to_delete.name);
    } else {
        log::warn!("Model '{}' not found in config list. Config not updated.", model_to_delete.name);
    }

    Ok(())
}

#[command]
pub async fn change_download_location_and_move_models(new_location: String) -> Result<(), CommandError> {
    log::info!("CMD: change_dl_loc_move: {}", new_location);
    let trimmed = new_location.trim();
    if trimmed.is_empty() {
        return Err("New download location empty.".into());
    }
    let new_path = PathBuf::from(trimmed);
    if !new_path.exists() {
        log::info!("New location {:?} missing. Creating...", new_path);
        fs::create_dir_all(&new_path)?;
    } else if !new_path.is_dir() {
        return Err(format!("New location '{}' is not a directory.", trimmed).into());
    }

    let mut config = read_config()?;
    let old_location_str = config.download_location.clone();
    let old_path = if old_location_str.trim().is_empty() {
        PathBuf::from(get_default_download_location()?)
    } else {
        PathBuf::from(&old_location_str)
    };
    log::info!("Old loc: {:?}, New loc: {:?}", old_path, new_path);

    if old_path == new_path {
        log::info!("Locations same. Ensuring config reflects input.");
        if config.download_location != trimmed {
            config.download_location = trimmed.to_string();
            write_config(&config)?;
            log::info!("Config location updated to match input.");
        } else {
            log::info!("Config location already matches.");
        }
        return Ok(());
    }

    let models_in_config = config.downloaded_models.clone();
    log::info!("Found {} models in config to move.", models_in_config.len());
    let mut move_errors : Vec<String> = Vec::new();

    for model in &models_in_config {
        // Determine subdirectory based on model type and family
        let is_translation = model.name.contains('/') || model.family.is_some();
        let sub_dir = if is_translation {
             let family = model.family.as_deref().unwrap_or("helsinki");
             let org_dir = if family == "nllb" { "facebook" } else { "helsinki-nlp" };
             PathBuf::from("translation").join(org_dir)
        } else {
             PathBuf::from("transcription").join("whisper-cpp")
        };

        // Handle the case where the model name in the config (e.g., "Helsinki-NLP/opus-mt-ja-en")
        // is different from the folder name on disk (e.g., "models--Helsinki-NLP--opus-mt-ja-en").
        let folder_name = if is_translation && model.name.contains('/') {
            format!("models--{}", model.name.replace('/', "--"))
        } else {
            model.name.clone()
        };

        let old_model_dir = old_path.join(&sub_dir).join(&folder_name);
        let new_model_dir = new_path.join(&sub_dir).join(&folder_name);

        log::info!("Check model '{}': Old {:?}, New {:?}", model.name, old_model_dir, new_model_dir);

        if old_model_dir.exists() {
            if old_model_dir.is_dir() {
                log::info!("Attempt move {:?} -> {:?}", old_model_dir, new_model_dir);

                 // Ensure parent dir exists
                 if let Some(parent) = new_model_dir.parent() {
                     if !parent.exists() {
                         let _ = fs::create_dir_all(parent);
                     }
                 }

                if new_model_dir.exists() {
                    log::warn!("Target {:?} exists. Removing before move.", new_model_dir);
                    if let Err(e) = fs::remove_dir_all(&new_model_dir) {
                        let m = format!("Failed remove target {:?} for '{}': {}", new_model_dir, model.name, e);
                        log::error!("{}", m);
                        move_errors.push(m);
                        continue;
                    }
                }
                if let Err(e) = fs::rename(&old_model_dir, &new_model_dir) {
                    let m = format!("Failed move '{}' {:?}->{:?}: {}", model.name, old_model_dir, new_model_dir, e);
                    log::error!("{}", m);
                    move_errors.push(m);
                } else {
                    log::info!("Moved '{}'.", model.name);
                }
            } else {
                log::warn!("Source path {:?} not dir. Skip.", old_model_dir);
            }
        } else {
             // Fallback check for legacy structure (Root/ModelName)
             let legacy_old_model_dir = old_path.join(&folder_name);
             if legacy_old_model_dir.exists() && legacy_old_model_dir.is_dir() {
                 log::info!("Found model at legacy path {:?}. Moving to new structure {:?}.", legacy_old_model_dir, new_model_dir);
                  // Ensure parent dir exists
                 if let Some(parent) = new_model_dir.parent() {
                     if !parent.exists() {
                         let _ = fs::create_dir_all(parent);
                     }
                 }
                  if new_model_dir.exists() {
                     log::warn!("Target {:?} exists. Removing before move.", new_model_dir);
                     if let Err(e)=fs::remove_dir_all(&new_model_dir){let m=format!("Failed remove target {:?} for '{}': {}", new_model_dir,model.name,e); log::error!("{}", m); move_errors.push(m); continue;}
                 }
                 if let Err(e)=fs::rename(&legacy_old_model_dir,&new_model_dir){let m=format!("Failed move '{}' {:?}->{:?}: {}", model.name,legacy_old_model_dir,new_model_dir,e); log::error!("{}", m); move_errors.push(m);} else { log::info!("Moved '{}' (from legacy).", model.name); }
             } else {
                 log::info!("Old path {:?} missing (and legacy check failed). Skip.", old_model_dir);
                 if new_model_dir.exists() && new_model_dir.is_dir() {
                     log::info!("Model '{}' already at new loc {:?}.", model.name, new_model_dir);
                 }
             }
        }
    }

    if !move_errors.is_empty() {
        log::error!("Errors moving models. Aborting config update.");
        return Err(CommandError::from(move_errors.join("\n")));
    }

    log::info!("Moves done. Updating config loc to '{}'.", trimmed);
    config.download_location = trimmed.to_string();
    log::info!("Updating model download_location entries...");
    for model_cfg in config.downloaded_models.iter_mut() {
        log::info!("Set stored loc for '{}' to '{}'", model_cfg.name, trimmed);
        model_cfg.download_location = Some(trimmed.to_string());
    }
    write_config(&config)?;
    log::info!("Config updated.");
    Ok(())
}
#[command] pub async fn download_model_command( app: AppHandle, cancellation_state: State<'_, DownloadCancellationState>, model_info: ModelInfo, download_location: String ) -> Result<(), CommandError> { /* ... */ log::info!("CMD: download_model: {} -> {}", model_info.name, download_location); let model_name = model_info.name.clone(); let target_dir = PathBuf::from(&download_location); if download_location.trim().is_empty() { return Err(CommandError::from(format!("Download location empty for '{}'.", model_name))); } if !target_dir.exists() { log::info!("Target dir {:?} missing. Creating...", target_dir); fs::create_dir_all(&target_dir)?; } else if !target_dir.is_dir() { return Err(CommandError::from(format!("Target path {:?} not dir.", target_dir))); } let cancel_flag = Arc::new(AtomicBool::new(false)); cancellation_state.0.insert(model_name.clone(), Arc::clone(&cancel_flag)); log::info!("Cancel token stored for {}", model_name); let download_result = download_and_save_bin(app.clone(), cancel_flag.clone(), model_info.clone(), download_location.clone()).await; if cancellation_state.0.remove(&model_name).is_some() { log::info!("Removed cancel token for {}", model_name); } else { log::warn!("Cancel token {} already removed.", model_name); } match download_result { Ok(_) => { log::info!("Download success for {}", model_name); app.emit("download-complete", &model_name).map_err(|e| CommandError::from(format!("Emit fail: {}", e)))?; Ok(()) } Err(e) => { log::error!("Download error for {}: {}", model_name, e); let _=app.emit("download-error", &ErrorPayload { model_name: model_name.clone(), error_message: format!("{}", e), }).map_err(|emit_err| log::error!("Emit error fail: {}", emit_err)); Err(e) } } }
async fn download_and_save_bin(
    app: AppHandle,
    cancel_flag: Arc<AtomicBool>,
    model_info: ModelInfo,
    download_base_location: String,
) -> Result<(), CommandError> {
    log::info!("CMD: download_and_save_bin: {} -> {}", model_info.name, download_base_location);
    let model_name = model_info.name.clone();
    let model_url = model_info.download_url.as_ref()
        .filter(|url| !url.trim().is_empty())
        .ok_or_else(|| CommandError::from(format!("Model '{}' missing URL.", model_name)))?;

    let bin_filename = Path::new(model_url).file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| CommandError::from(format!("Bad URL filename: {}", model_url)))?
        .to_string();

    // Updated logic: Save to transcription/whisper-cpp subdirectory
    let sub_dir = PathBuf::from("transcription").join("whisper-cpp");
    let model_dest_dir = PathBuf::from(&download_base_location).join(sub_dir).join(&model_name);

    let bin_filepath = model_dest_dir.join(&bin_filename);
    let temp_bin_filepath = model_dest_dir.join(format!("{}.part", bin_filename));

    if !model_dest_dir.exists() {
        log::info!("Creating model dir: {:?}", model_dest_dir);
        fs::create_dir_all(&model_dest_dir)
            .map_err(|e| format!("Failed create dir {:?}: {}", model_dest_dir, e))?;
    } else if !model_dest_dir.is_dir() {
        log::warn!("Expected dir {:?} not dir. Removing.", model_dest_dir);
        fs::remove_file(&model_dest_dir)
            .map_err(|e| format!("Failed remove file {:?}: {}", model_dest_dir, e))?;
        fs::create_dir_all(&model_dest_dir)
            .map_err(|e| format!("Failed re-create dir {:?}: {}", model_dest_dir, e))?;
    }

    if temp_bin_filepath.exists() {
        log::info!("Clean partial: {:?}", temp_bin_filepath);
        fs::remove_file(&temp_bin_filepath)
            .map_err(|e| format!("Failed clean partial {:?}: {}", temp_bin_filepath, e))?;
    }
    if bin_filepath.exists() {
        log::info!("Clean existing model: {:?}", bin_filepath);
        fs::remove_file(&bin_filepath)
            .map_err(|e| format!("Failed clean existing model {:?}: {}", bin_filepath, e))?;
    }

    log::info!("Starting download '{}': {}", model_name, model_url);
    let _ = app.emit("download-start", &model_name);
    let client = reqwest::Client::new();
    let response = client.get(model_url).send().await?;
    if !response.status().is_success() {
        return Err(format!("Download fail '{}': Status {} URL {}", model_name, response.status(), model_url).into());
    }

    let total_size = response.content_length();
    match total_size {
        Some(s) => log::info!("Download size: {} bytes", s),
        None => log::warn!("No Content-Length."),
    }

    let mut dest_file = File::create(&temp_bin_filepath)
        .map_err(|e| format!("Failed create temp {:?}: {}", temp_bin_filepath, e))?;

    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let _ = app.emit("download-progress", &DownloadProgress {
        model_name: model_name.clone(),
        downloaded_bytes: 0,
        total_bytes: total_size
    });

    while let Some(item_result) = stream.next().await {
        if cancel_flag.load(Ordering::Relaxed) {
            log::warn!("Cancel {}. Abort download.", model_name);
            drop(dest_file);
            let _ = fs::remove_file(&temp_bin_filepath);
            return Err(CommandError::from(format!("Download cancelled for '{}'.", model_name)));
        }
        match item_result {
            Ok(chunk) => {
                dest_file.write_all(&chunk).map_err(|e| format!("Failed write chunk: {}", e))?;
                downloaded += chunk.len() as u64;
                let _ = app.emit("download-progress", &DownloadProgress {
                    model_name: model_name.clone(),
                    downloaded_bytes: downloaded,
                    total_bytes: total_size
                });
            }
            Err(e) => {
                drop(dest_file);
                let _ = fs::remove_file(&temp_bin_filepath);
                return Err(format!("Error read stream '{}': {}", model_name, e).into());
            }
        }
    }
    drop(dest_file);
    log::info!("Download stream finished '{}'. Bytes: {}", model_name, downloaded);

    if cancel_flag.load(Ordering::Relaxed) {
        log::warn!("Cancel {} after download. Abort.", model_name);
        let _ = fs::remove_file(&temp_bin_filepath);
        return Err(CommandError::from(format!("Download cancelled post-dl '{}'.", model_name)));
    }

    if let Some(total) = total_size {
        if downloaded != total {
            let _ = fs::remove_file(&temp_bin_filepath);
            return Err(format!("Incomplete '{}': Expected {}, got {}.", model_name, total, downloaded).into());
        }
        log::info!("Size matches Content-Length '{}'.", model_name);
    }

    log::info!("Rename temp {:?} -> {:?}", temp_bin_filepath, bin_filepath);
    fs::rename(&temp_bin_filepath, &bin_filepath)
        .map_err(|e| format!("Failed rename {:?}->{:?}: {}", temp_bin_filepath, bin_filepath, e))?;

    log::info!("Update config for '{}'...", model_name);
    let mut config = read_config()?;
    let mut downloaded_model_info = model_info.clone();
    // Keep storing the ROOT location in config for consistency
    downloaded_model_info.download_location = Some(download_base_location.clone());

    if let Some(idx) = config.downloaded_models.iter().position(|m| m.name == downloaded_model_info.name) {
        log::info!("Model '{}' already in config. Updating.", model_info.name);
        config.downloaded_models[idx] = downloaded_model_info;
    } else {
        log::info!("Adding new model '{}' to config.", model_info.name);
        config.downloaded_models.push(downloaded_model_info);
    }
    write_config(&config)?;
    log::info!("Config updated for '{}'.", model_info.name);
    Ok(())
}
#[command] pub async fn cancel_download_command( cancellation_state: State<'_, DownloadCancellationState>, model_name: String, ) -> Result<(), CommandError> { /* ... */ log::info!("CMD: cancel_download: {}", model_name); if let Some(flag_entry)=cancellation_state.0.get(&model_name){let flag=flag_entry.value(); flag.store(true,Ordering::Relaxed); log::info!("Cancel flag set for {}",model_name);} else {log::warn!("No active download token for '{}'.",model_name);} Ok(()) }

 



/// Gets the saved theme preference ("light", "dark", "system", or None).
 #[command]
 pub async fn get_theme_preference() -> Result<Option<String>, CommandError> {
    log::info!("Config: get_theme_preference called.");
    let config = read_config()?;
    log::info!("Config: Returning theme preference: {:?}", config.theme);
    Ok(config.theme) // Return the Option<String> directly
 }

 /// Saves the theme preference ("light", "dark", "system").
 #[command]
 pub async fn set_theme_preference(theme: String) -> Result<(), CommandError> {
    log::info!("Config: set_theme_preference called with theme: {}", theme);
    // Validate input theme value
    if !["light", "dark", "system"].contains(&theme.as_str()) {
        let error_msg = format!("Invalid theme value provided: '{}'. Must be 'light', 'dark', or 'system'.", theme);
        log::error!("Config: {}", error_msg);
        return Err(CommandError::from(error_msg));
    }

    let mut config = read_config()?;
    config.theme = Some(theme.clone()); // Store the valid theme string
    write_config(&config)?;
    log::info!("Config: Theme preference '{}' saved.", theme);
    Ok(())
 }
 // --- End Theme Preference Commands ---

 #[command]
pub async fn check_python_libraries_installed<R: Runtime>(app: AppHandle<R>) -> Result<bool, CommandError> {
    let shell = app.shell();
    python_env::check_python_libraries_installed(&app, &shell).await
}

#[command]
pub async fn install_python_libraries<R: Runtime>(app: AppHandle<R>) -> Result<(), CommandError> {
    let shell = app.shell();
    python_env::install_python_libraries(&app, &shell).await
}

#[command]
pub async fn fetch_available_models_command(app: AppHandle) -> Result<serde_json::Value, CommandError> {
    log::info!("CMD: fetch_available_models_command");

    let script_path = app.path().resource_dir().unwrap().join("scripts/fetch_available_models.py");
    
    // Check if script exists
    if !script_path.exists() {
         return Err(CommandError::from(format!("Script not found at: {:?}", script_path)));
    }

    let python_path = python_env::get_python_path()?; 

    let output = app.shell()
        .command(python_path.to_str().unwrap())
        .args(&[script_path.to_str().unwrap()])
        .output()
        .await
        .map_err(|e| CommandError::from(format!("Failed to execute python script: {}", e)))?;

    if output.status.success() {
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let json_result: serde_json::Value = serde_json::from_str(&stdout_str)
            .map_err(|e| CommandError::from(format!("Failed to parse JSON output: {}", e)))?;
        log::info!("Successfully fetched available models.");
        Ok(json_result)
    } else {
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        log::error!("Python script failed: {}", stderr_str);
        Err(CommandError::from(format!("Python script failed: {}", stderr_str)))
    }
}