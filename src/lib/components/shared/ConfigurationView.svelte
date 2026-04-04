<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { type } from '@tauri-apps/plugin-os';
  import { listen } from '@tauri-apps/api/event';
  import { open } from '@tauri-apps/plugin-dialog';
  import { ask } from '@tauri-apps/plugin-dialog';
  import Dropdown from '$lib/components/shared/Dropdown.svelte';
  import { themePreference } from '$lib/stores/themeStore.js';
  import {
    saveDownloadLocation,
    getDownloadedModels,
    getAllDownloadedModels,
    getDownloadLocation,
    moveModelsAndUpdateLocation
  } from '$lib/services/configureActions';
  import { Input, Label, Button, Select, Accordion, AccordionItem } from 'flowbite-svelte';
  import {
    FolderOpen,
    ExternalLink,
    Settings2,
    MonitorCog,
    MessageSquareText,
    Users,
    Languages,
    SlidersHorizontal,
    TriangleAlert,
    ChevronDown,
    ChevronRight
  } from '@lucide/svelte';

  import TranscriptionConfiguration from './TranscriptionConfiguration.svelte';
  import TranslationConfiguration from './TranslationConfiguration.svelte';
  import DiarizationModelPanel from './DiarizationModelPanel.svelte';
  import AdvancedConfiguration from './AdvancedConfiguration.svelte';
  import LibrariesPanel from './LibrariesPanel.svelte';
  import HuggingFacePanel from './HuggingFacePanel.svelte';
  import { configStatus, updateConfigStatus } from '$lib/stores/configStatusStore.js';

  let activeTab = $state('application'); // 'application', 'transcription', 'diarization', 'translation', 'advanced'
  let isWinArm64 = false;
  let isFFmpegInstalled = false;
  let downloadLocation = $state('');
  let logsDirPath = $state('');
  let isLoadingConfig = $state(true);
  let configError = $state('');
  let isMovingModels = $state(false);
  let statusMessage = $state('');
  let isGeneralOpen = $state(false);
  let revealLabel = $state('Reveal in File Explorer');

  let isTranscriptionBusy = $state(false);
  let isTranslationBusy = $state(false);
  let isAdvancedBusy = $state(false);
  let translationModelCount = $state(0);
  let isBusy = $derived(isMovingModels || isTranscriptionBusy || isTranslationBusy || isAdvancedBusy);

  onMount(async () => {
    updateConfigStatus(true); // Force a refresh when the component mounts
    isLoadingConfig = true;
    configError = '';
    statusMessage = '';

    const osType = type();
    if (osType === 'macos') {
      revealLabel = 'Reveal in Finder';
    } else {
      revealLabel = 'Reveal in File Explorer';
    }

    try {
      [downloadLocation, logsDirPath] = await Promise.all([
        getDownloadLocation(),
        invoke('get_logs_dir_path')
      ]);
    } catch (e) {
      console.error('Error loading configuration:', e);
      configError = `Failed to load configuration: ${e.message || e}`;
    } finally {
      isLoadingConfig = false;
    }
  });

  async function pickDownloadLocation() {
    if (isBusy) return;
    try {
      const selected = await open({
        multiple: false,
        directory: true,
        title: 'Select Model Download Location',
        defaultPath: downloadLocation || undefined
      });
      if (!selected || typeof selected !== 'string') return;

      const newLocation = selected;
      if (newLocation === downloadLocation) {
        statusMessage = 'Selected location is the same as current.';
        setTimeout(() => (statusMessage = ''), 3000);
        return;
      }

      const currentModels = await getAllDownloadedModels();
      const modelsToMove = currentModels.length > 0 && downloadLocation;

      let confirmed = true;
      if (modelsToMove) {
        confirmed = await ask(
          `Change download location to:\n${newLocation}\n\nThis will move ${currentModels.length} downloaded model(s) from the current location to the new one. Proceed?`,
          {
            title: 'Confirm Location Change & Move',
            type: 'warning',
            okLabel: 'Yes, Move Files',
            cancelLabel: 'Cancel'
          }
        );
      } else {
        confirmed = await ask(
          `Set download location to:\n${newLocation}\n\nNew models will be downloaded here.`,
          {
            title: 'Confirm Location Change',
            type: 'info',
            okLabel: 'Confirm',
            cancelLabel: 'Cancel'
          }
        );
      }

      if (!confirmed) {
        statusMessage = 'Location change cancelled.';
        setTimeout(() => (statusMessage = ''), 3000);
        return;
      }

      isMovingModels = modelsToMove;
      statusMessage = modelsToMove
        ? 'Moving models and updating location...'
        : 'Updating location...';
      configError = '';

      try {
        // TODO: This function in the backend needs to be aware of both model types.
        if (modelsToMove) {
          await moveModelsAndUpdateLocation(newLocation);
          statusMessage = 'Download location updated and models moved successfully!';
        } else {
          await saveDownloadLocation(newLocation);
          statusMessage = 'Download location updated successfully!';
        }
        downloadLocation = newLocation;
      } catch (err) {
        configError = `Error changing location: ${err.message || err}`;
        statusMessage = '';
      } finally {
        isMovingModels = false;
        setTimeout(() => {
          if (!configError) {
            statusMessage = '';
          }
        }, 5000);
      }
    } catch (err) {
      configError = `Error selecting directory: ${err.message || err}`;
      isMovingModels = false;
    }
  }
</script>

<div class="flex flex-col h-full bg-white dark:bg-gray-900 text-gray-800 dark:text-gray-200">
  <!-- Tab Navigation -->
  <div
    class="border-b border-gray-200 dark:border-gray-700 flex-shrink-0 bg-gray-50/50 dark:bg-gray-800/50"
  >
    <ul
      class="flex flex-wrap -mb-px text-sm font-medium text-center text-gray-500 dark:text-gray-400"
    >
      <li class="me-2">
        <button
          type="button"
          onclick={() => (activeTab = 'application')}
          class="inline-flex items-center justify-center p-4 border-b-2 rounded-t-lg group transition-all {activeTab ===
          'application'
            ? 'text-blue-600 border-blue-600 active dark:text-blue-500 dark:border-blue-500'
            : 'border-transparent hover:text-gray-600 hover:border-gray-300 dark:hover:text-gray-300'}"
        >
          <MonitorCog
            size={18}
            class="me-2 {activeTab === 'application'
              ? 'text-blue-600 dark:text-blue-500'
              : 'text-gray-400 group-hover:text-gray-500 dark:text-gray-500 dark:group-hover:text-gray-300'}"
          />
          Application
          {#if !$configStatus.python_libraries_installed || !$configStatus.hf_token_present}
            <TriangleAlert size={14} class="ms-2 text-red-500" />
          {/if}
        </button>
      </li>
      <li class="me-2">
        <button
          type="button"
          onclick={() => (activeTab = 'transcription')}
          class="inline-flex items-center justify-center p-4 border-b-2 rounded-t-lg group transition-all {activeTab ===
          'transcription'
            ? 'text-blue-600 border-blue-600 active dark:text-blue-500 dark:border-blue-500'
            : 'border-transparent hover:text-gray-600 hover:border-gray-300 dark:hover:text-gray-300'}"
        >
          <MessageSquareText
            size={18}
            class="me-2 {activeTab === 'transcription'
              ? 'text-blue-600 dark:text-blue-500'
              : 'text-gray-400 group-hover:text-gray-500 dark:text-gray-500 dark:group-hover:text-gray-300'}"
          />
          Transcription
          {#if !$configStatus.python_libraries_installed || ($configStatus.selected_transcription_engine === 'whisper-cpp' && !$configStatus.whisper_cpp_installed) || ($configStatus.selected_transcription_engine === 'faster-whisper' && !$configStatus.faster_whisper_dependencies_installed)}
            <TriangleAlert size={14} class="ms-2 text-red-500" title="Missing required libraries" />
          {:else if ($configStatus.selected_transcription_engine === 'whisper-cpp' && !$configStatus.whisper_cpp_models_downloaded) || ($configStatus.selected_transcription_engine === 'faster-whisper' && !$configStatus.faster_whisper_models_downloaded)}
            <TriangleAlert size={14} class="ms-2 text-yellow-500" title="No models downloaded" />
          {/if}
        </button>
      </li>
      <li class="me-2">
        <button
          type="button"
          onclick={() => (activeTab = 'diarization')}
          class="inline-flex items-center justify-center p-4 border-b-2 rounded-t-lg group transition-all {activeTab ===
          'diarization'
            ? 'text-blue-600 border-blue-600 active dark:text-blue-500 dark:border-blue-500'
            : 'border-transparent hover:text-gray-600 hover:border-gray-300 dark:hover:text-gray-300'}"
        >
          <Users
            size={18}
            class="me-2 {activeTab === 'diarization'
              ? 'text-blue-600 dark:text-blue-500'
              : 'text-gray-400 group-hover:text-gray-500 dark:text-gray-500 dark:group-hover:text-gray-300'}"
          />
          Diarization
          {#if !$configStatus.python_libraries_installed}
            <TriangleAlert
              size={14}
              class="ms-2 text-red-500"
              title="Missing required Python libraries"
            />
          {:else if !$configStatus.diarization_model_downloaded}
            <TriangleAlert size={14} class="ms-2 text-yellow-500" title="No model downloaded" />
          {/if}
        </button>
      </li>
      <li class="me-2">
        <button
          type="button"
          onclick={() => (activeTab = 'translation')}
          class="inline-flex items-center justify-center p-4 border-b-2 rounded-t-lg group transition-all {activeTab ===
          'translation'
            ? 'text-blue-600 border-blue-600 active dark:text-blue-500 dark:border-blue-500'
            : 'border-transparent hover:text-gray-600 hover:border-gray-300 dark:hover:text-gray-300'}"
        >
          <Languages
            size={18}
            class="me-2 {activeTab === 'translation'
              ? 'text-blue-600 dark:text-blue-500'
              : 'text-gray-400 group-hover:text-gray-500 dark:text-gray-500 dark:group-hover:text-gray-300'}"
          />
          Translation
          {#if !$configStatus.python_libraries_installed}
            <TriangleAlert
              size={14}
              class="ms-2 text-red-500"
              title="Missing required Python libraries"
            />
          {:else if ($configStatus.selected_translation_engine === 'helsinki' && !$configStatus.helsinki_models_downloaded) || ($configStatus.selected_translation_engine === 'nllb' && !$configStatus.nllb_models_downloaded)}
            <TriangleAlert size={14} class="ms-2 text-yellow-500" title="No models downloaded" />
          {/if}
        </button>
      </li>
      <li class="me-2">
        <button
          type="button"
          onclick={() => (activeTab = 'advanced')}
          class="inline-flex items-center justify-center p-4 border-b-2 rounded-t-lg group transition-all {activeTab ===
          'advanced'
            ? 'text-blue-600 border-blue-600 active dark:text-blue-500 dark:border-blue-500'
            : 'border-transparent hover:text-gray-600 hover:border-gray-300 dark:hover:text-gray-300'}"
        >
          <SlidersHorizontal
            size={18}
            class="me-2 {activeTab === 'advanced'
              ? 'text-blue-600 dark:text-blue-500'
              : 'text-gray-400 group-hover:text-gray-500 dark:text-gray-500 dark:group-hover:text-gray-300'}"
          />
          Advanced
        </button>
      </li>
    </ul>
  </div>

  <!-- Main Content Area -->
  <div class="flex-grow min-h-0 overflow-y-auto p-8">
    <div class="max-w-3xl mx-auto h-full">
      {#if activeTab === 'application'}
        <div class="space-y-6">
          {#if isLoadingConfig}
            <p class="text-gray-500 dark:text-gray-400 text-center py-4">
              Loading configuration...
            </p>
          {/if}
          <LibrariesPanel />

          <Accordion
            class="w-full bg-white dark:bg-gray-800/60 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden"
            flush
          >
            <AccordionItem
              bind:open={isGeneralOpen}
              defaultClass="w-full flex items-center justify-between px-6 py-4 text-left font-semibold text-gray-900 dark:text-gray-200 bg-gray-50/50 dark:bg-gray-800/30 hover:bg-gray-100 dark:hover:bg-gray-700 focus:outline-none transition-colors border-b dark:border-gray-700"
            >
              <span
                slot="header"
                class="flex items-center gap-2 text-gray-900 dark:text-gray-200 text-base"
              >
                <Settings2 size={18} class="text-gray-500" />
                General Settings
              </span>
              <div class="p-6 space-y-6">
                <div class="space-y-2">
                  <Label for="theme-select">Theme</Label>
                  <Select
                    id="theme-select"
                    class="max-w-xs"
                    items={[
                      { value: 'system', name: 'System' },
                      { value: 'light', name: 'Light' },
                      { value: 'dark', name: 'Dark' }
                    ]}
                    bind:value={$themePreference}
                  />
                </div>

                <div class="space-y-2">
                  <Label for="download-location-input">Local Model Download Location</Label>
                  <div class="flex items-center gap-2 max-w-2xl">
                    <Input
                      id="download-location-input"
                      type="text"
                      bind:value={downloadLocation}
                      class="flex-grow cursor-not-allowed bg-gray-50 dark:bg-gray-800"
                      readonly
                      placeholder="Set a location..."
                      title={downloadLocation || 'No location set'}
                      autocomplete="off"
                      autocorrect="off"
                    />
                    <Button
                      color="alternative"
                      class="px-3"
                      onclick={pickDownloadLocation}
                      disabled={isBusy}
                      title={isBusy ? 'Operation in progress...' : 'Select model download folder'}
                    >
                      {#if isMovingModels}
                        Moving...
                      {:else}
                        <FolderOpen size={18} />
                      {/if}
                    </Button>
                  </div>
                  {#if isBusy && !isMovingModels}
                    <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                      Download in progress. Cannot change location now.
                    </p>
                  {/if}
                  {#if statusMessage}
                    <p class="text-xs text-indigo-600 dark:text-indigo-400 mt-1">{statusMessage}</p>
                  {/if}
                </div>

                <div class="space-y-2">
                  <Label for="application-logs-input">Application Logs</Label>
                  <div class="flex items-center gap-2 max-w-2xl">
                    <Input
                      id="application-logs-input"
                      type="text"
                      bind:value={logsDirPath}
                      class="flex-grow cursor-not-allowed bg-gray-50 dark:bg-gray-800"
                      readonly
                      title={logsDirPath || 'No logs directory found'}
                      autocomplete="off"
                      autocorrect="off"
                    />
                    <Button
                      color="alternative"
                      class="px-3"
                      onclick={() => invoke('reveal_in_file_explorer', { filePathStr: logsDirPath })}
                      title={revealLabel}
                    >
                      <ExternalLink size={18} />
                    </Button>
                  </div>
                </div>
              </div>
            </AccordionItem>
          </Accordion>
        </div>
      {:else if activeTab === 'transcription'}
        <TranscriptionConfiguration bind:isBusy={isTranscriptionBusy} {downloadLocation} />
      {:else if activeTab === 'diarization'}
        <DiarizationModelPanel
          arePythonLibrariesInstalled={$configStatus.python_libraries_installed}
        />
      {:else if activeTab === 'translation'}
        <TranslationConfiguration
          bind:isBusy={isTranslationBusy}
          {downloadLocation}
          bind:translationModelCount
        />
      {:else if activeTab === 'advanced'}
        <div class="pb-8">
          <AdvancedConfiguration bind:isBusy={isAdvancedBusy} />
        </div>
      {/if}
    </div>
  </div>
</div>

<style lang="postcss">
  .input {
    @apply bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-md px-2.5 py-1 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 dark:text-gray-200 placeholder:text-gray-400 dark:placeholder:text-gray-500;
  }
  .input:read-only {
    @apply bg-gray-100 dark:bg-gray-700 dark:text-gray-400 cursor-not-allowed;
  }
  .btn-blue {
    @apply px-2.5 py-1.5 border text-sm font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-1 transition-colors duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed;
    @apply border-transparent text-white bg-blue-600 hover:bg-blue-700 focus:ring-blue-500;
  }

  .overflow-y-auto::-webkit-scrollbar {
    width: 6px;
  }
  .overflow-y-auto::-webkit-scrollbar-track {
    background: transparent;
  }
  .overflow-y-auto::-webkit-scrollbar-thumb {
    background-color: rgba(156, 163, 175, 0.5);
    border-radius: 10px;
    border: 2px solid transparent;
    background-clip: content-box;
  }
  .overflow-y-auto::-webkit-scrollbar-thumb:hover {
    background-color: rgba(107, 114, 128, 0.6);
  }
  .overflow-y-auto {
    scrollbar-width: thin;
    scrollbar-color: rgba(156, 163, 175, 0.5) transparent;
  }
</style>
