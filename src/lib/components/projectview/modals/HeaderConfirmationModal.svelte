<!-- src/lib/components/projectview/modals/HeaderConfirmationModal.svelte -->
<script>
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import { fade } from 'svelte/transition';

	export let showModal = false;
	export let previewData = { fields: [], data: [] };
	export let tablePath = '';

	const dispatch = createEventDispatcher();

	let hasHeaders = true; // Default to "Yes"
	let modalElement;

	function handleConfirm() {
		if (!tablePath) {
			// Optionally, show a warning to the user that tablePath is missing
			console.error("[HeaderConfirmationModal] Cannot confirm: tablePath is missing.");
			// message("Cannot confirm: Table path is missing. Please report this bug.", { title: "Error", type: "error" });
			closeModal(); // Close the modal anyway to prevent further issues
			return;
		}
		dispatch('confirm', { hasHeaders });
		closeModal();
	}

	function closeModal() {
		showModal = false;
		dispatch('close');
	}

	function handleKeydown(event) {
		if (showModal && event.key === 'Escape') {
			closeModal();
		}
		if (event.key === 'Enter') {
			event.preventDefault();
			handleConfirm();
		}
	}

	onMount(() => {
		window.addEventListener('keydown', handleKeydown);
	});

	onDestroy(() => {
		window.removeEventListener('keydown', handleKeydown);
	});
</script>

{#if showModal}
	<div
		bind:this={modalElement}
		class="fixed inset-0 z-[130] flex items-center justify-center bg-gray-900 bg-opacity-60 backdrop-blur-sm"
		transition:fade={{ duration: 150 }}
		on:click|self={closeModal}
		role="dialog"
		aria-modal="true"
		aria-labelledby="header-modal-title"
	>
		<div
			class="bg-white dark:bg-surface-2 p-6 rounded-lg shadow-xl w-full max-w-2xl m-4 flex flex-col text-gray-800 dark:text-gray-200"
			on:click|stopPropagation
		>
			<h2 id="header-modal-title" class="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-4">
				Confirm Headers for <span class="font-mono bg-gray-100 dark:bg-gray-700 px-2 py-1 rounded">{tablePath.split(/[\/]/).pop()}</span>
			</h2>

			<p class="text-sm text-gray-600 dark:text-gray-400 mb-4">
				Does the first row of your file contain headers? We've assumed the following based on the file content.
			</p>

			<!-- Data Preview Table -->
			<div class="border border-gray-200 dark:border-gray-600 rounded-lg overflow-x-auto max-h-60 mb-4">
				<table class="w-full text-sm text-left">
					<thead class="bg-gray-50 dark:bg-gray-700 sticky top-0">
						<tr>
							{#if previewData && previewData.fields}
								{#each previewData.fields as header, i}
									<th scope="col" class="px-4 py-2 font-medium text-gray-600 dark:text-gray-300 truncate">
										{hasHeaders ? header : `Column ${i + 1}`}
									</th>
								{/each}
							{/if}
						</tr>
					</thead>
					<tbody>
						{#if previewData && previewData.data}
							{#each previewData.data.slice(0, 5) as row, rowIndex}
								<tr class="border-t border-gray-200 dark:border-gray-600">
									{#each previewData.fields as header, colIndex}
										<td class="px-4 py-2 whitespace-nowrap truncate max-w-xs">
											{row[header]}
										</td>
									{/each}
								</tr>
							{/each}
						{/if}
					</tbody>
				</table>
			</div>

			<!-- Header Options -->
			<div class="space-y-3">
				<label class="flex items-center p-3 border rounded-lg cursor-pointer transition-colors"
					   class:border-blue-500={hasHeaders}
					   class:dark:border-blue-400={hasHeaders}
					   class:border-gray-300={!hasHeaders}
					   class:dark:border-gray-600={!hasHeaders}
					   class:bg-blue-50={hasHeaders}
					   class:dark:bg-blue-900={hasHeaders} class:dark:bg-opacity-20={hasHeaders}>
					<input type="radio" bind:group={hasHeaders} name="header-option" value={true} class="h-4 w-4 text-blue-600 border-gray-300 focus:ring-blue-500">
					<div class="ml-3 text-sm">
						<p class="font-medium text-gray-900 dark:text-gray-200">Yes, the first row is the header.</p>
						<p class="text-gray-500 dark:text-gray-400">The first row will be used as column titles.</p>
					</div>
				</label>
				<label class="flex items-center p-3 border rounded-lg cursor-pointer transition-colors"
					   class:border-blue-500={!hasHeaders}
					   class:dark:border-blue-400={!hasHeaders}
					   class:border-gray-300={hasHeaders}
					   class:dark:border-gray-600={hasHeaders}
					   class:bg-blue-50={!hasHeaders}
					   class:dark:bg-blue-900={!hasHeaders} class:dark:bg-opacity-20={!hasHeaders}>
					<input type="radio" bind:group={hasHeaders} name="header-option" value={false} class="h-4 w-4 text-blue-600 border-gray-300 focus:ring-blue-500">
					<div class="ml-3 text-sm">
						<p class="font-medium text-gray-900 dark:text-gray-200">No, this file does not have a header row.</p>
						<p class="text-gray-500 dark:text-gray-400">Generic headers (A, B, C...) will be generated, and the first row will be treated as data.</p>
					</div>
				</label>
			</div>


			<!-- Footer Buttons -->
			<div class="flex justify-end space-x-3 pt-4 border-t border-gray-200 dark:border-gray-600 mt-6">
				<button type="button" on:click={closeModal} class="btn-secondary">
					Cancel
				</button>
				<button
					type="button"
					on:click={handleConfirm}
					class="btn-primary"
				>
					Confirm and Import
				</button>
			</div>
		</div>
	</div>
{/if}

<style lang="postcss">
	.btn-primary, .btn-secondary {
		@apply px-4 py-2 rounded-md shadow-sm text-sm font-medium transition duration-150 ease-in-out;
	}
	.btn-primary {
		@apply bg-blue-600 text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:focus:ring-offset-gray-800 disabled:opacity-50 disabled:cursor-not-allowed;
	}
	 .btn-secondary {
		@apply bg-gray-200 text-gray-700 hover:bg-gray-300 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-gray-500 dark:bg-gray-600 dark:text-gray-200 dark:hover:bg-gray-500 dark:focus:ring-offset-gray-800;
	}
</style>