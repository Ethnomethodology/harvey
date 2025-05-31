<!-- src/lib/components/projectview/notes/shared_panels/LeftInfoPanel.svelte -->
<script>
    import { onMount } from 'svelte';
    import { project } from '$lib/stores/projectStore.js';
    // import { get } from 'svelte/store'; // Not strictly needed if using $project
    import { readTextFile, writeTextFile, rename as fsRename } from '@tauri-apps/plugin-fs'; // Aliased to avoid conflict
    import { dirname, basename, sep, extname } from '@tauri-apps/api/path';
    import { confirm, message } from '@tauri-apps/plugin-dialog';
    import { renameProjectItem } from '$lib/services/projectService.js';

    let currentFileMetadata = null;
    let fullLoadedMetadataObject = null;
    let isEditing = false;
    let editableMetadata = {
        file_name: '',
        title: '',
        description: '',
        summary: ''
    };

    const EDIT_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-pencil-square" viewBox="0 0 16 16"><path d="M15.502 1.94a.5.5 0 0 1 0 .706L14.459 3.69l-2-2L13.502.646a.5.5 0 0 1 .707 0l1.293 1.293zm-1.75 2.456-2-2L4.939 9.21a.5.5 0 0 0-.121.196l-.805 2.414a.25.25 0 0 0 .316.316l2.414-.805a.5.5 0 0 0 .196-.12l6.813-6.814z"/><path fill-rule="evenodd" d="M1 13.5A1.5 1.5 0 0 0 2.5 15h11a1.5 1.5 0 0 0 1.5-1.5v-6a.5.5 0 0 0-1 0v6a.5.5 0 0 1-.5.5h-11a.5.5 0 0 1-.5-.5v-11a.5.5 0 0 1 .5-.5H9a.5.5 0 0 0 0-1H2.5A1.5 1.5 0 0 0 1 2.5z"/></svg>`;
    const CANCEL_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-x-circle" viewBox="0 0 16 16"><path d="M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14m0 1A8 8 0 1 0 8 0a8 8 0 0 0 0 16"/><path d="M4.646 4.646a.5.5 0 0 1 .708 0L8 7.293l2.646-2.647a.5.5 0 0 1 .708.708L8.707 8l2.647 2.646a.5.5 0 0 1-.708.708L8 8.707l-2.646 2.647a.5.5 0 0 1-.708-.708L7.293 8 4.646 5.354a.5.5 0 0 1 0-.708"/></svg>`;

    const AUDIO_EXTENSIONS = new Set(['mp3','wav','m4a','ogg','aac','flac']);
    const VIDEO_EXTENSIONS = new Set(['mp4','mov','avi','mkv','webm']);
    const IMAGE_EXTENSIONS = new Set(['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'tiff']);

    let previousSelectedItemPath = null;

    onMount(() => {
        console.log('[LeftInfoPanel] Mounted.');
        // Initialize previousSelectedItemPath when the component mounts.
        previousSelectedItemPath = selectedItemPathInStore;

        // Initial load if a path is already selected and metadata isn't loaded
        if (selectedItemPathInStore && !currentFileMetadata) {
            console.log('[LeftInfoPanel onMount] Initial load for:', selectedItemPathInStore);
            loadMetadata(selectedItemPathInStore);
        }
    });

    async function loadMetadata(filePath) {
        currentFileMetadata = null;
        fullLoadedMetadataObject = null;
        // Also reset edit mode if a new file is being loaded, to prevent stale edit state
        if (isEditing && filePath !== previousSelectedItemPath) {
            isEditing = false;
        }
        let metadataPath;
        try {
            console.log(`[LeftInfoPanel DEBUG] loadMetadata called with filePath: ${filePath}`);
            const dirName = await dirname(filePath);
            console.log(`[LeftInfoPanel DEBUG] dirname resolved to: ${dirName}`);
            const baseName = await basename(filePath);
            console.log(`[LeftInfoPanel DEBUG] basename resolved to: ${baseName}`);
            const originalExtension = await extname(baseName);
            console.log(`[LeftInfoPanel DEBUG] originalExtension resolved to: ${originalExtension}`);
            let fileNameWithoutExtension;
            if (originalExtension.length > 0) {
                fileNameWithoutExtension = baseName.substring(0, baseName.length - originalExtension.length - 1);
            } else {
                fileNameWithoutExtension = baseName;
            }
            console.log(`[LeftInfoPanel DEBUG] fileNameWithoutExtension: ${fileNameWithoutExtension}`);

            const currentSep = sep();
            console.log(`[LeftInfoPanel DEBUG] path.sep resolved to: ${currentSep}`);

            const metadataFileName = `.${fileNameWithoutExtension}.metadata.json`;
            console.log(`[LeftInfoPanel DEBUG] constructed metadataFileName: ${metadataFileName}`);

            metadataPath = `${dirName}${currentSep}${metadataFileName}`;

            console.log(`[LeftInfoPanel] Loading metadata from: ${metadataPath}`);

            const fileContents = await readTextFile(metadataPath);
            const parsed = JSON.parse(fileContents);

            if (parsed && parsed.metadata) {
                fullLoadedMetadataObject = parsed;
                currentFileMetadata = fullLoadedMetadataObject.metadata;
                console.log('[LeftInfoPanel] Full metadata object loaded. Metadata part:', currentFileMetadata);
            } else {
                console.warn('[LeftInfoPanel] Metadata file does not contain a "metadata" property or is empty:', metadataPath);
            }
        } catch (error) {
            let errorMessage = typeof error === 'string' ? error : (error.message || JSON.stringify(error));
            if (errorMessage.includes('os error 2') || errorMessage.toLowerCase().includes('no such file or directory')) {
                 console.log(`[LeftInfoPanel] Metadata file not found for ${filePath} at actual path: ${metadataPath}. This is normal if no metadata has been saved yet. Error: ${errorMessage}`);
            } else {
                console.error(`[LeftInfoPanel] Error loading or parsing metadata for ${filePath} at actual path: ${metadataPath}. Error:`, error);
            }
        }
        if (!currentFileMetadata) { // Also turn off editing if metadata load failed or file has no metadata part
            if (isEditing) {
                console.log('[LeftInfoPanel] No currentFileMetadata after load, turning isEditing to false.');
                isEditing = false;
            }
        }
    }

    function toggleEditMode() {
        console.log('[LeftInfoPanel] toggleEditMode called. isEditing before:', isEditing);
        isEditing = !isEditing;
        console.log('[LeftInfoPanel] isEditing after:', isEditing);
    }

    async function handleSaveMetadata() {
        console.log('[LeftInfoPanel] handleSaveMetadata called.');
        let renameProcessed = false; // Initialize renameProcessed flag
        if (!currentFileMetadata || !currentFileMetadata.file_path || !fullLoadedMetadataObject) {
            console.error('[LeftInfoPanel] Save error: Missing critical metadata or file path info.');
            await message('Cannot save: Critical metadata information is missing. Please try reloading the file.', { title: 'Save Error', type: 'error' });
            return;
        }

        try {
            const originalFilePath = currentFileMetadata.file_path;
            const originalFileNameWithExtension = currentFileMetadata.file_name; // This is full name with ext
            const originalFileExtension = await extname(originalFileNameWithExtension); // e.g., "png"

            let originalFileNameWithoutExtension;
            if (originalFileExtension.length > 0) {
                originalFileNameWithoutExtension = originalFileNameWithExtension.substring(0, originalFileNameWithExtension.length - originalFileExtension.length - 1);
            } else {
                originalFileNameWithoutExtension = originalFileNameWithExtension;
            }

            const editedFileNameWithoutExtension = editableMetadata.file_name.trim();

            const originalDir = await dirname(originalFilePath);
            const currentSep = sep();

            if (editedFileNameWithoutExtension !== originalFileNameWithoutExtension) {
                if (!editedFileNameWithoutExtension) {
                    await message('File name cannot be empty.', { title: 'Invalid File Name', type: 'error' });
                    isEditing = true; // Stay in edit mode as user needs to correct it
                    return;
                }

                let nameToSendToService;
                if (currentItemType === 'doc' || currentItemType === 'image' || currentItemType === 'table') {
                    nameToSendToService = originalFileExtension.length > 0
                        ? editedFileNameWithoutExtension + "." + originalFileExtension
                        : editedFileNameWithoutExtension;
                } else {
                    // For 'media', 'imported_transcript', send only the stem.
                    nameToSendToService = editedFileNameWithoutExtension;
                }

                try {
                    console.log(`[LeftInfoPanel] Calling renameProjectItem: path=${originalFilePath}, nameToSend=${nameToSendToService}, type=${currentItemType}`);
                    await renameProjectItem(originalFilePath, nameToSendToService, currentItemType);

                    // NEW LOGIC:
                    console.log('[LeftInfoPanel] Rename successful via renameProjectItem. Waiting for store update and reactive reload.');
                    isEditing = false;
                    renameProcessed = true; // Set the flag
                } catch (err) {
                    console.error(`[LeftInfoPanel] renameProjectItem failed:`, err);
                    await message(`Error renaming item: ${err.message || err}`, { title: 'Rename Failed', type: 'error' });
                    // Do not revert editableMetadata.file_name here, allow user to see and correct their input if desired or cancel.
                    isEditing = true; // Stay in edit mode on error
                    return; // Return on error
                }
            } // End of rename block

            // Modify the subsequent metadata save block
            if (!renameProcessed) { // Check the flag
                const metadataPathForSave = `${originalDir}${currentSep}.${originalFileNameWithoutExtension}.metadata.json`;

                let updatedFileMetadata = { ...currentFileMetadata }; // Start with currently loaded metadata for this path
            updatedFileMetadata.title = editableMetadata.title.trim();
            updatedFileMetadata.description = editableMetadata.description.trim();
            updatedFileMetadata.summary = editableMetadata.summary.trim();
            updatedFileMetadata.last_modified = new Date().toISOString();
            // file_name and file_path in updatedFileMetadata remain unchanged as no rename happened here.

            let objectToWrite = { ...fullLoadedMetadataObject };
            objectToWrite.metadata = updatedFileMetadata;
            objectToWrite.version = objectToWrite.version || "1.0";
            objectToWrite.last_modified_harvey = new Date().toISOString();

            console.log(`[LeftInfoPanel] Writing metadata to: ${metadataPathForSave}`);
            await writeTextFile(metadataPathForSave, JSON.stringify(objectToWrite, null, 2));

            currentFileMetadata = { ...updatedFileMetadata };
            fullLoadedMetadataObject = { ...objectToWrite };

            isEditing = false;
            await message('Metadata saved successfully!', { title: 'Success' });
            } // End of if(!renameProcessed)
        } catch (err) {
            console.error('[LeftInfoPanel] Error saving metadata:', err);
            await message(`Error saving metadata: ${err.message || err}. Please check console for details.`, { title: 'Save Failed', type: 'error' });
        }
    }

    $: selectedItemPathInStore = $project.selectedDocumentPath || $project.currentImportedTranscriptPath || $project.selectedMediaNotePath;

    let currentItemType = null;
    $: if (selectedItemPathInStore && $project.baseDirectory) {
        const path = selectedItemPathInStore;
        // Try to get the actual filename from currentFileMetadata first, as it's most reliable after load
        const name = currentFileMetadata?.file_name || (path ? path.substring(path.lastIndexOf(sep()) + 1) : '');
        const ext = name.includes('.') ? name.split('.').pop().toLowerCase() : '';

        if (AUDIO_EXTENSIONS.has(ext) || VIDEO_EXTENSIONS.has(ext)) {
            currentItemType = 'media';
        } else if (IMAGE_EXTENSIONS.has(ext)) {
            currentItemType = 'image';
        } else if (ext === 'pdf' || ext === 'json' || ext === 'txt' || ext === 'md') {
            const isImportedTranscript = $project.importedTranscriptFiles?.some(f => f.relativePath && `${$project.baseDirectory}${sep()}${f.relativePath}` === path);
            if (isImportedTranscript) {
                currentItemType = 'imported_transcript';
            } else {
                currentItemType = 'doc';
            }
        } else if (ext === 'csv' || ext === 'xlsx') {
            currentItemType = 'table';
        } else {
            currentItemType = 'unknown';
        }
        console.log(`[LeftInfoPanel] Determined currentItemType: ${currentItemType} for path: ${path} (name: ${name}, ext: ${ext})`);
    } else {
        currentItemType = null;
    }

    $: {
        if (selectedItemPathInStore !== previousSelectedItemPath) {
            console.log(`[LeftInfoPanel] selectedItemPathInStore has changed FROM '${previousSelectedItemPath}' TO '${selectedItemPathInStore}'.`);
            if (isEditing) {
                console.log('[LeftInfoPanel] Resetting isEditing to false due to path change.');
                isEditing = false;
            }
            if (selectedItemPathInStore) {
                loadMetadata(selectedItemPathInStore);
            } else {
                currentFileMetadata = null;
                fullLoadedMetadataObject = null;
                currentItemType = null; // Reset item type if no path
            }
            previousSelectedItemPath = selectedItemPathInStore;
        } else if (!selectedItemPathInStore && previousSelectedItemPath !== null) {
             // This case handles when a file was selected, and then deselected (path becomes null)
            console.log(`[LeftInfoPanel] selectedItemPathInStore became null (was '${previousSelectedItemPath}'). Resetting metadata and edit state.`);
            currentFileMetadata = null;
            fullLoadedMetadataObject = null;
            currentItemType = null; // Reset item type
            if (isEditing) {
                isEditing = false;
            }
            previousSelectedItemPath = null;
        }
    }

    $: if (isEditing && currentFileMetadata) {
        console.log('[LeftInfoPanel] Populating editableMetadata because isEditing is true and currentFileMetadata exists.');
        // For file_name, only populate the stem for editing
        if (currentFileMetadata.file_name) {
            // extname returns extension without a dot, e.g., "png"
            const ext = currentFileMetadata.file_name.includes('.') ? currentFileMetadata.file_name.substring(currentFileMetadata.file_name.lastIndexOf('.') + 1) : '';
            const nameWithoutExt = ext ? currentFileMetadata.file_name.substring(0, currentFileMetadata.file_name.length - (ext.length + (ext ? 1:0)) ) : currentFileMetadata.file_name;
            editableMetadata.file_name = nameWithoutExt;
        } else {
            editableMetadata.file_name = '';
        }
        editableMetadata.title = currentFileMetadata.title || '';
        editableMetadata.description = currentFileMetadata.description || '';
        editableMetadata.summary = currentFileMetadata.summary || '';
    } else if (!isEditing) {
        // Clear form when not editing or no metadata
        editableMetadata = { file_name: '', title: '', description: '', summary: '' };
    }

</script>

<div class="h-full bg-white dark:bg-gray-800 rounded-md shadow p-3 flex flex-col overflow-hidden">
    <h2 class="text-sm font-semibold mb-3 border-b pb-1 border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 flex-shrink-0 flex justify-between items-center">
        <span>Metadata</span>
        {#if currentFileMetadata}
            <button
                on:click={toggleEditMode}
                class="p-1 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                title={isEditing ? 'Cancel Edit' : 'Edit Metadata'}
            >
                {@html isEditing ? CANCEL_ICON_SVG : EDIT_ICON_SVG}
            </button>
        {/if}
    </h2>
    <div class="flex-grow overflow-y-auto min-h-0 text-xs relative">
        {#if currentFileMetadata}
            <div class="p-1 space-y-2"> {/* This space-y-2 might become redundant or need adjustment */}
                <!-- File Name (editable for stem, display full) -->
                <div class="mb-3">
                    <label class="font-medium text-gray-600 dark:text-gray-400 block mb-1">File Name:</label>
                    {#if isEditing}
                        <input type="text" bind:value={editableMetadata.file_name} class="mt-0.5 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50 dark:bg-gray-700 dark:border-gray-600 dark:text-white p-1 text-xs" placeholder="Enter name without extension"/>
                        {#if currentFileMetadata.file_name.includes('.')}
                            <span class="mt-1 text-gray-500 dark:text-gray-400 text-xs block">
                                Extension: {currentFileMetadata.file_name.substring(currentFileMetadata.file_name.lastIndexOf('.'))}
                            </span>
                        {/if}
                    {:else}
                        <span class="text-gray-800 dark:text-gray-200 block">{currentFileMetadata.file_name || 'N/A'}</span>
                    {/if}
                </div>

                <!-- File Path (read-only) -->
                <div class="mb-3">
                    <label class="font-medium text-gray-600 dark:text-gray-400 block mb-1">File Path:</label>
                    <span class="text-gray-800 dark:text-gray-200 break-all block">{currentFileMetadata.file_path || 'N/A'}</span>
                </div>

                <!-- Last Modified (read-only) -->
                <div class="mb-3">
                    <label class="font-medium text-gray-600 dark:text-gray-400 block mb-1">Last Modified:</label>
                    <span class="text-gray-800 dark:text-gray-200 block">{currentFileMetadata.last_modified ? new Date(currentFileMetadata.last_modified).toLocaleString() : 'N/A'}</span>
                </div>

                <!-- Title (editable) -->
                <div class="mb-3">
                    <label class="font-medium text-gray-600 dark:text-gray-400 block mb-1">Title:</label>
                    {#if isEditing}
                        <input type="text" bind:value={editableMetadata.title} class="mt-0.5 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50 dark:bg-gray-700 dark:border-gray-600 dark:text-white p-1 text-xs" />
                    {:else}
                        <span class="text-gray-800 dark:text-gray-200 block">{currentFileMetadata.title || 'N/A'}</span>
                    {/if}
                </div>

                <!-- Description (editable) -->
                <div class="mb-3">
                    <label class="font-medium text-gray-600 dark:text-gray-400 block mb-1">Description:</label>
                    {#if isEditing}
                        <textarea bind:value={editableMetadata.description} rows="3" class="mt-0.5 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50 dark:bg-gray-700 dark:border-gray-600 dark:text-white p-1 text-xs"></textarea>
                    {:else}
                        <span class="text-gray-800 dark:text-gray-200 whitespace-pre-wrap block">{currentFileMetadata.description || 'N/A'}</span>
                    {/if}
                </div>

                <!-- Summary (editable) -->
                <div class="mb-3">
                    <label class="font-medium text-gray-600 dark:text-gray-400 block mb-1">Summary:</label>
                    {#if isEditing}
                        <textarea bind:value={editableMetadata.summary} rows="2" class="mt-0.5 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50 dark:bg-gray-700 dark:border-gray-600 dark:text-white p-1 text-xs"></textarea>
                    {:else}
                        <span class="text-gray-800 dark:text-gray-200 whitespace-pre-wrap block">{currentFileMetadata.summary || 'N/A'}</span>
                    {/if}
                </div>

                {#if isEditing}
                    <div class="mt-3 flex justify-end">
                        <button
                            on:click={handleSaveMetadata}
                            class="px-3 py-1.5 bg-blue-600 hover:bg-blue-700 text-white rounded-md text-xs font-medium focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-opacity-50"
                        >
                            Save
                        </button>
                    </div>
                {/if}
            </div>
        {:else}
            <p class="text-gray-500 dark:text-gray-400 italic px-1 py-2">
                No file selected or metadata not available.
            </p>
        {/if}
    </div>
</div>

<style lang="postcss">
    .min-h-0 { min-height: 0; }
    .overflow-y-auto::-webkit-scrollbar { @apply w-[6px] h-[6px]; }
    .overflow-y-auto::-webkit-scrollbar-track { @apply bg-transparent; }
    .overflow-y-auto::-webkit-scrollbar-thumb { @apply rounded bg-gray-400/50 dark:bg-gray-500/50; }
    .overflow-y-auto::-webkit-scrollbar-thumb:hover { @apply bg-gray-500/70 dark:bg-gray-400/70; }
    .overflow-y-auto { scrollbar-width: thin; scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track); }
    :root { --scrollbar-thumb: rgba(156, 163, 175, 0.5); --scrollbar-track: transparent; }
    html.dark { --scrollbar-thumb: rgba(107, 114, 128, 0.5); }
</style>