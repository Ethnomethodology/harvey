<!-- src/lib/components/projectview/data/tables/TableView.svelte -->
<script>
    import { onMount, createEventDispatcher } from 'svelte';
    // LeftInfoPanel and RightInfoPanel are removed
    // panelStateStore might not be needed if panel collapsing handled by parent
    import TableViewerPanel from './TableViewerPanel.svelte';

    export let itemPath = null; // Receives the full path from DataView
    export let hasHeaders = true;
    export let activeSubItemPath = null;
    export let activeSubItemType = null;

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

    export function openView(view) {
        if (tableViewerPanelRef && typeof tableViewerPanelRef.openView === 'function') {
            tableViewerPanelRef.openView(view);
        } else {
            console.warn('[TableView] tableViewerPanelRef or openView is not available.');
        }
    }

    export function configureView(view) {
        if (tableViewerPanelRef && typeof tableViewerPanelRef.configureView === 'function') {
            tableViewerPanelRef.configureView(view);
        } else {
            console.warn('[TableView] tableViewerPanelRef or configureView is not available.');
        }
    }

    export function handleDeletedView(viewName) {
        if (tableViewerPanelRef && typeof tableViewerPanelRef.handleDeletedView === 'function') {
            tableViewerPanelRef.handleDeletedView(viewName);
        } else {
            console.warn('[TableView] tableViewerPanelRef or handleDeletedView is not available.');
        }
    }

    export function openLexicalDocument(docPath) {
        if (tableViewerPanelRef && typeof tableViewerPanelRef.openLexicalDocument === 'function') {
            tableViewerPanelRef.openLexicalDocument(docPath);
        } else {
            console.warn('[TableView] tableViewerPanelRef or openLexicalDocument is not available.');
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
                bind:activeSubItemPath
                bind:activeSubItemType
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