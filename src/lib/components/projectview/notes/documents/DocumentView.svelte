<!-- src/lib/components/projectview/notes/documents/DocumentView.svelte -->
<script>
    import { onMount, createEventDispatcher } from 'svelte';
    import { get } from 'svelte/store'; 
    // Import shared panels from the new location
    import LeftInfoPanel from '../shared_panels/LeftInfoPanel.svelte';
    import RightInfoPanel from '../shared_panels/RightInfoPanel.svelte';
    // Keep imports for the specific document panels
    import DocumentEditorPanel from './DocumentEditorPanel.svelte'; 
    import PDFViewerPanel from './PDFViewerPanel.svelte';          
    import { project, updateDocumentHighlights } from '$lib/stores/projectStore.js'; 
    import panelStateStore from '$lib/stores/panelStateStore.js';

    export let itemPath = null; 

    $: isPdf = itemPath ? itemPath.toLowerCase().endsWith('.pdf') : false;
    $: isJsonDoc = itemPath ? itemPath.toLowerCase().endsWith('.json') : false;
    
    // Reactive variable to get current highlights from the store
    let currentHighlightsFromStore = [];
    $: currentHighlightsFromStore = $project.selectedDocumentPath === itemPath ? $project.currentDocumentHighlights : [];


    const dispatch = createEventDispatcher();

    function forwardEvent(event) {
        console.log(`[DocumentView] Forwarding event: ${event.type}`);
		dispatch(event.type, event.detail);
	}

    function handlePdfHighlight(event) {
        console.log('[DocumentView] Received pdfhighlightevent from PDFViewerPanel:', event.detail);
        const currentSelectedPathInStore = get(project).selectedDocumentPath;
        
        if (currentSelectedPathInStore === itemPath) { 
            updateDocumentHighlights(event.detail); 
        } else {
            console.warn(`[DocumentView] PDF Highlight event for path ${itemPath} but store selected path is ${currentSelectedPathInStore}. Event ignored.`);
        }
    }

    onMount(() => {
		console.log('[DocumentView] Component container mounted. Document path:', itemPath, 'Is PDF:', isPdf);
	});

    $: { 
        console.log(`[DocumentView] Path is now ${itemPath}, isPdf is ${isPdf}, isJsonDoc is ${isJsonDoc}`);
    }

</script>

<!-- Main container for the Document View -->
<div class="flex flex-grow p-0 gap-1 w-full min-h-0 h-full">

    <!-- Left Panel (Shared) -->
    <div class="h-full flex-shrink-0 transition-all duration-300 ease-in-out"
         class:w-12={$panelStateStore.leftCollapsed}
         class:w-[20.588%]={!$panelStateStore.leftCollapsed} >
        <LeftInfoPanel itemPath={itemPath} itemType="document" />
    </div>

    <!-- Middle Panel - The Editor OR Viewer -->
    <div class="h-full flex-grow">
        {#key itemPath} 
            {#if itemPath} 
                {#if isPdf}
                    <PDFViewerPanel 
                        pdfPath={itemPath} 
                        initialHighlights={currentHighlightsFromStore} 
                        on:pdfhighlightevent={handlePdfHighlight} 
                    />
                {:else if isJsonDoc}
                    <DocumentEditorPanel />
                {:else}
                    <div class="h-full bg-gray-200 dark:bg-gray-700 rounded-md shadow flex items-center justify-center text-gray-500">
                        <span>Viewing for this document type ({itemPath?.split('.').pop()}) not implemented.</span>
                    </div>
                {/if} 
            {:else}
                <div class="h-full bg-gray-200 dark:bg-gray-700 rounded-md shadow flex items-center justify-center text-gray-500">
                    <span>No document path provided to DocumentView.</span>
                </div>
            {/if} 
        {/key}
    </div>

    <!-- Right Panel (Shared) -->
    <div class="h-full flex-shrink-0 transition-all duration-300 ease-in-out"
         class:w-12={$panelStateStore.rightCollapsed}
         class:w-[20.588%]={!$panelStateStore.rightCollapsed} >
        <RightInfoPanel itemPath={itemPath} itemType="document" />
    </div>

</div>

<style>
	.min-h-0 { min-height: 0; }
    .w-\[20\.588\%\] { width: 20.58825%; } /* Retained for explicit expanded width */
</style>
