<!-- src/lib/components/projectview/modals/TranscribeConfirmModal.svelte -->
<script>
	import { createEventDispatcher, onDestroy } from 'svelte';
	import { CheckCircle, XCircle, Clock, Loader } from 'lucide-svelte';
	import { transcriptStore } from '$lib/stores/transcriptStore.js';

	// Props
	export let fileName = ''; // Still needed for confirm view
	export let modelName = ''; // Still needed for confirm view
	export let language = ''; // Still needed for confirm view
	export let speakers = { count: 0, names: [] }; // Still needed for confirm view

	const dispatch = createEventDispatcher();

	// Event Handlers
	function handleConfirm() {
		dispatch('confirmStart');
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

	$: {
		console.log(`[JULES-DEBUG Modal Env] showModal=${showModal}`); // Prop
	}

	$: {
		if ($transcriptStore.showTranscribeModal) {
			console.log(`[JULES-DEBUG Modal React] Store state: isTranscribing=${$transcriptStore.isTranscribing}, jobStatus='${$transcriptStore.transcriptionJobStatus}', jobID='${$transcriptStore.transcriptionJobId ? $transcriptStore.transcriptionJobId.substring(0,8) : null}', progressMsg='${$transcriptStore.transcriptionProgress.message}', errorMsg='${$transcriptStore.transcriptionErrorMessage}'`);
		}
	}

	$: showModal = $transcriptStore.showTranscribeModal;
	$: isTranscribing = $transcriptStore.isTranscribing;
	$: jobStatus = $transcriptStore.transcriptionJobStatus;
	$: progressPercent = $transcriptStore.transcriptionProgress.percent;
	$: progressMessage = $transcriptStore.transcriptionProgress.message;
	$: currentErrorMessage = $transcriptStore.transcriptionErrorMessage;
	$: currentJobId = $transcriptStore.transcriptionJobId;


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
			// Decide if Escape should trigger 'closeAndReset' or 'runInBackgroundAndClose' depending on state
            if (isTranscribing && jobStatus === 'running') {
                // Maybe do nothing, or dispatch runInBackgroundAndClose
            } else if (!isTranscribing && (jobStatus === 'done' || jobStatus === 'error' || jobStatus === 'cancelled' || jobStatus === null)) {
                 handleCloseAndReset();
            }
		}
	}

	$: if (showModal && typeof window !== 'undefined') {
		window.addEventListener('keydown', handleKeydown);
	} else if (typeof window !== 'undefined') {
		window.removeEventListener('keydown', handleKeydown);
	}

    onDestroy(() => {
        if (typeof window !== 'undefined') {
             window.removeEventListener('keydown', handleKeydown);
        }
    });

</script>

{#if showModal}
	<div
		class="fixed inset-0 z-[120] flex items-center justify-center bg-black/50 backdrop-blur-sm"
		role="dialog"
		aria-modal="true"
		aria-labelledby="transcribe-modal-title"
		on:click={handleCloseAndReset}
	> <!-- Allow closing by clicking backdrop if appropriate, or remove -->
		<div
			class="bg-white dark:bg-gray-800 rounded-lg shadow-xl p-6 w-full max-w-md text-gray-800 dark:text-gray-200 flex flex-col"
			on:click|stopPropagation
			role="document"
		>
			<h2 id="transcribe-modal-title" class="text-lg font-semibold mb-4 text-center">{modalTitle}</h2>

			{#if !isTranscribing && jobStatus === null}
				<!-- CONFIRM VIEW -->
				<div class="space-y-2 text-sm mb-5 text-gray-700 dark:text-gray-300">
					<p><strong>File:</strong> <span class="font-mono break-all">{fileName || 'N/A'}</span></p>
					<p><strong>Model:</strong> <span class="font-mono">{modelName || 'N/A'}</span></p>
					<p><strong>Language:</strong> <span class="font-mono">{language || 'N/A'}</span></p>
					<p><strong>Speakers:</strong> {speakers?.count > 0 ? `${speakers.count} (${(speakers.names || []).slice(0, 3).join(', ')}${speakers.count > 3 ? ', ...' : ''})` : '0 (Diarization Disabled)'}</p>
					<p><strong>Translate to English:</strong> <span class="font-mono">{$transcriptStore.translateToEnglish ? 'Yes' : 'No'}</span></p>
				</div>
				<div class="flex justify-end space-x-3 mt-auto">
					<button class="btn-secondary" on:click={handleCloseAndReset}>Cancel</button>
					<button class="btn-primary" on:click={handleConfirm}>Start Transcription</button>
				</div>

			{:else if isTranscribing && (jobStatus === 'running' || jobStatus === 'initiating')}
				<!-- RUNNING OR INITIATING VIEW -->
				<div class="flex flex-col items-center space-y-4 mb-6">
                    <div class="w-16 h-16">
                        <Loader class="w-full h-full text-blue-500 animate-spin" />
                    </div>
					{#if jobStatus === 'running'}
						<div class="w-full bg-gray-200 dark:bg-gray-600 rounded-full h-2.5 overflow-hidden">
							<div
								class="bg-blue-600 dark:bg-blue-500 h-2.5 rounded-full transition-all duration-300 ease-out"
								style="width: {progressPercent}%"
							></div>
						</div>
					{/if}
					<p class="text-xs text-center text-gray-600 dark:text-gray-400 h-4">
						{#if jobStatus === 'running'}{progressPercent.toFixed(0)}% - {/if}{progressMessage || (jobStatus === 'initiating' ? 'Preparing...' : 'Processing...')}
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
		background-color: #9ca3af; /* bg-gray-400 */
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
		background-color: #fca5a5; /* bg-red-300 */
	}
    .dark .btn-action-cancel {
         background-color: #dc2626; /* dark:bg-red-600 */
    }
    .dark .btn-action-cancel:hover:not(:disabled) {
        background-color: #b91c1c; /* dark:hover:bg-red-700 */
    }
</style>