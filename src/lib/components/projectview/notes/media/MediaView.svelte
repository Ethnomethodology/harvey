<!-- src/lib/components/projectview/notes/media/MediaView.svelte -->
<script>
    import { onMount, createEventDispatcher } from 'svelte';
    import LeftInfoPanel from '../shared_panels/LeftInfoPanel.svelte';
    import RightInfoPanel from '../shared_panels/RightInfoPanel.svelte';
    import MediaEditorPanel from './MediaEditorPanel.svelte';

    export let itemPath = null;

    const dispatch = createEventDispatcher();

    function forwardEvent(event) {
        console.log(`[MediaView] Forwarding event: ${event.type} with detail:`, event.detail);
		dispatch(event.type, event.detail);
	}

    onMount(() => {
		console.log('[MediaView] Component container mounted. Media path:', itemPath);
	});

    $: {
        console.log(`[MediaView] Path is now ${itemPath}`);
    }

</script>

<div class="flex flex-grow p-0 gap-1 w-full min-h-0 h-full">
    <div class="w-[20.588%] h-full flex-shrink-0">
        <LeftInfoPanel itemPath={itemPath} itemType="media_note" />
    </div>

    <div class="w-[58.824%] h-full">
        {#key itemPath}
            {#if itemPath}
                <MediaEditorPanel
                    mediaPath={itemPath}
                    on:requestTranscriptionTabWithMedia={forwardEvent}
                    on:requestTrimInTranscriptionTab={forwardEvent}
                />
            {:else}
             <div class="h-full bg-gray-200 dark:bg-gray-700 rounded-md shadow flex items-center justify-center text-gray-500">
                 <span>No media file path provided to MediaView.</span>
             </div>
            {/if}
        {/key}
    </div>

    <div class="w-[20.588%] h-full flex-shrink-0">
        <RightInfoPanel itemPath={itemPath} itemType="media_note" />
    </div>
</div>

<style>
	.min-h-0 { min-height: 0; }
    .w-\[20\.588\%\] { width: 20.58825%; }
    .w-\[58\.824\%\] { width: 58.8235%; }
</style>