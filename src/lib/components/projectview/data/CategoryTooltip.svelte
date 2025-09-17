<!-- src/lib/components/projectview/notes/CategoryTooltip.svelte -->
<script>
  export let categoryName = '';
  export let files = []; // Expected: array of objects like { name: string, path: string }
  export let visible = false;
  export let x = 0;
  export let y = 0;

  // Reactive statement to prevent rendering if not visible,
  // though the #if block in template is the primary guard.
  $: if (!visible) {
    // Optional: any cleanup when becoming not visible, though likely not needed for this simple tooltip
  }
</script>

{#if visible}
  <div
    class="absolute bg-white dark:bg-gray-700 border border-gray-300 dark:border-border shadow-lg p-2 rounded-md z-50 text-xs text-gray-800 dark:text-gray-200 max-w-xs"
    style="left: {x}px; top: {y}px; pointer-events: none;"
  >
    {#if categoryName}
      <strong class="block mb-1">{categoryName}</strong>
    {/if}

    {#if files && files.length > 0}
      <ul class="list-none p-0 m-0 max-h-48 overflow-y-auto">
        {#each files as file (file.path || file.name)}
          <li class="truncate" title={file.name}>{file.name}</li>
        {/each}
      </ul>
    {:else}
      <p class="italic m-0">No files in this category.</p>
    {/if}
  </div>
{/if}

<style>
  /* Optional: Add any specific styles for the tooltip here if needed, e.g., for the scrollbar */
  .max-h-48::-webkit-scrollbar {
    width: 5px;
    height: 5px;
  }
  .max-h-48::-webkit-scrollbar-track {
    background: transparent;
  }
  .max-h-48::-webkit-scrollbar-thumb {
    background: #ccc;
    border-radius: 3px;
  }
  .max-h-48::-webkit-scrollbar-thumb:hover {
    background: #bbb;
  }
  .max-h-48 {
    scrollbar-width: thin;
    scrollbar-color: #ccc transparent;
  }
  html.dark .max-h-48::-webkit-scrollbar-thumb {
    background: #555;
  }
  html.dark .max-h-48::-webkit-scrollbar-thumb:hover {
    background: #666;
  }
  html.dark .max-h-48 {
    scrollbar-color: #555 transparent;
  }
</style>
