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
		cancelTranslationModelDownload
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

	let fromLanguage = 'en';
	let toLanguage = 'ja';
	let modelName = '';

	let showLogModal = false;
	let modalLogs = [];
	let isDownloading = false;

	let selectedOption = 'selectLanguages'; // Default to selecting languages

	const languages = [
		{ value: 'en', label: 'English' },
		{ value: 'ja', label: 'Japanese' },
	];

	$: isAnyModelDownloading = Object.values(downloadStatus).includes('downloading');
    $: isBusy = isAnyModelDownloading;

	let unlistenStart = null;
	let unlistenLog = null;
	let unlistenComplete = null;
	let unlistenError = null;
    let unlistenFinished = null;

	function handleOptionChange() {
		if (selectedOption === 'selectLanguages' && modelName.trim() !== '') {
			modelName = ''; // Clear model name if switching to language selection
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

	async function handleDownload() {
		if (isBusy) return;
		if (!downloadLocation || downloadLocation.trim() === '') {
			notificationStore.add('Please set a valid model download location first.', 'error');
			return;
		}

		configError = '';
		try {
			modalLogs = [];
			if (selectedOption === 'enterModelName') {
				if (!modelName.trim()) {
					notificationStore.add('Please enter a model name.', 'error');
					return;
				}
				await downloadTranslationModel(null, null, downloadLocation, modelName.trim());
			} else { // selectedOption === 'selectLanguages'
				if (fromLanguage === toLanguage) {
					notificationStore.add('From and To languages cannot be the same.', 'error');
					return;
				}
				await downloadTranslationModel(fromLanguage, toLanguage, downloadLocation);
			}
        } catch (err) {
			const errorMessage = selectedOption === 'enterModelName'
				? `Failed to start download for ${modelName}: ${err.message || err}`
				: `Failed to start download for ${fromLanguage}-${toLanguage}: ${err.message || err}`;
			notificationStore.add(errorMessage, 'error');
            isDownloading = false; // Set to false on error
		}
    }

    async function handleDelete(model) {
		if (isBusy) return;
		if (!model?.name) { notificationStore.add("Cannot delete model: Missing name.", 'error'); return; }
		const confirmed = await ask(`Are you sure you want to delete the model "${model.name}"? This will remove the entire model folder from disk.`, { title: 'Confirm Deletion', type: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' });
		if (!confirmed) return;
		try {
			await deleteTranslationModel(model); // Use deleteTranslationModel
			downloadedModels = await getLocalTranslationModels(); // Fix: Use getLocalTranslationModels
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
        return modelName; // Fallback for unexpected model name format
    }</script>

<div class="flex flex-col h-full">
	<InstallLogModal bind:showModal={showLogModal} logs={modalLogs} isInstalling={isDownloading} title="Downloading Translation Model" inProgressText="Downloading..." />
	{#if configError}
		<p class="text-red-600 bg-red-100 dark:bg-red-900/20 dark:text-red-400 p-3 rounded-md text-sm text-left py-2 mb-4 break-words flex-shrink-0">
			<span class="font-medium">Error:</span> {configError}
		</p>
	{/if}

	<p class="text-sm text-gray-600 dark:text-gray-400 mb-4">
		Harvey uses open-source <a href="https://huggingface.co/Helsinki-NLP" target="_blank" rel="noopener noreferrer" class="text-blue-600 dark:text-blue-400 hover:underline">Helsinki-NLP models</a> for offline translation. To translate transcripts, you must first download one of the available models.
	</p>
	{#if !$configStatus.python_libraries_installed}
		<p class="text-orange-600 dark:text-orange-400 text-sm">
			Please install the required Python libraries first to enable model downloads.
		</p>
	{:else}
		<div class="flex-grow space-y-3">
			<div class="pt-4 space-y-3">
				<div class="flex space-x-4 mb-4">
				<label class="inline-flex items-center">
					<input type="radio" class="form-radio" name="translationOption" value="selectLanguages" bind:group={selectedOption} on:change={handleOptionChange}>
					<span class="ml-2 text-gray-700 dark:text-gray-300">Select Languages</span>
				</label>
				<label class="inline-flex items-center">
					<input type="radio" class="form-radio" name="translationOption" value="enterModelName" bind:group={selectedOption} on:change={handleOptionChange}>
					<span class="ml-2 text-gray-700 dark:text-gray-300">Enter Model Name</span>
				</label>
			</div>

			{#if selectedOption === 'selectLanguages'}
				<div class="flex items-end space-x-2">
					<div class="flex flex-col">
						<label for="from-language" class="block text-sm font-medium text-gray-700 dark:text-gray-300">From</label>
						<Dropdown
							id="from-language"
							options={languages}
							bind:value={fromLanguage}
							placeholder="Select language"
							containerClasses="w-32"
						/>
					</div>
					<div class="flex flex-col">
						<label for="to-language" class="block text-sm font-medium text-gray-700 dark:text-gray-300">To</label>
						<Dropdown
							id="to-language"
							options={languages}
							bind:value={toLanguage}
							placeholder="Select language"
							containerClasses="w-32"
						/>
					</div>
					<button on:click={handleDownload} class="btn-blue-small">
						{#if isDownloading}
							Downloading...
						{:else}
							Download
						{/if}
					</button>
				</div>
			{:else if selectedOption === 'enterModelName'}
				<div class="flex items-end space-x-2">
					<div class="flex flex-col">
						<label for="model-name" class="block text-sm font-medium text-gray-700 dark:text-gray-300">Model Name</label>
						<input
							id="model-name"
							type="text"
							bind:value={modelName}

							class="input w-64"
							placeholder="e.g. Helsinki-NLP/opus-mt-en-jap"
						/>
					</div>
					<button on:click={handleDownload} class="btn-blue-small">
						{#if isDownloading}
							Downloading...
						{:else}
							Download
						{/if}
										</button>
									</div>
								{/if}
								
		</div>
		<div class="pt-4">
		<h4 class="mt-2 text-sm font-semibold text-gray-700 dark:text-gray-200">Downloaded Models <span class="text-xs font-normal {downloadedModels.length === 0 ? 'text-yellow-600 dark:text-yellow-400' : 'text-green-600 dark:text-green-400'}">({downloadedModels.length} downloaded)</span></h4>
								<ul class="mt-2 space-y-2">
									{#each downloadedModels as model (model.name)}
										<li class="p-2 border dark:border-gray-700 rounded-md">
											<div class="flex items-center justify-between">
												<p class="text-sm font-medium text-gray-900 dark:text-gray-200">{formatModelDisplayName(model.name)}</p>
												<button on:click={() => handleDelete(model)} class="btn-delete">Delete</button>
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
		@apply bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md px-2.5 py-1 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500;
	}
	.btn-blue-small, .btn-delete, .btn-cancel, .btn-retry {
		@apply px-2.5 py-1.5 border text-sm font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-1 transition-colors duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed;
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