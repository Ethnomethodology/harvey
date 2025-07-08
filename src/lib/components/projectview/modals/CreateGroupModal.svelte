<script>
    import { createEventDispatcher } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { message } from '@tauri-apps/plugin-dialog'; // For error messages
    import { updateProjectGroupsList } from '$lib/stores/projectStore.js';

    export let showModal = false;
    export let projectUuid = null; // Passed in from parent
    export let fileToAdd = null;   // Passed in from parent (contains relativePath, name)

    let groupName = '';
    let groupDescription = '';
    let isSaving = false;

    import { onMount } from 'svelte'; // Standard Svelte onMount

    const dispatch = createEventDispatcher();

    onMount(() => {
    });

    function closeModal() {
        if (isSaving) return;
        showModal = false;
        groupName = '';
        groupDescription = '';
        dispatch('close');
    }

    async function handleSave() {
        console.log('[CreateGroupModal] handleSave triggered. fileToAdd object:', fileToAdd);
        if (fileToAdd) {
            console.log('[CreateGroupModal] fileToAdd.name:', fileToAdd.name);
            console.log('[CreateGroupModal] fileToAdd.path:', fileToAdd.path);
            console.log('[CreateGroupModal] fileToAdd.relativePath (camelCase):', fileToAdd.relativePath);
            console.log('[CreateGroupModal] fileToAdd.relative_path (snake_case):', fileToAdd.relative_path);
            console.log('[CreateGroupModal] fileToAdd.asset_relative_path:', fileToAdd.asset_relative_path);
            console.log('[CreateGroupModal] fileToAdd keys:', Object.keys(fileToAdd));
        } else {
            console.log('[CreateGroupModal] fileToAdd is null or undefined.');
        }
        if (!groupName.trim()) {
            await message('Group name cannot be empty.', { title: 'Validation Error', type: 'error' });
            return;
        }
        if (!projectUuid) {
            await message('Project context is missing.', { title: 'Error', type: 'error' });
            return;
        }

        isSaving = true;
        try {
            const newGroup = await invoke('create_new_group', {
                projectId: projectUuid,
                name: groupName.trim(),
                description: groupDescription.trim() || null,
                fileAssetRelativePath: fileToAdd ? (fileToAdd.relative_path || fileToAdd.relativePath) : null,
            });

            // newGroup is the GroupData returned from the backend command
            if (newGroup) { // If group creation was successful (backend returns GroupData)
                if (projectUuid) { // Ensure projectUuid is valid before updating list
                    await updateProjectGroupsList(projectUuid);
                }
                if (fileToAdd && (fileToAdd.relative_path || fileToAdd.relativePath)) {
                    // Even if backend handles association, dispatch this event for UI consistency
                    // if fileToAdd was intended. The backend log will show if association failed.
                    dispatch('groupCreatedAndFileAdded', { group: newGroup, file: fileToAdd });
                } else {
                    dispatch('groupCreated', { group: newGroup });
                }
            }
            // If newGroup is null or command failed, an error would have been thrown by invoke
            closeModal();
        } catch (err) {
            console.error('Error creating group:', err); // Error might include file association issues now
            await message(`Failed to create group: ${err}`, { title: 'Error', type: 'error' });
        } finally {
            isSaving = false;
        }
    }

    // Handle Escape key to close modal
    function handleKeydown(event) {
        if (event.key === 'Escape') {
            closeModal();
        }
    }
</script>

<svelte:window on:keydown={handleKeydown}/>

{#if showModal}
    <div class="fixed inset-0 z-[60] bg-gray-900 bg-opacity-50 dark:bg-opacity-75 flex items-center justify-center p-4" on:click={closeModal} role="dialog" aria-modal="true" tabindex="0" on:keydown={(e) => { if (e.key === 'Escape') closeModal(); }}>
        <div class="bg-white dark:bg-gray-800 p-5 rounded-lg shadow-xl w-full max-w-md" on:click|stopPropagation role="document" tabindex="-1">
            <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Create New Group</h2>

            <div class="space-y-4">
                <div>
                    <label for="groupName" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Group Name <span class="text-red-500">*</span></label>
                    <input type="text" id="groupName" bind:value={groupName} placeholder="Enter group name"
                           class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white sm:text-sm"
                           required
                           autocomplete="off"
                           autocorrect="off"
                           autocapitalize="off"
                           spellcheck="false" />
                </div>
                <div>
                    <label for="groupDescription" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Description (Optional)</label>
                    <textarea id="groupDescription" bind:value={groupDescription} rows="3" placeholder="Enter group description"
                              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white sm:text-sm"></textarea>
                </div>
            </div>

            <div class="mt-6 flex justify-end space-x-3">
                <button type="button" on:click={closeModal} disabled={isSaving}
                        class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-500 rounded-md shadow-sm hover:bg-gray-50 dark:hover:bg-gray-600 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50">
                    Cancel
                </button>
                <button type="button" on:click={handleSave} disabled={isSaving}
                        class="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 border border-transparent rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:bg-blue-400">
                    {#if isSaving}
                        Saving...
                    {:else}
                        Save Group
                    {/if}
                </button>
            </div>
        </div>
    </div>
{/if}
