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
	import { configStatus, setTranscriptionModelsDownloaded } from '$lib/stores/configStatusStore.js';
	import InstallLogModal from '$lib/components/modals/InstallLogModal.svelte';
	import { v4 as uuidv4 } from 'uuid';

	export let downloadLocation = '';
	export let isBusy = false;

	let downloadedModels = [];
	let configError = '';
	let downloadStatus = {};
	let downloadProgress = {};

	// --- State variables for Modal binding ---
	let showLogModal = false;
	let modalLogs = [];
	let isDownloading = false;

	const WHISPER_CPP_INFO_URL = 'https://huggingface.co/ggerganov/whisper.cpp';
	const HUGGING_FACE_BASE = 'https://huggingface.co/ggerganov/whisper.cpp/resolve/main';

	const availableModels = [
		{ name: 'ggml-large-v3', language: 'Multilingual', size: '2.9 GiB', description: 'Latest and most accurate multilingual model.', download_url: `${HUGGING_FACE_BASE}/ggml-large-v3.bin`, info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-large-v3.bin` },
		{ name: 'ggml-large-v3-turbo', language: 'Multilingual', size: '1.5 GiB', description: 'Optimized for speed, great for real-time transcription.', download_url: `${HUGGING_FACE_BASE}/ggml-large-v3-turbo.bin`, info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-large-v3-turbo.bin` },
		{ name: 'ggml-large-v3-turbo-q5_0', language: 'Multilingual', size: '1.1 GiB', description: 'Quantized version of turbo model. Good balance.', download_url: `${HUGGING_FACE_BASE}/ggml-large-v3-turbo-q5_0.bin`, info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-large-v3-turbo-q5_0.bin` },
		{ name: 'ggml-medium.en', language: 'English-only', size: '1.5 GiB', description: 'Highest accuracy for English-only applications.', download_url: `${HUGGING_FACE_BASE}/ggml-medium.en.bin`, info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-medium.en.bin` },
		{ name: 'ggml-medium', language: 'Multilingual', size: '1.5 GiB', description: 'High accuracy across multiple languages.', download_url: `${HUGGING_FACE_BASE}/ggml-medium.bin`, info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-medium.bin` },
		{ name: 'ggml-small.en', language: 'English-only', size: '466 MiB', description: 'Excellent balance of speed and accuracy for English.', download_url: `${HUGGING_FACE_BASE}/ggml-small.en.bin`, info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-small.en.bin` },
		{ name: 'ggml-small', language: 'Multilingual', size: '466 MiB', description: 'Excellent balance for multilingual use.', download_url: `${HUGGING_FACE_BASE}/ggml-small.bin`, info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-small.bin` },
		{ name: 'ggml-base.en', language: 'English-only', size: '142 MiB', description: 'Fast and lightweight for English.', download_url: `${HUGGING_FACE_BASE}/ggml-base.en.bin`, info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-base.en.bin` },
		{ name: 'ggml-base', language: 'Multilingual', size: '142 MiB', description: 'Fast and lightweight for multilingual use.', download_url: `${HUGGING_FACE_BASE}/ggml-base.bin`, info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-base.bin` },
		{ name: 'ggml-tiny.en', language: 'English-only', size: '75 MiB', description: 'Smallest and fastest for English, for limited resources.', download_url: `${HUGGING_FACE_BASE}/ggml-tiny.en.bin`, info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-tiny.en.bin` },
		{ name: 'ggml-tiny', language: 'Multilingual', size: '75 MiB', description: 'Smallest and fastest multilingual model.', download_url: `${HUGGING_FACE_BASE}/ggml-tiny.bin`, info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-tiny.bin` },
	];

	// --- Marketplace / Search View State ---
	let searchQuery = '';
	let isFetchingModels = false;
	let hasFetched = false;
	let autoFetchTriggered = false;

	let modelDisplayData = {};
	let downloadedCount = 0;

	// Update display data reactively
	$: {
		const newData = {};
		const currentDownloaded = Array.isArray(downloadedModels) ? downloadedModels : [];
		downloadedCount = currentDownloaded.filter((m) => availableModels.some((am) => am.name === m.name)).length;

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
			};

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

	// Combined list logic:
	// If not fetched: show ONLY downloaded models (plus those currently downloading/cancelling/error)
	// If fetched: show ALL available models (filtered by search)
	// Always sort downloaded to top
	$: displayedModels = (() => {
		let baseList = [];
		if (!hasFetched) {
			// Show only models that are downloaded OR have active state (downloading/error etc)
			baseList = availableModels.filter((m) => {
				const status = modelDisplayData[m.name]?.status;
				return status && status !== 'not_downloaded';
			});
		} else {
			// Show all models
			baseList = availableModels;
		}

		// Filter by search query
		if (searchQuery.trim() !== '') {
			const q = searchQuery.toLowerCase();
			baseList = baseList.filter(
				(m) =>
					m.name.toLowerCase().includes(q) ||
					m.description.toLowerCase().includes(q) ||
					m.language.toLowerCase().includes(q)
			);
		}

		// Sort: Downloaded/Active first, then alphabetical
		return baseList.sort((a, b) => {
			const statusA = modelDisplayData[a.name]?.status;
			const statusB = modelDisplayData[b.name]?.status;
			const aActive = statusA && statusA !== 'not_downloaded';
			const bActive = statusB && statusB !== 'not_downloaded';

			if (aActive && !bActive) return -1;
			if (!aActive && bActive) return 1;
			return a.name.localeCompare(b.name);
		});
	})();

	// Filter logic triggers auto-fetch if searching
	$: {
		if (searchQuery.trim() === '') {
			autoFetchTriggered = false;
		} else {
			if (!hasFetched && !isFetchingModels && !autoFetchTriggered) {
				autoFetchTriggered = true;
				handleRefreshModels();
			}
		}
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
			console.error('Error loading transcription configuration:', e);
			configError = `Failed to load transcription configuration: ${e.message || e}`;
		}

		try {
			unlistenStart = await listen('download-start', (event) => {
				const modelName = event.payload;
				if (!modelName || !availableModels.some((m) => m.name === modelName)) return;
				downloadStatus = { ...downloadStatus, [modelName]: 'downloading' };
				downloadProgress = { ...downloadProgress, [modelName]: { downloadedBytes: 0, totalBytes: undefined } };
			});

			unlistenProgress = await listen('download-progress', (event) => {
				const { model_name, downloaded_bytes, total_bytes } = event.payload;
				if (!model_name || !availableModels.some((m) => m.name === model_name)) return;
				if (downloadStatus[model_name] === 'downloading') {
					downloadProgress = {
						...downloadProgress,
						[model_name]: { downloadedBytes: downloaded_bytes, totalBytes: total_bytes },
					};
				}
			});

			unlistenComplete = await listen('download-complete', async (event) => {
				const modelName = event.payload;
				if (!modelName || !availableModels.some((m) => m.name === modelName)) return;
				const newProgress = { ...downloadProgress };
				delete newProgress[modelName];
				downloadProgress = newProgress;
				downloadStatus = { ...downloadStatus, [modelName]: 'complete' };
				try {
					downloadedModels = await getDownloadedModels();
					setTranscriptionModelsDownloaded(downloadedModels.length > 0);
				} catch (e) {
					console.error(`Failed to refresh models after ${modelName} completion:`, e);
				}
			});

			unlistenError = await listen('download-error', (event) => {
				const payload = event.payload;
				if (!payload || !payload.model_name || !availableModels.some((m) => m.name === payload.model_name))
					return;
				const modelName = payload.model_name;
				const errorMessage = payload.error_message || 'Unknown error.';
				let finalStatus;
				if (errorMessage.toLowerCase().includes('cancel')) {
					finalStatus = 'cancelled';
				} else {
					finalStatus = 'error';
					alert(`Error downloading ${modelName}: ${errorMessage}`);
				}
				const newProgress = { ...downloadProgress };
				delete newProgress[modelName];
				downloadProgress = newProgress;
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

	async function handleRefreshModels() {
		if (isFetchingModels) return;
		isFetchingModels = true;
		setTimeout(() => {
			hasFetched = true;
			isFetchingModels = false;
		}, 600);
	}

	async function openLink(url) {
		if (!url) return;
		try {
			await openExternal(url);
		} catch (err) {
			console.error(`Failed to open external link ${url}:`, err);
			alert(`Could not open link: ${url}`);
		}
	}

	async function handleDownload(model) {
		if (isBusy) return;
		const currentStatus = modelDisplayData[model.name]?.status || 'not_downloaded';
		if (['downloading', 'complete', 'cancelling'].includes(currentStatus)) return;
		if (!downloadLocation || downloadLocation.trim() === '') {
			alert('Please set a valid model download location first.');
			return;
		}
		if (!model?.download_url) {
			alert(`Model "${model?.name || 'Unknown'}" is missing a download URL.`);
			return;
		}
		downloadStatus = { ...downloadStatus, [model.name]: 'downloading' };
		downloadProgress = { ...downloadProgress, [model.name]: { downloadedBytes: 0, totalBytes: undefined } };
		configError = '';
		try {
			await downloadModel(model, downloadLocation);
		} catch (err) {
			alert(`Failed to start download for ${model.name}: ${err.message || err}`);
			const newProgress = { ...downloadProgress };
			delete newProgress[model.name];
			downloadProgress = newProgress;
			downloadStatus = { ...downloadStatus, [model.name]: 'error' };
		}
	}

	async function handleDelete(model) {
		if (isBusy) return;
		if (!model?.name) {
			alert("Cannot delete model: Missing name.");
			return;
		}
		const modelName = model.name;
		configError = '';
		const confirmed = await ask(`Are you sure you want to delete the model "${modelName}"? This will remove it from disk.`, {
			title: 'Confirm Deletion',
			type: 'warning',
			okLabel: 'Delete',
			cancelLabel: 'Cancel',
		});
		if (!confirmed) return;
		const newStatus = { ...downloadStatus };
		delete newStatus[modelName];
		const newProgress = { ...downloadProgress };
		delete newProgress[modelName];
		downloadStatus = newStatus;
		downloadProgress = newProgress;
		try {
			await deleteModel(model);
			downloadedModels = await getDownloadedModels();
			setTranscriptionModelsDownloaded(downloadedModels.length > 0);
		} catch (err) {
			alert(`Failed to delete model ${modelName}: ${err.message || err}`);
			try {
				downloadedModels = await getDownloadedModels();
			} catch (refreshErr) {
				console.error('Failed to refresh models after delete error:', refreshErr);
			}
		}
	}

	async function handleCancel(modelName) {
		if (!modelName || !availableModels.some((m) => m.name === modelName)) return;
		const currentStatus = modelDisplayData[modelName]?.status;
		if (currentStatus !== 'downloading') return;
		downloadStatus = { ...downloadStatus, [modelName]: 'cancelling' };
		configError = '';
		try {
			await cancelDownload(modelName);
		} catch (err) {
			alert(`Failed to send cancel request for ${modelName}: ${err.message || err}`);
			downloadStatus = { ...downloadStatus, [modelName]: 'downloading' };
		}
	}
</script>

<div class="flex flex-col h-full overflow-y-auto p-1">
	<InstallLogModal
		bind:showModal={showLogModal}
		logs={modalLogs}
		isInstalling={isDownloading}
		title="Downloading Transcription Model"
		inProgressText="Downloading..."
	/>
	{#if configError}
		<p
			class="text-red-600 bg-red-100 dark:bg-red-900/20 dark:text-red-400 p-3 rounded-md text-sm text-left py-2 mb-4 break-words flex-shrink-0"
		>
			<span class="font-medium">Error:</span>
			{configError}
		</p>
	{/if}

	<div class="flex flex-col space-y-3 h-full">
		<div class="flex-shrink-0 mb-2">
			<p class="text-sm text-gray-600 dark:text-gray-400 px-1 mb-1">
				Harvey uses <a
					href="https://github.com/ggml-org/whisper.cpp"
					target="_blank"
					rel="noopener noreferrer"
					on:click|preventDefault={() => openLink('https://github.com/ggml-org/whisper.cpp')}
					class="text-blue-600 dark:text-blue-400 hover:underline">whisper.cpp</a
				> for transcription.
			</p>
			{#if !$configStatus.python_libraries_installed}
				<p class="text-orange-600 dark:text-orange-400 text-sm px-1">
					Please install the required Python libraries first to enable model downloads.
				</p>
			{/if}
		</div>

		<div class="flex flex-col space-y-3 flex-grow">
			<!-- Search Bar -->
			<div class="flex space-x-2 flex-shrink-0 p-0.5">
				<div class="relative flex-grow">
					<div class="absolute inset-y-0 left-0 pl-2.5 flex items-center pointer-events-none">
						<svg class="h-4 w-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"
							><path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
							></path></svg
						>
					</div>
					<input
						type="text"
						bind:value={searchQuery}
						class="input w-full"
						style="padding-left: 2.25rem;"
						placeholder="Browse all available models (e.g. 'large', 'turbo')..."
						autocomplete="off"
						autocorrect="off"
						autocapitalize="off"
						spellcheck="false"
					/>
				</div>
			</div>

			<div class="border dark:border-gray-700 rounded-md flex-grow bg-gray-50 dark:bg-gray-800/50 p-2">
				<div class="space-y-2">
					{#each displayedModels as model (model.name)}
						{@const display = modelDisplayData[model.name] || {
							status: 'not_downloaded',
							progressText: '',
							progressPercent: 0,
						}}
						{@const status = display.status}
						{@const isDownloadEnabled =
							!isBusy &&
							downloadLocation &&
							downloadLocation.trim() !== '' &&
							model.download_url &&
							$configStatus.python_libraries_installed}
						{@const isDeleteEnabled = !isBusy}
						{@const isCancelEnabled = status === 'downloading'}

						<div
							class="bg-white dark:bg-gray-800 border dark:border-gray-700 p-3 rounded-md shadow-sm flex flex-col hover:border-blue-400 transition-colors relative overflow-hidden"
						>
							{#if status === 'downloading'}
								<div
									class="absolute top-0 left-0 bottom-0 bg-blue-100 dark:bg-blue-900/50 bg-opacity-75 transition-all duration-150 ease-linear pointer-events-none"
									style:width={`${display.progressPercent}%`}
								></div>
								<div
									class="absolute top-0 left-0 bottom-0 border-r-2 border-blue-300 dark:border-blue-600 transition-all duration-150 ease-linear pointer-events-none"
									style:width={`${display.progressPercent}%`}
								></div>
							{/if}

							<div class="relative z-10 flex justify-between items-start">
								<div class="flex flex-col min-w-0 pr-4">
									<div class="flex items-center space-x-2">
										<span class="font-semibold text-gray-800 dark:text-gray-200 truncate" title={model.name}>
											{model.name}
										</span>
										<button
											class="text-gray-400 hover:text-blue-500 dark:text-gray-500 dark:hover:text-blue-400 focus:outline-none p-0.5 rounded hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
											title="View on GitHub"
											on:click|stopPropagation={() => openLink(model.info_url)}
										>
											<svg
												xmlns="http://www.w3.org/2000/svg"
												width="14"
												height="14"
												fill="currentColor"
												class="bi bi-box-arrow-up-right"
												viewBox="0 0 16 16"
											>
												<path
													fill-rule="evenodd"
													d="M8.636 3.5a.5.5 0 0 0-.5-.5H1.5A1.5 1.5 0 0 0 0 4.5v10A1.5 1.5 0 0 0 1.5 16h10a1.5 1.5 0 0 0 1.5-1.5V7.864a.5.5 0 0 0-1 0V14.5a.5.5 0 0 1-.5.5h-10a.5.5 0 0 1-.5-.5v-10a.5.5 0 0 1 .5-.5h6.636a.5.5 0 0 0 .5-.5"
												/>
												<path
													fill-rule="evenodd"
													d="M16 .5a.5.5 0 0 0-.5-.5h-5a.5.5 0 0 0 0 1h3.793L6.146 9.146a.5.5 0 1 0 .708.708L15 1.707V5.5a.5.5 0 0 0 1 0z"
												/>
											</svg>
										</button>
										{#if status === 'complete'}
											<span
												class="px-1.5 py-0.5 rounded text-[10px] font-medium bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300"
												>Installed</span
											>
										{/if}
									</div>
									<div class="text-xs text-gray-500 dark:text-gray-400 mt-0.5 flex flex-col space-y-0.5">
										<span class="truncate">{model.description}</span>
										<span class="flex items-center text-gray-400 space-x-2">
											<span>{model.language}</span>
											<span>&bull;</span>
											<span>{model.size}</span>
										</span>
									</div>
								</div>

								<div class="flex-shrink-0 flex items-center space-x-2 pt-1">
									{#if status === 'complete'}
										<button
											class="btn-delete"
											on:click={() => handleDelete(model)}
											disabled={!isDeleteEnabled}
											title="Delete model">Delete</button
										>
									{:else if status === 'downloading' || status === 'cancelling'}
										<div class="flex flex-col items-end">
											<span class="text-[10px] text-blue-700 dark:text-blue-300 font-medium tabular-nums mb-1">
												{#if status === 'cancelling'}Cancelling...{:else}{display.progressText ||
														'Starting...'}{/if}
											</span>
											<button
												class="btn-cancel"
												on:click={() => handleCancel(model.name)}
												disabled={!isCancelEnabled}
												title="Cancel download">Cancel</button
											>
										</div>
									{:else if status === 'error'}
										<button
											class="btn-retry"
											on:click={() => handleDownload(model)}
											disabled={!isDownloadEnabled}
											title="Retry download">Retry</button
										>
									{:else if status === 'cancelled'}
										<button
											class="btn-blue-small"
											on:click={() => handleDownload(model)}
											disabled={!isDownloadEnabled}>Download</button
										>
									{:else}
										<button
											class="btn-blue-small"
											on:click={() => handleDownload(model)}
											disabled={!isDownloadEnabled}
											title={!isDownloadEnabled ? 'Configure download location first' : 'Download model'}
											>Download</button
										>
									{/if}
								</div>
							</div>
						</div>
					{/each}
				</div>

				{#if !hasFetched && searchQuery.trim() === ''}
					<div class="py-4 flex justify-center">
						<button
							on:click={handleRefreshModels}
							class="btn-blue-small px-4 py-2 text-sm"
							title="Refresh available transcription models"
						>
							{#if isFetchingModels}
								Loading available models...
							{:else}
								List models from HuggingFace
							{/if}
						</button>
					</div>
				{/if}

				{#if hasFetched && displayedModels.length === 0 && searchQuery.trim() !== ''}
					<div class="flex flex-col items-center justify-center h-20 text-gray-500">
						<p>No models found matching "{searchQuery}".</p>
					</div>
				{/if}
			</div>
		</div>
	</div>
</div>

<style lang="postcss">
	.input {
		@apply bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md px-2.5 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500;
	}
	.btn-blue-small,
	.btn-delete,
	.btn-cancel,
	.btn-retry {
		@apply px-3 py-1.5 border text-xs font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-1 transition-colors duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed;
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
</style>
