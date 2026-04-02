<!-- src/lib/components/projectview/data/media/MediaEditorPanel.svelte -->
<script>
  import { onMount, onDestroy, tick, createEventDispatcher } from 'svelte';
  import { get } from 'svelte/store';
  import { isMediaEditorOpen } from '$lib/stores/mediaEditorStore.js';
  import {
    project, // Store, aliased to projectStore below for clarity in functions
    clearStandaloneTranscriptSplit
  } from '$lib/stores/projectStore.js';
  import { invoke } from '@tauri-apps/api/core';
  import { message } from '@tauri-apps/plugin-dialog';
  import { basename } from '@tauri-apps/api/path';
  import { project as projectStore } from '$lib/stores/projectStore.js';
  import { handleTrimMediaConfirm } from '$lib/services/projectService.js';

  import MediaPlayer from '../../shared/MediaPlayer.svelte';
  import MediaTranscriptEditorSubPanel from './MediaTranscriptEditorSubPanel.svelte';
  import InteractiveWaveform from '../../shared/InteractiveWaveform.svelte';
  import TimestampInput from '../../shared/TimestampInput.svelte';

  export let mediaPath = null;

  const dispatch = createEventDispatcher();

  let mediaPlayerInDataRef;

  let isDataPlayerVideoHidden = false; // State for MediaPlayer's video visibility

  // Split View State
  let primaryPanel;
  let secondaryPanel;
  let cleanupSync = () => {};
  let isScrollSyncEnabled = true;
  let primaryRowCount = 0;
  let secondaryRowCount = 0;
  let primaryHighlightedRowIndex = -1;
  let secondaryHighlightedRowIndex = -1;

  $: splitInfo =
    $projectStore.standaloneTranscriptSplits[$projectStore.activeTranscriptPathInDataTab];
  $: splitPartnerPath = splitInfo?.partner;
  $: orientation = splitInfo?.orientation || 'horizontal';

  function handleSyncManager(path, enabled) {
    if (path && enabled) {
      attemptSetupSync();
    } else {
      cleanupSync();
    }
  }

  $: handleSyncManager(splitPartnerPath, isScrollSyncEnabled);

  function toggleScrollSync() {
    isScrollSyncEnabled = !isScrollSyncEnabled;
  }

  function handleCursorRowChange(event, panelSource) {
    const index = event.detail.index;
    if (panelSource === 'primary') {
      primaryHighlightedRowIndex = index;
      secondaryHighlightedRowIndex = index; // Sync to partner
    } else {
      secondaryHighlightedRowIndex = index;
      primaryHighlightedRowIndex = index; // Sync to partner
    }
  }

  function attemptSetupSync() {
    cleanupSync();
    let attempts = 0;
    const interval = setInterval(() => {
      attempts++;
      if (primaryPanel && secondaryPanel) {
        const el1 = primaryPanel.getScrollElement();
        const el2 = secondaryPanel.getScrollElement();
        if (el1 && el2) {
          clearInterval(interval);
          startSync(el1, el2);
        }
      }
      if (attempts > 20) {
        clearInterval(interval);
      }
    }, 100);
  }

  function startSync(el1, el2) {
    let isSyncing = false;
    let activeElement = null;

    const handleInteraction = (e) => {
      activeElement = e.currentTarget;
      if (isScrollSyncEnabled) {
        // If it's a click or key event, we might want to sync immediately based on cursor
        if (e.type === 'click' || e.type === 'keyup') {
          syncFollower();
        }
      }
    };

    const syncFollower = () => {
      if (isSyncing || !primaryPanel || !secondaryPanel) return;
      isSyncing = true;

      const leader = activeElement === el1 ? primaryPanel : secondaryPanel;
      const follower = activeElement === el1 ? secondaryPanel : primaryPanel;

      // Try cursor row first if visible
      const cursorRow = leader.getCursorRowInfo();
      if (cursorRow && cursorRow.index !== -1 && cursorRow.visible) {
        follower.scrollToRow(cursorRow.index, cursorRow.offset);
      } else {
        // Fallback to top visible row
        const topRow = leader.getTopVisibleRowInfo();
        if (topRow && topRow.index !== -1) {
          follower.scrollToRow(topRow.index, topRow.offset);
        }
      }

      requestAnimationFrame(() => (isSyncing = false));
    };

    const onScroll = () => {
      if (isScrollSyncEnabled && activeElement) {
        syncFollower();
      }
    };

    el1.addEventListener('scroll', onScroll, { passive: true });
    el2.addEventListener('scroll', onScroll, { passive: true });

    el1.addEventListener('pointerover', handleInteraction);
    el2.addEventListener('pointerover', handleInteraction);
    el1.addEventListener('wheel', handleInteraction, { passive: true });
    el2.addEventListener('wheel', handleInteraction, { passive: true });
    el1.addEventListener('click', handleInteraction);
    el2.addEventListener('click', handleInteraction);
    el1.addEventListener('keyup', handleInteraction);
    el2.addEventListener('keyup', handleInteraction);

    cleanupSync = () => {
      el1.removeEventListener('scroll', onScroll);
      el2.removeEventListener('scroll', onScroll);
      el1.removeEventListener('pointerover', handleInteraction);
      el2.removeEventListener('pointerover', handleInteraction);
      el1.removeEventListener('wheel', handleInteraction);
      el2.removeEventListener('wheel', handleInteraction);
      el1.removeEventListener('click', handleInteraction);
      el2.removeEventListener('click', handleInteraction);
      el1.removeEventListener('keyup', handleInteraction);
      el2.removeEventListener('keyup', handleInteraction);
      cleanupSync = () => {};
    };
  }

  let showDataTrimUI = false;
  let currentTrimAudioBuffer = null; // Buffer for the active trim session
  let currentTrimAudioPeaks = null; // Peaks for the active trim session (for lazy loading)
  let dataTrimStartTime = 0;
  let dataTrimEndTime = 0;

  // LIVE MediaPlayer properties needed by InteractiveWaveform
  let dataMediaPlayerCurrentTime = 0;
  let dataMediaPlayerIsPlaying = false;
  let dataMediaPlayerDuration = 0; // Bound to MediaPlayer to get duration reactively

  onMount(() => {
    isMediaEditorOpen.set(true);
    showDataTrimUI = false;
    currentTrimAudioBuffer = null;
    currentTrimAudioPeaks = null;
  });

  onDestroy(() => {
    isMediaEditorOpen.set(false);
    cleanupSync();
  });

  function handleRequestDataTranscribe(event) {
    dispatch('requestTranscriptionTabWithMedia', { mediaPath: event.detail.mediaPath });
  }

  function handleRequestDataTrim(event) {
    if (showDataTrimUI) {
      showDataTrimUI = false;
      currentTrimAudioBuffer = null;
      currentTrimAudioPeaks = null;
    } else {
      const duration = event.detail.duration;
      const audioBuffer = event.detail.audioBuffer;
      const peaks = event.detail.peaks;
      const isReady = event.detail.isReady;

      if (isReady && duration > 0) {
        dataTrimStartTime = 0;
        dataTrimEndTime = duration;
        currentTrimAudioBuffer = audioBuffer;
        // Use peaks from event, or fall back to what we might have already captured
        currentTrimAudioPeaks = peaks || currentTrimAudioPeaks;
        showDataTrimUI = true;
      } else {
        showDataTrimUI = false;
        currentTrimAudioBuffer = null;
        currentTrimAudioPeaks = null;
        alert(
          'MediaPlayer reported not ready or essential data was missing from the event. Cannot show trim UI.'
        );
      }
    }
  }

  function handleMediaDataTrimBufferReady(event) {
    if (showDataTrimUI && event.detail && event.detail.audioBuffer) {
      currentTrimAudioBuffer = event.detail.audioBuffer;
    }
  }

  function handleMediaDataPeaksReady(event) {
    if (event.detail && event.detail.peaks) {
      console.log('[MediaEditorPanel] Received peaks update (lazy or initial).');
      currentTrimAudioPeaks = event.detail.peaks;
    }
  }

  function handleWaveformTrimUpdate(event) {
    if (event.detail) {
      dataTrimStartTime = event.detail.startTime;
      dataTrimEndTime = event.detail.endTime;
    }
  }

  async function handleConfirmDataTrim() {
    if (!mediaPath) {
      console.error('Trim Error: No mediaPath specified.');
      await message('Error: No media file is specified for trimming.', {
        title: 'Trim Error',
        type: 'error'
      });
      return;
    }
    if (dataTrimEndTime <= dataTrimStartTime) {
      await message('Error: Trim end time must be after start time.', {
        title: 'Trim Error',
        type: 'error'
      });
      return;
    }
    projectStore.update((p) => ({
      ...p,
      isLoading: true,
      statusMessage: 'Trimming media in data...'
    }));
    try {
      await handleTrimMediaConfirm(mediaPath, dataTrimStartTime, dataTrimEndTime);

      const fileName = await basename(mediaPath);
      let mediaTypeFolder = 'Media';
      if (fileName) {
        const lowerFileName = fileName.toLowerCase();
        if (
          lowerFileName.endsWith('.mp3') ||
          lowerFileName.endsWith('.wav') ||
          lowerFileName.endsWith('.m4a') ||
          lowerFileName.endsWith('.ogg') ||
          lowerFileName.endsWith('.aac')
        ) {
          mediaTypeFolder = 'Audios';
        } else if (
          lowerFileName.endsWith('.mp4') ||
          lowerFileName.endsWith('.mov') ||
          lowerFileName.endsWith('.avi') ||
          lowerFileName.endsWith('.webm') ||
          lowerFileName.endsWith('.mkv')
        ) {
          mediaTypeFolder = 'Videos';
        }
      }

      projectStore.update((p) => ({
        ...p,
        isLoading: false,
        statusMessage: `Trimmed ${fileName} saved to ${mediaTypeFolder}. Reloading media...`
      }));
      await message(`Trimmed ${fileName} saved to ${mediaTypeFolder}.`, {
        title: 'Trim Successful'
      });

      showDataTrimUI = false;
      currentTrimAudioBuffer = null;
      const tempPath = mediaPath;
      mediaPath = null; // This triggers reactivity to reload the player
      await tick();
      mediaPath = tempPath;
      dataTrimStartTime = 0;
      dataTrimEndTime = 0;
    } catch (error) {
      console.error('[MediaEditorPanel] Trim failed:', error);
      projectStore.update((p) => ({
        ...p,
        isLoading: false,
        error: `Trim failed: ${error.message || error}`,
        statusMessage: 'Trim failed.'
      }));
      await message(`Failed to trim media: ${error.message || error}`, {
        title: 'Trim Failed',
        type: 'error'
      });
    }
  }

  function handleCancelDataTrim() {
    showDataTrimUI = false;
    currentTrimAudioBuffer = null;
    currentTrimAudioPeaks = null;
    dataTrimStartTime = 0;
    dataTrimEndTime = 0;
  }

  function handlePlaySegment(detail) {
    if (mediaPlayerInDataRef) {
      mediaPlayerInDataRef.playSegment(detail.startTime, detail.endTime);
    }
  }
</script>

<div class="flex flex-col h-full w-full bg-white dark:bg-gray-900">
  <div
    class="border-b border-gray-200 dark:border-gray-700 flex flex-col
               {!isDataPlayerVideoHidden ? 'h-1/2' : 'h-auto flex-shrink-0'}"
  >
    {#if mediaPath}
      <MediaPlayer
        bind:this={mediaPlayerInDataRef}
        bind:localCurrentTime={dataMediaPlayerCurrentTime}
        bind:localDuration={dataMediaPlayerDuration}
        bind:localIsPlaying={dataMediaPlayerIsPlaying}
        bind:isVideoMinimized={isDataPlayerVideoHidden}
        explicitMediaPath={mediaPath}
        projectId={$projectStore.id}
        showLoopPauseButton={false}
        showDataTranscribeButton={false}
        showDataTrimButton={true}
        enableLooping={showDataTrimUI}
        loopStartTime={dataTrimStartTime}
        loopEndTime={dataTrimEndTime}
        on:requestDataTrim={handleRequestDataTrim}
        on:mediaDataTrimBufferReady={handleMediaDataTrimBufferReady}
        on:mediaDataPeaksReady={handleMediaDataPeaksReady}
        on:mediaLoadError={(e) =>
          projectStore.update((p) => ({
            ...p,
            statusMessage: `Error loading media in data: ${e.detail.error}`
          }))}
        class={!isDataPlayerVideoHidden ? 'flex-grow min-h-0' : ''}
      />
    {:else}
      <div
        class="w-full h-full bg-black flex items-center justify-center text-gray-500 dark:text-gray-600"
      >
        <span>Media player requires a path.</span>
      </div>
    {/if}
  </div>

  <div
    class="min-h-0 overflow-hidden bg-white dark:bg-gray-900 {!isDataPlayerVideoHidden
      ? 'h-1/2'
      : ''} flex flex-col"
    class:flex-grow={isDataPlayerVideoHidden}
  >
    {#if showDataTrimUI && mediaPath}
      <div class="inline-trim-ui-wrapper">
        <div class="flex justify-between items-center mb-1">
          <div class="flex items-center gap-x-2">
            <p class="text-xs text-gray-600 dark:text-gray-600">Adjust start and end times:</p>
            <TimestampInput
              value={dataTrimStartTime}
              on:update={(e) => (dataTrimStartTime = e.detail)}
            />
            <span class="text-gray-400 dark:text-white">—</span>
            <TimestampInput
              value={dataTrimEndTime}
              on:update={(e) => (dataTrimEndTime = e.detail)}
            />
          </div>
          <div class="space-x-2">
            <button
              class="bg-blue-600 hover:bg-blue-700 text-white text-xs font-semibold py-1 px-3 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-opacity-50"
              on:click={handleConfirmDataTrim}>Trim</button
            >
            <button
              class="bg-gray-500 hover:bg-gray-600 text-white text-xs font-semibold py-1 px-3 focus:outline-none focus:ring-2 focus:ring-gray-400 focus:ring-opacity-50"
              on:click={handleCancelDataTrim}>Cancel</button
            >
          </div>
        </div>
        {#if (currentTrimAudioBuffer || currentTrimAudioPeaks) && dataTrimEndTime > 0}
          <div
            class="waveform-container w-full h-[75px] bg-gray-100 dark:bg-gray-800 overflow-hidden"
          >
            <InteractiveWaveform
              startZoomedOut={true}
              externalAudioBuffer={currentTrimAudioBuffer}
              externalPeaks={currentTrimAudioPeaks}
              externalCurrentTime={dataMediaPlayerCurrentTime}
              externalDuration={dataMediaPlayerDuration}
              externalIsPlaying={dataMediaPlayerIsPlaying}
              externalSegments={[]}
              externalCurrentSegmentIndex={-1}
              isTrimming={true}
              bind:trimStartTime={dataTrimStartTime}
              bind:trimEndTime={dataTrimEndTime}
              isEditingSegment={false}
              editSegmentStartTime={0}
              editSegmentEndTime={0}
              on:trimupdate={handleWaveformTrimUpdate}
              on:seek={(e) => mediaPlayerInDataRef?.seekTo(e.detail.time)}
            />
          </div>
        {:else}
          <div
            class="w-full h-[100px] flex items-center justify-center bg-gray-100 dark:bg-gray-800 rounded text-xs text-gray-500"
          >
            <div class="flex items-center gap-2">
              <div
                class="animate-spin rounded-full h-4 w-4 border-2 border-gray-300 border-t-blue-600"
              ></div>
              <span>Loading waveform data...</span>
            </div>
          </div>
        {/if}
      </div>
    {/if}

    {#if splitPartnerPath && primaryRowCount > 0 && secondaryRowCount > 0 && primaryRowCount !== secondaryRowCount}
      <div
        class="bg-amber-50 dark:bg-amber-900/20 border-b border-amber-200 dark:border-amber-800/50 px-4 py-2 flex items-center gap-2 text-amber-800 dark:text-amber-200 text-xs shrink-0"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 20 20"
          fill="currentColor"
          class="w-4 h-4"
        >
          <path
            fill-rule="evenodd"
            d="M8.485 2.495c.673-1.167 2.357-1.167 3.03 0l6.28 10.875c.673 1.167-.17 2.625-1.516 2.625H3.72c-1.347 0-2.189-1.458-1.515-2.625l6.28-10.875zM10 5a.75.75 0 01.75.75v3.5a.75.75 0 01-1.5 0v-3.5A.75.75 0 0110 5zm0 9a1 1 0 100-2 1 1 0 000 2z"
            clip-rule="evenodd"
          />
        </svg>
        <span
          ><strong>Row Count Mismatch:</strong> The primary transcript has {primaryRowCount} rows, while
          the partner transcript has {secondaryRowCount} rows. Scroll sync may be inaccurate.</span
        >
      </div>
    {/if}

    <div class="flex-grow min-h-0 overflow-hidden">
      {#if splitPartnerPath}
        <div
          class="flex h-full w-full divide-gray-300 dark:divide-gray-600 {orientation ===
          'horizontal'
            ? 'flex-row divide-x'
            : 'flex-col divide-y'}"
        >
          <div
            class="{orientation === 'horizontal'
              ? 'w-1/2 h-full'
              : 'h-1/2 w-full'} overflow-hidden flex flex-col"
          >
            <div
              class="bg-gray-100 dark:bg-gray-800 px-2 py-1 text-xs font-semibold text-gray-600 dark:text-gray-400 border-b border-gray-300 dark:border-gray-600 flex items-center h-8"
            >
              <span class="truncate"
                >{$projectStore.activeTranscriptPathInDataTab.split(/[\\/]/).pop()}</span
              >
            </div>
            <div class="flex-grow overflow-hidden">
              {#key $projectStore.activeTranscriptPathInDataTab}
                <MediaTranscriptEditorSubPanel
                  bind:this={primaryPanel}
                  {mediaPath}
                  transcriptPath={$projectStore.activeTranscriptPathInDataTab}
                  isPrimary={true}
                  highlightedRowIndex={primaryHighlightedRowIndex}
                  on:playsegment={(e) => handlePlaySegment(e.detail)}
                  on:rowcountupdated={(e) => (primaryRowCount = e.detail.rowCount)}
                  on:cursorrowchange={(e) => handleCursorRowChange(e, 'primary')}
                />
              {/key}
            </div>
          </div>
          <div
            class="{orientation === 'horizontal'
              ? 'w-1/2 h-full'
              : 'h-1/2 w-full'} overflow-hidden flex flex-col"
          >
            <div
              class="bg-gray-100 dark:bg-gray-800 px-2 py-1 text-xs font-semibold text-gray-600 dark:text-gray-400 border-b border-gray-300 dark:border-gray-600 flex justify-between items-center h-8"
            >
              <div class="flex items-center min-w-0 flex-grow">
                <span class="truncate">{splitPartnerPath.split(/[\\/]/).pop()}</span>
              </div>
              <button
                class="ml-2 flex-shrink-0"
                class:text-black={!isScrollSyncEnabled}
                class:text-blue-500={isScrollSyncEnabled}
                class:dark:text-gray-400={!isScrollSyncEnabled}
                title={isScrollSyncEnabled ? 'Disable Scroll Sync' : 'Enable Scroll Sync'}
                on:click={toggleScrollSync}
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="16"
                  height="16"
                  fill="currentColor"
                  class="bi bi-arrow-down-up"
                  viewBox="0 0 16 16"
                >
                  <path
                    fill-rule="evenodd"
                    d="M11.5 15a.5.5 0 0 0 .5-.5V2.707l3.146 3.147a.5.5 0 0 0 .708-.708l-4-4a.5.5 0 0 0-.708 0l-4 4a.5.5 0 1 0 .708.708L11 2.707V14.5a.5.5 0 0 0 .5.5m-7-14a.5.5 0 0 1 .5.5v11.793l3.146-3.147a.5.5 0 0 1 .708.708l-4 4a.5.5 0 0 1-.708 0l-4-4a.5.5 0 0 1 .708-.708L4 13.293V1.5a.5.5 0 0 1 .5-.5"
                  />
                </svg>
              </button>
              <button
                class="hover:text-red-500 ml-2 flex-shrink-0"
                title="Close Split"
                on:click={() =>
                  clearStandaloneTranscriptSplit($projectStore.activeTranscriptPathInDataTab)}
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="14"
                  height="14"
                  fill="currentColor"
                  class="bi bi-x-lg"
                  viewBox="0 0 16 16"
                >
                  <path
                    d="M2.146 2.854a.5.5 0 1 1 .708-.708L8 7.293l5.146-5.147a.5.5 0 0 1 .708.708L8.707 8l5.147 5.146a.5.5 0 0 1-.708.708L8 8.707l-5.146 5.147a.5.5 0 0 1-.708-.708L7.293 8z"
                  />
                </svg>
              </button>
            </div>
            <div class="flex-grow overflow-hidden">
              {#key splitPartnerPath}
                <MediaTranscriptEditorSubPanel
                  bind:this={secondaryPanel}
                  {mediaPath}
                  transcriptPath={splitPartnerPath}
                  isPrimary={false}
                  highlightedRowIndex={secondaryHighlightedRowIndex}
                  on:playsegment={(e) => handlePlaySegment(e.detail)}
                  on:rowcountupdated={(e) => (secondaryRowCount = e.detail.rowCount)}
                  on:cursorrowchange={(e) => handleCursorRowChange(e, 'secondary')}
                />
              {/key}
            </div>
          </div>
        </div>
      {:else if $projectStore.activeTranscriptPathInDataTab}
        {#key $projectStore.activeTranscriptPathInDataTab}
          <MediaTranscriptEditorSubPanel
            bind:this={primaryPanel}
            {mediaPath}
            transcriptPath={$projectStore.activeTranscriptPathInDataTab}
            isPrimary={true}
            highlightedRowIndex={primaryHighlightedRowIndex}
            on:playsegment={(e) => handlePlaySegment(e.detail)}
            on:rowcountupdated={(e) => (primaryRowCount = e.detail.rowCount)}
            on:cursorrowchange={(e) => handleCursorRowChange(e, 'primary')}
          />
        {/key}
      {:else if !mediaPath}
        <div
          class="flex-grow flex items-center justify-center text-gray-500 dark:text-gray-400 p-4"
        >
          Select an audio or video file from the Data panel to view its player and data.
        </div>
      {:else}
        <div
          class="flex-grow flex flex-col items-center justify-center text-blue-600 dark:text-blue-400 p-4 text-center"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            class="h-10 w-10 mb-2 opacity-70"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            stroke-width="1.5"
            ><path
              stroke-linecap="round"
              stroke-linejoin="round"
              d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z"
            /></svg
          >
          <p class="font-semibold">No Transcription Yet</p>
          <p class="text-xs mt-1">
            To generate a transcript, you can use the main "Transcribe" feature in the Transcription
            tab.
          </p>
        </div>
      {/if}
    </div>
  </div>
</div>

<style lang="postcss">
  .flex-grow.min-h-0 {
    min-height: 0;
  }

  .inline-trim-ui-wrapper {
    position: fixed;
    bottom: 0;
    left: 0;
    width: 100%;
    z-index: 100;
    background-color: var(--color-bg-app-dark, #1f2937);
    padding: 0.5rem;
    border-top: 1px solid var(--color-border-strong, #374151);
    box-shadow: 0 -2px 10px rgba(0, 0, 0, 0.1);
  }

  :global(html:not(.dark)) .inline-trim-ui-wrapper {
    background-color: var(--color-bg-app-light, #f9fafb);
    border-top: 1px solid var(--color-border-strong-light, #e5e7eb);
  }

  .waveform-container {
    border: 1px solid var(--theme-dark-border, #4b5563);
  }
  :global(html:not(.dark)) .waveform-container {
    border: 1px solid var(--theme-border, #d1d5db);
  }
</style>
