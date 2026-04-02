<script>
  import { createEventDispatcher, onMount } from 'svelte';
  import { confirm } from '@tauri-apps/plugin-dialog';
  import { Modal, Label, Input, Textarea, Button } from 'flowbite-svelte';
  import { Tag, Trash2 } from '@lucide/svelte';

  export let showModal = false;
  export let tag = null;

  const dispatch = createEventDispatcher();

  let currentName = '';
  let currentDescription = '';
  let isLoading = false;
  let errorMessage = '';

  onMount(() => {
    if (tag) {
      currentName = tag.name;
      currentDescription = tag.description || '';
    }
  });

  function closeModal() {
    if (isLoading) return;
    dispatch('close');
  }

  async function handleSave() {
    if (!currentName.trim()) {
      errorMessage = 'Tag name cannot be empty.';
      return;
    }
    errorMessage = '';
    isLoading = true;
    try {
      dispatch('save', {
        id: tag.id,
        name: currentName,
        description: currentDescription
      });
    } catch (error) {
      errorMessage = `Failed to save tag: ${error.message}`;
    } finally {
      isLoading = false;
    }
  }

  async function handleDelete() {
    const confirmed = await confirm(
      `Are you sure you want to delete the tag "${tag.name}"? This will remove the tag from all associated highlights and cannot be undone.`,
      {
        title: 'Confirm Deletion',
        type: 'warning'
      }
    );

    if (confirmed) {
      isLoading = true;
      try {
        dispatch('delete', { id: tag.id });
      } catch (error) {
        errorMessage = `Failed to delete tag: ${error.message}`;
      } finally {
        isLoading = false;
      }
    }
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
  footerClass="px-6 py-4 flex items-center justify-between border-t dark:border-gray-700 bg-gray-50/80 backdrop-blur"
  on:close={closeModal}
>
  <div slot="header" class="flex items-center gap-2">
    <Tag class="w-5 h-5 text-gray-500" />
    <h3 class="text-lg font-semibold text-gray-900 dark:text-white">Edit Tag</h3>
  </div>

  {#if errorMessage}
    <div
      class="p-3 bg-red-100 dark:bg-red-700 border border-red-300 dark:border-red-600 text-red-700 dark:text-red-100 rounded-md text-sm"
    >
      {errorMessage}
    </div>
  {/if}

  <div class="space-y-4">
    <div>
      <Label for="editTagName" class="mb-1 text-sm font-medium text-gray-700 dark:text-gray-300"
        >Tag Name</Label
      >
      <Input
        type="text"
        id="editTagName"
        bind:value={currentName}
        required
        autocomplete="off"
        autocorrect="off"
        autocapitalize="off"
        spellcheck="false"
      />
    </div>

    <div>
      <Label
        for="editTagDescription"
        class="mb-1 text-sm font-medium text-gray-700 dark:text-gray-300"
        >Description (Optional)</Label
      >
      <Textarea
        id="editTagDescription"
        bind:value={currentDescription}
        rows="3"
        autocomplete="off"
        autocorrect="off"
        autocapitalize="off"
        spellcheck="false"
      ></Textarea>
    </div>
  </div>

  <svelte:fragment slot="footer">
    <Button
      color="red"
      outline
      on:click={handleDelete}
      disabled={isLoading}
      title="Delete this tag"
      class="px-3"
    >
      <Trash2 class="w-4 h-4 mr-2" />
      Delete
    </Button>
    <div class="flex space-x-3">
      <Button color="alternative" on:click={closeModal} disabled={isLoading} title="Cancel editing">
        Cancel
      </Button>
      <Button
        color="blue"
        on:click={handleSave}
        disabled={isLoading || !currentName.trim()}
        title="Save changes"
      >
        {#if isLoading}
          Saving...
        {:else}
          Save Changes
        {/if}
      </Button>
    </div>
  </svelte:fragment>
</Modal>
