import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { project } from '$lib/stores/projectStore.js';

/**
 * @typedef {object} Tag
 * @property {number} id
 * @property {string} name
 * @property {string | null} color
 */

// Writable store for holding all tags in the project.
// This will hold an array of Tag objects.
export const allTags = writable([]);

/**
 * Fetches all tags for the current project from the database and updates the store.
 */
export async function fetchAllTags() {
    const proj = get(project);
    if (!proj || !proj.id) {
        console.warn('[tagStore] fetchAllTags called without a project ID.');
        allTags.set([]);
        return;
    }

    try {
        const tagsFromDb = await invoke('get_all_tags', { projectId: proj.id });
        allTags.set(tagsFromDb || []);
    } catch (error) {
        console.error('[tagStore] Failed to fetch tags:', error);
        allTags.set([]);
    }
}

/**
 * Adds a new tag to the database if it doesn't already exist, then refreshes the store.
 * @param {string} newTagName - The name of the new tag to add.
 */
export async function addTag(newTagName) {
    if (!newTagName || typeof newTagName !== 'string' || newTagName.trim() === '') {
        return;
    }
    const tagToAdd = newTagName.trim();

    const currentTags = get(allTags);
    if (currentTags.some(tag => tag.name.toLowerCase() === tagToAdd.toLowerCase())) {
        // Tag already exists, no need to add it again.
        return;
    }

    const proj = get(project);
    if (!proj || !proj.id) {
        console.error('[tagStore] addTag called without a project ID.');
        return;
    }

    try {
        await invoke('add_tag', {
            projectId: proj.id,
            name: tagToAdd,
            color: null // Default color, can be changed later
        });

        // After adding, refresh the entire list to ensure consistency.
        await fetchAllTags();
    } catch (error) {
        console.error(`[tagStore] Failed to add new tag "${tagToAdd}":`, error);
    }
}

/**
 * Updates an existing tag in the database.
 * @param {number} tagId - The ID of the tag to update.
 * @param {string} newName - The new name for the tag.
 * @param {string | null} newColor - The new color for the tag.
 */
export async function updateTag(tagId, newName, newColor) {
    const proj = get(project);
    if (!proj || !proj.id) {
        console.error('[tagStore] updateTag called without a project ID.');
        return;
    }

    try {
        await invoke('update_tag', {
            projectId: proj.id,
            tagId: tagId,
            newName: newName,
            color: newColor
        });
        await fetchAllTags();
    } catch (error) {
        console.error(`[tagStore] Failed to update tag ${tagId}:`, error);
        throw error; // Re-throw to allow the component to handle it
    }
}

/**
 * Deletes a tag from the database.
 * @param {number} tagId - The ID of the tag to delete.
 */
export async function deleteTag(tagId) {
    const proj = get(project);
    if (!proj || !proj.id) {
        console.error('[tagStore] deleteTag called without a project ID.');
        return;
    }

    try {
        await invoke('delete_tag', {
            projectId: proj.id,
            tagId: tagId
        });
        await fetchAllTags();
    } catch (error) {
        console.error(`[tagStore] Failed to delete tag ${tagId}:`, error);
        throw error; // Re-throw to allow the component to handle it
    }
}
