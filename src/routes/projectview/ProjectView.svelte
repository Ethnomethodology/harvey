<!-- src/routes/projectview/ProjectView.svelte -->
<script>
	import { onMount, onDestroy, tick } from 'svelte';
	import { page } from '$app/stores';
	import { get } from 'svelte/store';
	import { emit, listen } from '@tauri-apps/api/event';
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
            loadTranscriptFile
	} from '$lib/services/projectService.js';
	import { getDownloadedModels } from '$lib/services/configureActions.js';
	import {
		languageOptions
	} from '$lib/constants/transcriptionOptions.js';
	import {
        project,
        hideUnsavedChangesPrompt,
        hideConversionPrompt,
        prepareDocumentView,
        prepareImportedTranscriptView,
        prepareMediaNoteView,
    } from '$lib/stores/projectStore.js';
    import { fetchAllTags } from '$lib/stores/tagStore.js';
    import { transcriptStore, setRanInBackground, setRanTranslationInBackground, toggleTranscribeModal, selectMedia as selectMediaStoreAction, clearTranscriptState, setDiarizationPreference, setSelectedModel, setSelectedLanguage, setTranslateToEnglish, updateSpeakerConfig, setTranslationStatus, toggleTranslateModal, clearPendingTranscriptData } from '$lib/stores/transcriptStore.js';
    import { message, confirm } from '@tauri-apps/plugin-dialog';
    import { getCurrentWindow } from '@tauri-apps/api/window';
	import { invoke } from '@tauri-apps/api/core';
    import { configStatus, updateConfigStatus } from '$lib/stores/configStatusStore.js';


	import BottomBar from '$lib/components/projectview/shared/BottomBar.svelte';
	import TranscribeConfirmModal from '$lib/components/projectview/modals/TranscribeConfirmModal.svelte';
    
    import UnsavedChangesModal from '$lib/components/projectview/modals/UnsavedChangesModal.svelte';
    import ConfirmConversionModal from '$lib/components/projectview/modals/ConfirmConversionModal.svelte';
    import ImportTranscriptSourceModal from '$lib/components/projectview/modals/ImportTranscriptSourceModal.svelte';
	import HeaderConfirmationModal from '$lib/components/projectview/modals/HeaderConfirmationModal.svelte';
	import ConfigurationModal from '$lib/components/modals/ConfigurationModal.svelte';
	import DataView from '$lib/components/projectview/data/DataView.svelte';
    import TranscriptionsView from '$lib/components/projectview/transcriptions/TranscriptionsView.svelte';
    import TagsView from '$lib/components/projectview/tags/TagsView.svelte';
    import { Loader } from 'lucide-svelte';
    import DataTopBar from '$lib/components/projectview/data/DataTopBar.svelte';
    import TranscriptionsTopBar from '$lib/components/projectview/transcriptions/TopBar.svelte';
    import SimpleTopBar from '$lib/components/projectview/shared/SimpleTopBar.svelte';
    import panelStateStore from '$lib/stores/panelStateStore.js';


	let transcribeModalRef;
    let transcriptionsViewRef;
    let dataViewRef;
    let tagsViewRef;
	let selectedTab = 'data';
    let importMenuVisible = false;
    let importMenuX = 0;
    let importMenuY = 0;
    let closeImportMenuListener = null;
    let appWindow = null;
    let removeCloseRequestListener = null;
    let handlingCloseRequest = false;
    let showImportTranscriptSourceModal = false;
	let showHeaderConfirmationModal = false;
	let showConfigurationModal = false;
	let headerConfirmationData = {};
    let unlistenTranscriptionComplete = null;
    let unlistenTranslationComplete = null;
    let unlistenSelectMedia = null;

	// Transcription configuration data
	let downloadedModelsList = [];
	
	let isLoadingTranscriptionConfig = true;

async function loadTranscriptionConfigData() {
		isLoadingTranscriptionConfig = true;
		try {
			const localModelsResult = await getDownloadedModels();
			downloadedModelsList = localModelsResult;
		} catch (e) {
			console.error("[ProjectView] Error during transcription configuration loading for modal:", e);
			downloadedModelsList = [];
		} finally {
			isLoadingTranscriptionConfig = false;
		}
	}

async function onConfirmTranscriptionStart(event) {
    const { selectedModel, transcriptionMode, selectedLanguage, translateToEnglish, enableDiarization, speakersConfig } = event.detail;

    // Update the central transcript store with the user's choices from the modal
    setSelectedModel(selectedModel);
    setSelectedLanguage(selectedLanguage);
    setTranslateToEnglish(translateToEnglish);
    setDiarizationPreference(enableDiarization);
    // The updateSpeakerConfig from the store expects individual arguments, not the object
    updateSpeakerConfig(speakersConfig.count, speakersConfig.names, speakersConfig.translatedNames);

    await handleConfirmStartTranscription(transcriptionMode);
}

// Reactive declaration for config issues
$: hasConfigIssues = !$configStatus.python_libraries_installed || !$configStatus.hf_token_present || !$configStatus.transcription_models_downloaded || !$configStatus.diarization_model_downloaded || !$configStatus.translation_models_downloaded;

	onMount(async () => {
        appWindow = getCurrentWindow();
		await loadTranscriptionConfigData(); // Load model/cloud config
        await updateConfigStatus(); // Update config status on mount

		const xmlPath = $page.url.searchParams.get('xmlPath');
		if (xmlPath && xmlPath.trim() !== '') {
			try {
                await loadProjectDataAndUpdateStore(xmlPath);
                await fetchAllTags(); // Fetch all tags after project data is loaded
            }
            catch (e) { console.error('[ProjectView] Error during initial project load:', e); }
		} else {
			project.update((p) => ({ ...p, isLoading: false, error: 'Project path is missing.', statusMessage: 'Error: Project path is missing.' }));
			console.error('[ProjectView] Mount error: Project XML path missing in URL parameters.');
		}
		initializeProgressListener();
        initializeTranslationProgressListener();

        unlistenTranslationComplete = await listen('translation_job_completed', (event) => {
            if (event.payload && event.payload.status === 'done') {
                const { newTranscriptPath } = event.payload;
                silentlyRefreshProjectData(get(project).xmlPath).then(() => {
                    if (newTranscriptPath) {
                        loadTranscriptFile(newTranscriptPath);
                    }
                });
            }
        });

		await tick();
		if (transcribeModalRef) { registerTranscribeModal(transcribeModalRef); }
        else { console.warn('[ProjectView] TranscribeConfirmModal reference not available on mount.'); }
		window.addEventListener('keydown', handleGlobalKeys);

        unlistenSelectMedia = await listen('select_media_in_transcription_tab', async (event) => {
            const { mediaPath } = event.payload;
            if (mediaPath) {
                await handleRequestMediaSelection({ detail: { mediaPath } });
            }
        });

        if (appWindow) {
            removeCloseRequestListener = await appWindow.listen('tauri://close-requested', handleWindowCloseRequest);
        } else {
            console.error('[ProjectView] Could not get appWindow reference to attach close listener.');
        }
	});

	onDestroy(() => {
		cleanupProgressListener();
        if (unlistenTranscriptionComplete) {
            unlistenTranscriptionComplete();
        }
        if (unlistenTranslationComplete) {
            unlistenTranslationComplete();
        }
        if (unlistenSelectMedia) {
            unlistenSelectMedia();
        }
		window.removeEventListener('keydown', handleGlobalKeys);
        if (closeImportMenuListener) { document.removeEventListener('click', closeImportMenuListener, { capture: true }); closeImportMenuListener = null; }
        if (removeCloseRequestListener) {
            removeCloseRequestListener();
            removeCloseRequestListener = null;
        }
	});

	

	function handleGlobalKeys(event) {
        const proj = get(project);
        const ts = get(transcriptStore);
        const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0;
        const modKey = isMac ? event.metaKey : event.ctrlKey;
        if (modKey && event.key.toLowerCase() === 's') {
            event.preventDefault();
            if (selectedTab === 'transcriptions' && transcriptionsViewRef) {
                transcriptionsViewRef.handleSaveTranscript();
            } else if (selectedTab === 'data') {
                const activeDocEditor = proj.activeDocumentEditorRef?.ref;
                const activeImpTsEditor = proj.activeImportedTranscriptEditorRef?.ref;
                const activeMediaNoteEditor = proj.activeMediaNoteEditorRef?.ref;

                if (!proj.autosaveEnabled) {
                    if ((proj.isDocumentDirty || proj.isDocumentMetadataDirty) && activeDocEditor && typeof activeDocEditor.save === 'function') {
                        activeDocEditor.save().catch(e => console.error("Ctrl+S document save failed", e));
                    } else if (proj.isImportedTranscriptDirty && activeImpTsEditor && typeof activeImpTsEditor.save === 'function') {
                        activeImpTsEditor.save().catch(e => console.error("Ctrl+S imported transcript save failed", e));
                    } else if (proj.isMediaNoteTranscriptDirty && activeMediaNoteEditor && typeof activeMediaNoteEditor.save === 'function') {
                        activeMediaNoteEditor.save().catch(e => console.error("Ctrl+S media note save failed", e));
                    }
                }
            } return;
        }
        if (modKey && event.key.toLowerCase() === 'e') { if (selectedTab === 'transcriptions' && transcriptionsViewRef) { event.preventDefault(); transcriptionsViewRef.handleToggleEditMode(); } return; }
        if (modKey && event.key.toLowerCase() === 'z' && !event.shiftKey) { if (selectedTab === 'transcriptions' && transcriptionsViewRef && ts.transcriptUndoStack?.length > 0) { event.preventDefault(); transcriptionsViewRef.handleUndoRequest(); } return; }
        if (modKey && (event.key.toLowerCase() === 'y' || (event.shiftKey && event.key.toLowerCase() === 'z'))) { if (selectedTab === 'transcriptions' && transcriptionsViewRef && ts.transcriptRedoStack?.length > 0) { event.preventDefault(); transcriptionsViewRef.handleRedoRequest(); } return; }
        if (event.key === 'F8') { if (selectedTab === 'transcriptions' && transcriptionsViewRef && transcriptionsViewRef.mediaPlayerRef) { event.preventDefault(); transcriptionsViewRef.mediaPlayerRef.handleTogglePlay(); } return; }
    }


	function handleModalClose(event) {
        const { acknowledged, finalStatus } = event.detail || {};
        toggleTranscribeModal(false);

        if (acknowledged) {
            if (finalStatus === 'done') {
                const jobFinishedPath = get(transcriptStore).mediaPathForLastJob;
                const currentSelectionPathInUI = get(transcriptStore).selectedMediaFile?.path;
                const activeMediaWhenJobStarted = get(transcriptStore).activeMediaDuringTranscriptionStart;
                const currentProjectXmlPath = get(project).xmlPath;
            const ranInBackground = get(transcriptStore).ranInBackground;
                // const pendingPath = get(transcriptStore).pendingTranscriptPathForJobDone;
                // const pendingSegments = get(transcriptStore).pendingSegmentsForJobDone;

            if (!ranInBackground && jobFinishedPath) {
                console.log('[ProjectView] Modal closed after foreground transcription, refreshing files and selecting media:', jobFinishedPath);
                let refreshPromise = refreshProjectFiles(jobFinishedPath); // This should select the media and trigger transcript load
                refreshPromise.then(() => {
                    console.log('[ProjectView HMC] Entered refreshPromise.then()');
                    // This block runs after refreshProjectFiles has completed and its UI updates have likely propagated
                    const projectFiles = get(project).files;
                    let mediaFileEntry = null;
                    // Function to find the media file entry (assuming it's similar to what refreshProjectFiles might use or what's available)
                    function findMediaByPathRecursive(nodes, path) {
                        if (!Array.isArray(nodes)) return null;
                        for (const node of nodes) {
                            if (node.file_type === 'media' && !node.is_directory && node.path === path) return node;
                            if (node.children?.length > 0) { const found = findMediaByPathRecursive(node.children, path); if (found) return found; }
                        }
                        return null;
                    }
                    if (!get(transcriptStore).ranInBackground && jobFinishedPath) { // Ensure to use the current ranInBackground value
                        let mediaFileEntry = null; // Defined here
                        // Function to find the media file entry (assuming it's similar to what refreshProjectFiles might use or what's available)
                        function findMediaByPathRecursive(nodes, path) { // Definition moved inside or ensured accessible
                            if (!Array.isArray(nodes)) return null;
                            for (const node of nodes) {
                                if (node.file_type === 'media' && !node.is_directory && node.path === path) return node;
                                if (node.children?.length > 0) { const found = findMediaByPathRecursive(node.children, path); if (found) return found; }
                            }
                            return null;
                        }
                        mediaFileEntry = findMediaByPathRecursive(projectFiles, jobFinishedPath);

                        if (mediaFileEntry) {
                            selectMediaStoreAction(mediaFileEntry); // This line should already exist

                            const newTranscriptPath = get(transcriptStore).pendingTranscriptPathForJobDone;
                            if (newTranscriptPath) {
                                console.log(`[ProjectView] Explicitly loading new transcript: ${newTranscriptPath}`);
                                loadTranscriptFile(newTranscriptPath).catch(err => {
                                    console.error(`[ProjectView] Error explicitly loading new transcript: ${err.message || err}`);
                                    // Optional: project.update(p => ({...p, error: `Failed to load new transcript: ${err.message || err}`}));
                                });
                            }
                        } else {
                            console.warn(`[ProjectView] Media file entry not found after refresh for path: ${jobFinishedPath}. Cannot auto-load transcript.`);
                        }
                    }
                // THE CLEANUP CODE SHOULD GO HERE, after the conditional processing,
                // but still inside .then()
                console.log('[ProjectView HMC .then()] Foreground processing in .then() complete. Clearing job context and pending data.');
                transcriptStore.update(ts => ({
                    ...ts,
                    mediaPathForLastJob: null,
                    activeMediaDuringTranscriptionStart: null
                }));
                clearPendingTranscriptData();

                }).catch(err => {
                    console.error("[ProjectView] Error during refreshProjectFiles sequence in handleModalClose:", err);
                    // CRITICAL: Also clear data on error to prevent stale state if refresh fails!
                    console.log('[ProjectView HMC .catch()] Error in refreshPromise. Clearing job context and pending data to prevent stale state.');
                    transcriptStore.update(ts => ({
                        ...ts,
                        mediaPathForLastJob: null,
                        activeMediaDuringTranscriptionStart: null
                    }));
                    clearPendingTranscriptData();
                });
            } else { // This 'else' corresponds to the "if (!ranInBackground && jobFinishedPath)"
                // This case means it ran in background OR (foreground but !jobFinishedPath).
                // A silent refresh might have already been done by the event listener if it was a background task.
                // Or, if it was foreground and no job path, the earlier call to silentlyRefreshProjectData handles it.
                console.log('[ProjectView HMC else] Modal closed (ranInBackground or no jobFinishedPath for foreground). Clearing modal-specific job context.');
                if (currentProjectXmlPath && get(transcriptStore).ranInBackground) { // Only if ran in background and refresh needed
                    // If it truly ran in background, the event listener should have refreshed.
                    // This silent refresh is more of a fallback if that event was missed or if state is complex.
                    // However, the primary silent refresh for background is now handled by the event listener.
                    // For foreground with no job path, silent refresh was done before promise.
                    // So, this specific call to silentlyRefreshProjectData might be redundant if event listener works.
                    // Let's keep it for now as a safeguard for the ranInBackground path.
                    silentlyRefreshProjectData(currentProjectXmlPath);
                } else if (!get(transcriptStore).ranInBackground && !jobFinishedPath) {
                    // Foreground, but no job path to refresh. A general silent refresh was already done.
                    console.log('[ProjectView HMC else] Foreground task with no jobFinishedPath. Silent refresh was done prior to promise.');
                }


                transcriptStore.update(ts => ({ // Clear context related to the job this modal instance was tracking
                    ...ts,
                    mediaPathForLastJob: null,
                    activeMediaDuringTranscriptionStart: null
                }));

                // If it was a foreground task but jobFinishedPath was null,
                // then HMC is responsible for clearing pending data because the .then() part of refreshPromise was skipped.
                if (!get(transcriptStore).ranInBackground && !jobFinishedPath) { // Check current ranInBackground
                     console.log('[ProjectView HMC else] jobFinishedPath was null for a foreground task. Clearing pending data now.');
                     clearPendingTranscriptData();
                }
                // Note: `silentlyRefreshProjectData` for the case of (foreground && !jobFinishedPath)
                // was already called before `refreshPromise` was defined.
            }
            // Synchronous cleanup removed from here
        }
    } else { // This 'else' corresponds to "if (acknowledged)"
        if (finalStatus === 'running' || finalStatus === 'cancelling') {
                console.log(`[ProjectView] TranscribeModal closed by user (acknowledged:false) while status was: ${finalStatus}. Background process continues.`);
            }
        }
    }
    function handleUnsavedResponse(event) { const action = event.type; const callback = get(project)[`onUnsaved${action.charAt(0).toUpperCase() + action.slice(1)}`]; if (typeof callback === 'function') { callback(); } else { console.warn(`[ProjectView] No valid callback for unsaved action: ${action}`); hideUnsavedChangesPrompt(); } } 
    function handleConversionResponse(event) { const action = event.type; const callback = get(project)[`onConversion${action.charAt(0).toUpperCase() + action.slice(1)}`]; if (typeof callback === 'function') { callback(); } else { console.warn(`[ProjectView] No valid callback for conversion action: ${action}`); hideConversionPrompt(); } }

    async function handleWindowCloseRequest() {
        if (handlingCloseRequest) return;
        handlingCloseRequest = true;
        let canProceed = false;
        try {
            if (selectedTab === 'data') {
                canProceed = await checkUnsavedChangesThenProceed(null, "closing the project window");
            } else if (selectedTab === 'transcriptions') {
                if (transcriptionsViewRef) {
                    try {
                        // Attempt to save and exit edit mode. handleToggleEditMode handles dirty check and confirmation.
                        await transcriptionsViewRef.handleToggleEditMode();
                        canProceed = true;
                    } catch (e) {
                        // If handleToggleEditMode throws, it means the user cancelled the save/discard.
                        canProceed = false;
                    }
                } else {
                    canProceed = true; // No transcriptionsViewRef, so no dirty state to manage
                }
            } else { canProceed = true; }
        } catch (error) { canProceed = false; }
        if (canProceed) {
            if (removeCloseRequestListener) { removeCloseRequestListener(); removeCloseRequestListener = null; }
            if (appWindow) { try { await appWindow.close(); } catch (error) { await message(`Error closing project window: ${error}`, {title: "Error", type: "error"});}}
            else { await message("Internal error: Could not get window reference to close.", {title: "Error", type: "error"});}
        }
        handlingCloseRequest = false;
    }


	async function handleTabClick(tabName) {
        if (selectedTab === tabName) {
            // If the same tab is clicked, toggle the corresponding panel
            if (tabName === 'data') {
                panelStateStore.toggleDataLeftPanel();
            } else if (tabName === 'transcriptions') {
                panelStateStore.toggleTranscriptionPanel();
            } else if (tabName === 'tags') {
                panelStateStore.toggleTagsLeftPanel();
            }
            return;
        }
        project.update(p => ({ ...p, isLoading: true, statusMessage: `Switching to ${tabName} tab...` }));

        let canProceed = true;
        // Check for unsaved changes in the transcriptions tab before switching away
        if (selectedTab === 'transcriptions' && get(transcriptStore).transcriptDirty) {
            project.update(p => ({
                ...p,
                showUnsavedChangesModal: true,
                unsavedItemName: 'current transcript',
                unsavedItemType: 'transcript',
                // Set up callbacks for the modal actions
                onUnsavedSave: async () => {
                    hideUnsavedChangesPrompt();
                    try {
                        await transcriptionsViewRef.handleSaveTranscript();
                        // After successful save, proceed with tab switch
                        await proceedTabSwitch(tabName);
                    } catch (e) {
                        // If save fails, keep user on current tab and show error
                        project.update(p => ({ ...p, isLoading: false, statusMessage: 'Save failed, tab switch cancelled.' }));
                        message(`Failed to save transcript: ${e.message || e}`, { title: "Save Error", type: "error" });
                    }
                },
                onUnsavedDiscard: async () => {
                    hideUnsavedChangesPrompt();
                    // Discard changes and proceed with tab switch
                    transcriptStore.update(ts => ({ ...ts, transcriptDirty: false, transcriptUndoStack: [], transcriptRedoStack: [] }));
                    await proceedTabSwitch(tabName);
                },
                onUnsavedCancel: () => {
                    hideUnsavedChangesPrompt();
                    // Cancel tab switch
                    project.update(p => ({ ...p, isLoading: false, statusMessage: 'Tab switch cancelled.' }));
                }
            }));
            return; // Stop here, wait for user interaction with the modal
        }
        // Existing data tab unsaved changes check (if any)
        else if (selectedTab === 'data') {
            canProceed = await checkUnsavedChangesThenProceed(null, "switching tabs");
        }

        if (!canProceed) {
            project.update(p => ({ ...p, isLoading: false, statusMessage: 'Tab switch cancelled.' }));
            return;
        }

        // If no unsaved changes or user chose to save/discard, proceed
        await proceedTabSwitch(tabName);
    }

    async function proceedTabSwitch(tabName) {
        selectedTab = tabName;

        project.update(p => ({...p, isDocumentLoading: false, isImportedTranscriptLoading: false, isMediaNoteTranscriptLoading: false}));

        if (selectedTab === 'data') {
            if (!get(project).selectedDocumentPath && !get(project).currentImportedTranscriptPath && !get(project).selectedMediaNotePath) {
                prepareDocumentView(null);
            }
        } else if (selectedTab === 'transcriptions') {
            prepareDocumentView(null);
            // If no media is selected, find and select the first one
            if (!get(transcriptStore).selectedMediaFile) {
                const proj = get(project);
                let firstMediaFile = null;

                function findFirstMediaRecursive(nodes) {
                    if (!Array.isArray(nodes)) return;
                    for (const node of nodes) {
                        if (node.file_type === 'media' && !node.is_directory) {
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
                    console.log(`[ProjectView] No media selected on transcriptions tab switch. Auto-selecting first media:`, firstMediaFile.path);
                    // Use a timeout to ensure the UI has updated before selecting the media
                    setTimeout(() => {
                        handleRequestMediaSelection({ detail: { mediaPath: firstMediaFile.path } });
                    }, 0);
                }
            }
        }

        if (tabName !== 'transcriptions' && transcriptionsViewRef?.mediaPlayerRef?.videoElement && !transcriptionsViewRef.mediaPlayerRef.videoElement.paused) {
            try { await transcriptionsViewRef.mediaPlayerRef.videoElement.pause(); } catch(e) { console.warn("Error pausing main video on tab switch:", e); }
        }
        await tick();
        project.update(p => ({...p, isLoading: false, statusMessage: `Switched to ${tabName} tab.`}));
    }

	async function handleRequestOpenTab(event) {
        const { tabName, loadNotePath } = event.detail;
        const path = loadNotePath;
        const itemLogName = path ? path.split(/[\\/]/).pop() : 'no specific item';
        project.update(p => ({ ...p, isLoading: true, statusMessage: `Opening ${itemLogName} in ${tabName} tab...` }));

        if (!tabName) {
            project.update(p => ({ ...p, isLoading: false, statusMessage: 'Error: Tab name missing.' }));
            return;
        }

        // If already on the data tab and the requested path is the currently active one, do nothing.
        // This prevents the loading spinner from getting stuck when clicking the same file.
        const currentProjectState = get(project);
        if (selectedTab === 'data' && path) {
            if (path === currentProjectState.selectedDocumentPath ||
                path === currentProjectState.currentImportedTranscriptPath ||
                path === currentProjectState.selectedMediaNotePath) {
                project.update(p => ({ ...p, isLoading: false, statusMessage: `Already viewing ${itemLogName}.` }));
                return;
            }
        }

        let canProceed = true;
        let actionContext = path ? `loading item '${itemLogName}'` : "switching tabs";

        if (selectedTab === 'data') {
            canProceed = await checkUnsavedChangesThenProceed(path, actionContext);
        } else if (selectedTab === 'transcriptions') {
            if (transcriptionsViewRef) {
                try {
                    // Attempt to save and exit edit mode. handleToggleEditMode handles dirty check and confirmation.
                    await transcriptionsViewRef.handleToggleEditMode();
                    canProceed = true;
                } catch (e) {
                    // If handleToggleEditMode throws, it means the user cancelled the save/discard.
                    canProceed = false;
                }
            }
        }

        if (!canProceed) {
            project.update(p => ({ ...p, isLoading: false, statusMessage: 'Action cancelled.' }));
            return;
        }

        if (selectedTab !== tabName) {
            await handleTabClick(tabName);
            await tick();
        } else {
             project.update(p => ({...p, isDocumentLoading: false, isImportedTranscriptLoading: false, isMediaNoteTranscriptLoading: false}));
        }

        if (tabName === 'data' && path) {
            const proj = get(project);
            const isImportedTranscript = proj.importedTranscriptFiles.some(f => `${proj.baseDirectory}/${f.relativePath}` === path);
            const isMediaNote = proj.files.some(f => f.path === path && (f.file_type === 'media'));
            if (isImportedTranscript) prepareImportedTranscriptView(path);
            else if (isMediaNote) prepareMediaNoteView(path);
            else {
                const extension = path.split('.').pop()?.toLowerCase();
                let itemType = 'documents';
                if (['csv', 'xlsx'].includes(extension)) itemType = 'tables';
                else if (['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'tiff'].includes(extension)) itemType = 'images';
                prepareDocumentView(path, itemType);
            }
        } else if (tabName === 'data' && !path) {
            prepareDocumentView(null);
        }

        const projState = get(project);
        if (!projState.isDocumentLoading && !projState.isImportedTranscriptLoading && !projState.isMediaNoteTranscriptLoading && !projState.isTranscribing && !projState.isImportingAsset) {
            project.update(p => ({...p, isLoading: false, statusMessage: path ? p.statusMessage : `Switched to ${tabName} tab.`}));
        }
    }

    async function handleRequestMediaSelection(event) {
        console.log(`[ProjectView] handleRequestMediaSelection: Received event with mediaPath: '${event.detail?.mediaPath}'`);
        const { mediaPath } = event.detail;
        const mediaName = mediaPath ? mediaPath.split(/[\\/]/).pop() : "Unknown Media";

        if (!mediaPath) {
            project.update(p => ({...p, isLoading: false, statusMessage: 'Error: Media path missing.'}));
            console.error('[ProjectView] handleRequestMediaSelection: mediaPath is null or undefined.');
            return;
        }
        project.update(p => ({...p, isLoading: true, statusMessage: `Selecting media ${mediaName}...`}));

        let canProceed = true;
        if (selectedTab === 'data') {
            console.log(`[ProjectView] handleRequestMediaSelection: Calling checkUnsavedChangesThenProceed for path '${mediaPath}' from current tab '${selectedTab}'`);
            canProceed = await checkUnsavedChangesThenProceed(mediaPath, "selecting media for transcription tab");
            console.log(`[ProjectView] handleRequestMediaSelection: checkUnsavedChangesThenProceed returned: ${canProceed}`);
        }

        if (!canProceed) {
            project.update(p => ({ ...p, isLoading: false, statusMessage: 'Media selection cancelled.' }));
            console.log(`[ProjectView] handleRequestMediaSelection: 'checkUnsavedChangesThenProceed' returned false. Aborting media selection.`);
            return;
        }

        if (selectedTab !== 'transcriptions') {
            console.log(`[ProjectView] handleRequestMediaSelection: Current tab is '${selectedTab}', switching to 'transcriptions'.`);
            await handleTabClick('transcriptions');
            await tick();
        } else {
            // If already on transcriptions tab, check if different media is being selected
            if (get(transcriptStore).selectedMediaFile?.path !== mediaPath && get(transcriptStore).selectedMediaFile?.path) {
                console.log("[ProjectView] handleRequestMediaSelection: Different media selected on transcriptions tab. Calling handleToggleEditMode.");
                if (transcriptionsViewRef) {
                    try {
                        // Attempt to save and exit edit mode. handleToggleEditMode handles dirty check and confirmation.
                        await transcriptionsViewRef.handleToggleEditMode();
                    } catch (e) {
                        // If handleToggleEditMode throws, it means the user cancelled the save/discard.
                        project.update(p => ({ ...p, isLoading: false, statusMessage: 'Media selection cancelled.' }));
                        console.log(`[ProjectView] handleRequestMediaSelection: 'handleToggleEditMode' threw an error. Aborting media selection.`);
                        return;
                    }
                }
                 console.log(`[ProjectView] handleRequestMediaSelection: Already on transcriptions tab, but different media. Clearing old transcript state.`);
                clearTranscriptState(); // Clear state if switching media within the same tab
            } else if (get(transcriptStore).selectedMediaFile?.path === mediaPath) {
                console.log(`[ProjectView] handleRequestMediaSelection: Media path '${mediaPath}' is already selected in transcriptions tab.`);
            }
        }
        project.update(p => ({...p, isLoading: true, statusMessage: `Loading ${mediaName} in Transcriptions...`})); // This might be redundant if handleTabClick sets loading
        await tick();

        let fileEntry = null;
        function findMediaByPathRecursive(nodes, path) { // Keep this helper function local
            if (!Array.isArray(nodes)) return null;
            for (const node of nodes) {
                if (node.file_type === 'media' && !node.is_directory && node.path === path) return node;
                if (node.children?.length > 0) { const found = findMediaByPathRecursive(node.children, path); if (found) return found; }
            }
            return null;
        }
        console.log(`[ProjectView] handleRequestMediaSelection: Attempting to find FileEntry for mediaPath: '${mediaPath}' in project files:`, get(project).files);
        fileEntry = findMediaByPathRecursive(get(project).files || [], mediaPath);
        console.log(`[ProjectView] handleRequestMediaSelection: findMediaByPathRecursive result (fileEntry):`, fileEntry);

        if (fileEntry) {
            console.log(`[ProjectView] handleRequestMediaSelection: Calling selectMediaStoreAction with fileEntry:`, fileEntry);
            selectMediaStoreAction(fileEntry);
            console.log(`[ProjectView] handleRequestMediaSelection: selectMediaStoreAction called.`);
        } else {
            console.error(`[ProjectView] handleRequestMediaSelection: FileEntry not found for path: '${mediaPath}'. An error message should be shown to the user.`);
            await message(`Error: Could not find media file (${mediaName}).`, {title: "Error", type:"error"});
            project.update(p => ({...p, statusMessage: `Error selecting ${mediaName}.`}));
        }
        await tick(); // Ensure UI updates after selection or error
        project.update(p => ({...p, isLoading: false })); // Ensure loading is off
    }

    async function handleRequestTranscriptionTabWithMedia(event) {
        const { mediaPath } = event.detail;
        const mediaName = mediaPath.split(/[\\/]/).pop();
        project.update(p => ({ ...p, isLoading: true, statusMessage: `Switching to transcribe ${mediaName}...` }));

        await handleTabClick('transcriptions');
        await tick();
        await handleRequestMediaSelection({ detail: { mediaPath } });
        await tick();

        project.update(p => ({ ...p, isLoading: false, statusMessage: `Ready to transcribe ${mediaName}. Please select model and language.` }));
    }

    async function handleRequestTranscriptionTabWithMediaAndDialog(event) {
        const { mediaPath } = event.detail;
        const mediaName = mediaPath.split(/[\\/]/).pop();
        project.update(p => ({ ...p, isLoading: true, statusMessage: `Switching to transcribe ${mediaName} and opening dialog...` }));

        await handleTabClick('transcriptions');
        await tick();
        await handleRequestMediaSelection({ detail: { mediaPath } });
        await tick();

        // Now trigger the transcription dialog
        requestTranscriptionService();

        project.update(p => ({ ...p, isLoading: false, statusMessage: `Ready to transcribe ${mediaName}. Dialog opened.` }));
    }

    async function handleRequestTrimInTranscriptionTab(event) {
        const { mediaPath } = event.detail;
        const mediaName = mediaPath.split(/[\\/]/).pop();
        project.update(p => ({ ...p, isLoading: true, statusMessage: `Preparing to trim ${mediaName}...` }));

        if (selectedTab !== 'transcriptions') {
            await handleTabClick('transcriptions');
            await tick();
        }

        const currentSelectedMedia = get(project).selectedMediaFile?.path;
        if (currentSelectedMedia !== mediaPath) {
            await handleRequestMediaSelection({ detail: { mediaPath } });
            await tick();
            await tick();
        } else {
            project.update(p => ({ ...p, statusMessage: `Media ${mediaName} already selected.`}));
        }
        
        if (transcriptionsViewRef && typeof transcriptionsViewRef.activateTrimModeOnPlayer === 'function') {
            transcriptionsViewRef.activateTrimModeOnPlayer();
            project.update(p => ({ ...p, isLoading: false, statusMessage: `Trim mode activated for ${mediaName}.` }));
        } else {
            console.warn("[ProjectView] transcriptionsViewRef or activateTrimModeOnPlayer is not available.");
            project.update(p => ({ ...p, isLoading: false, statusMessage: `Could not activate trim mode for ${mediaName}.` }));
        }
    }


	function handleImportMediaInSidebar(event) {
        event.preventDefault(); event.stopPropagation();
        if (importMenuVisible) { closeImportMenu(); return; }
        importMenuX = event.clientX; importMenuY = event.clientY; importMenuVisible = true;
        setTimeout(() => {
            if (closeImportMenuListener) document.removeEventListener('click', closeImportMenuListener, { capture: true });
            closeImportMenuListener = (e) => { const menu = document.getElementById('import-context-menu-div'); if (menu && !menu.contains(e.target)) closeImportMenu(); };
            document.addEventListener('click', closeImportMenuListener, { capture: true, once: true });
        }, 0);
    }

    async function triggerMediaImport(actionType) {
        project.update(p => ({...p, isLoading: true, statusMessage: `Preparing import...`}));
        let canProceed = true;
        if (selectedTab === 'data') canProceed = await checkUnsavedChangesThenProceed(null, `importing ${actionType || 'asset'}`);
        else if (selectedTab === 'transcriptions') {
            if (get(project).transcriptDirty) { // This should probably be get(transcriptStore).transcriptDirty
                const confirmImport = await confirm( `Discard unsaved transcript changes to import new ${actionType || 'asset'}?`, { title: "Unsaved Transcript", type: "warning", okLabel: "Discard and Import", cancelLabel: "Cancel" });
                if (!confirmImport) canProceed = false;
                else { clearTranscriptState(); if (transcriptionsViewRef?.handleToggleEditMode) transcriptionsViewRef.handleToggleEditMode(false); }
            }
        }
        if (!canProceed) { project.update(p => ({...p, isLoading: false, statusMessage: 'Import cancelled.'})); return; }
        try {
            if (actionType === 'audio' || actionType === 'video') {
                const importedPath = await importMediaFile(actionType);
                if (importedPath) {
                    await handleTabClick('data');
                    prepareMediaNoteView(importedPath);
                }
            }
            else if (actionType === 'document') {
                const importedPath = await importDocumentFile();
                if (importedPath) {
                    await handleTabClick('data');
                    prepareDocumentView(importedPath, 'documents');
                }
            }
            else if (actionType === 'table') {
				const importResult = await importTableFile();
                if (importResult && importResult.table_path && String(importResult.table_path).trim() !== '') {
					headerConfirmationData = {
						tablePath: importResult.table_path,
						previewData: importResult.preview_data,
					};
					showHeaderConfirmationModal = true;
                } else {
                    console.error("[ProjectView] Table import failed or returned no path.", importResult);
                    await message("Table import failed. No valid table path was returned.", { title: "Import Error", type: "error" });
                    project.update(p => ({...p, isLoading: false, isImportingAsset: false, statusMessage: `Table import failed.`}));
                }
            }
            else if (actionType === 'image') {
                const importedPath = await importImageFile();
                if (importedPath) {
                    await handleTabClick('data');
                    prepareDocumentView(importedPath, 'images');
                }
            }
            else if (actionType === 'transcript') { 
                showImportTranscriptSourceModal = true; 
                project.update(p => ({...p, isLoading: false})); 
            }
            else { await message(`Import type (${actionType}) not recognized.`, {title: "Import Error", type: "error"}); project.update(p => ({...p, isLoading: false}));}
        } catch (e) { project.update(p => ({...p, isLoading: false, isImportingAsset: false, statusMessage: `Import failed.`}));}
        project.update(p => ({...p, isLoading: false}));
    }

    async function handleImportTranscriptSourceConfirm(event) {
        const { sourceType } = event.detail; 
        showImportTranscriptSourceModal = false;
        if (sourceType === 'msWord') { 
            try { 
                const newTranscriptPath = await importTranscriptFile('msWord');
                if (newTranscriptPath) {
                    await handleTabClick('data');
                    prepareImportedTranscriptView(newTranscriptPath);
                }
            } catch (e) { 
                project.update(p => ({...p, isImportingAsset: false, isLoading: false}));
            }
        }
        else await message(`Import from "${sourceType}" not supported.`, { title: 'Import Error', type: 'error' });
    }

    function closeImportMenu() { if (importMenuVisible) { importMenuVisible = false; if (closeImportMenuListener) document.removeEventListener('click', closeImportMenuListener, { capture: true }); closeImportMenuListener = null;}}
        function handleImportMenuAction(event, actionType) { 
        closeImportMenu(); 
        triggerMediaImport(actionType); 
    }

	async function handleHeaderConfirmation(event) {
		const { hasHeaders } = event.detail;
		const { tablePath } = headerConfirmationData;
		try {
			await invoke('set_table_headers', { tablePathStr: tablePath, hasHeaders });
			await refreshProjectFiles();
			await handleTabClick('data');
			prepareDocumentView(tablePath, 'tables', hasHeaders);
		} catch (error) {
			console.error(`[ProjectView] Error setting table headers:`, error);
			await message(`Error setting table headers: ${error.message || error}`, { title: 'Error', type: 'error' });
		}
	}

    $: showLoadingOverlay = ($project.isLoading && (get(transcriptStore)?.isTranscribing ?? false)) || $project.isImportingAsset || ($project.selectedDocumentPath && $project.isDocumentLoading) || ($project.currentImportedTranscriptPath && $project.isImportedTranscriptLoading) || ($project.selectedMediaNotePath && $project.isMediaNoteTranscriptLoading);

</script>

<div class="relative flex flex-col h-screen w-full font-sans text-sm text-gray-900 dark:text-gray-200 overflow-hidden">
	<!-- Top Bar Area -->
	<div class="flex-shrink-0">
		{#if selectedTab === 'data'}
			<DataTopBar
				tableViewRef={dataViewRef?.tableViewRef}
				on:requestTranscriptionTabWithMediaAndDialog={handleRequestTranscriptionTabWithMediaAndDialog}
                on:requestImport={handleImportMediaInSidebar}
			/>
		{:else if selectedTab === 'transcriptions'}
			<TranscriptionsTopBar
				bind:this={transcriptionsViewRef}
                on:requestImport={handleImportMediaInSidebar}
				on:cancelTranslationRequest={handleCancelTranslationRequest}
				on:runTranslationInBackground={() => setRanTranslationInBackground(true)}
			/>
		{:else if selectedTab === 'tags'}
			<SimpleTopBar on:requestImport={handleImportMediaInSidebar} />
		{/if}
	</div>

	<!-- Main Content Area -->
	<div class="flex flex-grow w-full overflow-hidden min-h-0">
		<div class="w-12 h-full bg-white bg-gray-200 dark:bg-dark-bg-primary shadow-lg flex flex-col flex-shrink-0 py-1 overflow-hidden border-r border-gray-300 dark:border-border">
			<div class="flex flex-col space-y-2 w-full">             <button title="Data" aria-label="Data" class="w-full h-10 flex items-center justify-center transition-colors focus:outline-none relative focus:outline-2 focus:outline-blue-500 dark:focus:outline-blue-400" class:border-l-4={selectedTab === 'data'} class:border-blue-500={selectedTab === 'data'} class:dark:border-blue-400={selectedTab === 'data'} class:bg-white={selectedTab === 'data'} class:dark:bg-dark-bg-primary={selectedTab === 'data'} class:text-blue-500={selectedTab === 'data'} class:dark:text-accent={selectedTab === 'data'} class:hover:bg-gray-300={selectedTab !== 'data'} class:dark:hover:bg-dark-bg-icon-bar={selectedTab !== 'data'} class:text-gray-700={selectedTab !== 'data'} class:dark:text-gray-300={selectedTab !== 'data'} class:dark:hover:text-gray-100={selectedTab !== 'data'} class:hover:text-gray-900={selectedTab !== 'data'} on:click={() => handleTabClick('data')}> <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="currentColor" class="size-6"> <path d="M5 0h8a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2 2 2 0 0 1-2 2H3a2 2 0 0 1-2-2h1a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V4a1 1 0 0 0-1-1H3a1 1 0 0 0-1 1H1a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v9a1 1 0 0 0 1-1V2a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1H3a2 2 0 0 1 2-2"/> <path d="M1 6v-.5a.5.5 0 0 1 1 0V6h.5a.5.5 0 0 1 0 1h-2a.5.5 0 0 1 0-1zm0 3v-.5a.5.5 0 0 1 1 0V9h.5a.5.5 0 0 1 0 1h-2a.5.5 0 0 1 0-1zm0 2.5v.5H.5a.5.5 0 0 0 0 1h2a.5.5 0 0 0 0-1H2v-.5a.5.5 0 0 0-1 0"/> </svg> </button> <button title="Transcriptions" aria-label="Transcriptions" class="w-full h-10 flex items-center justify-center transition-colors focus:outline-none relative focus:outline-2 focus:outline-blue-500 dark:focus:outline-blue-400" class:border-l-4={selectedTab === 'transcriptions'} class:border-blue-500={selectedTab === 'transcriptions'} class:dark:border-blue-400={selectedTab === 'transcriptions'} class:bg-white={selectedTab === 'transcriptions'} class:dark:bg-dark-bg-primary={selectedTab === 'transcriptions'} class:text-blue-500={selectedTab === 'transcriptions'} class:dark:text-accent={selectedTab === 'transcriptions'} class:hover:bg-gray-300={selectedTab !== 'transcriptions'} class:dark:hover:bg-dark-bg-icon-bar={selectedTab !== 'transcriptions'} class:text-gray-700={selectedTab !== 'transcriptions'} class:dark:text-gray-300={selectedTab !== 'transcriptions'} class:dark:hover:text-gray-100={selectedTab !== 'transcriptions'} class:hover:text-gray-900={selectedTab !== 'transcriptions'} on:click={() => handleTabClick('transcriptions')}> <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="currentColor" class="size-6"> <path d="M14 1a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1h-2.5a2 2 0 0 0-1.6.8L8 14.333 6.1 11.8a2 2 0 0 0-1.6-.8H2a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1zM2 0a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2.5a1 1 0 0 1 .8.4l1.9 2.533a1 1 0 0 0 1.6 0l1.9-2.533a1 1 0 0 1 .8-.4H14a2 2 0 0 0 2-2V2a2 2 0 0 0-2-2z"/> <path d="M3 3.5a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9a.5.5 0 0 1-.5-.5M3 6a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9A.5.5 0 0 1 3 6m0 2.5a.5.5 0 0 1 .5-.5h5a.5.5 0 0 1 0 1h-5a.5.5 0 0 1-.5-.5"/> </svg> </button> <button title="Tags" aria-label="Tags" class="w-full h-10 flex items-center justify-center transition-colors focus:outline-none relative focus:outline-2 focus:outline-blue-500 dark:focus:outline-blue-400" class:border-l-4={selectedTab === 'tags'} class:border-blue-500={selectedTab === 'tags'} class:dark:border-blue-400={selectedTab === 'tags'} class:bg-white={selectedTab === 'tags'} class:dark:bg-dark-bg-primary={selectedTab === 'tags'} class:text-blue-500={selectedTab === 'tags'} class:dark:text-accent={selectedTab === 'tags'} class:hover:bg-gray-300={selectedTab !== 'tags'} class:dark:hover:bg-dark-bg-icon-bar={selectedTab !== 'tags'} class:text-gray-700={selectedTab !== 'tags'} class:dark:text-gray-300={selectedTab !== 'tags'} class:dark:hover:text-gray-100={selectedTab !== 'tags'} class:hover:text-gray-900={selectedTab !== 'tags'} on:click={() => handleTabClick('tags')}> <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-tags size-6" viewBox="0 0 16 16"> <path d="M3 2v4.586l7 7L14.586 9l-7-7zM2 2a1 1 0 0 1 1-1h4.586a1 1 0 0 1 .707.293l7 7a1 1 0 0 1 0 1.414l-4.586 4.586a1 1 0 0 1-1.414 0l-7-7A1 1 0 0 1 2 6.586z"/> <path d="M5.5 5a.5.5 0 1 1 0-1 .5.5 0 0 1 0 1m0 1a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3M1 7.086a1 1 0 0 0 .293.707L8.75 15.25l-.043.043a1 1 0 0 1-1.414 0l-7-7A1 1 0 0 1 0 7.586V3a1 1 0 0 1 1-1z"/> </svg> </button> </div>
			<div class="mt-auto flex flex-col space-y-2 pb-2 w-full"> <button title="Help" aria-label="Help" class="w-full h-10 rounded-tl-md rounded-bl-md flex items-center justify-center text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-500 hover:text-gray-900 dark:hover:text-gray-100 transition-colors focus:outline-none focus:outline-2 focus:outline-blue-500 dark:focus:outline-blue-400"> <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="size-6"> <path fill-rule="evenodd" d="M2.25 12c0-5.385 4.365-9.75 9.75-9.75s9.75 4.365 9.75 9.75-4.365 9.75-9.75 9.75S2.25 17.385 2.25 12Zm8.706-1.442c1.146-.573 2.437.463 2.126 1.706l-.709 2.836.042-.02a.75.75 0 0 1 .67 1.34l-.04.022c-1.147.573-2.438-.463-2.127-1.706l.71-2.836-.042.02a.75.75 0 1 1-.671-1.34l.041-.022ZM12 9a.75.75 0 1 0 0-1.5.75.75 0 0 0 0 1.5Z" clip-rule="evenodd" /> </svg> </button> <button title="Configure" aria-label="Configure" on:click={() => showConfigurationModal = true} class="w-full h-10 rounded-tl-md rounded-bl-md flex items-center justify-center text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-500 hover:text-gray-900 dark:hover:text-gray-100 transition-colors focus:outline-none focus:outline-2 focus:outline-blue-500 dark:focus:outline-blue-400"> <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="size-6" class:text-red-500={hasConfigIssues}> <path d="M17.004 10.407c.138.435-.216.842-.672.842h-3.465a.75.75 0 0 1-.65-.375l-1.732-3c-.229-.396-.053-.907.393-1.004a5.252 5.252 0 0 1 6.126 3.537ZM8.12 8.464c.307-.338.838-.235 1.066.16l1.732 3a.75.75 0 0 1 0 .75l-1.732 3c-.229.397-.76.5-1.067.161A5.23 5.23 0 0 1 6.75 12a5.23 5.23 0 0 1 1.37-3.536ZM10.878 17.13c-.447-.098-.623-.608-.394-1.004l1.733-3.002a.75.75 0 0 1 .65-.375h3.465c.457 0 .81.407.672.842a5.252 5.252 0 0 1-6.126 3.539Z" /> <path fill-rule="evenodd" d="M21 12.75a.75.75 0 1 0 0-1.5h-.783a8.22 8.22 0 0 0-.237-1.357l.734-.267a.75.75 0 1 0-.513-1.41l-.735.268a8.24 8.24 0 0 0-.689-1.192l.6-.503a.75.75 0 1 0-.964-1.149l-.6.504a8.3 8.3 0 0 0-1.054-.885l.391-.678a.75.75 0 1 0-1.299-.75l-.39.676a8.188 8.188 0 0 0-1.295-.47l.136-.77a.75.75 0 0 0-1.477-.26l-.136.77a8.36 8.36 0 0 0-1.377 0l-.136-.77a.75.75 0 1 0-1.477.26l.136.77c-.448.121-.88.28-1.294.47l-.39-.676a.75.75 0 0 0-1.3.75l.392.678a8.29 8.29 0 0 0-1.054.885l-.6-.504a.75.75 0 1 0-.965 1.149l.6.503a8.243 8.243 0 0 0-.689 1.192L3.8 8.216a.75.75 0 1 0-.513 1.41l.735.267a8.222 8.222 0 0 0-.238 1.356h-.783a.75.75 0 0 0 0 1.5h.783c.042.464.122.917.238 1.356l-.735.268a.75.75 0 0 0 .513 1.41l.735-.268c.197.417.428.816.69 1.191l-.6.504a.75.75 0 0 0 .963 1.15l.601-.505c.326.323.679.62 1.054.885l-.392.68a.75.75 0 0 0 1.3.75l.39-.679c.414.192.847.35 1.294.471l-.136.77a.75.75 0 0 0 1.477.261l.137-.772a8.332 8.332 0 0 0 1.376 0l.136.772a.75.75 0 1 0 1.477-.26l-.136-.771a8.19 8.19 0 0 0 1.294-.47l.391.677a.75.75 0 0 0 1.3-.75l-.393-.679a8.29 8.29 0 0 0 1.054-.885l.601.504a.75.75 0 1 0-.965 1.149l.6.503a8.243 8.243 0 0 0-.689 1.192L18.2 15.784a.75.75 0 1 0 .513-1.41l.735-.267a8.222 8.222 0 0 0 .237-1.356h.784Zm-2.657-3.06a6.744 6.744 0 0 0-1.19-2.053 6.784 6.784 0 0 0-1.82-1.51A6.705 6.705 0 0 0 12 5.25a6.8 6.8 0 0 0-1.225.11 6.7 6.7 0 0 0-2.15.793 6.784 6.784 0 0 0-2.952 3.489.76.76 0 0 1-.036.098A6.74 6.74 0 0 0 5.251 12a6.74 6.74 0 0 0 3.366 5.842l.009.005a6.704 6.704 0 0 0 2.18.798l.022.003a6.792 6.792 0 0 0 2.368-.004 6.704 6.704 0 0 0 2.205-.811 6.785 6.785 0 0 0 1.762-1.484l.009-.01.009-.01a6.743 6.743 0 0 0 1.18-2.066c.253-.707.39-1.469.39-2.263a6.74 6.74 0 0 0-.408-2.309Z" clip-rule="evenodd" /> </svg> </button>
			</div>
		</div>

		<div class="flex flex-col flex-1 h-full bg-gray-100 dark:bg-dark-bg-primary overflow-hidden min-w-0">
			<div class="flex flex-col flex-grow min-h-0 overflow-hidden">
				{#if selectedTab === 'transcriptions'}
					<TranscriptionsView
						bind:this={transcriptionsViewRef}
						on:requestopentab={handleRequestOpenTab}
						on:requestmediaselection={handleRequestMediaSelection}
					/>
				{:else if selectedTab === 'data'}
					 <DataView
						bind:this={dataViewRef}
						on:requestmediaselection={handleRequestMediaSelection}
						on:requestTranscriptionTabWithMedia={handleRequestTranscriptionTabWithMedia}
						on:requestTrimInTranscriptionTab={handleRequestTrimInTranscriptionTab}
						on:requestTranscriptionTabWithMediaAndDialog={handleRequestTranscriptionTabWithMediaAndDialog}
					 />
				{:else if selectedTab === 'tags'}
					<TagsView bind:this={tagsViewRef} />
				{/if}
			</div>
		</div>
	</div>
    <BottomBar />

	<TranscribeConfirmModal
        bind:this={transcribeModalRef}
        bind:showModal={$transcriptStore.showTranscribeModal}
        fileName={$transcriptStore.selectedMediaFile?.name ?? 'N/A'}
        modelName={$transcriptStore.selectedModelName ?? 'None Selected'}
        language={$transcriptStore.selectedLanguage ?? 'N/A'}
        speakers={$transcriptStore.speakers}
        jobId={$transcriptStore.transcriptionJobId}
		downloadedModelsList={downloadedModelsList}
		
		languageOptions={languageOptions}
		initialDiarizationEnabled={$transcriptStore.diarizationEnabledForNextJob}
        on:confirmStart={onConfirmTranscriptionStart}
        on:cancelRequest={handleCancelTranscriptionRequest}
        on:closeAndReset={() => {
            transcriptStore.update(ts => ({ ...ts, showTranscribeModal: false, transcriptionJobStatus: null, transcriptionErrorMessage: null, transcriptionJobId: null, isTranscribing: false, transcriptionProgress: { percent: 0, message: '' } }));
            // Also clear any pending data related to a job that was just acknowledged as done/error/cancelled
            clearPendingTranscriptData();
             const ranInBackground = get(transcriptStore).ranInBackground;
             if (ranInBackground) { // If it ran in background, ensure this is reset for next time.
                 setRanInBackground(false);
             }
        }}
        on:runInBackgroundAndClose={() => {
            setRanInBackground(true);
            transcriptStore.update(ts => ({ ...ts, showTranscribeModal: false }));
        }} />

    

    <UnsavedChangesModal bind:showModal={$project.showUnsavedChangesModal} itemName={$project.unsavedItemName} itemType={$project.unsavedItemType} on:save={handleUnsavedResponse} on:discard={handleUnsavedResponse} on:cancel={handleUnsavedResponse} />
    <ConfirmConversionModal bind:showModal={$project.showConfirmConversionModal} fileName={$project.conversionFileName} on:confirm={handleConversionResponse} on:cancel={handleConversionResponse} />
    <ImportTranscriptSourceModal bind:showModal={showImportTranscriptSourceModal} on:confirm={handleImportTranscriptSourceConfirm} on:close={() => showImportTranscriptSourceModal = false}/>
	<HeaderConfirmationModal
		bind:showModal={showHeaderConfirmationModal}
		tablePath={headerConfirmationData.tablePath}
		previewData={headerConfirmationData.previewData}
		on:confirm={handleHeaderConfirmation}
	/>


    {#if importMenuVisible}
        <div id="import-context-menu-div" class="fixed z-50 bg-white dark:bg-surface-2 border border-gray-300 dark:border-border rounded-md shadow-xl py-1 text-xs min-w-[120px]" style="left: {importMenuX}px; top: {importMenuY}px;" on:click|stopPropagation role="menu" tabindex="0" on:keydown={(e) => { if (e.key === 'Escape') closeImportMenu(); }}>
            <button on:click={(event) => handleImportMenuAction(event, 'audio')} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-d-gray-700 text-gray-800 dark:text-d-gray-200">Audio</button>
            <button on:click={(event) => handleImportMenuAction(event, 'document')} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-d-gray-700 text-gray-800 dark:text-d-gray-200">Document</button>
            <button on:click={(event) => handleImportMenuAction(event, 'image')} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-d-gray-700 text-gray-800 dark:text-d-gray-200">Image</button>
            <button on:click={(event) => handleImportMenuAction(event, 'table')} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-d-gray-700 text-gray-800 dark:text-d-gray-200">Table</button>
            <button on:click={(event) => handleImportMenuAction(event, 'transcript')} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-d-gray-700 text-gray-800 dark:text-d-gray-200">Transcript</button>
            <button on:click={(event) => handleImportMenuAction(event, 'video')} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-d-gray-700 text-gray-800 dark:text-d-gray-200">Video</button>
        </div>
    {/if}

    {#if showLoadingOverlay}
        <div class="absolute inset-0 z-[110] flex items-center justify-center bg-black/30 backdrop-blur-sm">
            <div class="flex flex-col items-center p-6 bg-white dark:bg-surface-2 rounded-lg shadow-xl">
                 <Loader class="w-12 h-12 text-blue-500 animate-spin mb-3" />
                 <p class="text-sm text-gray-700 dark:text-d-gray-300">{$project.statusMessage || 'Loading...'}</p>
            </div>
        </div>
    {/if}
</div>

<style lang="postcss">
	::-webkit-scrollbar { @apply w-2 h-2; }
	::-webkit-scrollbar-track { @apply bg-gray-100 rounded-lg; }
	::-webkit-scrollbar-thumb { @apply bg-gray-300 rounded-lg border-2 border-solid border-gray-100; }
	::-webkit-scrollbar-thumb:hover { @apply bg-gray-400; }
	* { scrollbar-width: thin; scrollbar-color: #d1d5db #f3f4f6; }
	.dark ::-webkit-scrollbar-track { @apply bg-gray-800; }
	.dark ::-webkit-scrollbar-thumb { @apply bg-gray-500 border-gray-800; }
	.dark ::-webkit-scrollbar-thumb:hover { @apply bg-gray-400; }
	.dark * { scrollbar-color: #6b7280 #1f2937; }
    .size-6 { width: 1.5rem; height: 1.5rem; }
    .min-h-0 { min-height: 0; }
</style>
<ConfigurationModal bind:showModal={showConfigurationModal} on:close={() => showConfigurationModal = false} />
