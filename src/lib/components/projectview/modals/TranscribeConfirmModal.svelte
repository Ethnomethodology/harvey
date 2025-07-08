<!-- src/lib/components/projectview/modals/TranscribeConfirmModal.svelte -->
<script>
	import { createEventDispatcher, onDestroy } from 'svelte';
	import { get } from 'svelte/store';
	import { CheckCircle, XCircle, Clock, Loader } from 'lucide-svelte';
	import { transcriptStore } from '$lib/stores/transcriptStore.js';
	// Only import getCloudModelLabel, languageOptions will come as a prop
	import { getCloudModelLabel } from '$lib/constants/transcriptionOptions.js';
	import SpeakersModal from './SpeakersModal.svelte';

	// Props
	export let fileName = '';
	export let downloadedModelsList = [];
	export let cloudConfig = null;
	export let languageOptions = []; // Expect languageOptions as a prop, default to empty array
	export let speakers = { count: 0, names: [], translatedNames: [] };


	const dispatch = createEventDispatcher();

	// Local state for editable fields
	let modalSelectedModel = '';
	let modalSelectedLanguage = 'auto';
	let modalTranslateToEnglish = false;
	let modalEnableDiarization = false;
	let modalSpeakersConfig = { count: 0, names: [], translatedNames: [] };
	let showNestedSpeakersModal = false;

	// Event Handlers
	function handleConfirm() {
		dispatch('confirmStart', {
			selectedModel: modalSelectedModel,
			selectedLanguage: modalSelectedLanguage,
			translateToEnglish: modalTranslateToEnglish,
			enableDiarization: modalEnableDiarization,
			speakersConfig: modalSpeakersConfig
		});
	}

	function handleCancelRequest() {
		dispatch('cancelRequest');
	}

	function handleCloseAndReset() {
		dispatch('closeAndReset');
	}

	function handleRunInBackgroundAndClose() {
		dispatch('runInBackgroundAndClose');
	}

	// --- Reactive Derivations from Store for UI ---
	// $: console.log('[ModalDebug] Store State:', $transcriptStore); // Uncomment for deep debugging

	// $: {
	// 	console.log(`[JULES-DEBUG Modal Env] showModal=${showModal}`); // Prop
	// }

	// $: {
	// 	if ($transcriptStore.showTranscribeModal) {
	// 		console.log(`[JULES-DEBUG Modal React] Store state: isTranscribing=${$transcriptStore.isTranscribing}, jobStatus='${$transcriptStore.transcriptionJobStatus}', jobID='${$transcriptStore.transcriptionJobId ? $transcriptStore.transcriptionJobId.substring(0,8) : null}', progressMsg='${$transcriptStore.transcriptionProgress.message}', errorMsg='${$transcriptStore.transcriptionErrorMessage}'`);
	// 	}
	// }

	$: showModal = $transcriptStore.showTranscribeModal;
	$: isTranscribing = $transcriptStore.isTranscribing;
	$: jobStatus = $transcriptStore.transcriptionJobStatus;
	$: progressPercent = $transcriptStore.transcriptionProgress.percent;
	$: progressMessage = $transcriptStore.transcriptionProgress.message;
	$: currentErrorMessage = $transcriptStore.transcriptionErrorMessage;
	$: currentJobId = $transcriptStore.transcriptionJobId;

	// When the modal is about to show the confirm view, initialize local states from the store
	$: if (showModal && !isTranscribing && jobStatus === null) {
		modalSelectedModel = $transcriptStore.selectedModelName || (downloadedModelsList.length > 0 ? downloadedModelsList[0].name : (cloudConfig?.model || ''));
		modalSelectedLanguage = $transcriptStore.selectedLanguage || 'auto';
		modalTranslateToEnglish = $transcriptStore.translateToEnglish;
		modalEnableDiarization = $transcriptStore.diarizationEnabledForNextJob;
		// Initialize modalSpeakersConfig from the speakers prop (which comes from transcriptStore initially)
		// Use a deep copy to prevent direct mutation of the prop or store value until confirmed.
		modalSpeakersConfig = JSON.parse(JSON.stringify(speakers || { count: 0, names: [], translatedNames: [] }));
	}

	// --- Title Logic ---
	$: modalTitle = (!isTranscribing && jobStatus === null) ? 'Confirm Transcription Settings' :
					 (isTranscribing && jobStatus === 'initiating') ? 'Initiating Transcription...' :
					 (isTranscribing && jobStatus === 'running') ? `Transcription Status${currentJobId ? ` (Job: ${currentJobId.substring(0, 8)})` : ''}` :
					 (jobStatus === 'cancelling') ? `Cancelling Job${currentJobId ? ` (${currentJobId.substring(0, 8)})` : ''}` : // Added 'cancelling'
					 (!isTranscribing && jobStatus === 'done') ? 'Transcription Complete' :
					 (!isTranscribing && jobStatus === 'error') ? 'Transcription Error' :
					 (!isTranscribing && jobStatus === 'cancelled') ? 'Transcription Cancelled' :
					 'Transcription Status';

    // Keyboard handling (optional, can be simplified or removed if not strictly needed by new design)
	function handleKeydown(event) {
		if (showModal && event.key === 'Escape') {
			// If transcribing and running/initiating, do nothing on Escape.
			if (isTranscribing && (jobStatus === 'running' || jobStatus === 'initiating')) {
				// Explicitly do nothing to prevent closure
				event.preventDefault(); // Also prevent any default browser behavior for Escape if modal is focused
				return;
			} else if (!isTranscribing && (jobStatus === 'done' || jobStatus === 'error' || jobStatus === 'cancelled' || jobStatus === null)) {
				// For terminal states or initial confirm state, allow close.
				handleCloseAndReset();
			}
			// Other states (e.g., 'cancelling') will also do nothing, which is fine.
		}
	}

	// $: if (showModal && typeof window !== 'undefined') {
	// 	window.addEventListener('keydown', handleKeydown);
	// } else if (typeof window !== 'undefined') {
	// 	window.removeEventListener('keydown', handleKeydown);
	// }
    // The global listener is removed. Keydown is now handled by the main div.

	onDestroy(() => {
        // if (typeof window !== 'undefined') {
        //      window.removeEventListener('keydown', handleKeydown);
        // }
        // Global listener cleanup is removed.
    });

</script>

{#if showModal}
	<div
		class="fixed inset-0 z-[120] flex items-center justify-center bg-black/50 backdrop-blur-sm"
		role="dialog"
		aria-modal="true"
		aria-labelledby="transcribe-modal-title"
		on:click={() => {
            if (!(isTranscribing && (jobStatus === 'running' || jobStatus === 'initiating'))) {
                handleCloseAndReset();
            }
        }}
        tabindex="-1"
        on:keydown={handleKeydown}
	>
		<div
			class="bg-white dark:bg-gray-800 rounded-lg shadow-xl p-6 w-full max-w-md text-gray-800 dark:text-gray-200 flex flex-col"
			role="document"
            tabindex="-1"
            on:click|stopPropagation
		>
			<h2 id="transcribe-modal-title" class="text-lg font-semibold mb-4 text-center">{modalTitle}</h2>

			{#if !isTranscribing && jobStatus === null}
				<!-- CONFIRM VIEW -->
				<div class="space-y-3 text-sm mb-5 text-gray-700 dark:text-gray-300 max-h-[60vh] overflow-y-auto pr-2">
					<div><strong>File:</strong> <span class="font-mono break-all ml-2">{fileName || 'N/A'}</span></div>

					<div class="space-y-1">
						<label for="modalModelSelect" class="block font-medium text-gray-900 dark:text-gray-100">Model:</label>
						<select id="modalModelSelect" class="ui-select w-full" bind:value={modalSelectedModel}>
							<option value="" disabled selected={!modalSelectedModel}>Select Model</option>
							{#if downloadedModelsList.length > 0}
							<optgroup label="Local Models">
								{#each downloadedModelsList as model (model.name)}
									<option value="{model.name}">{model.name}</option>
								{/each}
							</optgroup>
							{/if}
							{#if cloudConfig?.consent && cloudConfig.api_key && cloudConfig.model}
								{@const configuredCloudModelId = cloudConfig.model}
								{@const configuredCloudModelLabel = getCloudModelLabel(configuredCloudModelId)}
								<optgroup label="Cloud Models">
									<option value="{configuredCloudModelId}">{configuredCloudModelLabel} ☁️</option>
								</optgroup>
							{/if}
							{#if downloadedModelsList.length === 0 && !(cloudConfig?.consent && cloudConfig.api_key && cloudConfig.model)}
								<option value="" disabled>No models available</option>
							{/if}
						</select>
					</div>

					<div class="space-y-1">
						<label for="modalLanguageSelect" class="block font-medium text-gray-900 dark:text-gray-100">Language:</label>
						<select id="modalLanguageSelect" class="ui-select w-full" bind:value={modalSelectedLanguage}>
							{#each languageOptions as lang (lang.value)}
								<option value="{lang.value}">{lang.label}</option>
							{/each}
						</select>
					</div>

					<div class="flex items-center space-x-2 pt-1"> 
						<input type="checkbox" id="modalTranslateToEnglishCheckbox" class="ui-checkbox" bind:checked={modalTranslateToEnglish} disabled={modalSelectedLanguage === 'en'} />
						<label for="modalTranslateToEnglishCheckbox" class="text-sm text-gray-700 dark:text-gray-300 cursor-pointer select-none" class:opacity-50={modalSelectedLanguage === 'en'}>
							Translate to English
						</label>
					</div>

					<div class="pt-1 space-y-1"> 
						<div class="flex justify-between items-center">
							<div>
								<strong>Speakers:</strong>
								<span>{modalSpeakersConfig?.count > 0 ? modalSpeakersConfig.count : '0'}</span>
							</div>
							<button
								type="button"
								class="btn-xs-secondary"
								on:click={() => showNestedSpeakersModal = true}
							>
								Edit Speakers
							</button>
						</div>
						{#if modalSpeakersConfig?.count > 0 && modalSpeakersConfig.names && modalSpeakersConfig.names.length > 0}
						<div class="pl-4">
							<p class="text-xs text-gray-500 dark:text-gray-400 break-all">
								({modalSpeakersConfig.names.join(', ')})
							</p>
						</div>
						{/if}
					</div>

					<div class="pt-2">
						<div class="flex items-center space-x-2">
							<input type="checkbox" id="modalEnableDiarizationCheckbox" class="ui-checkbox" bind:checked={modalEnableDiarization}/>
							<label for="modalEnableDiarizationCheckbox" class="text-sm text-gray-700 dark:text-gray-300 cursor-pointer select-none">
								Identify different speakers (diarize)
							</label>
						</div>
						{#if modalEnableDiarization}
							<p class="text-xs mt-1.5 ml-0.5 px-2 py-1 rounded bg-yellow-300 text-black dark:bg-yellow-500 dark:text-black">
								Note: Speaker identification can significantly increase transcription time.
							</p>
						{/if}
					</div>
				</div>
				<div class="flex justify-end space-x-3 mt-auto pt-4 border-t border-gray-200 dark:border-gray-700">
					<button class="btn-secondary" on:click={handleCloseAndReset}>Cancel</button>
					<button class="btn-primary" on:click={handleConfirm} disabled={!modalSelectedModel || !modalSelectedLanguage}>Start Transcription</button>
				</div>

			{:else if isTranscribing && (jobStatus === 'running' || jobStatus === 'initiating')}
				<!-- RUNNING OR INITIATING VIEW -->
				<div class="flex flex-col items-center space-y-4 mb-6">
                    <div class="w-16 h-16">
                        <Loader class="w-full h-full text-blue-500 animate-spin" />
                    </div>
					<!-- Progress bar removed -->
					<p class="text-xs text-center text-gray-600 dark:text-gray-400 h-4">
                        {progressMessage || (jobStatus === 'initiating' ? 'Preparing...' : (jobStatus === 'running' ? 'Processing...' : 'Please wait...'))}
					</p>
				</div>
				<div class="flex justify-center space-x-2 mt-auto">
					<button class="btn-secondary" on:click={handleRunInBackgroundAndClose} disabled={jobStatus === 'initiating'}>
						Run in background
					</button>
					<button class="btn-action-cancel" on:click={handleCancelRequest} disabled={jobStatus === 'initiating'}>
						Request Cancellation
					</button>
				</div>

			{:else if jobStatus === 'cancelling'}
				<!-- CANCELLING VIEW (Added this state based on logic) -->
				<div class="flex flex-col items-center space-y-4 mb-6">
                    <div class="w-16 h-16">
                        <Clock class="w-full h-full text-orange-500" />
                    </div>
					<p class="text-xs text-center text-gray-600 dark:text-gray-400 h-4">
						{progressMessage || 'Attempting to cancel...'}
					</p>
				</div>
				<div class="flex justify-center space-x-2 mt-auto">
					<button class="btn-secondary" disabled>Cancelling...</button>
				</div>

			{:else if !isTranscribing && jobStatus === 'done'}
				<!-- DONE VIEW -->
				<div class="flex flex-col items-center space-y-3 mb-6 text-center">
					<CheckCircle class="w-16 h-16 text-green-500" />
					<p class="text-sm font-medium">{progressMessage || 'Transcription Complete!'}</p>
				</div>
				<div class="flex justify-center mt-auto">
					<button class="btn-primary" on:click={handleCloseAndReset}>Close</button>
				</div>

			{:else if !isTranscribing && jobStatus === 'cancelled'}
                <!-- CANCELLED VIEW -->
				<div class="flex flex-col items-center space-y-3 mb-6 text-center">
					<XCircle class="w-16 h-16 text-orange-500" />
					<p class="text-sm font-medium">{progressMessage || 'Transcription Cancelled'}</p>
				</div>
				<div class="flex justify-center mt-auto">
					<button class="btn-secondary" on:click={handleCloseAndReset}>Close</button>
				</div>

			{:else if !isTranscribing && jobStatus === 'error'}
				<!-- ERROR VIEW -->
				<div class="flex flex-col items-center space-y-3 mb-6 text-center">
					<XCircle class="w-16 h-16 text-red-500" />
					<p class="text-sm font-medium">An Error Occurred</p>
					<p class="text-xs bg-red-100 dark:bg-red-900/50 border border-red-300 dark:border-red-700 text-red-700 dark:text-red-300 p-2 rounded w-full text-left overflow-x-auto max-h-32">
						{currentErrorMessage || 'Unknown error during transcription.'}
					</p>
				</div>
				<div class="flex justify-center mt-auto">
					<button class="btn-secondary" on:click={handleCloseAndReset}>Close</button>
				</div>
			{:else}
				<!-- Fallback or initial brief loading state if necessary -->
				<div class="flex flex-col items-center space-y-4 py-8">
					<Loader class="w-12 h-12 text-gray-400 animate-spin" />
					<p class="text-sm text-gray-500">Loading status...</p>
				</div>
			{/if}
		</div>
	</div>
{/if}

{#if showNestedSpeakersModal}
	<SpeakersModal
		bind:showModal={showNestedSpeakersModal}
		currentSpeakers={modalSpeakersConfig}
		on:confirm={(e) => {
			modalSpeakersConfig = e.detail; // Update local config
			showNestedSpeakersModal = false;
		}}
		on:close={() => showNestedSpeakersModal = false}
	/>
{/if}

<style>
	/* Styles remain unchanged */
	.btn-primary,
	.btn-secondary,
	.btn-action-cancel {
		padding: 0.5rem 1rem;
		border: none;
		border-radius: 0.375rem; /* 6px */
		cursor: pointer;
		font-size: 0.875rem; /* 14px */
		font-weight: 500;
		transition: background-color 0.15s ease-in-out, opacity 0.15s ease-in-out;
		white-space: nowrap;
		display: inline-flex;
		align-items: center;
		justify-content: center;
	}
	.btn-primary {
		background-color: #3b82f6; /* bg-blue-500 */
		color: white;
	}
	.btn-primary:hover:not(:disabled) {
		background-color: #2563eb; /* hover:bg-blue-600 */
	}
	.btn-primary:disabled {
		opacity: 0.6;
		cursor: not-allowed;
		/* background-color: #9ca3af; Let default browser style handle disabled bg */
	}
	.btn-secondary {
		background-color: #e5e7eb; /* bg-gray-200 */
		color: #374151; /* text-gray-700 */
		border: 1px solid #d1d5db; /* border-gray-300 */
	}
	.dark .btn-secondary {
		background-color: #4b5563; /* dark:bg-gray-600 */
		color: #e5e7eb; /* dark:text-gray-200 */
		border-color: #6b7280; /* dark:border-gray-500 */
	}
	.btn-secondary:hover:not(:disabled) {
		background-color: #d1d5db; /* hover:bg-gray-300 */
	}
	.dark .btn-secondary:hover:not(:disabled) {
		background-color: #6b7280; /* dark:hover:bg-gray-500 */
	}
	.btn-secondary:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
	.btn-action-cancel {
		background-color: #ef4444; /* bg-red-500 */
		color: white;
	}
	.btn-action-cancel:hover:not(:disabled) {
		background-color: #dc2626; /* hover:bg-red-600 */
	}
	.btn-action-cancel:disabled {
		opacity: 0.6;
		cursor: not-allowed;
		/* background-color: #fca5a5; Let default browser style handle disabled bg */
	}
    .dark .btn-action-cancel {
         background-color: #dc2626; /* dark:bg-red-600 */
    }
    .dark .btn-action-cancel:hover:not(:disabled) {
        background-color: #b91c1c; /* dark:hover:bg-red-700 */
    }

	/* Checkbox style */
	.ui-checkbox {
		@apply w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600;
	}
	.ui-select {
		@apply block w-full pl-3 pr-10 py-2 text-sm border-gray-300 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 rounded-md dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500;
	}
	.btn-xs-secondary {
		@apply px-2 py-1 text-xs font-medium rounded border;
		@apply bg-gray-100 hover:bg-gray-200 text-gray-700 border-gray-300;
		@apply dark:bg-gray-700 dark:hover:bg-gray-600 dark:text-gray-200 dark:border-gray-500;
		@apply focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-1 dark:focus:ring-offset-gray-800;
	}
</style>