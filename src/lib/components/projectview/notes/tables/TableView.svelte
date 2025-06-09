<!-- src/lib/components/projectview/notes/tables/TableView.svelte -->
<script>
    import { onMount, createEventDispatcher } from 'svelte';
    // Import shared panels
    import LeftInfoPanel from '../shared_panels/LeftInfoPanel.svelte';
    import RightInfoPanel from '../shared_panels/RightInfoPanel.svelte';
    import panelStateStore from '$lib/stores/panelStateStore.js';
    // Import the specific table viewer panel (placeholder for now)
    import TableViewerPanel from './TableViewerPanel.svelte';

    export let itemPath = null; // Receives the full path from NotesView

    const dispatch = createEventDispatcher();

    function forwardEvent(event) {
        console.debug(`[TableView] Forwarding event: ${event.type}`); // DEBUG
		dispatch(event.type, event.detail);
	}

    onMount(() => {
		console.debug('[TableView] Component container mounted. Table path:', itemPath); // DEBUG
	});

    $: { // Log when path changes
        console.debug(`[TableView] Path is now ${itemPath}`); // DEBUG
    }

    let leftPanelClasses;
    $: leftPanelClasses = `h-full flex-shrink-0 transition-all duration-300 ease-in-out ${$panelStateStore.leftCollapsed ? 'w-12' : 'w-1/5'}`;

    let rightPanelClasses;
    $: rightPanelClasses = `h-full flex-shrink-0 transition-all duration-300 ease-in-out ${$panelStateStore.rightCollapsed ? 'w-12' : 'w-1/5'}`;

</script>

<!-- Main container for the Table View -->
<div class="flex flex-grow p-0 gap-1 w-full min-h-0 h-full">

    <!-- Left Panel (Shared) -->
    <div class="{leftPanelClasses}">
        <LeftInfoPanel itemPath={itemPath} itemType="table" />
    </div>

    <!-- Middle Panel - The Table Viewer -->
    <div class="h-full w-3/5">
        {#key itemPath} {#if itemPath}
             <TableViewerPanel tablePath={itemPath} />
        {:else}
             <!-- Optional: Show a placeholder if itemPath is null -->
             <div class="h-full bg-gray-200 dark:bg-gray-700 rounded-md shadow flex items-center justify-center text-gray-500">
                 <span>No table path provided to TableView.</span>
             </div>
        {/if} {/key}
    </div>

    <!-- Right Panel (Shared) -->
    <div class="{rightPanelClasses}">
        <RightInfoPanel itemPath={itemPath} itemType="table" />
    </div>

</div>

<style>
	.min-h-0 { min-height: 0; }
    /* Define width classes using arbitrary values */
    /* .w-\[20\.588\%\] { width: 20.58825%; } */ /* Replaced with w-1/5 */
</style>