<!-- src/lib/components/projectview/modals/LinkModal.svelte -->
<script>
  import { createEventDispatcher, onMount, tick } from 'svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { ExternalLink, Link as LinkIcon, Trash2 } from '@lucide/svelte';
  import { Modal, Button, Label, Input } from 'flowbite-svelte';

  export let showModal = false;
  export let initialUrl = ''; // URL if editing, empty if adding
  export let isEditing = false; // Determines button labels and remove option

  let modalElement;
  let url = '';

  $: if (showModal) {
    url = initialUrl || 'https://';
    setTimeout(() => {
      tick().then(() => {
        if (modalElement) {
          const el = modalElement.querySelector('input');
          if (el) {
            el.focus();
            try {
              if (!isEditing) {
                el.setSelectionRange(8, 8);
              } else {
                el.select();
              }
            } catch (e) {
              console.warn('Error setting input selection:', e);
              el.focus();
            }
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
      console.warn('Link URL is invalid or empty.');
      modalElement?.querySelector('input')?.focus();
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
    if (event.key === 'Enter') {
      event.preventDefault();
      handleConfirm();
    }
  }

  async function openExternalLink() {
    if (url && url !== 'https://' && url.trim() !== '') {
      try {
        await openUrl(url);
      } catch (e) {
        console.error('Failed to open link:', e);
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
  bodyClass="p-6 space-y-5 bg-white dark:bg-gray-900"
  headerClass="px-6 py-4 flex items-center justify-between border-b dark:border-gray-700 bg-gray-50/50"
  footerClass="px-6 py-4 flex items-center justify-between border-t dark:border-gray-700 bg-gray-50/80 backdrop-blur"
  on:close={closeModal}
>
  <div slot="header" class="flex items-center gap-2">
    <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
      <LinkIcon class="w-5 h-5 text-blue-600 dark:text-blue-400" />
    </div>
    <div class="flex flex-col">
      <h3 class="text-lg font-semibold text-gray-900 dark:text-white leading-tight">
        {isEditing ? 'Edit Link' : 'Add Link'}
      </h3>
      <p class="text-xs text-gray-500 dark:text-gray-400">Manage hyperlinked text</p>
    </div>
  </div>

  <div bind:this={modalElement} class="space-y-4">
    <div class="space-y-2">
      <Label
        for="link-url-input"
        class="text-xs font-bold uppercase tracking-wider text-gray-500 flex items-center gap-2"
      >
        <LinkIcon size={14} class="text-gray-400" />
        URL Address
      </Label>
      <div class="flex items-center gap-2">
        <Input
          id="link-url-input"
          type="text"
          bind:value={url}
          placeholder="https://example.com"
          on:keydown={handleKeydown}
          autocomplete="off"
          autocorrect="off"
          class="flex-grow bg-gray-50 dark:bg-gray-800"
        />
        <Button
          color="alternative"
          class="px-3"
          on:click={openExternalLink}
          disabled={!url || url === 'https://' || url.trim() === ''}
          title="Open Link"
        >
          <ExternalLink size={18} />
        </Button>
      </div>
    </div>
  </div>

  <svelte:fragment slot="footer">
    {#if isEditing}
      <Button color="red" outline on:click={handleRemove} title="Remove this link" class="px-3">
        <Trash2 class="w-4 h-4 mr-2" />
        Remove
      </Button>
    {/if}
    <div class="flex space-x-3 ml-auto">
      <Button color="alternative" on:click={closeModal} title="Cancel">Cancel</Button>
      <Button color="blue" on:click={handleConfirm} title={isEditing ? 'Update link' : 'Add link'}>
        {isEditing ? 'Update' : 'Add'}
      </Button>
    </div>
  </svelte:fragment>
</Modal>
