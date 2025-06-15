// Extracted and simplified Svelte store logic for testing
let currentProjectState;

// --- Copied and adapted from src/lib/stores/projectStore.js ---
const defaultEmptyJson = JSON.stringify({
    root: {
        children: [{ children: [], direction: null, format: '', indent: 0, type: 'paragraph', version: 1 }],
        direction: null, format: '', indent: 0, type: 'root', version: 1
    }
});

const initialTestState = {
    name: null,
    xmlPath: null,
    baseDirectory: null,
    files: [],
    documentFiles: [],
    tableFiles: [],
    imageFiles: [],
    importedTranscriptFiles: [],
    documentMetadataFiles: [],
    isLoading: false, // Start with false for clearer test assertions
    error: null,
    statusMessage: 'Initializing...',
    requestedNoteToLoad: null,
    selectedDocumentPath: null,
    currentDocumentJson: null,
    initialDocumentJson: null,
    isDocumentDirty: false,
    isDocumentLoading: false,
    documentError: null,
    activeDocumentEditorRef: null,
    currentDocumentFileLevelMetadata: { file_name: '', last_modified: '', title: '', description: '', summary: '' },
    currentDocumentHighlights: [],
    isDocumentMetadataDirty: false,
    currentPdfAnnotations: [],
    initialPdfAnnotations: [],
    isPdfAnnotationsDirty: false,
    currentImportedTranscriptPath: null,
    currentImportedTranscriptLexicalJson: null,
    initialImportedTranscriptLexicalJson: null,
    isImportedTranscriptDirty: false,
    isImportedTranscriptLoading: false,
    importedTranscriptError: null,
    activeImportedTranscriptEditorRef: null,
    selectedMediaNotePath: null,
    currentMediaNoteTranscriptJson: null,
    initialMediaNoteTranscriptJson: null,
    isMediaNoteTranscriptDirty: false,
    isMediaNoteTranscriptLoading: false,
    mediaNoteTranscriptError: null,
    activeMediaNoteEditorRef: null,
    autosaveEnabled: true,
    showUnsavedChangesModal: false,
    unsavedItemName: '',
    unsavedItemType: '',
    onUnsavedSave: () => {},
    onUnsavedDiscard: () => {},
    onUnsavedCancel: () => {},
    isImportingAsset: false,
    showConfirmConversionModal: false,
    conversionFileName: '',
    onConversionConfirm: () => {},
    onConversionCancel: () => {},
};

function updateProjectState(updater) {
    currentProjectState = updater(currentProjectState);
}

function getProjectState() {
    return currentProjectState;
}

function resetTestState() {
    currentProjectState = JSON.parse(JSON.stringify(initialTestState));
}


function prepareMediaNoteView(mediaPath) {
    const normalizedMediaPath = mediaPath ? mediaPath.replace(/\\/g, '/') : null;
    // console.debug(`[TestProjectStore] prepareMediaNoteView called for mediaPath: ${mediaPath}, normalized to: ${normalizedMediaPath}`);
    const newIsMediaNoteLoading = !!normalizedMediaPath;
    updateProjectState(p => {
        const otherFieldnotesStatesToClear = {
            selectedDocumentPath: null, currentDocumentJson: null, initialDocumentJson: null, isDocumentDirty: false,
            isDocumentLoading: false, documentError: null, activeDocumentEditorRef: null,
            currentDocumentFileLevelMetadata: { file_name: '', last_modified: '', title: '', description: '', summary: '' },
            currentDocumentHighlights: [], isDocumentMetadataDirty: false,
            currentPdfAnnotations: [], initialPdfAnnotations: [], isPdfAnnotationsDirty: false,
            currentImportedTranscriptPath: null, currentImportedTranscriptLexicalJson: null, initialImportedTranscriptLexicalJson: null,
            isImportedTranscriptDirty: false, isImportedTranscriptLoading: false,
            importedTranscriptError: null, activeImportedTranscriptEditorRef: null,
        };
        if (p.selectedMediaNotePath !== normalizedMediaPath || !p.selectedMediaNotePath) {
            return {
                ...p, ...otherFieldnotesStatesToClear,
                selectedMediaNotePath: normalizedMediaPath,
                isMediaNoteTranscriptLoading: newIsMediaNoteLoading,
                mediaNoteTranscriptError: null,
                isMediaNoteTranscriptDirty: false,
                currentMediaNoteTranscriptJson: null,
                initialMediaNoteTranscriptJson: null,
                activeMediaNoteEditorRef: null,
                statusMessage: normalizedMediaPath ? `Loading notes for media: ${normalizedMediaPath.split(/[\\/]/).pop()}` : 'Media note selection cleared.',
                isLoading: newIsMediaNoteLoading || p.isLoading,
            };
        }
        return {
            ...p, ...otherFieldnotesStatesToClear,
            selectedMediaNotePath: normalizedMediaPath,
            statusMessage: `Viewing notes for media: ${normalizedMediaPath.split(/[\\/]/).pop()}`,
            isMediaNoteTranscriptLoading: p.selectedMediaNotePath !== normalizedMediaPath ? newIsMediaNoteLoading : p.isMediaNoteTranscriptLoading,
        };
    });
    if (!normalizedMediaPath) {
        updateProjectState(p => ({ ...p, isMediaNoteTranscriptLoading: false, isLoading: false }));
    }
}

function setLoadedMediaNoteTranscriptData(mediaPath, jsonString) {
    // console.info(`[TestProjectStore] Setting loaded media note transcript data for media: ${mediaPath}`);
    const normalizedMediaPath = mediaPath ? mediaPath.replace(/\\/g, '/') : null; // Normalize here too for safety, though selectedMediaNotePath should be normalized
    updateProjectState(p => {
        if (p.selectedMediaNotePath === normalizedMediaPath) {
            const content = jsonString || defaultEmptyJson;
            return {
                ...p,
                currentMediaNoteTranscriptJson: content,
                initialMediaNoteTranscriptJson: content,
                isMediaNoteTranscriptDirty: false,
                isMediaNoteTranscriptLoading: false,
                mediaNoteTranscriptError: null,
                statusMessage: `Loaded notes for media: ${normalizedMediaPath.split(/[\\/]/).pop()}.`, // Added period
                isLoading: false,
            };
        }
        return p;
    });
}

function setMediaNoteTranscriptLoadFailed(mediaPath, errorMsg, isFileNotFound = false) {
    // console.error(`[TestProjectStore] Media note transcript load failed for media: ${mediaPath}`, errorMsg);
    const normalizedMediaPath = mediaPath ? mediaPath.replace(/\\/g, '/') : null; // Normalize here too
    updateProjectState(p => {
        if (p.selectedMediaNotePath === normalizedMediaPath) {
            return {
                ...p,
                currentMediaNoteTranscriptJson: defaultEmptyJson,
                initialMediaNoteTranscriptJson: defaultEmptyJson,
                isMediaNoteTranscriptDirty: false,
                isMediaNoteTranscriptLoading: false,
                mediaNoteTranscriptError: isFileNotFound ? "INFO:FILE_NOT_FOUND" : `Failed to load notes: ${errorMsg}`,
                statusMessage: isFileNotFound ? `No notes/transcription found for ${normalizedMediaPath.split(/[\\/]/).pop()}.` : `Error loading notes for ${normalizedMediaPath.split(/[\\/]/).pop()}.`,
                activeMediaNoteEditorRef: null,
                isLoading: false,
            };
        }
        return p;
    });
}

// --- Test Runner ---
const tests = {
    "Test Case 1: Import new video (Windows path) and expect file not found": () => {
        resetTestState();
        const videoPath = "C:\\path\\to\\input_video.mp4";
        const normalizedVideoPath = "C:/path/to/input_video.mp4";

        prepareMediaNoteView(videoPath);
        let state = getProjectState();
        assert(state.selectedMediaNotePath === normalizedVideoPath, `selectedMediaNotePath expected ${normalizedVideoPath}, got ${state.selectedMediaNotePath}`);
        assert(state.isLoading === true, "isLoading should be true after prepareMediaNoteView");
        assert(state.isMediaNoteTranscriptLoading === true, "isMediaNoteTranscriptLoading should be true after prepareMediaNoteView");
        assert(state.statusMessage === `Loading notes for media: input_video.mp4`, `Status message mismatch: ${state.statusMessage}`);

        // Simulate MediaEditorPanel calling setMediaNoteTranscriptLoadFailed because the notes JSON doesn't exist
        setMediaNoteTranscriptLoadFailed(normalizedVideoPath, "File not found", true); // Pass normalized path as MediaEditorPanel would use the store value
        state = getProjectState();
        assert(state.isLoading === false, "isLoading should be false after load failed");
        assert(state.isMediaNoteTranscriptLoading === false, "isMediaNoteTranscriptLoading should be false after load failed");
        assert(state.mediaNoteTranscriptError === "INFO:FILE_NOT_FOUND", `Error state mismatch: ${state.mediaNoteTranscriptError}`);
        assert(state.statusMessage === `No notes/transcription found for input_video.mp4.`, `Status message mismatch: ${state.statusMessage}`); // Added period
        console.log("Test Case 1: Passed");
    },

    "Test Case 1b: Import new video (Unix path) and expect file not found": () => {
        resetTestState();
        const videoPath = "/path/to/input_video.mp4";

        prepareMediaNoteView(videoPath);
        let state = getProjectState();
        assert(state.selectedMediaNotePath === videoPath, `selectedMediaNotePath expected ${videoPath}, got ${state.selectedMediaNotePath}`);
        assert(state.isLoading === true, "isLoading should be true");
        assert(state.isMediaNoteTranscriptLoading === true, "isMediaNoteTranscriptLoading should be true");

        setMediaNoteTranscriptLoadFailed(videoPath, "File not found", true);
        state = getProjectState();
        assert(state.isLoading === false, "isLoading should be false");
        assert(state.isMediaNoteTranscriptLoading === false, "isMediaNoteTranscriptLoading should be false");
        assert(state.mediaNoteTranscriptError === "INFO:FILE_NOT_FOUND", `Error state mismatch: ${state.mediaNoteTranscriptError}`);
        assert(state.statusMessage === `No notes/transcription found for input_video.mp4.`, `Status message mismatch: ${state.statusMessage}`); // Added period
        console.log("Test Case 1b: Passed");
    },

    "Test Case 1c: Clear selection after import": () => {
        resetTestState();
        prepareMediaNoteView("/path/to/input_video.mp4");
        setMediaNoteTranscriptLoadFailed("/path/to/input_video.mp4", "File not found", true);

        prepareMediaNoteView(null);
        let state = getProjectState();
        assert(state.isLoading === false, "isLoading should be false on clear");
        assert(state.isMediaNoteTranscriptLoading === false, "isMediaNoteTranscriptLoading should be false on clear");
        assert(state.selectedMediaNotePath === null, "selectedMediaNotePath should be null on clear");
        assert(state.statusMessage === "Media note selection cleared.", "Status message for clear selection incorrect"); // This one is correct (no pop)
        console.log("Test Case 1c: Passed");
    },

    "Test Case 3: Select existing media without notes": () => {
        resetTestState();
        const videoPath = "/path/to/existing_video_no_notes.mp4";
        prepareMediaNoteView(videoPath);
        setMediaNoteTranscriptLoadFailed(videoPath, "File not found", true);
        let state = getProjectState();
        assert(state.isLoading === false, "isLoading should be false");
        assert(state.isMediaNoteTranscriptLoading === false, "isMediaNoteTranscriptLoading should be false");
        assert(state.mediaNoteTranscriptError === "INFO:FILE_NOT_FOUND", `Error state mismatch: ${state.mediaNoteTranscriptError}`);
        assert(state.statusMessage === `No notes/transcription found for existing_video_no_notes.mp4.`, `Status message mismatch: ${state.statusMessage}`); // Added period
        console.log("Test Case 3: Passed");
    },

    "Test Case 3b: Select existing media with notes": () => {
        resetTestState();
        const videoPath = "/path/to/existing_video_with_notes.mp4";
        const noteContent = JSON.stringify({ root: { children: [{type: "paragraph", children: [{text: "Existing note"}]}] } });
        prepareMediaNoteView(videoPath);
        setLoadedMediaNoteTranscriptData(videoPath, noteContent);
        let state = getProjectState();
        assert(state.isLoading === false, "isLoading should be false");
        assert(state.isMediaNoteTranscriptLoading === false, "isMediaNoteTranscriptLoading should be false");
        assert(state.mediaNoteTranscriptError === null, `Error state should be null: ${state.mediaNoteTranscriptError}`);
        assert(state.currentMediaNoteTranscriptJson === noteContent, "Note content mismatch");
        assert(state.statusMessage === `Loaded notes for media: existing_video_with_notes.mp4.`, `Status message mismatch: ${state.statusMessage}`); // Added period
        console.log("Test Case 3b: Passed");
    },

    "Test Case Path Normalization in setLoaded/setFailed": () => {
        resetTestState();
        const winPath = "C:\\another\\video.mp4";
        const unixPath = "C:/another/video.mp4";

        prepareMediaNoteView(winPath); // Stores as unixPath
        let state = getProjectState();
        assert(state.selectedMediaNotePath === unixPath, "Path not normalized in prepareMediaNoteView");

        setLoadedMediaNoteTranscriptData(winPath, defaultEmptyJson);
        state = getProjectState();
        assert(state.isMediaNoteTranscriptLoading === false, "isMediaNoteTranscriptLoading not false after load with mixed path");
        assert(state.mediaNoteTranscriptError === null, "Error state not null after load with mixed path");
        assert(state.statusMessage === `Loaded notes for media: video.mp4.`);


        prepareMediaNoteView("C:\\third\\video.mp4");
        setMediaNoteTranscriptLoadFailed("C:\\third\\video.mp4", "Test error", false);
        state = getProjectState();
        assert(state.isMediaNoteTranscriptLoading === false, "isMediaNoteTranscriptLoading not false after fail with mixed path");
        assert(state.mediaNoteTranscriptError === "Failed to load notes: Test error", "Error message incorrect after fail with mixed path");
        assert(state.statusMessage === `Error loading notes for video.mp4.`);
        console.log("Test Case Path Normalization in setLoaded/setFailed: Passed");
    }
};

function assert(condition, message) {
    if (!condition) {
        throw new Error(`Assertion failed: ${message}`);
    }
}

// Run tests
let allTestsPassed = true;
for (const testName in tests) {
    try {
        tests[testName]();
    } catch (e) {
        console.error(`Test ${testName} Failed: ${e.message}`);
        allTestsPassed = false;
    }
}

if (allTestsPassed) {
    console.log("All projectStore unit tests passed!");
} else {
    console.error("Some projectStore unit tests failed.");
}
