<!-- src/lib/components/projectview/data/imported_transcripts/ImportedTranscriptView.svelte -->
<script>
    import { onMount, createEventDispatcher } from 'svelte';
    // LeftInfoPanel and RightInfoPanel are removed
    // panelStateStore might not be needed if panel collapsing handled by parent
    import TranscriptEditorPanel from './TranscriptEditorPanel.svelte';

    export let itemPath = null; // Receives the full path from DataView

    const dispatch = createEventDispatcher();

    function forwardEvent(event) {
        console.log(`[ImportedTranscriptView] Forwarding event: ${event.type}`);
		dispatch(event.type, event.detail);
	}

    onMount(() => {
		console.log('[ImportedTranscriptView] Component container mounted. Transcript path:', itemPath);
	});

    $: { 
        // console.log(`[ImportedTranscriptView] Path is now ${itemPath}`);
    }

</script>

<!-- Main container for the Imported Transcript View - this will now be the main content panel -->
<div class="h-full flex-grow min-w-0 bg-white dark:bg-gray-800">
    {#key itemPath}
        {#if itemPath}
            <TranscriptEditorPanel itemPath={itemPath} />
        {:else}
            <div class="h-full bg-gray-200 dark:bg-gray-700 flex items-center justify-center text-gray-500">
                <span>No transcript path provided to ImportedTranscriptView.</span>
            </div>
        {/if}
    {/key}
</div>

<style>
	.min-h-0 { min-height: 0; }
    /* Removed specific width classes as this component now fills the space given by DataView */
</style>