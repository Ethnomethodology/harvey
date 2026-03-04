<script>
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { open } from '@tauri-apps/plugin-dialog';
	import { ask } from '@tauri-apps/plugin-dialog';
	import Dropdown from '$lib/components/shared/Dropdown.svelte';
	import { themePreference } from '$lib/stores/themeStore.js';
	import {
		saveDownloadLocation,
		getDownloadedModels,
		getAllDownloadedModels,
		getDownloadLocation,
		moveModelsAndUpdateLocation
	} from '$lib/services/configureActions';

	import TranscriptionConfiguration from './TranscriptionConfiguration.svelte';
	import TranslationConfiguration from './TranslationConfiguration.svelte';
	import DiarizationModelPanel from './DiarizationModelPanel.svelte';
	import AdvancedConfiguration from './AdvancedConfiguration.svelte';
	import LibrariesPanel from './LibrariesPanel.svelte';
	import HuggingFacePanel from './HuggingFacePanel.svelte';
	import { configStatus, updateConfigStatus } from '$lib/stores/configStatusStore.js';

	let activeTab = 'application'; // 'application', 'transcription', 'diarization', 'translation', 'advanced'
	let isWinArm64 = false;
	let isFFmpegInstalled = false;
	let downloadLocation = '';
	let isLoadingConfig = true;
	let configError = '';
	let isMovingModels = false;
	let statusMessage = '';

	let isTranscriptionBusy = false;
	let isTranslationBusy = false;
	let isAdvancedBusy = false;
	let translationModelCount = 0;
	$: isBusy = isMovingModels || isTranscriptionBusy || isTranslationBusy || isAdvancedBusy;

	onMount(async () => {
		updateConfigStatus(true); // Force a refresh when the component mounts
		isLoadingConfig = true;
		configError = '';
		statusMessage = '';
		try {
			downloadLocation = await getDownloadLocation();
		} catch (e) {
			console.error('Error loading configuration:', e);
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

			const currentModels = await getAllDownloadedModels();
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

<div class="p-4 flex flex-col h-full bg-gray-50 dark:bg-gray-950 dark:text-gray-200">
	<!-- Tab Navigation -->
	<div class="mb-6 flex-shrink-0">
		<nav class="flex flex-wrap gap-2" aria-label="Tabs">
            <button
				on:click={() => activeTab = 'application'}
				class="px-2.5 py-1.5 font-medium text-sm rounded-md transition-colors duration-150 ease-in-out focus:outline-none flex items-center space-x-2"
				class:bg-blue-100={activeTab === 'application'}
				class:text-blue-700={activeTab === 'application'}
				class:dark:bg-blue-900={activeTab === 'application'}
				class:dark:text-blue-300={activeTab === 'application'}
				class:text-gray-500={activeTab !== 'application'}
				class:dark:text-gray-400={activeTab !== 'application'}
				class:hover:text-gray-700={activeTab !== 'application'}
				class:dark:hover:text-gray-200={activeTab !== 'application'}
				class:hover:bg-gray-100={activeTab !== 'application'}
				class:dark:hover:bg-gray-800={activeTab !== 'application'}
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
				class="px-2.5 py-1.5 font-medium text-sm rounded-md transition-colors duration-150 ease-in-out focus:outline-none flex items-center space-x-2"
				class:bg-blue-100={activeTab === 'transcription'}
				class:text-blue-700={activeTab === 'transcription'}
				class:dark:bg-blue-900={activeTab === 'transcription'}
				class:dark:text-blue-300={activeTab === 'transcription'}
				class:text-gray-500={activeTab !== 'transcription'}
				class:dark:text-gray-400={activeTab !== 'transcription'}
				class:hover:text-gray-700={activeTab !== 'transcription'}
				class:dark:hover:text-gray-200={activeTab !== 'transcription'}
				class:hover:bg-gray-100={activeTab !== 'transcription'}
				class:dark:hover:bg-gray-800={activeTab !== 'transcription'}
				aria-current={activeTab === 'transcription' ? 'page' : undefined}
			>
				<span>Transcription</span>
				{#if ($configStatus.selected_transcription_engine === 'whisper-cpp' && (!$configStatus.whisper_cpp_installed || !$configStatus.whisper_cpp_models_downloaded)) || ($configStatus.selected_transcription_engine === 'faster-whisper' && (!$configStatus.faster_whisper_dependencies_installed || !$configStatus.faster_whisper_models_downloaded || !$configStatus.python_libraries_installed))}
					<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="h-4 w-4 text-yellow-500" viewBox="0 0 16 16">
						<path d="M6.95.435c.58-.58 1.52-.58 2.1 0l6.515 6.516c.58.58.58 1.519 0 2.098L9.05 15.565c-.58.58-1.519.58-2.098 0L.435 9.05a1.48 1.48 0 0 1 0-2.098zm1.4.7a.495.495 0 0 0-.7 0L1.134 7.65a.495.495 0 0 0 0 .7l6.516 6.516a.495.495 0 0 0 .7 0l6.516-6.516a.495.495 0 0 0 0-.7L8.35 1.134z"/>
						<path d="M7.002 11a1 1 0 1 1 2 0 1 1 0 0 1-2 0M7.1 4.995a.905.905 0 1 1 1.8 0l-.35 3.507a.552.552 0 0 1-1.1 0z"/>
					</svg>
				{/if}
			</button>
			<button
				on:click={() => activeTab = 'diarization'}
				class="px-2.5 py-1.5 font-medium text-sm rounded-md transition-colors duration-150 ease-in-out focus:outline-none flex items-center space-x-2"
				class:bg-blue-100={activeTab === 'diarization'}
				class:text-blue-700={activeTab === 'diarization'}
				class:dark:bg-blue-900={activeTab === 'diarization'}
				class:dark:text-blue-300={activeTab === 'diarization'}
				class:text-gray-500={activeTab !== 'diarization'}
				class:dark:text-gray-400={activeTab !== 'diarization'}
				class:hover:text-gray-700={activeTab !== 'diarization'}
				class:dark:hover:text-gray-200={activeTab !== 'diarization'}
				class:hover:bg-gray-100={activeTab !== 'diarization'}
				class:dark:hover:bg-gray-800={activeTab !== 'diarization'}
				aria-current={activeTab === 'diarization' ? 'page' : undefined}
			>
				<span>Diarization</span>
				{#if !$configStatus.diarization_model_downloaded}
					<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="h-4 w-4 text-yellow-500" viewBox="0 0 16 16">
						<path d="M6.95.435c.58-.58 1.52-.58 2.1 0l6.515 6.516c.58.58.58 1.519 0 2.098L9.05 15.565c-.58.58-1.519.58-2.098 0L.435 9.05a1.48 1.48 0 0 1 0-2.098zm1.4.7a.495.495 0 0 0-.7 0L1.134 7.65a.495.495 0 0 0 0 .7l6.516 6.516a.495.495 0 0 0 .7 0l6.516-6.516a.495.495 0 0 0 0-.7L8.35 1.134z"/>
						<path d="M7.002 11a1 1 0 1 1 2 0 1 1 0 0 1-2 0M7.1 4.995a.905.905 0 1 1 1.8 0l-.35 3.507a.552.552 0 0 1-1.1 0z"/>
					</svg>
				{/if}
			</button>
			<button
				on:click={() => activeTab = 'translation'}
				class="px-2.5 py-1.5 font-medium text-sm rounded-md transition-colors duration-150 ease-in-out focus:outline-none flex items-center space-x-2"
				class:bg-blue-100={activeTab === 'translation'}
				class:text-blue-700={activeTab === 'translation'}
				class:dark:bg-blue-900={activeTab === 'translation'}
				class:dark:text-blue-300={activeTab === 'translation'}
				class:text-gray-500={activeTab !== 'translation'}
				class:dark:text-gray-400={activeTab !== 'translation'}
				class:hover:text-gray-700={activeTab !== 'translation'}
				class:dark:hover:text-gray-200={activeTab !== 'translation'}
				class:hover:bg-gray-100={activeTab !== 'translation'}
				class:dark:hover:bg-gray-800={activeTab !== 'translation'}
				aria-current={activeTab === 'translation' ? 'page' : undefined}
			>
				<span>Translation</span>
				{#if ($configStatus.selected_translation_engine === 'helsinki' && !$configStatus.helsinki_models_downloaded) || ($configStatus.selected_translation_engine === 'nllb' && !$configStatus.nllb_models_downloaded)}
					<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="h-4 w-4 text-yellow-500" viewBox="0 0 16 16">
						<path d="M6.95.435c.58-.58 1.52-.58 2.1 0l6.515 6.516c.58.58.58 1.519 0 2.098L9.05 15.565c-.58.58-1.519.58-2.098 0L.435 9.05a1.48 1.48 0 0 1 0-2.098zm1.4.7a.495.495 0 0 0-.7 0L1.134 7.65a.495.495 0 0 0 0 .7l6.516 6.516a.495.495 0 0 0 .7 0l6.516-6.516a.495.495 0 0 0 0-.7L8.35 1.134z"/>
						<path d="M7.002 11a1 1 0 1 1 2 0 1 1 0 0 1-2 0M7.1 4.995a.905.905 0 1 1 1.8 0l-.35 3.507a.552.552 0 0 1-1.1 0z"/>
					</svg>
				{/if}
			</button>
			<button
				on:click={() => activeTab = 'advanced'}
				class="px-2.5 py-1.5 font-medium text-sm rounded-md transition-colors duration-150 ease-in-out focus:outline-none flex items-center space-x-2"
				class:bg-blue-100={activeTab === 'advanced'}
				class:text-blue-700={activeTab === 'advanced'}
				class:dark:bg-blue-900={activeTab === 'advanced'}
				class:dark:text-blue-300={activeTab === 'advanced'}
				class:text-gray-500={activeTab !== 'advanced'}
				class:dark:text-gray-400={activeTab !== 'advanced'}
				class:hover:text-gray-700={activeTab !== 'advanced'}
				class:dark:hover:text-gray-200={activeTab !== 'advanced'}
				class:hover:bg-gray-100={activeTab !== 'advanced'}
				class:dark:hover:bg-gray-800={activeTab !== 'advanced'}
				aria-current={activeTab === 'advanced' ? 'page' : undefined}
			>
				<span>Advanced</span>
			</button>
		</nav>
	</div>

	<!-- Tab Content Area -->
	<div class="flex-grow min-h-0 overflow-y-auto pr-2 -mr-2">
		{#if activeTab === 'application'}
            <div class="p-1">
                {#if isLoadingConfig}
                    <p class="text-gray-500 dark:text-gray-400 text-center py-4">Loading configuration...</p>
                {:else if configError}
                    <p class="text-red-600 bg-red-100 dark:bg-red-900/20 dark:text-red-400 p-3 rounded-md text-sm text-left py-2 mb-4 break-words flex-shrink-0">
						<span class="font-medium">Error:</span> {configError}
					</p>
                {/if}

				<div class="mb-6 flex-shrink-0">
					<label for="theme-select" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Theme</label>
					<Dropdown
						containerClasses="w-48"
						options={[{value: 'system', label: 'System'}, {value: 'light', label: 'Light'}, {value: 'dark', label: 'Dark'}]}
						bind:value={$themePreference}
					/>
				</div>

                <div class="mb-6 flex-shrink-0">
                    <label for="download-location-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
						Local Model Download Location
					</label>
                    <div class="flex items-center space-x-2">
                        <input
							id="download-location-input"
							type="text"
							bind:value={downloadLocation}
							class="input w-full flex-grow"
							readonly
							placeholder="Set a location..."
							title={downloadLocation || 'No location set'}
							autocomplete="off"
							autocorrect="off"
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
                        <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
							Download in progress. Cannot change location now.
						</p>
                    {/if}
                    {#if statusMessage}
                        <p class="text-xs text-indigo-600 dark:text-indigo-400 mt-1">{statusMessage}</p>
                    {/if}
                </div>

				<div class="mb-6">
					<h3 class="block text-sm font-semibold text-gray-700 dark:text-gray-200 mb-1">Required Tools</h3>
					<LibrariesPanel />
					<HuggingFacePanel />
				</div>
            </div>
		{:else if activeTab === 'transcription'}
			<TranscriptionConfiguration bind:isBusy={isTranscriptionBusy} {downloadLocation} />
		{:else if activeTab === 'diarization'}
			<div class="p-1">
				<DiarizationModelPanel arePythonLibrariesInstalled={$configStatus.python_libraries_installed} />
			</div>
		{:else if activeTab === 'translation'}
			<TranslationConfiguration bind:isBusy={isTranslationBusy} {downloadLocation} bind:translationModelCount={translationModelCount} />
		{:else if activeTab === 'advanced'}
			<AdvancedConfiguration bind:isBusy={isAdvancedBusy} />
		{/if}
	</div>
</div>

<style lang="postcss">
	.input {
		@apply bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-md px-2.5 py-1 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 dark:text-gray-200 placeholder:text-gray-400 dark:placeholder:text-gray-500;
	}
	.input:read-only {
		@apply bg-gray-100 dark:bg-gray-700 dark:text-gray-400 cursor-not-allowed;
	}
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