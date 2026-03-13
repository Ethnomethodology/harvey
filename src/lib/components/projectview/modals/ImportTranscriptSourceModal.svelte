<script>
    import { createEventDispatcher } from 'svelte';
    import { Modal, Label, Select, Button } from 'flowbite-svelte';

    export let showModal = false;

    const dispatch = createEventDispatcher();

    let selectedSourceType = 'msWord'; // Default, only option for now

    function handleConfirm() {
        if (selectedSourceType) {
            dispatch('confirm', { sourceType: selectedSourceType });
        }
    }

    function handleClose() {
        dispatch('close');
    }
</script>

<Modal bind:open={showModal} size="sm" autoclose={false} outsideclose={true} class="w-full z-[120]" on:close={handleClose}>
    <h2 id="import-transcript-title" class="text-lg font-semibold" slot="header">Import Transcript From...</h2>

    <div class="space-y-4">
        <p class="text-sm text-gray-600 dark:text-gray-400">
            Select the source format of the transcript you want to import.
            The transcript file should contain the name of the associated audio file and the transcript text.
        </p>
        <div>
            <Label for="sourceType" class="mb-1 text-sm font-medium text-gray-700 dark:text-gray-300">Source Type</Label>
            <Select id="sourceType" bind:value={selectedSourceType}>
                <option value="msWord">MS Word Document (.docx)</option>
                <!-- Other options can be added here later -->
            </Select>
            {#if selectedSourceType === 'msWord'}
                <p class="mt-2 text-xs text-yellow-600 dark:text-yellow-400">
                    Microsoft Word transcript with speakers and timestamps only supported.
                </p>
            {/if}
        </div>
    </div>

    <svelte:fragment slot="footer">
        <div class="flex justify-end space-x-3 w-full">
            <Button color="alternative" on:click={handleClose}>
                Cancel
            </Button>
            <Button color="blue" on:click={handleConfirm} disabled={!selectedSourceType}>
                Proceed
            </Button>
        </div>
    </svelte:fragment>
</Modal>