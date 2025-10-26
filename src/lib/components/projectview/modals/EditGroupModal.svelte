<script>
    import { createEventDispatcher, onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { message } from '@tauri-apps/plugin-dialog';

    export let showModal = false;
    export let groupData = null; // Expected: { id, project_id, name, description }

    let currentName = '';
    let currentDescription = '';
    let isLoading = false;
    let errorMessage = '';

    const dispatch = createEventDispatcher();

    // onMount is not strictly needed for this $: block, but doesn't harm
    onMount(() => {
        // Initialization logic handled by the reactive block below
    });

    // Reactive block to update form fields when groupData changes or modal becomes visible
    $: if (groupData && showModal) {
        currentName = groupData.name || '';
        currentDescription = groupData.description || '';
        errorMessage = ''; // Clear previous errors when modal reopens or groupData changes
    } else if (!showModal) {
        // Optional: Clear fields when modal is not shown to prevent stale data if groupData is nulled out later
        // currentName = '';
        // currentDescription = '';
        // errorMessage = '';
    }


    async function handleSave() {
        if (!groupData || !groupData.id || !groupData.project_id) {
            errorMessage = "Group data is incomplete. Cannot save.";
            return;
        }
        if (!currentName.trim()) {
            errorMessage = "Group name cannot be empty.";
            return;
        }

        isLoading = true;
        errorMessage = '';

        try {
            const updatedGroup = await invoke('update_group_details', {
                projectId: groupData.project_id,
                groupId: groupData.id,
                name: currentName.trim(),
                description: currentDescription.trim() === '' ? null : currentDescription.trim()
            });
            dispatch('groupUpdated', updatedGroup);
            closeModal();
        } catch (err) {
            console.error("Error updating group:", err);
            errorMessage = typeof err === 'string' ? err : "Failed to update group.";
            // Specific check for unique constraint error from backend
            if (typeof err === 'string' && err.toLowerCase().includes("a group with this name already exists")) {
                errorMessage = err; // Use the backend's specific message
            }
        } finally {
            isLoading = false;
        }
    }

    function closeModal() {
        // Fields are reset by the $: block when showModal becomes false if groupData is also cleared by parent,
        // or when new groupData is passed upon reopening.
        dispatch('close');
    }
</script>

{#if showModal}
<div class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50 p-4" on:click={closeModal}>
    <div class="bg-white dark:bg-surface-2 p-6 rounded-lg shadow-xl w-full max-w-md" on:click|stopPropagation>
        <h2 class="text-lg font-semibold mb-4 text-gray-800 dark:text-gray-100">Edit Group</h2>

        {#if errorMessage}
            <div class="mb-4 p-3 bg-red-100 dark:bg-red-700 border border-red-300 dark:border-red-600 text-red-700 dark:text-red-100 rounded-md text-sm">
                {errorMessage}
            </div>
        {/if}

        <form on:submit|preventDefault={handleSave}>
            <div class="mb-4">
                <label for="editGroupName" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Group Name</label>
                <input
                    type="text"
                    id="editGroupName"
                    bind:value={currentName}
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white sm:text-sm"
                    required
                    autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false"
                />
            </div>

            <div class="mb-6">
                <label for="editGroupDescription" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Description (Optional)</label>
                <textarea
                    id="editGroupDescription"
                    bind:value={currentDescription}
                    rows="3"
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white sm:text-sm"
                    autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false"
                ></textarea>
            </div>

            <div class="flex justify-end space-x-3">
                <button
                    type="button"
                    on:click={closeModal}
                    disabled={isLoading}
                    class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-200 bg-gray-100 dark:bg-gray-600 hover:bg-gray-200 dark:hover:bg-gray-500 rounded-md border border-gray-300 dark:border-gray-500 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:focus:ring-offset-gray-800 disabled:opacity-50"
                >
                    Cancel
                </button>
                <button
                    type="submit"
                    disabled={isLoading || !currentName.trim()}
                    class="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:focus:ring-offset-gray-800 disabled:opacity-50"
                >
                    {#if isLoading}
                        Saving...
                    {:else}
                        Save Changes
                    {/if}
                </button>
            </div>
        </form>
    </div>
</div>
{/if}
