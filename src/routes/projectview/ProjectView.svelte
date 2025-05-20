<!-- src/routes/projectview/ProjectView.svelte -->
<script>
	import { onMount, onDestroy, tick } from 'svelte';
	import { page } from '$app/stores';
	import { get } from 'svelte/store';
	import {
		loadProjectDataAndUpdateStore,
		handleConfirmStartTranscription,
		handleCancelTranscriptionRequest,
		registerTranscribeModal,
		initializeProgressListener,
		cleanupProgressListener,
		importMediaFile,
		checkUnsavedChangesThenProceed,
        importDocumentFile,
        importTableFile,
        importImageFile,
        importTranscriptFile 
	} from '$lib/services/projectService.js';
	import {
        project,
        toggleTranscribeModal,
        selectMedia as selectMediaStoreAction,
        hideUnsavedChangesPrompt,
        hideConversionPrompt,
        clearTranscriptState,
        prepareDocumentView,
        prepareImportedTranscriptView, 
        clearDocumentEditorState,
    } from '$lib/stores/projectStore.js';
    import { message, confirm } from '@tauri-apps/plugin-dialog';
    import { getCurrentWindow } from '@tauri-apps/api/window';


	import BottomBar from '$lib/components/projectview/shared/BottomBar.svelte';
	import TranscribeConfirmModal from '$lib/components/projectview/modals/TranscribeConfirmModal.svelte';
    import UnsavedChangesModal from '$lib/components/projectview/modals/UnsavedChangesModal.svelte';
    import ConfirmConversionModal from '$lib/components/projectview/modals/ConfirmConversionModal.svelte';
    import ImportTranscriptSourceModal from '$lib/components/projectview/modals/ImportTranscriptSourceModal.svelte';
	import NotesView from '$lib/components/projectview/notes/NotesView.svelte';
    import TranscriptionsView from '$lib/components/projectview/transcriptions/TranscriptionsView.svelte';
    import { Loader } from 'lucide-svelte';


	let transcribeModalRef;
    let transcriptionsViewRef; 
    let mediaPlayerRef = null;

	let selectedTab = 'notes'; 
    let importMenuVisible = false;
    let importMenuX = 0;
    let importMenuY = 0;
    let closeImportMenuListener = null;
    let appWindow = null;
    let removeCloseRequestListener = null;
    let handlingCloseRequest = false;
    let showImportTranscriptSourceModal = false; 


	onMount(async () => {
        appWindow = getCurrentWindow();

		const xmlPath = $page.url.searchParams.get('xmlPath');
		if (xmlPath && xmlPath.trim() !== '') {
			try { await loadProjectDataAndUpdateStore(xmlPath); }
            catch (e) { console.error('[ProjectView] Error during initial project load:', e); }
		} else {
			project.update((p) => ({ ...p, isLoading: false, error: 'Project path is missing.', statusMessage: 'Error: Project path is missing.' }));
			console.error('[ProjectView] Mount error: Project XML path missing in URL parameters.');
		}
		initializeProgressListener();
		await tick();
		if (transcribeModalRef) { registerTranscribeModal(transcribeModalRef); }
        else { console.warn('[ProjectView] TranscribeConfirmModal reference not available on mount.'); }
		window.addEventListener('keydown', handleGlobalKeys);

        if (appWindow) {
            removeCloseRequestListener = await appWindow.listen('tauri://close-requested', handleWindowCloseRequest);
            console.log('[ProjectView] Added tauri://close-requested listener.');
        } else {
            console.error('[ProjectView] Could not get appWindow reference to attach close listener.');
        }

		console.log('[ProjectView] onMount finished.');
	});

	onDestroy(() => {
		cleanupProgressListener();
		window.removeEventListener('keydown', handleGlobalKeys);
        if (closeImportMenuListener) { document.removeEventListener('click', closeImportMenuListener, { capture: true }); closeImportMenuListener = null; }
        if (removeCloseRequestListener) {
            removeCloseRequestListener();
            console.log('[ProjectView] Removed tauri://close-requested listener.');
            removeCloseRequestListener = null;
        }
		console.log('[ProjectView] onDestroy: Cleaned up listeners.');
	});

	function handleGlobalKeys(event) { 
        const proj = get(project); 
        const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0; 
        const modKey = isMac ? event.metaKey : event.ctrlKey; 
        if (modKey && event.key.toLowerCase() === 's') { 
            event.preventDefault(); 
            if (selectedTab === 'transcriptions' && transcriptionsViewRef) { 
                transcriptionsViewRef.handleSaveTranscript(); 
            } else if (selectedTab === 'notes') { 
                const activeDocEditor = proj.activeDocumentEditorRef;
                const activeImpTsEditor = proj.activeImportedTranscriptEditorRef;

                if (!proj.autosaveEnabled) {
                    if (proj.isDocumentDirty && activeDocEditor && typeof activeDocEditor.save === 'function') {
                        console.log('[ProjectView Ctrl+S] Triggering save for Document Editor...');
                        activeDocEditor.save().catch(e => console.error("Ctrl+S document save failed", e));
                    } else if (proj.isImportedTranscriptDirty && activeImpTsEditor && typeof activeImpTsEditor.save === 'function') {
                        console.log('[ProjectView Ctrl+S] Triggering save for Imported Transcript Editor...');
                        activeImpTsEditor.save().catch(e => console.error("Ctrl+S imported transcript save failed", e));
                    }
                }
            } return; 
        } 
        if (modKey && event.key.toLowerCase() === 'e') { if (selectedTab === 'transcriptions' && transcriptionsViewRef) { event.preventDefault(); transcriptionsViewRef.handleToggleEditMode(); } return; } 
        if (modKey && event.key.toLowerCase() === 'z' && !event.shiftKey) { if (selectedTab === 'transcriptions' && transcriptionsViewRef && proj.transcriptUndoStack?.length > 0) { event.preventDefault(); transcriptionsViewRef.handleUndoRequest(); } return; } 
        if (modKey && (event.key.toLowerCase() === 'y' || (event.shiftKey && event.key.toLowerCase() === 'z'))) { if (selectedTab === 'transcriptions' && transcriptionsViewRef && proj.transcriptRedoStack?.length > 0) { event.preventDefault(); transcriptionsViewRef.handleRedoRequest(); } return; } 
        if (event.key === 'F8') { if (selectedTab === 'transcriptions' && mediaPlayerRef) { event.preventDefault(); mediaPlayerRef.handleTogglePlay(); } return; } 
    }

	function handleModalClose() { toggleTranscribeModal(false); }
    function handleUnsavedResponse(event) { const action = event.type; const callback = get(project)[`onUnsaved${action.charAt(0).toUpperCase() + action.slice(1)}`]; if (typeof callback === 'function') { callback(); } else { console.warn(`[ProjectView] No valid callback found for unsaved action: ${action}`); hideUnsavedChangesPrompt(); } }
    function handleConversionResponse(event) { const action = event.type; const callback = get(project)[`onConversion${action.charAt(0).toUpperCase() + action.slice(1)}`]; if (typeof callback === 'function') { callback(); } else { console.warn(`[ProjectView] No valid callback found for conversion action: ${action}`); hideConversionPrompt(); } }

    async function handleWindowCloseRequest() {
        if (handlingCloseRequest) {
            console.log('[ProjectView handleWindowCloseRequest] Already handling close request, ignoring re-entry.');
            return;
        }
        handlingCloseRequest = true;
        console.log(`[ProjectView handleWindowCloseRequest] Listener invoked. Current tab: ${selectedTab}.`);

        let canProceed = false;

        try {
            if (selectedTab === 'notes') {
                console.log("[ProjectView] Checking unsaved Fieldnotes item before close...");
                canProceed = await checkUnsavedChangesThenProceed(null, "closing the project window"); 
            } else if (selectedTab === 'transcriptions') {
                console.log("[ProjectView] Checking unsaved media transcript before close...");
                const isDirty = get(project).transcriptDirty;
                if (isDirty) {
                    const confirmClose = await confirm(
                        "You have unsaved media transcript changes. Discard them and close the project window?",
                        { title: "Unsaved Media Transcript", type: "warning", okLabel: "Discard and Close", cancelLabel: "Cancel" }
                    );
                    if (confirmClose) {
                        console.log("[ProjectView] User chose to discard media transcript changes.");
                        clearTranscriptState(); 
                        canProceed = true;
                    } else {
                        console.log("[ProjectView] User cancelled closing due to unsaved media transcript.");
                        canProceed = false;
                    }
                } else {
                    console.log("[ProjectView] Media transcript is not dirty.");
                    canProceed = true;
                }
            } else {
                console.warn(`[ProjectView] Unknown tab '${selectedTab}' during close request. Assuming safe to proceed.`);
                canProceed = true;
            }
        } catch (error) {
             console.error("[ProjectView handleWindowCloseRequest] Error during checks:", error);
             canProceed = false; 
        }

        if (canProceed) {
            console.log("[ProjectView] All checks passed. Attempting to close project window.");
            if (removeCloseRequestListener) {
                removeCloseRequestListener(); 
                removeCloseRequestListener = null; 
                console.log('[ProjectView] Removed close listener before closing window.');
            }
            if (appWindow) {
                try {
                    await appWindow.close(); 
                    console.log("[ProjectView] Window close command issued.");
                } catch (error) {
                    console.error("[ProjectView] Error attempting to close window:", error);
                     await message(`Error closing project window: ${error}`, {title: "Error", type: "error"});
                }
            } else {
                console.error("[ProjectView] Cannot close window: appWindow reference is null.");
                 await message("Internal error: Could not get window reference to close.", {title: "Error", type: "error"});
            }
        } else {
            console.log("[ProjectView] Window close aborted by checks or user cancellation.");
        }

        handlingCloseRequest = false; 
    }


	async function handleTabClick(tabName) {
        if (selectedTab === tabName) {
            console.log(`[ProjectView] handleTabClick called for already selected tab: ${tabName}. No switch.`);
            return;
        }
        console.log(`[ProjectView] Requesting tab switch from ${selectedTab} to ${tabName}`);

        let canProceed = true;

        if (selectedTab === 'notes') {
            canProceed = await checkUnsavedChangesThenProceed(null, "switching tabs");
        } else if (selectedTab === 'transcriptions') {
            const isDirty = get(project).transcriptDirty; 
            if (isDirty && transcriptionsViewRef) {
                const confirmSwitch = await confirm(
                    "You have unsaved media transcript changes. Discard them and switch tabs?",
                    { title: "Unsaved Media Transcript", type: "warning", okLabel: "Discard and Switch", cancelLabel: "Cancel Switch" }
                );
                if (!confirmSwitch) {
                    canProceed = false;
                } else {
                    clearTranscriptState(); 
                    if (transcriptionsViewRef.handleToggleEditMode) transcriptionsViewRef.handleToggleEditMode(false); 
                }
            }
        }

        if (!canProceed) {
            console.log('[ProjectView] Tab switch cancelled by unsaved changes check or user.');
            return;
        }

        const playerInstanceToPause = mediaPlayerRef; 
        selectedTab = tabName;
        console.log(`[ProjectView] Switched selectedTab state to: ${selectedTab}`);

        // Clear state of the *other* tab when switching
        if (selectedTab === 'notes') {
            clearTranscriptState(); 
        } else if (selectedTab === 'transcriptions') {
            prepareDocumentView(null); 
            prepareImportedTranscriptView(null); 
        }


        if (selectedTab !== 'transcriptions') { 
            if (playerInstanceToPause && typeof playerInstanceToPause.pause === 'function') {
                const videoElement = playerInstanceToPause.videoElement; 
                if (videoElement && !videoElement.paused) {
                    try {
                        await videoElement.pause();
                        console.log("[ProjectView] Paused media player on tab switch away from transcriptions.");
                    } catch(e) {
                        console.warn("[ProjectView] Error pausing video element:", e);
                        try {
                            await playerInstanceToPause.pause();
                        } catch (e2) {
                             console.warn("[ProjectView] Error pausing via component method:", e2);
                        }
                    }
                }
            }
        }
        await tick(); 
    }

    // --- MODIFIED ---
	async function handleRequestOpenTab(event) { 
        // Extract path using the key used in RichTextPreview dispatch
        const { tabName, loadNotePath } = event.detail;
        const path = loadNotePath; // Use the correct key

        console.log(`[ProjectView] Received requestopentab event: tab=${tabName}, path=${path}`);

        if (!tabName) {
            console.warn('[ProjectView] requestopentab event missing tabName.');
            return;
        }

        let canProceed = true;
        let actionContext = path ? `loading item ${path?.split(/[\\/]/).pop()}` : "switching tabs";

        if (selectedTab === 'notes') {
            canProceed = await checkUnsavedChangesThenProceed(path, actionContext);
        } else if (selectedTab === 'transcriptions') {
            const isDirty = get(project).transcriptDirty;
            if (isDirty && transcriptionsViewRef) {
                const confirmSwitch = await confirm(
                    `Discard unsaved media transcript changes to ${path ? 'load the requested item' : 'switch tabs'}?`,
                    { title: "Unsaved Media Transcript", type: "warning", okLabel: "Discard and Proceed", cancelLabel: "Cancel"});
                if (!confirmSwitch) {
                    canProceed = false;
                } else {
                    clearTranscriptState(); 
                    if (transcriptionsViewRef.handleToggleEditMode) transcriptionsViewRef.handleToggleEditMode(false);
                }
            }
        }

        if (!canProceed) {
            console.log(`[ProjectView] requestopentab/tab switch cancelled by unsaved changes check or user.`);
            return;
        }

        // If switching to 'notes' tab (regardless of whether a path is provided yet)
        if (tabName === 'notes') {
            if (selectedTab !== 'notes') {
                 await handleTabClick(tabName); // Switch to Notes tab first if not already there
                 await tick(); // Wait for tab switch UI changes
            }
            // *After* potentially switching tabs, prepare the view if a path exists
            if (path) {
                const proj = get(project);
                const isImportedTranscript = proj.importedTranscriptFiles.some(f => `${proj.baseDirectory}/${f.relativePath}` === path);

                if (isImportedTranscript) {
                    prepareImportedTranscriptView(path);
                } else {
                    const extension = path.split('.').pop()?.toLowerCase();
                    let itemType = 'documents'; 
                    if (['csv', 'xlsx'].includes(extension)) itemType = 'tables';
                    else if (['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'tiff'].includes(extension)) itemType = 'images';
                    prepareDocumentView(path, itemType);
                }
            } else {
                 // Switched to notes tab without a specific item, clear selections
                 prepareDocumentView(null);
                 prepareImportedTranscriptView(null);
            }
        } else { // Switching to a non-notes tab (e.g. Transcriptions)
             project.update(p => ({ ...p, requestedNoteToLoad: null })); 
             await handleTabClick(tabName); 
        }
    }
    // --- END MODIFIED ---

    async function handleRequestMediaSelection(event) { 
        const { mediaPath } = event.detail;
        console.log(`[ProjectView] Received requestmediaselection for path: ${mediaPath}`);
        if (!mediaPath) {
            console.warn("[ProjectView] requestmediaselection missing mediaPath.");
            return;
        }

        let canProceed = true;
        if (selectedTab === 'notes') {
            canProceed = await checkUnsavedChangesThenProceed(mediaPath, "selecting media");
        }
        
        if (!canProceed) {
            console.log('[ProjectView] Media selection cancelled by unsaved changes check.');
            return;
        }

        if (selectedTab !== 'transcriptions') {
            console.log(`[ProjectView] Switching tab from ${selectedTab} to transcriptions...`);
            await handleTabClick('transcriptions'); 
            console.log(`[ProjectView] Tab switched, current selectedTab state: ${selectedTab}`);
             await tick(); // Ensure tab is rendered before selecting media
        } else {
            const currentMediaPath = get(project).selectedMediaFile?.path;
            if (currentMediaPath !== mediaPath) {
                console.log('[ProjectView] Media path changed while on transcriptions tab, clearing old transcript state.');
                clearTranscriptState(); 
            }
        }

        let fileEntry = null;
        function findMediaByPathRecursive(nodes, path) {
            if (!Array.isArray(nodes)) return null;
            for (const node of nodes) {
                if (node.file_type === 'media' && !node.is_directory && node.path === path) {
                    return node; 
                }
                if (node.children && node.children.length > 0) {
                    const found = findMediaByPathRecursive(node.children, path);
                    if (found) return found; 
                }
            }
            return null; 
        }

        const currentFiles = get(project).files || [];
        fileEntry = findMediaByPathRecursive(currentFiles, mediaPath);

        if (fileEntry) {
            console.log(`[ProjectView] Found media entry for path ${mediaPath}. Selecting with store action...`);
            selectMediaStoreAction(fileEntry); 
        } else {
            console.error(`[ProjectView] Could not find media file entry for path ${mediaPath} in the project store.`);
            await message(`Error: Could not find the requested media file (${mediaPath.split(/[\\/]/).pop()}) in the project data.`, {title: "Error", type:"error"});
        }
    }

	function handleImportMediaInSidebar(event) {
        console.log('[ProjectView] Sidebar Import icon clicked');
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
            if (closeImportMenuListener) {
                 document.removeEventListener('click', closeImportMenuListener, { capture: true });
            }
            closeImportMenuListener = (e) => {
                const menuElement = document.getElementById('import-context-menu-div');
                if (menuElement && !menuElement.contains(e.target)) {
                    closeImportMenu();
                }
            };
            document.addEventListener('click', closeImportMenuListener, { capture: true, once: true });
        }, 0);
    }

    async function triggerMediaImport(importType = null) {
        console.log(`[ProjectView] Triggering import process (Type: ${importType || 'generic'})...`);
        let canProceed = true;

        if (selectedTab === 'notes') {
            canProceed = await checkUnsavedChangesThenProceed(null, `importing ${importType || 'asset'}`);
        } else if (selectedTab === 'transcriptions') {
            const isDirty = get(project).transcriptDirty;
            if (isDirty) {
                const confirmImport = await confirm(
                    `Discard unsaved media transcript changes to import new ${importType || 'asset'}?`,
                    { title: "Unsaved Media Transcript", type: "warning", okLabel: "Discard and Import", cancelLabel: "Cancel Import" }
                );
                if (!confirmImport) {
                    canProceed = false;
                } else {
                    clearTranscriptState(); 
                    if (transcriptionsViewRef && transcriptionsViewRef.handleToggleEditMode) transcriptionsViewRef.handleToggleEditMode(false); 
                }
            }
        }

        if (!canProceed) {
            console.log(`[ProjectView] Import cancelled due to unsaved changes check.`);
            return;
        }
        
        try {
            if (importType === 'audio' || importType === 'video') {
                await importMediaFile(importType);
            } else if (importType === 'document') {
                await importDocumentFile();
            } else if (importType === 'table') {
                await importTableFile();
            } else if (importType === 'image') { 
                await importImageFile();
            } else if (importType === 'transcript') { 
                showImportTranscriptSourceModal = true; 
            } else {
                console.warn(`[ProjectView] Unknown import type requested: ${importType}`);
                await message(`The requested import type (${importType}) is not recognized.`, {title: "Import Error", type: "error"});
            }
        } catch (e) {
            console.error('[ProjectView] Error during import service call:', e);
        }
    }

    async function handleImportTranscriptSourceConfirm(event) {
        const { sourceType } = event.detail;
        showImportTranscriptSourceModal = false; 

        if (sourceType === 'msWord') {
            try {
                await importTranscriptFile('msWord'); 
            } catch (e) {
                console.error('[ProjectView] Error calling importTranscriptFile (msWord):', e);
            }
        } else {
            console.warn(`[ProjectView] Unknown transcript import source type from modal: ${sourceType}`);
            await message(`Import from "${sourceType}" is not supported yet.`, { title: 'Import Error', type: 'error' });
        }
    }


    function closeImportMenu() {
        if (importMenuVisible) {
            importMenuVisible = false;
            if (closeImportMenuListener) {
                 document.removeEventListener('click', closeImportMenuListener, { capture: true });
                 closeImportMenuListener = null;
            }
        }
    }

    function handleImportMenuAction(actionType) {
        console.log(`[ProjectView] Import menu action selected: ${actionType}`);
        closeImportMenu(); 
        triggerMediaImport(actionType); 
    }

	$: modalProps = { fileName: $project.selectedMediaFile?.name ?? 'N/A', modelName: $project.selectedModelName ?? 'None Selected', language: $project.selectedLanguage ?? 'N/A', speakers: $project.speakers, jobId: $project.transcriptionJobId };
    $: showLoadingOverlay = $project.isImportingAsset || $project.isLoading || $project.isDocumentLoading || $project.isImportedTranscriptLoading;

</script>

<div class="relative flex h-screen w-full font-sans text-sm text-gray-900 dark:text-gray-200 overflow-hidden">

	<div class="w-12 h-full bg-white bg-gray-200 dark:bg-gray-600 shadow-lg flex flex-col flex-shrink-0 items-center py-2 overflow-hidden">
		<button title="Import" class="group w-8 h-8 mt-1 mb-4 rounded-full flex items-center justify-center transition-colors bg-white focus:outline-none border-2 border-black dark:border-blue-400 hover:border-blue-500 dark:hover:border-blue-300 focus:ring-2 focus:ring-blue-500 dark:focus:ring-blue-400 focus:ring-offset-2 dark:focus:ring-offset-gray-600" on:click={handleImportMediaInSidebar}> <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-6 text-black dark:text-blue-400 group-hover:text-blue-500 dark:group-hover:text-blue-300 transition-colors"> <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" /> </svg> </button>
		<div class="flex flex-col items-center space-y-2"> <button title="Fieldnotes" class="w-10 h-10 rounded-full flex items-center justify-center transition-colors focus:outline-none relative focus:ring-2 focus:ring-offset-2 dark:focus:ring-offset-gray-600 focus:ring-blue-300 dark:focus:ring-blue-800" class:ring-2={selectedTab === 'notes'} class:ring-blue-500={selectedTab === 'notes'} class:dark:ring-blue-400={selectedTab === 'notes'} class:bg-white={selectedTab === 'notes'} class:dark:bg-gray-500={selectedTab === 'notes'} class:text-blue-500={selectedTab === 'notes'} class:dark:text-gray-100={selectedTab === 'notes'} class:hover:bg-gray-300={selectedTab !== 'notes'} class:dark:hover:bg-gray-500={selectedTab !== 'notes'} class:text-gray-700={selectedTab !== 'notes'} class:dark:text-gray-300={selectedTab !== 'notes'} class:dark:hover:text-gray-100={selectedTab !== 'notes'} class:hover:text-gray-900={selectedTab !== 'notes'} on:click={() => handleTabClick('notes')}> <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="currentColor" class="size-6"> <path d="M5 0h8a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2 2 2 0 0 1-2 2H3a2 2 0 0 1-2-2h1a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V4a1 1 0 0 0-1-1H3a1 1 0 0 0-1 1H1a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v9a1 1 0 0 0 1-1V2a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1H3a2 2 0 0 1 2-2"/> <path d="M1 6v-.5a.5.5 0 0 1 1 0V6h.5a.5.5 0 0 1 0 1h-2a.5.5 0 0 1 0-1zm0 3v-.5a.5.5 0 0 1 1 0V9h.5a.5.5 0 0 1 0 1h-2a.5.5 0 0 1 0-1zm0 2.5v.5H.5a.5.5 0 0 0 0 1h2a.5.5 0 0 0 0-1H2v-.5a.5.5 0 0 0-1 0"/> </svg> </button> <button title="Transcriptions" class="w-10 h-10 rounded-full flex items-center justify-center transition-colors focus:outline-none relative focus:ring-2 focus:ring-offset-2 dark:focus:ring-offset-gray-600 focus:ring-blue-300 dark:focus:ring-blue-800" class:ring-2={selectedTab === 'transcriptions'} class:ring-blue-500={selectedTab === 'transcriptions'} class:dark:ring-blue-400={selectedTab === 'transcriptions'} class:bg-white={selectedTab === 'transcriptions'} class:dark:bg-gray-500={selectedTab === 'transcriptions'} class:text-blue-500={selectedTab === 'transcriptions'} class:dark:text-gray-100={selectedTab === 'transcriptions'} class:hover:bg-gray-300={selectedTab !== 'transcriptions'} class:dark:hover:bg-gray-500={selectedTab !== 'transcriptions'} class:text-gray-700={selectedTab !== 'transcriptions'} class:dark:text-gray-300={selectedTab !== 'transcriptions'} class:dark:hover:text-gray-100={selectedTab !== 'transcriptions'} class:hover:text-gray-900={selectedTab !== 'transcriptions'} on:click={() => handleTabClick('transcriptions')}> <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="currentColor" class="size-6"> <path d="M14 1a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1h-2.5a2 2 0 0 0-1.6.8L8 14.333 6.1 11.8a2 2 0 0 0-1.6-.8H2a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1zM2 0a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2.5a1 1 0 0 1 .8.4l1.9 2.533a1 1 0 0 0 1.6 0l1.9-2.533a1 1 0 0 1 .8-.4H14a2 2 0 0 0 2-2V2a2 2 0 0 0-2-2z"/> <path d="M3 3.5a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9a.5.5 0 0 1-.5-.5M3 6a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9A.5.5 0 0 1 3 6m0 2.5a.5.5 0 0 1 .5-.5h5a.5.5 0 0 1 0 1h-5a.5.5 0 0 1-.5-.5"/> </svg> </button> </div>
		<div class="mt-auto flex flex-col items-center space-y-2 pb-2"> <button title="Help" class="w-10 h-10 rounded-full flex items-center justify-center text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-500 hover:text-gray-900 dark:hover:text-gray-100 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-blue-400 focus:ring-offset-2 dark:focus:ring-offset-gray-600"> <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="size-6"> <path fill-rule="evenodd" d="M2.25 12c0-5.385 4.365-9.75 9.75-9.75s9.75 4.365 9.75 9.75-4.365 9.75-9.75 9.75S2.25 17.385 2.25 12Zm8.706-1.442c1.146-.573 2.437.463 2.126 1.706l-.709 2.836.042-.02a.75.75 0 0 1 .67 1.34l-.04.022c-1.147.573-2.438-.463-2.127-1.706l.71-2.836-.042.02a.75.75 0 1 1-.671-1.34l.041-.022ZM12 9a.75.75 0 1 0 0-1.5.75.75 0 0 0 0 1.5Z" clip-rule="evenodd" /> </svg> </button> <button title="Settings" class="w-10 h-10 rounded-full flex items-center justify-center text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-500 hover:text-gray-900 dark:hover:text-gray-100 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-blue-400 focus:ring-offset-2 dark:focus:ring-offset-gray-600"> <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="size-6"> <path d="M17.004 10.407c.138.435-.216.842-.672.842h-3.465a.75.75 0 0 1-.65-.375l-1.732-3c-.229-.396-.053-.907.393-1.004a5.252 5.252 0 0 1 6.126 3.537ZM8.12 8.464c.307-.338.838-.235 1.066.16l1.732 3a.75.75 0 0 1 0 .75l-1.732 3c-.229.397-.76.5-1.067.161A5.23 5.23 0 0 1 6.75 12a5.23 5.23 0 0 1 1.37-3.536ZM10.878 17.13c-.447-.098-.623-.608-.394-1.004l1.733-3.002a.75.75 0 0 1 .65-.375h3.465c.457 0 .81.407.672.842a5.252 5.252 0 0 1-6.126 3.539Z" /> <path fill-rule="evenodd" d="M21 12.75a.75.75 0 1 0 0-1.5h-.783a8.22 8.22 0 0 0-.237-1.357l.734-.267a.75.75 0 1 0-.513-1.41l-.735.268a8.24 8.24 0 0 0-.689-1.192l.6-.503a.75.75 0 1 0-.964-1.149l-.6.504a8.3 8.3 0 0 0-1.054-.885l.391-.678a.75.75 0 1 0-1.299-.75l-.39.676a8.188 8.188 0 0 0-1.295-.47l.136-.77a.75.75 0 0 0-1.477-.26l-.136.77a8.36 8.36 0 0 0-1.377 0l-.136-.77a.75.75 0 1 0-1.477.26l.136.77c-.448.121-.88.28-1.294.47l-.39-.676a.75.75 0 0 0-1.3.75l.392.678a8.29 8.29 0 0 0-1.054.885l-.6-.504a.75.75 0 1 0-.965 1.149l.6.503a8.243 8.243 0 0 0-.689 1.192L3.8 8.216a.75.75 0 1 0-.513 1.41l.735.267a8.222 8.222 0 0 0-.238 1.356h-.783a.75.75 0 0 0 0 1.5h.783c.042.464.122.917.238 1.356l-.735.268a.75.75 0 0 0 .513 1.41l.735-.268c.197.417.428.816.69 1.191l-.6.504a.75.75 0 0 0 .963 1.15l.601-.505c.326.323.679.62 1.054.885l-.392.68a.75.75 0 0 0 1.3.75l.39-.679c.414.192.847.35 1.294.471l-.136.77a.75.75 0 0 0 1.477.261l.137-.772a8.332 8.332 0 0 0 1.376 0l.136.772a.75.75 0 1 0 1.477-.26l-.136-.771a8.19 8.19 0 0 0 1.294-.47l.391.677a.75.75 0 0 0 1.3-.75l-.393-.679a8.29 8.29 0 0 0 1.054-.885l.601.504a.75.75 0 1 0-.965 1.149l.6.503a8.243 8.243 0 0 0-.689 1.192L18.2 15.784a.75.75 0 1 0 .513-1.41l.735-.267a8.222 8.222 0 0 0 .237-1.356h.784Zm-2.657-3.06a6.744 6.744 0 0 0-1.19-2.053 6.784 6.784 0 0 0-1.82-1.51A6.705 6.705 0 0 0 12 5.25a6.8 6.8 0 0 0-1.225.11 6.7 6.7 0 0 0-2.15.793 6.784 6.784 0 0 0-2.952 3.489.76.76 0 0 1-.036.098A6.74 6.74 0 0 0 5.251 12a6.74 6.74 0 0 0 3.366 5.842l.009.005a6.704 6.704 0 0 0 2.18.798l.022.003a6.792 6.792 0 0 0 2.368-.004 6.704 6.704 0 0 0 2.205-.811 6.785 6.785 0 0 0 1.762-1.484l.009-.01.009-.01a6.743 6.743 0 0 0 1.18-2.066c.253-.707.39-1.469.39-2.263a6.74 6.74 0 0 0-.408-2.309Z" clip-rule="evenodd" /> </svg> </button>
		</div>
	</div>

	<div class="flex flex-col flex-1 h-full bg-gray-100 dark:bg-app-bg-dark overflow-hidden min-w-0">
		<div class="flex flex-col flex-grow min-h-0 overflow-hidden">
			{#if selectedTab === 'transcriptions'}
                <TranscriptionsView bind:this={transcriptionsViewRef} bind:mediaPlayerRef={mediaPlayerRef} on:requestopentab={handleRequestOpenTab} on:requestmediaselection={handleRequestMediaSelection} />
			{:else if selectedTab === 'notes'}
				 <NotesView on:requestmediaselection={handleRequestMediaSelection} />
			{/if}
		</div>
		<BottomBar />
	</div>

	<TranscribeConfirmModal bind:this={transcribeModalRef} bind:showModal={$project.showTranscribeModal} fileName={modalProps.fileName} modelName={modalProps.modelName} language={modalProps.language} speakers={modalProps.speakers} jobId={modalProps.jobId} on:confirmStart={handleConfirmStartTranscription} on:cancelRequest={handleCancelTranscriptionRequest} on:close={handleModalClose} />
    <UnsavedChangesModal bind:showModal={$project.showUnsavedChangesModal} itemName={$project.unsavedItemName} itemType={$project.unsavedItemType} on:save={handleUnsavedResponse} on:discard={handleUnsavedResponse} on:cancel={handleUnsavedResponse} />
    <ConfirmConversionModal bind:showModal={$project.showConfirmConversionModal} fileName={$project.conversionFileName} on:confirm={handleConversionResponse} on:cancel={handleConversionResponse} />
    <ImportTranscriptSourceModal bind:showModal={showImportTranscriptSourceModal} on:confirm={handleImportTranscriptSourceConfirm} on:close={() => showImportTranscriptSourceModal = false}/>


    {#if importMenuVisible}
        <div id="import-context-menu-div" class="fixed z-50 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-md shadow-xl py-1 text-xs min-w-[120px]" style="left: {importMenuX}px; top: {importMenuY}px;" on:click|stopPropagation>
            <button on:click={() => handleImportMenuAction('audio')} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Audio</button>
            <button on:click={() => handleImportMenuAction('document')} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Document</button>
            <button on:click={() => handleImportMenuAction('image')} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Image</button>
            <button on:click={() => handleImportMenuAction('table')} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Table</button>
            <button on:click={() => handleImportMenuAction('transcript')} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Transcript</button>
            <button on:click={() => handleImportMenuAction('video')} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Video</button>
        </div>
    {/if}

    {#if showLoadingOverlay}
        <div class="absolute inset-0 z-40 flex items-center justify-center bg-black/30 backdrop-blur-sm">
            <div class="flex flex-col items-center p-6 bg-white dark:bg-gray-800 rounded-lg shadow-xl">
                 <Loader class="w-12 h-12 text-blue-500 animate-spin mb-3" />
                 <p class="text-sm text-gray-700 dark:text-gray-300">{$project.statusMessage || 'Loading...'}</p>
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