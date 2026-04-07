<script>
  import { createEventDispatcher } from 'svelte';

  export let showDialog = false;
  export let popupStyle = ''; // Style for positioning the dialog
  export let highlightOptions = []; // Array of color options
  export let defaultColor = 'rgba(255, 242, 117, 0.5)'; // Default yellow

  // New props for initial values
  export let initialTitle = '';
  export let initialDescription = '';
  export let initialColor = defaultColor;

  let title = initialTitle;
  let description = initialDescription;
  let selectedColor = initialColor;

  const dispatch = createEventDispatcher();

  function handleConfirm() {
    dispatch('confirm', {
      title,
      description,
      color: selectedColor
    });
    resetAndClose();
  }

  function handleCancel() {
    dispatch('cancel');
    resetAndClose();
  }

  function resetAndClose() {
    title = '';
    description = '';
    selectedColor = defaultColor;
    showDialog = false;
  }

  // Reactive block to update internal state when initial props change (e.g., for new annotation)
  $: if (showDialog) {
    title = initialTitle;
    description = initialDescription;
    selectedColor = initialColor;
  }
</script>

{#if showDialog}
  <div
    class="annotation-dialog absolute z-[1001] bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-700 rounded-md shadow-xl p-3 flex flex-col space-y-2"
    style={popupStyle}
  >
    <h3 class="text-lg font-semibold text-gray-900 dark:text-white">Add Annotation Details</h3>

    <div class="flex flex-col">
      <label
        for="annotation-title"
        class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Title (Optional)</label
      >
      <input
        type="text"
        id="annotation-title"
        bind:value={title}
        class="w-full px-2 py-1 border border-gray-300 dark:border-gray-700 rounded-md bg-gray-50 dark:bg-gray-800 text-gray-900 dark:text-white text-sm focus:ring-blue-500 focus:border-blue-500"
        placeholder="Enter title"
        autocomplete="off"
        autocorrect="off"
      />
    </div>

    <div class="flex flex-col">
      <label
        for="annotation-description"
        class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
        >Description (Optional)</label
      >
      <textarea
        id="annotation-description"
        bind:value={description}
        class="w-full px-2 py-1 border border-gray-300 dark:border-gray-700 rounded-md bg-gray-50 dark:bg-gray-800 text-gray-900 dark:text-white text-sm focus:ring-blue-500 focus:border-blue-500 h-16 resize-y"
        placeholder="Enter description"
        autocomplete="off"
        autocorrect="off"
      ></textarea>
    </div>

    <div class="flex flex-col">
      <span class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
        >Color (Mandatory)</span
      >
      <div class="flex items-center space-x-1">
        {#each highlightOptions as option (option.value)}
          <button
            title={option.label}
            class="w-6 h-6 rounded-full border border-gray-400 dark:border-gray-500 focus:outline-none focus:ring-2 focus:ring-offset-1 dark:focus:ring-offset-700"
            class:ring-blue-500={selectedColor === option.value}
            class:dark:ring-blue-400={selectedColor === option.value}
            class:ring-2={selectedColor === option.value}
            style:background-color={option.value}
            on:click={() => (selectedColor = option.value)}
          >
          </button>
        {/each}
      </div>
    </div>

    <div class="flex justify-end space-x-2 mt-3">
      <button
        class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-200 dark:bg-gray-600 rounded-md hover:bg-gray-300 dark:hover:bg-gray-500 focus:outline-none focus:ring-2 focus:ring-gray-500"
        on:click={handleCancel}
      >
        Cancel
      </button>
      <button
        class="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
        on:click={handleConfirm}
      >
        OK
      </button>
    </div>
  </div>
{/if}

<style lang="postcss">
  .annotation-dialog {
    min-width: 280px;
    max-width: 350px;
  }
</style>
