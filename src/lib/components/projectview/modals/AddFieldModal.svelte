<!-- src/lib/components/projectview/modals/AddFieldModal.svelte -->
<script>
  import { createEventDispatcher } from 'svelte';
  import { message } from '@tauri-apps/plugin-dialog';
  import { addDefinition } from '$lib/stores/customFieldStore.js';

  export let showModal = false;
  export let currentItemType = ''; // e.g., "doc", "image", "project" (if 'project' is a possibility for currentItemType)

  let fieldKey = '';
  let fieldNameDisplay = '';
  let fieldType = 'small_text'; // Default to 'small_text'
  let fieldValue = ''; // This is for the default_value of the definition
  let selectedScope = 'project'; // Default scope

  const dispatch = createEventDispatcher();

  async function handleAdd() {
    const trimmedFieldKey = fieldKey.trim();
    const trimmedFieldNameDisplay = fieldNameDisplay.trim();

    if (!trimmedFieldKey) {
      await message('Field Key cannot be empty.', { title: 'Validation Error', type: 'error' });
      return;
    }
    // Basic validation for fieldKey format (no spaces, alphanumeric + underscore/hyphen)
    if (!/^[a-zA-Z0-9_-]+$/.test(trimmedFieldKey)) {
        await message('Field Key can only contain letters, numbers, underscores, and hyphens (no spaces or special characters).', { title: 'Validation Error', type: 'error' });
        return;
    }
    if (!trimmedFieldNameDisplay) {
      await message('Field Name (Display) cannot be empty.', { title: 'Validation Error', type: 'error' });
      return;
    }

    try {
      // The scopeStr is simply selectedScope. If currentItemType is chosen, selectedScope will hold its value.
      await addDefinition(trimmedFieldKey, trimmedFieldNameDisplay, fieldType, selectedScope, fieldValue.trim() || null);
      // addDefinition in store already calls loadAllDefinitions()
      closeModalAndDispatchClose(); // Close modal on success
    } catch (err) {
      console.error("Error adding custom field definition:", err);
      await message(err.message || 'Failed to add custom field definition.', { title: 'Error', type: 'error' });
      // Do not close modal on error
    }
  }

  function closeModalAndDispatchClose() {
    fieldKey = '';
    fieldNameDisplay = '';
    fieldType = 'small_text';
    fieldValue = '';
    selectedScope = 'project';
    dispatch('close'); // Parent controls showModal prop
  }


  // Base input/select/textarea classes
  const formElementClasses = "block w-full rounded-md border border-gray-300 dark:border-gray-600 focus:ring-1 focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:bg-gray-700 dark:text-white dark:placeholder-gray-400 px-3 py-2 bg-white text-gray-900 shadow-sm";

</script>

{#if showModal}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50 backdrop-blur-sm"
    on:click={closeModalAndDispatchClose}
    role="dialog"
    aria-modal="true"
    aria-labelledby="addFieldModalTitle"
  >
    <div
      class="bg-white dark:bg-gray-800 p-6 rounded-lg shadow-xl w-full max-w-md text-gray-900 dark:text-gray-100"
      on:click|stopPropagation
    >
      <h2 id="addFieldModalTitle" class="text-lg font-semibold mb-6 text-gray-900 dark:text-white">Add Custom Field Definition</h2>

      <div class="space-y-4">
        <div>
          <label for="fieldKeyInput" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Field Key</label>
          <input
            type="text"
            id="fieldKeyInput"
            bind:value={fieldKey}
            class="{formElementClasses}"
            placeholder="e.g., case_id, photo_location (unique ID)"
            autocorrect="off"
            autocomplete="off"
          />
        </div>

        <div>
          <label for="fieldNameDisplayInput" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Field Name (Display)</label>
          <input
            type="text"
            id="fieldNameDisplayInput"
            bind:value={fieldNameDisplay}
            class="{formElementClasses}"
            placeholder="e.g., Case ID, Photo Location"
            autocorrect="off"
            autocomplete="off"
          />
        </div>

        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Scope</label>
          <div class="mt-1 space-y-2">
            <div class="flex items-center">
              <input id="scopeProject" name="scope" type="radio" bind:group={selectedScope} value={"project"}
                     class="focus:ring-blue-500 h-4 w-4 text-blue-600 border-gray-300 dark:border-gray-600 dark:bg-gray-700 dark:focus:ring-blue-600">
              <label for="scopeProject" class="ml-2 block text-sm text-gray-900 dark:text-gray-300">
                Make available across the project
              </label>
            </div>
            <div class="flex items-center">
              <input id="scopeSpecific" name="scope" type="radio" bind:group={selectedScope} value={currentItemType}
                     disabled={!currentItemType || currentItemType === 'project'}
                     class="focus:ring-blue-500 h-4 w-4 text-blue-600 border-gray-300 dark:border-gray-600 dark:bg-gray-700 dark:focus:ring-blue-600"
                     class:cursor-not-allowed={!currentItemType || currentItemType === 'project'}
                     class:opacity-50={!currentItemType || currentItemType === 'project'}>
              <label for="scopeSpecific" class="ml-2 block text-sm text-gray-900 dark:text-gray-300"
                     class:opacity-50={!currentItemType || currentItemType === 'project'}>
                Only applicable to {currentItemType || 'current type'}
                {#if !currentItemType || currentItemType === 'project'}
                    <span class="text-xs text-gray-500 dark:text-gray-400"> (Select an asset to enable this scope)</span>
                {/if}
              </label>
            </div>
          </div>
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
            <!-- Add other types like number, date, boolean as needed -->
          </select>
        </div>

        <div>
          <label for="fieldValueInput" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Default Value (Optional)</label>
          {#if fieldType === 'small_text'}
            <input
              type="text"
              id="fieldValueInput"
              bind:value={fieldValue}
              class="{formElementClasses}"
              placeholder="Enter default value for this field"
              autocorrect="off"
              autocomplete="off"
            />
          {:else if fieldType === 'long_text'}
            <textarea
              id="fieldValueInput"
              rows="3"
              bind:value={fieldValue}
              class="{formElementClasses}"
              placeholder="Enter default value for this field"
              autocorrect="off"
              autocomplete="off"
            ></textarea>
          {/if}
          <!-- Add inputs for other field types if necessary -->
        </div>
      </div>

      <!-- Buttons -->
      <div class="mt-8 flex justify-end space-x-3">
        <button
          type="button"
          on:click={closeModalAndDispatchClose}
          class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-500 rounded-md shadow-sm hover:bg-gray-50 dark:hover:bg-gray-600 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 dark:focus:ring-offset-gray-800"
        >
          Cancel
        </button>
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
