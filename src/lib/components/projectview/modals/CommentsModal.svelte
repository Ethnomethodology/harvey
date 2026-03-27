<script lang="ts">
    import { createEventDispatcher } from 'svelte';
    import { v4 as uuidv4 } from 'uuid';
    import { X, MoreVertical, MessageCircle, Reply, Pencil, Trash2, Clock } from '@lucide/svelte';
    import { 
        Modal, 
        Button, 
        Textarea, 
        Dropdown, 
        DropdownItem,
        Avatar
    } from 'flowbite-svelte';

    export let showModal = false;
    export let comments = [];
    export let highlightId = null;

    const dispatch = createEventDispatcher();

    let newCommentText = '';
    let editingCommentId = null;
    let editingText = '';
    let replyingToCommentId = null;
    let replyingToCommentText = '';
    let textareaWrapper: HTMLDivElement;

    function handleAction(action, comment) {
        if (action === 'delete') {
            dispatch('deletecomment', { highlightId, commentId: comment.id });
        } else if (action === 'edit') {
            editingCommentId = comment.id;
            editingText = comment.text;
        } else if (action === 'reply') {
            replyingToCommentId = comment.id;
            replyingToCommentText = comment.text;
            setTimeout(() => textareaWrapper?.querySelector('textarea')?.focus(), 0);
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

    function closeModal() {
        dispatch('close');
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
            handleAddComment();
        }
    }
</script>

<Modal
    bind:open={showModal}
    size="md"
    autoclose={false}
    outsideclose={true}
    class="w-full"
    backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
    dialogClass="fixed top-0 start-0 end-0 h-modal md:inset-0 md:h-full z-[10001] flex"
    bodyClass="p-6 space-y-4 bg-white dark:bg-gray-900"
    headerClass="px-6 py-4 flex items-center justify-between border-b dark:border-gray-700 bg-gray-50/50"
    footerClass="px-6 py-4 flex items-center justify-end space-x-3 rtl:space-x-reverse border-t dark:border-gray-700 bg-gray-50/80 backdrop-blur"
    on:close={closeModal}
>
    <div slot="header" class="flex items-center gap-2">
        <MessageCircle class="w-5 h-5 text-gray-500" />
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
            Comments
        </h3>
    </div>

    <div class="space-y-4 max-h-[50vh] overflow-y-auto pr-2 custom-scrollbar">
        {#each comments.filter(c => !c.parentId) as comment}
            <div class="space-y-3">
                <div class="p-4 rounded-xl bg-gray-50 dark:bg-gray-800/50 border border-gray-100 dark:border-gray-700 relative group">
                    {#if editingCommentId === comment.id}
                        <Textarea bind:value={editingText} rows="3" class="mb-2" placeholder="Edit comment..." autocomplete="off" autocorrect="off" />
                        <div class="flex justify-end gap-2">
                            <Button size="xs" color="alternative" on:click={handleCancelEdit}>Cancel</Button>
                            <Button size="xs" color="blue" on:click={handleSaveEdit}>Save</Button>
                        </div>
                    {:else}
                        <div class="flex justify-between items-start mb-2">
                            <div class="flex items-center gap-2">
                                <Avatar size="xs" border />
                                <div class="flex flex-col">
                                    <span class="text-xs font-semibold text-gray-900 dark:text-white">User</span>
                                    <span class="text-[10px] text-gray-500 flex items-center gap-1">
                                        <Clock class="w-3 h-3" />
                                        {new Date(comment.updatedAt).toLocaleString()}
                                    </span>
                                </div>
                            </div>
                            <button class="p-1 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors">
                                <MoreVertical class="w-4 h-4 text-gray-500" />
                            </button>
                            <Dropdown placement="left-start" class="w-32 z-[10002]" strategy="fixed">
                                {#if !comment.parentId}
                                    <DropdownItem class="flex items-center gap-2" on:click={() => handleAction('reply', comment)}>
                                        <Reply class="w-3.5 h-3.5" /> Reply
                                    </DropdownItem>
                                {/if}
                                <DropdownItem class="flex items-center gap-2" on:click={() => handleAction('edit', comment)}>
                                    <Pencil class="w-3.5 h-3.5" /> Edit
                                </DropdownItem>
                                <DropdownItem class="flex items-center gap-2 text-red-600 dark:text-red-400" on:click={() => handleAction('delete', comment)}>
                                    <Trash2 class="w-3.5 h-3.5" /> Delete
                                </DropdownItem>
                            </Dropdown>
                        </div>
                        <p class="text-sm text-gray-700 dark:text-gray-300 leading-relaxed">
                            {comment.text}
                        </p>
                    {/if}
                </div>

                <!-- Replies -->
                {#each comments.filter(r => r.parentId === comment.id) as reply}
                    <div class="ml-8 p-4 rounded-xl bg-gray-100/50 dark:bg-gray-800/30 border border-gray-200/50 dark:border-gray-700/50 relative group">
                        {#if editingCommentId === reply.id}
                            <Textarea bind:value={editingText} rows="2" class="mb-2" placeholder="Edit reply..." autocomplete="off" autocorrect="off" />
                            <div class="flex justify-end gap-2">
                                <Button size="xs" color="alternative" on:click={handleCancelEdit}>Cancel</Button>
                                <Button size="xs" color="blue" on:click={handleSaveEdit}>Save</Button>
                            </div>
                        {:else}
                            <div class="flex justify-between items-start mb-2">
                                <div class="flex items-center gap-2">
                                    <Avatar size="xs" border />
                                    <div class="flex flex-col">
                                        <span class="text-xs font-semibold text-gray-900 dark:text-white">User</span>
                                        <span class="text-[10px] text-gray-500 flex items-center gap-1">
                                            <Clock class="w-3 h-3" />
                                            {new Date(reply.updatedAt).toLocaleString()}
                                        </span>
                                    </div>
                                </div>
                                <button class="p-1 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors">
                                    <MoreVertical class="w-4 h-4 text-gray-500" />
                                </button>
                                <Dropdown placement="left-start" class="w-32 z-[10002]" strategy="fixed">
                                    <DropdownItem class="flex items-center gap-2" on:click={() => handleAction('edit', reply)}>
                                        <Pencil class="w-3.5 h-3.5" /> Edit
                                    </DropdownItem>
                                    <DropdownItem class="flex items-center gap-2 text-red-600 dark:text-red-400" on:click={() => handleAction('delete', reply)}>
                                        <Trash2 class="w-3.5 h-3.5" /> Delete
                                    </DropdownItem>
                                </Dropdown>
                            </div>
                            <p class="text-sm text-gray-700 dark:text-gray-300 leading-relaxed">
                                {reply.text}
                            </p>
                        {/if}
                    </div>
                {/each}
            </div>
        {/each}

        {#if comments.length === 0}
            <div class="flex flex-col items-center justify-center py-12 text-gray-400 dark:text-gray-600">
                <MessageCircle class="w-12 h-12 mb-2 opacity-20" />
                <p class="text-sm italic">No comments yet.</p>
            </div>
        {/if}
    </div>

    <div class="mt-4 pt-4 border-t dark:border-gray-700">
        {#if replyingToCommentId}
            <div class="flex items-center justify-between mb-2 px-2 py-1 bg-blue-50 dark:bg-blue-900/20 rounded-md">
                <div class="flex items-center gap-2 overflow-hidden">
                    <Reply class="w-3 h-3 text-blue-600 dark:text-blue-400 shrink-0" />
                    <span class="text-[11px] text-blue-800 dark:text-blue-300 truncate italic">
                        Replying to: "{replyingToCommentText}"
                    </span>
                </div>
                <button on:click={cancelReply} class="p-1 hover:bg-blue-100 dark:hover:bg-blue-800 rounded transition-colors">
                    <X class="w-3 h-3 text-blue-600 dark:text-blue-400" />
                </button>
            </div>
        {/if}
        <div bind:this={textareaWrapper} on:keydown={handleKeydown}>
        <Textarea
            bind:value={newCommentText}
            placeholder={replyingToCommentId ? 'Write a reply...' : 'Add a comment...'}
            rows="3"
            class="bg-white dark:bg-gray-800"
            autocomplete="off"
            autocorrect="off"
            spellcheck="false"
        />
        </div>
    </div>

    <svelte:fragment slot="footer">
        <span class="text-[10px] text-gray-400 dark:text-gray-500 mr-auto">⌘↵ to submit</span>
        <Button color="alternative" on:click={closeModal}>Close</Button>
        <Button 
            color="blue" 
            on:click={handleAddComment} 
            disabled={!newCommentText.trim()}
        >
            {replyingToCommentId ? 'Post Reply' : 'Post Comment'}
        </Button>
    </svelte:fragment>
</Modal>

<style lang="postcss">
    .custom-scrollbar::-webkit-scrollbar {
        width: 6px;
    }
    .custom-scrollbar::-webkit-scrollbar-track {
        @apply bg-transparent;
    }
    .custom-scrollbar::-webkit-scrollbar-thumb {
        @apply bg-gray-200 dark:bg-gray-700 rounded-full;
    }
    .custom-scrollbar::-webkit-scrollbar-thumb:hover {
        @apply bg-gray-300 dark:bg-gray-600;
    }
</style>