<!-- src/lib/components/projectview/modals/ImportTranscriptSourceModal.svelte -->
<script>
  import { createEventDispatcher } from 'svelte';
  import { Modal, Button, Label, Helper, Alert } from 'flowbite-svelte';
  import { FilePlus2, X, Info, FileText, CheckCircle2 } from '@lucide/svelte';

  export let showModal = false;

  const dispatch = createEventDispatcher();

  let selectedSourceType = 'msWord'; // Default, only option for now

  function handleConfirm() {
    if (selectedSourceType) {
      dispatch('confirm', { sourceType: selectedSourceType });
    }
  }

  function handleClose() {
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
  bodyClass="p-0 overflow-hidden bg-white dark:bg-gray-900"
  headerClass="px-6 py-4 flex items-center justify-between border-b dark:border-gray-700 bg-gray-50/50"
  footerClass="px-6 py-4 flex items-center justify-end space-x-3 rtl:space-x-reverse border-t dark:border-gray-700 bg-gray-50/80 backdrop-blur"
  on:close={handleClose}
>
  <div slot="header" class="flex items-center gap-2">
    <FilePlus2 class="w-5 h-5 text-gray-500" />
    <h3 class="text-lg font-semibold text-gray-900 dark:text-white">Import Transcript</h3>
  </div>

  <div class="p-6 space-y-6">
    <div
      class="bg-blue-50 dark:bg-blue-900/20 border border-blue-100 dark:border-blue-800/50 p-3 rounded-lg flex gap-3"
    >
      <Info size={18} class="text-blue-600 dark:text-blue-400 shrink-0 mt-0.5" />
      <p class="text-xs text-blue-800 dark:text-blue-300 leading-relaxed">
        Select the source format of the transcript you want to import.
      </p>
    </div>

    <div class="space-y-4">
      <Label class="text-xs font-bold uppercase tracking-wider text-gray-500">Source Format</Label>

      <div class="space-y-3">
        <!-- MS Word Option -->
        <div
          class="flex p-4 border rounded-xl transition-all cursor-pointer {selectedSourceType ===
          'msWord'
            ? 'border-blue-500 bg-blue-50/30 dark:bg-blue-900/10'
            : 'border-gray-100 dark:border-gray-800 hover:border-gray-200 dark:hover:border-gray-700'}"
          on:click={() => (selectedSourceType = 'msWord')}
        >
          <div class="flex items-center h-5">
            <input
              id="source-word"
              name="sourceType"
              type="radio"
              bind:group={selectedSourceType}
              value="msWord"
              class="w-4 h-4 text-blue-600 bg-white border-gray-300 focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600 cursor-pointer"
            />
          </div>
          <div class="ms-3 text-sm select-none flex-1">
            <div class="flex items-center gap-2 mb-1">
              <FileText size={14} class="text-blue-600 dark:text-blue-400" />
              <label
                for="source-word"
                class="font-bold text-gray-900 dark:text-white cursor-pointer"
                >MS Word Document (.docx)</label
              >
            </div>
            <p class="text-xs font-normal text-gray-500 dark:text-gray-400 leading-relaxed">
              Only supports transcripts with speaker names and timestamps. Ensure the formatting
              follows the template.
            </p>
          </div>
          {#if selectedSourceType === 'msWord'}
            <CheckCircle2 size={16} class="text-blue-600 dark:text-blue-400 mt-0.5" />
          {/if}
        </div>

        <!-- Placeholder for future options -->
        <div
          class="flex p-4 border border-gray-100 dark:border-gray-800 rounded-xl opacity-40 cursor-not-allowed grayscale"
        >
          <div class="flex items-center h-5">
            <input type="radio" disabled class="w-4 h-4 border-gray-200" />
          </div>
          <div class="ms-3 text-sm flex-1">
            <label class="font-bold text-gray-400">Other Formats</label>
            <p class="text-xs font-normal text-gray-400">Coming soon (SRT, VTT, etc.)</p>
          </div>
        </div>
      </div>
    </div>
  </div>

  <svelte:fragment slot="footer">
    <Button color="alternative" on:click={handleClose} title="Cancel and close">Cancel</Button>
    <Button
      color="blue"
      on:click={handleConfirm}
      disabled={!selectedSourceType}
      title="Start the import process"
    >
      Proceed
    </Button>
  </svelte:fragment>
</Modal>

<style lang="postcss">
</style>
