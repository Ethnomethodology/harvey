<script>
    import { createEventDispatcher } from 'svelte';

    const dispatch = createEventDispatcher();

    export let x = 0;
    export let y = 0;
    export let initialColor = 'rgba(255, 242, 117, 0.5)'; // Default yellow
    export let initialTitle = '';
    export let initialDescription = '';
    export let initialText = null; // If not null, shows a text content field (for speech bubbles)
    export let initialTextColor = 'black';
    export let initialFontSize = 14;
    export let initialBorderColor = null;
    export let initialBorderSize = 1;
    export let initialShape = 'rectangle';
    export let initialTailStyle = 'straight'; // New prop for tail style
    export let initialTailFlipped = false; // New prop for flipping curve
    export let initialRounded = false; // New prop for rounded corners
    export let initialIsOval = false; // New prop for oval shape
    export let isEditing = false; // New prop to indicate if we are editing an existing annotation
    export let panelBounds = null; // New prop to receive the bounding rectangle of the parent panel
    export let useSolidColors = false; // New prop to determine color palette
    export let isCensoredMode = false; // New prop for censored-only mode

    let title = initialTitle;
    let description = initialDescription;
    let text = initialText || '';
    let selectedColor = initialColor;
    let selectedTextColor = initialTextColor;
    let selectedFontSize = initialFontSize;
    let selectedBorderColor = selectedColor === 'url(#censoredPattern)' ? 'black' : (initialBorderColor || (initialColor.includes('255, 255, 255') ? 'rgba(156, 163, 175, 1)' : initialColor.replace(', 0.5', ', 1')));
    let selectedBorderSize = initialBorderSize;
    let selectedShape = (initialShape && initialShape.includes('circle')) ? 'circle' : 'rectangle';
    let selectedTailStyle = initialTailStyle || 'straight';
    let tailFlipped = initialTailFlipped || false;
    let rounded = initialRounded || false;
    let isOval = initialIsOval || false;

    const transparentColors = [
        { value: 'rgba(255, 255, 255, 0.5)', label: 'White' },
        { value: 'rgba(0, 0, 0, 0.5)', label: 'Black' },
        { value: 'rgba(255, 242, 117, 0.5)', label: 'Yellow' },
        { value: 'rgba(168, 255, 158, 0.5)', label: 'Green' },
        { value: 'rgba(174, 239, 255, 0.5)', label: 'Blue' },
        { value: 'rgba(255, 176, 207, 0.5)', label: 'Pink' },
        { value: 'rgba(208, 160, 255, 0.5)', label: 'Purple' },
        { value: 'transparent', label: 'Transparent' },
    ];

    const solidColors = [
        { value: 'rgba(255, 255, 255, 1)', label: 'White' },
        { value: 'rgba(0, 0, 0, 1)', label: 'Black' },
        { value: 'rgba(255, 242, 117, 1)', label: 'Yellow' },
        { value: 'rgba(168, 255, 158, 1)', label: 'Green' },
        { value: 'rgba(174, 239, 255, 1)', label: 'Blue' },
        { value: 'rgba(255, 176, 207, 1)', label: 'Pink' },
        { value: 'rgba(208, 160, 255, 1)', label: 'Purple' },
        { value: 'transparent', label: 'Transparent' },
    ];

    const censoredColors = [
        { value: 'url(#censoredPattern)', label: 'Anonymise' },
        { value: 'rgba(255, 255, 255, 1)', label: 'White' },
        { value: 'rgba(0, 0, 0, 1)', label: 'Black' },
    ];

    const textColors = [
        { value: 'black', label: 'Black' },
        { value: 'white', label: 'White' },
        { value: 'red', label: 'Red' },
        { value: 'blue', label: 'Blue' },
        { value: 'gray', label: 'Grey' },
        { value: 'transparent', label: 'Transparent' },
    ];

    const borderSizes = [1, 2, 3, 4, 5];
    const fontSizes = [10, 12, 14, 16, 18, 20, 24, 28, 32, 36];

    $: highlightOptions = isCensoredMode ? censoredColors : (useSolidColors ? solidColors : transparentColors);

    function handleSave() {
        dispatch('save', { 
            title, 
            description, 
            color: selectedColor, 
            text, 
            textColor: selectedTextColor, 
            fontSize: selectedFontSize, 
            borderColor: selectedBorderColor, 
            borderSize: selectedBorderSize, 
            shape: selectedShape,
            tailStyle: selectedTailStyle,
            tailFlipped,
            rounded,
            isOval
        });
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
    {#if !isCensoredMode}
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

            <div class="mb-3">
                <div class="flex space-x-2">
                    <button
                        class="flex-1 flex items-center justify-center py-1.5 text-xs font-medium border rounded transition-colors"
                        class:bg-blue-600={selectedTailStyle === 'straight'}
                        class:text-white={selectedTailStyle === 'straight'}
                        class:bg-gray-100={selectedTailStyle !== 'straight'}
                        class:dark:bg-gray-700={selectedTailStyle !== 'straight'}
                        on:click={() => (selectedTailStyle = 'straight')}
                    >
                        Straight Tail
                    </button>
                    <button
                        class="flex-1 flex items-center justify-center py-1.5 text-xs font-medium border rounded transition-colors"
                        class:bg-blue-600={selectedTailStyle === 'curved'}
                        class:text-white={selectedTailStyle === 'curved'}
                        class:bg-gray-100={selectedTailStyle !== 'curved'}
                        class:dark:bg-gray-700={selectedTailStyle !== 'curved'}
                        on:click={() => (selectedTailStyle = 'curved')}
                    >
                        Curved Tail
                    </button>
                </div>
            </div>

            {#if selectedTailStyle === 'curved'}
                <div class="mb-3">
                    <label class="flex items-center space-x-2 cursor-pointer">
                        <input
                            type="checkbox"
                            class="rounded border-gray-300 text-blue-600 focus:ring-blue-500 h-4 w-4"
                            bind:checked={tailFlipped}
                        />
                        <span class="text-sm font-medium text-gray-700 dark:text-gray-300">Flip Tail</span>
                    </label>
                </div>
            {/if}

            {#if selectedShape === 'rectangle'}
                <div class="mb-3">
                    <label class="flex items-center space-x-2 cursor-pointer">
                        <input
                            type="checkbox"
                            class="rounded border-gray-300 text-blue-600 focus:ring-blue-500 h-4 w-4"
                            bind:checked={rounded}
                        />
                        <span class="text-sm font-medium text-gray-700 dark:text-gray-300">Rounded Corners</span>
                    </label>
                </div>
            {/if}

            {#if selectedShape === 'circle'}
                <div class="mb-3">
                    <label class="flex items-center space-x-2 cursor-pointer">
                        <input
                            type="checkbox"
                            class="rounded border-gray-300 text-blue-600 focus:ring-blue-500 h-4 w-4"
                            bind:checked={isOval}
                        />
                        <span class="text-sm font-medium text-gray-700 dark:text-gray-300">Oval Shape</span>
                    </label>
                </div>
            {/if}
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
    {/if}

    {#if isCensoredMode}
        <div class="mb-4">
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Shape</label>
            <div class="flex space-x-2">
                <button
                    class="flex-1 flex justify-center py-1.5 text-xs font-medium border rounded transition-colors"
                    class:bg-blue-600={selectedShape === 'rectangle'}
                    class:text-white={selectedShape === 'rectangle'}
                    class:bg-gray-100={selectedShape !== 'rectangle'}
                    class:dark:bg-gray-700={selectedShape !== 'rectangle'}
                    title="Rectangle"
                    on:click={() => (selectedShape = 'rectangle')}
                >
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" viewBox="0 0 16 16">
                        <path d="M0 2a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2V2zM2 1a1 1 0 0 0-1 1v12a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V2a1 1 0 0 0-1-1H2z"/>
                    </svg>
                </button>
                <button
                    class="flex-1 flex justify-center py-1.5 text-xs font-medium border rounded transition-colors"
                    class:bg-blue-600={selectedShape === 'circle'}
                    class:text-white={selectedShape === 'circle'}
                    class:bg-gray-100={selectedShape !== 'circle'}
                    class:dark:bg-gray-700={selectedShape !== 'circle'}
                    title="Circle"
                    on:click={() => (selectedShape = 'circle')}
                >
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" viewBox="0 0 16 16">
                        <path d="M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14m0 1A8 8 0 1 0 8 0a8 8 0 0 0 0 16"/>
                    </svg>
                </button>
            </div>
        </div>
    {/if}

    <div class="mb-4">
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">{isCensoredMode ? 'Anonymise Style' : 'Background Color'}</label>
        <div class="flex items-center space-x-1.5">
            {#each highlightOptions as option}
                <button
                    title={option.label}
                    class="w-5 h-5 rounded-full border border-gray-300 dark:border-gray-500 transition-transform hover:scale-110 shadow-sm"
                    class:ring-2={selectedColor === option.value}
                    class:ring-blue-500={selectedColor === option.value}
                    style="background: {option.value === 'transparent' ? 'linear-gradient(45deg, rgba(255,255,255,1) 45%, rgba(255,0,0,1) 45%, rgba(255,0,0,1) 55%, rgba(255,255,255,1) 55%)' : (option.value === 'url(#censoredPattern)' ? 'linear-gradient(to bottom right, #fff 25%, #888 25%, #888 50%, #444 50%, #444 75%, #000 75%)' : option.value.replace(', 0.5', ', 1'))};"
                    on:click={() => {
                        selectedColor = option.value;
                        if (initialText === null && isEditing) handleSave(); // Auto-save color change if no text
                    }}
                >
                </button>
            {/each}
        </div>
    </div>

    {#if initialText !== null}
        <div class="mb-4">
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Text Color</label>
            <div class="flex items-center space-x-1.5">
                {#each textColors as option}
                    <button
                        title={option.label}
                        class="w-5 h-5 rounded-full border border-gray-300 dark:border-gray-500 transition-transform hover:scale-110 shadow-sm"
                        class:ring-2={selectedTextColor === option.value}
                                            class:ring-blue-500={selectedTextColor === option.value}
                                            style="background: {option.value === 'transparent' ? 'linear-gradient(45deg, rgba(255,255,255,1) 45%, rgba(255,0,0,1) 45%, rgba(255,0,0,1) 55%, rgba(255,255,255,1) 55%)' : option.value};"
                                            on:click={() => (selectedTextColor = option.value)}
                                        >
                        
                    </button>
                {/each}
            </div>
        </div>

        <div class="mb-4">
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Font Size</label>
            <div class="grid grid-cols-5 gap-1">
                {#each fontSizes as size}
                    <button
                        class="px-1 py-1 text-[10px] font-medium border rounded transition-colors"
                        class:bg-blue-600={selectedFontSize === size}
                        class:text-white={selectedFontSize === size}
                        class:bg-gray-100={selectedFontSize !== size}
                        class:dark:bg-gray-700={selectedFontSize !== size}
                        on:click={() => (selectedFontSize = size)}
                    >
                        {size}
                    </button>
                {/each}
            </div>
        </div>

        <div class="mb-4">
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Border Color</label>
            <div class="flex items-center space-x-1.5">
                {#each textColors as option}
                    <button
                        title={option.label}
                        class="w-5 h-5 rounded-full border border-gray-300 dark:border-gray-500 transition-transform hover:scale-110 shadow-sm"
                        class:ring-2={selectedBorderColor === option.value}
                                            class:ring-blue-500={selectedBorderColor === option.value}
                                            style="background: {option.value === 'transparent' ? 'linear-gradient(45deg, rgba(255,255,255,1) 45%, rgba(255,0,0,1) 45%, rgba(255,0,0,1) 55%, rgba(255,255,255,1) 55%)' : option.value};"
                                            on:click={() => (selectedBorderColor = option.value)}
                                        >
                        
                    </button>
                {/each}
            </div>
        </div>

        <div class="mb-4">
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Border Size</label>
            <div class="flex items-center space-x-2">
                {#each borderSizes as size}
                    <button
                        class="w-8 h-8 flex items-center justify-center text-xs font-medium border rounded transition-colors"
                        class:bg-blue-600={selectedBorderSize === size}
                        class:text-white={selectedBorderSize === size}
                        class:bg-gray-100={selectedBorderSize !== size}
                        class:dark:bg-gray-700={selectedBorderSize !== size}
                        on:click={() => (selectedBorderSize = size)}
                    >
                        {size}
                    </button>
                {/each}
            </div>
        </div>
    {/if}

    {#if isCensoredMode}
        <div class="mb-4 p-2 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded text-[10px] text-amber-800 dark:text-amber-200 leading-tight">
            <div class="flex items-start space-x-1.5">
                <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" fill="currentColor" class="mt-0.5 flex-shrink-0" viewBox="0 0 16 16">
                    <path d="M8 16A8 8 0 1 0 8 0a8 8 0 0 0 0 16m.93-9.412-1 4.705c-.07.34.029.533.308.533.19 0 .452-.113.688-.273l.111.19c-.3.213-.662.338-.958.338-.589 0-.813-.41-.699-1.112l1.047-4.973c.067-.318-.05-.562-.38-.562-.286 0-.633.163-.84.294l-.11-.191c.217-.152.56-.322.896-.322.604 0 .822.424.71.105zM8 5.5a1 1 0 1 1 0-2 1 1 0 0 1 0 2"/>
                </svg>
                <span>Anonymization is only permanent when the image is <strong>exported with annotations</strong>.</span>
            </div>
        </div>
    {/if}

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
