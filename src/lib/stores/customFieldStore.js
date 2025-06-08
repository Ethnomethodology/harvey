import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

// Main store for the definitions
export const customFieldDefinitions = writable([]);

// Optional: For loading and error states
export const isLoadingDefinitions = writable(false);
export const definitionError = writable(null);

/**
 * Fetches all custom field definitions from the backend and updates the store.
 */
export async function loadAllDefinitions() {
    isLoadingDefinitions.set(true);
    definitionError.set(null);
    console.debug('[customFieldStore] Attempting to load all definitions...'); // Downgraded
    try {
        const definitions = await invoke('get_all_custom_field_definitions_command');
        customFieldDefinitions.set(definitions || []); // Ensure it's an array, even if null/undefined from backend
        console.info('[customFieldStore] Definitions loaded successfully. Count:', definitions?.length || 0); // Info, less verbose
    } catch (err) {
        const errorMessage = err.message || String(err);
        console.error('[customFieldStore] Error loading definitions:', errorMessage); // Keep error
        definitionError.set(errorMessage);
        customFieldDefinitions.set([]); // Clear definitions on error to avoid displaying stale data
    } finally {
        isLoadingDefinitions.set(false);
    }
}

/**
 * Adds a new custom field definition via a backend command and then refreshes the list.
 * @param {string} fieldKey - The unique key for the field.
 * @param {string} fieldName - The user-friendly name for the field.
 * @param {string} fieldType - The type of the field (e.g., "small_text").
 * @param {string} scopeStr - The scope string (e.g., "project", "image").
 * @param {string|null} [defaultValue=null] - Optional default value for the field.
 * @returns {Promise<{success: boolean}>} A promise that resolves to an object indicating success.
 * @throws {Error} If the backend command fails, an error is thrown with the message.
 */
export async function addDefinition(fieldKey, fieldName, fieldType, scopeStr, defaultValue = null) {
    console.debug(`[customFieldStore] Attempting to add definition: key='${fieldKey}', name='${fieldName}', type='${fieldType}', scope='${scopeStr}'`); // Downgraded
    try {
        await invoke('create_custom_field_definition_command', {
            fieldKey,
            fieldName,
            fieldType,
            scopeStr,
            defaultValue // This can be null, Option<String> on Rust side handles it
        });
        console.info('[customFieldStore] Definition added successfully for key:', fieldKey); // Keep as info
        await loadAllDefinitions(); // Refresh the list to include the new definition
        return { success: true };
    } catch (err) {
        const errorMessage = err.message || String(err);
        console.error('[customFieldStore] Error adding definition for key:', fieldKey, errorMessage); // Keep error
        // Propagate error message for UI handling
        throw new Error(errorMessage);
    }
}

// Example of how to initialize the store when the app loads,
// though this might be better placed in a root component like App.svelte or a layout file.
// loadAllDefinitions(); // Auto-load on store initialization - commented out, should be called from UI layer.
