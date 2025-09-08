<script lang="ts">
    import { createEventDispatcher, onMount } from 'svelte';
    import { v4 as uuidv4 } from 'uuid';

    export let comments = [];
    export let highlightId = null;

    const dispatch = createEventDispatcher();

    let newCommentText = '';
    let activeMenuId = null;
    let editingCommentId = null;
    let editingText = '';
    let replyingToCommentId = null;
    let replyingToCommentText = '';

    function toggleMenu(commentId) {
        activeMenuId = activeMenuId === commentId ? null : commentId;
    }

    function handleAction(action, comment) {
        activeMenuId = null; // Close menu regardless of action
        if (action === 'delete') {
            dispatch('deletecomment', { highlightId, commentId: comment.id });
        } else if (action === 'edit') {
            editingCommentId = comment.id;
            editingText = comment.text;
        } else if (action === 'reply') {
            replyingToCommentId = comment.id;
            replyingToCommentText = comment.text;
        } else {
            console.log(action, comment);
        }
    }

    function handleSaveEdit() {
        if (!editingText.trim() || !editingCommentId) return;
        dispatch('editcomment', {
            highlightId,
            commentId: editingCommentId,
            newText: editingText.trim(),
        });
        editingCommentId = null;
        editingText = '';
    }

    function handleCancelEdit() {
        editingCommentId = null;
        editingText = '';
    }

    function handleAddComment() {
        if (!newCommentText.trim()) return;
        const newComment = {
            id: uuidv4(),
            text: newCommentText.trim(),
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            parentId: replyingToCommentId,
        };
        dispatch('addcomment', { highlightId, comment: newComment });
        newCommentText = '';
        replyingToCommentId = null;
        replyingToCommentText = '';
    }

    function cancelReply() {
        replyingToCommentId = null;
        replyingToCommentText = '';
    }
</script>

<div class="flex flex-col h-full">
    <div class="flex justify-between items-center mb-4">
        <h2 class="text-lg font-semibold text-gray-900 dark:text-white">Comments</h2>
        <button on:click={() => dispatch('close')} class="p-1 rounded-full hover:bg-gray-200 dark:hover:bg-gray-600">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-x-lg" viewBox="0 0 16 16">
                <path d="M2.146 2.854a.5.5 0 1 1 .708-.708L8 7.293l5.146-5.147a.5.5 0 0 1 .708.708L8.707 8l5.147 5.146a.5.5 0 0 1-.708.708L8 8.707l-5.146 5.147a.5.5 0 0 1-.708-.708L7.293 8z"/>
            </svg>
        </button>
    </div>

    <div class="space-y-4 flex-grow overflow-y-auto pr-2">
        {#each comments.filter(c => !c.parentId) as comment}
            <div class="p-3 rounded-lg bg-gray-100 dark:bg-gray-700 relative group">
                {#if editingCommentId === comment.id}
                    <textarea bind:value={editingText} class="w-full p-2 border rounded-md bg-white dark:bg-gray-800 text-black dark:text-white border-gray-300 dark:border-gray-600" rows="3"></textarea>
                    <div class="mt-2 flex justify-end gap-2">
                        <button on:click={handleCancelEdit} class="px-3 py-1 rounded text-xs bg-gray-200 dark:bg-gray-600 hover:bg-gray-300 dark:hover:bg-gray-500">Cancel</button>
                        <button on:click={handleSaveEdit} class="px-3 py-1 rounded text-xs bg-blue-600 text-white hover:bg-blue-700">Save</button>
                    </div>
                {:else}
                    <div class="flex justify-between items-start">
                        <p class="text-sm text-gray-800 dark:text-gray-200 flex-grow pr-8">{comment.text}</p>
                        <div class="absolute top-1 right-1">
                            <button on:click={() => toggleMenu(comment.id)} class="p-1 rounded-full hover:bg-gray-300 dark:hover:bg-gray-600 opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity">
                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-three-dots-vertical" viewBox="0 0 16 16">
                                    <path d="M9.5 13a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0m0-5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0m0-5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0"/>
                                </svg>
                            </button>
                            {#if activeMenuId === comment.id}
                                <div class="absolute right-0 mt-2 w-32 bg-white dark:bg-gray-900 rounded-md shadow-lg z-10 border border-gray-200 dark:border-gray-700">
                                    {#if !comment.parentId}
                                    <button on:click={() => handleAction('reply', comment)} class="block w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800">Reply</button>
                                    {/if}
                                    <button on:click={() => handleAction('edit', comment)} class="block w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800">Edit</button>
                                    <button on:click={() => handleAction('delete', comment)} class="block w-full text-left px-4 py-2 text-sm text-red-600 dark:text-red-400 hover:bg-gray-100 dark:hover:bg-gray-800">Delete</button>
                                </div>
                            {/if}
                        </div>
                    </div>
                    <p class="text-xs text-gray-500 dark:text-gray-400 mt-2 text-right">{new Date(comment.updatedAt).toLocaleString()}</p>
                {/if}
            </div>
            <!-- Replies would go here -->
            {#each comments.filter(r => r.parentId === comment.id) as reply}
                <div class="ml-8 p-3 rounded-lg bg-gray-200 dark:bg-gray-600 relative group">
                    {#if editingCommentId === reply.id}
                        <textarea bind:value={editingText} class="w-full p-2 border rounded-md bg-white dark:bg-gray-700 text-black dark:text-white border-gray-300 dark:border-gray-600" rows="2"></textarea>
                        <div class="mt-2 flex justify-end gap-2">
                            <button on:click={handleCancelEdit} class="px-3 py-1 rounded text-xs bg-gray-300 dark:bg-gray-500 hover:bg-gray-400 dark:hover:bg-gray-400">Cancel</button>
                            <button on:click={handleSaveEdit} class="px-3 py-1 rounded text-xs bg-blue-600 text-white hover:bg-blue-700">Save</button>
                        </div>
                    {:else}
                        <div class="flex justify-between items-start">
                            <p class="text-sm text-gray-800 dark:text-gray-200 flex-grow pr-8">{reply.text}</p>
                            <div class="absolute top-1 right-1">
                                <button on:click={() => toggleMenu(reply.id)} class="p-1 rounded-full hover:bg-gray-300 dark:hover:bg-gray-500 opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity">
                                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-three-dots-vertical" viewBox="0 0 16 16">
                                        <path d="M9.5 13a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0m0-5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0m0-5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0"/>
                                    </svg>
                                </button>
                                {#if activeMenuId === reply.id}
                                    <div class="absolute right-0 mt-2 w-32 bg-white dark:bg-gray-900 rounded-md shadow-lg z-10 border border-gray-200 dark:border-gray-700">
                                        <button on:click={() => handleAction('edit', reply)} class="block w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800">Edit</button>
                                        <button on:click={() => handleAction('delete', reply)} class="block w-full text-left px-4 py-2 text-sm text-red-600 dark:text-red-400 hover:bg-gray-100 dark:hover:bg-gray-800">Delete</button>
                                    </div>
                                {/if}
                            </div>
                        </div>
                        <p class="text-xs text-gray-500 dark:text-gray-400 mt-2 text-right">{new Date(reply.updatedAt).toLocaleString()}</p>
                    {/if}
                </div>
            {/each}
        {/each}

        {#if comments.length === 0}
            <p class="text-sm text-gray-500 dark:text-gray-400">No comments yet.</p>
        {/if}
    </div>

    <div class="mt-6">
        {#if replyingToCommentId}
            <div class="text-sm text-gray-600 dark:text-gray-400 mb-2">
                Replying to:
                <blockquote class="border-l-4 border-gray-300 dark:border-gray-600 pl-2 my-1 text-gray-500 italic truncate">
                    {replyingToCommentText}
                </blockquote>
                <button on:click={cancelReply} class="text-blue-500 hover:underline text-xs">(Cancel Reply)</button>
            </div>
        {/if}
        <textarea
            bind:value={newCommentText}
            class="w-full p-2 border rounded-md bg-white dark:bg-gray-700 text-black dark:text-white border-gray-300 dark:border-gray-600"
            placeholder={replyingToCommentId ? 'Add a reply...' : 'Add a comment...'}
            rows="3"
        ></textarea>
        <div class="mt-2 flex justify-end gap-2">
            <button on:click={handleAddComment} class="px-4 py-2 rounded-md text-sm font-medium bg-blue-600 text-white hover:bg-blue-700">
                {replyingToCommentId ? 'Add Reply' : 'Add Comment'}
            </button>
        </div>
    </div>
</div>
