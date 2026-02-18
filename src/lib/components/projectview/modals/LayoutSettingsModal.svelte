<!-- src/lib/components/projectview/modals/LayoutSettingsModal.svelte -->
<script>
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import { DOCX_LAYOUT_OPTIONS } from '$lib/constants/exportLayouts.js';
	import waveformLayoutStore from '$lib/stores/waveformLayoutStore.js'; // Import the new store
    import { transcriptStore, toggleDualMode } from '$lib/stores/transcriptStore.js';
	import Dropdown from '$lib/components/shared/Dropdown.svelte';

	export let showModal = false;
	export let currentLayoutKey = 'Layout2'; // Default to 'Segment Block' for DOCX
	export let hideWaveformOptions = false; // New prop to control waveform options visibility

	const dispatch = createEventDispatcher();

	let modalElement;
	let selectedDocxLayoutKey = currentLayoutKey;
	let selectedWaveformLayout; // Will be initialized from the store

	// Subscribe to the waveform layout store
	const unsubscribeWaveformStore = waveformLayoutStore.subscribe(value => {
		selectedWaveformLayout = value;
	});

	$: selectedDocxLayoutKey = currentLayoutKey; // Ensure internal state updates if prop changes

	function handleSelectDocxLayout(layoutKey) {
		selectedDocxLayoutKey = layoutKey;
		dispatch('selectLayout', layoutKey); // This is for DOCX export layout
		// We don't close modal here, user might want to change waveform too
	}

	function handleSelectWaveformLayout(event) {
		const newWaveformLayout = event.detail;
		waveformLayoutStore.setLayout(newWaveformLayout);
		// selectedWaveformLayout will update reactively due to store subscription
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
		if (unsubscribeWaveformStore) {
			unsubscribeWaveformStore();
		}
	});

	const waveformOptions = [
		{ value: 'horizontal', label: 'Horizontal' },
		{ value: 'vertical', label: 'Vertical' },
		{ value: 'none', label: 'None' }
	];
</script>

{#if showModal}
	<div
		bind:this={modalElement}
		class="fixed inset-0 z-[130] flex items-center justify-center bg-black/50 backdrop-blur-sm"
		on:click|self={closeModal}
		role="dialog"
		aria-modal="true"
		aria-labelledby="layout-settings-modal-title"
		tabindex="-1"
		on:keydown={handleKeydown}
	>
		<div
			class="bg-white dark:bg-gray-900 p-6 rounded-lg shadow-xl w-full max-w-lg m-4 flex flex-col text-gray-800 dark:text-gray-200"
			on:click|stopPropagation
			role="document"
		>
			<h2 id="layout-settings-modal-title" class="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-5">
				View Settings
			</h2>

			{#if !hideWaveformOptions}
			<!-- Waveform Display Section -->
			<div class="mb-6">
				<h3 class="text-md font-medium text-gray-700 dark:text-gray-300 mb-2">Waveform Display</h3>
				<p class="text-sm text-gray-600 dark:text-gray-400 mb-3">
					Choose how the audio waveform is displayed in the Transcription tab.
				</p>
				<Dropdown
					containerClasses="w-full"
					options={waveformOptions}
					bind:value={selectedWaveformLayout}
					on:change={handleSelectWaveformLayout}
				/>
			</div>
			{/if}

            <!-- Dual Transcript Mode Section -->
            <div class="mb-6">
                <h3 class="text-md font-medium text-gray-700 dark:text-gray-300 mb-2">Dual Transcript Mode</h3>
                <p class="text-sm text-gray-600 dark:text-gray-400 mb-3">
                    Display two transcripts in an interleaved view for simultaneous comparison and editing.
                </p>
				<Dropdown
					containerClasses="w-full"
					options={[{value: 'false', label: 'Disable'}, {value: 'true', label: 'Enable'}]}
					value={$transcriptStore.isDualModeActive ? 'true' : 'false'}
					on:change={(e) => toggleDualMode(e.detail === 'true')}
					disabled={$transcriptStore.transcriptDirty}
					title={$transcriptStore.transcriptDirty ? 'Save changes to enable' : ''}
				/>
            </div>

			<!-- DOCX Export Layout Section -->
			<div>
				<h3 class="text-md font-medium text-gray-700 dark:text-gray-300 mb-2">Transcript Export Layout</h3>
				<p class="text-sm text-gray-600 dark:text-gray-400 mb-3">
					This changes the layout for DOCX exports of the current transcript.
				</p>
				<div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
					{#each DOCX_LAYOUT_OPTIONS as layout (layout.id)}
						<button
							type="button"
							class="text-left p-3 border rounded-md transition-all duration-150 ease-in-out focus:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-blue-500 dark:focus-visible:ring-offset-gray-800"
							class:bg-blue-500={selectedDocxLayoutKey === layout.rustLayoutKey}
							class:text-white={selectedDocxLayoutKey === layout.rustLayoutKey}
							class:hover:bg-gray-100={selectedDocxLayoutKey !== layout.rustLayoutKey}
							class:dark:hover:bg-gray-700={selectedDocxLayoutKey !== layout.rustLayoutKey}
							class:border-blue-500={selectedDocxLayoutKey === layout.rustLayoutKey}
							class:dark:border-blue-400={selectedDocxLayoutKey === layout.rustLayoutKey}
							class:border-gray-300={selectedDocxLayoutKey !== layout.rustLayoutKey}
							class:dark:border-gray-600={selectedDocxLayoutKey !== layout.rustLayoutKey}
							class:shadow-md={selectedDocxLayoutKey === layout.rustLayoutKey}
							on:click={() => handleSelectDocxLayout(layout.rustLayoutKey)}
							title={layout.name}
							aria-pressed={selectedDocxLayoutKey === layout.rustLayoutKey}
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

			<!-- Footer Buttons -->
			<div class="flex justify-end space-x-3 pt-5 border-t border-gray-200 dark:border-gray-600 mt-6">
				<button type="button" on:click={closeModal} class="btn-secondary text-sm">
					Close
				</button>
				<!-- Apply button is removed as changes are applied reactively -->
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
    .ui-checkbox {
		@apply w-3.5 h-3.5 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600; /* Adjusted size */
	}
</style>
