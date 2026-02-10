<script>
  import { createEventDispatcher, onMount, tick } from 'svelte';

  export let showModal = false;
  export let initialSearchTerm = '';
  export let currentMatchIndex = -1;
  export let totalMatches = 0;

  let modalElement;
  let findInput;
  let replaceInput;
  
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
  $: if (showModal && !lastShowModal) {
    findTerm = initialSearchTerm || '';
    replaceTerm = ''; 
    // Reset position when opening? Or preserve? Let's reset to center-ish.
    x = 0;
    y = 0;
    
    // Sync parent state with what's in modal now
    handleFindChange();

    setTimeout(() => {
      tick().then(() => {
        if (findTerm && replaceInput) {
            replaceInput.focus();
        } else if (findInput) {
            findInput.focus();
        }
      });
    }, 0);
  }
  $: lastShowModal = showModal;

  // Keep findTerm in sync with initialSearchTerm if initialSearchTerm changes from parent
  $: if (showModal && initialSearchTerm !== undefined && initialSearchTerm !== findTerm) {
      findTerm = initialSearchTerm;
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
      
      // Capture pointer to continue receiving events even if move outside handle
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
</script>

{#if showModal}
  <div
    class="fixed inset-0 z-[120] flex items-center justify-center pointer-events-none"
    aria-labelledby="find-replace-modal-title"
    role="dialog"
    aria-modal="true"
    on:keydown|stopPropagation={handleKeydown}
    tabindex="-1"
  >
    <div
      bind:this={modalElement}
      class="bg-white dark:bg-surface-2 rounded-lg shadow-2xl p-4 w-full max-w-md mx-4 text-sm text-gray-900 dark:text-gray-200 pointer-events-auto border border-gray-300 dark:border-border relative"
      style="transform: translate({x}px, {y}px);"
      role="document"
    >
      <!-- Header / Drag Handle -->
      <div 
        class="drag-handle cursor-move flex justify-between items-center mb-3 pb-2 border-b border-gray-100 dark:border-border select-none"
        on:pointerdown={handlePointerDown}
        on:pointermove={handlePointerMove}
        on:pointerup={handlePointerUp}
      >
        <h2 id="find-replace-modal-title" class="text-lg font-semibold">
          Find & Replace
        </h2>
        <button 
          on:click={closeModal}
          class="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-full transition-colors"
          title="Close"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" fill="currentColor" class="bi bi-x" viewBox="0 0 16 16">
            <path d="M4.646 4.646a.5.5 0 0 1 .708 0L8 7.293l2.646-2.647a.5.5 0 0 1 .708.708L8.707 8l2.647 2.646a.5.5 0 0 1-.708.708L8 8.707l-2.646 2.647a.5.5 0 0 1-.708-.708L7.293 8 4.646 5.354a.5.5 0 0 1 0-.708z"/>
          </svg>
        </button>
      </div>

      <div class="mb-3">
        <label
          for="find-term-input"
          class="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1"
        >
          Find
        </label>
        <div class="flex gap-2">
          <input
            bind:this={findInput}
            id="find-term-input"
            type="text"
            bind:value={findTerm}
            on:input={handleFindChange}
            class="flex-grow px-3 py-1.5 border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500 text-sm"
            placeholder="Find..."
            autocomplete="off"
            autocorrect="off"
            autocapitalize="off"
            spellcheck="false"
          />
          <button
            type="button"
            class="btn-primary text-xs whitespace-nowrap"
            on:click={() => dispatch('findnext')}
            disabled={totalMatches === 0}
          >
            Find
          </button>
        </div>
        <!-- Search Options Toggles -->
        <div class="flex gap-2 mt-1.5">
          <button
            type="button"
            class="toggle-btn {isCaseSensitive ? 'active' : ''}"
            title="Match Case"
            on:click={toggleCaseSensitive}
          >
            Aa
          </button>
          <button
            type="button"
            class="toggle-btn {isRegex ? 'active' : ''}"
            title="Use Regular Expression"
            on:click={toggleRegex}
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-regex" viewBox="0 0 16 16">
              <path fill-rule="evenodd" d="M3.05 3.05a7 7 0 0 0 0 9.9.5.5 0 0 1-.707.707 8 8 0 0 1 0-11.314.5.5 0 1 1 .707.707m9.9-.707a.5.5 0 0 1 .707 0 8 8 0 0 1 0 11.314.5.5 0 0 1-.707-.707 7 7 0 0 0 0-9.9.5.5 0 0 1 0-.707M6 11a1 1 0 1 1-2 0 1 1 0 0 1 2 0m5-6.5a.5.5 0 0 0-1 0v2.117L8.257 5.57a.5.5 0 0 0-.514.858L9.528 7.5 7.743 8.571a.5.5 0 1 0 .514.858L10 8.383V10.5a.5.5 0 1 0 1 0V8.383l1.743 1.046a.5.5 0 0 0 .514-.858L11.472 7.5l1.785-1.071a.5.5 0 1 0-.514-.858L11 6.617z"/>
            </svg>
          </button>
          <button
            type="button"
            class="toggle-btn {isWholeWord ? 'active' : ''}"
            title="Match Whole Word"
            on:click={toggleWholeWord}
          >
            <span class="underline decoration-1 underline-offset-2">ab</span>
          </button>
        </div>
      </div>

      <div class="mb-4">
        <label
          for="replace-term-input"
          class="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1"
        >
          Replace with
        </label>
        <input
          bind:this={replaceInput}
          id="replace-term-input"
          type="text"
          bind:value={replaceTerm}
          class="w-full px-3 py-1.5 border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500 text-sm"
          placeholder="Replace with..."
          on:keydown={(e) => {
              if (e.key === 'Enter') {
                  e.preventDefault();
                  if (e.shiftKey) handleReplaceAll();
                  else handleReplace();
              }
          }}
          autocomplete="off"
          autocorrect="off"
          autocapitalize="off"
          spellcheck="false"
        />
      </div>

      <div class="flex justify-between items-center mb-4 text-xs text-gray-500 dark:text-gray-400">
          <span>
              {#if totalMatches > 0}
                  {currentMatchIndex + 1} of {totalMatches} matches
              {:else if findTerm}
                  No matches
              {/if}
          </span>
      </div>

      <div class="flex justify-end items-center gap-2">
        <button
          type="button"
          class="btn-secondary text-xs"
          on:click={closeModal}
        >
          Close
        </button>
        <button
          type="button"
          class="btn-primary text-xs"
          on:click={handleReplace}
          disabled={totalMatches === 0}
        >
          Replace
        </button>
        <button
          type="button"
          class="btn-primary text-xs"
          on:click={handleReplaceAll}
          disabled={totalMatches === 0}
        >
          Replace All
        </button>
      </div>
    </div>
  </div>
{/if}

<style lang="postcss">
  .btn-primary {
    @apply py-1.5 px-4 bg-blue-500 text-white border-none rounded-md cursor-pointer text-sm font-medium
      transition-colors duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed disabled:bg-gray-400;
  }
  .btn-primary:hover:not(:disabled) {
    @apply bg-blue-600;
  }
  .btn-secondary {
    @apply py-1.5 px-4 bg-gray-200 text-gray-800 border border-gray-300 rounded-md cursor-pointer text-sm font-medium
      transition-colors duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed;
  }
  .btn-secondary:hover:not(:disabled) {
    @apply bg-gray-300 border-gray-400;
  }
  .btn-secondary:disabled {
    @apply bg-gray-100 text-gray-400 border-gray-200;
  }

  .toggle-btn {
    @apply flex items-center justify-center w-6 h-6 rounded border border-gray-300 dark:border-gray-600 
      bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-300 cursor-pointer transition-colors text-[10px] font-bold;
  }
  .toggle-btn:hover {
    @apply bg-gray-100 dark:bg-gray-600;
  }
  .toggle-btn.active {
    @apply bg-blue-100 dark:bg-blue-900 border-blue-500 dark:border-blue-400 text-blue-700 dark:text-blue-300;
  }
</style>