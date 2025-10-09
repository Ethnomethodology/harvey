// src/lib/stores/configStatusStore.js
import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

// Initial state
const initialStatus = {
    python_libraries_installed: true,
    hf_token_present: true,
    transcription_models_downloaded: true,
    diarization_model_downloaded: true,
    translation_models_downloaded: true,
};

// Create the writable store
export const configStatus = writable(initialStatus);

// Function to fetch the status from the backend and update the store
export async function updateConfigStatus() {
    try {
        const status = await invoke('check_config_status');
        configStatus.set(status);
        console.log("Config status updated:", status);
    } catch (error) {
        console.error("Failed to check config status:", error);
        // Optionally, set the store to an error state or default values
        configStatus.set({
            python_libraries_installed: false,
            hf_token_present: false,
            transcription_models_downloaded: false,
            diarization_model_downloaded: false,
            translation_models_downloaded: false,
        });
    }
}
