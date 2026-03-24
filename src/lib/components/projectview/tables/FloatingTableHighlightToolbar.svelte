<!-- src/lib/components/projectview/tables/FloatingTableHighlightToolbar.svelte -->
<script>
  import { Trash2 } from '@lucide/svelte';
  import { onMount } from 'svelte';


  export let showToolbar = false;
  export let toolbarPosition = { top: 0, left: 0 };
  export let onChangeColor;
  export let onDelete;
  export let onClose;

  const highlightOptions = [
      { value: '#FFF275', label: 'Yellow' }, 
      { value: '#A8FF9E', label: 'Green' }, 
      { value: '#AEEFFF', label: 'Blue' },
      { value: '#FFB0CF', label: 'Pink' }, 
      { value: '#D0A0FF', label: 'Purple' },
  ];

  function handleChange(color) {
    if (onChangeColor) {
      onChangeColor(color);
    }
  }

  function handleDelete() {
    if (onDelete) {
      onDelete();
    }
  }

  function handleClose() {
    if (onClose) {
      onClose();
    }
  }
</script>

{#if showToolbar}
<div
  class="selection-toolbar"
  style="top: {toolbarPosition.top}px; left: {toolbarPosition.left}px;"
>
  <div class="highlight-options">
    {#each highlightOptions as option}
      <button
        class="color-box"
        style="background-color: {option.value};"
        on:click={() => handleChange(option.value)}
        title={option.label}
      ></button>
    {/each}
  </div>
  <button class="remove-highlight border-l border-gray-300 dark:border-gray-700 pl-1 ml-1" on:click={handleDelete} title="Remove Highlight">
    <Trash2 class="h-4 w-4" />
  </button>
</div>
{/if}

<style>
.selection-toolbar {
  position: fixed;
  z-index: 100000;
  background-color: #fff;
  border: 1px solid #9ca3af; /* gray-400 */
  border-radius: 4px;
  padding: 4px 8px;
  display: flex;
  align-items: center;
  gap: 4px;
  box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05); /* shadow-lg */
  pointer-events: auto; /* Ensure it catches clicks */
}

:global(html.dark) .selection-toolbar {
  background-color: #111827; /* gray-900 */
  border-color: #374151; /* gray-700 */
}

.highlight-options {
  display: flex;
  gap: 6px;
  margin-right: 4px;
}

.color-box {
  width: 18px; 
  height: 18px;
  border: 1px solid #9ca3af; /* gray-400 */
  border-radius: 9999px; /* rounded-full */
  cursor: pointer;
  transition: transform 0.1s ease;
}

.color-box:hover {
    transform: scale(1.1);
}

:global(html.dark) .color-box {
    border-color: #374151; /* gray-700 */
}

.remove-highlight, .close-toolbar {
  background: none;
  border: none;
  cursor: pointer;
  padding: 4px;
  border-radius: 0.25rem;
  display: flex;
  align-items: center;
  justify-content: center;
}

.remove-highlight {
  color: #ef4444; /* red-500 */
}

.remove-highlight:hover {
    background-color: #fee2e2; /* red-100 */
}

:global(html.dark) .remove-highlight:hover {
    background-color: #450a0a; /* red-950/20 */
}
</style>
