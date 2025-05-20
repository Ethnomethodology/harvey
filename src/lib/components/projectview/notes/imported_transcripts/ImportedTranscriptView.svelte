<!-- src/lib/components/projectview/notes/imported_transcripts/ImportedTranscriptView.svelte -->
<script>
    import { onMount, createEventDispatcher } from 'svelte';
    import LeftInfoPanel from '../shared_panels/LeftInfoPanel.svelte';
    import RightInfoPanel from '../shared_panels/RightInfoPanel.svelte';
    import TranscriptEditorPanel from './TranscriptEditorPanel.svelte';

    export let itemPath = null; // Receives the full path from NotesView

    const dispatch = createEventDispatcher();

    function forwardEvent(event) {
        console.log(`[ImportedTranscriptView] Forwarding event: ${event.type}`);
		dispatch(event.type, event.detail);
	}

    onMount(() => {
		console.log('[ImportedTranscriptView] Component container mounted. Transcript path:', itemPath);
	});

    $: { 
        console.log(`[ImportedTranscriptView] Path is now ${itemPath}`);
    }

</script>

<div class="flex flex-grow p-0 gap-1 w-full min-h-0 h-full">
    <div class="w-[20.588%] h-full flex-shrink-0">
        <LeftInfoPanel itemPath={itemPath} itemType="imported_transcript" />
    </div>

    <div class="w-[58.824%] h-full">
        {#key itemPath}
            {#if itemPath}
                <TranscriptEditorPanel itemPath={itemPath} />
            {:else}
             <div class="h-full bg-gray-200 dark:bg-gray-700 rounded-md shadow flex items-center justify-center text-gray-500">
                 <span>No transcript path provided to ImportedTranscriptView.</span>
             </div>
            {/if}
        {/key}
    </div>

    <div class="w-[20.588%] h-full flex-shrink-0">
        <RightInfoPanel itemPath={itemPath} itemType="imported_transcript" />
    </div>
</div>

<style>
	.min-h-0 { min-height: 0; }
    .w-\[20\.588\%\] { width: 20.58825%; }
    .w-\[58\.824\%\] { width: 58.8235%; }
</style>