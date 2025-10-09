<!-- src/lib/components/shared/WhisperModelPanel.svelte -->
<script>
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import InstallLogModal from '../modals/InstallLogModal.svelte';

  let isPanelOpen = false;
  let isModelInstalled = false;
  let isLoading = true;
  let isInstalling = false;
  let error = '';
  let showInstallModal = false;
  let installLogs = [];
  let unlisten;

  async function checkStatus() {
    isLoading = true;
    error = '';
    try {
      isModelInstalled = await invoke('check_whisper_model_installed');
    } catch (e) {
      console.error('Error checking Whisper model status:', e);
      error = `Failed to check model status: ${e.message || e}`;
      isModelInstalled = false;
    } finally {
      isLoading = false;
    }
  }

  async function handleInstall() {
    showInstallModal = true;
    isInstalling = true;
    installLogs = [];
    error = '';

    try {
      unlisten = await listen('whisper-installation-log', (event) => {
        installLogs = [...installLogs, event.payload.message];
      });

      await invoke('download_whisper_model');
      await checkStatus();
    } catch (e) {
      console.error('Error installing Whisper model:', e);
      error = `Failed to install model: ${e.message || e}`;
      installLogs = [...installLogs, `Error: ${e.message || e}`];
    } finally {
      isInstalling = false;
      if (unlisten) {
        unlisten();
      }
    }
  }

  onMount(checkStatus);

  onDestroy(() => {
    if (unlisten) {
      unlisten();
    }
  });
</script>

<div class="border-y border-gray-200">
  <button
    on:click={() => (isPanelOpen = !isPanelOpen)}
    class="w-full flex justify-between items-center py-3 text-left focus:outline-none"
  >
    <div class="flex items-center">
      <h3 class="block text-sm font-medium text-gray-700">Whisper Model</h3>
    </div>
    <div class="flex items-center">
      {#if isLoading}
        <span class="text-xs text-gray-500 mr-2">Checking...</span>
      {:else if isModelInstalled}
        <span class="text-sm font-medium text-green-600">Installed</span>
      {:else}
        <span class="text-sm font-medium text-red-600">Required</span>
      {/if}
      <svg
        class="w-6 h-6 transform transition-transform duration-200 ease-in-out {isPanelOpen ? 'rotate-180' : ''}"
        xmlns="http://www.w3.org/2000/svg"
        fill="none"
        viewBox="0 0 24 24"
        stroke-width="2"
        stroke="currentColor"
      >
        <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
      </svg>
    </div>
  </button>
</div>

{#if isPanelOpen}
  <div class="p-4 bg-gray-50 border-b border-gray-200 text-sm">
    <p class="mb-4">
      Harvey uses a Whisper model for audio transcription. The model will be downloaded and stored locally.
    </p>

    {#if isModelInstalled}
      <p class="text-green-600 mb-2">Whisper model is installed.</p>
    {:else}
      <div class="flex items-center">
        <p class="text-red-600 mr-4">Required model is not installed.</p>
        <button class="btn-blue-small" on:click={handleInstall} disabled={isInstalling}>
          {isInstalling ? 'Installing...' : 'Install'}
        </button>
      </div>
    {/if}

    {#if error && !showInstallModal}
      <p class="text-red-600 mt-4">{error}</p>
    {/if}
  </div>
{/if}

<InstallLogModal bind:showModal={showInstallModal} logs={installLogs} isInstalling={isInstalling} title="Whisper Model Installation" inProgressText="Installation in progress..." buttonInProgressText="Installing..." />

<style lang="postcss">
  .btn-blue-small {
    @apply px-2.5 py-1 text-xs border font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-1 transition-colors duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed;
    @apply border-transparent text-white bg-blue-600 hover:bg-blue-700 focus:ring-blue-500;
  }
</style>
