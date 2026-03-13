<!-- src/lib/components/projectview/modals/SpeakersModal.svelte -->
<script>
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import { updateSpeakerConfig } from '$lib/stores/transcriptStore.js';
    import { 
        Input, 
        Label, 
        Button, 
        Checkbox
    } from 'flowbite-svelte';
    import { Users, X, Plus, Trash2, Minus, UserPlus } from 'lucide-svelte';

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

    function increment() {
        if (localCount < MAX_SPEAKERS) localCount++;
    }

    function decrement() {
        if (localCount > MIN_SPEAKERS) localCount--;
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

    function removeSpeaker(index) {
        localNames = localNames.filter((_, i) => i !== index);
        localSecondNames = localSecondNames.filter((_, i) => i !== index);
        renderedCount--;
        localCount = renderedCount;
    }


	// Handle direct edit of a speaker name
	


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
		
		updateSpeakerConfig(renderedCount, finalNames, finalSecondNames);

		dispatch('confirm', { count: renderedCount, names: finalNames, translatedNames: finalSecondNames });
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

{#if showModal}
	<div
		bind:this={modalElement}
		class="fixed inset-0 z-[120] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
		on:click|self={cancel}
		role="dialog"
		aria-modal="true"
		aria-labelledby="speakers-modal-title"
        tabindex="-1"
	>
		<div class="bg-white dark:bg-gray-900 rounded-xl shadow-2xl w-full max-w-lg flex flex-col border border-gray-200 dark:border-gray-800 overflow-hidden" role="document" on:click|stopPropagation>
            <!-- Header -->
            <div class="px-6 py-5 border-b border-gray-200 dark:border-gray-800 flex justify-between items-center bg-gray-50/50 dark:bg-gray-800/50">
                <div class="flex items-center space-x-3">
                    <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
                        <Users size={20} class="text-blue-600 dark:text-blue-400" />
                    </div>
                    <h3 id="speakers-modal-title" class="text-lg font-bold text-gray-900 dark:text-white">
                        Configure Speakers
                    </h3>
                </div>
                <button on:click={cancel} class="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-full transition-all" title="Close">
                    <X size={20} />
                </button>
            </div>

            <div class="p-6 space-y-5">
                <!-- Speaker Count Input and Update Button -->
                <div class="flex items-end space-x-3">
                    <div class="flex-grow space-y-2">
                        <Label for="speaker-count-input">Number of Speakers</Label>
                        <div class="flex items-center space-x-1">
                            <Button
                                color="alternative"
                                on:click={decrement}
                                class="px-3"
                                title="Decrease"
                                disabled={localCount <= MIN_SPEAKERS}
                            >
                                <Minus size={16} />
                            </Button>
                            <Input
                                id="speaker-count-input"
                                type="number"
                                min={MIN_SPEAKERS}
                                max={MAX_SPEAKERS}
                                step="1"
                                bind:value={localCount}
                                placeholder="0-11"
                                class="text-center"
                            />
                            <Button
                                color="alternative"
                                on:click={increment}
                                class="px-3"
                                title="Increase"
                                disabled={localCount >= MAX_SPEAKERS}
                            >
                                <Plus size={16} />
                            </Button>
                        </div>
                    </div>
                    <Button
                        color="alternative"
                        on:click={updateRenderedRows}
                        class="px-4"
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
                                        />
                                        {#if addSecondNames}
                                            <Input
                                                size="sm"
                                                type="text"
                                                bind:value={localSecondNames[i]}
                                                placeholder={`2nd Language Name`}
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

            <!-- Footer -->
            <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-800 flex justify-end gap-3 bg-gray-50/80 dark:bg-gray-800/80 backdrop-blur-md">
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
			</div>
		</div>
	</div>
{/if}

<style lang="postcss">
    .custom-scrollbar::-webkit-scrollbar {
        width: 6px;
    }
    .custom-scrollbar::-webkit-scrollbar-track {
        @apply bg-transparent;
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