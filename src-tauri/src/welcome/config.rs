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
pub const PROJECT_FILE_EXTENSION: &str = "harvey.xml";

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
    let config_path = get_config_file_path()?;
    if !config_path.exists() { println!("Config file not found at {:?}, returning default.", config_path); return Ok(Config::default()); }
    println!("Reading config file from {:?}", config_path);
    let xml_content = fs::read_to_string(&config_path)?;
    if xml_content.trim().is_empty() { println!("Config file is empty, returning default."); return Ok(Config::default()); }
    match from_str::<Config>(&xml_content) {
        Ok(mut config) => {
            println!("Successfully parsed config file.");
            // Ensure models are sorted if present
            if !config.downloaded_models.is_empty() {
                 config.downloaded_models.sort_by(|a, b| a.name.cmp(&b.name));
            }
            Ok(config)
        }
        Err(e) => {
            eprintln!("Error parsing config file at {:?}: {}. Returning default config.", config_path, e);
            // Return default but maybe log the error more permanently?
            Ok(Config::default())
        }
    }
}
pub fn write_config(config: &Config) -> Result<(), CommandError> {
    let config_dir = ensure_config_dir_exists()?;
    let config_path = config_dir.join(CONFIG_FILE_NAME);
    println!("Writing config file to {:?}", config_path);
    let mut config_to_write = config.clone();
    // Sort projects by last opened, descending
    config_to_write.projects.sort_by(|a, b| b.last_opened_ts.cmp(&a.last_opened_ts));
    // Sort models alphabetically
    config_to_write.downloaded_models.sort_by(|a, b| a.name.cmp(&b.name));
    let xml_string = to_string_with_root("config", &config_to_write).map_err(CommandError::from)?;
    let file = File::create(&config_path)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(xml_string.as_bytes())?;
    writer.flush()?;
    println!("Successfully wrote config file.");
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
pub fn get_default_download_location() -> Result<String, CommandError> { let user_dirs = UserDirs::new().ok_or_else(|| CommandError::from("Cannot determine user directories."))?; let default_location: PathBuf = user_dirs.home_dir().join(CONFIG_DIR_NAME).join("models"); default_location.to_str().ok_or_else(|| CommandError::from("Failed to convert default download location path to string.")).map(|s| s.to_string()) }
// --- End Config Functions ---