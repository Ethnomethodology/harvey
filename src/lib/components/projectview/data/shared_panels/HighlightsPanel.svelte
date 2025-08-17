<!-- src/lib/components/projectview/data/shared_panels/HighlightsPanel.svelte -->
<script>
import { project, setDocumentHighlights, addCommentToHighlight } from '$lib/stores/projectStore.js';
    import TagMultiSelect from '$lib/components/projectview/shared/TagMultiSelect.svelte';
    import CommentsModal from '$lib/components/projectview/modals/CommentsModal.svelte';

    export let itemPath = null;
    export let itemType = null;

    let allTags = [];

    let showCommentsModal = false;
    let selectedHighlightForComments = null;

    function openCommentsModal(highlight) {
        selectedHighlightForComments = highlight;
        showCommentsModal = true;
    }

    function handleAddComment(event) {
        const { highlightId, comment } = event.detail;
        addCommentToHighlight(highlightId, comment);

        // Immediately update local state for modal reactivity
        const updatedHighlight = get(project).currentDocumentHighlights.find(h => h.id === highlightId);
        if (updatedHighlight) {
            selectedHighlightForComments = {
                ...selectedHighlightForComments,
                comments: updatedHighlight.comments
            };
        }
    }

    function groupHighlights(highlights) {
        if (!highlights || highlights.length === 0) {
            return [];
        }

        const map = new Map();
        for (const highlight of highlights) {
            if (!map.has(highlight.id)) {
                map.set(highlight.id, {
                    id: highlight.id,
                    color: highlight.color,
                    textParts: [],
                    tags: highlight.tags || [],
                    comments: highlight.comments || []
                });
            }
            map.get(highlight.id).textParts.push(highlight.text);
        }

        return Array.from(map.values()).map(group => ({
            ...group,
            text: group.textParts.join(' ')
        }));
    }

    $: groupedHighlights = groupHighlights($project.currentDocumentHighlights);

    $: {
        console.log('[Jules DEBUG] Highlights from store in panel:', $project.currentDocumentHighlights);
    }

    function handleTagsUpdate(highlightId, newTags) {
        const newHighlights = $project.currentDocumentHighlights.map(h => {
            if (h.id === highlightId) {
                return { ...h, tags: newTags };
            }
            return h;
        });
        setDocumentHighlights(newHighlights);
    }

    function handleCreateTag(newTag, highlightId) {
        if (!allTags.includes(newTag)) {
            allTags = [...allTags, newTag];
        }
        const highlight = $project.currentDocumentHighlights.find(h => h.id === highlightId);
        const newTags = [...(highlight?.tags || []), newTag];
        handleTagsUpdate(highlightId, newTags);
    }
</script>

<div class="h-full bg-white dark:bg-gray-800 rounded-md shadow flex flex-col overflow-hidden py-2">
    <div class="text-sm font-semibold border-b pb-1 px-1 border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 flex-shrink-0 flex items-center justify-between h-8 mb-2">
        <span class="ml-1">Highlights</span>
    </div>

    <div class="flex-grow overflow-y-auto overflow-x-hidden min-h-0 text-xs relative px-2">
        {#if groupedHighlights.length > 0}
            <ul class="space-y-2">
                {#each groupedHighlights as highlight (highlight.id)}
                    <li class="border rounded-md bg-white dark:bg-gray-700" style="border-left-color: {highlight.color}; border-left-width: 4px;">
                        <div class="p-2">
                            <p class="font-semibold text-black dark:text-white">{highlight.text}</p>
                        </div>
                        <div class="border-t border-gray-200 dark:border-gray-600 px-2 py-1 flex flex-col">
                            <div class="flex items-center w-full">
                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-tags-fill mr-2 flex-shrink-0" viewBox="0 0 16 16">
                                    <path d="M2 2a1 1 0 0 1 1-1h4.586a1 1 0 0 1 .707.293l7 7a1 1 0 0 1 0 1.414l-4.586 4.586a1 1 0 0 1-1.414 0l-7-7A1 1 0 0 1 2 6.586zm3.5 4a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3"/>
                                    <path d="M1.293 7.793A1 1 0 0 1 1 7.086V2a1 1 0 0 0-1 1v4.586a1 1 0 0 0 .293.707l7 7a1 1 0 0 0 1.414 0l.043-.043z"/>
                                </svg>
                                <div class="w-full relative">
                                    <TagMultiSelect
                                        allTags={allTags}
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
        {:else if itemType === 'doc'}
            <p class="text-gray-500 dark:text-gray-400 italic px-1 py-2">
                No highlights present for this document.
            </p>
        {:else}
            <p class="text-gray-500 dark:text-gray-400 italic px-1 py-2">
                Highlights are only available for document files.
            </p>
        {/if}
    </div>
</div>

<CommentsModal
    bind:showModal={showCommentsModal}
    comments={selectedHighlightForComments?.comments || []}
    highlightId={selectedHighlightForComments?.id}
    on:close={() => showCommentsModal = false}
    on:addcomment={handleAddComment}
/>
