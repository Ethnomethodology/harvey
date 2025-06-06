<!-- src/lib/components/projectview/notes/images/ImageView.svelte -->
<script>
    import { onMount, createEventDispatcher } from 'svelte';
    // Import shared panels
    import LeftInfoPanel from '../shared_panels/LeftInfoPanel.svelte';
    import RightInfoPanel from '../shared_panels/RightInfoPanel.svelte';
    import panelStateStore from '$lib/stores/panelStateStore.js';
    // Import the specific image viewer panel (placeholder for now)
    import ImageViewerPanel from './ImageViewerPanel.svelte';

    export let itemPath = null; // Receives the full path from NotesView

    const dispatch = createEventDispatcher();

    function forwardEvent(event) {
        console.log(`[ImageView] Forwarding event: ${event.type}`);
		dispatch(event.type, event.detail);
	}

    onMount(() => {
		console.log('[ImageView] Component container mounted. Image path:', itemPath);
	});

    $: { // Log when path changes
        console.log(`[ImageView] Path is now ${itemPath}`);
    }

</script>

<!-- Main container for the Image View -->
<div class="flex flex-grow p-0 gap-1 w-full min-h-0 h-full">

    <!-- Left Panel (Shared) -->
    <div class="h-full flex-shrink-0 transition-all duration-300 ease-in-out"
         class:w-12={$panelStateStore.leftCollapsed}
         class:w-[20.588%]={!$panelStateStore.leftCollapsed}>
        <LeftInfoPanel itemPath={itemPath} itemType="image" />
    </div>

    <!-- Middle Panel - The Image Viewer -->
    <div class="h-full flex-grow">
        {#key itemPath} {#if itemPath}
             <ImageViewerPanel imagePath={itemPath} />
        {:else}
             <!-- Optional: Show a placeholder if itemPath is null -->
             <div class="h-full bg-gray-200 dark:bg-gray-700 rounded-md shadow flex items-center justify-center text-gray-500">
                 <span>No image path provided to ImageView.</span>
             </div>
        {/if} {/key}
    </div>

    <!-- Right Panel (Shared) -->
    <div class="h-full flex-shrink-0 transition-all duration-300 ease-in-out"
         class:w-12={$panelStateStore.rightCollapsed}
         class:w-[20.588%]={!$panelStateStore.rightCollapsed}>
        <RightInfoPanel itemPath={itemPath} itemType="image" />
    </div>

</div>

<style>
	.min-h-0 { min-height: 0; }
    /* Define width classes using arbitrary values */
    .w-\[20\.588\%\] { width: 20.58825%; }
</style>