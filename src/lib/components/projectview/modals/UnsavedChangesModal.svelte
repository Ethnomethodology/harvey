<!-- src/lib/components/projectview/modals/UnsavedChangesModal.svelte -->
<script>
	import { createEventDispatcher } from 'svelte';
	import { fade } from 'svelte/transition';

	export let showModal = false;
	export let itemName = 'the current item';
    export let itemType = 'item'; // Added prop (e.g., 'document', 'imported transcript')

	const dispatch = createEventDispatcher();

	function handleSave() {
		dispatch('save');
	}

	function handleDiscard() {
		dispatch('discard');
	}

	function handleCancel() {
		dispatch('cancel');
	}

	// Handle Escape key to cancel
	function keydown(event) {
		if (event.key === 'Escape') {
			handleCancel();
		}
	}
</script>

<svelte:window on:keydown={keydown}/>

{#if showModal}
	<div
		class="fixed inset-0 z-[120] flex items-center justify-center bg-black bg-opacity-60 backdrop-blur-sm"
		transition:fade={{ duration: 150 }}
		on:click={handleCancel}
        on:keydown={(e) => { if (e.key === 'Escape') handleCancel(); }}
        role="dialog"
        aria-modal="true"
        aria-labelledby="unsaved-title"
        tabindex="0"
	>
		<div
			class="modal-content bg-white dark:bg-gray-900 rounded-lg shadow-xl p-6 w-full max-w-md text-gray-800 dark:text-gray-200"
			on:click|stopPropagation
		>
			<h2 id="unsaved-title" class="text-xl font-semibold mb-4">Unsaved Changes</h2>
			<p class="mb-6">
                You have unsaved changes in the {itemType} "<span class="font-medium">{itemName}</span>".
            </p>

			<div class="flex justify-end space-x-3">
				<button
					class="px-4 py-2 rounded-md text-sm font-medium bg-gray-200 dark:bg-gray-600 hover:bg-gray-300 dark:hover:bg-gray-500 focus:outline-none focus:ring-2 focus:ring-gray-400 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
					on:click={handleCancel}
				>
					Cancel
				</button>
                <button
					class="px-4 py-2 rounded-md text-sm font-medium bg-red-100 text-red-700 dark:bg-red-900 dark:text-red-200 hover:bg-red-200 dark:hover:bg-red-800 focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
					on:click={handleDiscard}
				>
					Discard Changes
				</button>
				<button
					class="px-4 py-2 rounded-md text-sm font-medium bg-blue-500 text-white hover:bg-blue-600 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
					on:click={handleSave}
				>
					Save
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	/* Optional: Add specific modal styles if needed */
</style>