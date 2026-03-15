<!-- src/lib/components/projectview/modals/UnsavedChangesModal.svelte -->
<script>
	import { createEventDispatcher } from 'svelte';
    import { Modal, Button } from 'flowbite-svelte';
    import { AlertTriangle } from 'lucide-svelte';

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
</script>

<Modal
	bind:open={showModal}
	size="sm"
	autoclose={false}
	outsideclose={true}
	class="w-full"
	backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
	dialogClass="fixed top-0 start-0 end-0 h-modal md:inset-0 md:h-full z-[10001] flex"
	bodyClass="p-6 space-y-4 bg-white dark:bg-gray-900"
	headerClass="px-6 py-4 flex items-center justify-between border-b dark:border-gray-700 bg-gray-50/50"
	footerClass="px-6 py-4 flex items-center justify-end space-x-3 rtl:space-x-reverse border-t dark:border-gray-700 bg-gray-50/80 backdrop-blur"
	on:close={handleCancel}
>
	<div slot="header" class="flex items-center gap-2">
		<AlertTriangle class="w-5 h-5 text-orange-500" />
		<h3 class="text-lg font-semibold text-gray-900 dark:text-white">
			Unsaved Changes
		</h3>
	</div>

	<div class="space-y-3">
		<p class="text-sm text-gray-700 dark:text-gray-300">
			You have unsaved changes in the {itemType} "<span class="font-semibold text-gray-900 dark:text-white">{itemName}</span>".
		</p>
		<p class="text-xs text-gray-500 dark:text-gray-400 italic">
			Would you like to save before closing?
		</p>
	</div>

	<svelte:fragment slot="footer">
		<Button color="alternative" on:click={handleCancel} title="Cancel and stay">
			Cancel
		</Button>
		<Button color="red" outline on:click={handleDiscard} title="Discard all changes">
			Discard
		</Button>
		<Button color="blue" on:click={handleSave} title="Save changes and close">
			Save
		</Button>
	</svelte:fragment>
</Modal>