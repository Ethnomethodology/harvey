<!-- src/lib/components/projectview/data/tables/TableView.svelte -->
<script>
    import { onMount, createEventDispatcher } from 'svelte';
    // LeftInfoPanel and RightInfoPanel are removed
    // panelStateStore might not be needed if panel collapsing handled by parent
    import TableViewerPanel from './TableViewerPanel.svelte';

    export let itemPath = null; // Receives the full path from DataView
    export let hasHeaders = true;

    const dispatch = createEventDispatcher();

    function forwardEvent(event) {
        console.debug(`[TableView] Forwarding event: ${event.type}`);
		dispatch(event.type, event.detail);
	}

    let tableViewerPanelRef;

    export async function getExportData() {
        console.log("[TableView] getExportData called. tableViewerPanelRef:", !!tableViewerPanelRef);
        if (tableViewerPanelRef) {
            return await tableViewerPanelRef.getExportData();
        }
        return null;
    }

    export function openChart(chart) {
        if (tableViewerPanelRef && typeof tableViewerPanelRef.openChart === 'function') {
            tableViewerPanelRef.openChart(chart);
        }
    }

    onMount(() => {
		console.debug('[TableView] Component container mounted. Table path:', itemPath);
	});

    $: {
        // console.debug(`[TableView] Path is now ${itemPath}`);
    }

</script>

<!-- Main container for the Table View - this will now be the main content panel -->
<div class="h-full flex-grow min-w-0 bg-white dark:bg-gray-800">
    {#key itemPath}
        {#if itemPath}
            <TableViewerPanel 
                bind:this={tableViewerPanelRef} 
                tablePath={itemPath} 
                hasHeaders={hasHeaders} 
                on:requestviewchange={forwardEvent}
            />
        {:else}
            <div class="h-full bg-gray-200 dark:bg-gray-700 flex items-center justify-center text-gray-500">
                <span>No table path provided to TableView.</span>
            </div>
        {/if}
    {/key}
</div>

<style>
	.min-h-0 { min-height: 0; }
    /* Removed specific width classes as this component now fills the space given by DataView */
</style>