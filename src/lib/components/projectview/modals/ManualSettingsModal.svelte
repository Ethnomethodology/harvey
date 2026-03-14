<!-- src/lib/components/projectview/modals/ManualSettingsModal.svelte -->
<script>
	import { createEventDispatcher } from 'svelte';
	import { X, Mic } from 'lucide-svelte';
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
                        <Mic size={20} class="text-blue-600 dark:text-blue-400" />
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

				<!-- Duration Input -->
				<div class="space-y-2">
					<Label for="segDuration">Default Segment Duration (seconds)</Label>
					<div class="flex items-center gap-3">
                        <Input
                            id="segDuration"
                            type="number"
                            min="1"
                            class="flex-1"
                            bind:value={duration}
                        />
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