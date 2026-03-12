<!-- src/lib/components/projectview/lexical/FloatingModifyHighlightToolbar.svelte -->
<script>
  import { Trash2 } from 'lucide-svelte';
  export let editor;
  export let showToolbar;
  export let toolbarPosition;
  export let onChangeColor;
  export let onDelete;

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
  <button class="remove-highlight" on:click={handleDelete}>
    <Trash2 class="h-4 w-4" />
  </button>
</div>
{/if}

<style>
.selection-toolbar {
  position: absolute;
  z-index: 10;
  background-color: #fff;
  border: 1px solid #9ca3af; /* gray-400 */
  border-radius: 4px;
  padding: 2px 4px;
  display: flex;
  align-items: center;
  gap: 2px;
  box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05); /* shadow-lg */
}

:global(html.dark) .selection-toolbar {
  background-color: #111827; /* gray-900 */
  border-color: #374151; /* gray-700 */
}

.highlight-options {
  display: flex;
  gap: 4px;
  margin-right: 4px;
}

.color-box {
  width: 16px; /* align with pdf viewer w-4 h-4 */
  height: 16px;
  border: 1px solid #9ca3af; /* gray-400 */
  border-radius: 9999px; /* rounded-full */
  cursor: pointer;
}

:global(html.dark) .color-box {
    border-color: #374151; /* gray-700 */
}

.remove-highlight {
  background: none;
  border: 1px solid transparent;
  cursor: pointer;
  padding: 4px;
  border-radius: 0.25rem;
  color: #ef4444; /* red-500 */
  display: flex;
  align-items: center;
  justify-content: center;
}

.remove-highlight:hover {
    background-color: #e5e7eb; /* gray-200 */
}

:global(html.dark) .remove-highlight {
    color: #f87171; /* red-400 */
}

:global(html.dark) .remove-highlight:hover {
    background-color: #374151; /* gray-700 */
}
</style>
