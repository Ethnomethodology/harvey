<!-- src/lib/components/projectview/modals/InsertTableModal.svelte -->
<script>
    import { createEventDispatcher, onMount } from 'svelte';
  
    export let showModal = false;
  
    let rows = 2;
    let columns = 3;
    let rowsInput;
    let columnsInput;
  
    const dispatch = createEventDispatcher();
  
    function handleConfirm() {
      if (rows > 0 && columns > 0) {
        dispatch('confirm', { rows, columns });
        closeModal();
      } else {
        alert('Please enter valid numbers for rows and columns (must be greater than 0).');
      }
    }
  
    function closeModal() {
      dispatch('close');
    }
  
    function handleKeydown(event) {
      if (event.key === 'Escape') {
        closeModal();
      }
      if (event.key === 'Enter') {
          if (document.activeElement === rowsInput || document.activeElement === columnsInput) {
              handleConfirm();
          }
      }
    }
  
    $: if (showModal && rowsInput) {
      setTimeout(() => rowsInput.focus(), 50);
    }
  </script>
  
  {#if showModal}
    <div class="fixed inset-0 z-[120] flex items-center justify-center bg-black bg-opacity-50" on:click={closeModal} role="dialog" aria-modal="true">
      <div class="bg-white dark:bg-surface-2 p-6 rounded-lg shadow-xl w-full max-w-sm text-gray-900 dark:text-gray-100" on:click|stopPropagation on:keydown={handleKeydown}>
        <h2 class="text-xl font-semibold mb-4">Insert Table</h2>
        
        <div class="mb-4">
          <label for="table-rows" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Rows:</label>
          <input
            type="number"
            id="table-rows"
            bind:this={rowsInput}
            bind:value={rows}
            min="1"
            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-gray-50"
            autocomplete="off"
            autocorrect="off"
          />
        </div>
  
        <div class="mb-6">
          <label for="table-columns" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Columns:</label>
          <input
            type="number"
            id="table-columns"
            bind:this={columnsInput}
            bind:value={columns}
            min="1"
            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-gray-50"
            autocomplete="off"
            autocorrect="off"
          />
        </div>
  
        <div class="flex justify-end space-x-3">
          <button
            type="button"
            class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 border border-gray-300 dark:border-gray-500 rounded-md focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:focus:ring-offset-gray-800"
            on:click={closeModal}
          >
            Cancel
          </button>
          <button
            type="button"
            class="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 border border-transparent rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:focus:ring-offset-gray-800"
            on:click={handleConfirm}
          >
            Insert
          </button>
        </div>
      </div>
    </div>
  {/if}