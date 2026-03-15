<!-- src/lib/components/projectview/modals/AddFieldModal.svelte -->
<script>
  import { createEventDispatcher } from 'svelte';
  import { message } from '@tauri-apps/plugin-dialog';
  import { addDefinition } from '$lib/stores/customFieldStore.js';
  import { 
    Button, 
    Label, 
    Input, 
    Select, 
    Radio, 
    Helper, 
    Alert
  } from 'flowbite-svelte';
  import { ChevronsLeftRightEllipsis, X, AlertTriangle, Info, Database, PlusSquare, Crosshair } from 'lucide-svelte';

  export let showModal = false;
  export let currentItemType = ''; // e.g., "doc", "image", "project"

  let uiErrorMessage = '';
  let userInputFieldName = '';
  let generatedFieldKey = '';
  let fieldType = 'small_text';
  let selectedScope = 'project';

  const typeDisplayNames = {
    'doc': 'Documents',
    'image': 'Images',
    'media': 'Media Files',
    'imported_transcript': 'Transcripts',
    'transcript': 'Transcripts',
    'table': 'Tables',
    'note': 'Notes'
  };

  $: displayType = typeDisplayNames[currentItemType] || currentItemType || 'current type';

  const dispatch = createEventDispatcher();

  function sanitizeToKey(inputName) {
    if (!inputName) return '';
    const trimmed = inputName.trim();
    if (!trimmed) return '';

    return trimmed
      .toLowerCase()
      .replace(/\s+/g, '_')
      .replace(/_+/g, '_')
      .replace(/[^a-z0-9_]/g, '')
      .substring(0, 50);
  }

  $: generatedFieldKey = sanitizeToKey(userInputFieldName);
  $: if (userInputFieldName) uiErrorMessage = '';

  async function handleAdd() {
    uiErrorMessage = '';
    const finalFieldName = userInputFieldName.trim();
    const finalFieldKey = generatedFieldKey;

    if (!finalFieldName) {
      await message('Field Name cannot be empty.', { title: 'Validation Error', type: 'error' });
      return;
    }
    if (!finalFieldKey) {
      await message('Field Key cannot be generated from the Field Name. Please ensure it contains alphanumeric characters.', { title: 'Validation Error', type: 'error' });
      return;
    }
    if (!/^[a-z0-9_]+$/.test(finalFieldKey) || finalFieldKey.startsWith('_') || finalFieldKey.endsWith('_')) {
       await message('Generated Field Key is invalid (must be alphanumeric with underscores, not starting/ending with underscore). Please adjust Field Name.', { title: 'Validation Error', type: 'error' });
       return;
    }

    try {
      await addDefinition(finalFieldKey, finalFieldName, fieldType, selectedScope);
      closeModalAndDispatchClose();
    } catch (err) {
      uiErrorMessage = err.message || 'Failed to add custom field definition.';
    }
  }

  function closeModalAndDispatchClose() {
    userInputFieldName = '';
    fieldType = 'small_text';
    selectedScope = 'project';
    dispatch('close');
  }

  function handleKeydown(event) {
    if (event.key === 'Escape') {
      closeModalAndDispatchClose();
    }
  }

  const fieldTypeOptions = [
    { value: 'small_text', name: 'Small Text' },
    { value: 'long_text', name: 'Long Text' }
  ];
</script>

{#if showModal}
  <div
    class="fixed inset-0 z-[120] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
    role="dialog"
    aria-modal="true"
    aria-labelledby="add-field-modal-title"
    on:click={closeModalAndDispatchClose}
    tabindex="-1"
    on:keydown={handleKeydown}
  >
    <div
      class="bg-white dark:bg-gray-900 rounded-xl shadow-2xl w-full max-w-md flex flex-col border border-gray-200 dark:border-gray-800 overflow-hidden"
      on:click|stopPropagation
    >
      <!-- Header -->
      <div class="px-6 py-5 border-b border-gray-200 dark:border-gray-800 flex justify-between items-center bg-gray-50/50 dark:bg-gray-800/50">
        <div class="flex items-center space-x-3">
          <div class="p-2 bg-indigo-100 dark:bg-indigo-900/30 rounded-lg">
            <ChevronsLeftRightEllipsis size={20} class="text-indigo-600 dark:text-indigo-400" />
          </div>
          <h3 id="add-field-modal-title" class="text-lg font-bold text-gray-900 dark:text-white">
            Add Custom Field
          </h3>
        </div>
        <button on:click={closeModalAndDispatchClose} class="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-full transition-all" title="Close">
          <X size={20} />
        </button>
      </div>

      <!-- Body -->
      <div class="p-6 space-y-6">
        {#if uiErrorMessage}
          <Alert color="red" class="items-start">
            <AlertTriangle slot="icon" class="w-5 h-5 shrink-0" />
            <div class="ml-2 text-sm font-medium">
              {uiErrorMessage}
            </div>
          </Alert>
        {/if}

        <div class="space-y-2">
          <Label for="userInputFieldName">Field Name</Label>
          <Input
            id="userInputFieldName"
            bind:value={userInputFieldName}
            placeholder="e.g., Collected Date, Interviewer Name"
            autocorrect="off"
            autocomplete="off"
          />
          <Helper class="text-xs text-gray-500">
            Field names should be unique and descriptive.
          </Helper>
        </div>

        <div class="space-y-4 p-4 bg-gray-50 dark:bg-gray-800/50 border border-gray-100 dark:border-gray-700 rounded-lg">
          <div class="flex items-center gap-2 mb-1">
            <Crosshair size={14} class="text-gray-400" />
            <Label class="font-bold text-xs uppercase tracking-wider text-gray-500">Scope</Label>
          </div>
          
          <div class="space-y-4">
            <!-- Global Scope -->
            <div class="flex">
              <div class="flex items-center h-5">
                <input 
                  id="scopeProject" 
                  name="scope" 
                  type="radio" 
                  bind:group={selectedScope} 
                  value="project" 
                  class="w-4 h-4 text-blue-600 bg-white border-gray-300 focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600 cursor-pointer"
                >
              </div>
              <div class="ms-2 text-sm select-none">
                <label for="scopeProject" class="font-medium text-gray-900 dark:text-white cursor-pointer">Global (Project-wide)</label>
                <p class="text-xs font-normal text-gray-500 dark:text-gray-400">Available for all project items, including documents, media, and tables.</p>
              </div>
            </div>

            <!-- Type-specific Scope -->
            <div class="flex" class:opacity-50={!currentItemType || currentItemType === 'project'}>
              <div class="flex items-center h-5">
                <input 
                  id="scopeSpecific" 
                  name="scope" 
                  type="radio" 
                  bind:group={selectedScope} 
                  value={currentItemType}
                  disabled={!currentItemType || currentItemType === 'project'}
                  class="w-4 h-4 text-blue-600 bg-white border-gray-300 focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600 disabled:cursor-not-allowed cursor-pointer"
                >
              </div>
              <div class="ms-2 text-sm select-none">
                <label for="scopeSpecific" class="font-medium text-gray-900 dark:text-white" class:cursor-pointer={currentItemType && currentItemType !== 'project'} class:cursor-not-allowed={!currentItemType || currentItemType === 'project'}>
                  Type-specific ({displayType})
                </label>
                <div class="text-xs font-normal text-gray-500 dark:text-gray-400">
                  Only applicable to {displayType}.
                  {#if !currentItemType || currentItemType === 'project'}
                    <span class="text-amber-600 dark:text-amber-400 block font-medium mt-0.5">Select an asset to enable this scope.</span>
                  {/if}
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="space-y-2">
          <Label for="fieldTypeSelect">Field Type</Label>
          <Select
            id="fieldTypeSelect"
            items={fieldTypeOptions}
            bind:value={fieldType}
          />
        </div>
      </div>

      <!-- Footer -->
      <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-800 flex justify-end gap-3 bg-gray-50/80 dark:bg-gray-800/80 backdrop-blur-md">
        <Button color="alternative" on:click={closeModalAndDispatchClose} title="Cancel and close">
          Cancel
        </Button>
        <Button 
          color="blue" 
          on:click={handleAdd} 
          title="Create the new custom field definition"
        >
          <PlusSquare size={18} class="mr-2" />
          Add Field
        </Button>
      </div>
    </div>
  </div>
{/if}

<style lang="postcss">
</style>
