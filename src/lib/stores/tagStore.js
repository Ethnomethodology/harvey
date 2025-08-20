import { writable } from 'svelte/store';

// Writable store for holding all tags in the project
export const allTags = writable([]);

/**
 * Initializes the tag store with a list of tags.
 * @param {string[]} tags - An array of tags to set.
 */
export function setTags(tags) {
    const uniqueTags = [...new Set(tags)];
    allTags.set(uniqueTags);
}

/**
 * Adds a new tag to the store if it doesn't already exist.
 * @param {string} newTag - The new tag to add.
 */
export function addTag(newTag) {
    if (!newTag || typeof newTag !== 'string' || newTag.trim() === '') {
        return;
    }
    const tagToAdd = newTag.trim();
    allTags.update(currentTags => {
        if (!currentTags.includes(tagToAdd)) {
            const newTags = [...currentTags, tagToAdd];
            console.log(`[tagStore] Adding new tag "${tagToAdd}". New tags:`, newTags);
            return newTags;
        }
        return currentTags;
    });
}
