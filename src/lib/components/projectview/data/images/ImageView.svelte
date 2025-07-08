<!-- src/lib/components/projectview/data/images/ImageView.svelte -->
<script>
    import { onMount, createEventDispatcher } from 'svelte';
    // LeftInfoPanel and RightInfoPanel are removed
    // panelStateStore might not be needed if panel collapsing handled by parent
    import ImageViewerPanel from './ImageViewerPanel.svelte';

    export let itemPath = null; // Receives the full path from DataView

    const dispatch = createEventDispatcher();

    function forwardEvent(event) {
        console.log(`[ImageView] Forwarding event: ${event.type}`);
		dispatch(event.type, event.detail);
	}

    onMount(() => {
		console.log('[ImageView] Component container mounted. Image path:', itemPath);
	});

    $: {
        // console.log(`[ImageView] Path is now ${itemPath}`);
    }

</script>

<!-- Main container for the Image View - this will now be the main content panel -->
<div class="h-full flex-grow min-w-0 bg-white dark:bg-gray-800 rounded-md shadow">
    {#key itemPath}
        {#if itemPath}
            <ImageViewerPanel imagePath={itemPath} />
        {:else}
            <div class="h-full bg-gray-200 dark:bg-gray-700 rounded-md shadow flex items-center justify-center text-gray-500">
                <span>No image path provided to ImageView.</span>
            </div>
        {/if}
    {/key}
</div>

<style>
	.min-h-0 { min-height: 0; }
    /* Removed specific width classes as this component now fills the space given by DataView */
</style>