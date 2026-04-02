<!-- src/lib/components/projectview/modals/TranslateDocumentModal.svelte -->
<script>
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { CheckCircle, XCircle, Clock, Loader, AlertTriangle, Languages, X } from '@lucide/svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { languageMap } from '$lib/constants/languageMap.js';
  import {
    transcriptStore,
    setRanTranslationInBackground,
    clearTranslationStatus
  } from '$lib/stores/transcriptStore.js';
  import { configStatus } from '$lib/stores/configStatusStore.js';
  import { basename } from '@tauri-apps/api/path';
  import { getSelectedTranslationEngine } from '$lib/services/configureActions';
  import { Modal, Button, Label, Select, Input } from 'flowbite-svelte';
  import { Settings2 } from '@lucide/svelte';
  import ManageModelsModal from './ManageModelsModal.svelte';

  export let activeDocumentPath = null;
  export let showModal = false;

  const dispatch = createEventDispatcher();

  let localModels = [];
  let filteredModels = [];
  let modelOptions = [];
  let selectedModel = '';
  let selectedEngine = 'helsinki';
  let documentName = '';
  let selectedSourceLanguage = 'auto';
  let selectedTargetLanguage = 'en';

  function formatDuration(seconds) {
    if (!seconds && seconds !== 0) return '0s';
    const m = Math.floor(seconds / 60);
    const s = Math.floor(seconds % 60);
    if (m === 0) return `${s}s`;
    return `${m}m ${s}s`;
  }

  let showManageModelsModal = false;
  let elapsedText = '';
  let timerInterval;

  function updateElapsed() {
    if ($transcriptStore.isTranslating && $transcriptStore.translationStartTime) {
      const now = Date.now();
      const diff = Math.floor((now - $transcriptStore.translationStartTime) / 1000);
      elapsedText = formatDuration(diff);
    } else {
      elapsedText = '';
    }
  }

  $: if ($transcriptStore.isTranslating && $transcriptStore.translationStartTime) {
    if (!timerInterval) {
      updateElapsed();
      timerInterval = setInterval(updateElapsed, 1000);
    }
  } else {
    if (timerInterval) {
      clearInterval(timerInterval);
      timerInterval = null;
    }
  }

  onDestroy(() => {
    if (timerInterval) clearInterval(timerInterval);
  });

  // Mapping for NLLB language options (based on supported list)
  const nllbLanguageOptions = [
    { value: 'auto', name: 'Auto-detect' },
    ...Array.from(languageMap.entries())
      .filter(([code, name]) => code.length === 2) // Keep standard 2-letter codes
      .map(([code, name]) => ({ value: code, name: name }))
      .sort((a, b) => a.name.localeCompare(b.name))
  ];

  // Filtered list for target (remove auto-detect)
  const nllbTargetLanguageOptions = nllbLanguageOptions.filter((opt) => opt.value !== 'auto');

  // Warning icon logic (same as ProjectView)
  $: hasCriticalConfigIssues = !$configStatus.python_libraries_installed;
  $: hasNonCriticalConfigIssues =
    !hasCriticalConfigIssues &&
    (!$configStatus.hf_token_present ||
      !$configStatus.transcription_models_downloaded ||
      !$configStatus.diarization_model_downloaded ||
      !$configStatus.translation_models_downloaded);

  function handleOpenConfig() {
    dispatch('openConfig');
  }

  function formatModelDisplayName(modelName) {
    const parts = modelName.split('/');
    const baseName = parts[parts.length - 1] || modelName;

    if (baseName.toLowerCase().includes('nllb')) {
      if (baseName.includes('600M')) return 'NLLB-200 Distilled (Small & Fast)';
      if (baseName.includes('1.3B')) return 'NLLB-200 (Medium)';
      if (baseName.includes('3.3B')) return 'NLLB-200 (Large)';
      return baseName;
    }

    if (parts.length === 2) {
      const langParts = parts[1].split('-');
      if (langParts.length >= 3 && langParts[0] === 'opus' && langParts[1] === 'mt') {
        const fromCode = langParts[langParts.length - 2];
        const toCode = langParts[langParts.length - 1];
        const fromLang = languageMap.get(fromCode);
        const toLang = languageMap.get(toCode);
        if (fromLang && toLang) {
          return `${fromLang} to ${toLang}`;
        }
      }
    }
    return modelName;
  }

  $: if (activeDocumentPath) {
    basename(activeDocumentPath).then((name) => {
      documentName = name;
    });
  }

  $: {
    filteredModels = localModels.filter((m) => m.family === selectedEngine);
    if (filteredModels.length > 0) {
      modelOptions = filteredModels.map((model) => ({
        value: model.name,
        name: formatModelDisplayName(model.name)
      }));
      if (!selectedModel || !filteredModels.some((m) => m.name === selectedModel)) {
        selectedModel = filteredModels[0]?.name || '';
      }
    } else {
      modelOptions = [];
      selectedModel = '';
    }
  }

  $: if (showModal) {
    loadData();
  }

  async function loadData() {
    try {
      const [models, engine] = await Promise.all([
        invoke('get_local_translation_models'),
        getSelectedTranslationEngine()
      ]);
      localModels = models;
      selectedEngine = engine || 'helsinki';

      // We don't easily have doc language here yet, so default to auto
      selectedSourceLanguage = 'auto';
    } catch (e) {
      console.error('Failed to fetch local translation data:', e);
    }
  }

  onMount(async () => {
    await loadData();
  });

  function handleConfirm() {
    let sourceLang = 'auto';
    let targetLang = 'auto';

    if (selectedModel.toLowerCase().includes('nllb')) {
      sourceLang = selectedSourceLanguage;
      targetLang = selectedTargetLanguage;
    } else if (selectedModel.includes('-')) {
      const parts = selectedModel.split('-');
      targetLang = parts[parts.length - 1];
      sourceLang = parts[parts.length - 2] || 'auto';
    }

    dispatch('confirm', {
      documentPath: activeDocumentPath,
      model: selectedModel,
      sourceLanguage: sourceLang,
      targetLanguage: targetLang
    });
  }

  function handleCancelRequest() {
    dispatch('cancelRequest');
  }

  function handleCloseAndReset() {
    if (!isTranslating || (jobStatus !== 'running' && jobStatus !== 'initiating')) {
      clearTranslationStatus(); // Reset translation-related states
    }
    dispatch('closeAndReset');
  }

  function handleRunInBackgroundAndClose() {
    setRanTranslationInBackground(true);
    dispatch('runInBackgroundAndClose');
  }

  $: isTranslating = $transcriptStore.isTranslating;
  $: jobStatus = $transcriptStore.translationJobStatus;
  $: progressMessage = $transcriptStore.translationProgress.message;
  $: currentErrorMessage = $transcriptStore.translationErrorMessage;
  $: translationOutputFileName = $transcriptStore.translationOutputFileName;

  let durationText = '';

  $: modalTitleText =
    !isTranslating && jobStatus === null
      ? 'Translate Document'
      : isTranslating && jobStatus === 'initiating'
        ? 'Initiating Translation'
        : isTranslating && jobStatus === 'running'
          ? 'Translation Status'
          : jobStatus === 'cancelling'
            ? `Cancelling Job`
            : !isTranslating && jobStatus === 'done'
              ? 'Translation Complete'
              : !isTranslating && jobStatus === 'error'
                ? 'Translation Error'
                : !isTranslating && jobStatus === 'cancelled'
                  ? 'Translation Cancelled'
                  : 'Translation Status';

  // Watch for completion to calculate duration
  $: if (!isTranslating && jobStatus === 'done') {
    const endTime = Date.now();
    const startTime = $transcriptStore.translationStartTime;
    const durationMs = startTime ? endTime - startTime : 0;
    const seconds = Math.floor(durationMs / 1000);
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;

    if (minutes > 0) {
      durationText = `${minutes}m ${remainingSeconds}s`;
    } else {
      durationText = `${seconds}s`;
    }
  }

  let translationFileName = '';
  $: if ($transcriptStore.translationSourcePath) {
    basename($transcriptStore.translationSourcePath).then((res) => {
      translationFileName = res;
    });
  } else {
    translationFileName = '';
  }
</script>

<Modal
  bind:open={showModal}
  size="md"
  autoclose={false}
  outsideclose={!(isTranslating && (jobStatus === 'running' || jobStatus === 'initiating'))}
  class="w-full"
  backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
  dialogClass="fixed top-0 start-0 end-0 h-modal md:inset-0 md:h-full z-[10001] flex"
  bodyClass="p-0 overflow-hidden bg-white dark:bg-gray-900"
  headerClass="px-6 py-4 flex items-center justify-between border-b dark:border-gray-700 bg-gray-50/50"
  footerClass="px-6 py-4 flex items-center justify-end space-x-3 rtl:space-x-reverse border-t dark:border-gray-700 bg-gray-50/80 backdrop-blur"
  on:close={handleCloseAndReset}
>
  <div slot="header" class="flex items-center gap-2">
    <div class="p-1.5 bg-blue-50 dark:bg-blue-900/30 rounded-lg">
      {#if !isTranslating && jobStatus === null}
        <Languages size={18} class="text-blue-600 dark:text-blue-400" />
      {:else if isTranslating}
        <Languages size={18} class="text-blue-600 dark:text-blue-400" />
      {:else if jobStatus === 'done'}
        <CheckCircle size={18} class="text-green-600 dark:text-green-400" />
      {:else if jobStatus === 'error'}
        <XCircle size={18} class="text-red-600 dark:text-red-400" />
      {:else}
        <Clock size={18} class="text-orange-600 dark:text-orange-400" />
      {/if}
    </div>
    <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
      {modalTitleText}
    </h3>
  </div>

  <div class="p-6 overflow-y-auto max-h-[70vh] custom-scrollbar">
    {#if !isTranslating && jobStatus === null}
      <!-- CONFIRM VIEW -->
      <div class="space-y-5">
        <div class="space-y-2">
          <Label>Document</Label>
          <Input
            type="text"
            disabled
            value={documentName}
            class="cursor-not-allowed bg-gray-50 dark:bg-gray-800 font-mono text-xs"
            autocomplete="off"
            autocorrect="off"
          />
        </div>

        <div class="space-y-2">
          {#if modelOptions.length === 0}
            <div
              class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 p-4 rounded-xl text-center space-y-3"
            >
              <p class="text-blue-800 dark:text-blue-300 font-medium text-sm">
                No {selectedEngine === 'helsinki' ? 'Helsinki-NLP' : 'NLLB'} models available
              </p>
              <Button
                color="alternative"
                size="xs"
                on:click={() => (showManageModelsModal = true)}
                title="Manage Models"
              >
                <Settings2 size={14} class="mr-2 text-blue-500" />
                Manage Models
              </Button>
            </div>
          {:else}
            <Label for="modelSelect">Translation Model</Label>
            <Select
              id="modelSelect"
              items={modelOptions}
              bind:value={selectedModel}
              placeholder="Select a Model"
            />
          {/if}
        </div>

        {#if selectedModel.toLowerCase().includes('nllb')}
          <div
            class="grid grid-cols-2 gap-4 p-4 bg-gray-50 dark:bg-gray-800/40 rounded-xl border border-gray-100 dark:border-gray-800"
          >
            <div class="space-y-2">
              <Label>Source Language</Label>
              <Select items={nllbLanguageOptions} bind:value={selectedSourceLanguage} />
            </div>
            <div class="space-y-2">
              <Label>Target Language</Label>
              <Select items={nllbTargetLanguageOptions} bind:value={selectedTargetLanguage} />
            </div>
          </div>
        {/if}
      </div>
    {:else if isTranslating && (jobStatus === 'running' || jobStatus === 'initiating')}
      <!-- RUNNING OR INITIATING VIEW -->
      <div class="flex flex-col items-center py-8 space-y-6 text-center">
        <div class="relative">
          <div
            class="w-20 h-20 bg-blue-50 dark:bg-blue-900/20 rounded-full flex items-center justify-center"
          >
            <Loader size={40} class="text-blue-600 dark:text-blue-400 animate-spin" />
          </div>
        </div>

        <div class="space-y-2 w-full px-4">
          <p class="text-lg font-bold text-gray-900 dark:text-white">
            {jobStatus === 'initiating' ? 'Preparing Job...' : 'Translating...'}
          </p>
          {#if translationFileName}
            <p
              class="text-sm font-medium text-blue-600 dark:text-blue-400 truncate max-w-xs mx-auto"
              title={translationFileName}
            >
              {translationFileName}
            </p>
          {/if}
          <p class="text-sm text-gray-500 dark:text-gray-400 h-10 flex items-center justify-center">
            {progressMessage || 'Processing translation segments...'}
          </p>
        </div>

        {#if elapsedText}
          <div
            class="bg-gray-100 dark:bg-gray-800 px-4 py-2 rounded-full font-mono text-xs text-gray-600 dark:text-gray-400"
          >
            Elapsed: {elapsedText}
          </div>
        {/if}
      </div>
    {:else if jobStatus === 'cancelling'}
      <div class="flex flex-col items-center py-12 space-y-4 text-center">
        <Clock size={48} class="text-orange-500 animate-pulse" />
        <div class="space-y-1">
          <p class="text-lg font-bold text-gray-900 dark:text-white">Stopping Translation</p>
          <p class="text-sm text-gray-500 dark:text-gray-400">
            Attempting to gracefully cancel the process...
          </p>
        </div>
      </div>
    {:else if !isTranslating && jobStatus === 'done'}
      <div class="flex flex-col items-center py-8 space-y-4 text-center">
        <div
          class="w-20 h-20 bg-green-50 dark:bg-green-900/20 rounded-full flex items-center justify-center"
        >
          <CheckCircle size={40} class="text-green-600 dark:text-green-400" />
        </div>
        <div class="space-y-1">
          <p class="text-lg font-bold text-gray-900 dark:text-white">Translation Complete!</p>
          {#if translationOutputFileName}
            <div class="flex flex-col items-center">
              <p
                class="text-xs font-medium text-green-600 dark:text-green-400 mt-2 bg-green-50/50 dark:bg-green-900/10 px-3 py-1 rounded-full border border-green-100/50 dark:border-green-800/20 max-w-[280px] truncate"
                title={translationOutputFileName}
              >
                Output: {translationOutputFileName}
              </p>
            </div>
          {/if}
          {#if durationText}
            <p class="text-sm text-gray-500 dark:text-gray-400">
              Total processing time: {durationText}
            </p>
          {/if}
        </div>
      </div>
    {:else if !isTranslating && jobStatus === 'cancelled'}
      <div class="flex flex-col items-center py-8 space-y-4 text-center">
        <div
          class="w-20 h-20 bg-orange-50 dark:bg-orange-900/20 rounded-full flex items-center justify-center"
        >
          <XCircle size={40} class="text-orange-600 dark:text-orange-400" />
        </div>
        <div class="space-y-1">
          <p class="text-lg font-bold text-gray-900 dark:text-white">Translation Cancelled</p>
          <p class="text-sm text-gray-500 dark:text-gray-400">
            {progressMessage || 'The job was stopped by user.'}
          </p>
        </div>
      </div>
    {:else if !isTranslating && jobStatus === 'error'}
      <div class="flex flex-col items-center py-6 space-y-4">
        <div
          class="w-16 h-16 bg-red-50 dark:bg-red-900/20 rounded-full flex items-center justify-center"
        >
          <XCircle size={32} class="text-red-600 dark:text-red-400" />
        </div>
        <p class="text-lg font-bold text-gray-900 dark:text-white">An Error Occurred</p>
        <div
          class="bg-red-50 dark:bg-red-900/20 border border-red-100 dark:border-red-800 p-4 rounded-xl w-full"
        >
          <p
            class="text-xs font-mono text-red-700 dark:text-red-300 break-words whitespace-pre-wrap max-h-40 overflow-y-auto"
          >
            {currentErrorMessage || 'Unknown error during translation.'}
          </p>
        </div>
      </div>
    {:else}
      <div class="flex flex-col items-center py-12 space-y-4">
        <Loader size={32} class="text-gray-400 animate-spin" />
        <p class="text-sm text-gray-500">Checking status...</p>
      </div>
    {/if}
  </div>

  <svelte:fragment slot="footer">
    {#if !isTranslating && jobStatus === null}
      <div class="flex justify-between w-full">
        <button
          on:click={() => (showManageModelsModal = true)}
          class="text-sm text-blue-600 dark:text-blue-400 hover:underline flex items-center gap-1.5"
          title="Manage Translation Models"
        >
          <Settings2 size={14} />
          Manage Models
        </button>
        <div class="flex gap-3">
          <Button color="alternative" on:click={handleCloseAndReset} title="Cancel">Cancel</Button>
          <Button
            color="blue"
            on:click={handleConfirm}
            title="Start Translation"
            disabled={!activeDocumentPath || !selectedModel}
          >
            Start Translation
          </Button>
        </div>
      </div>
    {:else if isTranslating && (jobStatus === 'running' || jobStatus === 'initiating')}
      <Button
        color="alternative"
        on:click={handleRunInBackgroundAndClose}
        disabled={jobStatus === 'initiating'}
        title="Run in Background"
      >
        Run in Background
      </Button>
      <Button
        color="red"
        on:click={handleCancelRequest}
        disabled={jobStatus === 'initiating'}
        title="Stop Translation"
      >
        Stop
      </Button>
    {:else if jobStatus === 'cancelling'}
      <Button color="alternative" disabled title="Stopping...">Stopping...</Button>
    {:else}
      <Button color="blue" on:click={handleCloseAndReset} title="Close">Close</Button>
    {/if}
  </svelte:fragment>
</Modal>

<ManageModelsModal
  bind:showModal={showManageModelsModal}
  on:modelsChanged={async () => {
    // Refresh local translation configuration
    selectedEngine = (await getSelectedTranslationEngine()) || 'helsinki';
    await loadData();
  }}
/>

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
