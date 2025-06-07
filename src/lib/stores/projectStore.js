// src/lib/stores/projectStore.js
import { writable, get } from 'svelte/store';
import { listen } from '@tauri-apps/api/event'; // Added for media_renamed event

const HARVEY_FILES_DIR = "harvey_files";
const MEDIA_DIR_NAME = 'Media';
const MEDIA_SUBDIR = 'media';
const TRANSCRIPTS_SUBDIR = 'transcripts';

const defaultEmptyJson = JSON.stringify({
    root: {
        children: [{ children: [], direction: null, format: '', indent: 0, type: 'paragraph', version: 1 }],
        direction: null, format: '', indent: 0, type: 'root', version: 1
    }
});

const createMinimalValidLexicalJson = () => {
    return JSON.stringify({
        root: {
            children: [{ type: 'paragraph', version: 1, children: [], direction: null, format: '', indent: 0 }],
            direction: null, format: '', indent: 0, type: 'root', version: 1
        }
    });
};


const initialState = {
    name: null,
    xmlPath: null,
    baseDirectory: null,
    files: [],
    documentFiles: [],
    tableFiles: [],
    imageFiles: [],
    importedTranscriptFiles: [],
    documentMetadataFiles: [],
    isLoading: true,
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

    currentDocumentFileLevelMetadata: {
        file_name: '', last_modified: '', title: '', description: '', summary: '',
    },
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
    isMediaNoteTranscriptDirty: false, // Key change: ensure this is false if file not found
    isMediaNoteTranscriptLoading: false,
    mediaNoteTranscriptError: null,
    activeMediaNoteEditorRef: null,

    autosaveEnabled: true,
    // transcriptUndoStack: [], // Moved to transcriptStore
    // transcriptRedoStack: [], // Moved to transcriptStore

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

export const project = writable({ ...initialState });

export const updateProjectStoreState = (newState) => project.update(s => ({...s, ...newState}));

export function prepareDocumentView(filePath, itemType = 'document') {
    console.log(`[ProjectStore] prepareDocumentView called for path: ${filePath}, type: ${itemType}`);
    const isPdf = filePath ? filePath.toLowerCase().endsWith('.pdf') : false;
    const isTable = itemType === 'tables';
    const isImage = itemType === 'images';
    const isJsonDocument = filePath && itemType === 'documents' && !isPdf;

    const defaultFileLevelMetadata = {
        file_name: '', last_modified: '', title: '', description: '', summary: '',
    };

    project.update(p => {
        const selectingSamePath = p.selectedDocumentPath === filePath;
        const newIsDocumentLoading = isJsonDocument && (!selectingSamePath || !p.currentDocumentJson) ||
                                   isPdf && (!selectingSamePath || !p.currentPdfAnnotations || (p.currentPdfAnnotations.length === 0 && !p.initialPdfAnnotations) );

        return {
            ...p,
            selectedDocumentPath: filePath,
            currentDocumentJson: (isJsonDocument && selectingSamePath) ? p.currentDocumentJson : (isJsonDocument ? null : null),
            initialDocumentJson: (isJsonDocument && selectingSamePath) ? p.initialDocumentJson : (isJsonDocument ? null : null),
            isDocumentDirty: (isJsonDocument && selectingSamePath) ? p.isDocumentDirty : false,
            activeDocumentEditorRef: (isJsonDocument && selectingSamePath) ? p.activeDocumentEditorRef : null,
            currentDocumentFileLevelMetadata: (isJsonDocument && selectingSamePath) ? p.currentDocumentFileLevelMetadata : { ...defaultFileLevelMetadata },
            currentDocumentHighlights: (isJsonDocument && selectingSamePath) ? p.currentDocumentHighlights : [],
            isDocumentMetadataDirty: (isJsonDocument && selectingSamePath) ? p.isDocumentMetadataDirty : false,
            currentPdfAnnotations: (isPdf && selectingSamePath) ? p.currentPdfAnnotations : [],
            initialPdfAnnotations: (isPdf && selectingSamePath) ? p.initialPdfAnnotations : [],
            isPdfAnnotationsDirty: (isPdf && selectingSamePath) ? p.isPdfAnnotationsDirty : false,
            isDocumentLoading: newIsDocumentLoading,
            documentError: null,
            statusMessage: filePath ? `Loading ${itemType}: ${filePath.split(/[\\/]/).pop()}` : `${itemType.charAt(0).toUpperCase() + itemType.slice(1)} selection cleared.`,
            isLoading: newIsDocumentLoading || p.isLoading, // If specific document starts loading, global isLoading should reflect this or stay true

            currentImportedTranscriptPath: null,
            currentImportedTranscriptLexicalJson: null,
            initialImportedTranscriptLexicalJson: null,
            isImportedTranscriptDirty: false,
            activeImportedTranscriptEditorRef: null,
            importedTranscriptError: null,
            isImportedTranscriptLoading: false,

            selectedMediaNotePath: null,
            currentMediaNoteTranscriptJson: null,
            initialMediaNoteTranscriptJson: null,
            isMediaNoteTranscriptDirty: false,
            mediaNoteTranscriptError: null,
            isMediaNoteTranscriptLoading: false,
            activeMediaNoteEditorRef: null,
        };
    });

    if (isJsonDocument && filePath) {
        import('$lib/services/projectService.js').then(async service => {
            if (service.loadActiveDocumentContent) await service.loadActiveDocumentContent();
            else { console.error("[ProjectStore] loadActiveDocumentContent not found."); project.update(p => { if(p.selectedDocumentPath === filePath) return ({ ...p, isDocumentLoading: false, documentError: "Internal error."}); return p; });}
            if (service.loadDocumentMetadata) { try { const meta = await service.loadDocumentMetadata(filePath); project.update(p => p.selectedDocumentPath === filePath && !isPdf ? { ...p, currentDocumentFileLevelMetadata: meta?.metadata || defaultFileLevelMetadata, currentDocumentHighlights: meta?.highlights || [], isDocumentMetadataDirty: false } : p); } catch (e) { project.update(p => p.selectedDocumentPath === filePath && !isPdf ? { ...p, documentError: (p.documentError || '') + ` Meta load failed.` } : p);}}
        }).catch(err => project.update(p => { if(p.selectedDocumentPath === filePath) return ({ ...p, isDocumentLoading: false, documentError: "Internal error."}); return p; }));
    } else if (isPdf && filePath) {
         import('$lib/services/projectService.js').then(async service => {
            if (service.loadPdfAnnotationsFromFile) await service.loadPdfAnnotationsFromFile(filePath);
            else { console.error("[ProjectStore] loadPdfAnnotationsFromFile not found."); project.update(p => {if(p.selectedDocumentPath === filePath) return ({ ...p, isDocumentLoading: false, documentError: "Internal error."}); return p;});}
         }).catch(err => project.update(p => {if(p.selectedDocumentPath === filePath) return ({ ...p, isDocumentLoading: false, documentError: "Internal error."}); return p; }));
    } else if (filePath && (isTable || isImage)) {
         project.update(p => ({ ...p, isDocumentLoading: false, isLoading: false })); // No specific loading, so turn off global too
    } else if (!filePath) {
         project.update(p => ({ ...p, isDocumentLoading: false, isLoading: false }));
    }
}
export function setLoadedDocumentData(filePath, jsonContent) { console.log(`[ProjectStore] Setting loaded document data (JSON) for: ${filePath}`); project.update(p => { if (p.selectedDocumentPath === filePath && !filePath.toLowerCase().endsWith('.pdf') ) { return { ...p, currentDocumentJson: jsonContent || defaultEmptyJson, initialDocumentJson: jsonContent || defaultEmptyJson, isDocumentDirty: false, isDocumentLoading: false, documentError: null, statusMessage: `Loaded document: ${filePath.split(/[\\/]/).pop()}`, isLoading: false }; } else { if(p.isDocumentLoading && p.selectedDocumentPath === filePath) { return { ...p, isDocumentLoading: false, isLoading: false }; } return p; } }); }
export function setDocumentLoadFailed(filePath, errorMsg) { console.error(`[ProjectStore] Document load failed for: ${filePath}`, errorMsg); project.update(p => { if (p.selectedDocumentPath === filePath && !filePath.toLowerCase().endsWith('.pdf') ) { return { ...p, currentDocumentJson: null, initialDocumentJson: null, isDocumentDirty: false, isDocumentLoading: false, activeDocumentEditorRef: null, documentError: `Failed to load document: ${errorMsg}`, statusMessage: `Error loading ${filePath.split(/[\\/]/).pop()}.`, currentDocumentFileLevelMetadata: { file_name: '', last_modified: '', title: '', description: '', summary: '' }, currentDocumentHighlights: [], isDocumentMetadataDirty: false, isLoading: false }; } else if (p.isDocumentLoading && p.selectedDocumentPath === filePath) { return { ...p, isDocumentLoading: false, isLoading: false }; } return p; }); }
export function setDocumentEditorContent(newJsonContent) { project.update(p => { if (p.selectedDocumentPath && !p.selectedDocumentPath.toLowerCase().endsWith('.pdf') ) { const initial = p.initialDocumentJson; const current = p.currentDocumentJson; const isNewDifferentFromInitial = initial !== newJsonContent; const newDirtyState = isNewDifferentFromInitial; if (current !== newJsonContent || p.isDocumentDirty !== newDirtyState) { return { ...p, currentDocumentJson: newJsonContent, isDocumentDirty: newDirtyState, }; } } return p; }); }
export function markDocumentAsSaved(savedJsonContent) { console.log('[ProjectStore] Marking document as saved (JSON).'); project.update(p => { if (p.selectedDocumentPath && !p.selectedDocumentPath.toLowerCase().endsWith('.pdf') ) { return { ...p, initialDocumentJson: savedJsonContent, currentDocumentJson: savedJsonContent, isDocumentDirty: false, statusMessage: `Document saved: ${p.selectedDocumentPath?.split(/[\\/]/).pop()}` }; } return p; }); }
export function markDocumentChangesDiscarded() { console.log('[ProjectStore] Marking document changes as discarded.'); project.update(p => { if (p.selectedDocumentPath) { const isPdf = p.selectedDocumentPath.toLowerCase().endsWith('.pdf'); return { ...p, currentDocumentJson: isPdf ? p.currentDocumentJson : p.initialDocumentJson, isDocumentDirty: isPdf ? p.isDocumentDirty : false, statusMessage: 'Document changes discarded.', currentDocumentFileLevelMetadata: p.currentDocumentFileLevelMetadata, currentDocumentHighlights: (isPdf || p.isDocumentMetadataDirty) ? [] : p.currentDocumentHighlights, isDocumentMetadataDirty: false, currentPdfAnnotations: isPdf ? (p.initialPdfAnnotations || []) : p.currentPdfAnnotations, isPdfAnnotationsDirty: false, }; } return p; }); }
export function clearDocumentEditorState() { console.log('[ProjectStore] Clearing document editor state.'); project.update(p => ({ ...p, selectedDocumentPath: null, currentDocumentJson: null, initialDocumentJson: null, isDocumentDirty: false, isDocumentLoading: false, documentError: null, activeDocumentEditorRef: null, currentDocumentFileLevelMetadata: { file_name: '', last_modified: '', title: '', description: '', summary: '' }, currentDocumentHighlights: [], isDocumentMetadataDirty: false, currentPdfAnnotations: [], initialPdfAnnotations: [], isPdfAnnotationsDirty: false })); }
export function setActiveDocumentEditorRef(editorInstance) { project.update(p => ({ ...p, activeDocumentEditorRef: editorInstance })); }
export function clearActiveDocumentEditorRef() { project.update(p => ({ ...p, activeDocumentEditorRef: null })); }
export function updateDocumentHighlights(newHighlightEvent) { const currentPath = get(project).selectedDocumentPath; if (currentPath && currentPath.toLowerCase().endsWith('.pdf')) { updatePdfAnnotations(newHighlightEvent); return; } project.update(p => { if (!p.selectedDocumentPath || p.selectedDocumentPath.toLowerCase().endsWith('.pdf')) { return p; } let highlights = JSON.parse(JSON.stringify(p.currentDocumentHighlights || [])); const { type, id, text, nodeKey, color } = newHighlightEvent; if (type === 'add') { if (!nodeKey) { console.warn("[ProjectStore updateDocumentHighlights] 'add' event missing nodeKey for Lexical doc."); return p; } const existingIndex = highlights.findIndex(h => h.id === id); const newHighlightData = { id, text, nodeKey, color: color || 'transparent', codes: [], comments: [], timestamp: new Date().toISOString() }; if (existingIndex === -1) highlights.push(newHighlightData); else highlights[existingIndex] = { ...newHighlightData, codes: highlights[existingIndex].codes || [], comments: highlights[existingIndex].comments || [] }; console.log(`[ProjectStore] Lexical Highlight ADDED/UPDATED: ID=${id}, NodeKey=${nodeKey}`); } else if (type === 'remove') { highlights = highlights.filter(h => h.id !== id); console.log(`[ProjectStore] Lexical Highlight REMOVED: ID=${id}`); } else if (type === 'update') { if (!nodeKey) { console.warn("[ProjectStore updateDocumentHighlights] 'update' event missing nodeKey for Lexical doc."); return p; } const existingIndex = highlights.findIndex(h => h.id === id); if (existingIndex !== -1) { highlights[existingIndex] = { ...highlights[existingIndex], text, nodeKey, color: color || highlights[existingIndex].color, timestamp: new Date().toISOString() }; console.log(`[ProjectStore] Lexical Highlight UPDATED: ID=${id}`); } } return { ...p, currentDocumentHighlights: highlights, isDocumentMetadataDirty: true }; }); }
export function markDocumentMetadataAsSaved(updatedFileLevelMetadata) { console.log('[ProjectStore] Marking Lexical document metadata as saved.'); project.update(p => { if (p.selectedDocumentPath && !p.selectedDocumentPath.toLowerCase().endsWith('.pdf')) { return { ...p, isDocumentMetadataDirty: false, currentDocumentFileLevelMetadata: updatedFileLevelMetadata ? { ...p.currentDocumentFileLevelMetadata, ...updatedFileLevelMetadata } : p.currentDocumentFileLevelMetadata }; } return p; }); }
export function updatePdfAnnotations(pdfHighlightEvent) { project.update(p => { if (!p.selectedDocumentPath || !p.selectedDocumentPath.toLowerCase().endsWith('.pdf')) { return p; } let annotations = Array.isArray(p.currentPdfAnnotations) ? JSON.parse(JSON.stringify(p.currentPdfAnnotations)) : []; let { type, id, ...highlightData } = pdfHighlightEvent; if (!type || type === 'pdfHighlight') type = 'add'; let annotationChanged = false; if (type === 'add') { const existingIndex = annotations.findIndex(h => h.id === id); const newAnnotation = { id, ...highlightData, timestamp: new Date().toISOString() }; if (existingIndex === -1) { annotations.push(newAnnotation); annotationChanged = true; } else { if (JSON.stringify(annotations[existingIndex]) !== JSON.stringify({ ...annotations[existingIndex], ...newAnnotation })) { annotations[existingIndex] = { ...annotations[existingIndex], ...newAnnotation }; annotationChanged = true; } } if(annotationChanged) console.log(`[ProjectStore] PDF Annotation ADDED/UPDATED: ID=${id}`); } else if (type === 'remove') { const initialLength = annotations.length; annotations = annotations.filter(h => h.id !== id); if (annotations.length < initialLength) { annotationChanged = true; console.log(`[ProjectStore] PDF Annotation REMOVED: ID=${id}`); } } else if (type === 'update') { const existingIndex = annotations.findIndex(h => h.id === id); if (existingIndex !== -1) { if (JSON.stringify(annotations[existingIndex]) !== JSON.stringify({ ...annotations[existingIndex], ...highlightData, timestamp: new Date().toISOString() })) { annotations[existingIndex] = { ...annotations[existingIndex], ...highlightData, timestamp: new Date().toISOString() }; annotationChanged = true; console.log(`[ProjectStore] PDF Annotation UPDATED: ID=${id}`); } } } if (annotationChanged) { return { ...p, currentPdfAnnotations: annotations, isPdfAnnotationsDirty: true, isDocumentDirty: true }; } return p; }); }
export function markPdfAnnotationsDirty(updatedAnnotations = null) { project.update(p => { if (p.selectedDocumentPath && p.selectedDocumentPath.toLowerCase().endsWith('.pdf')) { return { ...p, isPdfAnnotationsDirty: true, isDocumentDirty: false, currentPdfAnnotations: updatedAnnotations !== null ? updatedAnnotations : p.currentPdfAnnotations }; } return p; }); }
export function markPdfAnnotationsAsSaved() { console.log('[ProjectStore] Marking PDF annotations as saved.'); project.update(p => { if (p.selectedDocumentPath && p.selectedDocumentPath.toLowerCase().endsWith('.pdf')) { return { ...p, isPdfAnnotationsDirty: false, isDocumentDirty: false, initialPdfAnnotations: JSON.parse(JSON.stringify(p.currentPdfAnnotations)), statusMessage: 'PDF annotations saved.' }; } return p; }); }
export function setLoadedPdfAnnotations(annotationsArray) { console.log(`[ProjectStore] Setting loaded PDF annotations. Count: ${annotationsArray?.length || 0}`); project.update(p => ({ ...p, currentPdfAnnotations: Array.isArray(annotationsArray) ? annotationsArray : [], initialPdfAnnotations: Array.isArray(annotationsArray) ? JSON.parse(JSON.stringify(annotationsArray)) : [], isPdfAnnotationsDirty: false, isDocumentLoading: false, isLoading: false }));}
export function setPdfAnnotationsLoadFailed(filePath, errorMsg) { console.error(`[ProjectStore] PDF annotations load failed for: ${filePath}`, errorMsg); project.update(p => { if (p.selectedDocumentPath === filePath && filePath.toLowerCase().endsWith('.pdf')) { return { ...p, currentPdfAnnotations: [], initialPdfAnnotations: [], isPdfAnnotationsDirty: false, isDocumentLoading: false, documentError: (p.documentError ? p.documentError + "; " : "") + `Failed to load PDF annotations: ${errorMsg}`, statusMessage: `Error loading PDF annotations for ${filePath.split(/[\\/]/).pop()}.`, isLoading: false }; } if (p.isDocumentLoading && p.selectedDocumentPath !== filePath && filePath.toLowerCase().endsWith('.pdf')){ console.warn(`[ProjectStore setPdfAnnotationsLoadFailed] Error for non-selected but previously loading PDF ${filePath}. Clearing general document loading.`); return { ...p, isDocumentLoading: false, isLoading:false }; } return p; }); }

export function prepareImportedTranscriptView(filePath) {
    console.log(`[ProjectStore] prepareImportedTranscriptView called for path: ${filePath}`);
    const newIsLoading = !!filePath;
    project.update(p => ({
        ...p,
        currentImportedTranscriptPath: filePath,
        currentImportedTranscriptLexicalJson: p.currentImportedTranscriptPath === filePath ? p.currentImportedTranscriptLexicalJson : null,
        initialImportedTranscriptLexicalJson: p.currentImportedTranscriptPath === filePath ? p.initialImportedTranscriptLexicalJson : null,
        isImportedTranscriptDirty: p.currentImportedTranscriptPath === filePath ? p.isImportedTranscriptDirty : false,
        isImportedTranscriptLoading: newIsLoading,
        importedTranscriptError: null,
        activeImportedTranscriptEditorRef: p.currentImportedTranscriptPath === filePath ? p.activeImportedTranscriptEditorRef : null,
        statusMessage: filePath ? `Loading imported transcript: ${filePath.split(/[\\/]/).pop()}` : 'Imported transcript selection cleared.',
        isLoading: newIsLoading || p.isLoading,

        selectedDocumentPath: null,
        currentDocumentJson: null, initialDocumentJson: null, isDocumentDirty: false, isDocumentLoading: false, documentError: null, activeDocumentEditorRef: null,
        currentDocumentFileLevelMetadata: { file_name: '', last_modified: '', title: '', description: '', summary: '' },
        currentDocumentHighlights: [], isDocumentMetadataDirty: false,
        currentPdfAnnotations: [], initialPdfAnnotations: [], isPdfAnnotationsDirty: false,

        selectedMediaNotePath: null,
        currentMediaNoteTranscriptJson: null,
        initialMediaNoteTranscriptJson: null,
        isMediaNoteTranscriptDirty: false,
        mediaNoteTranscriptError: null,
        isMediaNoteTranscriptLoading: false,
        activeMediaNoteEditorRef: null,
    }));
    if (!filePath) {
        project.update(p => ({ ...p, isImportedTranscriptLoading: false, isLoading: false }));
    }
}
export function setLoadedImportedTranscriptData(filePath, lexicalJsonContent) { console.log(`[ProjectStore] Setting loaded data for imported transcript: ${filePath}`); const minimalValidJson = createMinimalValidLexicalJson(); project.update(p => { if (p.currentImportedTranscriptPath === filePath) { const isValid = lexicalJsonContent && typeof lexicalJsonContent === 'string' && lexicalJsonContent.length > 2; return { ...p, currentImportedTranscriptLexicalJson: isValid ? lexicalJsonContent : minimalValidJson, initialImportedTranscriptLexicalJson: isValid ? lexicalJsonContent : minimalValidJson, isImportedTranscriptDirty: false, isImportedTranscriptLoading: false, importedTranscriptError: isValid ? null : "Loaded content was invalid, showing empty editor.", statusMessage: `Loaded imported transcript: ${filePath.split(/[\\/]/).pop()}`, isLoading: false }; } else { if (p.isImportedTranscriptLoading && p.currentImportedTranscriptPath === filePath) { return { ...p, isImportedTranscriptLoading: false, isLoading: false }; } return p; } }); }
export function setImportedTranscriptLoadFailed(filePath, errorMsg) { console.error(`[ProjectStore] Imported transcript load failed for: ${filePath}`, errorMsg); project.update(p => { if (p.currentImportedTranscriptPath === filePath) { return { ...p, currentImportedTranscriptLexicalJson: createMinimalValidLexicalJson(), initialImportedTranscriptLexicalJson: createMinimalValidLexicalJson(), isImportedTranscriptDirty: false, isImportedTranscriptLoading: false, importedTranscriptError: `Failed to load transcript: ${errorMsg}`, statusMessage: `Error loading imported transcript ${filePath.split(/[\\/]/).pop()}.`, activeImportedTranscriptEditorRef: null, isLoading: false }; } else if (p.isImportedTranscriptLoading && p.currentImportedTranscriptPath === filePath) { return { ...p, isImportedTranscriptLoading: false, isLoading: false }; } return p; }); }
export function setImportedTranscriptEditorContent(filePath, newLexicalJsonContent) { project.update(p => { if (p.currentImportedTranscriptPath === filePath) { const initial = p.initialImportedTranscriptLexicalJson; const current = p.currentImportedTranscriptLexicalJson; const isNewDifferentFromInitial = initial !== newLexicalJsonContent; const newDirtyState = isNewDifferentFromInitial; if (current !== newLexicalJsonContent || p.isImportedTranscriptDirty !== newDirtyState) { return { ...p, currentImportedTranscriptLexicalJson: newLexicalJsonContent, isImportedTranscriptDirty: newDirtyState, }; } } return p; }); }
export function markImportedTranscriptAsSaved(filePath, savedLexicalJsonContent) { console.log(`[ProjectStore] Marking imported transcript as saved: ${filePath}`); project.update(p => { if (p.currentImportedTranscriptPath === filePath) { return { ...p, initialImportedTranscriptLexicalJson: savedLexicalJsonContent, currentImportedTranscriptLexicalJson: savedLexicalJsonContent, isImportedTranscriptDirty: false, statusMessage: `Imported transcript saved: ${filePath.split(/[\\/]/).pop()}` }; } return p; }); }
export function markImportedTranscriptChangesDiscarded(filePath) { console.log(`[ProjectStore] Marking imported transcript changes as discarded: ${filePath}`); project.update(p => { if (p.currentImportedTranscriptPath === filePath) { return { ...p, currentImportedTranscriptLexicalJson: p.initialImportedTranscriptLexicalJson, isImportedTranscriptDirty: false, statusMessage: 'Imported transcript changes discarded.'}; } return p; }); }
export function setActiveImportedTranscriptEditorRef(editorInstance) { project.update(p => ({ ...p, activeImportedTranscriptEditorRef: editorInstance })); }
export function clearActiveImportedTranscriptEditorRef() { project.update(p => ({ ...p, activeImportedTranscriptEditorRef: null })); }

export function prepareMediaNoteView(mediaPath) {
    console.log(`[ProjectStore] prepareMediaNoteView called for mediaPath: ${mediaPath}`);
    const newIsMediaNoteLoading = !!mediaPath;
    project.update(p => {
        const otherFieldnotesStatesToClear = {
            selectedDocumentPath: null,
            currentDocumentJson: null, initialDocumentJson: null, isDocumentDirty: false,
            isDocumentLoading: false, documentError: null, activeDocumentEditorRef: null,
            currentDocumentFileLevelMetadata: { file_name: '', last_modified: '', title: '', description: '', summary: '' },
            currentDocumentHighlights: [], isDocumentMetadataDirty: false,
            currentPdfAnnotations: [], initialPdfAnnotations: [], isPdfAnnotationsDirty: false,

            currentImportedTranscriptPath: null,
            currentImportedTranscriptLexicalJson: null, initialImportedTranscriptLexicalJson: null,
            isImportedTranscriptDirty: false, isImportedTranscriptLoading: false,
            importedTranscriptError: null, activeImportedTranscriptEditorRef: null,
        };

        if (p.selectedMediaNotePath !== mediaPath || !p.selectedMediaNotePath) {
            return {
                ...p,
                ...otherFieldnotesStatesToClear,
                selectedMediaNotePath: mediaPath,
                isMediaNoteTranscriptLoading: newIsMediaNoteLoading,
                mediaNoteTranscriptError: null,
                isMediaNoteTranscriptDirty: false, // Explicitly false when preparing a new/different view
                currentMediaNoteTranscriptJson: null,
                initialMediaNoteTranscriptJson: null,
                activeMediaNoteEditorRef: null,
                statusMessage: mediaPath ? `Loading notes for media: ${mediaPath.split(/[\\/]/).pop()}` : 'Media note selection cleared.',
                isLoading: newIsMediaNoteLoading || p.isLoading,
            };
        }
        // If re-selecting the same, just ensure other fieldnotes are clear
        return {
            ...p,
            ...otherFieldnotesStatesToClear,
            selectedMediaNotePath: mediaPath,
            statusMessage: `Viewing notes for media: ${mediaPath.split(/[\\/]/).pop()}`,
            isMediaNoteTranscriptLoading: p.selectedMediaNotePath !== mediaPath ? newIsMediaNoteLoading : p.isMediaNoteTranscriptLoading, // Re-trigger loading if path changed
        };
    });
    if (!mediaPath) {
        project.update(p => ({ ...p, isMediaNoteTranscriptLoading: false, isLoading: false }));
    }
}

export function setLoadedMediaNoteTranscriptData(mediaPath, jsonString) {
    console.log(`[ProjectStore] Setting loaded media note transcript data for media: ${mediaPath}`);
    project.update(p => {
        if (p.selectedMediaNotePath === mediaPath) {
            const content = jsonString || defaultEmptyJson;
            return {
                ...p,
                currentMediaNoteTranscriptJson: content,
                initialMediaNoteTranscriptJson: content,
                isMediaNoteTranscriptDirty: false, // Not dirty on fresh load
                isMediaNoteTranscriptLoading: false,
                mediaNoteTranscriptError: null,
                statusMessage: `Loaded notes for media: ${mediaPath.split(/[\\/]/).pop()}`,
                isLoading: false, // Turn off general loading indicator
            };
        }
        return p;
    });
}

export function setMediaNoteTranscriptLoadFailed(mediaPath, errorMsg, isFileNotFound = false) {
    console.error(`[ProjectStore] Media note transcript load failed for media: ${mediaPath}`, errorMsg);
    project.update(p => {
        if (p.selectedMediaNotePath === mediaPath) {
            return {
                ...p,
                currentMediaNoteTranscriptJson: defaultEmptyJson,
                initialMediaNoteTranscriptJson: defaultEmptyJson,
                isMediaNoteTranscriptDirty: false, // Not dirty if load failed or file not found
                isMediaNoteTranscriptLoading: false,
                mediaNoteTranscriptError: isFileNotFound ? "INFO:FILE_NOT_FOUND" : `Failed to load notes: ${errorMsg}`,
                statusMessage: isFileNotFound ? `No notes/transcription found for ${mediaPath.split(/[\\/]/).pop()}.` : `Error loading notes for ${mediaPath.split(/[\\/]/).pop()}.`,
                activeMediaNoteEditorRef: null,
                isLoading: false, // Turn off general loading indicator
            };
        }
        return p;
    });
}

export function setMediaNoteTranscriptEditorContent(mediaPath, newJsonContent) {
    project.update(p => {
        if (p.selectedMediaNotePath === mediaPath) {
            const initial = p.initialMediaNoteTranscriptJson;
            const current = p.currentMediaNoteTranscriptJson;
            // A new file (initially defaultEmptyJson) becomes dirty as soon as user types *anything* different.
            const isNewDifferentFromInitial = initial !== newJsonContent;
            const newDirtyState = isNewDifferentFromInitial;

            if (current !== newJsonContent || p.isMediaNoteTranscriptDirty !== newDirtyState) {
                return {
                    ...p,
                    currentMediaNoteTranscriptJson: newJsonContent,
                    isMediaNoteTranscriptDirty: newDirtyState,
                };
            }
        }
        return p;
    });
}

export function markMediaNoteTranscriptAsSaved(mediaPath, savedJsonContent) {
    console.log(`[ProjectStore] Marking media note transcript as saved for media: ${mediaPath}`);
    project.update(p => {
        if (p.selectedMediaNotePath === mediaPath) {
            return {
                ...p,
                initialMediaNoteTranscriptJson: savedJsonContent,
                currentMediaNoteTranscriptJson: savedJsonContent,
                isMediaNoteTranscriptDirty: false,
                mediaNoteTranscriptError: null,
                statusMessage: `Notes for media ${mediaPath.split(/[\\/]/).pop()} saved.`,
            };
        }
        return p;
    });
}

export function markMediaNoteTranscriptChangesDiscarded(mediaPath) {
    console.log(`[ProjectStore] Marking media note transcript changes as discarded for media: ${mediaPath}`);
    project.update(p => {
        if (p.selectedMediaNotePath === mediaPath) {
            // If the initial state was "file not found", discarding means resetting to that state.
            const errorToKeep = p.initialMediaNoteTranscriptJson === defaultEmptyJson && p.mediaNoteTranscriptError === "INFO:FILE_NOT_FOUND"
                ? "INFO:FILE_NOT_FOUND"
                : null;
            const statusToKeep = errorToKeep === "INFO:FILE_NOT_FOUND"
                ? `No notes/transcription found for ${mediaPath.split(/[\\/]/).pop()}.`
                : `Changes to notes for media ${mediaPath.split(/[\\/]/).pop()} discarded.`;

            return {
                ...p,
                currentMediaNoteTranscriptJson: p.initialMediaNoteTranscriptJson, // Revert to initial content (which might be defaultEmptyJson)
                isMediaNoteTranscriptDirty: false,
                mediaNoteTranscriptError: errorToKeep,
                statusMessage: statusToKeep,
            };
        }
        return p;
    });
}

export function setActiveMediaNoteEditorRef(mediaPath, editorRefInstance) {
    project.update(p => {
        if (p.selectedMediaNotePath === mediaPath) {
            return { ...p, activeMediaNoteEditorRef: { path: mediaPath, ref: editorRefInstance } };
        }
        if (p.activeMediaNoteEditorRef && p.activeMediaNoteEditorRef.path !== mediaPath) {}
        return p;
    });
}

export function clearActiveMediaNoteEditorRef() {
    project.update(p => {
        if (p.activeMediaNoteEditorRef) {
            return { ...p, activeMediaNoteEditorRef: null };
        }
        return p;
    });
}

export function toggleAutosave() { project.update(p => { const newState = !p.autosaveEnabled; console.log(`[ProjectStore] Toggling autosave to: ${newState}`); return { ...p, autosaveEnabled: newState, statusMessage: `Autosave ${newState ? 'enabled' : 'disabled'}` }; }); }
export function showUnsavedChangesPrompt(itemName, itemType, onSave, onDiscard, onCancel) { console.log(`[ProjectStore] Showing unsaved changes prompt for: ${itemName} (type: ${itemType})`); project.update(p => ({ ...p, showUnsavedChangesModal: true, unsavedItemName: itemName, unsavedItemType: itemType, onUnsavedSave: onSave, onUnsavedDiscard: onDiscard, onUnsavedCancel: onCancel, })); }
export function hideUnsavedChangesPrompt() { console.log('[ProjectStore] Hiding unsaved changes prompt.'); project.update(p => ({ ...p, showUnsavedChangesModal: false, unsavedItemName: '', unsavedItemType: '', onUnsavedSave: () => {}, onUnsavedDiscard: () => {}, onUnsavedCancel: () => {}, })); }
export function setAssetImportStatus(isImporting, message = null) { project.update(p => ({ ...p, isImportingAsset: isImporting, statusMessage: message !== null ? message : (isImporting ? 'Importing...' : p.statusMessage), error: isImporting ? null : p.error, documentError: isImporting ? null : p.documentError, importedTranscriptError: isImporting ? null : p.importedTranscriptError, isLoading: isImporting })); } // isLoading also true during import
export function showConversionPrompt(fileName, onConfirm, onCancel) { console.log(`[ProjectStore] Showing conversion prompt for: ${fileName}`); project.update(p => ({ ...p, showConfirmConversionModal: true, conversionFileName: fileName, onConversionConfirm: onConfirm, onConversionCancel: onCancel, })); }
export function hideConversionPrompt() { console.log('[ProjectStore] Hiding conversion prompt.'); project.update(p => ({ ...p, showConfirmConversionModal: false, conversionFileName: '', onConversionConfirm: () => {}, onConversionCancel: () => {}, })); }

// Listen for media rename events from the backend
listen('media_renamed', (event) => {
    console.log('[ProjectStore] Received media_renamed event:', event.payload);
    if (!event.payload) return;

    const { old_media_stem, new_media_stem, new_media_file_relative_path, new_absolute_path } = event.payload;

    project.update(p => {
        let updatedState = { ...p };
        let stateChanged = false;

        // Update selectedMediaNotePath (for Notes tab player)
        if (p.selectedMediaNotePath) {
            const currentNoteFileNameWithExt = p.selectedMediaNotePath.split(/[\/]/).pop();
            const currentNoteStem = currentNoteFileNameWithExt.substring(0, currentNoteFileNameWithExt.lastIndexOf('.'));
            const pathParts = p.selectedMediaNotePath.split(/[\/]/);
            const parentFolderForNote = pathParts.length > 2 ? pathParts[pathParts.length - 3] : null; // e.g. {OldStem}/Media/OldStem.mp4 -> OldStem

            if (currentNoteStem === old_media_stem && parentFolderForNote === old_media_stem) {
                updatedState.selectedMediaNotePath = new_absolute_path; // The event payload gives the new media path directly
                stateChanged = true;
                console.log('[ProjectStore] Updated selectedMediaNotePath due to rename (stem and folder match).');
            }
        }

        // Update the main 'files' array (file tree)
        function updateFileEntriesRecursive(nodes, oldStem, newStem, newAbsMediaPath, newRelMediaPath, baseDir) {
            if (!Array.isArray(nodes)) return { updatedNodes: nodes, changed: false };

            let overallChanged = false;
            const updatedNodes = nodes.map(node => {
                let nodeChanged = false;
                let updatedNode = { ...node };

                if (updatedNode.media_xml_identifier === oldStem) {
                    updatedNode.media_xml_identifier = newStem;
                    nodeChanged = true;

                    if (updatedNode.file_type === 'directory_media_stem') {
                        updatedNode.name = newStem;
                        // Path of the new stem folder, e.g., /app/project/.harvey_files/Media/NewStem
                        const mediaFileParentDir = newAbsMediaPath.substring(0, newAbsMediaPath.lastIndexOf('/'));
                        const newStemFolderPath = mediaFileParentDir.substring(0, mediaFileParentDir.lastIndexOf('/'));

                        updatedNode.path = newStemFolderPath;
                        if (baseDir && newStemFolderPath.startsWith(baseDir)) {
                            updatedNode.relative_path = newStemFolderPath.substring(baseDir.length + 1).replace(/\\/g, '/');
                        } else {
                            updatedNode.relative_path = newStemFolderPath.replace(/\\/g, '/'); // Fallback if baseDir is not applicable
                        }
                        nodeChanged = true;
                    } else if (updatedNode.file_type === MEDIA_SUBDIR || updatedNode.file_type === TRANSCRIPTS_SUBDIR || (updatedNode.is_directory && updatedNode.name === MEDIA_SUBDIR) || (updatedNode.is_directory && updatedNode.name === TRANSCRIPTS_SUBDIR)) {
                        // These are the "media" and "transcripts" subfolders inside the stem folder
                        const oldStemFolderPath = updatedNode.path.substring(0, updatedNode.path.lastIndexOf('/')); // .../OldStem
                        const newStemFolderPath = oldStemFolderPath.replace(oldStem, newStem); // .../NewStem
                        updatedNode.path = updatedNode.path.replace(oldStemFolderPath, newStemFolderPath);
                        updatedNode.relative_path = updatedNode.relative_path.replace(oldStem, newStem); // Simpler replacement for relative path
                        nodeChanged = true;
                    } else if (updatedNode.file_type === 'media' && updatedNode.path.includes(`/${oldStem}/`)) {
                        updatedNode.name = newAbsMediaPath.split(/[\/]/).pop();
                        updatedNode.path = newAbsMediaPath;
                        updatedNode.relative_path = newRelMediaPath;
                        nodeChanged = true;
                    } else if (updatedNode.file_type === 'transcript' && updatedNode.path.includes(`/${oldStem}/`)) {
                        if (updatedNode.name.startsWith(oldStem)) {
                            updatedNode.name = updatedNode.name.replace(oldStem, newStem);
                        }
                        updatedNode.path = updatedNode.path.replace(`/${oldStem}/`, `/${newStem}/`);
                        updatedNode.relative_path = updatedNode.relative_path.replace(`/${oldStem}/`, `/${newStem}/`);
                        nodeChanged = true;
                    }
                }

                if (updatedNode.children && updatedNode.children.length > 0) {
                    const result = updateFileEntriesRecursive(updatedNode.children, oldStem, newStem, newAbsMediaPath, newRelMediaPath, baseDir);
                    if (result.changed) {
                        updatedNode.children = result.updatedNodes;
                        nodeChanged = true;
                    }
                }
                if (nodeChanged) overallChanged = true;
                return updatedNode;
            });

            if (overallChanged) {
                updatedNodes.sort((a, b) => a.name.localeCompare(b.name));
            }
            return { updatedNodes, changed: overallChanged };
        }

        const filesUpdateResult = updateFileEntriesRecursive(updatedState.files, old_media_stem, new_media_stem, new_absolute_path, new_media_file_relative_path, p.baseDirectory);
        if (filesUpdateResult.changed) {
            updatedState.files = filesUpdateResult.updatedNodes;
            stateChanged = true;
            console.log('[ProjectStore] Updated main files tree due to media rename.');
        }

        return stateChanged ? updatedState : p;
    });
});

// Listen for item rename events from the backend
listen('item_renamed', (event) => {
    console.log('[ProjectStore] Received item_renamed event:', event.payload);
    if (!event.payload) return;

    const { old_path, new_path, new_name, item_type, project_xml_path, base_directory } = event.payload;

    project.update(p => {
        let updatedState = { ...p };
        let stateChanged = false;

        const normalized_old_path = old_path.replace(/\\/g, '/');
        const normalized_new_path = new_path.replace(/\\/g, '/');

        // Update Selected Path
        if (item_type === 'doc' && p.selectedDocumentPath === normalized_old_path) {
            updatedState.selectedDocumentPath = normalized_new_path;
            stateChanged = true;
            console.log('[ProjectStore item_renamed] Updated selectedDocumentPath.');
        } else if (item_type === 'imported_transcript' && p.currentImportedTranscriptPath === normalized_old_path) {
            updatedState.currentImportedTranscriptPath = normalized_new_path;
            stateChanged = true;
            console.log('[ProjectStore item_renamed] Updated currentImportedTranscriptPath.');
        }
        // TODO: Add checks for table and image if dedicated selected path variables are introduced.
        // For now, if a table or image was selected via selectedDocumentPath, it will be handled by the 'doc' case
        // if the item_type was misidentified or if they share the selection variable.
        // However, the backend sends specific item_types, so this should be accurate.
        // If a 'table' or 'image' type item was viewed using selectedDocumentPath, and that path matches, it should also update.
        // This logic might need refinement if tables/images get their own distinct selected paths in the store.
        else if ((item_type === 'table' || item_type === 'image') && p.selectedDocumentPath === normalized_old_path) {
            updatedState.selectedDocumentPath = normalized_new_path; // Assuming they use selectedDocumentPath
            stateChanged = true;
            console.log(`[ProjectStore item_renamed] Updated selectedDocumentPath for ${item_type}.`);
        }


        // Update File Lists
        let new_relative_path = '';
        if (base_directory && normalized_new_path.startsWith(base_directory)) {
            new_relative_path = normalized_new_path.substring(base_directory.length + 1).replace(/\\/g, '/');
        } else {
            new_relative_path = normalized_new_path.replace(/\\/g, '/'); // Fallback
            console.warn('[ProjectStore item_renamed] new_path did not start with base_directory. Base:', base_directory, 'NewPath:', normalized_new_path);
        }

        if (item_type === 'doc') {
            const docIndex = updatedState.documentFiles.findIndex(doc => doc.path === normalized_old_path || (p.baseDirectory + '/' + doc.relativePath).replace(/\\/g, '/') === normalized_old_path);
            if (docIndex > -1) {
                updatedState.documentFiles[docIndex].name = new_name;
                updatedState.documentFiles[docIndex].path = normalized_new_path;
                updatedState.documentFiles[docIndex].relativePath = new_relative_path;
                updatedState.documentFiles.sort((a, b) => a.name.localeCompare(b.name));
                stateChanged = true;
                console.log('[ProjectStore item_renamed] Updated documentFiles entry.');
            }
        } else if (item_type === 'table') {
            const tableIndex = updatedState.tableFiles.findIndex(table => table.path === normalized_old_path || (p.baseDirectory + '/' + table.relativePath).replace(/\\/g, '/') === normalized_old_path);
            if (tableIndex > -1) {
                updatedState.tableFiles[tableIndex].name = new_name;
                updatedState.tableFiles[tableIndex].path = normalized_new_path;
                updatedState.tableFiles[tableIndex].relativePath = new_relative_path;
                updatedState.tableFiles.sort((a, b) => a.name.localeCompare(b.name));
                stateChanged = true;
                console.log('[ProjectStore item_renamed] Updated tableFiles entry.');
            }
        } else if (item_type === 'image') {
            const imageIndex = updatedState.imageFiles.findIndex(img => img.path === normalized_old_path || (p.baseDirectory + '/' + img.relativePath).replace(/\\/g, '/') === normalized_old_path);
            if (imageIndex > -1) {
                updatedState.imageFiles[imageIndex].name = new_name;
                updatedState.imageFiles[imageIndex].path = normalized_new_path;
                updatedState.imageFiles[imageIndex].relativePath = new_relative_path;
                updatedState.imageFiles.sort((a, b) => a.name.localeCompare(b.name));
                stateChanged = true;
                console.log('[ProjectStore item_renamed] Updated imageFiles entry.');
            }
        } else if (item_type === 'imported_transcript') {
            const importedIndex = updatedState.importedTranscriptFiles.findIndex(it => it.path === normalized_old_path || (p.baseDirectory + '/' + it.relativePath).replace(/\\/g, '/') === normalized_old_path);
            if (importedIndex > -1) {
                updatedState.importedTranscriptFiles[importedIndex].name = new_name;
                updatedState.importedTranscriptFiles[importedIndex].path = normalized_new_path;
                updatedState.importedTranscriptFiles[importedIndex].relativePath = new_relative_path;
                updatedState.importedTranscriptFiles.sort((a, b) => a.name.localeCompare(b.name));
                stateChanged = true;
                console.log('[ProjectStore item_renamed] Updated importedTranscriptFiles entry.');
            }
        } else if (item_type === 'transcript') {
            // This is a media-associated transcript. Needs to update within the main `files` tree.
            function updateTranscriptInTreeRecursive(nodes) {
                if (!Array.isArray(nodes)) return { updatedNodes: nodes, changed: false };
                let overallChanged = false;
                const updatedNodes = nodes.map(node => {
                    let nodeChanged = false;
                    let updatedNode = { ...node };

                    if (updatedNode.file_type === 'transcript' && updatedNode.path === normalized_old_path) {
                        updatedNode.name = new_name;
                        updatedNode.path = normalized_new_path;
                        updatedNode.relative_path = new_relative_path;
                        nodeChanged = true;
                        console.log('[ProjectStore item_renamed] Updated transcript entry in main files tree.');
                    }

                    if (updatedNode.children && updatedNode.children.length > 0) {
                        const result = updateTranscriptInTreeRecursive(updatedNode.children);
                        if (result.changed) {
                            updatedNode.children = result.updatedNodes;
                            nodeChanged = true; // Propagate change upwards
                        }
                    }
                    if (nodeChanged) overallChanged = true;
                    return updatedNode;
                });
                 if (overallChanged && nodes.some(n => n.file_type === 'transcript')) { // Only sort if a transcript list was modified
                    updatedNodes.sort((a, b) => {
                        // Keep directory structure, sort transcripts by name
                        if (a.is_directory && !b.is_directory) return -1;
                        if (!a.is_directory && b.is_directory) return 1;
                        return a.name.localeCompare(b.name);
                    });
                }
                return { updatedNodes, changed: overallChanged };
            }

            const transcriptTreeUpdateResult = updateTranscriptInTreeRecursive(updatedState.files);
            if (transcriptTreeUpdateResult.changed) {
                updatedState.files = transcriptTreeUpdateResult.updatedNodes;
                stateChanged = true;
            }
        }

        return stateChanged ? updatedState : p;
    });
});