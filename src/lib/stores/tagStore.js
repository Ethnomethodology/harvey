import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { project } from '$lib/stores/projectStore.js';
import { triggerRefresh } from '$lib/stores/refresherStore.js';

/**
 * @typedef {object} Tag
 * @property {number} id
 * @property {string} name
 * @property {string | null} color
 * @property {string | null} description
 * @property {number | null} tag_group_id
 */

/**
 * @typedef {object} TagGroup
 * @property {number} id
 * @property {string} name
 * @property {string | null} description
 */

// Writable store for holding all tags in the project.
export const allTags = writable([]);

// Writable store for holding all tag groups in the project.
export const tagGroups = writable([]);

// Writable store for holding the currently selected tag/group in TagsView
// Structure: { type: 'tag' | 'group' | null, id: number | null }
export const selectedTagState = writable({ type: null, id: null });

/**
 * Fetches all tags and tag groups for the current project from the database and updates the stores.
 */
export async function fetchAllTags() {
    const proj = get(project);
    if (!proj || !proj.id) {
        console.warn('[tagStore] fetchAllTags called without a project ID.');
        allTags.set([]);
        tagGroups.set([]);
        return;
    }

    try {
        const [tagsFromDb, groupsFromDb] = await Promise.all([
            invoke('get_all_tags', { projectId: proj.id }),
            invoke('get_all_tag_groups', { projectId: proj.id })
        ]);

        allTags.set(tagsFromDb || []);
        tagGroups.set(groupsFromDb || []);
    } catch (error) {
        console.error('[tagStore] Failed to fetch tags/groups:', error);
        allTags.set([]);
        tagGroups.set([]);
    }
}

/**
 * Adds a new tag to the database if it doesn't already exist, then refreshes the store.
 * @param {string} newTagName - The name of the new tag to add.
 * @param {string|null} [description] - Optional description.
 * @param {number|null} [groupId] - Optional group ID.
 */
export async function addTag(newTagName, description = null, groupId = null) {
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
            color: null, // Default color
            description: description,
            tagGroupId: groupId
        });

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
 * @param {string | null} newDescription - The new description.
 * @param {number | null} newGroupId - The new group ID.
 */
export async function updateTag(tagId, newName, newColor, newDescription = null, newGroupId = null) {
    const proj = get(project);
    if (!proj || !proj.id) {
        console.error('[tagStore] updateTag called without a project ID.');
        return;
    }

    const tags = get(allTags);
    const originalTag = tags.find(tag => tag.id === tagId);
    if (!originalTag) {
        throw new Error('Tag not found.');
    }
    const oldName = originalTag.name;

    try {
        await invoke('update_tag', {
            projectId: proj.id,
            tagId: tagId,
            newName: newName,
            color: newColor,
            description: newDescription,
            tagGroupId: newGroupId
        });

        if (oldName !== newName) {
            await invoke('rename_tag_in_highlights', {
                projectId: proj.id,
                oldName: oldName,
                newName: newName,
            });
        }

        await fetchAllTags();
        triggerRefresh();
    } catch (error) {
        console.error(`[tagStore] Failed to update tag ${tagId}:`, error);
        throw error;
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

    const tags = get(allTags);
    const tagToDelete = tags.find(tag => tag.id === tagId);
    if (!tagToDelete) {
        throw new Error('Tag not found.');
    }
    const tagName = tagToDelete.name;

    try {
        await invoke('remove_tag_globally', {
            projectId: proj.id,
            tagName: tagName,
        });

        await invoke('delete_tag', {
            projectId: proj.id,
            tagId: tagId
        });

        await fetchAllTags();
        triggerRefresh();
    } catch (error) {
        console.error(`[tagStore] Failed to delete tag ${tagId}:`, error);
        throw error;
    }
}

// --- Group Functions ---

export async function addTagGroup(name, description = null) {
    if (!name || !name.trim()) return;
    const proj = get(project);
    if (!proj || !proj.id) return;

    try {
        await invoke('create_tag_group', {
            projectId: proj.id,
            name: name.trim(),
            description: description
        });
        await fetchAllTags();
    } catch (error) {
        console.error('[tagStore] Failed to create tag group:', error);
        throw error;
    }
}

export async function updateTagGroup(groupId, name, description = null) {
    const proj = get(project);
    if (!proj || !proj.id) return;

    try {
        await invoke('update_tag_group', {
            projectId: proj.id,
            groupId: groupId,
            name: name.trim(),
            description: description
        });
        await fetchAllTags();
    } catch (error) {
        console.error('[tagStore] Failed to update tag group:', error);
        throw error;
    }
}

export async function deleteTagGroup(groupId) {
    const proj = get(project);
    if (!proj || !proj.id) return;

    try {
        await invoke('delete_tag_group', {
            projectId: proj.id,
            groupId: groupId
        });
        await fetchAllTags();
    } catch (error) {
        console.error('[tagStore] Failed to delete tag group:', error);
        throw error;
    }
}

export async function moveTagToGroup(tagId, groupId) {
    const proj = get(project);
    if (!proj || !proj.id) return;

    try {
        await invoke('move_tag_to_group', {
            projectId: proj.id,
            tagId: tagId,
            groupId: groupId
        });
        // We don't necessarily need a full fetch if we update the local store smartly,
        // but full fetch is safer and easier.
        await fetchAllTags();
    } catch (error) {
        console.error('[tagStore] Failed to move tag to group:', error);
        throw error;
    }
}
