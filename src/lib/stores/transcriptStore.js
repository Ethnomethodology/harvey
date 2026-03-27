// src/lib/stores/transcriptStore.js

import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { message } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import notificationManager from '$lib/stores/notificationStore.js';
import { project as projectMainStore, updateProjectStoreState } from './projectStore.js';

function getFilename(path) {
    if (!path) return '';
    return path.split(/[\\/]/).pop();
}

function normalizePath(path) {
    if (typeof path !== 'string') {
        return path;
    }
    // On Windows, paths may start with the `\\?\` prefix. This removes it.
    let normalized = path.startsWith('\\\\?\\') ? path.substring(4) : path;

    // Normalize backslashes to forward slashes for consistent path handling.
    normalized = normalized.replace(/\\/g, '/');

    return normalized;
}


const DUAL_MODE_STORAGE_KEY = 'harvey-dual-mode';

function loadDualModeState() {
    if (typeof window === 'undefined') return false;
    try {
        const storedValue = localStorage.getItem(DUAL_MODE_STORAGE_KEY);
        return storedValue !== null ? JSON.parse(storedValue) : false;
    } catch (error) {
        console.error('[TranscriptStore] Error loading dual mode state from localStorage:', error);
        return false;
    }
}

export const initialTranscriptState = {
    segments: [],
    activeTranscript: null, // Holds { path, language_code, segments }
	currentTranscriptPath: null,
    transcriptDirty: false,
    selectedMediaFile: null,
    selectedModelName: null,
    selectedTranscriptionEngine: 'whisper-cpp',
    selectedLanguage: null,
    speakers: { count: 0, names: [], translatedNames: [] },
    player: { currentTime: 0, duration: 0, isPlaying: false, currentSegmentIndex: -1 },
    audioBuffer: null,
    audioBufferPeaks: null,
    isTranscriptLoading: false,
    isTranscribing: false,
    transcriptionProgress: { percent: 0, message: '' },
    transcriptionJobId: null,
    showTranscribeModal: false,
    transcriptionStartTime: null,
    mediaPathForLastJob: null,
    activeMediaDuringTranscriptionStart: null,
    transcriptUndoStack: [],
    transcriptRedoStack: [],
    pendingTranscriptPathForJobDone: null,
    pendingSegmentsForJobDone: null,
    ranInBackground: false,
    transcriptionJobStatus: null,
    transcriptionErrorMessage: null,
    translateToEnglish: false,
    diarizationEnabledForNextJob: false,

    // Transcription Mode & Manual Settings
    transcriptionMode: 'automatic', // 'automatic' | 'manual'
    manualSegmentSettings: {
        duration: 60,
        speakerMode: 'unassigned', // 'unassigned' | 'alternate'
        lastUsedSpeakerIndex: -1
    },

    // Additional Parameters
    initialPrompt: "",
    hotwords: "",

    // Compare two transcripts in interleaved mode
    isDualModeActive: loadDualModeState(),
    showDualTranscriptModal: false,
    secondaryTranscriptPath: null,
    secondaryTranscriptSegments: [],

    // Translation states
    isTranslating: false,
    translationProgress: { percent: 0, message: '' },
    translationJobId: null,
    translationStartTime: null,
    showTranslateModal: false,
    ranTranslationInBackground: false,
    translationJobStatus: null, // e.g., 'initiating', 'running', 'done', 'error', 'cancelled'
    translationErrorMessage: null,
    translationSourcePath: null,
    transcriptionOutputFileName: null,
    translationOutputFileName: null,
};

export const transcriptStore = writable({ ...initialTranscriptState });

export const MAX_UNDO_STACK_SIZE = 50;

const DEFAULT_MANUAL_SETTINGS = {
    duration: 60,
    speakerMode: 'unassigned',
    lastUsedSpeakerIndex: -1
};

function getManualSettingsKey(projectId, transcriptPath) {
    if (!projectId || !transcriptPath) return null;
    return `harvey-manual-settings-${projectId}-${normalizePath(transcriptPath)}`;
}

export function saveManualSettingsForTranscript(transcriptPath, settings) {
    if (typeof window === 'undefined') return;
    const projectData = get(projectMainStore);
    if (!projectData.id) return;
    
    const key = getManualSettingsKey(projectData.id, transcriptPath);
    if (key) {
        try {
            localStorage.setItem(key, JSON.stringify(settings));
        } catch (e) {
            console.error('[TranscriptStore] Failed to save manual settings:', e);
        }
    }
}

export function loadManualSettingsForTranscript(transcriptPath) {
    if (typeof window === 'undefined') return DEFAULT_MANUAL_SETTINGS;
    const projectData = get(projectMainStore);
    if (!projectData.id) return DEFAULT_MANUAL_SETTINGS;

    const key = getManualSettingsKey(projectData.id, transcriptPath);
    if (key) {
        try {
            const stored = localStorage.getItem(key);
            if (stored) {
                return { ...DEFAULT_MANUAL_SETTINGS, ...JSON.parse(stored) };
            }
        } catch (e) {
            console.error('[TranscriptStore] Failed to load manual settings:', e);
        }
    }
    return DEFAULT_MANUAL_SETTINGS;
}

// --- Transcript Management Functions ---

export function pushToUndoStack() {
    transcriptStore.update(ts => {
        const undoState = {
            segments: ts.segments,
            secondarySegments: ts.isDualModeActive ? ts.secondaryTranscriptSegments : null
        };
        const newUndoStack = [...ts.transcriptUndoStack, undoState];
        if (newUndoStack.length > MAX_UNDO_STACK_SIZE) {
            newUndoStack.shift();
        }
        return { ...ts, transcriptUndoStack: newUndoStack, transcriptRedoStack: [] };
    });
}

export function undoTranscriptChange() {
    const store = get(transcriptStore);
    if (store.transcriptUndoStack.length === 0) {
        return;
    }
    transcriptStore.update(ts => {
        const redoState = {
            segments: ts.segments,
            secondarySegments: ts.isDualModeActive ? ts.secondaryTranscriptSegments : null
        };
        const newUndoStack = [...ts.transcriptUndoStack];
        const prevState = newUndoStack.pop();
        const newRedoStack = [...ts.transcriptRedoStack, redoState];

        let newIndex = -1;
        const time = ts.player.currentTime;
        if (prevState.segments.length > 0 && ts.player.duration > 0 && time >= 0) {
            newIndex = findSegmentIndexWithBinarySearch(prevState.segments, time);
        }

        updateProjectStoreState({ statusMessage: 'Undo successful.' });
        return {
            ...ts,
            segments: prevState.segments,
            secondaryTranscriptSegments: prevState.secondarySegments !== null ? prevState.secondarySegments : ts.secondaryTranscriptSegments,
            transcriptUndoStack: newUndoStack,
            transcriptRedoStack: newRedoStack,
            transcriptDirty: true,
            player: { ...ts.player, currentSegmentIndex: newIndex }
        };
    });
}

export function redoTranscriptChange() {
    const store = get(transcriptStore);
    if (store.transcriptRedoStack.length === 0) {
        return;
    }
    transcriptStore.update(ts => {
        const undoState = {
            segments: ts.segments,
            secondarySegments: ts.isDualModeActive ? ts.secondaryTranscriptSegments : null
        };
        const newRedoStack = [...ts.transcriptRedoStack];
        const nextState = newRedoStack.pop();
        const newUndoStack = [...ts.transcriptUndoStack, undoState];

        let newIndex = -1;
        const time = ts.player.currentTime;
        if (nextState.segments.length > 0 && ts.player.duration > 0 && time >= 0) {
            newIndex = findSegmentIndexWithBinarySearch(nextState.segments, time);
        }

        updateProjectStoreState({ statusMessage: 'Redo successful.' });
        return {
            ...ts,
            segments: nextState.segments,
            secondaryTranscriptSegments: nextState.secondarySegments !== null ? nextState.secondarySegments : ts.secondaryTranscriptSegments,
            transcriptUndoStack: newUndoStack,
            transcriptRedoStack: newRedoStack,
            transcriptDirty: true,
            player: { ...ts.player, currentSegmentIndex: newIndex }
        };
    });
}

export function markTranscriptAsSaved() {
    transcriptStore.update(ts => ({
        ...ts,
        transcriptDirty: false,
        transcriptUndoStack: [],
        transcriptRedoStack: []
    }));
    updateProjectStoreState({ statusMessage: 'Media transcript saved.', error: null });
}

export function clearTranscriptState() {
    transcriptStore.update(ts => {
        if (ts.activeTranscript || ts.segments.length > 0 || ts.transcriptDirty || ts.isTranscriptLoading || ts.transcriptUndoStack.length > 0 || ts.transcriptRedoStack.length > 0 || ts.selectedMediaFile) {
            updateProjectStoreState({ statusMessage: 'Media transcript cleared.' });
            return {
                ...ts,
                selectedMediaFile: null,
                segments: [],
                activeTranscript: null,
				currentTranscriptPath: null,
                transcriptDirty: false,
                isTranscriptLoading: false,
                player: { currentTime: 0, duration: 0, isPlaying: false, currentSegmentIndex: -1 },
                audioBuffer: null,
                audioBufferPeaks: null,
                transcriptUndoStack: [],
                transcriptRedoStack: [],
                speakers: { count: 0, names: [], translatedNames: [] },
                initialPrompt: "",
                hotwords: "",
                activeMediaDuringTranscriptionStart: null,
                pendingTranscriptPathForJobDone: null,
                pendingSegmentsForJobDone: null,
            };
        }
        return ts;
    });
}

export async function selectMedia(fileEntry, transcriptPathToPrioritize = null) {
    const store = get(transcriptStore);
    
    const currentSelectedMedia = get(transcriptStore).selectedMediaFile;
    const currentSelectedPath = currentSelectedMedia?.path;

    // If the new file entry has the same path as the current one, only update transcripts.
    if (fileEntry && currentSelectedPath === fileEntry.path) {
        const transcriptsChanged = JSON.stringify(currentSelectedMedia?.associated_transcripts) !== JSON.stringify(fileEntry?.associated_transcripts);
        if (transcriptsChanged) {
            transcriptStore.update(ts => ({
                ...ts,
                selectedMediaFile: { ...ts.selectedMediaFile, associated_transcripts: fileEntry.associated_transcripts }
            }));
            await loadInitialTranscript(fileEntry, transcriptPathToPrioritize);
        } else if (transcriptPathToPrioritize && get(transcriptStore).currentTranscriptPath !== transcriptPathToPrioritize) {
            // If the transcript path is different, just switch to it without a full reload.
            await switchTranscript(transcriptPathToPrioritize);
        }
    } else {
        // If a different media is selected, deactivate dual mode if it's active.
        if (store.isDualModeActive) {
            console.log('[TranscriptStore] Media changed, deactivating dual mode.');
            await deactivateDualMode();
        }

        const transcriptsChanged = JSON.stringify(currentSelectedMedia?.associated_transcripts) !== JSON.stringify(fileEntry?.associated_transcripts);
        const shouldUpdateSelection = (!fileEntry && currentSelectedPath !== null) || (fileEntry && currentSelectedPath !== fileEntry.path) || transcriptsChanged;

        let speakersToLoad = { count: 0, names: [], translatedNames: [] };
        if (fileEntry && fileEntry.file_type === 'media' && !fileEntry.is_directory && fileEntry.speakers && typeof fileEntry.speakers === 'object') {
            const loadedCount = Number(fileEntry.speakers['@count']) || 0;
            const loadedNamesRaw = fileEntry.speakers.name;
            const loadedNames = Array.isArray(loadedNamesRaw) ? loadedNamesRaw : (loadedNamesRaw ? [loadedNamesRaw] : []);

            let loadedTranslatedNamesRaw = fileEntry.speakers.translatedNames || fileEntry.speakers.translated_names || fileEntry.speakers.second_names;
            let loadedTranslatedNames = [];

            if (Array.isArray(loadedTranslatedNamesRaw)) {
                loadedTranslatedNames = loadedTranslatedNamesRaw.map(name => (typeof name === 'string' ? name.trim() : ''));
            } else if (typeof loadedTranslatedNamesRaw === 'string' && loadedCount === 1) {
                loadedTranslatedNames = [loadedTranslatedNamesRaw.trim()];
            } else {
                loadedTranslatedNames = Array(loadedCount > 0 ? loadedCount : 0).fill('');
            }

            if (loadedTranslatedNames.length > loadedCount) {
                loadedTranslatedNames = loadedTranslatedNames.slice(0, loadedCount);
            } else {
                while (loadedTranslatedNames.length < loadedCount) {
                    loadedTranslatedNames.push('');
                }
            }

            speakersToLoad = {
                count: loadedCount,
                names: [...loadedNames],
                translatedNames: loadedTranslatedNames
            };

            if (speakersToLoad.count !== speakersToLoad.names.length) {
                console.warn(`[TranscriptStore selectMedia] Discrepancy count/names for ${fileEntry.name}. Adjusting.`);
                speakersToLoad.count = speakersToLoad.names.length;
                speakersToLoad.names = speakersToLoad.names.slice(0, speakersToLoad.count);
                speakersToLoad.translatedNames = Array(speakersToLoad.count).fill('');
            }
        } else if (fileEntry && fileEntry.file_type === 'media' && !fileEntry.is_directory) {
            speakersToLoad = { count: 0, names: [], translatedNames: [] };
        } else {
            speakersToLoad = { count: 0, names: [], translatedNames: [] };
        }

        const currentStoreSpeakers = get(transcriptStore).speakers;
        const speakersChanged = JSON.stringify(currentStoreSpeakers) !== JSON.stringify(speakersToLoad);

        if (shouldUpdateSelection || speakersChanged) {
            const newSelectedMedia = fileEntry && !fileEntry.is_directory && fileEntry.file_type === 'media' ? { ...fileEntry } : null;
            if (newSelectedMedia && (!newSelectedMedia.name || !newSelectedMedia.path)) {
                console.error("[TranscriptStore] CRITICAL: Attempting set selectedMediaFile without name/path!", newSelectedMedia);
            }
            if (newSelectedMedia && !newSelectedMedia.media_xml_identifier) {
                console.warn("[TranscriptStore] WARNING: Setting selectedMediaFile without media_xml_identifier! Saving might fail.", newSelectedMedia);
            }

            // Load additional parameters when media changes
            let initialPrompt = "";
            let hotwords = "";
            if (newSelectedMedia && newSelectedMedia.relative_path) {
                try {
                    const projectData = get(projectMainStore);
                    if (projectData && projectData.id) {
                        const paramsRes = await invoke('load_media_additional_parameters', {
                            projectId: projectData.id,
                            assetRelativePath: newSelectedMedia.relative_path
                        });
                        if (paramsRes) {
                            initialPrompt = paramsRes.initial_prompt || "";
                            hotwords = paramsRes.hotwords || "";
                        }
                    }
                } catch (err) {
                    console.warn("[TranscriptStore] Failed to load additional parameters:", err);
                }
            }

            transcriptStore.update((ts) => {
                const mediaPathChanged = ts.selectedMediaFile?.path !== newSelectedMedia?.path;
                return {
                    ...ts,
                    selectedMediaFile: newSelectedMedia,
                    audioBuffer: mediaPathChanged ? null : ts.audioBuffer,
                    audioBufferPeaks: mediaPathChanged ? null : ts.audioBufferPeaks,
                    player: {
                        currentTime: 0,
                        duration: mediaPathChanged ? 0 : ts.player.duration,
                        isPlaying: false,
                        currentSegmentIndex: -1
                    },
                    speakers: speakersToLoad,
                    initialPrompt: initialPrompt,
                    hotwords: hotwords,
                    segments: [],
                    activeTranscript: null,
                    currentTranscriptPath: null,
                    isTranscriptLoading: false,
                    transcriptUndoStack: [],
                    transcriptRedoStack: [],
                    transcriptDirty: false,
                };
            });
            const newlySelectedMedia = get(transcriptStore).selectedMediaFile;

            if (newlySelectedMedia && Array.isArray(newlySelectedMedia.associated_transcripts) && newlySelectedMedia.associated_transcripts.length > 0) {
                await loadInitialTranscript(newlySelectedMedia, transcriptPathToPrioritize);
            } else {
                
            }
        }
    }
}

// Helper function for binary search
function findSegmentIndexWithBinarySearch(segments, time) {
    let low = 0;
    let high = segments.length - 1;

    while (low <= high) {
        const mid = Math.floor((low + high) / 2);
        const segment = segments[mid];
        const isLastSegment = mid === segments.length - 1;

        const startTimeCheck = time >= (segment.start_time - 0.001);
        const endTimeCheck = isLastSegment ? time <= segment.end_time : time < segment.end_time;

        if (startTimeCheck && endTimeCheck) {
            return mid;
        } else if (time < segment.start_time) {
            high = mid - 1;
        } else {
            low = mid + 1;
        }
    }
    return -1;
}

export function updatePlayerTime(time) {
    transcriptStore.update((ts) => {
        let newIndex = -1;
        const segments = ts.segments;
        const numSegments = segments.length;

        if (numSegments > 0 && ts.player.duration > 0 && time >= 0) {
            newIndex = findSegmentIndexWithBinarySearch(segments, time);
        }

        if (ts.player.currentTime !== time || ts.player.currentSegmentIndex !== newIndex) {
            return { ...ts, player: { ...ts.player, currentTime: time, currentSegmentIndex: newIndex } };
        }
        return ts;
    });
}

export function setPlayerDuration(duration) {
    transcriptStore.update((ts) => ({ ...ts, player: { ...ts.player, duration: duration } }));
}

export function togglePlayerPlaying(isPlaying) {
    transcriptStore.update((ts) => ({ ...ts, player: { ...ts.player, isPlaying: isPlaying } }));
}

export function updatePlayerCurrentSegmentIndex(index) {
    const newIndex = (typeof index === 'number' && index >= -1) ? index : -1;
    transcriptStore.update((ts) => {
        if (ts.player.currentSegmentIndex !== newIndex) {
            return { ...ts, player: { ...ts.player, currentSegmentIndex: newIndex } };
        }
        return ts;
    });
}

export function setTranscriptData(path, data, inferSpeakers = false) {
    const normalizedInputPath = normalizePath(path);
    const newSegments = Array.isArray(data) ? data : [];
    transcriptStore.update((ts) => {
        let updatedSpeakers = ts.speakers;
        if (inferSpeakers) {
            let inferredPrimarySpeakers = { count: 0, names: [] };
            if (newSegments.length > 0) {
                const uniqueSpeakers = [...new Set(newSegments.map(s => s.speaker || 'Unknown'))];
                const knownSpeakers = uniqueSpeakers.filter(s => s && s !== 'Unknown');
                if (knownSpeakers.length > 0) {
                    knownSpeakers.sort((a, b) => a.localeCompare(b, undefined, {numeric: true, sensitivity: 'base'}));
                    inferredPrimarySpeakers = { count: knownSpeakers.length, names: knownSpeakers };
                } else {
                    inferredPrimarySpeakers = { count: 0, names: [] };
                }
            } else {
                inferredPrimarySpeakers = { count: 0, names: [] };
            }

            updatedSpeakers = {
                count: inferredPrimarySpeakers.count,
                names: inferredPrimarySpeakers.names,
                translatedNames: ts.speakers.translatedNames || []
            };
        }

        const mediaFile = ts.selectedMediaFile;
        const projectRootPath = get(projectMainStore).baseDirectory;

        let relativePathToMatch = normalizedInputPath;
        if (projectRootPath && normalizedInputPath.startsWith(projectRootPath)) {
            relativePathToMatch = normalizedInputPath.substring(projectRootPath.length).replace(/^[\\/]/, '');
        }
        
        // Ensure relativePathToMatch doesn't have a leading slash for comparison
        relativePathToMatch = relativePathToMatch.replace(/^[\\/]/, '');

        const transcriptInfo = mediaFile?.associated_transcripts?.find(t => {
            if (!t) return false;
            // Compare against relativePath if available, otherwise fallback to path
            const tRel = (t.relativePath || '').replace(/^[\\/]/, '').replace(/\\/g, '/');
            const tPath = normalizePath(t.path || '');
            return (tRel && tRel === relativePathToMatch) || (tPath && tPath === normalizedInputPath);
        });

        if (!transcriptInfo) {
            console.error(`[setTranscriptData] Could not find transcript info for path: ${normalizedInputPath}. Current selectedMediaFile:`, mediaFile);
            console.error(`[setTranscriptData] Available associated_transcripts:`, mediaFile?.associated_transcripts);
            // Clear transcript data if path is invalid or not found
            return {
                ...ts,
                segments: [],
                activeTranscript: null,
				currentTranscriptPath: null,
                isTranscriptLoading: false,
                transcriptDirty: false,
            };
        }

        const langCode = transcriptInfo.language_code || (normalizedInputPath.endsWith('.en.json') ? 'en' : 'original');
        const isTranslation = langCode.includes('-') || normalizedInputPath.endsWith('.en.json');
        const speakerNamesToUse = isTranslation ? updatedSpeakers.translatedNames : updatedSpeakers.names;
        const finalSegmentsForDisplay = remapSegmentSpeakerNames([...newSegments], updatedSpeakers, speakerNamesToUse);

        // Use the path from transcriptInfo as it is guaranteed to be normalized and match the project tree
        const targetPath = transcriptInfo.path;

        updateProjectStoreState({ statusMessage: `Media transcript loaded: ${targetPath.split(/[\\/]/).pop()}` });

        const loadedSettings = loadManualSettingsForTranscript(targetPath);

        return {
            ...ts,
            segments: finalSegmentsForDisplay,
            activeTranscript: {
                path: targetPath,
                language_code: langCode,
                segments: newSegments, // Store raw, unmapped segments
            },
			currentTranscriptPath: targetPath,
            isTranscriptLoading: false,
            speakers: updatedSpeakers,
            player: { ...ts.player, currentSegmentIndex: -1 },
            transcriptUndoStack: [],
            transcriptRedoStack: [],
            manualSegmentSettings: loadedSettings
        };
    });
}

export function updateSecondarySegment(index, updatedSegmentData) {
    transcriptStore.update(ts => {
        if (!ts.isDualModeActive || index < 0 || index >= ts.secondaryTranscriptSegments.length) {
            return ts;
        }
        const newSecondarySegments = [...ts.secondaryTranscriptSegments];
        newSecondarySegments[index] = { ...newSecondarySegments[index], ...updatedSegmentData };
        return { ...ts, secondaryTranscriptSegments: newSecondarySegments, transcriptDirty: true };
    });
}

export function updateSegment(index, updatedSegmentData, silent = false) {
    const currentSegments = get(transcriptStore).segments;
    if (index < 0 || index >= currentSegments.length) {
        console.warn('[TranscriptStore] updateSegment invalid index:', index);
        return;
    }
    let segmentToUpdate = { ...currentSegments[index] };
    let changed = false;
    for (const key in updatedSegmentData) {
        if (Object.hasOwnProperty.call(updatedSegmentData, key)) {
            let newValue = updatedSegmentData[key];
            let currentValue = segmentToUpdate[key];
            let valueChanged = false;
            if (key === 'start_time' || key === 'end_time') {
                const numVal = Number(newValue);
                if (!isNaN(numVal) && Math.abs(numVal - (Number(currentValue) || 0)) > 0.0001) {
                    segmentToUpdate[key] = numVal;
                    valueChanged = true;
                }
            } else if (key === 'text') {
                 const currentTextString = typeof currentValue === 'string' ? currentValue : JSON.stringify(currentValue);
                 const newTextString = typeof newValue === 'string' ? newValue : JSON.stringify(newValue);
                 if (currentTextString !== newTextString) {
                    segmentToUpdate[key] = newValue;
                    valueChanged = true;
                }
            } else if (key === 'speaker') {
                if (String(currentValue ?? '') !== String(newValue ?? '')) {
                    segmentToUpdate[key] = String(newValue ?? '');
                    valueChanged = true;
                }
            } else {
                 if (currentValue !== newValue) {
                    segmentToUpdate[key] = newValue;
                    valueChanged = true;
                }
            }
            if (valueChanged) changed = true;
        }
    }

    if (changed) {
        pushToUndoStack();
        transcriptStore.update((ts) => {
            const newSegments = [...ts.segments];
            newSegments[index] = segmentToUpdate;
            if (!silent) updateProjectStoreState({ statusMessage: 'Media transcript modified.' });
            return {
                ...ts,
                segments: newSegments,
                transcriptDirty: true,
            };
        });
    }
}

export function deleteTranscriptSegment(index) {
    const store = get(transcriptStore);
    if (index < 0 || index >= store.segments.length) {
        console.warn('[TranscriptStore] deleteTranscriptSegment called with invalid index:', index);
        return;
    }

    pushToUndoStack();

    if (store.isDualModeActive) {
        // In dual mode, we should ideally push the secondary segments to a secondary undo stack.
        // For now, we'll just delete from both.
        transcriptStore.update(ts => {
            const newSegments = ts.segments.filter((_, i) => i !== index);
            const newSecondarySegments = ts.secondaryTranscriptSegments.filter((_, i) => i !== index);
            // ... (player index logic as before)
            return {
                ...ts,
                segments: newSegments,
                secondaryTranscriptSegments: newSecondarySegments,
                transcriptDirty: true,
                // ... player update
            };
        });
    } else {
        transcriptStore.update(ts => {
            const oldIndex = ts.player.currentSegmentIndex;
            const newSegments = ts.segments.filter((_, i) => i !== index);
            let newPlayerIndex = -1;
            if (newSegments.length > 0) {
                if (oldIndex === index) {
                    newPlayerIndex = Math.max(-1, index - 1);
                } else if (oldIndex > index) {
                    newPlayerIndex = oldIndex - 1;
                } else {
                    newPlayerIndex = oldIndex;
                }
            }
            updateProjectStoreState({ statusMessage: 'Segment deleted (undoable).' });
            return {
                ...ts,
                segments: newSegments,
                transcriptDirty: true,
                player: { ...ts.player, currentSegmentIndex: newPlayerIndex }
            };
        });
    }
}


export function insertTranscriptSegment(index, newSegment) {
    const currentSegments = get(transcriptStore).segments;
    if (index < 0 || index > currentSegments.length) {
        console.warn('[TranscriptStore] insertTranscriptSegment called with invalid index:', index);
        return;
    }
    if (!newSegment || typeof newSegment.start_time !== 'number' || typeof newSegment.end_time !== 'number') {
        console.error('[TranscriptStore] insertTranscriptSegment called with invalid segment data:', newSegment);
        return;
    }
    pushToUndoStack();

    if (get(transcriptStore).isDualModeActive) {
        const newSecondarySegment = { ...newSegment, text: JSON.stringify({ "root": { "children": [{ "children": [], "direction": null, "format": "", "indent": 0, "type": "paragraph", "version": 1 }], "direction": null, "format": "", "indent": 0, "type": "root", "version": 1 } }) }; // Create a twin with empty text
        transcriptStore.update(ts => {
            const newSegments = [...ts.segments.slice(0, index), newSegment, ...ts.segments.slice(index)];
            const newSecondarySegments = [...ts.secondaryTranscriptSegments.slice(0, index), newSecondarySegment, ...ts.secondaryTranscriptSegments.slice(index)];
            return {
                ...ts,
                segments: newSegments,
                secondaryTranscriptSegments: newSecondarySegments,
                transcriptDirty: true,
                player: { ...ts.player, currentSegmentIndex: index }
            };
        });
    } else {
        transcriptStore.update(ts => {
            const segmentsBefore = ts.segments.slice(0, index);
            const segmentsAfter = ts.segments.slice(index);
            const newSegmentsArray = [...segmentsBefore, newSegment, ...segmentsAfter];
            const newPlayerIndex = index;
            updateProjectStoreState({ statusMessage: 'Segment inserted (undoable).' });
            return {
                ...ts,
                segments: newSegmentsArray,
                transcriptDirty: true,
                player: { ...ts.player, currentSegmentIndex: newPlayerIndex }
            };
        });
    }
}

export function splitTranscriptSegment(index) {
    const store = get(transcriptStore);
    if (index < 0 || index >= store.segments.length) {
        console.warn('[TranscriptStore] splitTranscriptSegment called with invalid index:', index);
        return;
    }

    const originalSegment = store.segments[index];
    const duration = originalSegment.end_time - originalSegment.start_time;
    if (duration <= 0.002) {
         console.warn('[TranscriptStore] splitTranscriptSegment: Segment too short to split.');
         return;
    }

    pushToUndoStack();

    const splitTime = originalSegment.start_time + (duration / 2);

    // Determine new speaker
    let newSpeaker = originalSegment.speaker;
    const speakerNames = store.speakers.names || [];
    if (speakerNames.length > 0) {
        const currentSpeakerIndex = speakerNames.indexOf(originalSegment.speaker);
        if (currentSpeakerIndex !== -1) {
            newSpeaker = speakerNames[(currentSpeakerIndex + 1) % speakerNames.length];
        } else if (speakerNames.length > 1) {
             // If current speaker not in list but we have speakers, maybe default to 2nd one or 1st?
             // Prompt says "speaker_2 or whatever the next name is".
             // If unknown, maybe just keep unknown or pick first. Let's keep unknown/current if not found.
        }
    }

    const newSegment = {
        ...originalSegment,
        start_time: splitTime,
        end_time: originalSegment.end_time,
        speaker: newSpeaker,
        text: JSON.stringify({ root: { children: [{ type: 'paragraph', version: 1, children: [], direction: null, format: '', indent: 0 }], type: 'root', version: 1, direction: null, format: '', indent: 0 } })
    };

    const updatedOriginalSegment = {
        ...originalSegment,
        end_time: splitTime
    };

    if (store.isDualModeActive) {
        // Handle secondary transcript
        const originalSecondary = store.secondaryTranscriptSegments[index];
        // Assuming synchronized segments, we split at the same time ratio?
        // Actually the prompt says "create the 2nd halves of interleaved segments".
        // It implies splitting the secondary segment as well.
        // We should check if secondary segment exists.
        
        let newSecondarySegments = [...store.secondaryTranscriptSegments];
        
        if (originalSecondary) {
             const secDuration = originalSecondary.end_time - originalSecondary.start_time;
             // We use the same split time relative to the segment? Or absolute? 
             // "divide the duration of the corresponding segment in 2"
             // Ideally we split both at their own midpoints if they aren't perfectly aligned, 
             // OR we enforce alignment. 
             // "interleaved together" suggests they correspond 1:1.
             // Let's split secondary at ITS midpoint to be safe/consistent with logic.
             const secSplitTime = originalSecondary.start_time + (secDuration / 2);
             
             // For speaker of secondary, we can follow same logic or keep same.
             // Prompt says "if the original segment got speaker_1 the new empty segment should get speaker_2".
             // It refers to the "segment" being split.
             // We'll apply same speaker rotation logic for secondary if possible, or just copy from primary's decision?
             // Usually secondary transcripts (translations) have same speakers.
             
             const newSecondarySegment = {
                ...originalSecondary,
                start_time: secSplitTime,
                end_time: originalSecondary.end_time,
                speaker: newSpeaker, // Match the primary's new speaker choice
                text: JSON.stringify({ root: { children: [{ type: 'paragraph', version: 1, children: [], direction: null, format: '', indent: 0 }], type: 'root', version: 1, direction: null, format: '', indent: 0 } })
             };
             
             const updatedOriginalSecondary = {
                  ...originalSecondary,
                  end_time: secSplitTime
             };
             
             newSecondarySegments = [
                 ...newSecondarySegments.slice(0, index),
                 updatedOriginalSecondary,
                 newSecondarySegment,
                 ...newSecondarySegments.slice(index + 1)
             ];
        } else {
            // Should not happen if lengths are equal, but handle gracefully
             newSecondarySegments = [
                 ...newSecondarySegments.slice(0, index + 1), // Just insert nothing or empty placeholder?
                 // If unmatched, we can't really split a non-existent segment.
                 // We insert a placeholder to keep length aligned?
                 // Let's assume they are aligned. If not, this might de-sync further.
                 // Ideally we insert a dummy segment to maintain 1:1.
                 { start_time: splitTime, end_time: splitTime + 1, speaker: newSpeaker, text: "{}" },
                 ...newSecondarySegments.slice(index + 1)
             ];
        }

        transcriptStore.update(ts => {
            const newSegments = [
                ...ts.segments.slice(0, index),
                updatedOriginalSegment,
                newSegment,
                ...ts.segments.slice(index + 1)
            ];
            
            return {
                ...ts,
                segments: newSegments,
                secondaryTranscriptSegments: newSecondarySegments,
                transcriptDirty: true,
                player: { ...ts.player, currentSegmentIndex: index + 1 }
            };
        });

    } else {
        transcriptStore.update(ts => {
            const newSegments = [
                ...ts.segments.slice(0, index),
                updatedOriginalSegment,
                newSegment,
                ...ts.segments.slice(index + 1)
            ];
            
            updateProjectStoreState({ statusMessage: 'Segment split (undoable).' });
            return {
                ...ts,
                segments: newSegments,
                transcriptDirty: true,
                player: { ...ts.player, currentSegmentIndex: index + 1 }
            };
        });
    }
}

export function mergeTranscriptSegments(index) {
    const store = get(transcriptStore);
    if (index < 0 || index >= store.segments.length - 1) {
        console.warn('[TranscriptStore] mergeTranscriptSegments called with invalid index:', index);
        return;
    }

    pushToUndoStack();

    function mergeLexicalText(text1, text2) {
        try {
            const json1 = JSON.parse(text1);
            const json2 = JSON.parse(text2);
            
            const mergedJson = {
                ...json1,
                root: {
                    ...json1.root,
                    children: [...(json1.root?.children || []), ...(json2.root?.children || [])]
                }
            };
            return JSON.stringify(mergedJson);
        } catch (e) {
            console.error('[TranscriptStore] Error merging Lexical JSON:', e);
            return text1; // Fallback to first text if merge fails
        }
    }

    const seg1 = store.segments[index];
    const seg2 = store.segments[index + 1];

    const mergedSegment = {
        ...seg1,
        end_time: seg2.end_time,
        text: mergeLexicalText(seg1.text, seg2.text)
    };

    if (store.isDualModeActive) {
        const sec1 = store.secondaryTranscriptSegments[index];
        const sec2 = store.secondaryTranscriptSegments[index + 1];
        
        let mergedSecondary = null;
        if (sec1 && sec2) {
            mergedSecondary = {
                ...sec1,
                end_time: sec2.end_time,
                text: mergeLexicalText(sec1.text, sec2.text)
            };
        }

        transcriptStore.update(ts => {
            const newSegments = [
                ...ts.segments.slice(0, index),
                mergedSegment,
                ...ts.segments.slice(index + 2)
            ];
            
            const newSecondarySegments = [...ts.secondaryTranscriptSegments];
            if (mergedSecondary) {
                newSecondarySegments.splice(index, 2, mergedSecondary);
            }

            return {
                ...ts,
                segments: newSegments,
                secondaryTranscriptSegments: newSecondarySegments,
                transcriptDirty: true,
                player: { ...ts.player, currentSegmentIndex: index }
            };
        });
    } else {
        transcriptStore.update(ts => {
            const newSegments = [
                ...ts.segments.slice(0, index),
                mergedSegment,
                ...ts.segments.slice(index + 2)
            ];
            
            updateProjectStoreState({ statusMessage: 'Segments merged (undoable).' });
            return {
                ...ts,
                segments: newSegments,
                transcriptDirty: true,
                player: { ...ts.player, currentSegmentIndex: index }
            };
        });
    }
}

export function setSelectedModel(modelName) {
    transcriptStore.update((ts) => ({ ...ts, selectedModelName: modelName || null }));
}

export function setSelectedTranscriptionEngine(engine) {
    transcriptStore.update((ts) => ({ ...ts, selectedTranscriptionEngine: engine || 'whisper-cpp' }));
}

export function setSelectedLanguage(languageCode) {
    transcriptStore.update((ts) => ({ ...ts, selectedLanguage: languageCode || null }));
}

export function setTranscriptionMode(mode) {
    transcriptStore.update((ts) => ({ ...ts, transcriptionMode: mode }));
}

export function updateManualSegmentSettings(settings) {
    transcriptStore.update((ts) => {
        const newSettings = { ...ts.manualSegmentSettings, ...settings };
        if (ts.currentTranscriptPath) {
            saveManualSettingsForTranscript(ts.currentTranscriptPath, newSettings);
        }
        return {
            ...ts,
            manualSegmentSettings: newSettings
        };
    });
}

export function updateSpeakerConfig(newCount, newNames, newTranslatedNames = null) {
    const count = Math.max(0, Math.min(11, Number(newCount) || 0));
    const names = Array.isArray(newNames) ? newNames : [];
    let nameCounter = 1;
    const validatedNames = [];
    for (let i = 0; i < count; i++) {
        let proposedName = names[i] && typeof names[i] === 'string' && names[i].trim() !== '' ? names[i].trim() : null;
        let finalName;
        if (proposedName && validatedNames.includes(proposedName)) {
            console.warn(`[TranscriptStore updateSpeakerConfig] Duplicate primary name: '${proposedName}'. Using default.`);
            proposedName = null;
        }
        if (!proposedName) {
            let defaultName = `Speaker ${nameCounter++}`;
            while (validatedNames.includes(defaultName) || (names.slice(0, i).includes(defaultName))) {
                defaultName = `Speaker ${nameCounter++}`;
            }
            finalName = defaultName;
        } else {
            finalName = proposedName;
        }
        validatedNames.push(finalName);
    }

    const validatedTranslatedNames = [];
    if (Array.isArray(newTranslatedNames)) {
        for (let i = 0; i < count; i++) {
            const proposedTranslatedName = (newTranslatedNames[i] && typeof newTranslatedNames[i] === 'string') ? newTranslatedNames[i].trim() : '';
            validatedTranslatedNames.push(proposedTranslatedName);
        }
    } else {
        for (let i = 0; i < count; i++) {
            validatedTranslatedNames.push('');
        }
    }

    while(validatedTranslatedNames.length < count) {
        validatedTranslatedNames.push('');
    }
    if(validatedTranslatedNames.length > count) {
        validatedTranslatedNames.splice(count);
    }


    const newSpeakerConfig = {
        count: count,
        names: validatedNames,
        translatedNames: validatedTranslatedNames
    };

    const currentTranscriptData = get(transcriptStore);
    const projectData = get(projectMainStore);

    const oldSegments = currentTranscriptData.segments;
    const oldSpeakerConfig = currentTranscriptData.speakers;
    const currentMediaFile = currentTranscriptData.selectedMediaFile;
    const projectXmlPath = projectData.xmlPath;
    const mediaIdentifier = currentMediaFile?.media_xml_identifier;

    if (!mediaIdentifier) {
        console.error("[TranscriptStore updateSpeakerConfig] Cannot save: Missing Media XML Identifier.");
        updateProjectStoreState({ error: "Save Error: Missing media identifier."});
        message("Error: Missing media identifier.", {title: "Save Error", type:"error"});
        return;
    }
    if (!projectXmlPath) {
        console.error("[TranscriptStore updateSpeakerConfig] Cannot save: Missing Project XML path.");
        updateProjectStoreState({ error: "Save Error: Missing project path." });
        message("Error: Project path missing.", {title: "Save Error", type:"error"});
        return;
    }

    const speakerMap = new Map();
    oldSpeakerConfig.names.forEach((oldName, index) => {
        if (index < newSpeakerConfig.names.length) {
            speakerMap.set(oldName, newSpeakerConfig.names[index]);
            speakerMap.set(`SPEAKER_${String(index).padStart(2,'0')}`, newSpeakerConfig.names[index]);
            speakerMap.set(`speaker_${index + 1}`, newSpeakerConfig.names[index]);
        } else {
            speakerMap.set(oldName, "Unknown");
            speakerMap.set(`SPEAKER_${String(index).padStart(2,'0')}`, "Unknown");
            speakerMap.set(`speaker_${index + 1}`, "Unknown");
        }
    });
    speakerMap.set("Unknown", "Unknown");
    newSpeakerConfig.names.forEach(newName => {
        if (!speakerMap.has(newName)) {
            speakerMap.set(newName, newName);
        }
    });

    let segmentsChanged = false;
    const newSegments = oldSegments.map(segment => {
        const currentSpeaker = segment.speaker || "Unknown";
        const mappedSpeaker = speakerMap.get(currentSpeaker) || "Unknown";
        if (mappedSpeaker !== currentSpeaker) {
            segmentsChanged = true;
            return { ...segment, speaker: mappedSpeaker };
        }
        return segment;
    });

    if (segmentsChanged) {
        pushToUndoStack();
    }

    transcriptStore.update((ts) => ({
        ...ts,
        speakers: newSpeakerConfig,
        segments: newSegments,
        transcriptDirty: ts.transcriptDirty || JSON.stringify(oldSpeakerConfig) !== JSON.stringify(newSpeakerConfig) || segmentsChanged,
    }));
    updateProjectStoreState({ statusMessage: 'Updating speaker configuration...' });

    const innerPayload = {
        project_xml_path: projectXmlPath,
        media_identifier: mediaIdentifier,
        count: newSpeakerConfig.count,
        names: newSpeakerConfig.names,
        translated_names: newSpeakerConfig.translatedNames
    };

    invoke('save_speaker_config', { payload: innerPayload })
        .then(() => {
            updateProjectStoreState({ statusMessage: 'Speaker configuration saved.', error: null });

            projectMainStore.update(p => {
                 const updatedFiles = JSON.parse(JSON.stringify(p.files));
                 function findAndUpdateMediaSpeakers(nodes, targetIdentifier, newSpeakerData) {
                     if (!Array.isArray(nodes)) return false;
                     let found = false;
                     for (const node of nodes) {
                         if (node.media_xml_identifier === targetIdentifier && (node.file_type === 'media' || node.file_type === 'directory_media_stem')) {
                             node.speakers = {
                                 '@count': newSpeakerData.count,
                                 name: newSpeakerData.names,
                                 translated_names: newSpeakerData.translatedNames
                             };
                             found = true;
                         }
                         if (node.children && node.children.length > 0) {
                             if (findAndUpdateMediaSpeakers(node.children, targetIdentifier, newSpeakerData)) {
                                 found = true;
                             }
                         }
                     }
                     return found;
                 }
                 const didUpdate = findAndUpdateMediaSpeakers(updatedFiles, mediaIdentifier, newSpeakerConfig);
                 if (didUpdate) {
                     return { ...p, files: updatedFiles };
                 } else {
                     console.warn("[TranscriptStore via projectMainStore] Could not find media identifier in project.files tree to update speakers.");
                     return p;
                 }
            });

        })
        .catch((error) => {
            console.error(`[TranscriptStore updateSpeakerConfig] Failed persist config for ${mediaIdentifier}:`, error);
            const errorMessage = error?.message || String(error);
            updateProjectStoreState({ error: `Failed save speaker config: ${errorMessage}`, statusMessage: 'Error saving speaker config.'});
            if (typeof message !== 'undefined') {
                message(`Error saving speaker settings: ${errorMessage}`, {title: "Save Error", type: "error"});
            }
        });
}

export function setAudioBuffer(buffer, peaks) {
    transcriptStore.update((ts) => ({ ...ts, audioBuffer: buffer, audioBufferPeaks: peaks }));
}



export function toggleTranscribeModal(show) {
    transcriptStore.update((ts) => ({ ...ts, showTranscribeModal: !!show }));
}

export function setTranscriptionStatus(isTranscribing, jobIdToSet = null, options = {}) {
    console.log(`[JULES-DEBUG TS setStatus] Called with: isTranscribing=${isTranscribing}, jobIdToSet=${jobIdToSet}, options=`, options);
    const {
        initialProgressMessage = '',
        mediaPath = null,
        status = null,
        errorMessage = null
    } = options;

    transcriptStore.update((ts) => {
        let updatedState = { ...ts };

        if (isTranscribing) {
            if (jobIdToSet && ts.transcriptionJobId === jobIdToSet &&
                (ts.transcriptionJobStatus === 'done' || ts.transcriptionJobStatus === 'error' || ts.transcriptionJobStatus === 'cancelled')) {
                console.warn(`[JULES-DEBUG TS setStatus] Attempted to set job ${jobIdToSet} to active, but it's already in terminal state: ${ts.transcriptionJobStatus}. Ignoring.`);
                return ts;
            }

            const newActiveMediaDuringStart = mediaPath || ts.selectedMediaFile?.path || ts.activeMediaDuringTranscriptionStart;
            const jobStatusToSet = status || (jobIdToSet ? 'running' : 'initiating');
            const messageToSet = initialProgressMessage || (jobStatusToSet === 'initiating' ? `Initiating...` : `Processing...`);
            
            // Set start time if starting fresh, otherwise keep existing
            const startTime = (!ts.isTranscribing || !ts.transcriptionStartTime) ? Date.now() : ts.transcriptionStartTime;

            updatedState = {
                ...ts,
                isTranscribing: true,
                transcriptionJobId: jobIdToSet !== null ? jobIdToSet : ts.transcriptionJobId,
                transcriptionStartTime: startTime,
                mediaPathForLastJob: mediaPath || ts.mediaPathForLastJob,
                activeMediaDuringTranscriptionStart: newActiveMediaDuringStart,
                transcriptionProgress: {
                    percent: (jobStatusToSet === 'running' && ts.transcriptionJobId === jobIdToSet && ts.transcriptionJobId !== null) ? ts.transcriptionProgress.percent : 0,
                    message: messageToSet
                },
                transcriptionJobStatus: jobStatusToSet,
                transcriptionErrorMessage: null,
                ranInBackground: false,
                showTranscribeModal: true,
            };
        } else {
            const currentJobStatus = status || ts.transcriptionJobStatus;
            let newShowModalConfig = ts.showTranscribeModal;

            if (currentJobStatus === 'done') {
                newShowModalConfig = ts.ranInBackground ? false : true;
            } else if (currentJobStatus === 'error' || currentJobStatus === 'cancelled') {
                newShowModalConfig = true;
            } else if (currentJobStatus === null) {
                newShowModalConfig = false;
            }

            updatedState = {
                ...ts,
                isTranscribing: false,
                transcriptionJobStatus: currentJobStatus,
                transcriptionErrorMessage: errorMessage || ts.transcriptionErrorMessage,
                showTranscribeModal: newShowModalConfig,
            };

            if (currentJobStatus === null) {
                updatedState.transcriptionJobId = null;
                updatedState.activeMediaDuringTranscriptionStart = null;
                updatedState.mediaPathForLastJob = null;
                updatedState.transcriptionProgress = { percent: 0, message: '' };
                updatedState.ranInBackground = false;
                updatedState.transcriptionStartTime = null;
            }
        }
        console.log(`[JULES-DEBUG TS setStatus Updated] Store updated. New jobStatus=${updatedState.transcriptionJobStatus}, new jobId=${updatedState.transcriptionJobId}, progressMsg='${updatedState.transcriptionProgress.message}', showModal=${updatedState.showTranscribeModal}`);
        return updatedState;
    });

    if (isTranscribing) {
        updateProjectStoreState({ error: null });
    }
}

export function updateTranscriptionProgress(progressPayload) {
    transcriptStore.update((ts) => {
        const eventJobId = progressPayload?.jobId;

        if (!eventJobId) {
            return ts;
        }

        if (ts.isTranscribing && ts.transcriptionJobStatus === 'initiating' && ts.transcriptionJobId === null) {
            return {
                ...ts,
                transcriptionJobId: eventJobId,
                transcriptionJobStatus: 'running',
                transcriptionProgress: {
                    percent: progressPayload?.percent ?? 0,
                    message: progressPayload?.message ?? ''
                },
            };
        }
        else if (ts.isTranscribing && ts.transcriptionJobStatus === 'running' && ts.transcriptionJobId === eventJobId) {
            return {
                ...ts,
                transcriptionProgress: {
                    percent: progressPayload?.percent ?? 0,
                    message: progressPayload?.message ?? ''
                },
            };
        } else {
            return ts;
        }
    });
}

export function clearTranscriptionStatus(finalStatusMessage = 'Ready', error = null) {
    transcriptStore.update(ts => {
        console.log(`[JULES-DEBUG TS clearStatus] Called. Current store before clear: isTranscribing=${ts.isTranscribing}, jobId=${ts.transcriptionJobId}, jobStatus=${ts.transcriptionJobStatus}`);
        return {
            ...ts,
            isTranscribing: false,
            activeMediaDuringTranscriptionStart: null,
            transcriptionStartTime: null,
        };
    });
    updateProjectStoreState({ statusMessage: finalStatusMessage, error: error });
}

export function prepareForNewTranscription() {
    transcriptStore.update(ts => {
        console.log('[JULES-DEBUG TS prepareNew] Called. Resetting transcription states and showing modal.');
        return {
            ...ts,
            isTranscribing: false,
            transcriptionJobId: null,
            transcriptionProgress: { percent: 0, message: '' },
            transcriptionJobStatus: null,
            transcriptionErrorMessage: null,
            showTranscribeModal: true,
            transcriptionStartTime: null
        };
    });
}

export function clearPendingTranscriptData() {
    transcriptStore.update(ts => {
        if (ts.pendingTranscriptPathForJobDone !== null || ts.pendingSegmentsForJobDone !== null) {
            console.log('[TranscriptStore] Clearing pending transcript data (path and segments).');
            return {
                ...ts,
                pendingTranscriptPathForJobDone: null,
                pendingSegmentsForJobDone: null
            };
        }
        return ts;
    });
}

export function setRanInBackground(value) {
    transcriptStore.update((ts) => ({ ...ts, ranInBackground: !!value }));
}

export async function loadInitialTranscript(mediaFileEntry, transcriptPathToPrioritize = null) {
    transcriptStore.update(ts => ({ ...ts, isTranscriptLoading: true, segments: [], activeTranscript: null, currentTranscriptPath: null, transcriptDirty: false }));
    updateProjectStoreState({ statusMessage: `Loading transcripts for ${mediaFileEntry.name}...` });
    get(transcriptStore).segments = [];

    const associatedTranscripts = mediaFileEntry.associated_transcripts || [];
    console.log('[TranscriptStore loadInitialTranscript] mediaFileEntry.associated_transcripts:', associatedTranscripts);
    if (associatedTranscripts.length === 0) {
        transcriptStore.update(ts => ({ ...ts, isTranscriptLoading: false }));
        updateProjectStoreState({ statusMessage: `No transcripts found for ${mediaFileEntry.name}.` });
        return;
    }

    // Prioritize loading: 1. 'original', 2. 'en', 3. first in list
    const sortedTranscripts = [...associatedTranscripts].sort((a, b) => {
        const langA = a.language_code || (a.path.endsWith('.en.json') ? 'en' : 'original');
        const langB = b.language_code || (b.path.endsWith('.en.json') ? 'en' : 'original');
        if (langA === 'original') return -1;
        if (langB === 'original') return 1;
        if (langA === 'en') return -1;
        if (langB === 'en') return 1;
        const nameA = a.name || a.path || '';
        const nameB = b.name || b.path || '';
        return nameA.localeCompare(nameB);
    });

    let transcriptToLoad = null;
    if (transcriptPathToPrioritize) {
        transcriptToLoad = associatedTranscripts.find(t => t.path === transcriptPathToPrioritize);
    }

    if (!transcriptToLoad && sortedTranscripts.length > 0) {
        transcriptToLoad = sortedTranscripts[0];
    }

    if (transcriptToLoad) {
        try {
            await switchTranscript(transcriptToLoad.path);
        } catch (e) {
            console.error(`[TranscriptStore loadInitialTranscript] Failed to load prioritized transcript ${transcriptToLoad.path}:`, e);
            // Try to load the next available transcript if the prioritized one fails
            const remainingTranscripts = sortedTranscripts.filter(t => t.path !== transcriptToLoad.path);
            if (remainingTranscripts.length > 0) {
                console.warn('[TranscriptStore loadInitialTranscript] Trying next available transcript.');
                await switchTranscript(remainingTranscripts[0].path);
            } else {
                console.error('[TranscriptStore loadInitialTranscript] No other transcripts available to load.');
                transcriptStore.update(ts => ({ ...ts, isTranscriptLoading: false, transcriptErrorMessage: `Failed to load any transcript for ${mediaFileEntry.name}.` }));
                updateProjectStoreState({ statusMessage: `Error loading transcripts for ${mediaFileEntry.name}.`, error: `Failed to load any transcript.` });
            }
        }
    } else {
        console.warn('[TranscriptStore loadInitialTranscript] No transcript found to load, even after prioritization and sorting.');
        transcriptStore.update(ts => ({ ...ts, isTranscriptLoading: false }));
        updateProjectStoreState({ statusMessage: `No transcripts found for ${mediaFileEntry.name}.` });
    }
}

function remapSegmentSpeakerNames(segmentsToRemap, speakerConfig, targetSpeakerNames = null) {
    const userNames = targetSpeakerNames && targetSpeakerNames.length > 0
                      ? targetSpeakerNames
                      : (speakerConfig && Array.isArray(speakerConfig.names) ? speakerConfig.names : []);

    if (userNames.length === 0) {
        return segmentsToRemap.map(seg => ({ ...seg }));
    }

    return segmentsToRemap.map(seg => {
        const newSegment = { ...seg };
        const originalSpeaker = newSegment.speaker ? String(newSegment.speaker).trim() : "Unknown";

        let userAssignedIndex = -1;

        if (originalSpeaker.toUpperCase().startsWith("SPEAKER_")) {
            const numStr = originalSpeaker.substring("SPEAKER_".length);
            const parsedNum = parseInt(numStr, 10);
            if (!isNaN(parsedNum)) {
                userAssignedIndex = parsedNum;
            }
        } else if (originalSpeaker.toLowerCase().startsWith("speaker_")) {
            const numStr = originalSpeaker.substring("speaker_".length);
            const parsedNum = parseInt(numStr, 10);
            if (!isNaN(parsedNum) && parsedNum > 0) {
                userAssignedIndex = parsedNum - 1;
            }
        }

        if (userAssignedIndex >= 0 && userAssignedIndex < userNames.length) {
            if (userNames[userAssignedIndex] && userNames[userAssignedIndex].trim() !== "") {
                newSegment.speaker = userNames[userAssignedIndex].trim();
            }
        } else {
            if (!userNames.includes(originalSpeaker) && originalSpeaker !== "Unknown") {
            }
        }
        return newSegment;
    });
}

export async function switchTranscript(path) {
    const store = get(transcriptStore);
    if (store.activeTranscript?.path === path) {
        return; // Already active
    }

    transcriptStore.update(ts => ({ ...ts, isTranscriptLoading: true }));

    try {
        const projectService = await import('../services/projectService.js');
        const normalizedPath = normalizePath(path);
        const jsonString = await invoke('load_transcript_json', { transcriptPath: normalizedPath });
        const segments = projectService.parseLexicalTableToSegments(jsonString);
        setTranscriptData(path, segments); // This will handle remapping speakers and updating the store
    } catch (e) {
        console.error(`[TranscriptStore] Failed to load transcript from ${path}:`, e);
        updateProjectStoreState({ error: `Failed to load transcript: ${e.message || e}` });
        transcriptStore.update(ts => ({ ...ts, isTranscriptLoading: false }));
    }
}


listen('media_renamed', (event) => {
    if (!event.payload) return;

    const { old_media_stem, new_media_stem, new_media_file_relative_path: rawRelativePath, new_absolute_path: rawAbsolutePath } = event.payload;
    const new_media_file_relative_path = normalizePath(rawRelativePath);
    const new_absolute_path = normalizePath(rawAbsolutePath);

    transcriptStore.update(ts => {
        if (ts.selectedMediaFile && ts.selectedMediaFile.media_xml_identifier === old_media_stem) {
            const newFileName = new_absolute_path.split(/[\/]/).pop();
            return {
                ...ts,
                selectedMediaFile: {
                    ...ts.selectedMediaFile,
                    name: newFileName,
                    path: new_absolute_path,
                    relative_path: new_media_file_relative_path,
                    media_xml_identifier: new_media_stem,
                },
            };
        }
        return ts;
    });
});

listen('custom_transcription_job_completed', async (event) => {
    if (!event.payload) return;
    const { status, jobFinishedPath: rawJobFinishedPath, transcriptFilePath: rawTranscriptFilePath, translatedTranscriptFilePath: rawTranslatedTranscriptFilePath, errorMessage } = event.payload;
    const jobFinishedPath = normalizePath(rawJobFinishedPath);
    const transcriptFilePath = normalizePath(rawTranscriptFilePath);
    const translatedTranscriptFilePath = normalizePath(rawTranslatedTranscriptFilePath);
    
    const currentStore = get(transcriptStore);



    if (currentStore.isTranscribing && jobFinishedPath === currentStore.mediaPathForLastJob) {
        const wasModalVisibleAtEventTime = currentStore.showTranscribeModal;
        const wasJobRunInBackground = currentStore.ranInBackground;

        const shouldShowToastNotification = wasJobRunInBackground || !wasModalVisibleAtEventTime;

        let finalProgressMessage = '';
        const updatePayload = {};
        let activePathToLoad = null;
        const pathUpdates = {};

        switch (status) {
            case 'done':
                const outputFilename = getFilename(transcriptFilePath || translatedTranscriptFilePath);
                finalProgressMessage = `Transcription successful: ${outputFilename}`;
                if (shouldShowToastNotification) {
                    notificationManager.add(finalProgressMessage, "success", 0);
                }
                updatePayload.transcriptionOutputFileName = outputFilename;
                updatePayload.transcriptionJobStatus = 'done';
                updatePayload.transcriptionErrorMessage = null;
                updatePayload.isTranscribing = false;

                if (currentStore.selectedMediaFile?.path === jobFinishedPath) {
                    let newOriginalPath = currentStore.originalTranscriptPath;
                    let newEnglishPath = currentStore.englishTranscriptPath;

                    // Determine which transcript to load after job completion.
                    // Prioritize the one matching the user's last active language if possible,
                    // otherwise default to the original, then the translation.
                    const currentActiveLangCode = currentStore.activeTranscript?.language_code;

                    if (currentActiveLangCode === 'en' && translatedTranscriptFilePath) {
                        activePathToLoad = translatedTranscriptFilePath;
                    } else if (transcriptFilePath) {
                        activePathToLoad = transcriptFilePath;
                    } else if (translatedTranscriptFilePath) {
                        activePathToLoad = translatedTranscriptFilePath;
                    }
                    if (Object.keys(pathUpdates).length > 0) {
                        Object.assign(updatePayload, pathUpdates);
                    }
                }
                break;
            case 'error':
                finalProgressMessage = `Transcription failed: ${errorMessage || 'Unknown error'}`;
                if (shouldShowToastNotification) {
                    notificationManager.add(finalProgressMessage, "error", 0);
                }
                updatePayload.transcriptionJobStatus = 'error';
                updatePayload.transcriptionErrorMessage = errorMessage;
                updatePayload.isTranscribing = false;
                break;
            case 'cancelled':
                finalProgressMessage = "Transcription cancelled";
                if (shouldShowToastNotification) {
                    notificationManager.add(finalProgressMessage, "info", 0);
                }
                updatePayload.transcriptionJobStatus = 'cancelled';
                updatePayload.transcriptionErrorMessage = null;
                updatePayload.isTranscribing = false;
                break;
            default:
                console.warn(`[TranscriptStore] Unknown status in custom_transcription_job_completed: ${status}`);
                return;
        }

        updatePayload.showTranscribeModal = shouldShowToastNotification ? false : true;
        updatePayload.transcriptionProgress = { ...currentStore.transcriptionProgress, message: finalProgressMessage };

        transcriptStore.update(ts => ({ ...ts, ...updatePayload }));

        if (status === 'done' && currentStore.selectedMediaFile?.path === jobFinishedPath) {
            try {
                const service = await import('../services/projectService.js');
                if (service.refreshProjectFiles && currentStore.selectedMediaFile?.path) {
                    console.log('[TranscriptStore] Refreshing project files to update transcript associations.');
                    await service.refreshProjectFiles(currentStore.selectedMediaFile.path, activePathToLoad);

                    // Force an update to the Data tab's active transcript path so "No Transcription Yet" is cleared
                    if (activePathToLoad) {
                        projectMainStore.update(p => ({
                            ...p,
                            activeTranscriptPathInDataTab: activePathToLoad,
                            mediaNoteTranscriptError: null
                        }));
                    }

                    const latestProjectStore = get(projectMainStore);
                    const allFiles = latestProjectStore.files;
                    const mediaPath = jobFinishedPath;

                    let updatedMediaFile = null;

                    function findMediaNodeByPath(nodes, path) {
                        if (!Array.isArray(nodes)) return null;
                        for (const node of nodes) {
                            if (node.path === path && node.file_type === 'media' && !node.is_directory) {
                                return node;
                            }
                            if (node.children && node.children.length > 0) {
                                const found = findMediaNodeByPath(node.children, path);
                                if (found) {
                                    return found;
                                }
                            }
                        }
                        return null;
                    }

                    updatedMediaFile = findMediaNodeByPath(allFiles, mediaPath);

                    if (updatedMediaFile) {
                        const { emit } = await import('@tauri-apps/api/event');
                        if (!wasJobRunInBackground) {
                            emit('select_media_in_transcription_tab', { mediaPath: updatedMediaFile.path });
                        }
                    } else {
                        console.warn(`[TranscriptStore] Could not find the updated media file in project store after refresh for path: ${mediaPath}`);
                    }
                }
            } catch (e) {
                console.error('[TranscriptStore] Error refreshing project files after job completion:', e);
            }
        }

    } else {
         
         return;
    }
});


listen('translation_job_completed', async (event) => {
    if (!event.payload) return;
    const { jobId, status, originalTranscriptPath: rawOriginalTranscriptPath, newTranscriptPath: rawNewTranscriptPath, errorMessage } = event.payload;
    const originalTranscriptPath = normalizePath(rawOriginalTranscriptPath);
    const newTranscriptPath = normalizePath(rawNewTranscriptPath);
    
    const currentStore = get(transcriptStore);

    if (currentStore.isTranslating && jobId === currentStore.translationJobId) {
        const wasModalVisibleAtEventTime = currentStore.showTranslateModal;
        const wasJobRunInBackground = currentStore.ranTranslationInBackground;
        const shouldShowToastNotification = wasJobRunInBackground || !wasModalVisibleAtEventTime;

        let finalProgressMessage = '';
        const updatePayload = {};

        switch (status) {
            case 'done':
                const outputFilenameTransl = getFilename(newTranscriptPath);
                finalProgressMessage = `Translation successful: ${outputFilenameTransl}`;
                if (shouldShowToastNotification) {
                    notificationManager.add(finalProgressMessage, "success", 0);
                }
                updatePayload.translationOutputFileName = outputFilenameTransl;
                updatePayload.translationJobStatus = 'done';
                updatePayload.translationErrorMessage = null;
                updatePayload.isTranslating = false;
                updatePayload.ranTranslationInBackground = false;
                updatePayload.translationSourcePath = null;
                break;
            case 'error':
                finalProgressMessage = `Translation failed: ${errorMessage || 'Unknown error'}`;
                if (shouldShowToastNotification) {
                    notificationManager.add(finalProgressMessage, "error", 0);
                }
                updatePayload.translationJobStatus = 'error';
                updatePayload.translationErrorMessage = errorMessage;
                updatePayload.isTranslating = false;
                updatePayload.ranTranslationInBackground = false;
                updatePayload.translationSourcePath = null;
                break;
            case 'cancelled':
                finalProgressMessage = "Translation cancelled";
                if (shouldShowToastNotification) {
                    notificationManager.add(finalProgressMessage, "info", 0);
                }
                updatePayload.translationJobStatus = 'cancelled';
                updatePayload.translationErrorMessage = null;
                updatePayload.isTranslating = false;
                updatePayload.ranTranslationInBackground = false;
                updatePayload.translationSourcePath = null;
                break;
            default:
                console.warn(`[TranscriptStore] Unknown status in translation_job_completed: ${status}`);
                return;
        }

        updatePayload.showTranslateModal = shouldShowToastNotification ? false : true;
        updatePayload.translationProgress = { ...currentStore.translationProgress, message: finalProgressMessage };

        transcriptStore.update(ts => ({ ...ts, ...updatePayload }));

        if (status === 'done') {
            try {
                const service = await import('../services/projectService.js');
                if (service.refreshProjectFiles) {
                    console.log('[TranscriptStore] Refreshing project files after translation completion.');
                    const mediaPath = currentStore.selectedMediaFile?.path;
                    await service.refreshProjectFiles(mediaPath, newTranscriptPath);

                    if (mediaPath) {
                        // After refreshing project files, re-select the media to ensure transcriptStore is updated
                        // with the newly available translated transcript.
                        const latestProjectStore = get(projectMainStore);
                        const allFiles = latestProjectStore.files;

                        let updatedMediaFile = null;

                        function findMediaNodeByPath(nodes, path) {
                            if (!Array.isArray(nodes)) return null;
                            for (const node of nodes) {
                                if (node.path === path && node.file_type === 'media' && !node.is_directory) {
                                    return node;
                                }
                                if (node.children && node.children.length > 0) {
                                    const found = findMediaNodeByPath(node.children, path);
                                    if (found) {
                                        return found;
                                    }
                                }
                            }
                            return null;
                        }

                        updatedMediaFile = findMediaNodeByPath(allFiles, mediaPath);

                        if (updatedMediaFile) {
                            console.log('[TranscriptStore] Re-selecting media after translation completion to update associated transcripts.', updatedMediaFile);
                            selectMedia(updatedMediaFile, newTranscriptPath);
                        } else {
                            console.warn(`[TranscriptStore] Could not find the updated media file in project store after translation refresh for path: ${mediaPath}`);
                        }
                    }
                }
            } catch (e) {
                console.error('[TranscriptStore] Error refreshing project files after translation completion:', e);
            }
        }
    }
});

export function setTranslateToEnglish(value) {
    transcriptStore.update(ts => {
        if (ts.translateToEnglish !== !!value) {
            return { ...ts, translateToEnglish: !!value };
        }
        return ts;
    });
}

export function setDiarizationPreference(value) {
    transcriptStore.update(ts => ({ ...ts, diarizationEnabledForNextJob: !!value }));
}

export function setSpeakerConfig(newSpeakerConfig) {
    transcriptStore.update((ts) => ({
        ...ts,
        speakers: newSpeakerConfig,
    }));
}

// --- Compare two transcripts in interleaved mode Functions ---

export function setDualTranscriptModal(show) {
    transcriptStore.update(ts => ({ ...ts, showDualTranscriptModal: !!show }));
}

export async function activateDualMode(primaryPath, secondaryPath) {
    console.log('[TranscriptStore] activateDualMode:', primaryPath, secondaryPath);
    
    // Set loading state or just proceed if loadTranscriptFile handles it
    transcriptStore.update(ts => ({
        ...ts,
        showDualTranscriptModal: false
    }));

    try {
        const projectService = await import('../services/projectService.js');
        
        // Use existing functions to load both. 
        // These expect ABSOLUTE paths when called directly like this if the backend needs them.
        await projectService.loadTranscriptFile(primaryPath);
        await setSecondaryTranscript(secondaryPath);

        // ONLY after successful load of BOTH, set dual mode active
        transcriptStore.update(ts => ({
            ...ts,
            isDualModeActive: true
        }));

        if (typeof window !== 'undefined') {
            localStorage.setItem(DUAL_MODE_STORAGE_KEY, JSON.stringify(true));
        }
    } catch (e) {
        console.error('[TranscriptStore] Error activating dual mode:', e);
        
        // Extract a readable error message
        let errorMsg = 'Unknown error';
        if (typeof e === 'string') errorMsg = e;
        else if (e instanceof Error) errorMsg = e.message;
        else if (e && typeof e === 'object' && e.message) errorMsg = e.message;
        
        message(`Failed to activate dual mode: ${errorMsg}`, { title: 'Error', type: 'error' });
        
        // Ensure dual mode is OFF on failure
        transcriptStore.update(ts => ({
            ...ts,
            isDualModeActive: false,
            secondaryTranscriptPath: null,
            secondaryTranscriptSegments: []
        }));
    }
}

export async function deactivateDualMode() {
    const store = get(transcriptStore);
    if (store.transcriptDirty) {
        // We should ideally attempt to save here as requested: "after saving changes (if there is anything to be saved)"
        try {
            const projectService = await import('../services/projectService.js');
            await projectService.saveTranscriptData();
        } catch (e) {
            const confirmed = await confirm('Failed to save changes. Deactivate Dual Mode anyway? Changes will be lost.', { title: 'Save Failed', type: 'warning' });
            if (!confirmed) return;
        }
    }

    if (typeof window !== 'undefined') {
        try {
            localStorage.setItem(DUAL_MODE_STORAGE_KEY, JSON.stringify(false));
        } catch (error) {
            console.error('[TranscriptStore] Error saving dual mode state to localStorage:', error);
        }
    }

    transcriptStore.update(ts => ({
        ...ts,
        isDualModeActive: false,
        secondaryTranscriptPath: null,
        secondaryTranscriptSegments: [],
        transcriptDirty: false, // Reset dirty after save/discard
        transcriptUndoStack: [],
        transcriptRedoStack: []
    }));
}

export async function setSecondaryTranscript(path) {
    if (!path) {
        transcriptStore.update(ts => ({
            ...ts,
            secondaryTranscriptPath: null,
            secondaryTranscriptSegments: [],
        }));
        return;
    }

    const store = get(transcriptStore);
    const normalizedInputPath = normalizePath(path);
    
    // Find the transcript info from the project tree to get the canonical normalized path
    const transcriptInfo = store.selectedMediaFile?.associated_transcripts?.find(t => {
        return t.path === normalizedInputPath || t.relativePath === normalizedInputPath;
    });

    const targetPath = transcriptInfo ? transcriptInfo.path : normalizedInputPath;

    if (store.secondaryTranscriptPath === targetPath) {
        return; // Already selected
    }

    // Automatically switch the primary if the user selects the same one
    if (store.currentTranscriptPath === targetPath) {
        const otherTranscripts = store.selectedMediaFile?.associated_transcripts?.filter(t => t.path !== targetPath) || [];
        if (otherTranscripts.length > 0) {
            await switchTranscript(otherTranscripts[0].path);
        } else {
            console.error("[TranscriptStore] Cannot switch primary away from secondary: no other transcripts available.");
            return;
        }
    }


    try {
        const projectService = await import('../services/projectService.js');
        const jsonString = await invoke('load_transcript_json', { transcriptPath: targetPath });
        const segments = projectService.parseLexicalTableToSegments(jsonString);

        const primarySegments = get(transcriptStore).segments;
        if (primarySegments.length !== segments.length) {
            message('The number of segments between the two transcripts are not the same. Please select a different pair or generate again.', { title: 'Segment Mismatch', type: 'error' });
            return; // Don't set the transcript
        }


        transcriptStore.update(ts => ({
            ...ts,
            secondaryTranscriptPath: targetPath,
            secondaryTranscriptSegments: segments,
        }));
    } catch (e) {
        console.error(`[TranscriptStore] Failed to load secondary transcript from ${targetPath}:`, e);
        updateProjectStoreState({ error: `Failed to load transcript: ${e.message || e}` });
        transcriptStore.update(ts => ({ ...ts, secondaryTranscriptPath: null, secondaryTranscriptSegments: [] }));
    }
}


export function toggleDualMode(active) {
    const store = get(transcriptStore);
    if (store.transcriptDirty) {
        message('Please save your changes before enabling or disabling Compare Transcripts.', { title: 'Unsaved Changes', type: 'error' });
        return;
    }

    if (typeof window !== 'undefined') {
        try {
            localStorage.setItem(DUAL_MODE_STORAGE_KEY, JSON.stringify(active));
        } catch (error) {
            console.error('[TranscriptStore] Error saving dual mode state to localStorage:', error);
        }
    }

    transcriptStore.update(ts => ({
        ...ts,
        isDualModeActive: active,
        secondaryTranscriptPath: active ? ts.secondaryTranscriptPath : null,
        secondaryTranscriptSegments: active ? ts.secondaryTranscriptSegments : [],
    }));

    if (active) {
        const currentPrimaryPath = get(transcriptStore).currentTranscriptPath;
        const associatedTranscripts = get(transcriptStore).selectedMediaFile?.associated_transcripts || [];
        const otherTranscripts = associatedTranscripts.filter(t => t.path !== currentPrimaryPath);

        if (otherTranscripts.length > 0) {
            const primaryIndex = associatedTranscripts.findIndex(t => t.path === currentPrimaryPath);
            let nextIndex = (primaryIndex + 1) % associatedTranscripts.length;
            let nextTranscript = associatedTranscripts[nextIndex];

            if (nextTranscript.path === currentPrimaryPath) {
                 nextIndex = (nextIndex + 1) % associatedTranscripts.length;
                 nextTranscript = associatedTranscripts[nextIndex];
            }

            setSecondaryTranscript(nextTranscript.path);
        }
    }
}

// --- Translation Management Functions ---

export function toggleTranslateModal(show) {
    transcriptStore.update((ts) => ({ ...ts, showTranslateModal: !!show }));
}

export function setRanTranslationInBackground(value) {
    transcriptStore.update((ts) => ({ ...ts, ranTranslationInBackground: !!value }));
}

export function setTranslationStatus(isTranslating, jobIdToSet = null, options = {}) {
    const {
        status = null,
        errorMessage = null,
        sourcePath = null
    } = options;

    transcriptStore.update((ts) => {
        let updatedState = { ...ts };

        if (isTranslating) {
            const jobStatusToSet = status || (jobIdToSet ? 'running' : 'initiating');
            
            // Set start time if starting fresh, otherwise keep existing
            const startTime = (!ts.isTranslating || !ts.translationStartTime) ? Date.now() : ts.translationStartTime;

            updatedState = {
                ...ts,
                isTranslating: true,
                translationJobId: jobIdToSet !== null ? jobIdToSet : ts.translationJobId,
                translationStartTime: startTime,
                translationProgress: {
                    percent: 0,
                    message: 'Initiating translation...'
                },
                translationJobStatus: jobStatusToSet,
                translationErrorMessage: null,
                ranTranslationInBackground: false,
                showTranslateModal: true,
                translationSourcePath: sourcePath || ts.translationSourcePath,
            };
        } else {
            const currentJobStatus = status || ts.translationJobStatus;
            let newShowModalConfig = ts.showTranslateModal;

            if (currentJobStatus === 'done') {
                newShowModalConfig = ts.ranTranslationInBackground ? false : true;
            } else if (currentJobStatus === 'error' || currentJobStatus === 'cancelled') {
                newShowModalConfig = true;
            } else if (currentJobStatus === null) {
                newShowModalConfig = false;
            }

            updatedState = {
                ...ts,
                isTranslating: false,
                translationJobId: jobIdToSet !== null ? jobIdToSet : ts.translationJobId,
                translationJobStatus: currentJobStatus,
                translationErrorMessage: errorMessage || ts.translationErrorMessage,
                showTranslateModal: newShowModalConfig,
                translationSourcePath: (currentJobStatus === 'done' || currentJobStatus === 'cancelled' || currentJobStatus === 'error' || currentJobStatus === null) ? null : ts.translationSourcePath
            };

            if (currentJobStatus === null) {
                updatedState.translationJobId = null;
                updatedState.translationProgress = { percent: 0, message: '' };
                updatedState.ranTranslationInBackground = false;
                updatedState.translationStartTime = null;
            }
        }
        return updatedState;
    });

    if (isTranslating) {
        updateProjectStoreState({ error: null });
    }
}

export function updateTranslationProgress(progressPayload) {
    transcriptStore.update((ts) => {
        const eventJobId = progressPayload?.jobId;
        if (!eventJobId || !ts.isTranslating || ts.translationJobId !== eventJobId) {
            return ts;
        }

        return {
            ...ts,
            translationJobStatus: 'running',
            translationProgress: {
                percent: progressPayload?.percent ?? ts.translationProgress.percent,
                message: progressPayload?.message ?? ts.translationProgress.message
            },
        };
    });
}

export function clearTranslationStatus(finalStatusMessage = 'Ready', error = null) {
    transcriptStore.update(ts => {
        console.log(`[JULES-DEBUG TS clearTranslationStatus] Called. Current store before clear: isTranslating=${ts.isTranslating}, jobId=${ts.translationJobId}, jobStatus=${ts.translationJobStatus}`);
        return {
            ...ts,
            isTranslating: false,
            translationJobId: null,
            translationProgress: { percent: 0, message: '' },
            translationJobStatus: null,
            translationErrorMessage: null,
            ranInBackground: false,
            showTranslateModal: false,
            translationStartTime: null,
        };
    });
    updateProjectStoreState({ statusMessage: finalStatusMessage, error: error });
}
