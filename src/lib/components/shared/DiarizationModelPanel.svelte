<!-- src/lib/components/shared/DiarizationModelPanel.svelte -->
<script>
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { open as openExternal } from '@tauri-apps/plugin-shell';
  import { configStatus, setDiarizationModelDownloaded } from '$lib/stores/configStatusStore.js';
  import InstallLogModal from '../modals/InstallLogModal.svelte';

  export let arePythonLibrariesInstalled = false;
  let hasAccess = false;
  let isLoading = true;
  let isDownloading = false;
  let error = '';
  let cachePath = '';
  let showInstallModal = false;
  let installLogs = [];
  let unlistenLog;
  let unlistenFinished;
  let isDeleting = false;

  async function handleDeleteModel() {
    isDeleting = true;
    error = '';
    try {
      await invoke('delete_diarization_model');
      await checkAccessStatus(); // Re-check status after deletion
      setDiarizationModelDownloaded(false);
    } catch (e) {
      console.error('Error deleting diarization model:', e);
      error = `Failed to delete model: ${e.message || e}`;
    } finally {
      isDeleting = false;
    }
  }

  async function checkAccessStatus() {
    isLoading = true;
    error = '';
    try {
      hasAccess = await invoke('check_diarization_model_access');
      if (hasAccess) {
        await getCachePath(); // If we have access, try to get the path
      }
    } catch (e) {
      console.error('Error checking diarization model access status:', e);
      error = `Failed to check access status: ${e.message || e}`;
      hasAccess = false;
    } finally {
      isLoading = false;
    }
  }

  async function getCachePath() {
      try {
        cachePath = await invoke('get_diarization_cache_path');
      } catch (e) {
        console.error('Error getting cache path:', e);
        // Don't show this error to the user, as it's not critical
      }
  }

  async function handleDownload() {
    showInstallModal = true;
    isDownloading = true;
    installLogs = [];
    error = '';

    try {
      unlistenLog = await listen('diarization-installation-log', (event) => {
        installLogs = [...installLogs, { id: installLogs.length, message: event.payload.message }];
      });

      await invoke('download_diarization_model');
      await checkAccessStatus(); // This will re-check access and get the cache path
    } catch (e) {
      console.error('Error downloading diarization model:', e);
      error = `Failed to download model: ${e.message || e}`;
      installLogs = [...installLogs, `Error: ${e.message || e}`];
      isDownloading = false; // Set to false on error
    } finally {
      if (unlistenLog) {
        unlistenLog();
      }
    }
  }

  onMount(async () => {
    // The checkAccessStatus is now triggered by the reactive block below

    unlistenFinished = await listen('diarization-installation-finished', async () => {
        isDownloading = false;
        await checkAccessStatus(); // Re-check status after installation attempt
        setDiarizationModelDownloaded(true);
    });
  });

  // Reactively check status based on Python library installation
  $: {
    if ($configStatus.python_libraries_installed) {
        checkAccessStatus();
    } else {
        // Reset local state if Python libraries are not installed
        hasAccess = false;
        isLoading = false;
        error = '';
        cachePath = '';
    }
  }

  onDestroy(() => {
    if (unlistenLog) {
      unlistenLog();
    }
    if (unlistenFinished) {
        unlistenFinished();
    }
  });

  function openLink(url) {
    openExternal(url).catch((err) => console.error(`Failed to open link: ${err}`));
  }
</script>

<div class="flex flex-col space-y-4 p-1">
  <div class="flex justify-between items-center mb-2">
    <h3 class="text-sm font-medium text-gray-700 dark:text-gray-200">Diarization Model</h3>
    <div class="flex items-center">
      {#if isLoading}
        <span class="text-xs text-gray-500 dark:text-gray-400 mr-2">Checking...</span>
      {:else if hasAccess}
        <span class="text-sm font-medium text-green-600 dark:text-green-400 mr-2">Downloaded</span>
      {:else}
        <span class="text-sm font-medium text-red-600 dark:text-red-400 mr-2">Download Required</span>
      {/if}
    </div>
  </div>

  <div class="p-4 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-sm text-sm text-gray-700 dark:text-gray-300">
    <div class="mb-4">
        <p class="mb-2">
            Speaker diarization is the process of automatically identifying and separating different speakers in an audio file.
        </p>
        <p class="mb-2">
            Harvey uses the <a href="https://huggingface.co/pyannote/speaker-diarization-3.1" on:click|preventDefault={() => openLink('https://huggingface.co/pyannote/speaker-diarization-3.1')} class="text-blue-600 dark:text-blue-400 hover:underline">pyannote/speaker-diarization-3.1</a> model for this purpose.
        </p>
        <p class="mb-4">
            Access is gated, so you must first accept the user agreement on the model's HuggingFace page before you can download it.
        </p>
        <button class="btn-blue" on:click={() => openLink('https://huggingface.co/pyannote/speaker-diarization-3.1')}>
            Request Access on HuggingFace
        </button>
    </div>

    {#if $configStatus.python_libraries_installed}
    <div class="flex items-center space-x-2">
        {#if !hasAccess}
            <button on:click={handleDownload} class="btn-blue" disabled={isDownloading}>
                {#if isDownloading}
                    Downloading...
                {:else}
                    Download Model
                {/if}
            </button>
        {/if}
    </div>

    {#if hasAccess && cachePath}
        <div class="flex items-center justify-between mt-2">
            <p class="text-green-600 dark:text-green-400 text-xs truncate mr-2">
                Model downloaded at <code class="bg-gray-200 dark:bg-gray-700 text-gray-800 dark:text-gray-200 rounded px-1 py-0.5">{cachePath}</code>
            </p>
            <button on:click={handleDeleteModel} class="btn-red-small flex-shrink-0" disabled={isDeleting}>
                {#if isDeleting}Deleting...{:else}Delete{/if}
            </button>
        </div>
    {/if}
    {:else}
        <p class="text-orange-600 dark:text-orange-400 text-sm">
            Please install the required Python libraries first to enable model downloads.
        </p>
    {/if}

    {#if error && !showInstallModal}
      <p class="text-red-600 dark:text-red-400 mt-4">{error}</p>
    {/if}
  </div>
</div>

<InstallLogModal bind:showModal={showInstallModal} logs={installLogs} isInstalling={isDownloading} title="Diarization Model Download" inProgressText="Download in progress..." buttonInProgressText="Downloading..." />

<style lang="postcss">
    .btn-red-small {
		@apply px-2.5 py-1 text-xs border font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-1 transition-colors duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed;
		@apply border-transparent text-white bg-red-600 hover:bg-red-700 focus:ring-red-500;
	}

	.btn-blue {
		@apply px-2.5 py-1.5 border text-sm font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-1 transition-colors duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed;
		@apply border-transparent text-white bg-blue-600 hover:bg-blue-700 focus:ring-blue-500;
	}
</style>