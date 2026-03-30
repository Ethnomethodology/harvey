<script>
  import { Toolbar, Button, Tooltip } from 'flowbite-svelte';
  import { Trash2 } from '@lucide/svelte';

  export let showToolbar = false;
  export let toolbarPosition = { top: 0, left: 0 };
  export let onHighlight;
  export let onRemoveHighlight;

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
  class="fixed z-[100000] pointer-events-auto"
  style="top: {toolbarPosition.top}px; left: {toolbarPosition.left}px;"
>
  <Toolbar embedded class="rounded-full shadow-xl bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-700 px-1 py-1 flex items-center gap-x-0.5">
    {#each highlightOptions as option}
      <Button color="none" class="p-1 rounded-full hover:scale-110 transition-transform duration-100" on:click={() => handleHighlight(option.value)}>
        <span class="w-[18px] h-[18px] rounded-full border border-gray-300 dark:border-gray-600 block shadow-sm" style="background-color: {option.value}"></span>
      </Button>
    {/each}
    <div class="w-px h-4 bg-gray-300 dark:bg-gray-700 mx-1"></div>
    <Button color="none" class="p-1.5 rounded-full hover:bg-red-50 dark:hover:bg-red-900/30 group" on:click={handleRemove}>
      <Trash2 class="w-4 h-4 text-red-500 group-hover:text-red-600" />
    </Button>
  </Toolbar>
</div>
{/if}

<style>
  /* Removed custom CSS in favor of Flowbite components and Tailwind utility classes */
</style>
