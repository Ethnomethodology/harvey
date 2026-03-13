<!-- src/lib/components/modals/ConfigurationModal.svelte -->
<script>
  import { createEventDispatcher } from 'svelte';
  import ConfigurationView from '$lib/components/shared/ConfigurationView.svelte';
  import { Modal, Button } from 'flowbite-svelte';

  export let showModal = false;

  const dispatch = createEventDispatcher();

  function close() {
    dispatch('close');
  }
</script>

<Modal bind:open={showModal} size="3xl" autoclose={false} outsideclose={true} class="w-full z-50" style="height: 90vh;" on:close={close}>
  <h2 id="configuration-modal-title" class="text-lg font-semibold text-gray-800" slot="header">Configurations</h2>

  <div class="flex-grow overflow-y-auto my-4 pr-2 -mr-2 h-full">
    <ConfigurationView />
  </div>

  <svelte:fragment slot="footer">
    <div class="flex justify-end space-x-3 w-full">
      <Button color="alternative" on:click={close}>
        Close
      </Button>
    </div>
  </svelte:fragment>
</Modal>
