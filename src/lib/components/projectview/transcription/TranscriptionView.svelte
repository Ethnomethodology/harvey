<!-- src/lib/components/projectview/transcription/TranscriptionView.svelte -->
<script>
    import { tick, createEventDispatcher, onMount, onDestroy } from "svelte";
    import { get } from "svelte/store";
    import { slide } from "svelte/transition"; // Import slide transition
    import { project } from "$lib/stores/projectStore.js";
    import {
        transcriptStore,
        deleteTranscriptSegment,
        undoTranscriptChange,
        redoTranscriptChange,
        insertTranscriptSegment,
        splitTranscriptSegment,
        mergeTranscriptSegments,
        selectMedia,
        markTranscriptAsSaved, // <-- Added this import
        deactivateDualMode,
        updatePlayerCurrentSegmentIndex,
    } from "$lib/stores/transcriptStore.js";
    import {
        saveTranscriptData,
        requestTranscription as requestTranscriptionService, // Renamed to avoid conflict
        convertAndSaveTranscriptAsDoc,
        loadTranscriptFile,
        replaceTranscriptText,
        replaceAllTranscriptText,
    } from "$lib/services/projectService.js";
    import { isLexicalEditMode } from "$lib/stores/mediaEditorStore.js";

    import { confirm, message } from "@tauri-apps/plugin-dialog";

    import TopBar from "./TopBar.svelte";
    import LeftPanel from "./LeftPanel.svelte";
    import panelStateStore from "$lib/stores/panelStateStore.js";
    import waveformLayoutStore from "$lib/stores/waveformLayoutStore.js"; // Added
    import MediaPlayer from "../shared/MediaPlayer.svelte";
    import InteractiveWaveform from "../shared/InteractiveWaveform.svelte"; // Added for horizontal waveform
    import VerticalWaveform from "./VerticalWaveform.svelte";
    import EditableTranscript from "./EditableTranscript.svelte";
    import RichTextPreview from "./RichTextPreview.svelte";
    import UnsavedChangesModal from "$lib/components/projectview/modals/UnsavedChangesModal.svelte"; // <-- Added this import
    import ManualSettingsModal from "$lib/components/projectview/modals/ManualSettingsModal.svelte";
    import { updateManualSegmentSettings } from "$lib/stores/transcriptStore.js";

    const dispatch = createEventDispatcher();

    export let mediaPlayerRef = null;
    let verticalWaveformRef = null;
    let horizontalWaveformRef = null; // Added for horizontal waveform
    let verticalWaveformWidthPx = 0; // To store the actual pixel width of the vertical waveform panel
    const HORIZONTAL_WAVEFORM_DEFAULT_HEIGHT_PX = 115; // Increased height to allow tooltips to render above and below without clipping
    let horizontalWaveformContainerHeightPx =
        HORIZONTAL_WAVEFORM_DEFAULT_HEIGHT_PX; // Initialize with default

    $: {
        if (currentWaveformLayout === "vertical") {
            horizontalWaveformContainerHeightPx = verticalWaveformWidthPx;
        } else if (currentWaveformLayout === "horizontal") {
            horizontalWaveformContainerHeightPx =
                HORIZONTAL_WAVEFORM_DEFAULT_HEIGHT_PX;
        } else {
            // 'none'
            horizontalWaveformContainerHeightPx = 0;
        }
    }

    let editableTranscriptRef;
    let richTextPreviewRef;
    let topBarRef;
    let leftPanelRef;

    // Reactive state for left panel visibility from store

    let currentWaveformLayout; // Will be updated by store subscription
    const unsubscribeWaveformLayout = waveformLayoutStore.subscribe((value) => {
        currentWaveformLayout = value;
        console.log(
            "[TranscriptionView] currentWaveformLayout updated to:",
            currentWaveformLayout,
        );
    });

    onDestroy(() => {
        if (unsubscribeWaveformLayout) unsubscribeWaveformLayout();
    });

    // Reactive statements for panel widths
    $: middlePanelWidthClass = (() => {
        if (currentWaveformLayout === "vertical") {
            return !$panelStateStore.transcriptionPanelCollapsed
                ? "w-[40%]"
                : "w-[47.5%]";
        } else {
            // 'horizontal' or 'none'
            return !$panelStateStore.transcriptionPanelCollapsed
                ? "w-[42.5%]"
                : "w-[50%]";
        }
    })();

    $: rightPanelWidthClass = (() => {
        if (currentWaveformLayout === "vertical") {
            return !$panelStateStore.transcriptionPanelCollapsed
                ? "w-[40%]"
                : "w-[47.5%]";
        } else {
            // 'horizontal' or 'none'
            return !$panelStateStore.transcriptionPanelCollapsed
                ? "w-[42.5%]"
                : "w-[50%]";
        }
    })();

    // State for MediaPlayer within THIS TranscriptionView
    // These are bound to the MediaPlayer's props.
    let isMediaPlayerTrimming = false;
    let mediaPlayerTrimStart = 0;
    let mediaPlayerTrimEnd = 0;

    let isMediaPlayerHidden = false; // New state variable

    // Logic to show media player by default if the new media is a video
    $: {
        const selectedMedia = $transcriptStore.selectedMediaFile;
        if (selectedMedia && selectedMedia.path) {
            const extension = selectedMedia.path
                .split(".")
                .pop()
                ?.toLowerCase();
            const videoExtensions = ["mp4", "mov", "webm", "avi", "mkv"];
            if (videoExtensions.includes(extension)) {
                isMediaPlayerHidden = false;
            }
        }
    }

    let isSegmentEditingActive = false;
    let currentEditSegmentStart = 0;
    let currentEditSegmentEnd = 0;

    $: panelEditModeActive = $isLexicalEditMode;

    let wasPlayingBeforeEdit = false;

    // State for ManualSettingsModal
    let isManualSettingsModalOpen = false;

    async function handlePreviousRequest() {
        editableTranscriptRef?.previous();
    }

    async function handleNextRequest() {
        editableTranscriptRef?.next();
    }

    async function handleSegmentClick(event) {
        const index = event.detail;
        const segment = get(transcriptStore).segments?.[index];
        if (segment && typeof segment.start_time === "number") {
            // Commit any pending edits in the editable transcript before switching/saving
            if (panelEditModeActive && editableTranscriptRef) {
                editableTranscriptRef.commitCurrentSegmentEdits();
                await tick();
            }

            // Sync store index
            updatePlayerCurrentSegmentIndex(index);

            editableTranscriptRef?.loadSegment?.(index);
            if (mediaPlayerRef) {
                mediaPlayerRef.seekTo(segment.start_time);
            }
            if (verticalWaveformRef) {
                verticalWaveformRef.scrollToTime(segment.start_time);
            }
            if (horizontalWaveformRef) {
                horizontalWaveformRef.scrollToTime(segment.start_time);
            }
        } else {
            console.warn(
                `[TranscriptionView] Invalid segment data for index ${index} on click.`,
            );
        }
    }

    async function handlePanelNavigate(event) {
        const detail = event.detail;

        // Commit any pending edits in the editable transcript before navigating/saving
        if (panelEditModeActive && editableTranscriptRef) {
            editableTranscriptRef.commitCurrentSegmentEdits();
            await tick();
        }

        if (detail && detail.action) {
            if (mediaPlayerRef) {
                switch (detail.action) {
                    case "toggle-play":
                        mediaPlayerRef.handleTogglePlay();
                        break;
                    case "rewind":
                        mediaPlayerRef.rewind10s();
                        break;
                    case "forward":
                        mediaPlayerRef.forward10s();
                        break;
                    case "speed-up":
                        mediaPlayerRef.changeSpeed(1);
                        break;
                    case "speed-down":
                        mediaPlayerRef.changeSpeed(-1);
                        break;
                }
            }
            return;
        }

        if (detail && typeof detail.time === "number") {
            if (mediaPlayerRef) mediaPlayerRef.seekTo(detail.time);
        } else if (detail && typeof detail.index === "number") {
            const segment = get(transcriptStore).segments?.[detail.index];
            if (segment && mediaPlayerRef) {
                const seekTime = isSegmentEditingActive
                    ? Math.max(
                          currentEditSegmentStart,
                          Math.min(
                              segment.start_time,
                              currentEditSegmentEnd - 0.001,
                          ),
                      )
                    : segment.start_time;
                mediaPlayerRef.seekTo(seekTime);
            }
        } else {
            console.warn(
                "[TranscriptionView] Unexpected navigation event detail:",
                detail,
            );
        }
    }

    function handleGlobalKeydown(event) {
        const isMac =
            typeof window !== "undefined" &&
            navigator.platform.toUpperCase().indexOf("MAC") >= 0;
        const modKey = isMac ? event.metaKey : event.ctrlKey;
        if (modKey && event.key.toLowerCase() === "e") {
            // Check if user is typing in some other input outside transcriptional panels if needed
            // For now, E is safe to toggle
            event.preventDefault();
            handleToggleEditMode();
        }
    }

    // This handler is for when the MediaPlayer component itself enters trim mode (e.g., user clicks its trim button)
    // or when its bound `isTrimming` prop changes.
    function handleMediaPlayerTrimModeEntered(event) {
        isMediaPlayerTrimming = true; // Ensure TranscriptionView's state is synced
        mediaPlayerTrimStart = event.detail.startTime;
        mediaPlayerTrimEnd = event.detail.endTime;
    }

    function handleMediaPlayerTrimModeCancelled() {
        isMediaPlayerTrimming = false;
    }

    function handleWaveformTrimUpdate(event) {
        const { startTime, endTime } = event.detail;
        if (mediaPlayerRef) mediaPlayerRef.updateTrimTimes(startTime, endTime);
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
            console.error(
                "[TranscriptionView] EditableTranscript ref not available for waveform update.",
            );
        }
    }

    export async function handleToggleEditMode() {
        console.log(
            "[TranscriptionView] handleToggleEditMode called. Current panelEditModeActive:",
            panelEditModeActive,
        );

        if (panelEditModeActive) {
            // Explicitly exiting edit mode
            isLexicalEditMode.set(false);
        } else {
            // Explicitly entering edit mode
            isLexicalEditMode.set(true);
            await tick();
            editableTranscriptRef?.focusEditor?.();
        }
    }

    /**
     * Exits edit mode if it's currently active.
     */
    export async function exitEditModeIfActive() {
        if (panelEditModeActive || get(transcriptStore).transcriptDirty) {
            await handleSaveTranscript();
        }
    }

    export async function enterManualEditMode() {
        console.log("[TranscriptionView] enterManualEditMode called.");
        panelEditModeActive = true;

        // Ensure store knows we are on segment 0
        updatePlayerCurrentSegmentIndex(0);

        await tick();

        // Check if segments are available in the store
        const store = get(transcriptStore);
        if (store.segments && store.segments.length > 0) {
            // Add a small delay to ensure EditableTranscript has received the store update
            // and updated its local 'segments' array via subscription.
            setTimeout(async () => {
                // Automatically select the first segment
                await handleSegmentClick({ detail: 0 });
                editableTranscriptRef?.focusEditor?.();
            }, 100);
        } else {
            console.warn(
                "[TranscriptionView] enterManualEditMode: No segments found in store to select.",
            );
        }
    }

    export async function handleSaveTranscript(isAutoSave = false) {
        // CRITICAL: Commit any pending edits in the editable transcript before saving
        if (panelEditModeActive && editableTranscriptRef) {
            editableTranscriptRef.commitCurrentSegmentEdits();
            await tick(); // Allow store to update from the commit
        }

        if (!get(transcriptStore).transcriptDirty) {
            return;
        }

        const tsStore = get(transcriptStore);

        try {
            project.update((p) => ({
                ...p,
                isLoading: true,
                statusMessage: "Saving transcript...",
            })); // Global loading state
            await saveTranscriptData(); // This service will use get(transcriptStore) for currentTranscriptPath and segments
            project.update((p) => ({
                ...p,
                isLoading: false,
                statusMessage: "Transcript saved.",
            })); // Global status
        } catch (error) {
            const errorMsg =
                error instanceof Error ? error.message : String(error);
            project.update((p) => ({
                ...p,
                isLoading: false,
                error: `Save failed: ${errorMsg}`,
                statusMessage: "Save failed.",
            })); // Global error
            await message(`Error saving transcript: ${errorMsg}`, {
                title: "Save Error",
                type: "error",
            });
            throw error;
        }
    }

    function handleRequestTranscriptionEvent() {
        requestTranscriptionService();
    }

    async function handleConvertToDocumentEvent() {
        if (get(transcriptStore).transcriptDirty) {
            await handleSaveTranscript();
        }
        showConfirmConversionModal = true;
    }

    async function confirmConvertToDocument() {
        try {
            project.update((p) => ({
                ...p,
                statusMessage: "Converting to document...",
            }));
            const newDocPath = await convertAndSaveTranscriptAsDoc();
            project.update((p) => ({
                ...p,
                statusMessage: "Converted to document successfully.",
            }));
            await message(`Transcript converted and saved as a new document.`, {
                title: "Conversion Successful",
            });
            dispatch("requestopentab", {
                tabName: "data",
                loadNotePath: newDocPath,
            });
        } catch (error) {
            await message(`Failed to convert: ${error.message || error}`, {
                title: "Conversion Error",
                type: "error",
            });
            project.update((p) => ({
                ...p,
                statusMessage: "Conversion failed.",
            }));
        } finally {
            showConfirmConversionModal = false;
        }
    }

    export function handleDeleteSegmentRequest(event) {
        const indexToDelete = event.detail;
        if (typeof indexToDelete === "number")
            deleteTranscriptSegment(indexToDelete);
    }
    export function handleSplitSegmentRequest(event) {
        const indexToSplit = event.detail;
        if (typeof indexToSplit === "number")
            splitTranscriptSegment(indexToSplit);
    }
    export function handleMergeSegmentRequest(event) {
        const indexToMerge = event.detail;
        if (typeof indexToMerge === "number")
            mergeTranscriptSegments(indexToMerge);
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
        // Detail can either be a simple index (from EditableTranscript shortcut)
        // or a full object with timestamps (from RichTextPreview manual calculation flow)
        if (typeof event.detail === "number") {
            const index = event.detail;
            if (
                richTextPreviewRef &&
                typeof richTextPreviewRef.handleInsertNewSegment === "function"
            ) {
                richTextPreviewRef.handleInsertNewSegment(index + 1); // Pass index + 1 to insert AFTER current
            } else {
                console.warn(
                    "[TranscriptionView] richTextPreviewRef.handleInsertNewSegment is not available.",
                );
            }
            return;
        }

        const { index, startTime, endTime, speaker } = event.detail || {};
        if (
            typeof index !== "number" ||
            typeof startTime !== "number" ||
            typeof endTime !== "number" ||
            endTime <= startTime
        )
            return;

        const newSegment = {
            start_time: startTime,
            end_time: endTime,
            speaker: speaker || "Unknown",
            text: JSON.stringify({
                root: {
                    children: [
                        {
                            type: "paragraph",
                            version: 1,
                            children: [],
                            direction: null,
                            format: "",
                            indent: 0,
                        },
                    ],
                    type: "root",
                    version: 1,
                    direction: null,
                    format: "",
                    indent: 0,
                },
            }),
        };
        insertTranscriptSegment(index, newSegment);
    }

    export function activateTrimModeOnPlayer() {
        if (
            mediaPlayerRef &&
            typeof mediaPlayerRef.enterTrimMode === "function"
        ) {
            mediaPlayerRef.enterTrimMode();
            // The `enterTrimMode` in MediaPlayer should ideally set its `isTrimming` prop.
            // Since `isMediaPlayerTrimming` is bound to MediaPlayer's `isTrimming` prop,
            // this should update `isMediaPlayerTrimming` here automatically.
            // We directly set it here to be certain the UI within TranscriptionView updates.
            isMediaPlayerTrimming = true;
        } else {
            console.warn(
                "[TranscriptionView] Could not activate trim mode: mediaPlayerRef or enterTrimMode method not found.",
            );
        }
    }

    $: if (mediaPlayerRef && mediaPlayerRef.videoElement) {
        const video = mediaPlayerRef.videoElement;
        if (isSegmentEditingActive) {
            if (!wasPlayingBeforeEdit && !video.paused) {
                wasPlayingBeforeEdit = true;
                try {
                    video.pause();
                } catch (e) {
                    console.warn("Error pausing on edit focus:", e);
                }
            }
        } else {
            if (wasPlayingBeforeEdit) {
                if (video.paused) {
                    try {
                        video.play().catch(console.error);
                    } catch (e) {
                        console.warn("Error resuming on edit blur:", e);
                    }
                }
                wasPlayingBeforeEdit = false;
            }
        }
    }

    function forwardLeftPanelEvents(event) {
        if (event.type === "requestopentab") {
            dispatch("requestopentab", event.detail);
        } else if (event.type === "requestmediaselection") {
            dispatch("requestmediaselection", event.detail);
        }
    }

    // Handlers for Manual Transcription Settings
    function handleRequestManualSettings() {
        isManualSettingsModalOpen = true;
    }

    function handleManualSettingsConfirm(event) {
        const { duration, speakerMode } = event.detail;
        updateManualSegmentSettings({ duration, speakerMode });
    }

    function handleReplaceTranscriptText(event) {
        const { segmentIndex, isPrimary, find, replace, offset, length } =
            event.detail;
        replaceTranscriptText(
            segmentIndex,
            isPrimary,
            find,
            replace,
            offset,
            length,
        );
    }

    function handleReplaceAllTranscriptText(event) {
        const { find, replace, isCaseSensitive, isRegex, isWholeWord } =
            event.detail;
        replaceAllTranscriptText(find, replace, {
            isCaseSensitive,
            isRegex,
            isWholeWord,
        });
    }

    // Helper function to find media by associated transcript path
    function findMediaByTranscriptPath(transcriptPath, projectFiles) {
        console.log(
            "[TranscriptionView] findMediaByTranscriptPath: Searching for transcriptPath:",
            transcriptPath,
        );
        console.log(
            "[TranscriptionView] findMediaByTranscriptPath: projectFiles structure:",
            JSON.stringify(projectFiles, null, 2),
        );

        if (!projectFiles) return null;

        function recurse(nodes) {
            for (const node of nodes) {
                console.log(
                    "[TranscriptionView] findMediaByTranscriptPath: Checking node:",
                    node.name,
                    "file_type:",
                    node.file_type,
                );
                if (node.file_type === "media" && node.associated_transcripts) {
                    console.log(
                        "[TranscriptionView] findMediaByTranscriptPath: Media node found, checking associated_transcripts:",
                        node.associated_transcripts,
                    );
                    if (
                        node.associated_transcripts.some(
                            (t) => t.path === transcriptPath,
                        )
                    ) {
                        console.log(
                            "[TranscriptionView] findMediaByTranscriptPath: Match found for transcriptPath:",
                            transcriptPath,
                            "in media node:",
                            node.name,
                        );
                        return node;
                    }
                }
                if (node.children) {
                    const found = recurse(node.children);
                    if (found) return found;
                }
            }
            return null;
        }
        return recurse(projectFiles);
    }

    async function handleRequestLoadItem(event) {
        console.log(
            "[TranscriptionView] handleRequestLoadItem called for item:",
            event.detail.name,
            "file_type:",
            event.detail.file_type,
        );

        // Commit any pending edits in the editable transcript before checking for dirty state
        if (panelEditModeActive && editableTranscriptRef) {
            editableTranscriptRef.commitCurrentSegmentEdits();
            await tick();
        }

        if (get(transcriptStore).transcriptDirty) {
            await handleSaveTranscript(true);
        }

        // Proceed with loading immediately
        await loadRequestedItem(event.detail);
    }

    // New helper function to encapsulate the loading logic
    async function loadRequestedItem(item) {
        // DO NOT exit edit mode here, preserve current mode across loads
        const store = get(transcriptStore);

        if (item.file_type === "media") {
            console.log("[TranscriptionView] Loading media via selectMedia.");
            selectMedia(item);
        } else if (
            item.file_type === "transcript" ||
            item.file_type === "audio-transcript" ||
            item.file_type === "video-transcript"
        ) {
            // Deactivate dual mode if active when clicking a transcript item
            if (store.isDualModeActive) {
                console.log(
                    "[TranscriptionView] Transcript clicked in Dual Mode, deactivating Dual Mode.",
                );
                await deactivateDualMode();
            }

            const currentProjectFiles = get(project).files;
            console.log(
                "[TranscriptionView] Calling findMediaByTranscriptPath with transcript path:",
                item.path,
            );
            const associatedMedia = findMediaByTranscriptPath(
                item.path,
                currentProjectFiles,
            );

            if (associatedMedia) {
                console.log(
                    "[TranscriptionView] Found associated media for transcript, selecting:",
                    associatedMedia.name,
                    associatedMedia,
                );
                selectMedia(associatedMedia, item.path);
            } else {
                console.warn(
                    "[TranscriptionView] No associated media found for transcript:",
                    item.path,
                );
                selectMedia(null);
            }

            try {
                await loadTranscriptFile(item.path);
                console.log(
                    "[TranscriptionView] Transcript loaded successfully.",
                );
            } catch (error) {
                console.error(
                    "[TranscriptionView] Error loading transcript:",
                    error,
                );
                message(`Error loading transcript: ${error.message || error}`, {
                    title: "Load Error",
                    type: "error",
                });
                return;
            }
        }
    }
</script>

<svelte:window on:keydown={handleGlobalKeydown} />

<div class="flex flex-col h-screen w-full overflow-hidden">
    <div class="flex flex-col flex-grow min-h-0 w-full overflow-hidden">
        <!-- Main Content Area (Panels) -->
        <div class="flex flex-grow min-h-0 w-full overflow-x-hidden">
            {#if !$panelStateStore.transcriptionPanelCollapsed}
                <div
                    class="w-64 h-full bg-white dark:bg-gray-900 overflow-y-auto flex-shrink-0 transition-all duration-300 ease-in-out"
                    transition:slide={{ duration: 300, axis: "x" }}
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
                    class="{isMediaPlayerHidden
                        ? ''
                        : $transcriptStore.englishSegments &&
                            $transcriptStore.englishSegments.length > 0 &&
                            $transcriptStore.originalSegments &&
                            $transcriptStore.originalSegments.length > 0
                          ? 'h-[calc(50%-1.75rem)]'
                          : 'h-1/2'} bg-white dark:bg-gray-950 flex flex-col"
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
                        bind:panelEditMode={panelEditModeActive}
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
            {#if currentWaveformLayout === "vertical"}
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
                    bind:previewEditMode={panelEditModeActive}
                    on:segmentclick={handleSegmentClick}
                    on:toggleedit={handleToggleEditMode}
                    on:requestopentab={(e) =>
                        dispatch("requestopentab", e.detail)}
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
                            mediaPlayerRef.playSegment(
                                segment.start_time,
                                segment.end_time,
                            );
                        }
                    }}
                    on:replacetranscripttext={handleReplaceTranscriptText}
                    on:replacealltranscripttext={handleReplaceAllTranscriptText}
                />
            </div>
        </div>
    </div>

    <!-- Horizontal Waveform Panel (Conditional) -->
    {#if currentWaveformLayout === "horizontal"}
        <div
            style="height: {horizontalWaveformContainerHeightPx}px;"
            class="border-t border-gray-200 dark:border-gray-700 z-[100] relative"
        >
            {#if $transcriptStore.selectedMediaFile && ($transcriptStore.audioBuffer || $transcriptStore.audioBufferPeaks)}
                <InteractiveWaveform
                    bind:this={horizontalWaveformRef}
                    externalAudioBuffer={$transcriptStore.audioBuffer}
                    externalCurrentTime={$transcriptStore.player
                        .currentTime}
                    externalDuration={$transcriptStore.player.duration}
                    externalSegments={$transcriptStore.segments}
                    externalCurrentSegmentIndex={$transcriptStore.player
                        .currentSegmentIndex}
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
