// src/lib/stores/projectStore.js
import { writable, get } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import { refreshProjectFiles } from '../services/projectService.js'; // Import refreshProjectFiles

export const groupContentNotification = writable(null);

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

    selectedGroupId: null,
    selectedGroupData: null,

    activeTranscriptPathInDataTab: null,
};

export const project = writable({ ...initialState });

export const updateProjectStoreState = (newState) => project.update(s => ({...s, ...newState}));

export const currentProjectGroupsList = writable([]);

export async function updateProjectGroupsList(projectId) {
    if (!projectId) {
        console.warn('[projectStore] updateProjectGroupsList called without projectId.');
        currentProjectGroupsList.set([]);
        return;
    }
    try {
        console.log(`[projectStore] Fetching groups for project: ${projectId}`);
        const { invoke } = await import('@tauri-apps/api/core');
        const groups = await invoke('get_project_groups', { projectId });
        const sortedGroups = groups.sort((a, b) => a.name.localeCompare(b.name));
        currentProjectGroupsList.set(sortedGroups);
        console.log(`[projectStore] Updated currentProjectGroupsList with ${sortedGroups.length} groups.`);
    } catch (error) {
        console.error('[projectStore] Error fetching project groups:', error);
        currentProjectGroupsList.set([]);
    }
}

export function clearSelectedGroup() {
    project.update(p => ({
        ...p,
        selectedGroupId: null,
        selectedGroupData: null,
    }));
}

export function setSelectedGroup(groupId, groupData) {
    project.update(p => ({
        ...p,
        selectedGroupId: groupId,
        selectedGroupData: groupData,
        selectedDocumentPath: null,
        currentDocumentJson: null,
        initialDocumentJson: null,
        isDocumentDirty: false,
        isDocumentLoading: false,
        documentError: null,
        activeDocumentEditorRef: null,
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
        statusMessage: groupData ? `Viewing group: ${groupData.name}` : 'Group selection cleared.',
    }));
}


export function prepareDocumentView(filePath, itemType = 'document') {
    console.debug(`[ProjectStore] prepareDocumentView called for path: ${filePath}, type: ${itemType}`);
    const isPdf = filePath ? filePath.toLowerCase().endsWith('.pdf') : false;
    const isTable = itemType === 'tables';
    const isImage = itemType === 'images';
    const isJsonDocument = filePath && itemType === 'documents' && !isPdf;

    const defaultFileLevelMetadata = {
        file_name: '', last_modified: '', title: '', description: '', summary: '',
    };

    project.update(p => {
        const selectingSamePath = p.selectedDocumentPath === filePath && filePath !== null; // Ensure filePath is not null for same path check

        // Determine if loading is needed only if filePath is valid
        let newIsDocumentLoading = false;
        if (filePath) {
            newIsDocumentLoading = (isJsonDocument && (!selectingSamePath || !p.currentDocumentJson)) ||
                                  (isPdf && (!selectingSamePath || !p.currentPdfAnnotations || (p.currentPdfAnnotations.length === 0 && !p.initialPdfAnnotations)));
        }

        return {
            ...p,
            // Clear group selection only if a file path is being actively set
            selectedGroupId: filePath ? null : p.selectedGroupId,
            selectedGroupData: filePath ? null : p.selectedGroupData,

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

            isDocumentLoading: newIsDocumentLoading, // This is now correctly conditional on filePath
            documentError: null,
            statusMessage: filePath ? `Loading ${itemType}: ${filePath.split(/[\\/]/).pop()}` : `${itemType.charAt(0).toUpperCase() + itemType.slice(1)} selection cleared.`,
            isLoading: newIsDocumentLoading, // Global isLoading reflects specific loading

            // Clear other view states
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

    if (filePath) { // Only proceed with async loading if filePath is valid
        if (isJsonDocument) {
            import('$lib/services/projectService.js').then(async service => {
                if (service.loadActiveDocumentContent) await service.loadActiveDocumentContent();
                else { console.error("[ProjectStore] loadActiveDocumentContent not found."); project.update(p => { if(p.selectedDocumentPath === filePath) return ({ ...p, isDocumentLoading: false, documentError: "Internal error."}); return p; });}
                if (service.loadDocumentMetadata) { try { const meta = await service.loadDocumentMetadata(filePath); project.update(p => p.selectedDocumentPath === filePath && !isPdf ? { ...p, currentDocumentFileLevelMetadata: meta?.metadata || defaultFileLevelMetadata, currentDocumentHighlights: meta?.highlights || [], isDocumentMetadataDirty: false } : p); } catch (e) { project.update(p => p.selectedDocumentPath === filePath && !isPdf ? { ...p, documentError: (p.documentError || '') + ` Meta load failed.` } : p);}}
            }).catch(err => project.update(p => { if(p.selectedDocumentPath === filePath) return ({ ...p, isDocumentLoading: false, documentError: "Internal error."}); return p; }));
        } else if (isPdf) {
             import('$lib/services/projectService.js').then(async service => {
                if (service.loadPdfAnnotationsFromFile) await service.loadPdfAnnotationsFromFile(filePath);
                else { console.error("[ProjectStore] loadPdfAnnotationsFromFile not found."); project.update(p => {if(p.selectedDocumentPath === filePath) return ({ ...p, isDocumentLoading: false, documentError: "Internal error."}); return p;});}
             }).catch(err => project.update(p => {if(p.selectedDocumentPath === filePath) return ({ ...p, isDocumentLoading: false, documentError: "Internal error."}); return p; }));
        } else if (isTable || isImage) {
             project.update(p => ({ ...p, isDocumentLoading: false, isLoading: false }));
        }
    } else { // If filePath is null (clearing selection)
         project.update(p => ({ ...p, isDocumentLoading: false, isLoading: false }));
    }
}
export function setLoadedDocumentData(filePath, jsonContent) { console.info(`[ProjectStore] Setting loaded document data (JSON) for: ${filePath}`); project.update(p => { if (p.selectedDocumentPath === filePath && !filePath.toLowerCase().endsWith('.pdf') ) { return { ...p, currentDocumentJson: jsonContent || defaultEmptyJson, initialDocumentJson: jsonContent || defaultEmptyJson, isDocumentDirty: false, isDocumentLoading: false, documentError: null, statusMessage: `Loaded document: ${filePath.split(/[\\/]/).pop()}`, isLoading: false }; } else { if(p.isDocumentLoading && p.selectedDocumentPath === filePath) { return { ...p, isDocumentLoading: false, isLoading: false }; } return p; } }); }
export function setDocumentLoadFailed(filePath, errorMsg) { console.error(`[ProjectStore] Document load failed for: ${filePath}`, errorMsg); project.update(p => { if (p.selectedDocumentPath === filePath && !filePath.toLowerCase().endsWith('.pdf') ) { return { ...p, currentDocumentJson: null, initialDocumentJson: null, isDocumentDirty: false, isDocumentLoading: false, activeDocumentEditorRef: null, documentError: `Failed to load document: ${errorMsg}`, statusMessage: `Error loading ${filePath.split(/[\\/]/).pop()}.`, currentDocumentFileLevelMetadata: { file_name: '', last_modified: '', title: '', description: '', summary: '' }, currentDocumentHighlights: [], isDocumentMetadataDirty: false, isLoading: false }; } else if (p.isDocumentLoading && p.selectedDocumentPath === filePath) { return { ...p, isDocumentLoading: false, isLoading: false }; } return p; }); }
export function setDocumentEditorContent(newJsonContent) { project.update(p => { if (p.selectedDocumentPath && !p.selectedDocumentPath.toLowerCase().endsWith('.pdf') ) { const initial = p.initialDocumentJson; const current = p.currentDocumentJson; const isNewDifferentFromInitial = initial !== newJsonContent; const newDirtyState = isNewDifferentFromInitial; if (current !== newJsonContent || p.isDocumentDirty !== newDirtyState) { return { ...p, currentDocumentJson: newJsonContent, isDocumentDirty: newDirtyState, }; } } return p; }); }
export function markDocumentAsSaved(savedJsonContent) { console.info('[ProjectStore] Marking document as saved (JSON).'); project.update(p => { if (p.selectedDocumentPath && !p.selectedDocumentPath.toLowerCase().endsWith('.pdf') ) { return { ...p, initialDocumentJson: savedJsonContent, currentDocumentJson: savedJsonContent, isDocumentDirty: false, statusMessage: `Document saved: ${p.selectedDocumentPath?.split(/[\\/]/).pop()}` }; } return p; }); }
export function markDocumentChangesDiscarded() { console.info('[ProjectStore] Marking document changes as discarded.'); project.update(p => { if (p.selectedDocumentPath) { const isPdf = p.selectedDocumentPath.toLowerCase().endsWith('.pdf'); return { ...p, currentDocumentJson: isPdf ? p.currentDocumentJson : p.initialDocumentJson, isDocumentDirty: isPdf ? p.isDocumentDirty : false, statusMessage: 'Document changes discarded.', currentDocumentFileLevelMetadata: p.currentDocumentFileLevelMetadata, currentDocumentHighlights: (isPdf || p.isDocumentMetadataDirty) ? [] : p.currentDocumentHighlights, isDocumentMetadataDirty: false, currentPdfAnnotations: isPdf ? (p.initialPdfAnnotations || []) : p.currentPdfAnnotations, isPdfAnnotationsDirty: false, }; } return p; }); }
export function clearDocumentEditorState() { console.info('[ProjectStore] Clearing document editor state.'); project.update(p => ({ ...p, selectedDocumentPath: null, currentDocumentJson: null, initialDocumentJson: null, isDocumentDirty: false, isDocumentLoading: false, documentError: null, activeDocumentEditorRef: null, currentDocumentFileLevelMetadata: { file_name: '', last_modified: '', title: '', description: '', summary: '' }, currentDocumentHighlights: [], isDocumentMetadataDirty: false, currentPdfAnnotations: [], initialPdfAnnotations: [], isPdfAnnotationsDirty: false })); }
export function setActiveDocumentEditorRef(editorInstance) { project.update(p => ({ ...p, activeDocumentEditorRef: editorInstance })); }
export function clearActiveDocumentEditorRef() { project.update(p => ({ ...p, activeDocumentEditorRef: null })); }
export function updateDocumentHighlights(newHighlightEvent) { const currentPath = get(project).selectedDocumentPath; if (currentPath && currentPath.toLowerCase().endsWith('.pdf')) { updatePdfAnnotations(newHighlightEvent); return; } project.update(p => { if (!p.selectedDocumentPath || p.selectedDocumentPath.toLowerCase().endsWith('.pdf')) { return p; } let highlights = JSON.parse(JSON.stringify(p.currentDocumentHighlights || [])); const { type, id, text, nodeKey, color } = newHighlightEvent; if (type === 'add') { if (!nodeKey) { console.warn("[ProjectStore updateDocumentHighlights] 'add' event missing nodeKey for Lexical doc."); return p; } const existingIndex = highlights.findIndex(h => h.id === id); const newHighlightData = { id, text, nodeKey, color: color || 'transparent', codes: [], comments: [], timestamp: new Date().toISOString() }; if (existingIndex === -1) highlights.push(newHighlightData); else highlights[existingIndex] = { ...newHighlightData, codes: highlights[existingIndex].codes || [], comments: highlights[existingIndex].comments || [] }; console.debug(`[ProjectStore] Lexical Highlight ADDED/UPDATED: ID=${id}, NodeKey=${nodeKey}`); } else if (type === 'remove') { highlights = highlights.filter(h => h.id !== id); console.debug(`[ProjectStore] Lexical Highlight REMOVED: ID=${id}`); } else if (type === 'update') { if (!nodeKey) { console.warn("[ProjectStore updateDocumentHighlights] 'update' event missing nodeKey for Lexical doc."); return p; } const existingIndex = highlights.findIndex(h => h.id === id); if (existingIndex !== -1) { highlights[existingIndex] = { ...highlights[existingIndex], text, nodeKey, color: color || highlights[existingIndex].color, timestamp: new Date().toISOString() }; console.debug(`[ProjectStore] Lexical Highlight UPDATED: ID=${id}`); } } return { ...p, currentDocumentHighlights: highlights, isDocumentMetadataDirty: true }; }); }
export function markDocumentMetadataAsSaved(updatedFileLevelMetadata) { console.info('[ProjectStore] Marking Lexical document metadata as saved.'); project.update(p => { if (p.selectedDocumentPath && !p.selectedDocumentPath.toLowerCase().endsWith('.pdf')) { return { ...p, isDocumentMetadataDirty: false, currentDocumentFileLevelMetadata: updatedFileLevelMetadata ? { ...p.currentDocumentFileLevelMetadata, ...updatedFileLevelMetadata } : p.currentDocumentFileLevelMetadata }; } return p; }); }
export function updatePdfAnnotations(pdfHighlightEvent) { project.update(p => { if (!p.selectedDocumentPath || !p.selectedDocumentPath.toLowerCase().endsWith('.pdf')) { return p; } let annotations = Array.isArray(p.currentPdfAnnotations) ? JSON.parse(JSON.stringify(p.currentPdfAnnotations)) : []; let { type, id, ...highlightData } = pdfHighlightEvent; if (!type || type === 'pdfHighlight') type = 'add'; let annotationChanged = false; if (type === 'add') { const existingIndex = annotations.findIndex(h => h.id === id); const newAnnotation = { id, ...highlightData, timestamp: new Date().toISOString() }; if (existingIndex === -1) { annotations.push(newAnnotation); annotationChanged = true; } else { if (JSON.stringify(annotations[existingIndex]) !== JSON.stringify({ ...annotations[existingIndex], ...newAnnotation })) { annotations[existingIndex] = { ...annotations[existingIndex], ...newAnnotation }; annotationChanged = true; } } if(annotationChanged) console.debug(`[ProjectStore] PDF Annotation ADDED/UPDATED: ID=${id}`); } else if (type === 'remove') { const initialLength = annotations.length; annotations = annotations.filter(h => h.id !== id); if (annotations.length < initialLength) { annotationChanged = true; console.debug(`[ProjectStore] PDF Annotation REMOVED: ID=${id}`); } } else if (type === 'update') { const existingIndex = annotations.findIndex(h => h.id === id); if (existingIndex !== -1) { if (JSON.stringify(annotations[existingIndex]) !== JSON.stringify({ ...annotations[existingIndex], ...highlightData, timestamp: new Date().toISOString() })) { annotations[existingIndex] = { ...annotations[existingIndex], ...highlightData, timestamp: new Date().toISOString() }; annotationChanged = true; console.debug(`[ProjectStore] PDF Annotation UPDATED: ID=${id}`); } } } if (annotationChanged) { return { ...p, currentPdfAnnotations: annotations, isPdfAnnotationsDirty: true, isDocumentDirty: true }; } return p; }); }
export function markPdfAnnotationsDirty(updatedAnnotations = null) { project.update(p => { if (p.selectedDocumentPath && p.selectedDocumentPath.toLowerCase().endsWith('.pdf')) { return { ...p, isPdfAnnotationsDirty: true, isDocumentDirty: false, currentPdfAnnotations: updatedAnnotations !== null ? updatedAnnotations : p.currentPdfAnnotations }; } return p; }); }
export function markPdfAnnotationsAsSaved() { console.info('[ProjectStore] Marking PDF annotations as saved.'); project.update(p => { if (p.selectedDocumentPath && p.selectedDocumentPath.toLowerCase().endsWith('.pdf')) { return { ...p, isPdfAnnotationsDirty: false, isDocumentDirty: false, initialPdfAnnotations: JSON.parse(JSON.stringify(p.currentPdfAnnotations)), statusMessage: 'PDF annotations saved.' }; } return p; }); }
export function setLoadedPdfAnnotations(annotationsArray) { console.info(`[ProjectStore] Setting loaded PDF annotations. Count: ${annotationsArray?.length || 0}`); project.update(p => ({ ...p, currentPdfAnnotations: Array.isArray(annotationsArray) ? annotationsArray : [], initialPdfAnnotations: Array.isArray(annotationsArray) ? JSON.parse(JSON.stringify(annotationsArray)) : [], isPdfAnnotationsDirty: false, isDocumentLoading: false, isLoading: false }));}
export function setPdfAnnotationsLoadFailed(filePath, errorMsg) { console.error(`[ProjectStore] PDF annotations load failed for: ${filePath}`, errorMsg); project.update(p => { if (p.selectedDocumentPath === filePath && filePath.toLowerCase().endsWith('.pdf')) { return { ...p, currentPdfAnnotations: [], initialPdfAnnotations: [], isPdfAnnotationsDirty: false, isDocumentLoading: false, documentError: (p.documentError ? p.documentError + "; " : "") + `Failed to load PDF annotations: ${errorMsg}`, statusMessage: `Error loading PDF annotations for ${filePath.split(/[\\/]/).pop()}.`, isLoading: false }; } if (p.isDocumentLoading && p.selectedDocumentPath !== filePath && filePath.toLowerCase().endsWith('.pdf')){ console.warn(`[ProjectStore setPdfAnnotationsLoadFailed] Error for non-selected but previously loading PDF ${filePath}. Clearing general document loading.`); return { ...p, isDocumentLoading: false, isLoading:false }; } return p; }); }

export function prepareImportedTranscriptView(filePath) {
    console.debug(`[ProjectStore] prepareImportedTranscriptView called for path: ${filePath}`);
    project.update(p => {
        const isReselectingSameLoadedPath = p.currentImportedTranscriptPath === filePath && !!filePath && !!p.currentImportedTranscriptLexicalJson;
        let finalIsImportedTranscriptLoading = false;
        let finalIsGlobalLoading = p.isLoading; // Preserve current global loading unless changed by this action
        let finalStatusMessage = p.statusMessage;

        if (!filePath) {
            finalStatusMessage = 'Imported transcript selection cleared.';
            finalIsGlobalLoading = false;
        } else if (isReselectingSameLoadedPath) {
            finalStatusMessage = `Viewing imported transcript: ${filePath.split(/[\/]/).pop()}`;
            finalIsGlobalLoading = false;
        } else {
            finalIsImportedTranscriptLoading = true;
            finalStatusMessage = `Loading imported transcript: ${filePath.split(/[\/]/).pop()}`;
            finalIsGlobalLoading = true;
        }

        return {
            ...p,
            // Clear group selection if a transcript path is being set
            selectedGroupId: filePath ? null : p.selectedGroupId,
            selectedGroupData: filePath ? null : p.selectedGroupData,

            currentImportedTranscriptPath: filePath,
            currentImportedTranscriptLexicalJson: isReselectingSameLoadedPath ? p.currentImportedTranscriptLexicalJson : null,
            initialImportedTranscriptLexicalJson: isReselectingSameLoadedPath ? p.initialImportedTranscriptLexicalJson : null,
            isImportedTranscriptDirty: isReselectingSameLoadedPath ? p.isImportedTranscriptDirty : false,
            isImportedTranscriptLoading: finalIsImportedTranscriptLoading,
            importedTranscriptError: null,
            activeImportedTranscriptEditorRef: isReselectingSameLoadedPath ? p.activeImportedTranscriptEditorRef : null,
            statusMessage: finalStatusMessage,
            isLoading: finalIsGlobalLoading,

            // Clear other view states
            selectedDocumentPath: null, /* ... other document fields ... */
            currentDocumentJson: null, initialDocumentJson: null, isDocumentDirty: false, isDocumentLoading: false, documentError: null, activeDocumentEditorRef: null, currentDocumentFileLevelMetadata: { file_name: '', last_modified: '', title: '', description: '', summary: '' }, currentDocumentHighlights: [], isDocumentMetadataDirty: false, currentPdfAnnotations: [], initialPdfAnnotations: [], isPdfAnnotationsDirty: false,
            selectedMediaNotePath: null, /* ... other media note fields ... */
            currentMediaNoteTranscriptJson: null, initialMediaNoteTranscriptJson: null, isMediaNoteTranscriptDirty: false, mediaNoteTranscriptError: null, isMediaNoteTranscriptLoading: false, activeMediaNoteEditorRef: null,
        };
    });
}
export function setLoadedImportedTranscriptData(filePath, lexicalJsonContent) { console.info(`[ProjectStore] Setting loaded data for imported transcript: ${filePath}`); const minimalValidJson = createMinimalValidLexicalJson(); project.update(p => { if (p.currentImportedTranscriptPath === filePath) { const isValid = lexicalJsonContent && typeof lexicalJsonContent === 'string' && lexicalJsonContent.length > 2; return { ...p, currentImportedTranscriptLexicalJson: isValid ? lexicalJsonContent : minimalValidJson, initialImportedTranscriptLexicalJson: isValid ? lexicalJsonContent : minimalValidJson, isImportedTranscriptDirty: false, isImportedTranscriptLoading: false, importedTranscriptError: isValid ? null : "Loaded content was invalid, showing empty editor.", statusMessage: `Loaded imported transcript: ${filePath.split(/[\\/]/).pop()}`, isLoading: false }; } else { if (p.isImportedTranscriptLoading && p.currentImportedTranscriptPath === filePath) { return { ...p, isImportedTranscriptLoading: false, isLoading: false }; } return p; } }); }
export function setImportedTranscriptLoadFailed(filePath, errorMsg) { console.error(`[ProjectStore] Imported transcript load failed for: ${filePath}`, errorMsg); project.update(p => { if (p.currentImportedTranscriptPath === filePath) { return { ...p, currentImportedTranscriptLexicalJson: createMinimalValidLexicalJson(), initialImportedTranscriptLexicalJson: createMinimalValidLexicalJson(), isImportedTranscriptDirty: false, isImportedTranscriptLoading: false, importedTranscriptError: `Failed to load transcript: ${errorMsg}`, statusMessage: `Error loading imported transcript ${filePath.split(/[\\/]/).pop()}.`, activeImportedTranscriptEditorRef: null, isLoading: false }; } else if (p.isImportedTranscriptLoading && p.currentImportedTranscriptPath === filePath) { return { ...p, isImportedTranscriptLoading: false, isLoading: false }; } return p; }); }
export function setImportedTranscriptEditorContent(filePath, newLexicalJsonContent) { project.update(p => { if (p.currentImportedTranscriptPath === filePath) { const initial = p.initialImportedTranscriptLexicalJson; const current = p.currentImportedTranscriptLexicalJson; const isNewDifferentFromInitial = initial !== newLexicalJsonContent; const newDirtyState = isNewDifferentFromInitial; if (current !== newLexicalJsonContent || p.isImportedTranscriptDirty !== newDirtyState) { return { ...p, currentImportedTranscriptLexicalJson: newLexicalJsonContent, isImportedTranscriptDirty: newDirtyState, }; } } return p; }); }
export function markImportedTranscriptAsSaved(filePath, savedLexicalJsonContent) { console.info(`[ProjectStore] Marking imported transcript as saved: ${filePath}`); project.update(p => { if (p.currentImportedTranscriptPath === filePath) { return { ...p, initialImportedTranscriptLexicalJson: savedLexicalJsonContent, currentImportedTranscriptLexicalJson: savedLexicalJsonContent, isImportedTranscriptDirty: false, statusMessage: `Imported transcript saved: ${filePath.split(/[\\/]/).pop()}` }; } return p; }); }
export function markImportedTranscriptChangesDiscarded(filePath) { console.info(`[ProjectStore] Marking imported transcript changes as discarded: ${filePath}`); project.update(p => { if (p.currentImportedTranscriptPath === filePath) { return { ...p, currentImportedTranscriptLexicalJson: p.initialImportedTranscriptLexicalJson, isImportedTranscriptDirty: false, statusMessage: 'Imported transcript changes discarded.'}; } return p; }); }
export function setActiveImportedTranscriptEditorRef(editorInstance) { project.update(p => ({ ...p, activeImportedTranscriptEditorRef: editorInstance })); }
export function clearActiveImportedTranscriptEditorRef() { project.update(p => ({ ...p, activeImportedTranscriptEditorRef: null })); }

export function prepareMediaNoteView(mediaPath) {
    const normalizedMediaPath = mediaPath ? mediaPath.replace(/\\/g, '/') : null;
    console.debug(`[ProjectStore] prepareMediaNoteView called for mediaPath: ${mediaPath}, normalized to: ${normalizedMediaPath}`);

    project.update(p => {
        const newIsMediaNoteLoading = !!normalizedMediaPath && (p.selectedMediaNotePath !== normalizedMediaPath || !p.currentMediaNoteTranscriptJson);
        let finalIsGlobalLoading = p.isLoading;
        if (newIsMediaNoteLoading) finalIsGlobalLoading = true;
        else if (!normalizedMediaPath) finalIsGlobalLoading = false;


        return {
            ...p,
            // Clear group selection if a media path is being set
            selectedGroupId: normalizedMediaPath ? null : p.selectedGroupId,
            selectedGroupData: normalizedMediaPath ? null : p.selectedGroupData,

            selectedMediaNotePath: normalizedMediaPath,
            currentMediaNoteTranscriptJson: (p.selectedMediaNotePath === normalizedMediaPath && !newIsMediaNoteLoading) ? p.currentMediaNoteTranscriptJson : null,
            initialMediaNoteTranscriptJson: (p.selectedMediaNotePath === normalizedMediaPath && !newIsMediaNoteLoading) ? p.initialMediaNoteTranscriptJson : null,
            isMediaNoteTranscriptDirty: (p.selectedMediaNotePath === normalizedMediaPath && !newIsMediaNoteLoading) ? p.isMediaNoteTranscriptDirty : false,
            isMediaNoteTranscriptLoading: newIsMediaNoteLoading,
            mediaNoteTranscriptError: null,
            activeMediaNoteEditorRef: (p.selectedMediaNotePath === normalizedMediaPath && !newIsMediaNoteLoading) ? p.activeMediaNoteEditorRef : null,

            statusMessage: normalizedMediaPath ? `Loading data for media: ${normalizedMediaPath.split(/[\\/]/).pop()}` : 'Media data selection cleared.',
            isLoading: finalIsGlobalLoading,

            // Clear other fieldnotes states
            selectedDocumentPath: null, /* ... */
            currentDocumentJson: null, initialDocumentJson: null, isDocumentDirty: false, isDocumentLoading: false, documentError: null, activeDocumentEditorRef: null, currentDocumentFileLevelMetadata: { file_name: '', last_modified: '', title: '', description: '', summary: '' }, currentDocumentHighlights: [], isDocumentMetadataDirty: false, currentPdfAnnotations: [], initialPdfAnnotations: [], isPdfAnnotationsDirty: false,
            currentImportedTranscriptPath: null, /* ... */
            currentImportedTranscriptLexicalJson: null, initialImportedTranscriptLexicalJson: null, isImportedTranscriptDirty: false, isImportedTranscriptLoading: false, importedTranscriptError: null, activeImportedTranscriptEditorRef: null,
            activeTranscriptPathInDataTab: null, // Clear active transcript when switching to other views
        };
    });

    if (normalizedMediaPath) {
        // Find the media file in the files tree to get its associated transcripts
        const currentProjectState = get(project);
        function findMediaFileInTree(nodes, path) {
            if (!Array.isArray(nodes)) return null;
            for (const node of nodes) {
                if (node.path === path && node.file_type === 'media') {
                    return node;
                }
                if (node.children) {
                    const found = findMediaFileInTree(node.children, path);
                    if (found) return found;
                }
            }
            return null;
        }

        const mediaFileNode = findMediaFileInTree(currentProjectState.files, normalizedMediaPath);
        const firstTranscriptPath = mediaFileNode?.associated_transcripts?.[0]?.path || null;

        project.update(p => ({
            ...p,
            activeTranscriptPathInDataTab: firstTranscriptPath,
            // If no transcript, set error state to display "No Transcription Yet"
            mediaNoteTranscriptError: firstTranscriptPath ? null : "INFO:FILE_NOT_FOUND",
            isMediaNoteTranscriptLoading: firstTranscriptPath ? true : false, // Only load if there's a transcript
            isLoading: firstTranscriptPath ? true : false, // Global loading
        }));
    } else { // If clearing selection, ensure global loading is false
        project.update(p => ({ ...p, isMediaNoteTranscriptLoading: false, isLoading: false, activeTranscriptPathInDataTab: null }));
    }
}

export function setLoadedMediaNoteTranscriptData(mediaPath, jsonString) {
    console.info(`[ProjectStore] Setting loaded media note transcript data for media: ${mediaPath}`);
    project.update(p => {
        if (p.selectedMediaNotePath === mediaPath) {
            const content = jsonString || defaultEmptyJson;
            return {
                ...p,
                currentMediaNoteTranscriptJson: content,
                initialMediaNoteTranscriptJson: content,
                isMediaNoteTranscriptDirty: false,
                isMediaNoteTranscriptLoading: false,
                mediaNoteTranscriptError: null,
                statusMessage: `Loaded data for media: ${mediaPath.split(/[\\/]/).pop()}`,
                isLoading: false,
            };
        }
        return p;
    });
}

export async function switchTranscriptInDataTab(newTranscriptPath) {
    const proj = get(project);
    if (proj.isMediaNoteTranscriptDirty) {
        const { confirm } = await import('@tauri-apps/plugin-dialog');
        const userConfirmed = await confirm('You have unsaved changes. Do you want to discard them and switch transcripts?', {
            title: 'Unsaved Changes',
            type: 'warning',
        });
        if (!userConfirmed) {
            return;
        }
    }

    project.update(p => {
        // Find the media file associated with the newTranscriptPath
        // This assumes that the media file entry contains associated_transcripts with their paths
        let mediaFileForNewTranscript = null;
        function findMediaFileByTranscriptPath(nodes, transcriptPath) {
            if (!Array.isArray(nodes)) return null;
            for (const node of nodes) {
                if (node.file_type === 'media' && node.associated_transcripts) {
                    if (node.associated_transcripts.some(t => t.path === transcriptPath)) {
                        return node;
                    }
                }
                if (node.children) {
                    const found = findMediaFileByTranscriptPath(node.children, transcriptPath);
                    if (found) return found;
                }
            }
            return null;
        }

        mediaFileForNewTranscript = findMediaFileByTranscriptPath(p.files, newTranscriptPath);

        return {
            ...p,
            activeTranscriptPathInDataTab: newTranscriptPath,
            // Update selectedMediaNotePath to trigger MediaEditorPanel to load the correct media
            selectedMediaNotePath: mediaFileForNewTranscript ? mediaFileForNewTranscript.path : p.selectedMediaNotePath,
            isMediaNoteTranscriptLoading: true,
            mediaNoteTranscriptError: null,
        };
    });

    // This will trigger the load in MediaEditorPanel
}

export function setMediaNoteTranscriptLoadFailed(mediaPath, errorMsg, isFileNotFound = false) {
    console.error(`[ProjectStore] Media note transcript load failed for media: ${mediaPath}`, errorMsg);
    project.update(p => {
        if (p.selectedMediaNotePath === mediaPath) {
            return {
                ...p,
                currentMediaNoteTranscriptJson: defaultEmptyJson,
                initialMediaNoteTranscriptJson: defaultEmptyJson,
                isMediaNoteTranscriptDirty: false,
                isMediaNoteTranscriptLoading: false,
                mediaNoteTranscriptError: isFileNotFound ? "INFO:FILE_NOT_FOUND" : `Failed to load data: ${errorMsg}`,
                statusMessage: isFileNotFound ? `No data/transcription found for ${mediaPath.split(/[\\/]/).pop()}.` : `Error loading data for ${mediaPath.split(/[\\/]/).pop()}.`,
                activeMediaNoteEditorRef: null,
                isLoading: false,
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
    console.info(`[ProjectStore] Marking media note transcript as saved for media: ${mediaPath}`);
    project.update(p => {
        if (p.selectedMediaNotePath === mediaPath) {
            return {
                ...p,
                initialMediaNoteTranscriptJson: savedJsonContent,
                currentMediaNoteTranscriptJson: savedJsonContent,
                isMediaNoteTranscriptDirty: false,
                mediaNoteTranscriptError: null,
                statusMessage: `Data for media ${mediaPath.split(/[\\/]/).pop()} saved.`,
            };
        }
        return p;
    });
}

export function markMediaNoteTranscriptChangesDiscarded(mediaPath) {
    console.info(`[ProjectStore] Marking media note transcript changes as discarded for media: ${mediaPath}`);
    project.update(p => {
        if (p.selectedMediaNotePath === mediaPath) {
            const errorToKeep = p.initialMediaNoteTranscriptJson === defaultEmptyJson && p.mediaNoteTranscriptError === "INFO:FILE_NOT_FOUND"
                ? "INFO:FILE_NOT_FOUND"
                : null;
            const statusToKeep = errorToKeep === "INFO:FILE_NOT_FOUND"
                ? `No data/transcription found for ${mediaPath.split(/[\\/]/).pop()}.`
                : `Changes to data for media ${mediaPath.split(/[\\/]/).pop()} discarded.`;

            return {
                ...p,
                currentMediaNoteTranscriptJson: p.initialMediaNoteTranscriptJson,
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

export function toggleAutosave() { project.update(p => { const newState = !p.autosaveEnabled; console.info(`[ProjectStore] Toggling autosave to: ${newState}`); return { ...p, autosaveEnabled: newState, statusMessage: `Autosave ${newState ? 'enabled' : 'disabled'}` }; }); }
export function showUnsavedChangesPrompt(itemName, itemType, onSave, onDiscard, onCancel) { console.info(`[ProjectStore] Showing unsaved changes prompt for: ${itemName} (type: ${itemType})`); project.update(p => ({ ...p, showUnsavedChangesModal: true, unsavedItemName: itemName, unsavedItemType: itemType, onUnsavedSave: onSave, onUnsavedDiscard: onDiscard, onUnsavedCancel: onCancel, })); }
export function hideUnsavedChangesPrompt() { console.info('[ProjectStore] Hiding unsaved changes prompt.'); project.update(p => ({ ...p, showUnsavedChangesModal: false, unsavedItemName: '', unsavedItemType: '', onUnsavedSave: () => {}, onUnsavedDiscard: () => {}, onUnsavedCancel: () => {}, })); }
export function setAssetImportStatus(isImporting, message = null) { project.update(p => ({ ...p, isImportingAsset: isImporting, statusMessage: message !== null ? message : (isImporting ? 'Importing...' : p.statusMessage), error: isImporting ? null : p.error, documentError: isImporting ? null : p.documentError, importedTranscriptError: isImporting ? null : p.importedTranscriptError, isLoading: isImporting ? true : p.isLoading })); }
export function showConversionPrompt(fileName, onConfirm, onCancel) { console.info(`[ProjectStore] Showing conversion prompt for: ${fileName}`); project.update(p => ({ ...p, showConfirmConversionModal: true, conversionFileName: fileName, onConversionConfirm: onConfirm, onConversionCancel: onCancel, })); }
export function hideConversionPrompt() { console.info('[ProjectStore] Hiding conversion prompt.'); project.update(p => ({ ...p, showConfirmConversionModal: false, conversionFileName: '', onConversionConfirm: () => {}, onConversionCancel: () => {}, })); }

// Listen for media rename events from the backend
listen('media_renamed', (event) => {
    console.info('[ProjectStore] Received media_renamed event:', event.payload);
    if (!event.payload) return;

    const { old_media_stem, new_media_stem, new_media_file_relative_path, new_absolute_path } = event.payload;

    project.update(p => {
        let updatedState = { ...p };
        let stateChanged = false;

        if (p.selectedMediaNotePath) {
            const currentNoteFileNameWithExt = p.selectedMediaNotePath.split(/[\/]/).pop();
            const currentNoteStem = currentNoteFileNameWithExt.substring(0, currentNoteFileNameWithExt.lastIndexOf('.'));
            const pathParts = p.selectedMediaNotePath.split(/[\/]/);
            const parentFolderForNote = pathParts.length > 2 ? pathParts[pathParts.length - 3] : null;

            if (currentNoteStem === old_media_stem && parentFolderForNote === old_media_stem) {
                updatedState.selectedMediaNotePath = new_absolute_path;
                stateChanged = true;
                console.debug('[ProjectStore] Updated selectedMediaNotePath due to rename (stem and folder match).');
            }
        }

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
                        const mediaFileParentDir = newAbsMediaPath.substring(0, newAbsMediaPath.lastIndexOf('/'));
                        const newStemFolderPath = mediaFileParentDir.substring(0, mediaFileParentDir.lastIndexOf('/'));

                        updatedNode.path = newStemFolderPath;
                        if (baseDir && newStemFolderPath.startsWith(baseDir)) {
                            updatedNode.relative_path = newStemFolderPath.substring(baseDir.length + 1).replace(/\\/g, '/');
                        } else {
                            updatedNode.relative_path = newStemFolderPath.replace(/\\/g, '/');
                        }
                        nodeChanged = true;
                    } else if (updatedNode.file_type === MEDIA_SUBDIR || updatedNode.file_type === TRANSCRIPTS_SUBDIR || (updatedNode.is_directory && updatedNode.name === MEDIA_SUBDIR) || (updatedNode.is_directory && updatedNode.name === TRANSCRIPTS_SUBDIR)) {
                        console.log(`[ProjectStore updateFileEntriesRecursive] Updating media/transcript subdir: ${updatedNode.name}`);
                        const oldStemFolderPath = updatedNode.path.substring(0, updatedNode.path.lastIndexOf('/'));
                        const newStemFolderPath = oldStemFolderPath.replace(oldStem, newStem);
                        updatedNode.path = updatedNode.path.replace(oldStemFolderPath, newStemFolderPath);
                        updatedNode.relative_path = updatedNode.relative_path.replace(oldStem, newStem);
                        nodeChanged = true;
                    } else if (updatedNode.file_type === 'media' && updatedNode.path.includes(`/${oldStem}/`)) {
                        console.log(`[ProjectStore updateFileEntriesRecursive] Updating media file: ${updatedNode.name} -> ${newAbsMediaPath.split(/[\/]/).pop()}`);
                        updatedNode.name = newAbsMediaPath.split(/[\/]/).pop();
                        updatedNode.path = newAbsMediaPath;
                        updatedNode.relative_path = newRelMediaPath;
                        nodeChanged = true;
                    } else if (updatedNode.file_type === 'transcript' && updatedNode.path.includes(`/${oldStem}/`)) {
                        console.log(`[ProjectStore updateFileEntriesRecursive] Updating transcript file: ${updatedNode.name}`);
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
            console.debug('[ProjectStore] Updated main files tree due to media rename.');
        }

        return stateChanged ? updatedState : p;
    });
});

listen('item_renamed', (event) => {
    console.info('[ProjectStore] Received item_renamed event:', event.payload);
    if (!event.payload) return;

    const { old_path, new_path, new_name, item_type, project_xml_path, base_directory } = event.payload;
    console.log('[ProjectStore item_renamed] Old path:', old_path, 'New path:', new_path);

    project.update(p => {
        let updatedState = { ...p };
        let stateChanged = false;

        const normalized_old_path = old_path.replace(/\\/g, '/');
        const normalized_new_path = new_path.replace(/\\/g, '/');

        if (item_type === 'doc' && p.selectedDocumentPath === normalized_old_path) {
            updatedState.selectedDocumentPath = normalized_new_path;
            stateChanged = true;
            console.debug('[ProjectStore item_renamed] Updated selectedDocumentPath.');
        } else if (item_type === 'imported_transcript' && p.currentImportedTranscriptPath === normalized_old_path) {
            updatedState.currentImportedTranscriptPath = normalized_new_path;
            stateChanged = true;
            console.debug('[ProjectStore item_renamed] Updated currentImportedTranscriptPath.');
        } else if (item_type === 'media' && p.selectedMediaNotePath === normalized_old_path) {
            updatedState.selectedMediaNotePath = normalized_new_path;
            stateChanged = true;
            console.debug('[ProjectStore item_renamed] Updated selectedMediaNotePath.');
        }
        else if ((item_type === 'table' || item_type === 'image') && p.selectedDocumentPath === normalized_old_path) {
            updatedState.selectedDocumentPath = normalized_new_path;
            stateChanged = true;
            console.debug(`[ProjectStore item_renamed] Updated selectedDocumentPath for ${item_type}.`);
        }


        let new_relative_path = '';
        if (base_directory && normalized_new_path.startsWith(base_directory)) {
            new_relative_path = normalized_new_path.substring(base_directory.length + 1).replace(/\\/g, '/');
        } else {
            new_relative_path = normalized_new_path.replace(/\\/g, '/');
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
                console.debug('[ProjectStore item_renamed] Updated documentFiles entry.');
            }
        } else if (item_type === 'table') {
            const tableIndex = updatedState.tableFiles.findIndex(table => table.path === normalized_old_path || (p.baseDirectory + '/' + table.relativePath).replace(/\\/g, '/') === normalized_old_path);
            if (tableIndex > -1) {
                updatedState.tableFiles[tableIndex].name = new_name;
                updatedState.tableFiles[tableIndex].path = normalized_new_path;
                updatedState.tableFiles[tableIndex].relativePath = new_relative_path;
                updatedState.tableFiles.sort((a, b) => a.name.localeCompare(b.name));
                stateChanged = true;
                console.debug('[ProjectStore item_renamed] Updated tableFiles entry.');
            }
        } else if (item_type === 'image') {
            const imageIndex = updatedState.imageFiles.findIndex(img => img.path === normalized_old_path || (p.baseDirectory + '/' + img.relativePath).replace(/\\/g, '/') === normalized_old_path);
            if (imageIndex > -1) {
                updatedState.imageFiles[imageIndex].name = new_name;
                updatedState.imageFiles[imageIndex].path = normalized_new_path;
                updatedState.imageFiles[imageIndex].relativePath = new_relative_path;
                updatedState.imageFiles.sort((a, b) => a.name.localeCompare(b.name));
                stateChanged = true;
                console.debug('[ProjectStore item_renamed] Updated imageFiles entry.');
            }
        } else if (item_type === 'imported_transcript') {
            const importedIndex = updatedState.importedTranscriptFiles.findIndex(it => it.path === normalized_old_path || (p.baseDirectory + '/' + it.relativePath).replace(/\\/g, '/') === normalized_old_path);
            if (importedIndex > -1) {
                updatedState.importedTranscriptFiles[importedIndex].name = new_name;
                updatedState.importedTranscriptFiles[importedIndex].path = normalized_new_path;
                updatedState.importedTranscriptFiles[importedIndex].relativePath = new_relative_path;
                updatedState.importedTranscriptFiles.sort((a, b) => a.name.localeCompare(b.name));
                stateChanged = true;
                console.debug('[ProjectStore item_renamed] Updated importedTranscriptFiles entry.');
            }
        } else if (item_type === 'transcript') {
            // For transcripts, the media_renamed event handles the tree update.
            // We still need to refresh the project files to ensure the UI is consistent.
            // The `refreshProjectFiles` function will handle selecting the correct media.
            console.debug('[ProjectStore item_renamed] Triggering full project refresh for transcript rename.');
            refreshProjectFiles(normalized_new_path); // Pass the new path to select it after refresh
        }

        // For all other item types, a full refresh is needed to update the file tree
        // and ensure the correct item is selected/displayed.
        if (stateChanged) {
            console.debug('[ProjectStore item_renamed] State changed, triggering full project refresh.');
            console.log('[ProjectStore item_renamed] Files before refresh:', JSON.parse(JSON.stringify(updatedState.files))); // Log before refresh
            refreshProjectFiles(normalized_new_path); // Pass the new path to select it after refresh
            console.log('[ProjectStore item_renamed] Files after refresh (async, may not be immediate):', JSON.parse(JSON.stringify(get(project).files))); // Log after refresh
        }

        return stateChanged ? updatedState : p;
    });
});