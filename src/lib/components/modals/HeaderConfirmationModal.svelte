<!-- src/lib/components/modals/HeaderConfirmationModal.svelte -->
<script>
    import { modalStore, hideHeaderConfirmationModal } from '$lib/stores/modalStore.js';

    let hasHeaders = true;

    function handleConfirm() {
        const { onConfirm, headerConfirmationData } = $modalStore;
        if (onConfirm) {
            onConfirm(headerConfirmationData.tablePath, hasHeaders);
        }
        hideHeaderConfirmationModal();
    }

    function handleCancel() {
        hideHeaderConfirmationModal();
    }
</script>

{#if $modalStore.isHeaderConfirmationDialogOpen}
<div class="fixed inset-0 bg-black bg-opacity-50 flex justify-center items-center">
    <div class="bg-white p-8 rounded-lg shadow-lg">
        <h2 class="text-lg font-bold mb-4">Confirm Header Row</h2>
        <p class="mb-4">Does the first row of the imported file contain headers?</p>
        
        <div class="mb-4">
            <label class="flex items-center mb-2">
                <input type="radio" bind:group={hasHeaders} name="header-option" value={true} class="mr-2">
                <span>Yes, the first row contains headers.</span>
            </label>
            <label class="flex items-center">
                <input type="radio" bind:group={hasHeaders} name="header-option" value={false} class="mr-2">
                <span>No, there are no headers in this file.</span>
            </label>
        </div>

        <div class="flex justify-end space-x-4 mt-6">
            <button on:click={handleCancel} class="btn">Cancel</button>
            <button on:click={handleConfirm} class="btn btn-primary">Confirm</button>
        </div>
    </div>
</div>
{/if}