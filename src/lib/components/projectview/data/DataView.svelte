<!-- src/lib/components/projectview/data/DataView.svelte -->
<script>
  import { onMount, createEventDispatcher, tick } from 'svelte';
  import panelStateStore from '$lib/stores/panelStateStore.svelte.js';
  import DataTopBar from './DataTopBar.svelte';
  import DataLeftPanel from './DataLeftPanel.svelte';
  import DocumentView from './documents/DocumentView.svelte';
  import TableView from './tables/TableView.svelte';
  import ImageView from './images/ImageView.svelte';
  import StandaloneTranscriptView from './standalone_transcripts/StandaloneTranscriptView.svelte';
  import MediaView from './media/MediaView.svelte';
  import GroupDetailView from './groups/GroupDetailView.svelte';
  import InfoPanel from './shared_panels/InfoPanel.svelte';
  import HighlightsPanel from './shared_panels/HighlightsPanel.svelte';
  import AttachmentsPanel from './shared_panels/AttachmentsPanel.svelte';
  import RightBar from './shared_panels/RightBar.svelte';
  import {
    project,
    prepareDocumentView,
    prepareStandaloneTranscriptView,
    prepareMediaNoteView
  } from '$lib/stores/projectStore.js';
  import { checkUnsavedChangesThenProceed, normalizePath } from '$lib/services/projectService.js';
  import { get } from 'svelte/store';
  import { slide } from 'svelte/transition';
  import { refresher } from '$lib/stores/refresherStore.js';
  import { Plus } from '@lucide/svelte';

  const dispatch = createEventDispatcher();

  function forwardEvent(event) {
    if (
      event.type === 'requestviewchange' ||
      event.type === 'requestmediaselection' ||
      event.type === 'requestTranscriptionTabWithMedia' ||
      event.type === 'requestTrimInTranscriptionTab' ||
      event.type === 'requestTranscriptionTabWithMediaAndDialog' ||
      event.type === 'requestTranslationTabWithMediaAndDialog'
    ) {
      console.debug(`[DataView] Forwarding event: ${event.type} with detail:`, event.detail);
    }
    dispatch(event.type, event.detail);
  }

  let activeViewType = 'placeholder';
  let activeItemPath = null;
  let activeItemTypeForInfoPanel = null; // To pass to InfoPanel
  export let activeSubItemPath = null; // Sub-item context for nested Lexical views
  export let activeSubItemType = null;
  export let tableViewRef;
  let imageViewRef;
  let documentViewRef;
  let standaloneTranscriptViewRef;

  export async function getExportData() {
    console.log('[DataView] getExportData called. tableViewRef:', !!tableViewRef);
    if (tableViewRef) {
      return await tableViewRef.getExportData();
    }
    return null;
  }

  export function triggerImageExport() {
    if (imageViewRef) {
      imageViewRef.triggerExport();
    } else {
      console.warn('[DataView] triggerImageExport called but imageViewRef is missing.');
    }
  }

  function handleRequestInsertAttachedImage(event) {
    const { imagePath } = event.detail;
    console.log('[DataView] Received requestInsertAttachedImage:', imagePath);
    if (
      activeViewType === 'documents' &&
      documentViewRef &&
      typeof documentViewRef.insertImage === 'function'
    ) {
      documentViewRef.insertImage(imagePath);
    } else if (
      activeViewType === 'standalone_transcript' &&
      standaloneTranscriptViewRef &&
      typeof standaloneTranscriptViewRef.insertImage === 'function'
    ) {
      standaloneTranscriptViewRef.insertImage(imagePath);
    } else {
      console.warn('[DataView] Active view does not support inserting images');
    }
  }

  function handleRequestPlayMedia(event) {
    const { mediaPath } = event.detail;
    console.log('[DataView] Received requestPlayMedia:', mediaPath);
    if (activeViewType === 'documents' && documentViewRef) {
      if (typeof documentViewRef.playMedia === 'function') {
        documentViewRef.playMedia(mediaPath);
      } else {
        console.warn('[DataView] documentViewRef.playMedia is not a function');
      }
    } else if (activeViewType === 'standalone_transcript' && standaloneTranscriptViewRef) {
      if (typeof standaloneTranscriptViewRef.playMedia === 'function') {
        standaloneTranscriptViewRef.playMedia(mediaPath);
      } else {
        console.warn('[DataView] standaloneTranscriptViewRef.playMedia is not a function');
      }
    }
  }

  function handleRequestOpenChart(event) {
    const { chart } = event.detail;
    console.log('[DataView] Received requestOpenChart:', chart);
    if (activeViewType === 'tables' && tableViewRef) {
      if (typeof tableViewRef.openChart === 'function') {
        tableViewRef.openChart(chart);
      } else {
        console.warn('[DataView] tableViewRef.openChart is not a function');
      }
    }
  }

  export function handleRequestOpenView(event) {
    const { view } = event.detail;
    console.log('[DataView] Received requestOpenView:', view);

    if (tableViewRef && typeof tableViewRef.openView === 'function') {
      try {
        tableViewRef.openView(view);
      } catch (err) {
        console.error('[DataView] Error opening view in table view:', err);
      }
    } else {
      console.warn('[DataView] tableViewRef or openView method not available yet.');
    }
  }

  function handleRequestConfigureView(event) {
    const { view } = event.detail;
    console.log('[DataView] Received requestConfigureView:', view);

    if (tableViewRef && typeof tableViewRef.configureView === 'function') {
      try {
        tableViewRef.configureView(view);
      } catch (err) {
        console.error('[DataView] Error configuring view in table view:', err);
      }
    } else {
      console.warn('[DataView] tableViewRef or configureView method not available yet.');
    }
  }

  function handleRequestDeleteView(event) {
    const { viewName } = event.detail;
    if (tableViewRef && typeof tableViewRef.handleDeletedView === 'function') {
      tableViewRef.handleDeletedView(viewName);
    }
  }

  export function handleRequestClearSubItem() {
    console.log('[DataView] Received requestClearSubItem');
    activeSubItemPath = null;
    activeSubItemType = null;
    activeItemTypeForInfoPanel = activeViewType;
  }

  export function handleRequestOpenLexicalDocument(event) {
    const { docPath } = event.detail;
    console.log('[DataView] Received requestOpenLexicalDocument:', docPath);

    let retries = 0;
    const tryOpen = () => {
      if (
        activeViewType === 'tables' &&
        tableViewRef &&
        typeof tableViewRef.openLexicalDocument === 'function'
      ) {
        try {
          const success = tableViewRef.openLexicalDocument(docPath);
          if (success !== false) return;
        } catch (err) {
          console.error('[DataView] Error opening lexical document in table view:', err);
          return;
        }
      }

      retries++;
      if (retries < 20) {
        setTimeout(tryOpen, 100);
      } else {
        console.warn(
          '[DataView] tableViewRef or openLexicalDocument method failed to become available after 2s.'
        );
      }
    };
    tryOpen();
  }

  let attachmentsPanelRef;

  function handleAttachmentDeleted(event) {
    const { path } = event.detail;
    if (
      activeViewType === 'documents' &&
      documentViewRef &&
      typeof documentViewRef.handleDeletedAttachment === 'function'
    ) {
      documentViewRef.handleDeletedAttachment(path);
    }
  }

  function handleRequestViewChange(event) {
    if (event.type === 'reset_base') {
      if (attachmentsPanelRef && typeof attachmentsPanelRef.resetSelection === 'function') {
        attachmentsPanelRef.resetSelection();
      }
      return;
    } else if (event.type === 'view_changed' || event.type === 'chart_opened') {
      if (attachmentsPanelRef && typeof attachmentsPanelRef.setSelectionByObject === 'function') {
        attachmentsPanelRef.setSelectionByObject(event.item);
      }
      return;
    }
    handleViewChangeRequest(event);
  }

  const IMAGE_EXTENSIONS_SET = new Set(['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'tiff']);

  project.subscribe((value) => {
    let pathFromStore = null;
    let typeFromStore = 'placeholder';
    let itemTypeForInfo = null;

    if (value.selectedMediaNotePath) {
      pathFromStore = value.selectedMediaNotePath;
      typeFromStore = 'media_note';
      itemTypeForInfo = 'doc'; // Treat the associated transcript as a doc for panels
    } else if (value.currentStandaloneTranscriptPath) {
      pathFromStore = value.currentStandaloneTranscriptPath;
      typeFromStore = 'standalone_transcript';
      itemTypeForInfo = 'standalone_transcript';
    } else if (value.selectedDocumentPath) {
      pathFromStore = value.selectedDocumentPath;
      const lowerPath = pathFromStore.toLowerCase();
      const extension = lowerPath.split('.').pop();

      if (
        lowerPath.endsWith('.pdf') ||
        (lowerPath.endsWith('.json') &&
          (!value.standaloneTranscriptFiles ||
            value.standaloneTranscriptFiles.every(
              (f) => normalizePath(`${value.baseDirectory}/${f.relativePath}`) !== pathFromStore
            )) &&
          !value.selectedMediaNotePath) ||
        lowerPath.endsWith('.txt') ||
        lowerPath.endsWith('.md')
      ) {
        typeFromStore = 'documents';
        itemTypeForInfo = 'doc'; // Matching what LeftInfoPanel used
      } else if (lowerPath.endsWith('.csv') || lowerPath.endsWith('.xlsx')) {
        typeFromStore = 'tables';
        itemTypeForInfo = 'table';
      } else if (IMAGE_EXTENSIONS_SET.has(extension)) {
        typeFromStore = 'images';
        itemTypeForInfo = 'images';
      } else {
        console.warn(
          `[DataView Store Sub] Path ${pathFromStore} (from selectedDocumentPath) has undetermined type.`
        );
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
      console.debug(
        `[DataView Store Sub] Synced. Path: ${activeItemPath}, ViewType: ${activeViewType}, InfoPanelType: ${activeItemTypeForInfoPanel}`
      );
    } else if (activeItemTypeForInfoPanel !== itemTypeForInfo) {
      // Path and view type might be same, but specific type for info panel changed (e.g. media_note to audio)
      activeItemTypeForInfoPanel = itemTypeForInfo;
      console.debug(`[DataView Store Sub] InfoPanelType updated to: ${activeItemTypeForInfoPanel}`);
    }

    if (
      itemTypeForInfo !== 'doc' &&
      itemTypeForInfo !== 'standalone_transcript' &&
      itemTypeForInfo !== 'table' &&
      panelStateStore.activeInfoPanelTab === 'attachments'
    ) {
      panelStateStore.setActiveInfoPanelTab('metadata');
    }
  });

  async function handleViewChangeRequest(eventDetailFromDispatch) {
    if (eventDetailFromDispatch?.type === 'refresh_metadata') {
      infoPanelRefreshKey++;
      highlightsPanelRefreshKey++;
      return;
    }
    const pathForView = eventDetailFromDispatch?.itemPath || eventDetailFromDispatch?.loadNotePath;
    const typeForView = eventDetailFromDispatch?.viewType;
    const hasHeadersForView = eventDetailFromDispatch?.hasHeaders;

    console.debug(
      `[DataView] Received requestviewchange. Path: ${pathForView}, Type: ${typeForView}`
    );

    const attachmentToOpen = eventDetailFromDispatch?.attachmentToOpen;

    // If the requested path and type are already active, do nothing.
    if (pathForView === activeItemPath && typeForView === activeViewType && !attachmentToOpen) {
      console.debug(
        `[DataView] Requested view change to already active item. Path: ${pathForView}, Type: ${typeForView}. Aborting redundant action.`
      );
      return;
    }

    if (!pathForView || !typeForView || typeForView === 'placeholder') {
      console.error(
        `[DataView] ABORTING: Invalid path or type. Path: '${pathForView}', Type: '${typeForView}'.`
      );
      return;
    }

    // Check if this is a media transcript that needs special handling
    const originalDocType = eventDetailFromDispatch?.originalDocType;
    // If it's explicitly an audio/video transcript (i.e. from the table link), it MUST go to ProjectView.
    // However, standard media_note clicks from the left panel should just be handled directly here
    // using prepareMediaNoteView unless it's a specific table intercept routing.
    if (
      originalDocType === 'audio_transcript' ||
      originalDocType === 'video_transcript' ||
      typeForView === 'media'
    ) {
      console.debug(
        `[DataView] Detected complex media/transcript link type, forwarding to ProjectView for parent resolution.`
      );
      // Inject tabName and loadNotePath so ProjectView's handleRequestOpenTab doesn't early return
      forwardEvent({
        type: 'requestviewchange',
        detail: { ...eventDetailFromDispatch, tabName: 'data', loadNotePath: pathForView }
      });
      return;
    }

    const canProceed = await checkUnsavedChangesThenProceed(pathForView, typeForView);
    if (!canProceed) {
      console.info('[DataView] View change cancelled.');
      return;
    }

    console.debug(
      `[DataView] Proceeding with view change - Path: ${pathForView}, Type: ${typeForView}`
    );

    const typeForStore =
      typeForView === 'document' || typeForView === 'documents'
        ? 'documents'
        : typeForView === 'table' || typeForView === 'tables'
          ? 'tables'
          : typeForView === 'image' || typeForView === 'images'
            ? 'images'
            : null;

    if (typeForStore) {
      prepareDocumentView(
        pathForView,
        typeForStore,
        hasHeadersForView !== undefined ? hasHeadersForView : true
      );
    } else if (typeForView === 'standalone_transcript') {
      prepareStandaloneTranscriptView(pathForView);
    } else if (typeForView === 'media_note') {
      prepareMediaNoteView(pathForView);
    } else {
      console.warn(`[DataView] Unknown typeForView: '${typeForView}'. Clearing views.`);
      prepareDocumentView(null, 'placeholder');
      prepareStandaloneTranscriptView(null);
      prepareMediaNoteView(null);
      activeItemTypeForInfoPanel = null;
    }
    console.debug(
      `[DataView] Store preparation actions dispatched for Path: ${pathForView}, Type: ${typeForView}.`
    );

    if (attachmentToOpen) {
      // Wait for Svelte to fully destroy old components and mount new ones based on activeItemPath change
      await tick();

      // Open the attachments panel
      panelStateStore.setActiveInfoPanelTab('attachments');

      handleRequestOpenLexicalDocument({ detail: { docPath: attachmentToOpen } });
    }
  }

  function handleRightBarTabChange(event) {
    const { tabName } = event.detail;
    // panelStateStore.setActiveInfoPanelTab(tabName); // This is already done in RightBar.svelte
    console.log(`[DataView] RightBar tab changed to: ${tabName}`);
    // InfoPanel will react to panelStateStore.activeInfoPanelTab
  }

  import { listen } from '@tauri-apps/api/event';

  let infoPanelRefreshKey = null;
  let highlightsPanelRefreshKey = Date.now();

  let unsubscribeRefresher;

  onMount(async () => {
    unsubscribeRefresher = refresher.subscribe(() => {
      highlightsPanelRefreshKey = Date.now();
    });

    const unlisten = await listen('metadata_updated', (event) => {
      console.log(`[DataView] Received metadata_updated event for path:`, event.payload);
      // Check if the updated item is the one currently being viewed.
      // The payload is the relative path of the asset.
      if (event.payload && event.payload === activeItemPath) {
        console.log(`[DataView] Refreshing InfoPanel for ${activeItemPath}`);
        infoPanelRefreshKey = Date.now();
      }
    });

    return () => {
      unlisten();
      if (unsubscribeRefresher) {
        unsubscribeRefresher();
      }
    };
  });
</script>

<div class="flex flex-col h-full w-full bg-gray-100 dark:bg-gray-950 overflow-hidden">
  <div class="flex flex-grow w-full min-h-0">
    <!-- Far Left Panel (File/Data Browser) -->
    <div
      class="{panelStateStore.dataLeftPanelCollapsed
        ? 'w-12'
        : 'w-64'} h-full flex-shrink-0 transition-all duration-300 ease-in-out"
    >
      <DataLeftPanel
        on:requestmediaselection={forwardEvent}
        on:requestviewchange={(event) => handleViewChangeRequest(event.detail)}
        on:requestTranscriptionTabWithMediaAndDialog={forwardEvent}
        on:requestTranslationTabWithMediaAndDialog={forwardEvent}
      />
    </div>

    <!-- Main Content Area (Middle) -->
    <div class="flex-grow h-full min-w-0 border-l border-gray-300 dark:border-gray-700">
      {#key activeItemPath + activeViewType}
        {#if activeViewType === 'placeholder' || !activeItemPath}
          <div class="h-full flex items-center justify-center text-gray-500 dark:text-gray-400 p-4">
            <div class="flex flex-col items-center justify-center space-y-3 text-center">
              <h3 class="text-xl font-semibold text-gray-800 dark:text-gray-100">Get started with your project</h3>
              <p class="text-[15px] text-gray-600 dark:text-gray-400 leading-relaxed">
                Import your media, documents, and tables using the <span class="inline-flex items-center justify-center w-6 h-6 rounded-full bg-blue-600 text-white mx-1.5 shadow-sm align-middle"><Plus class="w-4 h-4" strokeWidth={2.5} /></span> <strong>plus button</strong> on the left sidebar to begin working with your data locally.
              </p>
              <p class="text-[15px] text-gray-600 dark:text-gray-400 leading-relaxed mt-2">
                If you need help, look for the <span class="inline-flex items-center justify-center w-7 h-7 rounded-md text-gray-500 mx-1 align-middle"><svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" class="w-5 h-5 bi bi-question-circle" viewBox="0 0 16 16"><path d="M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14zm0 1A8 8 0 1 0 8 0a8 8 0 0 0 0 16z"/><path d="M5.255 5.786a.237.237 0 0 0 .241.247h.825c.138 0 .248-.113.266-.25.09-.656.54-1.134 1.342-1.134.686 0 1.314.343 1.314 1.168 0 .635-.374.927-.965 1.371-.673.489-1.206 1.06-1.168 1.987l.003.217a.25.25 0 0 0 .25.246h.811a.25.25 0 0 0 .25-.25v-.105c0-.718.273-.927 1.01-1.486.609-.463 1.244-.977 1.244-2.056 0-1.511-1.276-2.241-2.673-2.241-1.267 0-2.655.59-2.75 2.286zm1.557 5.763c0 .533.425.927 1.01.927.609 0 1.028-.394 1.028-.927 0-.552-.42-.94-1.029-.94-.584 0-1.009.388-1.009.94z"/></svg></span> <strong>help button</strong> on the bottom left.
              </p>
            </div>
          </div>
        {:else if activeViewType === 'documents'}
          <DocumentView bind:this={documentViewRef} itemPath={activeItemPath} />
        {:else if activeViewType === 'tables'}
          <TableView
            bind:this={tableViewRef}
            itemPath={activeItemPath}
            hasHeaders={$project.selectedDocumentOptions.hasHeaders}
            bind:activeSubItemPath
            bind:activeSubItemType
            on:requestviewchange={(event) => handleRequestViewChange(event.detail)}
          />
        {:else if activeViewType === 'images'}
          <ImageView bind:this={imageViewRef} itemPath={activeItemPath} />
        {:else if activeViewType === 'standalone_transcript'}
          <StandaloneTranscriptView
            bind:this={standaloneTranscriptViewRef}
            itemPath={activeItemPath}
          />
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
          <div class="h-full flex items-center justify-center text-gray-500 dark:text-gray-400">
            <span>Selected view type '{activeViewType}' not recognized or item path invalid.</span>
          </div>
        {/if}
      {/key}
    </div>

    <!-- New Info Panel (Right of Main Content, Left of RightBar) -->
    {#if !panelStateStore.infoPanelCollapsed && activeItemPath && activeViewType !== 'group_detail'}
      <div
        class="w-[20.588%] h-full flex-shrink-0 transition-all duration-300 ease-in-out border-l border-gray-300 dark:border-gray-700"
        transition:slide={{ duration: 300, axis: 'x' }}
      >
        {#if panelStateStore.activeInfoPanelTab === 'metadata'}
          <InfoPanel
            itemPath={activeItemPath}
            itemType={activeItemTypeForInfoPanel}
            refreshKey={infoPanelRefreshKey}
          />
        {:else if panelStateStore.activeInfoPanelTab === 'highlights'}
          <HighlightsPanel
            itemPath={activeSubItemPath || activeItemPath}
            itemType={activeSubItemType || activeItemTypeForInfoPanel}
            refreshKey={highlightsPanelRefreshKey}
          />
        {:else if panelStateStore.activeInfoPanelTab === 'attachments'}
          <AttachmentsPanel
            bind:this={attachmentsPanelRef}
            itemPath={activeItemPath}
            itemType={activeItemTypeForInfoPanel}
            refreshKey={infoPanelRefreshKey}
            on:requestInsertAttachedImage={handleRequestInsertAttachedImage}
            on:requestPlayMedia={handleRequestPlayMedia}
            on:requestOpenChart={handleRequestOpenChart}
            on:requestOpenView={handleRequestOpenView}
            on:requestConfigureView={handleRequestConfigureView}
            on:requestDeleteView={handleRequestDeleteView}
            on:requestOpenLexicalDocument={handleRequestOpenLexicalDocument}
            on:attachmentdeleted={handleAttachmentDeleted}
          />
        {/if}
      </div>
    {/if}
    <!-- Consider adding a toggle button for infoPanelCollapsed if needed, or manage via RightBar interaction -->

    <!-- New Right Bar (Far Right) -->
    {#if activeViewType !== 'group_detail' && activeItemPath}
      <div class="h-full flex-shrink-0">
        <RightBar on:tabchange={handleRightBarTabChange} itemType={activeItemTypeForInfoPanel} />
      </div>
    {/if}
  </div>
</div>

<style>
  .min-h-0 {
    min-height: 0;
  }
  .w-\[20\.588\%\] {
    width: 20.58825%;
  } /* For new InfoPanel, same as old LeftInfoPanel width */
</style>
