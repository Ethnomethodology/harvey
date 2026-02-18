<!-- src/lib/components/projectview/modals/ImageExportModal.svelte -->
<script>
    import { createEventDispatcher, onMount, onDestroy } from 'svelte';
    import { open } from '@tauri-apps/plugin-dialog';
    import { documentDir } from '@tauri-apps/api/path';
    import Dropdown from '$lib/components/shared/Dropdown.svelte';

    export let showModal = false;
    export let defaultFileName = 'export.png';

    const dispatch = createEventDispatcher();

    // Local state
    let exportFileName = '';
    let exportFormat = 'png';
    let exportDirectory = '';
    let includeAnnotations = false;
    let modalElement;
    let modalTitle = 'Export Image';

    const exportFormats = [
        { value: 'png', label: 'PNG (.png)', disabled: false },
        { value: 'jpg', label: 'JPEG (.jpg)', disabled: false }
    ];

    const PATH_SEPARATOR = '/';

    async function initializeModalState() {
        if (defaultFileName) {
            // Normalize path separators to forward slash without regex to avoid parser issues
            const normalized = defaultFileName.split('\\').join('/');
            const filenameWithExt = normalized.split('/').pop();
            const parts = filenameWithExt.split('.');
            
            if (parts.length > 1) {
                exportFormat = parts.pop().toLowerCase();
                if (exportFormat === 'jpeg') exportFormat = 'jpg';
                exportFileName = parts.join('.');
            } else {
                exportFileName = parts[0] || 'export';
                exportFormat = 'png';
            }
        } else {
            exportFileName = 'export';
            exportFormat = 'png';
        }

        if (!exportDirectory) {
            try {
                exportDirectory = await documentDir();
            } catch (e) {
                console.warn('[ImageExportModal] Failed to get document directory:', e);
            }
        }

        modalTitle = `Export Image: ${exportFileName}.${exportFormat}`;
    }

    $: if (showModal) {
        initializeModalState();
    }

    async function selectExportDirectory() {
        try {
            const selectedPath = await open({
                directory: true,
                defaultPath: exportDirectory || undefined,
                title: 'Select Export Directory',
            });

            if (selectedPath && typeof selectedPath === 'string') {
                exportDirectory = selectedPath;
            }
        } catch (error) {
            console.error('[ImageExportModal] Error selecting export directory:', error);
        }
    }

    function handleConfirm() {
        if (!exportFileName || exportFileName.trim() === '') {
            alert('Please enter a filename.');
            return;
        }
        if (!exportDirectory || exportDirectory.trim() === '') {
            alert('Please select an export directory.');
            return;
        }

        // Robust path normalization using split/join instead of regex
        const normalizedDir = exportDirectory.split('\\').join(PATH_SEPARATOR);
        
        const dir = normalizedDir.endsWith(PATH_SEPARATOR) 
            ? normalizedDir.slice(0, -1) 
            : normalizedDir;
        
        const fullExportPath = dir + PATH_SEPARATOR + exportFileName + '.' + exportFormat;

        dispatch('export', {
            filePath: fullExportPath,
            includeAnnotations
        });
        closeModal();
    }

    function closeModal() {
        showModal = false;
        dispatch('close');
    }

    function handleKeydown(event) {
        if (showModal && event.key === 'Escape') {
            closeModal();
        }
        if (event.key === 'Enter') {
            const confirmButton = modalElement?.querySelector('.btn-primary');
            if (confirmButton && !confirmButton.disabled) {
                handleConfirm();
            }
        }
    }

    onMount(() => {
        window.addEventListener('keydown', handleKeydown);
    });

    onDestroy(() => {
        window.removeEventListener('keydown', handleKeydown);
    });
</script>

{#if showModal}
    <div
        bind:this={modalElement}
        class="fixed inset-0 z-[120] flex items-center justify-center bg-black/50 backdrop-blur-sm"
        on:click|self={closeModal}
        role="dialog"
        aria-modal="true"
        aria-labelledby="image-export-modal-title"
        tabindex="-1"
    >
        <div
            class="bg-white dark:bg-gray-900 p-6 rounded-lg shadow-xl w-full max-w-md m-4 flex flex-col text-gray-800 dark:text-gray-200"
            on:click|stopPropagation
            role="document"
        >
            <h2 id="image-export-modal-title" class="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-5 truncate" title="{modalTitle}">
                {modalTitle}
            </h2>

            <div class="space-y-4 text-sm text-gray-700 dark:text-gray-300">
                <!-- Filename Input -->
                <div>
                    <label for="export-filename" class="block font-medium text-gray-700 dark:text-gray-300 mb-1">Filename:</label>
                    <input
                        id="export-filename"
                        type="text"
                        bind:value={exportFileName}
                        class="input-field w-full bg-white dark:bg-gray-700 border-gray-300 dark:border-gray-600 text-gray-900 dark:text-gray-200 focus:ring-blue-500 focus:border-blue-500"
                        placeholder="e.g., MyExportedImage"
                        autocomplete="off"
                        autocorrect="off"
                    />
                </div>

                <!-- Format Dropdown -->
                <div>
                    <label for="export-format" class="block font-medium text-gray-700 dark:text-gray-300 mb-1">Format:</label>
                    <Dropdown
                        containerClasses="w-full"
                        options={exportFormats}
                        bind:value={exportFormat}
                        placeholder="Select a Format"
                    />
                </div>

                <!-- Directory Selection -->
                <div>
                    <label for="export-directory" class="block font-medium text-gray-700 dark:text-gray-300 mb-1 pt-2">Export To:</label>
                    <div class="flex space-x-2">
                        <input
                            id="export-directory"
                            type="text"
                            bind:value={exportDirectory}
                            class="input-field flex-grow bg-gray-100 dark:bg-gray-600 border-gray-300 dark:border-gray-500 text-gray-600 dark:text-gray-300 cursor-not-allowed"
                            readonly
                            placeholder="Select directory..."
                            autocomplete="off"
                            autocorrect="off"
                        />
                        <button type="button" on:click={selectExportDirectory} class="btn-secondary flex-shrink-0 text-xs px-3 py-1.5">
                            Browse
                        </button>
                    </div>
                </div>

                <!-- Include Annotations Checkbox -->
                <div class="pt-2">
                    <label class="flex items-center space-x-3 cursor-pointer group">
                        <input 
                            type="checkbox" 
                            bind:checked={includeAnnotations} 
                            class="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600"
                        />
                        <div class="flex flex-col">
                            <span class="font-medium text-gray-700 dark:text-gray-300">Include Annotations</span>
                        </div>
                    </label>
                </div>
            </div>

            <!-- Footer Buttons -->
            <div class="flex justify-end space-x-3 pt-4 border-t border-gray-200 dark:border-gray-600 mt-6">
                <button type="button" on:click={closeModal} class="btn-secondary">
                    Cancel
                </button>
                <button
                    type="button"
                    on:click={handleConfirm}
                    class="btn-primary"
                    disabled={!exportFileName || exportFileName.trim() === '' || !exportDirectory || exportDirectory.trim() === ''}
                >
                    Export {exportFormat.toUpperCase()}
                </button>
            </div>
        </div>
    </div>
{/if}

<style lang="postcss">
    .btn-primary, .btn-secondary {
        @apply px-4 py-2 rounded-md shadow-sm text-sm font-medium transition duration-150 ease-in-out;
    }
    .btn-primary {
        @apply bg-blue-600 text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:focus:ring-offset-gray-800 disabled:opacity-50 disabled:cursor-not-allowed disabled:bg-gray-400 dark:disabled:bg-gray-600;
    }
    .btn-secondary {
        @apply bg-gray-200 text-gray-700 hover:bg-gray-300 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-gray-500 dark:bg-gray-600 dark:text-gray-200 dark:hover:bg-gray-500 dark:focus:ring-offset-gray-800;
    }
    .input-field {
        @apply block w-full px-3 py-2 border rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 text-sm;
    }
    .dark .input-field {
        @apply bg-gray-700 border-gray-600 text-gray-200 placeholder-gray-400;
    }
    .dark .input-field:read-only {
        @apply bg-gray-600 border-gray-500 text-gray-300 cursor-not-allowed;
    }
</style>
