<script>
    import { createEventDispatcher } from 'svelte';
    import { X } from 'lucide-svelte';

    export let showModal = false;

    const dispatch = createEventDispatcher();

    let selectedSourceType = 'msWord'; // Default, only option for now

    function handleConfirm() {
        if (selectedSourceType) {
            dispatch('confirm', { sourceType: selectedSourceType });
        }
    }

    function handleClose() {
        dispatch('close');
    }
</script>

{#if showModal}
    <div class="fixed inset-0 z-[120] bg-black/60 backdrop-blur-sm flex items-center justify-center p-4" on:click|self={handleClose} role="dialog" aria-modal="true" aria-labelledby="import-transcript-title" tabindex="0" on:keydown={(e) => { if (e.key === 'Escape') handleClose(); }}>
        <div class="bg-white dark:bg-surface-2 p-6 rounded-lg shadow-xl w-full max-w-md text-gray-900 dark:text-gray-200" on:click|stopPropagation role="document" tabindex="-1" on:keydown={(e) => { if (e.key === 'Escape') handleClose(); }}>
            <div class="flex justify-between items-center mb-4">
                <h2 id="import-transcript-title" class="text-lg font-semibold">Import Transcript From...</h2>
                <button on:click={handleClose} class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">
                    <X size={20} />
                </button>
            </div>

            <div class="space-y-4 mb-6">
                <p class="text-sm text-gray-600 dark:text-gray-400">
                    Select the source format of the transcript you want to import.
                    The transcript file should contain the name of the associated audio file and the transcript text.
                </p>
                <div>
                    <label for="sourceType" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Source Type</label>
                    <select
                        id="sourceType"
                        bind:value={selectedSourceType}
                        class="mt-1 block w-full pl-3 pr-10 py-2 text-base border-gray-300 dark:border-gray-600 dark:bg-gray-700 dark:text-gray-200 focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm rounded-md"
                    >
                        <option value="msWord">MS Word Document (.docx)</option>
                        <!-- Other options can be added here later -->
                    </select>
                </div>
            </div>

            <div class="flex justify-end space-x-3">
                <button
                    type="button"
                    class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 rounded-md border border-gray-300 dark:border-gray-500 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:focus:ring-offset-gray-800"
                    on:click={handleClose}
                >
                    Cancel
                </button>
                <button
                    type="button"
                    class="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md border border-transparent focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:focus:ring-offset-gray-800 disabled:opacity-50"
                    on:click={handleConfirm}
                    disabled={!selectedSourceType}
                >
                    Proceed
                </button>
            </div>
        </div>
    </div>
{/if}