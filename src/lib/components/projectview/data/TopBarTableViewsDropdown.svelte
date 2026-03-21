<script>
    import { invoke } from '@tauri-apps/api/core';
    import { project } from '$lib/stores/projectStore.js';
    import { refresher } from '$lib/stores/refresherStore.js';
    import { get } from 'svelte/store';
    import { Button, Dropdown, DropdownItem } from 'flowbite-svelte';
    import { ChevronDown, ChevronRight, Sheet, Table2, LayoutGrid, FolderClosed, FileText } from '@lucide/svelte';
    import { createEventDispatcher, onMount } from 'svelte';

    const dispatch = createEventDispatcher();

    export let tablePath = null;
    export let activeSubItemPath = null;
    export let activeSubItemType = null;

    let views = [];
    let isLoading = false;
    let buttonLabel = 'Base Table';
    let dropdownOpen = false;

    let activeIcon = Sheet;
    $: {
        if (activeSubItemType === null || activeSubItemType === undefined) {
            buttonLabel = 'Base Table';
            activeIcon = Sheet;
        } else if (activeSubItemType === 'view' && activeSubItemPath?.view_name) {
            buttonLabel = activeSubItemPath.view_name;
            if (activeSubItemPath.view_type === 'partial') activeIcon = Table2;
            else if (activeSubItemPath.view_type === 'pivot') activeIcon = LayoutGrid;
            else if (activeSubItemPath.view_type === 'survey') activeIcon = FolderClosed;
            else activeIcon = Table2;
        } else if (activeSubItemType === 'doc' && typeof activeSubItemPath === 'string') {
            buttonLabel = activeSubItemPath.split(/[\/\\]/).pop();
            activeIcon = FileText;
        } else {
            buttonLabel = 'Base Table';
            activeIcon = Sheet;
        }
    }

    $: normalizedTablePath = (() => {
        if (!tablePath || !$project || !$project.baseDirectory) return tablePath;
        let relative = tablePath.startsWith($project.baseDirectory) ? tablePath.substring($project.baseDirectory.length) : tablePath;
        return relative.replace(/\\/g, '/').replace(/^\//, '');
    })();

    // Re-fetch views when path, project ID, or refresher changes
    $: if (normalizedTablePath && $project.id && ($refresher || true)) {
        fetchTableViews(normalizedTablePath, $project.id);
    } else {
        views = [];
    }

    async function fetchTableViews(path, projectId) {
        isLoading = true;
        try {
            console.log(`[TopBarTableViewsDropdown] Fetching views for normalized path: ${path}`);
            const fetchedViews = await invoke('load_table_views_command', { projectId, tablePath: path });
            console.log(`[TopBarTableViewsDropdown] Fetched ${fetchedViews.length} views.`);
            const result = await invoke('get_asset_metadata_command', { projectId, assetRelativePath: path });
            
            let fileAttachments = [];
            if (result && result.custom_fields_json) {
                const customFields = JSON.parse(result.custom_fields_json);
                const attachmentsField = customFields.find(f => f.key === 'attachments');
                if (attachmentsField && attachmentsField.value) {
                    fileAttachments = JSON.parse(attachmentsField.value);
                }
            }

            let processedViews = [];
            let surveyFolders = {};

            fetchedViews.forEach(view => {
                if (view.view_type === 'survey') {
                    surveyFolders[view.view_name] = { view, docs: [] };
                } else {
                    processedViews.push({ ...view, isSurvey: false });
                }
            });

            fileAttachments.forEach(attachment => {
                if (typeof attachment === 'string' && attachment.endsWith('.json')) {
                    const parts = attachment.split(/[\/\\]/);
                    const attachmentsIdx = parts.indexOf('attachments');
                    if (attachmentsIdx !== -1 && parts.length > attachmentsIdx + 2) {
                        const folderName = parts[attachmentsIdx + 1];
                        let surveyViewName = folderName;
                        if (folderName.endsWith('_participants')) surveyViewName = folderName.slice(0, -13);
                        else if (folderName.endsWith('_questions')) surveyViewName = folderName.slice(0, -10);

                        if (surveyFolders[surveyViewName]) {
                            surveyFolders[surveyViewName].docs.push({
                                path: attachment,
                                name: attachment.split(/[\/\\]/).pop()
                            });
                        }
                    }
                }
            });

            Object.values(surveyFolders).forEach(surveyObj => {
                processedViews.push({
                    ...surveyObj.view,
                    isSurvey: true,
                    children: surveyObj.docs
                });
            });

            views = processedViews.sort((a, b) => a.view_name.localeCompare(b.view_name));
        } catch (e) {
            console.error("[TopBarTableViewsDropdown] Error fetching table views:", e);
            views = [];
        } finally {
            isLoading = false;
        }
    }

    function selectBaseTable() {
        dropdownOpen = false;
        dispatch('requestClearSubItem');
    }

    function selectView(view) {
        dropdownOpen = false;
        dispatch('requestOpenView', { view });
    }

    function selectDocument(docPath) {
        dropdownOpen = false;
        dispatch('requestOpenLexicalDocument', { docPath });
    }
</script>

<div class="relative inline-block text-left">
    <Button id="table-views-dropdown-btn" color="alternative" size="xs" class="min-w-[140px] max-w-[220px] justify-between px-3 !py-1.5 focus:ring-0 shadow-sm ml-2" title="Select Table View">
        <div class="flex items-center min-w-0 mr-2">
            <svelte:component this={activeIcon} class="w-3.5 h-3.5 mr-2 text-gray-500 shrink-0" />
            <span class="truncate">{buttonLabel}</span>
        </div>
        <ChevronDown class="w-3.5 h-3.5 text-gray-500 shrink-0" />
    </Button>
    <Dropdown bind:open={dropdownOpen} triggeredBy="#table-views-dropdown-btn" class="w-60 z-[1001] max-h-96 overflow-y-auto">
        <DropdownItem class="text-xs flex items-center {activeSubItemType === null || activeSubItemType === undefined ? 'font-bold bg-blue-50 dark:bg-gray-700' : ''}" on:click={selectBaseTable}>
            <Sheet class="w-4 h-4 mr-2.5 text-gray-400 shrink-0" />
            <span>Base Table</span>
        </DropdownItem>
        {#if views.length > 0}
            <div class="my-1 border-t border-gray-200 dark:border-gray-600"></div>
        {/if}
        {#each views as view, index}
            {#if view.isSurvey}
                <DropdownItem id="survey-menu-item-{index}" class="text-xs flex items-center justify-between {activeSubItemType === 'view' && activeSubItemPath?.view_name === view.view_name ? 'font-bold bg-blue-50 dark:bg-gray-700' : ''}">
                    <div class="flex items-center min-w-0">
                        <FolderClosed class="w-4 h-4 mr-2.5 text-gray-400 shrink-0" />
                        <span class="truncate">{view.view_name}</span>
                    </div>
                    <ChevronRight class="w-4 h-4 ml-2 text-gray-400 shrink-0" />
                </DropdownItem>
                <Dropdown placement="right-start" triggeredBy="#survey-menu-item-{index}" trigger="hover" class="w-60 z-[1002] max-h-80 overflow-y-auto shadow-xl border border-gray-200 dark:border-gray-700">
                    <DropdownItem class="text-xs flex items-center font-bold border-b border-gray-200 dark:border-gray-600 {activeSubItemType === 'view' && activeSubItemPath?.view_name === view.view_name ? 'bg-blue-50 dark:bg-gray-700' : ''}" on:click={() => selectView(view)}>
                        <FolderClosed class="w-4 h-4 mr-2.5 text-gray-400 shrink-0" />
                        <span class="truncate">{view.view_name} (Dataset)</span>
                    </DropdownItem>
                    {#each view.children as child}
                        <DropdownItem class="text-xs flex items-center {activeSubItemType === 'doc' && activeSubItemPath === child.path ? 'font-bold bg-blue-50 dark:bg-gray-700' : ''}" on:click={() => selectDocument(child.path)}>
                            <FileText class="w-4 h-4 mr-2.5 text-gray-400 shrink-0" />
                            <span class="truncate">{child.name}</span>
                        </DropdownItem>
                    {/each}
                </Dropdown>
            {:else}
                <DropdownItem class="text-xs flex items-center {activeSubItemType === 'view' && activeSubItemPath?.view_name === view.view_name ? 'font-bold bg-blue-50 dark:bg-gray-700' : ''}" on:click={() => selectView(view)}>
                    {#if view.view_type === 'pivot'}
                        <LayoutGrid class="w-4 h-4 mr-2.5 text-gray-400 shrink-0" />
                    {:else}
                        <Table2 class="w-4 h-4 mr-2.5 text-gray-400 shrink-0" />
                    {/if}
                    <span class="truncate">{view.view_name}</span>
                </DropdownItem>
            {/if}
        {/each}
    </Dropdown>
</div>
