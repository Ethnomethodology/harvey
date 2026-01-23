<!-- src/lib/components/projectview/modals/TranscribeConfirmModal.svelte -->
<script>
	import { createEventDispatcher, onDestroy } from 'svelte';
	import { get } from 'svelte/store';
	import { CheckCircle, XCircle, Clock, Loader } from 'lucide-svelte';
	import { transcriptStore } from '$lib/stores/transcriptStore.js';
	import SpeakersModal from './SpeakersModal.svelte';
	import Dropdown from '$lib/components/shared/Dropdown.svelte';

	// Props
	export let fileName = '';
	export let downloadedModelsList = [];
	export let languageOptions = []; // Expect languageOptions as a prop, default to empty array
	export let speakers = { count: 0, names: [], translatedNames: [] };
    export let mediaDuration = 0; // Added for manual mode validation
    export let lastSegmentEndTime = 0; // Added for manual mode validation

	const dispatch = createEventDispatcher();

	// Local state for editable fields
	let modalSelectedModel = '';
	let modalTranscriptionMode = 'automatic'; // Kept for store compatibility if needed, but UI uses modalTab
    let modalTab = 'automatic'; // 'automatic' | 'manual'
	let modalSelectedLanguage = 'auto';
	
	let modalEnableDiarization = false;
	let modalSpeakersConfig = { count: 0, names: [], translatedNames: [] };
	let showNestedSpeakersModal = false;

    // Manual Mode State
    let manualSegmentCount = 1;
    let manualSegmentDuration = 60;
    let manualSpeakerMode = 'unselected';

    // Derived state for manual validation
    $: totalDurationNeeded = manualSegmentCount * manualSegmentDuration;
    $: availableSpace = Math.max(0, mediaDuration - lastSegmentEndTime);
    $: isManualDurationValid = totalDurationNeeded <= availableSpace + 0.001; // tolerance

    function formatDuration(seconds) {
        if (!seconds && seconds !== 0) return '0s';
        const m = Math.floor(seconds / 60);
        const s = Math.floor(seconds % 60);
        return `${m}m ${s}s`;
    }

	// Event Handlers
	function handleConfirm() {
        if (modalTab === 'automatic') {
            dispatch('confirmStart', {
                transcriptionMode: 'automatic',
                selectedModel: modalSelectedModel,
                selectedLanguage: modalSelectedLanguage,
                enableDiarization: modalEnableDiarization,
                speakersConfig: modalSpeakersConfig
            });
        } else {
            dispatch('confirmStart', {
                transcriptionMode: 'manual',
                manualSettings: {
                    segmentCount: manualSegmentCount,
                    segmentDuration: manualSegmentDuration,
                    speakerMode: manualSpeakerMode,
                    startTime: lastSegmentEndTime
                },
                speakersConfig: modalSpeakersConfig // Pass speakers config even in manual for alternations
            });
        }
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
	$: showModal = $transcriptStore.showTranscribeModal;
	$: isTranscribing = $transcriptStore.isTranscribing;
	$: jobStatus = $transcriptStore.transcriptionJobStatus;
	$: progressPercent = $transcriptStore.transcriptionProgress.percent;
	$: progressMessage = $transcriptStore.transcriptionProgress.message;
	$: currentErrorMessage = $transcriptStore.transcriptionErrorMessage;
	$: currentJobId = $transcriptStore.transcriptionJobId;

	let isInitialized = false;
	// When the modal is about to show the confirm view, initialize local states from the store
	$: if (showModal && !isTranscribing && jobStatus === null && !isInitialized) {
		modalSelectedModel = $transcriptStore.selectedModelName || (downloadedModelsList.length > 0 ? downloadedModelsList[0].name : '');
		modalSelectedLanguage = $transcriptStore.selectedLanguage || 'auto';
        modalTab = $transcriptStore.transcriptionMode || 'automatic'; // Initialize tab from store
		
		modalEnableDiarization = $transcriptStore.diarizationEnabledForNextJob;
		// Initialize modalSpeakersConfig from the speakers prop (which comes from transcriptStore initially)
		// Use a deep copy to prevent direct mutation of the prop or store value until confirmed.
		modalSpeakersConfig = JSON.parse(JSON.stringify(speakers || { count: 0, names: [], translatedNames: [] }));
        
        // Initialize manual settings from store if available
        if ($transcriptStore.manualSegmentSettings) {
            manualSegmentDuration = $transcriptStore.manualSegmentSettings.duration || 60;
            manualSpeakerMode = $transcriptStore.manualSegmentSettings.speakerMode || 'unselected';
        }
        
		isInitialized = true;
	}

	// Reset the initialization flag when the modal is closed
	$: if (!showModal) {
		isInitialized = false;
	}

	// --- Title Logic ---
	$: modalTitle = (!isTranscribing && jobStatus === null) ? 'Transcription Settings' :
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
			class="bg-white dark:bg-surface-2 rounded-lg shadow-xl p-6 w-full max-w-md text-gray-800 dark:text-gray-200 flex flex-col"
			role="document"
            tabindex="-1"
            on:click|stopPropagation
		>
			<h2 id="transcribe-modal-title" class="text-lg font-semibold mb-4 text-center">{modalTitle}</h2>

			{#if !isTranscribing && jobStatus === null}
				<!-- CONFIRM VIEW -->
                
                <!-- Tabs -->
                <div class="flex border-b border-gray-200 dark:border-gray-700 mb-4">
                    <button
                        class="flex-1 py-2 text-sm font-medium text-center border-b-2 focus:outline-none transition-colors {modalTab === 'automatic' ? 'border-blue-500 text-blue-600 dark:text-blue-400' : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'}"
                        on:click={() => modalTab = 'automatic'}
                    >
                        Automatic
                    </button>
                    <button
                        class="flex-1 py-2 text-sm font-medium text-center border-b-2 focus:outline-none transition-colors {modalTab === 'manual' ? 'border-blue-500 text-blue-600 dark:text-blue-400' : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'}"
                        on:click={() => modalTab = 'manual'}
                    >
                        Manual
                    </button>
                </div>

				<div class="space-y-3 text-sm mb-5 text-gray-700 dark:text-gray-300 max-h-[60vh] overflow-y-auto pr-2">
					<div><strong>File:</strong> <span class="font-mono break-all ml-2">{fileName || 'N/A'}</span></div>

                    {#if modalTab === 'automatic'}
                        <!-- AUTOMATIC SETTINGS -->
                        <div class="space-y-1">
                            <label for="modalModelSelect" class="block font-medium text-gray-900 dark:text-gray-100">Model:</label>
                            <Dropdown
                                containerClasses="w-full"
                                options={downloadedModelsList.map(m => ({ value: m.name, label: m.name }))}
                                bind:value={modalSelectedModel}
                                placeholder="Select a Model"
                                disabled={downloadedModelsList.length === 0}
                            />
                        </div>

                        <div class="space-y-1">
                            <label for="modalLanguageSelect" class="block font-medium text-gray-900 dark:text-gray-100">Language:</label>
                            <Dropdown
                                containerClasses="w-full"
                                options={languageOptions}
                                bind:value={modalSelectedLanguage}
                                placeholder="Select a Language"
                            />
                        </div>

                        <div class="pt-2">
                            <div class="flex items-center space-x-2">
                                <input type="checkbox" id="modalEnableDiarizationCheckbox" class="ui-checkbox" bind:checked={modalEnableDiarization} autocomplete="off" autocorrect="off"/>
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

                    {:else}
                        <!-- MANUAL SETTINGS -->
                        <div class="p-3 bg-gray-50 dark:bg-surface-3 rounded border border-gray-200 dark:border-gray-700 mb-2">
                            <div class="flex justify-between mb-1">
                                <span>Available Space:</span>
                                <span class="font-medium">{formatDuration(availableSpace)}</span>
                            </div>
                        </div>

                        <div class="space-y-1">
                            <label for="manualSegCount" class="block font-medium text-gray-900 dark:text-gray-100">Number of Segments:</label>
                            <input
                                id="manualSegCount"
                                type="number"
                                min="1"
                                max="100"
                                bind:value={manualSegmentCount}
                                class="ui-select w-full"
                            />
                        </div>

                        <div class="space-y-1">
                            <label for="manualSegDuration" class="block font-medium text-gray-900 dark:text-gray-100">Duration (seconds):</label>
                            <div class="flex items-center gap-2">
                                <input
                                    id="manualSegDuration"
                                    type="number"
                                    min="1"
                                    bind:value={manualSegmentDuration}
                                    class="ui-select w-full"
                                />
                                <span class="text-xs text-gray-500 whitespace-nowrap min-w-[4rem]">
                                    ({formatDuration(manualSegmentDuration)})
                                </span>
                            </div>
                        </div>

                        <div class="pt-2">
                            <label class="block font-medium text-gray-900 dark:text-gray-100 mb-1">Speaker Assignment:</label>
                            <div class="flex gap-4">
                                <label class="inline-flex items-center cursor-pointer">
                                    <input type="radio" group={manualSpeakerMode} value="unselected" class="ui-checkbox rounded-full">
                                    <span class="ml-2">Unselected</span>
                                </label>
                                <label class="inline-flex items-center cursor-pointer">
                                    <input type="radio" group={manualSpeakerMode} value="alternate" class="ui-checkbox rounded-full" disabled={modalSpeakersConfig.names.length < 2}>
                                    <span class="ml-2" class:opacity-50={modalSpeakersConfig.names.length < 2}>Alternate</span>
                                </label>
                            </div>
                            {#if manualSpeakerMode === 'alternate' && modalSpeakersConfig.names.length < 2}
                                <p class="text-xs text-red-500 mt-1">Need at least 2 speakers configured.</p>
                            {/if}
                        </div>

                        <div class="mt-2 p-2 rounded {isManualDurationValid ? 'bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300' : 'bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-300'}">
                            <div class="flex justify-between text-xs">
                                <span>Total New Time:</span>
                                <span class="font-bold">{formatDuration(totalDurationNeeded)}</span>
                            </div>
                            {#if !isManualDurationValid}
                                <p class="text-xs mt-1 font-medium">Exceeds available space!</p>
                            {/if}
                        </div>
                    {/if}

					<div class="pt-1 space-y-1 border-t border-gray-200 dark:border-border mt-3">
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

				</div>
				<div class="flex justify-end space-x-3 mt-auto pt-4 border-t border-gray-200 dark:border-border">
					<button class="btn-secondary" on:click={handleCloseAndReset}>Cancel</button>
					<button 
                        class="btn-primary" 
                        on:click={handleConfirm} 
                        disabled={
                            (modalTab === 'automatic' && (!modalSelectedModel || !modalSelectedLanguage)) ||
                            (modalTab === 'manual' && !isManualDurationValid)
                        }
                    >
                        {modalTab === 'automatic' ? 'Start Transcription' : 'Add Segments'}
                    </button>
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