<script>
    import { createEventDispatcher, onMount, onDestroy } from 'svelte';
    import Dropdown from '$lib/components/shared/Dropdown.svelte';
    import LexicalEditor from '$lib/components/projectview/lexical/LexicalEditor.svelte';
    import { MessageSquare, Code, Check, Square, Circle, Info } from 'lucide-svelte';

    const dispatch = createEventDispatcher();

    export let x = 0;
    export let y = 0;
    export let initialColor = 'rgba(255, 242, 117, 0.5)'; // Default yellow
    export let initialTitle = '';
    export let initialDescription = '';
    export let initialText = null; // If not null, shows a text content field (for speech bubbles)
    export let initialHtml = null; // HTML representation for rendering
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
    let html = initialHtml || '';
    let selectedColor = initialColor;
    let selectedTextColor = initialTextColor;
    let selectedFontSize = initialFontSize;
    let selectedBorderColor = selectedColor === 'url(#censoredPattern)' ? 'black' : (initialBorderColor || (initialColor.includes('255, 255, 255') ? 'rgba(156, 163, 175, 1)' : initialColor.replace(', 0.5', ', 1')));
    let selectedBorderSize = initialBorderSize;
    let selectedShape = initialShape;
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

    $: notifyChanges(title, description, selectedColor, text, html, selectedTextColor, selectedFontSize, selectedBorderColor, selectedBorderSize, selectedShape, selectedTailStyle, tailFlipped, rounded, isOval);

    function notifyChanges() {
        dispatch('save', { 
            title, 
            description, 
            color: selectedColor, 
            text, 
            html,
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

    function handleDone() {
        dispatch('done');
    }

    function handleCancel() {
        dispatch('cancel');
    }

    function handleDelete() {
        dispatch('delete');
    }

    function handleLexicalChange(event) {
        text = event.detail.jsonString;
        html = event.detail.htmlString;
    }

    // Adjust position to keep dialog within viewport (basic implementation)
    let dialogElement;
    let dialogWidth = 500; // Increased width for Lexical toolbar
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

    function handleClickOutside(event) {
        if (dialogElement && !dialogElement.contains(event.target)) {
            // Don't close if clicking a dropdown menu or Lexical modal
            if (event.target.closest('.ui-dropdown-menu') || event.target.closest('.lexical-modal')) return;
            handleDone();
        }
    }

    onMount(() => {
        setTimeout(() => {
            window.addEventListener('pointerdown', handleClickOutside, true);
        }, 100);
    });

    onDestroy(() => {
        window.removeEventListener('pointerdown', handleClickOutside, true);
    });

    const lexicalToolbarConfig = {
        undo: false,
        redo: false,
        blockType: false,
        bold: true,
        italic: true,
        underline: true,
        strikethrough: true,
        link: false,
        fontFamily: true,
        fontSize: true, // Assuming we want this based on user request
        insertMenu: false,
        indent: false,
        outdent: false,
        align: false,
        textColor: true,
        highlight: true,
        clearFormatting: true,
        search: false
    };
</script>

<div
    bind:this={dialogElement}
    class="absolute z-[1001] bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-lg shadow-xl p-4"
    style="left: {x}px; top: {y}px; width: {dialogWidth}px;"
    on:click|stopPropagation
    on:pointerdown|stopPropagation
>
    {#if !isCensoredMode}
        {#if initialText !== null}
            <div class="mb-3">
                <label for="annotation-text" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Text Content</label>
                <div class="lexical-container border border-gray-300 dark:border-gray-700 rounded-md overflow-hidden bg-white dark:bg-gray-900">
                    <LexicalEditor
                        initialJson={text.startsWith('{') ? text : null}
                        placeholder={!text.startsWith('{') ? text : "Enter text..."}
                        editable={true}
                        toolbarConfig={lexicalToolbarConfig}
                        on:change={handleLexicalChange}
                    />
                </div>
            </div>

            {#if initialShape?.startsWith('speech-bubble')}
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
            {/if}

            {#if selectedShape === 'rectangle' || initialShape === 'speech-bubble-rect'}
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

            {#if selectedShape === 'circle' || initialShape === 'speech-bubble-circle'}
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
                    class="mt-1 block w-full rounded-md border-gray-300 shadow-sm text-sm dark:bg-gray-700 dark:border-gray-700 dark:text-white focus:ring-blue-500 focus:border-blue-500"
                    bind:value={title}
                    placeholder="Enter title"
                    autocomplete="off"
                />
            </div>
            <div class="mb-3">
                <label for="annotation-description" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Description</label>
                <textarea
                    id="annotation-description"
                    class="mt-1 block w-full rounded-md border-gray-300 shadow-sm text-sm dark:bg-gray-700 dark:border-gray-700 dark:text-white focus:ring-blue-500 focus:border-blue-500"
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
                    <Square class="w-4 h-4" />
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
                    <Circle class="w-4 h-4" />
                </button>
            </div>
        </div>
    {/if}

    <div class="mb-4">
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">{isCensoredMode ? 'Anonymise Style' : 'Background Color'}</label>
        <Dropdown
            containerClasses="w-full"
            options={highlightOptions}
            bind:value={selectedColor}
            on:change={(e) => selectedColor = e.detail}
            showColorPreview={true}
            boundaryRect={panelBounds}
        />
    </div>

    {#if initialText !== null}
        <!-- Lexical handles Text Color and Font Size, so we only show them for title/description if needed, but here it's speech bubble/text area -->
        <!-- User said we can remove them -->
        
        <div class="mb-4">
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Border Color</label>
            <Dropdown
                containerClasses="w-full"
                options={textColors}
                bind:value={selectedBorderColor}
                on:change={(e) => selectedBorderColor = e.detail}
                showColorPreview={true}
                boundaryRect={panelBounds}
            />
        </div>

        <div class="mb-4">
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Border Size</label>
            <Dropdown
                containerClasses="w-full"
                options={borderSizes.map(s => ({ value: s, label: s.toString() }))}
                bind:value={selectedBorderSize}
                on:change={(e) => selectedBorderSize = e.detail}
                boundaryRect={panelBounds}
            />
        </div>
    {/if}

    {#if isCensoredMode}
        <div class="mb-4 p-2 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded text-[10px] text-amber-800 dark:text-amber-200 leading-tight">
            <div class="flex items-start space-x-1.5">
                <Info class="w-3 h-3 mt-0.5 flex-shrink-0" />
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
                class="px-3 py-1.5 text-xs font-medium text-white bg-blue-600 rounded hover:bg-blue-700"
                on:click={handleDone}
            >
                Done
            </button>
        </div>
    </div>
</div>

<style lang="postcss">
    .lexical-container {
        height: 200px;
        display: flex;
        flex-direction: column;
        overflow: hidden;
    }
    
    :global(.lexical-container > .lexical-editor-root) {
        height: 100%;
        display: flex;
        flex-direction: column;
        flex: 1;
        min-height: 0;
    }

    :global(.lexical-container .toolbar) {
        @apply p-0.5 gap-0.5;
        flex-shrink: 0;
    }

    :global(.lexical-container .lexical-wrapper) {
        min-height: 0;
        overflow-y: auto !important;
        flex-grow: 1;
    }

    /* Scrollbar styles for the Lexical wrapper within the dialog */
    :global(.lexical-container .lexical-wrapper)::-webkit-scrollbar {
        @apply w-[8px];
    }
    :global(.lexical-container .lexical-wrapper)::-webkit-scrollbar-track {
        @apply bg-gray-100 dark:bg-gray-900 rounded-lg;
    }
    :global(.lexical-container .lexical-wrapper)::-webkit-scrollbar-thumb {
        @apply bg-gray-400 dark:bg-gray-700 rounded-lg border-2 border-solid border-gray-100 dark:border-gray-900;
    }
    :global(.lexical-container .lexical-wrapper)::-webkit-scrollbar-thumb:hover {
        @apply bg-gray-500 dark:bg-gray-600;
    }
    :global(.lexical-container .lexical-wrapper) {
        scrollbar-width: thin;
        scrollbar-color: theme('colors.gray.400') theme('colors.gray.100');
    }
    :global(html.dark .lexical-container .lexical-wrapper) {
        scrollbar-color: theme('colors.gray.700') theme('colors.gray.900');
    }
</style>
