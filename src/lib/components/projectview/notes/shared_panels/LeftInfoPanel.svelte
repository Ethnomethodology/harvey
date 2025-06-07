<!-- src/lib/components/projectview/notes/shared_panels/LeftInfoPanel.svelte -->
<script>
    import { onMount } from 'svelte';
    import { project } from '$lib/stores/projectStore.js';
    import { invoke } from '@tauri-apps/api/core';
    // fsRename might still be used by projectService.js, direct fs calls for metadata are removed.
    import { basename, extname as getFileExtname, sep as getPathSep } from '@tauri-apps/api/path'; // dirname removed
    import { confirm, message } from '@tauri-apps/plugin-dialog';
    import { renameProjectItem } from '$lib/services/projectService.js';
    import AddFieldModal from '$lib/components/projectview/modals/AddFieldModal.svelte';
    import FileEarmarkCodeIcon from '$lib/components/icons/FileEarmarkCodeIcon.svelte';
    import panelStateStore from '$lib/stores/panelStateStore.js';

    function formatDuration(seconds) {
        if (!seconds && seconds !== 0) return '';
        const h = Math.floor(seconds / 3600);
        const m = Math.floor((seconds % 3600) / 60);
        const s = Math.floor(seconds % 60);
        const ms = Math.round((seconds - Math.floor(seconds)) * 1000);

        let str = '';
        if (h > 0) str += `${h}:`;
        str += `${m < 10 && h > 0 ? '0' : ''}${m}:`;
        str += `${s < 10 ? '0' : ''}${s}`;
        if (ms > 0) str += `.${String(ms).padStart(3, '0').substring(0,3)}`;
        return str;
    }

    function formatBitrate(bps) {
        if (!bps) return '';
        if (bps >= 1000000) {
            return (bps / 1000000).toFixed(2) + ' Mbps';
        }
        if (bps >= 1000) {
            return (bps / 1000).toFixed(0) + ' kbps';
        }
        return bps + ' bps';
    }

    let currentFileMetadata = null; // Will store FileMetadata like structure, including customFields
    let fullLoadedMetadataObject = null; // May store the raw object from DB (FileMetadataWithCustomFieldsFromDb) or be refactored/removed
    let isEditing = false;
    let editableMetadata = {
        file_name: '',
        title: '',
        description: '',
        summary: '',
        customFields: []
    };
    let showAddFieldModal = false;

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

    async function loadMetadata(assetRelativePath) { // Renamed filePath to assetRelativePath for clarity
        currentFileMetadata = null;
        fullLoadedMetadataObject = null;
        if (isEditing && assetRelativePath !== previousSelectedItemPath) { // previousSelectedItemPath now stores relative path
            isEditing = false;
        }

        if (!assetRelativePath) {
            console.warn('[LeftInfoPanel] loadMetadata called with no assetRelativePath.');
            return;
        }

        try {
            console.log(`[LeftInfoPanel] Loading metadata from DB for relative path: ${assetRelativePath}`);
            const result = await invoke('get_asset_metadata_command', { assetRelativePath: assetRelativePath });

            if (result) {
                // result is FileMetadataWithCustomFieldsFromDb
                currentFileMetadata = {
                    file_name: result.file_name,
                    file_path: result.file_path, // This is the relative path (key)
                    last_modified: result.last_modified,
                    title: result.title || '',
                    description: result.description || '',
                    summary: result.summary || '',
                    duration_seconds: result.duration_seconds,
                    width: result.width,
                    height: result.height,
                    frame_rate: result.frame_rate,
                    bit_rate: result.bit_rate,
                    audio_codec: result.audio_codec,
                    video_codec: result.video_codec,
                    creation_time: result.creation_time,
                    customFields: result.custom_fields_json ? JSON.parse(result.custom_fields_json) : [],
                };
                fullLoadedMetadataObject = {
                    metadata: { ...currentFileMetadata },
                    customFields: currentFileMetadata.customFields,
                    asset_type: result.asset_type,
                    version: "db_1.0"
                };
                console.log('[LeftInfoPanel] Metadata loaded from DB:', currentFileMetadata);
            } else {
                console.warn('[LeftInfoPanel] No metadata found in DB for:', assetRelativePath);
                const baseNameStr = await basename(assetRelativePath); // Get basename from relative path
                currentFileMetadata = {
                    file_name: baseNameStr,
                    file_path: assetRelativePath,
                    last_modified: new Date().toISOString(),
                    title: '', description: '', summary: '', customFields: []
                };
                fullLoadedMetadataObject = { metadata: { ...currentFileMetadata }, customFields: [], version: "db_1.0_new" };
            }
        } catch (error) {
            console.error(`[LeftInfoPanel] Error loading metadata from DB for ${assetRelativePath}:`, error);
            const baseNameStrOnError = await basename(assetRelativePath).catch(() => assetRelativePath);
            currentFileMetadata = { file_name: baseNameStrOnError, file_path: assetRelativePath, title: '', description: '', summary: '', customFields: [] };
            fullLoadedMetadataObject = { metadata: { ...currentFileMetadata }, customFields: [], version: "db_1.0_error" };
            await message(`Error loading metadata: ${error}`, { title: 'Load Error', type: 'error' });
        }

        if (!currentFileMetadata) { // Should have been initialized by logic above
            const fallbackBasename = await basename(assetRelativePath).catch(() => 'Unknown File');
            currentFileMetadata = { file_name: fallbackBasename, file_path: assetRelativePath, title: '', description: '', summary: '', customFields: [] };
            if (isEditing) isEditing = false;
        }
    }

    function toggleEditMode() {
        console.log('[LeftInfoPanel] toggleEditMode called. isEditing before:', isEditing);
        isEditing = !isEditing;
        console.log('[LeftInfoPanel] isEditing after:', isEditing);
    }

    async function handleSaveMetadata() {
        console.log('[LeftInfoPanel] handleSaveMetadata called.');
        let renameProcessed = false;
        if (!currentFileMetadata || !currentFileMetadata.file_path) { // file_path is the relative path (DB key)
            console.error('[LeftInfoPanel] Save error: Missing file_path in currentFileMetadata.');
            await message('Cannot save: File path information is missing.', { title: 'Save Error', type: 'error' });
            return;
        }
        if (!currentItemType) {
            console.error('[LeftInfoPanel] Save error: currentItemType is not set.');
            await message('Cannot save: Item type is unknown.', { title: 'Save Error', type: 'error' });
            return;
        }

        try {
            // currentFileMetadata.file_path should be the relative path to the asset
            const assetKeyForDb = currentFileMetadata.file_path;
            const currentFullFileName = currentFileMetadata.file_name; // e.g., "myDocument.pdf" or "myTranscript.json"

            const currentFileExtension = await getFileExtname(currentFullFileName); // e.g., "pdf" or "json" (no dot)

            let currentFileNameWithoutExtension;
            if (currentFileExtension && currentFileExtension.length > 0) {
                currentFileNameWithoutExtension = currentFullFileName.substring(0, currentFullFileName.length - currentFileExtension.length -1);
            } else {
                currentFileNameWithoutExtension = currentFullFileName;
            }

            const editedFileNameStem = editableMetadata.file_name.trim(); // This is the stem from the input field

            // Check if file name (stem) has changed
            if (editedFileNameStem !== currentFileNameWithoutExtension) {
                if (!editedFileNameStem) {
                    await message('File name (stem) cannot be empty.', { title: 'Invalid File Name', type: 'error' });
                    isEditing = true;
                    return;
                }

                let nameToSendToBackendRenameService;
                // Backend's renameProjectItem expects stem for 'media' and 'imported_transcript', full new name for others.
                if (currentItemType === 'media' || currentItemType === 'imported_transcript') {
                    nameToSendToBackendRenameService = editedFileNameStem;
                } else {
                    nameToSendToBackendRenameService = currentFileExtension && currentFileExtension.length > 0
                        ? editedFileNameStem + "." + currentFileExtension
                        : editedFileNameStem;
                }

                try {
                    console.log(`[LeftInfoPanel] Calling renameProjectItem service: path=${assetKeyForDb}, nameToSend=${nameToSendToBackendRenameService}, type=${currentItemType}`);
                    // renameProjectItem service is expected to handle renaming in DB (key and fields) and filesystem, then update XML.
                    // The store update from XML change should trigger a reactive reload of metadata.
                    await renameProjectItem(assetKeyForDb, nameToSendToBackendRenameService, currentItemType);

                    console.log('[LeftInfoPanel] Rename successful via renameProjectItem. Store update should handle metadata reload.');
                    isEditing = false;
                    renameProcessed = true;
                    // Important: After a successful rename, currentFileMetadata might be stale if the path/key changed.
                    // The reactive flow an
                    // d loadMetadata are expected to pick up the new path from the store and reload.
                    // So, we might not need to save further metadata fields in this same execution path if rename occurred.
                    // The user would effectively save title/desc *after* the rename is committed and UI reloads.
                    // For now, if renameProcessed is true, we skip the direct metadata save part below.
                } catch (err) {
                    console.error(`[LeftInfoPanel] renameProjectItem failed:`, err);
                    await message(`Error renaming item: ${err.message || err}`, { title: 'Rename Failed', type: 'error' });
                    isEditing = true;
                    return;
                }
            }

            // If rename happened, the metadata (like title, desc) that might have been edited
            // in the same form submission won't be saved here because the component will likely reload
            // due to path changes. This is a common pattern: rename is a distinct operation.
            // If we wanted to save other fields *after* rename, the `assetKeyForDb` would need to be the *new* key.
            if (!renameProcessed) {
                const metadataPayloadForDb = {
                    file_name: currentFileMetadata.file_name, // This should be the current, correct filename
                    file_path: assetKeyForDb,                 // This is the asset's relative path (key)
                    last_modified: new Date().toISOString(),  // Will be updated by DB trigger too
                    title: editableMetadata.title.trim(),
                    description: editableMetadata.description.trim(),
                    summary: editableMetadata.summary.trim(),
                    // Pass through existing technical metadata if it's part of FileMetadata struct
                    duration_seconds: currentFileMetadata.duration_seconds,
                    width: currentFileMetadata.width,
                    height: currentFileMetadata.height,
                    frame_rate: currentFileMetadata.frame_rate,
                    bit_rate: currentFileMetadata.bit_rate,
                    audio_codec: currentFileMetadata.audio_codec,
                    video_codec: currentFileMetadata.video_codec,
                    creation_time: currentFileMetadata.creation_time,
                };

                const customFieldsToSaveForDb = editableMetadata.customFields || [];

                try {
                    console.log(`[LeftInfoPanel] Saving metadata to DB for: ${assetKeyForDb}`);
                    await invoke('update_asset_metadata_command', {
                        assetRelativePath: assetKeyForDb,
                        metadataPayload: metadataPayloadForDb,
                        customFieldsPayload: customFieldsToSaveForDb,
                        assetType: currentItemType
                    });

                    // Update local state to reflect saved data
                    currentFileMetadata.title = metadataPayloadForDb.title;
                    currentFileMetadata.description = metadataPayloadForDb.description;
                    currentFileMetadata.summary = metadataPayloadForDb.summary;
                    currentFileMetadata.last_modified = metadataPayloadForDb.last_modified; // Reflect new save time
                    currentFileMetadata.customFields = JSON.parse(JSON.stringify(customFieldsToSaveForDb));

                    if (fullLoadedMetadataObject) { // Update this as well if it's being used
                        fullLoadedMetadataObject.metadata = { ...currentFileMetadata }; // Update its metadata part
                        fullLoadedMetadataObject.customFields = currentFileMetadata.customFields;
                        // fullLoadedMetadataObject.asset_type should already be correct from load
                    }
                    isEditing = false;
                    await message('Metadata saved successfully!', { title: 'Success' });

                } catch (err) {
                    console.error('[LeftInfoPanel] Error saving metadata to DB:', err);
                    await message(`Error saving metadata: ${err}. Please check console.`, { title: 'Save Failed', type: 'error' });
                    // isEditing = true; // Optionally keep editing mode
                }
            }
        } catch (err) {
            console.error('[LeftInfoPanel] General error in handleSaveMetadata:', err);
            await message(`An unexpected error occurred: ${err.message || err}.`, { title: 'Error', type: 'error' });
            isEditing = true;
        }
    }

    $: selectedItemPathInStore = $project.selectedDocumentPath || $project.currentImportedTranscriptPath || $project.selectedMediaNotePath;

    let currentItemType = null;
    $: if (selectedItemPathInStore && $project.baseDirectory) {
        const path = selectedItemPathInStore;
        // Try to get the actual filename from currentFileMetadata first, as it's most reliable after load
        const name = currentFileMetadata?.file_name || (path ? path.substring(path.lastIndexOf(getPathSep) + 1) : '');
        const ext = name.includes('.') ? name.split('.').pop().toLowerCase() : '';

        if (AUDIO_EXTENSIONS.has(ext) || VIDEO_EXTENSIONS.has(ext)) {
            currentItemType = 'media';
        } else if (IMAGE_EXTENSIONS.has(ext)) {
            currentItemType = 'image';
        } else if (ext === 'pdf' || ext === 'json' || ext === 'txt' || ext === 'md') {
            const isImportedTranscript = $project.importedTranscriptFiles?.some(f => f.relativePath && `${$project.baseDirectory}${getPathSep}${f.relativePath}` === path);
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
        // selectedItemPathInStore is an absolute path from the projectStore
        // It needs to be converted to a relative path for DB operations and for previousSelectedItemPath tracking
        let currentRelativePath = null;
        if (selectedItemPathInStore && $project.baseDirectory && selectedItemPathInStore.startsWith($project.baseDirectory)) {
            let relPath = selectedItemPathInStore.substring($project.baseDirectory.length);
            const pathSep = $project.baseDirectory.includes('/') ? '/' : '\\'; // Determine separator from baseDirectory
            if (relPath.startsWith(pathSep)) {
                relPath = relPath.substring(1);
            }
            currentRelativePath = relPath.replace(/\\/g, '/'); // Normalize to forward slashes
        } else if (selectedItemPathInStore) {
            // If it's not under baseDirectory, it might be an external file or an issue.
            // For now, treat it as if no valid selection for DB metadata.
            console.warn(`[LeftInfoPanel] Selected path ${selectedItemPathInStore} is not relative to project base ${$project.baseDirectory}. Cannot load DB metadata.`);
        }

        if (currentRelativePath !== previousSelectedItemPath) {
            console.log(`[LeftInfoPanel] Relative path for metadata has changed FROM '${previousSelectedItemPath}' TO '${currentRelativePath}'.`);
            if (isEditing) {
                console.log('[LeftInfoPanel] Resetting isEditing to false due to path change.');
                isEditing = false;
            }
            if (currentRelativePath) {
                loadMetadata(currentRelativePath);
            } else {
                currentFileMetadata = null;
                fullLoadedMetadataObject = null;
                currentItemType = null;
            }
            previousSelectedItemPath = currentRelativePath; // Store the relative path now
        } else if (!currentRelativePath && previousSelectedItemPath !== null) {
            console.log(`[LeftInfoPanel] Relative path became null (was '${previousSelectedItemPath}'). Resetting metadata.`);
            currentFileMetadata = null;
            fullLoadedMetadataObject = null;
            currentItemType = null;
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
        editableMetadata.customFields = JSON.parse(JSON.stringify(currentFileMetadata.customFields || [])); // Deep copy
    } else if (!isEditing) {
        // Clear form when not editing or no metadata
        editableMetadata = { file_name: '', title: '', description: '', summary: '', customFields: [] };
    }

    function handleAddCustomFieldConfirm(event) {
        const newField = event.detail; // { key, type, value }
        if (!editableMetadata.customFields) {
            editableMetadata.customFields = [];
        }
        // Enforce unique field names (case-insensitive)
        if (editableMetadata.customFields.some(f => f.key.toLowerCase() === newField.key.toLowerCase())) {
            message(`A custom field with the name "${newField.key}" already exists.`, { title: 'Duplicate Field Name', type: 'warning' });
            return; // Keep the modal open by not setting showAddFieldModal to false
        }
        editableMetadata.customFields = [...editableMetadata.customFields, newField];
        showAddFieldModal = false;
    }

</script>

<div class="h-full bg-white dark:bg-gray-800 rounded-md shadow flex flex-col overflow-hidden transition-all duration-300 ease-in-out"
      class:p-3={!$panelStateStore.leftCollapsed}
      class:p-2={$panelStateStore.leftCollapsed}
      class:w-full={!$panelStateStore.leftCollapsed}
      class:w-12={$panelStateStore.leftCollapsed} >
    <h2 class="text-sm font-semibold border-b pb-1 border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 flex-shrink-0 flex items-center"
        class:mb-3={!$panelStateStore.leftCollapsed}
        class:mb-0={$panelStateStore.leftCollapsed}
        class:justify-between={!$panelStateStore.leftCollapsed}
        class:justify-center={$panelStateStore.leftCollapsed} >
        <button
            on:click={panelStateStore.toggleLeftPanel}
            class="p-1 text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
            title={$panelStateStore.leftCollapsed ? 'Expand Metadata' : 'Collapse Metadata'}
        >
            <FileEarmarkCodeIcon class="w-4 h-4"/>
        </button>
        {#if !$panelStateStore.leftCollapsed}
            <span class="ml-2">Metadata</span>
            {#if currentFileMetadata}
                <button
                    on:click={toggleEditMode}
                    class="p-1 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                    title={isEditing ? 'Cancel Edit' : 'Edit Metadata'}
                >
                    {@html isEditing ? CANCEL_ICON_SVG : EDIT_ICON_SVG}
                </button>
            {/if}
        {/if}
    </h2>
    <div class="flex-grow overflow-y-auto min-h-0 text-xs relative">
        {#if !$panelStateStore.leftCollapsed}
                <!-- Expanded View -->
                {#if currentFileMetadata}
                <div class="p-1 space-y-2">
                    <!-- File Name (editable for stem, display full) -->
                <div class="mb-3">
                    <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">File Name:</label>
                    {#if isEditing}
                        <input type="text" bind:value={editableMetadata.file_name} class="mt-0.5 block w-full rounded-md border border-gray-300 dark:border-gray-600 focus:ring-1 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white px-1.5 py-1 text-xs bg-white text-gray-900" placeholder="Enter name without extension" autocorrect="off" autocomplete="off"/>
                        {#if currentFileMetadata.file_name.includes('.')}
                            <span class="mt-1 text-gray-500 dark:text-gray-400 text-xs block">
                                Extension: {currentFileMetadata.file_name.substring(currentFileMetadata.file_name.lastIndexOf('.'))}
                            </span>
                        {/if}
                    {:else}
                        <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">{currentFileMetadata.file_name || 'N/A'}</span>
                    {/if}
                </div>

                <!-- File Path (read-only) -->
                <div class="mb-3">
                    <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">File Path:</label>
                    <span class="text-gray-900 dark:text-gray-100 break-all block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">{currentFileMetadata.file_path || ''}</span>
                </div>

                <!-- Last Modified (read-only) -->
                <div class="mb-3">
                    <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Last Modified:</label>
                    <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">{currentFileMetadata.last_modified ? new Date(currentFileMetadata.last_modified).toLocaleString() : ''}</span>
                </div>

                <!-- Title (editable) -->
                <div class="mb-3">
                    <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Title:</label>
                    {#if isEditing}
                        <input type="text" bind:value={editableMetadata.title} class="mt-0.5 block w-full rounded-md border border-gray-300 dark:border-gray-600 focus:ring-1 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white px-1.5 py-1 text-xs bg-white text-gray-900" autocorrect="off" autocomplete="off"/>
                    {:else}
                        <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">{currentFileMetadata.title || ''}</span>
                    {/if}
                </div>

                <!-- Description (editable) -->
                <div class="mb-3">
                    <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Description:</label>
                    {#if isEditing}
                        <textarea bind:value={editableMetadata.description} rows="3" class="mt-0.5 block w-full rounded-md border border-gray-300 dark:border-gray-600 focus:ring-1 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white px-1.5 py-1 text-xs bg-white text-gray-900" autocorrect="off" autocomplete="off"></textarea>
                    {:else}
                        <span class="text-gray-900 dark:text-gray-100 whitespace-pre-wrap block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">{currentFileMetadata.description || ''}</span>
                    {/if}
                </div>

                <!-- Summary (editable) -->
                <div class="mb-3">
                    <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Summary:</label>
                    {#if isEditing}
                        <textarea bind:value={editableMetadata.summary} rows="2" class="mt-0.5 block w-full rounded-md border border-gray-300 dark:border-gray-600 focus:ring-1 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white px-1.5 py-1 text-xs bg-white text-gray-900" autocorrect="off" autocomplete="off"></textarea>
                    {:else}
                        <span class="text-gray-900 dark:text-gray-100 whitespace-pre-wrap block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">{currentFileMetadata.summary || ''}</span>
                    {/if}
                </div>

                <!-- Technical Metadata Section -->
                {#if currentFileMetadata.duration_seconds || currentFileMetadata.width || currentFileMetadata.video_codec || currentFileMetadata.audio_codec || currentFileMetadata.bit_rate || currentFileMetadata.creation_time}
                    <hr class="my-4 border-gray-300 dark:border-gray-700">
                    <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 tracking-wider mb-2">Technical Details</h3>

                    {#if currentFileMetadata.duration_seconds}
                        <div class="mb-3">
                            <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Duration:</label>
                            <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">{formatDuration(currentFileMetadata.duration_seconds)}</span>
                        </div>
                    {/if}

                    {#if currentFileMetadata.width && currentFileMetadata.height}
                        <div class="mb-3">
                            <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Dimensions:</label>
                            <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">{currentFileMetadata.width} x {currentFileMetadata.height}</span>
                        </div>
                    {/if}

                    {#if currentFileMetadata.frame_rate}
                        <div class="mb-3">
                            <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Frame Rate:</label>
                            <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">{currentFileMetadata.frame_rate.toFixed(2)} fps</span>
                        </div>
                    {/if}

                    {#if currentFileMetadata.bit_rate}
                        <div class="mb-3">
                            <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Bit Rate:</label>
                            <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">{formatBitrate(currentFileMetadata.bit_rate)}</span>
                        </div>
                    {/if}

                    {#if currentFileMetadata.video_codec}
                        <div class="mb-3">
                            <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Video Codec:</label>
                            <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">{currentFileMetadata.video_codec || ''}</span>
                        </div>
                    {/if}

                    {#if currentFileMetadata.audio_codec}
                        <div class="mb-3">
                            <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Audio Codec:</label>
                            <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">{currentFileMetadata.audio_codec || ''}</span>
                        </div>
                    {/if}

                    {#if currentFileMetadata.creation_time}
                        <div class="mb-3">
                            <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Creation Time:</label>
                            <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">{currentFileMetadata.creation_time ? new Date(currentFileMetadata.creation_time).toLocaleString() : ''}</span>
                        </div>
                    {/if}
                {/if}
                <!-- End of Technical Metadata Section -->

                <!-- Custom Fields Section -->
                {#if ( (!isEditing && currentFileMetadata?.customFields?.length > 0) || (isEditing && editableMetadata?.customFields?.length > 0) || isEditing )}
                    <hr class="my-4 border-gray-300 dark:border-gray-700">
                    <div class="flex justify-between items-center mb-2">
                        <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 tracking-wider">Custom Fields</h3>
                        {#if isEditing}
                            <button
                                on:click={() => showAddFieldModal = true}
                                class="p-1 text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                                title="Add Custom Field"
                            >
                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-plus-circle" viewBox="0 0 16 16">
                                    <path d="M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14zm0 1A8 8 0 1 0 8 0a8 8 0 0 0 0 16z"/>
                                    <path d="M8 4a.5.5 0 0 1 .5.5v3h3a.5.5 0 0 1 0 1h-3v3a.5.5 0 0 1-1 0v-3h-3a.5.5 0 0 1 0-1h3v-3A.5.5 0 0 1 8 4z"/>
                                </svg>
                            </button>
                        {/if}
                    </div>
                {/if}

                <!-- Read Mode Custom Fields -->
                {#if !isEditing && currentFileMetadata && currentFileMetadata.customFields}
                    {#each currentFileMetadata.customFields as field, index (field.key + '-' + index)}
                        <div class="mb-3">
                            <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">{field.key}:</label>
                            <span class="text-gray-900 dark:text-gray-100 {field.type === 'long_text' ? 'whitespace-pre-wrap' : 'break-all'} block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">
                                {field.value || ''}
                            </span>
                        </div>
                    {/each}
                {/if}

                <!-- Edit Mode Custom Fields -->
                {#if isEditing && editableMetadata && editableMetadata.customFields}
                    {#each editableMetadata.customFields as field, index (field.key + '-' + index)}
                        <div class="mb-3">
                            <label for={`custom-field-edit-${index}`} class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">{field.key}:</label>
                            {#if field.type === 'small_text'}
                                <input
                                    type="text"
                                    id={`custom-field-edit-${index}`}
                                    bind:value={editableMetadata.customFields[index].value}
                                    class="mt-0.5 block w-full rounded-md border border-gray-300 dark:border-gray-600 focus:ring-1 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white px-1.5 py-1 text-xs bg-white text-gray-900"
                                    placeholder={`Enter value for ${field.key}`}
                                    autocorrect="off" autocomplete="off"/>
                            {:else if field.type === 'long_text'}
                                <textarea
                                    id={`custom-field-edit-${index}`}
                                    rows="3"
                                    bind:value={editableMetadata.customFields[index].value}
                                    class="mt-0.5 block w-full rounded-md border border-gray-300 dark:border-gray-600 focus:ring-1 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white px-1.5 py-1 text-xs bg-white text-gray-900"
                                    placeholder={`Enter value for ${field.key}`}
                                    autocorrect="off" autocomplete="off"></textarea>
                            {/if}
                            <!-- Optional: Add a small remove button here later -->
                            <!-- <button on:click={() => removeCustomField(index)} class="text-red-500 text-xs">Remove</button> -->
                        </div>
                    {/each}
                {/if}
                <!-- End of custom fields rendering -->

                {#if isEditing}
                    <div class="mt-4 flex justify-end items-center">
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
        {:else}
            <!-- Collapsed View: Vertical Labels -->
            <div class="pt-2 flex flex-col items-center space-y-1.5 text-gray-600 dark:text-gray-400 w-full px-0.5">
                <div class="text-xs w-full text-center truncate border border-gray-300 dark:border-gray-600 px-1 py-0.5 bg-gray-50 dark:bg-gray-700/30 rounded-sm" title={`File Name: ${currentFileMetadata?.file_name ?? 'N/A'}`}>Name</div>
                <div class="text-xs w-full text-center truncate border border-gray-300 dark:border-gray-600 px-1 py-0.5 bg-gray-50 dark:bg-gray-700/30 rounded-sm" title={`File Path: ${currentFileMetadata?.file_path ?? 'N/A'}`}>Path</div>
                <div class="text-xs w-full text-center truncate border border-gray-300 dark:border-gray-600 px-1 py-0.5 bg-gray-50 dark:bg-gray-700/30 rounded-sm" title={`Date: ${currentFileMetadata?.last_modified ? new Date(currentFileMetadata.last_modified).toLocaleString() : 'N/A'}`}>Date</div>
                <div class="text-xs w-full text-center truncate border border-gray-300 dark:border-gray-600 px-1 py-0.5 bg-gray-50 dark:bg-gray-700/30 rounded-sm" title={`Title: ${currentFileMetadata?.title ?? 'N/A'}`}>Title</div>
                <div class="text-xs w-full text-center truncate border border-gray-300 dark:border-gray-600 px-1 py-0.5 bg-gray-50 dark:bg-gray-700/30 rounded-sm" title={`Description: ${currentFileMetadata?.description ?? 'N/A'}`}>Desc</div>
                <div class="text-xs w-full text-center truncate border border-gray-300 dark:border-gray-600 px-1 py-0.5 bg-gray-50 dark:bg-gray-700/30 rounded-sm" title={`Summary: ${currentFileMetadata?.summary ?? 'N/A'}`}>Summ</div>
            </div>
        {/if}
    </div>
    
</div>

<AddFieldModal bind:showModal={showAddFieldModal} on:confirm={handleAddCustomFieldConfirm} on:close={() => showAddFieldModal = false} />