<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let itemType: string; // 'tag' or 'group'
	export let allOptions: string[] = [];
	export let assignedOptions: string[] = [];
	export let isEditable = true;
	export let placeholder: string = 'No items assigned.';

	let availableOptions: string[] = [];
	let showDropdown = false;
	let searchTerm = '';
	let rootElement: HTMLElement;

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
		showDropdown = false;
		searchTerm = '';
		dispatch('update', { options: assignedOptions });
	}

	function handleCreateNew() {
		if (!isEditable || !searchTerm.trim()) return;
		const newOption = searchTerm.trim();
		showDropdown = false;
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
		}
	}

	function handleClickOutside(event: MouseEvent) {
		if (rootElement && !rootElement.contains(event.target as Node)) {
			showDropdown = false;
		}
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
			class="absolute z-10 mt-1 w-full bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded-md shadow-lg max-h-60 overflow-y-auto"
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
			<ul>
				{#each filteredAvailableOptions as option (option)}
					<li
						on:click={() => addItem(option)}
						class="px-3 py-1.5 text-xs hover:bg-gray-100 dark:hover:bg-blue-500/10 cursor-pointer text-gray-700 dark:text-gray-200"
					>
						{option}
					</li>
				{/each}

                {#if isEditable && searchTerm && !allOptions.includes(searchTerm)}
                <li
                    on:click={handleCreateNew}
                    class="px-3 py-1.5 text-xs text-blue-600 dark:text-blue-500 hover:bg-gray-100 dark:hover:bg-blue-500/10 cursor-pointer border-t border-gray-200 dark:border-t-border"
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
