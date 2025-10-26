<script>
  import { createEventDispatcher, onMount, tick } from 'svelte';

  export let showModal = false;
  export let initialUrl = ''; // URL if editing, empty if adding
  export let isEditing = false; // Determines button labels and remove option

  let modalElement;
  let inputElement;
  let url = '';

  $: if (showModal) {
    url = initialUrl || 'https://';
    setTimeout(() => {
      tick().then(() => {
        if (inputElement) {
          inputElement.focus();
          try {
            if (!isEditing) {
              inputElement.setSelectionRange(8, 8);
            } else {
              inputElement.select();
            }
          } catch (e) {
            console.warn("Error setting input selection:", e);
            inputElement.focus();
          }
        }
      });
    }, 0);
  }

  const dispatch = createEventDispatcher();

  function handleConfirm() {
    if (url && url !== 'https://' && url.trim() !== '') {
      dispatch('confirm', { url: url.trim() });
      closeModal();
    } else {
      console.warn("Link URL is invalid or empty.");
      inputElement?.focus();
    }
  }

  function handleRemove() {
    dispatch('delete');
    closeModal();
  }

  function closeModal() {
    showModal = false;
    dispatch('close');
  }

  function handleKeydown(event) {
    // Listener is now on the modal div, so no showModal check needed here
    if (event.key === 'Enter') {
      event.preventDefault();
      handleConfirm();
    } else if (event.key === 'Escape') {
      closeModal();
    }
  }

  onMount(() => {
    // Component mounted
  });
</script>

<!-- Removed window listener -->

{#if showModal}
  <div
    class="fixed inset-0 z-[120] flex items-center justify-center bg-black bg-opacity-50 backdrop-blur-sm"
    aria-labelledby="link-modal-title"
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
      <h2 id="link-modal-title" class="text-lg font-semibold mb-3">
        {isEditing ? 'Edit Link' : 'Add Link'}
      </h2>

      <div class="mb-4">
        <label
          for="link-url-input"
          class="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1"
        >
          URL
        </label>
        <input
          bind:this={inputElement}
          id="link-url-input"
          type="text"
          bind:value={url}
          class="w-full px-3 py-1.5 border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500 text-sm"
          placeholder="https://example.com"
          on:keydown|stopPropagation={(e) => {
            // Allow Enter/Escape to bubble up to the main handler,
            // but stop others like arrow keys if needed inside input
            if (e.key !== 'Enter' && e.key !== 'Escape') {
              // e.stopPropagation(); // Optional: Only if arrow keys etc cause issues outside input
            }
          }}
        />
      </div>

      <div class="flex justify-end items-center gap-2 mt-5">
        {#if isEditing}
          <button
            type="button"
            class="btn-danger-secondary text-xs"
            on:click={handleRemove}
          >
            Remove Link
          </button>
          <div class="flex-grow"></div>
        {/if}
        <button
          type="button"
          class="btn-secondary text-xs"
          on:click={closeModal}
        >
          Cancel
        </button>
        <button
          type="button"
          class="btn-primary text-xs"
          on:click={handleConfirm}
        >
          {isEditing ? 'Update' : 'Add'}
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

  .btn-danger-secondary {
    @apply py-1 px-2 bg-transparent text-red-600 dark:text-red-400 border border-red-300 dark:border-red-600 rounded-md cursor-pointer
      text-xs font-medium transition-colors duration-150 ease-in-out;
  }
  .btn-danger-secondary:hover:not(:disabled) {
    @apply bg-red-50 dark:bg-red-900/30 border-red-400 dark:border-red-500;
  }
  .btn-danger-secondary:disabled {
    @apply bg-transparent text-red-300 dark:text-red-700 border-red-200 dark:border-red-800 opacity-50 cursor-not-allowed;
  }
</style>