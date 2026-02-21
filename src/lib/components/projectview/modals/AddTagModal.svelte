<script>
    import { createEventDispatcher } from 'svelte';

    export let showModal = false;

    const dispatch = createEventDispatcher();

    let name = '';
    let description = '';
    let isLoading = false;
    let errorMessage = '';

    function closeModal() {
        if (isLoading) return;
        dispatch('close');
    }

    async function handleSave() {
        if (!name.trim()) {
            errorMessage = 'Tag name is required.';
            return;
        }
        errorMessage = '';
        isLoading = true;
        try {
            dispatch('save', {
                name: name.trim(),
                description: description.trim()
            });
        } catch (error) {
            errorMessage = `Failed to create tag: ${error.message}`;
        } finally {
            isLoading = false;
        }
    }
</script>

{#if showModal}
<div class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50 p-4" on:click={closeModal}>
    <div class="bg-white dark:bg-gray-900 p-6 rounded-lg shadow-xl w-full max-w-md" on:click|stopPropagation>
        <h2 class="text-lg font-semibold mb-4 text-gray-800 dark:text-gray-100">Add Tag</h2>

        {#if errorMessage}
            <div class="mb-4 p-3 bg-red-100 dark:bg-red-700 border border-red-300 dark:border-red-600 text-red-700 dark:text-red-100 rounded-md text-sm">
                {errorMessage}
            </div>
        {/if}

        <form on:submit|preventDefault={handleSave}>
            <div class="mb-4">
                <label for="tagName" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Tag Name</label>
                <input
                    type="text"
                    id="tagName"
                    bind:value={name}
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white sm:text-sm"
                    required
                    autocomplete="off"
                    placeholder="e.g., Important, Review"
                />
            </div>

            <div class="mb-6">
                <label for="tagDescription" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Description (Optional)</label>
                <textarea
                    id="tagDescription"
                    bind:value={description}
                    rows="3"
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white sm:text-sm"
                    placeholder="Describe this tag..."
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
                    disabled={isLoading || !name.trim()}
                    class="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:focus:ring-offset-gray-800 disabled:opacity-50"
                >
                    {#if isLoading}
                        Creating...
                    {:else}
                        Create Tag
                    {/if}
                </button>
            </div>
        </form>
    </div>
</div>
{/if}
