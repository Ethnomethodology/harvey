<!-- src/lib/components/projectview/notes/shared_panels/LeftInfoPanel.svelte -->
<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { project } from '$lib/stores/projectStore.js';
    import { invoke } from '@tauri-apps/api/core';
    // fsRename might still be used by projectService.js, direct fs calls for metadata are removed.
    import { basename, extname as getFileExtname, sep as getPathSep, resolve } from '@tauri-apps/api/path'; // Added resolve
    import { confirm, message } from '@tauri-apps/plugin-dialog';
    import { renameProjectItem } from '$lib/services/projectService.js';
    import AddFieldModal from '$lib/components/projectview/modals/AddFieldModal.svelte';
    import CreateGroupModal from '$lib/components/projectview/modals/CreateGroupModal.svelte'; // Added
    import GroupMultiSelect from '$lib/components/projectview/infopanels/GroupMultiSelect.svelte'; // Added
    import type { GroupData } from '$lib/types'; // Added
    import FileEarmarkCodeIcon from '$lib/components/icons/FileEarmarkCodeIcon.svelte';
    import panelStateStore from '$lib/stores/panelStateStore.js';
    import { deleteDefinition, customFieldDefinitions as customFieldDefinitionsStore, loadAllDefinitions } from '$lib/stores/customFieldStore.js'; // Ensure deleteDefinition is imported
    import CategoryTooltip from '../CategoryTooltip.svelte';

    const TRASH_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-trash3" viewBox="0 0 16 16"><path d="M6.5 1h3a.5.5 0 0 1 .5.5v1H6v-1a.5.5 0 0 1 .5-.5M11 2.5v-1A1.5 1.5 0 0 0 9.5 0h-3A1.5 1.5 0 0 0 5 1.5v1H2.506a.58.58 0 0 0-.01 0H1.5a.5.5 0 0 0 0 1h.538l.853 10.66A2 2 0 0 0 4.885 16h6.23a2 2 0 0 0 1.994-1.84l.853-10.66h.538a.5.5 0 0 0 0-1h-.995a.59.59 0 0 0-.01 0zm1.958 1-.846 10.58a1 1 0 0 1-.997.92h-6.23a1 1 0 0 1-.997-.92L3.042 3.5zm-7.487 1a.5.5 0 0 1 .528.47l.5 8.5a.5.5 0 0 1-.998.06L5 5.03a.5.5 0 0 1 .47-.53Zm5.058 0a.5.5 0 0 1 .47.53l-.5 8.5a.5.5 0 1 1-.998-.06l.5-8.5a.5.5 0 0 1 .528-.47ZM8 4.5a.5.5 0 0 1 .5.5v8.5a.5.5 0 0 1-1 0V5a.5.5 0 0 1 .5-.5z"/></svg>`;

    let labelTooltipVisible = false;
    let labelTooltipTitle = '';
    let labelTooltipText = '';
    let labelTooltipX = 0;
    let labelTooltipY = 0;

    let documentClickHandler = null;

    function showLabelTooltip(event, title, textContent) {
        if (!$panelStateStore.leftCollapsed) return; // Ensure panel is actually collapsed

        const GITHUB_ISSUE_MAX_TOOLTIP_WIDTH = 280; // Max width in px, adjust as needed
        const GITHUB_ISSUE_TOOLTIP_OFFSET_X = 4; // Offset from the right edge of the label
        const GITHUB_ISSUE_VIEWPORT_MARGIN = 8; // Margin from viewport edges

        const targetRect = event.currentTarget.getBoundingClientRect();

        labelTooltipTitle = title;
        labelTooltipText = textContent || ''; // Ensure textContent is not null/undefined

        // Position tooltip to the right of the label
        let potentialX = targetRect.right + GITHUB_ISSUE_TOOLTIP_OFFSET_X;
        let potentialY = targetRect.top;

        // Basic collision detection with viewport (assuming tooltip width)
        // This doesn't dynamically get the tooltip's rendered width, uses a constant
        if (potentialX + GITHUB_ISSUE_MAX_TOOLTIP_WIDTH > window.innerWidth - GITHUB_ISSUE_VIEWPORT_MARGIN) {
            // If it overflows right, position it to the left of the label
            potentialX = targetRect.left - GITHUB_ISSUE_MAX_TOOLTIP_WIDTH - GITHUB_ISSUE_TOOLTIP_OFFSET_X;
            if (potentialX < GITHUB_ISSUE_VIEWPORT_MARGIN) { // If it also overflows left, reset to right
                 potentialX = targetRect.right + GITHUB_ISSUE_TOOLTIP_OFFSET_X;
            }
        }

        // Basic Y collision (less common for tooltips usually, but good to have)
        // Assuming tooltip height is roughly targetRect.height for simplicity here, real height unknown
        if (potentialY + targetRect.height > window.innerHeight - GITHUB_ISSUE_VIEWPORT_MARGIN) {
            potentialY = targetRect.bottom - targetRect.height; // Adjust if it overflows bottom
        }
        if (potentialY < GITHUB_ISSUE_VIEWPORT_MARGIN) {
            potentialY = GITHUB_ISSUE_VIEWPORT_MARGIN; // Adjust if it overflows top
        }

        labelTooltipX = potentialX;
        labelTooltipY = potentialY;
        labelTooltipVisible = true;

        if (labelTooltipVisible && !documentClickHandler) {
            documentClickHandler = (e_click) => {
                hideLabelTooltip();
            };
            setTimeout(() => {
                if (labelTooltipVisible) {
                    document.addEventListener('click', documentClickHandler);
                }
            }, 0);
        }
    }

    function hideLabelTooltip() {
        if (documentClickHandler) {
            document.removeEventListener('click', documentClickHandler);
            documentClickHandler = null;
        }
        labelTooltipVisible = false;
    }

    onDestroy(() => {
        if (documentClickHandler) {
            document.removeEventListener('click', documentClickHandler);
            documentClickHandler = null;
        }
    });

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

    // Helper function to get details of the original asset
    async function getOriginalAssetDetails(selectedPath, projectStore) {
        console.log('[LIP getOriginalAssetDetails] Called with selectedPath:', selectedPath, 'Project BaseDir:', projectStore?.baseDirectory);
        if (!selectedPath || !projectStore || !projectStore.baseDirectory) {
            console.warn('[LIP getOriginalAssetDetails] Missing selectedPath or projectStore data. Returning fallback.');
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

        console.log('[LIP getOriginalAssetDetails] Initial derived details - RelPath:', originalRelativePath, 'AbsPath:', originalAbsolutePath, 'FileName:', originalFileName, 'Type:', originalType);

        const selectedFileExt = await getFileExtname(selectedPath).then(ext => ext ? ext.toLowerCase() : '');

        if (selectedFileExt === 'json') {
            console.log('[LIP getOriginalAssetDetails] Selected file is JSON, checking if it is a view for another document.');
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
                                console.log('[LIP getOriginalAssetDetails] Identified original asset for JSON view - RelPath:', originalRelativePath, 'FileName:', originalFileName, 'Type:', originalType);
                                break;
                            }
                        }
                    }
                    if (isView) break;
                }
            }
        }

        console.log('[LIP getOriginalAssetDetails] Returning final details - RelPath:', originalRelativePath, 'AbsPath:', originalAbsolutePath, 'FileName:', originalFileName, 'Type:', originalType, 'isView:', isView);
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

    // State for groups
    let fileAssignedGroups: GroupData[] = [];
    let allProjectGroupsForPanel: GroupData[] = [];
    let isLoadingFileGroups: boolean = false;
    let isCreateGroupModalOpen = false;
    let createGroupModalFileToAssign: string | null = null;
    let currentAssetRelativePathForGroups: string | null = null; // To track for group loading

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
            // Optionally show a toast error to the user
        } finally {
            isLoadingFileGroups = false;
        }
    }

    onMount(async () => {
        previousSelectedItemPath = null;
        try {
            await loadAllDefinitions();
            await fetchAllProjectGroups(); // Fetch all groups when component mounts and project.id is available
        } catch (error) {
            message(`Error loading initial data: ${error.message || error}`, { title: 'Error', type: 'error' });
        }
    });

    // Refetch all project groups if project ID changes
    $: if ($project.id) {
        fetchAllProjectGroups();
    }

    async function loadMetadata(assetRelativePath) {
        currentFileMetadata = null;
        fullLoadedMetadataObject = null;
        if (isEditing && assetRelativePath !== previousSelectedItemPath) {
            isEditing = false;
        }

        if (!assetRelativePath) {
            return;
        }

        try {
            if (!$project.id || typeof $project.id !== 'string' || $project.id.trim() === '') {
                console.error('[LeftInfoPanel loadMetadata] Attempted to call get_asset_metadata_command without a valid project ID (from $project.id). Path:', assetRelativePath, 'Project ID:', $project.id);
                currentFileMetadata = null;
                return;
            }
            const result = await invoke('get_asset_metadata_command', {
                projectId: $project.id,
                assetRelativePath: assetRelativePath
            });

            if (result) {
                const originalFileNameToUse = currentOriginalAssetDetails?.originalFileName || await basename(assetRelativePath);
                const originalAbsolutePathToUse = currentOriginalAssetDetails?.originalAbsolutePath || ($project.baseDirectory ? `${$project.baseDirectory}${getPathSep}${assetRelativePath}` : assetRelativePath);

                currentFileMetadata = {
                    file_name: originalFileNameToUse,
                    file_path: assetRelativePath,
                    db_absolute_file_path: currentOriginalAssetDetails?.originalAbsolutePath || result.file_path,
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
                    metadata: { ...currentFileMetadata, file_path: currentOriginalAssetDetails?.originalAbsolutePath || result.file_path },
                    customFields: currentFileMetadata.customFields,
                    asset_type: currentItemType,
                    version: "db_1.0"
                };
            } else {
                const originalFileNameToUse = currentOriginalAssetDetails?.originalFileName || await basename(assetRelativePath);
                const originalAbsolutePathToUse = currentOriginalAssetDetails?.originalAbsolutePath || ($project.baseDirectory ? `${$project.baseDirectory}${getPathSep}${assetRelativePath}` : assetRelativePath);

                currentFileMetadata = {
                    file_name: originalFileNameToUse,
                    file_path: assetRelativePath,
                    db_absolute_file_path: originalAbsolutePathToUse,
                    last_modified: new Date().toISOString(),
                    title: '', description: '', summary: '', customFields: [],
                    duration_seconds: null, width: null, height: null, frame_rate: null, bit_rate: null, audio_codec: null, video_codec: null, creation_time: null
                };
                fullLoadedMetadataObject = { metadata: { ...currentFileMetadata }, customFields: [], version: "db_1.0_new" };
            }
        } catch (error) {
            const originalFileNameToUse = currentOriginalAssetDetails?.originalFileName || await basename(assetRelativePath || 'Unknown.file').catch(() => 'Unknown.file');
            const originalAbsolutePathToUse = currentOriginalAssetDetails?.originalAbsolutePath || ($project.baseDirectory && assetRelativePath ? `${$project.baseDirectory}${getPathSep}${assetRelativePath}` : assetRelativePath || '');

            currentFileMetadata = {
                file_name: originalFileNameToUse,
                file_path: assetRelativePath || '',
                db_absolute_file_path: originalAbsolutePathToUse,
                last_modified: new Date().toISOString(),
                title: '', description: '', summary: '', customFields: [],
                duration_seconds: null, width: null, height: null, frame_rate: null, bit_rate: null, audio_codec: null, video_codec: null, creation_time: null
            };
            fullLoadedMetadataObject = { metadata: { ...currentFileMetadata }, customFields: [], version: "db_1.0_error" };
            await message(`Error loading metadata: ${error}`, { title: 'Load Error', type: 'error' });
        }

        if (!currentFileMetadata) {
            const originalFileNameToUse = currentOriginalAssetDetails?.originalFileName || await basename(assetRelativePath || 'Unknown.file').catch(() => 'Unknown.file');
            const originalAbsolutePathToUse = currentOriginalAssetDetails?.originalAbsolutePath || ($project.baseDirectory && assetRelativePath ? `${$project.baseDirectory}${getPathSep}${assetRelativePath}` : assetRelativePath || '');

            currentFileMetadata = {
                file_name: originalFileNameToUse,
                file_path: assetRelativePath || '',
                db_absolute_file_path: originalAbsolutePathToUse,
                last_modified: new Date().toISOString(),
                title: '', description: '', summary: '', customFields: [],
                duration_seconds: null, width: null, height: null, frame_rate: null, bit_rate: null, audio_codec: null, video_codec: null, creation_time: null
            };
            if (isEditing) isEditing = false;
        }
    }

    function toggleEditMode() {
        isEditing = !isEditing;
    }

    async function handleSaveMetadata() {
        let renameProcessed = false;
        if (!currentFileMetadata || !currentFileMetadata.file_path) {
            await message('Cannot save: File path information is missing.', { title: 'Save Error', type: 'error' });
            return;
        }
        if (!currentItemType) {
            await message('Cannot save: Item type is unknown.', { title: 'Save Error', type: 'error' });
            return;
        }

        try {
            const assetKeyForDb = currentFileMetadata.file_path;
            const currentFullFileName = currentFileMetadata.file_name;

            const currentFileExtension = await getFileExtname(currentFullFileName);

            let currentFileNameWithoutExtension;
            if (currentFileExtension && currentFileExtension.length > 0) {
                currentFileNameWithoutExtension = currentFullFileName.substring(0, currentFullFileName.length - currentFileExtension.length -1);
            } else {
                currentFileNameWithoutExtension = currentFullFileName;
            }

            const editedFileNameStem = editableMetadata.file_name.trim();

            if (editedFileNameStem !== currentFileNameWithoutExtension) {
                if (!editedFileNameStem) {
                    await message('File name (stem) cannot be empty.', { title: 'Invalid File Name', type: 'error' });
                    isEditing = true;
                    return;
                }

                let nameToSendToBackendRenameService;
                if (currentItemType === 'media' || currentItemType === 'imported_transcript') {
                    nameToSendToBackendRenameService = editedFileNameStem;
                } else {
                    nameToSendToBackendRenameService = currentFileExtension && currentFileExtension.length > 0
                        ? editedFileNameStem + "." + currentFileExtension
                        : editedFileNameStem;
                }

                try {
                    await renameProjectItem(assetKeyForDb, nameToSendToBackendRenameService, currentItemType);
                    isEditing = false;
                    renameProcessed = true;
                } catch (err) {
                    await message(`Error renaming item: ${err.message || err}`, { title: 'Rename Failed', type: 'error' });
                    isEditing = true;
                    return;
                }
            }

            if (!renameProcessed) {
                const originalAssetAbsolutePath = currentFileMetadata.db_absolute_file_path;

                if (!originalAssetAbsolutePath || originalAssetAbsolutePath.trim() === '') {
                    await message('Cannot save: Original asset absolute path could not be determined. Please try reloading the item or checking project integrity.', { title: 'Save Error', type: 'error' });
                    isEditing = true;
                    return;
                }
                if (!originalAssetAbsolutePath.startsWith($project.baseDirectory) && !originalAssetAbsolutePath.startsWith('/') && !/^[a-zA-Z]:\\/.test(originalAssetAbsolutePath)) {
                }

                const metadataPayloadForDb = {
                    file_name: currentFileMetadata.file_name,
                    file_path: originalAssetAbsolutePath,
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
                        assetRelativePath: assetKeyForDb,
                        metadataPayload: metadataPayloadForDb,
                        customFieldsPayload: customFieldsToSaveForDb,
                        assetType: currentItemType
                    });
                    currentFileMetadata.db_absolute_file_path = originalAssetAbsolutePath;
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
                    await message('Metadata saved successfully!', { title: 'Success' });
                    await loadMetadata(assetKeyForDb);

                } catch (err) {
                    await message(`Error saving metadata: ${err}. Please check console.`, { title: 'Save Failed', type: 'error' });
                }
            }
        } catch (err) {
            await message(`An unexpected error occurred: ${err.message || err}.`, { title: 'Error', type: 'error' });
            isEditing = true;
        }
    }

    $: selectedItemPathInStore = $project.selectedDocumentPath || $project.currentImportedTranscriptPath || $project.selectedMediaNotePath;

    let currentItemType = null;

    // Main reactive block for deriving current asset details and loading metadata
    $: {
        (async () => {
            let newOriginalAssetDetails = null;
            let newCurrentRelativePath = null;
            let newCurrentItemType = null;

            const currentSelectedPathFromStore = selectedItemPathInStore;
            console.log('[LIP Reactive] Top: currentSelectedPathFromStore:', currentSelectedPathFromStore, 'Proj ID (from $project.id):', $project?.id, 'BaseDir:', $project?.baseDirectory);

            if (currentSelectedPathFromStore && $project && $project.baseDirectory && typeof $project.id === 'string' && $project.id.trim() !== '') {
                console.log('[LIP Reactive] Main IF condition PASSED.');
                newOriginalAssetDetails = await getOriginalAssetDetails(currentSelectedPathFromStore, $project);
                console.log('[LIP Reactive] newOriginalAssetDetails:', newOriginalAssetDetails);

                if (newOriginalAssetDetails) {
                    newCurrentRelativePath = newOriginalAssetDetails.originalRelativePath;
                    console.log('[LIP Reactive] newCurrentRelativePath:', newCurrentRelativePath, 'previousSelectedItemPath:', previousSelectedItemPath);

                    const originalExt = newOriginalAssetDetails.originalType;

                    if (AUDIO_EXTENSIONS.has(originalExt)) {
                        newCurrentItemType = 'audio';
                    } else if (VIDEO_EXTENSIONS.has(originalExt)) {
                        newCurrentItemType = 'video';
                    } else if (IMAGE_EXTENSIONS.has(originalExt)) {
                        newCurrentItemType = 'image';
                    } else if (originalExt === 'pdf' || originalExt === 'json' || originalExt === 'txt' || originalExt === 'md' || originalExt === 'docx' || originalExt === 'rtf' || originalExt === 'odt') {
                        let isImpTrans = false;
                        if ($project.importedTranscriptFiles && newOriginalAssetDetails.originalAbsolutePath) {
                            for (const f of $project.importedTranscriptFiles) {
                                if (!f.relativePath) continue;
                                try {
                                    const constructedAbsolutePath = await resolve($project.baseDirectory, f.relativePath);
                                    if (normalizePathForComparison(newOriginalAssetDetails.originalAbsolutePath) === normalizePathForComparison(constructedAbsolutePath)) {
                                        isImpTrans = true;
                                        break;
                                    }
                                } catch (e) {
                                    // console.error("[LeftInfoPanel Type Check] Error resolving path for imported transcript check:", e);
                                }
                            }
                        }
                        if (isImpTrans) {
                            newCurrentItemType = 'imported_transcript';
                        } else {
                            newCurrentItemType = 'doc';
                        }
                    } else if (originalExt === 'csv' || originalExt === 'xlsx') {
                        newCurrentItemType = 'table';
                    } else {
                        newCurrentItemType = 'unknown';
                    }
                    if (currentItemType !== newCurrentItemType) {
                        currentItemType = newCurrentItemType;
                    }

                    if (newCurrentRelativePath && newCurrentRelativePath !== previousSelectedItemPath && $project.id) {
                        console.log('[LIP Reactive] Conditions MET to call loadMetadata for path:', newCurrentRelativePath);
                        if (isEditing) {
                            isEditing = false;
                        }
                        currentOriginalAssetDetails = newOriginalAssetDetails;
                        currentAssetRelativePathForGroups = newCurrentRelativePath; // Update for group loading
                        await loadMetadata(newCurrentRelativePath);
                        await fetchFileAssignedGroups($project.id, newCurrentRelativePath); // Fetch groups for the new file
                        previousSelectedItemPath = newCurrentRelativePath;
                    } else if (newCurrentRelativePath && newCurrentRelativePath === previousSelectedItemPath) {
                        // Path is the same, but check if original details or project ID changed, which might warrant a reload of groups if project ID was previously missing.
                        const prevProjectId = $project.id; // Assuming $project.id might have been null before
                        if (JSON.stringify(currentOriginalAssetDetails) !== JSON.stringify(newOriginalAssetDetails) || ($project.id && !prevProjectId)) {
                            currentOriginalAssetDetails = newOriginalAssetDetails;
                            console.log('[LIP Reactive] Updated currentOriginalAssetDetails (content changed, path same).');
                        } else {
                            console.log('[LIP Reactive] Path and details same as previous. No metadata reload.');
                        }
                    } else {
                        console.warn('[LIP Reactive] Conditions NOT MET to call loadMetadata. newCurrentRelativePath:', newCurrentRelativePath, 'previousSelectedItemPath:', previousSelectedItemPath, 'Project ID valid?:', !!$project.id);
                        if (!newCurrentRelativePath && previousSelectedItemPath !== null) {
                             console.log('[LIP Reactive] Path became null, clearing metadata and groups.');
                             currentFileMetadata = null;
                             fullLoadedMetadataObject = null;
                             currentOriginalAssetDetails = null;
                             fileAssignedGroups = []; // Clear groups
                             currentAssetRelativePathForGroups = null;
                             if (isEditing) isEditing = false;
                             previousSelectedItemPath = null;
                        }
                    }
                } else {
                    console.warn('[LIP Reactive] newOriginalAssetDetails is NULL. Clearing metadata and groups. Selected path:', currentSelectedPathFromStore);
                    currentFileMetadata = null;
                    fullLoadedMetadataObject = null;
                    currentOriginalAssetDetails = null;
                    fileAssignedGroups = []; // Clear groups
                    currentAssetRelativePathForGroups = null;
                    if (isEditing) isEditing = false;
                    previousSelectedItemPath = null;
                }
            } else {
                console.warn('[LIP Reactive] Main IF condition FAILED. Clearing metadata and groups. Path:', currentSelectedPathFromStore, 'Proj:', !!$project, 'BaseDir:', !!$project?.baseDirectory, 'ID (from $project.id):', $project?.id);
                if (previousSelectedItemPath !== null || currentFileMetadata !== null) {
                    currentFileMetadata = null;
                    fullLoadedMetadataObject = null;
                    currentOriginalAssetDetails = null;
                    fileAssignedGroups = []; // Clear groups
                    currentAssetRelativePathForGroups = null;
                    if (isEditing) isEditing = false;
                    previousSelectedItemPath = null;
                }
            }
        })();
    }

    // Reactive block to manage editableMetadata and displayableCustomFields
    $: {
        if (currentFileMetadata && $customFieldDefinitionsStore) {
            const assetCustomValues = currentFileMetadata.customFields || [];

            let newEditableCustomFields = [];
            let newDisplayableCustomFields = [];

            for (const def of $customFieldDefinitionsStore) {
                let isApplicable = false;
                if (typeof def.scope === 'string') {
                    if (def.scope.toLowerCase() === 'project') {
                        isApplicable = true;
                    }
                } else if (def.scope && typeof def.scope === 'object') {
                    if (typeof def.scope.AssetType === 'string') {
                        const assetTypeScopeValue = def.scope.AssetType.toLowerCase();
                        if (assetTypeScopeValue === currentItemType) {
                            isApplicable = true;
                        } else if (assetTypeScopeValue === 'media' &&
                                   (currentItemType === 'audio' || currentItemType === 'video')) {
                            isApplicable = true;
                        }
                    }
                }

                if (isApplicable) {
                    const existingAssetField = assetCustomValues.find(cf => cf.key === def.field_key);
                    const valueToUse = existingAssetField?.value ?? def.default_value ?? '';

                    if (isEditing) {
                        newEditableCustomFields.push({
                            key: def.field_key,
                            name: def.field_name,
                            type: def.field_type,
                            value: valueToUse
                        });
                    } else {
                        newDisplayableCustomFields.push({
                            key: def.field_key,
                            name: def.field_name,
                            type: def.field_type,
                            value: valueToUse
                        });
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
            editableMetadata.customFields = [];
        }
    }

</script>

<div class="h-full bg-white dark:bg-gray-800 rounded-md shadow flex flex-col overflow-hidden p-2"
      class:w-full={!$panelStateStore.leftCollapsed}
      class:w-12={$panelStateStore.leftCollapsed} >
    <h2 class="text-sm font-semibold border-b pb-1 border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 flex-shrink-0 flex items-center h-7"
        class:mb-3={!$panelStateStore.leftCollapsed}
        class:mb-0={$panelStateStore.leftCollapsed}
        class:justify-between={!$panelStateStore.leftCollapsed}
        class:justify-center={$panelStateStore.leftCollapsed} >

        <!-- Group for left-aligned items -->
        <div class="flex items-center">
            <button
                on:click={panelStateStore.toggleLeftPanel}
                class="p-1 text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                title={$panelStateStore.leftCollapsed ? 'Expand Metadata' : 'Collapse Metadata'}
            >
                <FileEarmarkCodeIcon class="w-4 h-4"/>
            </button>
            {#if !$panelStateStore.leftCollapsed}
                <span class="ml-2">Metadata</span>
            {/if}
        </div>

        <!-- Group for right-aligned items (or items that will be pushed to the right by justify-between) -->
        {#if !$panelStateStore.leftCollapsed}
            {#if currentFileMetadata}
                <button
                    on:click={toggleEditMode}
                    class="p-1 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                    title={isEditing ? 'Cancel Edit' : 'Edit Metadata'}
                >
                    {@html isEditing ? CANCEL_ICON_SVG : EDIT_ICON_SVG}
                </button>
            {/if} <!-- :else placeholder removed as it's not needed for this layout -->
        {/if}
    </h2>
    <div class="flex-grow overflow-y-auto overflow-x-hidden min-h-0 text-xs relative">
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
                        <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px] break-words">{currentFileMetadata.file_name || 'N/A'}</span>
                    {/if}
                </div>

                <!-- File Path (read-only) -->
                <div class="mb-3">
                    <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">File Path:</label>
                    <span class="text-gray-900 dark:text-gray-100 break-all block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">{currentFileMetadata.file_path || ''}</span>
                </div>

                <!-- Created At (read-only) -->
                <div class="mb-3">
                    <label class="font-semibold text-gray-600 dark:text-gray-400 block mb-1">Created At:</label>
                    <span class="text-gray-900 dark:text-gray-100 block w-full rounded-md border border-gray-300 dark:border-gray-600 px-1.5 py-1 bg-gray-50 dark:bg-gray-700/30 min-h-[30px]">{currentFileMetadata.creation_time ? new Date(currentFileMetadata.creation_time).toLocaleString() : ''}</span>
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

                    <!-- The old creation_time display block under Technical Details is removed -->
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
                    {#if displayableCustomFields.length === 0 && $customFieldDefinitionsStore.filter(def => def.scope?.type === 'Project' || def.scope === 'project' || def.scope === 'Project' || ((def.scope?.type === 'AssetType' && def.scope?.value === currentItemType) || (typeof def.scope === 'string' && def.scope === currentItemType)) ).length > 0}
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
                    {#if editableMetadata.customFields.length === 0 && $customFieldDefinitionsStore.filter(def => def.scope?.type === 'Project' || def.scope === 'project' || def.scope === 'Project' || ((def.scope?.type === 'AssetType' && def.scope?.value === currentItemType) || (typeof def.scope === 'string' && def.scope === currentItemType)) ).length > 0}
                        <p class="text-xs text-gray-500 dark:text-gray-400 italic">No custom fields have values for this item. Edit to add.</p>
                    {/if}
                {/if}
                <!-- End of custom fields rendering -->

                <!-- Groups Section -->
                {#if currentAssetRelativePathForGroups && $project.id && !isEditing}
                    <div class="mt-3">
                        <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 mb-1">Groups</h3>
                        {#if isLoadingFileGroups}
                            <p class="text-xs text-gray-400 dark:text-gray-500 italic">Loading groups...</p>
                        {:else}
                            <GroupMultiSelect
                                fileAssetRelativePath={currentAssetRelativePathForGroups}
                                projectId={$project.id}
                                allProjectGroups={allProjectGroupsForPanel}
                                initiallyAssignedGroups={fileAssignedGroups}
                                on:groupsUpdated={() => fetchFileAssignedGroups($project.id, currentAssetRelativePathForGroups)}
                                on:createNewGroup={() => {
                                    createGroupModalFileToAssign = currentAssetRelativePathForGroups;
                                    isCreateGroupModalOpen = true;
                                }}
                                on:error={(e) => message(e.detail, { title: 'Group Error', type: 'error' })}
                            />
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
                <div class="text-xs w-full text-center truncate border border-gray-300 dark:border-gray-600 px-1 py-0.5 bg-gray-50 dark:bg-gray-700/30 rounded-sm"
                     on:mouseenter={(event) => showLabelTooltip(event, 'File Name', currentFileMetadata?.file_name ?? '')}
                     on:mouseleave={hideLabelTooltip}
                     on:focus={(event) => showLabelTooltip(event, 'File Name', currentFileMetadata?.file_name ?? '')}
                     on:blur={hideLabelTooltip}
                     on:click={() => panelStateStore.toggleLeftPanel()}
                     on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') panelStateStore.toggleLeftPanel(); }}
                     role="button" tabindex="0">Name</div>
                <div class="text-xs w-full text-center truncate border border-gray-300 dark:border-gray-600 px-1 py-0.5 bg-gray-50 dark:bg-gray-700/30 rounded-sm"
                     on:mouseenter={(event) => showLabelTooltip(event, 'File Path', currentFileMetadata?.file_path ?? '')}
                     on:mouseleave={hideLabelTooltip}
                     on:focus={(event) => showLabelTooltip(event, 'File Path', currentFileMetadata?.file_path ?? '')}
                     on:blur={hideLabelTooltip}
                     on:click={() => panelStateStore.toggleLeftPanel()}
                     on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') panelStateStore.toggleLeftPanel(); }}
                     role="button" tabindex="0">Path</div>
                <div class="text-xs w-full text-center truncate border border-gray-300 dark:border-gray-600 px-1 py-0.5 bg-gray-50 dark:bg-gray-700/30 rounded-sm"
                     on:mouseenter={(event) => showLabelTooltip(event, 'Last Modified', currentFileMetadata?.last_modified ? new Date(currentFileMetadata.last_modified).toLocaleString() : '')}
                     on:mouseleave={hideLabelTooltip}
                     on:focus={(event) => showLabelTooltip(event, 'Last Modified', currentFileMetadata?.last_modified ? new Date(currentFileMetadata.last_modified).toLocaleString() : '')}
                     on:blur={hideLabelTooltip}
                     on:click={() => panelStateStore.toggleLeftPanel()}
                     on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') panelStateStore.toggleLeftPanel(); }}
                     role="button" tabindex="0">Date</div>
                <div class="text-xs w-full text-center truncate border border-gray-300 dark:border-gray-600 px-1 py-0.5 bg-gray-50 dark:bg-gray-700/30 rounded-sm"
                     on:mouseenter={(event) => showLabelTooltip(event, 'Title', currentFileMetadata?.title ?? '')}
                     on:mouseleave={hideLabelTooltip}
                     on:focus={(event) => showLabelTooltip(event, 'Title', currentFileMetadata?.title ?? '')}
                     on:blur={hideLabelTooltip}
                     on:click={() => panelStateStore.toggleLeftPanel()}
                     on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') panelStateStore.toggleLeftPanel(); }}
                     role="button" tabindex="0">Title</div>
                <div class="text-xs w-full text-center truncate border border-gray-300 dark:border-gray-600 px-1 py-0.5 bg-gray-50 dark:bg-gray-700/30 rounded-sm"
                     on:mouseenter={(event) => showLabelTooltip(event, 'Description', currentFileMetadata?.description ?? '')}
                     on:mouseleave={hideLabelTooltip}
                     on:focus={(event) => showLabelTooltip(event, 'Description', currentFileMetadata?.description ?? '')}
                     on:blur={hideLabelTooltip}
                     on:click={() => panelStateStore.toggleLeftPanel()}
                     on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') panelStateStore.toggleLeftPanel(); }}
                     role="button" tabindex="0">Desc</div>
                <div class="text-xs w-full text-center truncate border border-gray-300 dark:border-gray-600 px-1 py-0.5 bg-gray-50 dark:bg-gray-700/30 rounded-sm"
                     on:mouseenter={(event) => showLabelTooltip(event, 'Summary', currentFileMetadata?.summary ?? '')}
                     on:mouseleave={hideLabelTooltip}
                     on:focus={(event) => showLabelTooltip(event, 'Summary', currentFileMetadata?.summary ?? '')}
                     on:blur={hideLabelTooltip}
                     on:click={() => panelStateStore.toggleLeftPanel()}
                     on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') panelStateStore.toggleLeftPanel(); }}
                     role="button" tabindex="0">Summ</div>
            </div>
        {/if}
    </div>
    
</div>

<CategoryTooltip
    bind:visible={labelTooltipVisible}
    categoryName={labelTooltipTitle}
    files={[{ name: labelTooltipText }]}
    x={labelTooltipX}
    y={labelTooltipY}
/>

<AddFieldModal bind:showModal={showAddFieldModal} currentItemType={currentItemType} on:close={() => showAddFieldModal = false} />

{#if isCreateGroupModalOpen && $project.id}
    <CreateGroupModal
        bind:showModal={isCreateGroupModalOpen}
        projectUuid={$project.id}
        fileToAdd={createGroupModalFileToAssign ? { relativePath: createGroupModalFileToAssign, name: '' } : null}
        on:groupCreated={async (event) => {
            isCreateGroupModalOpen = false;
            await fetchAllProjectGroups(); // Refresh all project groups
            if (event.detail.group && createGroupModalFileToAssign) {
                 // If fileToAssignOnCreate was used by modal, or if we need to ensure it's assigned
                 // For now, rely on GroupMultiSelect's on:groupsUpdated or an explicit fetch after adding
                 await fetchFileAssignedGroups($project.id, createGroupModalFileToAssign);
            } else if (event.detail.group && !createGroupModalFileToAssign) {
                // Just a group was created, no specific file to assign from here,
                // but still refresh assigned groups for current file in case it was auto-assigned by modal, or for consistency
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