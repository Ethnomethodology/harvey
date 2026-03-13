<!-- src/lib/components/projectview/modals/ImageExportModal.svelte -->
<script>
    import { createEventDispatcher, onMount, onDestroy } from 'svelte';
    import { open } from '@tauri-apps/plugin-dialog';
    import { documentDir } from '@tauri-apps/api/path';
    import { 
        Input, 
        Label, 
        Select, 
        Button, 
        Helper,
        Checkbox,
        Modal
    } from 'flowbite-svelte';
    import { Share, FolderOpen, X, Image } from 'lucide-svelte';

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
        { value: 'png', name: 'PNG (.png)', disabled: false },
        { value: 'jpg', name: 'JPEG (.jpg)', disabled: false }
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
        if (showModal && event.key === 'Enter') {
            handleConfirm();
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
        class="fixed inset-0 z-[120] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
        on:click|self={closeModal}
        role="dialog"
        aria-modal="true"
        aria-labelledby="image-export-modal-title"
        tabindex="-1"
    >
        <div
            class="bg-white dark:bg-gray-900 rounded-xl shadow-2xl w-full max-w-md flex flex-col border border-gray-200 dark:border-gray-800 overflow-hidden"
            on:click|stopPropagation
            role="document"
        >
            <!-- Header -->
            <div class="px-6 py-5 border-b border-gray-200 dark:border-gray-800 flex justify-between items-center bg-gray-50/50 dark:bg-gray-800/50">
                <div class="flex items-center space-x-3">
                    <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
                        <Share size={20} class="text-blue-600 dark:text-blue-400" />
                    </div>
                    <h3 id="image-export-modal-title" class="text-lg font-bold text-gray-900 dark:text-white truncate max-w-[250px]" title="{modalTitle}">
                        Export Image
                    </h3>
                </div>
                <button on:click={closeModal} class="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-full transition-all" title="Close">
                    <X size={20} />
                </button>
            </div>

            <div class="p-6 space-y-5">
                <!-- Filename Input -->
                <div class="space-y-2">
                    <Label for="export-filename">Filename</Label>
                    <Input
                        id="export-filename"
                        type="text"
                        bind:value={exportFileName}
                        placeholder="e.g., MyExportedImage"
                        autocomplete="off"
                    />
                </div>

                <!-- Format Dropdown -->
                <div class="space-y-2">
                    <Label for="export-format">Export Format</Label>
                    <Select
                        id="export-format"
                        items={exportFormats}
                        bind:value={exportFormat}
                    />
                </div>

                <!-- Directory Selection -->
                <div class="space-y-2">
                    <Label for="export-directory">Destination Directory</Label>
                    <div class="flex gap-2">
                        <Input
                            id="export-directory"
                            type="text"
                            bind:value={exportDirectory}
                            readonly
                            class="flex-grow cursor-not-allowed bg-gray-50 dark:bg-gray-800"
                        />
                        <Button color="alternative" on:click={selectExportDirectory} class="px-3" title="Browse">
                            <FolderOpen size={18} />
                        </Button>
                    </div>
                </div>

                <!-- Include Annotations Checkbox -->
                <div class="pt-2">
                    <Checkbox bind:checked={includeAnnotations}>
                        Include Annotations
                    </Checkbox>
                </div>
            </div>

            <!-- Footer -->
            <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-800 flex justify-end gap-3 bg-gray-50/80 dark:bg-gray-800/80 backdrop-blur-md">
                <Button color="alternative" on:click={closeModal} title="Cancel">
                    Cancel
                </Button>
                <Button
                    color="blue"
                    on:click={handleConfirm}
                    title="Export to {exportFormat.toUpperCase()}"
                    disabled={!exportFileName || exportFileName.trim() === '' || !exportDirectory || exportDirectory.trim() === ''}
                >
                    Export {exportFormat.toUpperCase()}
                </Button>
            </div>
        </div>
    </div>
{/if}

