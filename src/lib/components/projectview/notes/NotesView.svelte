<!-- src/lib/components/projectview/notes/NotesView.svelte -->
<script>
	import { onMount, createEventDispatcher } from 'svelte';
	import panelStateStore from '$lib/stores/panelStateStore.js';
	import NotesTopBar from './NotesTopBar.svelte';
	import NotesLeftPanel from './NotesLeftPanel.svelte';
    import DocumentView from './documents/DocumentView.svelte';
    import TableView from './tables/TableView.svelte';
    import ImageView from './images/ImageView.svelte';
    import ImportedTranscriptView from './imported_transcripts/ImportedTranscriptView.svelte';
    import MediaView from './media/MediaView.svelte';
    import GroupDetailView from './groups/GroupDetailView.svelte'; // Added
    import { project, prepareDocumentView, prepareImportedTranscriptView, prepareMediaNoteView } from '$lib/stores/projectStore.js';
    import { checkUnsavedChangesThenProceed } from '$lib/services/projectService.js';
    import { get } from 'svelte/store';

    const dispatch = createEventDispatcher();

    function forwardEvent(event) {
        // Check if the event is one of the new ones to be forwarded, or a generic one.
        if (event.type === 'requestviewchange' || event.type === 'requestmediaselection' ||
            event.type === 'requestTranscriptionTabWithMedia' || event.type === 'requestTrimInTranscriptionTab') {
            // Specific events handled by ProjectView or this component
             console.debug(`[NotesView] Forwarding event: ${event.type} with detail:`, event.detail); // DEBUG
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
                typeFromStore = 'placeholder'; // Default if no other type matches
            }
        } else if (value.selectedGroupId && value.selectedGroupData) {
            pathFromStore = value.selectedGroupId; // Using groupId as the "path" for keying the view
            typeFromStore = 'group_detail';
        }


        if (activeItemPath !== pathFromStore || activeViewType !== typeFromStore) {
            activeItemPath = pathFromStore;
            activeViewType = typeFromStore;
            console.debug(`[NotesView Store Sub] Synced local Svelte state. Path: ${activeItemPath}, ViewType: ${activeViewType}`); // DEBUG
        }
    });

    async function handleViewChangeRequest(eventDetailFromDispatch) {
        const pathForView = eventDetailFromDispatch?.itemPath;
        const typeForView = eventDetailFromDispatch?.viewType;

        console.debug(`[NotesView] Received requestviewchange. Path: ${pathForView}, Type: ${typeForView}`); // DEBUG

        if (!pathForView || !typeForView || typeForView === 'placeholder') {
            console.error(`[NotesView] ABORTING: Invalid path or type from event. Path: '${pathForView}', Type: '${typeForView}'.`); // ERROR
            prepareDocumentView(null, 'placeholder');
            prepareImportedTranscriptView(null);
            prepareMediaNoteView(null);
            return;
        }

        const canProceed = await checkUnsavedChangesThenProceed(pathForView, typeForView);
        if (!canProceed) {
            console.info('[NotesView] View change cancelled by unsaved changes check.'); // INFO
            return;
        }

        console.debug(`[NotesView] Proceeding with view change - Path: ${pathForView}, Type: ${typeForView}`); // DEBUG

        if (typeForView === 'documents' || typeForView === 'tables' || typeForView === 'images') {
            console.debug(`[NotesView] Calling prepareDocumentView for Path: ${pathForView}, Type: ${typeForView}`); // DEBUG
            prepareDocumentView(pathForView, typeForView);
        } else if (typeForView === 'imported_transcript') {
            console.debug(`[NotesView] Calling prepareImportedTranscriptView for Path: ${pathForView}`); // DEBUG
            prepareImportedTranscriptView(pathForView);
        } else if (typeForView === 'media_note') {
            console.debug(`[NotesView] Calling prepareMediaNoteView for Path: ${pathForView}`); // DEBUG
            prepareMediaNoteView(pathForView);
        } else {
            console.warn(`[NotesView] Unknown typeForView: '${typeForView}'. Clearing all specific views.`); // WARN
            prepareDocumentView(null, 'placeholder');
            prepareImportedTranscriptView(null);
            prepareMediaNoteView(null);
        }
        console.debug(`[NotesView] Store preparation actions dispatched for Path: ${pathForView}, Type: ${typeForView}.`); // DEBUG
    }


	onMount(() => {
		console.debug('[NotesView] Component container mounted.'); // DEBUG
	});

</script>

<div class="flex flex-col h-full w-full bg-gray-100 dark:bg-app-bg-dark overflow-hidden">

	<NotesTopBar />

	<div class="flex flex-grow p-1 gap-1 w-full min-h-0">

		<div class="{ $panelStateStore.notesLeftPanelCollapsed ? 'w-12' : 'w-[15%]' } h-full flex-shrink-0 transition-all duration-300 ease-in-out">
			<NotesLeftPanel
                on:requestmediaselection={forwardEvent}
                on:requestviewchange={ (event) => handleViewChangeRequest(event.detail) }
            />
		</div>

        <div class="flex-grow h-full min-w-0">
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
                {:else if activeViewType === 'group_detail' && $project.selectedGroupData}
                    <GroupDetailView
                        groupData={$project.selectedGroupData}
                        on:requestmediaselection={forwardEvent}
                        on:requestopentab={(event) => handleViewChangeRequest(event.detail)}
                        on:requestviewchange={(event) => handleViewChangeRequest(event.detail)}
                    />
                {:else}
                    <div class="h-full bg-gray-200 dark:bg-gray-700 rounded-md shadow flex items-center justify-center text-gray-500 dark:text-gray-400">
                        <span>Selected view type '{activeViewType}' not recognized, item path invalid, or required data missing.</span>
                    </div>
                {/if}
            {/key}
        </div>

	</div>
</div>

<style>
	.min-h-0 { min-height: 0; }
    /* .w-\[15\%\] { width: 15%; } */ /* No longer needed as Tailwind handles it or it's inline */
    /* .w-\[85\%\] { width: 85%; } */ /* No longer needed as flex-grow is used */
</style>