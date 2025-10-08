<!-- src/lib/components/shared/DiarizationModelPanel.svelte -->
<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open as openExternal } from '@tauri-apps/plugin-shell';

  let isPanelOpen = false;
  let hasAccess = false;
  let isLoading = true;
  let isDownloading = false;
  let error = '';
  let cachePath = '';

  async function checkAccessStatus() {
    isLoading = true;
    error = '';
    try {
      hasAccess = await invoke('check_diarization_model_access');
      if (hasAccess) {
        getCachePath(); // If we have access, try to get the path
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

  async function downloadDiarizationModel() {
    isDownloading = true;
    error = '';
    try {
      await invoke('download_diarization_model');
      await checkAccessStatus(); // This will re-check access and get the cache path
    } catch (e) {
      console.error('Error downloading diarization model:', e);
      error = `Failed to download model: ${e.message || e}`;
    } finally {
      isDownloading = false;
    }
  }

  onMount(checkAccessStatus);

  function openLink(url) {
    openExternal(url).catch((err) => console.error(`Failed to open link: ${err}`));
  }
</script>

<div class="border-y border-gray-200">
  <button
    on:click={() => (isPanelOpen = !isPanelOpen)}
    class="w-full flex justify-between items-center py-3 text-left focus:outline-none"
  >
    <div class="flex items-center">
      <h3 class="block text-sm font-medium text-gray-700">Access Diarization Model</h3>
    </div>
    <div class="flex items-center">
      {#if isLoading}
        <span class="text-xs text-gray-500 mr-2">Checking...</span>
      {:else if hasAccess}
        <span class="text-sm font-medium text-green-600">Granted</span>
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
    <p class="mb-2">
        This application uses the <code>pyannote/speaker-diarization-3.1</code> model for accurate speaker diarization.
    </p>
    <p class="mb-4">
        Access to this model is gated and requires you to accept the user agreement on its HuggingFace page.
    </p>

    <h4 class="font-semibold mb-2">Access Instructions</h4>
    <ol class="list-decimal list-inside mb-4 space-y-1">
      <li>
        Visit the model's page on HuggingFace: <a href="https://huggingface.co/pyannote/speaker-diarization-3.1" on:click|preventDefault={() => openLink('https://huggingface.co/pyannote/speaker-diarization-3.1')} class="text-blue-600 hover:underline">pyannote/speaker-diarization-3.1</a>.
      </li>
      <li>Accept the user agreement to gain access.</li>
      <li>Once you have access, you can download the model below.</li>
    </ol>

    <div class="flex items-center space-x-2">
        <button on:click={downloadDiarizationModel} class="btn-blue" disabled={hasAccess || isDownloading}>
            {#if isDownloading}
                Downloading...
            {:else if hasAccess}
                Model Downloaded
            {:else}
                Download Model
            {/if}
        </button>
    </div>

    {#if hasAccess && cachePath}
        <p class="text-xs text-gray-500 mt-2">
            Model files are located in: <code>{cachePath}</code>
        </p>
    {/if}

    {#if error}
      <p class="text-red-600 mt-4">{error}</p>
    {/if}
  </div>
{/if}

<style lang="postcss">
	.btn-blue {
		@apply px-2.5 py-1.5 border text-sm font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-1 transition-colors duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed;
		@apply border-transparent text-white bg-blue-600 hover:bg-blue-700 focus:ring-blue-500;
	}
</style>
