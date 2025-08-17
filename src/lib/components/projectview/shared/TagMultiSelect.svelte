<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let allTags: string[] = [];
	export let assignedTags: string[] = [];
	export let isEditable = true;

	let availableTags: string[] = [];
	let showDropdown = false;
	let searchTerm = '';

	const dispatch = createEventDispatcher();

	$: {
		updateAvailableTags();
	}

	function updateAvailableTags() {
		const assignedSet = new Set(assignedTags);
		availableTags = allTags.filter(t => !assignedSet.has(t));
	}

	function removeTag(tag: string) {
		if (!isEditable) return;
		assignedTags = assignedTags.filter(t => t !== tag);
		updateAvailableTags();
		dispatch('update', { tags: assignedTags });
	}

	function addTag(tag: string) {
		if (!isEditable) return;
		if (!assignedTags.includes(tag)) {
			assignedTags = [...assignedTags, tag];
		}
		updateAvailableTags();
		showDropdown = false;
		searchTerm = '';
		dispatch('update', { tags: assignedTags });
	}

	function handleCreateNewTag() {
		if (!isEditable || !searchTerm.trim()) return;
		const newTag = searchTerm.trim();
		showDropdown = false;
		searchTerm = '';
		dispatch('createtag', { tag: newTag });
	}

	$: filteredAvailableTags = searchTerm
		? availableTags.filter(tag =>
				tag.toLowerCase().includes(searchTerm.toLowerCase())
		  )
		: availableTags;

	function toggleDropdown() {
		showDropdown = !showDropdown;
		if (showDropdown) {
			searchTerm = '';
		}
	}
</script>

<div class="relative">
	<div
        class="flex flex-wrap items-center gap-1 mb-2 p-1 border border-gray-300 dark:border-gray-600 rounded-md min-h-[30px] w-full"
        on:click={() => isEditable && toggleDropdown()}
        class:cursor-pointer={isEditable}
        class:cursor-not-allowed={!isEditable}
        class:opacity-75={!isEditable}
        role={isEditable ? "button" : "presentation"}
        tabindex={isEditable ? 0 : -1}
        on:keydown={(e) => { if (isEditable && (e.key === 'Enter' || e.key === ' ')) toggleDropdown()}}
    >
		{#if assignedTags.length === 0}
			<span class="text-xs text-gray-500 dark:text-gray-400 px-2 py-1">No tags assigned.</span>
		{:else}
            {#each assignedTags as tag (tag)}
                <span
                    class="flex items-center bg-blue-100 dark:bg-blue-700 text-blue-800 dark:text-blue-200 text-xs font-medium px-2 py-0.5 rounded-full"
                >
                    {tag}
                    {#if isEditable}
                    <button
                        on:click|stopPropagation={() => removeTag(tag)}
                        class="ml-1.5 text-blue-600 dark:text-blue-300 hover:text-blue-800 dark:hover:text-blue-100"
                        aria-label="Remove {tag}"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="h-3 w-3">
                            <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd" />
                        </svg>
                    </button>
                    {/if}
                </span>
            {/each}
        {/if}
        {#if isEditable && assignedTags.length > 0}
            <span class="text-xs text-gray-500 dark:text-gray-400 px-2 py-1 flex-grow text-left">Add tag...</span>
        {/if}
	</div>

	{#if showDropdown}
		<div
			class="absolute z-10 mt-1 w-full bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-md shadow-lg max-h-60 overflow-y-auto"
		>
			<div class="p-2">
				<input
					type="text"
					bind:value={searchTerm}
					placeholder="Search or add new..."
					class="w-full px-2 py-1.5 text-xs border border-gray-300 dark:border-gray-500 rounded-md dark:bg-gray-700 dark:text-white focus:ring-blue-500 focus:border-blue-500"
				/>
			</div>
			<ul>
				{#each filteredAvailableTags as tag (tag)}
					<li
						on:click={() => addTag(tag)}
						class="px-3 py-1.5 text-xs hover:bg-gray-100 dark:hover:bg-gray-700 cursor-pointer text-gray-700 dark:text-gray-200"
					>
						{tag}
					</li>
				{/each}

                {#if isEditable && searchTerm && !allTags.includes(searchTerm)}
                <li
                    on:click={handleCreateNewTag}
                    class="px-3 py-1.5 text-xs text-blue-600 dark:text-blue-400 hover:bg-gray-100 dark:hover:bg-gray-700 cursor-pointer border-t border-gray-200 dark:border-gray-600"
                >
                    + Create new tag "{searchTerm}"
                </li>
                {/if}
			</ul>
		</div>
	{/if}
</div>

<svelte:window on:click={(event) => {
    const target = event.target as HTMLElement;
    if (!target.closest('.relative')) {
        showDropdown = false;
    }
}}/>

<style>
	.relative {
		position: relative;
	}
	.absolute {
		position: absolute;
	}
</style>
