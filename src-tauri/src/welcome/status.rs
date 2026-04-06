// src-tauri/src/welcome/status.rs

use super::{
    commands::{
        check_python_libraries_installed, get_downloaded_models, get_local_translation_models,
        is_ctranslate2_installed,
    },
    diarization::check_diarization_model_access,
    hf_auth::check_hf_auth_status,
};
use crate::welcome::config::CommandError;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigStatus {
    pub python_libraries_installed: bool,
    pub hf_token_present: bool,
    pub transcription_models_downloaded: bool,
    pub whisper_cpp_models_downloaded: bool,
    pub faster_whisper_models_downloaded: bool,
    pub diarization_model_downloaded: bool,
    pub translation_models_downloaded: bool,
    pub helsinki_models_downloaded: bool,
    pub nllb_models_downloaded: bool,
    pub ctranslate2_installed: bool,
    pub faster_whisper_dependencies_installed: bool,
    pub whisper_cpp_installed: bool,
}

use crate::welcome::config::{read_config, write_config};
use crate::welcome::python_env;

#[tauri::command]
pub async fn check_config_status<R: Runtime>(
    app_handle: AppHandle<R>,
) -> Result<ConfigStatus, CommandError> {
    let mut config = read_config()?;
    let mut config_changed = false;

    // --- Initialize status from config ---
    let mut python_libs_installed = config.verification_status.python_libraries_verified;
    let mut transcription_models_downloaded =
        config.verification_status.transcription_models_verified;
    let mut diarization_model_downloaded = config.verification_status.diarization_model_verified;
    let mut translation_models_downloaded = config.verification_status.translation_models_verified;
    let mut hf_token_present = config.verification_status.hf_token_verified;
    let mut ct2_installed = config.verification_status.ctranslate2_verified;
    let mut fw_deps_installed = config
        .verification_status
        .faster_whisper_dependencies_verified;
    let mut whisper_cpp_installed = config.verification_status.whisper_cpp_verified;

    // --- Lightweight Checks ---
    if python_libs_installed && !python_env::get_env_path()?.exists() {
        python_libs_installed = false;
        config.verification_status.python_libraries_verified = false;
        // Also reset dependent checks if python env is missing
        ct2_installed = false;
        config.verification_status.ctranslate2_verified = false;
        fw_deps_installed = false;
        config
            .verification_status
            .faster_whisper_dependencies_verified = false;
        whisper_cpp_installed = false;
        config.verification_status.whisper_cpp_verified = false;
        config_changed = true;
    }

    // Transcription and Translation: Instant checks via directory listing
    let models = get_downloaded_models().await?;
    let whisper_cpp_models_downloaded = models.iter().any(|m| {
        let family = m.family.as_deref().unwrap_or("whisper-cpp");
        (family == "whisper-cpp" || (m.family.is_none() && !m.name.contains('/')))
            && !m.name.contains("paraphrase")
    });
    let faster_whisper_models_downloaded = models.iter().any(|m| {
        let family = m.family.as_deref().unwrap_or("");
        family == "faster-whisper" && !m.name.contains("paraphrase")
    });
    let has_transcription = whisper_cpp_models_downloaded || faster_whisper_models_downloaded;

    let translation_models = get_local_translation_models().await?;
    let helsinki_models_downloaded = translation_models
        .iter()
        .any(|m| m.family.as_deref().unwrap_or("helsinki") == "helsinki");
    let nllb_models_downloaded = translation_models
        .iter()
        .any(|m| m.family.as_deref().unwrap_or("") == "nllb");
    let has_translation = !translation_models.is_empty();

    if transcription_models_downloaded != has_transcription {
        transcription_models_downloaded = has_transcription;
        config.verification_status.transcription_models_verified = transcription_models_downloaded;
        config_changed = true;
    }

    if translation_models_downloaded != has_translation {
        translation_models_downloaded = has_translation;
        config.verification_status.translation_models_verified = translation_models_downloaded;
        config_changed = true;
    }

    // Diarization: Lightweight check (directory existence)
    if diarization_model_downloaded {
        let config_copy = config.clone();
        let diarization_hub_path = std::path::PathBuf::from(&config_copy.download_location)
            .join("diarization")
            .join("hub");
        if !diarization_hub_path.exists() {
            diarization_model_downloaded = false;
            config.verification_status.diarization_model_verified = false;
            config_changed = true;
        }
    }

    // --- Conditional Heavy Checks (Only if currently false) ---
    // If they are false, we try to verify them once. If they are true, we trust the lightweight checks above.

    if !python_libs_installed
        && check_python_libraries_installed(app_handle.clone())
            .await
            .unwrap_or(false)
    {
        python_libs_installed = true;
        config.verification_status.python_libraries_verified = true;
        config_changed = true;
    }

    if python_libs_installed {
        if !ct2_installed {
            ct2_installed = is_ctranslate2_installed(app_handle.clone())
                .await
                .unwrap_or(false);
            if ct2_installed {
                config.verification_status.ctranslate2_verified = true;
                config_changed = true;
            }
        }
        if !fw_deps_installed {
            fw_deps_installed =
                super::commands::is_faster_whisper_dependencies_installed(app_handle.clone())
                    .await
                    .unwrap_or(false);
            if fw_deps_installed {
                config
                    .verification_status
                    .faster_whisper_dependencies_verified = true;
                config_changed = true;
            }
        }
    }

    if !whisper_cpp_installed {
        whisper_cpp_installed = super::commands::is_whisper_cpp_installed(app_handle.clone())
            .await
            .unwrap_or(false);
        if whisper_cpp_installed {
            config.verification_status.whisper_cpp_verified = true;
            config_changed = true;
        }
    }

    if !hf_token_present {
        hf_token_present = check_hf_auth_status(app_handle.clone()).unwrap_or(false);
        if hf_token_present {
            config.verification_status.hf_token_verified = true;
            config_changed = true;
        }
    }

    if !diarization_model_downloaded
        && python_libs_installed
        && check_diarization_model_access(app_handle.clone())
            .await
            .unwrap_or(false)
    {
        diarization_model_downloaded = true;
        config.verification_status.diarization_model_verified = true;
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
        whisper_cpp_models_downloaded,
        faster_whisper_models_downloaded,
        diarization_model_downloaded,
        translation_models_downloaded,
        helsinki_models_downloaded,
        nllb_models_downloaded,
        ctranslate2_installed: ct2_installed,
        faster_whisper_dependencies_installed: fw_deps_installed,
        whisper_cpp_installed,
    })
}
