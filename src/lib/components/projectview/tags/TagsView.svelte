<!-- src/lib/components/projectview/tags/TagsView.svelte -->
<script>
    import { onMount, afterUpdate } from 'svelte';
    import { slide } from 'svelte/transition';
    import { invoke } from '@tauri-apps/api/core';
    import { confirm } from '@tauri-apps/plugin-dialog';
    import { TabulatorFull as Tabulator } from 'tabulator-tables';
    import { project } from '$lib/stores/projectStore.js';
    import { allTags, updateTag, deleteTag } from '$lib/stores/tagStore.js';
    import { addCommentToHighlight, deleteComment, updateComment } from '$lib/stores/projectStore.js';
    import * as projectService from '$lib/services/projectService.js';
    import SimpleTopBar from '../shared/SimpleTopBar.svelte';
    import CommentsPanel from './CommentsPanel.svelte';

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

    let selectedTag = null;
    let tagInfo = null;
    let isLoading = false;
    let description = '';
    let tableContainer;
    let paginationContainer;
    let tabulatorInstance = null;
    let isEditing = false;
    let searchQuery = '';
    let tagNameInput = '';
    let tableReady = false;

    let selectedHighlight = null;
    let isCommentsPanelOpen = false;

    $: {
        if (typeof document !== 'undefined') {
            if (isCommentsPanelOpen) {
                document.body.style.overflow = 'hidden';
            } else {
                document.body.style.overflow = '';
            }
        }
    }

    let processedHighlights = [];
    $: {
        if (tagInfo && tagInfo.highlights) {
            processedHighlights = tagInfo.highlights.map(item => {
                // The backend sometimes returns the highlight nested, sometimes not.
                // This normalizes it.
                const highlight = item.highlight || item;
                return {
                    ...highlight,
                    // ensure other_tags is always an array
                    other_tags: highlight.other_tags || [],
                    // ensure comments is always an array
                    comments: highlight.comments || [],
                };
            });
        } else {
            processedHighlights = [];
        }
    }


    function showComments(highlightInfo) {
        selectedHighlight = highlightInfo;
        isCommentsPanelOpen = true;
    }

    afterUpdate(() => {
        if (tagInfo && tableContainer && !tabulatorInstance) {
            initializeTable(processedHighlights);
        } else if (tabulatorInstance && tableReady) {
            // This ensures we only update data if the table exists and is ready
            tabulatorInstance.setData(processedHighlights);
        }

        if (!tagInfo && tabulatorInstance) {
            tabulatorInstance.destroy();
            tabulatorInstance = null;
            tableReady = false;
        }
    });

    function handleSearch() {
        if (!searchQuery && searchQuery.trim() === '') {
            if (tabulatorInstance) {
                tabulatorInstance.clearFilter();
            }
            return;
        }
        if (tabulatorInstance) {
            const term = searchQuery.trim();
            if (term) {
                tabulatorInstance.setFilter("text", "like", term);
            } else {
                tabulatorInstance.clearFilter();
            }
        }
    }

    function initializeTable(data) {
        if (tabulatorInstance) {
            tabulatorInstance.destroy();
        }
        tableReady = false;
        tabulatorInstance = new Tabulator(tableContainer, {
            data: data,
            layout: "fitData",
            pagination: "local",
            paginationSize: 10,
            paginationAddRow: "table",
            initialFilter: [
                {field:"text", type:"like", value:searchQuery}
            ],
            columns: [
                { title: "File", field: "source.file_path", width: "20%", formatter: (cell) => {
                    const highlight = cell.getRow().getData();
                    const filePath = highlight.source.file_path;
                    const fileName = filePath.split(/[\\/]/).pop();
                    const icon = getIconForFileType(highlight.source.file_type);
                    // TODO: Make this a clickable link to open the file
                    return `<div class=\"flex items-center space-x-2\" title=\"${filePath}\">
                                <div class=\"w-8 h-8 rounded-full flex items-center justify-center p-1\" style=\"background-color: ${highlight.color};\">
                                    <span>${icon}</span>
                                </div>
                                <span>${fileName}</span>
                            </div>`;
                }},
                { title: "Content", field: "text", width: "50%", formatter: (cell) => {
                    const text = cell.getValue();
                    return `<div class=\"whitespace-normal word-break-break-word\">${text}</div>`;
                }},
                { title: "Other Tags", field: "other_tags", width: "20%", formatter: (cell) => {
                    const tags = cell.getValue() || [];
                    if (tags.length === 0) return '';

                    const tagElements = tags.map(tag =>
                        `<span class=\"inline-block bg-gray-200 dark:bg-gray-600 rounded-full px-2 py-1 text-xs font-semibold text-gray-700 dark:text-gray-200 mr-2 mb-1 border border-gray-300 dark:border-gray-500\">${tag}</span>`
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
                                <button title=\"Inspect\" class=\"mr-4\"><svg xmlns=\"http://www.w3.org/2000/svg\" width=\"16\" height=\"16\" fill=\"currentColor\" class=\"bi bi-eye\" viewBox=\"0 0 16 16\">
                                    <path d=\"M16 8s-3-5.5-8-5.5S0 8 0 8s3 5.5 8 5.5S16 8 16 8M1.173 8a13 13 0 0 1 1.66-2.043C4.12 4.668 5.88 3.5 8 3.5s3.879 1.168 5.168 2.457A13 13 0 0 1 14.828 8q-.086.13-.195.288c-.335.48-.83 1.12-1.465 1.755C11.879 11.332 10.119 12.5 8 12.5s-3.879-1.168-5.168-2.457A13 13 0 0 1 1.172 8z"/>
                                    <path d=\"M8 5.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5M4.5 8a3.5 3.5 0 1 1 7 0 3.5 3.5 0 0 1-7 0"/>
                                  </svg></button>
                                <button title=\"Comments\" class=\"relative p-1 rounded-full hover:bg-gray-200 dark:hover:bg-gray-600 mr-4\">
                                    <svg xmlns=\"http://www.w3.org/2000/svg\" width=\"16\" height=\"16\" fill=\"currentColor\" class=\"bi bi-chat\" viewBox=\"0 0 16 16\">
                                        <path d=\"M2.678 11.894a1 1 0 0 1 .287.801 11 11 0 0 1-.398 2c1.395-.323 2.247-.697 2.634-.893a1 1 0 0 1 .71-.074A8 8 0 0 0 8 14c3.996 0 7-2.807 7-6s-3.004-6-7-6-7 2.808-7 6c0 1.468.617 2.83 1.678 3.894m-.493 3.905a22 22 0 0 1-.713.129c-.2.032-.352-.176-.273-.362a10 10 0 0 0 .244-.637l.003-.01c.248-.72.45-1.548.524-2.319C.743 11.37 0 9.76 0 8c0-3.866 3.582-7 8-7s8 3.134 8 7-3.582 7-8 7a9 9 0 0 1-2.347-.306c-.52.263-1.639.742-3.468 1.105"/>
                                    </svg>
                                    ${commentPill}
                                </button>
                                <button title=\"Untag\">
                                    <svg xmlns=\"http://www.w3.org/2000/svg\" fill=\"none\" stroke=\"red\" stroke-width=\"1.2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"bi bi-tag-slash w-4 h-4\" viewBox=\"0 0 16 16">
                                      <path d=\"M2 1a1 1 0 0 0-1 1v4.586a1 1 0 0 0 .293.707l7 7a1 1 0 0 0 1.414 0l4.586-4.586a1 1 0 0 0 0-1.414l-7-7A1 1 0 0 0 6.586 1zm4 3.5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0"></path>
                                      <path d=\"M15.5 0.5 L 0.5 15.5"></path>
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
                        }
                    }
                }
            ],
            height: "100%",
            placeholder: "No highlights for this tag.",
        });
        tabulatorInstance.on("tableBuilt", () => {
            tableReady = true;
        });
    }

    async function handleSelectTag(tag) {
        isEditing = false;
        selectedTag = tag;
        tagInfo = null;
        description = '';
        try {
            isLoading = true;
            tagInfo = await invoke('get_tag_info', {
                projectId: $project.id,
                tagId: tag.id,
                tagName: tag.name,
            });
            console.log("tagInfo:", tagInfo);
            if (tagInfo) {
                description = tagInfo.description;
                tagNameInput = tagInfo.name;
            }
        } catch (error) {
            console.error(`Failed to load tag info for ${tag.name}:`, error);
        } finally {
            isLoading = false;
        }
    }

    async function handleSaveChanges() {
        if (!selectedTag) return;
        try {
            await updateTag(selectedTag.id, tagNameInput, null); // Color is not editable here yet
            isEditing = false;
            const newSelectedTag = $allTags.find(t => t.name === tagNameInput);
            if (newSelectedTag) {
                handleSelectTag(newSelectedTag);
            } else {
                selectedTag = null;
                tagInfo = null;
            }
            // Optionally, show a success notification
        } catch (error) {
            console.error(`Failed to save changes for ${selectedTag.name}:`, error);
            // Optionally, show an error notification
        }
    }

    async function handleDeleteTag() {
        if (!selectedTag) return;

        const confirmed = await confirm(`Are you sure you want to delete the tag \"${selectedTag.name}\"? This will remove the tag from all associated highlights and cannot be undone.`, {
            title: 'Confirm Deletion',
            type: 'warning',
        });

        if (confirmed) {
            try {
                await deleteTag(selectedTag.id);
                selectedTag = null;
                tagInfo = null;
                description = '';
                isEditing = false;
            } catch (error) {
                console.error(`Failed to delete tag ${selectedTag.name}:`, error);
                // Optionally, show an error notification
            }
        }
    }

    async function handleCommentAction(event) {
        const { type, detail } = event;
        const { highlightId, commentId, newText, comment } = detail;

        const highlightToUpdate = processedHighlights.find(h => h.id === highlightId);
        if (!highlightToUpdate) return;

        let docType = highlightToUpdate.source.file_type;
        if (docType === 'document' && highlightToUpdate.source.file_path.toLowerCase().endsWith('.pdf')) {
            docType = 'pdf';
        }

        // Manually update the local state for immediate UI feedback
        let newComments;
        if (type === 'addcomment') {
            newComments = [...(highlightToUpdate.comments || []), comment];
        } else if (type === 'deletecomment') {
            newComments = (highlightToUpdate.comments || []).filter(c => c.id !== commentId && c.parentId !== commentId);
        } else if (type === 'editcomment') {
            newComments = (highlightToUpdate.comments || []).map(c => {
                if (c.id === commentId) {
                    return { ...c, text: newText, updatedAt: new Date().toISOString() };
                }
                return c;
            });
        }

        const updatedHighlightWithNewComments = { ...highlightToUpdate, comments: newComments };

        // This triggers the UI to update
        selectedHighlight = updatedHighlightWithNewComments;

        // Update the item in the main list
        const anIndex = processedHighlights.findIndex(h => h.id === highlightId);
        if (anIndex !== -1) {
            processedHighlights[anIndex] = updatedHighlightWithNewComments;
            processedHighlights = [...processedHighlights];
        }

        // Call the original store function to update the store which in turn will save the data
        if (type === 'addcomment') {
            addCommentToHighlight(highlightId, comment, docType);
        } else if (type === 'deletecomment') {
            deleteComment(highlightId, commentId, docType);
        } else if (type === 'editcomment') {
            updateComment(highlightId, commentId, newText, docType);
        }

        // Persist the changes
        try {
            await projectService.saveHighlightChanges(updatedHighlightWithNewComments);
        } catch (error) {
            console.error("Failed to save highlight changes from TagsView:", error);
            // Optionally, show a notification to the user
        }
    }

    async function handleUntag(item) {
        if (!selectedTag) return;

        const confirmed = await confirm(`Are you sure you want to remove the tag \"${selectedTag.name}\" from this highlight?`);
        if (!confirmed) return;

        try {
            const highlight = getHighlight(item);
            await invoke('remove_tag_from_highlight', {
                projectId: $project.id,
                highlightId: highlight.id,
                tagToRemove: selectedTag.name,
                filePath: highlight.source.file_path,
                docType: highlight.source.file_type,
            });
            // Refresh the view
            await handleSelectTag(selectedTag);
        } catch (error) {
            console.error("Failed to remove tag from highlight:", error);
        }
    }
</script>

<div class="flex flex-col h-full w-full bg-gray-100 dark:bg-app-bg-dark overflow-hidden">
    <SimpleTopBar />
    <div class="flex h-full w-full p-1 gap-1">
        <!-- Left Panel: List of all tags -->
        <div class="w-1/4 h-full bg-gray-50 dark:bg-gray-700 p-4 border-r border-gray-200 dark:border-gray-600">
        <h2 class="text-lg font-semibold mb-4">All Tags</h2>
        {#if $allTags.length > 0}
            <ul>
                {#each $allTags as tag (tag.id)}
                    <li
                        class="p-2 rounded-md cursor-pointer hover:bg-gray-200 dark:hover:bg-gray-600"
                        class:bg-blue-200={selectedTag?.id === tag.id}
                        class:dark:bg-blue-800={selectedTag?.id === tag.id}
                        on:click={() => handleSelectTag(tag)}
                    >
                        {tag.name}
                    </li>
                {/each}
            </ul>
        {:else}
            <p>No tags found in this project.</p>
        {/if}
    </div>

    <!-- Middle Panel: Tag details and highlights -->
    <div class="w-3/4 h-full flex flex-col p-4 gap-4">
        {#if selectedTag}
            {#if isLoading}
                <p>Loading tag information...</p>
            {:else if tagInfo}
                <div class="h-[20%] flex flex-col">
                    <div>
                        <div>
                            {#if isEditing}
                                <input type="text" bind:value={tagNameInput} class="text-xl font-bold mb-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-md p-1" />
                            {:else}
                                <h2 class="text-xl font-bold mb-2">{tagInfo.name}</h2>
                            {/if}
                            <div class="mb-4">
                                <label for="tag-description" class="block text-sm font-medium text-gray-700 dark:text-gray-300">Description</label>
                                <textarea id="tag-description" rows="2" class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm dark:bg-gray-800 dark:border-gray-600" bind:value={description} readonly={!isEditing}></textarea>
                            </div>
                        </div>
                    </div>
                    <div class="flex space-x-2 mt-2">
                        {#if isEditing}
                            <button class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700" on:click={handleSaveChanges}>Save</button>
                            <button class="px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700" on:click={handleDeleteTag}>Delete</button>
                            <button class="px-4 py-2 bg-gray-200 text-black rounded-md hover:bg-gray-300" on:click={() => {isEditing = false; tagNameInput = tagInfo.name; description = tagInfo.description;}}>Cancel</button>
                        {:else}
                            <button class="px-4 py-2 bg-gray-200 text-black rounded-md hover:bg-gray-300" on:click={() => isEditing = true}>Edit</button>
                        {/if}
                    </div>
                </div>

                <div class="h-[75%] flex flex-col">
                    <div class="flex justify-between items-center mb-2 flex-shrink-0">
                        <h3 class="text-lg font-semibold">Highlights ({tagInfo.highlight_count})</h3>
                        <input type="text" placeholder="Search content..." bind:value={searchQuery} on:input={handleSearch} on:keydown={e => { if (e.key === 'Enter') { e.preventDefault(); e.stopPropagation(); } }} class="border rounded px-2 py-1 text-sm dark:bg-gray-800 dark:border-gray-600">
                    </div>
                    <div class="flex-grow overflow-auto" bind:this={tableContainer}>
                    </div>
                </div>
            {:else}
                <div class="flex items-center justify-center h-full">
                    <p class="text-gray-500">Select a tag to view its details.</p>
                </div>
            {/if}
        {:else}
            <div class="flex items-center justify-center h-full">
                <p class="text-gray-500">Select a tag to view its details.</p>
            </div>
        {/if}
    </div>

    </div>

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