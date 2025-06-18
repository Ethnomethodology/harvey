// src/lib/stores/transcriptStore.js
import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { message } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event'; // Added missing import for listen
// Import projectStore to access project-level details.
// This creates a partial cyclic dependency that we might want to resolve later
// by passing necessary values as arguments or through a service.
import { project as projectMainStore, updateProjectStoreState } from './projectStore.js';


export const initialTranscriptState = {
    segments: [],
    currentTranscriptPath: null,
    transcriptDirty: false,
    selectedMediaFile: null,
    selectedModelName: null,
    selectedLanguage: null,
    speakers: { count: 0, names: [], translatedNames: [] },
    player: { currentTime: 0, duration: 0, isPlaying: false, currentSegmentIndex: -1 },
    audioBuffer: null,
    isTranscriptLoading: false,
    isTranscribing: false,
    transcriptionProgress: { percent: 0, message: '' },
    transcriptionJobId: null,
    showTranscribeModal: false,
    mediaPathForLastJob: null, // Add this line
    activeMediaDuringTranscriptionStart: null,
    transcriptUndoStack: [],
    transcriptRedoStack: [],
    pendingTranscriptPathForJobDone: null,
    pendingSegmentsForJobDone: null,
    ranInBackground: false,
    transcriptionJobStatus: null, // Possible values: 'running', 'done', 'error', 'cancelled', null
    transcriptionErrorMessage: null, // Stores error message if any
    // Dual transcript additions
    translateToEnglish: false,
    activeTranscriptLanguage: 'original', // 'original' or 'english'
    originalSegments: [],
    englishSegments: [],
    originalTranscriptPath: null, // To store the path of the original language transcript
    englishTranscriptPath: null, // To store the path of the English language transcript
};

export const transcriptStore = writable({ ...initialTranscriptState });

export const MAX_UNDO_STACK_SIZE = 50;

// --- Transcript Management Functions ---

export function pushToUndoStack(currentSegments) {
    transcriptStore.update(ts => {
        const newUndoStack = [...ts.transcriptUndoStack, currentSegments];
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
        const currentSegments = ts.segments;
        const newUndoStack = [...ts.transcriptUndoStack];
        const previousSegments = newUndoStack.pop();
        const newRedoStack = [...ts.transcriptRedoStack, currentSegments];
        let newIndex = -1;
        const time = ts.player.currentTime;
        if (previousSegments.length > 0 && ts.player.duration > 0 && time >= 0) {
            const idx = previousSegments.findIndex((s, index) => {
                const isLastSegment = index === previousSegments.length - 1;
                const startTimeCheck = time >= (s.start_time - 0.001);
                const endTimeCheck = isLastSegment ? time <= s.end_time : time < s.end_time;
                return startTimeCheck && endTimeCheck;
            });
            newIndex = idx;
        }
        // Update global status message via projectStore for now
        updateProjectStoreState({ statusMessage: 'Undo successful.' });
        return {
            ...ts,
            segments: previousSegments,
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
        const currentSegments = ts.segments;
        const newRedoStack = [...ts.transcriptRedoStack];
        const nextSegments = newRedoStack.pop();
        const newUndoStack = [...ts.transcriptUndoStack, currentSegments];
        let newIndex = -1;
        const time = ts.player.currentTime;
        if (nextSegments.length > 0 && ts.player.duration > 0 && time >= 0) {
            const idx = nextSegments.findIndex((s, index) => {
                const isLastSegment = index === nextSegments.length - 1;
                const startTimeCheck = time >= (s.start_time - 0.001);
                const endTimeCheck = isLastSegment ? time <= s.end_time : time < s.end_time;
                return startTimeCheck && endTimeCheck;
            });
            newIndex = idx;
        }
        updateProjectStoreState({ statusMessage: 'Redo successful.' });
        return {
            ...ts,
            segments: nextSegments,
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
        if (ts.currentTranscriptPath || ts.segments.length > 0 || ts.transcriptDirty || ts.isTranscriptLoading || ts.transcriptUndoStack.length > 0 || ts.transcriptRedoStack.length > 0 || ts.selectedMediaFile) {
            updateProjectStoreState({ statusMessage: 'Media transcript cleared.' });
            return {
                ...ts,
                selectedMediaFile: null,
                segments: [],
                currentTranscriptPath: null,
                transcriptDirty: false,
                isTranscriptLoading: false,
                player: { currentTime: 0, duration: 0, isPlaying: false, currentSegmentIndex: -1 },
                audioBuffer: null,
                transcriptUndoStack: [],
                transcriptRedoStack: [],
                speakers: { count: 0, names: [], translatedNames: [] },
                activeMediaDuringTranscriptionStart: null, // Reset here as well
                pendingTranscriptPathForJobDone: null,
                pendingSegmentsForJobDone: null,
                // Reset dual transcript states
                activeTranscriptLanguage: 'original',
                originalSegments: [],
                englishSegments: [],
                originalTranscriptPath: null,
                englishTranscriptPath: null,
            };
        }
        return ts;
    });
}

export function selectMedia(fileEntry) {
    const currentSelectedPath = get(transcriptStore).selectedMediaFile?.path;
    const shouldUpdateSelection = (!fileEntry && currentSelectedPath !== null) || (fileEntry && currentSelectedPath !== fileEntry.path);

    let speakersToLoad = { count: 0, names: [], translatedNames: [] };
    if (fileEntry && fileEntry.file_type === 'media' && !fileEntry.is_directory && fileEntry.speakers && typeof fileEntry.speakers === 'object') {
        const loadedCount = Number(fileEntry.speakers['@count']) || 0;
        const loadedNamesRaw = fileEntry.speakers.name;
        const loadedNames = Array.isArray(loadedNamesRaw) ? loadedNamesRaw : (loadedNamesRaw ? [loadedNamesRaw] : []);

        speakersToLoad = {
            count: loadedCount,
            names: [...loadedNames],
            translatedNames: Array(loadedCount > 0 ? loadedCount : 0).fill('')
        };

        if (speakersToLoad.count !== speakersToLoad.names.length) {
            console.warn(`[TranscriptStore selectMedia] Discrepancy count/names for ${fileEntry.name}. Adjusting.`); // WARN
            speakersToLoad.count = speakersToLoad.names.length;
            // translatedNames should also be sliced if names were sliced, though this scenario implies data inconsistency.
            // For simplicity, if names.length is now the source of truth for count, translatedNames should match.
            speakersToLoad.names = speakersToLoad.names.slice(0, speakersToLoad.count);
            speakersToLoad.translatedNames = Array(speakersToLoad.count).fill('');
        }
    } else if (fileEntry && fileEntry.file_type === 'media' && !fileEntry.is_directory) {
        speakersToLoad = { count: 0, names: [], translatedNames: [] }; // Reset with translatedNames
    } else { // Handles null or non-media fileEntry
        speakersToLoad = { count: 0, names: [], translatedNames: [] }; // Reset with translatedNames
    }

    const currentStoreSpeakers = get(transcriptStore).speakers;
    const speakersChanged = JSON.stringify(currentStoreSpeakers) !== JSON.stringify(speakersToLoad);

    if (shouldUpdateSelection || speakersChanged) {
        const newSelectedMedia = fileEntry && !fileEntry.is_directory && fileEntry.file_type === 'media' ? fileEntry : null;
        if (newSelectedMedia && (!newSelectedMedia.name || !newSelectedMedia.path)) {
            console.error("[TranscriptStore] CRITICAL: Attempting set selectedMediaFile without name/path!", newSelectedMedia); // ERROR
        }
        if (newSelectedMedia && !newSelectedMedia.media_xml_identifier) {
            console.warn("[TranscriptStore] WARNING: Setting selectedMediaFile without media_xml_identifier! Saving might fail.", newSelectedMedia); // WARN
        }

        transcriptStore.update((ts) => ({
            ...ts,
            selectedMediaFile: newSelectedMedia,
            audioBuffer: null,
            player: { currentTime: 0, duration: 0, isPlaying: false, currentSegmentIndex: -1 },
            speakers: speakersToLoad,
            segments: [],
            currentTranscriptPath: null,
            transcriptDirty: false,
            isTranscriptLoading: false,
            transcriptUndoStack: [],
            transcriptRedoStack: [],
        }));

        const newlySelectedMedia = get(transcriptStore).selectedMediaFile;

        if (newlySelectedMedia && Array.isArray(newlySelectedMedia.associated_transcripts) && newlySelectedMedia.associated_transcripts.length > 0) {
            const firstTranscriptInfo = newlySelectedMedia.associated_transcripts[0];
            const firstTranscriptRelativePath = firstTranscriptInfo?.relativePath;

            if (firstTranscriptRelativePath && typeof firstTranscriptRelativePath === 'string') {
                const allFiles = get(projectMainStore).files; // Access files from projectMainStore
                let transcriptNodeToLoad = null;

                function findTranscriptNodeByRelativePath(nodes, relPath) {
                    if (!Array.isArray(nodes)) return null;
                    for (const node of nodes) {
                        if (node.file_type === 'transcript' && node.relative_path === relPath) {
                            return node;
                        }
                        if (node.children && node.children.length > 0) {
                            const found = findTranscriptNodeByRelativePath(node.children, relPath);
                            if (found) return found;
                        }
                    }
                    return null;
                }
                transcriptNodeToLoad = findTranscriptNodeByRelativePath(allFiles, firstTranscriptRelativePath);

                if (transcriptNodeToLoad && transcriptNodeToLoad.path) {
                    transcriptStore.update(ts => ({ ...ts, currentTranscriptPath: transcriptNodeToLoad.path, isTranscriptLoading: true }));
                    // Dynamic import of projectService to avoid circular dependencies at module load time
                    import('../services/projectService.js').then(service => {
                        if (typeof service.loadTranscriptFile === 'function') {
                            service.loadTranscriptFile(transcriptNodeToLoad.path) // This function will call setTranscriptData
                                .catch(error => {
                                    console.error(`[TranscriptStore] Auto-load first transcript failed:`, error); // ERROR
                                    transcriptStore.update(ts => ({...ts, isTranscriptLoading: false}));
                                    updateProjectStoreState({ error: `Failed to load transcript: ${error.message || error}`});
                                });
                        } else {
                            console.error("[TranscriptStore] loadTranscriptFile function not found in service."); // ERROR
                            transcriptStore.update(ts => ({...ts, isTranscriptLoading: false}));
                            updateProjectStoreState({ error: "Internal error: Transcript loading service unavailable."});
                        }
                    }).catch(err => {
                        console.error("[TranscriptStore] Failed import projectService for transcript load:", err); // ERROR
                        transcriptStore.update(ts => ({...ts, isTranscriptLoading: false}));
                        updateProjectStoreState({ error: "Internal error: Failed to import project service."});
                    });
                } else {
                    console.warn(`[TranscriptStore selectMedia] Could not find FileEntry node for first transcript relative path: ${firstTranscriptRelativePath}`); // WARN
                }
            } else {
                console.warn(`[TranscriptStore selectMedia] First associated transcript entry exists but lacks a valid 'relativePath' property. Entry:`, firstTranscriptInfo); // WARN
            }
        }
    }
}

export function updatePlayerTime(time) {
    transcriptStore.update((ts) => {
        let newIndex = -1;
        if (ts.segments.length > 0 && ts.player.duration > 0 && time >= 0) {
            const idx = ts.segments.findIndex((s, index) => {
                const isLastSegment = index === ts.segments.length - 1;
                const startTimeCheck = time >= (s.start_time - 0.001);
                const endTimeCheck = isLastSegment ? time <= s.end_time : time < s.end_time;
                return startTimeCheck && endTimeCheck;
            });
            newIndex = idx;
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
    const newSegments = Array.isArray(data) ? data : [];
    transcriptStore.update((ts) => {
        let updatedSpeakers = ts.speakers;
        if (inferSpeakers) {
            console.warn('[TranscriptStore] Speaker inference requested. Overwriting current.');
            let inferredSpeakers = { count: 0, names: [] };
            if (newSegments.length > 0) {
                const uniqueSpeakers = [...new Set(newSegments.map(s => s.speaker || 'Unknown'))];
                const knownSpeakers = uniqueSpeakers.filter(s => s && s !== 'Unknown');
                if (knownSpeakers.length > 0) {
                    knownSpeakers.sort((a, b) => a.localeCompare(b, undefined, {numeric: true, sensitivity: 'base'}));
                    inferredSpeakers = { count: knownSpeakers.length, names: knownSpeakers };
                } else {
                    inferredSpeakers = { count: 0, names: [] };
                }
            }
            updatedSpeakers = inferredSpeakers;
        }
        updateProjectStoreState({ statusMessage: path ? `Media transcript loaded.` : 'Media transcript cleared.', error: null });
        return {
            ...ts,
            currentTranscriptPath: path,
            segments: newSegments,
            transcriptDirty: false,
            isTranscriptLoading: false,
            speakers: updatedSpeakers,
            player: { ...ts.player, currentSegmentIndex: -1 }, // Reset segment index
            transcriptUndoStack: [],
            transcriptRedoStack: [],
        };

        // START: MODIFIED to update original/english specific stores
        let updatedOriginalSegments = ts.originalSegments;
        let updatedEnglishSegments = ts.englishSegments;
        let updatedOriginalTranscriptPath = ts.originalTranscriptPath;
        let updatedEnglishTranscriptPath = ts.englishTranscriptPath;
        let newActiveTranscriptLanguage = ts.activeTranscriptLanguage;

        // Determine if loading original or English based on path suffix,
        // or if the path explicitly matches one of the stored specific paths.
        if (path && (path === ts.originalTranscriptPath || (path.endsWith('.json') && !path.endsWith('.en.json')))) {
            updatedOriginalSegments = newSegments;
            updatedOriginalTranscriptPath = path;
            // If this is the one being actively loaded, set active language
            if (ts.currentTranscriptPath === path) {
                newActiveTranscriptLanguage = 'original';
            }
        } else if (path && (path === ts.englishTranscriptPath || path.endsWith('.en.json'))) {
            updatedEnglishSegments = newSegments;
            updatedEnglishTranscriptPath = path;
            // If this is the one being actively loaded, set active language
            if (ts.currentTranscriptPath === path) {
                newActiveTranscriptLanguage = 'english';
            }
        }

        return {
            ...ts,
            currentTranscriptPath: path, // This is the actively displayed transcript path
            segments: newSegments,       // These are the actively displayed segments
            originalSegments: updatedOriginalSegments,
            englishSegments: updatedEnglishSegments,
            originalTranscriptPath: updatedOriginalTranscriptPath,
            englishTranscriptPath: updatedEnglishTranscriptPath,
            activeTranscriptLanguage: newActiveTranscriptLanguage, // Update based on what was loaded
            transcriptDirty: false,
            isTranscriptLoading: false,
            speakers: updatedSpeakers,
            player: { ...ts.player, currentSegmentIndex: -1 },
            transcriptUndoStack: [],
            transcriptRedoStack: [],
        };
        // END: MODIFIED
    });
}

export function updateSegment(index, updatedSegmentData, silent = false) {
    const currentSegments = get(transcriptStore).segments;
    if (index < 0 || index >= currentSegments.length) {
        console.warn('[TranscriptStore] updateSegment invalid index:', index); // WARN
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
                 if (currentValue !== newValue) {
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
        pushToUndoStack(currentSegments); // Uses the function defined in this store
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
    } else {
        // if (!silent) console.debug('[TranscriptStore] updateSegment no changes needed index', index); // DEBUG
    }
}

export function deleteTranscriptSegment(index) {
    const currentSegments = get(transcriptStore).segments;
    if (index < 0 || index >= currentSegments.length) {
        console.warn('[TranscriptStore] deleteTranscriptSegment called with invalid index:', index); // WARN
        return;
    }
    pushToUndoStack(currentSegments);
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

export function insertTranscriptSegment(index, newSegment) {
    const currentSegments = get(transcriptStore).segments;
    if (index < 0 || index > currentSegments.length) {
        console.warn('[TranscriptStore] insertTranscriptSegment called with invalid index:', index); // WARN
        return;
    }
    if (!newSegment || typeof newSegment.start_time !== 'number' || typeof newSegment.end_time !== 'number') {
        console.error('[TranscriptStore] insertTranscriptSegment called with invalid segment data:', newSegment); // ERROR
        return;
    }
    pushToUndoStack(currentSegments);
    transcriptStore.update(ts => {
        const segmentsBefore = ts.segments.slice(0, index);
        const segmentsAfter = ts.segments.slice(index);
        const newSegmentsArray = [...segmentsBefore, newSegment, ...segmentsAfter]; // Renamed to avoid conflict
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

export function setSelectedModel(modelName) {
    transcriptStore.update((ts) => ({ ...ts, selectedModelName: modelName || null }));
}

export function setSelectedLanguage(languageCode) {
    transcriptStore.update((ts) => ({ ...ts, selectedLanguage: languageCode || null }));
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
            console.warn(`[TranscriptStore updateSpeakerConfig] Duplicate primary name: '${proposedName}'. Using default.`); // WARN
            proposedName = null;
        }
        if (!proposedName) {
            let defaultName = `Speaker ${nameCounter++}`; // Ensure space for uniqueness
            while (validatedNames.includes(defaultName) || (names.slice(0, i).includes(defaultName))) { // Check against already validated and remaining input names
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
        for (let i = 0; i < count; i++) { // Use validated 'count'
            // For translated names, allow duplicates and preserve empty strings if provided that way,
            // but default to empty string if not provided or not a string.
            const proposedTranslatedName = (newTranslatedNames[i] && typeof newTranslatedNames[i] === 'string') ? newTranslatedNames[i].trim() : '';
            validatedTranslatedNames.push(proposedTranslatedName);
        }
    } else {
        for (let i = 0; i < count; i++) {
            validatedTranslatedNames.push(''); // Default to empty strings if newTranslatedNames is not an array
        }
    }

    // Ensure translatedNames array has the same length as names array, padding with empty strings if necessary.
    while(validatedTranslatedNames.length < count) {
        validatedTranslatedNames.push('');
    }
    if(validatedTranslatedNames.length > count) {
        validatedTranslatedNames.splice(count); // Truncate if too long
    }


    const newSpeakerConfig = {
        count: count,
        names: validatedNames,
        translatedNames: validatedTranslatedNames
    };

    const currentTranscriptData = get(transcriptStore);
    const projectData = get(projectMainStore); // Get project data

    const oldSegments = currentTranscriptData.segments;
    const oldSpeakerConfig = currentTranscriptData.speakers;
    const currentMediaFile = currentTranscriptData.selectedMediaFile;
    const projectXmlPath = projectData.xmlPath; // from projectMainStore
    const mediaIdentifier = currentMediaFile?.media_xml_identifier;

    if (!mediaIdentifier) {
        console.error("[TranscriptStore updateSpeakerConfig] Cannot save: Missing Media XML Identifier."); // ERROR
        updateProjectStoreState({ error: "Save Error: Missing media identifier."});
        message("Error: Missing media identifier.", {title: "Save Error", type:"error"});
        return;
    }
    if (!projectXmlPath) {
        console.error("[TranscriptStore updateSpeakerConfig] Cannot save: Missing Project XML path."); // ERROR
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
        pushToUndoStack(oldSegments); // Assumes pushToUndoStack is defined in this store
    }

    transcriptStore.update((ts) => ({
        ...ts,
        speakers: newSpeakerConfig,
        segments: newSegments, // newSegments here
        transcriptDirty: ts.transcriptDirty || JSON.stringify(oldSpeakerConfig) !== JSON.stringify(newSpeakerConfig) || segmentsChanged,
    }));
    updateProjectStoreState({ statusMessage: 'Updating speaker configuration...' });

    const invokePayload = { projectXmlPath: projectXmlPath, mediaIdentifier: mediaIdentifier, count: newSpeakerConfig.count, names: newSpeakerConfig.names };
    invoke('save_speaker_config', invokePayload)
        .then(() => {
            updateProjectStoreState({ statusMessage: 'Speaker configuration saved.', error: null });

            // Update project.files in projectStore
            // This part is tricky. Ideally, projectStore would own 'files' and have a method to update it.
            // For now, directly call an exported update function from projectStore if available,
            // or make projectStore listen to an event from transcriptStore.
            // Let's assume there's an exported function `updateProjectFileSpeakers` in projectStore for now.
            // This requires projectStore to be refactored to expose such a function or handle this update.
            // For this step, we'll call projectMainStore.update directly, which is not ideal.
            projectMainStore.update(p => {
                 const updatedFiles = JSON.parse(JSON.stringify(p.files));
                 function findAndUpdateMediaSpeakers(nodes, targetIdentifier, newSpeakerData) {
                     if (!Array.isArray(nodes)) return false;
                     let found = false;
                     for (const node of nodes) {
                         if (node.media_xml_identifier === targetIdentifier && (node.file_type === 'media' || node.file_type === 'directory_media_stem')) {
                             node.speakers = { '@count': newSpeakerData.count, name: newSpeakerData.names };
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
                     console.warn("[TranscriptStore via projectMainStore] Could not find media identifier in project.files tree to update speakers."); // WARN
                     return p;
                 }
            });

        })
        .catch((error) => {
            console.error(`[TranscriptStore updateSpeakerConfig] Failed persist config for ${mediaIdentifier}:`, error); // ERROR
            const errorMessage = error?.message || String(error);
            updateProjectStoreState({ error: `Failed save speaker config: ${errorMessage}`, statusMessage: 'Error saving speaker config.'});
            if (typeof message !== 'undefined') {
                message(`Error saving speaker settings: ${errorMessage}`, {title: "Save Error", type: "error"});
            } else {
                console.error(`Error saving speaker settings: ${errorMessage}`); // ERROR
            }
        });
}

export function setAudioBuffer(buffer) {
    transcriptStore.update((ts) => ({ ...ts, audioBuffer: buffer }));
}

export function toggleTranscribeModal(show) {
    transcriptStore.update((ts) => ({ ...ts, showTranscribeModal: !!show }));
}

export function setTranscriptionStatus(isTranscribing, jobIdToSet = null, options = {}) {
    console.log(`[JULES-DEBUG TS setStatus] Called with: isTranscribing=${isTranscribing}, jobIdToSet=${jobIdToSet}, options=`, options);
    const {
        initialProgressMessage = '',
        mediaPath = null,
        status = null, // Explicit status like 'initiating', 'running', 'error', 'done', 'cancelled'
        errorMessage = null
    } = options;

    transcriptStore.update((ts) => {
        let updatedState = { ...ts };

        if (isTranscribing) {
            const newActiveMediaDuringStart = mediaPath || ts.selectedMediaFile?.path || ts.activeMediaDuringTranscriptionStart;
            // Determine job status: if explicit status is passed, use it.
            // Otherwise, if a jobId is being set, it's 'running'. If no jobId yet, it's 'initiating'.
            const jobStatusToSet = status || (jobIdToSet ? 'running' : 'initiating');
            const messageToSet = initialProgressMessage || (jobStatusToSet === 'initiating' ? `Initiating...` : `Processing...`);

            updatedState = {
                ...ts,
                isTranscribing: true,
                transcriptionJobId: jobIdToSet !== null ? jobIdToSet : ts.transcriptionJobId, // Update if new jobId is provided, else keep existing
                mediaPathForLastJob: mediaPath || ts.mediaPathForLastJob,
                activeMediaDuringTranscriptionStart: newActiveMediaDuringStart,
                transcriptionProgress: {
                    // Preserve percent if status is 'running' and jobId matches, otherwise reset to 0
                    percent: (jobStatusToSet === 'running' && ts.transcriptionJobId === jobIdToSet && ts.transcriptionJobId !== null) ? ts.transcriptionProgress.percent : 0,
                    message: messageToSet
                },
                transcriptionJobStatus: jobStatusToSet,
                transcriptionErrorMessage: null, // Clear previous errors when starting/initiating
                ranInBackground: false, // Reset this flag
                showTranscribeModal: true, // Ensure modal remains open while transcribing or initiating
            };
        } else {
            // This branch is for when isTranscribing is explicitly false (e.g. job finished, error, cancelled by event)
            updatedState = {
                ...ts,
                isTranscribing: false,
                transcriptionJobStatus: status || ts.transcriptionJobStatus, // e.g. 'done', 'error', 'cancelled'
                transcriptionErrorMessage: errorMessage || ts.transcriptionErrorMessage,
                showTranscribeModal: true, // Keep modal open to show final status/error
            };
        }
        console.log(`[JULES-DEBUG TS setStatus Updated] Store updated. New jobStatus=${updatedState.transcriptionJobStatus}, new jobId=${updatedState.transcriptionJobId}, progressMsg='${updatedState.transcriptionProgress.message}'`);
        return updatedState;
    });

    if (isTranscribing) {
        updateProjectStoreState({ error: null }); // Clear project-level errors when starting a new job
    }
}

export function updateTranscriptionProgress(progressPayload) {
    transcriptStore.update((ts) => {
        // console.log('[JULES-DEBUG TS updateTranscriptionProgress] Store state before update:',
        //     `isTranscribing: ${ts.isTranscribing}, storeJobId: ${ts.transcriptionJobId}, storeStatus: ${ts.transcriptionJobStatus}. Event payload:`, progressPayload);

        const eventJobId = progressPayload?.jobId;

        if (!eventJobId) {
            // console.log('[JULES-DEBUG TS updateProgress] No eventJobId in payload, skipping update.');
            return ts;
        }

        // Case 1: Store is 'initiating' and has no job ID yet.
        // The first progress event for the new job arrives. Adopt its ID and status.
        if (ts.isTranscribing && ts.transcriptionJobStatus === 'initiating' && ts.transcriptionJobId === null) {
            // console.log('[JULES-DEBUG TS updateProgress] Status is "initiating", adopting eventJobId:', eventJobId);
            return {
                ...ts,
                transcriptionJobId: eventJobId, // Adopt the job ID from the event
                transcriptionJobStatus: 'running', // Transition to 'running'
                transcriptionProgress: {
                    percent: progressPayload?.percent ?? 0,
                    message: progressPayload?.message ?? ''
                },
            };
        }
        // Case 2: Store is 'running' and the event's job ID matches the store's job ID.
        else if (ts.isTranscribing && ts.transcriptionJobStatus === 'running' && ts.transcriptionJobId === eventJobId) {
            // console.log('[JULES-DEBUG TS updateProgress] Status is "running" and job IDs match, updating progress.');
            return {
                ...ts,
                transcriptionProgress: {
                    percent: progressPayload?.percent ?? 0,
                    message: progressPayload?.message ?? ''
                },
            };
        } else {
            // console.log('[JULES-DEBUG TS updateProgress] No update. Conditions did not match.');
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
            showTranscribeModal: true // Ensure modal is shown
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
        return ts; // Return current state if no changes needed
    });
}

export function setRanInBackground(value) {
    transcriptStore.update((ts) => ({ ...ts, ranInBackground: !!value }));
}

// --- Helper for Speaker Name Remapping ---
function remapSegmentSpeakerNames(segmentsToRemap, speakerConfig, targetSpeakerNames = null) {
    const userNames = targetSpeakerNames && targetSpeakerNames.length > 0
                      ? targetSpeakerNames
                      : (speakerConfig && Array.isArray(speakerConfig.names) ? speakerConfig.names : []);

    if (userNames.length === 0) {
        // If no specific target names are provided and primary names are also empty,
        // or if speakerConfig itself is minimal/empty.
        // Return segments as is, assuming diarization might have put SPEAKER_XX.
        // Or, if preferred, map all to "Unknown". For now, returning as-is.
        return segmentsToRemap.map(seg => ({ ...seg }));
    }

    return segmentsToRemap.map(seg => {
        const newSegment = { ...seg }; // Work on a copy
        const originalSpeaker = newSegment.speaker ? String(newSegment.speaker).trim() : "Unknown";

        let userAssignedIndex = -1;

        // Try to parse "SPEAKER_XX" or "speaker_X"
        if (originalSpeaker.toUpperCase().startsWith("SPEAKER_")) {
            const numStr = originalSpeaker.substring("SPEAKER_".length);
            const parsedNum = parseInt(numStr, 10);
            if (!isNaN(parsedNum)) {
                userAssignedIndex = parsedNum; // Assumes SPEAKER_00 is index 0, SPEAKER_01 is index 1
            }
        } else if (originalSpeaker.toLowerCase().startsWith("speaker_")) {
            const numStr = originalSpeaker.substring("speaker_".length);
            const parsedNum = parseInt(numStr, 10);
            if (!isNaN(parsedNum) && parsedNum > 0) {
                userAssignedIndex = parsedNum - 1; // Assumes speaker_1 is index 0
            }
        }

        if (userAssignedIndex >= 0 && userAssignedIndex < userNames.length) {
            if (userNames[userAssignedIndex] && userNames[userAssignedIndex].trim() !== "") {
                newSegment.speaker = userNames[userAssignedIndex].trim();
            } else {
                // If the target username is empty, keep original or set to a default like "Speaker X"
                // For now, let's keep the original generic ID if the target name is empty.
                // This case should ideally be handled by speaker config validation ensuring no empty names.
            }
        } else {
            // If the speaker ID isn't in the generic format OR the index is out of bounds,
            // it might be an already user-defined name or a different system's ID.
            // Check if this name is one of the *current* userNames. If not, it's an old/unknown name.
            if (!userNames.includes(originalSpeaker) && originalSpeaker !== "Unknown") {
                // This could be an old name. For now, we don't have a reverse map here.
                // Simplest is to leave it, or if strict, map to "Unknown".
                // Let's leave it for now. The main purpose is mapping generic IDs.
            }
        }
        return newSegment;
    });
}


// --- Dual Transcript Switching Actions ---

export function switchToOriginalTranscript() {
    transcriptStore.update(ts => {
        if (ts.activeTranscriptLanguage === 'original' || ts.originalSegments.length === 0) {
            return ts; // Already original or no original segments to switch to
        }

        // Preserve undo/redo for the English transcript if needed, or clear
        // For simplicity, clearing undo/redo on switch.
        // TODO: Consider preserving undo/redo stacks per language.
        const newUndoStack = []; // Clear undo/redo for the new active transcript
        const newRedoStack = [];

        let newIndex = -1;
        const time = ts.player.currentTime;
        if (ts.originalSegments.length > 0 && ts.player.duration > 0 && time >= 0) {
            const idx = ts.originalSegments.findIndex((s, index) => {
                const isLastSegment = index === ts.originalSegments.length - 1;
                const startTimeCheck = time >= (s.start_time - 0.001);
                const endTimeCheck = isLastSegment ? time <= s.end_time : time < s.end_time;
                return startTimeCheck && endTimeCheck;
            });
            newIndex = idx;
        }
        updateProjectStoreState({ statusMessage: 'Switched to original transcript.' });

        const remappedSegments = remapSegmentSpeakerNames([...ts.originalSegments], ts.speakers, ts.speakers.names);

        return {
            ...ts,
            segments: remappedSegments,
            activeTranscriptLanguage: 'original',
            currentTranscriptPath: ts.originalTranscriptPath,
            transcriptDirty: false, // Assume switching doesn't make it dirty initially
            transcriptUndoStack: newUndoStack,
            transcriptRedoStack: newRedoStack,
            player: { ...ts.player, currentSegmentIndex: newIndex }
        };
    });
}

export function switchToEnglishTranscript() {
    transcriptStore.update(ts => {
        if (ts.activeTranscriptLanguage === 'english' || ts.englishSegments.length === 0) {
            return ts; // Already English or no English segments to switch to
        }

        const newUndoStack = [];
        const newRedoStack = [];

        let newIndex = -1;
        const time = ts.player.currentTime;
        if (ts.englishSegments.length > 0 && ts.player.duration > 0 && time >= 0) {
            const idx = ts.englishSegments.findIndex((s, index) => {
                const isLastSegment = index === ts.englishSegments.length - 1;
                const startTimeCheck = time >= (s.start_time - 0.001);
                const endTimeCheck = isLastSegment ? time <= s.end_time : time < s.end_time;
                return startTimeCheck && endTimeCheck;
            });
            newIndex = idx;
        }
        updateProjectStoreState({ statusMessage: 'Switched to English transcript.' });

        const remappedSegments = remapSegmentSpeakerNames([...ts.englishSegments], ts.speakers, ts.speakers.translatedNames);

        return {
            ...ts,
            segments: remappedSegments,
            activeTranscriptLanguage: 'english',
            currentTranscriptPath: ts.englishTranscriptPath,
            transcriptDirty: false,
            transcriptUndoStack: newUndoStack,
            transcriptRedoStack: newRedoStack,
            player: { ...ts.player, currentSegmentIndex: newIndex }
        };
    });
}


// Helper to update projectStore's status and error, if needed by transcript functions
// This is a placeholder for a better way to handle global state updates.
// export function updateGlobalStatus(statusMessage, error = null) {
//   projectMainStore.update(p => ({...p, statusMessage, error}));
// }

// Ensure projectStore exports project and possibly a way to update its state if needed, e.g.
// export const project = writable({...initialState});
// export const updateProjectStoreState = (newState) => project.update(s => ({...s, ...newState}));
// This updateProjectStoreState would need to be added to projectStore.js

// Listen for media rename events from the backend
listen('media_renamed', (event) => {
    if (!event.payload) return;

    const { old_media_stem, new_media_stem, new_media_file_relative_path, new_absolute_path } = event.payload;

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
                    media_xml_identifier: new_media_stem, // IMPORTANT: Use new_media_stem for the identifier
                },
                // Optionally, update currentTranscriptPath if it was pointing to a transcript of the old media stem
                // This depends on how associated transcripts are handled and if their paths change predictably.
                // For now, focusing on selectedMediaFile.
            };
        }
        return ts;
    });
});

listen('custom_transcription_job_completed', async (event) => {
    if (!event.payload) return;
    const { status, jobFinishedPath, transcriptFilePath, translatedTranscriptFilePath, errorMessage } = event.payload;
    const currentStore = get(transcriptStore);

    // Only process if the completed job was for the currently selected media
    if (currentStore.selectedMediaFile?.path === jobFinishedPath) {
        if (status === 'done') {
            console.log('[TranscriptStore] Received custom_transcription_job_completed:', event.payload);
            let activePathToLoad = null;
            let newOriginalPath = currentStore.originalTranscriptPath;
            let newEnglishPath = currentStore.englishTranscriptPath;

            // Update store with new paths first, so they are available for loadTranscriptFile
            const updates = {};
            if (transcriptFilePath) {
                updates.originalTranscriptPath = transcriptFilePath;
                newOriginalPath = transcriptFilePath; // for local var use
                // If current active lang is original, or if it's original and no translation will exist, this is the one to load.
                if (currentStore.activeTranscriptLanguage === 'original' || !translatedTranscriptFilePath) {
                    activePathToLoad = transcriptFilePath;
                }
            }
            if (translatedTranscriptFilePath) {
                updates.englishTranscriptPath = translatedTranscriptFilePath;
                newEnglishPath = translatedTranscriptFilePath; // for local var use
                // If current active lang is English, this is the one to load.
                if (currentStore.activeTranscriptLanguage === 'english') {
                    activePathToLoad = translatedTranscriptFilePath;
                }
            }
            // If no specific active path was determined (e.g. active lang was english, but only original came back),
            // default to loading the original if it exists.
            if (!activePathToLoad && newOriginalPath) {
                activePathToLoad = newOriginalPath;
            }

            transcriptStore.update(ts => ({ ...ts, ...updates }));

            if (activePathToLoad) {
                try {
                    const service = await import('../services/projectService.js');
                    if (service.loadTranscriptFile) {
                        console.log(`[TranscriptStore] Loading transcript after job completion: ${activePathToLoad}`);
                        await service.loadTranscriptFile(activePathToLoad);
                        // setTranscriptData called by loadTranscriptFile should now correctly populate
                        // originalSegments/englishSegments based on the path.
                    } else {
                        console.error('[TranscriptStore] loadTranscriptFile function not found in projectService.');
                        updateProjectStoreState({ error: 'Internal error: Transcript loading service unavailable.'});
                    }
                } catch (e) {
                    console.error(`[TranscriptStore] Error auto-loading transcript ${activePathToLoad}:`, e);
                    updateProjectStoreState({ error: `Failed to load transcript: ${e.message || e}`});
                }
            }

            // Refresh project files to update UI elements like LeftPanel about new transcript files
            try {
                const service = await import('../services/projectService.js');
                if (service.refreshProjectFiles && currentStore.selectedMediaFile?.path) {
                   console.log('[TranscriptStore] Refreshing project files to update transcript associations.');
                   await service.refreshProjectFiles(currentStore.selectedMediaFile.path);
                }
            } catch (e) {
                console.error('[TranscriptStore] Error refreshing project files after job completion:', e);
            }
            // clearTranscriptionStatus('Transcription complete.'); // Removed
            // toggleTranscribeModal(false); // Removed
            console.log(`[JULES-DEBUG TS eventComplete] Status: 'done'. Payload:`, event.payload);
            transcriptStore.update(ts => ({ ...ts, isTranscribing: false, transcriptionJobStatus: 'done', transcriptionErrorMessage: null }));

        } else if (status === 'error') {
            console.error(`[TranscriptStore] Transcription job failed for ${jobFinishedPath}: ${errorMessage}`);
            updateProjectStoreState({ error: `Transcription failed: ${errorMessage}` });
            // clearTranscriptionStatus(`Transcription failed: ${errorMessage}`, errorMessage); // Removed
            // toggleTranscribeModal(false); // Removed
            console.log(`[JULES-DEBUG TS eventComplete] Status: 'error'. Payload:`, event.payload);
            transcriptStore.update(ts => ({ ...ts, isTranscribing: false, transcriptionJobStatus: 'error', transcriptionErrorMessage: errorMessage }));
        } else if (status === 'cancelled') {
            console.info(`[TranscriptStore] Transcription job cancelled for ${jobFinishedPath}.`);
            updateProjectStoreState({ statusMessage: 'Transcription cancelled.' });
            // clearTranscriptionStatus('Transcription cancelled.'); // Removed
            // toggleTranscribeModal(false); // Removed
            console.log(`[JULES-DEBUG TS eventComplete] Status: 'cancelled'. Payload:`, event.payload);
            transcriptStore.update(ts => ({ ...ts, isTranscribing: false, transcriptionJobStatus: 'cancelled', transcriptionErrorMessage: null }));
        }
    } else {
         console.log('[TranscriptStore] Received custom_transcription_job_completed for a non-selected/different media file:', jobFinishedPath, currentStore.selectedMediaFile?.path);
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

// Listen for item rename events from the backend (specifically for currentTranscriptPath)
listen('item_renamed', (event) => {
    if (!event.payload) return;

    const { old_path, new_path, item_type } = event.payload;

    transcriptStore.update(ts => {
        // Normalize paths robustly (handles single/double backslashes and forward slashes)
        const normalized_old_path = old_path.replace(/[\\\/]+/g, '/');
        const normalized_new_path = new_path.replace(/[\\\/]+/g, '/');

        if (item_type === 'transcript' && ts.currentTranscriptPath && ts.currentTranscriptPath.replace(/[\\\/]+/g, '/') === normalized_old_path) {
            return { ...ts, currentTranscriptPath: normalized_new_path };
        }
        return ts;
    });
});
