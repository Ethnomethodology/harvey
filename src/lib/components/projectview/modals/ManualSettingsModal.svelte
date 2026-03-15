<!-- src/lib/components/projectview/modals/ManualSettingsModal.svelte -->
<script>
	import { createEventDispatcher } from 'svelte';
	import { X, Settings2, Plus, Minus } from 'lucide-svelte';
    import { 
        Modal,
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

<Modal
    bind:open={showModal}
    size="sm"
    autoclose={false}
    outsideclose={true}
    on:close={handleClose}
    backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
    dialogClass="fixed top-0 start-0 end-0 h-modal md:h-full z-[10001] w-full p-4 flex items-center justify-center"
    class="w-full p-0 overflow-hidden flex flex-col"
    headerClass="px-6 py-5 border-b border-gray-200 dark:border-gray-800 flex justify-between items-center bg-gray-50/50 dark:bg-gray-800/50"
>
    <!-- Header -->
    <div slot="header" class="flex items-center space-x-3 w-full">
        <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
            <Settings2 size={20} class="text-blue-600 dark:text-blue-400" />
        </div>
        <h3 id="manual-settings-title" class="text-lg font-bold text-gray-900 dark:text-white">
            Manual Settings
        </h3>
    </div>

    <div class="p-6 space-y-5">
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
    <div slot="footer" class="flex justify-end gap-3 w-full">
        <Button color="alternative" on:click={handleClose} title="Cancel">Cancel</Button>
        <Button color="blue" on:click={handleConfirm} title="Save Settings">Save Settings</Button>
    </div>
</Modal>

<style lang="postcss">
</style>