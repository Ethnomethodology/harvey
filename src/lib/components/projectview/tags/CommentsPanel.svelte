<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { v4 as uuidv4 } from 'uuid';
  import {
    X,
    MoreVertical,
    MessageCircle,
    Reply,
    Pencil,
    Trash2,
    Clock,
    UserRound
  } from '@lucide/svelte';

  export let comments = [];
  export let highlightId = null;

  const dispatch = createEventDispatcher();

  let newCommentText = '';
  let activeMenuId = null;
  let editingCommentId = null;
  let editingText = '';
  let replyingToCommentId = null;
  let replyingToCommentText = '';
  let commentTextarea: HTMLTextAreaElement;

  function toggleMenu(commentId) {
    activeMenuId = activeMenuId === commentId ? null : commentId;
  }

  function closeAllMenus() {
    activeMenuId = null;
  }

  function handleAction(action, comment) {
    activeMenuId = null;
    if (action === 'delete') {
      dispatch('deletecomment', { highlightId, commentId: comment.id });
    } else if (action === 'edit') {
      editingCommentId = comment.id;
      editingText = comment.text;
    } else if (action === 'reply') {
      replyingToCommentId = comment.id;
      replyingToCommentText = comment.text;
      setTimeout(() => commentTextarea?.focus(), 0);
    }
  }

  function handleSaveEdit() {
    if (!editingText.trim() || !editingCommentId) return;
    dispatch('editcomment', {
      highlightId,
      commentId: editingCommentId,
      newText: editingText.trim()
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
      parentId: replyingToCommentId
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

  function handleKeydown(e) {
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      handleAddComment();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
{#if activeMenuId}
  <div class="fixed inset-0 z-10" on:click={closeAllMenus}></div>
{/if}

<div class="flex flex-col h-full">
  <!-- Header -->
  <div class="flex justify-between items-center mb-4 pb-3 border-b dark:border-gray-600">
    <div class="flex items-center gap-2">
      <MessageCircle class="w-4 h-4 text-gray-500 dark:text-gray-400" />
      <h2 class="text-sm font-semibold text-gray-900 dark:text-white">Comments</h2>
    </div>
    <button
      on:click={() => dispatch('close')}
      class="p-1 rounded-full hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors"
      aria-label="Close comments panel"
    >
      <X class="w-4 h-4 text-gray-500 dark:text-gray-400" />
    </button>
  </div>

  <!-- Comment list -->
  <div class="space-y-4 flex-grow overflow-y-auto pr-1 custom-scrollbar">
    {#each comments.filter((c) => !c.parentId) as comment (comment.id)}
      <div class="space-y-2">
        <!-- Parent comment -->
        <div
          class="p-3 rounded-xl bg-gray-50 dark:bg-gray-800/50 border border-gray-100 dark:border-gray-700 relative group"
        >
          {#if editingCommentId === comment.id}
            <textarea
              bind:value={editingText}
              class="w-full p-2 text-sm border rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white border-gray-300 dark:border-gray-600 focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none resize-none"
              rows="3"
              autocomplete="off"
            ></textarea>
            <div class="mt-2 flex justify-end gap-2">
              <button
                on:click={handleCancelEdit}
                class="px-3 py-1 rounded-lg text-xs font-medium bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-200 transition-colors"
                >Cancel</button
              >
              <button
                on:click={handleSaveEdit}
                class="px-3 py-1 rounded-lg text-xs font-medium bg-blue-600 text-white hover:bg-blue-700 transition-colors"
                >Save</button
              >
            </div>
          {:else}
            <!-- Avatar + timestamp header -->
            <div class="flex justify-between items-start mb-2">
              <div class="flex items-center gap-2">
                <div
                  class="w-6 h-6 rounded-full bg-gray-100 dark:bg-gray-600 border border-gray-300 dark:border-gray-500 flex items-center justify-center shrink-0"
                >
                  <UserRound class="w-3.5 h-3.5 text-gray-500 dark:text-gray-300" />
                </div>
                <div class="flex flex-col">
                  <span class="text-sm font-semibold text-gray-900 dark:text-white leading-none"
                    >User</span
                  >
                  <span
                    class="text-xs text-gray-500 dark:text-gray-400 flex items-center gap-0.5 mt-0.5"
                  >
                    <Clock class="w-2.5 h-2.5" />
                    {new Date(comment.updatedAt).toLocaleString()}
                  </span>
                </div>
              </div>
              <!-- Menu trigger -->
              <div class="relative">
                <button
                  on:click|stopPropagation={() => toggleMenu(comment.id)}
                  class="p-1 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors opacity-0 group-hover:opacity-100 focus:opacity-100"
                  aria-label="More options"
                >
                  <MoreVertical class="w-3.5 h-3.5 text-gray-500" />
                </button>
                {#if activeMenuId === comment.id}
                  <div
                    class="absolute right-0 mt-1 w-32 bg-white dark:bg-gray-900 rounded-lg shadow-lg z-20 border border-gray-200 dark:border-gray-700 py-1"
                  >
                    <button
                      on:click={() => handleAction('reply', comment)}
                      class="flex items-center gap-2 w-full text-left px-3 py-1.5 text-xs text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-800"
                    >
                      <Reply class="w-3 h-3" /> Reply
                    </button>
                    <button
                      on:click={() => handleAction('edit', comment)}
                      class="flex items-center gap-2 w-full text-left px-3 py-1.5 text-xs text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-800"
                    >
                      <Pencil class="w-3 h-3" /> Edit
                    </button>
                    <button
                      on:click={() => handleAction('delete', comment)}
                      class="flex items-center gap-2 w-full text-left px-3 py-1.5 text-xs text-red-600 dark:text-red-400 hover:bg-gray-50 dark:hover:bg-gray-800"
                    >
                      <Trash2 class="w-3 h-3" /> Delete
                    </button>
                  </div>
                {/if}
              </div>
            </div>
            <p class="text-sm text-gray-700 dark:text-gray-300 leading-relaxed">{comment.text}</p>
          {/if}
        </div>

        <!-- Replies -->
        {#each comments.filter((r) => r.parentId === comment.id) as reply (reply.id)}
          <div
            class="ml-6 p-3 rounded-xl bg-gray-100/50 dark:bg-gray-800/30 border border-gray-200/50 dark:border-gray-700/50 relative group"
          >
            {#if editingCommentId === reply.id}
              <textarea
                bind:value={editingText}
                class="w-full p-2 text-sm border rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white border-gray-300 dark:border-gray-600 focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none resize-none"
                rows="2"
                autocomplete="off"
                autocorrect="off"
                spellcheck="false"
              ></textarea>
              <div class="mt-2 flex justify-end gap-2">
                <button
                  on:click={handleCancelEdit}
                  class="px-3 py-1 rounded-lg text-xs font-medium bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-200 transition-colors"
                  >Cancel</button
                >
                <button
                  on:click={handleSaveEdit}
                  class="px-3 py-1 rounded-lg text-xs font-medium bg-blue-600 text-white hover:bg-blue-700 transition-colors"
                  >Save</button
                >
              </div>
            {:else}
              <div class="flex justify-between items-start mb-2">
                <div class="flex items-center gap-2">
                  <div
                    class="w-6 h-6 rounded-full bg-gray-100 dark:bg-gray-600 border border-gray-300 dark:border-gray-500 flex items-center justify-center shrink-0"
                  >
                    <UserRound class="w-3.5 h-3.5 text-gray-500 dark:text-gray-300" />
                  </div>
                  <div class="flex flex-col">
                    <span class="text-sm font-semibold text-gray-900 dark:text-white leading-none"
                      >User</span
                    >
                    <span
                      class="text-xs text-gray-500 dark:text-gray-400 flex items-center gap-0.5 mt-0.5"
                    >
                      <Clock class="w-2.5 h-2.5" />
                      {new Date(reply.updatedAt).toLocaleString()}
                    </span>
                  </div>
                </div>
                <div class="relative">
                  <button
                    on:click|stopPropagation={() => toggleMenu(reply.id)}
                    class="p-1 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors opacity-0 group-hover:opacity-100 focus:opacity-100"
                    aria-label="More options"
                  >
                    <MoreVertical class="w-3.5 h-3.5 text-gray-500" />
                  </button>
                  {#if activeMenuId === reply.id}
                    <div
                      class="absolute right-0 mt-1 w-32 bg-white dark:bg-gray-900 rounded-lg shadow-lg z-20 border border-gray-200 dark:border-gray-700 py-1"
                    >
                      <button
                        on:click={() => handleAction('edit', reply)}
                        class="flex items-center gap-2 w-full text-left px-3 py-1.5 text-xs text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-800"
                      >
                        <Pencil class="w-3 h-3" /> Edit
                      </button>
                      <button
                        on:click={() => handleAction('delete', reply)}
                        class="flex items-center gap-2 w-full text-left px-3 py-1.5 text-xs text-red-600 dark:text-red-400 hover:bg-gray-50 dark:hover:bg-gray-800"
                      >
                        <Trash2 class="w-3 h-3" /> Delete
                      </button>
                    </div>
                  {/if}
                </div>
              </div>
              <p class="text-sm text-gray-700 dark:text-gray-300 leading-relaxed">{reply.text}</p>
            {/if}
          </div>
        {/each}
      </div>
    {/each}

    {#if comments.length === 0}
      <div class="flex flex-col items-center justify-center py-12 text-gray-400 dark:text-gray-600">
        <MessageCircle class="w-10 h-10 mb-2 opacity-20" />
        <p class="text-sm italic">No comments yet.</p>
      </div>
    {/if}
  </div>

  <!-- Input area -->
  <div class="mt-4 pt-4 border-t dark:border-gray-600">
    {#if replyingToCommentId}
      <div
        class="flex items-center justify-between mb-2 px-2 py-1.5 bg-blue-50 dark:bg-blue-900/20 rounded-lg border border-blue-100 dark:border-blue-800/40"
      >
        <div class="flex items-center gap-2 overflow-hidden">
          <Reply class="w-3 h-3 text-blue-600 dark:text-blue-400 shrink-0" />
          <span class="text-[11px] text-blue-800 dark:text-blue-300 truncate italic">
            Replying to: "{replyingToCommentText}"
          </span>
        </div>
        <button
          on:click={cancelReply}
          class="p-0.5 hover:bg-blue-100 dark:hover:bg-blue-800 rounded transition-colors"
          aria-label="Cancel reply"
        >
          <X class="w-3 h-3 text-blue-600 dark:text-blue-400" />
        </button>
      </div>
    {/if}
    <textarea
      bind:this={commentTextarea}
      bind:value={newCommentText}
      on:keydown={handleKeydown}
      class="w-full p-2.5 text-sm border rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white border-gray-300 dark:border-gray-600 focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none resize-none placeholder-gray-400 dark:placeholder-gray-500"
      placeholder={replyingToCommentId ? 'Write a reply...' : 'Add a comment...'}
      rows="3"
      autocomplete="off"
      autocorrect="off"
      spellcheck="false"
    ></textarea>
    <div class="mt-2 flex justify-between items-center">
      <span class="text-[10px] text-gray-400 dark:text-gray-500">⌘↵ to submit</span>
      <button
        on:click={handleAddComment}
        disabled={!newCommentText.trim()}
        class="px-4 py-1.5 rounded-lg text-sm font-medium bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
      >
        {replyingToCommentId ? 'Post Reply' : 'Post Comment'}
      </button>
    </div>
  </div>
</div>

<style>
  .custom-scrollbar::-webkit-scrollbar {
    width: 4px;
  }
  .custom-scrollbar::-webkit-scrollbar-track {
    background: transparent;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background: #d1d5db;
    border-radius: 9999px;
  }
  :global(.dark) .custom-scrollbar::-webkit-scrollbar-thumb {
    background: #374151;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb:hover {
    background: #9ca3af;
  }
</style>
