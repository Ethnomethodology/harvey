<script>
  import { project, toggleTagInHighlightLocal } from '$lib/stores/projectStore.js';
  import { allTags } from '$lib/stores/tagStore.js';
  import { Toolbar, Button, Dropdown, Checkbox } from 'flowbite-svelte';
  import { Trash2, Tag } from '@lucide/svelte';

  export let showToolbar;
  export let toolbarPosition;
  export let onChangeColor;
  export let onDelete;
  export let highlightId;
  export let docType;
  export let filePath;

  // Derive current tags for this specific highlight from the project store
  $: currentHighlight = (() => {
    let highlights = [];
    if (docType === 'pdf') highlights = $project.currentPdfAnnotations;
    else if (docType === 'table') highlights = $project.currentTableHighlights;
    else highlights = $project.currentDocumentHighlights;
    
    return highlights.find(h => h.id === highlightId);
  })();

  $: activeTags = currentHighlight?.tags || [];

  function handleTagToggle(tagName) {
    toggleTagInHighlightLocal(highlightId, tagName, docType, filePath);
  }

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
  class="fixed z-[100000] pointer-events-auto"
  style="top: {toolbarPosition.top}px; left: {toolbarPosition.left}px;"
>
  <Toolbar embedded class="rounded-full shadow-xl bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-700 px-1 py-1 flex items-center gap-x-0.5">
    {#each highlightOptions as option}
      <Button color="none" class="p-1 rounded-full hover:scale-110 transition-transform duration-100" on:click={() => handleChange(option.value)}>
        <span class="w-[18px] h-[18px] rounded-full border border-gray-300 dark:border-gray-600 block shadow-sm" style="background-color: {option.value}"></span>
      </Button>
    {/each}
    <div class="w-px h-4 bg-gray-300 dark:bg-gray-700 mx-1"></div>
    <Button color="none" class="p-1.5 rounded-full hover:bg-gray-100 dark:hover:bg-gray-800 group relative">
      <Tag class="w-4 h-4 text-gray-500 group-hover:text-blue-500" />
      <Dropdown class="w-48 p-3 space-y-1 text-sm z-[100001]">
        <li class="p-1 border-b border-gray-100 dark:border-gray-600 mb-1">
          <span class="font-medium text-gray-900 dark:text-gray-300">Tags</span>
        </li>
        {#each $allTags as tag}
          <li class="rounded hover:bg-gray-100 dark:hover:bg-gray-600">
            <Checkbox 
              checked={activeTags.includes(tag.name)} 
              on:change={() => handleTagToggle(tag.name)}
              class="items-center px-2 py-1.5 w-full cursor-pointer"
            >
              {tag.name}
            </Checkbox>
          </li>
        {/each}
        {#if $allTags.length === 0}
          <li class="p-2 text-gray-500 italic text-xs">No tags available</li>
        {/if}
      </Dropdown>
    </Button>
    <Button color="none" class="p-1.5 rounded-full hover:bg-red-50 dark:hover:bg-red-900/30 group" on:click={handleDelete}>
      <Trash2 class="w-4 h-4 text-red-500 group-hover:text-red-600" />
    </Button>
  </Toolbar>
</div>
{/if}

<style>
  /* Removed custom CSS in favor of Flowbite components and Tailwind utility classes */
</style>
