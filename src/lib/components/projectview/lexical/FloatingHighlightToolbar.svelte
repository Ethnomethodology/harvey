
<!-- src/lib/components/projectview/lexical/FloatingHighlightToolbar.svelte -->
<script>
  import { onMount } from 'svelte';

  export let editor;
  export let showToolbar;
  export let toolbarPosition;
  export let onHighlight;
  export let onRemoveHighlight;

  let isDarkMode = false;
  onMount(() => {
    isDarkMode = document.documentElement.classList.contains('dark');
  });

  const highlightOptions = [
      { value: '#FFF275', label: 'Yellow' }, 
      { value: '#A8FF9E', label: 'Green' }, 
      { value: '#AEEFFF', label: 'Blue' },
      { value: '#FFB0CF', label: 'Pink' }, 
      { value: '#D0A0FF', label: 'Purple' },
  ];

  function handleHighlight(color) {
    if (onHighlight) {
      onHighlight(color);
    }
  }

  function handleRemove() {
    if (onRemoveHighlight) {
      onRemoveHighlight();
    }
  }
</script>

{#if showToolbar}
<div
  class="selection-toolbar"
  class:dark={isDarkMode}
  style="top: {toolbarPosition.top}px; left: {toolbarPosition.left}px;"
>
  <div class="highlight-options">
    {#each highlightOptions as option}
      <button
        class="color-box"
        style="background-color: {option.value};"
        on:click={() => handleHighlight(option.value)}
        title={option.label}
      ></button>
    {/each}
  </div>
  <button class="remove-highlight" on:click={handleRemove}>
    <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
      <path stroke-linecap="round" stroke-linejoin="round" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" />
    </svg>
  </button>
</div>
{/if}

<style>
.selection-toolbar {
  position: absolute;
  z-index: 10;
  background-color: #fff;
  border: 1px solid #ccc;
  border-radius: 4px;
  padding: 4px;
  display: flex;
  align-items: center;
  box-shadow: 0 2px 5px rgba(0,0,0,0.1);
}

.selection-toolbar.dark {
    background-color: #2d3748;
    border-color: #4a5568;
}

.highlight-options {
  display: flex;
  gap: 4px;
  margin-right: 8px;
}

.color-box {
  width: 24px;
  height: 24px;
  border: 1px solid #ccc;
  border-radius: 4px;
  cursor: pointer;
}

.selection-toolbar.dark .color-box {
    border-color: #718096;
}

.remove-highlight {
  background: none;
  border: none;
  cursor: pointer;
  padding: 4px;
  color: #000;
}

.selection-toolbar.dark .remove-highlight {
    color: #fff;
}
</style>
