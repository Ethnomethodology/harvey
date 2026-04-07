<!-- src/lib/components/projectview/modals/AdditionalParametersModal.svelte -->
<script>
  import { createEventDispatcher } from 'svelte';
  import { X, SlidersHorizontal } from '@lucide/svelte';
  import { Modal, Button, Label, Textarea, Helper } from 'flowbite-svelte';

  export let showModal = false;
  export let currentEngine = 'whisper-cpp';
  export let initialPrompt = '';
  export let hotwords = '';

  const dispatch = createEventDispatcher();

  let localInitialPrompt = '';
  let localHotwords = '';

  $: if (showModal) {
    localInitialPrompt = initialPrompt;
    localHotwords = hotwords;
  }

  function handleConfirm() {
    dispatch('confirm', {
      initialPrompt: localInitialPrompt,
      hotwords: localHotwords
    });
    showModal = false;
  }

  function handleClose() {
    showModal = false;
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
  bodyClass="p-6 space-y-5 bg-white dark:bg-gray-900"
  headerClass="px-6 py-4 flex items-center justify-between border-b dark:border-gray-700 bg-gray-50/50"
  footerClass="px-6 py-4 flex items-center justify-end space-x-3 rtl:space-x-reverse border-t dark:border-gray-700 bg-gray-50/80 backdrop-blur"
  on:close={handleClose}
>
  <div slot="header" class="flex items-center gap-2">
    <SlidersHorizontal class="w-5 h-5 text-gray-500" />
    <h3 class="text-lg font-semibold text-gray-900 dark:text-white">Additional Parameters</h3>
  </div>

  <div class="space-y-5 overflow-y-auto max-h-[60vh] custom-scrollbar">
    {#if currentEngine === 'whisper-cpp'}
      <div class="space-y-2">
        <Label for="initialPromptInput">Prompt</Label>
        <Textarea
          id="initialPromptInput"
          bind:value={localInitialPrompt}
          rows="3"
          placeholder="e.g. Welcome to the AI podcast."
          autocomplete="off"
          autocorrect="off"
        />
        <Helper class="italic">
          Guides the model's style and vocabulary. Provide a sample sentence demonstrating desired
          formatting or technical terms.
        </Helper>
      </div>
    {:else if currentEngine === 'faster-whisper'}
      <div class="space-y-2">
        <Label for="initialPromptInput">Initial Prompt (Context & Formatting)</Label>
        <Textarea
          id="initialPromptInput"
          bind:value={localInitialPrompt}
          rows="2"
          placeholder="e.g. Hello. This is Dr. Smith from Acme Corp."
        />
        <Helper class="italic">
          Injects text into the model's context window. Best for instructing capitalization,
          punctuation, and general style.
        </Helper>
      </div>

      <div class="space-y-2 pt-4 border-t border-gray-100 dark:border-gray-800">
        <Label for="hotwordsInput">Hotwords (Jargon & Names)</Label>
        <Textarea
          id="hotwordsInput"
          bind:value={localHotwords}
          rows="2"
          placeholder="e.g. dysdiadochokinesia, Smith, Acme Corp"
        />
        <Helper class="italic">
          Comma-separated list. Explicitly boosts the probability of specific words during decoding
          to prevent misspellings.
        </Helper>
      </div>
    {/if}
  </div>

  <svelte:fragment slot="footer">
    <Button color="alternative" on:click={handleClose} title="Cancel">Cancel</Button>
    <Button color="blue" on:click={handleConfirm} title="Save Parameters">Save</Button>
  </svelte:fragment>
</Modal>

<style lang="postcss">
  .custom-scrollbar::-webkit-scrollbar {
    width: 6px;
  }
  .custom-scrollbar::-webkit-scrollbar-track {
    @apply bg-transparent;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    @apply bg-gray-200 dark:bg-gray-700 rounded-full;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb:hover {
    @apply bg-gray-300 dark:bg-gray-600;
  }
</style>
