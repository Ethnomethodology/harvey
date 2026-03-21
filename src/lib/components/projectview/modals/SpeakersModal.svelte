<!-- src/lib/components/projectview/modals/SpeakersModal.svelte -->
<script>
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import { updateSpeakerConfig } from '$lib/stores/transcriptStore.js';
    import { 
		Modal,
        Input, 
        Label, 
        Button, 
        Checkbox
    } from 'flowbite-svelte';
    import { Users, Plus, Trash2, Minus, UserPlus } from '@lucide/svelte';

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
			if (Array.isArray(currentSpeakers.translatedNames) && currentSpeakers.translatedNames.some(name => name && name.trim() !== '')) {
				addSecondNames = true;
				updateLocalSecondNames(renderedCount, currentSpeakers.translatedNames);
			} else {
				addSecondNames = false;
				updateLocalSecondNames(renderedCount, []);
			}

			// NEW: Enable confirm immediately ONLY if opening with a valid, non-zero count
			listGenerated = initialCount > 0 && initialCount === localCount;

		} else {
			// Default if currentSpeakers is somehow null/undefined
			localCount = 0;
			renderedCount = 0;
			localNames = [];
			listGenerated = false;
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
		const updatedNames = Array.from({ length: count }, (_, i) =>
			names[i] !== undefined && names[i] !== null ? names[i] : `Speaker-${i + 1}`
		);
		// Avoid unnecessary reactivity trigger if array content is identical
		if (JSON.stringify(localNames) !== JSON.stringify(updatedNames)) {
			localNames = updatedNames;
		}
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

    function increment() {
        if (localCount < MAX_SPEAKERS) localCount++;
    }

    function decrement() {
        if (localCount > MIN_SPEAKERS) localCount--;
    }

	// NEW: Reactive statement to disable confirm if count changes after list was generated
	$: if (hasInitialized && localCount !== renderedCount) {
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
	}

    function removeSpeaker(index) {
        localNames = localNames.filter((_, i) => i !== index);
        localSecondNames = localSecondNames.filter((_, i) => i !== index);
        renderedCount--;
        localCount = renderedCount;
    }


	// --- Modal Actions ---
	function confirm() {
		// Important: Confirm should use the *renderedCount* and *localNames*
		// which were set when "Add" was last clicked.
		// Only proceed if confirm is enabled
		if (!listGenerated) {
			return;
		}

		const finalNames = localNames.map((name, i) => (name && name.trim() !== '') ? name.trim() : `Speaker ${i + 1}`);
		const finalSecondNames = localSecondNames.map(n => n?.trim() || '');
		
		updateSpeakerConfig(renderedCount, finalNames, finalSecondNames);

		dispatch('confirm', { count: renderedCount, names: finalNames, translatedNames: finalSecondNames });
		closeModal();
	}

	function cancel() {
		closeModal();
	}

	function closeModal() {
		showModal = false; // This will trigger the reactive block to reset flags
		dispatch('close');
	}

	// --- Keyboard Handling ---
	function handleKeydown(event) {
		if (showModal && event.key === 'Escape') {
			cancel();
		}
        if (showModal && event.key === 'Enter') {
            if (localCount !== renderedCount) {
                updateRenderedRows();
            } else if (listGenerated) {
                confirm();
            }
        }
	}

	onMount(() => {
		window.addEventListener('keydown', handleKeydown);
	});

	onDestroy(() => {
		window.removeEventListener('keydown', handleKeydown);
	});
</script>

<Modal
	bind:open={showModal}
	size="md"
	autoclose={false}
	outsideclose={true}
	class="w-full"
	backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
	dialogClass="fixed top-0 start-0 end-0 h-modal md:inset-0 md:h-full z-[10001] flex"
	bodyClass="p-6 space-y-5 bg-white dark:bg-gray-900"
	headerClass="px-6 py-4 flex items-center justify-between border-b dark:border-gray-700 bg-gray-50/50"
	footerClass="px-6 py-4 flex items-center justify-end space-x-3 rtl:space-x-reverse border-t dark:border-gray-700 bg-gray-50/80 backdrop-blur"
	on:close={closeModal}
>
	<div slot="header" class="flex items-center gap-2">
		<Users class="w-5 h-5 text-gray-500" />
		<h3 class="text-lg font-semibold text-gray-900 dark:text-white">
			Configure Speakers
		</h3>
	</div>

	<div class="space-y-5">
		<!-- Speaker Count Input and Update Button -->
		<div class="flex items-end space-x-3">
			<div class="flex-grow space-y-2">
				<Label for="speaker-count-input">Number of Speakers</Label>
				<div class="relative flex items-center w-full">
					<button 
						type="button" 
						on:click={decrement}
						class="flex-shrink-0 bg-gray-100 dark:bg-gray-700 dark:hover:bg-gray-600 dark:border-gray-600 hover:bg-gray-200 border border-gray-300 rounded-s-lg p-2.5 h-10 focus:ring-gray-100 dark:focus:ring-gray-700 focus:ring-2 focus:outline-none transition-colors"
						disabled={localCount <= MIN_SPEAKERS}
						title="Decrease speaker count"
					>
						<Minus size={16} class="text-gray-900 dark:text-white" />
					</button>
					<input 
						type="text" 
						id="speaker-count-input" 
						class="bg-gray-50 border-x-0 border-gray-300 h-10 text-center text-gray-900 text-sm focus:ring-blue-500 focus:border-blue-500 block w-full py-2.5 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500" 
						bind:value={localCount}
						placeholder="0-11"
						required 
						autocomplete="off"
						autocorrect="off"
					/>
					<button 
						type="button" 
						on:click={increment}
						class="flex-shrink-0 bg-gray-100 dark:bg-gray-700 dark:hover:bg-gray-600 dark:border-gray-600 hover:bg-gray-200 border border-gray-300 rounded-e-lg p-2.5 h-10 focus:ring-gray-100 dark:focus:ring-gray-700 focus:ring-2 focus:outline-none transition-colors"
						disabled={localCount >= MAX_SPEAKERS}
						title="Increase speaker count"
					>
						<Plus size={16} class="text-gray-900 dark:text-white" />
					</button>
				</div>
			</div>
			<Button
				color="alternative"
				on:click={updateRenderedRows}
				class="px-4 h-10"
				title="Generate speaker name fields"
			>
				<UserPlus size={18} class="mr-2" />
				Add
			</Button>
		</div>

		<!-- Speaker Names Table/List -->
		<div class="space-y-4 pt-2">
			{#if renderedCount > 0}
				<div class="flex justify-between items-center">
					<h4 class="text-sm font-semibold text-gray-700 dark:text-gray-300">Speaker Names</h4>
					<Checkbox bind:checked={addSecondNames}>
						Add names in 2nd language
					</Checkbox>
				</div>
				
				<div class="space-y-3 max-h-72 overflow-y-auto pr-2 custom-scrollbar">
					{#each { length: renderedCount } as _, i (i)}
						<div class="flex items-center space-x-3 bg-gray-50 dark:bg-gray-800/40 p-3 rounded-lg border border-gray-100 dark:border-gray-800">
							<span class="text-xs font-mono text-gray-500 dark:text-gray-400 w-20 flex-shrink-0">
								Speaker {i + 1}
							</span>
							<div class="flex-grow flex gap-2">
								<Input
									size="sm"
									type="text"
									bind:value={localNames[i]}
									placeholder={`Name`}
									autocomplete="off"
									autocorrect="off"
								/>
								{#if addSecondNames}
									<Input
										size="sm"
										type="text"
										bind:value={localSecondNames[i]}
										placeholder={`2nd Language Name`}
										autocomplete="off"
										autocorrect="off"
									/>
								{/if}
							</div>
							<button 
								on:click={() => removeSpeaker(i)} 
								class="p-1.5 text-gray-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-md transition-colors"
								title="Remove Speaker"
							>
								<Trash2 size={16} />
							</button>
						</div>
					{/each}
				</div>
			{:else if listGenerated && renderedCount === 0}
				<div class="text-center py-8 bg-gray-50 dark:bg-gray-800/40 rounded-xl border border-dashed border-gray-200 dark:border-gray-700">
					<p class="text-sm text-gray-500 dark:text-gray-400">Zero speakers specified.</p>
				</div>
			{:else}
				<div class="text-center py-8 bg-gray-50 dark:bg-gray-800/40 rounded-xl border border-dashed border-gray-200 dark:border-gray-700">
					<p class="text-sm text-gray-500 dark:text-gray-400">Enter speaker count and click "Add" to configure.</p>
				</div>
			{/if}
		</div>
	</div>

	<svelte:fragment slot="footer">
		<Button color="alternative" on:click={cancel} title="Cancel">
			Cancel
		</Button>
		<Button
			color="blue"
			on:click={confirm}
			disabled={!listGenerated}
			title={!listGenerated ? 'Click Add button first' : 'Save speaker settings'}
		>
			Confirm
		</Button>
	</svelte:fragment>
</Modal>

<style lang="postcss">
    .custom-scrollbar::-webkit-scrollbar {
        width: 6px;
    }
    .custom-scrollbar::-webkit-scrollbar-track {
        @apply bg-transparent;
    }
    .custom-scrollbar::-webkit-scrollbar-thumb {
        @apply bg-gray-300 dark:bg-gray-700 rounded-full;
    }
    .custom-scrollbar::-webkit-scrollbar-thumb:hover {
        @apply bg-gray-400 dark:bg-gray-600;
    }
</style>