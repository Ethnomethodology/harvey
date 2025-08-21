<script>
    import { createEventDispatcher } from 'svelte';

    const dispatch = createEventDispatcher();

    export let show = false;
    export let x = 0;
    export let y = 0;
    export let initialColor = 'rgba(255, 242, 117, 0.5)';

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
        dispatch('save', { color: selectedColor });
        show = false;
    }

    function handleCancel() {
        dispatch('cancel');
        show = false;
    }

    $: if (show) {
        selectedColor = initialColor;
    }
</script>

{#if show}
<div
    class="color-picker-dialog fixed inset-0 z-[1001] bg-black/30 flex items-center justify-center"
    on:click|self={handleCancel}
>
    <div
        class="bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md shadow-xl p-4"
        tabindex="-1"
    >
        <h3 class="text-lg font-semibold mb-3 text-gray-900 dark:text-white">Select Color</h3>
        <div class="mb-4">
            <div class="flex space-x-2">
                {#each highlightOptions as option}
                    <button
                        title={option.label}
                        class="w-8 h-8 rounded-full border-2 border-transparent focus:outline-none focus:ring-2 focus:ring-offset-2 dark:focus:ring-offset-gray-700"
                        class:ring-blue-500={selectedColor === option.value}
                        class:dark:ring-blue-400={selectedColor === option.value}
                        style="background-color: {option.value};"
                        on:click={() => selectedColor = option.value}
                    ></button>
                {/each}
            </div>
        </div>
        <div class="flex justify-end space-x-2">
            <button
                class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-200 dark:bg-gray-600 rounded-md hover:bg-gray-300 dark:hover:bg-gray-500 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
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
</div>
{/if}
