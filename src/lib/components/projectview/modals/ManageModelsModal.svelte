<!-- src/lib/components/projectview/ManageModelsModal.svelte -->
<script>
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import ConfigurationView from '$lib/components/shared/ConfigurationView.svelte'; // Import the configure component

	export let showModal = false;

	let modalElement; // Reference to the modal container DOM element
	let isConfigureBusy = false; // Local state bound to Configure's busy state

	const dispatch = createEventDispatcher();

	// --- MODAL ACTIONS ---
	function close() {
		// Attempt to close, will be prevented by handleCloseAttempt if busy
		handleCloseAttempt();
	}

	function handleCloseAttempt() {
		if (isConfigureBusy) {
			console.log('ManageModelsModal: Close prevented - operation in progress.');
			alert(
				'An operation (downloading or moving models) is currently in progress. Please wait or cancel it before closing.'
			);
			return; // Prevent closing
		}
		// If not busy, signal the parent (TopBar) to close the modal
		showModal = false; // Update bound prop which closes the modal visually
		dispatch('close'); // DISPATCH EVENT
		console.log('ManageModelsModal: Closing and dispatched event.');
	}

	// --- KEYBOARD HANDLING ---
	function handleKeydown(event) {
		if (showModal && event.key === 'Escape') {
			// Only handle if modal is shown
			handleCloseAttempt();
		}
	}

	// --- Lifecycle for Keyboard Listener ---
	// Add/remove listener only when modal is shown/hidden might be slightly more efficient
	// but global listener is simpler here.
	onMount(() => {
		window.addEventListener('keydown', handleKeydown);
	});

	onDestroy(() => {
		window.removeEventListener('keydown', handleKeydown);
	});
</script>

{#if showModal}
	<div
		bind:this={modalElement}
		class="fixed inset-0 z-[120] flex items-center justify-center bg-gray-900 bg-opacity-60 backdrop-blur-sm"
		on:click|self={handleCloseAttempt}
		role="dialog"
		aria-modal="true"
		aria-labelledby="manage-models-title"
		tabindex="-1"
		on:keydown={handleKeydown}
	>
		<div
			class="bg-white rounded-lg shadow-xl w-full max-w-3xl m-4 flex flex-col max-h-[80vh]"
			role="document"
		>
			<!-- Modal Header -->
			<div class="flex justify-between items-center p-4 border-b border-gray-200 flex-shrink-0">
				<h2 id="manage-models-title" class="text-lg font-semibold text-gray-800">
					Manage Models
				</h2>
				<button
					on:click={handleCloseAttempt}
					class="p-1 rounded-full text-gray-400 hover:bg-gray-200 hover:text-gray-600 focus:outline-none focus:ring-2 focus:ring-offset-1 focus:ring-gray-400"
					aria-label="Close model manager"
					disabled={isConfigureBusy}
				>
					<!-- Close icon -->
					<svg
						xmlns="http://www.w3.org/2000/svg"
						viewBox="0 0 20 20"
						fill="currentColor"
						class="w-5 h-5"
					>
						<path
							d="M6.28 5.22a.75.75 0 0 0-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 1 0 1.06 1.06L10 11.06l3.72 3.72a.75.75 0 1 0 1.06-1.06L11.06 10l3.72-3.72a.75.75 0 0 0-1.06-1.06L10 8.94 6.28 5.22Z"
						/>
					</svg>
				</button>
			</div>

			<!-- Modal Body (Contains Configure Component) -->
			<div class="flex-grow overflow-y-auto">
				<!-- Bind the isBusy state from Configure -->
				<ConfigurationView bind:isBusy={isConfigureBusy} />
			</div>

			<!-- Modal Footer -->
			<div
				class="flex justify-end space-x-3 p-4 border-t border-gray-200 flex-shrink-0 bg-gray-50 rounded-b-lg"
			>
				<button
					type="button"
					on:click={handleCloseAttempt}
					class="px-4 py-2 bg-gray-600 text-white rounded-md shadow-sm hover:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-opacity-75 transition duration-150 ease-in-out text-sm font-medium"
					class:opacity-50={isConfigureBusy}
					class:cursor-not-allowed={isConfigureBusy}
					disabled={isConfigureBusy}
					title={isConfigureBusy ? 'Operation in progress...' : 'Close Model Manager'}
				>
					{isConfigureBusy ? 'Working...' : 'Close'}
				</button>
			</div>
		</div>
	</div>
{/if}