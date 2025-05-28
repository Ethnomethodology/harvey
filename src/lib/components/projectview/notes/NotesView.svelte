<!-- src/lib/components/projectview/notes/NotesView.svelte -->
<script>
	import { onMount, createEventDispatcher } from 'svelte';
	import NotesTopBar from './NotesTopBar.svelte';
	import NotesLeftPanel from './NotesLeftPanel.svelte';
    import DocumentView from './documents/DocumentView.svelte';
    import TableView from './tables/TableView.svelte';
    import ImageView from './images/ImageView.svelte';
    import ImportedTranscriptView from './imported_transcripts/ImportedTranscriptView.svelte';
    import MediaView from './media/MediaView.svelte';
    import { project, prepareDocumentView, prepareImportedTranscriptView, prepareMediaNoteView } from '$lib/stores/projectStore.js';
    import { checkUnsavedChangesThenProceed } from '$lib/services/projectService.js';
    import { get } from 'svelte/store';

    const dispatch = createEventDispatcher();

    function forwardEvent(event) {
        // Check if the event is one of the new ones to be forwarded, or a generic one.
        if (event.type === 'requestviewchange' || event.type === 'requestmediaselection' ||
            event.type === 'requestTranscriptionTabWithMedia' || event.type === 'requestTrimInTranscriptionTab') {
            // Specific events handled by ProjectView or this component
             console.log(`[NotesView] Forwarding event: ${event.type} with detail:`, event.detail);
        }
		dispatch(event.type, event.detail);
	}

    let activeViewType = 'placeholder';
    let activeItemPath = null;

    const IMAGE_EXTENSIONS_SET = new Set(['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'tiff']);
    const AUDIO_EXTENSIONS_SET = new Set(['mp3','wav','m4a','ogg','aac','flac']);
    const VIDEO_EXTENSIONS_SET = new Set(['mp4','mov','avi','mkv','webm']);


    project.subscribe(value => {
        let pathFromStore = null;
        let typeFromStore = 'placeholder';

        if (value.selectedMediaNotePath) {
            pathFromStore = value.selectedMediaNotePath;
            typeFromStore = 'media_note';
        } else if (value.currentImportedTranscriptPath) {
            pathFromStore = value.currentImportedTranscriptPath;
            typeFromStore = 'imported_transcript';
        } else if (value.selectedDocumentPath) {
            pathFromStore = value.selectedDocumentPath;
            const lowerPath = pathFromStore.toLowerCase();
            const extension = lowerPath.split('.').pop();

            if (lowerPath.endsWith('.pdf') ||
                (lowerPath.endsWith('.json') && (!value.importedTranscriptFiles || value.importedTranscriptFiles.every(f => `${value.baseDirectory}/${f.relativePath}` !== pathFromStore)) && (!value.selectedMediaNotePath) ) ||
                 lowerPath.endsWith('.txt') ||
                 lowerPath.endsWith('.md')) {
                typeFromStore = 'documents';
            } else if (lowerPath.endsWith('.csv') || lowerPath.endsWith('.xlsx')) {
                typeFromStore = 'tables';
            } else if (IMAGE_EXTENSIONS_SET.has(extension)) {
                typeFromStore = 'images';
            } else {
                console.warn(`[NotesView Store Sub] Path ${pathFromStore} (from selectedDocumentPath) has undetermined type.`);
                typeFromStore = 'placeholder';
            }
        }

        if (activeItemPath !== pathFromStore || activeViewType !== typeFromStore) {
            activeItemPath = pathFromStore;
            activeViewType = typeFromStore;
            console.log(`[NotesView Store Sub] Synced local Svelte state. Path: ${activeItemPath}, ViewType: ${activeViewType}`);
        }
    });

    async function handleViewChangeRequest(eventDetailFromDispatch) {
        const pathForView = eventDetailFromDispatch?.itemPath;
        const typeForView = eventDetailFromDispatch?.viewType;

        console.log(`[NotesView] Received requestviewchange. Path: ${pathForView}, Type: ${typeForView}`);

        if (!pathForView || !typeForView || typeForView === 'placeholder') {
            console.error(`[NotesView] ABORTING: Invalid path or type from event. Path: '${pathForView}', Type: '${typeForView}'.`);
            prepareDocumentView(null, 'placeholder');
            prepareImportedTranscriptView(null);
            prepareMediaNoteView(null);
            return;
        }

        const canProceed = await checkUnsavedChangesThenProceed(pathForView, typeForView);
        if (!canProceed) {
            console.log('[NotesView] View change cancelled by unsaved changes check.');
            return;
        }

        console.log(`[NotesView] Proceeding with view change - Path: ${pathForView}, Type: ${typeForView}`);

        if (typeForView === 'documents' || typeForView === 'tables' || typeForView === 'images') {
            console.log(`[NotesView] Calling prepareDocumentView for Path: ${pathForView}, Type: ${typeForView}`);
            prepareDocumentView(pathForView, typeForView);
        } else if (typeForView === 'imported_transcript') {
            console.log(`[NotesView] Calling prepareImportedTranscriptView for Path: ${pathForView}`);
            prepareImportedTranscriptView(pathForView);
        } else if (typeForView === 'media_note') {
            console.log(`[NotesView] Calling prepareMediaNoteView for Path: ${pathForView}`);
            prepareMediaNoteView(pathForView);
        } else {
            console.warn(`[NotesView] Unknown typeForView: '${typeForView}'. Clearing all specific views.`);
            prepareDocumentView(null, 'placeholder');
            prepareImportedTranscriptView(null);
            prepareMediaNoteView(null);
        }
        console.log(`[NotesView] Store preparation actions dispatched for Path: ${pathForView}, Type: ${typeForView}.`);
    }


	onMount(() => {
		console.log('[NotesView] Component container mounted.');
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
                {:else if activeViewType === 'media_note'}
                     <MediaView
                        itemPath={activeItemPath}
                        on:requestTranscriptionTabWithMedia={forwardEvent}
                        on:requestTrimInTranscriptionTab={forwardEvent}
                     />
                 {:else if activeViewType === 'audio'}
                     <div class="h-full bg-gray-200 dark:bg-gray-700 rounded-md shadow flex items-center justify-center text-gray-500 dark:text-gray-400"><span>Audio View Placeholder (NotesView)</span></div>
                 {:else if activeViewType === 'video'}
                     <div class="h-full bg-gray-200 dark:bg-gray-700 rounded-md shadow flex items-center justify-center text-gray-500 dark:text-gray-400"><span>Video View Placeholder (NotesView)</span></div>
                 {:else if activeViewType === 'transcripts'}
                     <div class="h-full bg-gray-200 dark:bg-gray-700 rounded-md shadow flex items-center justify-center text-gray-500 dark:text-gray-400"><span>Media Transcript View Placeholder (NotesView - this shouldn't normally be active here)</span></div>
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