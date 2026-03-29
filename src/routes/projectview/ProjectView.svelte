<!-- src/routes/projectview/ProjectView.svelte -->
<script>
    import { onMount, onDestroy, tick } from "svelte";
    import { page } from "$app/stores";
    import { goto } from "$app/navigation";
    import { get } from "svelte/store";
    import { emit, listen } from "@tauri-apps/api/event";
    import {
        loadProjectDataAndUpdateStore,
        handleConfirmStartTranscription,
        handleCancelTranscriptionRequest,
        registerTranscribeModal,
        initializeProgressListener,
        cleanupProgressListener,
        initializeTranslationProgressListener,
        handleCancelTranslationRequest,
        importMediaFile,
        checkUnsavedChangesThenProceed,
        importDocumentFile,
        importTableFile,
        importImageFile,
        importTranscriptFile,
        requestTranscription as requestTranscriptionService,
        refreshProjectFiles,
        silentlyRefreshProjectData,
        loadTranscriptFile,
        normalizePath,
        clearProjectDataStore,
        createManualTranscript,
        createNewDocument,
    } from "$lib/services/projectService.js";
    import {
        getDownloadedModels,
        getSelectedTranscriptionEngine,
    } from "$lib/services/configureActions.js";
    import { languageOptions } from "$lib/constants/transcriptionOptions.js";
    import {
        project,
        hideUnsavedChangesPrompt,
        hideConversionPrompt,
        prepareDocumentView,
        prepareStandaloneTranscriptView,
        prepareMediaNoteView,
    } from "$lib/stores/projectStore.js";
    import { fetchAllTags } from "$lib/stores/tagStore.js";
    import {
        transcriptStore,
        setRanInBackground,
        setRanTranslationInBackground,
        toggleTranscribeModal,
        selectMedia as selectMediaStoreAction,
        clearTranscriptState,
        setDiarizationPreference,
        setSelectedModel,
        setSelectedTranscriptionEngine,
        setSelectedLanguage,
        setTranslateToEnglish,
        updateSpeakerConfig,
        setTranslationStatus,
        toggleTranslateModal,
        clearPendingTranscriptData,
        insertTranscriptSegment,
    } from "$lib/stores/transcriptStore.js";
    import { message, confirm } from "@tauri-apps/plugin-dialog";
    import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
    import { invoke } from "@tauri-apps/api/core";
    import {
        configStatus,
        updateConfigStatus,
    } from "$lib/stores/configStatusStore.js";

    import BottomBar from "$lib/components/projectview/shared/BottomBar.svelte";
    import TranscribeConfirmModal from "$lib/components/projectview/modals/TranscribeConfirmModal.svelte";

    import UnsavedChangesModal from "$lib/components/projectview/modals/UnsavedChangesModal.svelte";
    import ConfirmConversionModal from "$lib/components/projectview/modals/ConfirmConversionModal.svelte";
    import ImportTranscriptSourceModal from "$lib/components/projectview/modals/ImportTranscriptSourceModal.svelte";
    import HeaderConfirmationModal from "$lib/components/projectview/modals/HeaderConfirmationModal.svelte";
    import ConfigurationModal from "$lib/components/modals/ConfigurationModal.svelte";
    import HelpModal from "$lib/components/modals/HelpModal.svelte";
    import DataView from "$lib/components/projectview/data/DataView.svelte";
    import TranscriptionView from "$lib/components/projectview/transcription/TranscriptionView.svelte";
    import TagsView from "$lib/components/projectview/tags/TagsView.svelte";
    import {
        Loader,
        Music,
        Film,
        FileText,
        MessageSquareText,
        Sheet,
        Image as ImageIcon,
    } from "@lucide/svelte";
    import DataTopBar from "$lib/components/projectview/data/DataTopBar.svelte";
    import TranscriptionTopBar from "$lib/components/projectview/transcription/TopBar.svelte";
    import SimpleTopBar from "$lib/components/projectview/shared/SimpleTopBar.svelte";
    import panelStateStore from "$lib/stores/panelStateStore.js";
    import CreateGroupModal from "$lib/components/projectview/modals/CreateGroupModal.svelte";
    import CreateTableModal from "$lib/components/projectview/modals/CreateTableModal.svelte";

    let transcribeModalRef;
    let transcriptionViewRef;
    let transcriptionTopBarRef;
    let dataViewRef;
    let activeSubItemPath = null;
    let activeSubItemType = null;
    let lastSelectedDocumentPath = null;
    $: if ($project.selectedDocumentPath !== lastSelectedDocumentPath) {
        lastSelectedDocumentPath = $project.selectedDocumentPath;
        activeSubItemPath = null;
        activeSubItemType = null;
    }
    let tagsViewRef;
    let selectedTab = "data";
    let importMenuVisible = false;
    let importMenuX = 0;
    let importMenuY = 0;
    let closeImportMenuListener = null;
    let handlingCloseRequest = false;
    let showImportTranscriptSourceModal = false;
    let showHeaderConfirmationModal = false;
    let showConfigurationModal = false;
    let showHelpModal = false;
    let showCreateGroupModal = false;
    let showCreateTableModal = false;
    let fileToAddForGroup = null;
    let headerConfirmationData = {};
    let unlistenTranscriptionComplete = null;
    let unlistenSelectMedia = null;
    let unlistenCloseRequested = null;
    let unlistenMenuEvents = [];

    // Determine platform-specific modifier key name
    const isMac =
        typeof window !== "undefined" &&
        navigator.platform.toUpperCase().indexOf("MAC") >= 0;
    const modKeyName = isMac ? "Cmd" : "Ctrl";

    // Transcription configuration data
    let downloadedModelsList = [];

    let isLoadingTranscriptionConfig = true;

    $: if ($transcriptStore.showTranscribeModal) {
        loadTranscriptionConfigData();
    }

    async function loadTranscriptionConfigData() {
        isLoadingTranscriptionConfig = true;
        try {
            const localModelsResult = await getDownloadedModels();
            const selectedEngine = await getSelectedTranscriptionEngine();
            const family = selectedEngine || "whisper-cpp";

            downloadedModelsList = localModelsResult.filter((m) => {
                if (family === "faster-whisper") {
                    return m.family === "faster-whisper";
                } else {
                    return (
                        m.family === "whisper-cpp" ||
                        (!m.family && !m.name.includes("/"))
                    );
                }
            });
        } catch (e) {
            console.error(
                "[ProjectView] Error during transcription configuration loading for modal:",
                e,
            );
            downloadedModelsList = [];
        } finally {
            isLoadingTranscriptionConfig = false;
        }
    }

    async function onConfirmTranscriptionStart(event) {
        const {
            transcriptionMode,
            selectedModel,
            selectedTranscriptionEngine,
            selectedLanguage,
            translateToEnglish,
            enableDiarization,
            speakersConfig,
            manualSettings,
            initialPrompt,
            hotwords,
        } = event.detail;

        // Common: Update speaker config
        if (speakersConfig) {
            updateSpeakerConfig(
                speakersConfig.count,
                speakersConfig.names,
                speakersConfig.translatedNames,
            );
        }

        if (transcriptionMode === "automatic") {
            // Update store with automatic settings
            setSelectedModel(selectedModel);
            setSelectedTranscriptionEngine(selectedTranscriptionEngine);
            setSelectedLanguage(selectedLanguage);
            setTranslateToEnglish(translateToEnglish);
            setDiarizationPreference(enableDiarization);
            transcriptStore.update((ts) => ({
                ...ts,
                initialPrompt: initialPrompt || "",
                hotwords: hotwords || "",
            }));

            await handleConfirmStartTranscription(transcriptionMode);
        } else if (transcriptionMode === "manual") {
            // Manual Logic
            toggleTranscribeModal(false); // Close modal

            // Ensure Dual Mode is OFF before creating manual segments
            const currentStore = get(transcriptStore);
            if (currentStore.isDualModeActive) {
                console.log(
                    "[ProjectView] Dual Mode is active during manual transcription start. Turning it OFF.",
                );
                transcriptStore.update((ts) => ({
                    ...ts,
                    isDualModeActive: false,
                    secondaryTranscriptPath: null,
                    secondaryTranscriptSegments: [],
                }));
                // Update localStorage as well to persist the change
                if (typeof window !== "undefined") {
                    localStorage.setItem(
                        "harvey-dual-mode",
                        JSON.stringify(false),
                    );
                }
            }

            const { segmentCount, segmentDuration, speakerMode, startTime } =
                manualSettings;
            const store = get(transcriptStore);
            const speakerNames = store.speakers.names;

            let currentStartTime = startTime;
            let lastSpeakerIndex = -1; // New transcript, start from first speaker or unassigned

            const newSegments = [];
            for (let i = 0; i < segmentCount; i++) {
                const newEndTime = currentStartTime + segmentDuration;
                let speaker = "Unknown";

                if (
                    speakerMode === "alternate" &&
                    speakerNames &&
                    speakerNames.length > 0
                ) {
                    const currentSpeakerIndex = i % speakerNames.length;
                    speaker = speakerNames[currentSpeakerIndex];
                }

                const newSegment = {
                    start_time: currentStartTime,
                    end_time: newEndTime,
                    speaker: speaker,
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

                newSegments.push(newSegment);
                currentStartTime = newEndTime;
            }

            if (store.selectedMediaFile?.path) {
                try {
                    project.update((p) => ({
                        ...p,
                        isLoading: true,
                        statusMessage: "Creating manual transcript...",
                    }));
                    await createManualTranscript(
                        store.selectedMediaFile.path,
                        newSegments,
                        manualSettings,
                    );

                    // Automatically enter edit mode
                    if (
                        transcriptionViewRef &&
                        typeof transcriptionViewRef.enterManualEditMode ===
                            "function"
                    ) {
                        await transcriptionViewRef.enterManualEditMode();
                    }
                } catch (e) {
                    console.error(
                        "[ProjectView] Error creating manual transcript:",
                        e,
                    );
                    message(
                        `Failed to create manual transcript: ${e.message || e}`,
                        { title: "Error", type: "error" },
                    );
                } finally {
                    project.update((p) => ({ ...p, isLoading: false }));
                }
            } else {
                console.error(
                    "[ProjectView] No media file selected, cannot create manual transcript.",
                );
                message("Error: No media file selected.", {
                    title: "Error",
                    type: "error",
                });
            }
        }
    }

    // Enhanced logic for critical issues: includes missing libraries for the selected engine 
    // OR if Python libraries are completely missing which impacts most features.
    $: hasCriticalConfigIssues =
        ($configStatus.selected_transcription_engine === "faster-whisper" &&
            !$configStatus.python_libraries_installed) ||
        ($configStatus.selected_transcription_engine === "whisper-cpp" &&
            !$configStatus.whisper_cpp_installed) ||
        !$configStatus.python_libraries_installed;

    $: hasNonCriticalConfigIssues =
        !hasCriticalConfigIssues &&
        (!$configStatus.hf_token_present ||
            !$configStatus.diarization_model_downloaded ||
            ($configStatus.selected_translation_engine === "helsinki" &&
                !$configStatus.helsinki_models_downloaded) ||
            ($configStatus.selected_translation_engine === "nllb" &&
                !$configStatus.nllb_models_downloaded) ||
            ($configStatus.selected_transcription_engine === "whisper-cpp" &&
                !$configStatus.whisper_cpp_models_downloaded) ||
            ($configStatus.selected_transcription_engine === "faster-whisper" &&
                (!$configStatus.faster_whisper_models_downloaded ||
                    !$configStatus.faster_whisper_dependencies_installed)));

    $: hasConfigIssues = hasCriticalConfigIssues || hasNonCriticalConfigIssues;

    onMount(async () => {
        const appWindow = getCurrentWindow();
        await appWindow.setMinSize(new LogicalSize(1024, 600));
        await appWindow.maximize();
        await invoke("set_menu_context", { context: "project" }).catch((err) =>
            console.warn("Failed to set menu context:", err),
        );

        await updateConfigStatus(true); // Force refresh global config status
        await loadTranscriptionConfigData(); // Load model/cloud config

        // Menu Event Listeners
        unlistenMenuEvents.push(
            await listen("menu:file:import:audio", () =>
                triggerMediaImport("audio"),
            ),
        );
        unlistenMenuEvents.push(
            await listen("menu:file:import:video", () =>
                triggerMediaImport("video"),
            ),
        );
        unlistenMenuEvents.push(
            await listen("menu:file:import:document", () =>
                triggerMediaImport("document"),
            ),
        );
        unlistenMenuEvents.push(
            await listen("menu:file:import:image", () =>
                triggerMediaImport("image"),
            ),
        );
        unlistenMenuEvents.push(
            await listen("menu:file:import:table", () =>
                triggerMediaImport("table"),
            ),
        );
        unlistenMenuEvents.push(
            await listen("menu:file:import:transcript", () =>
                triggerMediaImport("transcript"),
            ),
        );

        unlistenMenuEvents.push(
            await listen("menu:file:create:document", async () => {
                const currentProject = get(project);
                if (currentProject && currentProject.xmlPath) {
                    if (selectedTab !== "data") {
                        await handleTabClick("data");
                        await tick();
                    }
                    createNewDocument(currentProject.xmlPath);
                }
            }),
        );
        unlistenMenuEvents.push(
            await listen("menu:file:create:table", async () => {
                if (selectedTab !== "data") {
                    await handleTabClick("data");
                    await tick();
                }
                showCreateTableModal = true;
            }),
        );
        unlistenMenuEvents.push(
            await listen("menu:file:create:group", async () => {
                if (selectedTab !== "data") {
                    await handleTabClick("data");
                    await tick();
                }
                showCreateGroupModal = true;
            }),
        );
        unlistenMenuEvents.push(
            await listen("menu:file:create:tag", async () => {
                if (selectedTab !== "tags") {
                    await handleTabClick("tags");
                    await tick();
                }
                if (tagsViewRef) tagsViewRef.openAddTagModal();
            }),
        );
        unlistenMenuEvents.push(
            await listen("menu:file:create:tag-group", async () => {
                if (selectedTab !== "tags") {
                    await handleTabClick("tags");
                    await tick();
                }
                if (tagsViewRef) tagsViewRef.openAddGroupModal();
            }),
        );

        unlistenMenuEvents.push(
            await listen("menu:help:center", () => {
                showHelpModal = true;
            }),
        );

        unlistenMenuEvents.push(
            await listen("request-create-table-modal", () => {
                showCreateTableModal = true;
            }),
        );

        unlistenMenuEvents.push(
            await listen("request-create-group-modal", (event) => {
                fileToAddForGroup = event.payload?.fileToAdd || null;
                showCreateGroupModal = true;
            }),
        );

        const xmlPath = $page.url.searchParams.get("xmlPath");
        if (xmlPath && xmlPath.trim() !== "") {
            try {
                await loadProjectDataAndUpdateStore(xmlPath);
                await fetchAllTags(); // Fetch all tags after project data is loaded
            } catch (e) {
                console.error(
                    "[ProjectView] Error during initial project load:",
                    e,
                );
            }
        } else {
            project.update((p) => ({
                ...p,
                isLoading: false,
                error: "Project path is missing.",
                statusMessage: "Error: Project path is missing.",
            }));
            console.error(
                "[ProjectView] Mount error: Project XML path missing in URL parameters.",
            );
        }
        initializeProgressListener();
        initializeTranslationProgressListener();

        unlistenTranscriptionComplete = await listen(
            "custom_transcription_job_completed",
            (event) => {
                if (event.payload && event.payload.status === "done") {
                    console.log(
                        "[ProjectView] Transcription job completed event received, refreshing files silently.",
                    );
                    const currentProjectXmlPath = get(project).xmlPath;
                    if (currentProjectXmlPath) {
                        silentlyRefreshProjectData(currentProjectXmlPath);
                    } else {
                        console.error(
                            "[ProjectView] Cannot silently refresh project data: XML path is missing.",
                        );
                    }

                    const ranInBackground =
                        get(transcriptStore).ranInBackground;
                    if (ranInBackground) {
                        console.log(
                            "[ProjectView event_listener] Background job done, calling clearPendingTranscriptData.",
                        );
                        clearPendingTranscriptData();
                    } else {
                        console.log(
                            "[ProjectView event_listener] Foreground job done, NOT calling clearPendingTranscriptData here (handleModalClose will).",
                        );
                    }
                }
            },
        );

        await tick();
        if (transcribeModalRef) {
            registerTranscribeModal(transcribeModalRef);
        } else {
            console.warn(
                "[ProjectView] TranscribeConfirmModal reference not available on mount.",
            );
        }
        window.addEventListener("keydown", handleGlobalKeys);

        unlistenSelectMedia = await listen(
            "select_media_in_transcription_tab",
            async (event) => {
                const { mediaPath } = event.payload;
                if (mediaPath) {
                    await handleRequestMediaSelection({
                        detail: { mediaPath },
                    });
                }
            },
        );

        unlistenCloseRequested = await appWindow.onCloseRequested(
            async (event) => {
                event.preventDefault();
                await handleCloseProject();
            },
        );
    });

    onDestroy(() => {
        cleanupProgressListener();
        if (unlistenMenuEvents) {
            unlistenMenuEvents.forEach((unlisten) => unlisten());
            unlistenMenuEvents = [];
        }
        if (unlistenTranscriptionComplete) {
            unlistenTranscriptionComplete();
        }
        if (unlistenSelectMedia) {
            unlistenSelectMedia();
        }
        if (unlistenCloseRequested) {
            unlistenCloseRequested();
        }
        window.removeEventListener("keydown", handleGlobalKeys);
        if (closeImportMenuListener) {
            document.removeEventListener("click", closeImportMenuListener, {
                capture: true,
            });
            closeImportMenuListener = null;
        }
    });

    function handleGlobalKeys(event) {
        if (event.key === "Backspace") {
            const activeElement = document.activeElement;
            if (
                activeElement.tagName.toLowerCase() !== "input" &&
                activeElement.tagName.toLowerCase() !== "textarea" &&
                !activeElement.isContentEditable
            ) {
                event.preventDefault();
                return;
            }
        }

        const proj = get(project);
        const ts = get(transcriptStore);
        const isMac = navigator.platform.toUpperCase().indexOf("MAC") >= 0;
        const modKey = isMac ? event.metaKey : event.ctrlKey;
        // Using Ctrl+Alt for media controls ensures we don't hit macOS native Cmd window/tab shortcuts
        const mediaModKey = event.ctrlKey && event.altKey && !event.metaKey;

        // Playback speed adjustment shortcuts (Ctrl + Alt + [ and Ctrl + Alt + ])
        if (mediaModKey && (event.key === "[" || event.key === "]")) {
            const playerStoreValue = ts.player;
            if (playerStoreValue && selectedTab === "transcription") {
                event.preventDefault();
                let currentSpeed = playerStoreValue.playbackRate || 1.0;
                let newSpeed = currentSpeed;
                if (event.key === "[") {
                    newSpeed = Math.max(0.25, currentSpeed - 0.25);
                } else if (event.key === "]") {
                    newSpeed = Math.min(3.0, currentSpeed + 0.25);
                }

                if (newSpeed !== currentSpeed) {
                    transcriptStore.update((s) => ({
                        ...s,
                        player: { ...s.player, playbackRate: newSpeed },
                    }));
                }
            }
        }

        // Play/Pause, Rewind, Forward shortcuts
        const isF8 = event.key === "F8";
        const isF7 = event.key === "F7";
        const isF9 = event.key === "F9";
        const isCmdOptP = mediaModKey && event.key.toLowerCase() === "p";
        const isCmdOptLeft = mediaModKey && event.key === "ArrowLeft";
        const isCmdOptRight = mediaModKey && event.key === "ArrowRight";

        if (
            (isF8 || isCmdOptP) &&
            selectedTab === "transcription" &&
            transcriptionViewRef?.mediaPlayerRef
        ) {
            event.preventDefault();
            const playerEl = transcriptionViewRef.mediaPlayerRef.videoElement;
            if (playerEl) {
                if (playerEl.paused)
                    playerEl
                        .play()
                        .catch((e) => console.error("Play failed:", e));
                else playerEl.pause();
            }
        }

        if (
            (isF7 || isCmdOptLeft) &&
            selectedTab === "transcription" &&
            transcriptionViewRef?.mediaPlayerRef
        ) {
            event.preventDefault();
            const playerEl = transcriptionViewRef.mediaPlayerRef.videoElement;
            if (playerEl) {
                playerEl.currentTime = Math.max(0, playerEl.currentTime - 5);
            }
        }

        if (
            (isF9 || isCmdOptRight) &&
            selectedTab === "transcription" &&
            transcriptionViewRef?.mediaPlayerRef
        ) {
            event.preventDefault();
            const playerEl = transcriptionViewRef.mediaPlayerRef.videoElement;
            if (playerEl) {
                playerEl.currentTime = Math.min(
                    playerEl.duration || 0,
                    playerEl.currentTime + 5,
                );
            }
        }

        if (modKey && event.key.toLowerCase() === "s") {
            event.preventDefault();
            if (selectedTab === "transcription" && transcriptionViewRef) {
                transcriptionViewRef.handleSaveTranscript();
            } else if (selectedTab === "data") {
                const activeDocEditor = proj.activeDocumentEditorRef?.ref;
                const activeImpTsEditor =
                    proj.activeStandaloneTranscriptEditorRef?.ref;
                const activeMediaNoteEditor =
                    proj.activeMediaNoteEditorRef?.ref;

                if (false) {
                    if (
                        (proj.isDocumentDirty ||
                            proj.isDocumentMetadataDirty) &&
                        activeDocEditor &&
                        typeof activeDocEditor.save === "function"
                    ) {
                        activeDocEditor
                            .save()
                            .catch((e) =>
                                console.error(
                                    `${modKeyName}+S document save failed`,
                                    e,
                                ),
                            );
                    } else if (
                        proj.isStandaloneTranscriptDirty &&
                        activeImpTsEditor &&
                        typeof activeImpTsEditor.save === "function"
                    ) {
                        activeImpTsEditor
                            .save()
                            .catch((e) =>
                                console.error(
                                    `${modKeyName}+S imported transcript save failed`,
                                    e,
                                ),
                            );
                    } else if (
                        proj.isMediaNoteTranscriptDirty &&
                        activeMediaNoteEditor &&
                        typeof activeMediaNoteEditor.save === "function"
                    ) {
                        activeMediaNoteEditor
                            .save()
                            .catch((e) =>
                                console.error(
                                    `${modKeyName}+S media note save failed`,
                                    e,
                                ),
                            );
                    }
                }
            }
            return;
        }
        if (modKey && event.key.toLowerCase() === "e") {
            if (selectedTab === "transcription" && transcriptionViewRef) {
                event.preventDefault();
                transcriptionViewRef.handleToggleEditMode();
            }
            return;
        }
        if (modKey && event.key.toLowerCase() === "z" && !event.shiftKey) {
            if (
                selectedTab === "transcription" &&
                transcriptionViewRef &&
                ts.transcriptUndoStack?.length > 0
            ) {
                event.preventDefault();
                transcriptionViewRef.handleUndoRequest();
            }
            return;
        }
        if (
            modKey &&
            (event.key.toLowerCase() === "y" ||
                (event.shiftKey && event.key.toLowerCase() === "z"))
        ) {
            if (
                selectedTab === "transcription" &&
                transcriptionViewRef &&
                ts.transcriptRedoStack?.length > 0
            ) {
                event.preventDefault();
                transcriptionViewRef.handleRedoRequest();
            }
            return;
        }
        if (event.key === "F8") {
            if (
                selectedTab === "transcription" &&
                transcriptionViewRef &&
                transcriptionViewRef.mediaPlayerRef
            ) {
                event.preventDefault();
                transcriptionViewRef.mediaPlayerRef.handleTogglePlay();
            }
            return;
        }
    }

    function handleModalClose(event) {
        const { acknowledged, finalStatus } = event.detail || {};
        toggleTranscribeModal(false);

        if (acknowledged) {
            if (finalStatus === "done") {
                const jobFinishedPath =
                    get(transcriptStore).mediaPathForLastJob;
                const currentSelectionPathInUI =
                    get(transcriptStore).selectedMediaFile?.path;
                const activeMediaWhenJobStarted =
                    get(transcriptStore).activeMediaDuringTranscriptionStart;
                const currentProjectXmlPath = get(project).xmlPath;
                const ranInBackground = get(transcriptStore).ranInBackground;
                // const pendingPath = get(transcriptStore).pendingTranscriptPathForJobDone;
                // const pendingSegments = get(transcriptStore).pendingSegmentsForJobDone;

                if (!ranInBackground && jobFinishedPath) {
                    console.log(
                        "[ProjectView] Modal closed after foreground transcription, refreshing files and selecting media:",
                        jobFinishedPath,
                    );
                    let refreshPromise = refreshProjectFiles(jobFinishedPath); // This should select the media and trigger transcript load
                    refreshPromise
                        .then(async () => {
                            console.log(
                                "[ProjectView HMC] Entered refreshPromise.then()",
                            );
                            // This block runs after refreshProjectFiles has completed and its UI updates have likely propagated
                            const projectFiles = get(project).files;
                            let mediaFileEntry = null;
                            // Function to find the media file entry (assuming it's similar to what refreshProjectFiles might use or what's available)
                            function findMediaByPathRecursive(nodes, path) {
                                if (!Array.isArray(nodes)) return null;
                                for (const node of nodes) {
                                    if (
                                        node.file_type === "media" &&
                                        !node.is_directory &&
                                        node.path === path
                                    )
                                        return node;
                                    if (node.children?.length > 0) {
                                        const found = findMediaByPathRecursive(
                                            node.children,
                                            path,
                                        );
                                        if (found) return found;
                                    }
                                }
                                return null;
                            }
                            if (
                                !get(transcriptStore).ranInBackground &&
                                jobFinishedPath
                            ) {
                                // Ensure to use the current ranInBackground value
                                let mediaFileEntry = null; // Defined here
                                // Function to find the media file entry (assuming it's similar to what refreshProjectFiles might use or what's available)
                                function findMediaByPathRecursive(nodes, path) {
                                    // Definition moved inside or ensured accessible
                                    if (!Array.isArray(nodes)) return null;
                                    for (const node of nodes) {
                                        if (
                                            node.file_type === "media" &&
                                            !node.is_directory &&
                                            node.path === path
                                        )
                                            return node;
                                        if (node.children?.length > 0) {
                                            const found =
                                                findMediaByPathRecursive(
                                                    node.children,
                                                    path,
                                                );
                                            if (found) return found;
                                        }
                                    }
                                    return null;
                                }
                                mediaFileEntry = findMediaByPathRecursive(
                                    projectFiles,
                                    jobFinishedPath,
                                );

                                if (mediaFileEntry) {
                                    await selectMediaStoreAction(
                                        mediaFileEntry,
                                    ); // This line should already exist

                                    const newTranscriptPath =
                                        get(
                                            transcriptStore,
                                        ).pendingTranscriptPathForJobDone;
                                    if (newTranscriptPath) {
                                        console.log(
                                            `[ProjectView] Explicitly loading new transcript: ${newTranscriptPath}`,
                                        );
                                        loadTranscriptFile(
                                            newTranscriptPath,
                                        ).catch((err) => {
                                            console.error(
                                                `[ProjectView] Error explicitly loading new transcript: ${err.message || err}`,
                                            );
                                            // Optional: project.update(p => ({...p, error: `Failed to load new transcript: ${err.message || err}`}));
                                        });
                                    }
                                } else {
                                    console.warn(
                                        `[ProjectView] Media file entry not found after refresh for path: ${jobFinishedPath}. Cannot auto-load transcript.`,
                                    );
                                }
                            }
                            // THE CLEANUP CODE SHOULD GO HERE, after the conditional processing,
                            // but still inside .then()
                            console.log(
                                "[ProjectView HMC .then()] Foreground processing in .then() complete. Clearing job context and pending data.",
                            );
                            transcriptStore.update((ts) => ({
                                ...ts,
                                mediaPathForLastJob: null,
                                activeMediaDuringTranscriptionStart: null,
                            }));
                            clearPendingTranscriptData();
                        })
                        .catch((err) => {
                            console.error(
                                "[ProjectView] Error during refreshProjectFiles sequence in handleModalClose:",
                                err,
                            );
                            // CRITICAL: Also clear data on error to prevent stale state if refresh fails!
                            console.log(
                                "[ProjectView HMC .catch()] Error in refreshPromise. Clearing job context and pending data to prevent stale state.",
                            );
                            transcriptStore.update((ts) => ({
                                ...ts,
                                mediaPathForLastJob: null,
                                activeMediaDuringTranscriptionStart: null,
                            }));
                            clearPendingTranscriptData();
                        });
                } else {
                    // This 'else' corresponds to the "if (!ranInBackground && jobFinishedPath)"
                    // This case means it ran in background OR (foreground but !jobFinishedPath).
                    // A silent refresh might have already been done by the event listener if it was a background task.
                    // Or, if it was foreground and no job path, the earlier call to silentlyRefreshProjectData handles it.
                    console.log(
                        "[ProjectView HMC else] Modal closed (ranInBackground or no jobFinishedPath for foreground). Clearing modal-specific job context.",
                    );
                    if (
                        currentProjectXmlPath &&
                        get(transcriptStore).ranInBackground
                    ) {
                        // Only if ran in background and refresh needed
                        // If it truly ran in background, the event listener should have refreshed.
                        // This silent refresh is more of a fallback if that event was missed or if state is complex.
                        // However, the primary silent refresh for background is now handled by the event listener.
                        // For foreground with no job path, silent refresh was done before promise.
                        // So, this specific call to silentlyRefreshProjectData might be redundant if event listener works.
                        // Let's keep it for now as a safeguard for the ranInBackground path.
                        silentlyRefreshProjectData(currentProjectXmlPath);
                    } else if (
                        !get(transcriptStore).ranInBackground &&
                        !jobFinishedPath
                    ) {
                        // Foreground, but no job path to refresh. A general silent refresh was already done.
                        console.log(
                            "[ProjectView HMC else] Foreground task with no jobFinishedPath. Silent refresh was done prior to promise.",
                        );
                    }

                    transcriptStore.update((ts) => ({
                        // Clear context related to the job this modal instance was tracking
                        ...ts,
                        mediaPathForLastJob: null,
                        activeMediaDuringTranscriptionStart: null,
                    }));

                    // If it was a foreground task but jobFinishedPath was null,
                    // then HMC is responsible for clearing pending data because the .then() part of refreshPromise was skipped.
                    if (
                        !get(transcriptStore).ranInBackground &&
                        !jobFinishedPath
                    ) {
                        // Check current ranInBackground
                        console.log(
                            "[ProjectView HMC else] jobFinishedPath was null for a foreground task. Clearing pending data now.",
                        );
                        clearPendingTranscriptData();
                    }
                    // Note: `silentlyRefreshProjectData` for the case of (foreground && !jobFinishedPath)
                    // was already called before `refreshPromise` was defined.
                }
                // Synchronous cleanup removed from here
            }
        } else {
            // This 'else' corresponds to "if (acknowledged)"
            if (finalStatus === "running" || finalStatus === "cancelling") {
                console.log(
                    `[ProjectView] TranscribeModal closed by user (acknowledged:false) while status was: ${finalStatus}. Background process continues.`,
                );
            }
        }
    }
    function handleUnsavedResponse(event) {
        const action = event.type;
        const callback =
            get(project)[
                `onUnsaved${action.charAt(0).toUpperCase() + action.slice(1)}`
            ];
        if (typeof callback === "function") {
            callback();
        } else {
            console.warn(
                `[ProjectView] No valid callback for unsaved action: ${action}`,
            );
            hideUnsavedChangesPrompt();
        }
    }
    function handleConversionResponse(event) {
        const action = event.type;
        const callback =
            get(project)[
                `onConversion${action.charAt(0).toUpperCase() + action.slice(1)}`
            ];
        if (typeof callback === "function") {
            callback();
        } else {
            console.warn(
                `[ProjectView] No valid callback for conversion action: ${action}`,
            );
            hideConversionPrompt();
        }
    }

    async function handleCloseProject() {
        if (handlingCloseRequest) return;
        handlingCloseRequest = true;
        let canProceed = false;
        try {
            if (selectedTab === "data") {
                canProceed = await checkUnsavedChangesThenProceed(
                    null,
                    "closing the project",
                );
            } else if (selectedTab === "transcription") {
                if (transcriptionViewRef) {
                    try {
                        await transcriptionViewRef.exitEditModeIfActive();
                        canProceed = true;
                    } catch (e) {
                        canProceed = false;
                    }
                } else {
                    canProceed = true;
                }
            } else {
                canProceed = true;
            }
        } catch (error) {
            canProceed = false;
        }

        if (canProceed) {
            await clearProjectDataStore();
            const appWindow = getCurrentWindow();
            await appWindow.setMinSize(new LogicalSize(800, 600)); // Reset min size for welcome screen
            await appWindow.unmaximize();
            await appWindow.setSize(new LogicalSize(800, 600));
            await goto("/");
        }
        handlingCloseRequest = false;
    }

    async function handleTabClick(tabName) {
        if (selectedTab === tabName) {
            // If the same tab is clicked, toggle the corresponding panel
            if (tabName === "data") {
                panelStateStore.toggleDataLeftPanel();
            } else if (tabName === "transcription") {
                panelStateStore.toggleTranscriptionPanel();
            } else if (tabName === "tags") {
                panelStateStore.toggleTagsLeftPanel();
            }
            return true; // Return true as we are "on" the tab
        }

        // Perform silent saves if necessary before switching tabs
        if (selectedTab === "transcription") {
            if (transcriptionViewRef) {
                try {
                    await transcriptionViewRef.handleSaveTranscript();
                } catch (e) {
                    console.error(
                        "[ProjectView] Silent save failed before tab switch:",
                        e,
                    );
                }
            }
        } else if (selectedTab === "data") {
            // checkUnsavedChangesThenProceed handles its own silent saves for data tab items
            const canProceed = await checkUnsavedChangesThenProceed(
                null,
                "switching tabs",
            );
            if (!canProceed) {
                project.update((p) => ({
                    ...p,
                    isLoading: false,
                    statusMessage: "Tab switch cancelled.",
                }));
                return false;
            }
        }

        project.update((p) => ({
            ...p,
            isLoading: true,
            statusMessage: `Switching to ${tabName} tab...`,
        }));

        await proceedTabSwitch(tabName);
        return true;
    }

    async function proceedTabSwitch(tabName) {
        selectedTab = tabName;

        project.update((p) => ({
            ...p,
            isDocumentLoading: false,
            isStandaloneTranscriptLoading: false,
            isMediaNoteTranscriptLoading: false,
        }));

        if (selectedTab === "data") {
            if (
                !get(project).selectedDocumentPath &&
                !get(project).currentStandaloneTranscriptPath &&
                !get(project).selectedMediaNotePath
            ) {
                prepareDocumentView(null);
            }
        } else if (selectedTab === "transcription") {
            // prepareDocumentView(null); // Removed to persist Data tab state
            // If no media is selected, find and select the first one
            if (!get(transcriptStore).selectedMediaFile) {
                const proj = get(project);
                let firstMediaFile = null;

                function findFirstMediaRecursive(nodes) {
                    if (!Array.isArray(nodes)) return;
                    for (const node of nodes) {
                        if (node.file_type === "media" && !node.is_directory) {
                            firstMediaFile = node;
                            return;
                        }
                        if (node.children && node.children.length > 0) {
                            findFirstMediaRecursive(node.children);
                            if (firstMediaFile) return;
                        }
                    }
                }

                findFirstMediaRecursive(proj.files);

                if (firstMediaFile) {
                    console.log(
                        `[ProjectView] No media selected on transcription tab switch. Auto-selecting first media:`,
                        firstMediaFile.path,
                    );
                    // Use a timeout to ensure the UI has updated before selecting the media
                    setTimeout(() => {
                        handleRequestMediaSelection({
                            detail: { mediaPath: firstMediaFile.path },
                        });
                    }, 0);
                }
            }
        }

        if (
            tabName !== "transcription" &&
            transcriptionViewRef?.mediaPlayerRef?.videoElement &&
            !transcriptionViewRef.mediaPlayerRef.videoElement.paused
        ) {
            try {
                await transcriptionViewRef.mediaPlayerRef.videoElement.pause();
            } catch (e) {
                console.warn("Error pausing main video on tab switch:", e);
            }
        }
        await tick();
        project.update((p) => ({
            ...p,
            isLoading: false,
            statusMessage: `Switched to ${tabName} tab.`,
        }));
    }

    async function handleRequestOpenTab(event) {
        const {
            tabName,
            loadNotePath,
            highlightId,
            viewType,
            originalDocType,
            attachmentToOpen,
        } = event.detail;
        if (!tabName || !loadNotePath) return;

        const proj = get(project);
        let path = normalizePath(loadNotePath);

        // Robust absolute path resolution
        let cleanPath = path.replace(/^\/+/, ""); // Strip leading slashes for check
        if (proj.baseDirectory && !path.startsWith(proj.baseDirectory)) {
            // Check if it looks like a relative path belonging to the project
            if (
                cleanPath.startsWith("harvey_files") ||
                !cleanPath.includes("/")
            ) {
                path = normalizePath(`${proj.baseDirectory}/${cleanPath}`);
            }
        }

        const itemLogName = path.split(/[\\/]/).pop();
        console.log(
            `[ProjectView] handleRequestOpenTab: path=${path}, viewType=${viewType}, originalDocType=${originalDocType}`,
        );

        // Update requested highlight ID immediately so the target component can react
        project.update((p) => ({
            ...p,
            isLoading: true,
            statusMessage: `Opening ${itemLogName}...`,
            requestedHighlightId: highlightId || null,
        }));

        // Determine if we are already viewing this path
        const isAlreadyViewing =
            normalizePath(proj.selectedDocumentPath) === path ||
            normalizePath(proj.currentStandaloneTranscriptPath) === path ||
            normalizePath(proj.selectedMediaNotePath) === path;

        if (isAlreadyViewing) {
            console.log(
                `[ProjectView] Already viewing ${itemLogName}. Just ensuring correct tab.`,
            );
            if (selectedTab !== tabName) {
                await handleTabClick(tabName);
            }
            project.update((p) => ({ ...p, isLoading: false }));
            return;
        }

        // Not already viewing, proceed with full load
        let canProceed = await checkUnsavedChangesThenProceed(
            path,
            "opening item",
        );
        if (!canProceed) {
            project.update((p) => ({ ...p, isLoading: false }));
            return;
        }

        if (selectedTab !== tabName) {
            await handleTabClick(tabName);
            await tick();
        }

        // Determine type and prepare view
        if (tabName === "data") {
            const isStandaloneTranscript =
                viewType === "standalone_transcript" ||
                originalDocType === "standalone_transcript" ||
                proj.standaloneTranscriptFiles?.some(
                    (f) =>
                        normalizePath(
                            `${proj.baseDirectory}/${f.relative_path || f.relativePath}`,
                        ) === path,
                );

            const isMediaTranscript =
                viewType === "audio_transcript" ||
                viewType === "video_transcript" ||
                originalDocType === "audio_transcript" ||
                originalDocType === "video_transcript";

            if (isStandaloneTranscript) {
                prepareStandaloneTranscriptView(path);
            } else if (isMediaTranscript) {
                function findMediaByTranscriptPathRecursive(
                    nodes,
                    transcriptPath,
                ) {
                    if (!Array.isArray(nodes)) return null;
                    const normTranscriptPath = transcriptPath.replace(
                        /\\/g,
                        "/",
                    );
                    for (const node of nodes) {
                        if (
                            node.file_type === "media" &&
                            node.associated_transcripts?.some(
                                (t) =>
                                    t.path.replace(/\\/g, "/") ===
                                    normTranscriptPath,
                            )
                        )
                            return node;
                        const found = findMediaByTranscriptPathRecursive(
                            node.children || [],
                            transcriptPath,
                        );
                        if (found) return found;
                    }
                    return null;
                }
                const mediaNode = findMediaByTranscriptPathRecursive(
                    proj.files,
                    path,
                );
                if (mediaNode) {
                    console.log(
                        `[ProjectView] Found parent media for transcript deep-link: ${mediaNode.path}`,
                    );
                    prepareMediaNoteView(mediaNode.path, path); // dual-path call
                    // Selection highlighting in Left Panel
                    project.update((p) => ({
                        ...p,
                        activeTranscriptPathInDataTab: path,
                    }));
                } else {
                    console.warn(
                        `[ProjectView] Parent media not found for transcript: ${path}. Attempting standalone document view.`,
                    );
                    prepareDocumentView(path, "documents");
                }
            } else if (
                viewType === "media_note" ||
                viewType === "media" ||
                [
                    "mp3",
                    "wav",
                    "m4a",
                    "ogg",
                    "aac",
                    "flac",
                    "mp4",
                    "mov",
                    "avi",
                    "mkv",
                    "webm",
                ].includes(path.split(".").pop()?.toLowerCase())
            ) {
                prepareMediaNoteView(path);
            } else {
                const ext = path.split(".").pop()?.toLowerCase();
                let type = "documents";
                if (["csv", "xlsx"].includes(ext)) type = "tables";
                else if (
                    [
                        "jpg",
                        "jpeg",
                        "png",
                        "gif",
                        "bmp",
                        "webp",
                        "tiff",
                    ].includes(ext)
                )
                    type = "images";
                prepareDocumentView(path, type);
            }
        }

        if (selectedTab !== tabName) {
            await handleTabClick(tabName);
            await tick();
        }

        // Final check to clear loading overlay if sub-loading didn't trigger
        const projState = get(project);
        const stillLoading =
            projState.isDocumentLoading ||
            projState.isStandaloneTranscriptLoading ||
            projState.isMediaNoteTranscriptLoading;
        if (
            !stillLoading &&
            !projState.isTranscribing &&
            !projState.isImportingAsset
        ) {
            project.update((p) => ({ ...p, isLoading: false }));
        }

        if (tabName === "data" && attachmentToOpen) {
            await tick(); // ensure UI is updated before opening attachment panel
            if (panelStateStore) {
                panelStateStore.setActiveInfoPanelTab("attachments");
            }
            if (
                dataViewRef &&
                typeof dataViewRef.handleRequestOpenLexicalDocument ===
                    "function"
            ) {
                dataViewRef.handleRequestOpenLexicalDocument({
                    detail: { docPath: attachmentToOpen },
                });
            } else {
                console.warn(
                    "[ProjectView] dataViewRef or handleRequestOpenLexicalDocument not available",
                );
            }
        }
    }

    async function handleRequestMediaSelection(event) {
        console.log(
            `[ProjectView] handleRequestMediaSelection: Received event with mediaPath: '${event.detail?.mediaPath}'`,
        );
        const { mediaPath } = event.detail;
        const mediaName = mediaPath
            ? mediaPath.split(/[\\/]/).pop()
            : "Unknown Media";

        if (!mediaPath) {
            project.update((p) => ({
                ...p,
                isLoading: false,
                statusMessage: "Error: Media path missing.",
            }));
            console.error(
                "[ProjectView] handleRequestMediaSelection: mediaPath is null or undefined.",
            );
            return;
        }
        project.update((p) => ({
            ...p,
            isLoading: true,
            statusMessage: `Selecting media ${mediaName}...`,
        }));

        let canProceed = true;
        if (selectedTab === "data") {
            console.log(
                `[ProjectView] handleRequestMediaSelection: Calling checkUnsavedChangesThenProceed for path '${mediaPath}' from current tab '${selectedTab}'`,
            );
            canProceed = await checkUnsavedChangesThenProceed(
                mediaPath,
                "selecting media for transcription tab",
            );
            console.log(
                `[ProjectView] handleRequestMediaSelection: checkUnsavedChangesThenProceed returned: ${canProceed}`,
            );
        }

        if (!canProceed) {
            project.update((p) => ({
                ...p,
                isLoading: false,
                statusMessage: "Media selection cancelled.",
            }));
            console.log(
                `[ProjectView] handleRequestMediaSelection: 'checkUnsavedChangesThenProceed' returned false. Aborting media selection.`,
            );
            return;
        }

        if (selectedTab !== "transcription") {
            console.log(
                `[ProjectView] handleRequestMediaSelection: Current tab is '${selectedTab}', switching to 'transcription'.`,
            );
            await handleTabClick("transcription");
            await tick();
        } else {
            // If already on transcription tab, check if different media is being selected
            if (
                get(transcriptStore).selectedMediaFile?.path !== mediaPath &&
                get(transcriptStore).selectedMediaFile?.path
            ) {
                console.log(
                    "[ProjectView] handleRequestMediaSelection: Different media selected on transcription tab. Performing silent save if dirty.",
                );
                if (transcriptionViewRef) {
                    if (get(transcriptStore).transcriptDirty) {
                        try {
                            await transcriptionViewRef.handleSaveTranscript();
                        } catch (e) {
                            console.error(
                                "[ProjectView] Silent save failed before media selection:",
                                e,
                            );
                        }
                    }
                }
                console.log(
                    `[ProjectView] handleRequestMediaSelection: Already on transcription tab, but different media. Clearing old transcript state.`,
                );
                clearTranscriptState(); // Clear state if switching media within the same tab
            } else if (
                get(transcriptStore).selectedMediaFile?.path === mediaPath
            ) {
                console.log(
                    `[ProjectView] handleRequestMediaSelection: Media path '${mediaPath}' is already selected in transcription tab.`,
                );
            }
        }
        project.update((p) => ({
            ...p,
            isLoading: true,
            statusMessage: `Loading ${mediaName} in Transcription...`,
        })); // This might be redundant if handleTabClick sets loading
        await tick();

        let fileEntry = null;
        function findMediaByPathRecursive(nodes, path) {
            // Keep this helper function local
            if (!Array.isArray(nodes)) return null;
            for (const node of nodes) {
                if (
                    node.file_type === "media" &&
                    !node.is_directory &&
                    node.path === path
                )
                    return node;
                if (node.children?.length > 0) {
                    const found = findMediaByPathRecursive(node.children, path);
                    if (found) return found;
                }
            }
            return null;
        }
        console.log(
            `[ProjectView] handleRequestMediaSelection: Attempting to find FileEntry for mediaPath: '${mediaPath}' in project files:`,
            get(project).files,
        );
        fileEntry = findMediaByPathRecursive(
            get(project).files || [],
            mediaPath,
        );
        console.log(
            `[ProjectView] handleRequestMediaSelection: findMediaByPathRecursive result (fileEntry):`,
            fileEntry,
        );

        if (fileEntry) {
            console.log(
                `[ProjectView] handleRequestMediaSelection: Calling selectMediaStoreAction with fileEntry:`,
                fileEntry,
            );
            await selectMediaStoreAction(fileEntry);
            console.log(
                `[ProjectView] handleRequestMediaSelection: selectMediaStoreAction called.`,
            );
        } else {
            console.error(
                `[ProjectView] handleRequestMediaSelection: FileEntry not found for path: '${mediaPath}'. An error message should be shown to the user.`,
            );
            await message(`Error: Could not find media file (${mediaName}).`, {
                title: "Error",
                type: "error",
            });
            project.update((p) => ({
                ...p,
                statusMessage: `Error selecting ${mediaName}.`,
            }));
        }
        await tick(); // Ensure UI updates after selection or error
        project.update((p) => ({ ...p, isLoading: false })); // Ensure loading is off
    }

    async function handleRequestTranscriptionTabWithMedia(event) {
        const { mediaPath } = event.detail;
        const mediaName = mediaPath.split(/[\\/]/).pop();
        project.update((p) => ({
            ...p,
            isLoading: true,
            statusMessage: `Switching to transcribe ${mediaName}...`,
        }));

        await handleTabClick("transcription");
        await tick();
        await handleRequestMediaSelection({ detail: { mediaPath } });
        await tick();

        project.update((p) => ({
            ...p,
            isLoading: false,
            statusMessage: `Ready to transcribe ${mediaName}. Please select model and language.`,
        }));
    }

    async function handleRequestTranscriptionTabWithMediaAndDialog(event) {
        const { mediaPath } = event.detail;
        const mediaName = mediaPath.split(/[\\/]/).pop();
        project.update((p) => ({
            ...p,
            isLoading: true,
            statusMessage: `Switching to transcribe ${mediaName} and opening dialog...`,
        }));

        await handleTabClick("transcription");
        await tick();
        await handleRequestMediaSelection({ detail: { mediaPath } });
        await tick();

        // Now trigger the transcription dialog
        await requestTranscriptionService();

        project.update((p) => ({
            ...p,
            isLoading: false,
            statusMessage: `Ready to transcribe ${mediaName}. Dialog opened.`,
        }));
    }

    async function handleRequestTranslationTabWithMediaAndDialog(event) {
        const { mediaPath, transcriptPath } = event.detail;
        const mediaName = mediaPath.split(/[\\/]/).pop();
        project.update((p) => ({
            ...p,
            isLoading: true,
            statusMessage: `Switching to translate ${mediaName} and opening dialog...`,
        }));

        await handleTabClick("transcription");
        await tick();
        await handleRequestMediaSelection({ detail: { mediaPath } });
        await tick();

        if (
            transcriptPath &&
            transcriptPath !== get(transcriptStore).currentTranscriptPath
        ) {
            await loadTranscriptFile(transcriptPath);
            await tick();
        }

        // Now trigger the translation dialog
        if (transcriptionTopBarRef) {
            transcriptionTopBarRef.openTranslateModal();
        } else {
            console.warn(
                "[ProjectView] transcriptionTopBarRef not available. Opening translate modal via store.",
            );
            toggleTranslateModal(true);
        }

        project.update((p) => ({
            ...p,
            isLoading: false,
            statusMessage: `Ready to translate ${mediaName}. Dialog opened.`,
        }));
    }

    async function handleRequestTrimInTranscriptionTab(event) {
        const { mediaPath } = event.detail;
        const mediaName = mediaPath.split(/[\\/]/).pop();
        project.update((p) => ({
            ...p,
            isLoading: true,
            statusMessage: `Preparing to trim ${mediaName}...`,
        }));

        if (selectedTab !== "transcription") {
            await handleTabClick("transcription");
            await tick();
        }

        const currentSelectedMedia = get(project).selectedMediaFile?.path;
        if (currentSelectedMedia !== mediaPath) {
            await handleRequestMediaSelection({ detail: { mediaPath } });
            await tick();
            await tick();
        } else {
            project.update((p) => ({
                ...p,
                statusMessage: `Media ${mediaName} already selected.`,
            }));
        }

        if (
            transcriptionViewRef &&
            typeof transcriptionViewRef.activateTrimModeOnPlayer === "function"
        ) {
            transcriptionViewRef.activateTrimModeOnPlayer();
            project.update((p) => ({
                ...p,
                isLoading: false,
                statusMessage: `Trim mode activated for ${mediaName}.`,
            }));
        } else {
            console.warn(
                "[ProjectView] transcriptionViewRef or activateTrimModeOnPlayer is not available.",
            );
            project.update((p) => ({
                ...p,
                isLoading: false,
                statusMessage: `Could not activate trim mode for ${mediaName}.`,
            }));
        }
    }

    function handleImportMediaInSidebar(event) {
        event.preventDefault();
        event.stopPropagation();
        if (importMenuVisible) {
            closeImportMenu();
            return;
        }
        importMenuX = event.clientX;
        importMenuY = event.clientY;
        importMenuVisible = true;
        setTimeout(() => {
            if (closeImportMenuListener)
                document.removeEventListener("click", closeImportMenuListener, {
                    capture: true,
                });
            closeImportMenuListener = (e) => {
                const menu = document.getElementById("import-context-menu-div");
                if (menu && !menu.contains(e.target)) closeImportMenu();
            };
            document.addEventListener("click", closeImportMenuListener, {
                capture: true,
                once: true,
            });
        }, 0);
    }

    // Helper to switch tabs without toggling the panel if already active
    async function ensureTab(tabName) {
        if (selectedTab !== tabName) {
            return await handleTabClick(tabName);
        }
        return true;
    }

    async function triggerMediaImport(actionType) {
        project.update((p) => ({
            ...p,
            isLoading: true,
            statusMessage: `Preparing import...`,
        }));
        let canProceed = true;
        if (selectedTab === "data")
            canProceed = await checkUnsavedChangesThenProceed(
                null,
                `importing ${actionType || "asset"}`,
            );
        else if (selectedTab === "transcription") {
            if (get(transcriptStore).transcriptDirty) {
                if (transcriptionViewRef) {
                    await transcriptionViewRef.handleSaveTranscript();
                }
                clearTranscriptState();
            }
        }
        if (!canProceed) {
            project.update((p) => ({
                ...p,
                isLoading: false,
                statusMessage: "Import cancelled.",
            }));
            return;
        }
        try {
            if (actionType === "audio" || actionType === "video") {
                const importedPath = await importMediaFile(actionType);
                if (importedPath) {
                    if (await ensureTab("data")) {
                        prepareMediaNoteView(importedPath);
                    }
                }
            } else if (actionType === "document") {
                const importedPath = await importDocumentFile();
                if (importedPath) {
                    if (await ensureTab("data")) {
                        prepareDocumentView(importedPath, "documents");
                    }
                }
            } else if (actionType === "table") {
                const importResult = await importTableFile();
                if (
                    importResult &&
                    importResult.table_path &&
                    String(importResult.table_path).trim() !== ""
                ) {
                    headerConfirmationData = {
                        tablePath: importResult.table_path,
                        previewData: importResult.preview_data,
                    };
                    showHeaderConfirmationModal = true;
                } else {
                    // Canceled or error already handled in importTableFile
                    project.update((p) => ({
                        ...p,
                        isLoading: false,
                        isImportingAsset: false,
                    }));
                }
            } else if (actionType === "image") {
                const importedPath = await importImageFile();
                if (importedPath) {
                    if (await ensureTab("data")) {
                        prepareDocumentView(importedPath, "images");
                    }
                }
            } else if (actionType === "transcript") { // this actionType refers to "transcript" import context menu item, keep it.
                showImportTranscriptSourceModal = true;
                project.update((p) => ({ ...p, isLoading: false }));
            } else {
                await message(`Import type (${actionType}) not recognized.`, {
                    title: "Import Error",
                    type: "error",
                });
                project.update((p) => ({ ...p, isLoading: false }));
            }
        } catch (e) {
            project.update((p) => ({
                ...p,
                isLoading: false,
                isImportingAsset: false,
                statusMessage: `Import failed.`,
            }));
        }
        project.update((p) => ({ ...p, isLoading: false }));
    }

    async function handleImportTranscriptSourceConfirm(event) {
        const { sourceType } = event.detail;
        showImportTranscriptSourceModal = false;
        if (sourceType === "msWord") {
            try {
                const newTranscriptPath = await importTranscriptFile("msWord");
                if (newTranscriptPath) {
                    if (await ensureTab("data")) {
                        prepareStandaloneTranscriptView(newTranscriptPath);
                    }
                }
            } catch (e) {
                project.update((p) => ({
                    ...p,
                    isImportingAsset: false,
                    isLoading: false,
                }));
            }
        } else
            await message(`Import from "${sourceType}" not supported.`, {
                title: "Import Error",
                type: "error",
            });
    }

    function closeImportMenu() {
        if (importMenuVisible) {
            importMenuVisible = false;
            if (closeImportMenuListener)
                document.removeEventListener("click", closeImportMenuListener, {
                    capture: true,
                });
            closeImportMenuListener = null;
        }
    }
    function handleImportMenuAction(event, actionType) {
        closeImportMenu();
        triggerMediaImport(actionType);
    }

    async function handleHeaderConfirmation(event) {
        const { hasHeaders } = event.detail;
        const { tablePath } = headerConfirmationData;
        try {
            await invoke("set_table_headers", {
                tablePathStr: tablePath,
                hasHeaders,
            });
            await refreshProjectFiles();
            if (await ensureTab("data")) {
                prepareDocumentView(tablePath, "tables", hasHeaders);
            }
        } catch (error) {
            console.error(`[ProjectView] Error setting table headers:`, error);
            await message(
                `Error setting table headers: ${error.message || error}`,
                { title: "Error", type: "error" },
            );
        }
    }

    async function handleTableCreated(event) {
        const { path } = event.detail;
        await refreshProjectFiles();
        if (await ensureTab("data")) {
            await tick();
            prepareDocumentView(path, "tables");
        }
    }

    $: showLoadingOverlay =
        ($project.isLoading &&
            (get(transcriptStore)?.isTranscribing ?? false)) ||
        $project.isImportingAsset ||
        ($project.selectedDocumentPath && $project.isDocumentLoading) ||
        ($project.currentStandaloneTranscriptPath &&
            $project.isStandaloneTranscriptLoading) ||
        ($project.selectedMediaNotePath &&
            $project.isMediaNoteTranscriptLoading);
</script>

<div
    class="relative flex flex-col h-screen w-full font-sans text-sm text-gray-900 dark:text-gray-200 overflow-hidden"
>
    <!-- Top Bar Area -->
    <div class="flex-shrink-0 relative z-50">
        {#if selectedTab === "data"}
            <DataTopBar
                {dataViewRef}
                getExportData={dataViewRef?.getExportData}
                {activeSubItemPath}
                {activeSubItemType}
                on:requestTranscriptionTabWithMediaAndDialog={handleRequestTranscriptionTabWithMediaAndDialog}
                on:requestTranslationTabWithMediaAndDialog={handleRequestTranslationTabWithMediaAndDialog}
                on:requestImport={handleImportMediaInSidebar}
                on:requestImageExport={() => dataViewRef?.triggerImageExport()}
                on:openConfig={() => {
                    showConfigurationModal = true;
                    toggleTranslateModal(false);
                }}
                on:requestOpenLexicalDocument={(e) =>
                    dataViewRef?.handleRequestOpenLexicalDocument(e)}
                on:requestOpenView={(e) =>
                    dataViewRef?.handleRequestOpenView(e)}
                on:requestClearSubItem={() =>
                    dataViewRef?.handleRequestClearSubItem()}
                on:close={handleCloseProject}
            />
        {:else if selectedTab === "transcription"}
            <TranscriptionTopBar
                bind:this={transcriptionTopBarRef}
                on:requestImport={handleImportMediaInSidebar}
                on:cancelTranslationRequest={handleCancelTranslationRequest}
                on:runTranslationInBackground={() =>
                    setRanTranslationInBackground(true)}
                on:openConfig={() => {
                    showConfigurationModal = true;
                    toggleTranslateModal(false);
                }}
                on:close={handleCloseProject}
            />
        {:else if selectedTab === "tags"}
            <SimpleTopBar
                on:requestImport={handleImportMediaInSidebar}
                on:close={handleCloseProject}
            />
        {/if}
    </div>

    <!-- Main Content Area -->
    <div class="flex flex-grow w-full overflow-visible min-h-0">
        <div
            class="w-12 h-full bg-white bg-gray-200 dark:bg-gray-950 shadow-lg flex flex-col flex-shrink-0 py-1 border-r border-gray-300 dark:border-gray-700"
        >
            <div class="flex-grow flex flex-col space-y-2">
                <button
                    title="Data"
                    aria-label="Data"
                    class="w-full h-10 flex items-center justify-center transition-colors focus:outline-none relative focus:outline-2 focus:outline-blue-500 dark:focus:outline-blue-400"
                    class:border-l-4={selectedTab === "data"}
                    class:border-blue-500={selectedTab === "data"}
                    class:dark:border-blue-400={selectedTab === "data"}
                    class:bg-white={selectedTab === "data"}
                    class:dark:bg-gray-950={selectedTab === "data"}
                    class:text-blue-500={selectedTab === "data"}
                    class:dark:text-accent={selectedTab === "data"}
                    class:hover:bg-gray-300={selectedTab !== "data"}
                    class:dark:hover:bg-gray-800={selectedTab !== "data"}
                    class:text-gray-700={selectedTab !== "data"}
                    class:dark:text-gray-300={selectedTab !== "data"}
                    class:dark:hover:text-gray-100={selectedTab !== "data"}
                    class:hover:text-gray-900={selectedTab !== "data"}
                    on:click={() => handleTabClick("data")}
                >
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        fill="currentColor"
                        class="w-6 h-6 bi bi-journals"
                        viewBox="0 0 16 16"
                    >
                        <path
                            d="M5 0h8a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2 2 2 0 0 1-2 2H3a2 2 0 0 1-2-2h1a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V4a1 1 0 0 0-1-1H3a1 1 0 0 0-1 1H1a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v9a1 1 0 0 0 1-1V2a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1H3a2 2 0 0 1 2-2"
                        />
                        <path
                            d="M1 6v-.5a.5.5 0 0 1 1 0V6h.5a.5.5 0 0 1 0 1h-2a.5.5 0 0 1 0-1zm0 3v-.5a.5.5 0 0 1 1 0V9h.5a.5.5 0 0 1 0 1h-2a.5.5 0 0 1 0-1zm0 2.5v.5H.5a.5.5 0 0 0 0 1h2a.5.5 0 0 0 0-1H2v-.5a.5.5 0 0 0-1 0"
                        />
                    </svg>
                </button>
                <button
                    title="Transcription"
                    aria-label="Transcription"
                    class="w-full h-10 flex items-center justify-center transition-colors focus:outline-none relative focus:outline-2 focus:outline-blue-500 dark:focus:outline-blue-400"
                    class:border-l-4={selectedTab === "transcription"}
                    class:border-blue-500={selectedTab === "transcription"}
                    class:dark:border-blue-400={selectedTab === "transcription"}
                    class:bg-white={selectedTab === "transcription"}
                    class:dark:bg-gray-950={selectedTab === "transcription"}
                    class:text-blue-500={selectedTab === "transcription"}
                    class:dark:text-accent={selectedTab === "transcription"}
                    class:hover:bg-gray-300={selectedTab !== "transcription"}
                    class:dark:hover:bg-gray-800={selectedTab !==
                        "transcription"}
                    class:text-gray-700={selectedTab !== "transcription"}
                    class:dark:text-gray-300={selectedTab !== "transcription"}
                    class:dark:hover:text-gray-100={selectedTab !==
                        "transcription"}
                    class:hover:text-gray-900={selectedTab !== "transcription"}
                    on:click={() => handleTabClick("transcription")}
                >
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        fill="currentColor"
                        class="w-6 h-6 bi bi-chat-square-text"
                        viewBox="0 0 16 16"
                    >
                        <path
                            d="M14 1a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1h-2.5a2 2 0 0 0-1.6.8L8 14.333 6.1 11.8a2 2 0 0 0-1.6-.8H2a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1zM2 0a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2.5a1 1 0 0 1 .8.4l1.9 2.533a1 1 0 0 0 1.6 0l1.9-2.533a1 1 0 0 1 .8-.4H14a2 2 0 0 0 2-2V2a2 2 0 0 0-2-2z"
                        />
                        <path
                            d="M3 3.5a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9a.5.5 0 0 1-.5-.5M3 6a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9A.5.5 0 0 1 3 6m0 2.5a.5.5 0 0 1 .5-.5h5a.5.5 0 0 1 0 1h-5a.5.5 0 0 1-.5-.5"
                        />
                    </svg>
                </button>
                <button
                    title="Tags"
                    aria-label="Tags"
                    class="w-full h-10 flex items-center justify-center transition-colors focus:outline-none relative focus:outline-2 focus:outline-blue-500 dark:focus:outline-blue-400"
                    class:border-l-4={selectedTab === "tags"}
                    class:border-blue-500={selectedTab === "tags"}
                    class:dark:border-blue-400={selectedTab === "tags"}
                    class:bg-white={selectedTab === "tags"}
                    class:dark:bg-gray-950={selectedTab === "tags"}
                    class:text-blue-500={selectedTab === "tags"}
                    class:dark:text-accent={selectedTab === "tags"}
                    class:hover:bg-gray-300={selectedTab !== "tags"}
                    class:dark:hover:bg-gray-800={selectedTab !== "tags"}
                    class:text-gray-700={selectedTab !== "tags"}
                    class:dark:text-gray-300={selectedTab !== "tags"}
                    class:dark:hover:text-gray-100={selectedTab !== "tags"}
                    class:hover:text-gray-900={selectedTab !== "tags"}
                    on:click={() => handleTabClick("tags")}
                >
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        fill="currentColor"
                        class="w-6 h-6 bi bi-tags"
                        viewBox="0 0 16 16"
                    >
                        <path
                            d="M3 2v4.586l7 7L14.586 9l-7-7zM2 2a1 1 0 0 1 1-1h4.586a1 1 0 0 1 .707.293l7 7a1 1 0 0 1 0 1.414l-4.586 4.586a1 1 0 0 1-1.414 0l-7-7A1 1 0 0 1 2 6.586z"
                        />
                        <path
                            d="M5.5 5a.5.5 0 1 1 0-1 .5.5 0 0 1 0 1m0 1a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3M1 7.086a1 1 0 0 0 .293.707L8.75 15.25l-.043.043a1 1 0 0 1-1.414 0l-7-7A1 1 0 0 1 0 7.586V3a1 1 0 0 1 1-1z"
                        />
                    </svg>
                </button>
            </div>
            <div class="mt-auto flex flex-col space-y-2 pb-2 w-full">
                <button
                    title="Help"
                    aria-label="Help"
                    on:click={() => (showHelpModal = true)}
                    class="w-full h-10 rounded-tl-md rounded-bl-md flex items-center justify-center text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-500 hover:text-gray-900 dark:hover:text-gray-100 transition-colors focus:outline-none focus:outline-2 focus:outline-blue-500 dark:focus:outline-blue-400"
                >
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        fill="currentColor"
                        class="w-6 h-6 bi bi-question-circle"
                        viewBox="0 0 16 16"
                    >
                        <path
                            d="M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14m0 1A8 8 0 1 0 8 0a8 8 0 0 0 0 16"
                        />
                        <path
                            d="M5.255 5.786a.237.237 0 0 0 .241.247h.825c.138 0 .248-.113.266-.25.09-.656.54-1.134 1.342-1.134.686 0 1.314.343 1.314 1.168 0 .635-.374.927-.965 1.371-.673.489-1.206 1.06-1.168 1.987l.003.217a.25.25 0 0 0 .25.246h.811a.25.25 0 0 0 .25-.25v-.105c0-.718.273-.927 1.01-1.486.609-.463 1.244-.977 1.244-2.056 0-1.511-1.276-2.241-2.673-2.241-1.267 0-2.655.59-2.75 2.286m1.557 5.763c0 .533.425.927 1.01.927.609 0 1.028-.394 1.028-.927 0-.552-.42-.94-1.029-.94-.584 0-1.009.388-1.009.94"
                        />
                    </svg>
                </button>
                <button
                    title="Configure"
                    aria-label="Configure"
                    on:click={() => (showConfigurationModal = true)}
                    class="w-full h-10 rounded-tl-md rounded-bl-md flex items-center justify-center transition-colors hover:bg-gray-300 dark:hover:bg-gray-800 focus:outline-none focus:outline-2 focus:outline-blue-500 dark:focus:outline-blue-400 
                    {!hasConfigIssues ? 'text-gray-700 dark:text-gray-300' : ''}
                    {hasCriticalConfigIssues ? 'text-red-500' : ''}
                    {!hasCriticalConfigIssues && hasNonCriticalConfigIssues ? 'text-yellow-500' : ''}"
                >
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        fill="currentColor"
                        class="w-6 h-6 bi bi-gear-wide-connected"
                        viewBox="0 0 16 16"
                    >
                        <path
                            d="M7.068.727c.243-.97 1.62-.97 1.864 0l.071.286a.96.96 0 0 0 1.622.434l.205-.211c.695-.719 1.888-.03 1.613.931l-.08.284a.96.96 0 0 0 1.187 1.187l.283-.081c.96-.275 1.65.918.931 1.613l-.211.205a.96.96 0 0 0 .434 1.622l.286.071c.97.243.97 1.62 0 1.864l-.286.071a.96.96 0 0 0-.434 1.622l.211.205c.719.695.03 1.888-.931 1.613l-.284-.08a.96.96 0 0 0-1.187 1.187l.081.283c.275.96-.918 1.65-1.613.931l-.205-.211a.96.96 0 0 0-1.622.434l-.071.286c-.243.97-1.62.97-1.864 0l-.071-.286a.96.96 0 0 0-1.622-.434l-.205.211c-.695.719-1.888.03-1.613-.931l.08-.284a.96.96 0 0 0-1.186-1.187l-.284.081c-.96.275-1.65-.918-.931-1.613l.211-.205a.96.96 0 0 0-.434-1.622l-.286-.071c-.97-.243-.97-1.62 0-1.864l.286-.071a.96.96 0 0 0 .434-1.622l-.211-.205c-.719-.695-.03-1.888.931-1.613l.284.08a.96.96 0 0 0 1.187-1.186l-.081-.284c-.275-.96.918-1.65 1.613-.931l.205.211a.96.96 0 0 0 1.622-.434zM12.973 8.5H8.25l-2.834 3.779A4.998 4.998 0 0 0 12.973 8.5m0-1a4.998 4.998 0 0 0-7.557-3.779l2.834 3.78zM5.048 3.967l-.087.065zm-.431.355A4.98 4.98 0 0 0 3.002 8c0 1.455.622 2.765 1.615 3.678L7.375 8zm.344 7.646.087.065z"
                        />
                    </svg>
                </button>
            </div>
        </div>

        <div
            class="flex flex-col flex-1 h-full bg-gray-100 dark:bg-gray-950 min-w-0"
        >
            <div class="flex flex-col flex-grow min-h-0">
                {#if selectedTab === "transcription"}
                    <TranscriptionView
                        bind:this={transcriptionViewRef}
                        on:requestopentab={handleRequestOpenTab}
                        on:requestmediaselection={handleRequestMediaSelection}
                    />
                {:else if selectedTab === "data"}
                    <DataView
                        bind:this={dataViewRef}
                        bind:activeSubItemPath
                        bind:activeSubItemType
                        on:requestmediaselection={handleRequestMediaSelection}
                        on:requestTranscriptionTabWithMedia={handleRequestTranscriptionTabWithMedia}
                        on:requestTrimInTranscriptionTab={handleRequestTrimInTranscriptionTab}
                        on:requestTranscriptionTabWithMediaAndDialog={handleRequestTranscriptionTabWithMediaAndDialog}
                        on:requestTranslationTabWithMediaAndDialog={handleRequestTranslationTabWithMediaAndDialog}
                        on:requestviewchange={handleRequestOpenTab}
                    />
                {:else if selectedTab === "tags"}
                    <TagsView
                        bind:this={tagsViewRef}
                        on:requestviewchange={handleRequestOpenTab}
                    />
                {/if}
            </div>
        </div>
    </div>
    <BottomBar {selectedTab} />

    <TranscribeConfirmModal
        bind:this={transcribeModalRef}
        bind:showModal={$transcriptStore.showTranscribeModal}
        fileName={$transcriptStore.selectedMediaFile?.name ?? "N/A"}
        modelName={$transcriptStore.selectedModelName ?? "None Selected"}
        language={$transcriptStore.selectedLanguage ?? "N/A"}
        speakers={$transcriptStore.speakers}
        jobId={$transcriptStore.transcriptionJobId}
        {downloadedModelsList}
        mediaDuration={$transcriptStore.player?.duration || 0}
        lastSegmentEndTime={$transcriptStore.segments?.length > 0
            ? $transcriptStore.segments[$transcriptStore.segments.length - 1]
                  .end_time
            : 0}
        {languageOptions}
        initialDiarizationEnabled={$transcriptStore.diarizationEnabledForNextJob}
        on:confirmStart={onConfirmTranscriptionStart}
        on:cancelRequest={handleCancelTranscriptionRequest}
        on:openConfig={() => {
            showConfigurationModal = true;
            toggleTranscribeModal(false);
        }}
        on:closeAndReset={() => {
            transcriptStore.update((ts) => ({
                ...ts,
                showTranscribeModal: false,
                transcriptionJobStatus: null,
                transcriptionErrorMessage: null,
                transcriptionJobId: null,
                isTranscribing: false,
                transcriptionProgress: { percent: 0, message: "" },
                transcriptionStartTime: null,
            }));
            // Also clear any pending data related to a job that was just acknowledged as done/error/cancelled
            clearPendingTranscriptData();
            const ranInBackground = get(transcriptStore).ranInBackground;
            if (ranInBackground) {
                // If it ran in background, ensure this is reset for next time.
                setRanInBackground(false);
            }
        }}
        on:runInBackgroundAndClose={() => {
            setRanInBackground(true);
            transcriptStore.update((ts) => ({
                ...ts,
                showTranscribeModal: false,
            }));
        }}
    />

    <UnsavedChangesModal
        bind:showModal={$project.showUnsavedChangesModal}
        itemName={$project.unsavedItemName}
        itemType={$project.unsavedItemType}
        on:save={handleUnsavedResponse}
        on:discard={handleUnsavedResponse}
        on:cancel={handleUnsavedResponse}
    />
    <ConfirmConversionModal
        bind:showModal={$project.showConfirmConversionModal}
        fileName={$project.conversionFileName}
        on:confirm={handleConversionResponse}
        on:cancel={handleConversionResponse}
    />
    <ImportTranscriptSourceModal
        bind:showModal={showImportTranscriptSourceModal}
        on:confirm={handleImportTranscriptSourceConfirm}
        on:close={() => (showImportTranscriptSourceModal = false)}
    />
    <HeaderConfirmationModal
        bind:showModal={showHeaderConfirmationModal}
        tablePath={headerConfirmationData.tablePath}
        previewData={headerConfirmationData.previewData}
        on:confirm={handleHeaderConfirmation}
    />

    <CreateTableModal
        bind:showModal={showCreateTableModal}
        on:tableCreated={handleTableCreated}
    />

    <CreateGroupModal
        bind:showModal={showCreateGroupModal}
        projectUuid={$project?.id}
        fileToAdd={fileToAddForGroup}
        on:close={() => {
            showCreateGroupModal = false;
            fileToAddForGroup = null;
        }}
        on:groupCreatedAndFileAdded={(event) => {
            showCreateGroupModal = false;
            fileToAddForGroup = null;
            project.update((p) => ({
                ...p,
                statusMessage: `File ${event.detail.file.name} added to new group ${event.detail.group.name}.`,
            }));
        }}
        on:groupCreated={(event) => {
            showCreateGroupModal = false;
            fileToAddForGroup = null;
            project.update((p) => ({
                ...p,
                statusMessage: `Group ${event.detail.group.name} created.`,
            }));
        }}
    />

    {#if importMenuVisible}
        <div
            id="import-context-menu-div"
            class="fixed z-50 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded-md shadow-xl py-1 text-sm min-w-[140px]"
            style="left: {importMenuX}px; top: {importMenuY}px;"
            on:click|stopPropagation
            role="menu"
            tabindex="0"
            on:keydown={(e) => {
                if (e.key === "Escape") closeImportMenu();
            }}
        >
            <button
                on:click={(event) => handleImportMenuAction(event, "audio")}
                class="flex items-center space-x-2 w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-800 dark:text-gray-200"
            >
                <Music class="w-4 h-4" /><span>Audio</span>
            </button>
            <button
                on:click={(event) => handleImportMenuAction(event, "document")}
                class="flex items-center space-x-2 w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-800 dark:text-gray-200"
            >
                <FileText class="w-4 h-4" /><span>Document</span>
            </button>
            <button
                on:click={(event) => handleImportMenuAction(event, "image")}
                class="flex items-center space-x-2 w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-800 dark:text-gray-200"
            >
                <ImageIcon class="w-4 h-4" /><span>Image</span>
            </button>
            <button
                on:click={(event) => handleImportMenuAction(event, "table")}
                class="flex items-center space-x-2 w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-800 dark:text-gray-200"
            >
                <Sheet class="w-4 h-4" /><span>Table</span>
            </button>
            <button
                on:click={(event) =>
                    handleImportMenuAction(event, "transcript")}
                class="flex items-center space-x-2 w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-800 dark:text-gray-200"
            >
                <MessageSquareText class="w-4 h-4" /><span>Transcript</span>
            </button>
            <button
                on:click={(event) => handleImportMenuAction(event, "video")}
                class="flex items-center space-x-2 w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-800 dark:text-gray-200"
            >
                <Film class="w-4 h-4" /><span>Video</span>
            </button>
        </div>
    {/if}

    {#if showLoadingOverlay}
        <div
            class="absolute inset-0 z-[110] flex items-center justify-center bg-black/30 backdrop-blur-sm"
        >
            <div
                class="flex flex-col items-center p-6 bg-white dark:bg-gray-900 rounded-lg shadow-xl"
            >
                <Loader class="w-12 h-12 text-blue-500 animate-spin mb-3" />
                <p class="text-sm text-gray-700 dark:text-gray-400">
                    {$project.statusMessage || "Loading..."}
                </p>
            </div>
        </div>
    {/if}
</div>
<ConfigurationModal
    bind:showModal={showConfigurationModal}
    on:close={() => (showConfigurationModal = false)}
/>
<HelpModal
    bind:showModal={showHelpModal}
    on:close={() => (showHelpModal = false)}
/>

<style lang="postcss">
    ::-webkit-scrollbar {
        @apply w-2 h-2;
    }
    ::-webkit-scrollbar-track {
        @apply bg-gray-100 rounded-lg;
    }
    ::-webkit-scrollbar-thumb {
        @apply bg-gray-300 rounded-lg border-2 border-solid border-gray-100;
    }
    ::-webkit-scrollbar-thumb:hover {
        @apply bg-gray-400;
    }
    * {
        scrollbar-width: thin;
        scrollbar-color: #d1d5db #f3f4f6;
    }
    .dark ::-webkit-scrollbar-track {
        @apply bg-gray-800;
    }
    .dark ::-webkit-scrollbar-thumb {
        @apply bg-gray-500 border-gray-800;
    }
    .dark ::-webkit-scrollbar-thumb:hover {
        @apply bg-gray-400;
    }
    .dark * {
        scrollbar-color: #6b7280 #1f2937;
    }
    .size-6 {
        width: 1.5rem;
        height: 1.5rem;
    }
    .min-h-0 {
        min-height: 0;
    }
</style>
