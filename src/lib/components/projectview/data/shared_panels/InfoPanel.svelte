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
    import GroupMultiSelect from '$lib/components/projectview/shared/GroupMultiSelect.svelte';
    import { deleteDefinition, customFieldDefinitions as customFieldDefinitionsStore, loadAllDefinitions } from '$lib/stores/customFieldStore.js';

    const EDIT_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-pencil-square" viewBox="0 0 16 16"><path d="M15.502 1.94a.5.5 0 0 1 0 .706L14.459 3.69l-2-2L13.502.646a.5.5 0 0 1 .707 0l1.293 1.293zm-1.75 2.456-2-2L4.939 9.21a.5.5 0 0 0-.121.196l-.805 2.414a.25.25 0 0 0 .316.316l2.414-.805a.5.5 0 0 0 .196-.12l6.813-6.814z"/><path fill-rule="evenodd" d="M1 13.5A1.5 1.5 0 0 0 2.5 15h11a1.5 1.5 0 0 0 1.5-1.5v-6a.5.5 0 0 0-1 0v6a.5.5 0 0 1-.5.5h-11a.5.5 0 0 1-.5-.5v-11a.5.5 0 0 1 .5-.5H9a.5.5 0 0 0 0-1H2.5A1.5 1.5 0 0 0 1 2.5z"/></svg>`;
    const CANCEL_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-x-square" viewBox="0 0 16 16"><path d="M14 1a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H2a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1zM2 0a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V2a2 2 0 0 0-2-2z"/><path d="M4.646 4.646a.5.5 0 0 1 .708 0L8 7.293l2.646-2.647a.5.5 0 0 1 .708.708L8.707 8l2.647 2.646a.5.5 0 0 1-.708.708L8 8.707l-2.646 2.647a.5.5 0 0 1-.708-.708L7.293 8 4.646 5.354a.5.5 0 0 1 0-.708z"/></svg>`;
    const TRASH_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-trash3" viewBox="0 0 16 16"><path d="M6.5 1h3a.5.5 0 0 1 .5.5v1H6v-1a.5.5 0 0 1 .5-.5M11 2.5v-1A1.5 1.5 0 0 0 9.5 0h-3A1.5 1.5 0 0 0 5 1.5v1H2.506a.58.58 0 0 0-.01 0H1.5a.5.5 0 0 0 0 1h.538l.853 10.66A2 2 0 0 0 4.885 16h6.23a2 2 0 0 0 1.994-1.84l.853-10.66h.538a.5.5 0 0 0 0-1h-.995a.59.59 0 0 0-.01 0zm1.958 1-.846 10.58a1 1 0 0 1-.997.92h-6.23a1 1 0 0 1-.997-.92L3.042 3.5zm-7.487 1a.5.5 0 0 1 .528.47l.5 8.5a.5.5 0 0 1-.998.06L5 5.03a.5.5 0 0 1 .47-.53Zm5.058 0a.5.5 0 0 1 .47.53l-.5 8.5a.5.5 0 1 1-.998-.06l.5-8.5a.5.5 0 0 1 .528-.47ZM8 4.5a.5.5 0 0 1 .5.5v8.5a.5.5 0 0 1-1 0V5a.5.5 0 0 1 .5-.5z"/></svg>`;

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
    let isEditing = false;
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
                    customFields: result.custom_fields_json ? JSON.parse(result.custom_fields_json) : [],
                };
            } else {
                // console.log(`[InfoPanel loadMetadata] No metadata for ${assetRelativePathToLoad}. Setting defaults.`);
                currentFileMetadata = {
                    file_name: baseName, file_path: assetRelativePathToLoad, db_absolute_file_path: absPath,
                    last_modified: new Date().toISOString(), title: '', description: '', summary: '', customFields: [],
                    duration_seconds: null, width: null, height: null, frame_rate: null, bit_rate: null, audio_codec: null, video_codec: null, creation_time: null
                };
            }
        } catch (error) {
            console.error(`[InfoPanel loadMetadata] Error for ${assetRelativePathToLoad}:`, error);
            const baseNameOnError = currentOriginalAssetDetails?.originalFileName || await basename(assetRelativePathToLoad || 'Unknown.file').catch(() => 'Unknown.file');
            const absPathOnError = currentOriginalAssetDetails?.originalAbsolutePath || (projectStoreState.baseDirectory && assetRelativePathToLoad ? `${projectStoreState.baseDirectory}${getPathSep()}${assetRelativePathToLoad}` : assetRelativePathToLoad || '');
            currentFileMetadata = { // Set default structure on error
                file_name: baseNameOnError, file_path: assetRelativePathToLoad, db_absolute_file_path: absPathOnError,
                last_modified: new Date().toISOString(), title: '', description: '', summary: '', customFields: [],
                duration_seconds: null, width: null, height: null, frame_rate: null, bit_rate: null, audio_codec: null, video_codec: null, creation_time: null
            };
        }
        previousProcessedItemPath = assetRelativePathToLoad; // Crucial: update after attempt
    }

    function toggleEditMode() {
        isEditing = !isEditing;
    }

    async function handleSaveMetadata() {
        let renameProcessed = false;
        if (!currentFileMetadata || !currentFileMetadata.file_path || !itemType) {
            await message('Cannot save: Missing file path or item type.', { title: 'Save Error', type: 'error' });
            return;
        }

        try {
            const assetRelativePath = currentFileMetadata.file_path;
            const currentFullFileName = currentFileMetadata.file_name;
            const currentFileExtension = await getFileExtname(currentFullFileName).then(ext => ext ? ext.substring(1).toLowerCase() : '');
            const currentFileNameWithoutExtension = currentFileExtension ? currentFullFileName.substring(0, currentFullFileName.lastIndexOf('.')) : currentFullFileName;
            const editedFileNameStem = editableMetadata.file_name.trim();

            if (editedFileNameStem !== currentFileNameWithoutExtension) {
                if (!editedFileNameStem) {
                    await message('File name (stem) cannot be empty.', { title: 'Invalid File Name', type: 'error' });
                    isEditing = true; return;
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
                    isEditing = false; renameProcessed = true;
                    project.update(p => ({...p, isDocumentMetadataDirty: false, isMediaNoteMetadataDirty: false}));
                    // Rely on itemPath prop change from ProjectView/NotesView to trigger full reload if name change affects path
                } catch (err) {
                    await message(`Error renaming item: ${err.message || err}`, { title: 'Rename Failed', type: 'error' });
                    isEditing = true; return;
                }
            }

            if (!renameProcessed) {
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
                    // Update local state optimistically if not renamed
                    currentFileMetadata.title = metadataPayloadForDb.title;
                    currentFileMetadata.description = metadataPayloadForDb.description;
                    currentFileMetadata.summary = metadataPayloadForDb.summary;
                    currentFileMetadata.last_modified = metadataPayloadForDb.last_modified;
                    currentFileMetadata.customFields = JSON.parse(JSON.stringify(customFieldsToSaveForDb));
                    isEditing = false;
                    project.update(p => ({...p, isDocumentMetadataDirty: false, isMediaNoteMetadataDirty: false}));
                } catch (err) {
                    console.error("Error saving metadata:", err);
                    await message(`Error saving metadata: ${err}.`, { title: 'Save Failed', type: 'error' });
                }
            }
        } catch (err) {
            console.error("Unexpected error in handleSaveMetadata:", err);
            await message(`An unexpected error occurred: ${err.message || err}.`, { title: 'Error', type: 'error' });
            isEditing = true;
        }
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
                    isEditing = false;
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
                    if(isEditing) isEditing = false;
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
                if(isEditing) isEditing = false;
            }
        })();
    }

    $: {
        if (currentFileMetadata && $customFieldDefinitionsStore) {
            const assetCustomValues = currentFileMetadata.customFields || [];
            let newEditableCustomFields = [];
            let newDisplayableCustomFields = [];
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
                    if (isEditing) {
                        newEditableCustomFields.push({ key: def.field_key, name: def.field_name, type: def.field_type, value: valueToUse });
                    } else {
                        newDisplayableCustomFields.push({ key: def.field_key, name: def.field_name, type: def.field_type, value: valueToUse });
                    }
                }
            }
            newEditableCustomFields.sort((a, b) => a.name.localeCompare(b.name));
            newDisplayableCustomFields.sort((a, b) => a.name.localeCompare(b.name));
            editableMetadata.customFields = newEditableCustomFields;
            displayableCustomFields = newDisplayableCustomFields;
        } else {
            editableMetadata.customFields = [];
            displayableCustomFields = [];
        }

        if (isEditing && currentFileMetadata) {
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
        } else if (!isEditing) {
            editableMetadata.file_name = '';
            editableMetadata.title = '';
            editableMetadata.description = '';
            editableMetadata.summary = '';
        }
    }

    $: if (isEditing && currentFileMetadata) {
        const originalNameStem = currentFileMetadata.file_name.includes('.') ? currentFileMetadata.file_name.substring(0, currentFileMetadata.file_name.lastIndexOf('.')) : currentFileMetadata.file_name;
        const nameChanged = editableMetadata.file_name.trim() !== originalNameStem;
        const titleChanged = editableMetadata.title.trim() !== (currentFileMetadata.title || '');
        const descriptionChanged = editableMetadata.description.trim() !== (currentFileMetadata.description || '');
        const summaryChanged = editableMetadata.summary.trim() !== (currentFileMetadata.summary || '');
        let customFieldsChanged = false;
        if (editableMetadata.customFields.length !== (currentFileMetadata.customFields || []).length) {
            customFieldsChanged = true;
        } else {
            for (const editableField of editableMetadata.customFields) {
                const originalField = (currentFileMetadata.customFields || []).find(cf => cf.key === editableField.key);
                if (!originalField || (originalField.value || '') !== (editableField.value || '')) {
                    customFieldsChanged = true;
                    break;
                }
            }
        }
        const isDirty = nameChanged || titleChanged || descriptionChanged || summaryChanged || customFieldsChanged;
        if (itemType === 'doc' || itemType === 'table' || itemType === 'image' || itemType === 'imported_transcript') {
            project.update(p => ({ ...p, isDocumentMetadataDirty: isDirty }));
        } else if (itemType === 'media_data' || itemType === 'audio' || itemType === 'video') {
            project.update(p => ({ ...p, isMediaNoteMetadataDirty: isDirty }));
        }
    } else {
        const currentProjectState = get(project);
        if (itemType === 'doc' || itemType === 'table' || itemType === 'image' || itemType === 'imported_transcript') {
            if (currentProjectState.isDocumentMetadataDirty) project.update(p => ({ ...p, isDocumentMetadataDirty: false }));
        } else if (itemType === 'media_data' || itemType === 'audio' || itemType === 'video') {
            if (currentProjectState.isMediaNoteMetadataDirty) project.update(p => ({ ...p, isMediaNoteMetadataDirty: false }));
        }
    }
</script>

<div class="h-full bg-white dark:bg-dark-bg-secondary flex flex-col overflow-hidden">
    <div class="text-sm font-semibold border-b px-1 h-9 border-gray-300 dark:border-dark-bg-tertiary text-gray-700 dark:text-gray-300 flex-shrink-0 flex items-center justify-between mb-2">
        <div class="flex items-center space-x-2">
            <span>Metadata</span>
        </div>
        {#if currentFileMetadata}
            <button
                on:click={toggleEditMode}
                class="p-1 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
                title={isEditing ? 'Cancel Edit' : 'Edit Metadata'}
            >
                {@html isEditing ? CANCEL_ICON_SVG : EDIT_ICON_SVG}
            </button>
        {/if}
    </div>

    <div class="flex-grow overflow-y-auto overflow-x-hidden min-h-0 text-xs relative px-2">
        {#if currentFileMetadata}
            <div class="space-y-2">
                <div class="mb-3">
                    <label for="fileNameInput" class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">File Name:</label>
                    {#if isEditing}
                        <input type="text" id="fileNameInput" bind:value={editableMetadata.file_name} class="mt-0.5 block w-full border border-gray-300 dark:border-dark-bg-tertiary focus:ring-1 focus:ring-blue-500 focus:border-blue-500 dark:bg-dark-bg-secondary dark:text-white px-1.5 py-1 text-xs bg-white text-gray-900" placeholder="Enter name without extension" autocorrect="off" autocomplete="off"/>
                        {#if currentFileMetadata.file_name && currentFileMetadata.file_name.includes('.')}
                            <span class="mt-1 text-gray-500 dark:text-gray-400 text-xs block">
                                Extension: {currentFileMetadata.file_name.substring(currentFileMetadata.file_name.lastIndexOf('.'))}
                            </span>
                        {/if}
                    {:else}
                        <span id="fileNameInput" class="text-gray-900 dark:text-gray-100 block w-full border border-gray-300 dark:border-dark-bg-tertiary px-1.5 py-1 bg-gray-50 dark:bg-dark-bg-tertiary min-h-[30px] break-words">{currentFileMetadata.file_name || 'N/A'}</span>
                    {/if}
                </div>

                <div class="mb-3">
                    <label for="filePathAbsolute" class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">File Path:</label>
                    <span id="filePathAbsolute" class="text-gray-900 dark:text-gray-100 break-all block w-full border border-gray-300 dark:border-dark-bg-tertiary px-1.5 py-1 bg-gray-50 dark:bg-dark-bg-tertiary min-h-[30px]">{currentFileMetadata.db_absolute_file_path || ''}</span>
                </div>

                {#if currentFileMetadata.creation_time}
                <div class="mb-3">
                    <label for="createdAt" class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Created At:</label>
                    <span id="createdAt" class="text-gray-900 dark:text-gray-100 block w-full border border-gray-300 dark:border-dark-bg-tertiary px-1.5 py-1 bg-gray-50 dark:bg-dark-bg-tertiary min-h-[30px]">{new Date(currentFileMetadata.creation_time).toLocaleString()}</span>
                </div>
                {/if}

                {#if currentFileMetadata.last_modified}
                <div class="mb-3">
                    <label for="lastModified" class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Last Modified:</label>
                    <span id="lastModified" class="text-gray-900 dark:text-gray-100 block w-full border border-gray-300 dark:border-dark-bg-tertiary px-1.5 py-1 bg-gray-50 dark:bg-dark-bg-tertiary min-h-[30px]">{new Date(currentFileMetadata.last_modified).toLocaleString()}</span>
                </div>
                {/if}

                <div class="mb-3">
                    <label for="titleInput" class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Title:</label>
                    {#if isEditing}
                        <input type="text" id="titleInput" bind:value={editableMetadata.title} class="mt-0.5 block w-full border border-gray-300 dark:border-dark-bg-tertiary focus:ring-1 focus:ring-blue-500 focus:border-blue-500 dark:bg-dark-bg-secondary dark:text-white px-1.5 py-1 text-xs bg-white text-gray-900" autocorrect="off" autocomplete="off"/>
                    {:else}
                        <span id="titleInput" class="text-gray-900 dark:text-gray-100 block w-full border border-gray-300 dark:border-dark-bg-tertiary px-1.5 py-1 bg-gray-50 dark:bg-dark-bg-tertiary min-h-[30px] break-words">{currentFileMetadata.title || ''}</span>
                    {/if}
                </div>

                <div class="mb-3">
                    <label for="descriptionInput" class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Description:</label>
                    {#if isEditing}
                        <textarea id="descriptionInput" bind:value={editableMetadata.description} rows="3" class="mt-0.5 block w-full border border-gray-300 dark:border-dark-bg-tertiary focus:ring-1 focus:ring-blue-500 focus:border-blue-500 dark:bg-dark-bg-secondary dark:text-white px-1.5 py-1 text-xs bg-white text-gray-900" autocorrect="off" autocomplete="off"></textarea>
                    {:else}
                        <span id="descriptionInput" class="text-gray-900 dark:text-gray-100 whitespace-pre-wrap break-words break-all block w-full border border-gray-300 dark:border-dark-bg-tertiary px-1.5 py-1 bg-gray-50 dark:bg-dark-bg-tertiary min-h-[30px]">{currentFileMetadata.description || ''}</span>
                    {/if}
                </div>

                <div class="mb-3">
                    <label for="summaryInput" class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Summary:</label>
                    {#if isEditing}
                        <textarea id="summaryInput" bind:value={editableMetadata.summary} rows="2" class="mt-0.5 block w-full border border-gray-300 dark:border-dark-bg-tertiary focus:ring-1 focus:ring-blue-500 focus:border-blue-500 dark:bg-dark-bg-secondary dark:text-white px-1.5 py-1 text-xs bg-white text-gray-900" autocorrect="off" autocomplete="off"></textarea>
                    {:else}
                        <span id="summaryInput" class="text-gray-900 dark:text-gray-100 whitespace-pre-wrap break-words break-all block w-full border border-gray-300 dark:border-dark-bg-tertiary px-1.5 py-1 bg-gray-50 dark:bg-dark-bg-tertiary min-h-[30px]">{currentFileMetadata.summary || ''}</span>
                    {/if}
                </div>

                <!-- Attachments Section -->
                {#if itemType === 'doc' && currentFileMetadata?.customFields}
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
                                                on:click={() => invoke('reveal_in_file_explorer', { filePathStr: attachment })}
                                                title="Show in folder"
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

                {#if (currentFileMetadata.duration_seconds || currentFileMetadata.width || currentFileMetadata.video_codec || currentFileMetadata.audio_codec || currentFileMetadata.bit_rate) && !(currentFileMetadata.customFields && currentFileMetadata.customFields.some(cf => cf.key === '_isScreenshot' && cf.value === true))}
                    <hr class="my-4 border-gray-300 dark:border-gray-700">
                    <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 tracking-wider mb-2">Technical Details</h3>
                    {#if currentFileMetadata.duration_seconds}
                        <div class="mb-3">
                            <label for="durationInput" class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Duration:</label>
                        <span id="durationInput" class="text-gray-900 dark:text-gray-100 block w-full border border-gray-300 dark:border-dark-bg-tertiary px-1.5 py-1 bg-gray-50 dark:bg-dark-bg-tertiary min-h-[30px] break-words">{formatDuration(currentFileMetadata.duration_seconds)}</span>
                        </div>
                    {/if}
                    {#if currentFileMetadata.width && currentFileMetadata.height}
                        <div class="mb-3">
                            <label for="dimensionsInput" class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Dimensions:</label>
                            <span id="dimensionsInput" class="text-gray-900 dark:text-gray-100 block w-full border border-gray-300 dark:border-dark-bg-tertiary px-1.5 py-1 bg-gray-50 dark:bg-dark-bg-tertiary min-h-[30px] break-words">{currentFileMetadata.width} x {currentFileMetadata.height}</span>
                        </div>
                    {/if}
                    {#if currentFileMetadata.frame_rate}
                        <div class="mb-3">
                            <label for="frameRateInput" class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Frame Rate:</label>
                            <span id="frameRateInput" class="text-gray-900 dark:text-gray-100 block w-full border border-gray-300 dark:border-dark-bg-tertiary px-1.5 py-1 bg-gray-50 dark:bg-dark-bg-tertiary min-h-[30px] break-words">{currentFileMetadata.frame_rate.toFixed(2)} fps</span>
                        </div>
                    {/if}
                    {#if currentFileMetadata.bit_rate}
                        <div class="mb-3">
                            <label for="bitRateInput" class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Bit Rate:</label>
                            <span id="bitRateInput" class="text-gray-900 dark:text-gray-100 block w-full border border-gray-300 dark:border-dark-bg-tertiary px-1.5 py-1 bg-gray-50 dark:bg-dark-bg-tertiary min-h-[30px] break-words">{formatBitrate(currentFileMetadata.bit_rate)}</span>
                        </div>
                    {/if}
                    {#if currentFileMetadata.video_codec}
                        <div class="mb-3">
                            <label for="videoCodecInput" class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Video Codec:</label>
                            <span id="videoCodecInput" class="text-gray-900 dark:text-gray-100 block w-full border border-gray-300 dark:border-dark-bg-tertiary px-1.5 py-1 bg-gray-50 dark:bg-dark-bg-tertiary min-h-[30px] break-words">{currentFileMetadata.video_codec || ''}</span>
                        </div>
                    {/if}
                    {#if currentFileMetadata.audio_codec}
                        <div class="mb-3">
                            <label for="audioCodecInput" class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Audio Codec:</label>
                            <span id="audioCodecInput" class="text-gray-900 dark:text-gray-100 block w-full border border-gray-300 dark:border-dark-bg-tertiary px-1.5 py-1 bg-gray-50 dark:bg-dark-bg-tertiary min-h-[30px] break-words">{currentFileMetadata.audio_codec || ''}</span>
                        </div>
                    {/if}
                {/if}

                {#if currentAssetRelativePathForGroups && get(project).id}
                    <hr class="my-4 border-gray-300 dark:border-gray-700">
                    <div class="mb-2">
                        <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 tracking-wider mb-1">Groups</h3>
                        {#if isLoadingFileGroups}
                            <p class="text-xs text-gray-400 dark:text-gray-500 italic">Loading groups...</p>
                        {:else if isEditing}
                            <GroupMultiSelect
                                fileAssetRelativePath={currentAssetRelativePathForGroups}
                                projectId={get(project).id}
                                allProjectGroups={allProjectGroupsForPanel}
                                initiallyAssignedGroups={fileAssignedGroups}
                                isEditable={isEditing}
                                on:groupsUpdated={() => fetchFileAssignedGroups(get(project).id, currentAssetRelativePathForGroups)}
                                on:createNewGroup={() => {
                                    createGroupModalFileToAssign = currentAssetRelativePathForGroups;
                                    isCreateGroupModalOpen = true;
                                }}
                                on:error={(e) => message(e.detail, { title: 'Group Error', type: 'error' })}
                            />
                        {:else}
                            {#if fileAssignedGroups && fileAssignedGroups.length > 0}
                                <div class="flex flex-wrap gap-1 mt-1">
                                    {#each fileAssignedGroups as group (group.id)}
                                        <span class="px-2 py-0.5 text-xs bg-gray-200 dark:bg-dark-bg-tertiary text-gray-700 dark:text-gray-300 rounded-full">
                                            {group.name}
                                        </span>
                                    {/each}
                                </div>
                            {:else}
                                <p class="text-xs text-gray-400 dark:text-gray-500 italic mt-1">No groups assigned.</p>
                            {/if}
                        {/if}
                    </div>
                {/if}

                {#if $customFieldDefinitionsStore && ($customFieldDefinitionsStore.length > 0 || isEditing)}
                    <hr class="my-4 border-gray-300 dark:border-gray-700">
                    <div class="flex justify-between items-center mb-2">
                        <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 tracking-wider">Custom Fields</h3>
                        {#if isEditing}
                            <button
                                on:click={() => showAddFieldModal = true}
                                class="p-1 text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                title="Add Custom Field Definition"
                                aria-label="Add Custom Field Definition"
                            >
                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-plus-circle" viewBox="0 0 16 16">
                                    <path d="M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14zm0 1A8 8 0 1 0 8 0a8 8 0 0 0 0 16z"/>
                                    <path d="M8 4a.5.5 0 0 1 .5.5v3h3a.5.5 0 0 1 0 1h-3v3a.5.5 0 0 1-1 0v-3h-3a.5.5 0 0 1 0-1h3v-3A.5.5 0 0 1 8 4z"/>
                                </svg>
                            </button>
                        {/if}
                    </div>
                {/if}

                {#if !isEditing}
                    {#each displayableCustomFields as field, index (field.key + '-' + index)}
                        <div class="mb-3">
                            <label for={`custom-field-display-${index}`} class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">{field.name || field.key}:</label>
                            <span id={`custom-field-display-${index}`} class="text-gray-900 dark:text-gray-100 {field.type === 'long_text' ? 'whitespace-pre-wrap break-words break-all' : 'break-words break-all'} block w-full border border-gray-300 dark:border-dark-bg-tertiary px-1.5 py-1 bg-gray-50 dark:bg-dark-bg-tertiary min-h-[30px]">
                                {field.value || ''}
                            </span>
                        </div>
                    {/each}
                    {#if displayableCustomFields.length === 0 && $customFieldDefinitionsStore && $customFieldDefinitionsStore.filter(def => { const scope = def.scope; let applicable = false; if (typeof scope === 'string') { if (scope.toLowerCase() === 'project') applicable = true; } else if (scope && typeof scope === 'object' && typeof scope.AssetType === 'string') { const assetTypeScope = scope.AssetType.toLowerCase(); if (assetTypeScope === itemType) applicable = true; else if (assetTypeScope === 'media' && (itemType === 'audio' || itemType === 'video' || itemType === 'media_note')) applicable = true; } return applicable; }).length > 0}
                        <p class="text-xs text-gray-500 dark:text-gray-400 italic">No custom field values set for this item. Edit to add.</p>
                    {/if}
                {/if}

                {#if isEditing}
                    {#each editableMetadata.customFields as field, index (field.key + '-' + index)}
                        <div class="mb-3">
                            <div class="flex justify-between items-center mb-1">
                                <label for={`custom-field-edit-${index}`} class="font-semibold text-gray-600 dark:text-gray-400">{field.name || field.key}:</label>
                                <button
                                    on:click={() => handleDeleteCustomField(field.key)}
                                    title={`Delete '${field.name || field.key}' definition`}
                                    class="p-0.5 text-red-500 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300 focus:outline-none focus:ring-1 focus:ring-red-500"
                                >
                                    {@html TRASH_ICON_SVG}
                                </button>
                            </div>
                            {#if field.type === 'small_text'}
                                <input
                                    type="text"
                                    id={`custom-field-edit-${index}`}
                                    bind:value={editableMetadata.customFields[index].value}
                                    class="mt-0.5 block w-full border border-gray-300 dark:border-dark-bg-tertiary focus:ring-1 focus:ring-blue-500 focus:border-blue-500 dark:bg-dark-bg-secondary dark:text-white px-1.5 py-1 text-xs bg-white text-gray-900"
                                    placeholder={`Enter value for ${field.name || field.key}`}
                                    autocorrect="off" autocomplete="off"/>
                            {:else if field.type === 'long_text'}
                                <textarea
                                    id={`custom-field-edit-${index}`}
                                    rows="3"
                                    bind:value={editableMetadata.customFields[index].value}
                                    class="mt-0.5 block w-full border border-gray-300 dark:border-dark-bg-tertiary focus:ring-1 focus:ring-blue-500 focus:border-blue-500 dark:bg-dark-bg-secondary dark:text-white px-1.5 py-1 text-xs bg-white text-gray-900"
                                    placeholder={`Enter value for ${field.name || field.key}`}
                                    autocorrect="off" autocomplete="off"></textarea>
                            {/if}
                        </div>
                    {/each}
                     {#if editableMetadata.customFields.length === 0 && $customFieldDefinitionsStore && $customFieldDefinitionsStore.filter(def => { const scope = def.scope; let applicable = false; if (typeof scope === 'string') { if (scope.toLowerCase() === 'project') applicable = true; } else if (scope && typeof scope === 'object' && typeof scope.AssetType === 'string') { const assetTypeScope = scope.AssetType.toLowerCase(); if (assetTypeScope === itemType) applicable = true; else if (assetTypeScope === 'media' && (itemType === 'audio' || itemType === 'video' || itemType === 'media_note')) applicable = true; } return applicable; }).length > 0}
                        <p class="text-xs text-gray-500 dark:text-gray-400 italic">No custom fields have values for this item. Edit to add.</p>
                    {/if}
                {/if}

                {#if isEditing}
                    <div class="mt-4 flex justify-end items-center">
                        <button
                            on:click={handleSaveMetadata}
                            class="px-3 py-1.5 bg-blue-600 hover:bg-blue-700 text-white text-xs font-medium focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-opacity-50"
                        >
                            Save Changes
                        </button>
                    </div>
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
