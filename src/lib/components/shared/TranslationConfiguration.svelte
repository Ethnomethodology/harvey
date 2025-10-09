<script>
	import { onMount, onDestroy } from 'svelte';
	import { ask } from '@tauri-apps/plugin-dialog';
	import { listen } from '@tauri-apps/api/event';
	import { open as openExternal } from '@tauri-apps/plugin-shell';
	import {
		downloadTranslationModel,
		deleteTranslationModel,
        getLocalTranslationModels,
		cancelTranslationModelDownload
	} from '$lib/services/configureActions';
	import { updateConfigStatus } from '$lib/stores/configStatusStore.js';
import notificationStore from '$lib/stores/notificationStore';
import { get } from 'svelte/store';
import { v4 as uuidv4 } from 'uuid'; // Import uuidv4
import InstallLogModal from '$lib/components/modals/InstallLogModal.svelte';
import Dropdown from '$lib/components/shared/Dropdown.svelte';

	export let downloadLocation = '';
	export let isBusy = false;
	export let translationModelCount = 0;

	let downloadedModels = [];
	$: translationModelCount = downloadedModels.length;
	let configError = '';
	let downloadStatus = {};

	let fromLanguage = 'en';
	let toLanguage = 'ja';

	let showLogModal = false;
	let modalLogs = [];
	let isDownloading = false;

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
							const modelName = event.payload;
							downloadStatus = { ...downloadStatus, [modelName]: 'complete' };
							try {
								downloadedModels = await getLocalTranslationModels();
								await updateConfigStatus();
							} catch (e) { console.error(`Failed to refresh models after ${modelName} completion:`, e); }
							modalLogs = [...modalLogs, { id: uuidv4(), message: `Download complete for ${modelName}.` }];
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

		if (fromLanguage === toLanguage) {
			notificationStore.add('From and To languages cannot be the same.', 'error');
			return;
		}

		configError = '';
		try {
			modalLogs = [];
            await downloadTranslationModel(fromLanguage, toLanguage, downloadLocation);
        } catch (err) {
			notificationStore.add(`Failed to start download for ${fromLanguage}-${toLanguage}: ${err.message || err}`, 'error');
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
			await updateConfigStatus();
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

                const fromLang = languages.find(lang => lang.value === fromCode);
                const toLang = languages.find(lang => lang.value === toCode);

                if (fromLang && toLang) {
                    return `${fromLang.label} to ${toLang.label} (${modelName})`;
                }
            }
        }
        return modelName; // Fallback for unexpected model name format
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
			<div class="flex items-end space-x-2">
				<div class="flex flex-col">
					<label for="from-language" class="block text-sm font-medium text-gray-700">From</label>
					<Dropdown
						id="from-language"
						options={languages}
						bind:value={fromLanguage}
						placeholder="Select language"
						containerClasses="w-32"
					/>
				</div>
				<div class="flex flex-col">
					<label for="to-language" class="block text-sm font-medium text-gray-700">To</label>
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
			<div>
				<h4 class="text-sm font-medium text-gray-700">Downloaded Models <span class="text-xs font-normal {downloadedModels.length === 0 ? 'text-yellow-600' : 'text-green-600'}">({downloadedModels.length} downloaded)</span></h4>
				<ul class="mt-2 space-y-2">
					{#each downloadedModels as model (model.name)}
						<li class="p-2 border rounded-md">
							<div class="flex items-center justify-between">
								<p class="text-sm font-medium text-gray-900">{formatModelDisplayName(model.name)}</p>
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