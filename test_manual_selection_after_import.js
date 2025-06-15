// --- Mock Svelte Store ---
let currentProjectState;

const initialTestState = {
    name: "Test Project", xmlPath: "/fake/project.xml", baseDirectory: "/fake",
    files: [], // Will be populated by test setup
    documentFiles: [], tableFiles: [], imageFiles: [], importedTranscriptFiles: [], documentMetadataFiles: [],
    isLoading: false, error: null, statusMessage: 'Ready', // Start with a neutral state
    selectedMediaNotePath: null,
    currentMediaNoteTranscriptJson: null, initialMediaNoteTranscriptJson: null, isMediaNoteTranscriptDirty: false,
    isMediaNoteTranscriptLoading: false, mediaNoteTranscriptError: null, isImportingAsset: false,
    // Other states as needed
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
}

// --- Mock projectStore functions (copied from projectStore.js with normalization) ---
function prepareMediaNoteView(mediaPath) {
    const normalizedMediaPath = mediaPath ? mediaPath.replace(/\\/g, '/') : null;
    // console.debug(`[TestStore] prepareMediaNoteView called for mediaPath: ${mediaPath}, normalized to: ${normalizedMediaPath}`);
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
                isLoading: newIsMediaNoteLoading || p.isLoading, // p.isLoading might be true from another operation
            };
        }
        // If re-selecting the same path (and it's already loaded, this part might be different in actual store)
        return {
            ...p, ...otherFieldnotesStatesToClear,
            selectedMediaNotePath: normalizedMediaPath,
            statusMessage: `Viewing notes for media: ${normalizedMediaPath.split(/[\\/]/).pop()}`,
            // Ensure loading is re-triggered if somehow it was false but data is not there or path changed subtly
            isMediaNoteTranscriptLoading: p.selectedMediaNotePath !== normalizedMediaPath ? newIsMediaNoteLoading : (p.currentMediaNoteTranscriptJson === null ? newIsMediaNoteLoading : p.isMediaNoteTranscriptLoading),
        };
    });
    if (!normalizedMediaPath) {
        updateProjectState(p => ({ ...p, isMediaNoteTranscriptLoading: false, isLoading: false }));
    }
}

function setLoadedMediaNoteTranscriptData(mediaPath, jsonString) {
    const normalizedMediaPath = mediaPath ? mediaPath.replace(/\\/g, '/') : null;
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
                statusMessage: `Loaded notes for media: ${normalizedMediaPath.split(/[\\/]/).pop()}.`,
                isLoading: false,
            };
        }
        return p;
    });
}

function setMediaNoteTranscriptLoadFailed(mediaPath, errorMsg, isFileNotFound = false) {
    const normalizedMediaPath = mediaPath ? mediaPath.replace(/\\/g, '/') : null;
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

// --- Mock Tauri APIs ---
let mockInvokeResponses = {};
async function invoke(command, args) {
    // console.log(`Mock invoke: ${command}`, args);
    if (mockInvokeResponses[command] && args && mockInvokeResponses[command][args.filePath]) {
        const responseInstruction = mockInvokeResponses[command][args.filePath];
        if (responseInstruction.error) {
            return Promise.reject(new Error(responseInstruction.message || "File not found"));
        }
        return Promise.resolve(responseInstruction.data);
    }
    if (command === 'load_note_json') { // Default fallback for load_note_json
      return Promise.reject(new Error("File not found default mock"));
    }
    return Promise.resolve({});
}
async function basename(path) { return path ? path.split(/[\\/]/).pop() : ''; }


// --- Test Runner ---
const tests = {
    "Select Newly Imported Media (No Existing Notes)": async () => {
        const newlyImportedMediaName = "new_video.mp4";
        const newlyImportedMediaPath = `/fake/media_files/${newlyImportedMediaName}`;
        const notesPathForMedia = `/fake/transcripts/${newlyImportedMediaName.replace('.mp4', '.json')}`;

        resetTestState({
            files: [{ name: newlyImportedMediaName, path: newlyImportedMediaPath, file_type: "media" }],
            statusMessage: `${newlyImportedMediaName} imported successfully.`, // Simulate post-import state
            selectedMediaNotePath: null, // No media note selected yet
            isLoading: false,
            isImportingAsset: false,
        });

        // 1. Simulate user selecting the newly imported media
        prepareMediaNoteView(newlyImportedMediaPath);
        let state = getProjectState();
        assert(state.selectedMediaNotePath === newlyImportedMediaPath, "selectedMediaNotePath not set after prepare");
        assert(state.isLoading === true, "isLoading should be true after prepare");
        assert(state.isMediaNoteTranscriptLoading === true, "isMediaNoteTranscriptLoading should be true after prepare");
        assert(state.statusMessage === `Loading notes for media: ${newlyImportedMediaName}`, `Status message mismatch after prepare: ${state.statusMessage}`);

        // 2. Simulate MediaEditorPanel attempting to load notes (and failing)
        mockInvokeResponses['load_note_json'] = {
            [notesPathForMedia]: { error: true, message: "File not found" }
        };
        // This call would normally be made by MediaEditorPanel.svelte after deriving notesPathForMedia
        // Here we directly call what MediaEditorPanel would trigger in projectStore
        setMediaNoteTranscriptLoadFailed(newlyImportedMediaPath, "File not found", true);
        state = getProjectState();

        assert(state.isLoading === false, "isLoading should be false after load failed");
        assert(state.isMediaNoteTranscriptLoading === false, "isMediaNoteTranscriptLoading should be false after load failed");
        assert(state.mediaNoteTranscriptError === "INFO:FILE_NOT_FOUND", `Error state mismatch: ${state.mediaNoteTranscriptError}`);
        assert(state.statusMessage === `No notes/transcription found for ${newlyImportedMediaName}.`, `Final status message mismatch: ${state.statusMessage}`);
        console.log("Select Newly Imported Media (No Existing Notes): Passed");
    },

    "Select Existing Media (With Notes - Regression)": async () => {
        const existingMediaName = "existing_video.mp4";
        const existingMediaPath = `/fake/media_files/${existingMediaName}`;
        const notesPathForExistingMedia = `/fake/transcripts/${existingMediaName.replace('.mp4', '.json')}`;
        const noteContent = JSON.stringify({ root: { children: [{type: "paragraph", children: [{text: "Existing note"}]}] } });

        resetTestState({
            files: [{ name: existingMediaName, path: existingMediaPath, file_type: "media" }],
            selectedMediaNotePath: null,
            isLoading: false,
        });

        // 1. Simulate user selecting the existing media
        prepareMediaNoteView(existingMediaPath);
        let state = getProjectState();
        assert(state.selectedMediaNotePath === existingMediaPath, "selectedMediaNotePath not set (existing media)");
        assert(state.isLoading === true, "isLoading not true after prepare (existing media)");
        assert(state.isMediaNoteTranscriptLoading === true, "isMediaNoteTranscriptLoading not true after prepare (existing media)");

        // 2. Simulate MediaEditorPanel loading notes successfully
        mockInvokeResponses['load_note_json'] = {
            [notesPathForExistingMedia]: { data: noteContent }
        };
        // Similar to above, this is simulating the consequence of the load
        setLoadedMediaNoteTranscriptData(existingMediaPath, noteContent);
        state = getProjectState();

        assert(state.isLoading === false, "isLoading not false after load success (existing media)");
        assert(state.isMediaNoteTranscriptLoading === false, "isMediaNoteTranscriptLoading not false (existing media)");
        assert(state.currentMediaNoteTranscriptJson === noteContent, "Note content mismatch (existing media)");
        assert(state.mediaNoteTranscriptError === null, `Error state not null (existing media): ${state.mediaNoteTranscriptError}`);
        assert(state.statusMessage === `Loaded notes for media: ${existingMediaName}.`, `Final status (existing media): ${state.statusMessage}`);
        console.log("Select Existing Media (With Notes - Regression): Passed");
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
            resetTestState(); // Reset state before each test
            await tests[testName]();
        } catch (e) {
            console.error(`Test ${testName} Failed: ${e.message}\n${e.stack}`);
            allTestsPassed = false;
        }
    }

    if (allTestsPassed) {
        console.log("All manual selection after import tests passed!");
    } else {
        console.error("Some manual selection after import tests failed.");
    }
    mockInvokeResponses = {}; // Reset for any subsequent test runs in a larger suite
}

runTests();
