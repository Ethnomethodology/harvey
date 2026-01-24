<script>
	import { onMount, onDestroy } from 'svelte';
	import { ask } from '@tauri-apps/plugin-dialog';
	import { listen } from '@tauri-apps/api/event';
	import { open as openExternal } from '@tauri-apps/plugin-shell';
	import { configStatus } from '$lib/stores/configStatusStore.js';
	import {
		downloadTranslationModel,
		deleteTranslationModel,
        getLocalTranslationModels,
		cancelTranslationModelDownload,
		fetchAvailableModels
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

	let modelName = '';

	let showLogModal = false;
	let modalLogs = [];
	let isDownloading = false;

	let selectedOption = 'selectLanguages'; // Default to selecting languages

	// --- Marketplace / Search View State ---
	let availableModelsList = [];
	let filteredModels = [];
	let searchQuery = '';
	let isFetchingModels = false;
	let hasFetched = false;
	let autoFetchTriggered = false;

	$: isAnyModelDownloading = Object.values(downloadStatus).includes('downloading');
    $: isBusy = isAnyModelDownloading;

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

	async function handleRefreshModels() {
		if (isFetchingModels) return;
		isFetchingModels = true;
		// searchQuery = ''; // Preserving search query so filter applies after fetch
		try {
			const fetched = await fetchAvailableModels();
			if (fetched && Array.isArray(fetched) && fetched.length > 0) {
				// Sort by downloads descending
				availableModelsList = fetched.sort((a, b) => b.downloads - a.downloads);
				hasFetched = true;
				notificationStore.add(`Successfully fetched ${fetched.length} models.`, 'success');
			} else {
				notificationStore.add('Fetched model list was empty.', 'warning');
			}
		} catch (e) {
			notificationStore.add(`Failed to refresh models: ${e.message}`, 'error');
		} finally {
			isFetchingModels = false;
		}
	}

	// Filter logic
	$: {
		if (searchQuery.trim() === '') {
			filteredModels = availableModelsList.slice(0, 2000); // Show up to 2000 models
			autoFetchTriggered = false; // Reset trigger when cleared
		} else {
			const q = searchQuery.toLowerCase();
			
			// Auto-fetch if user searches before list is loaded
			if (!hasFetched && !isFetchingModels && availableModelsList.length === 0 && !autoFetchTriggered) {
				autoFetchTriggered = true;
				handleRefreshModels();
			}

			filteredModels = availableModelsList.filter(m => {
				const srcName = languageMap.get(m.src)?.toLowerCase() || '';
				const tgtName = languageMap.get(m.tgt)?.toLowerCase() || '';
				return (
					m.id.toLowerCase().includes(q) ||
					m.src?.toLowerCase().includes(q) ||
					m.tgt?.toLowerCase().includes(q) ||
					srcName.includes(q) ||
					tgtName.includes(q)
				);
			}).slice(0, 2000); // Limit results for performance
		}
	}

	onMount(async () => {
		configError = '';
		try {
			downloadedModels = await getLocalTranslationModels();
		} catch (e) {
			configError = `Failed to load model configuration: ${e.message || e}`;
		}

		try {
			unlistenStart = await listen('translation-download-start', (event) => {
				const modelName = event.payload;
				downloadStatus = { ...downloadStatus, [modelName]: 'downloading' };
				modalLogs = [...modalLogs, { id: uuidv4(), message: `Starting download for ${modelName}...` }];
				isDownloading = true;
				showLogModal = true;
			});
			unlistenLog = await listen('translation-download-log', (event) => {
				const { model_name, log_line } = event.payload;
				if (downloadStatus[model_name] === 'downloading') {
					modalLogs = [...modalLogs, { id: uuidv4(), message: log_line }];
				}
			});
			unlistenComplete = await listen('translation-download-complete', async (event) => {
				const downloadedModelName = event.payload;
				downloadStatus = { ...downloadStatus, [downloadedModelName]: 'complete' };
				try {
					downloadedModels = await getLocalTranslationModels();
					setTranslationModelsDownloaded(downloadedModels.length > 0);
				} catch (e) { console.error(`Failed to refresh models after ${downloadedModelName} completion:`, e); }
				modalLogs = [...modalLogs, { id: uuidv4(), message: `Download complete for ${downloadedModelName}.` }];
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
			});

			unlistenFinished = await listen('translation-download-finished', () => {
				console.log('Frontend: Received translation-download-finished event. Setting isDownloading to false.');
				isDownloading = false;
			});
		} catch (err) {
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

	async function handleDownload(targetModelName) {
		if (isBusy) return;
		if (!downloadLocation || downloadLocation.trim() === '') {
			notificationStore.add('Please set a valid model download location first.', 'error');
			return;
		}

		configError = '';
		const modelToDownload = targetModelName || modelName.trim();
		
		if (!modelToDownload) {
			notificationStore.add('Please enter a model name.', 'error');
			return;
		}

		try {
			modalLogs = [];
			await downloadTranslationModel(null, null, downloadLocation, modelToDownload);
		} catch (err) {
			notificationStore.add(`Failed to start download for ${modelToDownload}: ${err.message || err}`, 'error');
			isDownloading = false; 
		}
	}

    async function handleDelete(model) {
		if (isBusy) return;
		if (!model?.name) { notificationStore.add("Cannot delete model: Missing name.", 'error'); return; }
		const confirmed = await ask(`Are you sure you want to delete the model "${model.name}"? This will remove the entire model folder from disk.`, { title: 'Confirm Deletion', type: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' });
		if (!confirmed) return;
		try {
			await deleteTranslationModel(model); 
			downloadedModels = await getLocalTranslationModels(); 
            downloadStatus = { ...downloadStatus, [model.name]: 'not_downloaded' };
			setTranslationModelsDownloaded(downloadedModels.length > 0);
		} catch (err) {
			notificationStore.sendNotification(`Failed to delete model ${model.name}: ${err.message || err}`, 'error');
		}
    }

    function formatModelDisplayName(modelName) {
        const parts = modelName.split('/');
        if (parts.length === 2) {
            const langParts = parts[1].split('-');
            if (langParts.length >= 3 && langParts[0] === 'opus' && langParts[1] === 'mt') {
                const fromCode = langParts[langParts.length - 2];
                const toCode = langParts[langParts.length - 1];

                const fromLang = languageMap.get(fromCode);
                const toLang = languageMap.get(toCode);

                if (fromLang && toLang) {
                    return `${fromLang} to ${toLang} (${modelName})`;
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
	</script>

<div class="flex flex-col h-full overflow-hidden">
	<InstallLogModal bind:showModal={showLogModal} logs={modalLogs} isInstalling={isDownloading} title="Downloading Translation Model" inProgressText="Downloading..." />
	{#if configError}
		<p class="text-red-600 bg-red-100 dark:bg-red-900/20 dark:text-red-400 p-3 rounded-md text-sm text-left py-2 mb-4 break-words flex-shrink-0">
			<span class="font-medium">Error:</span> {configError}
		</p>
	{/if}

	<p class="text-sm text-gray-600 dark:text-gray-400 mb-4 flex-shrink-0">
		Harvey uses open-source <a href="https://huggingface.co/Helsinki-NLP" target="_blank" rel="noopener noreferrer" class="text-blue-600 dark:text-blue-400 hover:underline">Helsinki-NLP models</a> for offline translation.
	</p>
	{#if !$configStatus.python_libraries_installed}
		<p class="text-orange-600 dark:text-orange-400 text-sm flex-shrink-0">
			Please install the required Python libraries first to enable model downloads.
		</p>
	{:else}
		<div class="flex flex-col space-y-3 flex-grow overflow-hidden">
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
				<div class="flex flex-col space-y-2 h-full overflow-hidden">
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
								placeholder="Search languages (e.g. 'French', 'en-ja')..."
								autocomplete="off"
								autocorrect="off"
								autocapitalize="off"
								spellcheck="false"
							/>
						</div>
					</div>

					<div class="border dark:border-gray-700 rounded-md flex-grow overflow-y-auto bg-gray-50 dark:bg-gray-800/50 p-2">
						{#if !hasFetched && availableModelsList.length === 0}
							<div class="flex flex-col items-center justify-center h-full text-gray-500 space-y-3">
								<button on:click={handleRefreshModels} class="btn-blue-small px-4 py-2 text-sm" title="Fetch model list from Hugging Face">
									{#if isFetchingModels}
										Fetching available models...
									{:else}
										List models from HuggingFace
									{/if}
								</button>
							</div>
						{:else if filteredModels.length === 0}
							<div class="flex flex-col items-center justify-center h-full text-gray-500">
								<p>No models found matching "{searchQuery}".</p>
							</div>
						{:else}
							<div class="space-y-2">
								{#each filteredModels as model (model.id)}
									<div class="bg-white dark:bg-gray-800 border dark:border-gray-700 p-3 rounded-md shadow-sm flex justify-between items-center hover:border-blue-400 transition-colors">
										<div class="flex flex-col min-w-0 pr-4">
											<div class="flex items-center space-x-2">
												<span class="font-semibold text-gray-800 dark:text-gray-200 truncate">
													{#if model.src && model.tgt}
														{getLangName(model.src)} <span class="text-gray-400">→</span> {getLangName(model.tgt)}
													{:else}
														{model.id}
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
												{#if isModelDownloaded(model.id)}
													<span class="px-1.5 py-0.5 rounded text-[10px] font-medium bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300">Installed</span>
												{/if}
											</div>
											<div class="text-xs text-gray-500 dark:text-gray-400 mt-0.5 flex items-center space-x-3">
												<span class="truncate" title={model.id}>{model.id}</span>
												<span class="flex items-center" title="Downloads">
													<svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"></path></svg>
													{model.downloads.toLocaleString()}
												</span>
											</div>
										</div>
										<button 
											on:click={() => handleDownload(model.id)} 
											disabled={isModelDownloaded(model.id) || isBusy}
											class="btn-blue-small flex-shrink-0 disabled:opacity-50 disabled:bg-gray-400"
										>
											{#if isModelDownloaded(model.id)}
												Added
											{:else}
												Download
											{/if}
										</button>
									</div>
								{/each}
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
							placeholder="e.g. Helsinki-NLP/opus-mt-en-jap"
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

			<div class="pt-2 flex-shrink-0 border-t dark:border-gray-700 mt-2">
				<h4 class="text-sm font-semibold text-gray-700 dark:text-gray-200">
					Downloaded Models 
					<span class="text-xs font-normal {downloadedModels.length === 0 ? 'text-yellow-600 dark:text-yellow-400' : 'text-green-600 dark:text-green-400'}">
						({downloadedModels.length} installed)
					</span>
				</h4>
				<ul class="mt-2 space-y-2 max-h-32 overflow-y-auto">
					{#each downloadedModels as model (model.name)}
						<li class="p-2 border dark:border-gray-700 rounded-md bg-white dark:bg-gray-800">
							<div class="flex items-center justify-between">
								<p class="text-sm font-medium text-gray-900 dark:text-gray-200 truncate pr-2">{formatModelDisplayName(model.name)}</p>
								<button on:click={() => handleDelete(model)} class="btn-delete flex-shrink-0">Delete</button>
							</div>
						</li>
					{/each}
				</ul>
			</div>
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