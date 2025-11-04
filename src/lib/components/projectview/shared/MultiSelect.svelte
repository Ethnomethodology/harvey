<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let allItems: any[] = [];
	export let assignedItems: any[] = [];
	export let isEditable = true;
	export let displayField: string | null = null;
	export let itemType = 'item'; // e.g., 'tag', 'group'

	let availableItems: any[] = [];
	let showDropdown = false;
	let searchTerm = '';
	let rootElement: HTMLElement;

	const dispatch = createEventDispatcher();

	$: {
		updateAvailableItems();
	}

	function getItemValue(item: any) {
		return displayField ? item[displayField] : item;
	}

	function updateAvailableItems() {
		const assignedSet = new Set(assignedItems.map(getItemValue));
		availableItems = allItems.filter(item => !assignedSet.has(getItemValue(item)));
	}

	function removeItem(item: any) {
		if (!isEditable) return;
		assignedItems = assignedItems.filter(i => getItemValue(i) !== getItemValue(item));
		updateAvailableItems();
		dispatch('update', { items: assignedItems });
	}

	function addItem(item: any) {
		if (!isEditable) return;
		if (!assignedItems.find(i => getItemValue(i) === getItemValue(item))) {
			assignedItems = [...assignedItems, item];
		}
		updateAvailableItems();
		showDropdown = false;
		searchTerm = '';
		dispatch('update', { items: assignedItems });
	}

	function handleCreateNewItem() {
		if (!isEditable || !searchTerm.trim()) return;
		const newItem = searchTerm.trim();
		showDropdown = false;
		searchTerm = '';
		dispatch('createitem', { item: newItem });
	}

	$: filteredAvailableItems = searchTerm
		? availableItems.filter(item =>
				getItemValue(item).toLowerCase().includes(searchTerm.toLowerCase())
		  )
		: availableItems;

	function toggleDropdown() {
		showDropdown = !showDropdown;
		if (showDropdown) {
			searchTerm = '';
		}
	}

	function handleClickOutside(event: MouseEvent) {
		if (rootElement && !rootElement.contains(event.target as Node)) {
			showDropdown = false;
		}
	}
</script>

<div class="relative bg-white dark:bg-surface-2" bind:this={rootElement}>
	<div
        class="flex flex-wrap items-center gap-1 mb-2 p-1 border border-gray-300 dark:border-border rounded-md min-h-[30px] w-full"
        on:click={() => isEditable && toggleDropdown()}
        class:cursor-pointer={isEditable}
        class:cursor-not-allowed={!isEditable}
        class:opacity-75={!isEditable}
        role={isEditable ? "button" : "presentation"}
        tabindex={isEditable ? 0 : -1}
        on:keydown={(e) => { if (isEditable && (e.key === 'Enter' || e.key === ' ')) toggleDropdown()}}
    >
		{#if assignedItems.length === 0}
			<span class="text-xs text-gray-500 dark:text-text-secondary px-2 py-1">No {itemType}s assigned.</span>
		{:else}
            {#each assignedItems as item (getItemValue(item))}
                <span
                    class="flex items-center bg-blue-100 dark:bg-accent-primary text-blue-800 dark:text-white text-xs font-medium px-2 py-0.5 rounded-full"
                >
                    {getItemValue(item)}
                    {#if isEditable}
                    <button
                        on:click|stopPropagation={() => removeItem(item)}
                        class="ml-1.5 text-blue-600 dark:text-white/70 hover:text-blue-800 dark:hover:text-white"
                        aria-label="Remove {getItemValue(item)}"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="h-3 w-3">
                            <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd" />
                        </svg>
                    </button>
                    {/if}
                </span>
            {/each}
        {/if}
        {#if isEditable && assignedItems.length > 0}
            <span class="text-xs text-gray-500 dark:text-text-secondary px-2 py-1 flex-grow text-left">Add {itemType}...</span>
        {/if}
	</div>

	{#if showDropdown}
		<div
			class="absolute z-10 mt-1 w-full bg-white dark:bg-surface-2 border border-gray-300 dark:border-border rounded-md shadow-lg max-h-60 overflow-y-auto"
		>
			<div class="p-2">
				<input
					type="text"
					bind:value={searchTerm}
					placeholder="Search or add new..."
					class="w-full px-2 py-1.5 text-xs bg-white dark:bg-surface-3 text-gray-700 dark:text-text-primary border border-gray-300 dark:border-border rounded-md focus:ring-accent-primary focus:border-accent-primary"
					autocomplete="off"
					autocorrect="off"
				/>
			</div>
			<ul>
				{#each filteredAvailableItems as item (getItemValue(item))}
					<li
						on:click={() => addItem(item)}
						class="px-3 py-1.5 text-xs hover:bg-gray-100 dark:hover:bg-accent-background-hover cursor-pointer text-gray-700 dark:text-text-primary"
					>
						{getItemValue(item)}
					</li>
				{/each}

                {#if isEditable && searchTerm && !allItems.map(getItemValue).includes(searchTerm)}
                <li
                    on:click={handleCreateNewItem}
                    class="px-3 py-1.5 text-xs text-blue-600 dark:text-accent-primary hover:bg-gray-100 dark:hover:bg-accent-background-hover cursor-pointer border-t border-gray-200 dark:border-t-border"
                >
                    + Create new {itemType} "{searchTerm}"
                </li>
                {/if}
			</ul>
		</div>
	{/if}
</div>

<svelte:window on:click={handleClickOutside} />

<style>
	.relative {
		position: relative;
	}
	.absolute {
		position: absolute;
	}
</style>
