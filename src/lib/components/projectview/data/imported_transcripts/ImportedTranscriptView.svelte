<script>
    import { onMount, createEventDispatcher } from 'svelte';
    import { project, clearImportedTranscriptSplit } from '$lib/stores/projectStore.js';
    import TranscriptEditorPanel from './TranscriptEditorPanel.svelte';

    export let itemPath = null; // Receives the full path from DataView

    const dispatch = createEventDispatcher();

    $: splitPartnerPath = $project.importedTranscriptSplits[itemPath];

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
        <div class="flex h-full w-full divide-x divide-gray-300 dark:divide-gray-600">
            <div class="w-1/2 h-full overflow-hidden flex flex-col">
                <div class="bg-gray-100 dark:bg-surface-3 px-2 py-1 text-xs font-semibold text-gray-600 dark:text-gray-400 border-b border-gray-300 dark:border-gray-600 truncate">
                    {itemPath.split(/[\\/]/).pop()}
                </div>
                <div class="flex-grow overflow-hidden">
                    {#key itemPath}
                        <TranscriptEditorPanel itemPath={itemPath} isPrimary={true} />
                    {/key}
                </div>
            </div>
            <div class="w-1/2 h-full overflow-hidden flex flex-col">
                <div class="bg-gray-100 dark:bg-surface-3 px-2 py-1 text-xs font-semibold text-gray-600 dark:text-gray-400 border-b border-gray-300 dark:border-gray-600 truncate flex justify-between items-center">
                    <span class="truncate">{splitPartnerPath.split(/[\\/]/).pop()}</span>
                    <button 
                        class="hover:text-red-500" 
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
                <TranscriptEditorPanel itemPath={itemPath} isPrimary={true} />
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