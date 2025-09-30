<!-- src/lib/components/welcome/Configure.svelte -->
<script>
	import { onMount, onDestroy } from 'svelte';
	import { open } from '@tauri-apps/plugin-dialog';
	import { ask } from '@tauri-apps/plugin-dialog';
	import { listen } from '@tauri-apps/api/event';
	import { open as openExternal } from '@tauri-apps/plugin-shell';
	// --- UPDATED: Import cloud config actions ---
	import {
		downloadModel,
		deleteModel,
		saveDownloadLocation,
		getDownloadedModels,
		getDownloadLocation,
		cancelDownload,
		moveModelsAndUpdateLocation
	} from '$lib/services/configureActions';

	// --- Tab State ---
	let activeTab = 'local'; // 'local' or 'cloud'

	// --- State Variables (Local Tab) ---
	let downloadLocation = '';
	let downloadedModels = [];
	let isLoadingConfig = true; // General loading state (covers local and initial cloud load)
	let configError = ''; // General error state (initially for local, might merge later)
	let downloadStatus = {};
	let downloadProgress = {};
	let isMovingModels = false;
	let statusMessage = ''; // General status message

	// --- Define Available Local Models ---
	const WHISPER_CPP_INFO_URL = 'https://github.com/ggerganov/whisper.cpp';
	const HUGGING_FACE_BASE = 'https://huggingface.co/ggerganov/whisper.cpp/resolve/main';

	const availableModels = [
		// --- Best Quality ---
		{ name: 'ggml-large-v3', language: 'Multilingual', size: '2.9 GiB', description: 'Latest and most accurate multilingual model.', download_url: `${HUGGING_FACE_BASE}/ggml-large-v3.bin`, info_url: WHISPER_CPP_INFO_URL },
		{ name: 'ggml-large-v3-turbo', language: 'Multilingual', size: '1.5 GiB', description: 'Optimized for speed, great for real-time transcription.', download_url: `${HUGGING_FACE_BASE}/ggml-large-v3-turbo.bin`, info_url: WHISPER_CPP_INFO_URL },
		{ name: 'ggml-medium.en', language: 'English-only', size: '1.5 GiB', description: 'Highest accuracy for English-only applications.', download_url: `${HUGGING_FACE_BASE}/ggml-medium.en.bin`, info_url: WHISPER_CPP_INFO_URL },
		{ name: 'ggml-medium', language: 'Multilingual', size: '1.5 GiB', description: 'High accuracy across multiple languages.', download_url: `${HUGGING_FACE_BASE}/ggml-medium.bin`, info_url: WHISPER_CPP_INFO_URL },

		// --- Better Quality (Best All-Rounders) ---
		{ name: 'ggml-small.en', language: 'English-only', size: '466 MiB', description: 'Excellent balance of speed and accuracy for English.', download_url: `${HUGGING_FACE_BASE}/ggml-small.en.bin`, info_url: WHISPER_CPP_INFO_URL },
		{ name: 'ggml-small', language: 'Multilingual', size: '466 MiB', description: 'Excellent balance for multilingual use.', download_url: `${HUGGING_FACE_BASE}/ggml-small.bin`, info_url: WHISPER_CPP_INFO_URL },
		
		// --- Good Quality (Fast & Lightweight) ---
		{ name: 'ggml-base.en', language: 'English-only', size: '142 MiB', description: 'Fast and lightweight for English.', download_url: `${HUGGING_FACE_BASE}/ggml-base.en.bin`, info_url: WHISPER_CPP_INFO_URL },
		{ name: 'ggml-base', language: 'Multilingual', size: '142 MiB', description: 'Fast and lightweight for multilingual use.', download_url: `${HUGGING_FACE_BASE}/ggml-base.bin`, info_url: WHISPER_CPP_INFO_URL },
		{ name: 'ggml-tiny.en', language: 'English-only', size: '75 MiB', description: 'Smallest and fastest for English, for limited resources.', download_url: `${HUGGING_FACE_BASE}/ggml-tiny.en.bin`, info_url: WHISPER_CPP_INFO_URL },
		{ name: 'ggml-tiny', language: 'Multilingual', size: '75 MiB', description: 'Smallest and fastest multilingual model.', download_url: `${HUGGING_FACE_BASE}/ggml-tiny.bin`, info_url: WHISPER_CPP_INFO_URL },
	];

	// --- Computed State ---
	export let isBusy = false; // Exported busy state (primarily for local model downloads/moves)
	$: isAnyModelDownloading = Object.values(downloadStatus).includes('downloading');
	// Make busy state dependent on local operations OR cloud saving/loading
	$: isBusy = isAnyModelDownloading || isMovingModels;


	// --- Derived Reactive Data for Local Model Display ---
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
			const getText = (modelName) => {
				const progress = downloadProgress[modelName];
				if (status !== 'downloading' || !progress) return '';
				const downloadedMB = (progress.downloadedBytes / (1024 * 1024)).toFixed(1);
				if (progress.totalBytes && progress.totalBytes > 0) {
					const percentage = Math.min(100, Math.max(0, (progress.downloadedBytes / progress.totalBytes) * 100)).toFixed(0);
					const totalMB = (progress.totalBytes / (1024 * 1024)).toFixed(1);
					return `${percentage}% (${downloadedMB} / ${totalMB} MB)`;
				} else {
					return `${downloadedMB} MB`;
				}
			};
			const progressText = getText(name);
			newData[name] = { status, progressText };
		}
		modelDisplayData = newData;
	}

	// --- Event Listeners (Local Models) ---
	let unlistenStart = null;
	let unlistenProgress = null;
	let unlistenComplete = null;
	let unlistenError = null;

	// --- Lifecycle ---
	onMount(async () => {
		isLoadingConfig = true; // Start general loading
		configError = '';
		statusMessage = '';

		// --- Load Local Config ---
		try {
			console.log('Configure: Loading local configuration...');
			downloadLocation = await getDownloadLocation();
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
			console.log('Configure: Local configuration loaded.');
		} catch (e) {
			console.error('Error loading local configuration:', e);
			configError = `Failed to load local configuration: ${e.message || e}`;
		}

		isLoadingConfig = false; // Finish general loading

		// --- Setup Local Model Download Listeners ---
		try {
			unlistenStart = await listen('download-start', (event) => {
				const modelName = event.payload;
				if (!modelName || !availableModels.some(m => m.name === modelName)) return;
				console.log(`[Event] download-start received for ${modelName}`);
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
				console.log(`[Event] download-complete received for ${modelName}`);
				const newProgress = { ...downloadProgress }; delete newProgress[modelName]; downloadProgress = newProgress;
				downloadStatus = { ...downloadStatus, [modelName]: 'complete' };
				try {
					console.log(`Refreshing downloaded models list after ${modelName} complete...`);
					downloadedModels = await getDownloadedModels();
				} catch (e) { console.error(`Failed to refresh models after ${modelName} completion:`, e); }
			});

			unlistenError = await listen('download-error', (event) => {
				const payload = event.payload;
				if (!payload || !payload.model_name || !availableModels.some(m => m.name === payload.model_name)) return;
				const modelName = payload.model_name;
				const errorMessage = payload.error_message || 'Unknown error.';
				console.log(`[Event] download-error received for ${modelName}: ${errorMessage}`);
				let finalStatus;
				if (errorMessage.toLowerCase().includes('cancel')) { finalStatus = 'cancelled'; console.log(`Setting status for ${modelName} to cancelled.`); } else { finalStatus = 'error'; console.log(`Setting status for ${modelName} to error.`); alert(`Error downloading ${modelName}: ${errorMessage}`); }
				const newProgress = { ...downloadProgress }; delete newProgress[modelName]; downloadProgress = newProgress;
				downloadStatus = { ...downloadStatus, [modelName]: finalStatus };
			});
			console.log('Download event listeners attached.');
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
		console.log('Download event listeners detached.');
	});

	// --- Helper Functions ---
	function calculateProgress(modelName) {
		// (No changes)
		const progress = downloadProgress[modelName];
		const status = modelDisplayData[modelName]?.status;
		if (status !== 'downloading' || !progress || !progress.totalBytes || progress.totalBytes <= 0) {
			return 0;
		}
		return Math.min(100, Math.max(0, (progress.downloadedBytes / progress.totalBytes) * 100));
	}
	async function openLink(url) {
		// (No changes)
		if (!url) return;
		try { await openExternal(url); } catch (err) { console.error(`Failed to open external link ${url}:`, err); alert(`Could not open link: ${url}`); }
	}

	// --- Action Handlers (Local Tab) ---
	async function pickDownloadLocation() {
		// (No changes)
		if (isBusy) { console.log('Browse disabled: Operation in progress (download/move).'); return; }
		try {
			const selected = await open({ multiple: false, directory: true, title: 'Select Model Download Location', defaultPath: downloadLocation || undefined });
			if (!selected || typeof selected !== 'string') { console.log('Location selection cancelled.'); return; }
			const newLocation = selected;
			if (newLocation === downloadLocation) { console.log('Selected location is the same as the current one.'); statusMessage = 'Selected location is the same as current.'; setTimeout(() => (statusMessage = ''), 3000); return; }
			let currentModelsInOldLocation = [];
			try { currentModelsInOldLocation = await getDownloadedModels(); downloadedModels = currentModelsInOldLocation; } catch (refreshErr) { console.error("Failed to refresh downloaded models before move confirmation:", refreshErr); configError = `Error checking current models: ${refreshErr.message || refreshErr}`; return; }
			const modelsToMove = currentModelsInOldLocation.length > 0 && downloadLocation;
			let confirmed = true;
			if (modelsToMove) { confirmed = await ask(`Change download location to:\n${newLocation}\n\nThis will move ${currentModelsInOldLocation.length} downloaded model(s) from the current location to the new one. Proceed?`, { title: 'Confirm Location Change & Move', type: 'warning', okLabel: 'Yes, Move Files', cancelLabel: 'Cancel' }); } else { confirmed = await ask(`Set download location to:\n${newLocation}\n\nNew models will be downloaded here.`, { title: 'Confirm Location Change', type: 'info', okLabel: 'Confirm', cancelLabel: 'Cancel' }); }
			if (!confirmed) { console.log('Location change cancelled by user.'); statusMessage = 'Location change cancelled.'; setTimeout(() => (statusMessage = ''), 3000); return; }
			console.log('Proceeding with location change' + (modelsToMove ? ' and model move...' : '...'));
			isMovingModels = modelsToMove; statusMessage = modelsToMove ? 'Moving models and updating location...' : 'Updating location...'; configError = '';
			try {
				if (modelsToMove) { await moveModelsAndUpdateLocation(newLocation); statusMessage = 'Download location updated and models moved successfully!'; } else { await saveDownloadLocation(newLocation); statusMessage = 'Download location updated successfully!'; }
				downloadLocation = newLocation; console.log('Successfully updated download location setting to:', newLocation);
				 try { downloadedModels = await getDownloadedModels(); } catch (finalRefreshErr) { console.error("Failed to refresh models after location change:", finalRefreshErr); }
			} catch (err) { console.error('Error during location change / model move:', err); configError = `Error changing location: ${err.message || err}`; statusMessage = ''; } finally { isMovingModels = false; setTimeout(() => { if (!configError && statusMessage.startsWith('Download location updated')) { statusMessage = ''; } }, 5000); }
		} catch (err) { console.error('Error picking download location dialog:', err); configError = `Error selecting directory: ${err.message || err}`; isMovingModels = false; }
	}
	async function handleDownload(model) {
		// (No changes)
		if (isBusy) { console.log(`Download for ${model.name} blocked: Operation in progress.`); return; }
		const currentStatus = modelDisplayData[model.name]?.status || 'not_downloaded';
		if (currentStatus === 'downloading' || currentStatus === 'complete' || currentStatus === 'cancelling') { console.log(`Download for ${model.name} blocked: Status is ${currentStatus}.`); return; }
		if (!downloadLocation || downloadLocation.trim() === '') { alert('Please set a valid model download location first.'); return; }
		if (!model?.download_url) { alert(`Model "${model?.name || 'Unknown'}" is missing a download URL.`); return; }
		console.log(`Starting download for ${model.name}...`);
		downloadStatus = { ...downloadStatus, [model.name]: 'downloading' };
		downloadProgress = { ...downloadProgress, [model.name]: { downloadedBytes: 0, totalBytes: undefined } };
		configError = '';
		try { await downloadModel(model, downloadLocation); console.log(`Download command invoked for ${model.name}`); } catch (err) { console.error(`Error invoking download command for ${model.name}:`, err); alert(`Failed to start download for ${model.name}: ${err.message || err}`); const newProgress = { ...downloadProgress }; delete newProgress[model.name]; downloadProgress = newProgress; downloadStatus = { ...downloadStatus, [model.name]: 'error' }; }
	}
	async function handleDelete(model) {
		// (No changes)
		if (isBusy) { console.log("Delete disabled: Operation in progress."); return; }
		if (!model?.name) { alert("Cannot delete model: Missing name."); return; }
		const modelName = model.name; configError = '';
		const confirmed = await ask(`Are you sure you want to delete the model "${modelName}"? This will remove it from disk.`, { title: 'Confirm Deletion', type: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' });
		if (!confirmed) { console.log(`Deletion cancelled for ${modelName}`); return; }
		console.log(`Deletion confirmed for ${modelName}. Proceeding...`);
		const newStatus = { ...downloadStatus }; delete newStatus[modelName]; const newProgress = { ...downloadProgress }; delete newProgress[modelName];
		downloadStatus = newStatus; downloadProgress = newProgress;
		try { await deleteModel(model); console.log(`Delete command invoked for ${modelName}`); downloadedModels = await getDownloadedModels(); } catch (err) { console.error(`Error deleting model ${modelName}:`, err); alert(`Failed to delete model ${modelName}: ${err.message || err}`); try { downloadedModels = await getDownloadedModels(); } catch (refreshErr) { console.error("Failed to refresh models after delete error:", refreshErr); } }
	}
	async function handleCancel(modelName) {
		// (No changes)
		if (!modelName || !availableModels.some(m => m.name === modelName)) return;
		const currentStatus = modelDisplayData[modelName]?.status;
		if (currentStatus !== 'downloading') { console.log(`Cannot cancel ${modelName}, status is ${currentStatus}`); return; }
		console.log(`Requesting cancellation for ${modelName}...`);
		downloadStatus = { ...downloadStatus, [modelName]: 'cancelling' }; configError = '';
		try { await cancelDownload(modelName); console.log(`Cancel command invoked for ${modelName}`); } catch (err) { console.error(`Error invoking cancel command for ${modelName}:`, err); alert(`Failed to send cancel request for ${modelName}: ${err.message || err}`); downloadStatus = { ...downloadStatus, [modelName]: 'downloading' }; }
	}
</script>

<!-- Main Container -->
<div class="p-6 flex flex-col h-full bg-gray-50">
	<h2 class="text-xl font-semibold mb-4 text-gray-800 flex-shrink-0">Configure Transcription</h2>

	<!-- Tab Navigation -->
	<div class="border-b border-gray-200 mb-6 flex-shrink-0">
		<nav class="-mb-px flex space-x-8" aria-label="Tabs">
			<button
				on:click={() => activeTab = 'local'}
				class="whitespace-nowrap py-3 px-1 border-b-2 font-medium text-sm transition-colors duration-150 ease-in-out focus:outline-none"
				class:border-blue-500={activeTab === 'local'}
				class:text-blue-600={activeTab === 'local'}
				class:border-transparent={activeTab !== 'local'}
				class:text-gray-500={activeTab !== 'local'}
				class:hover:text-gray-700={activeTab !== 'local'}
				class:hover:border-gray-300={activeTab !== 'local'}
				aria-current={activeTab === 'local' ? 'page' : undefined}
			>
				Local Transcription
			</button>
		</nav>
	</div>

	<!-- Tab Content Area -->
	<div class="flex-grow min-h-0 overflow-y-auto pr-2 -mr-2">

		<!-- Local Transcription Tab Content -->
		{#if activeTab === 'local'}
			<div class="flex flex-col h-full">
				{#if isLoadingConfig}
					<p class="text-gray-500 text-center py-4">Loading configuration...</p>
				{:else if configError}
					<p class="text-red-600 bg-red-100 p-3 rounded-md text-sm text-left py-2 mb-4 break-words flex-shrink-0">
						<span class="font-medium">Error:</span> {configError}
					</p>
				{/if}

				<!-- Download Location Section -->
				<div class="mb-6 flex-shrink-0">
					<label for="download-location-input" class="block text-sm font-medium text-gray-700 mb-1">
						Local Model Download Location
					</label>
					<div class="flex items-center space-x-2">
						<input
							id="download-location-input"
							type="text"
							bind:value={downloadLocation}
							class="flex-grow shadow-sm focus:ring-blue-500 focus:border-blue-500 block w-full sm:text-sm border-gray-300 rounded-md bg-white text-gray-700 cursor-default"
							readonly
							placeholder="Set a location..."
							title={downloadLocation || 'No location set'}
						/>
						<button
							type="button"
							class="btn-blue flex-shrink-0"
							on:click={pickDownloadLocation}
							disabled={isBusy}
							title={isBusy ? 'Operation in progress...' : 'Select model download folder'}
						>
							{#if isMovingModels} Moving... {:else} Browse {/if}
						</button>
					</div>
					{#if isBusy}
						<p class="text-xs text-gray-500 mt-1">
							{#if isMovingModels} Moving models... {:else if isAnyModelDownloading} Download in progress... {/if}
							(Cannot change location now)
						</p>
					{/if}
					{#if statusMessage}
						<p class="text-xs text-indigo-600 mt-1">{statusMessage}</p>
					{/if}
				</div>

				<!-- Model List Section -->
				<div class="flex-grow space-y-3">
					<div class="sticky top-0 z-10 mb-3 border-b border-gray-200 bg-white pb-2 pt-1">
						<h3 class="text-lg font-medium text-gray-900">
							Available Models (whisper.cpp / ggml)
						</h3>
					</div>

					{#if isLoadingConfig}
						<!-- Placeholder if needed -->
					{:else}
						{#each availableModels as model (model.name)}
							{@const display = modelDisplayData[model.name] || { status: 'not_downloaded', progressText: '' }}
                            {@const status = display.status}
                            {@const isDownloadEnabled = !isBusy && downloadLocation && downloadLocation.trim() !== '' && model.download_url}
                            {@const isDeleteEnabled = !isBusy}
                            {@const isCancelEnabled = status === 'downloading'}
                            <div class="bg-white p-4 rounded-lg shadow border border-gray-200 relative overflow-hidden">
                                {#if status === 'downloading'}
                                    {@const progressPercent = calculateProgress(model.name)}
                                    <div class="absolute top-0 left-0 bottom-0 bg-blue-100 bg-opacity-75 transition-all duration-150 ease-linear pointer-events-none" style:width={`${progressPercent}%`}></div>
                                    <div class="absolute top-0 left-0 bottom-0 border-r-2 border-blue-300 transition-all duration-150 ease-linear pointer-events-none" style:width={`${progressPercent}%`}></div>
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
                                                <span class="text-xs text-blue-700 font-medium w-36 text-right truncate tabular-nums" title={display.progressText || (status === 'cancelling' ? 'Cancelling...' : 'Starting...')}>
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
                                            {#if !downloadLocation || downloadLocation.trim() === ''} Set a download location to enable download. {:else if !model.download_url} Download URL missing for this model. {/if}
                                        </p>
                                    {/if}
                                </div>
                            </div>
						{/each}
						{#if availableModels.length === 0}
							<p class="text-center text-gray-500 pt-4">No models defined in the application.</p>
						{/if}
					{/if}
				</div>
			</div>
		{/if}

	</div> <!-- End Tab Content Area -->

</div> <!-- End Main Container -->

<style lang="postcss">
	/* Shared button styles */
	.btn-blue, .btn-delete, .btn-cancel, .btn-retry, .btn-blue-small { /* Added btn-blue-small */
		@apply px-2.5 py-1.5 border text-sm font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-1 transition-colors duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed;
	}
	/* Default button */
	.btn-blue {
		@apply border-transparent text-white bg-blue-600 hover:bg-blue-700 focus:ring-blue-500;
	}
    /* Smaller buttons for model list actions */
    .btn-blue-small, .btn-delete, .btn-cancel, .btn-retry {
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

	/* Tab styles */
	/* Defined inline using Tailwind classes */

	/* Scrollbar styles */
	.overflow-y-auto::-webkit-scrollbar { width: 6px; }
	.overflow-y-auto::-webkit-scrollbar-track { background: transparent; }
	.overflow-y-auto::-webkit-scrollbar-thumb { background-color: rgba(156, 163, 175, 0.5); border-radius: 10px; border: 2px solid transparent; background-clip: content-box; }
	.overflow-y-auto::-webkit-scrollbar-thumb:hover { background-color: rgba(107, 114, 128, 0.6); }
	.overflow-y-auto { scrollbar-width: thin; scrollbar-color: rgba(156, 163, 175, 0.5) transparent; }
	.tabular-nums { font-variant-numeric: tabular-nums; }
</style>