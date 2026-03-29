<script>
  import { project, toggleTagInHighlightLocal } from '$lib/stores/projectStore.js';
  import { allTags, allTagGroups } from '$lib/stores/tagStore.js';
  import { Toolbar, Button, Dropdown, Checkbox, DropdownItem } from 'flowbite-svelte';
  import { Trash2, Tag, ChevronRight, Check } from '@lucide/svelte';
  import { onMount } from 'svelte';

  export let showToolbar = false;
  export let toolbarPosition = { top: 0, left: 0 };
  export let onChangeColor;
  export let onDelete;
  export let onClose;
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

  $: ungroupedTags = $allTags.filter(t => t.tag_group_id === null || t.tag_group_id === undefined);
  $: groupedTagsMap = $allTags.reduce((acc, tag) => {
    if (tag.tag_group_id !== null && tag.tag_group_id !== undefined) {
      if (!acc[tag.tag_group_id]) acc[tag.tag_group_id] = [];
      acc[tag.tag_group_id].push(tag);
    }
    return acc;
  }, {});

  function isGroupChecked(groupId) {
    const tags = groupedTagsMap[groupId] || [];
    return tags.some(t => activeTags.includes(t.name));
  }

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

  function handleClose() {
    if (onClose) {
      onClose();
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
    <Button color="none" class="p-1.5 rounded-full hover:bg-gray-100 dark:hover:bg-gray-800 group relative focus:ring-0">
      <Tag class="w-4 h-4 text-gray-500 group-hover:text-blue-500" />
      <Dropdown class="w-56 p-2 space-y-1 text-sm z-[100001]">
        <div class="px-2 py-1 border-b border-gray-100 dark:border-gray-600 mb-1">
          <span class="font-medium text-gray-900 dark:text-gray-300">Tags</span>
        </div>

        {#each $allTagGroups as group}
          <DropdownItem class="flex items-center justify-between px-2 py-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-600 cursor-pointer">
            <div class="flex items-center gap-2 truncate">
              {#if isGroupChecked(group.id)}
                <Check class="w-3.5 h-3.5 text-blue-500 shrink-0" />
              {:else}
                <div class="w-3.5 h-3.5 shrink-0"></div>
              {/if}
              <span class="truncate">{group.name}</span>
            </div>
            <ChevronRight class="w-4 h-4 text-gray-400 shrink-0" />
          </DropdownItem>
          <Dropdown placement="right-start" trigger="hover" class="w-48 p-2 space-y-1 z-[100002]">
            {#if (groupedTagsMap[group.id] || []).length > 0}
              {#each groupedTagsMap[group.id] as tag}
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
            {:else}
              <li class="p-2 text-gray-500 italic text-xs">No tags in group</li>
            {/if}
          </Dropdown>
        {/each}

        {#if ungroupedTags.length > 0}
          {#if $allTagGroups.length > 0}
             <div class="h-px bg-gray-100 dark:bg-gray-600 my-1"></div>
          {/if}
          {#each ungroupedTags as tag}
            <li class="rounded hover:bg-gray-100 dark:hover:bg-gray-600">
              <Checkbox
                checked={activeTags.includes(tag.name)}
                on:change={() => handleTagToggle(tag.name)}
                class="items-center px-2 py-1.5 w-full cursor-pointer ml-[22px]"
              >
                {tag.name}
              </Checkbox>
            </li>
          {/each}
        {/if}

        {#if $allTagGroups.length === 0 && ungroupedTags.length === 0}
          <div class="p-2 text-gray-500 italic text-xs text-center">No tags available</div>
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
