// --- Mock Svelte Store ---
let currentProjectState;
let prepareMediaNoteViewCallCount = 0;
let lastPrepareMediaNoteViewPath = null;

const initialTestState = {
    name: null, xmlPath: "/fake/project.xml", baseDirectory: "/fake", files: [], documentFiles: [],
    tableFiles: [], imageFiles: [], importedTranscriptFiles: [], documentMetadataFiles: [],
    isLoading: false, error: null, statusMessage: 'Initializing...', selectedMediaNotePath: null,
    currentMediaNoteTranscriptJson: null, initialMediaNoteTranscriptJson: null, isMediaNoteTranscriptDirty: false,
    isMediaNoteTranscriptLoading: false, mediaNoteTranscriptError: null, isImportingAsset: false,
};

function updateProjectState(updater) {
    currentProjectState = updater(currentProjectState);
}
function getProjectState() { return currentProjectState; }
function resetTestState(initialOverrides = {}) {
    currentProjectState = { ...JSON.parse(JSON.stringify(initialTestState)), ...initialOverrides };
    prepareMediaNoteViewCallCount = 0;
    lastPrepareMediaNoteViewPath = null;
}

// --- Mock projectStore functions ---
const projectStoreMock = {
    // Spy on prepareMediaNoteView
    prepareMediaNoteView: (mediaPath) => {
        prepareMediaNoteViewCallCount++;
        lastPrepareMediaNoteViewPath = mediaPath;
        // Minimal state update to reflect what it might do if it were to proceed
        // but the main test is that it's NOT called by importMediaFile.
        // For other tests (like previous files), this mock would be more complete.
        updateProjectState(p => ({
            ...p,
            // selectedMediaNotePath: mediaPath, // This should NOT happen from importMediaFile
            // isMediaNoteTranscriptLoading: !!mediaPath,
            // isLoading: !!mediaPath || p.isLoading,
            // statusMessage: `Loading notes for media: ${mediaPath ? mediaPath.split(/[\\/]/).pop() : ''}`
        }));
    },
    setAssetImportStatus: (isImporting, message = null) => {
        updateProjectState(p => ({ ...p, isImportingAsset: isImporting, statusMessage: message !== null ? message : (isImporting ? 'Importing...' : p.statusMessage), error: isImporting ? null : p.error, isLoading: isImporting }));
    },
    project: { update: updateProjectState, subscribe: () => (() => {}) },
};
const { prepareMediaNoteView, setAssetImportStatus } = projectStoreMock;

// --- Mock Tauri APIs ---
let mockInvokeResponses = {};
async function invoke(command, args) {
    if (mockInvokeResponses[command]) {
        const response = mockInvokeResponses[command];
        if (typeof response === 'function') return response(args);
        return response;
    }
    return Promise.resolve({});
}
async function basename(path) { return path ? path.split(/[\\/]/).pop() : ''; }

// --- Mock Svelte's get() ---
const get = (store) => (store === projectStoreMock.project) ? getProjectState() : undefined;

// --- Functions from projectService.js (adapted) ---
function findMediaPathByName(nodes, filename) { /* Not essential for these tests if prepareMediaNoteView isn't called */ return null; }

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
        await loadProjectDataAndUpdateStore(projectXmlPath);
        updateProjectState(p => ({ ...p, statusMessage: 'Project refreshed.', isLoading: false }));
    } catch (error) {
        updateProjectState(p => ({ ...p, error: `Refresh failed: ${error.message}`, statusMessage: 'Error refreshing file list.', isLoading: false }));
    }
}

// The function under test: importMediaFile (copied from projectService.js with latest changes)
async function importMediaFile(importType = null) { // Added importType param to match original
    const currentProject = get(projectStoreMock.project);
    const projectXmlPath = currentProject.xmlPath;
    // Removed dialogs for testing
    const sourceFilePath = "/mock/path/to/some_video.mp4"; // Mocked
    const filename = await basename(sourceFilePath);

    setAssetImportStatus(true, `Importing ${filename}...`);

    try {
        const backendResponse = await invoke('import_media', {
            sourceFilePathStr: sourceFilePath,
            projectXmlPathStr: projectXmlPath
        });

        if (!backendResponse || typeof backendResponse !== 'object') {
            console.warn('[TestService] import_media returned invalid response:', backendResponse);
            await refreshProjectFiles();
            projectStoreMock.project.update(p => ({ ...p, isImportingAsset: false, isLoading: false, statusMessage: `${filename} imported. File available in project list.` }));
            return;
        }

        const updatedFiles = backendResponse.files || backendResponse.updatedFiles;
        const newMediaPath = backendResponse.new_media_path || backendResponse.newMediaPath;

        if (!Array.isArray(updatedFiles)) {
            console.warn('[TestService] import_media returned no updatedFiles. Falling back to refresh.');
            await refreshProjectFiles();
            projectStoreMock.project.update(p => ({ ...p, isImportingAsset: false, isLoading: false, statusMessage: `${filename} imported. File available in project list.` }));
            return;
        }

        if (Array.isArray(updatedFiles)) {
            projectStoreMock.project.update(p => ({
                ...p,
                files: updatedFiles,
                isImportingAsset: false,
                isLoading: false,
                error: null,
                statusMessage: `${filename} imported successfully.`
            }));
            if (!newMediaPath) {
                 console.warn('[TestService] Successfully imported media, but backend did not return new_media_path for potential future use.');
            }
        } else {
            console.error('[TestService] Backend import_media returned invalid data:', updatedFiles);
            setAssetImportStatus(false, `Error importing ${filename}: Invalid data from backend.`);
            // Removed throw for simpler test flow; error is logged and status set
        }
    } catch (error) {
        console.error('[TestService] Failed to import media file:', error);
        setAssetImportStatus(false, `Error importing media: ${error.message || String(error)}`);
    }
}


// --- Test Runner ---
const tests = {
    "Happy Path: importMediaFile should not call prepareMediaNoteView": async () => {
        resetTestState({ xmlPath: "/fake/project.xml", selectedMediaNotePath: "/some/other/path.mp4" }); // Initial selected path
        const videoFileName = "some_video.mp4"; // From mocked sourceFilePath in importMediaFile
        const newMediaPathFromBackend = "/normalized/path/to/new_video.mp4";

        mockInvokeResponses['import_media'] = () => ({
            updatedFiles: [{ name: videoFileName, path: newMediaPathFromBackend, file_type: "media" }],
            new_media_path: newMediaPathFromBackend
        });

        await importMediaFile();
        const state = getProjectState();

        assert(state.isLoading === false, "isLoading should be false after import.");
        assert(state.isImportingAsset === false, "isImportingAsset should be false after import.");
        assert(state.selectedMediaNotePath === "/some/other/path.mp4", "selectedMediaNotePath should NOT change.");
        assert(state.statusMessage === `${videoFileName} imported successfully.`, `Status message mismatch: ${state.statusMessage}`);
        assert(prepareMediaNoteViewCallCount === 0, `prepareMediaNoteView was called ${prepareMediaNoteViewCallCount} times.`);
        console.log("Happy Path: Test Passed");
    },

    "Fallback Scenario (No updatedFiles): importMediaFile should not call prepareMediaNoteView": async () => {
        resetTestState({ xmlPath: "/fake/project.xml", selectedMediaNotePath: null });
        const videoFileName = "some_video.mp4";

        mockInvokeResponses['import_media'] = () => ({ some_other_info: 'value' }); // No updatedFiles
        mockInvokeResponses['load_project_data'] = () => ({ // For refreshProjectFiles
            files: [{ name: videoFileName, path: "/refreshed/path/to/some_video.mp4", file_type: "media" }]
        });

        await importMediaFile();
        const state = getProjectState();

        assert(state.isLoading === false, "isLoading should be false after fallback import.");
        assert(state.isImportingAsset === false, "isImportingAsset should be false after fallback import.");
        assert(state.selectedMediaNotePath === null, "selectedMediaNotePath should remain null.");
        assert(state.statusMessage === `${videoFileName} imported. File available in project list.`, `Status message mismatch: ${state.statusMessage}`);
        assert(prepareMediaNoteViewCallCount === 0, `prepareMediaNoteView was called ${prepareMediaNoteViewCallCount} times in fallback.`);
        console.log("Fallback Scenario (No updatedFiles): Test Passed");
    },
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
            resetTestState(); // Reset state before each test
            await tests[testName]();
        } catch (e) {
            console.error(`Test ${testName} Failed: ${e.message}\n${e.stack}`);
            allTestsPassed = false;
        }
    }

    if (allTestsPassed) {
        console.log("All importMediaFile behavior tests passed!");
    } else {
        console.error("Some importMediaFile behavior tests failed.");
    }
    // Reset mock for other potential test files in same session
    mockInvokeResponses = {};
}

runTests();
