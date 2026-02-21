<!-- src/lib/components/projectview/modals/ManualSettingsModal.svelte -->
<script>
	import { createEventDispatcher } from 'svelte';
	import { X } from 'lucide-svelte';
    import Dropdown from '$lib/components/shared/Dropdown.svelte';

	export let showModal = false;
	export let currentSettings = { duration: 60, speakerMode: 'unassigned' }; // Default
    export let speakerList = []; // Array of speaker names

	const dispatch = createEventDispatcher();

    let duration = 60;
    let speakerMode = 'unassigned';

    // Sync local state when modal opens
	$: if (showModal) {
        duration = currentSettings.duration || 60;
        speakerMode = currentSettings.speakerMode || 'unassigned';
        if (speakerMode === 'unselected') speakerMode = 'unassigned'; // Migration
	}

    $: speakerOptions = [
        { value: 'unassigned', label: 'Unassigned' },
        { value: 'alternate', label: 'Alternate Speakers', disabled: speakerList.length < 2 }
    ];

	function handleConfirm() {
		dispatch('confirm', {
            duration,
            speakerMode
        });
		showModal = false;
	}

	function handleClose() {
		showModal = false;
		dispatch('close');
	}
</script>

{#if showModal}
	<div
		class="fixed inset-0 z-[130] flex items-center justify-center bg-black/50 backdrop-blur-sm"
		role="dialog"
		aria-modal="true"
		on:click={handleClose}
	>
		<div
			class="bg-white dark:bg-gray-900 rounded-lg shadow-xl w-full max-w-sm flex flex-col max-h-[90vh]"
			on:click|stopPropagation
		>
			<!-- Header -->
			<div class="flex items-center justify-between px-4 py-3 border-b border-gray-200 dark:border-gray-700">
				<h3 class="text-lg font-semibold text-gray-800 dark:text-gray-200">Manual Transcription Settings</h3>
				<button class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200" on:click={handleClose}>
					<X class="w-5 h-5" />
				</button>
			</div>

			<!-- Body -->
			<div class="p-4 space-y-4 overflow-y-auto">
                <p class="text-sm text-gray-600 dark:text-gray-400">
                    Configure defaults for new segments added manually.
                </p>

				<!-- Duration Input -->
				<div class="space-y-1">
					<label for="segDuration" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
						Default Segment Duration (seconds)
					</label>
					<div class="flex items-center gap-2">
                        <input
                            id="segDuration"
                            type="number"
                            min="1"
                            class="ui-input flex-1"
                            bind:value={duration}
                        />
                        <span class="text-xs text-gray-500 min-w-[3rem]">
                            {Math.floor(duration / 60)}m {duration % 60}s
                        </span>
                    </div>
				</div>

				<!-- Speaker Mode -->
				<div class="space-y-2">
					<span class="block text-sm font-medium text-gray-700 dark:text-gray-300">Speaker Assignment</span>
					<div class="flex flex-col space-y-2 ml-1">
						{#each speakerOptions as option}
							<label class="flex items-center space-x-2 cursor-pointer {option.disabled ? 'opacity-50 cursor-not-allowed' : ''}">
								<input
									type="radio"
									name="speakerMode"
									value={option.value}
									bind:group={speakerMode}
									disabled={option.disabled}
									class="ui-radio"
								/>
								<span class="text-sm text-gray-700 dark:text-gray-300">{option.label}</span>
							</label>
						{/each}
					</div>
                    {#if speakerMode === 'alternate' && speakerList.length < 2}
                        <p class="text-xs text-orange-500">Add at least 2 speakers to enable alternation.</p>
                    {/if}
				</div>
			</div>

			<!-- Footer -->
			<div class="flex justify-end gap-2 px-4 py-3 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800 rounded-b-lg">
				<button class="btn-secondary" on:click={handleClose}>Cancel</button>
				<button class="btn-primary" on:click={handleConfirm}>Save Settings</button>
			</div>
		</div>
	</div>
{/if}

<style lang="postcss">
	.ui-input {
		@apply w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm;
        background-color: white;
	}
    :global(.dark) .ui-input {
        background-color: #0d0d0d;
        border-color: #333333;
        color: white;
        color-scheme: dark;
    }
    .ui-radio {
        @apply w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600;
    }
	.btn-primary {
		@apply px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed;
	}
	.btn-secondary {
		@apply px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-gray-700 dark:text-gray-200 dark:border-gray-600 dark:hover:bg-gray-600;
	}
</style>
