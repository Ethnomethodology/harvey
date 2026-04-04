<script>
  import { createEventDispatcher } from 'svelte';
  import { Modal, Button } from 'flowbite-svelte';
  import { RefreshCw } from '@lucide/svelte';

  export let showModal = false;
  export let fileName = 'the selected file';
  export let targetFormat = 'Lexical (.json)';

  const dispatch = createEventDispatcher();

  function handleConfirm() {
    dispatch('confirm');
  }

  function handleCancel() {
    dispatch('cancel');
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
  on:close={handleCancel}
>
  <div slot="header" class="flex items-center gap-2">
    <RefreshCw class="w-5 h-5 text-blue-500" />
    <h3 class="text-lg font-semibold text-gray-900 dark:text-white">Confirm Conversion</h3>
  </div>

  <div class="space-y-3">
    <p class="text-sm text-gray-700 dark:text-gray-300 leading-relaxed">
      The file "<span class="font-semibold text-gray-900 dark:text-white">{fileName}</span>" needs
      to be converted to
      <span class="font-semibold text-blue-600 dark:text-blue-400">{targetFormat}</span> before it can
      be imported.
    </p>
    <p class="text-xs text-gray-500 dark:text-gray-400 italic">
      This process may take a moment. Proceed with conversion and import?
    </p>
  </div>

  <svelte:fragment slot="footer">
    <Button color="alternative" on:click={handleCancel} title="Cancel and go back">Cancel</Button>
    <Button color="blue" on:click={handleConfirm} title="Convert and import the file">
      Convert & Import
    </Button>
  </svelte:fragment>
</Modal>
