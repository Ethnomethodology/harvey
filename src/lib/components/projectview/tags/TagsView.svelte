<!-- src/lib/components/projectview/tags/TagsView.svelte -->
<script>
    import { onMount, afterUpdate, onDestroy, createEventDispatcher } from 'svelte';
    import { slide } from 'svelte/transition';
    import { invoke } from '@tauri-apps/api/core';
    import { confirm } from '@tauri-apps/plugin-dialog';
    import { TabulatorFull as Tabulator } from 'tabulator-tables';
    import { dndzone } from 'svelte-dnd-action';
    import { project } from '$lib/stores/projectStore.js';
    import {
        allTags, allTagGroups,
        updateTag, deleteTag,
        createTagGroup, updateTagGroup, deleteTagGroup,
        fetchAllTags,
        selectedTag, selectedTagGroup, tagInfo, tagSearchQuery,
        selectTag, selectTagGroup,
        addTag
    } from '$lib/stores/tagStore.js';
    import { refresher } from '$lib/stores/refresherStore.js';
    import { addCommentToHighlight, deleteComment, updateComment } from '$lib/stores/projectStore.js';
    import * as projectService from '$lib/services/projectService.js';
    import SimpleTopBar from '../shared/SimpleTopBar.svelte';
    import CommentsPanel from './CommentsPanel.svelte';
    import EditTagModal from '../modals/EditTagModal.svelte';
    import AddTagModal from '../modals/AddTagModal.svelte';
    import AddTagGroupModal from '../modals/AddTagGroupModal.svelte';
    import EditTagGroupModal from '../modals/EditTagGroupModal.svelte';
    import panelStateStore from '$lib/stores/panelStateStore.js';
    import { get } from 'svelte/store';
    import { MoreVertical, SquarePen, Eye, MessageCircle, Sheet, Music, Film, FileText, Image as ImageIcon, MessageSquareText, CircleHelp } from 'lucide-svelte';

    let unsubscribePanelState;
    let unsubscribeRefresher;

    // UI State for Modals
    let isEditTagModalOpen = false;
    let isAddTagModalOpen = false;
    let isAddGroupModalOpen = false;
    let isEditGroupModalOpen = false;
    let isCommentsPanelOpen = false;

    let isLoading = false;

    // UI State for Context Menu / Dropdown
    let showAddMenu = false;

    // Exported methods for parent control
    export function openAddTagModal() {
        isAddTagModalOpen = true;
    }

    export function openAddGroupModal() {
        isAddGroupModalOpen = true;
    }

    // Derived Data for Display
    let groups = [];
    let ungroupedTags = [];

    // Internal DnD State
    const flipDurationMs = 200;
    let isDragging = false; // Flag to prevent store updates from overwriting local state during drag

    $: if (!isDragging) {
        // Rebuild structure when stores change
        // We need to map tags to groups
        const groupMap = new Map($allTagGroups.map(g => [g.id, { ...g, tags: [] }]));
        const _ungrouped = [];

        // Sort tags by name first to ensure stable initial order
        const sortedTags = [...$allTags].sort((a, b) => a.name.localeCompare(b.name));

        sortedTags.forEach(tag => {
            if (tag.tag_group_id && groupMap.has(tag.tag_group_id)) {
                groupMap.get(tag.tag_group_id).tags.push(tag);
            } else {
                _ungrouped.push(tag);
            }
        });

        groups = Array.from(groupMap.values()).sort((a, b) => a.name.localeCompare(b.name));
        ungroupedTags = _ungrouped;
    }

    onMount(async () => {
        await fetchAllTags();

        const currentTag = get(selectedTag);
        const currentGroup = get(selectedTagGroup);
        if (currentTag) {
            await selectTag(currentTag);
        } else if (currentGroup) {
            await selectTagGroup(currentGroup);
        }

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
            // Validate selection
            if ($selectedTag && !$allTags.some(t => t.id === $selectedTag.id)) {
                selectTag(null);
            }
            if ($selectedTagGroup && !$allTagGroups.some(g => g.id === $selectedTagGroup.id)) {
                selectTagGroup(null);
            }
        });

        // Click outside to close menu
        if (typeof window !== 'undefined') {
            window.addEventListener('click', closeAddMenu);
        }
    });

    onDestroy(() => {
        if (unsubscribePanelState) unsubscribePanelState();
        if (unsubscribeRefresher) unsubscribeRefresher();
        if (typeof window !== 'undefined') {
            window.removeEventListener('click', closeAddMenu);
        }
    });

    function closeAddMenu(e) {
        // If click is inside the menu button or menu itself, ignore
        // We need to use closest because e.target might be the SVG inside the button
        if (showAddMenu) {
            const menu = document.getElementById('add-tag-menu');
            const btn = document.getElementById('add-tag-btn');

            const clickedInsideMenu = menu && menu.contains(e.target);
            const clickedButton = btn && (btn === e.target || btn.contains(e.target));

            if (!clickedInsideMenu && !clickedButton) {
                showAddMenu = false;
            }
        }
    }

    import { mount, unmount } from 'svelte';

    // --- Icons ---
    function getIconComponentForFileType(fileType) {
        switch (fileType) {
            case 'audio': return Music;
            case 'video': return Film;
            case 'document': return FileText;
            case 'image': return ImageIcon;
            case 'table': return Sheet;
            case 'transcript': return MessageSquareText;
            case 'imported_transcript': return MessageSquareText;
            case 'audio_transcript': return Music;
            case 'video_transcript': return Film;
            default: return CircleHelp;
        }
    }

    // --- Table & Data Logic ---
    let description = '';
    let tableContainer;
    let tabulatorInstance = null;
    let tableReady = false;
    let processedHighlights = [];
    let selectedHighlight = null;

    const dispatch = createEventDispatcher();

    // Redraw table on panel resize/collapse
    $: if ($panelStateStore.tagsLeftPanelCollapsed !== undefined && tabulatorInstance && tableReady) {
        if (tableContainer) {
            tabulatorInstance.redraw(true);
            window.dispatchEvent(new Event('resize'));
        }
    }

    // Process highlights when tagInfo changes
    $: {
        if ($tagInfo && $tagInfo.highlights) {
            processedHighlights = $tagInfo.highlights.map(item => {
                const highlight = item.highlight || item;
                return {
                    ...highlight,
                    other_tags: highlight.other_tags || [],
                    comments: highlight.comments || [],
                };
            });
        } else {
            processedHighlights = [];
        }
    }

    // React to tagInfo or selection changes to update table
    $: {
        if ($tagInfo && tableContainer) {
            const isGroupView = !!$selectedTagGroup;
            // If table doesn't exist, create it
            if (!tabulatorInstance) {
                initializeTable(processedHighlights);
            } else if (tableReady) {
                // Check if we need to switch column structure
                // We rely on a custom property 'isGroupView' on the instance or check columns
                // But getting columns can be buggy if not ready.
                // Simplest is to check if we switched modes.
                // We can store the current mode in a variable
                if (currentTableMode !== (isGroupView ? 'group' : 'tag')) {
                    initializeTable(processedHighlights);
                } else {
                    // Just update data
                    tabulatorInstance.replaceData(processedHighlights)
                        .then(() => {
                            if (tabulatorInstance) {
                                tabulatorInstance.redraw();
                            }
                        })
                        .catch(err => console.error("Table update failed", err));
                }
            }
        } else if (!$tagInfo && tabulatorInstance) {
            // Cleanup if no tag selected
            tabulatorInstance.destroy();
            tabulatorInstance = null;
            tableReady = false;
            currentTableMode = null;
        }
    }

    let currentTableMode = null; // 'tag' or 'group'

    function initializeTable(data) {
        if (tabulatorInstance) {
            tabulatorInstance.destroy();
            tabulatorInstance = null;
        }
        tableReady = false;

        const isGroupView = !!$selectedTagGroup;
        currentTableMode = isGroupView ? 'group' : 'tag';

        let columns = [
            { title: "File", field: "source.file_path", widthGrow: 2, formatter: (cell, formatterParams, onRendered) => {
                const highlight = cell.getRow().getData();
                const filePath = highlight.source.file_path;
                const fileName = filePath.split(/[\\/]/).pop();
                const IconComponent = getIconComponentForFileType(highlight.source.file_type);
                const isDarkMode = document.documentElement.classList.contains('dark');
                const iconTextColor = (highlight.color && isDarkMode) ? '#111827' : 'currentColor';

                const container = document.createElement("div");
                container.className = "flex items-center space-x-2";
                container.title = filePath;

                const iconContainer = document.createElement("div");
                iconContainer.className = "w-8 h-8 rounded-full flex items-center justify-center p-1 flex-shrink-0 aspect-square";
                iconContainer.style.backgroundColor = highlight.color || 'transparent';
                iconContainer.style.color = iconTextColor;

                const iconSpan = document.createElement("span");
                mount(IconComponent, { target: iconSpan, props: { size: 16 } });
                iconContainer.appendChild(iconSpan);

                const textSpan = document.createElement("span");
                textSpan.textContent = fileName;

                container.appendChild(iconContainer);
                container.appendChild(textSpan);

                return container;
            }},
            { title: "Content", field: "text", widthGrow: 5, formatter: (cell) => {
                const text = cell.getValue();
                return `<div class=\"whitespace-normal word-break-break-word\">${text}</div>`;
            }},
        ];

        if (isGroupView) {
            columns.push({
                title: "Tag Name", field: "tags", widthGrow: 2, formatter: (cell) => {
                    const allTagsOnHighlight = cell.getValue() || [];
                    if (!$selectedTagGroup) return '';

                    const currentGroupTags = $allTags.filter(t => t.tag_group_id === $selectedTagGroup.id).map(t => t.name);
                    const matchingTags = allTagsOnHighlight.filter(t => currentGroupTags.includes(t));

                    if (matchingTags.length === 0) return '';

                    return matchingTags.map(tag =>
                        `<span class=\"inline-block bg-blue-100 dark:bg-blue-900 rounded-full px-2 py-1 text-xs font-semibold text-blue-800 dark:text-blue-100 mr-2 mb-1 border border-blue-200 dark:border-blue-800\">${tag}</span>`
                    ).join('');
                }
            });
        }

        columns.push({ title: "Other Tags", field: "other_tags", widthGrow: 2, formatter: (cell) => {
            const tags = cell.getValue() || [];
            if (tags.length === 0) return '';

            const tagElements = tags.map(tag =>
                `<span class=\"inline-block bg-gray-200 dark:bg-gray-800 rounded-full px-2 py-1 text-xs font-semibold text-gray-700 dark:text-gray-200 mr-2 mb-1 border border-gray-300 dark:border-gray-700\">${tag}</span>`
            ).join('');

            return `<div class=\"flex flex-wrap items-center\">${tagElements}</div>`;
        }});

        columns.push({
            title: "Actions", width: "10%", hozAlign: "center",
            formatter: (cell, formatterParams, onRendered) => {
                const highlight = cell.getRow().getData();
                const commentCount = highlight.comments.length;

                const container = document.createElement("div");
                container.className = "flex items-center justify-center gap-4";

                const inspectBtn = document.createElement("button");
                inspectBtn.title = "Inspect";
                mount(Eye, { target: inspectBtn, props: { size: 16 } });

                const commentsBtn = document.createElement("button");
                commentsBtn.title = "Comments";
                commentsBtn.className = "relative p-1 rounded-full hover:bg-gray-200 dark:hover:bg-gray-600";
                mount(MessageCircle, { target: commentsBtn, props: { size: 16 } });
                if (commentCount > 0) {
                    const pill = document.createElement("span");
                    pill.className = "absolute -top-1 -right-1 bg-blue-500 text-white text-xs rounded-full h-4 w-4 flex items-center justify-center";
                    pill.textContent = commentCount;
                    commentsBtn.appendChild(pill);
                }

                const untagBtn = document.createElement("button");
                untagBtn.title = "Untag";
                untagBtn.innerHTML = `
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" stroke="red" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" class="bi bi-tag-slash w-4 h-4" viewBox="0 0 16 16">
                      <path d="M2 1a1 1 0 0 0-1 1v4.586a1 1 0 0 0 .293.707l7 7a1 1 0 0 0 1.414 0l4.586-4.586a1 1 0 0 0 0-1.414l-7-7A1 1 0 0 0 6.586 1zm4 3.5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0"></path>
                      <path d="M15.5 0.5 L 0.5 15.5"></path>
                    </svg>
                `;

                container.appendChild(inspectBtn);
                container.appendChild(commentsBtn);
                container.appendChild(untagBtn);

                return container;
            },
            cellClick: (e, cell) => {
                const highlight = cell.getRow().getData();
                const target = e.target.closest('button');
                if (!target) return;
                if (target.title === 'Inspect') {
                    handleInspect(highlight);
                } else if (target.title === 'Comments') {
                    showComments(highlight);
                } else if (target.title === 'Untag') {
                    handleUntag(highlight);
                }
            }
        });

        tabulatorInstance = new Tabulator(tableContainer, {
            data: data,
            layout: "fitColumns",
            pagination: "local",
            paginationSize: 10,
            paginationAddRow: "table",
            initialFilter: [
                {field:"text", type:"like", value:$tagSearchQuery}
            ],
            columns: columns,
            height: "100%",
            placeholder: "No highlights for this tag.",
        });
        tabulatorInstance.on("tableBuilt", () => {
            tableReady = true;
        });
    }

    // --- Modal Handlers ---
    async function handleAddTag(event) {
        const { name, description } = event.detail;
        try {
            await addTag(name, description, null);
            isAddTagModalOpen = false;
        } catch (error) {
            console.error("Failed to add tag:", error);
        }
    }

    async function handleAddGroup(event) {
        const { name, description } = event.detail;
        try {
            await createTagGroup(name, description);
            isAddGroupModalOpen = false;
        } catch (error) {
            console.error("Failed to add group:", error);
        }
    }

    async function handleEditGroup(event) {
        const { id, name, description } = event.detail;
        try {
            await updateTagGroup(id, name, description);
            isEditGroupModalOpen = false;
            // Refresh selection if current group was updated
            if ($selectedTagGroup && $selectedTagGroup.id === id) {
                const updatedGroup = $allTagGroups.find(g => g.id === id);
                selectTagGroup(updatedGroup);
            }
        } catch (error) {
            console.error("Failed to update group:", error);
        }
    }

    async function handleDeleteGroup(event) {
        const { id } = event.detail;
        try {
            await deleteTagGroup(id);
            isEditGroupModalOpen = false;
            selectTagGroup(null);
        } catch (error) {
            console.error("Failed to delete group:", error);
        }
    }

    async function handleSaveTag(event) {
        const { id, name, description } = event.detail;
        // Use existing color and group if not provided (modal doesn't edit color/group currently)
        const currentColor = $selectedTag ? $selectedTag.color : null;
        const currentGroupId = $selectedTag ? $selectedTag.tag_group_id : null;
        try {
            await updateTag(id, name, currentColor, description, currentGroupId);
            isEditTagModalOpen = false;
        } catch (error) {
            console.error("Failed to update tag:", error);
        }
    }

    async function handleDeleteTagFromModal(event) {
        const { id } = event.detail;
        try {
            await deleteTag(id);
            isEditTagModalOpen = false;
            selectTag(null);
        } catch (error) {
            console.error("Failed to delete tag:", error);
        }
    }

    async function handleCommentAction(event) {
        const { type, highlightId, comment, commentId, text } = event.detail;
        const highlight = selectedHighlight;
        if (!highlight) return;

        try {
            if (type === 'add') {
                await addCommentToHighlight(highlightId, comment, highlight.source.file_type);
            } else if (type === 'update') {
                await updateComment(highlightId, commentId, text, highlight.source.file_type);
            } else if (type === 'delete') {
                await deleteComment(highlightId, commentId, highlight.source.file_type);
            }

            // Update local selectedHighlight to reflect changes in CommentsPanel
            // We need to find the updated highlight in our processedHighlights
            // Or just update the local object's comments array.
            // But projectStore functions update the store, so we should ideally reactive-ly get it.
            // For immediate UI update in the modal:
            if (tabulatorInstance) {
                // Find the row in Tabulator and refresh it
                const row = tabulatorInstance.getRow(highlightId);
                if (row) {
                    const rowData = row.getData();
                    // Sync the comments
                    // This is a bit manual because projectStore doesn't return the new array
                    // and we are working with a copy in Tabulator.
                    // But processedHighlights will update via reactivity ($tagInfo).
                    // Let's rely on the store subscription if possible.
                }
            }
        } catch (error) {
            console.error(`Failed to ${type} comment:`, error);
        }
    }

    function handleSearch() {
        if (tabulatorInstance) {
            tabulatorInstance.setFilter("text", "like", $tagSearchQuery);
        }
    }

    // --- DnD Handlers ---
    // For groups: dragging tags INTO a group or reordering within.
    function handleDndConsiderGroup(groupId, e) {
        isDragging = true;
        const idx = groups.findIndex(g => g.id === groupId);
        if (idx !== -1) {
            groups[idx].tags = e.detail.items;
            groups = [...groups]; // Trigger reactivity
        }
    }

    async function handleDndFinalizeGroup(groupId, e) {
        const idx = groups.findIndex(g => g.id === groupId);
        if (idx !== -1) {
            groups[idx].tags = e.detail.items;
            groups = [...groups];

            // Check for tags that moved into this group
            for (const tag of e.detail.items) {
                if (tag.tag_group_id !== groupId) {
                    try {
                        await updateTag(tag.id, tag.name, tag.color, tag.description, groupId);
                    } catch(err) {
                        console.error("Failed to move tag to group:", err);
                    }
                }
            }
        }
        isDragging = false;
    }

    // For ungrouped: dragging tags OUT of a group or reordering.
    function handleDndConsiderUngrouped(e) {
        isDragging = true;
        ungroupedTags = e.detail.items;
    }

    async function handleDndFinalizeUngrouped(e) {
        ungroupedTags = e.detail.items;

        for (const tag of e.detail.items) {
            if (tag.tag_group_id !== null) {
                try {
                    await updateTag(tag.id, tag.name, tag.color, tag.description, null);
                } catch(err) {
                    console.error("Failed to ungroup tag:", err);
                }
            }
        }
        isDragging = false;
    }

    // --- Selection ---
    function handleSelect(item, type) {
        if (type === 'group') {
            selectTagGroup(item);
        } else {
            selectTag(item);
        }
    }

    function openEditGroup(group, e) {
        e.stopPropagation();
        // Set selected group if not already (optional)
        isEditGroupModalOpen = true;
        // The modal needs the group object. We can bind it or pass it.
        // We'll use a temporary store or local var for the modal.
        // Re-using selectedTagGroup might be confusing if user didn't select it.
        // Let's assume we edit the passed group.
        // We need to pass 'group' prop to modal.
        currentEditingGroup = group;
    }

    let currentEditingGroup = null;

    // --- Action Handlers ---
    function handleInspect(highlight) {
        if (!highlight || !highlight.source) return;
        dispatch('requestviewchange', {
            tabName: 'data',
            loadNotePath: highlight.source.file_path,
            highlightId: highlight.id,
            viewType: highlight.source.file_type === 'audio' || highlight.source.file_type === 'video' ? 'media' : 
                      highlight.source.file_type === 'csv' ? 'table' :
                      highlight.source.file_type === 'image' ? 'image' : 
                      highlight.source.file_type === 'imported_transcript' ? 'transcript' : 'document',
            originalDocType: highlight.source.file_type
        });
    }

    function showComments(highlight) {
        selectedHighlight = highlight;
        isCommentsPanelOpen = true;
    }

    async function handleUntag(highlight) {
        // TODO: Implement untagging functionality.
        // This requires removing the tag from the specific annotation in the backend or updating the document.
        // For now, we'll log it and maybe show a message.
        console.warn('Untagging from Tags view is not yet fully implemented.', highlight);
        const doUntag = await confirm('Untagging from this view is not yet supported. Please inspect the file to remove the tag.', { title: 'Not Implemented', kind: 'info' });
        if (doUntag) {
             handleInspect(highlight);
        }
    }

</script>

<div class="flex flex-col h-full w-full bg-gray-100 dark:bg-gray-950 overflow-hidden">
    <div class="flex h-full w-full divide-x divide-gray-300 dark:divide-border">
        <!-- Left Panel: List of groups and tags -->
        {#if !$panelStateStore.tagsLeftPanelCollapsed}
        <div class="w-64 flex-shrink-0 h-full bg-white dark:bg-gray-900 flex flex-col" transition:slide={{ axis: 'x' }}>
            <!-- Header -->
            <h2 class="relative flex items-center justify-between text-sm font-semibold text-gray-700 dark:text-gray-400 px-1 h-9 border-b border-gray-200 dark:border-gray-800 mb-3">
                <div class="flex items-center space-x-2 transition-opacity duration-200">
                    <span class="pl-2">All Tags</span>
                </div>
                <button
                    id="add-tag-btn"
                    class="p-1 flex items-center justify-center text-gray-400 dark:text-gray-700 hover:text-gray-600 dark:hover:text-gray-400 transition-opacity duration-200"
                    on:click|stopPropagation={() => showAddMenu = !showAddMenu}
                    title="Options"
                >
                    <MoreVertical class="w-4 h-4" />
                </button>

                {#if showAddMenu}
                    <div id="add-tag-menu" class="absolute top-full right-0 w-40 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded shadow-lg z-50 py-1 font-normal">
                        <button
                            class="w-full text-left px-4 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-700 dark:text-white"
                            on:click={() => { showAddMenu = false; isAddTagModalOpen = true; }}
                        >
                            Add Tag
                        </button>
                        <button
                            class="w-full text-left px-4 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-700 dark:text-white"
                            on:click={() => { showAddMenu = false; isAddGroupModalOpen = true; }}
                        >
                            Add Tag Group
                        </button>
                    </div>
                {/if}
            </h2>

            <!-- List Content -->
            <div class="flex-1 overflow-y-auto p-2">
                <!-- Groups -->
                {#each groups as group (group.id)}
                    <div class="mb-2 rounded border border-transparent hover:border-gray-200 dark:hover:border-gray-600">
                        <!-- Group Header -->
                        <div
                            class="flex items-center justify-between px-2 py-1.5 cursor-pointer rounded hover:bg-gray-100 dark:hover:bg-gray-800"
                            class:bg-blue-100={$selectedTagGroup?.id === group.id}
                            class:dark:bg-blue-900={$selectedTagGroup?.id === group.id}
                            on:click={() => handleSelect(group, 'group')}
                        >
                            <div class="flex items-center">
                                <span class="font-semibold text-sm dark:text-white truncate"
                                    class:!text-blue-700={$selectedTagGroup?.id === group.id}
                                    class:dark:!text-blue-200={$selectedTagGroup?.id === group.id}
                                >{group.name}</span>
                            </div>
                        </div>

                        <!-- Group Tags (Indented) -->
                        <div
                            class="pl-4 pt-1 min-h-[2rem]"
                            use:dndzone={{items: group.tags, flipDurationMs}}
                            on:consider={(e) => handleDndConsiderGroup(group.id, e)}
                            on:finalize={(e) => handleDndFinalizeGroup(group.id, e)}
                        >
                            {#each group.tags as tag (tag.id)}
                                <div
                                    class="px-2 py-1.5 mb-1 rounded cursor-pointer text-xs flex items-center hover:bg-gray-100 dark:hover:bg-gray-800"
                                    class:bg-blue-100={$selectedTag?.id === tag.id}
                                    class:dark:bg-blue-900={$selectedTag?.id === tag.id}
                                    on:click|stopPropagation={() => handleSelect(tag, 'tag')}
                                >
                                    <span class="truncate dark:text-gray-400"
                                        class:!text-blue-700={$selectedTag?.id === tag.id}
                                        class:dark:!text-blue-200={$selectedTag?.id === tag.id}
                                    >{tag.name}</span>
                                </div>
                            {/each}
                        </div>
                    </div>
                {/each}

                <!-- Ungrouped Tags -->
                <div class="mt-4">
                    {#if groups.length > 0}
                        <h3 class="text-xs font-semibold text-gray-500 mb-2 uppercase px-2">Ungrouped Tags</h3>
                    {/if}
                    <div
                        class="min-h-[50px] p-2 rounded"
                        use:dndzone={{items: ungroupedTags, flipDurationMs}}
                        on:consider={handleDndConsiderUngrouped}
                        on:finalize={handleDndFinalizeUngrouped}
                    >
                        {#each ungroupedTags as tag (tag.id)}
                            <div
                                class="px-2 py-1.5 mb-1 rounded cursor-pointer text-xs flex items-center hover:bg-gray-100 dark:hover:bg-gray-800"
                                class:bg-blue-100={$selectedTag?.id === tag.id}
                                class:dark:bg-blue-900={$selectedTag?.id === tag.id}
                                on:click|stopPropagation={() => handleSelect(tag, 'tag')}
                            >
                                <span class="truncate dark:text-gray-400"
                                    class:!text-blue-700={$selectedTag?.id === tag.id}
                                    class:dark:!text-blue-200={$selectedTag?.id === tag.id}
                                >{tag.name}</span>
                            </div>
                        {/each}
                    </div>
                </div>
            </div>
        </div>
        {/if}

        <!-- Middle Panel -->
        <div class="h-full flex flex-col p-4 gap-4 flex-1 bg-white dark:bg-gray-950">
            {#if $selectedTag || $selectedTagGroup}
                {#if isLoading}
                    <p class="dark:text-gray-200">Loading information...</p>
                {:else if $tagInfo}
                    <div class="h-[20%] flex flex-col">
                        <div class="flex items-center space-x-2">
                            <h2 class="text-xl font-bold dark:text-white">{$tagInfo.name}</h2>
                            {#if $selectedTag}
                                <button on:click={() => isEditTagModalOpen = true} class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">
                                    <SquarePen class="w-4 h-4" />
                                </button>
                            {:else if $selectedTagGroup}
                                <button on:click={() => openEditGroup($selectedTagGroup, {stopPropagation:()=>{}})} class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">
                                    <SquarePen class="w-4 h-4" />
                                </button>
                            {/if}
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
                            <h3 class="text-lg font-semibold dark:text-white">Highlights ({$tagInfo.highlight_count})</h3>
                            <input type="text" placeholder="Search content..." bind:value={$tagSearchQuery} on:input={handleSearch} on:keydown={e => { if (e.key === 'Enter') { e.preventDefault(); e.stopPropagation(); } }} class="border rounded px-2 py-1 text-sm dark:bg-gray-800 dark:border-gray-700 dark:text-gray-200" autocomplete="off" autocorrect="off">
                        </div>
                        <div class="flex-grow relative border border-gray-300 dark:border-gray-700 rounded-md">
                            <div class="w-full h-full" bind:this={tableContainer}></div>
                        </div>
                    </div>
                {:else}
                    <div class="flex items-center justify-center h-full">
                        <p class="text-gray-500">Select a tag or group to view its details.</p>
                    </div>
                {/if}
            {:else}
                <div class="flex items-center justify-center h-full">
                    <p class="text-gray-500">Select a tag or group to view its details.</p>
                </div>
            {/if}
        </div>
    </div>

    <!-- Modals -->
    <AddTagModal
        showModal={isAddTagModalOpen}
        on:close={() => isAddTagModalOpen = false}
        on:save={handleAddTag}
    />
    <AddTagGroupModal
        showModal={isAddGroupModalOpen}
        on:close={() => isAddGroupModalOpen = false}
        on:save={handleAddGroup}
    />
    {#if isEditTagModalOpen && $selectedTag}
        <EditTagModal
            showModal={isEditTagModalOpen}
            tag={$selectedTag}
            on:close={() => isEditTagModalOpen = false}
            on:save={handleSaveTag}
            on:delete={handleDeleteTagFromModal}
        />
    {/if}
    {#if isEditGroupModalOpen && currentEditingGroup}
        <EditTagGroupModal
            showModal={isEditGroupModalOpen}
            group={currentEditingGroup}
            on:close={() => isEditGroupModalOpen = false}
            on:save={handleEditGroup}
            on:delete={handleDeleteGroup}
        />
    {/if}

    <!-- Right Panel: Highlight content (Now a floating panel) -->
    {#if isCommentsPanelOpen}
        <div class="fixed inset-0 bg-black bg-opacity-50 z-30" on:click={() => isCommentsPanelOpen = false}></div>
        <div
            class="fixed top-4 right-4 bottom-4 w-1/3 bg-gray-50 dark:bg-gray-700 p-4 border border-gray-200 dark:border-gray-600 overflow-y-auto shadow-lg z-40 rounded-lg"
            transition:slide={{ duration: 300, axis: 'x' }}
        >
            {#if selectedHighlight}
                <CommentsPanel
                    comments={selectedHighlight.comments || []}
                    highlightId={selectedHighlight.id}
                    on:addcomment={(e) => handleCommentAction(e)}
                    on:deletecomment={(e) => handleCommentAction(e)}
                    on:editcomment={(e) => handleCommentAction(e)}
                    on:close={() => isCommentsPanelOpen = false}
                />
            {/if}
        </div>
    {/if}
</div>

<style lang="postcss">
:global(.tabulator .tabulator-header .tabulator-col) {
    padding-left: 0px !important;
    @apply bg-gray-100 dark:bg-gray-800 text-gray-900 dark:text-gray-200 border-gray-300 dark:border-gray-700;
    color: inherit;
}

:global(.tabulator .tabulator-header) {
    @apply bg-gray-100 dark:bg-gray-800 border-b border-gray-300 dark:border-gray-700;
    color: inherit;
}

:global(.tabulator .tabulator-header .tabulator-col .tabulator-col-content .tabulator-col-title-holder .tabulator-col-title) {
    white-space: normal !important;
    @apply text-gray-900 dark:text-gray-200 font-semibold;
    color: inherit;
}

:global(.tabulator-cell) {
    overflow: hidden;
    word-break: break-word;
    border-right: 1px solid #ddd;
    min-height: 38px;
}

:global(.tabulator-cell svg) {
    display: inline-block;
    vertical-align: middle;
}

:global(html.dark .tabulator-row.highlighted-row .tabulator-cell) {
    color: #111827 !important;
}
:global(html.dark .tabulator-cell.highlighted-cell) {
    color: #111827 !important;
}

.flex-grow > div[bind\:this={tableContainer}] {
    position: absolute;
    inset: 0;
}
</style>
