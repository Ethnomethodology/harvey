// src-tauri/src/welcome/diarization.rs
use tauri::Manager;
use std::fs;
use std::path::PathBuf;
use std::env;
use hf_hub::{api::tokio::Api, Repo, RepoType};

// Re-implementing this helper to read the token.
fn get_hf_token(app_handle: &tauri::AppHandle) -> Result<String, String> {
    let config_dir = app_handle.path().app_config_dir().expect("Failed to get app config dir");
    let token_path = config_dir.join("hf_token");
    fs::read_to_string(token_path).map_err(|e| format!("Failed to read token: {}. Please ensure you have saved your HuggingFace token in the 'Application' tab.", e))
}

// Gets the expected local path for the diarization model directory.
fn get_diarization_model_path(app_handle: &tauri::AppHandle) -> PathBuf {
    let config_dir = app_handle.path().app_config_dir().expect("Failed to get app config dir");
    // Using a subdirectory within the general download location might be better,
    // but for simplicity, we'll place it in a known location.
    config_dir.join("models--pyannote--speaker-diarization-3.1")
}

#[tauri::command]
pub fn check_diarization_model_access(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let model_path = get_diarization_model_path(&app_handle);
    // The presence of `config.yaml` is a good indicator that the pipeline files are there.
    Ok(model_path.join("config.yaml").exists())
}

#[tauri::command]
pub async fn download_diarization_model(app_handle: tauri::AppHandle) -> Result<(), String> {
    let token = get_hf_token(&app_handle)?;
    let local_path = get_diarization_model_path(&app_handle);

    if !local_path.exists() {
        fs::create_dir_all(&local_path).map_err(|e| e.to_string())?;
    }

    // Set the token as an environment variable for hf-hub to pick it up.
    env::set_var("HUGGING_FACE_HUB_TOKEN", &token);

    let api = Api::new().map_err(|e| format!("Failed to create HuggingFace API client: {}", e))?;
    let repo = api.repo(Repo::new("pyannote/speaker-diarization-3.1".to_string(), RepoType::Model));

    // This will download all files from the repo into the specified local directory.
    // The hf-hub crate handles the cloning/downloading process.
    repo.download(&local_path).await.map_err(|e| format!("Failed to download model. Please ensure you have accepted the user agreement on the model's HuggingFace page. Error: {}", e))?;

    Ok(())
}
