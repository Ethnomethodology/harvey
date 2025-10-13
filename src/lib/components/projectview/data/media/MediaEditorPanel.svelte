<!-- src/lib/components/projectview/data/media/MediaEditorPanel.svelte -->
<script>
    import { onMount, onDestroy, tick, createEventDispatcher } from 'svelte';
    import { get } from 'svelte/store';
    import { isMediaEditorOpen } from '$lib/stores/mediaEditorStore.js';
    import {
        project, // Store, aliased to projectStore below for clarity in functions
        setLoadedMediaNoteTranscriptData,
        setMediaNoteTranscriptLoadFailed,
        setMediaNoteTranscriptEditorContent,
        markMediaNoteTranscriptAsSaved,
        markMediaNoteTranscriptChangesDiscarded,
        setActiveMediaNoteEditorRef,
        clearActiveMediaNoteEditorRef,
        setDocumentHighlights,
        highlightsLastUpdated
    } from '$lib/stores/projectStore.js';
    import { invoke } from '@tauri-apps/api/core';
    import { confirm, message } from '@tauri-apps/plugin-dialog';
    import { basename, dirname, join } from '@tauri-apps/api/path';
    import { project as projectStore } from '$lib/stores/projectStore.js';
    import { handleTrimMediaConfirm, saveDocumentContent } from '$lib/services/projectService.js';

    import MediaPlayer from '../../shared/MediaPlayer.svelte';
    import LexicalEditor from '$lib/components/projectview/lexical/LexicalEditor.svelte';
    import InteractiveWaveform from '../../shared/InteractiveWaveform.svelte';
    import { activeLayout } from '$lib/stores/layoutStore.js';

    export let mediaPath = null;

    const dispatch = createEventDispatcher();

    $: console.log('[MediaEditorPanel] $projectStore.id value:', $projectStore.id);

    let showDataTrimUI = false;
    let currentTrimAudioBuffer = null; // Buffer for the active trim session
    let dataTrimStartTime = 0;
    let dataTrimEndTime = 0;

    const mediaToolbarConfig = {
      undo: true, redo: true, blockType: true, bold: true, italic: true,
      underline: true, strikethrough: true, link: true, insertMenu: false,
      indent: true, outdent: true, align: true, textColor: true, highlight: true,
      clearFormatting: true, search: true
    };

    let lexicalEditorRef;
    let mediaPlayerInDataRef;

    let localEditorJsonState = '';
    let isDataPlayerVideoHidden = false; // State for MediaPlayer's video visibility
    let associatedTranscriptPath = null;
    let transcriptName = 'N/A';

    let currentTranscriptJson = null;
    let initialTranscriptJson = null;
    let isTranscriptDirty = false;
    let isTranscriptLoading = true;
    let transcriptLoadError = null;
    
    $: isFileNotFoundInfo = transcriptLoadError === "INFO:FILE_NOT_FOUND";

    // LIVE MediaPlayer properties needed by InteractiveWaveform
    let dataMediaPlayerCurrentTime = 0;
    let dataMediaPlayerIsPlaying = false;

    const defaultEmptyJson = JSON.stringify({
        root: { children: [{ type: 'paragraph', version: 1, children: [], direction: null, format: '', indent: 0 }],
            direction: null, format: '', indent: 0, type: 'root', version: 1 }
    });

    const unsubscribeProject = projectStore.subscribe(p => {
        if (p.selectedMediaNotePath === mediaPath) {
            if (currentTranscriptJson !== p.currentMediaNoteTranscriptJson) {
                currentTranscriptJson = p.currentMediaNoteTranscriptJson;
                if (lexicalEditorRef && localEditorJsonState !== currentTranscriptJson) {
                    lexicalEditorRef.resetEditorState(currentTranscriptJson || defaultEmptyJson);
                    localEditorJsonState = currentTranscriptJson || defaultEmptyJson;
                }
            }
            if (initialTranscriptJson !== p.initialMediaNoteTranscriptJson) { initialTranscriptJson = p.initialMediaNoteTranscriptJson; }
            if (isTranscriptDirty !== p.isMediaNoteTranscriptDirty) { isTranscriptDirty = p.isMediaNoteTranscriptDirty; }
            if (isTranscriptLoading !== p.isMediaNoteTranscriptLoading) { isTranscriptLoading = p.isMediaNoteTranscriptLoading; }
            if (transcriptLoadError !== p.mediaNoteTranscriptError) { transcriptLoadError = p.mediaNoteTranscriptError; }
        }
    });

    async function loadTranscript(path) {
        if (!path) {
            setMediaNoteTranscriptLoadFailed(mediaPath, "Associated transcript/note path could not be determined.", false);
            return;
        }
        projectStore.update(p => {
            if (p.selectedMediaNotePath === mediaPath) { return { ...p, isMediaNoteTranscriptLoading: true, mediaNoteTranscriptError: null }; }
            return p;
        });
        localEditorJsonState = defaultEmptyJson;
        if (lexicalEditorRef) lexicalEditorRef.resetEditorState(defaultEmptyJson);
        try {
            const jsonContent = await invoke('load_transcript_json', { transcriptPath: path });
            if (!jsonContent || jsonContent.trim() === '') {
                setMediaNoteTranscriptLoadFailed(mediaPath, "File not found during load.", true);
            } else {
                let parsed = JSON.parse(jsonContent);
                if (parsed && parsed.root && parsed.root.children) { setLoadedMediaNoteTranscriptData(mediaPath, jsonContent); }
                else { throw new Error("Invalid Lexical JSON structure."); }
            }
        } catch (error) {
            const errorMessage = error.message || String(error);
            if (errorMessage.toLowerCase().includes('file not found') || errorMessage.toLowerCase().includes('json file not found')) {
                 setMediaNoteTranscriptLoadFailed(mediaPath, "File not found during load attempt.", true);
            } else { setMediaNoteTranscriptLoadFailed(mediaPath, errorMessage, false); }
        }
    }

    let previousActiveTranscriptPathInDataTab = null;
    $: if ($projectStore.activeTranscriptPathInDataTab && $projectStore.activeTranscriptPathInDataTab !== previousActiveTranscriptPathInDataTab) {
        previousActiveTranscriptPathInDataTab = $projectStore.activeTranscriptPathInDataTab;
        associatedTranscriptPath = $projectStore.activeTranscriptPathInDataTab;
        transcriptName = associatedTranscriptPath.split(/[\\/]/).pop();
        console.log(`[MediaEditorPanel] activeTranscriptPathInDataTab changed to: ${associatedTranscriptPath}`);
        loadTranscript(associatedTranscriptPath);
        showDataTrimUI = false; // Hide trim UI when switching transcripts
        currentTrimAudioBuffer = null;
    } else if (!$projectStore.activeTranscriptPathInDataTab && previousActiveTranscriptPathInDataTab) {
        // If activeTranscriptPathInDataTab becomes null (e.g., media file deselected)
        previousActiveTranscriptPathInDataTab = null;
        associatedTranscriptPath = null;
        transcriptName = 'N/A';
        currentTranscriptJson = null; initialTranscriptJson = null; isTranscriptDirty = false;
        isTranscriptLoading = false; transcriptLoadError = null; showDataTrimUI = false; currentTrimAudioBuffer = null;
        if (lexicalEditorRef) lexicalEditorRef.resetEditorState(defaultEmptyJson);
        localEditorJsonState = defaultEmptyJson;
        // Also clear the selectedMediaNotePath if it matches the previous one
        if (get(projectStore).selectedMediaNotePath === mediaPath) {
            projectStore.update(p => ({ ...p, selectedMediaNotePath: null, currentMediaNoteTranscriptJson: null, initialMediaNoteTranscriptJson: null, isMediaNoteTranscriptDirty: false, isMediaNoteTranscriptLoading: false, mediaNoteTranscriptError: null, activeMediaNoteEditorRef: null }));
        }
    }

    // Initial load logic (when mediaPath first becomes available) - REMOVED, now handled by projectStore.js

    function handleEditorChange(event) {
        const newJson = event.detail.jsonString;
        if (localEditorJsonState !== newJson) {
            localEditorJsonState = newJson;
            if (get(projectStore).selectedMediaNotePath === mediaPath) {
                if (isFileNotFoundInfo && initialTranscriptJson === defaultEmptyJson) {
                    projectStore.update(p => ({...p, initialMediaNoteTranscriptJson: defaultEmptyJson, mediaNoteTranscriptError: null}));
                }
                setMediaNoteTranscriptEditorContent(mediaPath, newJson);
            }
        }
	}

    function handleHighlightsChange(event) {
        const { highlights } = event.detail;
        setDocumentHighlights(highlights);
    }

    async function handleSave() {
        if (!mediaPath) { console.error("[MediaEditorPanel] Save Error: No mediaPath for context."); await message("Cannot save: No media file is active for this note.", { title: "Save Error", type: "error" }); return; }
        if (!associatedTranscriptPath) { console.error(`[MediaEditorPanel - ${mediaPath}] Save Error: Associated data path is not determined.`); await message("Cannot save: Note file location is unknown.", { title: "Save Error", type: "error" }); return; }
        if (isTranscriptLoading || (transcriptLoadError && !isFileNotFoundInfo)) { console.error(`[MediaEditorPanel - ${mediaPath}] Save Error: Cannot save while loading or in error state.`); await message(`Cannot save: ${isTranscriptLoading ? 'Note is still loading.' : `Note failed to load (${transcriptLoadError})`}`, { title: "Save Error", type: "error" }); return; }

        const finalJsonToSave = localEditorJsonState || defaultEmptyJson;
        projectStore.update(p => ({ ...p, statusMessage: `Saving data for ${transcriptName}...`}));

        try {
            // Use the centralized saveDocumentContent service which now handles highlights
            await saveDocumentContent(associatedTranscriptPath, finalJsonToSave);
            // The service now handles marking things as saved, but we can keep this for local state consistency if needed.
            if (get(projectStore).selectedMediaNotePath === mediaPath) {
                markMediaNoteTranscriptAsSaved(mediaPath, finalJsonToSave);
            }
            projectStore.update(p => ({ ...p, statusMessage: `Data for ${transcriptName} saved.`}));
        } catch (error) {
             // Error message is already shown by the service, so we just update the status
             projectStore.update(p => ({ ...p, statusMessage: `Error saving data for ${transcriptName}.`}));
        }
    }

    async function handleDiscard() {
        const currentStoreState = get(projectStore);
        const dirtyFlagForThisNote = currentStoreState.selectedMediaNotePath === mediaPath && currentStoreState.isMediaNoteTranscriptDirty;
        if (dirtyFlagForThisNote) {
            const userConfirmed = await confirm(`Discard unsaved changes to the data for "${mediaPath.split(/[\\/]/).pop()}"?`, { type: 'warning', title: 'Discard Changes' });
            if (userConfirmed) {
                if (get(projectStore).selectedMediaNotePath === mediaPath) { markMediaNoteTranscriptChangesDiscarded(mediaPath); }
            }
        }
    }

    $: {
        const activeTranscriptPath = get(project).activeTranscriptPathInDataTab;
        if (activeTranscriptPath && activeTranscriptPath !== associatedTranscriptPath) {
            associatedTranscriptPath = activeTranscriptPath;
            transcriptName = activeTranscriptPath.split(/[\\/]/).pop();
            loadTranscript(activeTranscriptPath);
        } else if (!activeTranscriptPath && mediaPath) {
            if (associatedTranscriptPath) {
                associatedTranscriptPath = null;
                transcriptName = 'N/A';
                setMediaNoteTranscriptLoadFailed(mediaPath, "No transcript selected.", true);
            }
        }
    }

    onMount(() => {
        setActiveMediaNoteEditorRef(mediaPath, self);
        isMediaEditorOpen.set(true);
        // Initial load is now handled by the reactive blocks
        showDataTrimUI = false;
        currentTrimAudioBuffer = null;
    });

	onDestroy(() => {
        const activeRefTuple = get(projectStore).activeMediaNoteEditorRef;
        if (activeRefTuple && activeRefTuple.path === mediaPath) { clearActiveMediaNoteEditorRef(); }
        isMediaEditorOpen.set(false);
        unsubscribeProject();
	});

    export function save() { return handleSave(); }
    export function discard() { return handleDiscard(); }
    export function resetEditorState(jsonString) {
        if (lexicalEditorRef) {
            lexicalEditorRef.resetEditorState(jsonString || defaultEmptyJson);
            localEditorJsonState = jsonString || defaultEmptyJson;
        }
    }
    export function getItemPath() { return mediaPath; }
    export function updateLiveTranscriptionText(text, isFinal, startTime, endTime) {
        if (lexicalEditorRef) {
            lexicalEditorRef.updateLiveTranscriptionText(text, isFinal, startTime, endTime);
        }
    }

    const self = { save, discard, resetEditorState, getItemPath, updateLiveTranscriptionText };

    function handleRequestDataTranscribe(event) {
        dispatch('requestTranscriptionTabWithMedia', { mediaPath: event.detail.mediaPath });
    }

    function handleRequestDataTrim(event) {
        if (showDataTrimUI) {
            showDataTrimUI = false;
            currentTrimAudioBuffer = null;
            console.log('[MediaEditorPanel] Trim UI explicitly hidden by button toggle.');
        } else {
            const duration = event.detail.duration;
            const audioBuffer = event.detail.audioBuffer;
            const isReady = event.detail.isReady;

            if (isReady && audioBuffer && duration > 0) {
                dataTrimStartTime = 0;
                dataTrimEndTime = duration;
                currentTrimAudioBuffer = audioBuffer;
                showDataTrimUI = true;
                console.log(`[MediaEditorPanel] Trim UI shown based on event data. Duration: ${duration}, Buffer Present: ${!!audioBuffer}, isReady Signal from Player: ${isReady}`);
            } else {
                showDataTrimUI = false;
                currentTrimAudioBuffer = null;
                console.error(`[MediaEditorPanel] Error: MediaPlayer event indicated not ready or event data invalid. Duration from event: ${duration}, Buffer from event: ${!!audioBuffer}, isReady signal from event: ${isReady}`);
                alert("MediaPlayer reported not ready or essential data was missing from the event. Cannot show trim UI.");
            }
        }
    }

    function handleWaveformTrimUpdate(event) {
        if (event.detail) {
            dataTrimStartTime = event.detail.startTime;
            dataTrimEndTime = event.detail.endTime;
        }
    }

    async function handleConfirmDataTrim() {
        if (!mediaPath) { console.error("Trim Error: No mediaPath specified."); await message("Error: No media file is specified for trimming.", { title: "Trim Error", type: "error" }); return; }
        if (dataTrimEndTime <= dataTrimStartTime) { await message("Error: Trim end time must be after start time.", { title: "Trim Error", type: "error" }); return; }
        projectStore.update(p => ({ ...p, isLoading: true, statusMessage: 'Trimming media in data...' }));
        try {
            await handleTrimMediaConfirm(mediaPath, dataTrimStartTime, dataTrimEndTime); // This is an existing external function call

            const fileName = await basename(mediaPath);
            let mediaTypeFolder = 'Media'; // Default or could be 'Output' or similar if type unknown
            if (fileName) {
                const lowerFileName = fileName.toLowerCase();
                if (lowerFileName.endsWith('.mp3') || lowerFileName.endsWith('.wav') || lowerFileName.endsWith('.m4a') || lowerFileName.endsWith('.ogg') || lowerFileName.endsWith('.aac')) {
                    mediaTypeFolder = 'Audios';
                } else if (lowerFileName.endsWith('.mp4') || lowerFileName.endsWith('.mov') || lowerFileName.endsWith('.avi') || lowerFileName.endsWith('.webm') || lowerFileName.endsWith('.mkv')) {
                    mediaTypeFolder = 'Videos';
                }
            }

            projectStore.update(p => ({ ...p, isLoading: false, statusMessage: `Trimmed ${fileName} saved to ${mediaTypeFolder}. Reloading media...` }));
            await message(`Trimmed ${fileName} saved to ${mediaTypeFolder}.`, { title: 'Trim Successful' });

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
            projectStore.update(p => ({ ...p, isLoading: false, error: `Trim failed: ${error.message || error}`, statusMessage: 'Trim failed.' }));
            await message(`Failed to trim media: ${error.message || error}`, { title: 'Trim Failed', type: 'error' });
        }
    }

    function handleCancelDataTrim() {
        showDataTrimUI = false;
        currentTrimAudioBuffer = null;
        dataTrimStartTime = 0;
        dataTrimEndTime = 0;
        console.log('[MediaEditorPanel] Trim cancelled. UI hidden, times reset, buffer cleared.');
    }
</script>

<div class="flex flex-col h-full w-full bg-white dark:bg-surface-2">
    <div
        class="border-b border-gray-200 dark:border-border flex flex-col
               {!isDataPlayerVideoHidden ? 'h-1/2' : 'h-auto flex-shrink-0'}"
    >
        {#if mediaPath}
            <MediaPlayer
                bind:this={mediaPlayerInDataRef}
                bind:localCurrentTime={dataMediaPlayerCurrentTime}
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
                on:mediaLoadError={(e) => projectStore.update(p => ({...p, statusMessage: `Error loading media in data: ${e.detail.error}`}))}
                class="{!isDataPlayerVideoHidden ? 'flex-grow min-h-0' : ''}"
            />
        {:else}
            <div class="w-full h-full bg-black flex items-center justify-center text-gray-500 dark:text-d-gray-400">
                <span>Media player requires a path.</span>
            </div>
        {/if}
    </div>

    <div
        class="min-h-0 overflow-hidden bg-white dark:bg-dark-bg-form-field {!isDataPlayerVideoHidden ? 'h-1/2' : ''}"
        class:flex-grow={isDataPlayerVideoHidden}
    >
        {#if showDataTrimUI && mediaPath}
            <div class="inline-trim-ui-wrapper">
                <div class="flex justify-between items-center mb-1">
                    <p class="text-xs text-gray-600 dark:text-d-gray-400">
                        Adjust start and end times by dragging the red bars on both sides: {dataTrimStartTime.toFixed(3)}s — {dataTrimEndTime.toFixed(3)}s
                    </p>
                    <div class="space-x-2">
                        <button class="bg-blue-600 hover:bg-blue-700 text-white text-xs font-semibold py-1 px-3 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-opacity-50" on:click={handleConfirmDataTrim}>Trim</button>
                        <button class="bg-gray-500 hover:bg-gray-600 text-white text-xs font-semibold py-1 px-3 focus:outline-none focus:ring-2 focus:ring-gray-400 focus:ring-opacity-50" on:click={handleCancelDataTrim}>Cancel</button>
                    </div>
                </div>
                {#if currentTrimAudioBuffer && dataTrimEndTime > 0}
                    <div class="waveform-container w-full h-[75px] bg-gray-100 dark:bg-d-gray-700 overflow-hidden">
                        <InteractiveWaveform
                            startZoomedOut={true}
                            externalAudioBuffer={currentTrimAudioBuffer}
                            externalCurrentTime={dataMediaPlayerCurrentTime}
                            externalDuration={mediaPlayerInDataRef?.localDuration}
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
                    <div class="w-full h-[100px] flex items-center justify-center bg-gray-100 dark:bg-d-gray-700 rounded text-xs text-gray-500">
                        Audio data not available for waveform.
                    </div>
                {/if}
            </div>
        {/if}

        {#if isTranscriptLoading && mediaPath}
            <div class="flex-grow flex items-center justify-center text-gray-500 dark:text-d-gray-300 p-4">
                Loading data for <span class="font-semibold ml-1">{transcriptName}</span>...
            </div>
        {:else if transcriptLoadError && mediaPath}
            {#if isFileNotFoundInfo}
                <div class="flex-grow flex flex-col items-center justify-center text-blue-600 dark:text-blue-400 p-4 text-center">
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-10 w-10 mb-2 opacity-70" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z" /></svg>
                    <p class="font-semibold">No Transcription Yet</p>
                    <p class="text-xs mt-1">
                        To generate a transcript, you can use the main "Transcribe" feature in the Transcriptions tab.
                    </p>
                </div>
            {:else}
                <div class="flex-grow flex flex-col items-center justify-center text-orange-600 dark:text-orange-400 p-4 text-center">
                    <p class="font-semibold">Error Loading Data</p>
                    <p class="text-xs mt-1">{transcriptLoadError}</p>
                    <p class="text-xs mt-2">Please check the file or try again.</p>
                </div>
            {/if}
        {:else if !mediaPath}
            <div class="flex-grow flex items-center justify-center text-gray-500 dark:text-d-gray-300 p-4">
                Select an audio or video file from the Data panel to view its player and data.
            </div>
        {:else}
            <div class="lexical-editor-wrapper-style w-full h-full dark:text-gray-100 layout-{$activeLayout}">
                {#key mediaPath}
                    <LexicalEditor
                        bind:this={lexicalEditorRef}
                        initialJson={currentTranscriptJson || defaultEmptyJson}
                        editable={true}
                        placeholder="Enter data for this media file..."
                        on:change={handleEditorChange}
                        on:highlightschange={handleHighlightsChange}
                        on:highlightssaved={() => highlightsLastUpdated.set(new Date())}
                        toolbarConfig={mediaToolbarConfig}
                        activeLayout={$activeLayout}
                        documentPath={associatedTranscriptPath}
                        documentHighlights={$project.currentDocumentHighlights}
                    />
                {/key}
            </div>
        {/if}
    </div>
</div>

<style lang="postcss">
    .lexical-editor-wrapper-style {
        display: flex;
        flex-direction: column;
        @apply border-none shadow-none overflow-hidden;
    }
    .lexical-editor-wrapper-style > :global(.lexical-editor-root) {
        flex-grow: 1;
        min-height: 0;
        border: none !important;
        border-radius: 0 !important;
        box-shadow: none !important;
        overflow: hidden;
    }
    .lexical-editor-wrapper-style > :global(.lexical-editor-root > .lexical-wrapper) {
        overflow-y: auto;
        height: 100%;
        @apply p-3;
    }
    .lexical-editor-wrapper-style :global(.lexical-content) {
        @apply leading-normal whitespace-pre-wrap break-words;
        min-height: unset !important;
        font-family: Arial, Helvetica, sans-serif;
        font-size: 12pt;
        line-height: 1.5;
    }
    .lexical-editor-wrapper-style :global(.lexical-content p) {
        @apply mt-0 mb-0;
    }

    .lexical-editor-wrapper-style :global(.lexical-content table) {
        border-collapse: collapse;
        border-spacing: 0;
        width: 100%;
        border: 1px solid #ccc;
        margin-bottom: 1rem;
        table-layout: fixed;
    }
    .lexical-editor-wrapper-style :global(.lexical-content th),
    .lexical-editor-wrapper-style :global(.lexical-content td) {
        border: 1px solid #ccc;
        padding: 0.2rem 5.75pt;
        text-align: left;
        vertical-align: top;
        font-family: Arial, Helvetica, sans-serif;
        font-size: 12pt;
        line-height: 1.5;
        word-break: break-word;
    }
    .lexical-editor-wrapper-style :global(.lexical-content th) {
        background-color: #f0f0f0;
        font-weight: 600;
    }
    .lexical-editor-wrapper-style :global(.lexical-content th p),
    .lexical-editor-wrapper-style :global(.lexical-content td p) {
        @apply mt-0 mb-0;
    }

    .lexical-editor-wrapper-style :global(table.editor-table tr.editor-table-row:nth-child(2)) {
    background-color: #f2f2f2;
}

    /* =================================================================== */
    /* STYLES FOR LAYOUT 1 (DEFAULT)                                       */
    /* =================================================================== */
    .lexical-editor-wrapper-style.layout-Layout1 :global(.lexical-content table th:nth-child(1)),
    .lexical-editor-wrapper-style.layout-Layout1 :global(.lexical-content table td:nth-child(1)) {
        width: 5%;
    }
    .lexical-editor-wrapper-style.layout-Layout1 :global(.lexical-content table th:nth-child(2)),
    .lexical-editor-wrapper-style.layout-Layout1 :global(.lexical-content table td:nth-child(2)) {
        width: 15%;
    }
    .lexical-editor-wrapper-style.layout-Layout1 :global(.lexical-content table th:nth-child(3)),
    .lexical-editor-wrapper-style.layout-Layout1 :global(.lexical-content table td:nth-child(3)) {
        width: 15%;
    }
    .lexical-editor-wrapper-style.layout-Layout1 :global(.lexical-content table th:nth-child(4)),
    .lexical-editor-wrapper-style.layout-Layout1 :global(.lexical-content table td:nth-child(4)) {
        width: 65%;
    }

    /* =================================================================== */
    /* STYLES FOR LAYOUT 2                                                 */
    /* =================================================================== */
    .lexical-editor-wrapper-style.layout-Layout2 :global(.lexical-content table) {
        table-layout: auto;
        border: none;
    }
    .lexical-editor-wrapper-style.layout-Layout2 :global(.lexical-content table tr) {
        display: flex;
        flex-wrap: wrap;
        border: none;
    }
    .lexical-editor-wrapper-style.layout-Layout2 :global(.lexical-content table th),
    .lexical-editor-wrapper-style.layout-Layout2 :global(.lexical-content table td) {
        box-sizing: border-box;
        padding: 8px;
        border: 1px solid #ccc;
    }
    .lexical-editor-wrapper-style.layout-Layout2 :global(.lexical-content table th:nth-child(odd)),
    .lexical-editor-wrapper-style.layout-Layout2 :global(.lexical-content table td:nth-child(odd)) {
        flex: 1 0 25%;
    }
    .lexical-editor-wrapper-style.layout-Layout2 :global(.lexical-content table th:nth-child(even)),
    .lexical-editor-wrapper-style.layout-Layout2 :global(.lexical-content table td:nth-child(even)) {
        flex: 1 0 75%;
        margin-left: -1px;
    }
    .lexical-editor-wrapper-style.layout-Layout2 :global(.lexical-content table th:nth-child(n+3)),
    .lexical-editor-wrapper-style.layout-Layout2 :global(.lexical-content table td:nth-child(n+3)) {
        margin-top: -1px;
    }

    /* =================================================================== */
    /* STYLES FOR LAYOUT 3                                                 */
    /* =================================================================== */
    .lexical-editor-wrapper-style.layout-Layout3 :global(.lexical-content table) {
        table-layout: auto;
        border: none;
    }
    .lexical-editor-wrapper-style.layout-Layout3 :global(.lexical-content table tr) {
        display: flex;
        flex-wrap: wrap;
        border: none;
    }
    .lexical-editor-wrapper-style.layout-Layout3 :global(.lexical-content table th:nth-child(1)),
    .lexical-editor-wrapper-style.layout-Layout3 :global(.lexical-content table td:nth-child(1)) {
        display: none;
    }
    .lexical-editor-wrapper-style.layout-Layout3 :global(.lexical-content table th:nth-child(n+2)),
    .lexical-editor-wrapper-style.layout-Layout3 :global(.lexical-content table td:nth-child(n+2)) {
        box-sizing: border-box;
        padding: 8px;
        border: 1px solid #ccc;
    }
    .lexical-editor-wrapper-style.layout-Layout3 :global(.lexical-content table th:nth-child(2)),
    .lexical-editor-wrapper-style.layout-Layout3 :global(.lexical-content table td:nth-child(2)) {
        flex: 1 0 25%;
    }
    .lexical-editor-wrapper-style.layout-Layout3 :global(.lexical-content table th:nth-child(3)),
    .lexical-editor-wrapper-style.layout-Layout3 :global(.lexical-content table td:nth-child(3)) {
        flex: 1 0 75%;
        margin-left: -1px;
    }
    .lexical-editor-wrapper-style.layout-Layout3 :global(.lexical-content table th:nth-child(4)),
    .lexical-editor-wrapper-style.layout-Layout3 :global(.lexical-content table td:nth-child(4)) {
        flex: 1 0 100%;
        margin-top: -1px;
    }

    /* =================================================================== */
    /* STYLES FOR LAYOUT 4                                                 */
    /* =================================================================== */
    .lexical-editor-wrapper-style.layout-Layout4 :global(.lexical-content table) {
        table-layout: auto;
        border: none;
    }
    .lexical-editor-wrapper-style.layout-Layout4 :global(.lexical-content table tr) {
        display: flex;
        flex-wrap: nowrap;
        border: none;
    }
    .lexical-editor-wrapper-style.layout-Layout4 :global(.lexical-content table th:nth-child(-n+2)),
    .lexical-editor-wrapper-style.layout-Layout4 :global(.lexical-content table td:nth-child(-n+2)) {
        display: none;
    }
    .lexical-editor-wrapper-style.layout-Layout4 :global(.lexical-content table th:nth-child(n+3)),
    .lexical-editor-wrapper-style.layout-Layout4 :global(.lexical-content table td:nth-child(n+3)) {
        box-sizing: border-box;
        padding: 8px;
        border: 1px solid #ccc;
    }
    .lexical-editor-wrapper-style.layout-Layout4 :global(.lexical-content table th:nth-child(3)),
    .lexical-editor-wrapper-style.layout-Layout4 :global(.lexical-content table td:nth-child(3)) {
        flex: 1 0 25%;
    }
    .lexical-editor-wrapper-style.layout-Layout4 :global(.lexical-content table th:nth-child(4)),
    .lexical-editor-wrapper-style.layout-Layout4 :global(.lexical-content table td:nth-child(4)) {
        flex: 1 0 75%;
        margin-left: -1px;
    }

    /* =================================================================== */
    /* STYLES FOR LAYOUT 5                                                 */
    /* =================================================================== */
    .lexical-editor-wrapper-style.layout-Layout5 :global(.lexical-content table) {
        table-layout: auto;
        border: none;
    }
    .lexical-editor-wrapper-style.layout-Layout5 :global(.lexical-content table tr) {
        display: flex;
        flex-wrap: nowrap;
        border: none;
    }
    .lexical-editor-wrapper-style.layout-Layout5 :global(.lexical-content table th:nth-child(-n+3)),
    .lexical-editor-wrapper-style.layout-Layout5 :global(.lexical-content table td:nth-child(-n+3)) {
        display: none;
    }
    .lexical-editor-wrapper-style.layout-Layout5 :global(.lexical-content table th:nth-child(4)),
    .lexical-editor-wrapper-style.layout-Layout5 :global(.lexical-content table td:nth-child(4)) {
        flex: 1 0 100%;
        box-sizing: border-box;
        padding: 8px;
        border: 1px solid #ccc;
    }

    /* =================================================================== */
    /* COMMON RULE TO COLLAPSE ROWS VERTICALLY                             */
    /* =================================================================== */
    .lexical-editor-wrapper-style.layout-Layout2 :global(.lexical-content table tr + tr),
    .lexical-editor-wrapper-style.layout-Layout3 :global(.lexical-content table tr + tr),
    .lexical-editor-wrapper-style.layout-Layout4 :global(.lexical-content table tr + tr),
    .lexical-editor-wrapper-style.layout-Layout5 :global(.lexical-content table tr + tr) {
        margin-top: -1px;
    }


    .lexical-editor-wrapper-style-placeholder {
        display: flex;
        flex-direction: column;
        @apply border border-gray-300 dark:border-border overflow-hidden;
    }
     .lexical-editor-wrapper-style-placeholder.is-disabled {
        @apply bg-gray-100 border-gray-300 opacity-70 dark:bg-d-gray-700 dark:border-d-gray-500 dark:opacity-70;
    }

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
        box-shadow: 0 -2px 10px rgba(0,0,0,0.1);
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