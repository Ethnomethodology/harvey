<script>
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { open } from '@tauri-apps/plugin-dialog';
	import { ask } from '@tauri-apps/plugin-dialog';
	import {
		saveDownloadLocation,
		getDownloadedModels,
		getDownloadLocation,
		moveModelsAndUpdateLocation
	} from '$lib/services/configureActions';

	import TranscriptionConfiguration from './TranscriptionConfiguration.svelte';
	import TranslationConfiguration from './TranslationConfiguration.svelte';
	import PythonLibrariesPanel from './PythonLibrariesPanel.svelte';
	import HuggingFacePanel from './HuggingFacePanel.svelte';
	import { configStatus } from '$lib/stores/configStatusStore.js';

	let activeTab = 'application'; // 'application', 'transcription', or 'translation'

	let downloadLocation = '';
	let isLoadingConfig = true;
	let configError = '';
	let isMovingModels = false;
	let statusMessage = '';

	let isTranscriptionBusy = false;
	let isTranslationBusy = false;
	let translationModelCount = 0;
	$: isBusy = isMovingModels || isTranscriptionBusy || isTranslationBusy;

	onMount(async () => {
		isLoadingConfig = true;
		configError = '';
		statusMessage = '';
		try {
			downloadLocation = await getDownloadLocation();
		} catch (e) {
			console.error('Error loading download location:', e);
			configError = `Failed to load configuration: ${e.message || e}`;
		} finally {
			isLoadingConfig = false;
		}
	});

	async function pickDownloadLocation() {
		if (isBusy) return;
		try {
			const selected = await open({ multiple: false, directory: true, title: 'Select Model Download Location', defaultPath: downloadLocation || undefined });
			if (!selected || typeof selected !== 'string') return;

			const newLocation = selected;
			if (newLocation === downloadLocation) {
				statusMessage = 'Selected location is the same as current.';
				setTimeout(() => (statusMessage = ''), 3000);
				return;
			}

            // TODO: This needs to be updated to also get downloaded translation models
			const currentModels = await getDownloadedModels();
			const modelsToMove = currentModels.length > 0 && downloadLocation;

			let confirmed = true;
			if (modelsToMove) {
				confirmed = await ask(`Change download location to:\n${newLocation}\n\nThis will move ${currentModels.length} downloaded model(s) from the current location to the new one. Proceed?`, { title: 'Confirm Location Change & Move', type: 'warning', okLabel: 'Yes, Move Files', cancelLabel: 'Cancel' });
			} else {
				confirmed = await ask(`Set download location to:\n${newLocation}\n\nNew models will be downloaded here.`, { title: 'Confirm Location Change', type: 'info', okLabel: 'Confirm', cancelLabel: 'Cancel' });
			}

			if (!confirmed) {
				statusMessage = 'Location change cancelled.';
				setTimeout(() => (statusMessage = ''), 3000);
				return;
			}

			isMovingModels = modelsToMove;
			statusMessage = modelsToMove ? 'Moving models and updating location...' : 'Updating location...';
			configError = '';

			try {
                // TODO: This function in the backend needs to be aware of both model types.
				if (modelsToMove) {
					await moveModelsAndUpdateLocation(newLocation);
					statusMessage = 'Download location updated and models moved successfully!';
				} else {
					await saveDownloadLocation(newLocation);
					statusMessage = 'Download location updated successfully!';
				}
				downloadLocation = newLocation;
			} catch (err) {
				configError = `Error changing location: ${err.message || err}`;
				statusMessage = '';
			} finally {
				isMovingModels = false;
				setTimeout(() => { if (!configError) { statusMessage = ''; } }, 5000);
			}
		} catch (err) {
			configError = `Error selecting directory: ${err.message || err}`;
			isMovingModels = false;
		}
	}
</script>

<div class="p-6 flex flex-col h-full bg-gray-50">
	<!-- Tab Navigation -->
	<div class="border-b border-gray-200 mb-6 flex-shrink-0">
		<nav class="-mb-px flex space-x-8" aria-label="Tabs">
            <button
				on:click={() => activeTab = 'application'}
				class="whitespace-nowrap py-3 px-1 border-b-2 font-medium text-sm transition-colors duration-150 ease-in-out focus:outline-none flex items-center space-x-2"
				class:border-blue-500={activeTab === 'application'}
				class:text-blue-600={activeTab === 'application'}
				class:border-transparent={activeTab !== 'application'}
				class:text-gray-500={activeTab !== 'application'}
				class:hover:text-gray-700={activeTab !== 'application'}
				class:hover:border-gray-300={activeTab !== 'application'}
				aria-current={activeTab === 'application' ? 'page' : undefined}
			>
				<span>Application</span>
				{#if !$configStatus.python_libraries_installed || !$configStatus.hf_token_present}
					<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="h-4 w-4 text-red-500" viewBox="0 0 16 16">
						<path d="M6.95.435c.58-.58 1.52-.58 2.1 0l6.515 6.516c.58.58.58 1.519 0 2.098L9.05 15.565c-.58.58-1.519.58-2.098 0L.435 9.05a1.48 1.48 0 0 1 0-2.098zm1.4.7a.495.495 0 0 0-.7 0L1.134 7.65a.495.495 0 0 0 0 .7l6.516 6.516a.495.495 0 0 0 .7 0l6.516-6.516a.495.495 0 0 0 0-.7L8.35 1.134z"/>
						<path d="M7.002 11a1 1 0 1 1 2 0 1 1 0 0 1-2 0M7.1 4.995a.905.905 0 1 1 1.8 0l-.35 3.507a.552.552 0 0 1-1.1 0z"/>
					</svg>
				{/if}
			</button>
			<button
				on:click={() => activeTab = 'transcription'}
				class="whitespace-nowrap py-3 px-1 border-b-2 font-medium text-sm transition-colors duration-150 ease-in-out focus:outline-none flex items-center space-x-2"
				class:border-blue-500={activeTab === 'transcription'}
				class:text-blue-600={activeTab === 'transcription'}
				class:border-transparent={activeTab !== 'transcription'}
				class:text-gray-500={activeTab !== 'transcription'}
				class:hover:text-gray-700={activeTab !== 'transcription'}
				class:hover:border-gray-300={activeTab !== 'transcription'}
				aria-current={activeTab === 'transcription' ? 'page' : undefined}
			>
				<span>Transcription</span>
				{#if !$configStatus.transcription_models_downloaded || !$configStatus.diarization_model_downloaded}
					<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="h-4 w-4 text-red-500" viewBox="0 0 16 16">
						<path d="M6.95.435c.58-.58 1.52-.58 2.1 0l6.515 6.516c.58.58.58 1.519 0 2.098L9.05 15.565c-.58.58-1.519.58-2.098 0L.435 9.05a1.48 1.48 0 0 1 0-2.098zm1.4.7a.495.495 0 0 0-.7 0L1.134 7.65a.495.495 0 0 0 0 .7l6.516 6.516a.495.495 0 0 0 .7 0l6.516-6.516a.495.495 0 0 0 0-.7L8.35 1.134z"/>
						<path d="M7.002 11a1 1 0 1 1 2 0 1 1 0 0 1-2 0M7.1 4.995a.905.905 0 1 1 1.8 0l-.35 3.507a.552.552 0 0 1-1.1 0z"/>
					</svg>
				{/if}
			</button>
			<button
				on:click={() => activeTab = 'translation'}
				class="whitespace-nowrap py-3 px-1 border-b-2 font-medium text-sm transition-colors duration-150 ease-in-out focus:outline-none flex items-center space-x-2"
				class:border-blue-500={activeTab === 'translation'}
				class:text-blue-600={activeTab === 'translation'}
				class:border-transparent={activeTab !== 'translation'}
				class:text-gray-500={activeTab !== 'translation'}
				class:hover:text-gray-700={activeTab !== 'translation'}
				class:hover:border-gray-300={activeTab !== 'translation'}
				aria-current={activeTab === 'translation' ? 'page' : undefined}
			>
				<span>Translation</span>
				{#if !$configStatus.translation_models_downloaded}
					<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="h-4 w-4 text-red-500" viewBox="0 0 16 16">
						<path d="M6.95.435c.58-.58 1.52-.58 2.1 0l6.515 6.516c.58.58.58 1.519 0 2.098L9.05 15.565c-.58.58-1.519.58-2.098 0L.435 9.05a1.48 1.48 0 0 1 0-2.098zm1.4.7a.495.495 0 0 0-.7 0L1.134 7.65a.495.495 0 0 0 0 .7l6.516 6.516a.495.495 0 0 0 .7 0l6.516-6.516a.495.495 0 0 0 0-.7L8.35 1.134z"/>
						<path d="M7.002 11a1 1 0 1 1 2 0 1 1 0 0 1-2 0M7.1 4.995a.905.905 0 1 1 1.8 0l-.35 3.507a.552.552 0 0 1-1.1 0z"/>
					</svg>
				{/if}
			</button>
		</nav>
	</div>

	<!-- Tab Content Area -->
	<div class="flex-grow min-h-0 overflow-y-auto pr-2 -mr-2">
		{#if activeTab === 'application'}
            <div class="p-1">
                {#if isLoadingConfig}
                    <p class="text-gray-500 text-center py-4">Loading configuration...</p>
                {:else if configError}
                    <p class="text-red-600 bg-red-100 p-3 rounded-md text-sm text-left py-2 mb-4 break-words flex-shrink-0">
						<span class="font-medium">Error:</span> {configError}
					</p>
                {/if}

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
                    {#if isBusy && !isMovingModels}
                        <p class="text-xs text-gray-500 mt-1">
							Download in progress. Cannot change location now.
						</p>
                    {/if}
                    {#if statusMessage}
                        <p class="text-xs text-indigo-600 mt-1">{statusMessage}</p>
                    {/if}
                </div>

				<div class="mb-6">
					<h3 class="block text-sm font-medium text-gray-700 mb-1">Required Tools</h3>
					<PythonLibrariesPanel />
					<HuggingFacePanel />
				</div>
            </div>
		{:else if activeTab === 'transcription'}
			<TranscriptionConfiguration bind:isBusy={isTranscriptionBusy} {downloadLocation} />
		{:else if activeTab === 'translation'}
			<TranslationConfiguration bind:isBusy={isTranslationBusy} {downloadLocation} bind:translationModelCount={translationModelCount} />
		{/if}
	</div>
</div>

<style lang="postcss">
	.btn-blue {
		@apply px-2.5 py-1.5 border text-sm font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-1 transition-colors duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed;
		@apply border-transparent text-white bg-blue-600 hover:bg-blue-700 focus:ring-blue-500;
	}

	.overflow-y-auto::-webkit-scrollbar { width: 6px; }
	.overflow-y-auto::-webkit-scrollbar-track { background: transparent; }
	.overflow-y-auto::-webkit-scrollbar-thumb { background-color: rgba(156, 163, 175, 0.5); border-radius: 10px; border: 2px solid transparent; background-clip: content-box; }
	.overflow-y-auto::-webkit-scrollbar-thumb:hover { background-color: rgba(107, 114, 128, 0.6); }
	.overflow-y-auto { scrollbar-width: thin; scrollbar-color: rgba(156, 163, 175, 0.5) transparent; }
</style>