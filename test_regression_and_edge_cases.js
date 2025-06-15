// --- Mock Svelte Store and Global State ---
let currentProjectState;
let prepareMediaNoteViewCallLog = []; // To store { path, count }
let selectMediaCallLog = []; // To store { mediaFile, count }

const initialTestState = {
    name: "Test Project", xmlPath: "/fake/project.xml", baseDirectory: "/fake",
    files: [], documentFiles: [], tableFiles: [], imageFiles: [], importedTranscriptFiles: [], documentMetadataFiles: [],
    isLoading: false, error: null, statusMessage: 'Ready',
    selectedMediaNotePath: null,
    currentMediaNoteTranscriptJson: null, initialMediaNoteTranscriptJson: null, isMediaNoteTranscriptDirty: false,
    isMediaNoteTranscriptLoading: false, mediaNoteTranscriptError: null,
    isImportingAsset: false,
    // --- States for other views (to ensure they are cleared by prepareMediaNoteView) ---
    selectedDocumentPath: null, currentDocumentJson: null, initialDocumentJson: null, isDocumentDirty: false,
    isDocumentLoading: false, documentError: null, activeDocumentEditorRef: null,
    currentImportedTranscriptPath: null, currentImportedTranscriptLexicalJson: null, initialImportedTranscriptLexicalJson: null,
    isImportedTranscriptDirty: false, isImportedTranscriptLoading: false, importedTranscriptError: null,
};

const defaultEmptyJson = JSON.stringify({
    root: { children: [{ type: 'paragraph', version: 1, children: [], direction: null, format: '', indent: 0 }],
            direction: null, format: '', indent: 0, type: 'root', version: 1 }
});

function updateProjectState(updater) {
    currentProjectState = updater(currentProjectState);
}
function getProjectState() { return currentProjectState; }
function resetTestState(initialOverrides = {}) {
    currentProjectState = { ...JSON.parse(JSON.stringify(initialTestState)), ...initialOverrides };
    prepareMediaNoteViewCallLog = [];
    selectMediaCallLog = [];
}

// --- Mock projectStore functions ---
const projectStoreMock = {
    prepareMediaNoteView: (mediaPath) => {
        const normalizedMediaPath = mediaPath ? mediaPath.replace(/\\/g, '/') : null;
        const existingCall = prepareMediaNoteViewCallLog.find(c => c.path === normalizedMediaPath); // Compare with normalized
        if (existingCall) existingCall.count++; else prepareMediaNoteViewCallLog.push({ path: normalizedMediaPath, count: 1 });

        const newIsMediaNoteLoading = !!normalizedMediaPath;
        updateProjectState(p => {
            const otherFNsToClear = { selectedDocumentPath: null, currentDocumentJson: null, currentImportedTranscriptPath: null, /* other view states */ };
            // If path changes, or if it's the same path but no data is loaded/being loaded, then prepare for loading.
            if (p.selectedMediaNotePath !== normalizedMediaPath || !p.selectedMediaNotePath || (p.selectedMediaNotePath === normalizedMediaPath && !p.isMediaNoteTranscriptLoading && !p.currentMediaNoteTranscriptJson) ) {
                return { ...p, ...otherFNsToClear, selectedMediaNotePath: normalizedMediaPath, isMediaNoteTranscriptLoading: newIsMediaNoteLoading, mediaNoteTranscriptError: null, isMediaNoteTranscriptDirty: false, currentMediaNoteTranscriptJson: null, initialMediaNoteTranscriptJson: null, statusMessage: normalizedMediaPath ? `Loading notes for media: ${normalizedMediaPath.split(/[\\/]/).pop()}` : 'Media note selection cleared.', isLoading: newIsMediaNoteLoading || p.isLoading, };
            }
            // If it's the same path and it's already loading or loaded, status might just be viewing.
            return { ...p, ...otherFNsToClear, selectedMediaNotePath: normalizedMediaPath, statusMessage: `Viewing notes for media: ${normalizedMediaPath.split(/[\\/]/).pop()}`, isMediaNoteTranscriptLoading: p.isMediaNoteTranscriptLoading, isLoading: p.isLoading }; // Keep isLoading as is if just viewing
        });
        if (!normalizedMediaPath) { // Clearing selection
            updateProjectState(p => ({ ...p, selectedMediaNotePath: null, isMediaNoteTranscriptLoading: false, isLoading: false, currentMediaNoteTranscriptJson: null, initialMediaNoteTranscriptJson: null, mediaNoteTranscriptError: null, statusMessage: 'Media note selection cleared.' }));
        }
    },
    setLoadedMediaNoteTranscriptData: (mediaPath, jsonString) => {
        const normalizedMediaPath = mediaPath ? mediaPath.replace(/\\/g, '/') : null;
        updateProjectState(p => {
            if (p.selectedMediaNotePath === normalizedMediaPath) {
                return { ...p, currentMediaNoteTranscriptJson: jsonString || defaultEmptyJson, initialMediaNoteTranscriptJson: jsonString || defaultEmptyJson, isMediaNoteTranscriptDirty: false, isMediaNoteTranscriptLoading: false, mediaNoteTranscriptError: null, statusMessage: `Loaded notes for media: ${normalizedMediaPath.split(/[\\/]/).pop()}.`, isLoading: false, };
            } return p;
        });
    },
    setMediaNoteTranscriptLoadFailed: (mediaPath, errorMsg, isFileNotFound = false) => {
        const normalizedMediaPath = mediaPath ? mediaPath.replace(/\\/g, '/') : null;
        updateProjectState(p => {
            if (p.selectedMediaNotePath === normalizedMediaPath) {
                return { ...p, currentMediaNoteTranscriptJson: defaultEmptyJson, initialMediaNoteTranscriptJson: defaultEmptyJson, isMediaNoteTranscriptDirty: false, isMediaNoteTranscriptLoading: false, mediaNoteTranscriptError: isFileNotFound ? "INFO:FILE_NOT_FOUND" : `Failed to load notes: ${errorMsg}`, statusMessage: isFileNotFound ? `No notes/transcription found for ${normalizedMediaPath.split(/[\\/]/).pop()}.` : `Error loading notes for ${normalizedMediaPath.split(/[\\/]/).pop()}.`, isLoading: false, };
            } return p;
        });
    },
    setAssetImportStatus: (isImporting, message = null) => {
        updateProjectState(p => ({ ...p, isImportingAsset: isImporting, statusMessage: message !== null ? message : (isImporting ? 'Importing...' : p.statusMessage), error: isImporting ? null : p.error, isLoading: isImporting }));
    },
    project: { update: updateProjectState, subscribe: () => (() => {}) },
};
const { prepareMediaNoteView, setLoadedMediaNoteTranscriptData, setMediaNoteTranscriptLoadFailed, setAssetImportStatus } = projectStoreMock;

// --- Mock transcriptStore functions (very basic spy) ---
const transcriptStoreMock = {
    selectMedia: (mediaFile) => {
        const path = mediaFile ? (mediaFile.path ? mediaFile.path.replace(/\\/g, '/') : null) : null; // Normalize path for logging
        const existingCall = selectMediaCallLog.find(c => c.mediaFile?.path === path);
        if (existingCall) existingCall.count++; else selectMediaCallLog.push({ mediaFile: mediaFile ? {...mediaFile, path } : null, count: 1 });

        if (mediaFile && mediaFile.path) {
            prepareMediaNoteView(mediaFile.path); // prepareMediaNoteView will also normalize
        } else {
            prepareMediaNoteView(null);
        }
    }
};

// --- Mock Tauri APIs & Svelte get ---
let mockInvokeResponses = {};
async function invoke(command, args) {
    if (mockInvokeResponses[command]) {
        const response = typeof mockInvokeResponses[command] === 'function' ? mockInvokeResponses[command](args) : mockInvokeResponses[command];
        if (response && response.error && command === 'load_note_json' && args && args.filePath) {
             const filePathInstruction = response[args.filePath];
             if (filePathInstruction && filePathInstruction.error) {
                return Promise.reject(new Error(filePathInstruction.message || "File not found"));
             }
             if (filePathInstruction) {
                return Promise.resolve(filePathInstruction.data);
             }
        } else if (response && command === 'load_note_json' && args && args.filePath && response[args.filePath]) {
             return Promise.resolve(response[args.filePath].data);
        }
        if (response && response.error) return Promise.reject(new Error(response.message || "Generic error"));
        return response;
    }
    if (command === 'load_note_json') return Promise.reject(new Error("File not found (default mock)"));
    return Promise.resolve({});
}
async function basename(path) { return path ? path.split(/[\\/]/).pop() : ''; }
const get = (store) => (store === projectStoreMock.project) ? getProjectState() : undefined;


// --- projectService.js functions (adapted) ---
function findMediaPathByName(nodes, filename) {
  if (!Array.isArray(nodes)) return null;
  for (const node of nodes) {
    if (node.file_type === 'media' && !node.is_directory && node.name === filename) return node.path;
    if (node.children) { const found = findMediaPathByName(node.children, filename); if (found) return found; }
  } return null;
}

async function loadProjectDataAndUpdateStore(projectXmlPath, targetPathToSelect = null) {
    updateProjectState(current => ({ ...current, isLoading: true, error: null, statusMessage: 'Loading project data...' }));
    try {
        const loadedData = await invoke('load_project_data', { projectXmlPath });
        const dataToSet = {
            name: loadedData.project_name || 'Test Project', id: loadedData.project_uuid || 'uuid',
            xmlPath: loadedData.project_xml_path || projectXmlPath, baseDirectory: loadedData.base_directory || '/fake',
            files: loadedData.files || [], /*isLoading: false,*/ error: null, // isLoading will be handled by refreshProjectFiles or prepareMediaNoteView
            statusMessage: `Loaded project: ${loadedData.project_name || 'Test Project'}`
        };
        updateProjectState(current => ({ ...current, ...dataToSet }));

        let mediaFileToSelect = null;
        if (targetPathToSelect) {
            mediaFileToSelect = (loadedData.files || []).find(f => f.path === targetPathToSelect && f.file_type === 'media' && !f.is_directory);
        } else {
            const firstMediaNode = (loadedData.files || []).find(f => f.file_type === 'media' && !f.is_directory);
            if (firstMediaNode) mediaFileToSelect = firstMediaNode;
        }
        if (mediaFileToSelect) transcriptStoreMock.selectMedia(mediaFileToSelect);
        else transcriptStoreMock.selectMedia(null); // This will set isLoading: false if no media selected.

        // If selectMedia didn't trigger prepareMediaNoteView (e.g. no media files),
        // then isLoading might still be true here from the start of loadProjectDataAndUpdateStore.
        // refreshProjectFiles will handle setting it false.
        // However, if prepareMediaNoteView was called, it has its own isLoading management.
        // This logic is tricky. The `refreshProjectFiles` will handle final isLoading.

    } catch (error) {
        updateProjectState(current => ({ ...current, isLoading: false, error: error?.message || 'Unknown error', statusMessage: `Error loading project.` }));
        throw error;
    }
}

async function refreshProjectFiles() {
    const currentProj = get(projectStoreMock.project);
    const projectXmlPath = currentProj.xmlPath;
    if (!projectXmlPath) return;
    updateProjectState(p => ({ ...p, statusMessage: 'Refreshing file list...', isLoading: true }));
    try {
        await loadProjectDataAndUpdateStore(projectXmlPath); // This might trigger prepareMediaNoteView
        const currentState = getProjectState();
        // If prepareMediaNoteView was called, it would have set isLoading and its own status message.
        // Only override if no specific note loading is in progress.
        if (!currentState.isMediaNoteTranscriptLoading) {
            updateProjectState(p => ({ ...p, statusMessage: 'Project refreshed.', isLoading: false }));
        } else {
            // If isMediaNoteTranscriptLoading is true, prepareMediaNoteView has set isLoading = true.
            // We don't touch statusMessage, but ensure isLoading reflects the ongoing specific load.
             updateProjectState(p => ({ ...p, isLoading: true }));
        }
    } catch (error) {
        updateProjectState(p => ({ ...p, error: `Refresh failed: ${error.message}`, statusMessage: 'Error refreshing file list.', isLoading: false }));
    }
}


async function importMediaFile(projectXmlPathFromArg, sourceFilePathFromArg) {
    const currentProject = get(projectStoreMock.project);
    const projectXmlPath = projectXmlPathFromArg || currentProject.xmlPath;
    const filename = await basename(sourceFilePathFromArg);
    setAssetImportStatus(true, `Importing ${filename}...`);
    try {
        const backendResponse = await invoke('import_media', { sourceFilePathStr: sourceFilePathFromArg, projectXmlPathStr: projectXmlPath });
        if (!backendResponse || typeof backendResponse !== 'object') {
            await refreshProjectFiles();
            // After refresh, isLoading and statusMessage are set by refreshProjectFiles/prepareMediaNoteView.
            // We just need to ensure isImportingAsset is false.
            updateProjectState(p => ({ ...p, isImportingAsset: false }));
            // If refresh didn't trigger a note load, set a generic import success.
            if (!getProjectState().isMediaNoteTranscriptLoading && !getProjectState().isLoading) {
                 projectStoreMock.project.update(p => ({...p, statusMessage: `${filename} imported. File available in project list.`}));
            }
            return;
        }
        const updatedFiles = backendResponse.files || backendResponse.updatedFiles;
        const newMediaPath = backendResponse.new_media_path || backendResponse.newMediaPath;
        if (!Array.isArray(updatedFiles)) {
            await refreshProjectFiles();
            updateProjectState(p => ({ ...p, isImportingAsset: false }));
            if (!getProjectState().isMediaNoteTranscriptLoading && !getProjectState().isLoading) {
                 projectStoreMock.project.update(p => ({...p, statusMessage: `${filename} imported. File available in project list.`}));
            }
            return;
        }
        if (Array.isArray(updatedFiles)) {
            projectStoreMock.project.update(p => ({ ...p, files: updatedFiles, isImportingAsset: false, isLoading: false, error: null, statusMessage: `${filename} imported successfully.` }));
            if (!newMediaPath) console.warn('[TestService] Successfully imported media, but backend did not return new_media_path.');
        } else { setAssetImportStatus(false, `Error importing ${filename}: Invalid data from backend.`);}
    } catch (error) { setAssetImportStatus(false, `Error importing media: ${error.message || String(error)}`); }
}

// --- Test Runner ---
const tests = {
    "Import - Fallback Logic & Auto Select via Refresh": async () => {
        resetTestState({ xmlPath: "/fake/project.xml" });
        const videoToImport = "/import/fallback_video.mp4";
        const videoFileName = "fallback_video.mp4";
        const finalVideoPath = `/fake/media_files/${videoFileName}`; // Normalized
        const notesPathForMedia = `/fake/transcripts/${videoFileName.replace('.mp4', '.json')}`;

        mockInvokeResponses['import_media'] = () => ({ some_other_info: 'value' });
        mockInvokeResponses['load_project_data'] = () => ({
            files: [{ name: videoFileName, path: finalVideoPath, file_type: "media", is_directory: false }]
        });
        mockInvokeResponses['load_note_json'] = { [notesPathForMedia]: { error: true, message: "File not found" } };

        await importMediaFile(getProjectState().xmlPath, videoToImport);
        let state = getProjectState();

        assert(state.isImportingAsset === false, "isImportingAsset after import fallback not false");
        assert(prepareMediaNoteViewCallLog.length === 1, `prepareMediaNoteView call count mismatch: ${prepareMediaNoteViewCallLog.length}`);
        assert(prepareMediaNoteViewCallLog[0].path === finalVideoPath, `prepareMediaNoteView called with wrong path: ${prepareMediaNoteViewCallLog[0].path}`);
        assert(state.selectedMediaNotePath === finalVideoPath, "selectedMediaNotePath not set by auto-select");
        assert(state.isMediaNoteTranscriptLoading === true, "isMediaNoteTranscriptLoading not true after auto-select");
        assert(state.isLoading === true, "isLoading should be true as note loading has started");
        assert(state.statusMessage === `Loading notes for media: ${videoFileName}`, `Status for auto-select: "${state.statusMessage}"`);

        setMediaNoteTranscriptLoadFailed(finalVideoPath, "File not found", true);
        state = getProjectState();
        assert(state.isLoading === false, "isLoading not false after notes failed (fallback auto-select)");
        assert(state.isMediaNoteTranscriptLoading === false, "isMediaNoteTranscriptLoading not false (fallback auto-select)");
        assert(state.mediaNoteTranscriptError === "INFO:FILE_NOT_FOUND", "Error not INFO:FILE_NOT_FOUND (fallback auto-select)");
        console.log("Import - Fallback Logic & Auto Select via Refresh: Passed");
    },

    "Rapid Import then Select": async () => {
        resetTestState({ xmlPath: "/fake/project.xml" });
        const videoName = "rapid_video.mp4";
        const videoPath = `/fake/media/${videoName}`; // Normalized
        mockInvokeResponses['import_media'] = () => ({ updatedFiles: [{name: videoName, path: videoPath, file_type: 'media'}], new_media_path: videoPath });
        const notesJsonPath = `/fake/transcripts/${videoName.replace('.mp4', '.json')}`;
        mockInvokeResponses['load_note_json'] = { [notesJsonPath]: { error: true, message: "File not found" } };

        await importMediaFile(getProjectState().xmlPath, `/import/${videoName}`);
        let state = getProjectState();
        assert(state.isLoading === false && !state.isImportingAsset, "Import did not complete cleanly");
        assert(prepareMediaNoteViewCallLog.length === 0, "prepareMediaNoteView called by import (happy path)");
        assert(state.statusMessage === `${videoName} imported successfully.`, "Import status message incorrect");

        prepareMediaNoteView(videoPath); // Manual selection
        state = getProjectState();
        assert(state.isLoading === true && state.isMediaNoteTranscriptLoading === true, "Selection did not trigger loading state");
        assert(state.selectedMediaNotePath === videoPath, "Selected path not set by selection");
        assert(state.statusMessage === `Loading notes for media: ${videoName}`, "Select status message incorrect");

        setMediaNoteTranscriptLoadFailed(videoPath, "File not found", true); // Note load fails
        state = getProjectState();
        assert(state.isLoading === false && state.isMediaNoteTranscriptLoading === false, "Note load fail did not clear loading state");
        assert(state.mediaNoteTranscriptError === "INFO:FILE_NOT_FOUND", "Error not set by note fail");
        console.log("Rapid Import then Select: Passed");
    },

    "Import while another media note is selected": async () => {
        const openMediaName = "mediaA.mp4";
        const openMediaPath = `/fake/media/${openMediaName}`; // Normalized
        const openMediaNotesJson = JSON.stringify({ text: "Notes for A" });
        resetTestState({
            xmlPath: "/fake/project.xml",
            selectedMediaNotePath: openMediaPath,
            currentMediaNoteTranscriptJson: openMediaNotesJson,
            initialMediaNoteTranscriptJson: openMediaNotesJson,
            isMediaNoteTranscriptLoading: false, isLoading: false,
            statusMessage: `Loaded notes for media: ${openMediaName}.`
        });
        prepareMediaNoteViewCallLog = [{path: openMediaPath, count: 1}];

        const newVideoName = "mediaB.mp4";
        const newVideoPath = `/fake/media/${newVideoName}`; // Normalized
        mockInvokeResponses['import_media'] = () => ({ updatedFiles: [{name: newVideoName, path: newVideoPath, file_type: 'media'}], new_media_path: newVideoPath });

        await importMediaFile(getProjectState().xmlPath, `/import/${newVideoName}`);
        let state = getProjectState();
        assert(state.isLoading === false && !state.isImportingAsset, "Import of B did not complete cleanly");
        assert(state.selectedMediaNotePath === openMediaPath, "selectedMediaNotePath changed from A to B");
        assert(state.currentMediaNoteTranscriptJson === openMediaNotesJson, "Notes of A disturbed");
        const bCall = prepareMediaNoteViewCallLog.find(c => c.path === newVideoPath);
        assert(!bCall, "prepareMediaNoteView called for B during import");
        const aCall = prepareMediaNoteViewCallLog.find(c => c.path === openMediaPath);
        assert(aCall && aCall.count === 1, "prepareMediaNoteView for A was called again or not as expected");

        assert(state.statusMessage === `${newVideoName} imported successfully.`, "Status for B import incorrect");
        console.log("Import while another media note is selected: Passed");
    },

    "Clearing Media Selection": async () => {
        resetTestState({ xmlPath: "/fake/project.xml" });
        const mediaName = "selectable.mp4";
        const mediaPath = `/fake/media/${mediaName}`; // Normalized
        const notesJsonPath = `/fake/transcripts/${mediaName.replace('.mp4', '.json')}`;
        mockInvokeResponses['load_note_json'] = { [notesJsonPath]: { data: defaultEmptyJson } };

        prepareMediaNoteView(mediaPath);
        setLoadedMediaNoteTranscriptData(mediaPath, defaultEmptyJson);
        let state = getProjectState();
        assert(state.selectedMediaNotePath === mediaPath && !state.isLoading && !state.isMediaNoteTranscriptLoading, "Media not loaded correctly");

        prepareMediaNoteView(null); // Clear selection
        state = getProjectState();
        assert(state.selectedMediaNotePath === null, "selectedMediaNotePath not cleared");
        assert(state.currentMediaNoteTranscriptJson === null, "currentMediaNoteTranscriptJson not cleared");
        assert(state.isMediaNoteTranscriptLoading === false, "isMediaNoteTranscriptLoading not false after clear");
        assert(state.isLoading === false, "isLoading not false after clear");
        assert(state.statusMessage === "Media note selection cleared.", "Clear message incorrect");
        console.log("Clearing Media Selection: Passed");
    },
    "General Project Load with auto-select media": async () => {
        resetTestState({ xmlPath: "/fake/project.xml" });
        const mediaName = "first_media.mp4";
        const mediaPath = `/fake/media_files/${mediaName}`; // Normalized
        const notesJsonPath = `/fake/transcripts/${mediaName.replace('.mp4', '.json')}`;

        mockInvokeResponses['load_project_data'] = () => ({
            files: [{ name: mediaName, path: mediaPath, file_type: "media", is_directory: false }]
        });
        mockInvokeResponses['load_note_json'] = {
            [notesJsonPath]: { error: true, message: "File not found" }
        };

        await loadProjectDataAndUpdateStore(getProjectState().xmlPath);

        let state = getProjectState();
        assert(selectMediaCallLog.length === 1 && selectMediaCallLog[0].mediaFile?.path === mediaPath, `selectMedia not called or called with wrong path. Log: ${JSON.stringify(selectMediaCallLog)}`);
        assert(prepareMediaNoteViewCallLog.length === 1 && prepareMediaNoteViewCallLog[0].path === mediaPath, `prepareMediaNoteView not called by selectMedia pathway. Log: ${JSON.stringify(prepareMediaNoteViewCallLog)}`);

        assert(state.selectedMediaNotePath === mediaPath, "Auto-selected path not set");
        assert(state.isLoading === true, "isLoading not true after auto-select's prepare");
        assert(state.isMediaNoteTranscriptLoading === true, "isMediaNoteTranscriptLoading not true after auto-select's prepare");
        assert(state.statusMessage === `Loading notes for media: ${mediaName}`, `Status message mismatch after prepare: ${state.statusMessage}`);


        setMediaNoteTranscriptLoadFailed(mediaPath, "File not found", true);
        state = getProjectState();
        assert(state.isLoading === false, "isLoading not false after auto-select's notes failed");
        assert(state.isMediaNoteTranscriptLoading === false, "isMediaNoteTranscriptLoading not false after auto-select's notes failed");
        assert(state.mediaNoteTranscriptError === "INFO:FILE_NOT_FOUND", "Error not INFO:FILE_NOT_FOUND for auto-selected");
        console.log("General Project Load with auto-select media: Passed");
    }
};

function assert(condition, message) { if (!condition) throw new Error(`Assertion failed: ${message}`); }
async function runTests() {
    let allPassed = true;
    for (const testName in tests) {
        try {
            resetTestState();
            await tests[testName]();
        }
        catch (e) { console.error(`Test ${testName} Failed: ${e.message}\n${e.stack}`); allPassed = false; }
    }
    console.log(allPassed ? "All regression/edge case tests passed!" : "Some regression/edge case tests FAILED.");
    mockInvokeResponses = {};
}
runTests();
