<!-- src/lib/components/shared/HuggingFacePanel.svelte -->
<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open as openExternal } from '@tauri-apps/plugin-shell';
  import { setHfTokenPresent } from '$lib/stores/configStatusStore.js';

  let isPanelOpen = false;
  let isAuthenticated = false;
  let isLoading = true;
  let error = '';
  let authToken = '';
  const MASKED_TOKEN = '**********';

  async function checkAuthStatus() {
    isLoading = true;
    error = '';
    try {
        isAuthenticated = await invoke('check_hf_auth_status');
        if (isAuthenticated) {
            authToken = MASKED_TOKEN;
        } else {
            authToken = '';
        }
    } catch (e) {
        console.error('Error checking HuggingFace auth status:', e);
        error = `Failed to check auth status: ${e.message || e}`;
        isAuthenticated = false;
    } finally {
        isLoading = false;
    }
  }

  async function saveAuthToken() {
    if (authToken === MASKED_TOKEN || authToken.trim() === '') {
        return;
    }
    try {
        await invoke('save_hf_auth_token', { token: authToken });
        await checkAuthStatus();
        setHfTokenPresent(true);
    } catch (e) {
        console.error('Error saving HuggingFace auth token:', e);
        error = `Failed to save auth token: ${e.message || e}`;
    }
  }

  onMount(checkAuthStatus);

  function handleFocus() {
      if (authToken === MASKED_TOKEN) {
          authToken = '';
      }
  }

  function openLink(url) {
    openExternal(url).catch((err) => console.error(`Failed to open link: ${err}`));
  }
</script>

<div class="border-y border-gray-200 dark:border-gray-700">
  <button
    on:click={() => (isPanelOpen = !isPanelOpen)}
    class="w-full flex justify-between items-center py-3 text-left focus:outline-none"
  >
    <div class="flex items-center">
      <h3 class="block text-sm font-medium text-gray-700 dark:text-gray-200">HuggingFace Authentication</h3>
    </div>
    		<div class="flex items-center">
    			{#if isLoading}
				<span class="text-xs text-gray-500 dark:text-gray-400 mr-2">Checking...</span>
    			{:else if isAuthenticated}
				<span class="text-sm font-medium text-green-600 dark:text-green-400 mr-2">Authenticated</span>
    			{:else}
				<span class="text-sm font-medium text-red-600 dark:text-red-400 mr-2">Authentication Required</span>
    			{/if}
    			<svg        class="w-6 h-6 transform transition-transform duration-200 ease-in-out {isPanelOpen
          ? 'rotate-180'
          : ''} text-gray-500 dark:text-gray-400"
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
  <div class="p-4 bg-gray-100 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 text-sm text-gray-700 dark:text-gray-300">
    <p class="mb-4">
      HuggingFace is the primary source for Harvey's AI models. It is used to download:
    </p>
    <ul class="list-disc list-inside mb-4 pl-2 space-y-1 text-gray-600 dark:text-gray-400">
      <li><strong>Transcription models:</strong> High-performance Whisper models.</li>
      <li><strong>Translation models:</strong> Over 1,500 language pairs from Helsinki-NLP.</li>
      <li><strong>Diarization models:</strong> Advanced speaker identification tools.</li>
    </ul>
    <p class="mb-4">
      While most models are public, an access token is required to download <strong>gated models</strong> (like those for speaker identification) and helps avoid download rate limits.
    </p>

    <h4 class="font-semibold mb-2 dark:text-gray-200">Setup Instructions</h4>
    <ol class="list-decimal list-inside mb-4 space-y-1">
      <li>If you don't have one, create a HuggingFace account on their <a href="https://huggingface.co/join" on:click|preventDefault={() => openLink('https://huggingface.co/join')} class="text-blue-600 hover:underline dark:text-blue-400">website</a>.</li>
      <li>
        Generate an access token from your HuggingFace account settings. You can find it under <a href="https://huggingface.co/settings/tokens" on:click|preventDefault={() => openLink('https://huggingface.co/settings/tokens')} class="text-blue-600 hover:underline dark:text-blue-400">Access Tokens</a>.
      </li>
      <li>Paste the access token in the field below and click "Save".</li>
    </ol>

    <div class="flex items-center space-x-2">
      <input
        type="password"
        bind:value={authToken}
        on:focus={handleFocus}
        placeholder={isAuthenticated ? 'Token is set' : 'Enter your HuggingFace token'}
        class="flex-grow shadow-sm focus:ring-blue-500 focus:border-blue-500 block w-full sm:text-sm border-gray-300 rounded-md bg-white dark:bg-gray-700 dark:border-gray-600"
        autocomplete="off"
        autocorrect="off"
      />
      <button on:click={saveAuthToken} class="btn-blue">Save</button>
    </div>

    {#if error}
      <p class="text-red-600 dark:text-red-400 mt-4">{error}</p>
    {/if}
  </div>
{/if}

<style lang="postcss">
	.btn-blue {
		@apply px-2.5 py-1.5 border text-sm font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-1 transition-colors duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed;
		@apply border-transparent text-white bg-blue-600 hover:bg-blue-700 focus:ring-blue-500;
	}
</style>
