<script>
  import { createEventDispatcher } from 'svelte';
  import { Modal, Label, Input, Textarea, Button } from 'flowbite-svelte';
  import { Tag } from '@lucide/svelte';

  export let showModal = false;

  const dispatch = createEventDispatcher();

  let name = '';
  let description = '';
  let isLoading = false;
  let errorMessage = '';

  $: if (showModal) {
    name = '';
    description = '';
    errorMessage = '';
    isLoading = false;
  }

  function closeModal() {
    if (isLoading) return;
    dispatch('close');
  }

  async function handleSave() {
    if (!name.trim()) {
      errorMessage = 'Tag name is required.';
      return;
    }
    errorMessage = '';
    isLoading = true;
    try {
      dispatch('save', {
        name: name.trim(),
        description: description.trim()
      });
    } catch (error) {
      errorMessage = `Failed to create tag: ${error.message}`;
    } finally {
      isLoading = false;
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
  footerClass="px-6 py-4 flex items-center justify-end space-x-3 rtl:space-x-reverse border-t dark:border-gray-700 bg-gray-50/80 backdrop-blur"
  on:close={closeModal}
>
  <div slot="header" class="flex items-center gap-2">
    <Tag class="w-5 h-5 text-gray-500" />
    <h3 class="text-lg font-semibold text-gray-900 dark:text-white">Create Tag</h3>
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
      <Label for="tagName" class="mb-1 text-sm font-medium text-gray-700 dark:text-gray-300"
        >Tag Name</Label
      >
      <Input
        type="text"
        id="tagName"
        bind:value={name}
        required
        autocomplete="off"
        autocorrect="off"
        autocapitalize="off"
        spellcheck="false"
        placeholder="e.g., Important, Review"
      />
    </div>

    <div>
      <Label for="tagDescription" class="mb-1 text-sm font-medium text-gray-700 dark:text-gray-300"
        >Description (Optional)</Label
      >
      <Textarea
        id="tagDescription"
        bind:value={description}
        rows="3"
        autocomplete="off"
        autocorrect="off"
        autocapitalize="off"
        spellcheck="false"
        placeholder="Describe this tag..."
      ></Textarea>
    </div>
  </div>

  <svelte:fragment slot="footer">
    <Button
      color="alternative"
      on:click={closeModal}
      disabled={isLoading}
      title="Cancel adding tag"
    >
      Cancel
    </Button>
    <Button
      color="blue"
      on:click={handleSave}
      disabled={isLoading || !name.trim()}
      title="Save new tag"
    >
      {#if isLoading}
        Creating...
      {:else}
        Create Tag
      {/if}
    </Button>
  </svelte:fragment>
</Modal>
