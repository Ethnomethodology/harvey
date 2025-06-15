// --- Mock Svelte Store ---
let currentProjectState;

const initialTestState = {
    name: null, xmlPath: "/fake/project.xml", baseDirectory: "/fake", files: [], documentFiles: [],
    tableFiles: [], imageFiles: [], importedTranscriptFiles: [], documentMetadataFiles: [],
    isLoading: false, error: null, statusMessage: 'Initializing...', selectedMediaNotePath: null,
    currentMediaNoteTranscriptJson: null, initialMediaNoteTranscriptJson: null, isMediaNoteTranscriptDirty: false,
    isMediaNoteTranscriptLoading: false, mediaNoteTranscriptError: null, isImportingAsset: false,
    // Add other relevant initial states if needed by tested functions
};

function updateProjectState(updater) {
    currentProjectState = updater(currentProjectState);
}
function getProjectState() { return currentProjectState; }
function resetTestState(initialOverrides = {}) {
    currentProjectState = { ...JSON.parse(JSON.stringify(initialTestState)), ...initialOverrides };
}

const defaultEmptyJson = JSON.stringify({
    root: { children: [{ type: 'paragraph', version: 1, children: [], direction: null, format: '', indent: 0 }],
            direction: null, format: '', indent: 0, type: 'root', version: 1 }
});

// --- Mock projectStore functions (will be called by projectService) ---
const projectStoreMock = {
    prepareMediaNoteView: (mediaPath) => {
        const normalizedMediaPath = mediaPath ? mediaPath.replace(/\\/g, '/') : null;
        const newIsMediaNoteLoading = !!normalizedMediaPath;
        updateProjectState(p => {
            const otherFNsToClear = { selectedDocumentPath: null, currentImportedTranscriptPath: null };
            if (p.selectedMediaNotePath !== normalizedMediaPath || !p.selectedMediaNotePath) {
                return { ...p, ...otherFNsToClear, selectedMediaNotePath: normalizedMediaPath, isMediaNoteTranscriptLoading: newIsMediaNoteLoading, mediaNoteTranscriptError: null, isMediaNoteTranscriptDirty: false, currentMediaNoteTranscriptJson: null, initialMediaNoteTranscriptJson: null, statusMessage: normalizedMediaPath ? `Loading notes for media: ${normalizedMediaPath.split(/[\\/]/).pop()}` : 'Media note selection cleared.', isLoading: newIsMediaNoteLoading || p.isLoading, };
            }
            return { ...p, ...otherFNsToClear, selectedMediaNotePath: normalizedMediaPath, statusMessage: `Viewing notes for media: ${normalizedMediaPath.split(/[\\/]/).pop()}`, isMediaNoteTranscriptLoading: p.selectedMediaNotePath !== normalizedMediaPath ? newIsMediaNoteLoading : p.isMediaNoteTranscriptLoading, };
        });
        if (!normalizedMediaPath) {
            updateProjectState(p => ({ ...p, isMediaNoteTranscriptLoading: false, isLoading: false }));
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
    setAssetImportStatus: (isImporting, message = null) => { // Actual function from projectStore
        updateProjectState(p => ({ ...p, isImportingAsset: isImporting, statusMessage: message !== null ? message : (isImporting ? 'Importing...' : p.statusMessage), error: isImporting ? null : p.error, isLoading: isImporting }));
    },
    project: { update: updateProjectState, subscribe: () => (() => {}) }, // Mock store subscribe
};
const { prepareMediaNoteView, setLoadedMediaNoteTranscriptData, setMediaNoteTranscriptLoadFailed, setAssetImportStatus } = projectStoreMock;

// --- Mock Tauri APIs ---
let mockInvokeResponses = {};
async function invoke(command, args) {
    // console.log(`Mock invoke: ${command}`, args);
    if (mockInvokeResponses[command]) {
        const response = mockInvokeResponses[command];
        if (typeof response === 'function') return response(args);
        return response;
    }
    if (command === 'load_note_json') return Promise.reject(new Error("File not found")); // Default for notes
    return Promise.resolve({});
}
async function basename(path) { return path ? path.split(/[\\/]/).pop() : ''; }
// --- Mock Svelte's get() ---
const get = (store) => {
    if (store === projectStoreMock.project) return getProjectState();
    return undefined;
};

// --- Functions from projectService.js (adapted for test) ---
// (These would typically be imported, but are included here for a self-contained test file)
function findMediaPathByName(nodes, filename) {
  if (!Array.isArray(nodes)) return null;
  for (const node of nodes) {
    if (node.file_type === 'media' && !node.is_directory && node.name === filename) {
      return node.path;
    }
    if (node.children) {
      const found = findMediaPathByName(node.children, filename);
      if (found) return found;
    }
  }
  return null;
}

async function loadProjectDataAndUpdateStore(projectXmlPath) {
    updateProjectState(current => ({ ...current, isLoading: true, error: null, statusMessage: 'Loading project data...' }));
    try {
        const loadedData = await invoke('load_project_data', { projectXmlPath });
        const dataToSet = {
            name: loadedData.project_name || 'Test Project',
            id: loadedData.project_uuid || 'uuid',
            xmlPath: loadedData.project_xml_path || projectXmlPath,
            baseDirectory: loadedData.base_directory || '/fake',
            files: loadedData.files || [],
            isLoading: false, error: null,
            statusMessage: `Loaded project: ${loadedData.project_name || 'Test Project'}`
        };
        updateProjectState(current => ({ ...current, ...dataToSet }));
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
        await loadProjectDataAndUpdateStore(projectXmlPath); // Uses mocked invoke
        updateProjectState(p => ({ ...p, statusMessage: 'Project refreshed.', isLoading: false }));
    } catch (error) {
        updateProjectState(p => ({ ...p, error: `Refresh failed: ${error.message}`, statusMessage: 'Error refreshing file list.', isLoading: false }));
    }
}

// The function under test: importMediaFile (copied and adapted from projectService.js)
async function importMediaFile(projectXmlPathFromArg, sourceFilePathFromArg) { // Renamed args to avoid conflict
    const currentProject = get(projectStoreMock.project);
    const projectXmlPath = projectXmlPathFromArg || currentProject.xmlPath; // Use arg if provided for test setup

    setAssetImportStatus(true, `Importing ${await basename(sourceFilePathFromArg)}...`);
    const filename = await basename(sourceFilePathFromArg);

    try {
        const backendResponse = await invoke('import_media', {
            sourceFilePathStr: sourceFilePathFromArg,
            projectXmlPathStr: projectXmlPath
        });

        if (!backendResponse || typeof backendResponse !== 'object') {
            console.warn('[TestService] import_media returned invalid response:', backendResponse);
            await refreshProjectFiles();
            projectStoreMock.project.update(p => ({ ...p, isImportingAsset: false, isLoading: false, statusMessage: `${filename} imported (no metadata returned).` }));
            const proj = get(projectStoreMock.project);
            const realPath = findMediaPathByName(proj.files, filename);
            if (realPath) prepareMediaNoteView(realPath);
            return;
        }

        const updatedFiles = backendResponse.files || backendResponse.updatedFiles;
        const newMediaPath = backendResponse.new_media_path || backendResponse.newMediaPath;

        if (!Array.isArray(updatedFiles)) {
            console.warn('[TestService] import_media returned no updatedFiles. Falling back to refresh.');
            await refreshProjectFiles();
            projectStoreMock.project.update(p => ({ ...p, isImportingAsset: false, isLoading: false, statusMessage: `${filename} imported (refresh applied).` }));
            const proj = get(projectStoreMock.project);
            const realPath = findMediaPathByName(proj.files, filename);
            if (realPath) prepareMediaNoteView(realPath);
            return;
        }

        if (Array.isArray(updatedFiles)) {
            projectStoreMock.project.update(p => ({ ...p, files: updatedFiles, isImportingAsset: false, isLoading: false, error: null, statusMessage: `${filename} imported.` }));
            if (newMediaPath) {
                prepareMediaNoteView(newMediaPath);
            } else {
                console.warn('[TestService] Successfully imported media, but backend did not return new_media_path.');
            }
        } else {
            setAssetImportStatus(false, `Error importing ${filename}: Invalid data from backend.`);
        }
    } catch (error) {
        console.error('[TestService] Failed to import media file:', error);
        setAssetImportStatus(false, `Error importing media.`);
    }
}


// --- Test Runner ---
const tests = {
    "Fallback Scenario (No updatedFiles)": async () => {
        resetTestState({ xmlPath: "/fake/project.xml", baseDirectory: "/fake" });
        const videoToImport = "/import/input_video.mp4";
        const videoFileName = "input_video.mp4";
        const normalizedVideoPath = `/fake/media_files/${videoFileName}`; // Path as it would be in project files

        mockInvokeResponses['import_media'] = () => ({ some_other_info: 'value' }); // No updatedFiles
        mockInvokeResponses['load_project_data'] = () => ({ // Response for refreshProjectFiles
            project_name: "Test Project",
            project_uuid: "uuid_test",
            project_xml_path: "/fake/project.xml",
            base_directory: "/fake",
            files: [{ name: videoFileName, path: normalizedVideoPath, file_type: "media", is_directory: false }]
        });

        let state = getProjectState();
        assert(state.isLoading === false, "Initial isLoading should be false");

        await importMediaFile(state.xmlPath, videoToImport);
        state = getProjectState();

        assert(state.isImportingAsset === false, "isImportingAsset should be false after import attempt");
        assert(state.selectedMediaNotePath === normalizedVideoPath, `selectedMediaNotePath mismatch: ${state.selectedMediaNotePath}`);
        assert(state.statusMessage === `Loading notes for media: ${videoFileName}`, `Status message after prepareMediaNoteView: ${state.statusMessage}`); // Set by prepareMediaNoteView
        assert(state.isLoading === true, "isLoading should be true after prepareMediaNoteView call"); // Set by prepareMediaNoteView
        assert(state.isMediaNoteTranscriptLoading === true, "isMediaNoteTranscriptLoading should be true after prepareMediaNoteView call");

        // Simulate MediaEditorPanel's reaction (notes not found for new media)
        setMediaNoteTranscriptLoadFailed(normalizedVideoPath, "File not found", true);
        state = getProjectState();
        assert(state.isLoading === false, "isLoading should be false after notes load failed");
        assert(state.isMediaNoteTranscriptLoading === false, "isMediaNoteTranscriptLoading should be false after notes load failed");
        assert(state.statusMessage === `No notes/transcription found for ${videoFileName}.`, `Final status message mismatch: ${state.statusMessage}`);
        console.log("Fallback Scenario (No updatedFiles): Passed");
    },

    "Fallback Scenario (Invalid Backend Response)": async () => {
        resetTestState({ xmlPath: "/fake/project.xml", baseDirectory: "/fake" });
        const videoToImport = "/import/another_video.mp4";
        const videoFileName = "another_video.mp4";
        const normalizedVideoPath = `/fake/media_files/${videoFileName}`;

        mockInvokeResponses['import_media'] = () => null; // Invalid response
        mockInvokeResponses['load_project_data'] = () => ({
            files: [{ name: videoFileName, path: normalizedVideoPath, file_type: "media", is_directory: false }]
        });

        await importMediaFile(getProjectState().xmlPath, videoToImport);
        let state = getProjectState();

        assert(state.isImportingAsset === false, "isImportingAsset should be false");
        assert(state.selectedMediaNotePath === normalizedVideoPath, `selectedMediaNotePath mismatch: ${state.selectedMediaNotePath}`);
        assert(state.statusMessage === `Loading notes for media: ${videoFileName}`, `Status after prepareMediaNoteView: ${state.statusMessage}`);
        assert(state.isLoading === true, "isLoading after prepareMediaNoteView");
        assert(state.isMediaNoteTranscriptLoading === true, "isMediaNoteTranscriptLoading after prepareMediaNoteView");

        setMediaNoteTranscriptLoadFailed(normalizedVideoPath, "File not found", true);
        state = getProjectState();
        assert(state.isLoading === false, "isLoading after notes failed");
        assert(state.isMediaNoteTranscriptLoading === false, "isMediaNoteTranscriptLoading after notes failed");
        assert(state.statusMessage === `No notes/transcription found for ${videoFileName}.`, `Final status: ${state.statusMessage}`);
        console.log("Fallback Scenario (Invalid Backend Response): Passed");
    },

    "Happy Path (Regression Test)": async () => {
        resetTestState({ xmlPath: "/fake/project.xml", baseDirectory: "/fake" });
        const videoToImport = "/import/happy_video.mp4";
        const videoFileName = "happy_video.mp4";
        const normalizedVideoPath = "/fake/media_files/happy_video.mp4"; // Normalized path

        mockInvokeResponses['import_media'] = () => ({
            updatedFiles: [{ name: videoFileName, path: normalizedVideoPath, file_type: "media", is_directory: false }],
            new_media_path: normalizedVideoPath // Ensure this is normalized if it comes from backend
        });
        // No need to mock load_project_data as refreshProjectFiles won't be called.

        await importMediaFile(getProjectState().xmlPath, videoToImport);
        let state = getProjectState();

        // After importMediaFile completes its main block (not fallback)
        assert(state.isImportingAsset === false, "isImportingAsset should be false (happy path)");
        assert(state.isLoading === true, "isLoading true after prepareMediaNoteView (happy path)"); // from prepareMediaNoteView
        assert(state.selectedMediaNotePath === normalizedVideoPath, `selectedMediaNotePath mismatch (happy path): ${state.selectedMediaNotePath}`);
        assert(state.statusMessage === `Loading notes for media: ${videoFileName}`, `Status after prepare (happy path): ${state.statusMessage}`);

        setMediaNoteTranscriptLoadFailed(normalizedVideoPath, "File not found", true);
        state = getProjectState();
        assert(state.isLoading === false, "isLoading after notes failed (happy path)");
        assert(state.isMediaNoteTranscriptLoading === false, "isMediaNoteTranscriptLoading after notes failed (happy path)");
        assert(state.statusMessage === `No notes/transcription found for ${videoFileName}.`, `Final status (happy path): ${state.statusMessage}`);
        console.log("Happy Path (Regression Test): Passed");
    }
};

function assert(condition, message) {
    if (!condition) {
        throw new Error(`Assertion failed: ${message}`);
    }
}

async function runTests() {
    let allTestsPassed = true;
    for (const testName in tests) {
        try {
            await tests[testName]();
        } catch (e) {
            console.error(`Test ${testName} Failed: ${e.message}\n${e.stack}`);
            allTestsPassed = false;
        }
    }

    if (allTestsPassed) {
        console.log("All projectService fallback logic tests passed!");
    } else {
        console.error("Some projectService fallback logic tests failed.");
    }
}

runTests();
