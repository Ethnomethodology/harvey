import { writable, get } from 'svelte/store'; // Added get
import { invoke } from '@tauri-apps/api/core';
import { project } from '$lib/stores/projectStore.js'; // Added project import

// Main store for the definitions
export const customFieldDefinitions = writable([]);

// Optional: For loading and error states
export const isLoadingDefinitions = writable(false);
export const definitionError = writable(null);

/**
 * Helper function to get the current project ID from the projectStore.
 * Assumes project.xml is in a directory named after the project ID, under a 'projects' directory.
 * e.g., C:/Users/.../projects/My Project Name/project.xml -> "My Project Name"
 * e.g., /Users/.../projects/My Project Name/project.xml -> "My Project Name"
 * @returns {string|null} The project ID or null if not found/error.
 */
function getCurrentProjectId() {
    const currentProject = get(project); // 'project' store should already be imported

    if (currentProject && currentProject.id && typeof currentProject.id === 'string' && currentProject.id.trim() !== '') {
        console.debug(`[customFieldStore] Determined projectId from currentProject.id (UUID): "${currentProject.id}"`);
        return currentProject.id.trim();
    }

    // Log if the UUID is missing, as this is now unexpected after prior fixes
    console.warn(`[customFieldStore] getCurrentProjectId: currentProject.id (UUID) is missing, null, or empty. This might indicate an issue with project loading. Current project state:`, currentProject);
    return null;
}


/**
 * Fetches all custom field definitions from the backend and updates the store.
 */
export async function loadAllDefinitions() {
    isLoadingDefinitions.set(true);
    definitionError.set(null);
    const projectId = getCurrentProjectId();

    if (!projectId) {
        console.error('[customFieldStore] Cannot load definitions: Project ID could not be determined.');
        customFieldDefinitions.set([]);
        definitionError.set('Cannot load definitions: No active project selected or project path is invalid.');
        isLoadingDefinitions.set(false);
        return;
    }

    console.debug(`[customFieldStore] Attempting to load all definitions for projectId: ${projectId}...`);
    try {
        const definitions = await invoke('get_all_custom_field_definitions_command', { projectId });
        customFieldDefinitions.set(definitions || []); // Ensure it's an array, even if null/undefined from backend
        console.info(`[customFieldStore] Definitions loaded successfully for projectId ${projectId}. Count:`, definitions?.length || 0);
    } catch (err) {
        const errorMessage = err.message || String(err);
        console.error(`[customFieldStore] Error loading definitions for projectId ${projectId}:`, errorMessage);
        definitionError.set(errorMessage);
        customFieldDefinitions.set([]); // Clear definitions on error
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
 * @returns {Promise<{success: boolean}>} A promise that resolves to an object indicating success.
 * @throws {Error} If the backend command fails or projectId is not found, an error is thrown.
 */
export async function addDefinition(fieldKey, fieldName, fieldType, scopeStr) { // Removed defaultValue
    const projectId = getCurrentProjectId();

    if (!projectId) {
        const errorMsg = "Cannot add definition: No active project selected or project ID could not be determined.";
        console.error(`[customFieldStore] ${errorMsg}`);
        throw new Error(errorMsg);
    }

    console.debug(`[customFieldStore] Attempting to add definition for projectId ${projectId}: key='${fieldKey}', name='${fieldName}', type='${fieldType}', scope='${scopeStr}'`);
    try {
        await invoke('create_custom_field_definition_command', {
            projectId, // Added projectId
            fieldKey,
            fieldName,
            fieldType,
            scopeStr
            // defaultValue field removed from payload
        });
        console.info(`[customFieldStore] Definition added successfully for projectId ${projectId}, key: ${fieldKey}`);
        await loadAllDefinitions(); // Refresh the list
        return { success: true };
    } catch (err) {
        const errorMessage = err.message || String(err);
        console.error(`[customFieldStore] Error adding definition for projectId ${projectId}, key ${fieldKey}:`, errorMessage);
        throw new Error(errorMessage); // Propagate error
    }
}

/**
 * Deletes a custom field definition via a backend command and then refreshes the list.
 * @param {string} fieldKey - The unique key for the field to be deleted.
 * @returns {Promise<{success: boolean}>} A promise that resolves to an object indicating success.
 * @throws {Error} If the backend command fails or projectId is not found, an error is thrown.
 */
export async function deleteDefinition(fieldKey) {
    const projectId = getCurrentProjectId();

    if (!projectId) {
        const errorMsg = `[customFieldStore] Cannot delete definition: No active project or project ID could not be determined for fieldKey: ${fieldKey}`;
        console.error(errorMsg);
        throw new Error("Cannot delete definition: Project ID not found."); // User-facing
    }

    console.debug(`[customFieldStore] Attempting to delete definition for projectId ${projectId}, key: '${fieldKey}'`);
    try {
        await invoke('delete_custom_field_definition_command', {
            projectId,
            fieldKey
        });
        console.info(`[customFieldStore] Definition deleted successfully for projectId ${projectId}, key: ${fieldKey}`);
        await loadAllDefinitions(); // Refresh the list
        return { success: true };
    } catch (err) {
        const errorMessage = err.message || String(err);
        console.error(`[customFieldStore] Error deleting definition for projectId ${projectId}, key ${fieldKey}:`, errorMessage);
        throw new Error(`Failed to delete custom field '${fieldKey}': ${errorMessage}`);
    }
}

// Example of how to initialize the store when the app loads,
// though this might be better placed in a root component like App.svelte or a layout file.
// loadAllDefinitions(); // Auto-load on store initialization - commented out, should be called from UI layer.
