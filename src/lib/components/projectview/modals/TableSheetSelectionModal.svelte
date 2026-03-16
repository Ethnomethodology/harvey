<!-- src/lib/components/projectview/modals/TableSheetSelectionModal.svelte -->
<script>
    import { createEventDispatcher } from 'svelte';
    import { Modal, Button, Checkbox } from 'flowbite-svelte';
    import { Sheet } from 'lucide-svelte';

    export let showModal = false;
    export let sheets = [];
    export let filename = '';

    const dispatch = createEventDispatcher();

    let selectedSheets = [];

    // Track modal open state to run init exactly once per open
    let wasOpen = false;

    // Initialize selectedSheets when modal opens
    $: if (showModal && !wasOpen && sheets.length > 0) {
        wasOpen = true;
        // By default, select all sheets
        selectedSheets = [...sheets];
    } else if (!showModal && wasOpen) {
        wasOpen = false;
        selectedSheets = [];
    }

    function handleConfirm() {
        dispatch('confirm', { selectedSheets });
        showModal = false;
    }

    function handleCancel() {
        dispatch('cancel');
        showModal = false;
    }

    function toggleAll(event) {
        if (event.target.checked) {
            selectedSheets = [...sheets];
        } else {
            selectedSheets = [];
        }
    }

    function toggleSheet(sheet, isChecked) {
        if (isChecked) {
            if (!selectedSheets.includes(sheet)) {
                selectedSheets = [...selectedSheets, sheet];
            }
        } else {
            selectedSheets = selectedSheets.filter((s) => s !== sheet);
        }
    }
</script>

<Modal title={`Import Sheets from ${filename}`} bind:open={showModal} size="md" outsideclose={false} dismissable={false} on:close={() => {}}>
    <svelte:fragment slot="header">
        <div class="flex items-center space-x-2">
            <Sheet class="w-5 h-5 text-gray-500" />
            <h3 class="text-xl font-medium text-gray-900 dark:text-white">Select Sheets to Import</h3>
        </div>
    </svelte:fragment>

    <div class="space-y-4">
        <p class="text-sm text-gray-500 dark:text-gray-400">
            The file <strong>{filename}</strong> contains multiple sheets. Please select which sheets you would like to import. Each selected sheet will be imported as a separate table.
        </p>

        <div class="flex items-center mb-2">
            <Checkbox checked={selectedSheets.length === sheets.length && sheets.length > 0} indeterminate={selectedSheets.length > 0 && selectedSheets.length < sheets.length} on:change={toggleAll}>
                <span class="font-medium">Select All</span>
            </Checkbox>
        </div>

        <div class="border rounded-lg max-h-60 overflow-y-auto p-4 space-y-3 bg-gray-50 dark:bg-gray-800 dark:border-gray-700">
            {#each sheets as sheet}
                <div class="flex">
                    <div class="flex items-center h-5">
                        <Checkbox
                            id={`sheet-${sheet}`}
                            checked={selectedSheets.includes(sheet)}
                            on:change={(e) => toggleSheet(sheet, e.target.checked)}
                            value={sheet}
                            aria-describedby={`helper-text-${sheet}`}
                            class="w-4 h-4 text-neutral-primary border-default-medium bg-neutral-secondary-medium rounded focus:ring-2 focus:outline-none focus:ring-brand-subtle border border-default appearance-none"
                        />
                    </div>
                    <div class="ms-2 text-sm select-none">
                        <label for={`sheet-${sheet}`} class="font-medium text-heading mb-1">{sheet}</label>
                        <p id={`helper-text-${sheet}`} class="text-xs font-normal text-body">Will be imported as a separate table</p>
                    </div>
                </div>
            {/each}
        </div>

        {#if selectedSheets.length === 0}
            <p class="text-sm text-red-500">Please select at least one sheet to continue.</p>
        {/if}
    </div>

    <svelte:fragment slot="footer">
        <Button color="alternative" on:click={handleCancel}>Cancel</Button>
        <Button color="blue" on:click={handleConfirm} disabled={selectedSheets.length === 0}>Import {selectedSheets.length} {selectedSheets.length === 1 ? 'Sheet' : 'Sheets'}</Button>
    </svelte:fragment>
</Modal>
