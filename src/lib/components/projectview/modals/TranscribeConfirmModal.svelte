<!-- src/lib/components/projectview/modals/TranscribeConfirmModal.svelte -->
<script>
	import { createEventDispatcher, onDestroy } from 'svelte'; // Removed onMount
	import { CheckCircle, XCircle, Clock, Loader } from 'lucide-svelte';
	// Import project store to read progress data
	import { project } from '$lib/stores/projectStore.js';
	import { transcriptStore } from '$lib/stores/transcriptStore.js';

	// Props
	export let showModal = false;
	export let fileName = '';
	export let modelName = '';
	export let language = '';
	export let speakers = { count: 0 };
	export let jobId = null; // Kept for potential display

	const dispatch = createEventDispatcher();

	// Internal state for modal appearance/status
	let status = 'confirm'; // 'confirm', 'running', 'cancelling', 'done', 'error', 'cancelled'
	let errorMessage = '';
	let successMessage = '';
	let cancelledMessage = '';

	// --- Event Handlers ---
	function handleConfirm() {
		console.log('[Modal] Confirming transcription start...');
		status = 'running'; // Switch UI state immediately
		errorMessage = '';
		successMessage = '';
		cancelledMessage = '';
		dispatch('confirmStart'); // Tell parent/service to start backend
	}

	function handleCancelRequest() {
		if (status !== 'running') return; // Only cancel if running
		console.log('[Modal] Requesting transcription cancellation...');
		status = 'cancelling'; // Update UI state
		dispatch('cancelRequest'); // Tell parent/service to request cancellation
	}

	function closeModal() {
    let dispatchPayload = { acknowledged: false, finalStatus: status }; // Default payload

    if (status === 'running' || status === 'cancelling') {
        console.warn('[Modal] Closing modal while status is:', status);
        // Optionally, could trigger handleCancelRequest here if desired
        // For now, just a normal close, user might not want to cancel
    }

    if (status === 'done' || status === 'error' || status === 'cancelled') {
        dispatchPayload.acknowledged = true;
    }

    // Reset internal state ONLY when closing (via X, background, or final Close button)
    status = 'confirm'; // Always reset to confirm
    errorMessage = '';
    successMessage = '';
    cancelledMessage = '';
    // transcriptStore.update(ts => ({ ...ts, showTranscribeModal: false })); // Directly control store
    dispatch('close', dispatchPayload); // Signal parent/service to update store's showModal AND handle post-acknowledgment tasks
}

	// --- Public methods (callable from parent/service via bind:this) ---
	// *** ADD 'export' to make this function callable from outside ***
	export function updateProgress(newProgress, newMessage) {
        // This function is NO LONGER NEEDED here as progress is read directly from the store.
        // Kept as placeholder in case it's called, but ideally remove calls from service.
		// console.warn('[Modal updateProgress] This function is deprecated. Progress read from store.');
		// if (status === 'running' || status === 'cancelling') {
			// No local state to update
		// }
	}

	export function setStatusDone(message = 'Transcription complete.') {
		console.log(`[Modal] Setting status to 'done'`);
		status = 'done';
		successMessage = message;
		errorMessage = '';
	}

	export function setStatusError(errorMsg = 'An unknown error occurred.') {
		console.log(`[Modal] Setting status to 'error': ${errorMsg}`);
		status = 'error';
		errorMessage = errorMsg;
		successMessage = '';
		cancelledMessage = '';
	}

	export function setStatusCancelled(message = 'Transcription cancelled.') {
		console.log(`[Modal] Setting status to 'cancelled'`);
		status = 'cancelled';
		cancelledMessage = message;
		errorMessage = '';
		successMessage = '';
	}

	// Can still be called by service if needed, updates UI state
	export function setStatusCancelling(message = 'Requesting cancellation...') {
        if (status === 'running') { // Only switch if currently processing
             status = 'cancelling';
             // Note: progressMessage is now derived from the store below
        }
    }

	// --- Keyboard handling ---
	function handleKeydown(event) {
		if (showModal && event.key === 'Escape') {
			// closeModal(); // Escape key functionality will be handled differently or removed based on new requirements
            // For now, let's disable it to ensure modal stays open unless explicitly closed by new buttons
		}
	}

	// --- Reactive Derivations from Store ---
	// Get progress directly from the global store when the modal is potentially running/cancelling
	// Use internal status to gate reading from store, otherwise show defaults
	$: currentProgressPercent = (status === 'running' || status === 'cancelling') && $transcriptStore.isTranscribing
								? $transcriptStore.transcriptionProgress.percent
								: (status === 'done' ? 100 : 0);

	$: currentProgressMessage = status === 'running' ? ($transcriptStore.transcriptionProgress.message || 'Processing...')
							   : status === 'cancelling' ? ($transcriptStore.transcriptionProgress.message || 'Cancelling...')
							   : ''; // No message needed in other states displayed here

	$: currentJobId = $transcriptStore.transcriptionJobId; // Get current Job ID from store

	// --- Title Logic ---
	$: modalTitle = status === 'confirm' ? 'Confirm Transcription Settings' :
					 status === 'running' ? `Transcription Status${currentJobId ? ` (Job: ${currentJobId.substring(0, 8)})` : ''}` :
					 status === 'cancelling' ? `Cancelling Job${currentJobId ? ` (${currentJobId.substring(0, 8)})` : ''}` :
					 status === 'done' ? 'Transcription Complete' :
					 status === 'error' ? 'Transcription Error' :
					 status === 'cancelled' ? 'Transcription Cancelled' :
					 'Transcription Status';


    // Add/remove keyboard listener based on modal visibility
	$: if (showModal && typeof window !== 'undefined') {
		window.addEventListener('keydown', handleKeydown);

    // If the modal is being shown, is currently in 'confirm' state (e.g., from a previous closeModal() call or initial state),
    // AND a transcription job is globally active ($transcriptStore.isTranscribing is true),
    // then transition the modal's internal status to 'running'.
    if (status === 'confirm' && $transcriptStore.isTranscribing) {
        console.log('[Modal] Syncing to "running" state: Modal was "confirm" and a global transcription is active.');
        status = 'running';
        // Clear any potentially stale messages when transitioning to 'running' for a new/ongoing job.
        successMessage = '';
        errorMessage = '';
        cancelledMessage = '';
    }
    // If the modal's status was externally set to 'done', 'error', or 'cancelled' (by projectService for a foreground job),
    // this block will not alter that status. The modal will correctly display that terminal state.
    // The user will then interact with the modal's buttons (e.g., "Close"), which will call `closeModal()`,
    // and `closeModal()` is responsible for resetting the status to 'confirm' at that point.
    // If the modal is opened for a new transcription (status='confirm', isTranscribing=false), this block also does nothing,
    // leaving the modal in the 'confirm' state, ready for user input.

	} else if (typeof window !== 'undefined') { // Equivalent to: if (!showModal && ...)
    // When the modal is not shown, remove the keydown listener.
		window.removeEventListener('keydown', handleKeydown);
	}

    // Cleanup listener on component destroy
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
	>
		<div
			class="bg-white dark:bg-gray-800 rounded-lg shadow-xl p-6 w-full max-w-md text-gray-800 dark:text-gray-200 flex flex-col"
			on:click|stopPropagation
			role="document"
		>
			<h2 id="transcribe-modal-title" class="text-lg font-semibold mb-4 text-center">{modalTitle}</h2>

			{#if status === 'confirm'}
				<!-- Confirmation View -->
				<div class="space-y-2 text-sm mb-5 text-gray-700 dark:text-gray-300">
					<p><strong>File:</strong> <span class="font-mono break-all">{fileName || 'N/A'}</span></p>
					<p><strong>Model:</strong> <span class="font-mono">{modelName || 'N/A'}</span></p>
					<p><strong>Language:</strong> <span class="font-mono">{language || 'N/A'}</span></p>
					<p><strong>Speakers:</strong> {speakers?.count > 0 ? `${speakers.count} (${(speakers.names || []).slice(0, 3).join(', ')}${speakers.count > 3 ? ', ...' : ''})` : '0 (Diarization Disabled)'}</p>
				</div>
				<div class="flex justify-end space-x-3 mt-auto">
					<button class="btn-secondary" on:click={closeModal}>Cancel</button>
					<button class="btn-primary" on:click={handleConfirm}>Start Transcription</button>
				</div>

			{:else if status === 'running' || status === 'cancelling'}
				<!-- Processing/Cancelling View -->
				<div class="flex flex-col items-center space-y-4 mb-6">
                    <div class="w-16 h-16">
                        {#if status === 'running'}
                            <Loader class="w-full h-full text-blue-500 animate-spin" />
                        {:else} <!-- cancelling -->
                            <Clock class="w-full h-full text-orange-500" />
                        {/if}
                    </div>
					<div class="w-full bg-gray-200 dark:bg-gray-600 rounded-full h-2.5 overflow-hidden">
						<div
							class="bg-blue-600 dark:bg-blue-500 h-2.5 rounded-full transition-all duration-300 ease-out"
							style="width: {currentProgressPercent}%"
						></div>
					</div>
					<p class="text-xs text-center text-gray-600 dark:text-gray-400 h-4">
						{currentProgressPercent.toFixed(0)}% - {currentProgressMessage}
					</p>
				</div>
				<div class="flex justify-center space-x-2 mt-auto"> <!-- MODIFIED: Added space-x-2 for button spacing -->
					{#if status === 'running'}
						<button class="btn-secondary" on:click={() => { dispatch('runInBackground'); closeModal(); }}>
							Run in background
						</button>
						<button class="btn-action-cancel" on:click={handleCancelRequest}>
							Request Cancellation
						</button>
					{:else} <!-- cancelling -->
						<button class="btn-secondary" disabled>Cancelling...</button>
					{/if}
				</div>

			{:else if status === 'done'}
				<!-- Done View -->
				<div class="flex flex-col items-center space-y-3 mb-6 text-center">
					<CheckCircle class="w-16 h-16 text-green-500" />
					<p class="text-sm font-medium">{successMessage || 'Transcription Complete!'}</p>
				</div>
				<div class="flex justify-center mt-auto">
					<button class="btn-primary" on:click={closeModal}>Close</button>
				</div>

			{:else if status === 'cancelled'}
                <!-- Cancelled View -->
				<div class="flex flex-col items-center space-y-3 mb-6 text-center">
					<XCircle class="w-16 h-16 text-orange-500" />
					<p class="text-sm font-medium">{cancelledMessage || 'Transcription Cancelled'}</p>
				</div>
				<div class="flex justify-center mt-auto">
					<button class="btn-secondary" on:click={closeModal}>Close</button>
				</div>

			{:else if status === 'error'}
				<!-- Error View -->
				<div class="flex flex-col items-center space-y-3 mb-6 text-center">
					<XCircle class="w-16 h-16 text-red-500" />
					<p class="text-sm font-medium">An Error Occurred</p>
					<p class="text-xs bg-red-100 dark:bg-red-900/50 border border-red-300 dark:border-red-700 text-red-700 dark:text-red-300 p-2 rounded w-full text-left overflow-x-auto max-h-32">
						{errorMessage || 'Unknown error during transcription.'}
					</p>
				</div>
				<div class="flex justify-center mt-auto">
					<button class="btn-secondary" on:click={closeModal}>Close</button>
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