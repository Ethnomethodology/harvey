<script>
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { fly } from 'svelte/transition';

  export let options = [];
  export let value = '';
  export let placeholder = 'Select an option';
  export let containerClasses = '';

  let isOpen = false;
  let dropdownElement;
  const dispatch = createEventDispatcher();

  function selectOption(optionValue) {
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
      class="btn btn-sm btn-outline w-full justify-between font-normal text-left"
      on:click={() => (isOpen = !isOpen)}
      aria-haspopup="true"
      aria-expanded={isOpen}
    >
      <span class="truncate">{selectedLabel}</span>
      <svg
        class="h-5 w-5 flex-shrink-0"
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 20 20"
        fill="currentColor"
        aria-hidden="true"
      >
        <path
          fill-rule="evenodd"
          d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
          clip-rule="evenodd"
        />
      </svg>
    </button>
  </div>

  {#if isOpen}
    <div
      transition:fly={{ y: -5, duration: 100 }}
      class="origin-top-right absolute right-0 mt-2 w-full rounded-md shadow-lg bg-base-100 ring-1 ring-black ring-opacity-5 focus:outline-none z-10"
      role="menu"
      aria-orientation="vertical"
      aria-labelledby="options-menu"
    >
      <div class="py-1" role="none">
        {#each options as option}
          <button
            class="btn btn-sm btn-ghost w-full justify-start font-normal text-left"
            role="menuitem"
            on:click={() => selectOption(option.value)}
            class:bg-primary={value === option.value}
            class:text-primary-content={value === option.value}
          >
            {option.label}
          </button>
        {/each}
      </div>
    </div>
  {/if}
</div>
