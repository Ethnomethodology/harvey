<!-- src/lib/components/projectview/modals/FindReplaceModal.svelte -->
<script>
  import { createEventDispatcher, onMount, tick, onDestroy } from 'svelte';
  import {
    CaseSensitive,
    Regex,
    WholeWord,
    X,
    Search,
    Replace,
    ReplaceAll,
    ChevronDown,
    ChevronUp
  } from '@lucide/svelte';
  import { Button, Label, Input, Helper, Badge, Tooltip } from 'flowbite-svelte';

  export let showModal = false;
  export let initialSearchTerm = '';
  export let currentMatchIndex = -1;
  export let totalMatches = 0;

  let modalElement;
  let findInputElement;
  let replaceInputElement;

  let findTerm = '';
  let replaceTerm = '';
  let isCaseSensitive = false;
  let isRegex = false;
  let isWholeWord = false;

  // Draggable state
  let x = 0;
  let y = 0;
  let isDragging = false;
  let startX, startY;

  let lastShowModal = false;
  let prevInitialSearchTerm = '';

  // Initialize on open
  $: if (showModal && !lastShowModal) {
    findTerm = initialSearchTerm || '';
    prevInitialSearchTerm = initialSearchTerm || '';
    replaceTerm = '';
    // Reset position when opening
    x = 0;
    y = 0;

    // Sync parent state with what's in modal now
    handleFindChange();

    setTimeout(() => {
      tick().then(() => {
        if (findTerm && replaceInputElement) {
          replaceInputElement.focus();
        } else if (findInputElement) {
          findInputElement.focus();
        }
      });
    }, 0);
  }
  $: lastShowModal = showModal;

  // Sync if initialSearchTerm changes externally while modal is open
  $: if (showModal && initialSearchTerm !== prevInitialSearchTerm) {
    findTerm = initialSearchTerm || '';
    prevInitialSearchTerm = initialSearchTerm;
  }

  const dispatch = createEventDispatcher();

  function handleReplace() {
    dispatch('replace', { find: findTerm, replace: replaceTerm });
  }

  function handleReplaceAll() {
    dispatch('replaceall', { find: findTerm, replace: replaceTerm });
  }

  function handleFindChange() {
    dispatch('findchange', { term: findTerm, isCaseSensitive, isRegex, isWholeWord });
  }

  function toggleCaseSensitive() {
    isCaseSensitive = !isCaseSensitive;
    handleFindChange();
  }

  function toggleRegex() {
    isRegex = !isRegex;
    handleFindChange();
  }

  function toggleWholeWord() {
    isWholeWord = !isWholeWord;
    handleFindChange();
  }

  function closeModal() {
    showModal = false;
    dispatch('close');
  }

  function handleKeydown(event) {
    if (event.key === 'Escape') {
      closeModal();
    }
  }

  function handlePointerDown(e) {
    if (e.target.closest('.drag-handle')) {
      isDragging = true;
      startX = e.clientX - x;
      startY = e.clientY - y;
      e.target.setPointerCapture(e.pointerId);
    }
  }

  function handlePointerMove(e) {
    if (!isDragging) return;
    x = e.clientX - startX;
    y = e.clientY - startY;
  }

  function handlePointerUp(e) {
    isDragging = false;
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
  });
</script>

{#if showModal}
  <div
    class="fixed inset-0 z-[120] flex items-center justify-center pointer-events-none"
    aria-labelledby="find-replace-modal-title"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
  >
    <div
      bind:this={modalElement}
      class="bg-white dark:bg-gray-900 rounded-xl shadow-2xl w-full max-w-md mx-4 flex flex-col border border-gray-200 dark:border-gray-800 overflow-hidden pointer-events-auto"
      style="transform: translate({x}px, {y}px);"
      role="document"
    >
      <!-- Header / Drag Handle -->
      <div
        class="drag-handle cursor-move px-6 py-4 border-b border-gray-200 dark:border-gray-800 flex justify-between items-center bg-gray-50/50 dark:bg-gray-800/50 select-none"
        on:pointerdown={handlePointerDown}
        on:pointermove={handlePointerMove}
        on:pointerup={handlePointerUp}
      >
        <div class="flex items-center space-x-3">
          <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
            <Search size={18} class="text-blue-600 dark:text-blue-400" />
          </div>
          <h3 id="find-replace-modal-title" class="text-lg font-bold text-gray-900 dark:text-white">
            Find & Replace
          </h3>
        </div>
        <button
          on:click={closeModal}
          class="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-full transition-all"
          title="Close"
        >
          <X size={20} />
        </button>
      </div>

      <div class="p-6 space-y-5">
        <!-- Find Section -->
        <div class="space-y-2">
          <div class="flex justify-between items-center">
            <Label for="find-term-input">Find</Label>
            {#if findTerm}
              <Badge color={totalMatches > 0 ? 'blue' : 'red'} size="xs" class="font-mono">
                {#if totalMatches > 0}
                  {currentMatchIndex + 1} / {totalMatches}
                {:else}
                  No matches
                {/if}
              </Badge>
            {/if}
          </div>
          <div class="flex gap-2">
            <div class="flex-grow">
              <Input
                id="find-term-input"
                type="text"
                bind:value={findTerm}
                on:input={handleFindChange}
                placeholder="Search text..."
                autocomplete="off"
                autocorrect="off"
              >
                <svelte:fragment slot="left">
                  <Search class="w-4 h-4 text-gray-400" />
                </svelte:fragment>
              </Input>
              <!-- Internal binding hack for focus since Flowbite doesn't easily expose the input ref -->
              <input type="hidden" bind:this={findInputElement} />
            </div>
            <div class="flex gap-1">
              <Button
                color="alternative"
                size="xs"
                class="px-2"
                on:click={() => dispatch('findprev')}
                disabled={totalMatches === 0}
                title="Previous match"
              >
                <ChevronUp size={16} />
              </Button>
              <Button
                color="alternative"
                size="xs"
                class="px-2"
                on:click={() => dispatch('findnext')}
                disabled={totalMatches === 0}
                title="Next match"
              >
                <ChevronDown size={16} />
              </Button>
            </div>
          </div>

          <!-- Search Options Toggles -->
          <div class="flex gap-2 pt-1">
            <button
              type="button"
              class="toggle-btn {isCaseSensitive ? 'active' : ''}"
              on:click={toggleCaseSensitive}
              id="case-sensitive-toggle"
            >
              <CaseSensitive size={16} />
              <Tooltip triggeredBy="#case-sensitive-toggle">Match Case</Tooltip>
            </button>
            <button
              type="button"
              class="toggle-btn {isRegex ? 'active' : ''}"
              on:click={toggleRegex}
              id="regex-toggle"
            >
              <Regex size={16} />
              <Tooltip triggeredBy="#regex-toggle">Use Regular Expression</Tooltip>
            </button>
            <button
              type="button"
              class="toggle-btn {isWholeWord ? 'active' : ''}"
              on:click={toggleWholeWord}
              id="whole-word-toggle"
            >
              <WholeWord size={16} />
              <Tooltip triggeredBy="#whole-word-toggle">Match Whole Word</Tooltip>
            </button>
          </div>
        </div>

        <!-- Replace Section -->
        <div
          class="space-y-2 bg-gray-50 dark:bg-gray-800/40 p-4 rounded-xl border border-gray-100 dark:border-gray-800"
        >
          <Label for="replace-term-input">Replace with</Label>
          <Input
            id="replace-term-input"
            type="text"
            bind:value={replaceTerm}
            placeholder="Replacement text..."
            on:keydown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                if (e.shiftKey) handleReplaceAll();
                else handleReplace();
              }
            }}
            autocomplete="off"
          />
          <input type="hidden" bind:this={replaceInputElement} />
          <Helper class="text-[10px] italic"
            >Press Enter to replace, Shift+Enter for Replace All</Helper
          >
        </div>
      </div>

      <!-- Footer -->
      <div
        class="px-6 py-4 border-t border-gray-200 dark:border-gray-800 flex justify-end gap-2 bg-gray-50/80 dark:bg-gray-800/80 backdrop-blur-md"
      >
        <Button color="alternative" on:click={closeModal} title="Close search">Close</Button>
        <Button
          color="blue"
          on:click={handleReplace}
          disabled={totalMatches === 0}
          title="Replace current match"
        >
          <Replace size={16} class="mr-2" />
          Replace
        </Button>
        <Button
          color="blue"
          on:click={handleReplaceAll}
          disabled={totalMatches === 0}
          title="Replace all occurrences"
        >
          <ReplaceAll size={16} class="mr-2" />
          Replace All
        </Button>
      </div>
    </div>
  </div>
{/if}

<style lang="postcss">
  .toggle-btn {
    @apply flex items-center justify-center w-8 h-8 rounded-lg border border-gray-200 dark:border-gray-700 
      bg-white dark:bg-gray-800 text-gray-500 dark:text-gray-400 transition-all duration-200 hover:bg-gray-50 dark:hover:bg-gray-700;
  }
  .toggle-btn.active {
    @apply bg-blue-50 dark:bg-blue-900/30 border-blue-500 dark:border-blue-400 text-blue-600 dark:text-blue-400 ring-2 ring-blue-500/20;
  }
</style>
