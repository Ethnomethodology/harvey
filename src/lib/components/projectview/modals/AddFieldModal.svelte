<!-- src/lib/components/projectview/modals/AddFieldModal.svelte -->
<script>
  import { createEventDispatcher } from 'svelte';
  import { message } from '@tauri-apps/plugin-dialog'; // Using Tauri dialog for consistency

  export let showModal = false;

  let fieldName = '';
  let fieldType = 'small_text'; // Default to 'small_text'
  let fieldValue = '';

  const dispatch = createEventDispatcher();

  async function handleAdd() {
    const trimmedFieldName = fieldName.trim();
    if (!trimmedFieldName) {
      // Using Tauri's message dialog instead of alert()
      await message('Field Name cannot be empty.', { title: 'Validation Error', type: 'error' });
      return;
    }
    dispatch('confirm', {
      key: trimmedFieldName,
      type: fieldType,
      value: fieldValue.trim()
    });
    closeModal();
  }

  function closeModal() {
    fieldName = '';
    fieldType = 'small_text';
    fieldValue = '';
    // showModal = false; // Parent controls this prop, but good for internal state if used differently
    dispatch('close');
  }

  // Base input/select/textarea classes - adapted from prompt and project context
  const formElementClasses = "block w-full rounded-md border border-gray-300 dark:border-gray-600 focus:ring-1 focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:bg-gray-700 dark:text-white dark:placeholder-gray-400 px-3 py-2 bg-white text-gray-900 shadow-sm";

</script>

{#if showModal}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50 backdrop-blur-sm"
    on:click={closeModal}
    role="dialog"
    aria-modal="true"
    aria-labelledby="addFieldModalTitle"
  >
    <div
      class="bg-white dark:bg-gray-800 p-6 rounded-lg shadow-xl w-full max-w-md text-gray-900 dark:text-gray-100"
      on:click|stopPropagation
    >
      <h2 id="addFieldModalTitle" class="text-lg font-semibold mb-6 text-gray-900 dark:text-white">Add Custom Field</h2>

      <div class="space-y-4">
        <div>
          <label for="fieldNameInput" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Field Name</label>
          <input
            type="text"
            id="fieldNameInput"
            bind:value={fieldName}
            class="{formElementClasses}"
            placeholder="e.g., Case ID, Location"
          />
        </div>

        <div>
          <label for="fieldTypeSelect" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Field Type</label>
          <select
            id="fieldTypeSelect"
            bind:value={fieldType}
            class="{formElementClasses}"
          >
            <option value="small_text">Small Text</option>
            <option value="long_text">Long Text</option>
          </select>
        </div>

        <div>
          <label for="fieldValueInput" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Value</label>
          {#if fieldType === 'small_text'}
            <input
              type="text"
              id="fieldValueInput"
              bind:value={fieldValue}
              class="{formElementClasses}"
            />
          {:else if fieldType === 'long_text'}
            <textarea
              id="fieldValueInput"
              rows="3"
              bind:value={fieldValue}
              class="{formElementClasses}"
            ></textarea>
          {/if}
        </div>
      </div>

      <!-- Buttons -->
      <div class="mt-8 flex justify-end space-x-3">
        <button
          type="button"
          on:click={closeModal}
          class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-500 rounded-md shadow-sm hover:bg-gray-50 dark:hover:bg-gray-600 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 dark:focus:ring-offset-gray-800"
        >
          Cancel
        </button>
        <button
          type="button"
          on:click={handleAdd}
          class="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 border border-transparent rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:focus:ring-offset-gray-800"
        >
          Add Field
        </button>
      </div>
    </div>
  </div>
{/if}

<style lang="postcss">
  /* Basic focus style consistency if not fully covered by Tailwind focus classes */
  input:focus, select:focus, textarea:focus {
    outline: 2px solid transparent;
    outline-offset: 2px;
    --tw-ring-shadow: var(--tw-ring-inset) 0 0 0 calc(1px + var(--tw-ring-offset-width)) var(--tw-ring-color);
    box-shadow: var(--tw-ring-offset-shadow), var(--tw-ring-shadow), var(--tw-shadow, 0 0 #0000);
    border-color: var(--tw-ring-color); /* Ensure border color matches ring color on focus */
  }
</style>
