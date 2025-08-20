<!-- src/lib/components/projectview/lexical/FloatingModifyHighlightToolbar.svelte -->
<script>
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
    <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
        <path stroke-linecap="round" stroke-linejoin="round" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
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

.remove-highlight {
  background: none;
  border: none;
  cursor: pointer;
  padding: 4px;
}
</style>
