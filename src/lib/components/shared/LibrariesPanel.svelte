<script>
	import { onMount, onDestroy } from 'svelte';
	import { get } from 'svelte/store';
	import { invoke } from "@tauri-apps/api/core";
	import { ask } from "@tauri-apps/plugin-dialog";
	import { listen } from '@tauri-apps/api/event';
	import { configStatus, updateConfigStatus, setPythonLibrariesInstalled } from '$lib/stores/configStatusStore.js';
	import InstallLogModal from '../modals/InstallLogModal.svelte';

	let isPanelOpen = false;
	let error = '';
	let showInstallModal = false;
	let showInfo = false;
	let installLogs = [];
	let isInstalling = false;
	let unlistenLog;
    let unlistenFinished;
	let isDeleting = false;

	async function handleDelete() {
		const confirmed = await ask("Are you sure you want to delete the local library environment? This will require a full re-installation to use AI features again.", { title: "Confirm Deletion", type: "warning", okLabel: "Delete", cancelLabel: "Cancel" });
		if (!confirmed) return;
		isDeleting = true;
		error = '';
		try {
			await invoke('delete_virtual_env');
			setPythonLibrariesInstalled(false);
		} catch (e) {
			console.error('Error deleting virtual environment:', e);
			error = `Failed to delete environment: ${e.payload || e}`;
		} finally {
			isDeleting = false;
		}
	}

	onMount(async () => {
        unlistenFinished = await listen('installation-finished', async () => {
            isInstalling = false;
            await updateConfigStatus(true);
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

<div class="border-y border-gray-200 dark:border-gray-700">
	<button
		on:click={() => (isPanelOpen = !isPanelOpen)}
		class="w-full flex justify-between items-center py-3 text-left focus:outline-none"
	>
		<div class="flex items-center">
			<h3 class="block text-sm font-medium text-gray-700 dark:text-gray-200">Libraries</h3>
		</div>
		<div class="flex items-center">
			{#if !$configStatus.isInitialized}
				<span class="text-xs text-gray-500 dark:text-gray-400 mr-2">Checking...</span>
			{:else if $configStatus.python_libraries_installed}
				<span class="text-sm font-medium text-green-600 dark:text-green-400 mr-2">Installed</span>
			{:else}
				<span class="text-sm font-medium text-red-600 dark:text-red-400 mr-2"
					>Installation Required</span
				>
			{/if}
			<svg
				class="w-6 h-6 transform transition-transform duration-200 ease-in-out {isPanelOpen
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
		<p class="mb-2">
			Harvey uses <strong>micromamba</strong> to install and manage a few required libraries.
			<button class="text-blue-600 dark:text-blue-400 hover:underline ml-1 focus:outline-none" on:click={() => showInfo = !showInfo}>
				{showInfo ? 'Hide info' : 'More info'}
			</button>
		</p>
		
		{#if showInfo}
			<ul class="list-disc list-inside mb-4 pl-2 space-y-1 text-gray-600 dark:text-gray-400">
				<li><strong>PyTorch & Transformers:</strong> The AI engine for running translation and analysis models locally.</li>
				<li><strong>pyannote.audio:</strong> Specifically for speaker identification (diarization).</li>
				<li><strong>FFmpeg:</strong> For processing audio and video files.</li>
				<li><strong>Pandoc:</strong> For converting and importing documents (e.g., MS Word).</li>
			</ul>
		{/if}

		<p class="mb-4 text-xs text-gray-500 dark:text-gray-500">
			Once downloaded, everything runs offline on your device to ensure privacy.
		</p>
		<div class="flex items-center">
			{#if !$configStatus.isInitialized}
				<p class="text-gray-500 dark:text-gray-400">Checking...</p>
			{:else if $configStatus.python_libraries_installed}
				<p class="text-green-600 dark:text-green-400 mr-4">Libraries are installed.</p>
				<button class="btn-red-small" on:click={handleDelete} disabled={isDeleting || isInstalling}>
					{#if isDeleting}Deleting...{:else}Delete{/if}
				</button>
			{:else}
				<p class="text-red-600 dark:text-red-400 mr-4">Required libraries are not installed.</p>
				<button class="btn-blue-small" on:click={handleInstall} disabled={isInstalling || isDeleting}>
					{#if isInstalling}Installing...{:else}Install{/if}
				</button>
			{/if}
		</div>
		{#if error}
			<p class="text-red-600 dark:text-red-400 mt-4">{error}</p>
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
