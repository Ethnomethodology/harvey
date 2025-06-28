<!-- src/lib/components/projectview/modals/LayoutSettingsModal.svelte -->
<script>
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import { DOCX_LAYOUT_OPTIONS } from '$lib/constants/exportLayouts.js';

	export let showModal = false;
	export let currentLayoutKey = 'Layout2'; // Default to 'Segment Block'

	const dispatch = createEventDispatcher();

	let modalElement;
	let selectedLayoutKey = currentLayoutKey;

	$: selectedLayoutKey = currentLayoutKey; // Ensure internal state updates if prop changes

	function handleSelectLayout(layoutKey) {
		selectedLayoutKey = layoutKey;
		dispatch('selectLayout', layoutKey);
		// Optionally close modal on selection, or require a confirm button
		// For now, let's assume selection implies confirmation for simplicity
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
		on:click|self={closeModal}
		role="dialog"
		aria-modal="true"
		aria-labelledby="layout-settings-modal-title"
	>
		<div
			class="bg-white dark:bg-gray-800 p-6 rounded-lg shadow-xl w-full max-w-md m-4 flex flex-col text-gray-800 dark:text-gray-200"
			on:click|stopPropagation
		>
			<h2 id="layout-settings-modal-title" class="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-5">
				Select View Layout
			</h2>

			<div class="space-y-3">
				<p class="text-sm text-gray-600 dark:text-gray-400 mb-3">
					This will change the layout of the current media only.
				</p>
				<div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
					{#each DOCX_LAYOUT_OPTIONS as layout (layout.id)}
						<button
							type="button"
							class="text-left p-3 border rounded-md transition-all duration-150 ease-in-out focus:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-blue-500 dark:focus-visible:ring-offset-gray-800"
							class:bg-blue-500={selectedLayoutKey === layout.rustLayoutKey}
							class:text-white={selectedLayoutKey === layout.rustLayoutKey}
							class:hover:bg-gray-100={selectedLayoutKey !== layout.rustLayoutKey}
							class:dark:hover:bg-gray-700={selectedLayoutKey !== layout.rustLayoutKey}
							class:border-blue-500={selectedLayoutKey === layout.rustLayoutKey}
							class:dark:border-blue-400={selectedLayoutKey === layout.rustLayoutKey}
							class:border-gray-300={selectedLayoutKey !== layout.rustLayoutKey}
							class:dark:border-gray-600={selectedLayoutKey !== layout.rustLayoutKey}
							class:shadow-md={selectedLayoutKey === layout.rustLayoutKey}
							on:click={() => handleSelectLayout(layout.rustLayoutKey)}
							title={layout.name}
							aria-pressed={selectedLayoutKey === layout.rustLayoutKey}
						>
							<div class="font-medium mb-1.5 text-sm">{layout.name}</div>
							<div class="{layout.previewClasses} min-h-[24px] opacity-80">
								{#each layout.columnStyles as style}
									<div class="{style.class} !p-1 !text-xs">{style.content}</div>
								{/each}
							</div>
						</button>
					{/each}
				</div>
			</div>

			<!-- Footer Buttons (Optional, if explicit confirm is needed) -->
			<div class="flex justify-end space-x-3 pt-5 border-t border-gray-200 dark:border-gray-600 mt-6">
				<button type="button" on:click={closeModal} class="btn-secondary text-sm">
					Close
				</button>
				<!-- <button
					type="button"
					on:click={() => { dispatch('selectLayout', selectedLayoutKey); closeModal(); }}
					class="btn-primary text-sm"
					disabled={selectedLayoutKey === currentLayoutKey}
				>
					Apply Layout
				</button> -->
			</div>
		</div>
	</div>
{/if}

<style lang="postcss">
	/* Basic button styles - can be inherited or defined if this modal is used standalone */
	.btn-primary, .btn-secondary {
		@apply px-4 py-1.5 rounded-md shadow-sm font-medium transition duration-150 ease-in-out;
	}
	.btn-primary {
		@apply bg-blue-600 text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:focus:ring-offset-gray-800 disabled:opacity-50 disabled:cursor-not-allowed;
	}
	.btn-secondary {
		@apply bg-gray-200 text-gray-700 hover:bg-gray-300 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-gray-500 dark:bg-gray-600 dark:text-gray-200 dark:hover:bg-gray-500 dark:focus:ring-offset-gray-800;
	}

	/* Ensure preview styles are scoped or specific enough */
	/* The `!p-1` and `!text-xs` in the template help override generic styles from layout.previewClasses if needed */
</style>
