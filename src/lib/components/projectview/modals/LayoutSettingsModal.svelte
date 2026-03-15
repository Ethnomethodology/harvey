<!-- src/lib/components/projectview/modals/LayoutSettingsModal.svelte -->
<script>
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import { DOCX_LAYOUT_OPTIONS } from '$lib/constants/exportLayouts.js';
	import waveformLayoutStore from '$lib/stores/waveformLayoutStore.js';
    import { 
		Modal,
        Button, 
        Label, 
        Select, 
        Helper
    } from 'flowbite-svelte';
    import { LayoutDashboard, X, Waves } from 'lucide-svelte';

	export let showModal = false;
	export let currentLayoutKey = 'Layout2'; // Default to 'Segment Block' for DOCX
	export let hideWaveformOptions = true; // Default to true now that it's in the top bar

	const dispatch = createEventDispatcher();

	let selectedDocxLayoutKey = currentLayoutKey;
	let selectedWaveformLayout; 

	// Subscribe to the waveform layout store
	const unsubscribeWaveformStore = waveformLayoutStore.subscribe(value => {
		selectedWaveformLayout = value;
	});

	$: selectedDocxLayoutKey = currentLayoutKey;

	function handleSelectDocxLayout(layoutKey) {
		selectedDocxLayoutKey = layoutKey;
		dispatch('selectLayout', layoutKey);
	}

	function handleSelectWaveformLayout(e) {
		const newWaveformLayout = e.target.value;
		waveformLayoutStore.setLayout(newWaveformLayout);
	}

	function closeModal() {
		showModal = false;
		dispatch('close');
	}

	onDestroy(() => {
		if (unsubscribeWaveformStore) {
			unsubscribeWaveformStore();
		}
	});

	const waveformOptions = [
		{ value: 'horizontal', name: 'Horizontal' },
		{ value: 'vertical', name: 'Vertical' },
		{ value: 'none', name: 'None' }
	];
</script>

<Modal
	bind:open={showModal}
	size="md"
	autoclose={false}
	outsideclose={true}
	class="w-full"
	backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
	dialogClass="fixed top-0 start-0 end-0 h-modal md:inset-0 md:h-full z-[10001] flex"
	bodyClass="p-0 bg-white dark:bg-gray-900"
	headerClass="px-6 py-4 flex items-center justify-between border-b dark:border-gray-700 bg-gray-50/50"
	footerClass="px-6 py-4 flex items-center justify-end border-t dark:border-gray-700 bg-gray-50/80 backdrop-blur"
	on:close={closeModal}
>
	<div slot="header" class="flex items-center gap-2">
		<LayoutDashboard class="w-5 h-5 text-gray-500" />
		<h3 class="text-lg font-semibold text-gray-900 dark:text-white">
			View Settings
		</h3>
	</div>

	<div class="p-6 space-y-6 overflow-y-auto max-h-[70vh] custom-scrollbar">
		{#if !hideWaveformOptions}
		<!-- Waveform Display Section -->
		<div class="space-y-3 bg-gray-50 dark:bg-gray-800/40 p-4 rounded-xl border border-gray-100 dark:border-gray-800">
			<div class="flex items-center gap-2 text-gray-900 dark:text-white font-semibold text-sm">
				<Waves size={18} class="text-blue-500" />
				<span>Waveform Display</span>
			</div>
			<p class="text-xs text-gray-500 dark:text-gray-400">
				Choose how the audio waveform is displayed in the Transcription tab.
			</p>
			<Select
				items={waveformOptions}
				bind:value={selectedWaveformLayout}
				on:change={handleSelectWaveformLayout}
			/>
		</div>
		{/if}

		<!-- DOCX Export Layout Section -->
		<div class="space-y-4">
			<div class="space-y-1">
				<Label class="text-gray-900 dark:text-white font-semibold text-sm">Transcript Layout</Label>
				<p class="text-xs text-gray-500 dark:text-gray-400">
					This changes the layout of the transcript on screen.
				</p>
			</div>
			<div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
				{#each DOCX_LAYOUT_OPTIONS as layout (layout.id)}
					<button
						type="button"
						class="text-left p-4 border rounded-xl transition-all duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-blue-500 group relative {selectedDocxLayoutKey === layout.rustLayoutKey ? 'bg-blue-50 dark:bg-blue-900/20 border-blue-500 dark:border-blue-400' : 'border-gray-200 dark:border-gray-700 hover:border-blue-300'}"
						on:click={() => handleSelectDocxLayout(layout.rustLayoutKey)}
						title="Select {layout.name} layout"
					>
						<div class="font-bold mb-2 text-sm {selectedDocxLayoutKey === layout.rustLayoutKey ? 'text-blue-700 dark:text-blue-300' : ''}">
							{layout.name}
						</div>
						<div class="{layout.previewClasses} min-h-[24px] opacity-80 rounded shadow-sm overflow-hidden border border-gray-100 dark:border-gray-800 bg-white dark:bg-gray-800">
							{#each layout.columnStyles as style}
								<div class="{style.class} !p-1 !text-[10px] leading-tight flex items-center justify-center">{style.content}</div>
							{/each}
						</div>
						{#if selectedDocxLayoutKey === layout.rustLayoutKey}
							<div class="absolute top-2 right-2 w-2 h-2 bg-blue-500 rounded-full"></div>
						{/if}
					</button>
				{/each}
			</div>
		</div>
	</div>

	<svelte:fragment slot="footer">
		<Button color="blue" on:click={closeModal} title="Close settings" class="px-8">
			Close
		</Button>
	</svelte:fragment>
</Modal>

<style lang="postcss">
    .custom-scrollbar::-webkit-scrollbar {
        width: 6px;
    }
    .custom-scrollbar::-webkit-scrollbar-track {
        @apply bg-transparent;
    }
    .custom-scrollbar::-webkit-scrollbar-thumb {
        @apply bg-gray-200 dark:bg-gray-700 rounded-full;
    }
    .custom-scrollbar::-webkit-scrollbar-thumb:hover {
        @apply bg-gray-300 dark:bg-gray-600;
    }
</style>