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

#[tauri::command]
pub async fn check_config_status<R: Runtime>(app_handle: AppHandle<R>) -> Result<ConfigStatus, CommandError> {
    // let python_libs = check_python_libraries_installed(app_handle.clone()).await.unwrap_or(false);
    let hf_token = check_hf_auth_status(app_handle.clone()).unwrap_or(false);
    let transcription_models = !get_downloaded_models().await?.is_empty();
    let diarization_model = check_diarization_model_access(app_handle.clone()).await.unwrap_or(false);
    let translation_models = !get_local_translation_models().await?.is_empty();

    Ok(ConfigStatus {
        python_libraries_installed: python_libs,
        hf_token_present: hf_token,
        transcription_models_downloaded: transcription_models,
        diarization_model_downloaded: diarization_model,
        translation_models_downloaded: translation_models,
    })
}
