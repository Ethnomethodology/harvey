<script>
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';

	// Props
	export let showModal = false;
	export let currentSpeakers = { count: 0, names: [] }; // Receive current config

	// Local state for editing
	let localCount = 0; // Bound to the input, represents desired count
	let renderedCount = 0; // Controls how many rows are actually shown
	let localNames = []; // Holds the names for the rendered rows

	// Optional second-language speaker names
	let addSecondNames = false;
	let localSecondNames = [];

	// Helper to sync second-language names array
	function updateLocalSecondNames(count, currentSecondNames) {
	  const names = Array.isArray(currentSecondNames) ? currentSecondNames : [];
	  const updated = Array.from({ length: count }, (_, i) =>
	    names[i] != null ? names[i] : ''
	  );
	  if (JSON.stringify(localSecondNames) !== JSON.stringify(updated)) {
	    localSecondNames = updated;
	  }
	}

	let modalElement;
	let hasInitialized = false;
	let listGenerated = false; // NEW: Tracks if the 'Add' button has been clicked for the current count

	const dispatch = createEventDispatcher();
	const MAX_SPEAKERS = 11;
	const MIN_SPEAKERS = 0;

	// Function to initialize or reset local state from props
	function initializeState() {
		if (currentSpeakers) {
			let initialCount = Number(currentSpeakers.count) || 0;
			initialCount = Math.max(MIN_SPEAKERS, Math.min(MAX_SPEAKERS, initialCount));
			localCount = initialCount;
			renderedCount = initialCount; // Initialize rendered count too
			// Initialize names based on the initial rendered count
			updateLocalNames(renderedCount, Array.isArray(currentSpeakers.names) ? [...currentSpeakers.names] : []);

			// Initialize second-language names if present
			updateLocalSecondNames(renderedCount, currentSpeakers.secondNames);

			// NEW: Enable confirm immediately ONLY if opening with a valid, non-zero count
			listGenerated = initialCount > 0 && initialCount === localCount;

			console.log('Modal state initialized/reset:', { localCount, renderedCount, localNames, listGenerated });
		} else {
			// Default if currentSpeakers is somehow null/undefined
			localCount = 0;
			renderedCount = 0;
			localNames = [];
			listGenerated = false;
			console.log('Modal state initialized (default):', { localCount, renderedCount, localNames, listGenerated });
		}

		// Mark initialization complete *after* setting initial state
		if (showModal && !hasInitialized) {
			hasInitialized = true;
		}
	}


	// Initialize local state when modal FIRST becomes visible
	$: if (showModal && !hasInitialized) {
		initializeState();
	}
	 // Reset flag when modal closes
	 $: if (!showModal) {
		hasInitialized = false;
		listGenerated = false; // Also reset listGenerated flag on close
	 }


	// Helper to synchronize localNames array with a specific count
	function updateLocalNames(count, currentNames) {
		const names = Array.isArray(currentNames) ? currentNames : [];
		// Only rebuild if needed
		// if (count !== localNames.length) { // Original check - might not be sufficient if names changed
			const updatedNames = Array.from({ length: count }, (_, i) =>
				names[i] !== undefined && names[i] !== null ? names[i] : `Speaker-${i + 1}`
			);
			// Avoid unnecessary reactivity trigger if array content is identical
			if (JSON.stringify(localNames) !== JSON.stringify(updatedNames)) {
				localNames = updatedNames;
				console.log('Updated localNames array:', localNames);
			}
		// }
	}

	// Reactive statement JUST for clamping localCount as user types
	$: {
		if (hasInitialized) { // Prevent running on initial mount before state is set
			let numCount = Number(localCount) || 0;
			const clampedCount = Math.max(MIN_SPEAKERS, Math.min(MAX_SPEAKERS, numCount));
			if (clampedCount !== numCount) {
				localCount = clampedCount; // Update the bound variable if clamping needed
			}
		}
	}

	// NEW: Reactive statement to disable confirm if count changes after list was generated
	$: if (hasInitialized && localCount !== renderedCount) {
		console.log(`[SpeakersModal] localCount (${localCount}) differs from renderedCount (${renderedCount}). Disabling confirm.`);
		listGenerated = false; // Disable confirm, user must click 'Add' again
	}


	// Function called by the "Add" button (previously "Update List")
	function updateRenderedRows() {
		// Use the current (potentially clamped) localCount
		const countToRender = localCount; // Already clamped by the reactive block above
		renderedCount = countToRender;
		// Update the names array to match the new rendered count
		updateLocalNames(renderedCount, localNames);
		updateLocalSecondNames(renderedCount, localSecondNames);
		listGenerated = true; // NEW: Enable confirm button after clicking Add
		console.log(`Updating rendered rows to ${renderedCount}. Confirm enabled: ${listGenerated}`);
	}


	// Handle direct edit of a speaker name
	function handleNameInput(index, event) {
		// Ensure index is valid for the *currently rendered* names
		if(index >= 0 && index < localNames.length) {
			localNames[index] = event.target.value;
			// Force reactivity update for the array
			 localNames = localNames;
		}
	}

	function handleSecondNameInput(index, event) {
	  if (index >= 0 && index < localSecondNames.length) {
	    localSecondNames[index] = event.target.value;
	    localSecondNames = localSecondNames;
	  }
	}


	// --- Modal Actions ---
	function confirm() {
		// Important: Confirm should use the *renderedCount* and *localNames*
		// which were set when "Add" was last clicked.
		// Only proceed if confirm is enabled
		if (!listGenerated) {
			console.warn("Confirm button clicked while disabled. This shouldn't happen.");
			return;
		}

		const finalNames = localNames.map((name, i) => (name && name.trim() !== '') ? name.trim() : `Speaker ${i + 1}`);
		const finalSecondNames = localSecondNames.map(n => n?.trim() || '');
		dispatch('confirm', { count: renderedCount, names: finalNames, secondNames: finalSecondNames });
		closeModal();
	}

	function cancel() {
		closeModal();
	}

	function closeModal() {
		showModal = false; // This will trigger the reactive block to reset flags
	}

	// --- Keyboard Handling ---
	function handleKeydown(event) {
		if (showModal && event.key === 'Escape') {
			cancel();
		}
		// Optional: Allow Enter in number field to trigger 'Add'?
		// if (showModal && event.key === 'Enter' && event.target.id === 'speaker-count-input') {
		//     event.preventDefault(); // Prevent form submission if applicable
		//     updateRenderedRows();
		// }
		// Optional: Allow Enter in name fields to trigger confirm IF enabled?
		// if (showModal && event.key === 'Enter' && listGenerated && event.target.tagName === 'INPUT' && event.target.type === 'text') {
		//     event.preventDefault();
		//     confirm();
		// }
	}

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
		on:click|self={cancel}
		role="dialog"
		aria-modal="true"
		aria-labelledby="speakers-modal-title"
	>
		<div class="bg-white p-6 rounded-lg shadow-xl w-full max-w-lg m-4 flex flex-col" role="document">
			<h2 id="speakers-modal-title" class="text-lg font-semibold text-gray-800 mb-5">
				Configure Speakers
			</h2>

			<!-- Speaker Count Input and Update Button -->
			<div class="mb-4 flex items-center space-x-3">
				<label for="speaker-count-input" class="block text-sm font-medium text-gray-700 flex-shrink-0">
					Number of Speakers:
				</label>
				<input
					id="speaker-count-input"
					type="number"
					min={MIN_SPEAKERS}
					max={MAX_SPEAKERS}
					step="1"
					bind:value={localCount}
					class="input-field w-20 text-center"
				/>
				<button
					type="button"
					on:click={updateRenderedRows}
					class="btn-secondary text-xs px-3 py-1"
					title="Generate or update the speaker name fields below"
				>
					Add
				</button>
			</div>

			<!-- Speaker Names Table/List -->
			<div class="mb-5 border-t pt-4 max-h-72 overflow-y-auto">
				{#if renderedCount > 0}
					<h3 class="text-sm font-medium text-gray-700 mb-2">Speaker Names:</h3>
					<label class="inline-flex items-center space-x-2 mb-3">
					  <input type="checkbox" bind:checked={addSecondNames} class="form-checkbox" />
					  <span class="text-sm text-gray-700">Add names in 2nd language</span>
					</label>
					<div class="space-y-2">
						{#each { length: renderedCount } as _, i (i)}
							<div class="flex items-center space-x-3">
								<span class="text-sm text-gray-600 w-28 text-right font-mono flex-shrink-0">
									Speaker {i + 1}: 
								</span>
								<input
									type="text"
									value={localNames[i] || ''}
									on:input={(e) => handleNameInput(i, e)}
									placeholder={`Speaker ${i + 1}`}
									class="input-field flex-grow min-w-0"
								/>
								{#if addSecondNames}
								  <input
									type="text"
									bind:value={localSecondNames[i]}
									on:input={(e) => handleSecondNameInput(i, e)}
									placeholder={`Speaker ${i + 1} (2nd lang)`}
									class="input-field flex-grow min-w-0"
								  />
								{/if}
							</div>
						{/each}
					</div>
				{:else if listGenerated && renderedCount === 0}
					<p class="text-sm text-gray-500 italic text-center py-2">Zero speakers specified. Click 'Confirm' to save this setting.</p>
				{:else}
					<p class="text-sm text-gray-500 italic text-center py-2">Enter the number of speakers and click "Add".</p>
				{/if}
			</div>

			<!-- Footer Buttons -->
			<div class="flex justify-end space-x-3 pt-4 border-t border-gray-200 mt-auto">
				<button
					type="button"
					on:click={cancel}
					class="btn-secondary"
				>
					Cancel
				</button>
				<button
					type="button"
					on:click={confirm}
					class="btn-primary"
					disabled={!listGenerated}
					title={!listGenerated ? 'Click Add button first' : 'Confirm speaker settings'}
				>
					Confirm
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.btn-primary { padding: 0.4rem 1rem; background-color: #3b82f6; color: white; border: none; border-radius: 0.375rem; cursor: pointer; font-size: 0.875rem; font-weight: 500; transition: background-color 0.15s ease-in-out, opacity 0.15s ease-in-out; }
	.btn-primary:hover:not(:disabled) { background-color: #2563eb; }
	/* NEW: Style for disabled primary button */
	.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; background-color: #9ca3af; }

	.btn-secondary { padding: 0.4rem 1rem; background-color: #e5e7eb; color: #374151; border: 1px solid #d1d5db; border-radius: 0.375rem; cursor: pointer; font-size: 0.875rem; font-weight: 500; transition: background-color 0.15s ease-in-out; }
	.btn-secondary:hover { background-color: #d1d5db; }
	/* Specific style for the small add button */
	.btn-secondary.text-xs { font-size: 0.75rem; padding: 0.25rem 0.75rem; }

	.input-field { border: 1px solid #d1d5db; padding: 0.4rem 0.6rem; border-radius: 0.375rem; font-size: 0.875rem; background-color: white; color: #374151; }
	.input-field:focus { outline: 2px solid transparent; outline-offset: 2px; border-color: #3b82f6; box-shadow: 0 0 0 1px #3b82f6; }
	.overflow-y-auto::-webkit-scrollbar { width: 5px; }
	.overflow-y-auto::-webkit-scrollbar-track { background: transparent; }
	.overflow-y-auto::-webkit-scrollbar-thumb { background-color: rgba(156, 163, 175, 0.5); border-radius: 10px; border: 2px solid transparent; background-clip: content-box; }
	.overflow-y-auto::-webkit-scrollbar-thumb:hover { background-color: rgba(107, 114, 128, 0.6); }
	.overflow-y-auto { scrollbar-width: thin; scrollbar-color: rgba(156, 163, 175, 0.5) transparent; }
</style>