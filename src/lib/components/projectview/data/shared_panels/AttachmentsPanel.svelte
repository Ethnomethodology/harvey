<script>
    import { onMount, createEventDispatcher } from 'svelte';
    import { get } from 'svelte/store';
    import { project } from '$lib/stores/projectStore.js';
    import { invoke } from '@tauri-apps/api/core';
    import { basename, extname as getFileExtname, sep as getPathSep, resolve } from '@tauri-apps/api/path';
    import notificationStore from '$lib/stores/notificationStore.js';
    import { Dropdown, DropdownItem } from 'flowbite-svelte';
    import { FileAudio, PlayCircle, Plus, PieChart, ChartBar, ChartColumn, LineChart, ScatterChart, SquareChartGantt, Table2, LayoutGrid, Trash2, MoreVertical, ExternalLink, Settings } from 'lucide-svelte';

    export let itemPath = null;
    export let itemType = null;
    export let refreshKey = null;

    const dispatch = createEventDispatcher();

    let attachments = [];
    let isLoading = true;
    let previousProcessedItemPath = null;
    let currentTrackIndex = -1;

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

    function getFileName(path) {
        if (typeof path === 'object' && path.chart_name) return path.chart_name;
        if (typeof path === 'object' && path.view_name) return path.view_name;
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
            } else {
                dispatch('requestPlayMedia', { mediaPath: attachment });
            }
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
        const confirmed = await ask(`Are you sure you want to delete view ${view.view_name}?`, { title: 'Delete View', type: 'warning' });
        if (!confirmed) return;

        const projectStoreState = get(project);

        try {
            await invoke('delete_table_view_command', {
                projectId: projectStoreState.id,
                tablePath: previousProcessedItemPath,
                viewName: view.view_name
            });
            notificationStore.add('View deleted.', 'success');
            attachments = attachments.filter(a => a.view_name !== view.view_name);
            dispatch('viewSaved');
        } catch (error) {
            console.error('Failed to delete view via attachments panel:', error);
            notificationStore.add('Failed to delete view.', 'error');
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
                    { name: 'Audio/Video Files', extensions: ['mp3', 'wav', 'm4a', 'ogg', 'aac', 'flac', 'mp4', 'mov', 'avi', 'mkv', 'webm'] }
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
            if (itemType === 'table') {
                const charts = await invoke('load_chart_configs_command', {
                    projectId: projectStoreState.id,
                    tablePath: assetRelativePathToLoad
                });
                const views = await invoke('load_table_views_command', {
                    projectId: projectStoreState.id,
                    tablePath: assetRelativePathToLoad
                });
                attachments = [...charts, ...views];
            } else {
                const result = await invoke('get_asset_metadata_command', {
                    projectId: projectStoreState.id,
                    assetRelativePath: assetRelativePathToLoad
                });

                if (result && result.custom_fields_json) {
                    const customFields = JSON.parse(result.custom_fields_json);
                    const attachmentsField = customFields.find(f => f.key === 'attachments');
                    if (attachmentsField && attachmentsField.value) {
                        attachments = JSON.parse(attachmentsField.value);
                    }
                }
            }
        } catch (error) {
            console.error(`[AttachmentsPanel] Error loading metadata for ${assetRelativePathToLoad}:`, error);
        } finally {
            isLoading = false;
            previousProcessedItemPath = assetRelativePathToLoad;
        }
    }

    $: {
        (async () => {
            const currentProjectStoreState = get(project);
            const isSupportedType = itemType === 'doc' || itemType === 'imported_transcript' || itemType === 'table';
            if (itemPath && isSupportedType && currentProjectStoreState?.baseDirectory) {
                const newOriginalDetails = await getOriginalAssetDetails(itemPath, currentProjectStoreState);
                const newDerivedRelativePath = newOriginalDetails?.originalRelativePath;

                if (newDerivedRelativePath && (newDerivedRelativePath !== previousProcessedItemPath || refreshKey)) {
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
                {#each attachments as attachment, i (attachment)}
                    <li
                        class="p-2 flex items-center justify-between group cursor-pointer"
                        class:bg-blue-100={currentTrackIndex === i}
                        class:dark:bg-blue-800={currentTrackIndex === i}
                        on:click={() => playTrack(i)}
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
                            {:else}
                                <FileAudio class="w-4 h-4 text-gray-400 shrink-0" />
                            {/if}
                            <span class="text-sm text-gray-800 dark:text-gray-200 truncate" title={typeof attachment === 'object' ? (attachment.chart_name || attachment.view_name) : attachment}>
                                {getFileName(attachment)}
                            </span>
                        </div>
                        {#if typeof attachment === 'object' && attachment.chart_name}
                            <div class="opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity">
                                <button class="text-gray-500 dark:text-gray-400 p-1 hover:bg-gray-100 dark:hover:bg-gray-800 rounded" title="Chart Options" id="chart-options-{i}" on:click|stopPropagation>
                                    <MoreVertical class="w-4 h-4" />
                                </button>
                                <Dropdown triggeredBy="#chart-options-{i}" class="w-36 z-50" on:click={(e) => e.stopPropagation()}>
                                    <DropdownItem class="flex items-center gap-2" on:click={(e) => { e.stopPropagation(); currentTrackIndex = i; dispatch('requestOpenChart', { chart: attachment }); }}>
                                        <ExternalLink class="w-4 h-4 text-gray-500" /> Open
                                    </DropdownItem>
                                    <DropdownItem class="flex items-center gap-2 text-red-600 dark:text-red-400" on:click={(e) => { e.stopPropagation(); handleDeleteChart(attachment); }}>
                                        <Trash2 class="w-4 h-4" /> Delete
                                    </DropdownItem>
                                </Dropdown>
                            </div>
                        {:else if typeof attachment === 'object' && attachment.view_name}
                            <div class="opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity">
                                <button class="text-gray-500 dark:text-gray-400 p-1 hover:bg-gray-100 dark:hover:bg-gray-800 rounded" title="View Options" id="view-options-{i}" on:click|stopPropagation>
                                    <MoreVertical class="w-4 h-4" />
                                </button>
                                <Dropdown triggeredBy="#view-options-{i}" class="w-36 z-50" on:click={(e) => e.stopPropagation()}>
                                    <DropdownItem class="flex items-center gap-2" on:click={(e) => { e.stopPropagation(); currentTrackIndex = i; dispatch('requestOpenView', { view: attachment }); }}>
                                        <ExternalLink class="w-4 h-4 text-gray-500" /> Open
                                    </DropdownItem>
                                    <DropdownItem class="flex items-center gap-2" on:click={(e) => { e.stopPropagation(); currentTrackIndex = i; dispatch('requestConfigureView', { view: attachment }); }}>
                                        <Settings class="w-4 h-4 text-gray-500" /> Configure
                                    </DropdownItem>
                                    <DropdownItem class="flex items-center gap-2 text-red-600 dark:text-red-400" on:click={(e) => { e.stopPropagation(); handleDeleteView(attachment); }}>
                                        <Trash2 class="w-4 h-4" /> Delete
                                    </DropdownItem>
                                </Dropdown>
                            </div>
                        {:else}
                            <button class="text-gray-500 dark:text-gray-400 opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity flex items-center justify-center" title="Play" on:click|stopPropagation={() => playTrack(i)}>
                                <PlayCircle class="w-4 h-4" />
                            </button>
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
