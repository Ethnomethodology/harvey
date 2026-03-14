<!-- src/lib/components/modals/ConfigurationModal.svelte -->
<script>
  import { createEventDispatcher } from 'svelte';
  import ConfigurationView from '$lib/components/shared/ConfigurationView.svelte';
  import { Modal, Button } from 'flowbite-svelte';
  import { Settings } from 'lucide-svelte';

  export let showModal = false;

  const dispatch = createEventDispatcher();

  function close() {
    dispatch('close');
  }
</script>

<Modal
  bind:open={showModal}
  autoclose={false}
  outsideclose={true}
  dialogClass="fixed top-0 start-0 end-0 h-modal md:inset-0 md:h-full z-[200] w-full p-4 flex"
  class="w-full md:w-[50vw] md:max-w-[50vw] h-[80vh] flex flex-col p-0 overflow-hidden"
  bodyClass="flex-grow overflow-y-auto bg-white dark:bg-gray-900 p-0"
  headerClass="px-6 py-5 border-b border-gray-200 dark:border-gray-800 flex justify-between items-center bg-gray-50/50 dark:bg-gray-800/50"
  footerClass="px-6 py-4 border-t border-gray-200 dark:border-gray-800 flex justify-end gap-3 bg-gray-50/80 dark:bg-gray-800/80 backdrop-blur-md"
  on:close={close}
>
  <svelte:fragment slot="header">
    <div class="flex items-center space-x-3">
      <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
        <Settings size={20} class="text-blue-600 dark:text-blue-400" />
      </div>
      <h3 id="configuration-modal-title" class="text-lg font-bold text-gray-900 dark:text-white truncate">
        Configurations
      </h3>
    </div>
  </svelte:fragment>

  <ConfigurationView />

  <svelte:fragment slot="footer">
    <Button color="alternative" on:click={close} title="Close Configuration">
      Close
    </Button>
  </svelte:fragment>
</Modal>
