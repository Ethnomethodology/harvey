<script>
	import { onMount, onDestroy } from 'svelte';
	import { ask } from '@tauri-apps/plugin-dialog';
	import { listen } from '@tauri-apps/api/event';
	import { open as openExternal } from '@tauri-apps/plugin-shell';
	import {
		configStatus,
		updateConfigStatus,
		setSelectedTranslationEngineStore,
		setHelsinkiModelsDownloaded,
		setNllbModelsDownloaded
	} from '$lib/stores/configStatusStore.js';
	import {
		downloadTranslationModel,
		deleteTranslationModel,
        getLocalTranslationModels,
		cancelTranslationModelDownload,
		fetchAvailableModels,
		getSelectedTranslationEngine,
		setSelectedTranslationEngine,
		isCTranslate2Installed,
        installFasterWhisperDependencies
	} from '$lib/services/configureActions';
	import { setTranslationModelsDownloaded } from '$lib/stores/configStatusStore.js';
	import notificationStore from '$lib/stores/notificationStore.js';
	import { get } from 'svelte/store';
	import { v4 as uuidv4 } from 'uuid'; // Import uuidv4
	import InstallLogModal from '$lib/components/modals/InstallLogModal.svelte';
	import Dropdown from '$lib/components/shared/Dropdown.svelte';
	import { languageMap } from '$lib/constants/languageMap.js';

	export let downloadLocation = '';
	export let isBusy = false;
	export let translationModelCount = 0;

	let downloadedModels = [];
	$: translationModelCount = downloadedModels.length;
	let configError = '';
	let downloadStatus = {};
	let ct2Installed = true;

	let modelName = '';

	let showLogModal = false;
	let modalLogs = [];
	let isDownloading = false;
    let isInstallingDependencies = false;
    let isChecking = false;

	let selectedOption = 'selectLanguages'; // Default to selecting languages
	$: selectedEngine = $configStatus.selected_translation_engine;

    $: helsinkiDownloadedCount = Array.isArray(downloadedModels) ? downloadedModels.filter(m => m.family === 'helsinki' || !m.family).length : 0;
    $: nllbDownloadedCount = Array.isArray(downloadedModels) ? downloadedModels.filter(m => m.family === 'nllb').length : 0;

    $: {
        if ($configStatus.isInitialized) {
            setHelsinkiModelsDownloaded(helsinkiDownloadedCount > 0);
            setNllbModelsDownloaded(nllbDownloadedCount > 0);
        }
    }

	// --- Marketplace / Search View State ---
	let availableModelsList = [];
	let filteredModels = [];
	let searchQuery = '';
	let isFetchingModels = false;
	let hasFetched = false;
	let autoFetchTriggered = false;

	$: isAnyModelDownloading = Object.values(downloadStatus).includes('downloading');
    $: isBusy = isAnyModelDownloading;

	let modelDisplayData = {};

	// Update display data reactively
	$: {
		const newData = {};
		const currentDownloaded = Array.isArray(downloadedModels) ? downloadedModels : [];

		// Track all models we know about (available + downloaded)
		const allKnownModels = [...availableModelsList];
		for (const dm of currentDownloaded) {
			if (!allKnownModels.some(am => am.id === dm.name)) {
				// Add downloaded model to known list if it's not there (e.g. from manual entry)
				allKnownModels.push({
					id: dm.name,
					src: null, // We'll rely on formatModelDisplayName for these
					tgt: null,
					downloads: 0,
					family: dm.family || (dm.name.toLowerCase().includes('nllb') ? 'nllb' : 'helsinki')
				});
			}
		}

		for (const model of allKnownModels) {
			const id = model.id;
			const getStatus = (modelId) => {
				const liveStatus = downloadStatus[modelId];
				if (liveStatus && liveStatus !== 'not_downloaded') return liveStatus;
				return currentDownloaded.some((m) => m?.name === modelId) ? 'complete' : 'not_downloaded';
			};
			const status = getStatus(id);
			
			// We don't have progress percent for translation yet in this component's logic, 
			// but we can add it if backend supports it. For now, just status.
			newData[id] = { status };
		}
		modelDisplayData = newData;
	}

	// Combined list logic
	$: displayedModels = (() => {
		let baseList = [];
		if (!hasFetched) {
			// Show only models that are downloaded OR have active state
			baseList = [...downloadedModels.map(m => ({
				id: m.name,
				src: null,
				tgt: null,
				downloads: 0,
				size: m.size,
				family: m.family || (m.name.toLowerCase().includes('nllb') ? 'nllb' : 'helsinki')
			}))];
			
			// Add any that are currently downloading but not yet in downloadedModels
			for (const id in downloadStatus) {
				if (downloadStatus[id] === 'downloading' && !baseList.some(m => m.id === id)) {
					baseList.push({ id, src: null, tgt: null, downloads: 0, family: id.toLowerCase().includes('nllb') ? 'nllb' : 'helsinki' });
				}
			}
		} else {
			baseList = [...availableModelsList];
			// Ensure all downloaded models are in the list and enrich with local info (like size)
			for (const dm of downloadedModels) {
				const existingIndex = baseList.findIndex(am => am.id === dm.name);
				if (existingIndex === -1) {
					baseList.push({ 
						id: dm.name, 
						src: null, 
						tgt: null, 
						downloads: 0, 
						size: dm.size,
						family: dm.family || (dm.name.toLowerCase().includes('nllb') ? 'nllb' : 'helsinki') 
					});
				} else if (dm.size) {
					// Enrich existing entry with local size
					baseList[existingIndex] = { ...baseList[existingIndex], size: dm.size };
				}
			}
		}

		// Filter by engine/family
		baseList = baseList.filter(m => m.family === selectedEngine);

		// Filter by search query
		if (searchQuery.trim() !== '') {
			const q = searchQuery.toLowerCase();
			baseList = baseList.filter(m => {
				const srcName = languageMap.get(m.src)?.toLowerCase() || '';
				const tgtName = languageMap.get(m.tgt)?.toLowerCase() || '';
				const displayName = formatModelDisplayName(m.id).toLowerCase();
				return (
					m.id.toLowerCase().includes(q) ||
					m.src?.toLowerCase().includes(q) ||
					m.tgt?.toLowerCase().includes(q) ||
					srcName.includes(q) ||
					tgtName.includes(q) ||
					displayName.includes(q)
				);
			});
		}

		// Sort: Downloaded/Active first, then by downloads descending
		return baseList.sort((a, b) => {
			const statusA = modelDisplayData[a.id]?.status;
			const statusB = modelDisplayData[b.id]?.status;
			const aActive = statusA && statusA !== 'not_downloaded';
			const bActive = statusB && statusB !== 'not_downloaded';

			if (aActive && !bActive) return -1;
			if (!aActive && bActive) return 1;
			
			if (aActive && bActive) return a.id.localeCompare(b.id);
			
			return (b.downloads || 0) - (a.downloads || 0);
		}).slice(0, 2000);
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
	let unlistenLog = null;
	let unlistenComplete = null;
	let unlistenError = null;
    let unlistenFinished = null;

	function handleOptionChange() {
		if (selectedOption === 'selectLanguages' && modelName.trim() !== '') {
			modelName = ''; 
		}
	}

	async function handleEngineChange(newEngine) {
        setSelectedTranslationEngineStore(newEngine);
		await setSelectedTranslationEngine(newEngine);
	}

	async function handleRefreshModels() {
		if (isFetchingModels) return;
		isFetchingModels = true;
		try {
			const fetched = await fetchAvailableModels();
			if (fetched && Array.isArray(fetched) && fetched.length > 0) {
				availableModelsList = fetched;
				hasFetched = true;
			} else {
				notificationStore.add('Fetched model list was empty.', 'warning');
			}
		} catch (e) {
			notificationStore.add(`Failed to refresh models: ${e.message}`, 'error');
		} finally {
			isFetchingModels = false;
		}
	}

	onMount(async () => {
		configError = '';
		try {
			downloadedModels = await getLocalTranslationModels();
			ct2Installed = await isCTranslate2Installed();
		} catch (e) {
			configError = `Failed to load model configuration: ${e.message || e}`;
		}

		try {
			unlistenStart = await listen('translation-download-start', (event) => {
				const modelName = event.payload;
				downloadStatus = { ...downloadStatus, [modelName]: 'downloading' };
				modalLogs = [...modalLogs, { id: uuidv4(), message: `Starting download for ${modelName}...` }];
				isDownloading = true;
                isInstallingDependencies = false;
				showLogModal = true;
			});
			unlistenLog = await listen('translation-download-log', (event) => {
				const { model_name, log_line } = event.payload;
                
                // Detect installation activity to update spinner text
                if (log_line.includes("CTranslate2") || log_line.includes("micromamba") || log_line.includes("Optimizing")) {
                    isInstallingDependencies = true;
                }

				if (downloadStatus[model_name] === 'downloading' || isInstallingDependencies || model_name === "System") {
					modalLogs = [...modalLogs, { id: uuidv4(), message: log_line }];
				}
			});
			unlistenComplete = await listen('translation-download-complete', async (event) => {
				const downloadedModelName = event.payload;
				downloadStatus = { ...downloadStatus, [downloadedModelName]: 'complete' };
				// Clear from local status tracking to let reactive derived handle it via downloadedModels
				setTimeout(() => {
					if (downloadStatus[downloadedModelName] === 'complete') {
						const nextStatus = { ...downloadStatus };
						delete nextStatus[downloadedModelName];
						downloadStatus = nextStatus;
					}
				}, 1000);

				try {
					downloadedModels = await getLocalTranslationModels();
					ct2Installed = await isCTranslate2Installed();
					setTranslationModelsDownloaded(downloadedModels.length > 0);
				} catch (e) { console.error(`Failed to refresh models after ${downloadedModelName} completion:`, e); }
				modalLogs = [...modalLogs, { id: uuidv4(), message: `Download complete for ${downloadedModelName}.` }];
                isDownloading = false;
                isInstallingDependencies = false;
				if (modelName.trim() === downloadedModelName) {
					modelName = '';
				}
			});
			unlistenError = await listen('translation-download-error', (event) => {
				const { model_name, error_message } = event.payload;
				let finalStatus;
				if (error_message.toLowerCase().includes('cancel')) { finalStatus = 'cancelled'; } else { finalStatus = 'error'; notificationStore.add(`Error downloading ${model_name}: ${error_message}`, 'error'); }
				downloadStatus = { ...downloadStatus, [model_name]: finalStatus };
				modalLogs = [...modalLogs, { id: uuidv4(), message: `Error downloading ${model_name}: ${error_message}` }];
				isDownloading = false;
                isInstallingDependencies = false;
			});

			unlistenFinished = await listen('translation-download-finished', async () => {
				console.log('Frontend: Received translation-download-finished event. Setting isDownloading to false.');
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
		if (unlistenLog) unlistenLog();
		if (unlistenComplete) unlistenComplete();
		if (unlistenError) unlistenError();
		if (unlistenFinished) unlistenFinished();
	});

	async function handleDownload(targetModelId) {
		if (isBusy || isDownloading || isInstallingDependencies) return;
		if (!downloadLocation || downloadLocation.trim() === '') {
			notificationStore.add('Please set a valid model download location first.', 'error');
			return;
		}

		configError = '';
		const modelToDownload = targetModelId || modelName.trim();
		
		if (!modelToDownload) {
			notificationStore.add('Please enter a model name.', 'error');
			return;
		}

		// Find family from list if possible
		const knownModel = availableModelsList.find(m => m.id === modelToDownload);
		const family = knownModel ? knownModel.family : (modelToDownload.toLowerCase().includes('nllb') ? 'nllb' : 'helsinki');

		try {
			modalLogs = [];
            
            // For Helsinki models, we might need to install CTranslate2
            const willInstallDeps = family === 'helsinki' && !ct2Installed;
            
            if (willInstallDeps) {
                isInstallingDependencies = true;
            } else {
                isDownloading = true;
            }
            showLogModal = true;

			await downloadTranslationModel(null, null, downloadLocation, modelToDownload, family);
		} catch (err) {
			notificationStore.add(`Failed to start download for ${modelToDownload}: ${err.message || err}`, 'error');
			isDownloading = false; 
            isInstallingDependencies = false;
		}
	}

    async function handleDelete(model) {
		if (isBusy) return;
		const modelNameForDelete = model.id || model.name;
		if (!modelNameForDelete) { notificationStore.add("Cannot delete model: Missing name.", 'error'); return; }
		const confirmed = await ask(`Are you sure you want to delete the model "${modelNameForDelete}"? This will remove the entire model folder from disk.`, { title: 'Confirm Deletion', type: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' });
		if (!confirmed) return;
		try {
			// Pass the full model object (which contains .family)
			await deleteTranslationModel(model); 
			downloadedModels = await getLocalTranslationModels(); 
            downloadStatus = { ...downloadStatus, [modelNameForDelete]: 'not_downloaded' };
			setTranslationModelsDownloaded(downloadedModels.length > 0);
		} catch (err) {
			notificationStore.add(`Failed to delete model ${modelNameForDelete}: ${err.message || err}`, 'error');
		}
    }

	async function handleCancel(modelId) {
		if (isBusy) return;
		downloadStatus = { ...downloadStatus, [modelId]: 'cancelling' };
		try {
			await cancelTranslationModelDownload(modelId);
		} catch (err) {
			notificationStore.add(`Failed to cancel download for ${modelId}: ${err.message || err}`, 'error');
			downloadStatus = { ...downloadStatus, [modelId]: 'downloading' };
		}
	}

    async function handleInstallDependencies() {
        if (isInstallingDependencies) return;
        isInstallingDependencies = true;
        showLogModal = true;
        modalLogs = [{ id: uuidv4(), message: "Starting installation of optimized translation backend (CTranslate2)..." }];
        
        const unlistenLog = await listen('installation-log', (event) => {
            modalLogs = [...modalLogs, { id: uuidv4(), message: event.payload.message }];
        });

        try {
            await installFasterWhisperDependencies(); // This installs ctranslate2, faster-whisper, and sounddevice
            modalLogs = [...modalLogs, { id: uuidv4(), message: "Installation successful!" }];
            isChecking = true;
            try {
                await updateConfigStatus(true);
                ct2Installed = await isCTranslate2Installed();
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

    function formatModelDisplayName(modelName) {
        const parts = modelName.split('/');
		const baseName = parts[parts.length - 1] || modelName;

		if (baseName.toLowerCase().includes('nllb')) {
            const isDistilled = baseName.includes('distilled');
            let sizeLabel = "";
            
			if (baseName.includes('600M')) sizeLabel = "600M";
			else if (baseName.includes('1.3B')) sizeLabel = "1.3B";
			else if (baseName.includes('3.3B')) sizeLabel = "3.3B";
            
            if (sizeLabel === "600M") return "NLLB-200 Distilled (Small & Fast)";
            if (sizeLabel === "1.3B") return isDistilled ? "NLLB-200 Distilled (Medium)" : "NLLB-200 (Medium)";
            if (sizeLabel === "3.3B") return isDistilled ? "NLLB-200 Distilled (Large)" : "NLLB-200 (Large)";
            
			return baseName;
		}

        if (parts.length === 2) {
            const langParts = parts[1].split('-');
            if (langParts.length >= 3 && langParts[0] === 'opus' && langParts[1] === 'mt') {
                const fromCode = langParts[langParts.length - 2];
                const toCode = langParts[langParts.length - 1];

                const fromLang = languageMap.get(fromCode);
                const toLang = languageMap.get(toCode);

                if (fromLang && toLang) {
                    return `${fromLang} to ${toLang}`;
                }
            }
        }
        return modelName; 
    }
	
	function getLangName(code) {
		return languageMap.get(code) || code;
	}
	
	function isModelDownloaded(id) {
		return downloadedModels.some(m => m.name === id);
	}

	async function openLink(url) {
		if (!url) return;
		try {
			await openExternal(url);
		} catch (err) {
			console.error(`Failed to open external link ${url}:`, err);
		}
	}
	</script>

<div class="flex flex-col h-full overflow-hidden">
	<div class="flex justify-between items-center mb-2 px-1">
		<h3 class="text-sm font-medium text-gray-700 dark:text-gray-200">Translation Models</h3>
		<div class="flex items-center">
			{#if (selectedEngine === 'helsinki' ? helsinkiDownloadedCount : nllbDownloadedCount) > 0}
				<span class="text-sm font-medium text-green-600 dark:text-green-400 uppercase">
					{selectedEngine === 'helsinki' ? helsinkiDownloadedCount : nllbDownloadedCount} {selectedEngine === 'helsinki' ? 'HELSINKI-NLP' : 'NLLB'} {(selectedEngine === 'helsinki' ? helsinkiDownloadedCount : nllbDownloadedCount) === 1 ? 'MODEL' : 'MODELS'} DOWNLOADED
				</span>
			{:else}
				<span class="text-sm font-medium text-red-600 dark:text-red-400 uppercase">NO {selectedEngine === 'helsinki' ? 'HELSINKI-NLP' : 'NLLB'} MODELS DOWNLOADED</span>
			{/if}
		</div>
	</div>

	<InstallLogModal bind:showModal={showLogModal} logs={modalLogs} isInstalling={isDownloading || isInstallingDependencies} isChecking={isChecking} title={isInstallingDependencies ? "Installing Dependencies" : "Downloading Translation Model"} inProgressText={isInstallingDependencies ? "Installing..." : "Downloading..."} />
	{#if configError}
		<p class="text-red-600 bg-red-100 dark:bg-red-900/20 dark:text-red-400 p-3 rounded-md text-sm text-left py-2 mb-4 break-words flex-shrink-0">
			<span class="font-medium">Error:</span> {configError}
		</p>
	{/if}

	<div class="bg-blue-50 dark:bg-blue-900/10 border border-blue-100 dark:border-blue-800 rounded-md p-3 mb-4 flex-shrink-0">
		<div class="flex items-center justify-between mb-2">
			<span class="text-sm font-semibold text-blue-800 dark:text-blue-300">Select Translation Engine</span>
			<div class="flex space-x-2">
				<button 
					class="px-3 py-1 text-xs rounded-full border transition-all"
					class:bg-blue-600={selectedEngine === 'helsinki'}
					class:text-white={selectedEngine === 'helsinki'}
					class:border-transparent={selectedEngine === 'helsinki'}
					class:bg-white={selectedEngine !== 'helsinki'}
					class:dark:bg-gray-800={selectedEngine !== 'helsinki'}
					class:text-gray-600={selectedEngine !== 'helsinki'}
					class:dark:text-gray-400={selectedEngine !== 'helsinki'}
					class:border-gray-200={selectedEngine !== 'helsinki'}
					class:dark:border-gray-700={selectedEngine !== 'helsinki'}
					on:click={() => handleEngineChange('helsinki')}
				>
					Helsinki-NLP
				</button>
				<button 
					class="px-3 py-1 text-xs rounded-full border transition-all"
					class:bg-blue-600={selectedEngine === 'nllb'}
					class:text-white={selectedEngine === 'nllb'}
					class:border-transparent={selectedEngine === 'nllb'}
					class:bg-white={selectedEngine !== 'nllb'}
					class:dark:bg-gray-800={selectedEngine !== 'nllb'}
					class:text-gray-600={selectedEngine !== 'nllb'}
					class:dark:text-gray-400={selectedEngine !== 'nllb'}
					class:border-gray-200={selectedEngine !== 'nllb'}
					class:dark:border-gray-700={selectedEngine !== 'nllb'}
					on:click={() => handleEngineChange('nllb')}
				>
					NLLB (Meta)
				</button>
			</div>
		</div>
		
		{#if selectedEngine === 'helsinki'}
			<div class="text-[11px] text-blue-700/80 dark:text-blue-400/80 leading-relaxed mb-2">
				<p><strong class="text-blue-800 dark:text-blue-300">Pros:</strong> Lightweight, very fast on CPU, high quality for common language pairs.</p>
				<p><strong class="text-blue-800 dark:text-blue-300">Cons:</strong> Requires separate model for every language pair (e.g. ja-en, fr-en).</p>
			</div>

			{#if !ct2Installed && translationModelCount > 0}
				<div class="bg-orange-50 dark:bg-orange-900/20 border border-orange-200 dark:border-orange-800 rounded p-2 text-[11px] text-orange-800 dark:text-orange-300 flex items-center justify-between">
					<div class="flex items-center">
						<svg class="w-3.5 h-3.5 mr-1.5 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
							<path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd"></path>
						</svg>
						<p>CTranslate2 backend is missing. Optimized translation is disabled.</p>
					</div>
                    <button class="bg-orange-600 hover:bg-orange-700 text-white px-2 py-1 rounded text-[10px] transition-colors" on:click={handleInstallDependencies} disabled={isInstallingDependencies}>
                        Install Now
                    </button>
				</div>
			{/if}
		{:else}
			<div class="text-[11px] text-blue-700/80 dark:text-blue-400/80 leading-relaxed">
				<p><strong class="text-blue-800 dark:text-blue-300">Pros:</strong> One model supports 200+ languages. Great for rare languages.</p>
				<p><strong class="text-blue-800 dark:text-blue-300">Cons:</strong> Very heavy resource usage, large file size, and slower on CPUs. Best with GPU.</p>
                {#if !ct2Installed && translationModelCount > 0}
                    <div class="mt-2 bg-orange-50 dark:bg-orange-900/20 border border-orange-200 dark:border-orange-800 rounded p-2 text-[11px] text-orange-800 dark:text-orange-300 flex items-center justify-between">
                        <div class="flex items-center">
                            <svg class="w-3.5 h-3.5 mr-1.5 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
                                <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd"></path>
                            </svg>
                            <p>CTranslate2 backend is missing. Translation may fail.</p>
                        </div>
                        <button class="bg-orange-600 hover:bg-orange-700 text-white px-2 py-1 rounded text-[10px] transition-colors" on:click={handleInstallDependencies} disabled={isInstallingDependencies}>
                            Install Now
                        </button>
                    </div>
                {/if}
			</div>
		{/if}
	</div>

	{#if !$configStatus.python_libraries_installed}
		<p class="text-orange-600 dark:text-orange-400 text-sm flex-shrink-0 px-1">
			Please install the required Python libraries first to enable model downloads.
		</p>
	{:else}
		<div class="flex flex-col space-y-3 flex-grow overflow-hidden px-1">
			<div class="flex space-x-4 mb-2 flex-shrink-0">
				<label class="inline-flex items-center cursor-pointer">
					<input type="radio" class="form-radio" name="translationOption" value="selectLanguages" bind:group={selectedOption} on:change={handleOptionChange}>
					<span class="ml-2 text-gray-700 dark:text-gray-300 font-medium">Browse Models</span>
				</label>
				<label class="inline-flex items-center cursor-pointer">
					<input type="radio" class="form-radio" name="translationOption" value="enterModelName" bind:group={selectedOption} on:change={handleOptionChange}>
					<span class="ml-2 text-gray-700 dark:text-gray-300 font-medium">Manual Entry</span>
				</label>
			</div>

			{#if selectedOption === 'selectLanguages'}
				<div class="flex flex-col space-y-3 h-full overflow-hidden">
					<div class="flex space-x-2 flex-shrink-0 p-0.5">
						<div class="relative flex-grow">
							<div class="absolute inset-y-0 left-0 pl-2.5 flex items-center pointer-events-none">
								<svg class="h-4 w-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path></svg>
							</div>
							<input
								type="text"
								bind:value={searchQuery}
								class="input w-full"
								style="padding-left: 2.25rem;"
								placeholder={selectedEngine === 'helsinki' ? "Search languages (e.g. 'French', 'en-ja')..." : "Search NLLB models..."}
								autocomplete="off"
								autocorrect="off"
								autocapitalize="off"
								spellcheck="false"
							/>
						</div>
					</div>

					<div class="border dark:border-gray-700 rounded-md flex-grow overflow-y-auto bg-gray-50 dark:bg-gray-800/50 p-2">
						<div class="space-y-2">
							{#each displayedModels as model (model.id)}
								{@const display = modelDisplayData[model.id] || { status: 'not_downloaded' }}
								{@const status = display.status}
								{@const isDownloadEnabled = !isBusy && downloadLocation && downloadLocation.trim() !== '' && $configStatus.python_libraries_installed}
								{@const isDeleteEnabled = !isBusy}
								{@const isCancelEnabled = status === 'downloading'}

								<div class="bg-white dark:bg-gray-800 border dark:border-gray-700 p-3 rounded-md shadow-sm flex flex-col hover:border-blue-400 transition-colors relative overflow-hidden">
									<div class="relative z-10 flex justify-between items-start">
										<div class="flex flex-col min-w-0 pr-4">
											<div class="flex items-center space-x-2">
												<span class="font-semibold text-gray-800 dark:text-gray-200 truncate">
													{#if model.family === 'helsinki' && model.src && model.tgt}
														{getLangName(model.src)} <span class="text-gray-400">→</span> {getLangName(model.tgt)}
													{:else}
														{formatModelDisplayName(model.id)}
													{/if}
												</span>
												<button 
													class="text-gray-400 hover:text-blue-500 dark:text-gray-500 dark:hover:text-blue-400 focus:outline-none p-0.5 rounded hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
													title="View on Hugging Face"
													on:click|stopPropagation={() => openExternal(`https://huggingface.co/${model.id}`)}
												>
													<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-box-arrow-up-right" viewBox="0 0 16 16">
														<path fill-rule="evenodd" d="M8.636 3.5a.5.5 0 0 0-.5-.5H1.5A1.5 1.5 0 0 0 0 4.5v10A1.5 1.5 0 0 0 1.5 16h10a1.5 1.5 0 0 0 1.5-1.5V7.864a.5.5 0 0 0-1 0V14.5a.5.5 0 0 1-.5.5h-10a.5.5 0 0 1-.5-.5v-10a.5.5 0 0 1 .5-.5h6.636a.5.5 0 0 0 .5-.5"/>
														<path fill-rule="evenodd" d="M16 .5a.5.5 0 0 0-.5-.5h-5a.5.5 0 0 0 0 1h3.793L6.146 9.146a.5.5 0 1 0 .708.708L15 1.707V5.5a.5.5 0 0 0 1 0z"/>
													</svg>
												</button>
												{#if status === 'complete'}
													<span class="px-1.5 py-0.5 rounded text-[10px] font-medium bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300">Installed</span>
												{/if}
											</div>
											<div class="text-xs text-gray-500 dark:text-gray-400 mt-0.5 flex items-center space-x-3">
												<span class="truncate" title={model.id}>{model.id}</span>
												{#if model.size}
													<span class="flex items-center text-gray-400" title="Size on disk">
														<span>&bull;</span>
														<span class="ml-1">{model.size}</span>
													</span>
												{/if}
												{#if model.downloads > 0}
													<span class="flex items-center" title="Downloads">
														<svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"></path></svg>
														{model.downloads.toLocaleString()}
													</span>
												{/if}
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
														{#if status === 'cancelling'}Cancelling...{:else}Downloading...{/if}
													</span>
													<button
														class="btn-cancel"
														on:click={() => handleCancel(model.id)}
														disabled={!isCancelEnabled}
														title="Cancel download">Cancel</button
													>
												</div>
											{:else if status === 'error'}
												<button
													class="btn-retry"
													on:click={() => handleDownload(model.id)}
													disabled={!isDownloadEnabled}
													title="Retry download">Retry</button
												>
											{:else if status === 'cancelled'}
												<button
													class="btn-blue-small"
													on:click={() => handleDownload(model.id)}
													disabled={!isDownloadEnabled}>Download</button
												>
											{:else}
												<button
													class="btn-blue-small"
													on:click={() => handleDownload(model.id)}
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
								<button on:click={handleRefreshModels} class="btn-blue-small px-4 py-2 text-sm" title="Refresh available translation models">
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
			{:else if selectedOption === 'enterModelName'}
				<div class="flex items-end space-x-2 flex-shrink-0">
					<div class="flex flex-col flex-grow">
						<label for="model-name" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Model Name</label>
						<input
							id="model-name"
							type="text"
							bind:value={modelName}
							autocomplete="off"
							autocorrect="off"
							class="input w-full"
							placeholder={selectedEngine === 'helsinki' ? "e.g. Helsinki-NLP/opus-mt-en-jap" : "e.g. facebook/nllb-200-distilled-600M"}
						/>
					</div>
					<button on:click={() => handleDownload(null)} class="btn-blue-small mb-0.5">
						{#if isDownloading}
							Downloading...
						{:else}
							Download
						{/if}
					</button>
				</div>
			{/if}
		</div>
	{/if}
</div>

<style lang="postcss">
	.input {
		@apply bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md px-2.5 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500;
	}
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