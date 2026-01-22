import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { v4 as uuidv4 } from 'uuid';
import { project } from '$lib/stores/projectStore.js';
import { triggerRefresh } from '$lib/stores/refresherStore.js';

/**
 * @typedef {object} Tag
 * @property {number} id
 * @property {string} name
 * @property {string | null} color
 * @property {string | null} description
 * @property {string | null} tag_group_id
 */

/**
 * @typedef {object} TagGroup
 * @property {string} id
 * @property {string} name
 * @property {string | null} description
 */

// Writable store for holding all tags in the project.
export const allTags = writable([]);
// Writable store for holding all tag groups in the project.
export const allTagGroups = writable([]);

// Store for the currently selected tag in the Tags view
export const selectedTag = writable(null);
// Store for the currently selected tag group in the Tags view
export const selectedTagGroup = writable(null);

// Store for the details of the currently selected tag or tag group
export const tagInfo = writable(null);
// Store for the search query in the Tags view
export const tagSearchQuery = writable('');

/**
 * Fetches all tags and tag groups for the current project from the database and updates the store.
 */
export async function fetchAllTags() {
    const proj = get(project);
    if (!proj || !proj.id) {
        console.warn('[tagStore] fetchAllTags called without a project ID.');
        allTags.set([]);
        allTagGroups.set([]);
        return;
    }

    try {
        const [tagsFromDb, groupsFromDb] = await Promise.all([
            invoke('get_all_tags', { projectId: proj.id }),
            invoke('get_tag_groups', { projectId: proj.id })
        ]);

        allTags.set(tagsFromDb || []);
        allTagGroups.set(groupsFromDb || []);

        // Validate selected tag still exists
        const currentSelectedTag = get(selectedTag);
        if (currentSelectedTag && !tagsFromDb.some(t => t.id === currentSelectedTag.id)) {
            selectedTag.set(null);
            if (!get(selectedTagGroup)) {
                tagInfo.set(null);
            }
        }

        // Validate selected group still exists
        const currentSelectedGroup = get(selectedTagGroup);
        if (currentSelectedGroup && !groupsFromDb.some(g => g.id === currentSelectedGroup.id)) {
            selectedTagGroup.set(null);
            if (!get(selectedTag)) {
                tagInfo.set(null);
            }
        }

        // If we have a selected tag, refresh its info as well (to get highlight count etc potentially updated)
        if (currentSelectedTag) {
             // Re-fetch info to ensure consistency
             selectTag(currentSelectedTag);
        } else if (currentSelectedGroup) {
             selectTagGroup(currentSelectedGroup);
        }

    } catch (error) {
        console.error('[tagStore] Failed to fetch tags/groups:', error);
        allTags.set([]);
        allTagGroups.set([]);
    }
}

/**
 * Sets the selected tag and fetches its info.
 */
export async function selectTag(tag) {
    if (!tag) {
        selectedTag.set(null);
        if (!get(selectedTagGroup)) tagInfo.set(null);
        return;
    }

    selectedTag.set(tag);
    selectedTagGroup.set(null); // Deselect group
    tagInfo.set(null); // Clear previous info while loading

    const proj = get(project);
    try {
        const info = await invoke('get_tag_info', {
            projectId: proj.id,
            tagId: tag.id,
            tagName: tag.name,
        });
        tagInfo.set(info);
    } catch (error) {
        console.error(`[tagStore] Failed to load tag info for ${tag.name}:`, error);
        tagInfo.set(null);
    }
}

/**
 * Sets the selected tag group and fetches its info.
 */
export async function selectTagGroup(group) {
    if (!group) {
        selectedTagGroup.set(null);
        if (!get(selectedTag)) tagInfo.set(null);
        return;
    }

    selectedTagGroup.set(group);
    selectedTag.set(null); // Deselect tag
    tagInfo.set(null); // Clear previous info while loading

    const proj = get(project);
    try {
        const info = await invoke('get_tag_group_info', {
            projectId: proj.id,
            groupId: group.id
        });
        tagInfo.set(info);
    } catch (error) {
        console.error(`[tagStore] Failed to load tag group info for ${group.name}:`, error);
        tagInfo.set(null);
    }
}

/**
 * Adds a new tag to the database if it doesn't already exist, then refreshes the store.
 * @param {string} newTagName
 * @param {string} [description]
 * @param {string} [tagGroupId]
 */
export async function addTag(newTagName, description = null, tagGroupId = null) {
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
            color: null,
            description: description,
            tagGroupId: tagGroupId
        });

        // After adding, refresh the entire list to ensure consistency.
        await fetchAllTags();
    } catch (error) {
        console.error(`[tagStore] Failed to add new tag "${tagToAdd}":`, error);
        throw error;
    }
}

/**
 * Updates an existing tag in the database.
 * @param {number} tagId
 * @param {string} newName
 * @param {string | null} newColor
 * @param {string | null} description
 * @param {string | null} tagGroupId
 */
export async function updateTag(tagId, newName, newColor, description = null, tagGroupId = null) {
    const proj = get(project);
    if (!proj || !proj.id) {
        console.error('[tagStore] updateTag called without a project ID.');
        return;
    }

    // Find the original tag name before updating
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
            description: description,
            tagGroupId: tagGroupId
        });

        // After successfully renaming the tag, cascade the change to all highlights
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
        throw error; // Re-throw to allow the component to handle it
    }
}

export async function createTagGroup(name, description) {
    const proj = get(project);
    if (!proj || !proj.id) return;

    const groupId = uuidv4();
    try {
        await invoke('create_tag_group', {
            projectId: proj.id,
            groupId,
            name,
            description
        });
        await fetchAllTags();
    } catch (error) {
        console.error('[tagStore] Failed to create tag group:', error);
        throw error;
    }
}

export async function updateTagGroup(groupId, name, description) {
    const proj = get(project);
    if (!proj || !proj.id) return;

    try {
        await invoke('update_tag_group', {
            projectId: proj.id,
            groupId,
            name,
            description
        });
        await fetchAllTags();
        triggerRefresh();
    } catch (error) {
        console.error('[tagStore] Failed to update tag group:', error);
        throw error;
    }
}

export async function deleteTagGroup(groupId) {
    const proj = get(project);
    if (!proj || !proj.id) return;

    // Get all tags in this group to remove them from highlights first
    // Note: The backend delete_tag_group cascades deletion of tags from DB, but we need to handle highlight text removal if necessary.
    // However, the requirement is "show warning that tags under them will be deleted but highlights will remain".
    // This implies we should treat it like deleting each tag individually.
    // Currently deleteTag handles global removal. We should probably do that for each tag in the group.

    const tags = get(allTags);
    const tagsInGroup = tags.filter(t => t.tag_group_id === groupId);

    try {
        // Remove each tag from highlights globally
        for (const tag of tagsInGroup) {
             await invoke('remove_tag_globally', {
                projectId: proj.id,
                tagName: tag.name,
            });
        }

        await invoke('delete_tag_group', {
            projectId: proj.id,
            groupId
        });
        await fetchAllTags();
        triggerRefresh();
    } catch (error) {
        console.error('[tagStore] Failed to delete tag group:', error);
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

    // Find the tag name before deleting
    const tags = get(allTags);
    const tagToDelete = tags.find(tag => tag.id === tagId);
    if (!tagToDelete) {
        throw new Error('Tag not found.');
    }
    const tagName = tagToDelete.name;

    try {
        // First, remove the tag from all highlights that use it
        await invoke('remove_tag_globally', {
            projectId: proj.id,
            tagName: tagName,
        });

        // Then, delete the tag itself from the central list
        await invoke('delete_tag', {
            projectId: proj.id,
            tagId: tagId
        });

        await fetchAllTags();
        triggerRefresh();
    } catch (error) {
        console.error(`[tagStore] Failed to delete tag ${tagId}:`, error);
        throw error; // Re-throw to allow the component to handle it
    }
}
