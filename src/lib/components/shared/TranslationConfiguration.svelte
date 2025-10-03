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

	export let downloadLocation = '';
	export let isBusy = false;
	let isModelsPanelOpen = false;

	let downloadedModels = [];
	let configError = '';
	let downloadStatus = {};
	let downloadProgress = {};

    const HUGGING_FACE_MODEL_URL = 'https://huggingface.co';

	const availableModels = [
        {
            name: 'opus-mt-jap-en',
            language: 'Japanese to English',
            size: '~300 MB',
            description: 'Translate Japanese text to English.',
            download_url: `${HUGGING_FACE_MODEL_URL}/Helsinki-NLP/opus-mt-jap-en`,
            info_url: `${HUGGING_FACE_MODEL_URL}/Helsinki-NLP/opus-mt-jap-en`,
        },
        {
            name: 'opus-mt-en-jap',
            language: 'English to Japanese',
            size: '~300 MB',
            description: 'Translate English text to Japanese.',
            download_url: `${HUGGING_FACE_MODEL_URL}/Helsinki-NLP/opus-mt-en-jap`,
            info_url: `${HUGGING_FACE_MODEL_URL}/Helsinki-NLP/opus-mt-en-jap`,
        },
	];

	$: isAnyModelDownloading = Object.values(downloadStatus).includes('downloading');
    $: isBusy = isAnyModelDownloading;

	let modelDisplayData = {};
	$: {
		const newData = {};
		const currentDownloaded = Array.isArray(downloadedModels) ? downloadedModels : [];
		for (const model of availableModels) {
			const name = model.name;
			const getStatus = (modelName) => {
				const liveStatus = downloadStatus[modelName];
				if (liveStatus && liveStatus !== 'not_downloaded') return liveStatus;
				return currentDownloaded.some((m) => m?.name === modelName) ? 'complete' : 'not_downloaded';
			};
			const status = getStatus(name);
			const progress = downloadProgress[name];

			const getProgressPercent = () => {
				if (status !== 'downloading' || !progress || !progress.total_bytes || progress.total_bytes <= 0) {
					return 0;
				}
				return Math.min(100, Math.max(0, (progress.downloaded_bytes / progress.total_bytes) * 100));
			}

			const getText = () => {
				if (status !== 'downloading' || !progress) return '';
                const downloadedMB = (progress.downloaded_bytes / (1024 * 1024)).toFixed(1);
				if (progress.total_bytes && progress.total_bytes > 0) {
					const percentage = getProgressPercent().toFixed(0);
					const totalMB = (progress.total_bytes / (1024 * 1024)).toFixed(1);
					return `${percentage}% (${downloadedMB} / ${totalMB} MB) - ${progress.file_name}`;
				} else {
					return `${downloadedMB} MB - ${progress.file_name}`;
				}
			};
			const progressText = getText();
			const progressPercent = getProgressPercent();
			newData[name] = { status, progressText, progressPercent };
		}
		modelDisplayData = newData;
	}

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
				if (!modelName || !availableModels.some(m => m.name === modelName)) return;
				downloadStatus = { ...downloadStatus, [modelName]: 'downloading' };
				downloadProgress = { ...downloadProgress, [modelName]: { downloaded_bytes: 0, total_bytes: 0, file_name: 'Starting...' } };
			});

			unlistenProgress = await listen('translation-download-progress', (event) => {
                const { model_name, file_name, downloaded_bytes, total_bytes } = event.payload;
				if (!model_name || !availableModels.some(m => m.name === model_name)) return;
				if (downloadStatus[model_name] === 'downloading') {
					downloadProgress = { ...downloadProgress, [model_name]: { file_name, downloaded_bytes, total_bytes } };
				}
			});

			unlistenComplete = await listen('translation-download-complete', async (event) => {
				const modelName = event.payload;
				if (!modelName || !availableModels.some(m => m.name === modelName)) return;
				const newProgress = { ...downloadProgress }; delete newProgress[modelName]; downloadProgress = newProgress;
				downloadStatus = { ...downloadStatus, [modelName]: 'complete' };
				try {
					downloadedModels = await getDownloadedModels();
				} catch (e) { console.error(`Failed to refresh models after ${modelName} completion:`, e); }
			});

			unlistenError = await listen('translation-download-error', (event) => {
				const { model_name, error_message } = event.payload;
				if (!model_name || !availableModels.some(m => m.name === model_name)) return;
				let finalStatus;
				if (error_message.toLowerCase().includes('cancel')) { finalStatus = 'cancelled'; } else { finalStatus = 'error'; alert(`Error downloading ${model_name}: ${error_message}`); }
				const newProgress = { ...downloadProgress }; delete newProgress[model_name]; downloadProgress = newProgress;
				downloadStatus = { ...downloadStatus, [model_name]: finalStatus };
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

	async function openLink(url) {
		if (!url) return;
		try { await openExternal(url); } catch (err) { alert(`Could not open link: ${url}`); }
	}

    async function handleDownload(model) {
		if (isBusy) return;
		const currentStatus = modelDisplayData[model.name]?.status || 'not_downloaded';
		if (['downloading', 'complete', 'cancelling'].includes(currentStatus)) return;
		if (!downloadLocation || downloadLocation.trim() === '') { alert('Please set a valid model download location first.'); return; }
		configError = '';
		try {
            await downloadTranslationModel(model, downloadLocation);
        } catch (err) {
			alert(`Failed to start download for ${model.name}: ${err.message || err}`);
			downloadStatus = { ...downloadStatus, [model.name]: 'error' };
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

    async function handleCancel(modelName) {
		if (!modelName || !availableModels.some(m => m.name === modelName)) return;
		const currentStatus = modelDisplayData[modelName]?.status;
		if (currentStatus !== 'downloading') return;
		downloadStatus = { ...downloadStatus, [modelName]: 'cancelling' };
		try {
            await cancelDownload(modelName); // Re-uses the same action
        } catch (err) {
			alert(`Failed to send cancel request for ${modelName}: ${err.message || err}`);
			downloadStatus = { ...downloadStatus, [modelName]: 'downloading' };
		}
    }

</script>

<div class="flex flex-col h-full">
	{#if configError}
		<p class="text-red-600 bg-red-100 p-3 rounded-md text-sm text-left py-2 mb-4 break-words flex-shrink-0">
			<span class="font-medium">Error:</span> {configError}
		</p>
	{/if}

	<div class="flex-grow space-y-3">
		<div class="border-y border-gray-200">
			<button on:click={() => isModelsPanelOpen = !isModelsPanelOpen} class="w-full flex justify-between items-center py-3 text-left focus:outline-none">
				<h3 class="text-lg font-medium text-gray-900">
					Available Models <span class="text-base font-normal text-gray-500">({downloadedModels.filter(m => availableModels.some(am => am.name === m.name)).length} downloaded)</span>
				</h3>
				<svg class="w-6 h-6 transform transition-transform duration-200 ease-in-out {isModelsPanelOpen ? 'rotate-180' : ''}" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor">
					<path stroke-linecap="round" stroke-linejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
				</svg>
			</button>
		</div>

		{#if isModelsPanelOpen}
			<div class="pt-4 space-y-3">
				{#each availableModels as model (model.name)}
					{@const display = modelDisplayData[model.name] || { status: 'not_downloaded', progressText: '', progressPercent: 0 }}
                    {@const status = display.status}
                    {@const isDownloadEnabled = !isBusy && downloadLocation && downloadLocation.trim() !== '' && model.download_url}
                    {@const isDeleteEnabled = !isBusy}
                    {@const isCancelEnabled = status === 'downloading'}
					<div class="bg-white p-4 rounded-lg shadow border border-gray-200 relative overflow-hidden">
                        {#if status === 'downloading'}
                            <div class="absolute top-0 left-0 bottom-0 bg-blue-100 bg-opacity-75 transition-all duration-150 ease-linear pointer-events-none" style:width={`${display.progressPercent}%`}></div>
                            <div class="absolute top-0 left-0 bottom-0 border-r-2 border-blue-300 transition-all duration-150 ease-linear pointer-events-none" style:width={`${display.progressPercent}%`}></div>
                        {/if}
						<div class="relative z-10">
							<div class="flex justify-between items-start mb-2">
								<p class="text-md font-semibold text-gray-800 truncate mr-4 pt-1" title={model.name}>
									{model.name}
								</p>
								<div class="flex-shrink-0 flex items-center space-x-2">
									{#if status === 'complete'}
                                        <button class="btn-delete" on:click={() => handleDelete(model)} disabled={!isDeleteEnabled} title={isDeleteEnabled ? `Delete model ${model.name}` : 'Operation in progress...'}> Delete </button>
                                    {:else if status === 'downloading' || status === 'cancelling'}
                                        <span class="text-xs text-blue-700 font-medium w-48 text-right truncate tabular-nums" title={display.progressText || (status === 'cancelling' ? 'Cancelling...' : 'Starting...')}>
                                            {#if status === 'cancelling'}Cancelling...{:else}{display.progressText || 'Starting...'}{/if}
                                        </span>
                                        <button class="btn-cancel" on:click={() => handleCancel(model.name)} disabled={!isCancelEnabled} title={isCancelEnabled ? 'Cancel download' : 'Cannot cancel'}> Cancel </button>
                                    {:else if status === 'error'}
                                        <span class="text-xs text-red-600 font-medium">Error</span>
                                        <button class="btn-retry" on:click={() => handleDownload(model)} disabled={!isDownloadEnabled} title={!isDownloadEnabled ? 'Set location or download ongoing' : 'Download failed. Click to retry.'}> Retry </button>
                                    {:else if status === 'cancelled'}
                                        <span class="text-xs text-gray-500 font-medium">Cancelled</span>
                                        <button class="btn-blue-small" on:click={() => handleDownload(model)} disabled={!isDownloadEnabled} title={!isDownloadEnabled ? 'Set location or download ongoing' : 'Download cancelled. Click to try again.'}> Download </button>
                                    {:else}
                                        <button class="btn-blue-small" on:click={() => handleDownload(model)} title={!downloadLocation || downloadLocation.trim() === '' ? 'Set download location first' : !model.download_url ? 'Download URL missing' : isBusy ? 'Operation in progress...' : `Download model ${model.name}`} disabled={!isDownloadEnabled}> Download </button>
                                    {/if}
								</div>
							</div>
							<div class="text-sm text-gray-600 space-y-1 mt-1">
								<p><span class="font-medium text-gray-700">Language:</span> {model.language || '-'} <span class="mx-2 text-gray-300">|</span> <span class="font-medium text-gray-700">Size:</span> {model.size || '-'}</p>
								<p><span class="font-medium text-gray-700">Description:</span> {model.description || '-'}</p>
								{#if model.info_url}
									<p><a href={model.info_url} on:click|preventDefault={() => openLink(model.info_url)} class="text-blue-600 hover:text-blue-800 hover:underline text-xs" title="Open model info page in browser"> Learn more... </a></p>
								{/if}
							</div>
							{#if status === 'not_downloaded' && !isDownloadEnabled && !isBusy}
								<p class="text-xs text-orange-600 mt-2">
									Set a download location to enable download.
								</p>
							{/if}
						</div>
					</div>
				{/each}
			</div>
		{/if}
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