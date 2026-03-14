<!-- src/lib/components/projectview/modals/ManualSettingsModal.svelte -->
<script>
	import { createEventDispatcher } from 'svelte';
	import { X, Settings2, Plus, Minus } from 'lucide-svelte';
    import { 
        Button, 
        Label, 
        Input, 
        Radio, 
        Helper 
    } from 'flowbite-svelte';

	export let showModal = false;
	export let currentSettings = { duration: 60, speakerMode: 'unassigned' }; // Default
    export let speakerList = []; // Array of speaker names

	const dispatch = createEventDispatcher();

    let duration = 60;
    let speakerMode = 'unassigned';
    let isInitialized = false;

    // Sync local state when modal opens
	$: if (showModal && !isInitialized) {
        duration = currentSettings.duration || 60;
        speakerMode = currentSettings.speakerMode || 'unassigned';
        if (speakerMode === 'unselected') speakerMode = 'unassigned'; // Migration
        isInitialized = true;
	}

    // Reset initialization flag when modal closes
    $: if (!showModal) {
        isInitialized = false;
    }

    $: speakerOptions = [
        { value: 'unassigned', label: 'Unassigned' },
        { value: 'alternate', label: 'Alternate Speakers', disabled: speakerList.length < 2 }
    ];

    function incrementDuration() {
        duration = duration + 1;
    }
    function decrementDuration() {
        duration = Math.max(1, duration - 1);
    }

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
		class="fixed inset-0 z-[130] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
		role="dialog"
		aria-modal="true"
		on:click={handleClose}
	>
		<div
			class="bg-white dark:bg-gray-900 rounded-xl shadow-2xl w-full max-w-sm flex flex-col border border-gray-200 dark:border-gray-800 overflow-hidden"
			on:click|stopPropagation
		>
            <!-- Header -->
            <div class="px-6 py-5 border-b border-gray-200 dark:border-gray-800 flex justify-between items-center bg-gray-50/50 dark:bg-gray-800/50">
                <div class="flex items-center space-x-3">
                    <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
                        <Settings2 size={20} class="text-blue-600 dark:text-blue-400" />
                    </div>
                    <h3 id="manual-settings-title" class="text-lg font-bold text-gray-900 dark:text-white">
                        Manual Settings
                    </h3>
                </div>
                <button on:click={handleClose} class="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-full transition-all" title="Close">
                    <X size={20} />
                </button>
            </div>

			<div class="p-6 space-y-5 overflow-y-auto max-h-[60vh]">
                <p class="text-sm text-gray-600 dark:text-gray-400">
                    Configure defaults for new segments added manually.
                </p>

				<!-- Duration Input with custom buttons -->
				<div class="space-y-2">
					<Label for="segDuration">Default Segment Duration (seconds)</Label>
					<div class="flex items-center gap-3">
                        <div class="relative flex items-center w-full max-w-[9rem]">
                            <button 
                                type="button" 
                                on:click={decrementDuration}
                                class="flex-shrink-0 bg-gray-100 dark:bg-gray-700 dark:hover:bg-gray-600 dark:border-gray-600 hover:bg-gray-200 border border-gray-300 rounded-s-lg p-2 h-9 focus:ring-gray-100 dark:focus:ring-gray-700 focus:ring-2 focus:outline-none transition-colors"
                            >
                                <Minus size={14} class="text-gray-900 dark:text-white" />
                            </button>
                            <input 
                                type="text" 
                                id="segDuration" 
                                class="bg-gray-50 border-x-0 border-gray-300 h-9 text-center text-gray-900 text-sm focus:ring-blue-500 focus:border-blue-500 block w-full py-2 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500" 
                                bind:value={duration}
                                required 
                            />
                            <button 
                                type="button" 
                                on:click={incrementDuration}
                                class="flex-shrink-0 bg-gray-100 dark:bg-gray-700 dark:hover:bg-gray-600 dark:border-gray-600 hover:bg-gray-200 border border-gray-300 rounded-e-lg p-2 h-9 focus:ring-gray-100 dark:focus:ring-gray-700 focus:ring-2 focus:outline-none transition-colors"
                            >
                                <Plus size={14} class="text-gray-900 dark:text-white" />
                            </button>
                        </div>
                        <div class="bg-gray-100 dark:bg-gray-800 px-3 py-2 rounded-lg font-mono text-xs text-gray-600 dark:text-gray-400 min-w-[4.5rem] text-center border border-gray-200 dark:border-gray-700">
                            {Math.floor(duration / 60)}m {duration % 60}s
                        </div>
                    </div>
				</div>

				<!-- Speaker Mode -->
				<div class="space-y-3 pt-2">
					<Label>Speaker Assignment</Label>
					<div class="grid grid-cols-1 gap-2">
						{#each speakerOptions as option}
                            <Radio
                                name="speakerMode"
                                value={option.value}
                                bind:group={speakerMode}
                                disabled={option.disabled}
                                class="p-2.5 rounded-lg border border-gray-100 dark:border-gray-800 bg-gray-50/50 dark:bg-gray-800/30"
                            >
                                {option.label}
                            </Radio>
						{/each}
					</div>
                    {#if speakerMode === 'alternate' && speakerList.length < 2}
                        <Helper color="yellow" class="italic text-[11px]">Add at least 2 speakers to enable alternation.</Helper>
                    {/if}
				</div>
			</div>

            <!-- Footer -->
            <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-800 flex justify-end gap-3 bg-gray-50/80 dark:bg-gray-800/80 backdrop-blur-md">
				<Button color="alternative" on:click={handleClose} title="Cancel">Cancel</Button>
				<Button color="blue" on:click={handleConfirm} title="Save Settings">Save Settings</Button>
			</div>
		</div>
	</div>
{/if}

<style lang="postcss">
</style>