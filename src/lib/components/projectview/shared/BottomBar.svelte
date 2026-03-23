<!-- src/lib/components/projectview/BottomBar.svelte -->
<script>
  // *** FIXED: Import single project store ***
import { project } from '$lib/stores/projectStore.js';

  // *** FIXED: Read relevant state from $project ***
  $: isDirty = $project.transcriptDirty;
  $: statusMessage = $project.statusMessage; // Use the message from the store
  $: isLoading = $project.isLoading;
  $: error = $project.error;
  $: textCount = $project.documentTextCount;
  $: isIdle = !isLoading && !error && (!statusMessage || statusMessage === 'Ready' || statusMessage.includes('Loaded data') || statusMessage.includes('Loaded project') || statusMessage.includes('Project refreshed'));

</script>

<div class="p-1.5 px-4 text-xs bg-white dark:bg-gray-950 border-t border-gray-200 dark:border-gray-800 text-gray-600 dark:text-gray-400 flex justify-between items-center h-8 flex-shrink-0">
  <div class="flex items-center flex-grow min-w-0">
    {#if isLoading}
      <span>Loading project...</span>
    {:else if error}
      <span class="text-red-600 truncate" title={error}>Error: {error}</span>
    {:else}
      <span class="truncate" title={statusMessage}>{statusMessage || 'Ready'}</span>
    {/if}
  </div>

  <div class="flex items-center gap-4 flex-shrink-0 ml-4">
    {#if isIdle && textCount}
      <span class="text-gray-500 font-medium" title="Word & Character Count">
        {textCount.words} words, {textCount.chars} chars
      </span>
    {/if}
    {#if isDirty && !isLoading && !error}
      <span class="text-orange-600 font-medium">Unsaved Changes</span>
    {/if}
  </div>
</div>