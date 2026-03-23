<script>
    import { onMount, createEventDispatcher } from 'svelte';
    import { get } from 'svelte/store';
    import { project, switchTranscriptInDataTab } from '$lib/stores/projectStore.js';
    import { invoke } from '@tauri-apps/api/core';
    import { basename, extname as getFileExtname, sep as getPathSep, resolve } from '@tauri-apps/api/path';
    import { refresher, triggerRefresh } from '$lib/stores/refresherStore.js';
    import notificationStore from '$lib/stores/notificationStore.js';
    import { Dropdown, DropdownItem } from 'flowbite-svelte';
    import { Music, PlayCircle, Plus, PieChart, ChartBar, ChartColumn, LineChart, ScatterChart, SquareChartGantt, Table2, LayoutGrid, Trash2, MoreVertical, ExternalLink, Settings, FolderClosed, FolderOpen as FolderOpenIcon, FileText, Image as ImageIcon, MessageSquareText, SquareArrowOutUpLeft, FilePenLine } from '@lucide/svelte';

    export let itemPath = null;
    export let itemType = null;
    export let refreshKey = null;

    const dispatch = createEventDispatcher();

    let attachments = [];
    import FileRenameModal from '../../modals/FileRenameModal.svelte';
    import ImagePreviewModal from '../../modals/ImagePreviewModal.svelte';

    let isLoading = true;
    let previousProcessedItemPath = null;
    let currentTrackIndex = -1;

    let showRenameModal = false;
    let itemToRename = null;

    let showImagePreviewModal = false;
    let imagePreviewPath = '';

    // Helper to search the file tree for active media file
    function findFileInTree(nodes, path) {
        for (const node of nodes) {
            if (node.path === path) return node;
            if (node.children) {
                const found = findFileInTree(node.children, path);
                if (found) return found;
            }
        }
        return null;
    }

    // We will group string attachments by their folder if they are inside attachments/something/
    let groupedAttachments = { root: [], folders: {} };
    let expandedFolders = {};

    function processAttachmentsForGrouping(rawAttachments) {
        let root = [];
        let folders = {};

        rawAttachments.forEach((attachment, originalIndex) => {
            // Objects (charts, views) or files not matching the pattern go to root
            if (typeof attachment !== 'string') {
                // Ignore the "survey" view configuration object itself from rendering in the attachments list,
                // as its generated documents are what we want to show.
                if (attachment.view_type === 'survey') return;

                root.push({ attachment, originalIndex });
                return;
            }

            // We expect string paths. Let's see if it's inside a nested folder under attachments
            // Example: "harvey_files/tables/xyz/attachments/survey_2026_participants/Participant_1.json"
            const parts = attachment.split(/[\/\\]/);
            const attachmentsIdx = parts.indexOf('attachments');

            if (attachmentsIdx !== -1 && parts.length > attachmentsIdx + 2) {
                // It's in a subfolder: attachments -> folder_name -> file_name
                const folderName = parts[attachmentsIdx + 1];
                if (!folders[folderName]) {
                    folders[folderName] = [];
                    expandedFolders[folderName] = expandedFolders[folderName] || false;
                }
                folders[folderName].push({ attachment, originalIndex });
            } else {
                root.push({ attachment, originalIndex });
            }
        });

        groupedAttachments = { root, folders };
    }

    export function resetSelection() {
        currentTrackIndex = -1;
    }

    export function setSelectionByObject(attachmentObj) {
        if (!attachmentObj || !attachments) {
            currentTrackIndex = -1;
            return;
        }

        const targetName = attachmentObj.chart_name || attachmentObj.view_name;
        const targetType = attachmentObj.chart_type || attachmentObj.view_type;

        if (!targetName) return;

        const idx = attachments.findIndex(a => {
            if (typeof a !== 'object') return false;
            const aName = a.chart_name || a.view_name;
            const aType = a.chart_type || a.view_type;
            return aName === targetName && aType === targetType;
        });

        if (idx !== -1) {
            currentTrackIndex = idx;
        } else {
            currentTrackIndex = -1;
        }
    }

    function toggleFolder(folderName) {
        expandedFolders[folderName] = !expandedFolders[folderName];
        expandedFolders = { ...expandedFolders };
    }

    function getFileName(path) {
        if (typeof path === 'object' && path.chart_name) return path.chart_name;
        if (typeof path === 'object' && path.view_name) return path.view_name;
        if (typeof path === 'object' && path.is_transcript) return path.displayLabel || path.name || path.path.split(/[\/\\]/).pop();
        return path.split(/[\/\\]/).pop() || path;
    }

    function playTrack(index) {
        if (index >= 0 && index < attachments.length) {
            currentTrackIndex = index;
            const attachment = attachments[index];
            if (typeof attachment === 'object' && attachment.chart_name) {
                dispatch('requestOpenChart', { chart: attachment });
            } else if (typeof attachment === 'object' && attachment.view_name) {
                dispatch('requestOpenView', { view: attachment });
            } else if (typeof attachment === 'object' && attachment.is_transcript) {
                switchTranscriptInDataTab(attachment.path);
            } else if (typeof attachment === 'string' && /\.(png|jpe?g|gif|webp|svg)$/i.test(attachment)) {
                imagePreviewPath = attachment;
                showImagePreviewModal = true;
                return;
            } else {
                dispatch('requestPlayMedia', { mediaPath: attachment });
            }
        }
    }

    function openRenameModal(transcript) {
        if (!transcript || !transcript.path) return;
        itemToRename = {
            path: transcript.path,
            name: transcript.name || transcript.path.split(/[\/\\]/).pop(),
            file_type: 'transcript'
        };
        showRenameModal = true;
    }

    async function handleRenameConfirm(event) {
        const { newName } = event.detail;
        const item = itemToRename;

        if (!item || !newName || newName.trim() === '') {
            console.error("[AttachmentsPanel] Rename confirmation failed: Missing item or new name.");
            showRenameModal = false;
            itemToRename = null;
            return;
        }

        const finalNewName = newName.trim();
        showRenameModal = false;

        const { renameProjectItem } = await import('$lib/services/projectService.js');
        const { confirm } = await import('@tauri-apps/plugin-dialog');

        try {
            await renameProjectItem(item.path, finalNewName, item.file_type);
            notificationStore.add('Transcript renamed successfully.', 'success');
            await loadAttachments(previousProcessedItemPath);
            triggerRefresh();
        } catch (err) {
            console.error(`[AttachmentsPanel] Rename service call failed:`, err);
            notificationStore.add(`Failed to rename transcript: ${err.message || err}`, 'error');
        } finally {
            itemToRename = null;
        }
    }

    function handleRenameModalClose() {
        showRenameModal = false;
        itemToRename = null;
    }

    async function handleDeleteTranscript(transcript) {
        if (!transcript || !transcript.path) return;

        const { ask } = await import('@tauri-apps/plugin-dialog');
        const confirmed = await ask(`Are you sure you want to delete the transcript file "${transcript.name}"?\n\nThis will remove it from the project.\n\nThis action cannot be undone.`, { title: 'Delete Transcript', type: 'warning' });
        if (!confirmed) return;

        const { deleteProjectItem } = await import('$lib/services/projectService.js');

        const projectStoreState = get(project);
        const isActive = projectStoreState.activeTranscriptPathInDataTab === transcript.path;

        try {
            await deleteProjectItem(transcript.path);
            notificationStore.add('Transcript deleted.', 'success');

            if (isActive) {
                // Find another transcript to fall back to after deletion
                const currentFiles = get(project).files;
                const activeMediaFile = findFileInTree(currentFiles, projectStoreState.selectedMediaNotePath);

                if (activeMediaFile && activeMediaFile.associated_transcripts && activeMediaFile.associated_transcripts.length > 0) {
                    const fallbackTranscript = activeMediaFile.associated_transcripts[0];
                    switchTranscriptInDataTab(fallbackTranscript.path);
                } else {
                    switchTranscriptInDataTab(null);
                }
            }

            // Refresh attachments
            await loadAttachments(previousProcessedItemPath);
            triggerRefresh();
        } catch (error) {
            console.error('Failed to delete transcript via attachments panel:', error);
            notificationStore.add(`Failed to delete transcript: ${error.message || error}`, 'error');
        }
    }

    async function handleDeleteChart(chart) {
        if (!chart || !chart.chart_name) return;

        const { ask } = await import('@tauri-apps/plugin-dialog');
        const confirmed = await ask(`Are you sure you want to delete ${chart.chart_name}?`, { title: 'Delete Chart', type: 'warning' });
        if (!confirmed) return;

        const projectStoreState = get(project);

        // Match logic of ChartModal by converting active active item path to relative if needed, but the backend stores the relative table path.
        // We know we fetched these charts using `previousProcessedItemPath`.
        try {
            await invoke('delete_chart_config_command', {
                projectId: projectStoreState.id,
                tablePath: previousProcessedItemPath,
                chartName: chart.chart_name
            });
            notificationStore.add('Chart deleted.', 'success');
            // Optimistic update
            attachments = attachments.filter(a => a.chart_name !== chart.chart_name);
            dispatch('chartSaved'); // Optionally trigger broader UI refresh if needed
        } catch (error) {
            console.error('Failed to delete chart via attachments panel:', error);
            notificationStore.add('Failed to delete chart.', 'error');
        }
    }

    async function handleDeleteView(view) {
        if (!view || !view.view_name) return;

        const { ask } = await import('@tauri-apps/plugin-dialog');
        let promptMessage = `Are you sure you want to delete view ${view.view_name}?`;
        if (view.view_type === 'survey') {
            promptMessage += `\n\nWARNING: Deleting this Survey Data Table view will also permanently delete ALL generated .json documents associated with it. This action cannot be undone.`;
        }

        const confirmed = await ask(promptMessage, { title: 'Delete View', type: 'warning' });
        if (!confirmed) return;

        const projectStoreState = get(project);

        try {
            await invoke('delete_table_view_command', {
                projectId: projectStoreState.id,
                tablePath: previousProcessedItemPath,
                viewName: view.view_name,
                projectXmlPathStr: projectStoreState.xmlPath
            });
            notificationStore.add('View deleted.', 'success');

            // If it was a survey view, we need to completely reload to clear out the deleted documents
            if (view.view_type === 'survey') {
                await loadAttachments(previousProcessedItemPath);
            } else {
                attachments = attachments.filter(a => a.view_name !== view.view_name);
            }
            dispatch('viewSaved');
            dispatch('requestDeleteView', { viewName: view.view_name });
            triggerRefresh();
        } catch (error) {
            console.error('Failed to delete view via attachments panel:', error);
            notificationStore.add('Failed to delete view.', 'error');
        }
    }

    async function handleDeleteDocument(documentPath) {
        if (!documentPath) return;

        const { ask } = await import('@tauri-apps/plugin-dialog');
        const confirmed = await ask(`Are you sure you want to permanently delete this document?\n\nThis action cannot be undone.`, { title: 'Delete Document', type: 'warning' });
        if (!confirmed) return;

        const projectStoreState = get(project);
        if (!projectStoreState.xmlPath || !previousProcessedItemPath) return;

        try {
            // Document paths in attachments list might be absolute (due to legacy behavior) or relative.
            // The backend handles resolving this properly now in delete_attachment_command.
            // But we try to pass a relative path if possible.
            let attachmentRelPath = documentPath;
            if (documentPath.startsWith(projectStoreState.baseDirectory)) {
                attachmentRelPath = documentPath.substring(projectStoreState.baseDirectory.length);
                attachmentRelPath = attachmentRelPath.replace(/\\/g, '/').replace(/^\//, '');
            }

            await invoke('delete_attachment_command', {
                projectXmlPathStr: projectStoreState.xmlPath,
                assetRelativePath: previousProcessedItemPath,
                attachmentRelativePath: attachmentRelPath
            });
            notificationStore.add('Document deleted.', 'success');
            await loadAttachments(previousProcessedItemPath);
            dispatch('attachmentdeleted', { path: documentPath });
        } catch (error) {
            console.error('Failed to delete document:', error);
            notificationStore.add(`Failed to delete document: ${error}`, 'error');
        }
    }

    async function handleAddAttachment() {
        const projectStoreState = get(project);
        if (!projectStoreState.xmlPath || !itemPath) return;

        try {
            const { open } = await import('@tauri-apps/plugin-dialog');
            const selected = await open({
                multiple: true,
                filters: [
                    { name: 'Media/Attachments', extensions: ['mp3', 'wav', 'm4a', 'ogg', 'aac', 'flac', 'mp4', 'mov', 'avi', 'mkv', 'webm', 'png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'] }
                ]
            });

            if (selected) {
                const sourceFiles = Array.isArray(selected) ? selected : [selected];
                const originalDetails = await getOriginalAssetDetails(itemPath, projectStoreState);
                const assetRelativePath = originalDetails?.originalRelativePath;
                
                if (assetRelativePath) {
                    for (const sourceFilePath of sourceFiles) {
                        await invoke('upload_attachment', {
                            projectXmlPathStr: projectStoreState.xmlPath,
                            assetRelativePath: assetRelativePath,
                            sourceFilePathStr: sourceFilePath
                        });
                    }
                    await loadAttachments(assetRelativePath);
                    notificationStore.add('Attachment(s) added successfully.', 'success');
                }
            }
        } catch (error) {
            console.error('[AttachmentsPanel] Error adding attachment:', error);
            notificationStore.add(`Error adding attachment: ${error.message || error}`, 'error');
        }
    }

    async function getOriginalAssetDetails(selectedPath, projectStoreState) {
        if (!selectedPath || !projectStoreState || !projectStoreState.baseDirectory) return null;
        let originalRelativePath = selectedPath.startsWith(projectStoreState.baseDirectory) ? selectedPath.substring(projectStoreState.baseDirectory.length) : selectedPath;
        originalRelativePath = originalRelativePath.replace(/\\/g, '/').replace(/^\//, '');
        return { originalRelativePath };
    }

    async function loadAttachments(assetRelativePathToLoad) {
        isLoading = true;
        attachments = [];
        currentTrackIndex = -1;
        const projectStoreState = get(project);
        if (!projectStoreState.id || !assetRelativePathToLoad) {
            isLoading = false;
            return;
        }

        try {
            let loadedAttachments = [];

            if (itemType === 'table') {
                const charts = await invoke('load_chart_configs_command', {
                    projectId: projectStoreState.id,
                    tablePath: assetRelativePathToLoad
                });
                const views = await invoke('load_table_views_command', {
                    projectId: projectStoreState.id,
                    tablePath: assetRelativePathToLoad
                });
                loadedAttachments = [...charts, ...views];
            } else if (projectStoreState.selectedMediaNotePath) {
                const activeMediaFile = findFileInTree(projectStoreState.files, projectStoreState.selectedMediaNotePath);
                if (activeMediaFile && activeMediaFile.associated_transcripts) {
                    const { languageOptions } = await import('$lib/constants/transcriptionOptions.js');
                    const getLanguageLabel = (langCode) => {
                        if (!langCode || langCode === 'original') return 'Original';
                        let targetCode = langCode;
                        if (langCode.includes('-')) {
                            targetCode = langCode.split('-').pop(); // e.g., 'en-hi' -> 'hi'
                        }
                        const option = languageOptions.find(opt => opt.value === targetCode);
                        return option ? option.label : targetCode;
                    };
                    const mappedTranscripts = activeMediaFile.associated_transcripts.map(t => {
                        const langLabel = getLanguageLabel(t.language_code || 'original');
                        let fileName = t.name;
                        if (!fileName && t.path) {
                            try {
                                const pathParts = t.path.split(/[\/\\]/);
                                fileName = pathParts[pathParts.length - 1];
                                if (fileName.toLowerCase().endsWith('.json')) {
                                    fileName = fileName.substring(0, fileName.length - 5);
                                }
                            } catch (e) {
                                fileName = '';
                            }
                        }
                        const fileNamePart = fileName ? ` (${fileName})` : '';
                        const displayLabel = `${langLabel}${fileNamePart}`;
                        return { ...t, is_transcript: true, displayLabel };
                    }).sort((a, b) => a.displayLabel.localeCompare(b.displayLabel));

                    loadedAttachments = [...loadedAttachments, ...mappedTranscripts];
                }
            }

            // Always attempt to load raw file attachments from asset_metadata for all types
            try {
                const result = await invoke('get_asset_metadata_command', {
                    projectId: projectStoreState.id,
                    assetRelativePath: assetRelativePathToLoad
                });

                if (result && result.custom_fields_json) {
                    const customFields = JSON.parse(result.custom_fields_json);
                    const attachmentsField = customFields.find(f => f.key === 'attachments');
                    if (attachmentsField && attachmentsField.value) {
                        const fileAttachments = JSON.parse(attachmentsField.value);
                        loadedAttachments = [...loadedAttachments, ...fileAttachments];
                    }
                }
            } catch (metaError) {
                console.error(`[AttachmentsPanel] Could not load raw asset_metadata attachments:`, metaError);
            }

            attachments = loadedAttachments;
        } catch (error) {
            console.error(`[AttachmentsPanel] Error loading metadata for ${assetRelativePathToLoad}:`, error);
        } finally {
            processAttachmentsForGrouping(attachments);
            isLoading = false;
            previousProcessedItemPath = assetRelativePathToLoad;
        }
    }

    $: if (attachments) {
        processAttachmentsForGrouping(attachments);
    }

    $: {
        // Tie to $refresher so that triggerRefresh() across the app reloads attachments too.
        $refresher;
        (async () => {
            const currentProjectStoreState = get(project);
            const isSupportedType = itemType === 'doc' || itemType === 'imported_transcript' || itemType === 'table' || currentProjectStoreState.selectedMediaNotePath;
            const currentPathToUse = itemType === 'doc' && currentProjectStoreState.selectedMediaNotePath ? currentProjectStoreState.selectedMediaNotePath : itemPath;
            if (currentPathToUse && isSupportedType && currentProjectStoreState?.baseDirectory) {
                const newOriginalDetails = await getOriginalAssetDetails(currentPathToUse, currentProjectStoreState);
                const newDerivedRelativePath = newOriginalDetails?.originalRelativePath;

                // Also reload if refreshKey changes OR $refresher increments, but we handle the initial load as well
                if (newDerivedRelativePath) {
                    await loadAttachments(newDerivedRelativePath);
                } else if (!newDerivedRelativePath) {
                    attachments = [];
                    previousProcessedItemPath = null;
                    currentTrackIndex = -1;
                }
            } else {
                attachments = [];
                previousProcessedItemPath = null;
                currentTrackIndex = -1;
            }
        })();
    }
</script>

<div class="h-full bg-white dark:bg-gray-900 flex flex-col overflow-hidden">
    <div class="text-sm font-semibold border-b pb-1 px-1 border-gray-300 dark:border-gray-800 text-gray-700 dark:text-gray-300 flex-shrink-0 flex items-center justify-between h-9 mb-2">
        <span class="ml-1">Attachments</span>
        {#if itemType !== 'doc' && itemType !== 'table'}
            <button 
                on:click={handleAddAttachment}
                class="p-1 hover:bg-gray-200 dark:hover:bg-gray-800 rounded-full transition-colors text-blue-600 dark:text-blue-400 flex items-center justify-center"
                title="Add Attachment"
            >
                <Plus class="w-4 h-4" />
            </button>
        {/if}
    </div>
    <div class="flex-grow overflow-y-auto min-h-0">
        {#if isLoading}
            <p class="text-xs text-gray-500 dark:text-gray-400 italic px-2 py-4">Loading...</p>
        {:else if attachments.length > 0}
            <ul class="divide-y divide-gray-200 dark:divide-gray-800">
                <!-- Render Folders First -->
                {#each Object.keys(groupedAttachments.folders) as folderName}
                    <li class="flex flex-col border-b border-gray-100 dark:border-gray-800 last:border-0">
                        <div
                            class="p-2 flex items-center justify-between group cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
                            on:click={() => toggleFolder(folderName)}
                        >
                            <div class="flex items-center space-x-2 truncate">
                                {#if expandedFolders[folderName]}
                                    <FolderOpenIcon class="w-4 h-4 text-blue-500 shrink-0" />
                                {:else}
                                    <FolderClosed class="w-4 h-4 text-blue-500 shrink-0" />
                                {/if}
                                <span class="text-sm font-medium text-gray-800 dark:text-gray-200 truncate" title={folderName}>
                                    {folderName}
                                </span>
                            </div>
                            <div class="flex items-center gap-2 shrink-0">
                                <span class="text-xs text-gray-400 dark:text-gray-500 group-hover:hidden transition-opacity">{groupedAttachments.folders[folderName].length} items</span>
                                <button class="text-gray-500 dark:text-gray-400 p-1 hover:bg-gray-200 dark:hover:bg-gray-700 rounded opacity-0 group-hover:opacity-100 transition-opacity focus:opacity-100"
                                    title="Delete Folder"
                                    on:click|stopPropagation={() => {
                                        // The view name is the folder name without the trailing "_participants" or "_questions"
                                        const viewName = folderName.endsWith('_participants') ? folderName.slice(0, -13) :
                                                         folderName.endsWith('_questions') ? folderName.slice(0, -10) : folderName;
                                        handleDeleteView({ view_name: viewName, view_type: 'survey' });
                                    }}
                                >
                                    <Trash2 class="w-3.5 h-3.5 text-red-500" />
                                </button>
                            </div>
                        </div>

                        {#if expandedFolders[folderName]}
                            <ul class="pl-4 bg-gray-50/50 dark:bg-gray-900/50 pb-1">
                                {#each groupedAttachments.folders[folderName] as { attachment, originalIndex } (attachment)}
                                    <li
                                        class="py-1.5 pr-2 pl-3 flex items-center justify-between group cursor-pointer border-l-2 border-transparent hover:border-blue-400"
                                        class:bg-blue-100={currentTrackIndex === originalIndex}
                                        class:dark:bg-blue-800={currentTrackIndex === originalIndex}
                                        on:click={() => { playTrack(originalIndex); dispatch('requestOpenLexicalDocument', { docPath: attachment }); }}
                                    >
                                        <div class="flex items-center space-x-2 truncate">
                                            <FileText class="w-3.5 h-3.5 text-gray-400 shrink-0" />
                                            <span class="text-xs text-gray-700 dark:text-gray-300 truncate" title={getFileName(attachment)}>
                                                {getFileName(attachment)}
                                            </span>
                                        </div>
                                        <div class="opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity flex items-center justify-center shrink-0">
                                            <button class="text-gray-500 dark:text-gray-400 p-0.5 hover:bg-gray-200 dark:hover:bg-gray-700 rounded" title="Document Options" id="doc-options-folder-{originalIndex}" on:click|stopPropagation>
                                                <MoreVertical class="w-3.5 h-3.5" />
                                            </button>
                                            <Dropdown triggeredBy="#doc-options-folder-{originalIndex}" class="w-36 z-50" on:click={(e) => e.stopPropagation()}>
                                                <DropdownItem class="flex items-center gap-2" on:click={(e) => { e.stopPropagation(); playTrack(originalIndex); dispatch('requestOpenLexicalDocument', { docPath: attachment }); }}>
                                                    <SquareArrowOutUpLeft class="w-4 h-4 text-gray-500" /> Open
                                                </DropdownItem>
                                                <DropdownItem class="flex items-center gap-2 text-red-600 dark:text-red-400" on:click={(e) => { e.stopPropagation(); handleDeleteDocument(attachment); }}>
                                                    <Trash2 class="w-4 h-4" /> Delete
                                                </DropdownItem>
                                            </Dropdown>
                                        </div>
                                    </li>
                                {/each}
                            </ul>
                        {/if}
                    </li>
                {/each}

                <!-- Render Root Items -->
                {#each groupedAttachments.root as { attachment, originalIndex } (attachment)}
                    <li
                        class="p-2 flex items-center justify-between group cursor-pointer"
                        class:bg-blue-100={currentTrackIndex === originalIndex || (typeof attachment === 'object' && attachment.is_transcript && attachment.path === $project.activeTranscriptPathInDataTab)}
                        class:dark:bg-blue-800={currentTrackIndex === originalIndex || (typeof attachment === 'object' && attachment.is_transcript && attachment.path === $project.activeTranscriptPathInDataTab)}
                        on:click={() => playTrack(originalIndex)}
                    >
                        <div class="flex items-center space-x-3 truncate">
                            {#if typeof attachment === 'object' && attachment.chart_name}
                                {#if attachment.chart_type === 'bar'}<ChartBar class="w-4 h-4 text-gray-400 shrink-0" />{/if}
                                {#if attachment.chart_type === 'column'}<ChartColumn class="w-4 h-4 text-gray-400 shrink-0" />{/if}
                                {#if attachment.chart_type === 'line'}<LineChart class="w-4 h-4 text-gray-400 shrink-0" />{/if}
                                {#if attachment.chart_type === 'scatter'}<ScatterChart class="w-4 h-4 text-gray-400 shrink-0" />{/if}
                                {#if attachment.chart_type === 'pie'}<PieChart class="w-4 h-4 text-gray-400 shrink-0" />{/if}
                                {#if attachment.chart_type === 'gantt'}<SquareChartGantt class="w-4 h-4 text-gray-400 shrink-0" />{/if}
                            {:else if typeof attachment === 'object' && attachment.view_name}
                                {#if attachment.view_type === 'partial'}<Table2 class="w-4 h-4 text-gray-400 shrink-0" />{/if}
                                {#if attachment.view_type === 'pivot'}<LayoutGrid class="w-4 h-4 text-gray-400 shrink-0" />{/if}
                            {:else if typeof attachment === 'object' && attachment.is_transcript}
                                <MessageSquareText class="w-4 h-4 text-gray-400 shrink-0" />
                            {:else if typeof attachment === 'string' && attachment.endsWith('.json')}
                                <FileText class="w-4 h-4 text-gray-400 shrink-0" />
                            {:else if typeof attachment === 'string' && /\.(png|jpe?g|gif|webp|svg)$/i.test(attachment)}
                                <ImageIcon class="w-4 h-4 text-gray-400 shrink-0" />
                            {:else}
                                <Music class="w-4 h-4 text-gray-400 shrink-0" />
                            {/if}
                            <span class="text-sm text-gray-800 dark:text-gray-200 truncate" title={typeof attachment === 'object' ? (attachment.chart_name || attachment.view_name || (attachment.is_transcript ? attachment.displayLabel : '')) : attachment}>
                                {getFileName(attachment)}
                            </span>
                        </div>
                        {#if typeof attachment === 'object' && attachment.chart_name}
                            <div class="opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity">
                                <button class="text-gray-500 dark:text-gray-400 p-1 hover:bg-gray-100 dark:hover:bg-gray-800 rounded" title="Chart Options" id="chart-options-{originalIndex}" on:click|stopPropagation>
                                    <MoreVertical class="w-4 h-4" />
                                </button>
                                <Dropdown triggeredBy="#chart-options-{originalIndex}" class="w-36 z-50" on:click={(e) => e.stopPropagation()}>
                                    <DropdownItem class="flex items-center gap-2" on:click={(e) => { e.stopPropagation(); playTrack(originalIndex); }}>
                                        <SquareArrowOutUpLeft class="w-4 h-4 text-gray-500" /> Open
                                    </DropdownItem>
                                    <DropdownItem class="flex items-center gap-2 text-red-600 dark:text-red-400" on:click={(e) => { e.stopPropagation(); handleDeleteChart(attachment); }}>
                                        <Trash2 class="w-4 h-4" /> Delete
                                    </DropdownItem>
                                </Dropdown>
                            </div>
                        {:else if typeof attachment === 'object' && attachment.view_name}
                            <div class="opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity">
                                <button class="text-gray-500 dark:text-gray-400 p-1 hover:bg-gray-100 dark:hover:bg-gray-800 rounded" title="View Options" id="view-options-{originalIndex}" on:click|stopPropagation>
                                    <MoreVertical class="w-4 h-4" />
                                </button>
                                <Dropdown triggeredBy="#view-options-{originalIndex}" class="w-36 z-50" on:click={(e) => e.stopPropagation()}>
                                    <DropdownItem class="flex items-center gap-2" on:click={(e) => { e.stopPropagation(); playTrack(originalIndex); }}>
                                        <SquareArrowOutUpLeft class="w-4 h-4 text-gray-500" /> Open
                                    </DropdownItem>
                                    <DropdownItem class="flex items-center gap-2" on:click={(e) => { e.stopPropagation(); currentTrackIndex = originalIndex; dispatch('requestConfigureView', { view: attachment }); }}>
                                        <Settings class="w-4 h-4 text-gray-500" /> Configure
                                    </DropdownItem>
                                    <DropdownItem class="flex items-center gap-2 text-red-600 dark:text-red-400" on:click={(e) => { e.stopPropagation(); handleDeleteView(attachment); }}>
                                        <Trash2 class="w-4 h-4" /> Delete
                                    </DropdownItem>
                                </Dropdown>
                            </div>
                        {:else if typeof attachment === 'object' && attachment.is_transcript}
                            <div class="opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity">
                                <button class="text-gray-500 dark:text-gray-400 p-1 hover:bg-gray-100 dark:hover:bg-gray-800 rounded" title="Transcript Options" id="transcript-options-{originalIndex}" on:click|stopPropagation>
                                    <MoreVertical class="w-4 h-4" />
                                </button>
                                <Dropdown triggeredBy="#transcript-options-{originalIndex}" class="w-36 z-50" on:click={(e) => e.stopPropagation()}>
                                    <DropdownItem class="flex items-center gap-2" on:click={(e) => { e.stopPropagation(); currentTrackIndex = originalIndex; switchTranscriptInDataTab(attachment.path); }}>
                                        <SquareArrowOutUpLeft class="w-4 h-4 text-gray-500" /> Open
                                    </DropdownItem>
                                    <DropdownItem class="flex items-center gap-2" on:click={(e) => { e.stopPropagation(); openRenameModal(attachment); }}>
                                        <FilePenLine class="w-4 h-4 text-gray-500" /> Rename...
                                    </DropdownItem>
                                    <DropdownItem class="flex items-center gap-2 text-red-600 dark:text-red-400" on:click={(e) => { e.stopPropagation(); handleDeleteTranscript(attachment); }}>
                                        <Trash2 class="w-4 h-4" /> Delete
                                    </DropdownItem>
                                </Dropdown>
                            </div>
                        {:else if typeof attachment === 'string' && attachment.endsWith('.json')}
                            <div class="opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity flex items-center justify-center">
                                <button class="text-gray-500 dark:text-gray-400 p-1 hover:bg-gray-100 dark:hover:bg-gray-800 rounded" title="Document Options" id="doc-options-{originalIndex}" on:click|stopPropagation>
                                    <MoreVertical class="w-4 h-4" />
                                </button>
                                <Dropdown triggeredBy="#doc-options-{originalIndex}" class="w-36 z-50" on:click={(e) => e.stopPropagation()}>
                                    <DropdownItem class="flex items-center gap-2" on:click={(e) => { e.stopPropagation(); playTrack(originalIndex); dispatch('requestOpenLexicalDocument', { docPath: attachment }); }}>
                                        <SquareArrowOutUpLeft class="w-4 h-4 text-gray-500" /> Open
                                    </DropdownItem>
                                    <DropdownItem class="flex items-center gap-2 text-red-600 dark:text-red-400" on:click={(e) => { e.stopPropagation(); handleDeleteDocument(attachment); }}>
                                        <Trash2 class="w-4 h-4" /> Delete
                                    </DropdownItem>
                                </Dropdown>
                            </div>
                        {:else if typeof attachment === 'string' && /\.(png|jpe?g|gif|webp|svg)$/i.test(attachment)}
                            <div class="opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity flex items-center justify-center">
                                <button class="text-gray-500 dark:text-gray-400 p-1 hover:bg-gray-100 dark:hover:bg-gray-800 rounded" title="Image Options" id="image-options-{originalIndex}" on:click|stopPropagation>
                                    <MoreVertical class="w-4 h-4" />
                                </button>
                                <Dropdown triggeredBy="#image-options-{originalIndex}" class="w-36 z-50" on:click={(e) => e.stopPropagation()}>
                                    <DropdownItem class="flex items-center gap-2" on:click={(e) => { e.stopPropagation(); playTrack(originalIndex); }}>
                                        <SquareArrowOutUpLeft class="w-4 h-4 text-gray-500" /> Open
                                    </DropdownItem>
                                    <DropdownItem class="flex items-center gap-2 text-red-600 dark:text-red-400" on:click={(e) => { e.stopPropagation(); handleDeleteDocument(attachment); }}>
                                        <Trash2 class="w-4 h-4" /> Delete
                                    </DropdownItem>
                                </Dropdown>
                            </div>
                        {:else}
                            <div class="opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity flex items-center justify-center">
                                <button class="text-gray-500 dark:text-gray-400 p-1 hover:bg-gray-100 dark:hover:bg-gray-800 rounded" title="Media Options" id="media-options-{originalIndex}" on:click|stopPropagation>
                                    <MoreVertical class="w-4 h-4" />
                                </button>
                                <Dropdown triggeredBy="#media-options-{originalIndex}" class="w-36 z-50" on:click={(e) => e.stopPropagation()}>
                                    <DropdownItem class="flex items-center gap-2" on:click={(e) => { e.stopPropagation(); playTrack(originalIndex); }}>
                                        <SquareArrowOutUpLeft class="w-4 h-4 text-gray-500" /> Open
                                    </DropdownItem>
                                    <DropdownItem class="flex items-center gap-2 text-red-600 dark:text-red-400" on:click={(e) => { e.stopPropagation(); handleDeleteDocument(attachment); }}>
                                        <Trash2 class="w-4 h-4" /> Delete
                                    </DropdownItem>
                                </Dropdown>
                            </div>
                        {/if}
                    </li>
                {/each}
            </ul>
        {:else}
            <p class="text-xs text-gray-500 dark:text-gray-400 italic px-2 py-4">
                No attachments found.
            </p>
        {/if}
    </div>
</div>

<FileRenameModal
    bind:showModal={showRenameModal}
    currentName={itemToRename?.name || ''}
    itemType={itemToRename?.file_type || ''}
    on:confirm={handleRenameConfirm}
    on:close={handleRenameModalClose}
/>

<ImagePreviewModal
    bind:showModal={showImagePreviewModal}
    imagePath={imagePreviewPath}
    on:insert={(e) => dispatch('requestInsertAttachedImage', { imagePath: e.detail.path })}
    on:delete={(e) => handleDeleteDocument(e.detail.path)}
    on:cancel={() => showImagePreviewModal = false}
/>
