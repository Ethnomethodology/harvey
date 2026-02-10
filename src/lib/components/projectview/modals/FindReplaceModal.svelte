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

  let lastShowModal = false;
  $: if (showModal && !lastShowModal) {
    findTerm = initialSearchTerm;
    // We don't reset replaceTerm here if the user is just toggling or re-opening? 
    // Usually resetting on open is expected.
    replaceTerm = ''; 
    
    setTimeout(() => {
      tick().then(() => {
        // If there's already a search term, focus replace. Otherwise focus find.
        if (findTerm && replaceInput) {
            replaceInput.focus();
        } else if (findInput) {
            findInput.focus();
        }
      });
    }, 0);
  }
  $: lastShowModal = showModal;

  const dispatch = createEventDispatcher();

  function handleReplace() {
    dispatch('replace', { find: findTerm, replace: replaceTerm });
  }

  function handleReplaceAll() {
    dispatch('replaceall', { find: findTerm, replace: replaceTerm });
  }

  function handleFindChange() {
      dispatch('findchange', { term: findTerm });
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
</script>

{#if showModal}
  <div
    class="fixed inset-0 z-[120] flex items-center justify-center bg-black bg-opacity-50 backdrop-blur-sm"
    aria-labelledby="find-replace-modal-title"
    role="dialog"
    aria-modal="true"
    on:click|self={closeModal}
    on:keydown|stopPropagation={handleKeydown}
    tabindex="-1"
  >
    <div
      bind:this={modalElement}
      class="bg-white dark:bg-surface-2 rounded-lg shadow-xl p-4 w-full max-w-md mx-4 text-sm text-gray-900 dark:text-gray-200"
      role="document"
    >
      <h2 id="find-replace-modal-title" class="text-lg font-semibold mb-3">
        Find & Replace
      </h2>

      <div class="mb-3">
        <label
          for="find-term-input"
          class="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1"
        >
          Find
        </label>
        <input
          bind:this={findInput}
          id="find-term-input"
          type="text"
          bind:value={findTerm}
          on:input={handleFindChange}
          class="w-full px-3 py-1.5 border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500 text-sm"
          placeholder="Find..."
          autocomplete="off"
          autocorrect="off"
          autocapitalize="off"
          spellcheck="false"
        />
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
</style>