<script>
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { fly } from 'svelte/transition';

  export let options = [];
  export let value = '';
  export let placeholder = 'Select an option';
  export let containerClasses = '';
  export let disabled = false;

  let isOpen = false;
  let dropdownElement;
  const dispatch = createEventDispatcher();

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
    document.addEventListener('click', handleClickOutside, true);
  });

  onDestroy(() => {
    document.removeEventListener('click', handleClickOutside, true);
  });

  $: selectedLabel = options.find(opt => opt.value === value)?.label || placeholder;
</script>

<div class="relative inline-block text-left {containerClasses}" on:keydown={handleKeydown} bind:this={dropdownElement}>
  <div>
    <button
      type="button"
      class="ui-select w-full flex justify-between items-center"
      on:click={() => { if (!disabled) isOpen = !isOpen; }}
      aria-haspopup="true"
      aria-expanded={isOpen}
      {disabled}
    >
      <span class="truncate">{selectedLabel}</span>
    </button>
  </div>

  {#if isOpen}
    <div
      transition:fly={{ y: -5, duration: 100 }}
      class="origin-top-right absolute right-0 mt-2 w-full rounded-md shadow-lg z-10"
      style="background-color: var(--ui-option-bg); border: 1px solid var(--ui-select-border);"
      role="menu"
      aria-orientation="vertical"
    >
      <div class="py-1" role="none">
        {#each options as option}
          <button
            class="dropdown-option block w-full text-left px-3 py-1.5 text-sm"
            style="color: var(--ui-option-text);"
            role="menuitem"
            on:click={() => selectOption(option.value)}
            title={option.label}
          >
            <span class="truncate">{option.label}</span>
          </button>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .dropdown-option:hover {
    background-color: var(--ui-option-hover-bg);
  }
</style>
