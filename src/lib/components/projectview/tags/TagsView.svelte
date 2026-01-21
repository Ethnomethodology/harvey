<!-- src/lib/components/projectview/tags/TagsView.svelte -->
<script>
    import { onMount, afterUpdate, onDestroy } from 'svelte';
    import { slide } from 'svelte/transition';
    import { flip } from 'svelte/animate';
    import { dndzone } from 'svelte-dnd-action';
    import { invoke } from '@tauri-apps/api/core';
    import { confirm } from '@tauri-apps/plugin-dialog';
    import { TabulatorFull as Tabulator } from 'tabulator-tables';
    import { createEventDispatcher } from 'svelte';
    import { project } from '$lib/stores/projectStore.js';
    import { allTags, tagGroups, selectedTagState, updateTag, deleteTag, fetchAllTags, addTag, addTagGroup, moveTagToGroup, updateTagGroup, deleteTagGroup } from '$lib/stores/tagStore.js';
    import { refresher } from '$lib/stores/refresherStore.js';
    import { addCommentToHighlight, deleteComment, updateComment } from '$lib/stores/projectStore.js';
    import * as projectService from '$lib/services/projectService.js';
    import CommentsPanel from './CommentsPanel.svelte';
    import EditTagModal from '../modals/EditTagModal.svelte';
    import AddTagModal from '../modals/AddTagModal.svelte';
    import AddTagGroupModal from '../modals/AddTagGroupModal.svelte';
    import panelStateStore from '$lib/stores/panelStateStore.js';

    let unsubscribePanelState;
    let unsubscribeRefresher;
    let unsubscribeTags;
    let unsubscribeGroups;

    const dispatch = createEventDispatcher();

    // Local state for DnD
    let dndItems = [];
    const flipDurationMs = 200;

    onMount(() => {
        unsubscribePanelState = panelStateStore.subscribe(state => {
            if (tabulatorInstance) {
                tabulatorInstance.redraw(true);
                window.dispatchEvent(new Event('resize'));
            }
        });

        let isFirstRun = true;
        unsubscribeRefresher = refresher.subscribe(async () => {
            if (isFirstRun) {
                isFirstRun = false;
                return;
            }
            await fetchAllTags();
            checkSelectionValidity();
        });

        // Sync local DnD state with stores
        unsubscribeTags = allTags.subscribe(() => updateDndItems());
        unsubscribeGroups = tagGroups.subscribe(() => updateDndItems());

        // Restore selected item state
        const savedState = get(selectedTagState);
        if (savedState && savedState.id && savedState.type) {
            // We need to wait for dndItems to be populated?
            // dndItems is reactive to stores.
            // But on mount stores might be empty.
            // We can try to set it immediately if stores have data, or set a pending flag.
            // Since stores are subscribed, updateDndItems will run.
            // We can call restoreSelection() there.
        }
    });

    onDestroy(() => {
        if (unsubscribePanelState) unsubscribePanelState();
        if (unsubscribeRefresher) unsubscribeRefresher();
        if (unsubscribeTags) unsubscribeTags();
        if (unsubscribeGroups) unsubscribeGroups();
    });

    function updateDndItems() {
        // Construct the tree structure for DnD
        // Root items: All Groups + Tags with no group
        // We use a prefix to avoid ID collisions: 'g-' for groups, 't-' for tags

        const groups = $tagGroups.map(g => ({
            id: `g-${g.id}`,
            realId: g.id,
            name: g.name,
            type: 'group',
            description: g.description,
            // Children tags
            items: $allTags
                .filter(t => t.tag_group_id === g.id)
                .map(t => ({
                    id: `t-${t.id}`,
                    realId: t.id,
                    name: t.name,
                    type: 'tag',
                    description: t.description,
                    color: t.color
                }))
        }));

        const orphanTags = $allTags
            .filter(t => !t.tag_group_id)
            .map(t => ({
                id: `t-${t.id}`,
                realId: t.id,
                name: t.name,
                type: 'tag',
                description: t.description,
                color: t.color
            }));

        dndItems = [...groups, ...orphanTags];
        restoreSelection();
    }

    import { get } from 'svelte/store'; // Ensure get is imported if not already

    function restoreSelection() {
        if (selectedItem) return; // Already selected
        const savedState = get(selectedTagState);
        if (savedState && savedState.id && savedState.type) {
            // Find the item in dndItems
            let found = null;
            if (savedState.type === 'group') {
                found = dndItems.find(i => i.type === 'group' && i.realId === savedState.id);
            } else if (savedState.type === 'tag') {
                // Search top level
                found = dndItems.find(i => i.type === 'tag' && i.realId === savedState.id);
                if (!found) {
                    // Search in groups
                    for (const group of dndItems.filter(i => i.type === 'group')) {
                        found = group.items.find(t => t.type === 'tag' && t.realId === savedState.id);
                        if (found) break;
                    }
                }
            }
            if (found) {
                handleSelect(found, false); // Pass false to avoid re-saving state if it matches
            }
        }
    }

    function checkSelectionValidity() {
        if (selectedItem) {
            if (selectedItem.type === 'tag') {
                if (!$allTags.some(t => t.id === selectedItem.realId)) {
                    selectedItem = null;
                    infoData = null;
                }
            } else if (selectedItem.type === 'group') {
                if (!$tagGroups.some(g => g.id === selectedItem.realId)) {
                    selectedItem = null;
                    infoData = null;
                }
            }
        }
    }

    // --- Icons ---
    const AUDIO_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-music-note-beamed w-4 h-4" viewBox="0 0 16 16"><path d="M6 13c0 1.105-1.12 2-2.5 2S1 14.105 1 13s1.12-2 2.5-2 2.5.896 2.5 2m9-2c0 1.105-1.12 2-2.5 2s-2.5-.895-2.5-2 1.12-2 2.5-2 2.5.895 2.5 2"/><path fill-rule="evenodd" d="M14 11V2h1v9zM6 3v10H5V3z"/><path d="M5 2.905a1 1 0 0 1 .9-.995l8-.8a1 1 0 0 1 1.1.995V3L5 4z"/></svg>`;
    const VIDEO_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-film w-4 h-4" viewBox="0 0 16 16"><path d="M0 1a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H1a1 1 0 0 1-1-1zm4 0v6h8V1zm8 8H4v6h8zM1 1v2h2V1zm2 3H1v2h2zM1 7v2h2V7zm2 3H1v2h2zm-2 3v2h2v-2zM15 1h-2v2h2zm-2 3v2h2V4zm2 3h-2v2h2zm-2 3v2h2v-2zm2 3h-2v2h2z"/></svg>`;
    const DOCUMENT_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-files w-4 h-4" viewBox="0 0 16 16"><path d="M13 0H6a2 2 0 0 0-2 2 2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h7a2 2 0 0 0 2-2 2 2 0 0 0 2-2V2a2 2 0 0 0-2-2m0 13V4a2 2 0 0 0-2-2H5a1 1 0 0 1 1-1h7a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1M3 4a1 1 0 0 1 1-1h7a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z"/></svg>`;
    const IMAGE_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-images w-4 h-4" viewBox="0 0 16 16"><path d="M4.502 9a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3"/><path d="M14.002 13a2 2 0 0 1-2 2h-10a2 2 0 0 1-2-2V5A2 2 0 0 1 2 3a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v8a2 2 0 0 1-1.998 2M14 2H4a1 1 0 0 0-1 1h9.002a2 2 0 0 1 2 2v7A1 1 0 0 0 15 11V3a1 1 0 0 0-1-1M2.002 4a1 1 0 0 0-1 1v8l2.646-2.354a.5.5 0 0 1 .63-.062l2.66 1.773 3.71-3.71a.5.5 0 0 1 .577-.094l1.777 1.947V5a1 1 0 0 0-1-1z"/></svg>`;
    const TABLE_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-table w-4 h-4" viewBox="0 0 16 16"><path d="M0 2a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2zm15 2h-4v3h4zm0 4h-4v3h4zm0 4h-4v3h3a1 1 0 0 0 1-1zm-5 3v-3H6v3zm-5 0v-3H1v2a1 1 0 0 0 1 1zm-4-4h4V8H1zm0-4h4V4H1zm5-3v3h4V4zm4 4H6v3h4z"/></svg>`;
    const TRANSCRIPT_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chat-square-text w-4 h-4" viewBox="0 0 16 16"><path d="M14 1a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1h-2.5a2 2 0 0 0-1.6.8L8 14.333 6.1 11.8a2 2 0 0 0-1.6-.8H2a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1zM2 0a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2.5a1 1 0 0 1 .8.4l1.9 2.533a1 1 0 0 0 1.6 0l1.9-2.533a1 1 0 0 1 .8-.4H14a2 2 0 0 0 2-2V2a2 2 0 0 0-2-2z"/><path d="M3 3.5a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9a.5.5 0 0 1-.5-.5M3 6a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9A.5.5 0 0 1 3 6m0 2.5a.5.5 0 0 1 .5-.5h5a.5.5 0 0 1 0 1h-5a.5.5 0 0 1-.5-.5"/></svg>`;
    const UNKNOWN_ICON = `<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"><path stroke-linecap="round" stroke-linejoin="round" d="M9.879 7.519c1.171-1.025 3.071-1.025 4.242 0 1.172 1.025 1.172 2.687 0 3.712-.203.179-.43.326-.67.442-.745.361-1.45.999-1.45 1.827v.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 5.25h.008v.008H12v-.008Z" /></svg>`;

    function getIconForFileType(fileType) {
        switch (fileType) {
            case 'audio': return AUDIO_ICON;
            case 'video': return VIDEO_ICON;
            case 'document': return DOCUMENT_ICON;
            case 'image': return IMAGE_ICON;
            case 'table': return TABLE_ICON;
            case 'transcript': return TRANSCRIPT_ICON;
            case 'imported_transcript': return TRANSCRIPT_ICON;
            case 'audio_transcript': return AUDIO_ICON;
            case 'video_transcript': return VIDEO_ICON;
            default: return UNKNOWN_ICON;
        }
    }

    // --- Selection State ---
    let selectedItem = null; // { type: 'tag' | 'group', id, realId, name }
    let infoData = null; // Data returned from backend (TagInfo)
    let isLoading = false;
    let description = '';

    // --- Table State ---
    let tableContainer;
    let tabulatorInstance = null;
    let searchQuery = '';
    let tableReady = false;
    let processedHighlights = [];

    // --- Panels & Modals State ---
    let selectedHighlight = null;
    let isCommentsPanelOpen = false;
    let isEditModalOpen = false;
    let isAddTagModalOpen = false;
    let isAddTagGroupModalOpen = false;
    let showMenu = false;

    // --- DOM refs ---
    let menuButton;
    let menuDropdown;

    // --- Computed ---
    $: {
        if (typeof document !== 'undefined') {
            document.body.style.overflow = isCommentsPanelOpen ? 'hidden' : '';
        }
    }

    $: if ($panelStateStore.tagsLeftPanelCollapsed !== undefined && tabulatorInstance && tableReady) {
        if (tableContainer) {
            tabulatorInstance.redraw(true);
            window.dispatchEvent(new Event('resize'));
        }
    }

    $: {
        if (infoData && infoData.highlights) {
            processedHighlights = infoData.highlights.map(item => {
                const highlight = item.highlight || item;
                return {
                    ...highlight,
                    other_tags: highlight.other_tags || [],
                    comments: highlight.comments || [],
                    // Inject tag_name if present (for groups)
                    tag_name: item.tag_name
                };
            });
        } else {
            processedHighlights = [];
        }
    }

    // --- Event Handlers ---

    function toggleMenu() {
        showMenu = !showMenu;
    }

    function closeMenu(e) {
        if (menuButton && menuButton.contains(e.target)) return;
        if (menuDropdown && menuDropdown.contains(e.target)) return;
        showMenu = false;
    }

    async function handleSelect(item, updateStore = true) {
        selectedItem = item;
        if (updateStore) {
            selectedTagState.set({ type: item.type, id: item.realId });
        }
        infoData = null;
        description = '';
        isLoading = true;

        try {
            if (item.type === 'tag') {
                infoData = await invoke('get_tag_info', {
                    projectId: $project.id,
                    tagId: item.realId,
                    tagName: item.name,
                });
            } else if (item.type === 'group') {
                infoData = await invoke('get_tag_group_info', {
                    projectId: $project.id,
                    groupId: item.realId,
                });
            }

            if (infoData) {
                description = infoData.description;
            }
        } catch (error) {
            console.error(`Failed to load info for ${item.name}:`, error);
        } finally {
            isLoading = false;
        }
    }

    afterUpdate(() => {
        if (infoData && tableContainer && !tabulatorInstance) {
            initializeTable(processedHighlights);
        } else if (tabulatorInstance && tableReady) {
            // Check if we need to rebuild columns (e.g., switched from Tag to Group)
            // But getting current columns from instance is synchronous.
            // Simplified: Rebuild table if selected type changed?
            // Actually, just setColumns if needed.
            // Or destroy and recreate if columns structure differs significantly.
            // For now, let's try to update data. If columns need change, we might need logic.
            // Let's destroy and recreate table if the column structure requirement changes.
            // Group view needs "Tag Name" column. Tag view does not.

            const hasTagNameColumn = tabulatorInstance.getColumn("tag_name");
            const needsTagNameColumn = selectedItem?.type === 'group';

            if ((needsTagNameColumn && !hasTagNameColumn) || (!needsTagNameColumn && hasTagNameColumn)) {
                initializeTable(processedHighlights);
            } else {
                tabulatorInstance.setData(processedHighlights);
            }
        }

        if (!infoData && tabulatorInstance) {
            tabulatorInstance.destroy();
            tabulatorInstance = null;
            tableReady = false;
        }
    });

    function handleSearch() {
        if (!tabulatorInstance) return;
        if (searchQuery && searchQuery.trim() !== '') {
            tabulatorInstance.setFilter("text", "like", searchQuery.trim());
        } else {
            tabulatorInstance.clearFilter();
        }
    }

    function initializeTable(data) {
        if (tabulatorInstance) {
            tabulatorInstance.destroy();
        }
        tableReady = false;

        const columns = [
            // Conditionally add Tag Name column
            ...(selectedItem?.type === 'group' ? [{
                title: "Tag Name", field: "tag_name", widthGrow: 1, formatter: (cell) => {
                    const val = cell.getValue();
                    return `<span class="font-semibold text-blue-600 dark:text-blue-400">${val || ''}</span>`;
                }
            }] : []),
            { title: "File", field: "source.file_path", widthGrow: 2, formatter: (cell) => {
                const highlight = cell.getRow().getData();
                const filePath = highlight.source.file_path;
                const fileName = filePath.split(/[\\/]/).pop();
                const icon = getIconForFileType(highlight.source.file_type);
                const isDarkMode = document.documentElement.classList.contains('dark');
                const iconTextColor = (highlight.color && isDarkMode) ? '#111827' : 'currentColor';
                return `<div class=\"flex items-center space-x-2\" title=\"${filePath}\">
                            <div class="w-8 h-8 rounded-full flex items-center justify-center p-1 flex-shrink-0 aspect-square" style="background-color: ${highlight.color};">
                                <span style="color: ${iconTextColor};">${icon}</span>
                            </div>
                            <span>${fileName}</span>
                        </div>`;
            }},
            { title: "Content", field: "text", widthGrow: 5, formatter: (cell) => {
                const text = cell.getValue();
                return `<div class=\"whitespace-normal word-break-break-word\">${text}</div>`;
            }},
            { title: "Other Tags", field: "other_tags", widthGrow: 2, formatter: (cell) => {
                const tags = cell.getValue() || [];
                if (tags.length === 0) return '';
                const tagElements = tags.map(tag =>
                    `<span class=\"inline-block bg-gray-200 dark:bg-surface-3 rounded-full px-2 py-1 text-xs font-semibold text-gray-700 dark:text-text-primary mr-2 mb-1 border border-gray-300 dark:border-border\">${tag}</span>`
                ).join('');
                return `<div class=\"flex flex-wrap items-center\">${tagElements}</div>`;
            }},
            {
                title: "Actions", width: "10%", hozAlign: "center",
                formatter: (cell) => {
                    const highlight = cell.getRow().getData();
                    const commentCount = highlight.comments.length;
                    const commentPill = commentCount > 0 ? `<span class=\"absolute -top-1 -right-1 bg-blue-500 text-white text-xs rounded-full h-4 w-4 flex items-center justify-center\">${commentCount}</span>` : '';
                    return `<div class=\"flex items-center\">
                            <button title=\"Inspect\" class=\"mr-4 hover:bg-gray-200 dark:hover:bg-gray-600 rounded-full p-1\">
                                <svg xmlns=\"http://www.w3.org/2000/svg\" width=\"16\" height=\"16\" fill=\"currentColor\" class=\"bi bi-eye\" viewBox=\"0 0 16 16\">
                                    <path d="M16 8s-3-5.5-8-5.5S0 8 0 8s3 5.5 8 5.5S16 8 16 8M1.173 8a13 13 0 0 1 1.66-2.043C4.12 4.668 5.88 3.5 8 3.5s3.879 1.168 5.168 2.457A13 13 0 0 1 14.828 8q-.086.13-.195.288c-.335.48-.83 1.12-1.465 1.755C11.879 11.332 10.119 12.5 8 12.5s-3.879-1.168-5.168-2.457A13 13 0 0 1 1.172 8z"/>
                                    <path d="M8 5.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5M4.5 8a3.5 3.5 0 1 1 7 0 3.5 3.5 0 0 1-7 0"/>
                                </svg>
                            </button>
                            <button title=\"Comments\" class=\"relative p-1 rounded-full hover:bg-gray-200 dark:hover:bg-gray-600 mr-4\">
                                <svg xmlns=\"http://www.w3.org/2000/svg\" width=\"16\" height=\"16\" fill=\"currentColor\" class=\"bi bi-chat\" viewBox=\"0 0 16 16\">
                                    <path d="M2.678 11.894a1 1 0 0 1 .287.801 11 11 0 0 1-.398 2c1.395-.323 2.247-.697 2.634-.893a1 1 0 0 1 .71-.074A8 8 0 0 0 8 14c3.996 0 7-2.807 7-6s-3.004-6-7-6-7 2.808-7 6c0 1.468.617 2.83 1.678 3.894m-.493 3.905a22 22 0 0 1-.713.129c-.2.032-.352-.176-.273-.362a10 10 0 0 0 .244-.637l.003-.01c.248-.72.45-1.548.524-2.319C.743 11.37 0 9.76 0 8c0-3.866 3.582-7 8-7s8 3.134 8 7-3.582 7-8 7a9 9 0 0 1-2.347-.306c-.52.263-1.639.742-3.468 1.105"/>
                                </svg>
                                ${commentPill}
                            </button>
                            <button title="Untag">
                                <svg xmlns="http://www.w3.org/2000/svg" fill="none" stroke="red" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" class="bi bi-tag-slash w-4 h-4" viewBox="0 0 16 16">
                                  <path d="M2 1a1 1 0 0 0-1 1v4.586a1 1 0 0 0 .293.707l7 7a1 1 0 0 0 1.414 0l4.586-4.586a1 1 0 0 0 0-1.414l-7-7A1 1 0 0 0 6.586 1zm4 3.5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0"></path>
                                  <path d="M15.5 0.5 L 0.5 15.5"></path>
                                </svg>
                            </button>
                        </div>`;
                },
                cellClick: (e, cell) => {
                    const highlight = cell.getRow().getData();
                    const target = e.target.closest('button');
                    if (!target) return;
                    if (target.title === 'Comments') {
                        showComments(highlight);
                    } else if (target.title === 'Untag') {
                        handleUntag(highlight);
                    } else if (target.title === 'Inspect') {
                        handleInspect(highlight);
                    }
                }
            }
        ];

        tabulatorInstance = new Tabulator(tableContainer, {
            data: data,
            layout: "fitColumns",
            pagination: "local",
            paginationSize: 10,
            paginationAddRow: "table",
            initialFilter: [{field:"text", type:"like", value:searchQuery}],
            columns: columns,
            height: "100%",
            placeholder: "No highlights.",
        });
        tabulatorInstance.on("tableBuilt", () => {
            tableReady = true;
        });
    }

    // --- Modal Handlers ---

    function openAddTagModal() {
        isAddTagModalOpen = true;
        showMenu = false;
    }

    function openAddTagGroupModal() {
        isAddTagGroupModalOpen = true;
        showMenu = false;
    }

    async function handleAddTag(event) {
        const { name, description } = event.detail;
        await addTag(name, description);
        isAddTagModalOpen = false;
    }

    async function handleAddTagGroup(event) {
        const { name, description } = event.detail;
        await addTagGroup(name, description);
        isAddTagGroupModalOpen = false;
    }

    async function handleSaveTag(event) {
        const { id, name, description } = event.detail;
        if (selectedItem.type === 'tag') {
            await updateTag(id, name, null, description);
        } else if (selectedItem.type === 'group') {
            await updateTagGroup(id, name, description);
        }
        isEditModalOpen = false;
        await handleSelect(selectedItem); // Refresh info
    }

    async function handleDeleteTagFromModal(event) {
        const { id } = event.detail;
        if (selectedItem.type === 'tag') {
            await deleteTag(id);
        } else if (selectedItem.type === 'group') {
            await deleteTagGroup(id);
        }
        isEditModalOpen = false;
        selectedItem = null;
        infoData = null;
    }

    // --- DnD Handlers ---

    function handleRootConsider(e) {
        dndItems = e.detail.items;
    }

    async function handleRootFinalize(e) {
        dndItems = e.detail.items;
        // Logic to detect changes if needed, but for "Unassigned -> Unassigned" or "Group -> Group" (reorder), we don't persist order yet.
        // But if an item changed from Group to Root?
        // Wait, dndzone on root only handles items at root level.
        // If I drag a tag OUT of a group into root, it should trigger here?
        // No, `svelte-dnd-action` doesn't automatically handle moving between zones unless you handle `dropFromOthers`?
        // Actually, dndzone handles drops from other zones if they share type?
        // Yes, by default.
        // So if I drag a tag from a group (inner zone) to root (outer zone), `handleRootFinalize` is called with new items.
        // I need to find which item was added/moved.

        // However, `dndItems` structure is complex (Groups and Tags).
        // If I drop a Tag into Root, `dndItems` will have that Tag.
        // I need to detect this and call `moveTagToGroup(tagId, null)`.

        // Iterating to find items that are tags and have a groupID that is NOT null (meaning they came from a group)
        // or items that are tags and were in a group in store?

        // This logic can be tricky. A simpler way:
        // When `finalize` happens, look at the items.
        // If an item is a tag (type='tag') and its store state says it belongs to a group,
        // but now it is in root list -> Move to NULL group.

        for (const item of dndItems) {
            if (item.type === 'tag') {
                const originalTag = $allTags.find(t => t.id === item.realId);
                if (originalTag && originalTag.tag_group_id !== null) {
                    // It was in a group, now in root.
                    await moveTagToGroup(item.realId, null);
                }
            }
        }
    }

    function handleGroupConsider(e, groupId) {
        // Update the specific group's items in dndItems
        const groupIndex = dndItems.findIndex(i => i.id === groupId);
        if (groupIndex !== -1) {
            dndItems[groupIndex].items = e.detail.items;
            dndItems = [...dndItems];
        }
    }

    async function handleGroupFinalize(e, groupRealId, groupId) {
        const groupIndex = dndItems.findIndex(i => i.id === groupId);
        if (groupIndex !== -1) {
            dndItems[groupIndex].items = e.detail.items;
            dndItems = [...dndItems];

            // Check for tags that moved INTO this group
            for (const item of e.detail.items) {
                if (item.type === 'tag') {
                    const originalTag = $allTags.find(t => t.id === item.realId);
                    // If tag had different group (or null), update it
                    if (originalTag && originalTag.tag_group_id !== groupRealId) {
                        await moveTagToGroup(item.realId, groupRealId);
                    }
                }
            }
        }
    }

    // --- Misc ---
    function showComments(highlight) {
        selectedHighlight = highlight;
        isCommentsPanelOpen = true;
    }

    async function handleUntag(item) {
        // Logic for untagging.
        // If we are in Tag view, removing that tag removes the row.
        // If in Group view, removing the tag removes the row (as it no longer belongs to the group via that tag).
        // `item.tag_name` tells us which tag it is?
        // Wait, `item` in table data has `tag_name` if group view.
        // If I click untag, I should remove THAT tag.

        let tagToRemove = selectedItem.name; // Default for Tag view
        if (selectedItem.type === 'group') {
            if (!item.tag_name) {
                console.error("Cannot untag: missing tag name in group view row.");
                return;
            }
            tagToRemove = item.tag_name;
        }

        const confirmed = await confirm(`Remove tag "${tagToRemove}" from this highlight?`);
        if (!confirmed) return;

        try {
            await invoke('remove_tag_from_highlight', {
                projectId: $project.id,
                highlightId: item.id,
                tagToRemove: tagToRemove,
                filePath: item.source.file_path,
                docType: item.source.original_doc_type || item.source.file_type,
            });
            await handleSelect(selectedItem); // Refresh
        } catch (error) {
            console.error("Failed to remove tag:", error);
        }
    }

    function handleInspect(highlight) {
        // Trigger event to open the file in Data tab.
        // Ideally we would also scroll to the highlight, but that requires more complex plumbing.
        // For now, opening the file is the first step.
        // ProjectView handles 'requestopentab' with 'loadNotePath'.
        dispatch('requestopentab', {
            tabName: 'data',
            loadNotePath: highlight.source.file_path
        });
    }

    async function handleCommentAction(event) {
        // ... (Similar to original, adapting for new structure)
        const { type, detail } = event;
        const { highlightId, commentId, newText, comment } = detail;
        // ... (rest of logic same as before, just updating processedHighlights)
        // For brevity, using the previous logic structure:
        const highlightToUpdate = processedHighlights.find(h => h.id === highlightId);
        if (!highlightToUpdate) return;

        let docType = highlightToUpdate.source.file_type;
        if (docType === 'document' && highlightToUpdate.source.file_path.toLowerCase().endsWith('.pdf')) docType = 'pdf';

        let newComments;
        if (type === 'addcomment') newComments = [...(highlightToUpdate.comments || []), comment];
        else if (type === 'deletecomment') newComments = (highlightToUpdate.comments || []).filter(c => c.id !== commentId && c.parentId !== commentId);
        else if (type === 'editcomment') newComments = (highlightToUpdate.comments || []).map(c => c.id === commentId ? { ...c, text: newText, updatedAt: new Date().toISOString() } : c);

        const updatedHighlight = { ...highlightToUpdate, comments: newComments };
        selectedHighlight = updatedHighlight; // update panel

        const idx = processedHighlights.findIndex(h => h.id === highlightId);
        if (idx !== -1) {
            processedHighlights[idx] = updatedHighlight;
            processedHighlights = [...processedHighlights]; // trigger reactivity
        }

        if (type === 'addcomment') addCommentToHighlight(highlightId, comment, docType);
        else if (type === 'deletecomment') deleteComment(highlightId, commentId, docType);
        else if (type === 'editcomment') updateComment(highlightId, commentId, newText, docType);

        try {
            await projectService.saveHighlightChanges(updatedHighlight);
        } catch (e) { console.error(e); }
    }

</script>

<svelte:window on:click={closeMenu} />

<div class="flex flex-col h-full w-full bg-gray-100 dark:bg-dark-bg-primary overflow-hidden">
    <div class="flex h-full w-full divide-x divide-gray-300 dark:divide-border">

        <!-- Left Panel -->
        {#if !$panelStateStore.tagsLeftPanelCollapsed}
        <div class="w-64 flex-shrink-0 h-full bg-white dark:bg-surface-2 p-4 flex flex-col" transition:slide={{ axis: 'x' }}>
            <div class="flex justify-between items-center mb-4">
                <h2 class="text-sm font-semibold dark:text-text-primary">All Tags</h2>
                <div class="relative">
                    <button bind:this={menuButton} on:click|stopPropagation={toggleMenu} class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">
                        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-three-dots-vertical" viewBox="0 0 16 16">
                            <path d="M9.5 13a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0m0-5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0m0-5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0"/>
                        </svg>
                    </button>
                    {#if showMenu}
                        <div bind:this={menuDropdown} class="absolute right-0 mt-2 w-40 bg-white dark:bg-surface-3 rounded-md shadow-lg z-10 border border-gray-200 dark:border-border">
                            <button on:click={openAddTagModal} class="block w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-text-primary hover:bg-gray-100 dark:hover:bg-dark-bg-tertiary">Add Tag</button>
                            <button on:click={openAddTagGroupModal} class="block w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-text-primary hover:bg-gray-100 dark:hover:bg-dark-bg-tertiary">Add Tag Group</button>
                        </div>
                    {/if}
                </div>
            </div>

            <!-- Draggable List -->
            <div class="flex-grow overflow-y-auto" use:dndzone={{items: dndItems, flipDurationMs}} on:consider={handleRootConsider} on:finalize={handleRootFinalize}>
                {#each dndItems as item (item.id)}
                    <div animate:flip={{duration: flipDurationMs}}>
                        {#if item.type === 'group'}
                            <div class="mb-2">
                                <div
                                    class="p-2 rounded-md cursor-pointer flex items-center font-bold text-gray-700 dark:text-text-primary hover:bg-gray-100 dark:hover:bg-dark-bg-tertiary"
                                    class:bg-blue-100={selectedItem?.id === item.id}
                                    class:dark:bg-blue-900={selectedItem?.id === item.id}
                                    on:click={() => handleSelect(item)}
                                >
                                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-folder mr-2" viewBox="0 0 16 16">
                                        <path d="M.54 3.87.5 3a2 2 0 0 1 2-2h3.672a2 2 0 0 1 1.414.586l.828.828A2 2 0 0 0 9.828 3h3.982a2 2 0 0 1 1.992 2.181l-.637 7A2 2 0 0 1 13.174 14H2.826a2 2 0 0 1-1.991-1.819l-.637-7a1.99 1.99 0 0 1 .342-1.31zM2.19 4a1 1 0 0 0-.996 1.09l.637 7a1 1 0 0 0 .995.91h10.348a1 1 0 0 0 .995-.91l.637-7A1 1 0 0 0 13.81 4H2.19zm4.69-1.707A1 1 0 0 0 6.172 2H2.5a1 1 0 0 0-1 .981l.006.139C1.72 3.042 1.95 3 2.19 3h5.396l-.707-.707z"/>
                                    </svg>
                                    {item.name}
                                </div>
                                <div class="ml-4 min-h-[10px] border-l-2 border-gray-200 dark:border-border pl-2" use:dndzone={{items: item.items, flipDurationMs}} on:consider={(e) => handleGroupConsider(e, item.id)} on:finalize={(e) => handleGroupFinalize(e, item.realId, item.id)}>
                                    {#each item.items as child (child.id)}
                                        <div
                                            animate:flip={{duration: flipDurationMs}}
                                            class="p-2 rounded-md cursor-pointer hover:bg-gray-100 dark:hover:bg-dark-bg-tertiary text-xs"
                                            class:bg-blue-200={selectedItem?.id === child.id}
                                            class:dark:bg-blue-800={selectedItem?.id === child.id}
                                            on:click|stopPropagation={() => handleSelect(child)}
                                        >
                                            <span class:dark:!text-blue-200={selectedItem?.id === child.id} class="dark:text-text-secondary">{child.name}</span>
                                        </div>
                                    {/each}
                                </div>
                            </div>
                        {:else}
                            <div
                                class="p-2 rounded-md cursor-pointer hover:bg-gray-100 dark:hover:bg-dark-bg-tertiary text-xs mb-1"
                                class:bg-blue-200={selectedItem?.id === item.id}
                                class:dark:bg-blue-800={selectedItem?.id === item.id}
                                on:click={() => handleSelect(item)}
                            >
                                <span class:dark:!text-blue-200={selectedItem?.id === item.id} class="dark:text-text-secondary">{item.name}</span>
                            </div>
                        {/if}
                    </div>
                {/each}
            </div>
        </div>
        {/if}

        <!-- Middle Panel -->
        <div class="h-full flex flex-col p-4 gap-4 flex-1 bg-white dark:bg-surface-1">
            {#if selectedItem}
                {#if isLoading}
                    <p class="dark:text-text-primary">Loading information...</p>
                {:else if infoData}
                    <div class="h-[20%] flex flex-col">
                        <div class="flex items-center space-x-2">
                            <h2 class="text-xl font-bold dark:text-white">{infoData.name}</h2>
                            <button on:click={() => isEditModalOpen = true} class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">
                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-pencil-square w-4 h-4" viewBox="0 0 16 16"><path d="M15.502 1.94a.5.5 0 0 1 0 .706L14.459 3.69l-2-2L13.502.646a.5.5 0 0 1 .707 0l1.293 1.293zm-1.75 2.456-2-2L4.939 9.21a.5.5 0 0 0-.121.196l-.805 2.414a.25.25 0 0 0 .316.316l2.414-.805a.5.5 0 0 0 .196-.12l6.813-6.814z"></path><path fill-rule="evenodd" d="M1 13.5A1.5 1.5 0 0 0 2.5 15h11a1.5 1.5 0 0 0 1.5-1.5v-6a.5.5 0 0 0-1 0v6a.5.5 0 0 1-.5.5h-11a.5.5 0 0 1-.5-.5v-11a.5.5 0 0 1 .5-.5H9a.5.5 0 0 0 0-1H2.5A1.5 1.5 0 0 0 1 2.5z"></path></svg>
                            </button>
                        </div>
                        <div class="mt-2">
                            {#if description}
                                <p class="text-sm text-gray-600 dark:text-white whitespace-pre-wrap">{description}</p>
                            {:else}
                                <p class="text-sm text-gray-500 italic dark:text-white">No description provided.</p>
                            {/if}
                        </div>
                    </div>

                    <div class="h-[75%] flex flex-col">
                        <div class="flex justify-between items-center mb-2 flex-shrink-0">
                            <h3 class="text-lg font-semibold dark:text-white">Highlights ({infoData.highlight_count})</h3>
                            <input type="text" placeholder="Search content..." bind:value={searchQuery} on:input={handleSearch} class="border rounded px-2 py-1 text-sm dark:bg-surface-3 dark:border-border dark:text-text-primary">
                        </div>
                        <div class="flex-grow overflow-auto border border-gray-300 dark:border-border rounded-md" bind:this={tableContainer}></div>
                    </div>
                {:else}
                    <div class="flex items-center justify-center h-full">
                        <p class="text-gray-500">Select an item to view its details.</p>
                    </div>
                {/if}
            {:else}
                <div class="flex items-center justify-center h-full">
                    <p class="text-gray-500">Select a tag or group to view its details.</p>
                </div>
            {/if}
        </div>
    </div>

    <!-- Right Panel (Comments) -->
    {#if isCommentsPanelOpen}
        <div class="fixed inset-0 bg-black bg-opacity-50 z-30" on:click={() => isCommentsPanelOpen = false}></div>
        <div class="fixed top-4 right-4 bottom-4 w-1/3 bg-gray-50 dark:bg-gray-700 p-4 border border-gray-200 dark:border-gray-600 overflow-y-auto shadow-lg z-40 rounded-lg" transition:slide={{ duration: 300, axis: 'x' }}>
            {#if selectedHighlight}
                <CommentsPanel
                    comments={selectedHighlight.comments || []}
                    highlightId={selectedHighlight.id}
                    on:addcomment={handleCommentAction}
                    on:deletecomment={handleCommentAction}
                    on:editcomment={handleCommentAction}
                    on:close={() => isCommentsPanelOpen = false}
                />
            {/if}
        </div>
    {/if}

    <!-- Modals -->
    <AddTagModal showModal={isAddTagModalOpen} on:close={() => isAddTagModalOpen = false} on:save={handleAddTag} />
    <AddTagGroupModal showModal={isAddTagGroupModalOpen} on:close={() => isAddTagGroupModalOpen = false} on:save={handleAddTagGroup} />
    {#if isEditModalOpen && selectedItem}
        <EditTagModal
            showModal={isEditModalOpen}
            tag={{ id: selectedItem.realId, name: selectedItem.name, description: description }}
            on:close={() => isEditModalOpen = false}
            on:save={handleSaveTag}
            on:delete={handleDeleteTagFromModal}
        />
    {/if}
</div>
