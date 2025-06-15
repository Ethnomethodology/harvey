<!-- src/lib/components/projectview/notes/shared_panels/LeftInfoPanel.svelte -->
<script>
    import { onMount, onDestroy } from 'svelte';
    import { project } from '$lib/stores/projectStore.js';
    import { invoke } from '@tauri-apps/api/core';
    // fsRename might still be used by projectService.js, direct fs calls for metadata are removed.
    import { basename, extname as getFileExtname, sep as getPathSep, resolve } from '@tauri-apps/api/path'; // Added resolve
    import { confirm, message } from '@tauri-apps/plugin-dialog';
    import { renameProjectItem } from '$lib/services/projectService.js';
    import AddFieldModal from '$lib/components/projectview/modals/AddFieldModal.svelte';
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
                // console.debug(`[LeftInfoPanel] User confirmed deletion for fieldKey: ${fieldKey}`); // Removed
                await deleteDefinition(fieldKey); // This already calls loadAllDefinitions
                await message('Custom field definition deleted successfully.', { title: 'Success' });
            } catch (error) {
                // console.error(`[LeftInfoPanel] Error deleting custom field definition ${fieldKey}:`, error); // Removed
                await message(`Failed to delete custom field: ${error.message || error}`, { title: 'Error', type: 'error' });
            }
        } else {
            // console.debug(`[LeftInfoPanel] User cancelled deletion for fieldKey: ${fieldKey}`); // Removed
        }
    }

    // Helper function to get details of the original asset
    async function getOriginalAssetDetails(selectedPath, projectStore) {
        if (!selectedPath || !projectStore || !projectStore.baseDirectory) {
            // console.warn('[LeftInfoPanel] getOriginalAssetDetails: Missing selectedPath or projectStore data.'); // Kept as warn // Removed
            const fallbackName = selectedPath ? await basename(selectedPath) : 'Unknown.file';
            return {
                originalRelativePath: selectedPath, // Fallback, might not be relative
                originalAbsolutePath: selectedPath,
                originalType: fallbackName.includes('.') ? fallbackName.substring(fallbackName.lastIndexOf('.') + 1) : 'unknown',
                originalFileName: fallbackName,
                isView: false
            };
        }

        const pathSep = projectStore.baseDirectory.includes('/') ? '/' : '\\';
        let originalRelativePath = selectedPath.startsWith(projectStore.baseDirectory)
            ? selectedPath.substring(projectStore.baseDirectory.length + (selectedPath.startsWith(projectStore.baseDirectory + pathSep) ? 1 : 0))
            : selectedPath; // Fallback if not under base dir (should not happen for project files)
        originalRelativePath = originalRelativePath.replace(/\\/g, '/'); // Normalize to forward slashes

        let originalAbsolutePath = selectedPath;
        let originalFileName = await basename(selectedPath);
        let originalType = await getFileExtname(originalFileName).then(ext => ext ? ext.toLowerCase() : 'unknown');
        let isView = false;

        const selectedFileExt = await getFileExtname(selectedPath).then(ext => ext ? ext.toLowerCase() : '');

        if (selectedFileExt === 'json') {
            // This JSON might be a view of another document (e.g., a .docx file)
            const selectedFileNameStem = originalFileName.substring(0, originalFileName.length - (selectedFileExt.length + 1));
            const potentialOriginalExtensions = ['docx', 'pdf', 'txt', 'md']; // Add more as needed

            // Check $project.documentFiles
            if (projectStore.documentFiles && Array.isArray(projectStore.documentFiles)) {
                for (const docExt of potentialOriginalExtensions) {
                    const potentialOriginalFileName = `${selectedFileNameStem}.${docExt}`;
                    // Search for this file in projectStore.documentFiles
                    // documentFiles usually store relative paths
                    for (const docFile of projectStore.documentFiles) {
                        if (docFile.name === potentialOriginalFileName) {
                            // Found a potential original document
                            const resolvedOriginalAbsolutePath = await resolve(projectStore.baseDirectory, docFile.relativePath);
                            const normalizedSelectedPath = normalizePathForComparison(selectedPath);
                            const normalizedViewPathForOriginal = normalizePathForComparison(await resolve(projectStore.baseDirectory, `${docFile.relativePath}.json`)); // Assuming view is named original.ext.json

                            // Heuristic: if the selected JSON path is what a view of this docFile would be named
                            // This check needs refinement. For now, we assume if a docx has a json with same stem, it's the view.
                            // A more robust way would be if the JSON file itself has a pointer to its original.
                            // Or, if the selected JSON file's name matches "<original_stem>.json" and an "<original_stem>.<docExt>" exists.
                            // The current check is more direct: is there a docFile whose name matches what we'd expect for an original of this JSON?
                            // Let's refine the condition for identifying the "view" scenario.
                            // If selectedPath is "/path/to/file.docx.json" and we find "file.docx"
                            // A common pattern for derived files is adding an extension, e.g., original.docx -> original.docx.json
                            // Or, original.docx -> original.json (if in different dir or if only one representation is kept in the same dir)

                            // For this specific task, the problem states: "a JSON representation of a DOCX file"
                            // e.g. selectedItemPathInStore points to a .json file that is a view of an original document (e.g., a .docx file)
                            // This implies the JSON file might be, for example, "MyDoc.json" as a view for "MyDoc.docx".

                            // Let's assume the JSON view has the same stem as the original.
                            const currentJsonStem = selectedFileNameStem;
                            const originalDocStem = docFile.name.substring(0, docFile.name.lastIndexOf('.'));

                            if (currentJsonStem === originalDocStem) {
                                originalRelativePath = docFile.relativePath.replace(/\\/g, '/');
                                originalAbsolutePath = await resolve(projectStore.baseDirectory, docFile.relativePath);
                                originalFileName = docFile.name;
                                originalType = await getFileExtname(originalFileName).then(ext => ext ? ext.toLowerCase() : 'unknown');
                                isView = true;
                                // console.debug(`[LeftInfoPanel] getOriginalAssetDetails: Identified original asset for JSON view: ${originalFileName} (Rel: ${originalRelativePath})`); // Downgraded // Removed
                                break; // Found original, break from docFile loop
                            }
                        }
                    }
                    if (isView) break; // Found original, break from potentialOriginalExtensions loop
                }
            }
            // TODO: Could also check $project.files for media if similar pattern applies.
        }

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

    let currentFileMetadata = null; // Will store FileMetadata like structure, including customFields
    let fullLoadedMetadataObject = null; // May store the raw object from DB (FileMetadataWithCustomFieldsFromDb) or be refactored/removed
    let currentOriginalAssetDetails = null; // Will store { originalRelativePath, originalAbsolutePath, originalType, originalFileName, isView }
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
        let normalized = p.replace(/\\/g, '/'); // Convert all backslashes to forward slashes
        normalized = normalized.replace(/\/\/{2,}/g, '/'); // Collapse multiple (2 or more) forward slashes into one
        return normalized;
    }

    const EDIT_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-pencil-square" viewBox="0 0 16 16"><path d="M15.502 1.94a.5.5 0 0 1 0 .706L14.459 3.69l-2-2L13.502.646a.5.5 0 0 1 .707 0l1.293 1.293zm-1.75 2.456-2-2L4.939 9.21a.5.5 0 0 0-.121.196l-.805 2.414a.25.25 0 0 0 .316.316l2.414-.805a.5.5 0 0 0 .196-.12l6.813-6.814z"/><path fill-rule="evenodd" d="M1 13.5A1.5 1.5 0 0 0 2.5 15h11a1.5 1.5 0 0 0 1.5-1.5v-6a.5.5 0 0 0-1 0v6a.5.5 0 0 1-.5.5h-11a.5.5 0 0 1-.5-.5v-11a.5.5 0 0 1 .5-.5H9a.5.5 0 0 0 0-1H2.5A1.5 1.5 0 0 0 1 2.5z"/></svg>`;
    const CANCEL_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-x-circle" viewBox="0 0 16 16"><path d="M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14m0 1A8 8 0 1 0 8 0a8 8 0 0 0 0 16"/><path d="M4.646 4.646a.5.5 0 0 1 .708 0L8 7.293l2.646-2.647a.5.5 0 0 1 .708.708L8.707 8l2.647 2.646a.5.5 0 0 1-.708.708L8 8.707l-2.646 2.647a.5.5 0 0 1-.708-.708L7.293 8 4.646 5.354a.5.5 0 0 1 0-.708"/></svg>`;

    const AUDIO_EXTENSIONS = new Set(['mp3','wav','m4a','ogg','aac','flac']);
    const VIDEO_EXTENSIONS = new Set(['mp4','mov','avi','mkv','webm']);
    const IMAGE_EXTENSIONS = new Set(['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'tiff']);

    let previousSelectedItemPath = null;
    let displayableCustomFields = []; // For read mode

    onMount(async () => {
        // console.debug('[LeftInfoPanel] Mounted.'); // Downgraded // Removed
        previousSelectedItemPath = null;
        // console.debug('[LeftInfoPanel onMount] Initial selectedItemPathInStore (at mount):', selectedItemPathInStore); // Downgraded // Removed
        try {
            // console.debug('[LeftInfoPanel onMount] Loading all custom field definitions...'); // Downgraded // Removed
            await loadAllDefinitions();
            // console.info('[LeftInfoPanel onMount] Custom field definitions loaded.'); // Kept as info - important one-time setup // Removed
        } catch (error) {
            // console.error('[LeftInfoPanel onMount] Error loading custom field definitions:', error); // Keep as error // Removed
            message(`Error loading custom field definitions: ${error.message || error}`, { title: 'Error', type: 'error' }); // Keep for user
        }
    });

    async function loadMetadata(assetRelativePath) { // assetRelativePath is the original relative path
        currentFileMetadata = null;
        fullLoadedMetadataObject = null;
        if (isEditing && assetRelativePath !== previousSelectedItemPath) { // previousSelectedItemPath now stores relative path
            isEditing = false;
        }

        if (!assetRelativePath) {
            // console.warn('[LeftInfoPanel] loadMetadata called with no assetRelativePath.'); // Keep as warn // Removed
            return;
        }

        try {
            // console.debug(`[LeftInfoPanel] Loading metadata from DB for relative path: ${assetRelativePath}`); // Downgraded // Removed
            const result = await invoke('get_asset_metadata_command', { assetRelativePath: assetRelativePath });

            if (result) {
                // result is FileMetadataWithCustomFieldsFromDb
                // assetRelativePath is the original relative path
                // currentOriginalAssetDetails should be up-to-date due to the reactive chain
                const originalFileNameToUse = currentOriginalAssetDetails?.originalFileName || await basename(assetRelativePath);
                const originalAbsolutePathToUse = currentOriginalAssetDetails?.originalAbsolutePath || ($project.baseDirectory ? `${$project.baseDirectory}${getPathSep}${assetRelativePath}` : assetRelativePath);

                currentFileMetadata = {
                    file_name: originalFileNameToUse,       // Use original filename from currentOriginalAssetDetails
                    file_path: assetRelativePath,           // This is original relative path (DB key)
                    // Prioritize freshly derived originalAbsolutePath. Fallback to result.file_path if currentOriginalAssetDetails is somehow not fully populated.
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
                    // Ensure this also uses the definitive original absolute path for its 'file_path' field, consistent with currentFileMetadata
                    metadata: { ...currentFileMetadata, file_path: currentOriginalAssetDetails?.originalAbsolutePath || result.file_path },
                    customFields: currentFileMetadata.customFields,
                    asset_type: currentItemType, // Use currentItemType which is already based on original asset
                    version: "db_1.0"
                };
                // console.debug('[LeftInfoPanel] Metadata loaded from DB for:', assetRelativePath); // Downgraded, removed full object log // Removed
            } else {
                // console.warn('[LeftInfoPanel] No metadata found in DB for:', assetRelativePath); // Keep as warn // Removed
                // assetRelativePath is original relative path
                const originalFileNameToUse = currentOriginalAssetDetails?.originalFileName || await basename(assetRelativePath);
                const originalAbsolutePathToUse = currentOriginalAssetDetails?.originalAbsolutePath || ($project.baseDirectory ? `${$project.baseDirectory}${getPathSep}${assetRelativePath}` : assetRelativePath);

                currentFileMetadata = {
                    file_name: originalFileNameToUse,
                    file_path: assetRelativePath,        // Original relative path
                    db_absolute_file_path: originalAbsolutePathToUse, // Original absolute path
                    last_modified: new Date().toISOString(),
                    title: '', description: '', summary: '', customFields: [],
                    duration_seconds: null, width: null, height: null, frame_rate: null, bit_rate: null, audio_codec: null, video_codec: null, creation_time: null
                };
                fullLoadedMetadataObject = { metadata: { ...currentFileMetadata }, customFields: [], version: "db_1.0_new" }; // Here, currentFileMetadata.db_absolute_file_path is originalAbsolutePathToUse
            }
        } catch (error) {
            // console.error(`[LeftInfoPanel] Error loading metadata from DB for ${assetRelativePath}:`, error); // Removed
            const originalFileNameToUse = currentOriginalAssetDetails?.originalFileName || await basename(assetRelativePath || 'Unknown.file').catch(() => 'Unknown.file');
            const originalAbsolutePathToUse = currentOriginalAssetDetails?.originalAbsolutePath || ($project.baseDirectory && assetRelativePath ? `${$project.baseDirectory}${getPathSep}${assetRelativePath}` : assetRelativePath || '');

            currentFileMetadata = {
                file_name: originalFileNameToUse,       // Original filename
                file_path: assetRelativePath || '',     // Original relative path
                db_absolute_file_path: originalAbsolutePathToUse, // Original absolute path
                last_modified: new Date().toISOString(),
                title: '', description: '', summary: '', customFields: [],
                duration_seconds: null, width: null, height: null, frame_rate: null, bit_rate: null, audio_codec: null, video_codec: null, creation_time: null
            };
            fullLoadedMetadataObject = { metadata: { ...currentFileMetadata }, customFields: [], version: "db_1.0_error" };  // Here, currentFileMetadata.db_absolute_file_path is originalAbsolutePathToUse
            await message(`Error loading metadata: ${error}`, { title: 'Load Error', type: 'error' }); // Keep for user
        }

        // Fallback if currentFileMetadata is still null after try-catch
        if (!currentFileMetadata) { // This block might be redundant if currentOriginalAssetDetails is guaranteed to be set before loadMetadata is called.
            // console.warn('[LeftInfoPanel] currentFileMetadata is null after load attempt, creating fallback structure for:', assetRelativePath); // Keep as warn // Removed
            const originalFileNameToUse = currentOriginalAssetDetails?.originalFileName || await basename(assetRelativePath || 'Unknown.file').catch(() => 'Unknown.file');
            const originalAbsolutePathToUse = currentOriginalAssetDetails?.originalAbsolutePath || ($project.baseDirectory && assetRelativePath ? `${$project.baseDirectory}${getPathSep}${assetRelativePath}` : assetRelativePath || '');

            currentFileMetadata = {
                file_name: originalFileNameToUse,       // Original filename
                file_path: assetRelativePath || '',     // Original relative path
                db_absolute_file_path: originalAbsolutePathToUse, // Original absolute path
                last_modified: new Date().toISOString(),
                title: '', description: '', summary: '', customFields: [],
                duration_seconds: null, width: null, height: null, frame_rate: null, bit_rate: null, audio_codec: null, video_codec: null, creation_time: null
            };
            if (isEditing) isEditing = false;
        }
    }

    function toggleEditMode() {
        // console.debug('[LeftInfoPanel] toggleEditMode called. isEditing before:', isEditing); // Downgraded // Removed
        isEditing = !isEditing;
        // console.debug('[LeftInfoPanel] isEditing after:', isEditing); // Downgraded // Removed
    }

    async function handleSaveMetadata() {
        // console.debug('[LeftInfoPanel] handleSaveMetadata called.'); // Downgraded // Removed
        let renameProcessed = false;
        if (!currentFileMetadata || !currentFileMetadata.file_path) { // file_path is the relative path (DB key)
            // console.error('[LeftInfoPanel] Save error: Missing file_path in currentFileMetadata.'); // Keep as error // Removed
            await message('Cannot save: File path information is missing.', { title: 'Save Error', type: 'error' }); // Keep for user
            return;
        }
        if (!currentItemType) {
            // console.error('[LeftInfoPanel] Save error: currentItemType is not set.'); // Keep as error // Removed
            await message('Cannot save: Item type is unknown.', { title: 'Save Error', type: 'error' }); // Keep for user
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
                    // console.log(`[LeftInfoPanel] Calling renameProjectItem service: path=${assetKeyForDb}, nameToSend=${nameToSendToBackendRenameService}, type=${currentItemType}`); // Removed
                    // renameProjectItem service is expected to handle renaming in DB (key and fields) and filesystem, then update XML.
                    // The store update from XML change should trigger a reactive reload of metadata.
                    await renameProjectItem(assetKeyForDb, nameToSendToBackendRenameService, currentItemType);

                    // console.log('[LeftInfoPanel] Rename successful via renameProjectItem. Store update should handle metadata reload.'); // Removed
                    isEditing = false;
                    renameProcessed = true;
                    // Important: After a successful rename, currentFileMetadata might be stale if the path/key changed.
                    // The reactive flow an
                    // d loadMetadata are expected to pick up the new path from the store and reload.
                    // So, we might not need to save further metadata fields in this same execution path if rename occurred.
                    // The user would effectively save title/desc *after* the rename is committed and UI reloads.
                    // For now, if renameProcessed is true, we skip the direct metadata save part below.
                } catch (err) {
                    // console.error(`[LeftInfoPanel] renameProjectItem failed:`, err); // Removed
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
                // currentFileMetadata.file_path is the original relative path (assetKeyForDb)
                // currentFileMetadata.db_absolute_file_path should be the original absolute path,
                // populated correctly by loadMetadata from currentOriginalAssetDetails or from DB record.
                const originalAssetAbsolutePath = currentFileMetadata.db_absolute_file_path;

                if (!originalAssetAbsolutePath || originalAssetAbsolutePath.trim() === '') {
                    // console.error('[LeftInfoPanel] Save error: Original asset absolute path is missing or empty in currentFileMetadata.db_absolute_file_path.'); // Keep as error // Removed
                    // console.error('[LeftInfoPanel] currentFileMetadata details:', JSON.stringify(currentFileMetadata)); // Removed verbose object log
                    // console.error('[LeftInfoPanel] currentOriginalAssetDetails:', JSON.stringify(currentOriginalAssetDetails)); // Removed verbose object log
                    await message('Cannot save: Original asset absolute path could not be determined. Please try reloading the item or checking project integrity.', { title: 'Save Error', type: 'error' }); // Keep for user
                    isEditing = true; // Keep editing mode
                    return; // Do not proceed
                }

                // Further check: Ensure it looks like an absolute path (simple check, might need OS-specifics for robustness)
                // For this project, paths usually start with $project.baseDirectory or are fully qualified.
                // A simple check is if it contains the project's base directory, or starts with '/' or a drive letter e.g. C:\
                // This is a basic sanity check. `resolve` in `getOriginalAssetDetails` should ensure it's absolute.
                if (!originalAssetAbsolutePath.startsWith($project.baseDirectory) && !originalAssetAbsolutePath.startsWith('/') && !/^[a-zA-Z]:\\/.test(originalAssetAbsolutePath)) {
                     // console.warn(`[LeftInfoPanel] Save warning: The determined absolute path "${originalAssetAbsolutePath}" might not be truly absolute.`); // Keep as warn // Removed
                }

                const metadataPayloadForDb = {
                    file_name: currentFileMetadata.file_name, // This is originalFileName
                    file_path: originalAssetAbsolutePath,     // This is originalAbsolutePath
                    last_modified: new Date().toISOString(),
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

                // console.debug('[LeftInfoPanel] Save Details - Key:', assetKeyForDb, 'AbsPath:', originalAssetAbsolutePath, 'Type:', currentItemType); // Downgraded critical checks // Removed
                // console.debug('[LeftInfoPanel] Save Payload - Metadata:', JSON.stringify(metadataPayloadForDb)); // Downgraded verbose object log // Removed
                // console.debug('[LeftInfoPanel] Save Payload - Custom Fields:', JSON.stringify(customFieldsToSaveForDb)); // Downgraded verbose object log // Removed


                try {
                    await invoke('update_asset_metadata_command', {
                        projectXmlPathStr: $project.xmlPath, // Added this line
                        assetRelativePath: assetKeyForDb,       // Original relative path (key for DB lookup)
                        metadataPayload: metadataPayloadForDb,  // Contains original absolute path in its file_path field
                        customFieldsPayload: customFieldsToSaveForDb,
                        assetType: currentItemType              // Original asset type
                    });

                    // Update local state to reflect saved data
                    // currentFileMetadata.db_absolute_file_path should already be originalAssetAbsolutePath
                    // If the backend potentially changes the path upon update (e.g. sanitization), the response should provide it.
                    // For now, assume it remains originalAssetAbsolutePath.
                    currentFileMetadata.db_absolute_file_path = originalAssetAbsolutePath;
                    currentFileMetadata.title = metadataPayloadForDb.title;
                    currentFileMetadata.description = metadataPayloadForDb.description;
                    currentFileMetadata.summary = metadataPayloadForDb.summary;
                    currentFileMetadata.last_modified = metadataPayloadForDb.last_modified; // Reflect new save time
                    currentFileMetadata.customFields = JSON.parse(JSON.stringify(customFieldsToSaveForDb));

                    if (fullLoadedMetadataObject) {
                        fullLoadedMetadataObject.metadata = { ...currentFileMetadata, file_path: metadataPayloadForDb.file_path }; // Ensure full object also reflects absolute path in its 'metadata.file_path'
                        fullLoadedMetadataObject.customFields = currentFileMetadata.customFields;
                    }
                    isEditing = false;
                    await message('Metadata saved successfully!', { title: 'Success' }); // Keep for user

                    // Explicitly reload metadata to refresh UI with any backend-derived changes (e.g. last_modified)
                    // assetKeyForDb is the original relative path, which is what loadMetadata expects
                    // console.info('[LeftInfoPanel] Explicitly reloading metadata after save for:', assetKeyForDb); // Kept as info // Removed
                    await loadMetadata(assetKeyForDb);

                } catch (err) {
                    // console.error('[LeftInfoPanel] Error saving metadata to DB:', err); // Keep as error // Removed
                    await message(`Error saving metadata: ${err}. Please check console.`, { title: 'Save Failed', type: 'error' }); // Keep for user
                    // isEditing = true; // Optionally keep editing mode
                }
            }
        } catch (err) {
            // console.error('[LeftInfoPanel] General error in handleSaveMetadata:', err); // Keep as error // Removed
            await message(`An unexpected error occurred: ${err.message || err}.`, { title: 'Error', type: 'error' }); // Keep for user
            isEditing = true;
        }
    }

    $: selectedItemPathInStore = $project.selectedDocumentPath || $project.currentImportedTranscriptPath || $project.selectedMediaNotePath;

    let currentItemType = null; // This will now be derived from currentOriginalAssetDetails.originalType primarily

    // Main reactive block for deriving current asset details and loading metadata
    $: {
        // Log selectedItemPathInStore changes directly in the reaction if needed, or rely on onMount + path change logs

        // Variables newCurrentRelativePath and newCurrentItemType were previously declared here.
        // They need to be inside the async IIFE or passed if their values before await are important.
        // Based on the logic, they are determined *after* `await getOriginalAssetDetails`, so their declaration
        // should be inside the async IIFE.

        (async () => {
            // CORRECTED: Declare variables at the top of the async function scope
            let newOriginalAssetDetails = null;
            let newCurrentRelativePath = null;
            let newCurrentItemType = null; // Was `newType` in the erroneous example, matching to actual code.

            const currentSelectedPathFromStore = selectedItemPathInStore; // Capture for this async operation
            // console.debug('[LeftInfoPanel Reactive] selectedItemPathInStore is now:', currentSelectedPathFromStore); // Removed

            if (currentSelectedPathFromStore && $project && $project.baseDirectory) {
                newOriginalAssetDetails = await getOriginalAssetDetails(currentSelectedPathFromStore, $project); // Assign to declared variable
                // console.debug('[LeftInfoPanel Reactive] newOriginalAssetDetails determined:', newOriginalAssetDetails); // Removed

                if (newOriginalAssetDetails) {
                    newCurrentRelativePath = newOriginalAssetDetails.originalRelativePath; // Assign to declared variable
                    // console.debug('[LeftInfoPanel Reactive] originalRelativePath derived is:', newCurrentRelativePath); // Removed
                    const originalExt = newOriginalAssetDetails.originalType;

                    // --- Start of new currentItemType derivation ---
                    if (AUDIO_EXTENSIONS.has(originalExt)) {
                        newCurrentItemType = 'audio'; // Assign to declared variable
                    } else if (VIDEO_EXTENSIONS.has(originalExt)) {
                        newCurrentItemType = 'video'; // Assign to declared variable
                    } else if (IMAGE_EXTENSIONS.has(originalExt)) {
                        newCurrentItemType = 'image'; // Assign to declared variable
                    } else if (originalExt === 'pdf' || originalExt === 'json' || originalExt === 'txt' || originalExt === 'md' || originalExt === 'docx' || originalExt === 'rtf' || originalExt === 'odt') {
                        let isImpTrans = false;
                        if ($project.importedTranscriptFiles && newOriginalAssetDetails.originalAbsolutePath) { // Uses newOriginalAssetDetails
                            for (const f of $project.importedTranscriptFiles) {
                                if (!f.relativePath) continue;
                                try {
                                    const constructedAbsolutePath = await resolve($project.baseDirectory, f.relativePath);
                                    if (normalizePathForComparison(newOriginalAssetDetails.originalAbsolutePath) === normalizePathForComparison(constructedAbsolutePath)) { // Uses newOriginalAssetDetails
                                        isImpTrans = true;
                                        break;
                                    }
                                } catch (e) {
                                    // console.error("[LeftInfoPanel Type Check] Error resolving path for imported transcript check:", e); // Removed
                                }
                            }
                        }
                        if (isImpTrans) {
                            newCurrentItemType = 'imported_transcript'; // Assign to declared variable
                        } else {
                            newCurrentItemType = 'doc'; // Assign to declared variable
                        }
                    } else if (originalExt === 'csv' || originalExt === 'xlsx') {
                        newCurrentItemType = 'table'; // Assign to declared variable
                    } else {
                        newCurrentItemType = 'unknown'; // Assign to declared variable
                    }
                    // --- End of new currentItemType derivation ---
                    if (currentItemType !== newCurrentItemType) { // Compare with component-level currentItemType
                        currentItemType = newCurrentItemType; // Update component-level currentItemType
                         // console.debug(`[LeftInfoPanel Reactive] currentItemType updated to: ${currentItemType}`); // Removed
                    }


                    if (newCurrentRelativePath && newCurrentRelativePath !== previousSelectedItemPath) {
                        // console.info(`[LeftInfoPanel Reactive] Path changed FROM '${previousSelectedItemPath}' TO '${newCurrentRelativePath}'. Triggering metadata load.`); // Kept as info // Removed
                        if (isEditing) {
                            // console.debug('[LeftInfoPanel Reactive] Resetting isEditing to false due to path change.'); // Removed
                            isEditing = false;
                        }
                        currentOriginalAssetDetails = newOriginalAssetDetails;
                        await loadMetadata(newCurrentRelativePath);
                        previousSelectedItemPath = newCurrentRelativePath;
                    } else if (!newCurrentRelativePath && previousSelectedItemPath !== null) {
                        // console.info(`[LeftInfoPanel Reactive] Path became null (was '${previousSelectedItemPath}'). Resetting metadata.`); // Removed
                        currentFileMetadata = null;
                        fullLoadedMetadataObject = null;
                        currentOriginalAssetDetails = null;
                        currentItemType = null;
                        if (isEditing) isEditing = false;
                        previousSelectedItemPath = null;
                    } else if (newCurrentRelativePath && newCurrentRelativePath === previousSelectedItemPath) {
                        if (JSON.stringify(currentOriginalAssetDetails) !== JSON.stringify(newOriginalAssetDetails)) {
                             currentOriginalAssetDetails = newOriginalAssetDetails;
                             // console.debug('[LeftInfoPanel Reactive] Updated currentOriginalAssetDetails as content changed but path remained same.'); // Removed
                        }
                    }
                } else {
                    // console.warn(`[LeftInfoPanel Reactive] getOriginalAssetDetails returned null/undefined for ${currentSelectedPathFromStore}.`); // Removed
                    if (previousSelectedItemPath !== null) {
                        currentFileMetadata = null;
                        fullLoadedMetadataObject = null;
                        currentOriginalAssetDetails = null;
                        currentItemType = null;
                        if (isEditing) isEditing = false;
                        previousSelectedItemPath = null;
                    }
                }
            } else if (!currentSelectedPathFromStore && previousSelectedItemPath !== null) {
                // console.info(`[LeftInfoPanel Reactive] No item selected (currentSelectedPathFromStore is null). Resetting metadata.`); // Kept as info // Removed
                currentFileMetadata = null;
                fullLoadedMetadataObject = null;
                currentOriginalAssetDetails = null;
                currentItemType = null;
                if (isEditing) isEditing = false;
                previousSelectedItemPath = null;
            }
        })();
    } // End of main reactive block

    // Remove or comment out the old currentItemType derivation block, as it's now handled within the main reactive block
    /*
    $: if (selectedItemPathInStore && $project.baseDirectory) {
        // ... old logic ...
    } else {
        // ... old logic ...
    }
    */

    // Reactive block to manage editableMetadata and displayableCustomFields based on definitions and current asset data
    $: {
        if (currentFileMetadata && $customFieldDefinitionsStore) {
            const assetCustomValues = currentFileMetadata.customFields || []; // These are {key, value, type} from asset's JSON

            let newEditableCustomFields = [];
            let newDisplayableCustomFields = [];

            // console.debug('[LeftInfoPanel CustomFieldsBlock] Running. Definitions count:', $customFieldDefinitionsStore.length, 'isEditing:', isEditing, 'currentItemType:', currentItemType); // Removed
            // console.debug('[LeftInfoPanel CustomFieldsBlock] Store content (first item):', JSON.stringify($customFieldDefinitionsStore.length > 0 ? $customFieldDefinitionsStore[0] : "Empty store")); // Removed
            for (const def of $customFieldDefinitionsStore) {
                // if (def.field_key === 'only_audio' || def.field_key === 'cod' || def.field_key === 'only_tables' || def.field_key === 'cat' || def.field_key === 'only_for_docs' || def.field_key === 'available_across_project' || def.field_key === 'test235' || def.field_key === 'date_added') { // Removed
                    // console.debug(`[LeftInfoPanel CustomFieldsBlock] Definition for key '${def.field_key}':`, JSON.stringify(def)); // Removed
                // } // Removed
                // Determine if the definition is applicable by scope
                let isApplicable = false; // Default to false
                if (typeof def.scope === 'string') {
                    // Handles project scope when it's a string like "Project" (case-insensitive)
                    if (def.scope.toLowerCase() === 'project') {
                        isApplicable = true;
                    }
                } else if (def.scope && typeof def.scope === 'object') { // Handles object scopes if def.scope is a non-null object
                    // Check for asset-type specific scopes like { "AssetType": "image" }
                    if (typeof def.scope.AssetType === 'string') {
                        const assetTypeScopeValue = def.scope.AssetType.toLowerCase();
                        if (assetTypeScopeValue === currentItemType) { // currentItemType is already lowercase
                            isApplicable = true;
                        } else if (assetTypeScopeValue === 'media' &&
                                   (currentItemType === 'audio' || currentItemType === 'video')) {
                            // Backward compatibility: "media" scope applies to new "audio" and "video" currentItemTypes
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
                            name: def.field_name, // Display name from definition
                            type: def.field_type, // Type from definition
                            value: valueToUse
                        });
                    } else { // This is for read mode (i.e., !isEditing)
                        newDisplayableCustomFields.push({
                            key: def.field_key,
                            name: def.field_name,
                            type: def.field_type,
                            value: valueToUse
                        });
                    }
                }
            }
            // Sort fields alphabetically by name for consistent display
            newEditableCustomFields.sort((a, b) => a.name.localeCompare(b.name));
            // console.debug('[LeftInfoPanel CustomFieldsBlock] newEditableCustomFields populated. Count:', newEditableCustomFields.length); // Removed
            newDisplayableCustomFields.sort((a, b) => a.name.localeCompare(b.name));
            // console.debug('[LeftInfoPanel CustomFieldsBlock] newDisplayableCustomFields populated. Count:', newDisplayableCustomFields.length); // Removed

            editableMetadata.customFields = newEditableCustomFields;
            displayableCustomFields = newDisplayableCustomFields;
        } else {
            editableMetadata.customFields = [];
            displayableCustomFields = [];
        }

        // Populate standard metadata fields for editing
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
            // When exiting edit mode, clear all editable fields
            editableMetadata.file_name = '';
            editableMetadata.title = '';
            editableMetadata.description = '';
            editableMetadata.summary = '';
            editableMetadata.customFields = []; // Explicitly clear custom fields as well
        }
    }


    // This function is no longer used as AddFieldModal directly calls the store.
    // function handleAddCustomFieldConfirm(event) { ... }

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