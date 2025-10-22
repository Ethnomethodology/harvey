<script>
	import { onMount, onDestroy } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { arePythonLibsInstalled } from '$lib/stores/pythonStore.js';
	import { updateConfigStatus } from '$lib/stores/configStatusStore.js';
	import InstallLogModal from '../modals/InstallLogModal.svelte';

	let isPanelOpen = false;
	let isLoading = true;
	let error = '';
	let showInstallModal = false;
	let installLogs = [];
	let isInstalling = false;
	let unlistenLog;
    let unlistenFinished;
	let isDeleting = false;

	async function handleDelete() {
		isDeleting = true;
		error = '';
		try {
			await invoke('delete_virtual_env');
			await checkStatus(); // Re-check status after deletion
			await updateConfigStatus();
		} catch (e) {
			console.error('Error deleting virtual environment:', e);
			error = `Failed to delete environment: ${e.payload || e}`;
		} finally {
			isDeleting = false;
		}
	}

	async function checkStatus() {
		try {
			isLoading = true;
			const status = await invoke('check_python_libraries_installed');
            arePythonLibsInstalled.set(status);
		} catch (e) {
			console.error('Error checking Python library status:', e);
			error = '';
			arePythonLibsInstalled.set(false);
		} finally {
			isLoading = false;
		}
	}

	onMount(async () => {
        await checkStatus();

        // Temporary diagnostic code
        invoke('list_venv_lib_contents')
            .then(contents => console.log('Venv lib contents:', contents))
            .catch(err => console.error('Error listing venv lib contents:', err));

        unlistenFinished = await listen('installation-finished', async () => {
            isInstalling = false;
            await checkStatus(); // Re-check status after installation attempt
            await updateConfigStatus();
        });
    });

	async function handleInstall() {
		showInstallModal = true;
		isInstalling = true;
		installLogs = [];

		try {
			unlistenLog = await listen('installation-log', (event) => {
				installLogs = [...installLogs, { id: installLogs.length, message: event.payload.message }];
			});

			await invoke('install_python_libraries');
		} catch (e) {
			console.error('Error installing Python libraries:', e);
			installLogs = [...installLogs, { id: installLogs.length, message: `Error: ${e.payload}` }];
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
			<h3 class="block text-sm font-medium text-gray-700">Libraries</h3>
		</div>
		<div class="flex items-center">
			{#if isLoading}
				<span class="text-xs text-gray-500 mr-2">Checking...</span>
			{:else if $arePythonLibsInstalled}
				<span class="text-sm font-medium text-green-600 mr-2">Installed</span>
			{:else}
				<span class="text-sm font-medium text-red-600 mr-2">Installation Required</span>
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
		                    To enable advanced features like identifying different speakers (diarization) and translating transcripts, Harvey needs to install a few extra components.
		                </p>		<div class="flex items-center">
			{#if $arePythonLibsInstalled}
				<p class="text-green-600 mr-4">Libraries are installed.</p>
                <button class="btn-red-small" on:click={handleDelete} disabled={isDeleting || isInstalling}>
                    {#if isDeleting}Deleting...{:else}Delete{/if}
                </button>
			{:else}
				<p class="text-red-600 mr-4">Required libraries are not installed.</p>
                <button class="btn-blue-small" on:click={handleInstall} disabled={isInstalling || isDeleting}>
                    {#if isInstalling}Installing...{:else}Install{/if}
                </button>
			{/if}
		</div>
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

    .btn-red-small {
		@apply px-2.5 py-1 text-xs border font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-1 transition-colors duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed;
		@apply border-transparent text-white bg-red-600 hover:bg-red-700 focus:ring-red-500;
	}
</style>
