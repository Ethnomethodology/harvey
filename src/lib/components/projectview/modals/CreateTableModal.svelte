<!-- src/lib/components/projectview/modals/CreateTableModal.svelte -->
<script>
    import { createEventDispatcher } from 'svelte';
    import { project } from '$lib/stores/projectStore.js';
    import { invoke } from '@tauri-apps/api/core';

    export let showModal = false;

    const dispatch = createEventDispatcher();

    let step = 1;
    let numFields = 1;
    let fields = [];

    function goToStep2() {
        fields = Array.from({ length: numFields }, (_, i) => `Field-${i + 1}`);
        step = 2;
    }

    async function handleSubmit() {
        const nonEmptyFields = fields.filter(f => f.trim() !== '');
        if (nonEmptyFields.length === 0) {
            alert('Please provide at least one column name.');
            return;
        }

        try {
            const newTablePath = await invoke('create_new_table', {
                projectXmlPath: $project.xmlPath,
                headers: nonEmptyFields
            });
            closeModal();
            dispatch('tableCreated', { path: newTablePath });
        } catch (error) {
            console.error('Error creating new table:', error);
            alert(`Error creating table: ${error.message || error}`);
        }
    }

    function closeModal() {
        step = 1;
        numFields = 1;
        fields = [];
        showModal = false;
        dispatch('close');
    }
</script>

{#if showModal}
<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
    <div class="bg-white dark:bg-gray-700 p-6 rounded-lg shadow-xl w-full max-w-md">
        {#if step === 1}
            <h3 class="text-lg font-bold mb-4">Create New Table</h3>
            <label for="num-fields-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                How many columns?
            </label>
            <input
                id="num-fields-input"
                type="number"
                bind:value={numFields}
                min="1"
                class="mt-1 block w-full px-3 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-border rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
                autocomplete="off"
                autocorrect="off"
            />
            <div class="mt-6 flex justify-end space-x-2">
                <button
                    class="px-4 py-2 text-sm font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-2 text-gray-700 bg-gray-200 hover:bg-gray-300 dark:bg-gray-600 dark:hover:bg-gray-500"
                    on:click={closeModal}
                >
                    Cancel
                </button>
                <button
                    class="px-4 py-2 text-sm font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-2 text-white bg-blue-600 hover:bg-blue-700"
                    on:click={goToStep2}
                    disabled={numFields < 1}
                >
                    Next
                </button>
            </div>
        {:else if step === 2}
            <h3 class="text-lg font-bold mb-4">Define Column Names</h3>
            <div class="space-y-2 max-h-64 overflow-y-auto">
                {#each fields as _, i}
                    <input
                        type="text"
                        bind:value={fields[i]}
                        class="block w-full px-3 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-border rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
                        autocomplete="off"
                        autocorrect="off"
                    />
                {/each}
            </div>
            <div class="mt-6 flex justify-between">
                <button
                    class="px-4 py-2 text-sm font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-2 text-gray-700 bg-gray-200 hover:bg-gray-300 dark:bg-gray-600 dark:hover:bg-gray-500"
                    on:click={() => step = 1}
                >
                    Back
                </button>
                <div class="flex space-x-2">
                    <button
                        class="px-4 py-2 text-sm font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-2 text-gray-700 bg-gray-200 hover:bg-gray-300 dark:bg-gray-600 dark:hover:bg-gray-500"
                        on:click={closeModal}
                    >
                        Cancel
                    </button>
                    <button
                        class="px-4 py-2 text-sm font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-2 text-white bg-blue-600 hover:bg-blue-700"
                        on:click={handleSubmit}
                    >
                        Create Table
                    </button>
                </div>
            </div>
        {/if}
    </div>
</div>
{/if}
