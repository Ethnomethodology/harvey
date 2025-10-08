<script>
	import { onMount, onDestroy } from 'svelte';
	import { ask } from '@tauri-apps/plugin-dialog';
	import { listen } from '@tauri-apps/api/event';
	import { open as openExternal } from '@tauri-apps/plugin-shell';
	import {
		downloadTranslationModel,
		deleteModel, // Re-using the same delete action
        getDownloadedModels, // Re-using to check status
		cancelDownload // Re-using the same cancel action
	} from '$lib/services/configureActions';
	import InstallLogModal from '$lib/components/modals/InstallLogModal.svelte';

	export let downloadLocation = '';
	export let isBusy = false;

	let downloadedModels = [];
	let configError = '';
	let downloadStatus = {};

	let fromLanguage = 'en';
	let toLanguage = 'ja';

	let showLogModal = false;
	let modalLogs = [];
	let isDownloading = false;

	const languages = [
		{ code: 'en', name: 'English' },
		{ code: 'ja', name: 'Japanese' },
	];

	$: isAnyModelDownloading = Object.values(downloadStatus).includes('downloading');
    $: isBusy = isAnyModelDownloading;

	let unlistenStart = null;
	let unlistenProgress = null;
	let unlistenComplete = null;
	let unlistenError = null;

	onMount(async () => {
		configError = '';
		try {
			downloadedModels = await getDownloadedModels();
		} catch (e) {
			configError = `Failed to load model configuration: ${e.message || e}`;
		}

		try {
			unlistenStart = await listen('translation-download-start', (event) => {
				const modelName = event.payload;
				downloadStatus = { ...downloadStatus, [modelName]: 'downloading' };
				modalLogs = [...modalLogs, `Starting download for ${modelName}...`];
				isDownloading = true;
				showLogModal = true;
			});

			unlistenProgress = await listen('translation-download-progress', (event) => {
                const { model_name, file_name, downloaded_bytes, total_bytes } = event.payload;
				if (downloadStatus[model_name] === 'downloading') {
					const downloadedMB = (downloaded_bytes / (1024 * 1024)).toFixed(1);
					const totalMB = total_bytes ? (total_bytes / (1024 * 1024)).toFixed(1) : '??';
					const percentage = total_bytes ? ((downloaded_bytes / total_bytes) * 100).toFixed(0) : '0';
					modalLogs = [...modalLogs, `Downloading ${file_name}: ${percentage}% (${downloadedMB} / ${totalMB} MB)`];
				}
			});

			unlistenComplete = await listen('translation-download-complete', async (event) => {
				const modelName = event.payload;
				downloadStatus = { ...downloadStatus, [modelName]: 'complete' };
				try {
					downloadedModels = await getDownloadedModels();
				} catch (e) { console.error(`Failed to refresh models after ${modelName} completion:`, e); }
				modalLogs = [...modalLogs, `Download complete for ${modelName}.`];
				isDownloading = false;
			});

			unlistenError = await listen('translation-download-error', (event) => {
				const { model_name, error_message } = event.payload;
				let finalStatus;
				if (error_message.toLowerCase().includes('cancel')) { finalStatus = 'cancelled'; } else { finalStatus = 'error'; alert(`Error downloading ${model_name}: ${error_message}`); }
				downloadStatus = { ...downloadStatus, [model_name]: finalStatus };
				modalLogs = [...modalLogs, `Error downloading ${model_name}: ${error_message}`];
				isDownloading = false;
			});
		} catch (err) {
			configError = 'Could not set up download monitoring.';
		}
	});

	onDestroy(() => {
		if (unlistenStart) unlistenStart();
		if (unlistenProgress) unlistenProgress();
		if (unlistenComplete) unlistenComplete();
		if (unlistenError) unlistenError();
	});

	async function handleDownload() {
		if (isBusy) return;
		if (!downloadLocation || downloadLocation.trim() === '') { alert('Please set a valid model download location first.'); return; }
		configError = '';
		try {
			modalLogs = [];
            await downloadTranslationModel(fromLanguage, toLanguage, downloadLocation);
        } catch (err) {
			alert(`Failed to start download for ${fromLanguage}-${toLanguage}: ${err.message || err}`);
		}
    }

    async function handleDelete(model) {
		if (isBusy) return;
		if (!model?.name) { alert("Cannot delete model: Missing name."); return; }
		const confirmed = await ask(`Are you sure you want to delete the model "${model.name}"? This will remove the entire model folder from disk.`, { title: 'Confirm Deletion', type: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' });
		if (!confirmed) return;
		try {
			await deleteModel(model); // Re-uses the same action
			downloadedModels = await getDownloadedModels();
            downloadStatus = { ...downloadStatus, [model.name]: 'not_downloaded' };
		} catch (err) {
			alert(`Failed to delete model ${model.name}: ${err.message || err}`);
		}
    }

</script>

<div class="flex flex-col h-full">
	<InstallLogModal bind:showModal={showLogModal} logs={modalLogs} isInstalling={isDownloading} title="Downloading Translation Model" inProgressText="Downloading..." />
	{#if configError}
		<p class="text-red-600 bg-red-100 p-3 rounded-md text-sm text-left py-2 mb-4 break-words flex-shrink-0">
			<span class="font-medium">Error:</span> {configError}
		</p>
	{/if}

	<div class="flex-grow space-y-3">
		<div class="pt-4 space-y-3">
			<div class="flex items-center space-x-2">
				<div class="flex-1">
					<label for="from-language" class="block text-sm font-medium text-gray-700">From Language</label>
					<select id="from-language" bind:value={fromLanguage} class="mt-1 block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm rounded-md">
						{#each languages as lang}
							<option value={lang.code}>{lang.name}</option>
						{/each}
					</select>
				</div>
				<div class="flex-1">
					<label for="to-language" class="block text-sm font-medium text-gray-700">To Language</label>
					<select id="to-language" bind:value={toLanguage} class="mt-1 block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm rounded-md">
						{#each languages as lang}
							<option value={lang.code}>{lang.name}</option>
						{/each}
					</select>
				</div>
				<button on:click={handleDownload} class="btn-blue-small mt-6">Download</button>
			</div>
			<div>
				<h4 class="text-sm font-medium text-gray-700">Downloaded Models</h4>
				<ul class="mt-2 space-y-2">
					{#each downloadedModels.filter(m => m.name.startsWith('Helsinki-NLP')) as model}
						<li class="p-2 border rounded-md">
							<div class="flex items-center justify-between">
								<p class="text-sm font-medium text-gray-900">{model.name}</p>
								<button on:click={() => handleDelete(model)} class="btn-delete">Delete</button>
							</div>
						</li>
					{/each}
				</ul>
			</div>
		</div>
	</div>
</div>

<style lang="postcss">
	.btn-blue-small, .btn-delete, .btn-cancel, .btn-retry {
		@apply px-2.5 py-1.5 border text-sm font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-1 transition-colors duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed;
        @apply px-2.5 py-1 text-xs;
    }
    .btn-blue-small {
        @apply border-transparent text-white bg-blue-600 hover:bg-blue-700 focus:ring-blue-500;
    }
	.btn-delete {
		@apply border-gray-300 text-red-700 bg-red-50 hover:bg-red-100 focus:ring-red-400;
	}
	.btn-cancel {
		@apply border-gray-300 text-gray-700 bg-gray-100 hover:bg-gray-200 focus:ring-indigo-500;
	}
    .btn-retry {
        @apply border-transparent text-white bg-yellow-500 hover:bg-yellow-600 focus:ring-yellow-500;
    }
    .tabular-nums { font-variant-numeric: tabular-nums; }
</style>