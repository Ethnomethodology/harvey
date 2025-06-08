// src/lib/stores/transcriptStore.js
import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { message } from '@tauri-apps/plugin-dialog';
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
    speakers: { count: 0, names: [] },
    player: { currentTime: 0, duration: 0, isPlaying: false, currentSegmentIndex: -1 },
    audioBuffer: null,
    isTranscriptLoading: false,
    isTranscribing: false,
    transcriptionProgress: { percent: 0, message: '' },
    transcriptionJobId: null,
    showTranscribeModal: false,
    transcriptUndoStack: [],
    transcriptRedoStack: [],
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
        console.debug('[TranscriptStore] Undo stack empty.'); // DEBUG
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
        console.info('[TranscriptStore] Undoing transcript change.'); // INFO
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
        console.debug('[TranscriptStore] Redo stack empty.'); // DEBUG
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
        console.info('[TranscriptStore] Redoing transcript change.'); // INFO
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
    console.info('[TranscriptStore] Marking media transcript as saved, clearing undo/redo stacks.'); // INFO
    transcriptStore.update(ts => ({
        ...ts,
        transcriptDirty: false,
        transcriptUndoStack: [],
        transcriptRedoStack: []
    }));
    updateProjectStoreState({ statusMessage: 'Media transcript saved.', error: null });
}

export function clearTranscriptState() {
    console.info('[TranscriptStore] Clearing media transcript state (main TranscriptionsView).'); // INFO
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
                speakers: { count: 0, names: [] }
            };
        }
        return ts;
    });
}

export function selectMedia(fileEntry) {
    console.debug('[TranscriptStore selectMedia] ACTION START. Received fileEntry:', fileEntry ? `Name: ${fileEntry.name}, Path: ${fileEntry.path}` : 'null'); // DEBUG
    const currentSelectedPath = get(transcriptStore).selectedMediaFile?.path;
    const shouldUpdateSelection = (!fileEntry && currentSelectedPath !== null) || (fileEntry && currentSelectedPath !== fileEntry.path);

    let speakersToLoad = { count: 0, names: [] };
    if (fileEntry && fileEntry.file_type === 'media' && !fileEntry.is_directory && fileEntry.speakers && typeof fileEntry.speakers === 'object') {
        const loadedCount = Number(fileEntry.speakers['@count']) || 0;
        const loadedNamesRaw = fileEntry.speakers.name;
        const loadedNames = Array.isArray(loadedNamesRaw) ? loadedNamesRaw : (loadedNamesRaw ? [loadedNamesRaw] : []);
        speakersToLoad = { count: loadedCount, names: [...loadedNames] };
        if (speakersToLoad.count !== speakersToLoad.names.length) {
            console.warn(`[TranscriptStore selectMedia] Discrepancy count/names for ${fileEntry.name}. Adjusting.`); // WARN
            speakersToLoad.count = speakersToLoad.names.length;
            speakersToLoad.names = speakersToLoad.names.slice(0, speakersToLoad.count);
        }
        console.debug(`[TranscriptStore selectMedia] Speakers FOUND on FileEntry '${fileEntry.name}':`, JSON.stringify(speakersToLoad)); // DEBUG
    } else if (fileEntry && fileEntry.file_type === 'media' && !fileEntry.is_directory) {
        console.debug(`[TranscriptStore selectMedia] No valid speaker config on entry '${fileEntry.name}'. Using default.`); // DEBUG
        speakersToLoad = { count: 0, names: [] };
    } else {
        console.debug('[TranscriptStore selectMedia] No valid media file selected or clearing. Using default speakers.'); // DEBUG
        speakersToLoad = { count: 0, names: [] };
    }

    const currentStoreSpeakers = get(transcriptStore).speakers;
    const speakersChanged = JSON.stringify(currentStoreSpeakers) !== JSON.stringify(speakersToLoad);

    if (shouldUpdateSelection || speakersChanged) {
        console.debug(`[TranscriptStore selectMedia] Updating store. SelectionChanged: ${shouldUpdateSelection}, SpeakersChanged: ${speakersChanged}`); // DEBUG
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
        updateProjectStoreState({ statusMessage: newSelectedMedia ? `Selected media: ${newSelectedMedia.name}` : 'Media selection cleared.' });
        console.debug('[TranscriptStore selectMedia] Store update complete for media selection/resets.'); // DEBUG

        const newlySelectedMedia = get(transcriptStore).selectedMediaFile;
        console.debug(`[TranscriptStore selectMedia] Checking associated transcripts for: ${newlySelectedMedia?.name ?? 'null'}`); // DEBUG
        // console.debug(`[TranscriptStore selectMedia]   -> associated_transcripts object:`, newlySelectedMedia?.associated_transcripts); // DEBUG - potentially verbose

        if (newlySelectedMedia && Array.isArray(newlySelectedMedia.associated_transcripts) && newlySelectedMedia.associated_transcripts.length > 0) {
            const firstTranscriptInfo = newlySelectedMedia.associated_transcripts[0];
            // console.debug(`[TranscriptStore selectMedia]   -> First transcript info object:`, firstTranscriptInfo); // DEBUG - potentially verbose
            const firstTranscriptRelativePath = firstTranscriptInfo?.relativePath;

            if (firstTranscriptRelativePath && typeof firstTranscriptRelativePath === 'string') {
                console.debug(`[TranscriptStore selectMedia] First associated transcript relative path: ${firstTranscriptRelativePath}`); // DEBUG
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
                    console.info(`[TranscriptStore selectMedia] Found first transcript node: ${transcriptNodeToLoad.path}. Auto-loading...`); // INFO
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
        } else {
            console.info(`[TranscriptStore selectMedia] No associated transcripts found for ${newlySelectedMedia?.name ?? 'selected media'}.`); // INFO
        }
    } else {
        console.debug(`[TranscriptStore selectMedia] Selection/speakers unchanged for ${fileEntry?.name ?? 'File'}.`); // DEBUG
    }
    console.debug('[TranscriptStore selectMedia] ACTION END.'); // DEBUG
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
    console.info(`[TranscriptStore] setTranscriptData called with path: ${path}, inferSpeakers: ${inferSpeakers}`); // INFO
    const newSegments = Array.isArray(data) ? data : [];
    if (newSegments.length > 0 && typeof newSegments[0].text !== 'undefined') { // Added a check for text property
        console.debug('[TranscriptStore load‑from‑disk]', { firstSegType: typeof newSegments[0].text, firstSegStart: newSegments[0].start_time, first120: typeof newSegments[0].text === 'string' ? newSegments[0].text.slice(0, 120) : String(newSegments[0].text).slice(0, 120) }); // DEBUG
    }
    transcriptStore.update((ts) => {
        let updatedSpeakers = ts.speakers;
        if (inferSpeakers) {
            console.warn('[TranscriptStore] Speaker inference requested. Overwriting current.'); // WARN
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
            console.debug('[TranscriptStore] Inferred speakers:', updatedSpeakers); // DEBUG
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
            transcriptUndoStack: [], // Clear undo/redo on new load
            transcriptRedoStack: [],
        };
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
                    if (key === 'text') {
                        console.debug('[TranscriptStore store‑updateSegment text]', { idx: index, typeof: typeof newValue, first120: typeof newValue === 'string' ? newValue.slice(0, 120) : String(newValue).slice(0, 120) }); // DEBUG
                    }
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
            if (!silent) console.debug('[TranscriptStore] Updated segment', index); // DEBUG
            if (!silent) updateProjectStoreState({ statusMessage: 'Media transcript modified.' });
            return {
                ...ts,
                segments: newSegments,
                transcriptDirty: true,
            };
        });
    } else {
        if (!silent) console.debug('[TranscriptStore] updateSegment no changes needed index', index); // DEBUG
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
        console.info(`[TranscriptStore] Deleted segment index ${index}. New player index: ${newPlayerIndex}`); // INFO
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
        console.info(`[TranscriptStore] Inserted new segment at index ${index}. New player index: ${newPlayerIndex}`); // INFO
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
    console.info(`[TranscriptStore] Set model: ${modelName}`); // INFO
    transcriptStore.update((ts) => ({ ...ts, selectedModelName: modelName || null }));
}

export function setSelectedLanguage(languageCode) {
    console.info(`[TranscriptStore] Set language: ${languageCode}`); // INFO
    transcriptStore.update((ts) => ({ ...ts, selectedLanguage: languageCode || null }));
}

export function updateSpeakerConfig(newCount, newNames) {
    console.debug(`[TranscriptStore updateSpeakerConfig] Received: count=${newCount}, names=`, newNames); // DEBUG
    const count = Math.max(0, Math.min(11, Number(newCount) || 0));
    const names = Array.isArray(newNames) ? newNames : [];
    let nameCounter = 1;
    const validatedNames = [];
    for (let i = 0; i < count; i++) {
        let proposedName = names[i] && names[i].trim() !== '' ? names[i].trim() : null;
        let finalName;
        if (proposedName && validatedNames.includes(proposedName)) {
            console.warn(`[TranscriptStore updateSpeakerConfig] Duplicate name: '${proposedName}'. Using default.`); // WARN
            proposedName = null;
        }
        if (!proposedName) {
            let defaultName = `Speaker-${nameCounter++}`;
            while (validatedNames.includes(defaultName) || (names.length > validatedNames.length && names.slice(validatedNames.length).includes(defaultName))) {
                defaultName = `Speaker ${nameCounter++}`;
            }
            finalName = defaultName;
        } else {
            finalName = proposedName;
        }
        validatedNames.push(finalName);
    }
    console.debug('[TranscriptStore] Validated speaker names:', validatedNames); // DEBUG
    const newSpeakerConfig = { count: count, names: validatedNames };

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

    console.info(`[TranscriptStore updateSpeakerConfig] Saving for Media ID: ${mediaIdentifier} in project: ${projectXmlPath}`); // INFO
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
    console.debug('[TranscriptStore updateSpeakerConfig] Speaker remapping:', speakerMap); // DEBUG

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
        console.info('[TranscriptStore updateSpeakerConfig] Remapped speaker names. Pushing undo.'); // INFO
        pushToUndoStack(oldSegments); // Assumes pushToUndoStack is defined in this store
    }

    transcriptStore.update((ts) => ({
        ...ts,
        speakers: newSpeakerConfig,
        segments: newSegments, // newSegments here
        transcriptDirty: ts.transcriptDirty || JSON.stringify(oldSpeakerConfig) !== JSON.stringify(newSpeakerConfig) || segmentsChanged,
    }));
    updateProjectStoreState({ statusMessage: 'Updating speaker configuration...' });
    console.debug('[TranscriptStore updateSpeakerConfig] Updated store speakers/segments.'); // DEBUG

    const invokePayload = { projectXmlPath: projectXmlPath, mediaIdentifier: mediaIdentifier, count: newSpeakerConfig.count, names: newSpeakerConfig.names };
    console.debug('[TranscriptStore updateSpeakerConfig] Calling backend save_speaker_config:', invokePayload); // DEBUG
    invoke('save_speaker_config', invokePayload)
        .then(() => {
            console.info(`[TranscriptStore updateSpeakerConfig] Persisted config for ${mediaIdentifier}.`); // INFO
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
                             console.debug(`[TranscriptStore via projectMainStore] Found node (${node.name}, type: ${node.file_type}) for identifier ${targetIdentifier}. Updating speakers.`); // DEBUG
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
                     console.info("[TranscriptStore via projectMainStore] Successfully updated speaker data in project.files tree."); // INFO
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
    console.debug('[TranscriptStore] Setting AudioBuffer:', buffer ? `(${buffer.duration.toFixed(2)}s)` : 'null'); // DEBUG
    transcriptStore.update((ts) => ({ ...ts, audioBuffer: buffer }));
}

export function toggleTranscribeModal(show) {
    transcriptStore.update((ts) => ({ ...ts, showTranscribeModal: !!show }));
}

export function setTranscriptionStatus(isTranscribing, jobId = null, statusMessage = '') {
    transcriptStore.update((ts) => ({
        ...ts,
        isTranscribing: !!isTranscribing,
        transcriptionJobId: jobId,
        // Update global status message as well for now
        // statusMessage: statusMessage || (isTranscribing ? 'Starting transcription...' : 'Ready'),
        transcriptionProgress: isTranscribing ? { percent: 0, message: '' } : ts.transcriptionProgress,
        // error: isTranscribing ? null : ts.error // Use global error for now
    }));
    // Also update global status/error
     updateProjectStoreState({
        statusMessage: statusMessage || (isTranscribing ? 'Starting transcription...' : get(projectMainStore).statusMessage),
        error: isTranscribing ? null : get(projectMainStore).error
    });
}

export function updateTranscriptionProgress(progressPayload) {
    transcriptStore.update((ts) => {
        if (ts.isTranscribing && ts.transcriptionJobId && progressPayload?.jobId === ts.transcriptionJobId) {
            const newMessage = progressPayload?.message ?? ts.transcriptionProgress.message;
            return {
                ...ts,
                transcriptionProgress: { percent: progressPayload?.percent ?? 0, message: newMessage },
            };
        }
        return ts;
    });
}

export function clearTranscriptionStatus(finalStatusMessage = 'Ready', error = null) {
    transcriptStore.update((ts) => ({
        ...ts,
        isTranscribing: false,
        transcriptionProgress: { percent: 0, message: '' },
        transcriptionJobId: null,
    }));
    updateProjectStoreState({ statusMessage: finalStatusMessage, error: error });
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

import { listen } from '@tauri-apps/api/event';

// Listen for media rename events from the backend
listen('media_renamed', (event) => {
    console.info('[TranscriptStore] Received media_renamed event:', event.payload); // INFO
    if (!event.payload) return;

    const { old_media_stem, new_media_stem, new_media_file_relative_path, new_absolute_path } = event.payload;

    transcriptStore.update(ts => {
        if (ts.selectedMediaFile && ts.selectedMediaFile.media_xml_identifier === old_media_stem) {
            const newFileName = new_absolute_path.split(/[\/]/).pop();
            console.info(`[TranscriptStore] media_renamed: Updating selectedMediaFile from ${ts.selectedMediaFile.name} to ${newFileName}`); // INFO
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

// Listen for item rename events from the backend (specifically for currentTranscriptPath)
listen('item_renamed', (event) => {
    console.info('[TranscriptStore] Received item_renamed event:', event.payload); // INFO
    if (!event.payload) return;

    const { old_path, new_path, item_type } = event.payload;

    transcriptStore.update(ts => {
        // Normalize paths robustly (handles single/double backslashes and forward slashes)
        const normalized_old_path = old_path.replace(/[\\\/]+/g, '/');
        const normalized_new_path = new_path.replace(/[\\\/]+/g, '/');

        if (item_type === 'transcript' && ts.currentTranscriptPath && ts.currentTranscriptPath.replace(/[\\\/]+/g, '/') === normalized_old_path) {
            console.info(`[TranscriptStore] item_renamed: Updating currentTranscriptPath from ${ts.currentTranscriptPath} to ${normalized_new_path}`); // INFO
            return { ...ts, currentTranscriptPath: normalized_new_path };
        }
        return ts;
    });
});
