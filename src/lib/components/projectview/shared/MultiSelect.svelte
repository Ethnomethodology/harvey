<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let itemType: string; // 'tag' or 'group'
	export let allOptions: string[] = [];
    /** Optional grouped options for rendering categories in the dropdown. 
     * If provided, dropdown will show headers when search is empty. */
    export let groupedOptions: { name: string, options: string[] }[] | null = null;
	export let assignedOptions: string[] = [];
	export let isEditable = true;
	export let placeholder: string = 'No items assigned.';

	let availableOptions: string[] = [];
	let showDropdown = false;
	let searchTerm = '';
	let rootElement: HTMLElement;
	let dropdownX = 0;
	let dropdownY = 0;
	let dropdownWidth = 0;
	let placement = 'bottom';
	let dropdownElement: HTMLElement;

	const dispatch = createEventDispatcher();

	$: {
        const _trigger1 = allOptions;
        const _trigger2 = assignedOptions;
		updateAvailableOptions();
	}

	function updateAvailableOptions() {
        if (!allOptions || !assignedOptions) return;
		const assignedSet = new Set(assignedOptions);
		availableOptions = allOptions.filter(o => !assignedSet.has(o));
	}

	function removeItem(option: string) {
		if (!isEditable) return;
		assignedOptions = assignedOptions.filter(o => o !== option);
		updateAvailableOptions();
		dispatch('update', { options: assignedOptions });
	}

	function addItem(option: string) {
		if (!isEditable) return;
		if (!assignedOptions.includes(option)) {
			assignedOptions = [...assignedOptions, option];
		}
		updateAvailableOptions();
		searchTerm = '';
		dispatch('update', { options: assignedOptions });
	}

	function handleCreateNew() {
		if (!isEditable || !searchTerm.trim()) return;
		const newOption = searchTerm.trim();
		searchTerm = '';
		dispatch('create', { option: newOption });
	}

	$: filteredAvailableOptions = searchTerm
		? availableOptions.filter(option =>
				option.toLowerCase().includes(searchTerm.toLowerCase())
		  )
		: availableOptions;

	function toggleDropdown() {
		showDropdown = !showDropdown;
		if (showDropdown) {
			searchTerm = '';
			updateDropdownPosition();
		}
	}

	function updateDropdownPosition() {
		if (rootElement) {
			const rect = rootElement.getBoundingClientRect();

			// Calculate the actual or estimated height of the dropdown
			let currentDropdownHeight = 240; // Default max height
			if (dropdownElement) {
				currentDropdownHeight = dropdownElement.offsetHeight;
			} else {
				// Estimate if not yet mounted (search input ~42px, padding ~8px, items ~28px each)
				const estimatedItemCount = groupedOptions ?
					(searchTerm ? filteredAvailableOptions.length : availableOptions.length + groupedOptions.length) :
					filteredAvailableOptions.length;
				currentDropdownHeight = Math.min(240, 50 + (estimatedItemCount * 28));
			}

			const margin = 5;

            // Document-relative coordinates (allows for absolute positioning attached to body)
            const scrollX = window.scrollX || window.pageXOffset;
            const scrollY = window.scrollY || window.pageYOffset;
            
            // Default position: below
			dropdownX = rect.left + scrollX;
			dropdownY = rect.bottom + scrollY + margin;
			dropdownWidth = rect.width;
			placement = 'bottom';

            // Flip logic: if it overflows viewport bottom, check if there's more space above
            if (rect.bottom + currentDropdownHeight > window.innerHeight) {
                const spaceAbove = rect.top;
                const spaceBelow = window.innerHeight - rect.bottom;
                
                if (spaceAbove > spaceBelow && spaceAbove > 100) {
                    // Position above
                    placement = 'top';
                    dropdownY = rect.top + scrollY - margin;
                }
            }
            
            // Horizontal shift to stay within viewport (still relative to document width)
            if (rect.left + dropdownWidth > window.innerWidth) {
                dropdownX = Math.max(margin, window.innerWidth - dropdownWidth - margin) + scrollX;
            }
		}
	}

	function portal(node) {
		document.body.appendChild(node);
		return {
			destroy() {
				if (node.parentNode) {
					node.parentNode.removeChild(node);
				}
			}
		};
	}

	function handleClickOutside(event: MouseEvent) {
		if (rootElement && !rootElement.contains(event.target as Node)) {
			// Also check if clicked inside the portaled dropdown
			const dropdown = document.querySelector('.multi-select-dropdown');
			if (dropdown && dropdown.contains(event.target as Node)) {
				return;
			}
			showDropdown = false;
		}
	}

    import { onMount, onDestroy } from 'svelte';
    let resizeObserver: ResizeObserver | null = null;

    onMount(() => {
        if (typeof ResizeObserver !== 'undefined' && rootElement) {
            resizeObserver = new ResizeObserver(() => {
                if (showDropdown) updateDropdownPosition();
            });
            resizeObserver.observe(rootElement);
        }
    });

    onDestroy(() => {
        if (resizeObserver) resizeObserver.disconnect();
    });

    // Reactive trigger for position update when showDropdown becomes true
    // Also re-calculate if the number of options changes significantly (e.g. typing)
    $: if (showDropdown && rootElement && (filteredAvailableOptions || true)) {
        setTimeout(updateDropdownPosition, 0);
    }
</script>

<div class="relative bg-white dark:bg-gray-900" bind:this={rootElement}>
	<div
        class="flex flex-wrap items-center gap-1 mb-2 p-1 border border-gray-300 dark:border-gray-700 rounded-md min-h-[30px] w-full"
        on:click={() => isEditable && toggleDropdown()}
        class:cursor-pointer={isEditable}
        class:cursor-not-allowed={!isEditable}
        class:opacity-75={!isEditable}
        role={isEditable ? "button" : "presentation"}
        tabindex={isEditable ? 0 : -1}
        on:keydown={(e) => { if (isEditable && (e.key === 'Enter' || e.key === ' ')) toggleDropdown()}}
    >
		{#if assignedOptions.length === 0}
			<span class="text-xs text-gray-500 dark:text-gray-400 px-2 py-1">{placeholder}</span>
		{:else}
            {#each assignedOptions as option (option)}
                <span
                    class="flex items-center bg-blue-100 dark:bg-blue-600 text-blue-800 dark:text-white text-xs font-medium px-2 py-0.5 rounded-full"
                >
                    {option}
                    {#if isEditable}
                    <button
                        on:click|stopPropagation={() => removeItem(option)}
                        class="ml-1.5 text-blue-600 dark:text-white/70 hover:text-blue-800 dark:hover:text-white"
                        aria-label="Remove {option}"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="h-3 w-3">
                            <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd" />
                        </svg>
                    </button>
                    {/if}
                </span>
            {/each}
        {/if}
        {#if isEditable && assignedOptions.length > 0}
            <span class="text-xs text-gray-500 dark:text-gray-400 px-2 py-1 flex-grow text-left">Add {itemType}...</span>
        {/if}
	</div>

	{#if showDropdown}
		<div
			use:portal
			bind:this={dropdownElement}
			class="absolute z-[999999] bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded-md shadow-lg max-h-60 overflow-y-auto multi-select-dropdown"
			style="top: {dropdownY}px; left: {dropdownX}px; width: {dropdownWidth}px; {placement === 'top' ? 'transform: translateY(-100%);' : ''}"
		>
			<div class="p-2">
				<input
					type="text"
					bind:value={searchTerm}
					placeholder="Search or add new..."
					class="w-full px-2 py-1.5 text-xs bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-200 border border-gray-300 dark:border-gray-700 rounded-md focus:ring-blue-500 focus:border-blue-500"
					autocomplete="off"
					autocorrect="off"
				/>
			</div>
			<ul class="pb-1">
                {#if !searchTerm && groupedOptions && groupedOptions.length > 0}
                    <!-- Grouped View -->
                    {#each groupedOptions as group}
                        {@const groupOptions = group.options.filter(o => availableOptions.includes(o))}
                        {#if groupOptions.length > 0}
                            <div class="relative mt-4 mb-3 mx-2">
                                <span class="absolute -top-2 left-2 px-1 text-[9px] font-bold uppercase tracking-wider bg-white dark:bg-gray-900 text-blue-600 dark:text-blue-400 z-10">
                                    {group.name}
                                </span>
                                <div class="pt-3 pb-1 border border-blue-200 dark:border-blue-800/60 rounded-md">
                                    {#each groupOptions as option (option)}
                                        <li
                                            on:click|stopPropagation={() => addItem(option)}
                                            class="px-3 py-1.5 text-xs hover:bg-gray-100 dark:hover:bg-blue-500/10 cursor-pointer text-gray-700 dark:text-gray-200 transition-colors"
                                        >
                                            {option}
                                        </li>
                                    {/each}
                                </div>
                            </div>
                        {/if}
                    {/each}
                    
                    <!-- Handle ungrouped tags -->
                    {@const allGroupedOptions = new Set(groupedOptions.flatMap(g => g.options))}
                    {@const ungroupedOptions = availableOptions.filter(o => !allGroupedOptions.has(o))}
                    {#if ungroupedOptions.length > 0}
                        <div class="relative mt-4 mb-3 mx-2">
                            <span class="absolute -top-2 left-2 px-1 text-[9px] font-bold uppercase tracking-wider bg-white dark:bg-gray-900 text-gray-400 dark:text-gray-500 z-10">
                                Ungrouped
                            </span>
                            <div class="pt-3 pb-1 border border-gray-200 dark:border-gray-800/60 rounded-md">
                                {#each ungroupedOptions as option (option)}
                                    <li
                                        on:click|stopPropagation={() => addItem(option)}
                                        class="px-3 py-1.5 text-xs hover:bg-gray-100 dark:hover:bg-blue-500/10 cursor-pointer text-gray-700 dark:text-gray-200 transition-colors"
                                    >
                                        {option}
                                    </li>
                                {/each}
                            </div>
                        </div>
                    {/if}
                {:else}
                    <!-- Flat Filtered View (for search or if no grouping) -->
    				{#each filteredAvailableOptions as option (option)}
    					<li
    						on:click|stopPropagation={() => addItem(option)}
    						class="px-3 py-1.5 text-xs hover:bg-gray-100 dark:hover:bg-blue-500/10 cursor-pointer text-gray-700 dark:text-gray-200 transition-colors"
    					>
    						{option}
    					</li>
    				{/each}
                {/if}

                {#if isEditable && searchTerm && !allOptions.includes(searchTerm)}
                <li
                    on:click|stopPropagation={handleCreateNew}
                    class="px-3 py-1.5 text-xs text-blue-600 dark:text-blue-500 hover:bg-gray-100 dark:hover:bg-blue-500/10 cursor-pointer border-t border-gray-200 dark:border-t-border"
                >
                    + Create new {itemType} "{searchTerm}"
                </li>
                {/if}
			</ul>
		</div>
	{/if}
</div>

<svelte:window 
	on:click={handleClickOutside} 
	on:resize={updateDropdownPosition}
	on:scroll|capture={updateDropdownPosition}
/>

<style>
	.relative {
		position: relative;
	}
	.absolute {
		position: absolute;
	}
</style>
