<!-- src/lib/components/projectview/modals/ManageModelsModal.svelte -->
<script>
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import ConfigurationView from '$lib/components/shared/ConfigurationView.svelte'; // Import the configure component
    import { Modal, Button } from 'flowbite-svelte';
    import { X, Cpu } from '@lucide/svelte';

	export let showModal = false;

	let isConfigureBusy = false; // Local state bound to Configure's busy state

	const dispatch = createEventDispatcher();

	function handleCloseAttempt() {
		if (isConfigureBusy) {
			console.log('ManageModelsModal: Close prevented - operation in progress.');
			alert(
				'An operation (downloading or moving models) is currently in progress. Please wait or cancel it before closing.'
			);
			return; // Prevent closing
		}
		// If not busy, signal the parent (TopBar) to close the modal
		showModal = false; // Update bound prop which closes the modal visually
		dispatch('modelsChanged');
		dispatch('close'); // DISPATCH EVENT
		console.log('ManageModelsModal: Closing and dispatched event.');
	}
</script>

<Modal
	bind:open={showModal}
	size="lg"
	autoclose={false}
	outsideclose={!isConfigureBusy}
	class="w-full"
	backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
	dialogClass="fixed top-0 start-0 end-0 h-modal md:inset-0 md:h-full z-[10001] flex"
	bodyClass="p-0 bg-white dark:bg-gray-900 overflow-hidden flex flex-col"
	headerClass="px-6 py-4 flex items-center justify-between border-b dark:border-gray-700 bg-gray-50/50"
	footerClass="px-6 py-4 flex items-center justify-end border-t dark:border-gray-700 bg-gray-50/80 backdrop-blur"
	on:close={handleCloseAttempt}
>
	<div slot="header" class="flex items-center gap-2">
		<Cpu class="w-5 h-5 text-gray-500" />
		<h3 class="text-lg font-semibold text-gray-900 dark:text-white">
			Manage Models
		</h3>
	</div>

	<!-- Modal Body (Contains Configure Component) -->
	<div class="flex-grow overflow-y-auto custom-scrollbar max-h-[70vh]">
		<!-- Bind the isBusy state from Configure -->
		<ConfigurationView bind:isBusy={isConfigureBusy} />
	</div>

	<svelte:fragment slot="footer">
		<Button
			color="alternative"
			on:click={handleCloseAttempt}
			disabled={isConfigureBusy}
			title={isConfigureBusy ? 'Operation in progress...' : 'Close Model Manager'}
			class="px-8"
		>
			{isConfigureBusy ? 'Working...' : 'Close'}
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