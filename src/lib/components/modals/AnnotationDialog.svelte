<script>
    import { createEventDispatcher } from 'svelte';

    const dispatch = createEventDispatcher();

    export let show = false;
    export let x = 0;
    export let y = 0;
    export let initialTitle = '';
    export let initialDescription = '';
    export let initialColor = 'rgba(255, 242, 117, 0.5)'; // Default yellow

    let title = initialTitle;
    let description = initialDescription;
    let selectedColor = initialColor;

    const highlightOptions = [
        { value: 'rgba(255, 242, 117, 0.5)', label: 'Yellow' },
        { value: 'rgba(168, 255, 158, 0.5)', label: 'Green' },
        { value: 'rgba(174, 239, 255, 0.5)', label: 'Blue' },
        { value: 'rgba(255, 176, 207, 0.5)', label: 'Pink' },
        { value: 'rgba(208, 160, 255, 0.5)', label: 'Purple' },
        { value: 'rgba(255, 255, 255, 0.5)', label: 'White' },
    ];

    function handleSave() {
        dispatch('save', { title, description, color: selectedColor });
        show = false;
    }

    function handleCancel() {
        dispatch('cancel');
        show = false;
    }

    $: if (show) {
        title = initialTitle;
        description = initialDescription;
        selectedColor = initialColor;
    }
</script>

{#if show}
<div
    class="annotation-dialog absolute z-[1001] bg-white dark:bg-d-gray-700 border border-gray-300 dark:border-border rounded-md shadow-xl p-4"
    style="left: {x}px; top: {y}px;"
    tabindex="-1"
    on:introstart={(e) => e.target.focus()}
>
    <h3 class="text-lg font-semibold mb-3 text-gray-900 dark:text-white">Annotation Details</h3>
    <div class="mb-3">
        <label for="annotation-title" class="block text-sm font-medium text-gray-700 dark:text-d-gray-300">Title (Optional)</label>
        <input
            type="text"
            id="annotation-title"
            class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-d-gray-800 dark:border-border dark:text-white"
            bind:value={title}
        />
    </div>
    <div class="mb-3">
        <label for="annotation-description" class="block text-sm font-medium text-gray-700 dark:text-d-gray-300">Description (Optional)</label>
        <textarea
            id="annotation-description"
            rows="3"
            class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-d-gray-800 dark:border-border dark:text-white"
            bind:value={description}
        ></textarea>
    </div>
    <div class="mb-4">
        <label class="block text-sm font-medium text-gray-700 dark:text-d-gray-300 mb-1">Color</label>
        <div class="flex space-x-2">
            {#each highlightOptions as option}
                <button
                    title={option.label}
                    class="w-6 h-6 rounded-full border border-gray-400 dark:border-d-gray-500 focus:outline-none focus:ring-2 focus:ring-offset-1 dark:focus:ring-offset-d-gray-700"
                    class:ring-blue-500={selectedColor === option.value}
                    class:dark:ring-blue-400={selectedColor === option.value}
                    class:ring-2={selectedColor === option.value}
                    style:background-color={option.value}
                    on:click={() => selectedColor = option.value}
                ></button>
            {/each}
        </div>
    </div>
    <div class="flex justify-end space-x-2">
        <button
            class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-d-gray-300 bg-gray-200 dark:bg-d-gray-600 rounded-md hover:bg-gray-300 dark:hover:bg-d-gray-500 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
            on:click={handleCancel}
        >
            Cancel
        </button>
        <button
            class="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
            on:click={handleSave}
        >
            OK
        </button>
    </div>
</div>
{/if}

<style lang="postcss">
    .annotation-dialog {
        /* Basic styling for the dialog */
        min-width: 300px;
        max-width: 400px;
    }
</style>