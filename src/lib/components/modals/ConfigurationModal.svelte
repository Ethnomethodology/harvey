<!-- src/lib/components/modals/ConfigurationModal.svelte -->
<script>
  import { createEventDispatcher } from 'svelte';
  import ConfigurationView from '$lib/components/shared/ConfigurationView.svelte';
  import { Modal, Button } from 'flowbite-svelte';
  import { Settings } from '@lucide/svelte';

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
  placement="center"
  size="lg"
  backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
  dialogClass="fixed top-0 start-0 end-0 h-modal md:h-full z-[10001] w-full p-4 flex items-center justify-center"
  class="h-[80vh] flex flex-col p-0 overflow-hidden mx-auto"
  bodyClass="flex-grow overflow-y-auto bg-white dark:bg-gray-900 p-0 pb-8"
  headerClass="px-6 py-5 border-b border-gray-200 dark:border-gray-800 flex justify-between items-center bg-gray-50/50 dark:bg-gray-800/50"
  on:close={close}
>
  <svelte:fragment slot="header">
    <div class="flex items-center space-x-3">
      <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
        <Settings size={20} class="text-blue-600 dark:text-blue-400" />
      </div>
      <h3
        id="configuration-modal-title"
        class="text-lg font-bold text-gray-900 dark:text-white truncate"
      >
        Configurations
      </h3>
    </div>
  </svelte:fragment>

  <ConfigurationView />
</Modal>
