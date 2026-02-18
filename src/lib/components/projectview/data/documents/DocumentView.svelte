<!-- src/lib/components/projectview/data/documents/DocumentView.svelte -->
<script>
    import { onMount, createEventDispatcher } from 'svelte';
    import { get } from 'svelte/store';
    import { invoke, convertFileSrc } from '@tauri-apps/api/core';
    // LeftInfoPanel and RightInfoPanel are removed
    import DocumentEditorPanel from './DocumentEditorPanel.svelte'; 
    import PDFViewerPanel from './PDFViewerPanel.svelte';          
    import { project, updateDocumentHighlights } from '$lib/stores/projectStore.js'; 
    import MediaPlayer from '../../shared/MediaPlayer.svelte';
    // panelStateStore might not be needed here anymore if panel collapsing is handled by parent (DataView)

    export let itemPath = null; 

    $: isPdf = itemPath ? itemPath.toLowerCase().endsWith('.pdf') : false;
    $: isJsonDoc = itemPath ? itemPath.toLowerCase().endsWith('.json') : false; // Assuming .json are editable docs if not PDFs
    
    let currentHighlightsFromStore = [];
    $: currentHighlightsFromStore = $project.selectedDocumentPath === itemPath ? $project.currentDocumentHighlights : [];

    const dispatch = createEventDispatcher();

    // Media Player State
    let mediaPath = null;
    let attachments = [];
    let mediaPlayerRef;
    let isVideoHidden = false;
    let currentTime = 0;
    let isPlaying = false;

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

    export function playMedia(path) {
        if (path) {
            mediaPath = path;
        }
    }

    async function loadAttachments(path) {
        const projectStoreState = get(project);
        if (!projectStoreState.id || !path) {
            attachments = [];
            mediaPath = null;
            return;
        }

        // Derived relative path
        let assetRelativePath = path.startsWith(projectStoreState.baseDirectory) 
            ? path.substring(projectStoreState.baseDirectory.length) 
            : path;
        assetRelativePath = assetRelativePath.replace(/\\/g, '/').replace(/^\//, '');

        try {
            const result = await invoke('get_asset_metadata_command', {
                projectId: projectStoreState.id,
                assetRelativePath: assetRelativePath
            });

            if (result && result.custom_fields_json) {
                const customFields = JSON.parse(result.custom_fields_json);
                const attachmentsField = customFields.find(f => f.key === 'attachments');
                if (attachmentsField && attachmentsField.value) {
                    attachments = JSON.parse(attachmentsField.value);
                    console.log("[DocumentView] Loaded attachments:", attachments);
                    // Do not auto-set mediaPath
                    // if (attachments.length > 0) {
                    //    mediaPath = convertFileSrc(attachments[0]);
                    // } else {
                    //    mediaPath = null;
                    // }
                } else {
                    attachments = [];
                    // mediaPath = null;
                }
            } else {
                attachments = [];
                // mediaPath = null;
            }
        } catch (error) {
            console.error(`[DocumentView] Error loading attachments:`, error);
            attachments = [];
            mediaPath = null;
        }
    }

    // Reload attachments when itemPath changes
    $: if (itemPath) {
        mediaPath = null;
        loadAttachments(itemPath);
    }

    onMount(() => {
		console.log('[DocumentView] Component container mounted. Document path:', itemPath, 'Is PDF:', isPdf);
        if (itemPath) loadAttachments(itemPath);
	});

    $: { 
        // console.log(`[DocumentView] Path is now ${itemPath}, isPdf is ${isPdf}, isJsonDoc is ${isJsonDoc}`);
    }

</script>

<!-- Main container for the Document View - this will now be the main content panel -->
<div class="h-full flex flex-col flex-grow min-w-0 bg-white dark:bg-gray-900">
    {#if mediaPath}
        <div class="border-b border-gray-200 dark:border-gray-700 flex flex-col {!isVideoHidden ? 'h-1/2' : 'h-auto flex-shrink-0'}">
            <MediaPlayer
                bind:this={mediaPlayerRef}
                bind:isVideoMinimized={isVideoHidden}
                bind:localCurrentTime={currentTime}
                bind:localIsPlaying={isPlaying}
                explicitMediaPath={mediaPath}
                autoPlay={true}
                projectId={$project.id}
                showLoopPauseButton={false}
                showDataTranscribeButton={false}
                showDataTrimButton={false} 
                class="{!isVideoHidden ? 'flex-grow min-h-0' : ''}"
            />
        </div>
    {/if}

    <div class="flex-grow min-h-0 overflow-hidden {mediaPath && !isVideoHidden ? 'h-1/2' : 'h-full'}">
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
                    <div class="h-full bg-gray-200 dark:bg-gray-800 flex items-center justify-center text-gray-500 dark:text-gray-600">
                        <span>Viewing for this document type ({itemPath?.split('.').pop()}) not implemented.</span>
                    </div>
                {/if} 
            {:else}
                <div class="h-full bg-gray-200 dark:bg-gray-800 flex items-center justify-center text-gray-500 dark:text-gray-600">
                    <span>No document path provided to DocumentView.</span>
                </div>
            {/if}
        {/key}
    </div>
</div>

<style>
	.min-h-0 { min-height: 0; }
    /* Removed specific width classes as this component now fills the space given by DataView */
</style>
