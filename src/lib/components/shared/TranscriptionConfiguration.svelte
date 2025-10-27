<script>
	import { onMount, onDestroy } from 'svelte';
	import { ask } from '@tauri-apps/plugin-dialog';
	import { listen } from '@tauri-apps/api/event';
	import { open as openExternal } from '@tauri-apps/plugin-shell';
	import {
		downloadModel,
		deleteModel,
		getDownloadedModels,
		cancelDownload,
	} from '$lib/services/configureActions';
	import { setTranscriptionModelsDownloaded } from '$lib/stores/configStatusStore.js';
	import DiarizationModelPanel from './DiarizationModelPanel.svelte';

	export let downloadLocation = '';
	export let isBusy = false;
	let isModelsPanelOpen = false;

	let downloadedModels = [];
	let configError = '';
	let downloadStatus = {};
	let downloadProgress = {};

	const WHISPER_CPP_INFO_URL = 'https://github.com/ggerganov/whisper.cpp';
	const HUGGING_FACE_BASE = 'https://huggingface.co/ggerganov/whisper.cpp/resolve/main';

	const availableModels = [
		{ name: 'ggml-large-v3', language: 'Multilingual', size: '2.9 GiB', description: 'Latest and most accurate multilingual model.', download_url: `${HUGGING_FACE_BASE}/ggml-large-v3.bin`, info_url: WHISPER_CPP_INFO_URL },
		{ name: 'ggml-large-v3-turbo', language: 'Multilingual', size: '1.5 GiB', description: 'Optimized for speed, great for real-time transcription.', download_url: `${HUGGING_FACE_BASE}/ggml-large-v3-turbo.bin`, info_url: WHISPER_CPP_INFO_URL },
		{ name: 'ggml-medium.en', language: 'English-only', size: '1.5 GiB', description: 'Highest accuracy for English-only applications.', download_url: `${HUGGING_FACE_BASE}/ggml-medium.en.bin`, info_url: WHISPER_CPP_INFO_URL },
		{ name: 'ggml-medium', language: 'Multilingual', size: '1.5 GiB', description: 'High accuracy across multiple languages.', download_url: `${HUGGING_FACE_BASE}/ggml-medium.bin`, info_url: WHISPER_CPP_INFO_URL },
		{ name: 'ggml-small.en', language: 'English-only', size: '466 MiB', description: 'Excellent balance of speed and accuracy for English.', download_url: `${HUGGING_FACE_BASE}/ggml-small.en.bin`, info_url: WHISPER_CPP_INFO_URL },
		{ name: 'ggml-small', language: 'Multilingual', size: '466 MiB', description: 'Excellent balance for multilingual use.', download_url: `${HUGGING_FACE_BASE}/ggml-small.bin`, info_url: WHISPER_CPP_INFO_URL },
		{ name: 'ggml-base.en', language: 'English-only', size: '142 MiB', description: 'Fast and lightweight for English.', download_url: `${HUGGING_FACE_BASE}/ggml-base.en.bin`, info_url: WHISPER_CPP_INFO_URL },
		{ name: 'ggml-base', language: 'Multilingual', size: '142 MiB', description: 'Fast and lightweight for multilingual use.', download_url: `${HUGGING_FACE_BASE}/ggml-base.bin`, info_url: WHISPER_CPP_INFO_URL },
		{ name: 'ggml-tiny.en', language: 'English-only', size: '75 MiB', description: 'Smallest and fastest for English, for limited resources.', download_url: `${HUGGING_FACE_BASE}/ggml-tiny.en.bin`, info_url: WHISPER_CPP_INFO_URL },
		{ name: 'ggml-tiny', language: 'Multilingual', size: '75 MiB', description: 'Smallest and fastest multilingual model.', download_url: `${HUGGING_FACE_BASE}/ggml-tiny.bin`, info_url: WHISPER_CPP_INFO_URL },
	];

	let modelDisplayData = {};
    let downloadedCount = 0;
	$: {
		const newData = {};
		const currentDownloaded = Array.isArray(downloadedModels) ? downloadedModels : [];
        downloadedCount = currentDownloaded.filter(m => availableModels.some(am => am.name === m.name)).length;

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
				if (status !== 'downloading' || !progress || !progress.totalBytes || progress.totalBytes <= 0) {
					return 0;
				}
				return Math.min(100, Math.max(0, (progress.downloadedBytes / progress.totalBytes) * 100));
			}

			const getText = () => {
				if (status !== 'downloading' || !progress) return '';
				const downloadedMB = (progress.downloadedBytes / (1024 * 1024)).toFixed(1);
				if (progress.totalBytes && progress.totalBytes > 0) {
					const percentage = getProgressPercent().toFixed(0);
					const totalMB = (progress.totalBytes / (1024 * 1024)).toFixed(1);
					return `${percentage}% (${downloadedMB} / ${totalMB} MB)`;
				} else {
					return `${downloadedMB} MB`;
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
			const initialStatus = {};
			if (Array.isArray(downloadedModels)) {
				downloadedModels.forEach((m) => {
					if (m?.name && availableModels.some(avail => avail.name === m.name)) {
						initialStatus[m.name] = 'complete';
					}
				});
			}
			downloadStatus = initialStatus;
			downloadProgress = {};
		} catch (e) {
			console.error('Error loading transcription configuration:', e);
			configError = `Failed to load transcription configuration: ${e.message || e}`;
		}

		try {
			unlistenStart = await listen('download-start', (event) => {
				const modelName = event.payload;
				if (!modelName || !availableModels.some(m => m.name === modelName)) return;
				downloadStatus = { ...downloadStatus, [modelName]: 'downloading' };
				downloadProgress = { ...downloadProgress, [modelName]: { downloadedBytes: 0, totalBytes: undefined } };
			});

			unlistenProgress = await listen('download-progress', (event) => {
				const { model_name, downloaded_bytes, total_bytes } = event.payload;
				if (!model_name || !availableModels.some(m => m.name === model_name)) return;
				if (downloadStatus[model_name] === 'downloading') {
					downloadProgress = { ...downloadProgress, [model_name]: { downloadedBytes: downloaded_bytes, totalBytes: total_bytes } };
				}
			});

			unlistenComplete = await listen('download-complete', async (event) => {
				const modelName = event.payload;
				if (!modelName || !availableModels.some(m => m.name === modelName)) return;
				const newProgress = { ...downloadProgress }; delete newProgress[modelName]; downloadProgress = newProgress;
				downloadStatus = { ...downloadStatus, [modelName]: 'complete' };
				try {
					downloadedModels = await getDownloadedModels();
					setTranscriptionModelsDownloaded(downloadedModels.length > 0);
				} catch (e) { console.error(`Failed to refresh models after ${modelName} completion:`, e); }
			});

			unlistenError = await listen('download-error', (event) => {
				const payload = event.payload;
				if (!payload || !payload.model_name || !availableModels.some(m => m.name === payload.model_name)) return;
				const modelName = payload.model_name;
				const errorMessage = payload.error_message || 'Unknown error.';
				let finalStatus;
				if (errorMessage.toLowerCase().includes('cancel')) { finalStatus = 'cancelled'; } else { finalStatus = 'error'; alert(`Error downloading ${modelName}: ${errorMessage}`); }
				const newProgress = { ...downloadProgress }; delete newProgress[modelName]; downloadProgress = newProgress;
				downloadStatus = { ...downloadStatus, [modelName]: finalStatus };
			});
		} catch (err) {
			console.error('Failed to attach download event listeners:', err);
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
		try { await openExternal(url); } catch (err) { console.error(`Failed to open external link ${url}:`, err); alert(`Could not open link: ${url}`); }
	}

	async function handleDownload(model) {
		if (isBusy) return;
		const currentStatus = modelDisplayData[model.name]?.status || 'not_downloaded';
		if (['downloading', 'complete', 'cancelling'].includes(currentStatus)) return;
		if (!downloadLocation || downloadLocation.trim() === '') { alert('Please set a valid model download location first.'); return; }
		if (!model?.download_url) { alert(`Model "${model?.name || 'Unknown'}" is missing a download URL.`); return; }
		downloadStatus = { ...downloadStatus, [model.name]: 'downloading' };
		downloadProgress = { ...downloadProgress, [model.name]: { downloadedBytes: 0, totalBytes: undefined } };
		configError = '';
		try { await downloadModel(model, downloadLocation); } catch (err) {
			alert(`Failed to start download for ${model.name}: ${err.message || err}`);
			const newProgress = { ...downloadProgress }; delete newProgress[model.name]; downloadProgress = newProgress;
			downloadStatus = { ...downloadStatus, [model.name]: 'error' };
		}
	}

	async function handleDelete(model) {
		if (isBusy) return;
		if (!model?.name) { alert("Cannot delete model: Missing name."); return; }
		const modelName = model.name;
		configError = '';
		const confirmed = await ask(`Are you sure you want to delete the model "${modelName}"? This will remove it from disk.`, { title: 'Confirm Deletion', type: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' });
		if (!confirmed) return;
		const newStatus = { ...downloadStatus }; delete newStatus[modelName]; const newProgress = { ...downloadProgress }; delete newProgress[modelName];
		downloadStatus = newStatus; downloadProgress = newProgress;
		try {
			await deleteModel(model);
			downloadedModels = await getDownloadedModels();
			setTranscriptionModelsDownloaded(downloadedModels.length > 0);
		} catch (err) {
			alert(`Failed to delete model ${modelName}: ${err.message || err}`);
			try { downloadedModels = await getDownloadedModels(); } catch (refreshErr) { console.error("Failed to refresh models after delete error:", refreshErr); }
		}
	}

	async function handleCancel(modelName) {
		if (!modelName || !availableModels.some(m => m.name === modelName)) return;
		const currentStatus = modelDisplayData[modelName]?.status;
		if (currentStatus !== 'downloading') return;
		downloadStatus = { ...downloadStatus, [modelName]: 'cancelling' };
		configError = '';
		try { await cancelDownload(modelName); } catch (err) {
			alert(`Failed to send cancel request for ${modelName}: ${err.message || err}`);
			downloadStatus = { ...downloadStatus, [modelName]: 'downloading' };
		}
	}
</script>

<div class="flex flex-col h-full">
	{#if configError}
		<p class="text-red-600 bg-red-100 dark:bg-red-900/20 dark:text-red-400 p-3 rounded-md text-sm text-left py-2 mb-4 break-words flex-shrink-0">
			<span class="font-medium">Error:</span> {configError}
		</p>
	{/if}

	<div class="flex-grow space-y-3">
		<div class="border-y border-gray-200 dark:border-gray-700">
			<button on:click={() => isModelsPanelOpen = !isModelsPanelOpen} class="w-full flex justify-between items-center py-3 text-left focus:outline-none">
				<h3 class="block text-sm font-medium text-gray-700 dark:text-gray-200">
					Transcription Models
				</h3>
                <div class="flex items-center">
                    {#if downloadedCount > 0}
                        <span class="text-sm font-medium text-green-600 dark:text-green-400 mr-2">{downloadedCount} downloaded</span>
                    {:else}
                        <span class="text-sm font-medium text-red-600 dark:text-red-400 mr-2">None downloaded</span>
                    {/if}
                    <svg class="w-6 h-6 transform transition-transform duration-200 ease-in-out {isModelsPanelOpen ? 'rotate-180' : ''} text-gray-500 dark:text-gray-400" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
                    </svg>
                </div>
			</button>
		</div>

		{#if isModelsPanelOpen}
			<div class="pt-4 space-y-3">
				<p class="text-sm text-gray-600 dark:text-gray-400 px-1 mb-3">
					Harvey uses <code class="bg-gray-200 dark:bg-gray-700 text-gray-800 dark:text-gray-200 rounded px-1 py-0.5">whisper.cpp</code> for transcription. To transcribe audio or video files, you must first download one of the available models.
				</p>
				{#each availableModels as model (model.name)}
					{@const display = modelDisplayData[model.name] || { status: 'not_downloaded', progressText: '', progressPercent: 0 }}
					{@const status = display.status}
					{@const isDownloadEnabled = !isBusy && downloadLocation && downloadLocation.trim() !== '' && model.download_url}
					{@const isDeleteEnabled = !isBusy}
					{@const isCancelEnabled = status === 'downloading'}
					<div class="bg-white dark:bg-gray-800 p-4 rounded-lg shadow border border-gray-200 dark:border-gray-700 relative overflow-hidden">
						{#if status === 'downloading'}
							<div class="absolute top-0 left-0 bottom-0 bg-blue-100 dark:bg-blue-900/50 bg-opacity-75 transition-all duration-150 ease-linear pointer-events-none" style:width={`${display.progressPercent}%`}></div>
							<div class="absolute top-0 left-0 bottom-0 border-r-2 border-blue-300 dark:border-blue-600 transition-all duration-150 ease-linear pointer-events-none" style:width={`${display.progressPercent}%`}></div>
						{/if}
						<div class="relative z-10">
							<div class="flex justify-between items-start mb-2">
								<p class="text-md font-semibold text-gray-800 dark:text-gray-100 truncate mr-4 pt-1" title={model.name}>
									{model.name}
								</p>
								<div class="flex-shrink-0 flex items-center space-x-2">
									{#if status === 'complete'}
										<button class="btn-delete" on:click={() => handleDelete(model)} disabled={!isDeleteEnabled} title={isDeleteEnabled ? `Delete model ${model.name}` : 'Operation in progress...'}> Delete </button>
									{:else if status === 'downloading' || status === 'cancelling'}
										<span class="text-xs text-blue-700 dark:text-blue-300 font-medium w-36 text-right truncate tabular-nums" title={display.progressText || (status === 'cancelling' ? 'Cancelling...' : 'Starting...')}>
											{#if status === 'cancelling'}Cancelling...{:else}{display.progressText || 'Starting...'}{/if}
										</span>
										<button class="btn-cancel" on:click={() => handleCancel(model.name)} disabled={!isCancelEnabled} title={isCancelEnabled ? 'Cancel download' : 'Cannot cancel'}> Cancel </button>
									{:else if status === 'error'}
										<span class="text-xs text-red-600 dark:text-red-400 font-medium">Error</span>
										<button class="btn-retry" on:click={() => handleDownload(model)} disabled={!isDownloadEnabled} title={!isDownloadEnabled ? 'Set location or download ongoing' : 'Download failed. Click to retry.'}> Retry </button>
									{:else if status === 'cancelled'}
										<span class="text-xs text-gray-500 dark:text-gray-400 font-medium">Cancelled</span>
										<button class="btn-blue-small" on:click={() => handleDownload(model)} disabled={!isDownloadEnabled} title={!isDownloadEnabled ? 'Set location or download ongoing' : 'Download cancelled. Click to try again.'}> Download </button>
									{:else}
										 <button class="btn-blue-small" on:click={() => handleDownload(model)} title={!downloadLocation || downloadLocation.trim() === '' ? 'Set download location first' : !model.download_url ? 'Download URL missing' : isBusy ? 'Operation in progress...' : `Download model ${model.name}`} disabled={!isDownloadEnabled}> Download </button>
									{/if}
								</div>
							</div>
							<div class="text-sm text-gray-600 dark:text-gray-300 space-y-1 mt-1">
								<p><span class="font-medium text-gray-700 dark:text-gray-200">Language:</span> {model.language || '-'} <span class="mx-2 text-gray-300 dark:text-gray-600">|</span> <span class="font-medium text-gray-700 dark:text-gray-200">Size:</span> {model.size || '-'}</p>
								<p><span class="font-medium text-gray-700 dark:text-gray-200">Description:</span> {model.description || '-'}</p>
								{#if model.info_url}
									<p><a href={model.info_url} on:click|preventDefault={() => openLink(model.info_url)} class="text-blue-600 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300 hover:underline text-xs" title="Open model info page in browser"> Learn more... </a></p>
								{/if}
							</div>
							{#if status === 'not_downloaded' && !isDownloadEnabled && !isBusy}
								<p class="text-xs text-orange-600 dark:text-orange-400 mt-2">
									{#if !downloadLocation || downloadLocation.trim() === ''} Set a download location to enable download. {:else if !model.download_url} Download URL missing for this model. {/if}
								</p>
							{/if}
						</div>
					</div>
				{/each}
				{#if availableModels.length === 0}
					<p class="text-center text-gray-500 dark:text-gray-400 pt-4">No models defined in the application.</p>
				{/if}
			</div>
		{/if}
		<DiarizationModelPanel />
	</div>
</div>

<style lang="postcss">
	.btn-blue, .btn-delete, .btn-cancel, .btn-retry, .btn-blue-small {
		@apply px-2.5 py-1.5 border text-sm font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-1 transition-colors duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed;
	}
	.btn-blue {
		@apply border-transparent text-white bg-blue-600 hover:bg-blue-700 focus:ring-blue-500;
	}
    .btn-blue-small, .btn-delete, .btn-cancel, .btn-retry {
		@apply px-2.5 py-1 text-xs;
	}
	.btn-blue-small {
         @apply border-transparent text-white bg-blue-600 hover:bg-blue-700 focus:ring-blue-500;
    }
	.btn-delete {
		@apply border-gray-300 dark:border-gray-600 text-red-700 dark:text-red-300 bg-red-50 dark:bg-red-900/20 hover:bg-red-100 dark:hover:bg-red-800/20 focus:ring-red-400;
	}
	.btn-cancel {
		@apply border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-200 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 focus:ring-indigo-500;
	}
    .btn-retry {
        @apply border-transparent text-white bg-yellow-500 hover:bg-yellow-600 focus:ring-yellow-500;
    }
	.tabular-nums { font-variant-numeric: tabular-nums; }
</style>