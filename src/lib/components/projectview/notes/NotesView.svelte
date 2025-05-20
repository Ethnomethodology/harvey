<!-- src/lib/components/projectview/notes/NotesView.svelte -->
<script>
	import { onMount, createEventDispatcher } from 'svelte';
	import NotesTopBar from './NotesTopBar.svelte';
	import NotesLeftPanel from './NotesLeftPanel.svelte';
    import DocumentView from './documents/DocumentView.svelte';
    import TableView from './tables/TableView.svelte';
    import ImageView from './images/ImageView.svelte';
    import ImportedTranscriptView from './imported_transcripts/ImportedTranscriptView.svelte'; 
    import { project, prepareDocumentView, prepareImportedTranscriptView } from '$lib/stores/projectStore.js'; 
    import { checkUnsavedChangesThenProceed } from '$lib/services/projectService.js'; 
    import { get } from 'svelte/store';
    // import { convertFileSrc } from '@tauri-apps/api/core'; // Not used here directly

    const dispatch = createEventDispatcher();

    function forwardEvent(event) {
        if (event.type === 'requestviewchange') {
            // Event is already handled directly by NotesView's on:requestviewchange
            // console.log("[NotesView forwardEvent] requestviewchange should be handled by NotesView directly now.");
        } else {
		    dispatch(event.type, event.detail);
        }
	}

    let activeViewType = 'placeholder'; 
    let activeItemPath = null;

    const IMAGE_EXTENSIONS_SET = new Set(['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'tiff']); // Renamed for clarity

    project.subscribe(value => {
        let pathFromStore = null;
        let typeFromStore = 'placeholder';

        // Determine the single active item and its type from the store
        if (value.currentImportedTranscriptPath) {
            pathFromStore = value.currentImportedTranscriptPath;
            typeFromStore = 'imported_transcript';
        } else if (value.selectedDocumentPath) { 
            pathFromStore = value.selectedDocumentPath;
            const lowerPath = pathFromStore.toLowerCase();
            const extension = lowerPath.split('.').pop();
            
            // Ensure this logic correctly identifies .json documents vs. .json imported_transcripts
            // This relies on currentImportedTranscriptPath being null if it's a regular document
            if (lowerPath.endsWith('.pdf') || 
                (lowerPath.endsWith('.json') && value.importedTranscriptFiles?.every(f => `${value.baseDirectory}/${f.relativePath}` !== pathFromStore) ) || 
                 lowerPath.endsWith('.txt') || 
                 lowerPath.endsWith('.md')) {
                typeFromStore = 'documents';
            } else if (lowerPath.endsWith('.csv') || lowerPath.endsWith('.xlsx')) {
                typeFromStore = 'tables';
            } else if (IMAGE_EXTENSIONS_SET.has(extension)) {
                typeFromStore = 'images';
            } else if (value.importedTranscriptFiles?.some(f => `${value.baseDirectory}/${f.relativePath}` === pathFromStore)) {
                // This case might be redundant if currentImportedTranscriptPath is correctly set
                typeFromStore = 'imported_transcript';
            } else {
                console.warn(`[NotesView Store Sub] Path ${pathFromStore} is selectedDocumentPath but type couldn't be determined.`);
                typeFromStore = 'placeholder'; // Fallback if type is ambiguous from selectedDocumentPath
            }
        }
        
        // Only update local Svelte state if it truly differs from the store's derived state
        if (activeItemPath !== pathFromStore || activeViewType !== typeFromStore) {
            activeItemPath = pathFromStore;
            activeViewType = typeFromStore;
            console.log(`[NotesView Store Sub] Synced local Svelte state. Path: ${activeItemPath}, ViewType: ${activeViewType}`);
        }
    });

    async function handleViewChangeRequest(eventDetailFromDispatch) {
        // Capture values at the start of the function scope
        const pathForView = eventDetailFromDispatch?.itemPath;
        const typeForView = eventDetailFromDispatch?.viewType;

        console.log(`[NotesView] Received requestviewchange. Path: ${pathForView}, Type: ${typeForView}`);
        
        if (!pathForView || !typeForView || typeForView === 'placeholder') {
            console.error(`[NotesView] ABORTING: Invalid path or type from event. Path: '${pathForView}', Type: '${typeForView}'.`);
            // Clear views if essential info is missing
            prepareDocumentView(null, 'placeholder');
            prepareImportedTranscriptView(null);
            return;
        }

        const canProceed = await checkUnsavedChangesThenProceed(pathForView, typeForView); // typeForView here is for action context description
        if (!canProceed) {
            console.log('[NotesView] View change cancelled by unsaved changes check.');
            return;
        }
        
        console.log(`[NotesView] Proceeding with view change - Path: ${pathForView}, Type: ${typeForView}`);

        // These calls update the global Svelte store.
        // The `project.subscribe` block above will react to these store changes
        // to update local `activeItemPath` and `activeViewType` for the {#key}.
        if (typeForView === 'documents' || typeForView === 'tables' || typeForView === 'images') {
            console.log(`[NotesView] Calling prepareDocumentView for Path: ${pathForView}, Type: ${typeForView}`);
            prepareDocumentView(pathForView, typeForView); 
        } else if (typeForView === 'imported_transcript') {
            console.log(`[NotesView] Calling prepareImportedTranscriptView for Path: ${pathForView}`);
            prepareImportedTranscriptView(pathForView); 
        } else {
            console.warn(`[NotesView] Unknown typeForView: '${typeForView}'. Clearing all specific views.`);
            prepareDocumentView(null, 'placeholder'); 
            prepareImportedTranscriptView(null);
        }
        // The local activeItemPath and activeViewType will be updated by the project.subscribe block.
        console.log(`[NotesView] Store preparation actions dispatched for Path: ${pathForView}, Type: ${typeForView}.`);
    }


	onMount(() => {
		console.log('[NotesView] Component container mounted.');
        // The project.subscribe block will handle setting the initial activeItemPath and activeViewType
        // based on the store's state when the component mounts.
	});

</script>

<div class="flex flex-col h-full w-full bg-gray-100 dark:bg-app-bg-dark overflow-hidden">

	<NotesTopBar />

	<div class="flex flex-grow p-1 gap-1 w-full min-h-0">

		<div class="w-[15%] h-full flex-shrink-0">
			<NotesLeftPanel
                on:requestmediaselection={forwardEvent}
                on:requestviewchange={ (event) => handleViewChangeRequest(event.detail) }
            />
		</div>

        <div class="w-[85%] h-full">
            {#key activeItemPath + activeViewType} 
                {#if activeViewType === 'placeholder' || !activeItemPath}
                    <div class="h-full bg-gray-200 dark:bg-gray-700 rounded-md shadow flex items-center justify-center text-gray-500 dark:text-gray-400">
                        <span>Select an item from the Fieldnotes panel to view or edit.</span>
                    </div>
                {:else if activeViewType === 'documents'}
                    <DocumentView itemPath={activeItemPath} />
                {:else if activeViewType === 'tables'}
                     <TableView itemPath={activeItemPath} />
                 {:else if activeViewType === 'images'}
                     <ImageView itemPath={activeItemPath} />
                {:else if activeViewType === 'imported_transcript'}
                     <ImportedTranscriptView itemPath={activeItemPath} /> 
                 {:else if activeViewType === 'audio'}
                     <div class="h-full bg-gray-200 dark:bg-gray-700 rounded-md shadow flex items-center justify-center text-gray-500 dark:text-gray-400"><span>Audio View Placeholder</span></div>
                 {:else if activeViewType === 'video'}
                     <div class="h-full bg-gray-200 dark:bg-gray-700 rounded-md shadow flex items-center justify-center text-gray-500 dark:text-gray-400"><span>Video View Placeholder</span></div>
                 {:else if activeViewType === 'transcripts'} 
                     <div class="h-full bg-gray-200 dark:bg-gray-700 rounded-md shadow flex items-center justify-center text-gray-500 dark:text-gray-400"><span>Media Transcript View Placeholder</span></div>
                {:else}
                    <div class="h-full bg-gray-200 dark:bg-gray-700 rounded-md shadow flex items-center justify-center text-gray-500 dark:text-gray-400">
                        <span>Selected view type '{activeViewType}' not recognized or item path invalid.</span>
                    </div>
                {/if}
            {/key}
        </div>

	</div>
</div>

<style>
	.min-h-0 { min-height: 0; }
    .w-\[15\%\] { width: 15%; }
    .w-\[85\%\] { width: 85%; }
</style>

