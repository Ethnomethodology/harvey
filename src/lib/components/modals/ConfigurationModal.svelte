<!-- src/lib/components/modals/ConfigurationModal.svelte -->
<script>
  import { createEventDispatcher } from 'svelte';
  import ConfigurationView from '$lib/components/shared/ConfigurationView.svelte';

  export let showModal = false;

  const dispatch = createEventDispatcher();

  function close() {
    dispatch('close');
  }

  function handleKeydown(event) {
    if (event.key === 'Escape') {
      close();
    }
  }
</script>

{#if showModal}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-gray-900 bg-opacity-60 backdrop-blur-sm"
    on:click|self={close}
    on:keydown={handleKeydown}
    role="dialog"
    aria-modal="true"
    aria-labelledby="configuration-modal-title"
    tabindex="-1"
  >
    <div class="bg-white p-6 rounded-lg shadow-xl w-full max-w-3xl m-4 flex flex-col" style="height: 90vh;" role="document">
      <div class="flex-shrink-0 flex justify-between items-center pb-4 border-b border-gray-200">
        <h2 id="configuration-modal-title" class="text-lg font-semibold text-gray-800">Configurations</h2>
        <button on:click={close} aria-label="Close" class="text-gray-400 hover:text-gray-600 focus:outline-none focus:ring-2 focus:ring-gray-400 rounded-full p-1">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" /></svg>
        </button>
      </div>
      <div class="flex-grow overflow-y-auto my-4 pr-2 -mr-2">
        <ConfigurationView />
      </div>
      <div class="flex-shrink-0 flex justify-end space-x-3 pt-4 border-t border-gray-200">
        <button
            type="button"
            on:click={close}
            class="px-4 py-2 bg-gray-200 text-gray-800 rounded-md shadow-sm hover:bg-gray-300 focus:outline-none focus:ring-2 focus:ring-gray-400 focus:ring-opacity-75 transition duration-150 ease-in-out text-sm font-medium"
        >
            Close
        </button>
      </div>
    </div>
  </div>
{/if}
