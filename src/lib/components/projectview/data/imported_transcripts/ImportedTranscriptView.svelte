<script>
    import { onMount, createEventDispatcher } from 'svelte';
    import { project, clearImportedTranscriptSplit } from '$lib/stores/projectStore.js';
    import TranscriptEditorPanel from './TranscriptEditorPanel.svelte';
    import ThinMediaPlayer from '../shared_panels/ThinMediaPlayer.svelte';

    export let itemPath = null; // Receives the full path from DataView

    const dispatch = createEventDispatcher();

    $: splitInfo = $project.importedTranscriptSplits[itemPath];
    $: splitPartnerPath = splitInfo?.partner;
    $: orientation = splitInfo?.orientation || 'horizontal';

    function forwardEvent(event) {
        console.log(`[ImportedTranscriptView] Forwarding event: ${event.type}`);
		dispatch(event.type, event.detail);
	}

    onMount(() => {
		console.log('[ImportedTranscriptView] Component container mounted. Transcript path:', itemPath);
	});

</script>

<!-- Main container for the Imported Transcript View -->
<div class="h-full flex-grow min-w-0 bg-white dark:bg-surface-2 overflow-hidden">
    {#if splitPartnerPath}
        <div class="flex h-full w-full divide-gray-300 dark:divide-gray-600 {orientation === 'horizontal' ? 'flex-row divide-x' : 'flex-col divide-y'}">
            <div class="{orientation === 'horizontal' ? 'w-1/2 h-full' : 'h-1/2 w-full'} overflow-hidden flex flex-col">
                <div class="bg-gray-100 dark:bg-surface-3 px-2 py-1 text-xs font-semibold text-gray-600 dark:text-gray-400 border-b border-gray-300 dark:border-gray-600 flex items-center h-8">
                    <span class="truncate">{itemPath.split(/[\\/]/).pop()}</span>
                </div>
                <div class="flex-grow overflow-hidden">
                    {#key itemPath}
                        <TranscriptEditorPanel itemPath={itemPath} isPrimary={true} />
                    {/key}
                </div>
            </div>
            <div class="{orientation === 'horizontal' ? 'w-1/2 h-full' : 'h-1/2 w-full'} overflow-hidden flex flex-col">
                <div class="bg-gray-100 dark:bg-surface-3 px-2 py-1 text-xs font-semibold text-gray-600 dark:text-gray-400 border-b border-gray-300 dark:border-gray-600 flex justify-between items-center h-8">
                    <div class="flex items-center min-w-0 flex-grow">
                        <span class="truncate">{splitPartnerPath.split(/[\\/]/).pop()}</span>
                    </div>
                    <button 
                        class="hover:text-red-500 ml-2 flex-shrink-0" 
                        title="Close Split"
                        on:click={() => clearImportedTranscriptSplit(itemPath)}
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-x-lg" viewBox="0 0 16 16">
                            <path d="M2.146 2.854a.5.5 0 1 1 .708-.708L8 7.293l5.146-5.147a.5.5 0 0 1 .708.708L8.707 8l5.147 5.146a.5.5 0 0 1-.708.708L8 8.707l-5.146 5.147a.5.5 0 0 1-.708-.708L7.293 8z"/>
                        </svg>
                    </button>
                </div>
                <div class="flex-grow overflow-hidden">
                    {#key splitPartnerPath}
                        <TranscriptEditorPanel itemPath={splitPartnerPath} isPrimary={false} />
                    {/key}
                </div>
            </div>
        </div>
    {:else}
        {#key itemPath}
            {#if itemPath}
                <div class="h-full flex flex-col">
                    <div class="bg-gray-100 dark:bg-surface-3 px-2 py-1 text-xs font-semibold text-gray-600 dark:text-gray-400 border-b border-gray-300 dark:border-gray-600 flex items-center h-8">
                        <span class="truncate">{itemPath.split(/[\\/]/).pop()}</span>
                    </div>
                    <div class="flex-grow overflow-hidden">
                        <TranscriptEditorPanel itemPath={itemPath} isPrimary={true} />
                    </div>
                </div>
            {:else}
                <div class="h-full bg-gray-200 dark:bg-d-gray-700 flex items-center justify-center text-gray-500">
                    <span>No transcript path provided to ImportedTranscriptView.</span>
                </div>
            {/if}
        {/key}
    {/if}
</div>

<style>
	.min-h-0 { min-height: 0; }
    /* Removed specific width classes as this component now fills the space given by DataView */
</style>