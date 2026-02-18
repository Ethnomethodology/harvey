// src/lib/stores/configStatusStore.js
import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

// Initial state
const initialStatus = {
    isInitialized: false,
    python_libraries_installed: true,
    hf_token_present: true,
    transcription_models_downloaded: true,
    diarization_model_downloaded: true,
    translation_models_downloaded: true,
    ctranslate2_installed: true,
    faster_whisper_dependencies_installed: true,
};

// Create the writable store
export const configStatus = writable(initialStatus);

// Function to fetch the status from the backend and update the store
export async function updateConfigStatus(force = false) {
    if (get(configStatus).isInitialized && !force) {
        console.log("Config status already initialized and not forced, skipping backend check.");
        return;
    }
    
    // Reset initialization status to show checking state in UI
    configStatus.update(s => ({ ...s, isInitialized: false }));

    try {
        const status = await invoke('check_config_status');
        configStatus.set({ ...status, isInitialized: true });
        console.log("Config status updated:", status);
    } catch (error) {
        console.error("Failed to check config status:", error);
        configStatus.set({
            isInitialized: true, // Mark as initialized even on error to prevent retries
            python_libraries_installed: false,
            hf_token_present: false,
            transcription_models_downloaded: false,
            diarization_model_downloaded: false,
            translation_models_downloaded: false,
            ctranslate2_installed: false,
            faster_whisper_dependencies_installed: false,
        });
    }
}

// Granular setters for reactive updates
export const setPythonLibrariesInstalled = (status) => {
    configStatus.update(s => ({ ...s, python_libraries_installed: status }));
};
export const setHfTokenPresent = (status) => {
    configStatus.update(s => ({ ...s, hf_token_present: status }));
};
export const setTranscriptionModelsDownloaded = (status) => {
    configStatus.update(s => ({ ...s, transcription_models_downloaded: status }));
};
export const setDiarizationModelDownloaded = (status) => {
    configStatus.update(s => ({ ...s, diarization_model_downloaded: status }));
};
export const setTranslationModelsDownloaded = (status) => {
    configStatus.update(s => ({ ...s, translation_models_downloaded: status }));
};
export const setFasterWhisperDependenciesInstalled = (status) => {
    configStatus.update(s => ({ ...s, faster_whisper_dependencies_installed: status }));
};
