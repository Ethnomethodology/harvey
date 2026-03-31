<script>
	import { onMount, onDestroy } from 'svelte';
	import { get } from 'svelte/store';
	import { invoke } from "@tauri-apps/api/core";
	import { ask } from "@tauri-apps/plugin-dialog";
	import { listen } from '@tauri-apps/api/event';
	import { open as openExternal } from '@tauri-apps/plugin-shell';
	import { configStatus, updateConfigStatus, setPythonLibrariesInstalled } from '$lib/stores/configStatusStore.js';
	import InstallLogModal from '../modals/InstallLogModal.svelte';

	let isPanelOpen = false;
	let error = '';
	let showInstallModal = false;
	let showInfo = false;
	let installLogs = [];
	let isInstalling = false;
	let isChecking = false;
	let unlistenLog;
    let unlistenFinished;
	let isDeleting = false;

	function openLink(url) {
        openExternal(url).catch((err) => console.error(`Failed to open link: ${err}`));
    }

	async function handleDelete() {
		const confirmed = await ask("Are you sure you want to delete the local library environment? This will delete all installed libraries AND downloaded models. It will require a full re-installation to use AI features again.", { title: "Confirm Deletion", type: "warning", okLabel: "Delete", cancelLabel: "Cancel" });
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
            isChecking = true;
            try {
                await updateConfigStatus(true);
            } finally {
                isChecking = false;
            }
        });
    });

	async function handleInstall() {
		showInstallModal = true;
		isInstalling = true;
		isChecking = false;
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

<div class="bg-white dark:bg-gray-800/60 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden">
	<div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 bg-gray-50/50 dark:bg-gray-800/30 flex justify-between items-center">
		<h3 class="text-base font-semibold text-gray-900 dark:text-white">Libraries</h3>
		<div class="flex items-center">
			{#if !$configStatus.isInitialized}
				<span class="text-xs text-gray-500 dark:text-gray-400 uppercase">Checking...</span>
			{:else if $configStatus.python_libraries_installed}
				<span class="text-xs font-medium text-green-600 dark:text-green-400 uppercase">Installed</span>
			{:else}
				<span class="text-xs font-medium text-red-600 dark:text-red-400 uppercase">Installation Required</span>
			{/if}
		</div>
	</div>

	<div class="p-6 text-sm text-gray-700 dark:text-gray-300">
		<p class="mb-2">
			Harvey uses <strong><button class="text-blue-600 dark:text-blue-400 hover:underline focus:outline-none font-bold" on:click={() => openLink('https://mamba.readthedocs.io/en/latest/user_guide/micromamba.html')}>micromamba</button></strong> to install and manage a few required libraries.
			<button class="text-blue-600 dark:text-blue-400 hover:underline ml-1 focus:outline-none" on:click={() => showInfo = !showInfo}>
				{showInfo ? 'Hide info' : 'More info'}
			</button>
		</p>
		
		{#if showInfo}
			<ul class="list-disc list-inside mb-4 pl-2 space-y-1 text-gray-600 dark:text-gray-400">
				<li>
					<strong>
						<button class="text-blue-600 hover:underline" on:click={() => openLink('https://www.python.org/')}>Python</button> & 
						<button class="text-blue-600 hover:underline" on:click={() => openLink('https://mamba.readthedocs.io/en/latest/user_guide/micromamba.html')}>micromamba</button>:
					</strong> Core runtime for executing AI models locally.
				</li>
				<li>
					<strong>
						<button class="text-blue-600 hover:underline" on:click={() => openLink('https://pytorch.org/')}>PyTorch</button> & 
						<button class="text-blue-600 hover:underline" on:click={() => openLink('https://huggingface.co/docs/transformers/index')}>Transformers</button>:
					</strong> The AI engine for running translation and analysis models locally.
				</li>
				<li>
					<strong>
						<button class="text-blue-600 hover:underline" on:click={() => openLink('https://github.com/pyannote/pyannote-audio')}>pyannote.audio</button>:
					</strong> Specifically for speaker identification (diarization).
				</li>
				<li>
					<strong>
						<button class="text-blue-600 hover:underline" on:click={() => openLink('https://ffmpeg.org/')}>FFmpeg</button>:
					</strong> For processing audio and video files.
				</li>
				<li>
					<strong>
						<button class="text-blue-600 hover:underline" on:click={() => openLink('https://pandoc.org/')}>Pandoc</button>:
					</strong> For converting and importing documents (e.g., MS Word).
				</li>
			</ul>
		{/if}

		<p class="mb-6 text-xs text-gray-500 dark:text-gray-500">
			Once downloaded, everything runs offline on your device to ensure privacy.
		</p>
		<div class="flex items-center">
			{#if !$configStatus.isInitialized}
				<p class="text-gray-500 dark:text-gray-400">Checking...</p>
			{:else if $configStatus.python_libraries_installed}
				<p class="text-green-600 dark:text-green-400 mr-4 font-medium">Libraries are installed.</p>
				<button class="btn-red-small" on:click={handleDelete} disabled={isDeleting || isInstalling}>
					{#if isDeleting}Deleting...{:else}Delete{/if}
				</button>
			{:else}
				<p class="text-red-600 dark:text-red-400 mr-4 font-medium">Required libraries are not installed.</p>
				<button class="btn-blue-small" on:click={handleInstall} disabled={isInstalling || isDeleting}>
					{#if isInstalling}Installing...{:else}Install{/if}
				</button>
			{/if}
		</div>
		{#if error}
			<p class="text-red-600 dark:text-red-400 mt-4">{error}</p>
		{/if}
	</div>
</div>

<InstallLogModal bind:showModal={showInstallModal} logs={installLogs} isInstalling={isInstalling} isChecking={isChecking} title="Installation Logs" inProgressText="Installation in progress..." buttonInProgressText="Installing..." />

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
