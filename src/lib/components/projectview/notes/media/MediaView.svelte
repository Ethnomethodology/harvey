<!-- src/lib/components/projectview/notes/media/MediaView.svelte -->
<script>
    import { onMount, createEventDispatcher } from 'svelte';
    import LeftInfoPanel from '../shared_panels/LeftInfoPanel.svelte';
    import RightInfoPanel from '../shared_panels/RightInfoPanel.svelte';
    import MediaEditorPanel from './MediaEditorPanel.svelte'; // This will be created next

    export let itemPath = null; // Receives the full media file path from NotesView

    const dispatch = createEventDispatcher();

    // Forward events if needed by parent (NotesView or ProjectView)
    function forwardEvent(event) {
        console.log(`[MediaView] Forwarding event: ${event.type}`);
		dispatch(event.type, event.detail);
	}

    onMount(() => {
		console.log('[MediaView] Component container mounted. Media path:', itemPath);
	});

    $: { 
        // Log when the media path changes
        console.log(`[MediaView] Path is now ${itemPath}`);
    }

</script>

<div class="flex flex-grow p-0 gap-1 w-full min-h-0 h-full">
    <!-- Left Panel (Shared) -->
    <div class="w-[20.588%] h-full flex-shrink-0">
        <!-- itemType="media_note" to provide context to LeftInfoPanel if it needs specific logic -->
        <LeftInfoPanel itemPath={itemPath} itemType="media_note" />
    </div>

    <!-- Middle Panel - The Media Player and Transcript Editor -->
    <div class="w-[58.824%] h-full">
        {#key itemPath}
            {#if itemPath}
                <MediaEditorPanel mediaPath={itemPath} />
            {:else}
             <div class="h-full bg-gray-200 dark:bg-gray-700 rounded-md shadow flex items-center justify-center text-gray-500">
                 <span>No media file path provided to MediaView.</span>
             </div>
            {/if}
        {/key}
    </div>

    <!-- Right Panel (Shared) -->
    <div class="w-[20.588%] h-full flex-shrink-0">
        <RightInfoPanel itemPath={itemPath} itemType="media_note" />
    </div>
</div>

<style>
	.min-h-0 { min-height: 0; }
    /* Define width classes using arbitrary values, consistent with other views */
    .w-\[20\.588\%\] { width: 20.58825%; }
    .w-\[58\.824\%\] { width: 58.8235%; }
</style>