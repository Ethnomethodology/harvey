<!-- src/lib/components/projectview/transcriptions/TranscriptionsView.svelte -->

<script>
    import { tick, createEventDispatcher, onMount, onDestroy } from 'svelte';
    import { get } from 'svelte/store';
    import {
        project,
        deleteTranscriptSegment,
        undoTranscriptChange,
        redoTranscriptChange,
        markTranscriptAsSaved,
        insertTranscriptSegment,
        updatePlayerCurrentSegmentIndex
    } from '$lib/stores/projectStore.js';
    import { saveTranscriptData } from '$lib/services/projectService.js';
    import { confirm, message } from '@tauri-apps/plugin-dialog';

    // --- Import Child Components ---
    import TopBar from './TopBar.svelte';
    import LeftPanel from './LeftPanel.svelte';
    import MediaPlayer from './MediaPlayer.svelte';
    import InteractiveWaveform from './InteractiveWaveform.svelte';
    import EditableTranscript from './EditableTranscript.svelte';
    import RichTextPreview from './RichTextPreview.svelte';

    const dispatch = createEventDispatcher();

    // --- Exported Props ---
    export let mediaPlayerRef = null;
    export let handleRequestOpenTab = (event) => { dispatch('requestopentab', event.detail); };

    // --- Component References ---
    let editableTranscriptRef;
    let richTextPreviewRef;

    // --- State Variables ---
    let isMediaPlayerTrimming = false;
    let mediaPlayerTrimStart = 0;
    let mediaPlayerTrimEnd = 0;
    let isSegmentEditingActive = false; // Is the user currently focused/interacting with EditableTranscript fields?
    let currentEditSegmentStart = 0;
    let currentEditSegmentEnd = 0;
    let panelEditModeActive = false; // Is the transcript generally in "edit mode"?
    let wasPlayingBeforeEdit = false;

    // --- Event Handlers ---
    function handleSegmentClick(event) { if (panelEditModeActive) return; const index = event.detail; const segment = get(project).segments?.[index]; if (segment && typeof segment.start_time === 'number') { console.log(`[TranscriptionsView] Segment ${index} clicked. Seeking player and loading editor.`); editableTranscriptRef?.loadSegment?.(index); if (mediaPlayerRef) { mediaPlayerRef.seekTo(segment.start_time); } } else { console.warn(`[TranscriptionsView] Invalid segment data for index ${index} on click.`); } }
    function handlePanelNavigate(event) { const detail = event.detail; if (detail && typeof detail.time === 'number') { if (mediaPlayerRef) mediaPlayerRef.seekTo(detail.time); } else if (detail && typeof detail.index === 'number') { const segment = get(project).segments?.[detail.index]; if (segment && mediaPlayerRef) { const seekTime = isSegmentEditingActive ? Math.max(currentEditSegmentStart, Math.min(segment.start_time, currentEditSegmentEnd - 0.001)) : segment.start_time; mediaPlayerRef.seekTo(seekTime); } } else { console.warn('[TranscriptionsView] Unexpected navigation event detail:', detail); } }
    function handleWaveformTrimUpdate(event) { const { startTime, endTime } = event.detail; if(mediaPlayerRef) mediaPlayerRef.updateTrimTimes(startTime, endTime); isMediaPlayerTrimming = true; mediaPlayerTrimStart = startTime; mediaPlayerTrimEnd = endTime; }
    function handleSegmentEditFocus(event) { const { isEditing, startTime, endTime } = event.detail; isSegmentEditingActive = isEditing; currentEditSegmentStart = startTime ?? 0; currentEditSegmentEnd = endTime ?? 0; }

    // --- MODIFIED: handleWaveformSegmentUpdate ---
    function handleWaveformSegmentUpdate(event) {
        const { startTime, endTime } = event.detail;
        if (editableTranscriptRef) {
            // --- REMOVED CONDITIONAL CHECK ---
            // Always pass the update to EditableTranscript when the waveform handle moves.
            // Let EditableTranscript decide how to handle it internally.
            console.log(`[TranscriptionsView] Waveform segment update received: Start=${startTime.toFixed(3)}, End=${endTime.toFixed(3)}. Calling updateTimesFromExternal.`);
            editableTranscriptRef.updateTimesFromExternal(startTime, endTime);
            // --- END REMOVED CHECK ---
        } else {
            console.error('[TranscriptionsView] EditableTranscript ref not available for waveform update.');
        }
    }

    // --- Toggle Edit Mode ---
    export async function handleToggleEditMode() {
        const wasEditing = panelEditModeActive; const isDirty = get(project).transcriptDirty;
        if (wasEditing && isDirty) {
            console.log("[TranscriptionsView] Exiting edit mode via toggle, attempting save...");
            try { await handleSaveTranscript(); panelEditModeActive = false; console.log("[TranscriptionsView] Save successful via toggle, edit mode deactivated."); }
            catch (error) {
                 console.error("[TranscriptionsView] Save failed on toggle-off:", error);
                 const discard = await confirm( `Failed to save changes: ${error.message}\n\nDiscard changes and exit edit mode anyway?`, { title: "Save Failed", type: "warning", okLabel: "Discard & Exit", cancelLabel: "Keep Editing" } );
                 if (discard) { console.log("[TranscriptionsView] User chose discard after save failed."); project.update(p => ({ ...p, transcriptDirty: false, transcriptUndoStack: [], transcriptRedoStack: [] })); panelEditModeActive = false; editableTranscriptRef?.forceReloadFromStore?.(); }
                 else { console.log("[TranscriptionsView] User chose keep editing after save failed."); }
            }
        } else if (wasEditing && !isDirty) { panelEditModeActive = false; console.log("[TranscriptionsView] Exiting edit mode via toggle, no changes detected."); }
        else { panelEditModeActive = true; console.log("[TranscriptionsView] Entering edit mode via toggle."); tick().then(() => { editableTranscriptRef?.focusEditor?.(); }); }
    }

    // --- Save Transcript ---
    export async function handleSaveTranscript() {
        let editsCommitted = false;
        if (editableTranscriptRef) { console.log("[TranscriptionsView] Committing pending edits..."); editsCommitted = editableTranscriptRef.commitCurrentSegmentEdits(); if (editsCommitted) { await tick(); console.log("[TranscriptionsView] Edits committed."); } else { console.log("[TranscriptionsView] No pending edits committed."); } }
        else { console.warn("[TranscriptionsView] Cannot commit edits: EditableTranscript ref not available."); }
        const proj = get(project);
        if (!proj.transcriptDirty) { console.log("[TranscriptionsView] Save skipped: Transcript not dirty after commit."); if (panelEditModeActive) { console.log("[TranscriptionsView] Exiting edit mode after no-op save."); panelEditModeActive = false; } return; }
        console.log("[TranscriptionsView] Proceeding with saveTranscriptData service call...");
        try {
            project.update(p => ({ ...p, isLoading: true, statusMessage: 'Saving transcript...' })); await saveTranscriptData(); markTranscriptAsSaved(); panelEditModeActive = false; project.update(p => ({ ...p, isLoading: false, statusMessage: 'Transcript saved.' })); console.log("[TranscriptionsView] Transcript saved successfully and edit mode deactivated.");
        } catch (error) {
            console.error('[TranscriptionsView] saveTranscriptData failed:', error); const errorMsg = error instanceof Error ? error.message : String(error); project.update(p => ({ ...p, isLoading: false, error: `Save failed: ${errorMsg}`, statusMessage: 'Save failed.' })); await message(`Error saving transcript: ${errorMsg}`, {title: "Save Error", type: "error"}); throw error;
        }
    }

    // --- Other Action Handlers ---
    export function handleDeleteSegmentRequest(event) { const indexToDelete = event.detail; if (typeof indexToDelete === 'number') { deleteTranscriptSegment(indexToDelete); } else { console.error("[TranscriptionsView] Invalid index for deletetranscriptsegment:", indexToDelete); } }
    export function handleUndoRequest() { undoTranscriptChange(); }
    export function handleRedoRequest() { redoTranscriptChange(); }
    export function handleInsertSegmentRequest(event) { const { index, startTime, endTime } = event.detail; if (typeof index !== 'number' || typeof startTime !== 'number' || typeof endTime !== 'number' || endTime <= startTime) { console.error("[TranscriptionsView] Invalid data for insertnewsegment:", event.detail); return; } const newSegment = { start_time: startTime, end_time: endTime, speaker: "Unknown", text: JSON.stringify({ root: { children: [{ type: 'paragraph', version: 1, children: [], direction: null, format: '', indent: 0 }], type: 'root', version: 1, direction: null, format: '', indent: 0 } }), }; insertTranscriptSegment(index, newSegment); }

    // --- Reactive Statements ---
    $: if (mediaPlayerRef && mediaPlayerRef.videoElement) { const video = mediaPlayerRef.videoElement; if (isSegmentEditingActive) { if (!wasPlayingBeforeEdit && !video.paused) { wasPlayingBeforeEdit = true; try { video.pause(); console.log("[TranscriptionsView Reactive] Paused player on segment edit focus."); } catch (e) { console.warn("[TranscriptionsView Reactive] Error pausing:", e); } } } else { if (wasPlayingBeforeEdit) { if (video.paused) { try { video.play().catch(console.error); console.log("[TranscriptionsView Reactive] Resuming player on segment edit blur."); } catch (e) { console.warn("[TranscriptionsView Reactive] Error resuming:", e); } } else { console.log("[TranscriptionsView Reactive] Left segment edit, player already playing."); } wasPlayingBeforeEdit = false; } } }

</script>

<div class="flex flex-col h-screen w-full overflow-hidden">

    <TopBar />

    <div class="flex flex-grow min-h-0 p-1 gap-1 w-full">

        <div class="w-[15%] h-full bg-white dark:bg-gray-800 rounded-md shadow overflow-y-auto">
            <LeftPanel on:requestopentab="{handleRequestOpenTab}" />
        </div>

        <div class="w-[40%] h-full flex flex-col gap-1">
            <div class="h-1/2 bg-white dark:bg-gray-800 rounded-md shadow flex flex-col">
                <MediaPlayer
                    bind:this="{mediaPlayerRef}"
                    bind:isTrimming="{isMediaPlayerTrimming}"
                    bind:trimStartTime="{mediaPlayerTrimStart}"
                    bind:trimEndTime="{mediaPlayerTrimEnd}"
                    isEditingSegment="{isSegmentEditingActive}"
                    editSegmentStartTime="{currentEditSegmentStart}"
                    editSegmentEndTime="{currentEditSegmentEnd}"
                />
            </div>
            <div class="h-1/2 min-h-0 bg-white dark:bg-gray-800 rounded-md shadow overflow-y-auto">
                 <EditableTranscript
                    bind:this="{editableTranscriptRef}"
                    bind:panelEditMode="{panelEditModeActive}"
                    bind:previewEditMode="{panelEditModeActive}"
                    on:navigate="{handlePanelNavigate}"
                    on:segmenteditfocus="{handleSegmentEditFocus}"
                    on:save="{handleSaveTranscript}"
                    on:toggleedit="{handleToggleEditMode}"
                 />
            </div>
        </div>

        <div class="w-[45%] h-full bg-white dark:bg-gray-800 rounded-md shadow overflow-y-auto">
             <RichTextPreview
                bind:this="{richTextPreviewRef}"
                bind:previewEditMode="{panelEditModeActive}"
                on:segmentclick="{handleSegmentClick}"
                on:toggleedit="{handleToggleEditMode}"
                on:requestopentab="{handleRequestOpenTab}"
                on:deletetranscriptsegment="{handleDeleteSegmentRequest}"
                on:insertnewsegment="{handleInsertSegmentRequest}"
                on:undo="{handleUndoRequest}"
                on:redo="{handleRedoRequest}"
             />
        </div>
    </div>

    <div class="h-24 w-full px-1 pb-1 flex-shrink-0">
        <div class="h-full bg-white dark:bg-gray-800 rounded-md shadow">
            <InteractiveWaveform
                isTrimming="{isMediaPlayerTrimming}"
                trimStartTime="{mediaPlayerTrimStart}"
                trimEndTime="{mediaPlayerTrimEnd}"
                isEditingSegment="{isSegmentEditingActive}"
                editSegmentStartTime="{currentEditSegmentStart}"
                editSegmentEndTime="{currentEditSegmentEnd}"
                on:navigate="{handlePanelNavigate}"
                on:trimupdate="{handleWaveformTrimUpdate}"
                on:segmentupdate="{handleWaveformSegmentUpdate}"
            />
        </div>
    </div>

    </div>

<style lang="postcss">
    .min-h-0 { min-height: 0; }
    /* Add any component-specific styles if needed, */
    /* but most layout is handled by Tailwind classes inline */
</style>