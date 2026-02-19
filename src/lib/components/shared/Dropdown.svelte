<script>
  import { createEventDispatcher, onMount, onDestroy, tick } from 'svelte';
  import { fly } from 'svelte/transition';

  export let options = [];
  export let value = '';
  export let placeholder = 'Select an option';
  export let containerClasses = '';
  export let disabled = false;
  export let showColorPreview = false; // New prop to show color swatch
  export let boundaryRect = null; // Optional prop to specify boundaries for upward opening

  let isOpen = false;
  let dropdownElement;
  let openUpward = false;
  const dispatch = createEventDispatcher();

  async function toggleDropdown() {
    if (disabled) return;
    
    if (!isOpen) {
      checkPosition();
    }
    
    isOpen = !isOpen;
  }

  function checkPosition() {
    if (!dropdownElement) return;
    const rect = dropdownElement.getBoundingClientRect();
    const menuHeight = 192; // max-h-48 is 12rem = 192px
    
    let spaceBelow;
    if (boundaryRect) {
      spaceBelow = boundaryRect.bottom - rect.bottom;
    } else {
      spaceBelow = window.innerHeight - rect.bottom;
    }
    
    openUpward = spaceBelow < menuHeight && rect.top > menuHeight;
  }

  function selectOption(optionValue) {
    if (disabled) return;
    value = optionValue;
    dispatch('change', value);
    isOpen = false;
  }

  function handleKeydown(event) {
    if (event.key === 'Escape') {
      isOpen = false;
    }
  }

  function handleClickOutside(event) {
    if (dropdownElement && !dropdownElement.contains(event.target)) {
      isOpen = false;
    }
  }

  onMount(() => {
    window.addEventListener('mousedown', handleClickOutside, true);
    window.addEventListener('resize', checkPosition);
    window.addEventListener('scroll', checkPosition, true);
  });

  onDestroy(() => {
    window.removeEventListener('mousedown', handleClickOutside, true);
    window.removeEventListener('resize', checkPosition);
    window.removeEventListener('scroll', checkPosition, true);
  });

  $: selectedOption = options.find(opt => opt.value === value);
  $: selectedLabel = selectedOption?.label || placeholder;

  function isColor(val) {
    if (typeof val !== 'string') return false;
    const lowerVal = val.toLowerCase().trim();
    return lowerVal.startsWith('rgba(') || 
           lowerVal.startsWith('rgb(') || 
           lowerVal.startsWith('#') || 
           lowerVal === 'transparent' || 
           lowerVal.startsWith('url(') ||
           ['black', 'white', 'red', 'blue', 'gray', 'grey', 'green', 'yellow', 'pink', 'purple', 'orange'].includes(lowerVal);
  }

  function getColorStyle(val) {
    if (!val) return '';
    const lowerVal = val.toLowerCase().trim();
    if (lowerVal === 'transparent') {
      return 'background: linear-gradient(45deg, #fff 45%, #f00 45%, #f00 55%, #fff 55%); border: 1px solid #ccc;';
    }
    if (lowerVal.startsWith('url(#censoredpattern)')) {
      return 'background: linear-gradient(to bottom right, #fff 25%, #888 25%, #888 50%, #444 50%, #444 75%, #000 75%);';
    }
    // For rgba colors, show at full opacity if they are highlight colors (0.5 opacity)
    const processedColor = val.replace(/,\s*0\.5\s*\)/, ', 1)');
    return `background-color: ${processedColor};`;
  }
</script>

<div class="relative inline-block text-left {containerClasses}" on:keydown={handleKeydown} bind:this={dropdownElement}>
  <div>
    <button
      type="button"
      class="ui-select w-full flex justify-between items-center"
      on:click={toggleDropdown}
      aria-haspopup="true"
      aria-expanded={isOpen}
      {disabled}
    >
      <div class="flex items-center truncate">
        {#if showColorPreview && selectedOption && isColor(selectedOption.value)}
          <span class="color-swatch mr-2" style={getColorStyle(selectedOption.value)}></span>
        {/if}
        <span class="truncate">{selectedLabel}</span>
      </div>
    </button>
  </div>

  {#if isOpen}
    <div
      transition:fly={{ y: openUpward ? 5 : -5, duration: 100 }}
      class="ui-dropdown-menu origin-top-right absolute right-0 w-full rounded-md shadow-lg z-[1002]"
      class:bottom-full={openUpward}
      class:mb-2={openUpward}
      class:top-full={!openUpward}
      class:mt-2={!openUpward}
      role="menu"
      aria-orientation="vertical"
    >
      <div class="py-1" role="none">
        {#each options as option}
          <button
            class="dropdown-option flex items-center"
            role="menuitem"
            on:click={() => selectOption(option.value)}
            title={option.label}
          >
            {#if showColorPreview && isColor(option.value)}
              <span class="color-swatch mr-2" style={getColorStyle(option.value)}></span>
            {/if}
            <span class="truncate">{option.label}</span>
          </button>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .dropdown-option {
    @apply block w-full text-left px-3 py-1.5 text-xs font-normal truncate;
  }
  .ui-dropdown-menu {
    @apply max-h-48 overflow-y-auto bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700;
  }
  .color-swatch {
    @apply w-3 h-3 rounded-full flex-shrink-0 border border-gray-300 dark:border-gray-600;
    display: inline-block;
  }
</style>
