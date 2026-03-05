<script>
	import { onMount, onDestroy } from 'svelte';
	import { get } from 'svelte/store';
	import { invoke } from "@tauri-apps/api/core";
	import { ask } from "@tauri-apps/plugin-dialog";
	import { listen } from '@tauri-apps/api/event';
	import { open as openExternal } from '@tauri-apps/plugin-shell';
	import {
		configStatus,
		updateConfigStatus,
		setSelectedTranscriptionEngineStore,
		setWhisperCppModelsDownloaded,
		setFasterWhisperModelsDownloaded
	} from '$lib/stores/configStatusStore.js';
	import {
		availableWhisperCppModels,
		availableFasterWhisperModels
	} from '$lib/constants/models.js';
	import {
		downloadModel,
		downloadFasterWhisperModel,
		deleteModel,
		cancelFasterWhisperModelDownload,
		cancelDownload,
		setSelectedTranscriptionEngine,
		getSelectedTranscriptionEngine,
        installFasterWhisperDependencies,
        getDependencyCheckErrors,
        getDownloadedModels
	} from '$lib/services/configureActions';
	import { setTranscriptionModelsDownloaded } from '$lib/stores/configStatusStore.js';
	import notificationStore from '$lib/stores/notificationStore.js';
	import { v4 as uuidv4 } from 'uuid';
	import InstallLogModal from '../modals/InstallLogModal.svelte';

	export let downloadLocation = '';
	export let isBusy = false;
	export let totalDownloadedCount = 0;

	let downloadedModels = [];
	let configError = '';
	let downloadStatus = {}; // { modelName: 'not_downloaded' | 'downloading' | 'complete' | 'error' | 'cancelling' | 'cancelled' }
	let downloadProgressData = {}; // { modelName: { downloadedBytes, totalBytes } }

	let showLogModal = false;
	let modalLogs = [];
	let isDownloading = false;
	let isInstallingDependencies = false;
	let isChecking = false;
	let dependencyErrors = [];

	$: selectedEngine = $configStatus.selected_transcription_engine;

	$: whisperCppDownloadedCount = Array.isArray(downloadedModels) ? downloadedModels.filter(m => m.family === 'whisper-cpp' || (!m.family && !m.name.includes('/'))).length : 0;
	$: fasterWhisperDownloadedCount = Array.isArray(downloadedModels) ? downloadedModels.filter(m => m.family === 'faster-whisper').length : 0;

	$: {
		if ($configStatus.isInitialized) {
			setWhisperCppModelsDownloaded(whisperCppDownloadedCount > 0);
			setFasterWhisperModelsDownloaded(fasterWhisperDownloadedCount > 0);
		}
	}

	$: hasDownloadedFasterWhisper = fasterWhisperDownloadedCount > 0;
	$: hasDownloadedWhisperCpp = whisperCppDownloadedCount > 0;
	let unlistenStart = null;
	let unlistenStartFW = null;
	let unlistenLog = null;
	let unlistenComplete = null;
	let unlistenCompleteFW = null;
	let unlistenError = null;
	let unlistenErrorFW = null;
	let unlistenDownloadProgress = null;
	let unlistenFinished = null;

	let availableModelsList = [];
	let isFetchingModels = false;
	let hasFetched = false;
	let autoFetchTriggered = false;

	// Computed: determine display state for each model
	let modelDisplayData = {};
	$: {
		const newData = {};
		const currentDownloaded = Array.isArray(downloadedModels) ? downloadedModels : [];

		const targetList = selectedEngine === 'whisper-cpp' ? availableWhisperCppModels : availableFasterWhisperModels;

		for (const model of targetList) {
			const id = model.name;
			const getStatus = (modelName) => {
				const liveStatus = downloadStatus[modelName];
				if (liveStatus && liveStatus !== 'not_downloaded') return liveStatus;
				return currentDownloaded.some((m) => m?.name === modelName) ? 'complete' : 'not_downloaded';
			};

			const status = getStatus(id);
			const progress = downloadProgressData[id] || { downloadedBytes: 0, totalBytes: 0 };
			const progressPercent =
				progress.totalBytes > 0 ? Math.round((progress.downloadedBytes / progress.totalBytes) * 100) : 0;

			const getText = () => {
				if (status === 'complete') return 'Installed';
				if (status === 'downloading') {
					if (progress.totalBytes > 0) {
						const downloadedMB = (progress.downloadedBytes / (1024 * 1024)).toFixed(1);
						const totalMB = (progress.totalBytes / (1024 * 1024)).toFixed(1);
						return `${progressPercent}% (${downloadedMB} / ${totalMB} MB)`;
					} else {
						const downloadedMB = (progress.downloadedBytes / (1024 * 1024)).toFixed(1);
						return `${downloadedMB} MB`;
					}
				}
				return '';
			};

			newData[id] = {
				status,
				progressPercent,
				progressText: getText()
			};
		}
		modelDisplayData = newData;
	}

	$: displayedModels = (() => {
		let baseList = selectedEngine === 'whisper-cpp' ? [...availableWhisperCppModels] : [...availableFasterWhisperModels];

		// Enrichment with local info
		return baseList
			.map((m) => {
				const local = downloadedModels.find((dm) => dm.name === m.name);
				return {
					...m,
					sizeOnDisk: local?.size || null,
					family: local?.family || (selectedEngine === 'faster-whisper' ? 'faster-whisper' : 'whisper-cpp'),
					isInstalled: !!local
				};
			})
			.sort((a, b) => {
				const statusA = modelDisplayData[a.name]?.status;
				const statusB = modelDisplayData[b.name]?.status;
				const aActive = statusA && statusA !== 'not_downloaded';
				const bActive = statusB && statusB !== 'not_downloaded';

				if (aActive && !bActive) return -1;
				if (!aActive && bActive) return 1;
				return 0; // Maintain original list order otherwise
			});
	})();

	$: {
		if ($configStatus.isInitialized && !$configStatus.python_libraries_installed) {
			autoFetchTriggered = false;
		} else {
			if (!hasFetched && !isFetchingModels && !autoFetchTriggered && selectedEngine === 'faster-whisper') {
				autoFetchTriggered = true;
				// Maybe add auto-fetch here if needed
			}
		}
	}

	async function handleEngineChange(newEngine) {
		setSelectedTranscriptionEngineStore(newEngine);
		await setSelectedTranscriptionEngine(newEngine);
	}

	onMount(async () => {
		configError = '';
		try {
			const persistedEngine = await getSelectedTranscriptionEngine();
			if (persistedEngine) {
				setSelectedTranscriptionEngineStore(persistedEngine);
			}
			const models = await getDownloadedModels();
			downloadedModels = Array.isArray(models) ? models : [];
			totalDownloadedCount = downloadedModels.length;
		} catch (e) {
			configError = `Failed to load model configuration: ${e.message || e}`;
		}

		try {
			unlistenStart = await listen('download-start', (event) => {
				const modelName = event.payload;
				downloadStatus = { ...downloadStatus, [modelName]: 'downloading' };
				modalLogs = [...modalLogs, { id: uuidv4(), message: `Starting download for ${modelName}...` }];
				isDownloading = true;
				isInstallingDependencies = false;
				showLogModal = true;
			});

			unlistenStartFW = await listen('transcription-download-start', (event) => {
				const modelName = event.payload;
				downloadStatus = { ...downloadStatus, [modelName]: 'downloading' };
				modalLogs = [...modalLogs, { id: uuidv4(), message: `Starting download for ${modelName}...` }];
				isDownloading = true;
				isInstallingDependencies = false;
				showLogModal = true;
			});

			unlistenLog = await listen('transcription-download-log', (event) => {
				const { model_name, log_line } = event.payload;
                // If it's a "System" log or matches current model, show it
				if (log_line.includes("Installing whisper.cpp") || log_line.includes("micromamba")) {
                    isInstallingDependencies = true;
                }
				if (downloadStatus[model_name] === 'downloading' || isInstallingDependencies || model_name === "System") {
					modalLogs = [...modalLogs, { id: uuidv4(), message: log_line }];
				}
			});

			unlistenDownloadProgress = await listen('download-progress', (event) => {
				const { model_name, downloaded_bytes, total_bytes } = event.payload;
				downloadProgressData = {
					...downloadProgressData,
					[model_name]: { downloadedBytes: downloaded_bytes, totalBytes: total_bytes }
				};
			});

			unlistenComplete = await listen('transcription-download-complete', async (event) => {
				const downloadedModelName = event.payload;
				downloadStatus = { ...downloadStatus, [downloadedModelName]: 'complete' };

				// Clear progress data for this model
				const nextProgressData = { ...downloadProgressData };
				delete nextProgressData[downloadedModelName];
				downloadProgressData = nextProgressData;

				// Clear from local status tracking after a delay to let reactive derived handle it via downloadedModels
				setTimeout(() => {
					if (downloadStatus[downloadedModelName] === 'complete') {
						const nextStatus = { ...downloadStatus };
						delete nextStatus[downloadedModelName];
						downloadStatus = nextStatus;
					}
				}, 1000);

				try {
					const models = await getDownloadedModels();
					downloadedModels = Array.isArray(models) ? models : [];
					totalDownloadedCount = downloadedModels.length;
					setTranscriptionModelsDownloaded(totalDownloadedCount > 0);
				} catch (e) {
					console.error(`Failed to refresh models after ${downloadedModelName} completion:`, e);
				}
				modalLogs = [...modalLogs, { id: uuidv4(), message: `Download complete for ${downloadedModelName}.` }];
				isDownloading = false;
				isInstallingDependencies = false;
			});

			unlistenError = await listen('download-error', (event) => {
				const { model_name, error_message } = event.payload;
				let finalStatus;
				if (error_message.toLowerCase().includes('cancel')) {
					finalStatus = 'cancelled';
				} else {
					finalStatus = 'error';
					notificationStore.add(`Error downloading ${model_name}: ${error_message}`, 'error');
				}
				downloadStatus = { ...downloadStatus, [model_name]: finalStatus };
				modalLogs = [...modalLogs, { id: uuidv4(), message: `Error downloading ${model_name}: ${error_message}` }];
				isDownloading = false;
				isInstallingDependencies = false;
			});

			unlistenErrorFW = await listen('transcription-download-error', (event) => {
				const { model_name, error_message } = event.payload;
				let finalStatus;
				if (error_message.toLowerCase().includes('cancel')) {
					finalStatus = 'cancelled';
				} else {
					finalStatus = 'error';
					notificationStore.add(`Error downloading ${model_name}: ${error_message}`, 'error');
				}
				downloadStatus = { ...downloadStatus, [model_name]: finalStatus };
				modalLogs = [...modalLogs, { id: uuidv4(), message: `Error downloading ${model_name}: ${error_message}` }];
				isDownloading = false;
				isInstallingDependencies = false;
			});

			unlistenFinished = await listen('transcription-download-finished', async () => {
				console.log('Frontend: Received transcription-download-finished event. Setting isDownloading to false.');
				isDownloading = false;
				isInstallingDependencies = false;
				isChecking = true;
				try {
					await updateConfigStatus(true);
				} finally {
					isChecking = false;
				}
			});
		} catch (err) {
			console.error('Failed to attach download event listeners:', err);
			configError = 'Could not set up download monitoring.';
		}
	});

	onDestroy(() => {
		if (unlistenStart) unlistenStart();
		if (unlistenStartFW) unlistenStartFW();
		if (unlistenLog) unlistenLog();
		if (unlistenComplete) unlistenComplete();
		if (unlistenCompleteFW) unlistenCompleteFW();
		if (unlistenError) unlistenError();
		if (unlistenErrorFW) unlistenErrorFW();
		if (unlistenDownloadProgress) unlistenDownloadProgress();
		if (unlistenFinished) unlistenFinished();
	});

	async function handleDownload(model) {
		if (isBusy || isDownloading || isInstallingDependencies) return;
		if (!downloadLocation || downloadLocation.trim() === '') {
			notificationStore.add('Please set a valid model download location first.', 'error');
			return;
		}

		configError = '';
		const modelName = model.name;

		try {
			modalLogs = [];

			// Check if we will likely need to install dependencies
			const willInstallDeps = (selectedEngine === 'faster-whisper' && !$configStatus.faster_whisper_dependencies_installed) ||
				(selectedEngine === 'whisper-cpp' && !$configStatus.whisper_cpp_installed);

			if (willInstallDeps) {
				isInstallingDependencies = true;
			} else {
				isDownloading = true;
			}
			showLogModal = true;

			if (selectedEngine === 'faster-whisper') {
				await downloadFasterWhisperModel(model, downloadLocation);
			} else {
				await downloadModel(model, downloadLocation);
			}
		} catch (err) {
			notificationStore.add(`Failed to start download for ${modelName}: ${err.message || err}`, 'error');
			isDownloading = false;
			isInstallingDependencies = false;
		}
	}

	async function handleDelete(model) {
		if (isBusy) return;
		const modelNameForDelete = model.name;
		const confirmed = await ask(
			`Are you sure you want to delete the model "${modelNameForDelete}"? This will remove the model file from disk.`,
			{ title: 'Confirm Deletion', type: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' }
		);
		if (!confirmed) return;

		try {
			await deleteModel(model);
			const models = await getDownloadedModels();
			downloadedModels = Array.isArray(models) ? models : [];
			totalDownloadedCount = downloadedModels.length;
			downloadStatus = { ...downloadStatus, [modelNameForDelete]: 'not_downloaded' };
			setTranscriptionModelsDownloaded(totalDownloadedCount > 0);
		} catch (err) {
			notificationStore.add(`Failed to delete model ${modelNameForDelete}: ${err.message || err}`, 'error');
		}
	}

	async function handleCancel(model) {
		if (isBusy) return;
		const modelName = model.name;
		const display = modelDisplayData[modelName] || {};
		const currentStatus = display.status;

		if (currentStatus !== 'downloading') return;
		downloadStatus = { ...downloadStatus, [modelName]: 'cancelling' };
		configError = '';
		try {
			if (model.family === 'faster-whisper') {
				await cancelFasterWhisperModelDownload(modelName);
			} else {
				await cancelDownload(modelName);
			}
		} catch (err) {
			alert(`Failed to send cancel request for ${modelName}: ${err.message || err}`);
			downloadStatus = { ...downloadStatus, [modelName]: 'downloading' };
		}
	}

	async function handleInstallFWDependencies() {
		if (isInstallingDependencies) return;
		isInstallingDependencies = true;
		showLogModal = true;
		modalLogs = [{ id: uuidv4(), message: "Starting installation of Faster-Whisper dependencies..." }];
		dependencyErrors = [];

		const unlistenLog = await listen('installation-log', (event) => {
			modalLogs = [...modalLogs, { id: uuidv4(), message: event.payload.message }];
		});

		try {
			await installFasterWhisperDependencies();
			modalLogs = [...modalLogs, { id: uuidv4(), message: "Installation successful!" }];
			isChecking = true;
			try {
				await updateConfigStatus(true);
			} finally {
				isChecking = false;
			}
		} catch (err) {
			let errorMsg = typeof err === 'object' ? (err.message || JSON.stringify(err)) : String(err);
			modalLogs = [...modalLogs, { id: uuidv4(), message: `Installation failed: ${errorMsg}` }];
			console.error("Installation failed:", err);
		} finally {
			unlistenLog();
			isInstallingDependencies = false;
		}
	}

	async function checkDependencyErrors() {
		try {
			dependencyErrors = await getDependencyCheckErrors();
		} catch (e) {
			console.error("Failed to get dependency errors:", e);
		}
	}

	async function handleInstallWCDependencies() {
		if (isInstallingDependencies) return;
		isInstallingDependencies = true;
		showLogModal = true;
		modalLogs = [{ id: uuidv4(), message: "Starting installation of whisper.cpp dependencies..." }];
		dependencyErrors = [];

		const unlistenLog = await listen('installation-log', (event) => {
			modalLogs = [...modalLogs, { id: uuidv4(), message: event.payload.log_line || event.payload.message || event.payload.status }];
		});

		try {
			await invoke('install_whisper_cpp_dependencies_command');
			modalLogs = [...modalLogs, { id: uuidv4(), message: "Installation successful!" }];
			isChecking = true;
			try {
				await updateConfigStatus(true);
			} finally {
				isChecking = false;
			}
		} catch (err) {
			modalLogs = [...modalLogs, { id: uuidv4(), message: `Installation failed: ${err}` }];
		} finally {
			unlistenLog();
			isInstallingDependencies = false;
		}
	}

	$: if ($configStatus.isInitialized && !$configStatus.faster_whisper_dependencies_installed && hasDownloadedFasterWhisper) {
		checkDependencyErrors();
	}
</script>

<div class="flex flex-col h-full overflow-y-auto p-1">
	<div class="flex justify-between items-center mb-2 px-1">
		<h3 class="text-sm font-medium text-gray-700 dark:text-gray-200">Transcription Models</h3>
		<div class="flex items-center">
			{#if (selectedEngine === 'whisper-cpp' ? whisperCppDownloadedCount : fasterWhisperDownloadedCount) > 0}
				<span class="text-sm font-medium text-green-600 dark:text-green-400 uppercase">
					{selectedEngine === 'whisper-cpp' ? whisperCppDownloadedCount : fasterWhisperDownloadedCount} {selectedEngine === 'whisper-cpp' ? 'WHISPER.CPP' : 'FASTER-WHISPER'} {(selectedEngine === 'whisper-cpp' ? whisperCppDownloadedCount : fasterWhisperDownloadedCount) === 1 ? 'MODEL' : 'MODELS'} DOWNLOADED
				</span>
			{:else}
				<span class="text-sm font-medium text-red-600 dark:text-red-400 uppercase">NO {selectedEngine === 'whisper-cpp' ? 'WHISPER.CPP' : 'FASTER-WHISPER'} MODELS DOWNLOADED</span>
			{/if}
		</div>
	</div>

	{#if $configStatus.isInitialized && !$configStatus.faster_whisper_dependencies_installed && hasDownloadedFasterWhisper}
		<div class="mb-4 flex flex-col bg-orange-100 dark:bg-orange-900/30 border border-orange-200 dark:border-orange-800 p-3 rounded-md shadow-sm">
			<div class="flex items-center justify-between mb-2">
				<div class="flex items-center">
					<svg class="w-4 h-4 text-orange-600 dark:text-orange-400 mr-2 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
						<path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd"></path>
					</svg>
					<span class="text-xs text-orange-800 dark:text-orange-300 font-medium">Faster-Whisper libraries are missing.</span>
				</div>
				<button class="bg-orange-600 hover:bg-orange-700 text-white px-3 py-1.5 rounded-md text-[11px] font-semibold transition-colors shadow-sm" on:click={handleInstallFWDependencies} disabled={isInstallingDependencies}>
					Install Now
				</button>
			</div>
			{#if dependencyErrors.length > 0}
				<div class="mt-1 text-[10px] text-orange-700/80 dark:text-orange-400/80 font-mono bg-orange-50/50 dark:bg-orange-950/30 p-2 rounded border border-orange-200/50 dark:border-orange-800/50 max-h-32 overflow-y-auto">
					{#each dependencyErrors as err}
						<div class="mb-1 last:mb-0">{err}</div>
					{/each}
				</div>
			{/if}
		</div>
	{/if}

	{#if $configStatus.isInitialized && !$configStatus.whisper_cpp_installed && hasDownloadedWhisperCpp}
		<div class="mb-4 flex flex-col bg-orange-100 dark:bg-orange-900/30 border border-orange-200 dark:border-orange-800 p-3 rounded-md shadow-sm">
			<div class="flex items-center justify-between mb-2">
				<div class="flex items-center">
					<svg class="w-4 h-4 text-orange-600 dark:text-orange-400 mr-2 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
						<path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd"></path>
					</svg>
					<span class="text-xs text-orange-800 dark:text-orange-300 font-medium">Whisper.cpp library is missing.</span>
				</div>
				<button class="bg-orange-600 hover:bg-orange-700 text-white px-3 py-1.5 rounded-md text-[11px] font-semibold transition-colors shadow-sm" on:click={handleInstallWCDependencies} disabled={isInstallingDependencies}>
					Install Now
				</button>
			</div>
		</div>
	{/if}

	<!-- Engine Toggle -->
	<div class="bg-blue-50 dark:bg-blue-900/10 border border-blue-100 dark:border-blue-800 rounded-md p-3 mb-4 flex-shrink-0">
		<div class="flex items-center justify-between mb-2">
			<span class="text-sm font-semibold text-blue-800 dark:text-blue-300">Select Transcription Engine</span>
			<div class="flex space-x-2">
				<button
					class="px-3 py-1 text-xs rounded-full border transition-all"
					class:bg-blue-600={selectedEngine === 'whisper-cpp'}
					class:text-white={selectedEngine === 'whisper-cpp'}
					class:border-transparent={selectedEngine === 'whisper-cpp'}
					class:bg-white={selectedEngine !== 'whisper-cpp'}
					class:dark:bg-gray-800={selectedEngine !== 'whisper-cpp'}
					class:text-gray-600={selectedEngine !== 'whisper-cpp'}
					class:dark:text-gray-400={selectedEngine !== 'whisper-cpp'}
					class:border-gray-200={selectedEngine !== 'whisper-cpp'}
					class:dark:border-gray-700={selectedEngine !== 'whisper-cpp'}
					on:click={() => handleEngineChange('whisper-cpp')}
				>
					whisper.cpp
				</button>
				<button
					class="px-3 py-1 text-xs rounded-full border transition-all"
					class:bg-blue-600={selectedEngine === 'faster-whisper'}
					class:text-white={selectedEngine === 'faster-whisper'}
					class:border-transparent={selectedEngine === 'faster-whisper'}
					class:bg-white={selectedEngine !== 'faster-whisper'}
					class:dark:bg-gray-800={selectedEngine !== 'faster-whisper'}
					class:text-gray-600={selectedEngine !== 'faster-whisper'}
					class:dark:text-gray-400={selectedEngine !== 'faster-whisper'}
					class:border-gray-200={selectedEngine !== 'faster-whisper'}
					class:dark:border-gray-700={selectedEngine !== 'faster-whisper'}
					on:click={() => handleEngineChange('faster-whisper')}
				>
					faster-whisper
				</button>
			</div>
		</div>

		{#if selectedEngine === 'whisper-cpp'}
			<div class="text-[11px] text-blue-700/80 dark:text-blue-400/80 leading-relaxed">
				<p><strong class="text-blue-800 dark:text-blue-300">Pros:</strong> Native Metal support on Mac, extremely fast, lightweight, high accuracy with GGML models.</p>
				<p><strong class="text-blue-800 dark:text-blue-300">Cons:</strong> Less optimized for NVIDIA GPUs than faster-whisper.</p>
			</div>
		{:else}
			<div class="text-[11px] text-blue-700/80 dark:text-blue-400/80 leading-relaxed">
				<p><strong class="text-blue-800 dark:text-blue-300">Pros:</strong> Blazing fast on NVIDIA GPUs, supports integer8 quantization for lower memory usage.</p>
				<p><strong class="text-blue-800 dark:text-blue-300">Cons:</strong> Slower on Macs compared to whisper.cpp, larger library dependencies.</p>
			</div>
		{/if}
	</div>

	<div class="border dark:border-gray-700 rounded-md flex-grow overflow-y-auto bg-gray-50 dark:bg-gray-800/50 p-2">
		<div class="space-y-2">
			{#each displayedModels as model (model.name)}
				{@const display = modelDisplayData[model.name] || { status: 'not_downloaded' }}
				{@const status = display.status}
				{@const isDownloadEnabled = !isBusy && downloadLocation && downloadLocation.trim() !== '' && $configStatus.python_libraries_installed}
				{@const isDeleteEnabled = !isBusy}
				{@const isCancelEnabled = status === 'downloading'}

				<div class="bg-white dark:bg-gray-800 border dark:border-gray-700 p-3 rounded-md shadow-sm flex flex-col hover:border-blue-400 transition-colors relative overflow-hidden">
					<!-- Progress Bar Overlay (for downloading models) -->
					{#if status === 'downloading' || status === 'cancelling'}
						<div
							class="absolute bottom-0 left-0 h-1 bg-blue-100 dark:bg-blue-900/30 transition-all duration-300"
							style:width={`${display.progressPercent}%`}
						></div>
					{/if}

					<div class="relative z-10 flex justify-between items-start">
						<div class="flex flex-col min-w-0 pr-4">
							<div class="flex items-center space-x-2">
								<span class="font-semibold text-gray-800 dark:text-gray-200 truncate">{model.name}</span>
								{#if model.info_url}
									<button 
										class="text-gray-400 hover:text-blue-500 dark:text-gray-500 dark:hover:text-blue-400 focus:outline-none p-0.5 rounded hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
										title="View on Hugging Face"
										on:click|stopPropagation={() => openExternal(model.info_url)}
									>
										<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-box-arrow-up-right" viewBox="0 0 16 16">
											<path fill-rule="evenodd" d="M8.636 3.5a.5.5 0 0 0-.5-.5H1.5A1.5 1.5 0 0 0 0 4.5v10A1.5 1.5 0 0 0 1.5 16h10a1.5 1.5 0 0 0 1.5-1.5V7.864a.5.5 0 0 0-1 0V14.5a.5.5 0 0 1-.5.5h-10a.5.5 0 0 1-.5-.5v-10a.5.5 0 0 1 .5-.5h6.636a.5.5 0 0 0 .5-.5"/>
											<path fill-rule="evenodd" d="M16 .5a.5.5 0 0 0-.5-.5h-5a.5.5 0 0 0 0 1h3.793L6.146 9.146a.5.5 0 1 0 .708.708L15 1.707V5.5a.5.5 0 0 0 1 0z"/>
										</svg>
									</button>
								{/if}
								{#if status === 'complete'}
									<span class="px-1.5 py-0.5 rounded text-[10px] font-medium bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300">Installed</span>
								{/if}
							</div>
							<div class="text-xs text-gray-500 dark:text-gray-400 mt-0.5 flex items-center space-x-3">
								<span class="flex items-center" title="Engine Family">
									{#if model.family}
										<span>{model.family === 'faster-whisper' ? 'Faster-Whisper' : 'Whisper.cpp'}</span>
									{/if}
								</span>
								<span class="flex items-center text-gray-400" title="Model Size">
									<span>&bull;</span>
									<span class="ml-1">{model.size}</span>
								</span>
								{#if model.sizeOnDisk}
									<span class="flex items-center text-gray-400" title="Size on disk">
										<span>&bull;</span>
										<span class="ml-1">{model.sizeOnDisk}</span>
									</span>
								{/if}
							</div>
							{#if model.description}
								<div class="text-[11px] text-gray-400 dark:text-gray-500 mt-1 italic line-clamp-1">
									{model.description}
								</div>
							{/if}
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
										{#if status === 'cancelling'}Cancelling...{:else}{display.progressText || (model.family === 'faster-whisper' ? 'Downloading...' : 'Starting...')}{/if}
									</span>
									<button
										class="btn-cancel"
										on:click={() => handleCancel(model)}
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
	</div>
</div>

<InstallLogModal bind:showModal={showLogModal} logs={modalLogs} isInstalling={isDownloading || isInstallingDependencies} isChecking={isChecking} title={isInstallingDependencies ? "Installing Dependencies" : "Downloading Transcription Model"} inProgressText={isInstallingDependencies ? "Installing..." : "Downloading..."} />

<style lang="postcss">
	.btn-blue-small, .btn-delete, .btn-cancel, .btn-retry {
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