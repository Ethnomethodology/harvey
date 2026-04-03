import { invoke } from '@tauri-apps/api/core';
import { v4 as uuidv4 } from 'uuid';
import { project } from '$lib/stores/projectStore.js';
import { triggerRefresh } from '$lib/stores/refresherStore.js';
import { get } from 'svelte/store'; // Still needed for projectStore until it's migrated

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

// Svelte 5 State object for all tag-related data
export const tagStore = $state({
  allTags: [],
  allTagGroups: [],
  selectedTag: null,
  selectedTagGroup: null,
  tagInfo: null,
  tagSearchQuery: ''
});

/**
 * Fetches all tags and tag groups for the current project from the database and updates the store.
 */
export async function fetchAllTags() {
  const proj = get(project);
  if (!proj || !proj.id) {
    console.warn('[tagStore] fetchAllTags called without a project ID.');
    tagStore.allTags = [];
    tagStore.allTagGroups = [];
    return;
  }

  try {
    const [tagsFromDb, groupsFromDb] = await Promise.all([
      invoke('get_all_tags', { projectId: proj.id }),
      invoke('get_tag_groups', { projectId: proj.id })
    ]);

    tagStore.allTags = tagsFromDb || [];
    tagStore.allTagGroups = groupsFromDb || [];

    // Validate selected tag still exists
    if (tagStore.selectedTag && !tagsFromDb.some((t) => t.id === tagStore.selectedTag.id)) {
      tagStore.selectedTag = null;
      if (!tagStore.selectedTagGroup) {
        tagStore.tagInfo = null;
      }
    }

    // Validate selected group still exists
    if (tagStore.selectedTagGroup && !groupsFromDb.some((g) => g.id === tagStore.selectedTagGroup.id)) {
      tagStore.selectedTagGroup = null;
      if (!tagStore.selectedTag) {
        tagStore.tagInfo = null;
      }
    }

    // If we have a selected tag, refresh its info as well (to get highlight count etc potentially updated)
    if (tagStore.selectedTag) {
      // Re-fetch info to ensure consistency
      selectTag(tagStore.selectedTag);
    } else if (tagStore.selectedTagGroup) {
      selectTagGroup(tagStore.selectedTagGroup);
    }
  } catch (error) {
    console.error('[tagStore] Failed to fetch tags/groups:', error);
    tagStore.allTags = [];
    tagStore.allTagGroups = [];
  }
}

/**
 * Sets the selected tag and fetches its info.
 */
export async function selectTag(tag) {
  if (!tag) {
    tagStore.selectedTag = null;
    if (!tagStore.selectedTagGroup) tagStore.tagInfo = null;
    return;
  }

  tagStore.selectedTag = tag;
  tagStore.selectedTagGroup = null; // Deselect group
  tagStore.tagInfo = null; // Clear previous info while loading

  const proj = get(project);
  try {
    const info = await invoke('get_tag_info', {
      projectId: proj.id,
      tagId: tag.id,
      tagName: tag.name
    });
    tagStore.tagInfo = info;
  } catch (error) {
    console.error(`[tagStore] Failed to load tag info for ${tag.name}:`, error);
    tagStore.tagInfo = null;
  }
}

/**
 * Sets the selected tag group and fetches its info.
 */
export async function selectTagGroup(group) {
  if (!group) {
    tagStore.selectedTagGroup = null;
    if (!tagStore.selectedTag) tagStore.tagInfo = null;
    return;
  }

  tagStore.selectedTagGroup = group;
  tagStore.selectedTag = null; // Deselect tag
  tagStore.tagInfo = null; // Clear previous info while loading

  const proj = get(project);
  try {
    const info = await invoke('get_tag_group_info', {
      projectId: proj.id,
      groupId: group.id
    });
    tagStore.tagInfo = info;
  } catch (error) {
    console.error(`[tagStore] Failed to load tag group info for ${group.name}:`, error);
    tagStore.tagInfo = null;
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

  if (tagStore.allTags.some((tag) => tag.name.toLowerCase() === tagToAdd.toLowerCase())) {
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
  const originalTag = tagStore.allTags.find((tag) => tag.id === tagId);
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
        newName: newName
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

  const tagsInGroup = tagStore.allTags.filter((t) => t.tag_group_id === groupId);

  try {
    // Remove each tag from highlights globally
    for (const tag of tagsInGroup) {
      await invoke('remove_tag_globally', {
        projectId: proj.id,
        tagName: tag.name
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
  const tagToDelete = tagStore.allTags.find((tag) => tag.id === tagId);
  if (!tagToDelete) {
    throw new Error('Tag not found.');
  }
  const tagName = tagToDelete.name;

  try {
    // First, remove the tag from all highlights that use it
    await invoke('remove_tag_globally', {
      projectId: proj.id,
      tagName: tagName
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
