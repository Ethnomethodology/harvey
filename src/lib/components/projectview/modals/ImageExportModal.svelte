<script>
    import { createEventDispatcher } from 'svelte';
    import { save } from '@tauri-apps/plugin-dialog';

    export let showModal = false;

    const dispatch = createEventDispatcher();

    let exportFormat = 'png';
    let includeAnnotations = true;
    let filePath = '';

    async function handleBrowse() {
        const selected = await save({
            title: 'Export Image',
            filters: [{
                name: 'Image',
                extensions: [exportFormat]
            }]
        });
        if (selected) {
            filePath = selected;
        }
    }

    function handleConfirm() {
        if (!filePath) return;
        dispatch('confirm', {
            filePath,
            format: exportFormat,
            includeAnnotations
        });
        showModal = false;
        filePath = '';
    }

    function handleCancel() {
        showModal = false;
        filePath = '';
    }
</script>

{#if showModal}
<div class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50">
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-full max-w-md p-6">
        <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Export Image</h2>

        <div class="space-y-4">
            <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Format</label>
                <select bind:value={exportFormat} class="w-full border-gray-300 dark:border-gray-600 rounded-md shadow-sm dark:bg-gray-700 dark:text-white">
                    <option value="png">PNG</option>
                    <option value="jpg">JPEG</option>
                </select>
            </div>

            <div class="flex items-center">
                <input id="include-annotations" type="checkbox" bind:checked={includeAnnotations} class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded">
                <label for="include-annotations" class="ml-2 block text-sm text-gray-900 dark:text-gray-300">
                    Include Annotations (censored, text, bubbles)
                </label>
            </div>

            <p class="text-xs text-gray-500 italic">Note: Standard highlights (Rectangles, Circles, Polygons) are not exported.</p>

            <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Destination</label>
                <div class="flex space-x-2">
                    <input type="text" readonly value={filePath} placeholder="Select destination..." class="flex-grow border-gray-300 dark:border-gray-600 rounded-md shadow-sm bg-gray-50 dark:bg-gray-700 dark:text-gray-400 text-sm px-3 py-2 cursor-not-allowed">
                    <button on:click={handleBrowse} class="px-3 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-md hover:bg-gray-300 dark:hover:bg-gray-600 text-sm">Browse</button>
                </div>
            </div>
        </div>

        <div class="mt-6 flex justify-end space-x-3">
            <button on:click={handleCancel} class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-md hover:bg-gray-50 dark:hover:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500">Cancel</button>
            <button on:click={handleConfirm} disabled={!filePath} class="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed">Export</button>
        </div>
    </div>
</div>
{/if}
