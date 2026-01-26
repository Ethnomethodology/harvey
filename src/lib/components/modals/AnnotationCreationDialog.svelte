<script>
    import { createEventDispatcher } from 'svelte';

    const dispatch = createEventDispatcher();

    export let x = 0;
    export let y = 0;
    export let initialColor = 'rgba(255, 242, 117, 0.5)'; // Default yellow
    export let initialTitle = '';
    export let initialDescription = '';
    export let initialText = null; // If not null, shows a text content field (for speech bubbles)
    export let isEditing = false; // New prop to indicate if we are editing an existing annotation
    export let panelBounds = null; // New prop to receive the bounding rectangle of the parent panel
    export let useSolidColors = false; // New prop to determine color palette

    let title = initialTitle;
    let description = initialDescription;
    let text = initialText || '';
    let selectedColor = initialColor;

    const transparentColors = [
        { value: 'rgba(255, 242, 117, 0.5)', label: 'Yellow' },
        { value: 'rgba(168, 255, 158, 0.5)', label: 'Green' },
        { value: 'rgba(174, 239, 255, 0.5)', label: 'Blue' },
        { value: 'rgba(255, 176, 207, 0.5)', label: 'Pink' },
        { value: 'rgba(208, 160, 255, 0.5)', label: 'Purple' },
        { value: 'rgba(255, 255, 255, 0.5)', label: 'White' },
    ];

    const solidColors = [
        { value: 'rgba(255, 242, 117, 1)', label: 'Yellow' },
        { value: 'rgba(168, 255, 158, 1)', label: 'Green' },
        { value: 'rgba(174, 239, 255, 1)', label: 'Blue' },
        { value: 'rgba(255, 176, 207, 1)', label: 'Pink' },
        { value: 'rgba(208, 160, 255, 1)', label: 'Purple' },
        { value: 'rgba(255, 255, 255, 1)', label: 'White' },
    ];

    $: highlightOptions = useSolidColors ? solidColors : transparentColors;

    function handleSave() {
        dispatch('save', { title, description, color: selectedColor, text });
    }

    function handleCancel() {
        dispatch('cancel');
    }

    function handleDelete() {
        dispatch('delete');
    }

    // Adjust position to keep dialog within viewport (basic implementation)
    let dialogElement;
    let dialogWidth = 200; // Simplified dialog
    let dialogHeight = 200;

    $: if (dialogElement && panelBounds) {
        let newX = x;
        let newY = y;

        const currentDialogRect = dialogElement.getBoundingClientRect();
        const actualDialogWidth = currentDialogRect.width > 0 ? currentDialogRect.width : dialogWidth;
        const actualDialogHeight = currentDialogRect.height > 0 ? currentDialogRect.height : dialogHeight;

        if (newX + actualDialogWidth > panelBounds.width) {
            newX = panelBounds.width - actualDialogWidth - 10;
        }
        if (newX < 0) {
            newX = 10;
        }
        if (newY + actualDialogHeight > panelBounds.height) {
            newY = panelBounds.height - actualDialogHeight - 10;
        }
        if (newY < 0) {
            newY = 10;
        }

        dialogElement.style.left = `${newX}px`;
        dialogElement.style.top = `${newY}px`;
    }
</script>

<div
    bind:this={dialogElement}
    class="absolute z-[1001] bg-white dark:bg-gray-800 border border-gray-300 dark:border-border rounded-lg shadow-xl p-4"
    style="left: {x}px; top: {y}px; min-width: 200px;"
>
    {#if initialText !== null}
        <div class="mb-3">
            <label for="annotation-text" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Text Content</label>
            <textarea
                id="annotation-text"
                class="mt-1 block w-full rounded-md border-gray-300 shadow-sm text-sm dark:bg-gray-700 dark:border-border dark:text-white focus:ring-blue-500 focus:border-blue-500"
                bind:value={text}
                placeholder="Enter text..."
                rows="2"
            ></textarea>
        </div>
    {:else}
        <div class="mb-3">
            <label for="annotation-title" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Title</label>
            <input
                type="text"
                id="annotation-title"
                class="mt-1 block w-full rounded-md border-gray-300 shadow-sm text-sm dark:bg-gray-700 dark:border-border dark:text-white focus:ring-blue-500 focus:border-blue-500"
                bind:value={title}
                placeholder="Enter title"
                autocomplete="off"
            />
        </div>
        <div class="mb-3">
            <label for="annotation-description" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Description</label>
            <textarea
                id="annotation-description"
                class="mt-1 block w-full rounded-md border-gray-300 shadow-sm text-sm dark:bg-gray-700 dark:border-border dark:text-white focus:ring-blue-500 focus:border-blue-500"
                bind:value={description}
                placeholder="Enter description"
                rows="2"
            ></textarea>
        </div>
    {/if}

    <div class="mb-4">
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Color</label>
        <div class="grid grid-cols-3 gap-2">
            {#each highlightOptions as option}
                <button
                    title={option.label}
                    class="w-full h-8 rounded border-2 focus:outline-none focus:ring-2 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                    class:border-blue-500={selectedColor === option.value}
                    class:dark:border-blue-400={selectedColor === option.value}
                    style="background-color: {option.value};"
                    on:click={() => {
                        selectedColor = option.value;
                        if (initialText === null && isEditing) handleSave(); // Auto-save color change if no text
                    }}
                >
                </button>
            {/each}
        </div>
    </div>

    <div class="flex justify-between space-x-2 mt-2">
        {#if isEditing}
            <button
                class="px-3 py-1.5 text-xs font-medium text-red-700 dark:text-red-300 bg-red-100 dark:bg-red-900/30 rounded hover:bg-red-200 dark:hover:bg-red-900/50"
                on:click={handleDelete}
            >
                Delete
            </button>
        {/if}
        <div class="flex space-x-2">
            <button
                class="px-3 py-1.5 text-xs font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700 rounded hover:bg-gray-200 dark:hover:bg-gray-600"
                on:click={handleCancel}
            >
                Cancel
            </button>
            <button
                class="px-3 py-1.5 text-xs font-medium text-white bg-blue-600 rounded hover:bg-blue-700"
                on:click={handleSave}
            >
                {isEditing ? 'Update' : 'Add'}
            </button>
        </div>
    </div>
</div>
