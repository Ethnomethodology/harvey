<!-- src/lib/components/modals/HeaderConfirmationModal.svelte -->
<script>
  import { modalStore, hideHeaderConfirmationModal } from '$lib/stores/modalStore.js';
  import { Modal, Radio, Button } from 'flowbite-svelte';

  let hasHeaders = true;

  function handleConfirm() {
    const { onConfirm, headerConfirmationData } = $modalStore;
    if (onConfirm) {
      onConfirm(headerConfirmationData.tablePath, hasHeaders);
    }
    hideHeaderConfirmationModal();
  }

  function handleCancel() {
    hideHeaderConfirmationModal();
  }
</script>

<Modal
  bind:open={$modalStore.isHeaderConfirmationDialogOpen}
  size="md"
  autoclose={false}
  outsideclose={true}
  backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
  dialogClass="fixed top-0 start-0 end-0 h-modal md:h-full z-[10001] w-full p-4 flex items-center justify-center"
  class="w-full"
  on:close={handleCancel}
>
  <h2 class="text-lg font-bold" slot="header">Confirm Header Row</h2>

  <p class="mb-4">Does the first row of the imported file contain headers?</p>

  <div class="mb-4 space-y-2">
    <Radio bind:group={hasHeaders} value={true} class="mr-2">
      Yes, the first row contains headers.
    </Radio>
    <Radio bind:group={hasHeaders} value={false} class="mr-2">
      No, there are no headers in this file.
    </Radio>
  </div>

  <svelte:fragment slot="footer">
    <div class="flex justify-end space-x-4 w-full">
      <Button color="alternative" on:click={handleCancel}>Cancel</Button>
      <Button color="blue" on:click={handleConfirm}>Confirm</Button>
    </div>
  </svelte:fragment>
</Modal>
