<!-- src/lib/components/projectview/data/shared_panels/HighlightsPanel.svelte -->
<script>
    import { get } from 'svelte/store';
    import { onMount, onDestroy } from 'svelte';
    import { refresher } from '$lib/stores/refresherStore.js';

    import { project, setDocumentHighlights, addCommentToHighlight, deleteComment, updateComment, setImportedTranscriptHighlights, updatePdfAnnotations, updateImageAnnotations, setTableHighlights } from '$lib/stores/projectStore.js';
    import { saveImageAnnotations, saveTableHighlights, loadHighlightsForFile } from '$lib/services/projectService.js';
    import { allTags as allTagsStore, addTag, fetchAllTags } from '$lib/stores/tagStore.js';
    import TagMultiSelect from '$lib/components/projectview/shared/TagMultiSelect.svelte';
    import CommentsModal from '$lib/components/projectview/modals/CommentsModal.svelte';

    export let itemPath = null;
    export let itemType = null;
	export let refreshKey = null;

    let unsubscribeRefresher;

    onMount(() => {
        let isFirstRun = true;
        unsubscribeRefresher = refresher.subscribe(async () => {
            if (isFirstRun) {
                isFirstRun = false;
                return;
            }
            if (itemPath) {
                console.log('[HighlightsPanel] Refresher triggered, re-loading highlights for', itemPath);
                await loadHighlightsForFile(itemPath, itemType);
                await fetchAllTags(); // Also refresh the list of all available tags
            }
        });
    });

	$: if (refreshKey) {
		if (itemPath) {
			loadHighlightsForFile(itemPath, itemType);
			fetchAllTags();
		}
	}

    onDestroy(() => {
        if (unsubscribeRefresher) {
            unsubscribeRefresher();
        }
    });

    let showCommentsModal = false;
    let selectedHighlightId = null;

    function openCommentsModal(highlight) {
        selectedHighlightId = highlight.id;
        showCommentsModal = true;
    }

    function closeModal() {
        showCommentsModal = false;
        selectedHighlightId = null;
    }

    // --- Reactive State based on Store and Props ---
    let activeHighlights = [];
    let effectiveType = null;

    $: {
        const p = $project;
        const currentPath = p.selectedDocumentPath;
        const selectedType = p.selectedDocumentType;
        
        // Prioritize store state for determining the type of the active document
        if (p.selectedMediaNotePath) {
            effectiveType = 'doc'; // Media transcripts are handled like docs
            activeHighlights = p.currentDocumentHighlights || [];
        } else if (p.currentImportedTranscriptPath) {
            effectiveType = 'imported_transcript';
            activeHighlights = p.currentImportedTranscriptHighlights || [];
        } else if (selectedType === 'tables') {
            effectiveType = 'table';
            activeHighlights = p.currentTableHighlights || [];
        } else if (selectedType === 'images') {
            effectiveType = 'image';
            activeHighlights = p.currentImageAnnotations || [];
        } else if (currentPath?.toLowerCase().endsWith('.pdf')) {
            effectiveType = 'pdf';
            activeHighlights = p.currentPdfAnnotations || [];
        } else {
            effectiveType = 'doc';
            activeHighlights = p.currentDocumentHighlights || [];
        }
    }

    $: selectedHighlightForComments = activeHighlights.find(h => h.id === selectedHighlightId) || null;

    // This function handles the structure conversion for different annotation types
    function processHighlights(highlights, type) {
        if (!highlights || highlights.length === 0) return [];

        if (type === 'image') {
            return highlights.map(annotation => {
                const titleBody = annotation.body.find(b => b.purpose === 'commenting' && b.type === 'Title');
                const descriptionBody = annotation.body.find(b => b.purpose === 'commenting' && b.type === 'Description');
                const colorBody = annotation.body.find(b => b.purpose === 'highlighting');

                return {
                    id: annotation.id,
                    color: colorBody ? colorBody.value : 'rgba(255, 242, 117, 0.5)',
                    title: titleBody ? titleBody.value : 'No title',
                    description: descriptionBody ? descriptionBody.value : 'No description',
                    tags: annotation.tags || [],
                    comments: annotation.comments || []
                };
            });
        } else if (type === 'table') {
            return highlights.map(h => ({
                id: h.id,
                color: h.color,
                text: h.text,
                tags: h.tags || [],
                comments: h.comments || []
            }));
        } else { // Handles 'doc', 'pdf', 'imported_transcript'
            const isPdf = type === 'pdf';
            const map = new Map();
            for (const highlight of highlights) {
                if (!map.has(highlight.id)) {
                    map.set(highlight.id, {
                        id: highlight.id,
                        color: highlight.color,
                        textParts: [],
                        tags: highlight.tags || [],
                        comments: highlight.comments || [],
                        pageIndex: highlight.pageIndex,
                        quadPoints: highlight.quadPoints,
                        documentOrder: highlight.documentOrder
                    });
                }
                map.get(highlight.id).textParts.push(highlight.text);
            }
            let result = Array.from(map.values()).map(group => ({
                ...group,
                text: group.textParts.join(' ')
            }));

            if (isPdf) {
                result.sort((a, b) => {
                    if (a.pageIndex !== b.pageIndex) return a.pageIndex - b.pageIndex;
                    const ay = (a.quadPoints && a.quadPoints.length > 0) ? a.quadPoints[0][1] : 0;
                    const by = (b.quadPoints && b.quadPoints.length > 0) ? b.quadPoints[0][1] : 0;
                    if (Math.abs(ay - by) > 5) return ay - by;
                    const ax = (a.quadPoints && a.quadPoints.length > 0) ? a.quadPoints[0][0] : 0;
                    const bx = (b.quadPoints && b.quadPoints.length > 0) ? b.quadPoints[0][0] : 0;
                    return ax - bx;
                });
            } else {
                result.sort((a, b) => (a.documentOrder ?? 0) - (b.documentOrder ?? 0));
            }
            return result;
        }
    }

    $: processedHighlights = processHighlights(activeHighlights, effectiveType);

    async function handleHighlightsUpdate(newHighlights) {
        if (effectiveType === 'imported_transcript') {
            setImportedTranscriptHighlights(newHighlights);
        } else if (effectiveType === 'pdf') {
            updatePdfAnnotations(newHighlights, true);
        } else if (effectiveType === 'image') {
            updateImageAnnotations(newHighlights);
            await saveImageAnnotations();
        } else if (effectiveType === 'table') {
            setTableHighlights(newHighlights);
            await saveTableHighlights();
        } else {
            setDocumentHighlights(newHighlights);
        }
    }

    function handleTagsUpdate(highlightId, newTags) {
        const newHighlights = activeHighlights.map(h => {
            if (h.id === highlightId) {
                return { ...h, tags: newTags };
            }
            return h;
        });
        handleHighlightsUpdate(newHighlights);
    }

    async function handleCreateTag(newTag, highlightId) {
        try {
            await addTag(newTag);
            const highlight = activeHighlights.find(h => h.id === highlightId);
            if (highlight && !(highlight.tags || []).includes(newTag)) {
                const newTags = [...(highlight.tags || []), newTag];
                handleTagsUpdate(highlightId, newTags);
            }
        } catch (error) {
            console.error('Failed to create tag:', error);
        }
    }

    function handleCommentAction(event) {
        const { type, detail } = event;
        const { highlightId, commentId, newText, comment } = detail;

        if (type === 'addcomment') {
            addCommentToHighlight(highlightId, comment, effectiveType);
        } else if (type === 'deletecomment') {
            deleteComment(highlightId, commentId, effectiveType);
        } else if (type === 'editcomment') {
            updateComment(highlightId, commentId, newText, effectiveType);
        }

        if (effectiveType === 'image') {
            saveImageAnnotations();
        } else if (effectiveType === 'table') {
            saveTableHighlights();
        }
    }
</script>

<div class="h-full bg-white dark:bg-dark-bg-secondary flex flex-col overflow-hidden">
    <div class="text-sm font-semibold border-b px-1 h-9 border-gray-300 dark:border-dark-bg-tertiary text-gray-700 dark:text-gray-300 flex-shrink-0 flex items-center justify-between mb-2">
        <div class="flex items-center space-x-2">
            <span class="ml-1">Highlights</span>
        </div>
    </div>

    <div class="flex-grow overflow-y-auto overflow-x-hidden min-h-0 text-xs relative px-2">
        {#if processedHighlights.length > 0}
            <ul class="space-y-2">
                {#each processedHighlights as highlight (highlight.id)}
                    <li class="border" style="border-left-color: {highlight.color}; border-left-width: 4px;">
                        <div class="p-2 bg-white dark:bg-surface-3">
                            {#if effectiveType === 'image'}
                                <p class="font-semibold text-black dark:text-white">{highlight.title || 'No Title'}</p>
                                <p class="text-gray-600 dark:text-gray-300 mt-1">{highlight.description || 'No Description'}</p>
                            {:else}
                                {#if highlight.pageIndex !== undefined && highlight.pageIndex !== null}
                                    <p class="text-xs text-gray-500 dark:text-gray-400 mb-1">Page {highlight.pageIndex + 1}</p>
                                {/if}
                                <p class="font-semibold text-black dark:text-white">{highlight.text}</p>
                            {/if}
                        </div>
                        <div class="border-t border-gray-200 dark:border-dark-bg-tertiary px-2 py-1 flex flex-col bg-white dark:bg-surface-3">
                            <div class="flex items-center w-full">
                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-tags-fill mr-2 flex-shrink-0" viewBox="0 0 16 16">
                                    <path d="M2 2a1 1 0 0 1 1-1h4.586a1 1 0 0 1 .707.293l7 7a1 1 0 0 1 0 1.414l-4.586 4.586a1 1 0 0 1-1.414 0l-7-7A1 1 0 0 1 2 6.586zm3.5 4a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3"/>
                                    <path d="M1.293 7.793A1 1 0 0 1 1 7.086V2a1 1 0 0 0-1 1v4.586a1 1 0 0 0 .293.707l7 7a1 1 0 0 0 1.414 0l.043-.043z"/>
                                </svg>
                                <div class="w-full relative">
                                    <TagMultiSelect
                                        allTags={$allTagsStore.map(t => t.name)}
                                        assignedTags={highlight.tags}
                                        on:update={(e) => handleTagsUpdate(highlight.id, e.detail.tags)}
                                        on:createtag={(e) => handleCreateTag(e.detail.tag, highlight.id)}
                                    />
                                </div>
                            </div>
                            <div class="flex justify-end w-full mt-1">
                                <button on:click={() => openCommentsModal(highlight)} class="relative p-1 rounded-full hover:bg-gray-200 dark:hover:bg-gray-600">
                                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chat-fill" viewBox="0 0 16 16">
                                        <path d="M8 15c4.418 0 8-3.134 8-7s-3.582-7-8-7-8 3.134-8 7c0 1.76.743 3.37 1.97 4.6-.097 1.016-.417 2.13-.771 2.966-.079.186.074.394.273.362 2.256-.37 3.597-.938 4.18-1.234A9 9 0 0 0 8 15"/>
                                    </svg>
                                    {#if highlight.comments && highlight.comments.length > 0}
                                        <span class="absolute -top-1 -right-1 bg-blue-500 text-white text-xs rounded-full h-4 w-4 flex items-center justify-center">
                                            {highlight.comments.length}
                                        </span>
                                    {/if}
                                </button>
                            </div>
                        </div>
                    </li>
                {/each}
            </ul>
        {:else if effectiveType === 'doc' || effectiveType === 'media' || effectiveType === 'imported_transcript' || effectiveType === 'image' || effectiveType === 'table' || effectiveType === 'pdf'}
            <p class="text-gray-500 dark:text-gray-400 italic px-1 py-2">
                No highlights for this item.
            </p>
        {:else}
            <p class="text-gray-500 dark:text-gray-400 italic px-1 py-2">
                Highlights are not available for this item type.
            </p>
        {/if}
    </div>
</div>

<CommentsModal
    bind:showModal={showCommentsModal}
    comments={selectedHighlightForComments?.comments || []}
    highlightId={selectedHighlightForComments?.id}
    on:close={closeModal}
    on:addcomment={(e) => handleCommentAction(e)}
    on:deletecomment={(e) => handleCommentAction(e)}
    on:editcomment={(e) => handleCommentAction(e)}
/>
