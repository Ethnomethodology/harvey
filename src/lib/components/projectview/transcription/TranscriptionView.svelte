<!-- src/lib/components/projectview/transcription/TranscriptionView.svelte -->
<script>
  import { tick, createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import { slide } from 'svelte/transition';
  import { project } from '$lib/stores/projectStore.js';
  import {
    transcriptStore,
    deleteTranscriptSegment,
    undoTranscriptChange,
    redoTranscriptChange,
    insertTranscriptSegment,
    splitTranscriptSegment,
    mergeTranscriptSegments,
    selectMedia,
    markTranscriptAsSaved,
    deactivateDualMode,
    updatePlayerCurrentSegmentIndex,
    updateManualSegmentSettings
  } from '$lib/stores/transcriptStore.js';
  import {
    saveTranscriptData,
    requestTranscription as requestTranscriptionService,
    convertAndSaveTranscriptAsDoc,
    loadTranscriptFile,
    replaceTranscriptText,
    replaceAllTranscriptText
  } from '$lib/services/projectService.js';
  import { mediaEditorStore } from '$lib/stores/mediaEditorStore.svelte.js';

  import { confirm, message } from '@tauri-apps/plugin-dialog';

  import TopBar from './TopBar.svelte';
  import LeftPanel from './LeftPanel.svelte';
  import panelStateStore from '$lib/stores/panelStateStore.svelte.js';
  import waveformLayoutStore from '$lib/stores/waveformLayoutStore.js';
  import MediaPlayer from '../shared/MediaPlayer.svelte';
  import InteractiveWaveform from '../shared/InteractiveWaveform.svelte';
  import VerticalWaveform from './VerticalWaveform.svelte';
  import EditableTranscript from './EditableTranscript.svelte';
  import RichTextPreview from './RichTextPreview.svelte';
  import UnsavedChangesModal from '$lib/components/projectview/modals/UnsavedChangesModal.svelte';
  import ManualSettingsModal from '$lib/components/projectview/modals/ManualSettingsModal.svelte';
  import { getErrorMessage } from '$lib/utils/errorUtils.js';

  const dispatch = createEventDispatcher();

  // --- Props (Runes) ---
  let { mediaPlayerRef = $bindable(null) } = $props();

  // --- Internal State (Runes) ---
  let verticalWaveformRef = $state(null);
  let horizontalWaveformRef = $state(null);
  let verticalWaveformWidthPx = $state(0);
  const HORIZONTAL_WAVEFORM_DEFAULT_HEIGHT_PX = 75;
  let horizontalWaveformContainerHeightPx = $state(HORIZONTAL_WAVEFORM_DEFAULT_HEIGHT_PX);

  let editableTranscriptRef = $state();
  let richTextPreviewRef = $state();
  let topBarRef = $state();
  let leftPanelRef = $state();
  let lastNavigateClickRatio = $state(0.5);

  let currentWaveformLayout = $state();
  let isMediaPlayerTrimming = $state(false);
  let mediaPlayerTrimStart = $state(0);
  let mediaPlayerTrimEnd = $state(0);
  let isMediaPlayerHidden = $state(false);
  let isSegmentEditingActive = $state(false);
  let currentEditSegmentStart = $state(0);
  let currentEditSegmentEnd = $state(0);
  let wasPlayingBeforeEdit = $state(false);
  let isManualSettingsModalOpen = $state(false);
  let lastCenterScrollIndex = $state(-1);
  let showConfirmConversionModal = $state(false);

  // --- Derived State (Runes) ---
  const isVideoMedia = $derived((() => {
    const selectedMedia = $transcriptStore.selectedMediaFile;
    if (selectedMedia && selectedMedia.path) {
      const extension = selectedMedia.path.split('.').pop()?.toLowerCase();
      const videoExtensions = ['mp4', 'mov', 'webm', 'avi', 'mkv'];
      return videoExtensions.includes(extension);
    }
    return false;
  })());

  const isAudioMedia = $derived((() => {
    const selectedMedia = $transcriptStore.selectedMediaFile;
    if (selectedMedia && selectedMedia.path) {
      const extension = selectedMedia.path.split('.').pop()?.toLowerCase();
      const audioExtensions = ['mp3', 'wav', 'm4a', 'ogg', 'flac', 'aac'];
      return audioExtensions.includes(extension);
    }
    return false;
  })());

  const panelEditModeActive = $derived(mediaEditorStore.isLexicalEditMode);

  const middlePanelWidthClass = $derived((() => {
    if (currentWaveformLayout === 'vertical') {
      return !panelStateStore.transcriptionPanelCollapsed ? 'w-[40%]' : 'w-[47.5%]';
    } else {
      return !panelStateStore.transcriptionPanelCollapsed ? 'w-[42.5%]' : 'w-[50%]';
    }
  })());

  const rightPanelWidthClass = $derived((() => {
    if (currentWaveformLayout === 'vertical') {
      return !panelStateStore.transcriptionPanelCollapsed ? 'w-[40%]' : 'w-[47.5%]';
    } else {
      return !panelStateStore.transcriptionPanelCollapsed ? 'w-[42.5%]' : 'w-[50%]';
    }
  })());

  // --- Effects (Runes) ---
  $effect(() => {
    if (currentWaveformLayout === 'vertical') {
      horizontalWaveformContainerHeightPx = verticalWaveformWidthPx;
    } else if (currentWaveformLayout === 'horizontal') {
      horizontalWaveformContainerHeightPx = HORIZONTAL_WAVEFORM_DEFAULT_HEIGHT_PX;
    } else {
      horizontalWaveformContainerHeightPx = 0;
    }
  });

  $effect(() => {
    if (isVideoMedia) {
      isMediaPlayerHidden = false;
    }
  });

  // --- Subscriptions ---
  let unsubscribeWaveformLayout;
  onMount(() => {
    unsubscribeWaveformLayout = waveformLayoutStore.subscribe((value) => {
      currentWaveformLayout = value;
      console.log('[TranscriptionView] currentWaveformLayout updated to:', currentWaveformLayout);
    });
  });

  onDestroy(() => {
    if (unsubscribeWaveformLayout) unsubscribeWaveformLayout();
  });

  async function handlePreviousRequest() {
    editableTranscriptRef?.previous();
  }

  async function handleNextRequest() {
    editableTranscriptRef?.next();
  }

  async function handleSegmentClick(event) {
    const index = event.detail;
    const segment = get(transcriptStore).segments?.[index];
    if (segment && typeof segment.start_time === 'number') {
      if (panelEditModeActive && editableTranscriptRef) {
        editableTranscriptRef.commitCurrentSegmentEdits();
        await tick();
      }
      updatePlayerCurrentSegmentIndex(index);
      editableTranscriptRef?.loadSegment?.(index);
    }
  }

  async function handlePanelNavigate(event) {
    const detail = event.detail;
    if (panelEditModeActive && editableTranscriptRef) {
      editableTranscriptRef.commitCurrentSegmentEdits();
      await tick();
    }

    if (detail && detail.action) {
      if (mediaPlayerRef) {
        switch (detail.action) {
          case 'toggle-play': mediaPlayerRef.handleTogglePlay(); break;
          case 'rewind': mediaPlayerRef.rewind10s(); break;
          case 'forward': mediaPlayerRef.forward10s(); break;
          case 'speed-up': mediaPlayerRef.changeSpeed(1); break;
          case 'speed-down': mediaPlayerRef.changeSpeed(-1); break;
        }
      }
      return;
    }

    if (detail && typeof detail.time === 'number') {
      lastNavigateClickRatio = detail.ratio ?? 0.5;
      if (mediaPlayerRef) mediaPlayerRef.seekTo(detail.time);
      setTimeout(() => { lastNavigateClickRatio = 0.5; }, 500);
    } else if (detail && typeof detail.index === 'number') {
      const index = detail.index;
      const segment = get(transcriptStore).segments?.[index];
      if (segment) {
        currentEditSegmentStart = segment.start_time;
        currentEditSegmentEnd = segment.end_time;
        updatePlayerCurrentSegmentIndex(index);
        if (mediaPlayerRef) {
          const seekTime = segment.start_time;
          mediaPlayerRef.seekTo(seekTime, index);
          verticalWaveformRef?.scrollToTime(seekTime);
          horizontalWaveformRef?.scrollToTime(seekTime);
        }
      }
    }
  }

  function handleGlobalKeydown(event) {
    const isMac = typeof window !== 'undefined' && navigator.platform.toUpperCase().indexOf('MAC') >= 0;
    const modKey = isMac ? event.metaKey : event.ctrlKey;
    if (modKey && event.key.toLowerCase() === 'e') {
      event.preventDefault();
      handleToggleEditMode();
    }
  }

  function handleMediaPlayerTrimModeEntered(event) {
    isMediaPlayerTrimming = true;
    mediaPlayerTrimStart = event.detail.startTime;
    mediaPlayerTrimEnd = event.detail.endTime;
  }

  function handleMediaPlayerTrimModeCancelled() {
    isMediaPlayerTrimming = false;
  }

  function handleWaveformTrimUpdate(event) {
    const { startTime, endTime } = event.detail;
    if (mediaPlayerRef) mediaPlayerRef.updateTrimTimes(startTime, endTime);
    mediaPlayerTrimStart = startTime;
    mediaPlayerTrimEnd = endTime;
  }

  function handleSegmentEditFocus(event) {
    const { isEditing, startTime, endTime } = event.detail;
    isSegmentEditingActive = isEditing;
    currentEditSegmentStart = startTime ?? 0;
    currentEditSegmentEnd = endTime ?? 0;
  }

  function handleWaveformSegmentUpdate(event) {
    const { startTime, endTime } = event.detail;
    if (editableTranscriptRef) {
      editableTranscriptRef.updateTimesFromExternal(startTime, endTime);
    }
  }

  export async function handleToggleEditMode() {
    if (panelEditModeActive) {
      await handleSaveTranscript(true);
      mediaEditorStore.isLexicalEditMode = false;
    } else {
      mediaEditorStore.isLexicalEditMode = true;
      await tick();
      editableTranscriptRef?.focusEditor?.();
    }
  }

  export async function exitEditModeIfActive() {
    if (panelEditModeActive || get(transcriptStore).transcriptDirty) {
      await handleSaveTranscript();
    }
  }

  export async function enterManualEditMode() {
    mediaEditorStore.isLexicalEditMode = true;
    updatePlayerCurrentSegmentIndex(0);
    await tick();
    const store = get(transcriptStore);
    if (store.segments && store.segments.length > 0) {
      setTimeout(async () => {
        await handleSegmentClick({ detail: 0 });
        editableTranscriptRef?.focusEditor?.();
      }, 100);
    }
  }

  export async function handleSaveTranscript(isAutoSave = false) {
    if (panelEditModeActive && editableTranscriptRef) {
      editableTranscriptRef.commitCurrentSegmentEdits();
      await tick();
    }
    if (!get(transcriptStore).transcriptDirty) return;
    try {
      project.update(p => ({ ...p, isLoading: true, statusMessage: 'Saving transcript...' }));
      await saveTranscriptData();
      project.update(p => ({ ...p, isLoading: false, statusMessage: 'Transcript saved.' }));
    } catch (error) {
      const errorMsg = getErrorMessage(error);
      project.update(p => ({ ...p, isLoading: false, error: `Save failed: ${errorMsg}`, statusMessage: 'Save failed.' }));
      await message(`Error saving transcript: ${errorMsg}`, { title: 'Save Error', type: 'error' });
      throw error;
    }
  }

  function handleRequestTranscriptionEvent() {
    requestTranscriptionService();
  }

  async function handleConvertToDocumentEvent() {
    if (get(transcriptStore).transcriptDirty) await handleSaveTranscript();
    showConfirmConversionModal = true;
  }

  async function confirmConvertToDocument() {
    try {
      project.update(p => ({ ...p, statusMessage: 'Converting to document...' }));
      const newDocPath = await convertAndSaveTranscriptAsDoc();
      project.update(p => ({ ...p, statusMessage: 'Converted to document successfully.' }));
      await message(`Transcript converted and saved as a new document.`, { title: 'Conversion Successful' });
      dispatch('requestopentab', { tabName: 'data', loadNotePath: newDocPath });
    } catch (error) {
      const errorMsg = getErrorMessage(error);
      await message(`Failed to convert: ${errorMsg}`, { title: 'Conversion Error', type: 'error' });
      project.update(p => ({ ...p, statusMessage: 'Conversion failed.' }));
    } finally {
      showConfirmConversionModal = false;
    }
  }

  export function handleDeleteSegmentRequest(event) {
    const indexToDelete = event.detail;
    if (typeof indexToDelete === 'number') deleteTranscriptSegment(indexToDelete);
  }
  export function handleSplitSegmentRequest(event) {
    const indexToSplit = event.detail;
    if (typeof indexToSplit === 'number') splitTranscriptSegment(indexToSplit);
  }
  export function handleMergeSegmentRequest(event) {
    const indexToMerge = event.detail;
    if (typeof indexToMerge === 'number') mergeTranscriptSegments(indexToMerge);
  }
  export function handleUndoRequest() {
    undoTranscriptChange();
    editableTranscriptRef?.forceReloadFromStore?.();
  }
  export function handleRedoRequest() {
    redoTranscriptChange();
    editableTranscriptRef?.forceReloadFromStore?.();
  }
  export function handleInsertSegmentRequest(event) {
    if (typeof event.detail === 'number') {
      const index = event.detail;
      richTextPreviewRef?.handleInsertNewSegment?.(index + 1);
      return;
    }
    const { index, startTime, endTime, speaker } = event.detail || {};
    if (typeof index !== 'number' || typeof startTime !== 'number' || typeof endTime !== 'number' || endTime <= startTime) return;
    const newSegment = {
      start_time: startTime, end_time: endTime, speaker: speaker || 'Unknown',
      text: JSON.stringify({ root: { children: [{ type: 'paragraph', version: 1, children: [], direction: null, format: '', indent: 0 }], type: 'root', version: 1, direction: null, format: '', indent: 0 } })
    };
    insertTranscriptSegment(index, newSegment);
  }

  export function activateTrimModeOnPlayer() {
    if (mediaPlayerRef?.enterTrimMode) {
      mediaPlayerRef.enterTrimMode();
      isMediaPlayerTrimming = true;
    }
  }

  $effect(() => {
    const curIdx = $transcriptStore.player.currentSegmentIndex;
    const totalSegs = $transcriptStore.segments?.length || 0;
    if (curIdx !== lastCenterScrollIndex && curIdx >= 0 && curIdx < totalSegs) {
      const isPlaying = $transcriptStore.player.isPlaying;
      const isSignificantJump = lastCenterScrollIndex !== -1 && Math.abs(curIdx - lastCenterScrollIndex) > 1;
      if (!isPlaying || isSignificantJump) {
        const segment = $transcriptStore.segments[curIdx];
        if (segment) {
          const scrollTime = lastNavigateClickRatio !== 0.5 ? $transcriptStore.player.currentTime || segment.start_time : segment.start_time;
          verticalWaveformRef?.scrollToTime(scrollTime, lastNavigateClickRatio);
          horizontalWaveformRef?.scrollToTime(scrollTime, lastNavigateClickRatio);
        }
      }
      lastCenterScrollIndex = curIdx;
    } else if (curIdx === -1) {
      lastCenterScrollIndex = -1;
    }
  });

  $effect(() => {
    if (mediaPlayerRef?.videoElement) {
      const video = mediaPlayerRef.videoElement;
      if (isSegmentEditingActive) {
        if (!wasPlayingBeforeEdit && !video.paused) {
          wasPlayingBeforeEdit = true;
          try { video.pause(); } catch (e) {}
        }
      } else {
        if (wasPlayingBeforeEdit) {
          if (video.paused) { try { video.play().catch(() => {}); } catch (e) {} }
          wasPlayingBeforeEdit = false;
        }
      }
    }
  });

  function forwardLeftPanelEvents(event) {
    dispatch(event.type, event.detail);
  }

  function handleRequestManualSettings() {
    isManualSettingsModalOpen = true;
  }

  function handleManualSettingsConfirm(event) {
    const { duration, speakerMode } = event.detail;
    updateManualSegmentSettings({ duration, speakerMode });
  }

  function handleReplaceTranscriptText(event) {
    const { segmentIndex, isPrimary, find, replace, offset, length } = event.detail;
    replaceTranscriptText(segmentIndex, isPrimary, find, replace, offset, length);
  }

  function handleReplaceAllTranscriptText(event) {
    const { find, replace, isCaseSensitive, isRegex, isWholeWord } = event.detail;
    replaceAllTranscriptText(find, replace, { isCaseSensitive, isRegex, isWholeWord });
  }

  function getStemRelPath(relPath) {
    if (!relPath) return '';
    const parts = relPath.replace(/\\/g, '/').split('/');
    return parts.length >= 3 ? parts.slice(0, parts.length - 2).join('/') : '';
  }

  function findMediaByTranscriptRelativePath(transcriptRelativePath, projectFiles) {
    if (!projectFiles || !transcriptRelativePath) return null;
    const targetStemPath = getStemRelPath(transcriptRelativePath);
    if (!targetStemPath) return null;
    function recurse(nodes) {
      for (const node of nodes) {
        if (node.file_type === 'media' && node.relative_path) {
          if (getStemRelPath(node.relative_path) === targetStemPath) return node;
        }
        if (node.children) { const found = recurse(node.children); if (found) return found; }
      }
      return null;
    }
    return recurse(projectFiles);
  }

  async function handleRequestLoadItem(event) {
    if (panelEditModeActive && editableTranscriptRef) {
      editableTranscriptRef.commitCurrentSegmentEdits();
      await tick();
    }
    if (get(transcriptStore).transcriptDirty) await handleSaveTranscript(true);
    await loadRequestedItem(event.detail);
  }

  async function loadRequestedItem(item) {
    const store = get(transcriptStore);
    if (item.file_type === 'media') {
      selectMedia(item);
    } else if (item.file_type.includes('transcript')) {
      if (store.isDualModeActive) await deactivateDualMode();
      const associatedMedia = findMediaByTranscriptRelativePath(item.relative_path, get(project).files);
      selectMedia(associatedMedia, item.path);
      try { await loadTranscriptFile(item.path); } catch (error) {
        const errorMsg = getErrorMessage(error);
        message(`Error loading transcript: ${errorMsg}`, { title: 'Load Error', type: 'error' });
      }
    }
  }
</script>


<svelte:window on:keydown={handleGlobalKeydown} />

<div class="flex flex-col h-full w-full">
  <div class="flex flex-col flex-grow min-h-0 w-full">
    <!-- Main Content Area (Panels) -->
    <div class="flex flex-grow min-h-0 w-full overflow-x-hidden">
      {#if !panelStateStore.transcriptionPanelCollapsed}
        <div
          class="w-64 h-full bg-white dark:bg-gray-900 overflow-y-auto flex-shrink-0 transition-all duration-300 ease-in-out"
          transition:slide={{ duration: 300, axis: 'x' }}
        >
          <LeftPanel
            bind:this={leftPanelRef}
            on:requestopentab={forwardLeftPanelEvents}
            on:requestmediaselection={forwardLeftPanelEvents}
            on:requestLoadItem={handleRequestLoadItem}
          />
        </div>
      {/if}

      <!-- Middle Panel: MediaPlayer and EditableTranscript -->
      <div
        class="{middlePanelWidthClass} h-full flex flex-col transition-all duration-300 ease-in-out border-l border-gray-300 dark:border-gray-700"
      >
        <div
          class="bg-white dark:bg-gray-950 flex flex-col flex-shrink-0 {isVideoMedia &&
          !isMediaPlayerHidden
            ? $transcriptStore.englishSegments &&
              $transcriptStore.englishSegments.length > 0 &&
              $transcriptStore.originalSegments &&
              $transcriptStore.originalSegments.length > 0
              ? 'h-[calc(50%-1.75rem)]'
              : 'h-1/2'
            : 'h-[64px]'}"
        >
          <MediaPlayer
            bind:this={mediaPlayerRef}
            bind:isTrimming={isMediaPlayerTrimming}
            bind:trimStartTime={mediaPlayerTrimStart}
            bind:trimEndTime={mediaPlayerTrimEnd}
            bind:isEditingSegment={isSegmentEditingActive}
            bind:editSegmentStartTime={currentEditSegmentStart}
            bind:editSegmentEndTime={currentEditSegmentEnd}
            projectId={$project.id}
            xmlPath={$project.xmlPath}
            bind:isVideoMinimized={isMediaPlayerHidden}
            showLoopPauseButton={true}
            showDataTranscribeButton={false}
            showDataTrimButton={false}
            showMainTrimButton={false}
            on:trimModeEntered={handleMediaPlayerTrimModeEntered}
            on:trimModeCancelled={handleMediaPlayerTrimModeCancelled}
          />
        </div>
        <div
          class="flex-grow min-h-0 bg-white dark:bg-gray-950 overflow-y-auto border-t border-gray-300 dark:border-gray-700"
        >
          <EditableTranscript
            bind:this={editableTranscriptRef}
            panelEditMode={panelEditModeActive}
            on:navigate={handlePanelNavigate}
            on:segmenteditfocus={handleSegmentEditFocus}
            on:toggleedit={handleToggleEditMode}
            on:previous={handlePreviousRequest}
            on:next={handleNextRequest}
            on:insertnewsegment={handleInsertSegmentRequest}
          />
        </div>
      </div>

      <!-- Vertical Waveform Panel (Conditional) -->
      {#if currentWaveformLayout === 'vertical'}
        <div
          bind:clientWidth={verticalWaveformWidthPx}
          class="w-16 h-full flex-shrink-0 transition-all duration-300 ease-in-out border-l border-gray-300 dark:border-gray-700"
        >
          {#if $transcriptStore.selectedMediaFile && ($transcriptStore.audioBuffer || $transcriptStore.audioBufferPeaks)}
            <VerticalWaveform
              bind:this={verticalWaveformRef}
              audioBuffer={$transcriptStore.audioBuffer}
              externalPeaks={$transcriptStore.audioBufferPeaks}
              currentTime={$transcriptStore.player.currentTime}
              duration={$transcriptStore.player.duration}
              isEditingSegment={isSegmentEditingActive}
              editSegmentStartTime={currentEditSegmentStart}
              editSegmentEndTime={currentEditSegmentEnd}
              on:navigate={handlePanelNavigate}
              on:segmentupdate={handleWaveformSegmentUpdate}
            />
          {:else if $transcriptStore.selectedMediaFile}
            <div
              class="flex items-center justify-center h-full text-xs text-gray-400 dark:text-gray-700 bg-white dark:bg-gray-950 p-1"
            >
              Waveform still loading...
            </div>
          {:else}
            <div
              class="flex items-center justify-center h-full text-xs text-gray-400 dark:text-gray-700 bg-white dark:bg-gray-950 p-1"
            >
              Select media.
            </div>
          {/if}
        </div>
      {/if}

      <!-- Right Panel: RichTextPreview -->
      <div
        class="{rightPanelWidthClass} h-full bg-white dark:bg-gray-950 overflow-y-auto transition-all duration-300 ease-in-out flex flex-col border-l border-gray-300 dark:border-gray-700"
      >
        <RichTextPreview
          bind:this={richTextPreviewRef}
          previewEditMode={panelEditModeActive}
          on:segmentclick={handleSegmentClick}
          on:toggleedit={handleToggleEditMode}
          on:requestopentab={(e) => dispatch('requestopentab', e.detail)}
          on:deletetranscriptsegment={handleDeleteSegmentRequest}
          on:splittranscriptsegment={handleSplitSegmentRequest}
          on:mergetranscriptsegment={handleMergeSegmentRequest}
          on:insertnewsegment={handleInsertSegmentRequest}
          on:undo={handleUndoRequest}
          on:redo={handleRedoRequest}
          on:convertToDocument={handleConvertToDocumentEvent}
          on:requestmanualsettings={handleRequestManualSettings}
          on:playsegment={(e) => {
            const segment = $transcriptStore.segments?.[e.detail];
            if (segment && mediaPlayerRef) {
              mediaPlayerRef.playSegment(segment.start_time, segment.end_time);
            }
          }}
          on:replacetranscripttext={handleReplaceTranscriptText}
          on:replacealltranscripttext={handleReplaceAllTranscriptText}
        />
      </div>
    </div>
  </div>

  <!-- Horizontal Waveform Panel (Conditional) -->
  {#if currentWaveformLayout === 'horizontal'}
    <div
      style="height: {horizontalWaveformContainerHeightPx}px;"
      class="border-t border-gray-200 dark:border-gray-700 relative z-10 overflow-visible"
    >
      {#if $transcriptStore.selectedMediaFile && ($transcriptStore.audioBuffer || $transcriptStore.audioBufferPeaks)}
        <InteractiveWaveform
          bind:this={horizontalWaveformRef}
          externalAudioBuffer={$transcriptStore.audioBuffer}
          externalCurrentTime={$transcriptStore.player.currentTime}
          externalDuration={$transcriptStore.player.duration}
          externalSegments={$transcriptStore.segments}
          externalCurrentSegmentIndex={$transcriptStore.player.currentSegmentIndex}
          isEditingSegment={isSegmentEditingActive}
          editSegmentStartTime={currentEditSegmentStart}
          editSegmentEndTime={currentEditSegmentEnd}
          showTrimUI={panelEditModeActive}
          fixedHeightPx={horizontalWaveformContainerHeightPx}
          compactMode={false}
          on:navigate={handlePanelNavigate}
          on:segmentupdate={handleWaveformSegmentUpdate}
        />
      {:else if $transcriptStore.selectedMediaFile}
        <div
          class="flex items-center justify-center h-full text-xs text-gray-400 dark:text-gray-700 bg-white dark:bg-gray-950 p-1"
        >
          Waveform still loading...
        </div>
      {:else}
        <div
          class="flex items-center justify-center h-full text-xs text-gray-400 dark:text-gray-700 bg-white dark:bg-gray-950 p-1"
        >
          Select media to display waveform.
        </div>
      {/if}
    </div>
  {/if}

  {#if isManualSettingsModalOpen}
    <ManualSettingsModal
      bind:showModal={isManualSettingsModalOpen}
      currentSettings={$transcriptStore.manualSegmentSettings}
      speakerList={$transcriptStore.speakers?.names || []}
      on:confirm={handleManualSettingsConfirm}
      on:close={() => (isManualSettingsModalOpen = false)}
    />
  {/if}
</div>

<style lang="postcss">
  .min-h-0 {
    min-height: 0;
  }
  /* Ensure Tailwind JIT picks up these dynamic classes */
  .h-\[100px\] {
    height: 100px;
  }
</style>
