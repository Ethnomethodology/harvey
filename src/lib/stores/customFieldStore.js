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
    const currentProject = get(project);
    if (currentProject && currentProject.xmlPath) {
        const path = currentProject.xmlPath.replace(/\\/g, '/'); // Normalize separators
        const parts = path.split('/');

        // Primary parsing strategy: Find 'projects' directory index
        const projectsDirIndex = parts.lastIndexOf('projects');

        if (projectsDirIndex !== -1 &&
            projectsDirIndex < parts.length - 2 && // Ensure there's a project name and project.xml after 'projects'
            parts[parts.length - 1].toLowerCase() === 'project.xml') {

            const projectId = parts[projectsDirIndex + 1]; // Project name is the segment after 'projects'

            if (projectId && projectId.trim() !== '') {
                console.debug(`[customFieldStore] Determined projectId: ${projectId} (using 'projects' segment)`);
                return projectId;
            }
        }

        // Fallback parsing strategy: Original logic (second to last part)
        // This is useful if the "projects" folder name isn't standard but the project name is still the parent of project.xml
        if (parts.length >= 2 && parts[parts.length - 1].toLowerCase() === 'project.xml') {
            const projectId = parts[parts.length - 2];
            if (projectId && projectId.trim() !== '') {
                // Log as warn if primary strategy failed but fallback worked
                if (projectsDirIndex === -1 || projectsDirIndex >= parts.length - 2) {
                     console.warn(`[customFieldStore] Determined projectId: "${projectId}" using fallback path parsing (path did not contain '/projects/[projectName]/project.xml' structure as expected). Path was: ${currentProject.xmlPath}`);
                } else {
                    // This case should ideally not be hit if primary logic for projectId.trim() worked.
                    // However, if primary logic found 'projects' but projectId was empty, and fallback found a non-empty one.
                    console.debug(`[customFieldStore] Determined projectId: "${projectId}" using fallback path parsing (primary 'projects' segment was empty/invalid). Path was: ${currentProject.xmlPath}`);
                }
                return projectId;
            }
        }

        console.error('[customFieldStore] Could not parse projectId from xmlPath after trying primary and fallback strategies:', currentProject.xmlPath);
        return null;
    }
    // Changed to warn because this state (no project loaded) can be normal (e.g. on app startup before a project is opened)
    console.warn('[customFieldStore] No active project or xmlPath found in project store when trying to get projectId.');
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

// Example of how to initialize the store when the app loads,
// though this might be better placed in a root component like App.svelte or a layout file.
// loadAllDefinitions(); // Auto-load on store initialization - commented out, should be called from UI layer.
