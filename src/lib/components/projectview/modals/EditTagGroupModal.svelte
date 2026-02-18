<script>
    import { createEventDispatcher, onMount } from 'svelte';
    import { confirm } from '@tauri-apps/plugin-dialog';

    export let showModal = false;
    export let group = null;

    const dispatch = createEventDispatcher();

    let name = '';
    let description = '';
    let isLoading = false;
    let errorMessage = '';

    onMount(() => {
        if (group) {
            name = group.name;
            description = group.description || '';
        }
    });

    function closeModal() {
        if (isLoading) return;
        dispatch('close');
    }

    async function handleSave() {
        if (!name.trim()) {
            errorMessage = 'Group name is required.';
            return;
        }
        errorMessage = '';
        isLoading = true;
        try {
            dispatch('save', {
                id: group.id,
                name: name.trim(),
                description: description.trim()
            });
        } catch (error) {
            errorMessage = `Failed to update group: ${error.message}`;
        } finally {
            isLoading = false;
        }
    }

    async function handleDelete() {
        const confirmed = await confirm(`Are you sure you want to delete the group "${group.name}"? All tags within this group will be deleted. Highlights will remain but will be untagged.`, {
            title: 'Confirm Group Deletion',
            type: 'warning',
        });

        if (confirmed) {
            isLoading = true;
            try {
                dispatch('delete', { id: group.id });
            } catch (error) {
                errorMessage = `Failed to delete group: ${error.message}`;
            } finally {
                isLoading = false;
            }
        }
    }
</script>

{#if showModal}
<div class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50 p-4" on:click={closeModal}>
    <div class="bg-white dark:bg-gray-900 p-6 rounded-lg shadow-xl w-full max-w-md" on:click|stopPropagation>
        <h2 class="text-lg font-semibold mb-4 text-gray-800 dark:text-gray-100">Edit Tag Group</h2>

        {#if errorMessage}
            <div class="mb-4 p-3 bg-red-100 dark:bg-red-700 border border-red-300 dark:border-red-600 text-red-700 dark:text-red-100 rounded-md text-sm">
                {errorMessage}
            </div>
        {/if}

        <form on:submit|preventDefault={handleSave}>
            <div class="mb-4">
                <label for="groupName" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Group Name</label>
                <input
                    type="text"
                    id="groupName"
                    bind:value={name}
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white sm:text-sm"
                    required
                    autocomplete="off"
                />
            </div>

            <div class="mb-6">
                <label for="groupDescription" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Description (Optional)</label>
                <textarea
                    id="groupDescription"
                    bind:value={description}
                    rows="3"
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white sm:text-sm"
                ></textarea>
            </div>

            <div class="flex justify-between items-center">
                <button
                    type="button"
                    on:click={handleDelete}
                    disabled={isLoading}
                    class="px-4 py-2 text-sm font-medium text-red-600 dark:text-red-400 hover:bg-red-100 dark:hover:bg-red-900/50 rounded-md border border-red-300 dark:border-red-600 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500 dark:focus:ring-offset-gray-800 disabled:opacity-50"
                >
                    Delete Group
                </button>
                <div class="flex space-x-3">
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
                        disabled={isLoading || !name.trim()}
                        class="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:focus:ring-offset-gray-800 disabled:opacity-50"
                    >
                        {#if isLoading}
                            Saving...
                        {:else}
                            Save Changes
                        {/if}
                    </button>
                </div>
            </div>
        </form>
    </div>
</div>
{/if}
