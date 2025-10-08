<script>
	import { onMount, onDestroy } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import InstallLogModal from '../modals/InstallLogModal.svelte';

	let isPanelOpen = false;
	let areLibsInstalled = false;
	let isLoading = true;
	let error = '';
	let showInstallModal = false;
	let installLogs = [];
	let isInstalling = false;
	let unlistenLog;
    let unlistenFinished;

	async function checkStatus() {
		try {
			isLoading = true;
			areLibsInstalled = await invoke('check_python_libraries_installed');
		} catch (e) {
			console.error('Error checking Python library status:', e);
			error = `Failed to check dependency status: ${e.message || e}`;
			areLibsInstalled = false;
		} finally {
			isLoading = false;
		}
	}

	onMount(async () => {
        await checkStatus();

        unlistenFinished = await listen('installation-finished', () => {
            isInstalling = false;
            checkStatus(); // Re-check status after installation attempt
        });
    });

	async function handleInstall() {
		showInstallModal = true;
		isInstalling = true;
		installLogs = [];

		try {
			unlistenLog = await listen('installation-log', (event) => {
				installLogs = [...installLogs, event.payload.message];
			});

			await invoke('install_python_libraries');
		} catch (e) {
			console.error('Error installing Python libraries:', e);
			installLogs = [...installLogs, `Error: ${e.message || e}`];
            isInstalling = false; // Set to false on error
		} finally {
			if (unlistenLog) {
				unlistenLog();
			}
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
</script>

<div class="border-y border-gray-200">
	<button
		on:click={() => (isPanelOpen = !isPanelOpen)}
		class="w-full flex justify-between items-center py-3 text-left focus:outline-none"
	>
		<div class="flex items-center">
			<h3 class="block text-sm font-medium text-gray-700">Python Libraries</h3>
		</div>
		<div class="flex items-center">
			{#if isLoading}
				<span class="text-xs text-gray-500 mr-2">Checking...</span>
			{:else if areLibsInstalled}
				<span class="text-sm font-medium text-green-600">Installed</span>
			{:else}
				<span class="text-sm font-medium text-red-600">Required</span>
			{/if}
			<svg
				class="w-6 h-6 transform transition-transform duration-200 ease-in-out {isPanelOpen
					? 'rotate-180'
					: ''}"
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
			Specific Python libraries are needed for advanced features like speaker diarization and translation. These will be installed in a dedicated virtual environment (`harvey_env`) to avoid conflicts with other Python projects.
		</p>

        <p class="mb-4">
            Main tools being installed:
            <code class="text-xs">pyannote.audio</code> for diarization,
            and <code class="text-xs">transformers</code> & <code class="text-xs">sacremoses</code> for translation.
        </p>

		{#if areLibsInstalled}
			<p class="text-green-600 mb-2">Libraries are installed in the virtual environment.</p>
		{:else}
            <div class="flex items-center">
                <p class="text-red-600 mr-4">
                    Required libraries are not installed.
                </p>
                <button
                    class="btn-blue-small"
                    on:click={handleInstall}
                    disabled={isInstalling}
                >
                    {isInstalling ? 'Installing...' : 'Install'}
                </button>
            </div>
		{/if}
		{#if error}
			<p class="text-red-600 mt-4">{error}</p>
		{/if}
	</div>
{/if}

<InstallLogModal bind:showModal={showInstallModal} logs={installLogs} isInstalling={isInstalling} title="Installation Logs" inProgressText="Installation in progress..." buttonInProgressText="Installing..." />

<style lang="postcss">
	.btn-blue-small {
		@apply px-2.5 py-1 text-xs border font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-1 transition-colors duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed;
		@apply border-transparent text-white bg-blue-600 hover:bg-blue-700 focus:ring-blue-500;
	}
</style>
