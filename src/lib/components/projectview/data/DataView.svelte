<!-- src/lib/components/projectview/data/DataView.svelte -->
<script>
	import { onMount, createEventDispatcher } from 'svelte';
	import panelStateStore from '$lib/stores/panelStateStore.js';
	import DataTopBar from './DataTopBar.svelte';
	import DataLeftPanel from './DataLeftPanel.svelte';
    import DocumentView from './documents/DocumentView.svelte';
    import TableView from './tables/TableView.svelte';
    import ImageView from './images/ImageView.svelte';
    import ImportedTranscriptView from './imported_transcripts/ImportedTranscriptView.svelte';
    import MediaView from './media/MediaView.svelte';
    import GroupDetailView from './groups/GroupDetailView.svelte';
    import InfoPanel from './shared_panels/InfoPanel.svelte'; // NEW
    import RightBar from './shared_panels/RightBar.svelte';   // NEW
    import { project, prepareDocumentView, prepareImportedTranscriptView, prepareMediaNoteView } from '$lib/stores/projectStore.js';
    import { checkUnsavedChangesThenProceed } from '$lib/services/projectService.js';
    import { get } from 'svelte/store';
    import { slide } from 'svelte/transition';

    const dispatch = createEventDispatcher();

    function forwardEvent(event) {
        if (event.type === 'requestviewchange' || event.type === 'requestmediaselection' ||
            event.type === 'requestTranscriptionTabWithMedia' || event.type === 'requestTrimInTranscriptionTab' || event.type === 'requestTranscriptionTabWithMediaAndDialog') {
             console.debug(`[DataView] Forwarding event: ${event.type} with detail:`, event.detail);
        }
		dispatch(event.type, event.detail);
	}

    let activeViewType = 'placeholder';
    let activeItemPath = null;
    let activeItemTypeForInfoPanel = null; // To pass to InfoPanel

    const IMAGE_EXTENSIONS_SET = new Set(['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'tiff']);

    project.subscribe(value => {
        let pathFromStore = null;
        let typeFromStore = 'placeholder';
        let itemTypeForInfo = null;

        if (value.selectedMediaNotePath) {
            pathFromStore = value.selectedMediaNotePath;
            typeFromStore = 'media_note';
            itemTypeForInfo = 'media_note'; // or 'audio' / 'video' if more specific needed by InfoPanel
        } else if (value.currentImportedTranscriptPath) {
            pathFromStore = value.currentImportedTranscriptPath;
            typeFromStore = 'imported_transcript';
            itemTypeForInfo = 'imported_transcript';
        } else if (value.selectedDocumentPath) {
            pathFromStore = value.selectedDocumentPath;
            const lowerPath = pathFromStore.toLowerCase();
            const extension = lowerPath.split('.').pop();

            if (lowerPath.endsWith('.pdf') ||
                (lowerPath.endsWith('.json') && (!value.importedTranscriptFiles || value.importedTranscriptFiles.every(f => `${value.baseDirectory}/${f.relativePath}` !== pathFromStore)) && (!value.selectedMediaNotePath) ) ||
                 lowerPath.endsWith('.txt') ||
                 lowerPath.endsWith('.md')) {
                typeFromStore = 'documents';
                itemTypeForInfo = 'doc'; // Matching what LeftInfoPanel used
            } else if (lowerPath.endsWith('.csv') || lowerPath.endsWith('.xlsx')) {
                typeFromStore = 'tables';
                itemTypeForInfo = 'table';
            } else if (IMAGE_EXTENSIONS_SET.has(extension)) {
                typeFromStore = 'images';
                itemTypeForInfo = 'image';
            } else {
                console.warn(`[DataView Store Sub] Path ${pathFromStore} (from selectedDocumentPath) has undetermined type.`);
                typeFromStore = 'placeholder';
                itemTypeForInfo = null;
            }
        } else if (value.selectedGroupId && value.selectedGroupData) {
            pathFromStore = value.selectedGroupId;
            typeFromStore = 'group_detail';
            itemTypeForInfo = 'group'; // Or null if InfoPanel doesn't show metadata for groups
        }


        if (activeItemPath !== pathFromStore || activeViewType !== typeFromStore) {
            activeItemPath = pathFromStore;
            activeViewType = typeFromStore;
            activeItemTypeForInfoPanel = itemTypeForInfo; // Update type for InfoPanel
            console.debug(`[DataView Store Sub] Synced. Path: ${activeItemPath}, ViewType: ${activeViewType}, InfoPanelType: ${activeItemTypeForInfoPanel}`);
        } else if (activeItemTypeForInfoPanel !== itemTypeForInfo) {
            // Path and view type might be same, but specific type for info panel changed (e.g. media_note to audio)
            activeItemTypeForInfoPanel = itemTypeForInfo;
             console.debug(`[DataView Store Sub] InfoPanelType updated to: ${activeItemTypeForInfoPanel}`);
        }
    });

    async function handleViewChangeRequest(eventDetailFromDispatch) {
        const pathForView = eventDetailFromDispatch?.itemPath;
        const typeForView = eventDetailFromDispatch?.viewType;

        console.debug(`[DataView] Received requestviewchange. Path: ${pathForView}, Type: ${typeForView}`);

        // If the requested path and type are already active, do nothing.
        if (pathForView === activeItemPath && typeForView === activeViewType) {
            console.debug(`[DataView] Requested view change to already active item. Path: ${pathForView}, Type: ${typeForView}. Aborting redundant action.`);
            return;
        }

        if (!pathForView || !typeForView || typeForView === 'placeholder') {
            console.error(`[DataView] ABORTING: Invalid path or type. Path: '${pathForView}', Type: '${typeForView}'.`);
            prepareDocumentView(null, 'placeholder');
            prepareImportedTranscriptView(null);
            prepareMediaNoteView(null);
            activeItemTypeForInfoPanel = null; // Clear for InfoPanel too
            return;
        }

        const canProceed = await checkUnsavedChangesThenProceed(pathForView, typeForView);
        if (!canProceed) {
            console.info('[DataView] View change cancelled.');
            return;
        }

        console.debug(`[DataView] Proceeding with view change - Path: ${pathForView}, Type: ${typeForView}`);

        if (typeForView === 'documents' || typeForView === 'tables' || typeForView === 'images') {
            prepareDocumentView(pathForView, typeForView);
            // activeItemTypeForInfoPanel will be set by the project.subscribe block
        } else if (typeForView === 'imported_transcript') {
            prepareImportedTranscriptView(pathForView);
        } else if (typeForView === 'media_note') {
            prepareMediaNoteView(pathForView);
        } else {
            console.warn(`[DataView] Unknown typeForView: '${typeForView}'. Clearing views.`);
            prepareDocumentView(null, 'placeholder');
            prepareImportedTranscriptView(null);
            prepareMediaNoteView(null);
            activeItemTypeForInfoPanel = null;
        }
        console.debug(`[DataView] Store preparation actions dispatched for Path: ${pathForView}, Type: ${typeForView}.`);
    }

    function handleRightBarTabChange(event) {
        const { tabName } = event.detail;
        // panelStateStore.setActiveInfoPanelTab(tabName); // This is already done in RightBar.svelte
        console.log(`[DataView] RightBar tab changed to: ${tabName}`);
        // InfoPanel will react to panelStateStore.activeInfoPanelTab
    }

	onMount(() => {
	});

</script>

<div class="flex flex-col h-full w-full bg-gray-100 dark:bg-app-bg-dark overflow-hidden">

	<DataTopBar on:requestTranscriptionTabWithMediaAndDialog={forwardEvent} />

	<div class="flex flex-grow w-full min-h-0 p-1 gap-1">
        <!-- Far Left Panel (File/Data Browser) -->
		<div class="{ $panelStateStore.dataLeftPanelCollapsed ? 'w-12' : 'w-[15%]' } h-full flex-shrink-0 transition-all duration-300 ease-in-out">
			<DataLeftPanel
                on:requestmediaselection={forwardEvent}
                on:requestviewchange={ (event) => handleViewChangeRequest(event.detail) }
                on:requestTranscriptionTabWithMediaAndDialog={forwardEvent}
            />
		</div>

        <!-- Main Content Area (Middle) -->
        <div class="flex-grow h-full min-w-0">
            {#key activeItemPath + activeViewType}
                {#if activeViewType === 'placeholder' || !activeItemPath}
                    <div class="h-full bg-gray-200 dark:bg-gray-700 rounded-md shadow flex items-center justify-center text-gray-500 dark:text-gray-400">
                        <span>Select an item from the Data panel to view or edit.</span>
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
                 {:else if activeViewType === 'group_detail' && $project.selectedGroupData}
                    <GroupDetailView
                        groupData={$project.selectedGroupData}
                        on:requestmediaselection={forwardEvent}
                        on:requestopentab={(event) => handleViewChangeRequest(event.detail)}
                        on:requestviewchange={(event) => handleViewChangeRequest(event.detail)}
                    />
                {:else}
                    <div class="h-full bg-gray-200 dark:bg-gray-700 rounded-md shadow flex items-center justify-center text-gray-500 dark:text-gray-400">
                        <span>Selected view type '{activeViewType}' not recognized or item path invalid.</span>
                    </div>
                {/if}
            {/key}
        </div>

        <!-- New Info Panel (Right of Main Content, Left of RightBar) -->
        {#if !$panelStateStore.infoPanelCollapsed && activeItemPath && activeViewType !== 'group_detail'}
            <div class="w-[20.588%] h-full flex-shrink-0 transition-all duration-300 ease-in-out" transition:slide="{{ duration: 300, axis: 'x' }}">
                <InfoPanel itemPath={activeItemPath} itemType={activeItemTypeForInfoPanel} />
            </div>
        {/if}
        <!-- Consider adding a toggle button for infoPanelCollapsed if needed, or manage via RightBar interaction -->

        <!-- New Right Bar (Far Right) -->
        {#if activeViewType !== 'group_detail' && activeItemPath}
            <div class="h-full flex-shrink-0">
                <RightBar on:tabchange={handleRightBarTabChange} />
            </div>
        {/if}
	</div>
</div>

<style>
	.min-h-0 { min-height: 0; }
    .w-\[15\%\] { width: 15%; } /* For DataLeftPanel */
    .w-\[20\.588\%\] { width: 20.58825%; } /* For new InfoPanel, same as old LeftInfoPanel width */
</style>
