<script>
    import { createEventDispatcher } from 'svelte';

    export let show = false;
    export let title = 'Select a Color';

    const dispatch = createEventDispatcher();

    let selectedColor = '#fef08a'; // Default to a yellow/highlight color

    // A simple palette of colors
    const colors = [
        '#fef08a', '#fde047', '#f97316', '#ef4444', '#ec4899',
        '#d946ef', '#8b5cf6', '#3b82f6', '#06b6d4', '#14b8a6',
        '#22c55e', '#84cc16'
    ];

    function handleConfirm() {
        dispatch('confirm', { color: selectedColor });
    }

    function handleClose() {
        dispatch('close');
    }
</script>

{#if show}
<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-[100]" on:click|self={handleClose} role="dialog" aria-modal="true" aria-labelledby="color-picker-title">
    <div class="bg-white dark:bg-gray-800 p-6 rounded-lg shadow-xl w-full max-w-xs text-gray-900 dark:text-gray-100" on:click|stopPropagation>
        <h3 id="color-picker-title" class="text-lg font-bold mb-4">{title}</h3>
        <div class="grid grid-cols-6 gap-2 mb-4">
            {#each colors as color}
            <button
                type="button"
                aria-label="Select color {color}"
                class="w-8 h-8 rounded-full border-2 transition-all"
                style="background-color: {color};"
                class:border-blue-500={selectedColor === color}
                class:scale-110={selectedColor === color}
                class:border-transparent={selectedColor !== color}
                on:click={() => selectedColor = color}
            ></button>
            {/each}
        </div>
        <div class="flex justify-end space-x-2 pt-4 border-t border-gray-200 dark:border-border">
            <button class="px-4 py-2 text-sm rounded-md bg-gray-200 hover:bg-gray-300 dark:bg-gray-600 dark:hover:bg-gray-500" on:click={handleClose}>Cancel</button>
            <button class="px-4 py-2 text-sm rounded-md bg-blue-600 text-white hover:bg-blue-700" on:click={handleConfirm}>Apply</button>
        </div>
    </div>
</div>
{/if}
