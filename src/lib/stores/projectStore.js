// src/lib/stores/projectStore.js
import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { message } from '@tauri-apps/plugin-dialog';

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
    segments: [],
    currentTranscriptPath: null,
    transcriptDirty: false,
    selectedMediaFile: null,
    selectedModelName: null,
    selectedLanguage: null,
    speakers: { count: 0, names: [] },
    player: { currentTime: 0, duration: 0, isPlaying: false, currentSegmentIndex: -1 },
    audioBuffer: null,
    isLoading: true,
    isTranscriptLoading: false,
    error: null,
    statusMessage: 'Initializing...',
    isTranscribing: false,
    transcriptionProgress: { percent: 0, message: '' },
    transcriptionJobId: null,
    showTranscribeModal: false,
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
    currentDocumentHighlights: [], // For Lexical JSON documents
    isDocumentMetadataDirty: false, 

    // PDF Specific Annotation State
    currentPdfAnnotations: [], 
    initialPdfAnnotations: [], // To store initially loaded PDF annotations for discard operations
    isPdfAnnotationsDirty: false,

    currentImportedTranscriptPath: null,
    currentImportedTranscriptLexicalJson: null,
    initialImportedTranscriptLexicalJson: null,
    isImportedTranscriptDirty: false,
    isImportedTranscriptLoading: false,
    importedTranscriptError: null,
    activeImportedTranscriptEditorRef: null,

    autosaveEnabled: true,
    transcriptUndoStack: [],
    transcriptRedoStack: [],

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

const MAX_UNDO_STACK_SIZE = 50;

// --- Functions for Media-Associated Transcripts ---
function pushToUndoStack(currentSegments) { project.update(p => { const newUndoStack = [...p.transcriptUndoStack, currentSegments]; if (newUndoStack.length > MAX_UNDO_STACK_SIZE) { newUndoStack.shift(); } return { ...p, transcriptUndoStack: newUndoStack, transcriptRedoStack: [] }; }); }
export function undoTranscriptChange() { const undoStack = get(project).transcriptUndoStack; if (undoStack.length === 0) { console.log('[ProjectStore] Undo stack empty.'); return; } project.update(p => { const currentSegments = p.segments; const newUndoStack = [...p.transcriptUndoStack]; const previousSegments = newUndoStack.pop(); const newRedoStack = [...p.transcriptRedoStack, currentSegments]; let newIndex = -1; const time = p.player.currentTime; if (previousSegments.length > 0 && p.player.duration > 0 && time >= 0) { const idx = previousSegments.findIndex((s, index) => { const isLastSegment = index === previousSegments.length - 1; const startTimeCheck = time >= (s.start_time - 0.001); const endTimeCheck = isLastSegment ? time <= s.end_time : time < s.end_time; return startTimeCheck && endTimeCheck; }); newIndex = idx; } console.log('[ProjectStore] Undoing transcript change.'); return { ...p, segments: previousSegments, transcriptUndoStack: newUndoStack, transcriptRedoStack: newRedoStack, transcriptDirty: true, statusMessage: 'Undo successful.', player: { ...p.player, currentSegmentIndex: newIndex } }; }); }
export function redoTranscriptChange() { const redoStack = get(project).transcriptRedoStack; if (redoStack.length === 0) { console.log('[ProjectStore] Redo stack empty.'); return; } project.update(p => { const currentSegments = p.segments; const newRedoStack = [...p.transcriptRedoStack]; const nextSegments = newRedoStack.pop(); const newUndoStack = [...p.transcriptUndoStack, currentSegments]; let newIndex = -1; const time = p.player.currentTime; if (nextSegments.length > 0 && p.player.duration > 0 && time >= 0) { const idx = nextSegments.findIndex((s, index) => { const isLastSegment = index === nextSegments.length - 1; const startTimeCheck = time >= (s.start_time - 0.001); const endTimeCheck = isLastSegment ? time <= s.end_time : time < s.end_time; return startTimeCheck && endTimeCheck; }); newIndex = idx; } console.log('[ProjectStore] Redoing transcript change.'); return { ...p, segments: nextSegments, transcriptUndoStack: newUndoStack, transcriptRedoStack: newRedoStack, transcriptDirty: true, statusMessage: 'Redo successful.', player: { ...p.player, currentSegmentIndex: newIndex } }; }); }
export function markTranscriptAsSaved() { console.log('[ProjectStore] Marking media transcript as saved, clearing undo/redo stacks.'); project.update(p => ({ ...p, transcriptDirty: false, transcriptUndoStack: [], transcriptRedoStack: [], statusMessage: 'Media transcript saved.', error: null, })); }
export function clearTranscriptState() { console.log('[ProjectStore] Clearing media transcript state.'); project.update(p => { if (p.currentTranscriptPath || p.segments.length > 0 || p.transcriptDirty || p.isTranscriptLoading || p.transcriptUndoStack.length > 0 || p.transcriptRedoStack.length > 0) { return { ...p, segments: [], currentTranscriptPath: null, transcriptDirty: false, isTranscriptLoading: false, statusMessage: 'Media transcript cleared.', player: { ...p.player, currentSegmentIndex: -1 }, transcriptUndoStack: [], transcriptRedoStack: [], }; } return p; }); }
export function selectMedia(fileEntry) { console.log('[ProjectStore selectMedia] ACTION START. Received fileEntry:', fileEntry ? `Name: ${fileEntry.name}, Path: ${fileEntry.path}` : 'null'); const currentSelectedPath = get(project).selectedMediaFile?.path; const shouldUpdateSelection = (!fileEntry && currentSelectedPath !== null) || (fileEntry && currentSelectedPath !== fileEntry.path); let speakersToLoad = { count: 0, names: [] }; if (fileEntry && fileEntry.file_type === 'media' && !fileEntry.is_directory && fileEntry.speakers && typeof fileEntry.speakers === 'object') { const loadedCount = Number(fileEntry.speakers['@count']) || 0; const loadedNamesRaw = fileEntry.speakers.name; const loadedNames = Array.isArray(loadedNamesRaw) ? loadedNamesRaw : (loadedNamesRaw ? [loadedNamesRaw] : []); speakersToLoad = { count: loadedCount, names: [...loadedNames] }; if (speakersToLoad.count !== speakersToLoad.names.length) { console.warn(`[ProjectStore selectMedia] Discrepancy count/names for ${fileEntry.name}. Adjusting.`); speakersToLoad.count = speakersToLoad.names.length; speakersToLoad.names = speakersToLoad.names.slice(0, speakersToLoad.count); } console.log(`[ProjectStore selectMedia] Speakers FOUND on FileEntry '${fileEntry.name}':`, JSON.stringify(speakersToLoad)); } else if (fileEntry && fileEntry.file_type === 'media' && !fileEntry.is_directory) { console.log(`[ProjectStore selectMedia] No valid speaker config on entry '${fileEntry.name}'. Using default.`); speakersToLoad = { count: 0, names: [] }; } else { console.log('[ProjectStore selectMedia] No valid media file selected or clearing. Using default speakers.'); speakersToLoad = { count: 0, names: [] }; } const currentStoreSpeakers = get(project).speakers; const speakersChanged = JSON.stringify(currentStoreSpeakers) !== JSON.stringify(speakersToLoad); if (shouldUpdateSelection || speakersChanged) { console.log(`[ProjectStore selectMedia] Updating store. SelectionChanged: ${shouldUpdateSelection}, SpeakersChanged: ${speakersChanged}`); const newSelectedMedia = fileEntry && !fileEntry.is_directory && fileEntry.file_type === 'media' ? fileEntry : null; if (newSelectedMedia && (!newSelectedMedia.name || !newSelectedMedia.path)) { console.error("[ProjectStore] CRITICAL: Attempting set selectedMediaFile without name/path!", newSelectedMedia); } if (newSelectedMedia && !newSelectedMedia.media_xml_identifier) { console.warn("[ProjectStore] WARNING: Setting selectedMediaFile without media_xml_identifier! Saving might fail.", newSelectedMedia); } project.update((p) => ({ ...p, selectedMediaFile: newSelectedMedia, audioBuffer: null, player: { currentTime: 0, duration: 0, isPlaying: false, currentSegmentIndex: -1 }, speakers: speakersToLoad, statusMessage: newSelectedMedia ? `Selected media: ${newSelectedMedia.name}` : 'Media selection cleared.', segments: [], currentTranscriptPath: null, transcriptDirty: false, isTranscriptLoading: false, transcriptUndoStack: [], transcriptRedoStack: [], })); console.log('[ProjectStore selectMedia] Store update complete for media selection/resets.'); const newlySelectedMedia = get(project).selectedMediaFile; console.log(`[ProjectStore selectMedia] Checking associated transcripts for: ${newlySelectedMedia?.name ?? 'null'}`); console.log(`[ProjectStore selectMedia]   -> associated_transcripts object:`, newlySelectedMedia?.associated_transcripts); if (newlySelectedMedia && Array.isArray(newlySelectedMedia.associated_transcripts) && newlySelectedMedia.associated_transcripts.length > 0) { const firstTranscriptInfo = newlySelectedMedia.associated_transcripts[0]; console.log(`[ProjectStore selectMedia]   -> First transcript info object:`, firstTranscriptInfo); const firstTranscriptRelativePath = firstTranscriptInfo?.relativePath; if (firstTranscriptRelativePath && typeof firstTranscriptRelativePath === 'string') { console.log(`[ProjectStore selectMedia] First associated transcript relative path: ${firstTranscriptRelativePath}`); const allFiles = get(project).files; let transcriptNodeToLoad = null; function findTranscriptNodeByRelativePath(nodes, relPath) { if (!Array.isArray(nodes)) return null; for (const node of nodes) { if (node.file_type === 'transcript' && node.relative_path === relPath) { return node; } if (node.children && node.children.length > 0) { const found = findTranscriptNodeByRelativePath(node.children, relPath); if (found) return found; } } return null; } transcriptNodeToLoad = findTranscriptNodeByRelativePath(allFiles, firstTranscriptRelativePath); if (transcriptNodeToLoad && transcriptNodeToLoad.path) { console.log(`[ProjectStore selectMedia] Found first transcript node: ${transcriptNodeToLoad.path}. Auto-loading...`); project.update(p => ({ ...p, currentTranscriptPath: transcriptNodeToLoad.path })); import('$lib/services/projectService.js').then(service => { if (typeof service.loadTranscriptFile === 'function') { service.loadTranscriptFile(transcriptNodeToLoad.path) .catch(error => { console.error(`[ProjectStore] Auto-load first transcript failed:`, error); }); } else { console.error("[ProjectStore] loadTranscriptFile function not found in service."); } }).catch(err => { console.error("[ProjectStore] Failed import projectService for transcript load:", err); }); } else { console.warn(`[ProjectStore selectMedia] Could not find FileEntry node for first transcript relative path: ${firstTranscriptRelativePath}`); } } else { console.warn(`[ProjectStore selectMedia] First associated transcript entry exists but lacks a valid 'relativePath' property. Entry:`, firstTranscriptInfo); } } else { console.log(`[ProjectStore selectMedia] No associated transcripts found for ${newlySelectedMedia?.name ?? 'selected media'}.`); } } else { console.log(`[ProjectStore selectMedia] Selection/speakers unchanged for ${fileEntry?.name ?? 'File'}.`); } console.log('[ProjectStore selectMedia] ACTION END.'); }
export function updatePlayerTime(time) { project.update((p) => { let newIndex = -1; if (p.segments.length > 0 && p.player.duration > 0 && time >= 0) { const idx = p.segments.findIndex((s, index) => { const isLastSegment = index === p.segments.length - 1; const startTimeCheck = time >= (s.start_time - 0.001); const endTimeCheck = isLastSegment ? time <= s.end_time : time < s.end_time; return startTimeCheck && endTimeCheck; }); newIndex = idx; } if (p.player.currentTime !== time || p.player.currentSegmentIndex !== newIndex) { return { ...p, player: { ...p.player, currentTime: time, currentSegmentIndex: newIndex } }; } return p; }); }
export function setPlayerDuration(duration) { project.update((p) => ({ ...p, player: { ...p.player, duration: duration } })); }
export function togglePlayerPlaying(isPlaying) { project.update((p) => ({ ...p, player: { ...p.player, isPlaying: isPlaying } })); }
export function updatePlayerCurrentSegmentIndex(index) { const newIndex = (typeof index === 'number' && index >= -1) ? index : -1; project.update((p) => { if (p.player.currentSegmentIndex !== newIndex) { return { ...p, player: { ...p.player, currentSegmentIndex: newIndex } }; } return p; }); }
export function setTranscriptData(path, data, inferSpeakers = false) { console.log(`[ProjectStore] setTranscriptData called with path: ${path}, inferSpeakers: ${inferSpeakers}`); const newSegments = Array.isArray(data) ? data : []; project.update((p) => { let updatedSpeakers = p.speakers; if (inferSpeakers) { console.warn('[ProjectStore] Speaker inference requested. Overwriting current.'); let inferredSpeakers = { count: 0, names: [] }; if (newSegments.length > 0) { const uniqueSpeakers = [...new Set(newSegments.map(s => s.speaker || 'Unknown'))]; const knownSpeakers = uniqueSpeakers.filter(s => s && s !== 'Unknown'); if (knownSpeakers.length > 0) { knownSpeakers.sort((a, b) => a.localeCompare(b, undefined, {numeric: true, sensitivity: 'base'})); inferredSpeakers = { count: knownSpeakers.length, names: knownSpeakers }; } else { inferredSpeakers = { count: 0, names: [] }; } } updatedSpeakers = inferredSpeakers; console.log('[ProjectStore] Inferred speakers:', updatedSpeakers); } return { ...p, currentTranscriptPath: path, segments: newSegments, transcriptDirty: false, isTranscriptLoading: false, speakers: updatedSpeakers, statusMessage: path ? `Media transcript loaded.` : 'Media transcript cleared.', error: null, player: { ...p.player, currentSegmentIndex: -1 }, transcriptUndoStack: [], transcriptRedoStack: [], }; }); }
export function updateSegment(index, updatedSegmentData, silent = false) { const currentSegments = get(project).segments; if (index < 0 || index >= currentSegments.length) { console.warn('[ProjectStore] updateSegment invalid index:', index); return; } let segmentToUpdate = { ...currentSegments[index] }; let changed = false; for (const key in updatedSegmentData) { if (Object.hasOwnProperty.call(updatedSegmentData, key)) { let newValue = updatedSegmentData[key]; let currentValue = segmentToUpdate[key]; let valueChanged = false; if (key === 'start_time' || key === 'end_time') { const numVal = Number(newValue); if (!isNaN(numVal) && Math.abs(numVal - (Number(currentValue) || 0)) > 0.0001) { segmentToUpdate[key] = numVal; valueChanged = true; } } else if (key === 'text') { if (currentValue !== newValue) { segmentToUpdate[key] = newValue; valueChanged = true; } } else if (key === 'speaker') { if (String(currentValue ?? '') !== String(newValue ?? '')) { segmentToUpdate[key] = String(newValue ?? ''); valueChanged = true; } } else { if (currentValue !== newValue) { segmentToUpdate[key] = newValue; valueChanged = true; } } if (valueChanged) changed = true; } } if (changed) { pushToUndoStack(currentSegments); project.update((p) => { const newSegments = [...p.segments]; newSegments[index] = segmentToUpdate; if (!silent) console.log('[ProjectStore] Updated segment', index); return { ...p, segments: newSegments, transcriptDirty: true, statusMessage: silent ? p.statusMessage : 'Media transcript modified.', }; }); } else { if (!silent) console.log('[ProjectStore] updateSegment no changes needed index', index); } }
export function deleteTranscriptSegment(index) { const currentSegments = get(project).segments; if (index < 0 || index >= currentSegments.length) { console.warn('[ProjectStore] deleteTranscriptSegment called with invalid index:', index); return; } pushToUndoStack(currentSegments); project.update(p => { const oldIndex = p.player.currentSegmentIndex; const newSegments = p.segments.filter((_, i) => i !== index); let newPlayerIndex = -1; if (newSegments.length > 0) { if (oldIndex === index) { newPlayerIndex = Math.max(-1, index - 1); } else if (oldIndex > index) { newPlayerIndex = oldIndex - 1; } else { newPlayerIndex = oldIndex; } } console.log(`[ProjectStore] Deleted segment index ${index}. New player index: ${newPlayerIndex}`); return { ...p, segments: newSegments, transcriptDirty: true, statusMessage: 'Segment deleted (undoable).', player: { ...p.player, currentSegmentIndex: newPlayerIndex } }; }); }
export function insertTranscriptSegment(index, newSegment) { const currentSegments = get(project).segments; if (index < 0 || index > currentSegments.length) { console.warn('[ProjectStore] insertTranscriptSegment called with invalid index:', index); return; } if (!newSegment || typeof newSegment.start_time !== 'number' || typeof newSegment.end_time !== 'number') { console.error('[ProjectStore] insertTranscriptSegment called with invalid segment data:', newSegment); return; } pushToUndoStack(currentSegments); project.update(p => { const segmentsBefore = p.segments.slice(0, index); const segmentsAfter = p.segments.slice(index); const newSegments = [...segmentsBefore, newSegment, ...segmentsAfter]; const newPlayerIndex = index; console.log(`[ProjectStore] Inserted new segment at index ${index}. New player index: ${newPlayerIndex}`); return { ...p, segments: newSegments, transcriptDirty: true, statusMessage: 'Segment inserted (undoable).', player: { ...p.player, currentSegmentIndex: newPlayerIndex } }; }); }
export function setSelectedModel(modelName) { console.log(`[ProjectStore] Set model: ${modelName}`); project.update((p) => ({ ...p, selectedModelName: modelName || null })); }
export function setSelectedLanguage(languageCode) { console.log(`[ProjectStore] Set language: ${languageCode}`); project.update((p) => ({ ...p, selectedLanguage: languageCode || null })); }
export function updateSpeakerConfig(newCount, newNames) { console.log(`[ProjectStore updateSpeakerConfig] Received: count=${newCount}, names=`, newNames); const count = Math.max(0, Math.min(11, Number(newCount) || 0)); const names = Array.isArray(newNames) ? newNames : []; let nameCounter = 1; const validatedNames = []; for (let i = 0; i < count; i++) { let proposedName = names[i] && names[i].trim() !== '' ? names[i].trim() : null; let finalName; if (proposedName && validatedNames.includes(proposedName)) { console.warn(`[ProjectStore updateSpeakerConfig] Duplicate name: '${proposedName}'. Using default.`); proposedName = null; } if (!proposedName) { let defaultName = `Speaker-${nameCounter++}`; while (validatedNames.includes(defaultName) || (names.length > validatedNames.length && names.slice(validatedNames.length).includes(defaultName))) { defaultName = `Speaker ${nameCounter++}`; } finalName = defaultName; } else { finalName = proposedName; } validatedNames.push(finalName); } console.log('[ProjectStore] Validated speaker names:', validatedNames); const newSpeakerConfig = { count: count, names: validatedNames }; const currentState = get(project); const oldSegments = currentState.segments; const oldSpeakerConfig = currentState.speakers; const currentMediaFile = currentState.selectedMediaFile; const projectXmlPath = currentState.xmlPath; const mediaIdentifier = currentMediaFile?.media_xml_identifier; if (!mediaIdentifier) { console.error("[ProjectStore updateSpeakerConfig] Cannot save: Missing Media XML Identifier."); project.update(p => ({ ...p, error: "Save Error: Missing media identifier." })); message("Error: Missing media identifier.", {title: "Save Error", type:"error"}); return; } if (!projectXmlPath) { console.error("[ProjectStore updateSpeakerConfig] Cannot save: Missing Project XML path."); project.update(p => ({ ...p, error: "Save Error: Missing project path." })); message("Error: Project path missing.", {title: "Save Error", type:"error"}); return; } console.log(`[ProjectStore updateSpeakerConfig] Saving for Media ID: ${mediaIdentifier} in project: ${projectXmlPath}`); const speakerMap = new Map(); oldSpeakerConfig.names.forEach((oldName, index) => { if (index < newSpeakerConfig.names.length) { speakerMap.set(oldName, newSpeakerConfig.names[index]); speakerMap.set(`SPEAKER_${String(index).padStart(2,'0')}`, newSpeakerConfig.names[index]); speakerMap.set(`speaker_${index + 1}`, newSpeakerConfig.names[index]); } else { speakerMap.set(oldName, "Unknown"); speakerMap.set(`SPEAKER_${String(index).padStart(2,'0')}`, "Unknown"); speakerMap.set(`speaker_${index + 1}`, "Unknown"); } }); speakerMap.set("Unknown", "Unknown"); newSpeakerConfig.names.forEach(newName => { if (!speakerMap.has(newName)) { speakerMap.set(newName, newName); } }); console.log('[ProjectStore updateSpeakerConfig] Speaker remapping:', speakerMap); let segmentsChanged = false; const newSegments = oldSegments.map(segment => { const currentSpeaker = segment.speaker || "Unknown"; const mappedSpeaker = speakerMap.get(currentSpeaker) || "Unknown"; if (mappedSpeaker !== currentSpeaker) { segmentsChanged = true; return { ...segment, speaker: mappedSpeaker }; } return segment; }); if (segmentsChanged) { console.log('[ProjectStore updateSpeakerConfig] Remapped speaker names. Pushing undo.'); pushToUndoStack(oldSegments); } project.update((p) => ({ ...p, speakers: newSpeakerConfig, segments: newSegments, transcriptDirty: p.transcriptDirty || JSON.stringify(oldSpeakerConfig) !== JSON.stringify(newSpeakerConfig) || segmentsChanged, statusMessage: 'Updating speaker configuration...' })); console.log('[ProjectStore updateSpeakerConfig] Updated store speakers/segments.'); const invokePayload = { projectXmlPath: projectXmlPath, mediaIdentifier: mediaIdentifier, count: newSpeakerConfig.count, names: newSpeakerConfig.names }; console.log('[ProjectStore updateSpeakerConfig] Calling backend save_speaker_config:', invokePayload); invoke('save_speaker_config', invokePayload) .then(() => { console.log(`[ProjectStore updateSpeakerConfig] Persisted config for ${mediaIdentifier}.`); project.update(p => ({ ...p, statusMessage: 'Speaker configuration saved.', error: null })); project.update(p => { const updatedFiles = JSON.parse(JSON.stringify(p.files)); function findAndUpdateMediaSpeakers(nodes, targetIdentifier, newSpeakerData) { if (!Array.isArray(nodes)) return false; let found = false; for (const node of nodes) { if (node.media_xml_identifier === targetIdentifier && (node.file_type === 'media' || node.file_type === 'directory_media_stem')) { console.log(`[ProjectStore updateSpeakerConfig] Found node (${node.name}, type: ${node.file_type}) for identifier ${targetIdentifier}. Updating speakers.`); node.speakers = { '@count': newSpeakerData.count, name: newSpeakerData.names }; found = true; } if (node.children && node.children.length > 0) { if (findAndUpdateMediaSpeakers(node.children, targetIdentifier, newSpeakerData)) { found = true; } } } return found; } const didUpdate = findAndUpdateMediaSpeakers(updatedFiles, mediaIdentifier, newSpeakerConfig); if (didUpdate) { console.log("[ProjectStore updateSpeakerConfig] Successfully updated speaker data in project.files tree."); return { ...p, files: updatedFiles }; } else { console.warn("[ProjectStore updateSpeakerConfig] Could not find media identifier in project.files tree to update speakers."); return p; } }); }) .catch((error) => { console.error(`[ProjectStore updateSpeakerConfig] Failed persist config for ${mediaIdentifier}:`, error); const errorMessage = error?.message || String(error); project.update(p => ({ ...p, error: `Failed save speaker config: ${errorMessage}`, statusMessage: 'Error saving speaker config.' })); if (typeof message !== 'undefined') { message(`Error saving speaker settings: ${errorMessage}`, {title: "Save Error", type: "error"}); } else { console.error(`Error saving speaker settings: ${errorMessage}`); } }); }
export function setAudioBuffer(buffer) { console.log('[ProjectStore] Setting AudioBuffer:', buffer ? `(${buffer.duration.toFixed(2)}s)` : 'null'); project.update((p) => ({ ...p, audioBuffer: buffer })); }
export function toggleTranscribeModal(show) { project.update((p) => ({ ...p, showTranscribeModal: !!show })); }
export function setTranscriptionStatus(isTranscribing, jobId = null, statusMessage = '') { project.update((p) => ({ ...p, isTranscribing: !!isTranscribing, transcriptionJobId: jobId, statusMessage: statusMessage || (isTranscribing ? 'Starting transcription...' : 'Ready'), transcriptionProgress: isTranscribing ? { percent: 0, message: '' } : p.transcriptionProgress, error: isTranscribing ? null : p.error })); }
export function updateTranscriptionProgress(progressPayload) { project.update((p) => { if (p.isTranscribing && p.transcriptionJobId && progressPayload?.jobId === p.transcriptionJobId) { const newMessage = progressPayload?.message ?? p.transcriptionProgress.message; return { ...p, transcriptionProgress: { percent: progressPayload?.percent ?? 0, message: newMessage }, }; } return p; }); }
export function clearTranscriptionStatus(finalStatusMessage = 'Ready', error = null) { project.update((p) => ({ ...p, isTranscribing: false, transcriptionProgress: { percent: 0, message: '' }, transcriptionJobId: null, statusMessage: finalStatusMessage, error: error })); }


// --- General Document (.json files) and PDF Functions ---
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
        
        // Determine loading state based on type
        let newIsDocumentLoading = false;
        if (isJsonDocument && (!selectingSamePath || !p.currentDocumentJson)) {
            newIsDocumentLoading = true; // Load if new JSON doc or current one isn't loaded
        } else if (isPdf && (!selectingSamePath || !p.currentPdfAnnotations || p.currentPdfAnnotations.length === 0 && !p.initialPdfAnnotations ) ) { 
             // Load if new PDF, or current PDF's annotations aren't loaded (and not explicitly empty from a previous load)
            newIsDocumentLoading = true;
        }


        return {
            ...p,
            selectedDocumentPath: filePath,

            // Lexical JSON document states
            currentDocumentJson: (isJsonDocument && selectingSamePath) ? p.currentDocumentJson : (isJsonDocument ? null : null),
            initialDocumentJson: (isJsonDocument && selectingSamePath) ? p.initialDocumentJson : (isJsonDocument ? null : null),
            isDocumentDirty: (isJsonDocument && selectingSamePath) ? p.isDocumentDirty : false,
            activeDocumentEditorRef: (isJsonDocument && selectingSamePath) ? p.activeDocumentEditorRef : null,
            
            currentDocumentFileLevelMetadata: (isJsonDocument && selectingSamePath) ? p.currentDocumentFileLevelMetadata : { ...defaultFileLevelMetadata },
            currentDocumentHighlights: (isJsonDocument && selectingSamePath) ? p.currentDocumentHighlights : [],
            isDocumentMetadataDirty: (isJsonDocument && selectingSamePath) ? p.isDocumentMetadataDirty : false,

            // PDF specific annotation states
            currentPdfAnnotations: (isPdf && selectingSamePath) ? p.currentPdfAnnotations : [], 
            initialPdfAnnotations: (isPdf && selectingSamePath) ? p.initialPdfAnnotations : [], 
            isPdfAnnotationsDirty: (isPdf && selectingSamePath) ? p.isPdfAnnotationsDirty : false,

            isDocumentLoading: newIsDocumentLoading,
            documentError: null,
            
            statusMessage: filePath ? `Loading ${itemType}: ${filePath.split(/[\\/]/).pop()}` : `${itemType.charAt(0).toUpperCase() + itemType.slice(1)} selection cleared.`,
            
            // Reset other types if a new document/pdf is selected
            currentImportedTranscriptPath: (isJsonDocument || isPdf || isTable || isImage || !filePath) ? null : p.currentImportedTranscriptPath, 
            currentImportedTranscriptLexicalJson: (isJsonDocument || isPdf || isTable || isImage || !filePath) ? null : p.currentImportedTranscriptLexicalJson,
            initialImportedTranscriptLexicalJson: (isJsonDocument || isPdf || isTable || isImage || !filePath) ? null : p.initialImportedTranscriptLexicalJson,
            isImportedTranscriptDirty: (isJsonDocument || isPdf || isTable || isImage || !filePath) ? false : p.isImportedTranscriptDirty,
            activeImportedTranscriptEditorRef: (isJsonDocument || isPdf || isTable || isImage || !filePath) ? null : p.activeImportedTranscriptEditorRef,
            importedTranscriptError: (isJsonDocument || isPdf || isTable || isImage || !filePath) ? null : p.importedTranscriptError,

        };
    });

    if (isJsonDocument && filePath) {
        console.log(`[ProjectStore] prepareDocumentView: Path is JSON document, attempting JSON load and Lexical metadata load.`);
        import('$lib/services/projectService.js').then(async service => {
            if (service.loadActiveDocumentContent) {
                await service.loadActiveDocumentContent(); 
            } else {
                console.error("[ProjectStore] loadActiveDocumentContent function not found in projectService.");
                project.update(p => { if(p.selectedDocumentPath === filePath) return ({ ...p, isDocumentLoading: false, documentError: "Internal error: LADC service missing."}); return p; });
            }
            // Load Lexical-specific metadata (which includes Lexical highlights)
            if (service.loadDocumentMetadata && filePath) {
                try {
                    const fullMetadataObject = await service.loadDocumentMetadata(filePath); 
                    project.update(p => {
                        if (p.selectedDocumentPath === filePath && !isPdf) { 
                           return { 
                               ...p, 
                               currentDocumentFileLevelMetadata: fullMetadataObject?.metadata || { ...defaultFileLevelMetadata },
                               currentDocumentHighlights: fullMetadataObject?.highlights || [], 
                               isDocumentMetadataDirty: false 
                           };
                        }
                        return p;
                     });
                } catch (metaError) { 
                    console.error("[ProjectStore] Failed to load Lexical document metadata:", metaError);
                     project.update(p => {
                        if (p.selectedDocumentPath === filePath && !isPdf) {
                           return { ...p, documentError: (p.documentError || '') + ` Lexical Metadata load failed: ${metaError.message || metaError}` };
                        }
                        return p;
                    });
                }
            }
        }).catch(err => { 
            console.error("[ProjectStore] Failed import projectService for JSON document/metadata load:", err);
            project.update(p => { if(p.selectedDocumentPath === filePath) return ({ ...p, isDocumentLoading: false, documentError: "Internal error loading document service."}); return p; });
        });
    } else if (isPdf && filePath) {
         console.log(`[ProjectStore] prepareDocumentView: Path is PDF. Initiating PDF annotation load.`);
         // isDocumentLoading was set true in the main update above
         import('$lib/services/projectService.js').then(async service => {
            if (service.loadPdfAnnotationsFromFile) {
                await service.loadPdfAnnotationsFromFile(filePath); // This will set isDocumentLoading to false on completion/error
            } else {
                console.error("[ProjectStore] loadPdfAnnotationsFromFile function not found in projectService.");
                project.update(p => {if(p.selectedDocumentPath === filePath) return ({ ...p, isDocumentLoading: false, documentError: "Internal error: LPAF service missing."}); return p;});
            }
         }).catch(err => {
            console.error("[ProjectStore] Failed import projectService for PDF annotations:", err);
            project.update(p => {if(p.selectedDocumentPath === filePath) return ({ ...p, isDocumentLoading: false, documentError: "Failed to load PDF annotation service."}); return p; });
         });
    } else if (filePath && (isTable || isImage)) {
         console.log(`[ProjectStore] prepareDocumentView: Path is Table or Image. Viewer will handle rendering. No specific loading here.`);
         project.update(p => ({ ...p, isDocumentLoading: false }));
    } else if (!filePath) {
         console.log(`[ProjectStore] prepareDocumentView: No path. States already cleared.`);
         project.update(p => ({ ...p, isDocumentLoading: false })); // Ensure loading is false if path is cleared
    }
}
// --- (Rest of your projectStore.js functions from your provided file) ...
// setLoadedDocumentData, setDocumentLoadFailed, setDocumentEditorContent, 
// markDocumentAsSaved, markDocumentChangesDiscarded, clearDocumentEditorState, 
// setActiveDocumentEditorRef, clearActiveDocumentEditorRef, updateDocumentHighlights,
// markDocumentMetadataAsSaved, PDF Annotation functions, Imported Transcript functions,
// Shared/Generic Store Functions
export function setLoadedDocumentData(filePath, jsonContent) { console.log(`[ProjectStore] Setting loaded document data (JSON) for: ${filePath}`); project.update(p => { if (p.selectedDocumentPath === filePath && !filePath.toLowerCase().endsWith('.pdf') ) { return { ...p, currentDocumentJson: jsonContent || defaultEmptyJson, initialDocumentJson: jsonContent || defaultEmptyJson, isDocumentDirty: false, isDocumentLoading: false, documentError: null, statusMessage: `Loaded document: ${filePath.split(/[\\/]/).pop()}` }; } else { if(p.isDocumentLoading && p.selectedDocumentPath === filePath) { return { ...p, isDocumentLoading: false }; } return p; } }); }
export function setDocumentLoadFailed(filePath, errorMsg) { console.error(`[ProjectStore] Document load failed for: ${filePath}`, errorMsg); project.update(p => { if (p.selectedDocumentPath === filePath && !filePath.toLowerCase().endsWith('.pdf') ) { return { ...p, currentDocumentJson: null, initialDocumentJson: null, isDocumentDirty: false, isDocumentLoading: false, activeDocumentEditorRef: null, documentError: `Failed to load document: ${errorMsg}`, statusMessage: `Error loading ${filePath.split(/[\\/]/).pop()}.`, currentDocumentFileLevelMetadata: { file_name: '', last_modified: '', title: '', description: '', summary: '' }, currentDocumentHighlights: [], isDocumentMetadataDirty: false }; } else if (p.isDocumentLoading && p.selectedDocumentPath === filePath) { return { ...p, isDocumentLoading: false }; } return p; }); }
export function setDocumentEditorContent(newJsonContent) { project.update(p => { if (p.selectedDocumentPath && !p.selectedDocumentPath.toLowerCase().endsWith('.pdf') ) { const initial = p.initialDocumentJson; const current = p.currentDocumentJson; const isNewDifferentFromInitial = initial !== newJsonContent; const newDirtyState = isNewDifferentFromInitial; if (current !== newJsonContent || p.isDocumentDirty !== newDirtyState) { return { ...p, currentDocumentJson: newJsonContent, isDocumentDirty: newDirtyState, }; } } return p; }); }
export function markDocumentAsSaved(savedJsonContent) { console.log('[ProjectStore] Marking document as saved (JSON).'); project.update(p => { if (p.selectedDocumentPath && !p.selectedDocumentPath.toLowerCase().endsWith('.pdf') ) { return { ...p, initialDocumentJson: savedJsonContent, currentDocumentJson: savedJsonContent, isDocumentDirty: false, statusMessage: `Document saved: ${p.selectedDocumentPath?.split(/[\\/]/).pop()}` }; } return p; }); }
export function markDocumentChangesDiscarded() { console.log('[ProjectStore] Marking document changes as discarded.'); project.update(p => { if (p.selectedDocumentPath) { const isPdf = p.selectedDocumentPath.toLowerCase().endsWith('.pdf'); return { ...p, currentDocumentJson: isPdf ? p.currentDocumentJson : p.initialDocumentJson, /* Keep PDF content if any, reset Lexical */ isDocumentDirty: isPdf ? p.isDocumentDirty : false, statusMessage: 'Document changes discarded.', currentDocumentFileLevelMetadata: p.currentDocumentFileLevelMetadata, /* Keep file level metadata */ currentDocumentHighlights: (isPdf || p.isDocumentMetadataDirty) ? [] : p.currentDocumentHighlights, /* Clear Lexical highlights if PDF or if metadata was dirty */ isDocumentMetadataDirty: false, /* Always reset this on discard */ currentPdfAnnotations: isPdf ? (p.initialPdfAnnotations || []) : p.currentPdfAnnotations, /* Reset PDF annots to initial */ isPdfAnnotationsDirty: false, /* Reset PDF dirty flag */ }; } return p; }); } 
export function clearDocumentEditorState() { console.log('[ProjectStore] Clearing document editor state.'); project.update(p => ({ ...p, selectedDocumentPath: null, currentDocumentJson: null, initialDocumentJson: null, isDocumentDirty: false, isDocumentLoading: false, documentError: null, activeDocumentEditorRef: null, currentDocumentFileLevelMetadata: { file_name: '', last_modified: '', title: '', description: '', summary: '' }, currentDocumentHighlights: [], isDocumentMetadataDirty: false, currentPdfAnnotations: [], initialPdfAnnotations: [], isPdfAnnotationsDirty: false })); }
export function setActiveDocumentEditorRef(editorInstance) { project.update(p => ({ ...p, activeDocumentEditorRef: editorInstance })); }
export function clearActiveDocumentEditorRef() { project.update(p => ({ ...p, activeDocumentEditorRef: null })); }

export function updateDocumentHighlights(newHighlightEvent) {
    // If the active document is a PDF, delegate to updatePdfAnnotations instead
    const currentPath = get(project).selectedDocumentPath;
    if (currentPath && currentPath.toLowerCase().endsWith('.pdf')) {
        updatePdfAnnotations(newHighlightEvent);
        return;
    }
    project.update(p => {
        if (!p.selectedDocumentPath || p.selectedDocumentPath.toLowerCase().endsWith('.pdf')) {
            return p; 
        }
        let highlights = JSON.parse(JSON.stringify(p.currentDocumentHighlights || [])); 
        const { type, id, text, nodeKey, color } = newHighlightEvent; 
        if (type === 'add') {
            if (!nodeKey) { console.warn("[ProjectStore updateDocumentHighlights] 'add' event missing nodeKey for Lexical doc."); return p; }
            const existingIndex = highlights.findIndex(h => h.id === id);
            const newHighlightData = { id, text, nodeKey, color: color || 'transparent', codes: [], comments: [], timestamp: new Date().toISOString() };
            if (existingIndex === -1) highlights.push(newHighlightData);
            else highlights[existingIndex] = { ...newHighlightData, codes: highlights[existingIndex].codes || [], comments: highlights[existingIndex].comments || [] };
            console.log(`[ProjectStore] Lexical Highlight ADDED/UPDATED: ID=${id}, NodeKey=${nodeKey}`);
        } else if (type === 'remove') {
            highlights = highlights.filter(h => h.id !== id);
            console.log(`[ProjectStore] Lexical Highlight REMOVED: ID=${id}`);
        } else if (type === 'update') { 
             if (!nodeKey) { console.warn("[ProjectStore updateDocumentHighlights] 'update' event missing nodeKey for Lexical doc."); return p; }
             const existingIndex = highlights.findIndex(h => h.id === id);
             if (existingIndex !== -1) {
                highlights[existingIndex] = { ...highlights[existingIndex], text, nodeKey, color: color || highlights[existingIndex].color, timestamp: new Date().toISOString() };
                console.log(`[ProjectStore] Lexical Highlight UPDATED: ID=${id}`);
             }
        }
        return { ...p, currentDocumentHighlights: highlights, isDocumentMetadataDirty: true };
    });
}

export function markDocumentMetadataAsSaved(updatedFileLevelMetadata) {
    console.log('[ProjectStore] Marking Lexical document metadata as saved.');
    project.update(p => {
        if (p.selectedDocumentPath && !p.selectedDocumentPath.toLowerCase().endsWith('.pdf')) {
            return { 
                ...p, 
                isDocumentMetadataDirty: false,
                currentDocumentFileLevelMetadata: updatedFileLevelMetadata ? { ...p.currentDocumentFileLevelMetadata, ...updatedFileLevelMetadata } : p.currentDocumentFileLevelMetadata
            };
        }
        return p;
    });
}

export function updatePdfAnnotations(pdfHighlightEvent) {
    project.update(p => {
        if (!p.selectedDocumentPath || !p.selectedDocumentPath.toLowerCase().endsWith('.pdf')) {
            return p;
        }
        let annotations = Array.isArray(p.currentPdfAnnotations) ? JSON.parse(JSON.stringify(p.currentPdfAnnotations)) : [];
        let { type, id, ...highlightData } = pdfHighlightEvent;
        // Treat missing or generic type as an "add" action
        if (!type || type === 'pdfHighlight') type = 'add';

        if (type === 'add') {
            const existingIndex = annotations.findIndex(h => h.id === id);
            const newAnnotation = { id, ...highlightData, timestamp: new Date().toISOString() }; 
            if (existingIndex === -1) annotations.push(newAnnotation);
            else annotations[existingIndex] = { ...annotations[existingIndex], ...newAnnotation };
            console.log(`[ProjectStore] PDF Annotation ADDED/UPDATED: ID=${id}`);
        } else if (type === 'remove') {
            annotations = annotations.filter(h => h.id !== id);
            console.log(`[ProjectStore] PDF Annotation REMOVED: ID=${id}`);
        } else if (type === 'update') { 
             const existingIndex = annotations.findIndex(h => h.id === id);
             if (existingIndex !== -1) {
                annotations[existingIndex] = { ...annotations[existingIndex], ...highlightData, timestamp: new Date().toISOString() };
                console.log(`[ProjectStore] PDF Annotation UPDATED: ID=${id}`);
             }
        }
        return { 
            ...p, 
            currentPdfAnnotations: annotations, 
            isPdfAnnotationsDirty: true,
            // Mark generic DocDirty so existing autosave watcher triggers
            isDocumentDirty: true 
        };
    });
}

export function markPdfAnnotationsAsSaved() {
    console.log('[ProjectStore] Marking PDF annotations as saved.');
    project.update(p => {
        if (p.selectedDocumentPath && p.selectedDocumentPath.toLowerCase().endsWith('.pdf')) {
            return { ...p, isPdfAnnotationsDirty: false, initialPdfAnnotations: JSON.parse(JSON.stringify(p.currentPdfAnnotations)), statusMessage: 'PDF annotations saved.' };
        }
        return p;
    });
}

export function setLoadedPdfAnnotations(annotationsArray) {
     console.log(`[ProjectStore] Setting loaded PDF annotations. Count: ${annotationsArray?.length || 0}`);
     project.update(p => ({ 
        ...p, 
        currentPdfAnnotations: Array.isArray(annotationsArray) ? annotationsArray : [], 
        initialPdfAnnotations: Array.isArray(annotationsArray) ? JSON.parse(JSON.stringify(annotationsArray)) : [],
        isPdfAnnotationsDirty: false,
        isDocumentLoading: false // Assuming this is called after PDF annotations are loaded
    }));
}

// ADDED: New function for PDF annotation load failure
export function setPdfAnnotationsLoadFailed(filePath, errorMsg) {
    console.error(`[ProjectStore] PDF annotations load failed for: ${filePath}`, errorMsg);
    project.update(p => {
        if (p.selectedDocumentPath === filePath && filePath.toLowerCase().endsWith('.pdf')) {
            return {
                ...p,
                currentPdfAnnotations: [],
                initialPdfAnnotations: [],
                isPdfAnnotationsDirty: false,
                isDocumentLoading: false, 
                documentError: (p.documentError ? p.documentError + "; " : "") + `Failed to load PDF annotations: ${errorMsg}`,
                statusMessage: `Error loading PDF annotations for ${filePath.split(/[\\/]/).pop()}.`
            };
        }
        console.warn(`[ProjectStore setPdfAnnotationsLoadFailed] Path mismatch or not a PDF. Store: ${p.selectedDocumentPath}, Error for: ${filePath}`);
        return p;
    });
}

// --- Imported Transcript Functions (Keep existing) ---
// ...
export function prepareImportedTranscriptView(filePath) {
    console.log(`[ProjectStore] prepareImportedTranscriptView called for path: ${filePath}`);
    project.update(p => ({
        ...p,
        currentImportedTranscriptPath: filePath,
        currentImportedTranscriptLexicalJson: p.currentImportedTranscriptPath === filePath ? p.currentImportedTranscriptLexicalJson : null,
        initialImportedTranscriptLexicalJson: p.currentImportedTranscriptPath === filePath ? p.initialImportedTranscriptLexicalJson : null,
        isImportedTranscriptDirty: p.currentImportedTranscriptPath === filePath ? p.isImportedTranscriptDirty : false,
        isImportedTranscriptLoading: !!filePath,
        importedTranscriptError: null,
        activeImportedTranscriptEditorRef: p.currentImportedTranscriptPath === filePath ? p.activeImportedTranscriptEditorRef : null,
        statusMessage: filePath ? `Loading imported transcript: ${filePath.split(/[\\/]/).pop()}` : 'Imported transcript selection cleared.',
        // Clear other document/PDF states
        selectedDocumentPath: null, 
        currentDocumentJson: null, initialDocumentJson: null, isDocumentDirty: false, isDocumentLoading: false, documentError: null, activeDocumentEditorRef: null,
        currentDocumentFileLevelMetadata: { file_name: '', last_modified: '', title: '', description: '', summary: '' }, 
        currentDocumentHighlights: [], isDocumentMetadataDirty: false, 
        currentPdfAnnotations: [], initialPdfAnnotations: [], isPdfAnnotationsDirty: false,
    }));
}
export function setLoadedImportedTranscriptData(filePath, lexicalJsonContent) {
    console.log(`[ProjectStore] Setting loaded data for imported transcript: ${filePath}`);
    const minimalValidJson = createMinimalValidLexicalJson();
    project.update(p => {
        if (p.currentImportedTranscriptPath === filePath) {
            const isValid = lexicalJsonContent && typeof lexicalJsonContent === 'string' && lexicalJsonContent.length > 2;
            return {
                ...p,
                currentImportedTranscriptLexicalJson: isValid ? lexicalJsonContent : minimalValidJson,
                initialImportedTranscriptLexicalJson: isValid ? lexicalJsonContent : minimalValidJson,
                isImportedTranscriptDirty: false,
                isImportedTranscriptLoading: false,
                importedTranscriptError: isValid ? null : "Loaded content was invalid, showing empty editor.",
                statusMessage: `Loaded imported transcript: ${filePath.split(/[\\/]/).pop()}`
            };
        } else {
            if (p.isImportedTranscriptLoading && p.currentImportedTranscriptPath === filePath) { // Loading was for this path but it changed
                 return { ...p, isImportedTranscriptLoading: false };
            }
            return p;
        }
    });
}
export function setImportedTranscriptLoadFailed(filePath, errorMsg) {
    console.error(`[ProjectStore] Imported transcript load failed for: ${filePath}`, errorMsg);
    project.update(p => {
        if (p.currentImportedTranscriptPath === filePath) {
            return {
                ...p,
                currentImportedTranscriptLexicalJson: createMinimalValidLexicalJson(),
                initialImportedTranscriptLexicalJson: createMinimalValidLexicalJson(),
                isImportedTranscriptDirty: false,
                isImportedTranscriptLoading: false,
                importedTranscriptError: `Failed to load transcript: ${errorMsg}`,
                statusMessage: `Error loading imported transcript ${filePath.split(/[\\/]/).pop()}.`,
                activeImportedTranscriptEditorRef: null
            };
        }
        else if (p.isImportedTranscriptLoading && p.currentImportedTranscriptPath === filePath) {
             return { ...p, isImportedTranscriptLoading: false };
        }
        return p;
    });
}
export function setImportedTranscriptEditorContent(filePath, newLexicalJsonContent) {
    project.update(p => {
        if (p.currentImportedTranscriptPath === filePath) {
            const initial = p.initialImportedTranscriptLexicalJson;
            const current = p.currentImportedTranscriptLexicalJson;
            const isNewDifferentFromInitial = initial !== newLexicalJsonContent;
            const newDirtyState = isNewDifferentFromInitial;
            if (current !== newLexicalJsonContent || p.isImportedTranscriptDirty !== newDirtyState) {
                return { ...p, currentImportedTranscriptLexicalJson: newLexicalJsonContent, isImportedTranscriptDirty: newDirtyState, };
            }
        }
        return p;
    });
}
export function markImportedTranscriptAsSaved(filePath, savedLexicalJsonContent) {
    console.log(`[ProjectStore] Marking imported transcript as saved: ${filePath}`);
    project.update(p => {
        if (p.currentImportedTranscriptPath === filePath) {
            return { ...p, initialImportedTranscriptLexicalJson: savedLexicalJsonContent, currentImportedTranscriptLexicalJson: savedLexicalJsonContent, isImportedTranscriptDirty: false, statusMessage: `Imported transcript saved: ${filePath.split(/[\\/]/).pop()}` };
        }
        return p;
    });
}
export function markImportedTranscriptChangesDiscarded(filePath) {
    console.log(`[ProjectStore] Marking imported transcript changes as discarded: ${filePath}`);
    project.update(p => {
        if (p.currentImportedTranscriptPath === filePath) {
            return { ...p, currentImportedTranscriptLexicalJson: p.initialImportedTranscriptLexicalJson, isImportedTranscriptDirty: false, statusMessage: 'Imported transcript changes discarded.'};
        }
        return p;
    });
}
export function setActiveImportedTranscriptEditorRef(editorInstance) { project.update(p => ({ ...p, activeImportedTranscriptEditorRef: editorInstance })); }
export function clearActiveImportedTranscriptEditorRef() { project.update(p => ({ ...p, activeImportedTranscriptEditorRef: null })); }


// --- Shared/Generic Store Functions (Keep existing) ---
// ...
export function toggleAutosave() { project.update(p => { const newState = !p.autosaveEnabled; console.log(`[ProjectStore] Toggling autosave to: ${newState}`); return { ...p, autosaveEnabled: newState, statusMessage: `Autosave ${newState ? 'enabled' : 'disabled'}` }; }); }
export function showUnsavedChangesPrompt(itemName, itemType, onSave, onDiscard, onCancel) { console.log(`[ProjectStore] Showing unsaved changes prompt for: ${itemName} (type: ${itemType})`); project.update(p => ({ ...p, showUnsavedChangesModal: true, unsavedItemName: itemName, unsavedItemType: itemType, onUnsavedSave: onSave, onUnsavedDiscard: onDiscard, onUnsavedCancel: onCancel, })); }
export function hideUnsavedChangesPrompt() { console.log('[ProjectStore] Hiding unsaved changes prompt.'); project.update(p => ({ ...p, showUnsavedChangesModal: false, unsavedItemName: '', unsavedItemType: '', onUnsavedSave: () => {}, onUnsavedDiscard: () => {}, onUnsavedCancel: () => {}, })); }
export function setAssetImportStatus(isImporting, message = null) { project.update(p => ({ ...p, isImportingAsset: isImporting, statusMessage: message !== null ? message : (isImporting ? 'Importing...' : p.statusMessage), error: isImporting ? null : p.error, documentError: isImporting ? null : p.documentError, importedTranscriptError: isImporting ? null : p.importedTranscriptError })); }
export function showConversionPrompt(fileName, onConfirm, onCancel) { console.log(`[ProjectStore] Showing conversion prompt for: ${fileName}`); project.update(p => ({ ...p, showConfirmConversionModal: true, conversionFileName: fileName, onConversionConfirm: onConfirm, onConversionCancel: onCancel, })); }
export function hideConversionPrompt() { console.log('[ProjectStore] Hiding conversion prompt.'); project.update(p => ({ ...p, showConfirmConversionModal: false, conversionFileName: '', onConversionConfirm: () => {}, onConversionCancel: () => {}, })); }