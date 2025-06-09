<!-- src/lib/components/projectview/transcriptions/TranscriptionsView.svelte -->
<script>
    import { tick, createEventDispatcher, onMount, onDestroy } from 'svelte';
    import { get } from 'svelte/store';
    import { project } from '$lib/stores/projectStore.js';
    import {
        transcriptStore,
        deleteTranscriptSegment,
        undoTranscriptChange,
        redoTranscriptChange,
        markTranscriptAsSaved, // This is now in transcriptStore, but saveTranscriptData service calls it.
        insertTranscriptSegment,
        updatePlayerCurrentSegmentIndex
    } from '$lib/stores/transcriptStore.js';
    import {
        saveTranscriptData,
        requestTranscription as requestTranscriptionService, // Renamed to avoid conflict
        convertAndSaveTranscriptAsDoc
    } from '$lib/services/projectService.js';
    import { confirm, message } from '@tauri-apps/plugin-dialog';

    import TopBar from './TopBar.svelte';
    import LeftPanel from './LeftPanel.svelte';
    import MediaPlayer from '../shared/MediaPlayer.svelte';
    import InteractiveWaveform from './InteractiveWaveform.svelte';
    import EditableTranscript from './EditableTranscript.svelte';
    import RichTextPreview from './RichTextPreview.svelte';

    const dispatch = createEventDispatcher();

    export let mediaPlayerRef = null;

    let editableTranscriptRef;
    let richTextPreviewRef;
    let topBarRef;
    let leftPanelRef;

    // State for MediaPlayer within THIS TranscriptionsView
    // These are bound to the MediaPlayer's props.
    let isMediaPlayerTrimming = false;
    let mediaPlayerTrimStart = 0;
    let mediaPlayerTrimEnd = 0;

    let isSegmentEditingActive = false;
    let currentEditSegmentStart = 0;
    let currentEditSegmentEnd = 0;

    let panelEditModeActive = false;
    let wasPlayingBeforeEdit = false;

    function handleSegmentClick(event) {
        if (panelEditModeActive) return;
        const index = event.detail;
        const segment = get(transcriptStore).segments?.[index];
        if (segment && typeof segment.start_time === 'number') {
            editableTranscriptRef?.loadSegment?.(index);
            if (mediaPlayerRef) {
                mediaPlayerRef.seekTo(segment.start_time);
            }
        } else {
            console.warn(`[TranscriptionsView] Invalid segment data for index ${index} on click.`);
        }
    }

    function handlePanelNavigate(event) {
        const detail = event.detail;
        if (detail && typeof detail.time === 'number') {
            if (mediaPlayerRef) mediaPlayerRef.seekTo(detail.time);
        } else if (detail && typeof detail.index === 'number') {
            const segment = get(transcriptStore).segments?.[detail.index];
            if (segment && mediaPlayerRef) {
                const seekTime = isSegmentEditingActive ? Math.max(currentEditSegmentStart, Math.min(segment.start_time, currentEditSegmentEnd - 0.001)) : segment.start_time;
                mediaPlayerRef.seekTo(seekTime);
            }
        } else {
            console.warn('[TranscriptionsView] Unexpected navigation event detail:', detail);
        }
    }

    // This handler is for when the MediaPlayer component itself enters trim mode (e.g., user clicks its trim button)
    // or when its bound `isTrimming` prop changes.
    function handleMediaPlayerTrimModeEntered(event) {
        console.log('[TranscriptionsView] MediaPlayer dispatched trimModeEntered:', event.detail);
        isMediaPlayerTrimming = true; // Ensure TranscriptionsView's state is synced
        mediaPlayerTrimStart = event.detail.startTime;
        mediaPlayerTrimEnd = event.detail.endTime;
    }

    function handleMediaPlayerTrimModeCancelled() {
        console.log('[TranscriptionsView] MediaPlayer dispatched trimModeCancelled.');
        isMediaPlayerTrimming = false;
    }


    function handleWaveformTrimUpdate(event) {
        const { startTime, endTime } = event.detail;
        if(mediaPlayerRef) mediaPlayerRef.updateTrimTimes(startTime, endTime);
        // These local states are already bound or updated via handleMediaPlayerTrimModeEntered
        // isMediaPlayerTrimming = true; // Not needed here if MediaPlayer manages its own button UI
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
        } else {
            console.error('[TranscriptionsView] EditableTranscript ref not available for waveform update.');
        }
    }

    export async function handleToggleEditMode() {
        const wasEditing = panelEditModeActive;
        const isDirty = get(transcriptStore).transcriptDirty;
        if (wasEditing && isDirty) {
            try {
                await handleSaveTranscript();
                panelEditModeActive = false;
            } catch (error) {
                 const discard = await confirm( `Failed to save changes: ${error.message}\n\nDiscard changes and exit edit mode anyway?`, { title: "Save Failed", type: "warning", okLabel: "Discard & Exit", cancelLabel: "Keep Editing" } );
                 if (discard) {
                    // transcriptDirty, transcriptUndoStack, transcriptRedoStack are in transcriptStore now
                    transcriptStore.update(ts => ({ ...ts, transcriptDirty: false, transcriptUndoStack: [], transcriptRedoStack: [] }));
                    panelEditModeActive = false;
                    editableTranscriptRef?.forceReloadFromStore?.();
                 }
            }
        } else if (wasEditing && !isDirty) {
            panelEditModeActive = false;
        } else {
            panelEditModeActive = true;
            await tick();
            editableTranscriptRef?.focusEditor?.();
        }
    }

    export async function handleSaveTranscript() {
        if (panelEditModeActive && editableTranscriptRef) {
            const editsCommitted = editableTranscriptRef.commitCurrentSegmentEdits();
            if (editsCommitted) await tick();
        }
        const tsStore = get(transcriptStore);
        if (!tsStore.transcriptDirty) {
            if(topBarRef) topBarRef.showSavedIndicator(false);
            return;
        }
        try {
            project.update(p => ({ ...p, isLoading: true, statusMessage: 'Saving transcript...' })); // Global loading state
            await saveTranscriptData(); // This service will use get(transcriptStore) for currentTranscriptPath and segments
            // markTranscriptAsSaved(); // This is now called by saveTranscriptData service via transcriptStore.
            if (panelEditModeActive) panelEditModeActive = false;
            project.update(p => ({ ...p, isLoading: false, statusMessage: 'Transcript saved.' })); // Global status
            if(topBarRef) topBarRef.showSavedIndicator(true);
        } catch (error) {
            const errorMsg = error instanceof Error ? error.message : String(error);
            project.update(p => ({ ...p, isLoading: false, error: `Save failed: ${errorMsg}`, statusMessage: 'Save failed.' })); // Global error
            if(topBarRef) topBarRef.showSavedIndicator(false);
            await message(`Error saving transcript: ${errorMsg}`, {title: "Save Error", type: "error"});
            throw error;
        }
    }

    function handleRequestTranscriptionEvent() {
        requestTranscriptionService();
    }

    async function handleConvertToDocumentEvent() {
        if (get(transcriptStore).transcriptDirty) {
			const confirmConvert = await confirm( "You have unsaved transcript changes. Save them before converting?", { title: "Unsaved Changes", type: "warning", okLabel: "Save & Convert", cancelLabel: "Cancel" });
			if (!confirmConvert) return;
			try { await handleSaveTranscript(); }
            catch (e) { await message(`Save failed: ${e.message || e}. Cannot convert.`, {type:'error', title: 'Save Error'}); return; }
		}
        try {
            project.update(p => ({...p, statusMessage: "Converting to document..."}));
            const newDocPath = await convertAndSaveTranscriptAsDoc();
            project.update(p => ({...p, statusMessage: "Converted to document successfully."}));
            await message(`Transcript converted and saved as a new document.`, { title: 'Conversion Successful'});
            dispatch('requestopentab', { tabName: 'notes', loadNotePath: newDocPath });
        } catch (error) {
            await message(`Failed to convert: ${error.message || error}`, { title: 'Conversion Error', type: 'error' });
            project.update(p => ({...p, statusMessage: "Conversion failed."}));
        }
    }

    export function handleDeleteSegmentRequest(event) { const indexToDelete = event.detail; if (typeof indexToDelete === 'number') deleteTranscriptSegment(indexToDelete); }
    export function handleUndoRequest() { undoTranscriptChange(); editableTranscriptRef?.forceReloadFromStore?.(); }
    export function handleRedoRequest() { redoTranscriptChange(); editableTranscriptRef?.forceReloadFromStore?.(); }
    export function handleInsertSegmentRequest(event) { const { index, startTime, endTime } = event.detail; if (typeof index !== 'number' || typeof startTime !== 'number' || typeof endTime !== 'number' || endTime <= startTime) return; const newSegment = { start_time: startTime, end_time: endTime, speaker: "Unknown", text: JSON.stringify({ root: { children: [{ type: 'paragraph', version: 1, children: [], direction: null, format: '', indent: 0 }], type: 'root', version: 1, direction: null, format: '', indent: 0 } }), }; insertTranscriptSegment(index, newSegment); }

    export function activateTrimModeOnPlayer() {
        console.log("[TranscriptionsView] activateTrimModeOnPlayer called.");
        if (mediaPlayerRef && typeof mediaPlayerRef.enterTrimMode === 'function') {
            mediaPlayerRef.enterTrimMode();
            // The `enterTrimMode` in MediaPlayer should ideally set its `isTrimming` prop.
            // Since `isMediaPlayerTrimming` is bound to MediaPlayer's `isTrimming` prop,
            // this should update `isMediaPlayerTrimming` here automatically.
            // We directly set it here to be certain the UI within TranscriptionsView updates.
            isMediaPlayerTrimming = true;
            console.log("[TranscriptionsView] Trim mode activated on its MediaPlayer. isMediaPlayerTrimming set to true.");
        } else {
            console.warn("[TranscriptionsView] Could not activate trim mode: mediaPlayerRef or enterTrimMode method not found.");
        }
    }

    $: if (mediaPlayerRef && mediaPlayerRef.videoElement) {
        const video = mediaPlayerRef.videoElement;
        if (isSegmentEditingActive) {
            if (!wasPlayingBeforeEdit && !video.paused) {
                wasPlayingBeforeEdit = true;
                try { video.pause(); } catch (e) { console.warn("Error pausing on edit focus:", e); }
            }
        } else {
            if (wasPlayingBeforeEdit) {
                if (video.paused) {
                    try { video.play().catch(console.error); } catch (e) { console.warn("Error resuming on edit blur:", e); }
                }
                wasPlayingBeforeEdit = false;
            }
        }
    }

    function forwardLeftPanelEvents(event) {
        if (event.type === 'requestopentab') {
            dispatch('requestopentab', event.detail);
        } else if (event.type === 'requestmediaselection') {
            dispatch('requestmediaselection', event.detail);
        }
    }

    onMount(() => { console.log('[TranscriptionsView] Mounted.'); });
    onDestroy(() => { console.log('[TranscriptionsView] Destroyed.'); });

</script>

<div class="flex flex-col h-screen w-full overflow-hidden">
    <TopBar
        bind:this={topBarRef}
        bind:editMode={panelEditModeActive}
        on:requestTranscription={handleRequestTranscriptionEvent}
        on:save={handleSaveTranscript}
        on:toggleEditMode={handleToggleEditMode}
    />
    <div class="flex flex-grow min-h-0 p-1 gap-1 w-full">
        <div class="w-[15%] h-full bg-white dark:bg-gray-800 rounded-md shadow overflow-y-auto">
            <LeftPanel bind:this={leftPanelRef} on:requestopentab={forwardLeftPanelEvents} on:requestmediaselection={forwardLeftPanelEvents} />
        </div>
        <div class="w-[40%] h-full flex flex-col gap-1">
            <div class="h-1/2 bg-white dark:bg-gray-800 rounded-md shadow flex flex-col">
                <MediaPlayer
                    bind:this={mediaPlayerRef}
                    bind:isTrimming={isMediaPlayerTrimming}
                    bind:trimStartTime={mediaPlayerTrimStart}
                    bind:trimEndTime={mediaPlayerTrimEnd}
                    bind:isEditingSegment={isSegmentEditingActive}
                    bind:editSegmentStartTime={currentEditSegmentStart}
                    bind:editSegmentEndTime={currentEditSegmentEnd}
                    showLoopPauseButton={true}
                    showNotesTranscribeButton={false}
                    showNotesTrimButton={false}
                    on:trimModeEntered={handleMediaPlayerTrimModeEntered}
                    on:trimModeCancelled={handleMediaPlayerTrimModeCancelled}
                />
            </div>
            <div class="h-1/2 min-h-0 bg-white dark:bg-gray-800 rounded-md shadow overflow-y-auto">
                 <EditableTranscript
                    bind:this={editableTranscriptRef}
                    bind:panelEditMode={panelEditModeActive}
                    on:navigate={handlePanelNavigate}
                    on:segmenteditfocus={handleSegmentEditFocus}
                    on:save={handleSaveTranscript}
                    on:toggleedit={handleToggleEditMode}
                 />
            </div>
        </div>
        <div class="w-[45%] h-full bg-white dark:bg-gray-800 rounded-md shadow overflow-y-auto">
             <RichTextPreview
                bind:this={richTextPreviewRef}
                bind:previewEditMode={panelEditModeActive}
                on:segmentclick={handleSegmentClick}
                on:toggleedit={handleToggleEditMode}
                on:requestopentab={(e) => dispatch('requestopentab', e.detail)}
                on:deletetranscriptsegment={handleDeleteSegmentRequest}
                on:insertnewsegment={handleInsertSegmentRequest}
                on:undo={handleUndoRequest}
                on:redo={handleRedoRequest}
                on:convertToDocument={handleConvertToDocumentEvent}
             />
        </div>
    </div>
    <div class="h-24 w-full px-1 pb-1 flex-shrink-0">
        <div class="h-full bg-white dark:bg-gray-800 rounded-md shadow">
            {#if $transcriptStore.selectedMediaFile && $transcriptStore.audioBuffer}
                <InteractiveWaveform
                    audioBuffer={$transcriptStore.audioBuffer}
                    currentTime={$transcriptStore.player.currentTime}
                    duration={$transcriptStore.player.duration}
                    segments={$transcriptStore.segments}
                    isPlaying={$transcriptStore.player.isPlaying}
                    currentSegmentIndex={$transcriptStore.player.currentSegmentIndex}
                    on:seek={(e) => mediaPlayerRef?.seekTo(e.detail.time)}
                    bind:isEditingSegment={isSegmentEditingActive}
                    bind:editSegmentStartTime={currentEditSegmentStart}
                    bind:editSegmentEndTime={currentEditSegmentEnd}
                    bind:isTrimming={isMediaPlayerTrimming}
                    bind:trimStartTime={mediaPlayerTrimStart}
                    bind:trimEndTime={mediaPlayerTrimEnd}
                    on:updateTrimTimes={(event) => mediaPlayerRef?.updateTrimTimes(event.detail.startTime, event.detail.endTime)}
                    on:navigate={handlePanelNavigate}
                    on:trimupdate={handleWaveformTrimUpdate}
                    on:segmentupdate={handleWaveformSegmentUpdate}
                />
            {:else if $transcriptStore.selectedMediaFile && !$transcriptStore.audioBuffer && !$project.isLoading && !$project.error?.includes('Media Error')}
                <div class="flex items-center justify-center h-full text-sm text-gray-500 dark:text-gray-400">
                    {#if $transcriptStore.selectedMediaFile?.name?.toLowerCase().endsWith('.mp4') || $transcriptStore.selectedMediaFile?.name?.toLowerCase().endsWith('.mov')  || $transcriptStore.selectedMediaFile?.name?.toLowerCase().endsWith('.webm') || $transcriptStore.selectedMediaFile?.name?.toLowerCase().endsWith('.mkv') || $transcriptStore.selectedMediaFile?.name?.toLowerCase().endsWith('.avi')}
                        Video file loaded. Waveform view is for audio analysis.
                    {:else if $project.error && $project.error.includes("decode")}
                        Could not decode audio data for waveform.
                    {:else}
                        Audio buffer not available for waveform.
                    {/if}
                </div>
            {:else if !$transcriptStore.selectedMediaFile && !$project.isLoading}
                    <div class="flex items-center justify-center h-full text-sm text-gray-500 dark:text-gray-400">Select media to view waveform.</div>
            {/if}
        </div>
    </div>
</div>

<style lang="postcss">
    .min-h-0 { min-height: 0; }
</style>