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

	let activeTab = 'application'; // 'application', 'transcription', or 'translation'

	let downloadLocation = '';
	let isLoadingConfig = true;
	let configError = '';
	let isMovingModels = false;
	let statusMessage = '';
	let isFfmpegInstalled = true;

	let isTranscriptionBusy = false;
	let isTranslationBusy = false;
	$: isBusy = isMovingModels || isTranscriptionBusy || isTranslationBusy;

	onMount(async () => {
		isLoadingConfig = true;
		configError = '';
		statusMessage = '';
		try {
			isFfmpegInstalled = await invoke('check_ffmpeg_installed');
			downloadLocation = await getDownloadLocation();
		} catch (e) {
			console.error('Error loading download location:', e);
			isFfmpegInstalled = false;
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
				class="whitespace-nowrap py-3 px-1 border-b-2 font-medium text-sm transition-colors duration-150 ease-in-out focus:outline-none"
				class:border-blue-500={activeTab === 'application'}
				class:text-blue-600={activeTab === 'application'}
				class:border-transparent={activeTab !== 'application'}
				class:text-gray-500={activeTab !== 'application'}
				class:hover:text-gray-700={activeTab !== 'application'}
				class:hover:border-gray-300={activeTab !== 'application'}
				aria-current={activeTab === 'application' ? 'page' : undefined}
			>
				Application
			</button>
			<button
				on:click={() => activeTab = 'transcription'}
				class="whitespace-nowrap py-3 px-1 border-b-2 font-medium text-sm transition-colors duration-150 ease-in-out focus:outline-none"
				class:border-blue-500={activeTab === 'transcription'}
				class:text-blue-600={activeTab === 'transcription'}
				class:border-transparent={activeTab !== 'transcription'}
				class:text-gray-500={activeTab !== 'transcription'}
				class:hover:text-gray-700={activeTab !== 'transcription'}
				class:hover:border-gray-300={activeTab !== 'transcription'}
				aria-current={activeTab === 'transcription' ? 'page' : undefined}
			>
				Transcription
			</button>
			<button
				on:click={() => activeTab = 'translation'}
				class="whitespace-nowrap py-3 px-1 border-b-2 font-medium text-sm transition-colors duration-150 ease-in-out focus:outline-none"
				class:border-blue-500={activeTab === 'translation'}
				class:text-blue-600={activeTab === 'translation'}
				class:border-transparent={activeTab !== 'translation'}
				class:text-gray-500={activeTab !== 'translation'}
				class:hover:text-gray-700={activeTab !== 'translation'}
				class:hover:border-gray-300={activeTab !== 'translation'}
				aria-current={activeTab === 'translation' ? 'page' : undefined}
			>
				Translation
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

				{#if !isFfmpegInstalled}
					<p
						class="text-red-600 bg-red-100 p-3 rounded-md text-sm text-left py-2 mb-4 break-words flex-shrink-0"
					>
						<span class="font-medium">Dependency Error:</span> FFmpeg is not installed or could not be
						found in your system's PATH. FFmpeg is required for all audio and video processing.
						Please install it to continue.
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
            </div>
		{:else if activeTab === 'transcription'}
			<TranscriptionConfiguration bind:isBusy={isTranscriptionBusy} {downloadLocation} />
		{:else if activeTab === 'translation'}
			<TranslationConfiguration bind:isBusy={isTranslationBusy} {downloadLocation} />
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