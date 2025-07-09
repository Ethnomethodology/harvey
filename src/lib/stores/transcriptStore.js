// src/lib/stores/transcriptStore.js
import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { message } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event'; // Added missing import for listen
import notificationManager from '$lib/stores/notificationStore.js';
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
    audioBufferPeaks: null, // Added for storing pre-computed peaks
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
    // transcribedOriginalLanguageCode: null, // REMOVED
    // wasTranslatedToEnglish: false,       // REMOVED
    // languageUsedForJob: null, // REMOVED
    diarizationEnabledForNextJob: false,
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
            newIndex = findSegmentIndexWithBinarySearch(previousSegments, time);
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
            newIndex = findSegmentIndexWithBinarySearch(nextSegments, time);
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
                audioBufferPeaks: null,
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
                // transcribedOriginalLanguageCode: null, // REMOVED
                // wasTranslatedToEnglish: false, // REMOVED
                // languageUsedForJob: null, // REMOVED
            };
        }
        return ts;
    });
}

export function selectMedia(fileEntry, transcriptPathToPrioritize = null) {
    const currentSelectedMedia = get(transcriptStore).selectedMediaFile;
    const currentSelectedPath = currentSelectedMedia?.path;

    // Force a re-selection if the associated transcripts list has changed, even if the path is the same.
    const transcriptsChanged = JSON.stringify(currentSelectedMedia?.associated_transcripts) !== JSON.stringify(fileEntry?.associated_transcripts);
    const shouldUpdateSelection = (!fileEntry && currentSelectedPath !== null) || (fileEntry && currentSelectedPath !== fileEntry.path) || transcriptsChanged;

    let speakersToLoad = { count: 0, names: [], translatedNames: [] };
    if (fileEntry && fileEntry.file_type === 'media' && !fileEntry.is_directory && fileEntry.speakers && typeof fileEntry.speakers === 'object') {
        const loadedCount = Number(fileEntry.speakers['@count']) || 0;
        const loadedNamesRaw = fileEntry.speakers.name;
        const loadedNames = Array.isArray(loadedNamesRaw) ? loadedNamesRaw : (loadedNamesRaw ? [loadedNamesRaw] : []);

        // --- START MODIFICATION ---
        let loadedTranslatedNamesRaw = fileEntry.speakers.translatedNames || fileEntry.speakers.translated_names || fileEntry.speakers.second_names;
        let loadedTranslatedNames = [];

        if (Array.isArray(loadedTranslatedNamesRaw)) {
            loadedTranslatedNames = loadedTranslatedNamesRaw.map(name => (typeof name === 'string' ? name.trim() : ''));
        } else if (typeof loadedTranslatedNamesRaw === 'string' && loadedCount === 1) {
            // Handle case where it might be a single string for a single speaker
            loadedTranslatedNames = [loadedTranslatedNamesRaw.trim()];
        } else {
            // Default to empty strings if not found or not in expected format
            loadedTranslatedNames = Array(loadedCount > 0 ? loadedCount : 0).fill('');
        }

        // Ensure the array has the correct length
        if (loadedTranslatedNames.length > loadedCount) {
            loadedTranslatedNames = loadedTranslatedNames.slice(0, loadedCount);
        } else {
            while (loadedTranslatedNames.length < loadedCount) {
                loadedTranslatedNames.push('');
            }
        }
        // --- END MODIFICATION ---

        speakersToLoad = {
            count: loadedCount,
            names: [...loadedNames], // Primary names are already handled
            translatedNames: loadedTranslatedNames // Assign the processed translated names
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
        // Ensure all properties of fileEntry (including 'transcripts') are copied to newSelectedMedia
        const newSelectedMedia = fileEntry && !fileEntry.is_directory && fileEntry.file_type === 'media' ? { ...fileEntry } : null;
        if (newSelectedMedia && (!newSelectedMedia.name || !newSelectedMedia.path)) {
            console.error("[TranscriptStore] CRITICAL: Attempting set selectedMediaFile without name/path!", newSelectedMedia); // ERROR
        }
        if (newSelectedMedia && !newSelectedMedia.media_xml_identifier) {
            console.warn("[TranscriptStore] WARNING: Setting selectedMediaFile without media_xml_identifier! Saving might fail.", newSelectedMedia); // WARN
        }

        transcriptStore.update((ts) => {
            const mediaPathChanged = ts.selectedMediaFile?.path !== newSelectedMedia?.path;
            return {
                ...ts,
                selectedMediaFile: newSelectedMedia, // newSelectedMedia now explicitly includes 'transcripts'
                audioBuffer: mediaPathChanged ? null : ts.audioBuffer,
                audioBufferPeaks: mediaPathChanged ? null : ts.audioBufferPeaks,
                player: {
                    currentTime: 0,
                    duration: mediaPathChanged ? 0 : ts.player.duration,
                    isPlaying: false,
                    currentSegmentIndex: -1
                },
                speakers: speakersToLoad,
                segments: [],
                currentTranscriptPath: null,
                isTranscriptLoading: false,
                transcriptUndoStack: [],
                transcriptRedoStack: [],
                // transcribedOriginalLanguageCode: null, // REMOVED
                // wasTranslatedToEnglish: false, // REMOVED
                // languageUsedForJob: null, // REMOVED
            };
        });

        const newlySelectedMedia = get(transcriptStore).selectedMediaFile;

        // Use associated_transcripts, which is populated by the backend
        if (newlySelectedMedia && Array.isArray(newlySelectedMedia.associated_transcripts) && newlySelectedMedia.associated_transcripts.length > 0) {
            let transcriptPathToLoad = null;

            // Step 1: Prioritize the transcriptPathToPrioritize if provided and valid
            if (transcriptPathToPrioritize) {
                const prioritizedTranscript = newlySelectedMedia.associated_transcripts.find(t => t.path === transcriptPathToPrioritize);
                if (prioritizedTranscript) {
                    transcriptPathToLoad = prioritizedTranscript.path;
                    console.log(`[TranscriptStore selectMedia] Prioritizing clicked transcript: ${transcriptPathToLoad}`);
                }
            }

            // Step 2: If no prioritized transcript, fall back to conventional or first available
            if (!transcriptPathToLoad) {
                const mediaName = newlySelectedMedia.name; // e.g., "my_audio.wav"
                const mediaNameStem = mediaName.includes('.') ? mediaName.substring(0, mediaName.lastIndexOf('.')) : mediaName; // e.g., "my_audio"

                // Look for a conventionally named transcript (e.g., media_name_stem.json)
                const conventionalTranscriptName = `${mediaNameStem}.json`;
                const conventionalTranscript = newlySelectedMedia.associated_transcripts.find(t => {
                    const tName = t.path ? t.path.split(/[/]/).pop() : '';
                    return tName.toLowerCase() === conventionalTranscriptName.toLowerCase();
                });

                if (conventionalTranscript && conventionalTranscript.path) {
                    transcriptPathToLoad = conventionalTranscript.path;
                    console.log(`[TranscriptStore selectMedia] Found conventional transcript: ${transcriptPathToLoad}`);
                } else {
                    // Fallback: load the first transcript in the list
                    const firstTranscriptInfo = newlySelectedMedia.associated_transcripts[0];
                    if (firstTranscriptInfo && firstTranscriptInfo.path) {
                        transcriptPathToLoad = firstTranscriptInfo.path;
                        console.log(`[TranscriptStore selectMedia] No conventional transcript found. Loading first available: ${transcriptPathToLoad}`);
                    }
                }
            }
                const transcriptInfoToLoad = newlySelectedMedia.associated_transcripts.find(t => t.path === transcriptPathToLoad);
                const relativePathToLoad = transcriptInfoToLoad?.relativePath;

                if (relativePathToLoad) {
                    const allFiles = get(projectMainStore).files;
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
                    transcriptNodeToLoad = findTranscriptNodeByRelativePath(allFiles, relativePathToLoad);

                    if (transcriptNodeToLoad && transcriptNodeToLoad.path) {
                        transcriptStore.update(ts => ({ ...ts, currentTranscriptPath: transcriptNodeToLoad.path, isTranscriptLoading: true }));
                        import('../services/projectService.js').then(service => {
                            if (typeof service.loadTranscriptFile === 'function') {
                                service.loadTranscriptFile(transcriptNodeToLoad.path)
                                    .catch(error => {
                                        console.error(`[TranscriptStore] Auto-load default transcript failed:`, error);
                                        transcriptStore.update(ts => ({...ts, isTranscriptLoading: false}));
                                        updateProjectStoreState({ error: `Failed to load default transcript: ${error.message || error}`});
                                    });
                            } else {
                                console.error("[TranscriptStore] loadTranscriptFile function not found in service.");
                                transcriptStore.update(ts => ({...ts, isTranscriptLoading: false}));
                                updateProjectStoreState({ error: "Internal error: Transcript loading service unavailable."});
                            }
                        }).catch(err => {
                            console.error("[TranscriptStore] Failed import projectService for transcript load:", err);
                            transcriptStore.update(ts => ({...ts, isTranscriptLoading: false}));
                            updateProjectStoreState({ error: "Internal error: Failed to import project service."});
                        });
                    } else {
                        console.warn(`[TranscriptStore selectMedia] Could not find FileEntry node for default transcript relative path: ${relativePathToLoad}`);
                    }
                } else {
                     console.warn(`[TranscriptStore selectMedia] Default transcript to load (${transcriptPathToLoad}) does not have a relativePath or was not found in associated_transcripts.`);
                }
            } else {
                console.log('[TranscriptStore selectMedia] No associated transcripts found or no path to load for the selected media.');
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

        // Check if time falls within the current segment's range (with tolerance)
        const startTimeCheck = time >= (segment.start_time - 0.001);
        const endTimeCheck = isLastSegment ? time <= segment.end_time : time < segment.end_time;

        if (startTimeCheck && endTimeCheck) {
            return mid; // Time is within this segment
        } else if (time < segment.start_time) {
            high = mid - 1; // Time is before this segment
        } else {
            low = mid + 1; // Time is after this segment
        }
    }
    return -1; // Time is not within any segment
}

export function updatePlayerTime(time) {
    transcriptStore.update((ts) => {
        let newIndex = -1;
        const segments = ts.segments;
        const numSegments = segments.length;

        if (numSegments > 0 && ts.player.duration > 0 && time >= 0) {
            // Always use binary search for reliable segment index finding
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
    const newSegments = Array.isArray(data) ? data : [];
    transcriptStore.update((ts) => {
        let updatedSpeakers = ts.speakers;
        if (inferSpeakers) {
            // console.warn('[TranscriptStore] Speaker inference requested. Overwriting current primary names and count.'); // Updated log message
            let inferredPrimarySpeakers = { count: 0, names: [] }; // Renamed for clarity
            if (newSegments.length > 0) {
                const uniqueSpeakers = [...new Set(newSegments.map(s => s.speaker || 'Unknown'))];
                const knownSpeakers = uniqueSpeakers.filter(s => s && s !== 'Unknown');
                if (knownSpeakers.length > 0) {
                    knownSpeakers.sort((a, b) => a.localeCompare(b, undefined, {numeric: true, sensitivity: 'base'}));
                    inferredPrimarySpeakers = { count: knownSpeakers.length, names: knownSpeakers };
                } else {
                    // If only "Unknown" speakers, or no speakers, count is 0, names empty
                    inferredPrimarySpeakers = { count: 0, names: [] };
                }
            } else {
                // No segments, so no speakers to infer
                inferredPrimarySpeakers = { count: 0, names: [] };
            }

            // Merge with existing translatedNames:
            updatedSpeakers = {
                count: inferredPrimarySpeakers.count, // Get count from inference
                names: inferredPrimarySpeakers.names,   // Get primary names from inference
                translatedNames: ts.speakers.translatedNames || [] // Preserve existing translatedNames from the store
            };
        }
        // updateProjectStoreState({ statusMessage: path ? `Media transcript loaded.` : 'Media transcript cleared.', error: null }); // Moved later

        let newActiveTranscriptLanguage = ts.activeTranscriptLanguage;
        let finalRawSegmentsToProcess = newSegments; // These are the segments from the 'path' just loaded
        let updatedOriginalSegments = ts.originalSegments;
        let updatedEnglishSegments = ts.englishSegments;
        let updatedOriginalTranscriptPath = ts.originalTranscriptPath;
        let updatedEnglishTranscriptPath = ts.englishTranscriptPath;

        if (path && (path === ts.originalTranscriptPath || (path.endsWith('.json') && !path.endsWith('.en.json')))) {
            updatedOriginalSegments = newSegments;
            updatedOriginalTranscriptPath = path;
            newActiveTranscriptLanguage = 'original';
            // finalRawSegmentsToProcess is already newSegments (correct for original)
        } else if (path && (path === ts.englishTranscriptPath || path.endsWith('.en.json'))) {
            updatedEnglishSegments = newSegments;
            updatedEnglishTranscriptPath = path;
            newActiveTranscriptLanguage = 'english';
            // finalRawSegmentsToProcess is already newSegments (correct for English)
        } else if (!path) { // Clearing data
            newActiveTranscriptLanguage = 'original'; // Default when clearing
            finalRawSegmentsToProcess = [];
            updatedOriginalSegments = [];
            updatedEnglishSegments = [];
            updatedOriginalTranscriptPath = null;
            updatedEnglishTranscriptPath = null;
        }
        // If path is provided but doesn't match known patterns, newActiveTranscriptLanguage remains ts.activeTranscriptLanguage
        // and finalRawSegmentsToProcess remains newSegments. This case might need review if it occurs.

        let finalSegmentsForDisplay = [];
        if (finalRawSegmentsToProcess.length > 0) {
            if (newActiveTranscriptLanguage === 'english') {
                console.log('[TranscriptStore setTranscriptData] Remapping for ENGLISH display using translatedNames.');
                finalSegmentsForDisplay = remapSegmentSpeakerNames([...finalRawSegmentsToProcess], updatedSpeakers, updatedSpeakers.translatedNames);
            } else { // 'original' or any other case defaults to primary names
                console.log('[TranscriptStore setTranscriptData] Remapping for ORIGINAL (or default) display using primary names.');
                finalSegmentsForDisplay = remapSegmentSpeakerNames([...finalRawSegmentsToProcess], updatedSpeakers, updatedSpeakers.names);
            }
        }

        // Only update status message after all decisions are made
        updateProjectStoreState({ statusMessage: path ? `Media transcript loaded.` : 'Media transcript cleared.', error: null });

        // transcribedOriginalLanguageCode and wasTranslatedToEnglish logic removed as per previous commit.

        return {
            ...ts,
            currentTranscriptPath: path,
            segments: finalSegmentsForDisplay,
            originalSegments: updatedOriginalSegments,
            englishSegments: updatedEnglishSegments,
            originalTranscriptPath: updatedOriginalTranscriptPath,
            englishTranscriptPath: updatedEnglishTranscriptPath,
            activeTranscriptLanguage: newActiveTranscriptLanguage,
            isTranscriptLoading: false,
            speakers: updatedSpeakers,
            player: { ...ts.player, currentSegmentIndex: -1 },
            transcriptUndoStack: [],
            transcriptRedoStack: [],
        };
    });
}

export function updateSegment(index, updatedSegmentData, silent = false) {
    console.log("[TranscriptStore] updateSegment called for index:", index, "data:", updatedSegmentData);
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
                 // For text, especially Lexical JSON, a simple !== might not be enough.
                 // Compare stringified versions to detect actual content changes.
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
        console.log("[TranscriptStore] updateSegment: Changes detected, pushing to undo stack and marking dirty.");
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
        console.log("[TranscriptStore] updateSegment: No changes detected.");
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

    // Construct the inner payload object
    const innerPayload = {
        project_xml_path: projectXmlPath,
        media_identifier: mediaIdentifier,
        count: newSpeakerConfig.count,
        names: newSpeakerConfig.names,
        translated_names: newSpeakerConfig.translatedNames
    };

    // Wrap the innerPayload inside an object with the key "payload"
    invoke('save_speaker_config', { payload: innerPayload })
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
                             // Ensure the structure being saved into project.files also includes translated_names
                             // if other parts of the UI expect it directly from here.
                             // The backend saves it to XML, this is about in-memory store consistency.
                             node.speakers = {
                                 '@count': newSpeakerData.count,
                                 name: newSpeakerData.names,
                                 // Assuming the backend will be the source of truth on next load,
                                 // but for immediate UI consistency after save, we can add it here too.
                                 // The key here should match what selectMedia expects (e.g., translated_names or translatedNames)
                                 // Based on selectMedia, it checks for translatedNames then translated_names.
                                 // To be safe and align with backend, using translated_names.
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
                 // Pass the full newSpeakerConfig (which includes translatedNames)
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

export function setAudioBuffer(buffer, peaks = null) {
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
        status = null, // Explicit status like 'initiating', 'running', 'error', 'done', 'cancelled'
        errorMessage = null
    } = options;

    transcriptStore.update((ts) => {
        let updatedState = { ...ts };

        if (isTranscribing) {
            // Safeguard: If trying to set an already completed/failed job to active again, ignore.
            if (jobIdToSet && ts.transcriptionJobId === jobIdToSet &&
                (ts.transcriptionJobStatus === 'done' || ts.transcriptionJobStatus === 'error' || ts.transcriptionJobStatus === 'cancelled')) {
                console.warn(`[JULES-DEBUG TS setStatus] Attempted to set job ${jobIdToSet} to active, but it's already in terminal state: ${ts.transcriptionJobStatus}. Ignoring.`);
                return ts; // Return current state, do not change
            }

            const newActiveMediaDuringStart = mediaPath || ts.selectedMediaFile?.path || ts.activeMediaDuringTranscriptionStart;
            // Determine job status: if explicit status is passed, use it.
            // Otherwise, if a jobId is being set, it's 'running'. If no jobId yet, it's 'initiating'.
            const jobStatusToSet = status || (jobIdToSet ? 'running' : 'initiating');
            const messageToSet = initialProgressMessage || (jobStatusToSet === 'initiating' ? `Initiating...` : `Processing...`);

            updatedState = {
                ...ts,
                isTranscribing: true,
                transcriptionJobId: jobIdToSet !== null ? jobIdToSet : ts.transcriptionJobId,
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
            // isTranscribing is false (job is ending or being cleared)
            const currentJobStatus = status || ts.transcriptionJobStatus;
            let newShowModalConfig = ts.showTranscribeModal; // Default to current modal visibility

            if (currentJobStatus === 'done') {
                // If job is done, modal visibility depends on whether it ran in background.
                // If it ran in background, the custom_transcription_job_completed listener
                // would have already set showTranscribeModal to false and shown a toast.
                // This function should respect that.
                newShowModalConfig = ts.ranInBackground ? false : true;
            } else if (currentJobStatus === 'error' || currentJobStatus === 'cancelled') {
                // For errors or cancellations, always show the modal to display the message.
                newShowModalConfig = true;
            } else if (currentJobStatus === null) {
                // If status is being explicitly cleared to null (e.g., after user acknowledges modal), hide modal.
                newShowModalConfig = false;
            }
            // If currentJobStatus is 'running' or 'initiating' but isTranscribing is false,
            // this is an unusual state. The default newShowModalConfig (ts.showTranscribeModal)
            // will be used, or the specific conditions above will take precedence.

            updatedState = {
                ...ts,
                isTranscribing: false,
                transcriptionJobStatus: currentJobStatus,
                transcriptionErrorMessage: errorMessage || ts.transcriptionErrorMessage,
                showTranscribeModal: newShowModalConfig,
            };

            // If currentJobStatus is being set to null (full reset), clear related fields.
            if (currentJobStatus === null) {
                updatedState.transcriptionJobId = null;
                updatedState.activeMediaDuringTranscriptionStart = null;
                updatedState.mediaPathForLastJob = null;
                updatedState.transcriptionProgress = { percent: 0, message: '' };
                // Reset ranInBackground only when the job is fully cleared to null state.
                // This ensures that if setTranscriptionStatus(false, {status: 'done'}) is called
                // for a background job, 'ranInBackground' is still true for the logic above
                // and for the event listener.
                updatedState.ranInBackground = false;
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
    const currentStore = get(transcriptStore); // Get store state *before* updates

    if (currentStore.isTranscribing && jobFinishedPath === currentStore.mediaPathForLastJob) {
        const wasModalVisibleAtEventTime = currentStore.showTranscribeModal;
        const wasJobRunInBackground = currentStore.ranInBackground;
        const shouldShowToastNotification = wasJobRunInBackground || !wasModalVisibleAtEventTime;

        let finalProgressMessage = '';
        const updatePayload = {};
        let activePathToLoad = null; // For 'done' status data loading
        const pathUpdates = {}; // For 'done' status path updates

        switch (status) {
            case 'done':
                finalProgressMessage = "Transcription successful";
                if (shouldShowToastNotification) {
                    notificationManager.add(finalProgressMessage, "success", 0);
                }
                updatePayload.transcriptionJobStatus = 'done';
                updatePayload.transcriptionErrorMessage = null;
                updatePayload.isTranscribing = false;

                // Logic for determining paths to load (largely existing)
                if (currentStore.selectedMediaFile?.path === jobFinishedPath) {
                    let newOriginalPath = currentStore.originalTranscriptPath;
                    let newEnglishPath = currentStore.englishTranscriptPath;

                    if (transcriptFilePath) {
                        pathUpdates.originalTranscriptPath = transcriptFilePath;
                        newOriginalPath = transcriptFilePath;
                        if (currentStore.activeTranscriptLanguage === 'original' || !translatedTranscriptFilePath) {
                            activePathToLoad = transcriptFilePath;
                        }
                    }
                    if (translatedTranscriptFilePath) {
                        pathUpdates.englishTranscriptPath = translatedTranscriptFilePath;
                        newEnglishPath = translatedTranscriptFilePath;
                        if (currentStore.activeTranscriptLanguage === 'english') {
                            activePathToLoad = translatedTranscriptFilePath;
                        }
                    }
                    if (!activePathToLoad && newOriginalPath) {
                        activePathToLoad = newOriginalPath;
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
                } else {
                    updateProjectStoreState({ error: `Transcription failed: ${errorMessage || 'Unknown error'}` });
                }
                updatePayload.transcriptionJobStatus = 'error';
                updatePayload.transcriptionErrorMessage = errorMessage;
                updatePayload.isTranscribing = false;
                break;
            case 'cancelled':
                finalProgressMessage = "Transcription cancelled";
                if (shouldShowToastNotification) {
                    notificationManager.add(finalProgressMessage, "info", 0);
                } else {
                    updateProjectStoreState({ statusMessage: 'Transcription cancelled.' });
                }
                updatePayload.transcriptionJobStatus = 'cancelled';
                updatePayload.transcriptionErrorMessage = null;
                updatePayload.isTranscribing = false;
                break;
            default:
                console.warn(`[TranscriptStore] Unknown status in custom_transcription_job_completed: ${status}`);
                return; // Don't proceed with updates for unknown status
        }

        updatePayload.showTranscribeModal = shouldShowToastNotification ? false : true;
        updatePayload.transcriptionProgress = { ...currentStore.transcriptionProgress, message: finalProgressMessage };

        transcriptStore.update(ts => ({ ...ts, ...updatePayload }));

        // Perform async operations after store update for 'done' status
        if (status === 'done' && currentStore.selectedMediaFile?.path === jobFinishedPath) {
            if (activePathToLoad) {
                try {
                    const service = await import('../services/projectService.js');
                    if (service.loadTranscriptFile) {
                        console.log(`[TranscriptStore] Loading transcript after job completion: ${activePathToLoad}`);
                        await service.loadTranscriptFile(activePathToLoad);
                    } else {
                        console.error('[TranscriptStore] loadTranscriptFile function not found in projectService.');
                        updateProjectStoreState({ error: 'Internal error: Transcript loading service unavailable.'});
                    }
                } catch (e) {
                    console.error(`[TranscriptStore] Error auto-loading transcript ${activePathToLoad}:`, e);
                    updateProjectStoreState({ error: `Failed to load transcript: ${e.message || e}`});
                }
            }

            try {
                const service = await import('../services/projectService.js');
                if (service.refreshProjectFiles && currentStore.selectedMediaFile?.path) {
                    console.log('[TranscriptStore] Refreshing project files to update transcript associations.');
                    await service.refreshProjectFiles(currentStore.selectedMediaFile.path);

                    // After refreshing, get the latest project files state
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
                        console.log('[TranscriptStore] Found updated media file, dispatching event to re-select it in the transcription tab.');
                        const { emit } = await import('@tauri-apps/api/event');
                        emit('select_media_in_transcription_tab', { mediaPath: updatedMediaFile.path });
                    } else {
                        console.warn(`[TranscriptStore] Could not find the updated media file in project store after refresh for path: ${mediaPath}`);
                    }
                }
            } catch (e) {
                console.error('[TranscriptStore] Error refreshing project files after job completion:', e);
            }
        }

    } else {
         console.log('[TranscriptStore] Received job completion for a job not actively tracked or for a different media path:', jobFinishedPath, currentStore.mediaPathForLastJob, `Is transcribing: ${currentStore.isTranscribing}`);
         // Optionally, explicitly set isTranscribing to false if this job ID was the one being tracked,
         // but the media path doesn't match (e.g. user switched media while job was running)
         // For now, the primary condition handles the main logic flow.
         // Consider if any cleanup is needed if jobID matches but path does not.
         return;
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
