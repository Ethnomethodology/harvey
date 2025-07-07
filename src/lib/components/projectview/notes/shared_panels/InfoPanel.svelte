<!-- src/lib/components/projectview/notes/shared_panels/InfoPanel.svelte -->
<script>
    import { onMount, onDestroy, getContext } from 'svelte'; // Added getContext
    import { get } from 'svelte/store';
    import { project, updateDocumentMetadataDirty, updateMediaNoteMetadataDirty } from '$lib/stores/projectStore.js'; // Added dirty flags
    import { invoke } from '@tauri-apps/api/core';
    import { basename, extname as getFileExtname, sep as getPathSep, resolve } from '@tauri-apps/api/path';
    import { confirm, message } from '@tauri-apps/plugin-dialog';
    import { renameProjectItem } from '$lib/services/projectService.js';
    import AddFieldModal from '$lib/components/projectview/modals/AddFieldModal.svelte';
    import CreateGroupModal from '$lib/components/projectview/modals/CreateGroupModal.svelte';
    import GroupMultiSelect from '$lib/components/projectview/shared/GroupMultiSelect.svelte';
    // Icon import for collapse button is removed
    // panelStateStore import for direct collapse control is removed
    import { deleteDefinition, customFieldDefinitions as customFieldDefinitionsStore, loadAllDefinitions } from '$lib/stores/customFieldStore.js';
    // CategoryTooltip for collapsed view is removed

    const TRASH_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-trash3" viewBox="0 0 16 16"><path d="M6.5 1h3a.5.5 0 0 1 .5.5v1H6v-1a.5.5 0 0 1 .5-.5M11 2.5v-1A1.5 1.5 0 0 0 9.5 0h-3A1.5 1.5 0 0 0 5 1.5v1H2.506a.58.58 0 0 0-.01 0H1.5a.5.5 0 0 0 0 1h.538l.853 10.66A2 2 0 0 0 4.885 16h6.23a2 2 0 0 0 1.994-1.84l.853-10.66h.538a.5.5 0 0 0 0-1h-.995a.59.59 0 0 0-.01 0zm1.958 1-.846 10.58a1 1 0 0 1-.997.92h-6.23a1 1 0 0 1-.997-.92L3.042 3.5zm-7.487 1a.5.5 0 0 1 .528.47l.5 8.5a.5.5 0 0 1-.998.06L5 5.03a.5.5 0 0 1 .47-.53Zm5.058 0a.5.5 0 0 1 .47.53l-.5 8.5a.5.5 0 1 1-.998-.06l.5-8.5a.5.5 0 0 1 .528-.47ZM8 4.5a.5.5 0 0 1 .5.5v8.5a.5.5 0 0 1-1 0V5a.5.5 0 0 1 .5-.5z"/></svg>`;

    // Removed labelTooltip related variables and functions

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

    async function getOriginalAssetDetails(selectedPath, projectStore) {
        // console.log('[InfoPanel getOriginalAssetDetails] Called with selectedPath:', selectedPath, 'Project BaseDir:', projectStore?.baseDirectory);
        if (!selectedPath || !projectStore || !projectStore.baseDirectory) {
            // console.warn('[InfoPanel getOriginalAssetDetails] Missing selectedPath or projectStore data. Returning fallback.');
            const fallbackName = selectedPath ? await basename(selectedPath) : 'Unknown.file';
            return {
                originalRelativePath: selectedPath,
                originalAbsolutePath: selectedPath,
                originalType: fallbackName.includes('.') ? fallbackName.substring(fallbackName.lastIndexOf('.') + 1) : 'unknown',
                originalFileName: fallbackName,
                isView: false
            };
        }

        const pathSep = projectStore.baseDirectory.includes('/') ? '/' : '\\';
        let originalRelativePath = selectedPath.startsWith(projectStore.baseDirectory)
            ? selectedPath.substring(projectStore.baseDirectory.length + (selectedPath.startsWith(projectStore.baseDirectory + pathSep) ? 1 : 0))
            : selectedPath;
        originalRelativePath = originalRelativePath.replace(/\\/g, '/');

        let originalAbsolutePath = selectedPath;
        let originalFileName = await basename(selectedPath);
        let originalType = await getFileExtname(originalFileName).then(ext => ext ? ext.toLowerCase() : 'unknown');
        let isView = false;

        // console.log('[InfoPanel getOriginalAssetDetails] Initial derived details - RelPath:', originalRelativePath, 'AbsPath:', originalAbsolutePath, 'FileName:', originalFileName, 'Type:', originalType);

        const selectedFileExt = await getFileExtname(selectedPath).then(ext => ext ? ext.toLowerCase() : '');

        if (selectedFileExt === 'json') {
            // console.log('[InfoPanel getOriginalAssetDetails] Selected file is JSON, checking if it is a view for another document.');
            const selectedFileNameStem = originalFileName.substring(0, originalFileName.length - (selectedFileExt.length + 1));
            const potentialOriginalExtensions = ['docx', 'pdf', 'txt', 'md'];

            if (projectStore.documentFiles && Array.isArray(projectStore.documentFiles)) {
                for (const docExt of potentialOriginalExtensions) {
                    const potentialOriginalFileName = `${selectedFileNameStem}.${docExt}`;
                    for (const docFile of projectStore.documentFiles) {
                        if (docFile.name === potentialOriginalFileName) {
                            const currentJsonStem = selectedFileNameStem;
                            const originalDocStem = docFile.name.substring(0, docFile.name.lastIndexOf('.'));

                            if (currentJsonStem === originalDocStem) {
                                originalRelativePath = docFile.relativePath.replace(/\\/g, '/');
                                originalAbsolutePath = await resolve(projectStore.baseDirectory, docFile.relativePath);
                                originalFileName = docFile.name;
                                originalType = await getFileExtname(originalFileName).then(ext => ext ? ext.toLowerCase() : 'unknown');
                                isView = true;
                                // console.log('[InfoPanel getOriginalAssetDetails] Identified original asset for JSON view - RelPath:', originalRelativePath, 'FileName:', originalFileName, 'Type:', originalType);
                                break;
                            }
                        }
                    }
                    if (isView) break;
                }
            }
        }

        // console.log('[InfoPanel getOriginalAssetDetails] Returning final details - RelPath:', originalRelativePath, 'AbsPath:', originalAbsolutePath, 'FileName:', originalFileName, 'Type:', originalType, 'isView:', isView);
        return {
            originalRelativePath,
            originalAbsolutePath,
            originalType,
            originalFileName,
            isView
        };
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
        if (bps >= 1000000) {
            return (bps / 1000000).toFixed(2) + ' Mbps';
        }
        if (bps >= 1000) {
            return (bps / 1000).toFixed(0) + ' kbps';
        }
        return bps + ' bps';
    }

    let currentFileMetadata = null;
    let fullLoadedMetadataObject = null;
    let currentOriginalAssetDetails = null;
    let isEditing = false;
    let editableMetadata = {
        file_name: '',
        title: '',
        description: '',
        summary: '',
        customFields: []
    };
    let showAddFieldModal = false;

    function normalizePathForComparison(p) {
        if (!p) return '';
        let normalized = p.replace(/\\/g, '/');
        normalized = normalized.replace(/\/\/{2,}/g, '/');
        return normalized;
    }

    const EDIT_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-pencil-square" viewBox="0 0 16 16"><path d="M15.502 1.94a.5.5 0 0 1 0 .706L14.459 3.69l-2-2L13.502.646a.5.5 0 0 1 .707 0l1.293 1.293zm-1.75 2.456-2-2L4.939 9.21a.5.5 0 0 0-.121.196l-.805 2.414a.25.25 0 0 0 .316.316l2.414-.805a.5.5 0 0 0 .196-.12l6.813-6.814z"/><path fill-rule="evenodd" d="M1 13.5A1.5 1.5 0 0 0 2.5 15h11a1.5 1.5 0 0 0 1.5-1.5v-6a.5.5 0 0 0-1 0v6a.5.5 0 0 1-.5.5h-11a.5.5 0 0 1-.5-.5v-11a.5.5 0 0 1 .5-.5H9a.5.5 0 0 0 0-1H2.5A1.5 1.5 0 0 0 1 2.5z"/></svg>`;
    const CANCEL_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-x-circle" viewBox="0 0 16 16"><path d="M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14m0 1A8 8 0 1 0 8 0a8 8 0 0 0 0 16"/><path d="M4.646 4.646a.5.5 0 0 1 .708 0L8 7.293l2.646-2.647a.5.5 0 0 1 .708.708L8.707 8l2.647 2.646a.5.5 0 0 1-.708.708L8 8.707l-2.646 2.647a.5.5 0 0 1-.708-.708L7.293 8 4.646 5.354a.5.5 0 0 1 0-.708"/></svg>`;

    const AUDIO_EXTENSIONS = new Set(['mp3','wav','m4a','ogg','aac','flac']);
    const VIDEO_EXTENSIONS = new Set(['mp4','mov','avi','mkv','webm']);
    const IMAGE_EXTENSIONS = new Set(['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'tiff']);

    let previousSelectedItemPath = null;
    let displayableCustomFields = [];

    let fileAssignedGroups = [];
    let allProjectGroupsForPanel = [];
    let isLoadingFileGroups = false;
    let isCreateGroupModalOpen = false;
    let createGroupModalFileToAssign = null;
    let currentAssetRelativePathForGroups = null;

    async function fetchAllProjectGroups() {
        if ($project.id) {
            try {
                allProjectGroupsForPanel = await invoke('get_project_groups', { projectId: $project.id });
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
            fileAssignedGroups = await invoke('get_groups_for_file_asset', {
                projectId: projectId,
                fileAssetRelativePath: assetRelativePath
            });
        } catch (error) {
            console.error(`Failed to fetch groups for asset ${assetRelativePath}:`, error);
            fileAssignedGroups = [];
        } finally {
            isLoadingFileGroups = false;
        }
    }

    onMount(async () => {
        previousSelectedItemPath = null; // Initialize here
        try {
            await loadAllDefinitions();
            if ($project.id) { // Ensure project.id is available before fetching
                 await fetchAllProjectGroups();
            }
        } catch (error) {
            message(`Error loading initial data for InfoPanel: ${error.message || error}`, { title: 'Error', type: 'error' });
        }
    });

    $: if ($project.id && allProjectGroupsForPanel.length === 0) { // If project.id becomes available later
        fetchAllProjectGroups();
    }


    async function loadMetadata(assetRelativePathToLoad) {
        // console.log(`[InfoPanel loadMetadata] Called for assetRelativePath: ${assetRelativePathToLoad}`);
        currentFileMetadata = null;
        fullLoadedMetadataObject = null;
        if (isEditing && assetRelativePathToLoad !== previousSelectedItemPath) {
            isEditing = false; // Cancel edit mode if item changes
        }

        if (!assetRelativePathToLoad) {
            // console.log('[InfoPanel loadMetadata] assetRelativePath is null, clearing metadata.');
            previousSelectedItemPath = null; // Reset previous path
            return;
        }

        try {
            if (!$project.id || typeof $project.id !== 'string' || $project.id.trim() === '') {
                console.error('[InfoPanel loadMetadata] Attempted to call get_asset_metadata_command without a valid project ID. Path:', assetRelativePathToLoad, 'Project ID:', $project.id);
                currentFileMetadata = null; // Clear metadata
                previousSelectedItemPath = assetRelativePathToLoad; // Update previous path
                return;
            }
            const result = await invoke('get_asset_metadata_command', {
                projectId: $project.id,
                assetRelativePath: assetRelativePathToLoad
            });

            // console.log(`[InfoPanel loadMetadata] Metadata from backend for ${assetRelativePathToLoad}:`, result);

            if (result) {
                const originalFileNameToUse = currentOriginalAssetDetails?.originalFileName || await basename(assetRelativePathToLoad);
                // const originalAbsolutePathToUse = currentOriginalAssetDetails?.originalAbsolutePath || ($project.baseDirectory ? `${$project.baseDirectory}${getPathSep}${assetRelativePathToLoad}` : assetRelativePathToLoad);

                currentFileMetadata = {
                    file_name: originalFileNameToUse,
                    file_path: assetRelativePathToLoad, // Store the relative path used for loading
                    db_absolute_file_path: currentOriginalAssetDetails?.originalAbsolutePath || result.file_path, // Prefer original, fallback to DB's
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
                    metadata: { ...currentFileMetadata, file_path: currentFileMetadata.db_absolute_file_path }, // Ensure full path for this object
                    customFields: currentFileMetadata.customFields,
                    asset_type: itemType, // Use the itemType prop
                    version: "db_1.0"
                };
            } else {
                // console.log(`[InfoPanel loadMetadata] No metadata found in DB for ${assetRelativePathToLoad}. Initializing empty.`);
                const originalFileNameToUse = currentOriginalAssetDetails?.originalFileName || await basename(assetRelativePathToLoad);
                const originalAbsolutePathToUse = currentOriginalAssetDetails?.originalAbsolutePath || ($project.baseDirectory ? `${$project.baseDirectory}${getPathSep}${assetRelativePathToLoad}` : assetRelativePathToLoad);
                currentFileMetadata = {
                    file_name: originalFileNameToUse,
                    file_path: assetRelativePathToLoad,
                    db_absolute_file_path: originalAbsolutePathToUse,
                    last_modified: new Date().toISOString(), // Placeholder
                    title: '', description: '', summary: '', customFields: [],
                    duration_seconds: null, width: null, height: null, frame_rate: null, bit_rate: null, audio_codec: null, video_codec: null, creation_time: null
                };
                fullLoadedMetadataObject = { metadata: { ...currentFileMetadata }, customFields: [], version: "db_1.0_new" };
            }
        } catch (error) {
            console.error(`[InfoPanel loadMetadata] Error loading metadata for ${assetRelativePathToLoad}:`, error);
            const originalFileNameToUse = currentOriginalAssetDetails?.originalFileName || await basename(assetRelativePathToLoad || 'Unknown.file').catch(() => 'Unknown.file');
            const originalAbsolutePathToUse = currentOriginalAssetDetails?.originalAbsolutePath || ($project.baseDirectory && assetRelativePathToLoad ? `${$project.baseDirectory}${getPathSep}${assetRelativePathToLoad}` : assetRelativePathToLoad || '');
            currentFileMetadata = {
                file_name: originalFileNameToUse,
                file_path: assetRelativePathToLoad || '',
                db_absolute_file_path: originalAbsolutePathToUse,
                last_modified: new Date().toISOString(),
                title: '', description: '', summary: '', customFields: [],
                duration_seconds: null, width: null, height: null, frame_rate: null, bit_rate: null, audio_codec: null, video_codec: null, creation_time: null
            };
            fullLoadedMetadataObject = { metadata: { ...currentFileMetadata }, customFields: [], version: "db_1.0_error" };
            // await message(`Error loading metadata for ${assetRelativePathToLoad}: ${error}`, { title: 'Load Error', type: 'error' });
        }
        previousSelectedItemPath = assetRelativePathToLoad; // Update previous path after loading
    }

    function toggleEditMode() {
        isEditing = !isEditing;
        // if (!isEditing && currentFileMetadata) { // Reset fields if cancelling edit
        //    loadMetadata(currentFileMetadata.file_path);
        // }
    }

    async function handleSaveMetadata() {
        let renameProcessed = false;
        if (!currentFileMetadata || !currentFileMetadata.file_path) {
            await message('Cannot save: File path information is missing.', { title: 'Save Error', type: 'error' });
            return;
        }
        if (!itemType) { // Use itemType prop
            await message('Cannot save: Item type is unknown.', { title: 'Save Error', type: 'error' });
            return;
        }

        try {
            const assetKeyForDb = currentFileMetadata.file_path; // This is the relative path
            const currentFullFileName = currentFileMetadata.file_name; // Original file name from metadata

            const currentFileExtension = await getFileExtname(currentFullFileName).then(ext => ext ? ext.toLowerCase() : '');

            let currentFileNameWithoutExtension;
            if (currentFileExtension && currentFileExtension.length > 0) {
                currentFileNameWithoutExtension = currentFullFileName.substring(0, currentFullFileName.length - (currentFileExtension.length +1) );
            } else {
                currentFileNameWithoutExtension = currentFullFileName;
            }

            const editedFileNameStem = editableMetadata.file_name.trim();

            if (editedFileNameStem !== currentFileNameWithoutExtension) {
                if (!editedFileNameStem) {
                    await message('File name (stem) cannot be empty.', { title: 'Invalid File Name', type: 'error' });
                    isEditing = true; // Keep editing mode
                    return;
                }

                let nameToSendToBackendRenameService;
                // Use itemType prop for logic
                if (itemType === 'media_note' || itemType === 'audio' || itemType === 'video' || itemType === 'imported_transcript') {
                    nameToSendToBackendRenameService = editedFileNameStem;
                } else { // For doc, table, image
                    nameToSendToBackendRenameService = currentFileExtension && currentFileExtension.length > 0
                        ? editedFileNameStem + "." + currentFileExtension
                        : editedFileNameStem;
                }

                let effectiveItemTypeForRename = itemType;
                if (itemType === 'media_note') effectiveItemTypeForRename = 'media'; // Map media_note to media for backend

                try {
                    // renameProjectItem expects the original relative path
                    await renameProjectItem(assetKeyForDb, nameToSendToBackendRenameService, effectiveItemTypeForRename);
                    // After successful rename, the projectStore will update, which should trigger a reload of this panel
                    // via the reactive itemPath prop changing.
                    isEditing = false;
                    renameProcessed = true;
                    // No direct loadMetadata here, rely on NotesView/ProjectView to update itemPath if it changed due to rename
                    // This component should react to itemPath prop changes.
                    project.update(p => ({...p, isDocumentMetadataDirty: false, isMediaNoteMetadataDirty: false})); // Reset dirty flags
                } catch (err) {
                    await message(`Error renaming item: ${err.message || err}`, { title: 'Rename Failed', type: 'error' });
                    isEditing = true; // Keep editing mode on failure
                    return; // Stop further processing
                }
            }

            if (!renameProcessed) { // Only save other metadata if rename didn't happen or wasn't needed
                const originalAssetAbsolutePath = currentFileMetadata.db_absolute_file_path;

                if (!originalAssetAbsolutePath || originalAssetAbsolutePath.trim() === '') {
                    await message('Cannot save: Original asset absolute path could not be determined.', { title: 'Save Error', type: 'error' });
                    isEditing = true;
                    return;
                }

                const metadataPayloadForDb = {
                    file_name: currentFileMetadata.file_name, // Use the (potentially new) file_name from currentFileMetadata
                    file_path: originalAssetAbsolutePath, // Always use the absolute path for DB record key
                    last_modified: new Date().toISOString(),
                    title: editableMetadata.title.trim(),
                    description: editableMetadata.description.trim(),
                    summary: editableMetadata.summary.trim(),
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
                    await invoke('update_asset_metadata_command', {
                        projectXmlPathStr: $project.xmlPath,
                        assetRelativePath: assetKeyForDb, // Use relative path to identify asset in project context
                        metadataPayload: metadataPayloadForDb,
                        customFieldsPayload: customFieldsToSaveForDb,
                        assetType: itemType // Use itemType prop
                    });
                    // Update local state to reflect saved data without full reload if not renamed
                    currentFileMetadata.title = metadataPayloadForDb.title;
                    currentFileMetadata.description = metadataPayloadForDb.description;
                    currentFileMetadata.summary = metadataPayloadForDb.summary;
                    currentFileMetadata.last_modified = metadataPayloadForDb.last_modified;
                    currentFileMetadata.customFields = JSON.parse(JSON.stringify(customFieldsToSaveForDb));

                    if (fullLoadedMetadataObject) {
                        fullLoadedMetadataObject.metadata = { ...currentFileMetadata, file_path: metadataPayloadForDb.file_path };
                        fullLoadedMetadataObject.customFields = currentFileMetadata.customFields;
                    }
                    isEditing = false;
                    project.update(p => ({...p, isDocumentMetadataDirty: false, isMediaNoteMetadataDirty: false})); // Reset dirty flags
                    // No explicit success message here, changes reflected should be enough
                    // Consider a subtle save indicator if needed
                } catch (err) {
                    console.error("Error saving metadata via update_asset_metadata_command:", err);
                    await message(`Error saving metadata: ${err}. Please check console.`, { title: 'Save Failed', type: 'error' });
                }
            }
        } catch (err) {
            console.error("Unexpected error in handleSaveMetadata:", err);
            await message(`An unexpected error occurred: ${err.message || err}.`, { title: 'Error', type: 'error' });
            isEditing = true; // Keep editing mode
        }
    }

    // Props to receive from parent
    export let itemPath = null; // This will be the key prop to trigger updates
    export let itemType = null; // This will be passed from parent view (e.g. DocumentView, MediaView)

    // Main reactive block:
    // When itemPath or itemType changes, get original asset details and then load metadata.
    $: {
        // console.log(`[InfoPanel Reactive Block] itemPath: "${itemPath}", itemType: "${itemType}", previousSelectedItemPath: "${previousSelectedItemPath}"`);
        if (itemPath && itemType) {
            if (itemPath !== previousSelectedItemPath) {
                // console.log(`[InfoPanel Reactive Block] Path changed from "${previousSelectedItemPath}" to "${itemPath}". Reloading.`);
                isEditing = false; // Reset edit mode when item changes
                (async () => {
                    currentOriginalAssetDetails = await getOriginalAssetDetails(itemPath, get(project));
                    // console.log(`[InfoPanel Reactive Block] currentOriginalAssetDetails set to:`, currentOriginalAssetDetails);
                    if (currentOriginalAssetDetails && currentOriginalAssetDetails.originalRelativePath) {
                        currentAssetRelativePathForGroups = currentOriginalAssetDetails.originalRelativePath;
                        await loadMetadata(currentOriginalAssetDetails.originalRelativePath);
                        if (get(project).id && currentAssetRelativePathForGroups) {
                             await fetchFileAssignedGroups(get(project).id, currentAssetRelativePathForGroups);
                        }
                    } else {
                        // console.warn(`[InfoPanel Reactive Block] Could not derive originalRelativePath for itemPath: ${itemPath}. Metadata might be incorrect.`);
                        currentFileMetadata = null; // Clear if path is invalid or details missing
                        currentAssetRelativePathForGroups = null;
                        fileAssignedGroups = [];
                    }
                })();
            } else {
                // console.log(`[InfoPanel Reactive Block] itemPath "${itemPath}" is same as previous. No full reload, but checking itemType.`);
                // Item path is the same, but itemType might have changed (less likely but possible if store updates weirdly)
                // Or, if metadata was null and now path/type are valid, try loading.
                if (currentFileMetadata === null && currentOriginalAssetDetails && currentOriginalAssetDetails.originalRelativePath) {
                    // console.log(`[InfoPanel Reactive Block] Metadata was null, attempting reload for same path.`);
                     (async () => {
                        await loadMetadata(currentOriginalAssetDetails.originalRelativePath);
                         if (get(project).id && currentAssetRelativePathForGroups) {
                             await fetchFileAssignedGroups(get(project).id, currentAssetRelativePathForGroups);
                         }
                    })();
                }
            }
        } else {
            // console.log(`[InfoPanel Reactive Block] itemPath or itemType is null. Clearing metadata.`);
            currentFileMetadata = null;
            fullLoadedMetadataObject = null;
            currentOriginalAssetDetails = null;
            previousSelectedItemPath = null;
            isEditing = false;
            currentAssetRelativePathForGroups = null;
            fileAssignedGroups = [];
        }
    }


    // Reactive block to manage editableMetadata and displayableCustomFields for the UI
    $: {
        if (currentFileMetadata && $customFieldDefinitionsStore) {
            const assetCustomValues = currentFileMetadata.customFields || [];
            let newEditableCustomFields = [];
            let newDisplayableCustomFields = [];

            for (const def of $customFieldDefinitionsStore) {
                let isApplicable = false;
                if (typeof def.scope === 'string') { // Project-wide scope
                    if (def.scope.toLowerCase() === 'project') isApplicable = true;
                } else if (def.scope && typeof def.scope === 'object' && typeof def.scope.AssetType === 'string') { // AssetType specific scope
                    const assetTypeScopeValue = def.scope.AssetType.toLowerCase();
                    if (assetTypeScopeValue === itemType) isApplicable = true; // Direct match with itemType prop
                    else if (assetTypeScopeValue === 'media' && (itemType === 'audio' || itemType === 'video' || itemType === 'media_note')) isApplicable = true;
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

        } else { // Clear if no metadata or definitions
            editableMetadata.customFields = [];
            displayableCustomFields = [];
        }

        // Populate editable fields when edit mode starts or metadata changes while editing
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
        } else if (!isEditing) { // Clear editable fields when not in edit mode
            editableMetadata.file_name = '';
            editableMetadata.title = '';
            editableMetadata.description = '';
            editableMetadata.summary = '';
            // editableMetadata.customFields are already handled above based on isEditing
        }
    }

    // Update project store's dirty flags based on local changes
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
        } else if (itemType === 'media_note' || itemType === 'audio' || itemType === 'video') {
            project.update(p => ({ ...p, isMediaNoteMetadataDirty: isDirty }));
        }
    } else { // Not editing or no metadata, so not dirty from this panel's perspective
        if (itemType === 'doc' || itemType === 'table' || itemType === 'image' || itemType === 'imported_transcript') {
            if (get(project).isDocumentMetadataDirty) project.update(p => ({ ...p, isDocumentMetadataDirty: false }));
        } else if (itemType === 'media_note' || itemType === 'audio' || itemType === 'video') {
            if (get(project).isMediaNoteMetadataDirty) project.update(p => ({ ...p, isMediaNoteMetadataDirty: false }));
        }
    }


</script>

<div class="h-full bg-white dark:bg-gray-800 rounded-md shadow flex flex-col overflow-hidden py-2">
    <!-- Header with Title and Edit Button -->
    <div class="text-sm font-semibold border-b pb-1 px-1 border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 flex-shrink-0 flex items-center justify-between h-8 mb-2">
        <span class="ml-1">Metadata</span>
        {#if currentFileMetadata}
            <button
                on:click={toggleEditMode}
                class="p-1 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                title={isEditing ? 'Cancel Edit' : 'Edit Metadata'}
            >
                {@html isEditing ? CANCEL_ICON_SVG : EDIT_ICON_SVG}
            </button>
        {/if}
    </div>

    <!-- Content Area -->
    <div class="flex-grow overflow-y-auto overflow-x-hidden min-h-0 text-xs relative px-2">
        {#if currentFileMetadata}
            <div class="space-y-2">
                <!-- File Name (editable for stem, display full) -->
                <div class="mb-3">
                    <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">File Name:</label>
                    {#if isEditing}
                        <input type="text" bind:value={editableMetadata.file_name} class="mt-0.5 block w-full rounded-md border border-gray-300 dark:border-gray-600 focus:ring-1 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white px-1.5 py-1 text-xs bg-white text-gray-900" placeholder="Enter name without extension" autocorrect="off" autocomplete="off"/>
                        {#if currentFileMetadata.file_name && currentFileMetadata.file_name.includes('.')}
                            <span class="mt-1 text-gray-500 dark:text-gray-400 text-xs block">
                                Extension: {currentFileMetadata.file_name.substring(currentFileMetadata.file_name.lastIndexOf('.'))}
                            </span>
                        {/if}
                    {:else}
                        <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px] break-words">{currentFileMetadata.file_name || 'N/A'}</span>
                    {/if}
                </div>

                <!-- File Path (read-only) -->
                <div class="mb-3">
                    <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">File Path (Relative):</label>
                    <span class="text-gray-900 dark:text-gray-100 break-all block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">{currentFileMetadata.file_path || ''}</span>
                </div>

                <!-- Created At (read-only) -->
                {#if currentFileMetadata.creation_time}
                <div class="mb-3">
                    <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Created At:</label>
                    <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">{new Date(currentFileMetadata.creation_time).toLocaleString()}</span>
                </div>
                {/if}

                <!-- Last Modified (read-only) -->
                {#if currentFileMetadata.last_modified}
                <div class="mb-3">
                    <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Last Modified:</label>
                    <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">{new Date(currentFileMetadata.last_modified).toLocaleString()}</span>
                </div>
                {/if}

                <!-- Title (editable) -->
                <div class="mb-3">
                    <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Title:</label>
                    {#if isEditing}
                        <input type="text" bind:value={editableMetadata.title} class="mt-0.5 block w-full rounded-md border border-gray-300 dark:border-gray-600 focus:ring-1 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white px-1.5 py-1 text-xs bg-white text-gray-900" autocorrect="off" autocomplete="off"/>
                    {:else}
                        <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px] break-words">{currentFileMetadata.title || ''}</span>
                    {/if}
                </div>

                <!-- Description (editable) -->
                <div class="mb-3">
                    <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Description:</label>
                    {#if isEditing}
                        <textarea bind:value={editableMetadata.description} rows="3" class="mt-0.5 block w-full rounded-md border border-gray-300 dark:border-gray-600 focus:ring-1 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white px-1.5 py-1 text-xs bg-white text-gray-900" autocorrect="off" autocomplete="off"></textarea>
                    {:else}
                        <span class="text-gray-900 dark:text-gray-100 whitespace-pre-wrap break-words break-all block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">{currentFileMetadata.description || ''}</span>
                    {/if}
                </div>

                <!-- Summary (editable) -->
                <div class="mb-3">
                    <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Summary:</label>
                    {#if isEditing}
                        <textarea bind:value={editableMetadata.summary} rows="2" class="mt-0.5 block w-full rounded-md border border-gray-300 dark:border-gray-600 focus:ring-1 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white px-1.5 py-1 text-xs bg-white text-gray-900" autocorrect="off" autocomplete="off"></textarea>
                    {:else}
                        <span class="text-gray-900 dark:text-gray-100 whitespace-pre-wrap break-words break-all block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">{currentFileMetadata.summary || ''}</span>
                    {/if}
                </div>

                <!-- Technical Metadata Section -->
                {#if (currentFileMetadata.duration_seconds || currentFileMetadata.width || currentFileMetadata.video_codec || currentFileMetadata.audio_codec || currentFileMetadata.bit_rate) && !(currentFileMetadata.customFields && currentFileMetadata.customFields.some(cf => cf.key === '_isScreenshot' && cf.value === true))}
                    <hr class="my-4 border-gray-300 dark:border-gray-700">
                    <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 tracking-wider mb-2">Technical Details</h3>

                    {#if currentFileMetadata.duration_seconds}
                        <div class="mb-3">
                            <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Duration:</label>
                            <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px] break-words">{formatDuration(currentFileMetadata.duration_seconds)}</span>
                        </div>
                    {/if}

                    {#if currentFileMetadata.width && currentFileMetadata.height}
                        <div class="mb-3">
                            <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Dimensions:</label>
                            <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px] break-words">{currentFileMetadata.width} x {currentFileMetadata.height}</span>
                        </div>
                    {/if}

                    {#if currentFileMetadata.frame_rate}
                        <div class="mb-3">
                            <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Frame Rate:</label>
                            <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px] break-words">{currentFileMetadata.frame_rate.toFixed(2)} fps</span>
                        </div>
                    {/if}

                    {#if currentFileMetadata.bit_rate}
                        <div class="mb-3">
                            <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Bit Rate:</label>
                            <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px] break-words">{formatBitrate(currentFileMetadata.bit_rate)}</span>
                        </div>
                    {/if}

                    {#if currentFileMetadata.video_codec}
                        <div class="mb-3">
                            <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Video Codec:</label>
                            <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px] break-words">{currentFileMetadata.video_codec || ''}</span>
                        </div>
                    {/if}

                    {#if currentFileMetadata.audio_codec}
                        <div class="mb-3">
                            <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Audio Codec:</label>
                            <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px] break-words">{currentFileMetadata.audio_codec || ''}</span>
                        </div>
                    {/if}
                {/if}
                <!-- End of Technical Metadata Section -->

                <!-- Custom Fields Section -->
                {#if $customFieldDefinitionsStore && ($customFieldDefinitionsStore.length > 0 || isEditing)}
                    <hr class="my-4 border-gray-300 dark:border-gray-700">
                    <div class="flex justify-between items-center mb-2">
                        <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 tracking-wider">Custom Fields</h3>
                        {#if isEditing}
                            <button
                                on:click={() => showAddFieldModal = true}
                                class="p-1 text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                                title="Add Custom Field Definition"
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
                {#if !isEditing}
                    {#each displayableCustomFields as field, index (field.key + '-' + index)}
                        <div class="mb-3">
                            <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">{field.name || field.key}:</label>
                            <span class="text-gray-900 dark:text-gray-100 {field.type === 'long_text' ? 'whitespace-pre-wrap break-words break-all' : 'break-words break-all'} block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">
                                {field.value || ''}
                            </span>
                        </div>
                    {/each}
                    {#if displayableCustomFields.length === 0 && $customFieldDefinitionsStore && $customFieldDefinitionsStore.filter(def => { const scope = def.scope; let applicable = false; if (typeof scope === 'string') { if (scope.toLowerCase() === 'project') applicable = true; } else if (scope && typeof scope === 'object' && typeof scope.AssetType === 'string') { const assetTypeScope = scope.AssetType.toLowerCase(); if (assetTypeScope === itemType) applicable = true; else if (assetTypeScope === 'media' && (itemType === 'audio' || itemType === 'video' || itemType === 'media_note')) applicable = true; } return applicable; }).length > 0}
                        <p class="text-xs text-gray-500 dark:text-gray-400 italic">No custom field values set for this item. Edit to add.</p>
                    {/if}
                {/if}

                <!-- Edit Mode Custom Fields -->
                {#if isEditing}
                    {#each editableMetadata.customFields as field, index (field.key + '-' + index)}
                        <div class="mb-3">
                            <div class="flex justify-between items-center mb-1">
                                <label for={`custom-field-edit-${index}`} class="font-semibold text-gray-600 dark:text-gray-400">{field.name || field.key}:</label>
                                <button
                                    on:click={() => handleDeleteCustomField(field.key)}
                                    title={`Delete '${field.name || field.key}' definition`}
                                    class="p-0.5 text-red-500 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300 rounded focus:outline-none focus:ring-1 focus:ring-red-500"
                                >
                                    {@html TRASH_ICON_SVG}
                                </button>
                            </div>
                            {#if field.type === 'small_text'}
                                <input
                                    type="text"
                                    id={`custom-field-edit-${index}`}
                                    bind:value={editableMetadata.customFields[index].value}
                                    class="mt-0.5 block w-full rounded-md border border-gray-300 dark:border-gray-600 focus:ring-1 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white px-1.5 py-1 text-xs bg-white text-gray-900"
                                    placeholder={`Enter value for ${field.name || field.key}`}
                                    autocorrect="off" autocomplete="off"/>
                            {:else if field.type === 'long_text'}
                                <textarea
                                    id={`custom-field-edit-${index}`}
                                    rows="3"
                                    bind:value={editableMetadata.customFields[index].value}
                                    class="mt-0.5 block w-full rounded-md border border-gray-300 dark:border-gray-600 focus:ring-1 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white px-1.5 py-1 text-xs bg-white text-gray-900"
                                    placeholder={`Enter value for ${field.name || field.key}`}
                                    autocorrect="off" autocomplete="off"></textarea>
                            {/if}
                            <!-- TODO: Add support for other field types (number, date, boolean) -->
                        </div>
                    {/each}
                     {#if editableMetadata.customFields.length === 0 && $customFieldDefinitionsStore && $customFieldDefinitionsStore.filter(def => { const scope = def.scope; let applicable = false; if (typeof scope === 'string') { if (scope.toLowerCase() === 'project') applicable = true; } else if (scope && typeof scope === 'object' && typeof scope.AssetType === 'string') { const assetTypeScope = scope.AssetType.toLowerCase(); if (assetTypeScope === itemType) applicable = true; else if (assetTypeScope === 'media' && (itemType === 'audio' || itemType === 'video' || itemType === 'media_note')) applicable = true; } return applicable; }).length > 0}
                        <p class="text-xs text-gray-500 dark:text-gray-400 italic">No custom fields have values for this item. Edit to add.</p>
                    {/if}
                {/if}
                <!-- End of custom fields rendering -->

                <!-- Groups Section -->
                {#if currentAssetRelativePathForGroups && $project.id}
                    <div class="mt-3">
                        <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 mb-1">Groups</h3>
                        {#if isLoadingFileGroups}
                            <p class="text-xs text-gray-400 dark:text-gray-500 italic">Loading groups...</p>
                        {:else if isEditing}
                            <GroupMultiSelect
                                fileAssetRelativePath={currentAssetRelativePathForGroups}
                                projectId={$project.id}
                                allProjectGroups={allProjectGroupsForPanel}
                                initiallyAssignedGroups={fileAssignedGroups}
                                isEditable={isEditing}
                                on:groupsUpdated={() => fetchFileAssignedGroups($project.id, currentAssetRelativePathForGroups)}
                                on:createNewGroup={() => {
                                    createGroupModalFileToAssign = currentAssetRelativePathForGroups;
                                    isCreateGroupModalOpen = true;
                                }}
                                on:error={(e) => message(e.detail, { title: 'Group Error', type: 'error' })}
                            />
                        {:else}
                            <!-- Read-only display -->
                            {#if fileAssignedGroups && fileAssignedGroups.length > 0}
                                <div class="flex flex-wrap gap-1 mt-1">
                                    {#each fileAssignedGroups as group (group.id)}
                                        <span class="px-2 py-0.5 text-xs bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-md">
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
                <!-- End of Groups Section -->


                {#if isEditing}
                    <div class="mt-4 flex justify-end items-center">
                        <button
                            on:click={handleSaveMetadata}
                            class="px-3 py-1.5 bg-blue-600 hover:bg-blue-700 text-white rounded-md text-xs font-medium focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-opacity-50"
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

<!-- Removed CategoryTooltip as collapsed view is gone -->
<!-- <CategoryTooltip ... /> -->

<AddFieldModal bind:showModal={showAddFieldModal} currentItemType={itemType} on:close={() => showAddFieldModal = false} />

{#if isCreateGroupModalOpen && $project.id}
    <CreateGroupModal
        bind:showModal={isCreateGroupModalOpen}
        projectUuid={$project.id}
        fileToAdd={createGroupModalFileToAssign ? { relativePath: createGroupModalFileToAssign, name: '' } : null}
        on:groupCreated={async (event) => {
            isCreateGroupModalOpen = false;
            await fetchAllProjectGroups();
            if (event.detail.group && createGroupModalFileToAssign) {
                 await fetchFileAssignedGroups($project.id, createGroupModalFileToAssign);
            } else if (event.detail.group && !createGroupModalFileToAssign) {
                 if (currentAssetRelativePathForGroups) {
                    await fetchFileAssignedGroups($project.id, currentAssetRelativePathForGroups);
                 }
            }
            createGroupModalFileToAssign = null;
        }}
        on:groupCreatedAndFileAdded={async (event) => {
            isCreateGroupModalOpen = false;
            await fetchAllProjectGroups();
            if (event.detail.file && event.detail.file.relativePath === currentAssetRelativePathForGroups) {
                await fetchFileAssignedGroups($project.id, currentAssetRelativePathForGroups);
            }
             createGroupModalFileToAssign = null;
        }}
        on:close={() => {
            isCreateGroupModalOpen = false;
            createGroupModalFileToAssign = null;
        }}
    />
{/if}