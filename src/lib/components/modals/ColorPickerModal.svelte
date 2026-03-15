<script>
    import { createEventDispatcher } from 'svelte';
    import { Modal, Button } from 'flowbite-svelte';

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

<Modal bind:open={show} size="xs" autoclose={false} outsideclose={true} backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm" dialogClass="fixed top-0 start-0 end-0 h-modal md:h-full z-[10001] w-full p-4 flex items-center justify-center" class="w-full" on:close={handleClose}>
    <h3 id="color-picker-title" class="text-lg font-bold text-gray-900 dark:text-gray-100" slot="header">{title}</h3>

    <div class="grid grid-cols-6 gap-2">
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

    <svelte:fragment slot="footer">
        <div class="flex justify-end space-x-2 w-full">
            <Button color="alternative" on:click={handleClose}>Cancel</Button>
            <Button color="blue" on:click={handleConfirm}>Apply</Button>
        </div>
    </svelte:fragment>
</Modal>
