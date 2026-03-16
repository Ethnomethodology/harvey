<!-- src/lib/components/shared/DiarizationModelPanel.svelte -->
<script>
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from "@tauri-apps/api/core";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { listen } from '@tauri-apps/api/event';
  import { open as openExternal } from '@tauri-apps/plugin-shell';
  import { configStatus, setDiarizationModelDownloaded, updateConfigStatus, setHfTokenPresent } from '$lib/stores/configStatusStore.js';
  import InstallLogModal from '../modals/InstallLogModal.svelte';
  import { Input, Button } from 'flowbite-svelte';

  export let arePythonLibrariesInstalled = false;
  let hasAccess = false;
  let isLoading = true;
  let isDownloading = false;
  let isChecking = false;
  let error = '';
  let cachePath = '';
  let showInstallModal = false;
  let installLogs = [];
  let unlistenLog;
  let unlistenFinished;
  let isDeleting = false;

  // HF Auth State
  let isAuthenticated = false;
  let isAuthLoading = true;
  let authToken = '';
  const MASKED_TOKEN = '**********';

  async function checkAuthStatus() {
    isAuthLoading = true;
    try {
        isAuthenticated = await invoke('check_hf_auth_status');
        if (isAuthenticated) {
            authToken = MASKED_TOKEN;
            setHfTokenPresent(true);
        } else {
            authToken = '';
            setHfTokenPresent(false);
        }
    } catch (e) {
        console.error('Error checking HuggingFace auth status:', e);
        isAuthenticated = false;
        setHfTokenPresent(false);
    } finally {
        isAuthLoading = false;
    }
  }

  async function saveAuthToken() {
    if (authToken === MASKED_TOKEN || authToken.trim() === '') {
        return;
    }
    try {
        await invoke('save_hf_auth_token', { token: authToken });
        await checkAuthStatus();
    } catch (e) {
        console.error('Error saving HuggingFace auth token:', e);
        error = `Failed to save auth token: ${e.message || e}`;
    }
  }

  function handleFocus() {
      if (authToken === MASKED_TOKEN) {
          authToken = '';
      }
  }

  async function handleDeleteModel() {
    const confirmed = await ask("Are you sure you want to delete the diarization model? This will remove it from your disk.", { title: "Confirm Deletion", type: "warning", okLabel: "Delete", cancelLabel: "Cancel" });
    if (!confirmed) return;
    isDeleting = true;
    error = '';
    try {
      await invoke('delete_diarization_model');
      hasAccess = false;
      cachePath = '';
      setDiarizationModelDownloaded(false);
      // Wait a bit before refreshing config to let filesystem settle
      setTimeout(async () => {
          await updateConfigStatus(true);
          isDeleting = false;
      }, 500);
    } catch (e) {
      console.error('Error deleting diarization model:', e);
      error = `Failed to delete model: ${e.message || e}`;
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
    isChecking = false;
    installLogs = [];
    error = '';

    try {
      unlistenLog = await listen('diarization-installation-log', (event) => {
        installLogs = [...installLogs, { id: installLogs.length, message: event.payload.message }];
      });

      await invoke('download_diarization_model');
    } catch (e) {
      console.error('Error downloading diarization model:', e);
      error = `Failed to download model: ${e.message || e}`;
      installLogs = [...installLogs, { id: installLogs.length, message: `Error: ${e.message || e}` }];
      isDownloading = false; // Set to false on error
    } finally {
      if (unlistenLog) {
        unlistenLog();
      }
    }
  }

  onMount(async () => {
    await checkAuthStatus();
    unlistenFinished = await listen('diarization-installation-finished', async () => {
        isDownloading = false;
        isChecking = true;
        try {
            await updateConfigStatus(true);
            await checkAccessStatus(); // Re-check status after installation attempt
            setDiarizationModelDownloaded(hasAccess);
        } finally {
            isChecking = false;
        }
    });
  });

  // Reactively check status based on Python library installation
  $: {
    if ($configStatus.python_libraries_installed) {
        if ($configStatus.diarization_model_downloaded) {
            hasAccess = true;
            isLoading = false;
            if (!cachePath) getCachePath();
        } else {
            checkAccessStatus();
        }
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

<div class="flex flex-col space-y-4 pt-2 pb-8">
  <div class="flex justify-between items-center mb-2 px-1">
    <h3 class="text-sm font-medium text-gray-700 dark:text-gray-200">Diarization Model</h3>
    <div class="flex items-center">
      {#if !$configStatus.python_libraries_installed}
        <span class="text-sm font-medium text-red-600 dark:text-red-400 uppercase">PYTHON LIBRARIES MISSING</span>
      {:else if isLoading || isAuthLoading}
        <span class="text-xs text-gray-500 dark:text-gray-400 uppercase">CHECKING...</span>
      {:else if hasAccess}
        <span class="text-sm font-medium text-green-600 dark:text-green-400 uppercase">MODEL DOWNLOADED</span>
      {:else}
        <span class="text-sm font-medium text-yellow-600 dark:text-yellow-400 uppercase">NO MODEL DOWNLOADED</span>
      {/if}
    </div>
  </div>

  <div class="text-sm text-gray-700 dark:text-gray-300">
    <p class="mb-8 text-gray-600 dark:text-gray-400">
      Speaker diarization automatically identifies and separates different speakers in an audio file. Harvey uses the gated <a href="https://huggingface.co/pyannote/speaker-diarization-3.1" on:click|preventDefault={() => openLink('https://huggingface.co/pyannote/speaker-diarization-3.1')} class="text-blue-600 dark:text-blue-400 hover:underline font-medium">pyannote/speaker-diarization-3.1</a> model for this purpose. Follow the steps below to authenticate and download the model.
    </p>

    <ol class="relative text-gray-700 dark:text-gray-300 border-s border-gray-200 dark:border-gray-700 ml-3.5">
      <!-- Step 1: Create HF Account -->
      <li class="mb-10 ms-8">
          <span class="absolute flex items-center justify-center w-8 h-8 rounded-full -start-4 ring-4 ring-white dark:ring-gray-900 {isAuthenticated ? 'bg-green-100 text-green-600 dark:bg-green-900/30 dark:text-green-400' : 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400'}">
              <span class="font-medium text-sm">1</span>
          </span>
          <h3 class="font-medium leading-tight text-gray-900 dark:text-white mb-2">Create HuggingFace Account</h3>
          <p class="text-sm text-gray-600 dark:text-gray-400">If you don't have one, create a free HuggingFace account on their <a href="https://huggingface.co/join" on:click|preventDefault={() => openLink('https://huggingface.co/join')} class="text-blue-600 hover:underline dark:text-blue-400 font-medium">website</a>.</p>
      </li>

      <!-- Step 2: Create Auth Token -->
      <li class="mb-10 ms-8">
          <span class="absolute flex items-center justify-center w-8 h-8 rounded-full -start-4 ring-4 ring-white dark:ring-gray-900 {isAuthenticated ? 'bg-green-100 text-green-600 dark:bg-green-900/30 dark:text-green-400' : 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400'}">
              <span class="font-medium text-sm">2</span>
          </span>
          <h3 class="font-medium leading-tight text-gray-900 dark:text-white mb-2">Generate Access Token</h3>
          <p class="text-sm text-gray-600 dark:text-gray-400">Generate an access token from your HuggingFace account settings. You can find it under <a href="https://huggingface.co/settings/tokens" on:click|preventDefault={() => openLink('https://huggingface.co/settings/tokens')} class="text-blue-600 hover:underline dark:text-blue-400 font-medium">Access Tokens</a>.</p>
      </li>

      <!-- Step 3: Validate Token -->
      <li class="mb-10 ms-8">
          <span class="absolute flex items-center justify-center w-8 h-8 rounded-full -start-4 ring-4 ring-white dark:ring-gray-900 {isAuthenticated ? 'bg-green-100 text-green-600 dark:bg-green-900/30 dark:text-green-400' : 'bg-blue-100 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400'}">
              <span class="font-medium text-sm">3</span>
          </span>
          <h3 class="font-medium leading-tight text-gray-900 dark:text-white mb-2">Save Token to Harvey</h3>
          <p class="text-sm text-gray-600 dark:text-gray-400 mb-3">Paste your HuggingFace access token below.</p>
          <div class="flex items-center gap-2 max-w-md">
            <Input
              type="password"
              bind:value={authToken}
              on:focus={handleFocus}
              placeholder={isAuthenticated ? 'Token is set' : 'Enter your HuggingFace token'}
              autocomplete="off"
              autocorrect="off"
            />
            <Button color="alternative" on:click={saveAuthToken}>Save</Button>
          </div>
          {#if error && error.includes('auth')}
            <p class="text-red-600 dark:text-red-400 mt-2 text-xs">{error}</p>
          {/if}
      </li>

      <!-- Step 4: Accept User Agreement -->
      <li class="mb-10 ms-8">
          <span class="absolute flex items-center justify-center w-8 h-8 rounded-full -start-4 ring-4 ring-white dark:ring-gray-900 {hasAccess ? 'bg-green-100 text-green-600 dark:bg-green-900/30 dark:text-green-400' : (isAuthenticated ? 'bg-blue-100 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400' : 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400')}">
              <span class="font-medium text-sm">4</span>
          </span>
          <h3 class="font-medium leading-tight text-gray-900 dark:text-white mb-2">Accept Diarization Agreement</h3>
          <p class="text-sm text-gray-600 dark:text-gray-400 mb-3">Accept the user agreement on the Pyannote HuggingFace page to unlock access to the model.</p>
          <Button color="alternative" size="xs" on:click={() => openLink('https://huggingface.co/pyannote/speaker-diarization-3.1')}>
             Open Pyannote Agreement
          </Button>
      </li>

      <!-- Step 5: Download Model -->
      <li class="ms-8">
          <span class="absolute flex items-center justify-center w-8 h-8 rounded-full -start-4 ring-4 ring-white dark:ring-gray-900 {hasAccess ? 'bg-green-100 text-green-600 dark:bg-green-900/30 dark:text-green-400' : (isAuthenticated && !$configStatus.python_libraries_installed ? 'bg-orange-100 text-orange-600 dark:bg-orange-900/30 dark:text-orange-400' : 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400')}">
              <span class="font-medium text-sm">5</span>
          </span>
          <h3 class="font-medium leading-tight text-gray-900 dark:text-white mb-2">Download Model</h3>

          {#if !$configStatus.python_libraries_installed}
            <p class="text-sm text-orange-600 dark:text-orange-400 mb-3">
              Python libraries are missing. Please install them in the General Settings before downloading.
            </p>
          {:else if hasAccess && cachePath}
            <p class="text-sm text-green-600 dark:text-green-400 mb-3">Model is successfully downloaded.</p>
            <div class="flex items-center gap-4">
                <code class="text-xs bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-200 rounded px-2 py-1 flex-grow truncate">{cachePath}</code>
                <Button color="red" size="xs" on:click={handleDeleteModel} disabled={isDeleting} class="flex-shrink-0">
                    {#if isDeleting}Deleting...{:else}Delete Model{/if}
                </Button>
            </div>
          {:else}
            <p class="text-sm text-gray-600 dark:text-gray-400 mb-3">Once authenticated and approved, download the model to your machine.</p>
            <Button color="blue" on:click={handleDownload} disabled={isDownloading || !isAuthenticated}>
                {#if isDownloading}
                    Downloading...
                {:else}
                    Download Model
                {/if}
            </Button>
            {#if error && !error.includes('auth')}
              <p class="text-red-600 dark:text-red-400 mt-3 text-xs">{error}</p>
            {/if}
          {/if}
      </li>
    </ol>
  </div>
</div>

<InstallLogModal bind:showModal={showInstallModal} logs={installLogs} isInstalling={isDownloading} isChecking={isChecking} title="Diarization Model Download" inProgressText="Download in progress..." buttonInProgressText="Downloading..." />

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