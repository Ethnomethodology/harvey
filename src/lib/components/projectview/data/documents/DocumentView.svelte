<!-- src/lib/components/projectview/data/documents/DocumentView.svelte -->
<script>
    import { onMount, createEventDispatcher } from 'svelte';
    import { get } from 'svelte/store'; 
    // LeftInfoPanel and RightInfoPanel are removed
    import DocumentEditorPanel from './DocumentEditorPanel.svelte'; 
    import PDFViewerPanel from './PDFViewerPanel.svelte';          
    import { project, updateDocumentHighlights } from '$lib/stores/projectStore.js'; 
    // panelStateStore might not be needed here anymore if panel collapsing is handled by parent (DataView)

    export let itemPath = null; 

    $: isPdf = itemPath ? itemPath.toLowerCase().endsWith('.pdf') : false;
    $: isJsonDoc = itemPath ? itemPath.toLowerCase().endsWith('.json') : false; // Assuming .json are editable docs if not PDFs
    
    let currentHighlightsFromStore = [];
    $: currentHighlightsFromStore = $project.selectedDocumentPath === itemPath ? $project.currentDocumentHighlights : [];

    const dispatch = createEventDispatcher();

    // Forwarding events might still be needed if DocumentEditorPanel or PDFViewerPanel emit them
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
        // console.log(`[DocumentView] Path is now ${itemPath}, isPdf is ${isPdf}, isJsonDoc is ${isJsonDoc}`);
    }

</script>

<!-- Main container for the Document View - this will now be the main content panel -->
<div class="h-full flex-grow min-w-0 bg-white dark:bg-gray-800 shadow">
    {#key itemPath}
        {#if itemPath}
            {#if isPdf}
                <PDFViewerPanel
                    pdfPath={itemPath}
                    initialHighlights={currentHighlightsFromStore}
                    on:pdfhighlightevent={handlePdfHighlight}
                />
            {:else if isJsonDoc} <!-- Assuming .json documents are handled by DocumentEditorPanel -->
                <DocumentEditorPanel />
            {:else} <!-- Fallback for other non-PDF, non-JSON document types -->
                <div class="h-full bg-gray-200 dark:bg-gray-700 shadow flex items-center justify-center text-gray-500">
                    <span>Viewing for this document type ({itemPath?.split('.').pop()}) not implemented.</span>
                </div>
            {/if} 
        {:else}
            <div class="h-full bg-gray-200 dark:bg-gray-700 shadow flex items-center justify-center text-gray-500">
                <span>No document path provided to DocumentView.</span>
            </div>
        {/if}
    {/key}
</div>

<style>
	.min-h-0 { min-height: 0; }
    /* Removed specific width classes as this component now fills the space given by DataView */
</style>
