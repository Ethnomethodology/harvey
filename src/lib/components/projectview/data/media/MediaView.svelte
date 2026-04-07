<!-- src/lib/components/projectview/data/media/MediaView.svelte -->
<script>
  import { onMount, createEventDispatcher } from 'svelte';
  // LeftInfoPanel and RightInfoPanel are removed
  // panelStateStore might not be needed if panel collapsing handled by parent
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


</script>

<!-- Main container for the Media View - this will now be the main content panel -->
<div class="h-full flex-grow min-w-0 bg-white dark:bg-gray-950">
  {#key itemPath}
    {#if itemPath}
      <MediaEditorPanel
        mediaPath={itemPath}
        on:requestTranscriptionTabWithMedia={forwardEvent}
        on:requestTrimInTranscriptionTab={forwardEvent}
      />
    {:else}
      <div
        class="h-full bg-gray-200 dark:bg-gray-800 flex items-center justify-center text-gray-500"
      >
        <span>No media file path provided to MediaView.</span>
      </div>
    {/if}
  {/key}
</div>

<style>

  /* Removed specific width classes as this component now fills the space given by DataView */
</style>
