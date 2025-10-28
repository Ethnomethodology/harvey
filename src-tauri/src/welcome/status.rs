// src-tauri/src/welcome/status.rs

use serde::{Serialize, Deserialize};
use tauri::{AppHandle, Runtime};
use super::{
    commands::{check_python_libraries_installed, get_downloaded_models, get_local_translation_models},
    diarization::check_diarization_model_access,
    hf_auth::check_hf_auth_status,
};
use crate::welcome::config::CommandError;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigStatus {
    pub python_libraries_installed: bool,
    pub hf_token_present: bool,
    pub transcription_models_downloaded: bool,
    pub diarization_model_downloaded: bool,
    pub translation_models_downloaded: bool,
}

use crate::welcome::config::{read_config, write_config};
use crate::welcome::commands::get_download_location;
use crate::welcome::python_env;

#[tauri::command]
pub async fn check_config_status<R: Runtime>(app_handle: AppHandle<R>) -> Result<ConfigStatus, CommandError> {
    let mut config = read_config()?;
    let mut config_changed = false;

    // --- Initialize status from config ---
    let mut python_libs_installed = config.verification_status.python_libraries_verified;
    let mut transcription_models_downloaded = config.verification_status.transcription_models_verified;
    let mut diarization_model_downloaded = config.verification_status.diarization_model_verified;
    let mut translation_models_downloaded = config.verification_status.translation_models_verified;
    let mut hf_token_present = config.verification_status.hf_token_verified;

    // --- Lightweight Checks ---
    if python_libs_installed && !python_env::get_env_path()?.exists() {
        python_libs_installed = false;
        config.verification_status.python_libraries_verified = false;
        config_changed = true;
    }
    if transcription_models_downloaded {
        let download_location = get_download_location().await?;
        if !std::path::Path::new(&download_location).exists() {
            transcription_models_downloaded = false;
            config.verification_status.transcription_models_verified = false;
            config_changed = true;
        }
    }
    if diarization_model_downloaded {
        let hf_hub_path = dirs::home_dir().map(|h| h.join(".cache/huggingface/hub")).unwrap_or_default();
        if !hf_hub_path.exists() {
            diarization_model_downloaded = false;
            config.verification_status.diarization_model_verified = false;
            config_changed = true;
        }
    }
    if translation_models_downloaded {
        let download_location = get_download_location().await?;
        if !std::path::Path::new(&download_location).exists() {
            translation_models_downloaded = false;
            config.verification_status.translation_models_verified = false;
            config_changed = true;
        }
    }

    // --- Full Checks (if necessary) ---
    if !python_libs_installed && check_python_libraries_installed(app_handle.clone()).await.unwrap_or(false) {
        python_libs_installed = true;
        config.verification_status.python_libraries_verified = true;
        config_changed = true;
    }
    let current_hf_token_status = check_hf_auth_status(app_handle.clone()).unwrap_or(false);
    if hf_token_present != current_hf_token_status {
        hf_token_present = current_hf_token_status;
        config.verification_status.hf_token_verified = hf_token_present;
        config_changed = true;
    }
    if !transcription_models_downloaded && !get_downloaded_models().await?.is_empty() {
        transcription_models_downloaded = true;
        config.verification_status.transcription_models_verified = true;
        config_changed = true;
    }
    if !diarization_model_downloaded && check_diarization_model_access(app_handle.clone()).await.unwrap_or(false) {
        diarization_model_downloaded = true;
        config.verification_status.diarization_model_verified = true;
        config_changed = true;
    }
    if !translation_models_downloaded && !get_local_translation_models().await?.is_empty() {
        translation_models_downloaded = true;
        config.verification_status.translation_models_verified = true;
        config_changed = true;
    }

    // --- Finalize ---
    if config_changed {
        write_config(&config)?;
    }

    Ok(ConfigStatus {
        python_libraries_installed: python_libs_installed,
        hf_token_present,
        transcription_models_downloaded,
        diarization_model_downloaded,
        translation_models_downloaded,
    })
}
