<!-- src/lib/components/projectview/modals/FileRenameModal.svelte -->
<script>
  import { createEventDispatcher, onMount, tick } from 'svelte';
  import { Modal, Label, Input, Button, Helper } from 'flowbite-svelte';
  import { PencilLine } from '@lucide/svelte';

  export let showModal = false;
  export let currentName = '';
  export let itemType = ''; // 'media', 'transcript', 'note', 'doc', 'other'
  export let isMediaRename = false; // Explicit prop if needed, though itemType usually sufficient

  let newNameBase = '';
  let inputElement;
  let errorMessage = '';

  const dispatch = createEventDispatcher();

  // Determines if the user should only input a base name (stem).
  // 'transcript' here refers to media-associated transcripts which become .json.
  $: isStemInputMode = [
    'media',
    'doc',
    'table',
    'image',
    'standalone_transcript',
    'note',
    'transcript'
  ].includes(itemType);

  let currentBaseName = '';
  let currentExtension = '';
  let currentDisplayName = '';

  $: titleType =
    itemType === 'standalone_transcript'
      ? 'Transcript'
      : itemType === 'survey view'
        ? 'Survey View'
        : itemType
          ? itemType.charAt(0).toUpperCase() + itemType.slice(1)
          : 'Item';

  function updateNameParts() {
    if (currentName) {
      const lastDotIndex = currentName.lastIndexOf('.');
      if (lastDotIndex > 0 && lastDotIndex < currentName.length - 1) {
        // Found a likely extension
        currentBaseName = currentName.substring(0, lastDotIndex);
        currentExtension = currentName.substring(lastDotIndex);
        // Show only stem for types where user provides stem
        if (isStemInputMode) {
          currentDisplayName = currentBaseName;
        } else {
          currentDisplayName = currentName; // Show full name for others by default
        }
      } else {
        // No extension found or dot is at the beginning
        currentBaseName = currentName;
        currentExtension = '';
        currentDisplayName = currentName; // Show the full name if no extension
        // console.warn(`[Rename Modal] Could not extract extension from '${currentName}' for itemType '${itemType}'.`);
      }
    } else {
      // No current name provided
      currentBaseName = '';
      currentExtension = '';
      currentDisplayName = '';
    }
    newNameBase = currentBaseName; // Initialize input with base name
  }

  $: if (currentName || itemType) {
    updateNameParts();
  }

  $: if (showModal) {
    errorMessage = '';
    updateNameParts(); // Ensure parts are recalculated when shown
    tick().then(() => {
      inputElement?.focus();
      inputElement?.select();
    });
  }

  function handleConfirm() {
    const baseNameInput = newNameBase.trim();
    errorMessage = ''; // Clear previous error

    if (!baseNameInput) {
      errorMessage = 'Name cannot be empty.';
    } else if (/[<>:"/\\|?*]/.test(baseNameInput)) {
      errorMessage = 'Name contains invalid characters (< > : " / \\ | ? *).';
    } else if (baseNameInput.startsWith('.')) {
      errorMessage = 'Name cannot start with a dot.';
    }

    if (isStemInputMode) {
      // For stem inputs, disallow dots in the stem itself,
      // except for media-associated 'transcript' type where stem can have dots before .json is added.
      if (baseNameInput.includes('.') && itemType !== 'transcript') {
        errorMessage = `Base name for ${itemType} cannot contain dots. Extension is handled automatically.`;
      }
    } else {
      // User provides full name, must include an extension.
      if (!baseNameInput.includes('.') && itemType !== 'survey view') {
        errorMessage = 'Filename must include an extension.';
      }
    }

    if (errorMessage) {
      return;
    }

    let nameToSend = '';
    let isSameName = false;

    if (
      itemType === 'media' ||
      itemType === 'doc' ||
      itemType === 'table' ||
      itemType === 'image' ||
      itemType === 'standalone_transcript'
    ) {
      nameToSend = baseNameInput; // Send stem
      if (nameToSend === currentBaseName) {
        isSameName = true;
      }
    } else if (
      itemType === 'note' ||
      itemType === 'audio_transcript' ||
      itemType === 'video_transcript'
    ) {
      // 'transcript' here is media-associated
      // These types have a fixed .json extension added to the stem.
      nameToSend = `${baseNameInput}.json`;
      if (nameToSend === currentName) {
        isSameName = true;
      }
    } else {
      // User provided full name (e.g., a generic file type not explicitly handled as stem input)
      nameToSend = baseNameInput;
      if (nameToSend === currentName) {
        isSameName = true;
      }
    }

    if (isSameName) {
      errorMessage = 'New name is the same as the current name.';
      return;
    }

    console.log(
      `[Rename Modal] Dispatching confirm. Item Type: '${itemType}', Name to Send: '${nameToSend}' (Original Full: '${currentName}', Original Base: '${currentBaseName}')`
    );
    dispatch('confirm', { newName: nameToSend });
    closeModal();
  }

  function handleKeyDown(event) {
    if (event.key === 'Enter') {
      handleConfirm();
    } else if (event.key === 'Escape') {
      closeModal();
    }
  }

  function closeModal() {
    showModal = false;
    dispatch('close');
  }
</script>

<Modal
  bind:open={showModal}
  size="sm"
  autoclose={false}
  outsideclose={true}
  class="w-full"
  backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
  dialogClass="fixed top-0 start-0 end-0 h-modal md:inset-0 md:h-full z-[10001] flex"
  bodyClass="p-6 space-y-4 bg-white dark:bg-gray-900"
  headerClass="px-6 py-4 flex items-center justify-between border-b dark:border-gray-700 bg-gray-50/50"
  footerClass="px-6 py-4 flex items-center justify-end space-x-3 rtl:space-x-reverse border-t dark:border-gray-700 bg-gray-50/80 backdrop-blur"
  on:close={closeModal}
>
  <div slot="header" class="flex items-center gap-2">
    <PencilLine class="w-5 h-5 text-gray-500" />
    <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
      Rename {titleType}
    </h3>
  </div>

  <div class="space-y-4">
    <div>
      <Label
        for="current-name-display"
        class="mb-1 text-sm font-medium text-gray-700 dark:text-gray-400">Current name:</Label
      >
      <Input
        id="current-name-display"
        type="text"
        readonly
        value={currentDisplayName}
        class="cursor-not-allowed bg-gray-100 dark:bg-gray-700"
        autocomplete="off"
        autocorrect="off"
        autocapitalize="off"
        spellcheck="false"
      />
    </div>

    <div>
      <Label for="new-name" class="mb-1 text-sm font-medium text-gray-700 dark:text-gray-300"
        >New name:</Label
      >
      <Input
        bind:this={inputElement}
        bind:value={newNameBase}
        on:keydown={handleKeyDown}
        id="new-name"
        type="text"
        required
        autocomplete="off"
        autocorrect="off"
        autocapitalize="off"
        spellcheck="false"
      />
      {#if isStemInputMode}
        <Helper class="mt-1 text-xs text-gray-500 dark:text-gray-400">
          {#if (itemType === 'doc' || itemType === 'table' || itemType === 'image') && currentExtension}
            Enter the new file name.
          {:else if itemType === 'note' || itemType === 'audio_transcript' || itemType === 'video_transcript'}
            Enter the new file name.
          {:else if itemType === 'media' || itemType === 'standalone_transcript'}
            Enter the new file name.
          {:else}
            Enter just the file name. The original extension '<code
              >{currentExtension || '.ext'}</code
            >' will be used.
          {/if}
        </Helper>
      {:else}
        <Helper class="mt-1 text-xs text-gray-500 dark:text-gray-400">
          Enter the full new filename including the extension.
        </Helper>
      {/if}
    </div>

    {#if errorMessage}
      <p class="text-sm text-red-600 dark:text-red-400" role="alert">{errorMessage}</p>
    {/if}
  </div>

  <svelte:fragment slot="footer">
    <Button color="alternative" on:click={closeModal} title="Cancel renaming">Cancel</Button>
    <Button
      color="blue"
      on:click={handleConfirm}
      disabled={!newNameBase.trim() || !!errorMessage}
      title="Save changes"
    >
      Rename
    </Button>
  </svelte:fragment>
</Modal>
