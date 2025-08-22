<script>
    import { createEventDispatcher } from 'svelte';
    import { HIGHLIGHT_OPTIONS } from '$lib/constants/highlightOptions.js';

    export let show = false;

    const dispatch = createEventDispatcher();
    const colors = HIGHLIGHT_OPTIONS.map(option => option.value);

    function selectColor(color) {
        dispatch('save', { color });
        show = false;
    }

    function closeModal() {
        dispatch('cancel');
        show = false;
    }
</script>

{#if show}
<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50" on:click={closeModal} role="dialog" aria-modal="true" on:keydown={(e) => e.key === 'Escape' && closeModal()}>
    <div class="bg-white dark:bg-gray-700 p-6 rounded-lg shadow-xl" on:click|stopPropagation>
        <h3 class="text-lg font-bold mb-4">Select a Color</h3>
        <div class="grid grid-cols-5 gap-2">
            {#each colors as color}
            <button
                aria-label="Select color {color}"
                class="w-10 h-10 rounded-full cursor-pointer border-2 border-transparent hover:border-blue-500"
                style="background-color: {color};"
                on:click={() => selectColor(color)}
            ></button>
            {/each}
        </div>
        <div class="mt-6 flex justify-end">
            <button
                class="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-200 hover:bg-gray-300 dark:bg-gray-600 dark:hover:bg-gray-500 rounded-md"
                on:click={closeModal}
            >
                Cancel
            </button>
        </div>
    </div>
</div>
{/if}
