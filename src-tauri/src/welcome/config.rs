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
};
use csv;
use tauri_plugin_shell;
use tauri;


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
#[serde(rename = "config")]
pub struct Config {
    #[serde(rename = "project", default)]
    pub projects: Vec<ProjectInfo>,
    #[serde(rename = "download_location", default)]
    pub download_location: String,
    #[serde(rename = "downloaded_models", default)]
    pub downloaded_models: Vec<ModelInfo>,

    // --- Cloud Configuration Fields ---
    #[serde(rename = "cloudApiKey", default, skip_serializing_if = "Option::is_none")]
    pub cloud_api_key: Option<String>,
    #[serde(rename = "cloudModel", default, skip_serializing_if = "Option::is_none")]
    pub cloud_model: Option<String>,
    #[serde(rename = "cloudConsent", default, skip_serializing_if = "Option::is_none")]
    pub cloud_consent: Option<bool>,

    // --- NEW: Theme Preference Field ---
    #[serde(rename = "themePreference", default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>, // Stores "light", "dark", or "system"
    // --- End NEW Fields ---
}

// Struct for Cloud Config Payload (Unchanged)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CloudConfigPayload {
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub consent: Option<bool>,
}


#[derive(Debug, Serialize)]
pub struct CommandError {
    pub message: String,
}

// --- Error conversions (Unchanged) ---
impl From<std::io::Error> for CommandError { fn from(error: std::io::Error) -> Self { CommandError { message: format!("IO Error: {}", error) } } }
impl From<quick_xml::Error> for CommandError { fn from(error: quick_xml::Error) -> Self { CommandError { message: format!("XML Processing Error: {}", error) } } }
impl From<quick_xml::DeError> for CommandError { fn from(error: quick_xml::DeError) -> Self { CommandError { message: format!("XML Deserialization Error: {}", error) } } }
impl From<String> for CommandError { fn from(message: String) -> Self { CommandError { message } } }
impl From<&str> for CommandError { fn from(message: &str) -> Self { CommandError { message: message.to_string() } } }
impl From<reqwest::Error> for CommandError { fn from(error: reqwest::Error) -> Self { CommandError { message: format!("HTTP Download Error: {}", error) } } }
impl From<zip::result::ZipError> for CommandError { fn from(error: zip::result::ZipError) -> Self { CommandError { message: format!("ZIP Extraction Error: {}", error) } } }
impl From<csv::Error> for CommandError { fn from(error: csv::Error) -> Self { CommandError { message: format!("CSV Processing Error: {}", error) } } }
impl From<tauri_plugin_shell::Error> for CommandError { fn from(error: tauri_plugin_shell::Error) -> Self { CommandError { message: format!("Shell Command Error: {}", error) } } }
impl From<tauri::Error> for CommandError { fn from(error: tauri::Error) -> Self { CommandError { message: format!("Tauri API Error: {}", error) } } }
// --- End Error Conversions ---


// --- Config functions ---
pub fn get_config_dir() -> Result<PathBuf, CommandError> { UserDirs::new().map(|dirs| dirs.home_dir().join(CONFIG_DIR_NAME)).ok_or_else(|| CommandError::from("Could not find user home directory.")) }
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