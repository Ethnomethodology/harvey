// src-tauri/src/welcome/config.rs

use chrono::{DateTime, Utc};
use directories::UserDirs;
use quick_xml::{
    de::from_str,
    se::to_string_with_root,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::PathBuf,
    fmt,
};
use csv;
use tauri_plugin_shell;
use tauri;
use rust_xlsxwriter;
use calamine;


pub const CONFIG_DIR_NAME: &str = ".harvey";
pub const CONFIG_FILE_NAME: &str = "config.xml";
pub const PROJECT_FILE_EXTENSION: &str = "harvey";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectInfo {
    pub name: String,
    pub path: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_ts: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub last_opened_ts: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>, // e.g., "helsinki", "nllb"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
}

// *** Config Struct with Theme Preference ***
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct VerificationStatus {
    #[serde(default)]
    pub python_libraries_verified: bool,
    #[serde(default)]
    pub transcription_models_verified: bool,
    #[serde(default)]
    pub translation_models_verified: bool,
    #[serde(default)]
    pub diarization_model_verified: bool,
    #[serde(default)]
    pub hf_token_verified: bool,
    #[serde(default)]
    pub ctranslate2_verified: bool,
    #[serde(default)]
    pub faster_whisper_dependencies_verified: bool,
    #[serde(default)]
    pub whisper_cpp_verified: bool,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename = "config")]
pub struct Config {
    #[serde(rename = "project", default)]
    pub projects: Vec<ProjectInfo>,
    #[serde(rename = "download_location", default)]
    pub download_location: String,
    #[serde(rename = "downloaded_models", default)]
    pub downloaded_models: Vec<ModelInfo>,

    #[serde(rename = "selected_translation_family", default)]
    pub selected_translation_family: Option<String>, // "helsinki" or "nllb"

    #[serde(rename = "selected_transcription_engine", default)]
    pub selected_transcription_engine: Option<String>, // "whisper-cpp" or "faster-whisper"

    // --- NEW: Theme Preference Field ---
    #[serde(rename = "themePreference", default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>, // Stores "light", "dark", or "system"

    #[serde(rename = "ffmpegPath", default, skip_serializing_if = "Option::is_none")]
    pub ffmpeg_path: Option<String>,

    #[serde(rename = "verification_status", default)]
    pub verification_status: VerificationStatus,

    #[serde(rename = "advanced_translation", default, skip_serializing_if = "Option::is_none")]
    pub advanced_translation: Option<AdvancedTranslationConfig>,

    #[serde(rename = "advanced_transcription", default, skip_serializing_if = "Option::is_none")]
    pub advanced_transcription: Option<AdvancedTranscriptionConfig>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct AdvancedTranslationConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub helsinki_batch_size: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nllb_batch_size: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_threads: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_preference: Option<String>, // "auto", "cpu", "cuda", "mps"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diarization_device: Option<String>, // "auto", "cpu", "cuda", "mps"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diarization_threads: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantization_preference: Option<String>, // "int8" or "float16"
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct AdvancedTranscriptionConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faster_whisper_compute_type: Option<String>, // "int8", "float16", "int8_float16"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faster_whisper_beam_size: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_threads: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_preference: Option<String>, // "auto", "cpu", "cuda", "mps"
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "payload")] // This will serialize to { "type": "...", "payload": "..." }
pub enum CommandError {
    Io(String),
    XmlProcessing(String),
    XmlDeserialization(String),
    Message(String),
    HttpDownload(String),
    ZipExtraction(String),
    CsvProcessing(String),
    ShellCommand(String),
    TauriApi(String),
    AssetMetadataNotFound(String), // New variant
    RusqliteError(String), // Added for rusqlite errors
    JsonProcessing(String),
    Path(String), // New variant for path-related errors
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::Io(msg) => write!(f, "IO Error: {}", msg),
            CommandError::XmlProcessing(msg) => write!(f, "XML Processing Error: {}", msg),
            CommandError::XmlDeserialization(msg) => write!(f, "XML Deserialization Error: {}", msg),
            CommandError::Message(msg) => write!(f, "{}", msg),
            CommandError::HttpDownload(msg) => write!(f, "HTTP Download Error: {}", msg),
            CommandError::ZipExtraction(msg) => write!(f, "ZIP Extraction Error: {}", msg),
            CommandError::CsvProcessing(msg) => write!(f, "CSV Processing Error: {}", msg),
            CommandError::ShellCommand(msg) => write!(f, "Shell Command Error: {}", msg),
            CommandError::TauriApi(msg) => write!(f, "Tauri API Error: {}", msg),
            CommandError::AssetMetadataNotFound(msg) => write!(f, "{}", msg),
            CommandError::RusqliteError(msg) => write!(f, "Database Error: {}", msg),
            CommandError::JsonProcessing(msg) => write!(f, "JSON Processing Error: {}", msg),
            CommandError::Path(msg) => write!(f, "Path Error: {}", msg),
        }
    }
}

impl std::error::Error for CommandError {}

// --- Error conversions ---
impl From<std::io::Error> for CommandError { fn from(error: std::io::Error) -> Self { CommandError::Io(error.to_string()) } }
impl From<quick_xml::Error> for CommandError { fn from(error: quick_xml::Error) -> Self { CommandError::XmlProcessing(error.to_string()) } }
impl From<quick_xml::DeError> for CommandError { fn from(error: quick_xml::DeError) -> Self { CommandError::XmlDeserialization(error.to_string()) } }
impl From<String> for CommandError { fn from(message: String) -> Self { CommandError::Message(message) } }
impl From<&str> for CommandError { fn from(message: &str) -> Self { CommandError::Message(message.to_string()) } }
impl From<reqwest::Error> for CommandError { fn from(error: reqwest::Error) -> Self { CommandError::HttpDownload(error.to_string()) } }
impl From<zip::result::ZipError> for CommandError { fn from(error: zip::result::ZipError) -> Self { CommandError::ZipExtraction(error.to_string()) } }
impl From<zip_extract::ZipExtractError> for CommandError { fn from(error: zip_extract::ZipExtractError) -> Self { CommandError::ZipExtraction(error.to_string()) } }
impl From<csv::Error> for CommandError { fn from(error: csv::Error) -> Self { CommandError::CsvProcessing(error.to_string()) } }
impl From<serde_json::Error> for CommandError { fn from(error: serde_json::Error) -> Self { CommandError::JsonProcessing(error.to_string()) } }
impl From<tauri_plugin_shell::Error> for CommandError { fn from(error: tauri_plugin_shell::Error) -> Self { CommandError::ShellCommand(error.to_string()) } }
impl From<tauri::Error> for CommandError { fn from(error: tauri::Error) -> Self { CommandError::TauriApi(error.to_string()) } }
impl From<rusqlite::Error> for CommandError { fn from(error: rusqlite::Error) -> Self { CommandError::RusqliteError(error.to_string()) } }
impl From<rust_xlsxwriter::XlsxError> for CommandError { fn from(error: rust_xlsxwriter::XlsxError) -> Self { CommandError::Message(error.to_string()) } }
impl From<calamine::XlsxError> for CommandError { fn from(error: calamine::XlsxError) -> Self { CommandError::Message(error.to_string()) } }
impl From<crate::utils::PathError> for CommandError { fn from(error: crate::utils::PathError) -> Self { CommandError::Path(error.to_string()) } }
// --- End Error Conversions ---


// --- Config functions ---
pub fn get_config_dir() -> Result<PathBuf, CommandError> { UserDirs::new().map(|dirs| dirs.home_dir().join(CONFIG_DIR_NAME)).ok_or_else(|| CommandError::Message("Could not find user home directory.".to_string())) }
pub fn get_config_file_path() -> Result<PathBuf, CommandError> { get_config_dir().map(|dir| dir.join(CONFIG_FILE_NAME)) }
pub fn ensure_config_dir_exists() -> Result<PathBuf, CommandError> { let config_dir = get_config_dir()?; fs::create_dir_all(&config_dir)?; Ok(config_dir) }
pub fn read_config() -> Result<Config, CommandError> {
    use rusqlite::Connection;
    use crate::projectview::db_handler::{get_db_path, init_db};

    // Ensure DB is initialized
    if let Err(e) = init_db() {
        eprintln!("Failed to initialize DB in read_config: {:?}", e);
    }

    let db_path = get_db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| CommandError::RusqliteError(e.to_string()))?;

    let mut config = Config::default();

    let config_path = get_config_file_path()?;
    if config_path.exists() {
        println!("Found legacy config file at {:?}. Migrating to SQLite...", config_path);
        if let Ok(xml_content) = fs::read_to_string(&config_path) {
            if !xml_content.trim().is_empty() {
                if let Ok(old_config) = from_str::<Config>(&xml_content) {
                    let _ = write_config(&old_config);
                }
            }
        }
        let mut bak_path = config_path.clone();
        bak_path.set_extension("xml.bak");
        let _ = fs::rename(&config_path, bak_path);
    }

    // Load global settings
    if let Ok(mut stmt) = conn.prepare("SELECT key, value FROM global_settings") {
        if let Ok(settings_iter) = stmt.query_map([], |row| {
            let key: String = row.get(0)?;
            let value: String = row.get(1)?;
            Ok((key, value))
        }) {
            for result in settings_iter.flatten() {
                let (key, value) = result;
                match key.as_str() {
                    "download_location" => config.download_location = value,
                    "selected_translation_family" => config.selected_translation_family = Some(value),
                    "selected_transcription_engine" => config.selected_transcription_engine = Some(value),
                    "themePreference" => config.theme = Some(value),
                    "ffmpegPath" => config.ffmpeg_path = Some(value),
                    "verification_status" => {
                        if let Ok(vs) = serde_json::from_str::<VerificationStatus>(&value) {
                            config.verification_status = vs;
                        }
                    }
                    "advanced_translation" => {
                        if let Ok(at) = serde_json::from_str::<AdvancedTranslationConfig>(&value) {
                            config.advanced_translation = Some(at);
                        }
                    }
                    "advanced_transcription" => {
                        if let Ok(at) = serde_json::from_str::<AdvancedTranscriptionConfig>(&value) {
                            config.advanced_transcription = Some(at);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Load projects
    if let Ok(mut stmt) = conn.prepare("SELECT name, xml_path, created_at, last_opened_ts FROM projects") {
        if let Ok(projects_iter) = stmt.query_map([], |row| {
            let name: String = row.get(0)?;
            let path: String = row.get(1)?;
            let created_at: String = row.get(2)?;
            let last_opened_ts: String = row.get(3)?;
            Ok((name, path, created_at, last_opened_ts))
        }) {
            for result in projects_iter.flatten() {
                let (name, path, created_at, last_opened_ts) = result;
                let created_ts = chrono::DateTime::parse_from_rfc3339(&format!("{}Z", created_at.replace(" ", "T")))
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let last_opened_ts = chrono::DateTime::parse_from_rfc3339(&format!("{}Z", last_opened_ts.replace(" ", "T")))
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                config.projects.push(ProjectInfo {
                    name,
                    path,
                    created_ts,
                    last_opened_ts,
                });
            }
        }
    }
    config.projects.sort_by(|a, b| b.last_opened_ts.cmp(&a.last_opened_ts));

    // Load downloaded models
    if let Ok(mut stmt) = conn.prepare("SELECT name, family, language, size, description, download_location, download_url FROM downloaded_models") {
        if let Ok(models_iter) = stmt.query_map([], |row| {
            let name: String = row.get(0)?;
            let family: Option<String> = row.get(1)?;
            let language: Option<String> = row.get(2)?;
            let size: Option<String> = row.get(3)?;
            let description: Option<String> = row.get(4)?;
            let download_location: Option<String> = row.get(5)?;
            let download_url: Option<String> = row.get(6)?;
            Ok(ModelInfo { name, family, language, size, description, download_location, download_url })
        }) {
            for result in models_iter.flatten() {
                config.downloaded_models.push(result);
            }
        }
    }
    config.downloaded_models.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(config)
}

pub fn write_config(config: &Config) -> Result<(), CommandError> {
    use rusqlite::{Connection, params};
    use crate::projectview::db_handler::{get_db_path, init_db};

    if let Err(e) = init_db() {
        eprintln!("Failed to initialize DB in write_config: {:?}", e);
    }
    let db_path = get_db_path()?;
    let mut conn = Connection::open(&db_path).map_err(|e| CommandError::RusqliteError(e.to_string()))?;

    let tx = conn.transaction().map_err(|e| CommandError::RusqliteError(e.to_string()))?;

    // Save global settings
    {
        let mut stmt = tx.prepare("INSERT OR REPLACE INTO global_settings (key, value) VALUES (?1, ?2)").map_err(|e| CommandError::RusqliteError(e.to_string()))?;
        stmt.execute(params!["download_location", &config.download_location]).map_err(|e| CommandError::RusqliteError(e.to_string()))?;
        if let Some(val) = &config.selected_translation_family { stmt.execute(params!["selected_translation_family", val]).map_err(|e| CommandError::RusqliteError(e.to_string()))?; }
        if let Some(val) = &config.selected_transcription_engine { stmt.execute(params!["selected_transcription_engine", val]).map_err(|e| CommandError::RusqliteError(e.to_string()))?; }
        if let Some(val) = &config.theme { stmt.execute(params!["themePreference", val]).map_err(|e| CommandError::RusqliteError(e.to_string()))?; }
        if let Some(val) = &config.ffmpeg_path { stmt.execute(params!["ffmpegPath", val]).map_err(|e| CommandError::RusqliteError(e.to_string()))?; }

        if let Ok(vs) = serde_json::to_string(&config.verification_status) {
            stmt.execute(params!["verification_status", vs]).map_err(|e| CommandError::RusqliteError(e.to_string()))?;
        }

        if let Some(at) = &config.advanced_translation {
            if let Ok(at_json) = serde_json::to_string(at) {
                stmt.execute(params!["advanced_translation", at_json]).map_err(|e| CommandError::RusqliteError(e.to_string()))?;
            }
        }

        if let Some(at) = &config.advanced_transcription {
            if let Ok(at_json) = serde_json::to_string(at) {
                stmt.execute(params!["advanced_transcription", at_json]).map_err(|e| CommandError::RusqliteError(e.to_string()))?;
            }
        }
    }

    // Projects update
    // We also want to support adding projects directly if they were only in the old config.xml but not in SQLite.
    {
        let mut stmt_update = tx.prepare("UPDATE projects SET last_opened_ts = ?1, name = ?2 WHERE xml_path = ?3").map_err(|e| CommandError::RusqliteError(e.to_string()))?;
        let mut stmt_check = tx.prepare("SELECT 1 FROM projects WHERE xml_path = ?1").map_err(|e| CommandError::RusqliteError(e.to_string()))?;
        let mut stmt_insert = tx.prepare("INSERT INTO projects (id, name, root_path, xml_path, created_at, last_opened_ts, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)").map_err(|e| CommandError::RusqliteError(e.to_string()))?;

        for project in &config.projects {
            let ts_str = project.last_opened_ts.format("%Y-%m-%d %H:%M:%S").to_string();
            let created_str = project.created_ts.format("%Y-%m-%d %H:%M:%S").to_string();
            let exists: bool = stmt_check.exists(params![project.path]).unwrap_or(false);
            if exists {
                let _ = stmt_update.execute(params![ts_str, project.name, project.path]);
            } else {
                let uuid = uuid::Uuid::new_v4().to_string();
                let root_path = PathBuf::from(&project.path).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                let _ = stmt_insert.execute(params![uuid, project.name, root_path, project.path, created_str, ts_str, ts_str]);
            }
        }
    }

    // Downloaded models
    {
        tx.execute("DELETE FROM downloaded_models", []).map_err(|e| CommandError::RusqliteError(e.to_string()))?;
        let mut stmt = tx.prepare("INSERT INTO downloaded_models (name, family, language, size, description, download_location, download_url) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)").map_err(|e| CommandError::RusqliteError(e.to_string()))?;
        for model in &config.downloaded_models {
            stmt.execute(params![
                model.name,
                model.family,
                model.language,
                model.size,
                model.description,
                model.download_location,
                model.download_url
            ]).map_err(|e| CommandError::RusqliteError(e.to_string()))?;
        }
    }

    tx.commit().map_err(|e| CommandError::RusqliteError(e.to_string()))?;

    println!("Successfully wrote global config to harvey.sqlite");
    Ok(())
}
pub fn add_or_update_project_in_config(project_info: ProjectInfo) -> Result<(), CommandError> {
    let mut config = read_config()?;
    match config.projects.iter_mut().find(|p| p.path == project_info.path) {
        Some(existing_project) => { println!("Updating existing project in config: {}", project_info.name); existing_project.last_opened_ts = project_info.last_opened_ts; existing_project.name = project_info.name; }
        None => { println!("Adding new project to config: {}", project_info.name); config.projects.push(project_info); }
    }
    write_config(&config)
}

pub fn rename_project_in_db(old_path: &str, new_path: &str, new_name: &str) -> Result<(), CommandError> {
    use rusqlite::{Connection, params};
    use crate::projectview::db_handler::get_db_path;

    println!("Attempting database update for rename: {} -> {}", old_path, new_path);

    let db_path = get_db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| CommandError::RusqliteError(e.to_string()))?;

    let now = Utc::now();
    let ts_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

    let mut stmt = conn.prepare("UPDATE projects SET xml_path = ?1, name = ?2, last_opened_ts = ?3, updated_at = ?4 WHERE xml_path = ?5")
        .map_err(|e| CommandError::RusqliteError(e.to_string()))?;
    
    let rows_affected = stmt.execute(params![new_path, new_name, ts_str, ts_str, old_path])
        .map_err(|e| CommandError::RusqliteError(e.to_string()))?;

    if rows_affected == 0 {
        println!("Warning: No project found in database with path '{}' to rename.", old_path);
    } else {
        println!("Successfully updated database record for renamed project ({} rows affected).", rows_affected);
    }

    Ok(())
}

pub fn get_default_download_location() -> Result<String, CommandError> { let user_dirs = UserDirs::new().ok_or_else(|| CommandError::from("Cannot determine user directories."))?; let default_location: PathBuf = user_dirs.home_dir().join(CONFIG_DIR_NAME).join("models"); default_location.to_str().ok_or_else(|| CommandError::from("Failed to convert default download location path to string.")).map(|s| s.to_string()) }
// --- End Config Functions ---