<!-- src/lib/components/projectview/data/shared_panels/InfoPanel.svelte -->
<script>
    import { onMount, onDestroy, getContext } from 'svelte';
    import { get } from 'svelte/store';
    import { project } from '$lib/stores/projectStore.js';
    import { invoke } from '@tauri-apps/api/core';
    import { basename, extname as getFileExtname, sep as getPathSep, resolve } from '@tauri-apps/api/path';
    import { confirm, message } from '@tauri-apps/plugin-dialog';
    import { renameProjectItem } from '$lib/services/projectService.js';
    import AddFieldModal from '$lib/components/projectview/modals/AddFieldModal.svelte';
    import CreateGroupModal from '$lib/components/projectview/modals/CreateGroupModal.svelte';
    import MultiSelect from '$lib/components/projectview/shared/MultiSelect.svelte';
    import { deleteDefinition, customFieldDefinitions as customFieldDefinitionsStore, loadAllDefinitions } from '$lib/stores/customFieldStore.js';
    import panelStateStore from '$lib/stores/panelStateStore.js';
    import { SquarePen, XSquare, Trash2, PlusCircle } from 'lucide-svelte';
    import { Input, Label, Textarea, Button } from 'flowbite-svelte';

    async function handleDeleteCustomField(fieldKey) {
        const confirmed = await confirm(
            'Are you sure you want to delete this custom field definition? This will remove it from the project and cannot be reversed. Any data stored in this field for assets will also be effectively orphaned.',
            { title: 'Confirm Deletion', type: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' }
        );
        if (confirmed) {
            try {
                await deleteDefinition(fieldKey);
                await message('Custom field definition deleted successfully.', { title: 'Success' });
            } catch (error) {
                await message(`Failed to delete custom field: ${error.message || error}`, { title: 'Error', type: 'error' });
            }
        }
    }

    $: {
        if (refreshKey && currentOriginalAssetDetails?.originalRelativePath) {
            console.log(`[InfoPanel] Refresh triggered by key: ${refreshKey}`);
            loadMetadata(currentOriginalAssetDetails.originalRelativePath);
        }
    }

    async function getOriginalAssetDetails(selectedPath, projectStoreState) {
        if (!selectedPath || !projectStoreState || !projectStoreState.baseDirectory) {
            const fallbackName = selectedPath ? await basename(selectedPath) : 'Unknown.file';
            return {
                originalRelativePath: selectedPath, // Fallback to selectedPath if no baseDir
                originalAbsolutePath: selectedPath,
                originalType: fallbackName.includes('.') ? fallbackName.substring(fallbackName.lastIndexOf('.') + 1) : 'unknown',
                originalFileName: fallbackName,
                isView: false
            };
        }

        const pathSep = projectStoreState.baseDirectory.includes('/') ? '/' : '\\';
        let originalRelativePath = selectedPath.startsWith(projectStoreState.baseDirectory + pathSep)
            ? selectedPath.substring(projectStoreState.baseDirectory.length + 1)
            : (selectedPath.startsWith(projectStoreState.baseDirectory) ? selectedPath.substring(projectStoreState.baseDirectory.length) : selectedPath);
        originalRelativePath = originalRelativePath.replace(/\\/g, '/');
        if (originalRelativePath.startsWith('/')) {
            originalRelativePath = originalRelativePath.substring(1);
        }


        let originalAbsolutePath = selectedPath;
        let originalFileName = await basename(selectedPath);
        let originalType = await getFileExtname(originalFileName).then(ext => ext ? ext.substring(1).toLowerCase() : 'unknown'); // remove dot
        let isView = false;

        const selectedFileExt = await getFileExtname(selectedPath).then(ext => ext ? ext.substring(1).toLowerCase() : '');


        if (selectedFileExt === 'json') {
            const selectedFileNameStem = originalFileName.substring(0, originalFileName.lastIndexOf('.'));
            const potentialOriginalExtensions = ['docx', 'pdf', 'txt', 'md'];

            if (projectStoreState.documentFiles && Array.isArray(projectStoreState.documentFiles)) {
                for (const docExt of potentialOriginalExtensions) {
                    const potentialOriginalFileName = `${selectedFileNameStem}.${docExt}`;
                    for (const docFile of projectStoreState.documentFiles) {
                        if (docFile.name === potentialOriginalFileName) {
                            const currentJsonStem = selectedFileNameStem;
                            const originalDocStem = docFile.name.substring(0, docFile.name.lastIndexOf('.'));

                            if (currentJsonStem === originalDocStem) {
                                originalRelativePath = docFile.relativePath.replace(/\\/g, '/');
                                originalAbsolutePath = await resolve(projectStoreState.baseDirectory, docFile.relativePath);
                                originalFileName = docFile.name;
                                originalType = await getFileExtname(originalFileName).then(ext => ext ? ext.substring(1).toLowerCase() : 'unknown');
                                isView = true;
                                break;
                            }
                        }
                    }
                    if (isView) break;
                }
            }
        }
        return { originalRelativePath, originalAbsolutePath, originalType, originalFileName, isView };
    }

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
        if (bps >= 1000000) return (bps / 1000000).toFixed(2) + ' Mbps';
        if (bps >= 1000) return (bps / 1000).toFixed(0) + ' kbps';
        return bps + ' bps';
    }

    let currentFileMetadata = null;
    let currentOriginalAssetDetails = null; // Store the full details object
    let editableMetadata = { file_name: '', title: '', description: '', summary: '', customFields: [] };
    let showAddFieldModal = false;
    let displayableCustomFields = [];
    let fileAssignedGroups = [];
    let allProjectGroupsForPanel = [];
    let isLoadingFileGroups = false;
    let isCreateGroupModalOpen = false;
    let createGroupModalFileToAssign = null;
    let currentAssetRelativePathForGroups = null; // This is originalRelativePath
    let previousProcessedItemPath = null; // This will store the *originalRelativePath* of the previously processed item

    async function fetchAllProjectGroups() {
        const currentProjectId = get(project).id;
        if (currentProjectId) {
            try {
                allProjectGroupsForPanel = await invoke('get_project_groups', { projectId: currentProjectId });
            } catch (error) {
                console.error("Failed to fetch all project groups for panel:", error);
                allProjectGroupsForPanel = [];
            }
        }
    }

    async function fetchFileAssignedGroups(projectId, assetRelativePath) {
        if (!projectId || !assetRelativePath) {
            fileAssignedGroups = [];
            return;
        }
        isLoadingFileGroups = true;
        try {
            fileAssignedGroups = await invoke('get_groups_for_file_asset', { projectId, fileAssetRelativePath: assetRelativePath });
        } catch (error) {
            console.error(`Failed to fetch groups for asset ${assetRelativePath}:`, error);
            fileAssignedGroups = [];
        } finally {
            isLoadingFileGroups = false;
        }
    }

    onMount(async () => {
        try {
            await loadAllDefinitions();
            await fetchAllProjectGroups();
        } catch (error) {
            message(`Error loading initial data for InfoPanel: ${error.message || error}`, { title: 'Error', type: 'error' });
        }
    });

    async function loadMetadata(assetRelativePathToLoad) {
        previousProcessedItemPath = assetRelativePathToLoad; // Update after successful load
        currentFileMetadata = null; // Clear existing before load

        const projectStoreState = get(project);
        if (!projectStoreState.id || !assetRelativePathToLoad) {
            previousProcessedItemPath = assetRelativePathToLoad; // Update to prevent re-loop if path is briefly invalid
            return;
        }

        try {
            const result = await invoke('get_asset_metadata_command', {
                projectId: projectStoreState.id,
                assetRelativePath: assetRelativePathToLoad
            });

            const baseName = currentOriginalAssetDetails?.originalFileName || await basename(assetRelativePathToLoad);
            const absPath = currentOriginalAssetDetails?.originalAbsolutePath || (projectStoreState.baseDirectory ? `${projectStoreState.baseDirectory}${getPathSep()}${assetRelativePathToLoad}` : assetRelativePathToLoad);

            if (result) {
                currentFileMetadata = {
                    file_name: baseName,
                    file_path: assetRelativePathToLoad,
                    db_absolute_file_path: absPath, // Use derived absolute path
                    last_modified: result.last_modified,
                    title: result.title || '',
                    description: result.description || '',
                    summary: result.summary || '',
                    duration_seconds: result.duration_seconds,
                    width: result.width, height: result.height, frame_rate: result.frame_rate,
                    bit_rate: result.bit_rate, audio_codec: result.audio_codec, video_codec: result.video_codec,
                    creation_time: result.creation_time,
                    file_type: result.file_type || '', // Load file_type
                    customFields: result.custom_fields_json ? (() => {
                        try {
                            const parsed = JSON.parse(result.custom_fields_json);
                            return Array.isArray(parsed) ? parsed : [];
                        } catch (e) {
                            console.error("Error parsing custom_fields_json:", e);
                            return [];
                        }
                    })() : [],
                };
            } else {
                // console.log(`[InfoPanel loadMetadata] No metadata for ${assetRelativePathToLoad}. Setting defaults.`);
                currentFileMetadata = {
                    file_name: baseName, file_path: assetRelativePathToLoad, db_absolute_file_path: absPath,
                    last_modified: new Date().toISOString(), title: '', description: '', summary: '', customFields: [],
                    duration_seconds: null, width: null, height: null, frame_rate: null, bit_rate: null, audio_codec: null, video_codec: null, creation_time: null,
                    file_type: ''
                };
            }
        } catch (error) {
            console.error(`[InfoPanel loadMetadata] Error for ${assetRelativePathToLoad}:`, error);
            const baseNameOnError = currentOriginalAssetDetails?.originalFileName || await basename(assetRelativePathToLoad || 'Unknown.file').catch(() => 'Unknown.file');
            const absPathOnError = currentOriginalAssetDetails?.originalAbsolutePath || (projectStoreState.baseDirectory && assetRelativePathToLoad ? `${projectStoreState.baseDirectory}${getPathSep()}${assetRelativePathToLoad}` : assetRelativePathToLoad || '');
            currentFileMetadata = { // Set default structure on error
                file_name: baseNameOnError, file_path: assetRelativePathToLoad, db_absolute_file_path: absPathOnError,
                last_modified: new Date().toISOString(), title: '', description: '', summary: '', customFields: [],
                duration_seconds: null, width: null, height: null, frame_rate: null, bit_rate: null, audio_codec: null, video_codec: null, creation_time: null,
                file_type: ''
            };
        }
        previousProcessedItemPath = assetRelativePathToLoad; // Crucial: update after attempt
    }

    let saveTimeout;
    function debouncedSaveMetadata() {
        clearTimeout(saveTimeout);
        saveTimeout = setTimeout(() => {
            saveTextMetadata();
        }, 1000);
    }

    async function handleFileNameBlur() {
        await saveFileName();
    }

    async function handleFileNameKeydown(e) {
        if (e.key === 'Enter') {
            e.preventDefault();
            e.target.blur(); // Triggers blur which saves
        }
    }

    async function saveFileName() {
        if (!currentFileMetadata || !currentFileMetadata.file_path || !itemType) return;

        const currentFullFileName = currentFileMetadata.file_name;
        const currentFileExtension = await getFileExtname(currentFullFileName).then(ext => ext ? ext.substring(1).toLowerCase() : '');
        const currentFileNameWithoutExtension = currentFileExtension ? currentFullFileName.substring(0, currentFullFileName.lastIndexOf('.')) : currentFullFileName;
        const editedFileNameStem = editableMetadata.file_name.trim();

        if (editedFileNameStem !== currentFileNameWithoutExtension) {
            if (!editedFileNameStem) {
                await message('File name (stem) cannot be empty.', { title: 'Invalid File Name', type: 'error' });
                editableMetadata.file_name = currentFileNameWithoutExtension; // revert
                return;
            }
            let nameToSend;
            if (itemType === 'media_data' || itemType === 'audio' || itemType === 'video') {
                nameToSend = editedFileNameStem;
            } else if (itemType === 'imported_transcript') {
                nameToSend = editedFileNameStem.endsWith('.json') ? editedFileNameStem : `${editedFileNameStem}.json`;
            } else {
                nameToSend = currentFileExtension ? `${editedFileNameStem}.${currentFileExtension}` : editedFileNameStem;
            }

            let effectiveItemTypeForRename = (itemType === 'media_data' || itemType === 'audio' || itemType === 'video') ? 'media' : (itemType === 'imported_transcript' ? 'imported_transcript' : itemType);

            try {
                await renameProjectItem(currentFileMetadata.db_absolute_file_path, nameToSend, effectiveItemTypeForRename);
                // Rely on itemPath prop change from ProjectView/NotesView to trigger full reload if name change affects path
            } catch (err) {
                await message(`Error renaming item: ${err.message || err}`, { title: 'Rename Failed', type: 'error' });
            }
        }
    }

    async function saveTextMetadata() {
        let renameProcessed = false;
        if (!currentFileMetadata || !currentFileMetadata.file_path || !itemType) {
            await message('Cannot save: Missing file path or item type.', { title: 'Save Error', type: 'error' });
            return;
        }

        try {
            const assetRelativePath = currentFileMetadata.file_path;
            const projectStoreState = get(project);
            const metadataPayloadForDb = {
                file_name: currentFileMetadata.file_name,
                file_path: currentFileMetadata.db_absolute_file_path,
                last_modified: new Date().toISOString(),
                title: editableMetadata.title.trim(), description: editableMetadata.description.trim(), summary: editableMetadata.summary.trim(),
                duration_seconds: currentFileMetadata.duration_seconds, width: currentFileMetadata.width, height: currentFileMetadata.height,
                frame_rate: currentFileMetadata.frame_rate, bit_rate: currentFileMetadata.bit_rate,
                audio_codec: currentFileMetadata.audio_codec, video_codec: currentFileMetadata.video_codec,
                creation_time: currentFileMetadata.creation_time,
            };
            const customFieldsToSaveForDb = editableMetadata.customFields || [];
            try {
                await invoke('update_asset_metadata_command', {
                    projectXmlPathStr: projectStoreState.xmlPath,
                    assetRelativePath: assetRelativePath,
                    metadataPayload: metadataPayloadForDb,
                    customFieldsPayload: customFieldsToSaveForDb,
                    assetType: itemType
                });
                // Update local state optimistically
                currentFileMetadata.title = metadataPayloadForDb.title;
                currentFileMetadata.description = metadataPayloadForDb.description;
                currentFileMetadata.summary = metadataPayloadForDb.summary;
                currentFileMetadata.last_modified = metadataPayloadForDb.last_modified;
                currentFileMetadata.customFields = JSON.parse(JSON.stringify(customFieldsToSaveForDb));

                project.update(p => ({...p, isDocumentMetadataDirty: false, isMediaNoteMetadataDirty: false}));
            } catch (err) {
                console.error("Error saving metadata:", err);
                await message(`Error saving metadata: ${err}.`, { title: 'Save Failed', type: 'error' });
            }
        } catch (err) {
            console.error("Unexpected error in saveTextMetadata:", err);
            await message(`An unexpected error occurred: ${err.message || err}.`, { title: 'Error', type: 'error' });
        }
    }

    async function handleGroupsUpdate(event) {
        const newGroupNames = event.detail.options;
        const oldGroupNames = fileAssignedGroups.map(g => g.name);

        const addedGroupNames = newGroupNames.filter(name => !oldGroupNames.includes(name));
        const removedGroupNames = oldGroupNames.filter(name => !newGroupNames.includes(name));

        const projectId = get(project).id;

        for (const name of addedGroupNames) {
            const group = allProjectGroupsForPanel.find(g => g.name === name);
            if (group) {
                try {
                    await invoke('add_file_to_existing_group', {
                        projectId: projectId,
                        groupId: group.id,
                        fileAssetRelativePath: currentAssetRelativePathForGroups
                    });
                } catch (err) {
                    console.error(`Error adding file to group '${name}':`, err);
                    message(`Failed to add group '${name}': ${err}`, { title: 'Group Error', type: 'error' });
                }
            }
        }

        for (const name of removedGroupNames) {
            const group = fileAssignedGroups.find(g => g.name === name);
            if (group) {
                try {
                    await invoke('remove_file_from_group', {
                        projectId: projectId,
                        groupId: group.id,
                        fileAssetRelativePath: currentAssetRelativePathForGroups
                    });
                } catch (err) {
                    console.error(`Error removing file from group '${name}':`, err);
                    message(`Failed to remove group '${name}': ${err}`, { title: 'Group Error', type: 'error' });
                }
            }
        }

        await fetchFileAssignedGroups(projectId, currentAssetRelativePathForGroups);
    }

    function handleCreateGroup() {
        createGroupModalFileToAssign = currentAssetRelativePathForGroups;
        isCreateGroupModalOpen = true;
    }

    export let itemPath = null;
    export let itemType = null;
    export let refreshKey = null;

    $: {
        if ($project.fileRenamed && $project.fileRenamed.newPath) {
            // This is a temporary solution to force a reload of the metadata.
            // A better solution would be to have a more robust event system.
            if (itemPath === $project.fileRenamed.oldPath) {
                itemPath = $project.fileRenamed.newPath;
            }
        }
    }

    $: {
        (async () => {
            const currentProjectStoreState = get(project);
            if (itemPath && itemType && currentProjectStoreState?.baseDirectory) {
                const newOriginalDetails = await getOriginalAssetDetails(itemPath, currentProjectStoreState);
                const newDerivedRelativePath = newOriginalDetails?.originalRelativePath;

                if (newDerivedRelativePath && newDerivedRelativePath !== previousProcessedItemPath) {
                    currentOriginalAssetDetails = newOriginalDetails; // Store details
                    currentAssetRelativePathForGroups = newDerivedRelativePath;

                    await loadMetadata(newDerivedRelativePath); // This updates previousSelectedItemPath

                    if (currentProjectStoreState.id && currentAssetRelativePathForGroups) {
                        await fetchFileAssignedGroups(currentProjectStoreState.id, currentAssetRelativePathForGroups);
                    }
                } else if (!newDerivedRelativePath && itemPath) {
                    // Valid itemPath from prop, but couldn't derive relative path from it (e.g., item not in project files yet)
                    currentFileMetadata = null; // Clear metadata to show "no data" or placeholder
                    // Don't clear previousSelectedItemPath here, as itemPath might be a temporary invalid value during a transition
                } else if (!itemPath) {
                    // itemPath prop is null, means no item is selected or active
                    currentFileMetadata = null;
                    currentOriginalAssetDetails = null;
                    currentAssetRelativePathForGroups = null;
                    fileAssignedGroups = [];
                    previousProcessedItemPath = null; // Explicitly clear if itemPath prop becomes null
                }
                // If newDerivedRelativePath === previousProcessedItemPath, we do nothing to prevent reloads for the same item.
            } else {
                // itemPath, itemType, or project details missing. Clear everything.
                currentFileMetadata = null;
                currentOriginalAssetDetails = null;
                currentAssetRelativePathForGroups = null;
                fileAssignedGroups = [];
                if (itemPath === null) { // Only clear previous if itemPath is definitively null
                    previousProcessedItemPath = null;
                }
            }
        })();
    }

    $: {
        if (currentFileMetadata && $customFieldDefinitionsStore) {
            const assetCustomValues = currentFileMetadata.customFields || [];
            let newEditableCustomFields = [];
            for (const def of $customFieldDefinitionsStore) {
                let isApplicable = false;
                if (typeof def.scope === 'string') {
                    if (def.scope.toLowerCase() === 'project') isApplicable = true;
                } else if (def.scope && typeof def.scope === 'object' && typeof def.scope.AssetType === 'string') {
                    const assetTypeScopeValue = def.scope.AssetType.toLowerCase();
                    if (assetTypeScopeValue === itemType) isApplicable = true;
                    else if (assetTypeScopeValue === 'media' && (itemType === 'audio' || itemType === 'video' || itemType === 'media_data')) isApplicable = true;
                }
                if (isApplicable) {
                    const existingAssetField = assetCustomValues.find(cf => cf.key === def.field_key);
                    const valueToUse = existingAssetField?.value ?? def.default_value ?? '';
                    newEditableCustomFields.push({ key: def.field_key, name: def.field_name, type: def.field_type, value: valueToUse });
                }
            }
            newEditableCustomFields.sort((a, b) => a.name.localeCompare(b.name));
            editableMetadata.customFields = newEditableCustomFields;
        } else {
            editableMetadata.customFields = [];
        }

        if (currentFileMetadata) {
            if (currentFileMetadata.file_name) {
                const ext = currentFileMetadata.file_name.includes('.') ? currentFileMetadata.file_name.substring(currentFileMetadata.file_name.lastIndexOf('.') + 1) : '';
                const nameWithoutExt = ext ? currentFileMetadata.file_name.substring(0, currentFileMetadata.file_name.length - (ext.length + (ext ? 1:0)) ) : currentFileMetadata.file_name;
                editableMetadata.file_name = nameWithoutExt;
            } else {
                editableMetadata.file_name = '';
            }
            editableMetadata.title = currentFileMetadata.title || '';
            editableMetadata.description = currentFileMetadata.description || '';
            editableMetadata.summary = currentFileMetadata.summary || '';
        } else {
            editableMetadata.file_name = '';
            editableMetadata.title = '';
            editableMetadata.description = '';
            editableMetadata.summary = '';
        }
    }

    // Always clear dirty flags since we don't have manual saves anymore
    $: {
        const currentProjectState = get(project);
        if (itemType === 'doc' || itemType === 'table' || itemType === 'image' || itemType === 'imported_transcript') {
            if (currentProjectState.isDocumentMetadataDirty) project.update(p => ({ ...p, isDocumentMetadataDirty: false }));
        } else if (itemType === 'media_data' || itemType === 'audio' || itemType === 'video') {
            if (currentProjectState.isMediaNoteMetadataDirty) project.update(p => ({ ...p, isMediaNoteMetadataDirty: false }));
        }
    }
</script>

<div class="h-full bg-white dark:bg-gray-900 flex flex-col overflow-hidden">
    <div class="text-sm font-semibold border-b px-1 h-9 border-gray-300 dark:border-gray-800 text-gray-700 dark:text-gray-300 flex-shrink-0 flex items-center justify-between mb-2">
        <div class="flex items-center space-x-2">
            <span class="ml-1">Metadata</span>
        </div>
    </div>

    <div class="flex-grow overflow-y-auto overflow-x-hidden min-h-0 text-xs relative px-2">
        {#if currentFileMetadata}
            <div class="space-y-3 mt-1 pb-4">
                <div>
                    <Label for="fileNameInput" class="mb-1 text-xs text-gray-500 font-semibold uppercase tracking-wide">File Name</Label>
                    <Input type="text" id="fileNameInput" bind:value={editableMetadata.file_name} class="text-xs !py-1.5" size="sm" placeholder="Enter name without extension" autocorrect="off" autocomplete="off" on:blur={handleFileNameBlur} on:keydown={handleFileNameKeydown} />
                    {#if currentFileMetadata.file_name && currentFileMetadata.file_name.includes('.')}
                        <p class="mt-1 text-[10px] text-gray-500 dark:text-gray-400">
                            Extension: {currentFileMetadata.file_name.substring(currentFileMetadata.file_name.lastIndexOf('.'))}
                        </p>
                    {/if}
                </div>

                <div>
                    <Label for="filePathAbsolute" class="mb-1 text-xs text-gray-500 font-semibold uppercase tracking-wide">File Path</Label>
                    <div id="filePathAbsolute" class="text-xs text-gray-900 dark:text-gray-100 break-all px-2 py-1.5 bg-gray-50 dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-md min-h-[30px]">{currentFileMetadata.db_absolute_file_path || ''}</div>
                </div>

                <div>
                    <Label for="fileTypeDisplay" class="mb-1 text-xs text-gray-500 font-semibold uppercase tracking-wide">File Type</Label>
                    <div id="fileTypeDisplay" class="text-xs text-gray-900 dark:text-gray-100 break-words px-2 py-1.5 bg-gray-50 dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-md min-h-[30px]">{currentFileMetadata.file_type || 'N/A'}</div>
                </div>

                {#if currentFileMetadata.creation_time}
                <div>
                    <Label for="createdAt" class="mb-1 text-xs text-gray-500 font-semibold uppercase tracking-wide">Created At</Label>
                    <div id="createdAt" class="text-xs text-gray-900 dark:text-gray-100 px-2 py-1.5 bg-gray-50 dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-md min-h-[30px]">{new Date(currentFileMetadata.creation_time).toLocaleString()}</div>
                </div>
                {/if}

                {#if currentFileMetadata.last_modified}
                <div>
                    <Label for="lastModified" class="mb-1 text-xs text-gray-500 font-semibold uppercase tracking-wide">Last Modified</Label>
                    <div id="lastModified" class="text-xs text-gray-900 dark:text-gray-100 px-2 py-1.5 bg-gray-50 dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-md min-h-[30px]">{new Date(currentFileMetadata.last_modified).toLocaleString()}</div>
                </div>
                {/if}

                <div>
                    <Label for="titleInput" class="mb-1 text-xs text-gray-500 font-semibold uppercase tracking-wide">Title</Label>
                    <Input type="text" id="titleInput" bind:value={editableMetadata.title} on:input={debouncedSaveMetadata} class="text-xs !py-1.5" size="sm" autocorrect="off" autocomplete="off" />
                </div>

                <div>
                    <Label for="descriptionInput" class="mb-1 text-xs text-gray-500 font-semibold uppercase tracking-wide">Description</Label>
                    <Textarea id="descriptionInput" bind:value={editableMetadata.description} on:input={debouncedSaveMetadata} rows="3" class="text-xs !py-1.5" autocorrect="off" autocomplete="off" />
                </div>

                <div>
                    <Label for="summaryInput" class="mb-1 text-xs text-gray-500 font-semibold uppercase tracking-wide">Summary</Label>
                    <Textarea id="summaryInput" bind:value={editableMetadata.summary} on:input={debouncedSaveMetadata} rows="2" class="text-xs !py-1.5" autocorrect="off" autocomplete="off" />
                </div>

                <!-- Attachments Section -->
                {#if (itemType === 'doc' || itemType === 'imported_transcript') && currentFileMetadata?.customFields && Array.isArray(currentFileMetadata.customFields)}
                    {@const attachmentsField = currentFileMetadata.customFields.find(f => f.key === 'attachments')}
                    {#if attachmentsField && attachmentsField.value}
                        {@const attachments = JSON.parse(attachmentsField.value)}
                        {#if Array.isArray(attachments) && attachments.length > 0}
                            <hr class="my-4 border-gray-300 dark:border-gray-700">
                            <div class="mb-3">
                                <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 tracking-wider mb-2">Attachments</h3>
                                <ul class="list-disc list-inside space-y-1 pl-1">
                                    {#each attachments as attachment, i (attachment)}
                                        <li class="text-gray-900 dark:text-gray-100">
                                            <button
                                                class="text-blue-600 dark:text-blue-400 hover:underline text-left"
                                                on:click={() => {
                                                    panelStateStore.setActiveInfoPanelTab('attachments');
                                                    panelStateStore.toggleInfoPanel(false);
                                                }}
                                                title="Open in attachments panel"
                                            >
                                                {attachment.split(/[/\\]/).pop() || attachment}
                                            </button>
                                        </li>
                                    {/each}
                                </ul>
                            </div>
                        {/if}
                    {/if}
                {/if}

                {#if (currentFileMetadata.duration_seconds || currentFileMetadata.width || currentFileMetadata.video_codec || currentFileMetadata.audio_codec || currentFileMetadata.bit_rate) && !(Array.isArray(currentFileMetadata.customFields) && currentFileMetadata.customFields.some(cf => cf.key === '_isScreenshot' && cf.value === true))}
                    <hr class="my-4 border-gray-300 dark:border-gray-700">
                    <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 tracking-wider mb-2">Technical Details</h3>
                    {#if currentFileMetadata.duration_seconds}
                        <div class="mb-3">
                            <label for="durationInput" class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Duration:</label>
                        <span id="durationInput" class="text-gray-900 dark:text-gray-100 block w-full border border-gray-300 dark:border-gray-800 px-1.5 py-1 bg-gray-50 dark:bg-gray-800 min-h-[30px] break-words">{formatDuration(currentFileMetadata.duration_seconds)}</span>
                        </div>
                    {/if}
                    {#if currentFileMetadata.width && currentFileMetadata.height}
                        <div class="mb-3">
                            <label for="dimensionsInput" class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Dimensions:</label>
                            <span id="dimensionsInput" class="text-gray-900 dark:text-gray-100 block w-full border border-gray-300 dark:border-gray-800 px-1.5 py-1 bg-gray-50 dark:bg-gray-800 min-h-[30px] break-words">{currentFileMetadata.width} x {currentFileMetadata.height}</span>
                        </div>
                    {/if}
                    {#if currentFileMetadata.frame_rate}
                        <div class="mb-3">
                            <label for="frameRateInput" class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Frame Rate:</label>
                            <span id="frameRateInput" class="text-gray-900 dark:text-gray-100 block w-full border border-gray-300 dark:border-gray-800 px-1.5 py-1 bg-gray-50 dark:bg-gray-800 min-h-[30px] break-words">{currentFileMetadata.frame_rate.toFixed(2)} fps</span>
                        </div>
                    {/if}
                    {#if currentFileMetadata.bit_rate}
                        <div class="mb-3">
                            <label for="bitRateInput" class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Bit Rate:</label>
                            <span id="bitRateInput" class="text-gray-900 dark:text-gray-100 block w-full border border-gray-300 dark:border-gray-800 px-1.5 py-1 bg-gray-50 dark:bg-gray-800 min-h-[30px] break-words">{formatBitrate(currentFileMetadata.bit_rate)}</span>
                        </div>
                    {/if}
                    {#if currentFileMetadata.video_codec}
                        <div class="mb-3">
                            <label for="videoCodecInput" class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Video Codec:</label>
                            <span id="videoCodecInput" class="text-gray-900 dark:text-gray-100 block w-full border border-gray-300 dark:border-gray-800 px-1.5 py-1 bg-gray-50 dark:bg-gray-800 min-h-[30px] break-words">{currentFileMetadata.video_codec || ''}</span>
                        </div>
                    {/if}
                    {#if currentFileMetadata.audio_codec}
                        <div class="mb-3">
                            <label for="audioCodecInput" class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Audio Codec:</label>
                            <span id="audioCodecInput" class="text-gray-900 dark:text-gray-100 block w-full border border-gray-300 dark:border-gray-800 px-1.5 py-1 bg-gray-50 dark:bg-gray-800 min-h-[30px] break-words">{currentFileMetadata.audio_codec || ''}</span>
                        </div>
                    {/if}
                {/if}

                {#if currentAssetRelativePathForGroups && get(project).id}
                    <hr class="my-4 border-gray-300 dark:border-gray-700">
                    <div class="mb-2">
                        <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 tracking-wider mb-1">Groups</h3>
                        {#if isLoadingFileGroups}
                            <p class="text-xs text-gray-400 dark:text-gray-500 italic">Loading groups...</p>
                        {:else}
                            <MultiSelect
                                itemType="group"
                                allOptions={allProjectGroupsForPanel.map(g => g.name)}
                                assignedOptions={fileAssignedGroups.map(g => g.name)}
                                isEditable={true}
                                placeholder="No groups assigned."
                                on:update={handleGroupsUpdate}
                                on:create={handleCreateGroup}
                            />
                        {/if}
                    </div>
                {/if}

                {#if $customFieldDefinitionsStore}
                    <hr class="my-4 border-gray-300 dark:border-gray-700">
                    <div class="flex justify-between items-center mb-2">
                        <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 tracking-wider">Custom Fields</h3>
                        <button
                            on:click={() => showAddFieldModal = true}
                            class="p-1 text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200 focus:outline-none focus:ring-2 focus:ring-blue-500 flex items-center justify-center"
                            title="Add Custom Field Definition"
                            aria-label="Add Custom Field Definition"
                        >
                            <PlusCircle class="w-4 h-4" />
                        </button>
                    </div>

                    <div class="space-y-3">
                        {#each editableMetadata.customFields as field, index (field.key + '-' + index)}
                            <div>
                                <div class="flex justify-between items-center mb-1">
                                    <Label for={`custom-field-edit-${field.key}`} class="text-xs text-gray-500 font-semibold uppercase tracking-wide">{field.name || field.key}</Label>
                                    <button
                                        on:click={() => handleDeleteCustomField(field.key)}
                                        title={`Delete '${field.name || field.key}' definition`}
                                        class="p-0.5 text-red-500 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300 focus:outline-none focus:ring-1 focus:ring-red-500 flex items-center justify-center rounded"
                                    >
                                        <Trash2 class="w-3 h-3" />
                                    </button>
                                </div>
                                {#if field.type === 'small_text'}
                                    <Input
                                        type="text"
                                        id={`custom-field-edit-${field.key}`}
                                        bind:value={editableMetadata.customFields[index].value}
                                        on:input={debouncedSaveMetadata}
                                        class="text-xs !py-1.5"
                                        size="sm"
                                        placeholder={`Enter value for ${field.name || field.key}`}
                                        autocorrect="off" autocomplete="off" />
                                {:else if field.type === 'long_text'}
                                    <Textarea
                                        id={`custom-field-edit-${field.key}`}
                                        rows="3"
                                        class="text-xs !py-1.5"
                                        bind:value={editableMetadata.customFields[index].value}
                                        on:input={debouncedSaveMetadata}
                                        placeholder={`Enter value for ${field.name || field.key}`}
                                        autocorrect="off" autocomplete="off" />
                                {/if}
                            </div>
                        {/each}
                    </div>
                     {#if editableMetadata.customFields.length === 0 && $customFieldDefinitionsStore && $customFieldDefinitionsStore.filter(def => { const scope = def.scope; let applicable = false; if (typeof scope === 'string') { if (scope.toLowerCase() === 'project') applicable = true; } else if (scope && typeof scope === 'object' && typeof scope.AssetType === 'string') { const assetTypeScope = scope.AssetType.toLowerCase(); if (assetTypeScope === itemType) applicable = true; else if (assetTypeScope === 'media' && (itemType === 'audio' || itemType === 'video' || itemType === 'media_note')) applicable = true; } return applicable; }).length > 0}
                        <p class="text-xs text-gray-500 dark:text-gray-400 italic">No custom fields have values for this item. Add a definition above.</p>
                    {/if}
                {/if}
            </div>
        {:else}
            <p class="text-gray-500 dark:text-gray-400 italic px-1 py-2">
                Select an item to view its metadata.
            </p>
        {/if}
    </div>
</div>

<AddFieldModal bind:showModal={showAddFieldModal} currentItemType={itemType} on:close={() => showAddFieldModal = false} />

{#if isCreateGroupModalOpen && get(project).id}
    <CreateGroupModal
        bind:showModal={isCreateGroupModalOpen}
        projectUuid={get(project).id}
        fileToAdd={createGroupModalFileToAssign ? { relativePath: createGroupModalFileToAssign, name: '' } : null}
        on:groupCreated={async (event) => {
            isCreateGroupModalOpen = false;
            await fetchAllProjectGroups();
            if (event.detail.group && createGroupModalFileToAssign) {
                 await fetchFileAssignedGroups(get(project).id, createGroupModalFileToAssign);
            } else if (event.detail.group && !createGroupModalFileToAssign) {
                 if (currentAssetRelativePathForGroups) {
                    await fetchFileAssignedGroups(get(project).id, currentAssetRelativePathForGroups);
                 }
            }
            createGroupModalFileToAssign = null;
        }}
        on:groupCreatedAndFileAdded={async (event) => {
            isCreateGroupModalOpen = false;
            await fetchAllProjectGroups();
            if (event.detail.file && event.detail.file.relativePath === currentAssetRelativePathForGroups) {
                await fetchFileAssignedGroups(get(project).id, currentAssetRelativePathForGroups);
            }
             createGroupModalFileToAssign = null;
        }}
        on:close={() => {
            isCreateGroupModalOpen = false;
            createGroupModalFileToAssign = null;
        }}
    />
{/if}
