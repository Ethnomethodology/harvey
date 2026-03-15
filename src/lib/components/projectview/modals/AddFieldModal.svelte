<!-- src/lib/components/projectview/modals/AddFieldModal.svelte -->
<script>
  import { createEventDispatcher } from 'svelte';
  import { message } from '@tauri-apps/plugin-dialog';
  import { addDefinition } from '$lib/stores/customFieldStore.js';
  import { 
    Modal,
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

<Modal
    bind:open={showModal}
    size="md"
    autoclose={false}
    outsideclose={true}
    on:close={closeModalAndDispatchClose}
    backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
    dialogClass="fixed top-0 start-0 end-0 h-modal md:h-full z-[10001] w-full p-4 flex items-center justify-center"
    class="w-full p-0 overflow-hidden flex flex-col"
    headerClass="px-6 py-5 border-b border-gray-200 dark:border-gray-800 flex justify-between items-center bg-gray-50/50 dark:bg-gray-800/50"
>
    <!-- Header -->
    <div slot="header" class="flex items-center space-x-3 w-full">
        <div class="p-2 bg-indigo-100 dark:bg-indigo-900/30 rounded-lg">
            <ChevronsLeftRightEllipsis size={20} class="text-indigo-600 dark:text-indigo-400" />
        </div>
        <h3 id="add-field-modal-title" class="text-lg font-bold text-gray-900 dark:text-white">
            Add Custom Field
        </h3>
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
                        <Radio bind:group={selectedScope} value="project" id="scopeProject" name="scope">
                            <span class="font-medium text-gray-900 dark:text-white">Global (Project-wide)</span>
                            <p class="text-xs font-normal text-gray-500 dark:text-gray-400">Available for all project items, including documents, media, and tables.</p>
                        </Radio>
                    </div>
                </div>

                <!-- Type-specific Scope -->
                <div class="flex" class:opacity-50={!currentItemType || currentItemType === 'project'}>
                    <div class="flex items-center h-5">
                        <Radio 
                            bind:group={selectedScope} 
                            value={currentItemType} 
                            id="scopeSpecific" 
                            name="scope"
                            disabled={!currentItemType || currentItemType === 'project'}
                        >
                            <span class="font-medium text-gray-900 dark:text-white">Type-specific ({displayType})</span>
                            <div class="text-xs font-normal text-gray-500 dark:text-gray-400">
                                Only applicable to {displayType}.
                                {#if !currentItemType || currentItemType === 'project'}
                                    <span class="text-amber-600 dark:text-amber-400 block font-medium mt-0.5">Select an asset to enable this scope.</span>
                                {/if}
                            </div>
                        </Radio>
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
    <div slot="footer" class="flex justify-end gap-3 w-full">
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
</Modal>

<style lang="postcss">
</style>
