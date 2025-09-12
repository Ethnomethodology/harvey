<!-- src/lib/components/projectview/BottomBar.svelte -->
<script>
  // *** FIXED: Import single project store ***
import { project } from '$lib/stores/projectStore.js';

  // *** FIXED: Read relevant state from $project ***
  $: isDirty = $project.transcriptDirty;
  $: statusMessage = $project.statusMessage; // Use the message from the store
  $: isLoading = $project.isLoading;
  $: error = $project.error;

</script>

<div class="p-1.5 px-4 text-xs bg-white dark:bg-dark-bg-primary border-t border-gray-200 dark:border-dark-bg-tertiary text-gray-600 dark:text-dark-text-secondary flex justify-between items-center h-8 flex-shrink-0">
{#if isLoading}
  <span>Loading project...</span>
  {:else if error}
      <span class="text-red-600 truncate" title={error}>Error: {error}</span>
{:else}
  <span class="truncate" title={statusMessage}>{statusMessage || 'Ready'}</span>
{/if}

  {#if isDirty && !isLoading && !error}
     <span class="text-orange-600 font-medium flex-shrink-0 ml-4">Unsaved Changes</span>
  {/if}
</div>