<!-- src/lib/components/projectview/modals/ConfirmConversionModal.svelte -->
<script>
	import { createEventDispatcher } from 'svelte';
	import { fade } from 'svelte/transition';

	export let showModal = false;
	export let fileName = 'the selected file';
    export let targetFormat = 'Lexical (.json)';

	const dispatch = createEventDispatcher();

	function handleConfirm() {
		dispatch('confirm');
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
        role="dialog"
        aria-modal="true"
        aria-labelledby="conversion-title"
	>
		<div
			class="modal-content bg-white dark:bg-gray-800 rounded-lg shadow-xl p-6 w-full max-w-md text-gray-800 dark:text-gray-200"
			on:click|stopPropagation
		>
			<h2 id="conversion-title" class="text-xl font-semibold mb-4">Confirm Conversion</h2>
			<p class="mb-6 text-sm">
                The file "<span class="font-medium">{fileName}</span>" needs to be converted to
                <span class="font-medium">{targetFormat}</span> before it can be imported.
                This may take a moment.
            </p>
            <p class="mb-6 text-sm text-gray-600 dark:text-gray-400">
                Proceed with conversion and import?
            </p>

			<div class="flex justify-end space-x-3">
				<button
					class="px-4 py-2 rounded-md text-sm font-medium bg-gray-200 dark:bg-gray-600 hover:bg-gray-300 dark:hover:bg-gray-500 focus:outline-none focus:ring-2 focus:ring-gray-400 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
					on:click={handleCancel}
				>
					Cancel
				</button>
				<button
					class="px-4 py-2 rounded-md text-sm font-medium bg-blue-500 text-white hover:bg-blue-600 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
					on:click={handleConfirm}
				>
					Convert & Import
				</button>
			</div>
		</div>
	</div>
{/if}