<script lang="ts">
    import { createEventDispatcher, onMount } from 'svelte';
    import { v4 as uuidv4 } from 'uuid';

    export let showModal = false;
    export let comments = [];
    export let highlightId = null;

    const dispatch = createEventDispatcher();

    let newCommentText = '';

    function handleAddComment() {
        if (!newCommentText.trim()) return;
        const newComment = {
            id: uuidv4(),
            text: newCommentText.trim(),
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            parentId: null,
        };
        dispatch('addcomment', { highlightId, comment: newComment });
        newCommentText = '';
    }

    function closeModal() {
        dispatch('close');
    }
</script>

{#if showModal}
<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50" on:click={closeModal}>
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl p-6 w-full max-w-lg" on:click|stopPropagation>
        <h2 class="text-lg font-semibold mb-4 text-gray-900 dark:text-white">Comments</h2>

        <div class="space-y-4 max-h-96 overflow-y-auto">
            {#each comments.filter(c => !c.parentId) as comment}
                <div class="p-3 rounded-lg bg-gray-100 dark:bg-gray-700">
                    <p class="text-sm text-gray-800 dark:text-gray-200">{comment.text}</p>
                    <p class="text-xs text-gray-500 dark:text-gray-400 mt-2 text-right">{new Date(comment.updatedAt).toLocaleString()}</p>
                </div>
                <!-- Replies would go here -->
                {#each comments.filter(r => r.parentId === comment.id) as reply}
                    <div class="ml-8 p-3 rounded-lg bg-gray-200 dark:bg-gray-600">
                        <p class="text-sm text-gray-800 dark:text-gray-200">{reply.text}</p>
                        <p class="text-xs text-gray-500 dark:text-gray-400 mt-2 text-right">{new Date(reply.updatedAt).toLocaleString()}</p>
                    </div>
                {/each}
            {/each}

            {#if comments.length === 0}
                <p class="text-sm text-gray-500 dark:text-gray-400">No comments yet.</p>
            {/if}
        </div>

        <div class="mt-6">
            <textarea
                bind:value={newCommentText}
                class="w-full p-2 border rounded-md bg-white dark:bg-gray-700 text-black dark:text-white border-gray-300 dark:border-gray-600"
                placeholder="Add a comment..."
                rows="3"
            ></textarea>
            <div class="mt-2 flex justify-end gap-2">
                <button on:click={closeModal} class="px-4 py-2 rounded-md text-sm font-medium bg-gray-200 dark:bg-gray-600 text-gray-800 dark:text-gray-200 hover:bg-gray-300 dark:hover:bg-gray-500">Cancel</button>
                <button on:click={handleAddComment} class="px-4 py-2 rounded-md text-sm font-medium bg-blue-600 text-white hover:bg-blue-700">Add Comment</button>
            </div>
        </div>
    </div>
</div>
{/if}
