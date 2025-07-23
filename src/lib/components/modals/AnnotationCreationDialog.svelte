<script>
    import { createEventDispatcher } from 'svelte';

    const dispatch = createEventDispatcher();

    export let x = 0;
    export let y = 0;
    export let initialColor = 'rgba(255, 242, 117, 0.5)'; // Default yellow
    export let initialTitle = '';
    export let initialDescription = '';
    export let isEditing = false; // New prop to indicate if we are editing an existing annotation

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
    }

    function handleCancel() {
        dispatch('cancel');
    }

    function handleDelete() {
        dispatch('delete');
    }

    // Adjust position to keep dialog within viewport (basic implementation)
    let dialogElement;
    let dialogWidth = 250; // Approximate width
    let dialogHeight = 250; // Approximate height

    $: if (dialogElement) {
        const viewportWidth = window.innerWidth;
        const viewportHeight = window.innerHeight;

        let newX = x;
        let newY = y;

        // Prevent going off right edge
        if (newX + dialogWidth > viewportWidth - 20) {
            newX = viewportWidth - dialogWidth - 20;
        }
        // Prevent going off bottom edge
        if (newY + dialogHeight > viewportHeight - 20) {
            newY = viewportHeight - dialogHeight - 20;
        }
        // Prevent going off left edge
        if (newX < 20) {
            newX = 20;
        }
        // Prevent going off top edge
        if (newY < 20) {
            newY = 20;
        }

        dialogElement.style.left = `${newX}px`;
        dialogElement.style.top = `${newY}px`;
    }
</script>

<div
    bind:this={dialogElement}
    class="absolute z-[1001] bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg shadow-xl p-4"
    style="left: {x}px; top: {y}px; min-width: 250px;"
>
    <h3 class="text-lg font-semibold mb-3 text-gray-900 dark:text-white">{isEditing ? 'Edit Annotation' : 'New Annotation'}</h3>

    <div class="mb-3">
        <label for="annotation-title" class="block text-sm font-medium text-gray-700 dark:text-gray-300">Title (Optional)</label>
        <input
            type="text"
            id="annotation-title"
            class="mt-1 block w-full rounded-md border-gray-300 shadow-sm text-sm dark:bg-gray-700 dark:border-gray-600 dark:text-white focus:ring-blue-500 focus:border-blue-500"
            bind:value={title}
            placeholder="Enter title"
        />
    </div>

    <div class="mb-3">
        <label for="annotation-description" class="block text-sm font-medium text-gray-700 dark:text-gray-300">Description (Optional)</label>
        <textarea
            id="annotation-description"
            class="mt-1 block w-full rounded-md border-gray-300 shadow-sm text-sm dark:bg-gray-700 dark:border-gray-600 dark:text-white focus:ring-blue-500 focus:border-blue-500"
            bind:value={description}
            placeholder="Enter description"
            rows="3"
        ></textarea>
    </div>

    <div class="mb-4">
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Color</label>
        <div class="flex space-x-2">
            {#each highlightOptions as option}
                <button
                    title={option.label}
                    class="w-6 h-6 rounded-full border-2 focus:outline-none focus:ring-2 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                    class:border-blue-500={selectedColor === option.value}
                    class:dark:border-blue-400={selectedColor === option.value}
                    style="background-color: {option.value};"
                    on:click={() => (selectedColor = option.value)}
                >
                </button>
            {/each}
        </div>
    </div>

    <div class="flex justify-between space-x-2">
        {#if isEditing}
            <button
                class="px-4 py-2 text-sm font-medium text-red-700 dark:text-red-300 bg-red-200 dark:bg-red-700 rounded-md hover:bg-red-300 dark:hover:bg-red-600 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500"
                on:click={handleDelete}
            >
                Delete
            </button>
        {/if}
        <div class="flex space-x-2">
            <button
                class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-200 dark:bg-gray-700 rounded-md hover:bg-gray-300 dark:hover:bg-gray-600 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-gray-500"
                on:click={handleCancel}
            >
                Cancel
            </button>
            <button
                class="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
                on:click={handleSave}
            >
                Save
            </button>
        </div>
    </div>
</div>
