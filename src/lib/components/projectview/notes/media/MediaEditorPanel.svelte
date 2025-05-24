<!-- src/lib/components/projectview/notes/media/MediaEditorPanel.svelte -->
<script>
    import { onMount, onDestroy, tick } from 'svelte';
    import { get } from 'svelte/store';
    import {
        project,
        setLoadedMediaNoteTranscriptData,
        setMediaNoteTranscriptLoadFailed,
        setMediaNoteTranscriptEditorContent,
        markMediaNoteTranscriptAsSaved,
        markMediaNoteTranscriptChangesDiscarded,
        setActiveMediaNoteEditorRef,
        clearActiveMediaNoteEditorRef
    } from '$lib/stores/projectStore.js';
    import { invoke } from '@tauri-apps/api/core';
    import { confirm, message } from '@tauri-apps/plugin-dialog';
    import { basename, dirname, join, sep } from '@tauri-apps/api/path'; // For path manipulation

    import MediaPlayer from '../../transcriptions/MediaPlayer.svelte'; // Reusing the MediaPlayer
    import LexicalEditor from '$lib/components/projectview/lexical/LexicalEditor.svelte';

    export let mediaPath = null; // Full path to the media file

    let lexicalEditorRef;
    let mediaPlayerInNotesRef; // Specific ref for the MediaPlayer in this component

    let localEditorJsonState = '';
    let associatedTranscriptPath = null;
    let transcriptName = 'N/A';

    // State from store for the current media note's transcript
    let currentTranscriptJson = null;
    let initialTranscriptJson = null;
    let isTranscriptDirty = false;
    let isTranscriptLoading = true; // Start as true when path changes
    let transcriptLoadError = null;

    const defaultEmptyJson = JSON.stringify({
        root: {
            children: [{ type: 'paragraph', version: 1, children: [], direction: null, format: '', indent: 0 }],
            direction: null, format: '', indent: 0, type: 'root', version: 1
        }
    });

    // Subscribe to relevant parts of the project store
    const unsubscribeProject = project.subscribe(p => {
        // Only update if this panel's mediaPath matches the store's selectedMediaNotePath
        if (p.selectedMediaNotePath === mediaPath) {
            if (currentTranscriptJson !== p.currentMediaNoteTranscriptJson) {
                currentTranscriptJson = p.currentMediaNoteTranscriptJson;
                if (lexicalEditorRef && localEditorJsonState !== currentTranscriptJson) {
                    console.log(`[MediaEditorPanel Store Sub - ${mediaPath}] Updating editorRef state from store.`);
                    lexicalEditorRef.resetEditorState(currentTranscriptJson || defaultEmptyJson);
                    localEditorJsonState = currentTranscriptJson || defaultEmptyJson;
                }
            }
            if (initialTranscriptJson !== p.initialMediaNoteTranscriptJson) {
                initialTranscriptJson = p.initialMediaNoteTranscriptJson;
            }
            if (isTranscriptDirty !== p.isMediaNoteTranscriptDirty) {
                isTranscriptDirty = p.isMediaNoteTranscriptDirty;
            }
            if (isTranscriptLoading !== p.isMediaNoteTranscriptLoading) {
                isTranscriptLoading = p.isMediaNoteTranscriptLoading;
            }
            if (transcriptLoadError !== p.mediaNoteTranscriptError) {
                transcriptLoadError = p.mediaNoteTranscriptError;
            }
        } else if (mediaPath && p.selectedMediaNotePath !== mediaPath && currentTranscriptJson !== null) {
            // This panel is no longer the active one, clear its specific state
            // console.log(`[MediaEditorPanel Store Sub - ${mediaPath}] No longer active media note. Clearing local state.`);
            // currentTranscriptJson = null; // Keep current until new data is loaded for this path
            // initialTranscriptJson = null;
            // isTranscriptDirty = false;
            // transcriptLoadError = null;
            // if (lexicalEditorRef) lexicalEditorRef.resetEditorState(defaultEmptyJson);
            // localEditorJsonState = defaultEmptyJson;
        }
    });

    // Function to derive the transcript path
    async function deriveTranscriptPath(currentMediaPath) {
        if (!currentMediaPath) return null;
        try {
            const mediaFilename = await basename(currentMediaPath);
            const mediaStem = mediaFilename.includes('.') ? mediaFilename.substring(0, mediaFilename.lastIndexOf('.')) : mediaFilename;
            
            // Path: .../ProjectName/harvey_files/Media/MediaStem/media/MediaStem.mp3
            // Need to go up two levels from 'media' to 'MediaStem', then to 'transcripts'
            const mediaDir = await dirname(currentMediaPath); // .../harvey_files/Media/MediaStem/media
            const mediaStemDir = await dirname(mediaDir);   // .../harvey_files/Media/MediaStem
            
            if (!mediaStemDir) {
                console.error(`[MediaEditorPanel] Could not derive mediaStemDir from ${mediaDir}`);
                return null;
            }
            const transcriptsDir = await join(mediaStemDir, 'transcripts');
            transcriptName = `${mediaStem}.json`;
            return await join(transcriptsDir, transcriptName);
        } catch (e) {
            console.error(`[MediaEditorPanel] Error deriving transcript path for ${currentMediaPath}:`, e);
            return null;
        }
    }

    async function loadTranscript(path) {
        if (!path) {
            setMediaNoteTranscriptLoadFailed(mediaPath, "Associated transcript path could not be determined.");
            return;
        }
        
        project.update(p => {
            // Ensure loading state is specifically for *this* mediaPath
            if (p.selectedMediaNotePath === mediaPath) {
                return { ...p, isMediaNoteTranscriptLoading: true, mediaNoteTranscriptError: null };
            }
            return p;
        });
        localEditorJsonState = defaultEmptyJson; // Reset before load attempt
        if (lexicalEditorRef) lexicalEditorRef.resetEditorState(defaultEmptyJson);


        try {
            console.log(`[MediaEditorPanel - ${mediaPath}] Loading transcript from derived path: ${path}`);
            const jsonContent = await invoke('load_note_json', { filePath: path });

            if (!jsonContent || jsonContent.trim() === '') {
                console.log(`[MediaEditorPanel - ${mediaPath}] Transcript file is empty or not found at ${path}. Using default empty content.`);
                setLoadedMediaNoteTranscriptData(mediaPath, defaultEmptyJson);
                transcriptLoadError = `No transcription found for this media. An empty transcript file will be created if you save.`;
            } else {
                // Validate if it's proper Lexical JSON
                let parsed;
                try {
                    parsed = JSON.parse(jsonContent);
                    if (parsed && parsed.root && parsed.root.children) {
                        setLoadedMediaNoteTranscriptData(mediaPath, jsonContent);
                        transcriptLoadError = null; // Clear previous error
                    } else {
                        throw new Error("Invalid Lexical JSON structure.");
                    }
                } catch (e) {
                    console.warn(`[MediaEditorPanel - ${mediaPath}] Content at ${path} is not valid Lexical JSON. Error: ${e.message}. Displaying as error.`);
                    setMediaNoteTranscriptLoadFailed(mediaPath, "Transcript file contains invalid data.");
                }
            }
        } catch (error) {
            // This catch is for invoke errors (e.g., file not found by Rust)
            console.error(`[MediaEditorPanel - ${mediaPath}] Error loading transcript from ${path}:`, error);
            if (error.message && error.message.toLowerCase().includes('file not found')) {
                 console.log(`[MediaEditorPanel - ${mediaPath}] Transcript file not found at ${path}. Using default empty content and setting specific message.`);
                 setLoadedMediaNoteTranscriptData(mediaPath, defaultEmptyJson); // Still allow editing a new one
                 transcriptLoadError = `No transcription found. An empty transcript file will be created if you save.`;
            } else {
                setMediaNoteTranscriptLoadFailed(mediaPath, error.message || "Failed to load transcript.");
            }
        }
    }

    // Reactive effect to load transcript when mediaPath changes
    let previousMediaPath = null;
    $: if (mediaPath && mediaPath !== previousMediaPath) {
        previousMediaPath = mediaPath;
        console.log(`[MediaEditorPanel] mediaPath changed to: ${mediaPath}`);
        transcriptLoadError = null; // Clear previous errors
        isTranscriptLoading = true; // Set loading true immediately

        deriveTranscriptPath(mediaPath).then(path => {
            associatedTranscriptPath = path;
            if (path) {
                loadTranscript(path);
            } else {
                // This will be handled by loadTranscript, but good to log here too
                console.error(`[MediaEditorPanel - ${mediaPath}] Failed to derive transcript path.`);
                setMediaNoteTranscriptLoadFailed(mediaPath, "Could not determine transcript file location.");
            }
        });
    } else if (!mediaPath && previousMediaPath) {
        // Media path cleared
        previousMediaPath = null;
        associatedTranscriptPath = null;
        transcriptName = 'N/A';
        currentTranscriptJson = null;
        initialTranscriptJson = null;
        isTranscriptDirty = false;
        isTranscriptLoading = false;
        transcriptLoadError = null;
        if (lexicalEditorRef) lexicalEditorRef.resetEditorState(defaultEmptyJson);
        localEditorJsonState = defaultEmptyJson;
        // Also clear the store state if this was the active media note
        if (get(project).selectedMediaNotePath === previousMediaPath) {
            project.update(p => ({
                ...p,
                selectedMediaNotePath: null,
                currentMediaNoteTranscriptJson: null,
                initialMediaNoteTranscriptJson: null,
                isMediaNoteTranscriptDirty: false,
                isMediaNoteTranscriptLoading: false,
                mediaNoteTranscriptError: null,
                activeMediaNoteEditorRef: null,
            }));
        }
    }

    function handleEditorChange(event) {
        const newJson = event.detail.jsonString;
        if (localEditorJsonState !== newJson) {
            localEditorJsonState = newJson;
            // Update the store only if this mediaPath is the currently selected one
            if (get(project).selectedMediaNotePath === mediaPath) {
                setMediaNoteTranscriptEditorContent(mediaPath, newJson);
            }
        }
	}

    async function handleSave() {
        if (!mediaPath) {
            console.error("[MediaEditorPanel] Save Error: No mediaPath for context.");
            await message("Cannot save: No media file is active for this note.", { title: "Save Error", type: "error" });
            return;
        }
        if (!associatedTranscriptPath) {
            console.error(`[MediaEditorPanel - ${mediaPath}] Save Error: Associated transcript path is not determined.`);
            await message("Cannot save: Transcript file location is unknown.", { title: "Save Error", type: "error" });
            return;
        }
        if (isTranscriptLoading || transcriptLoadError && !transcriptLoadError.startsWith("No transcription found")) {
            console.error(`[MediaEditorPanel - ${mediaPath}] Save Error: Cannot save while loading or in error state.`);
            await message(`Cannot save: ${isTranscriptLoading ? 'Transcript is still loading.' : `Transcript failed to load (${transcriptLoadError})`}`, { title: "Save Error", type: "error" });
            return;
        }

        const finalJsonToSave = localEditorJsonState || defaultEmptyJson;

        console.log(`[MediaEditorPanel - ${mediaPath}] Attempting to save transcript to: ${associatedTranscriptPath}`);
        project.update(p => ({ ...p, statusMessage: `Saving transcript ${transcriptName}...`}));

        try {
            // Using save_note_json as it directly writes JSON content to a path.
            // If XML update is needed for these "note transcripts",
            // then save_document_and_update_xml would be used, requiring projectXmlPath etc.
            // For simplicity now, just save to the derived path.
            await invoke('save_note_json', {
                targetPath: associatedTranscriptPath,
                jsonContent: finalJsonToSave
            });

            // Update the store's initial state to match the saved state
            if (get(project).selectedMediaNotePath === mediaPath) {
                markMediaNoteTranscriptAsSaved(mediaPath, finalJsonToSave);
            }
            transcriptLoadError = null; // Clear "No transcription found" message after successful save
            console.log(`[MediaEditorPanel - ${mediaPath}] Transcript save successful to ${associatedTranscriptPath}.`);
            project.update(p => ({ ...p, statusMessage: `Transcript ${transcriptName} saved.`}));

        } catch (error) {
             console.error(`[MediaEditorPanel - ${mediaPath}] Save failed for ${associatedTranscriptPath}:`, error);
             await message(`Failed to save transcript: ${error.message || error}`, { title: 'Save Error', type: 'error' });
             project.update(p => ({ ...p, statusMessage: `Error saving transcript ${transcriptName}.`}));
        }
    }

    async function handleDiscard() {
        // Only use the store's dirty flag for this specific media note path
        const currentStoreState = get(project);
        const dirtyFlagForThisNote = currentStoreState.selectedMediaNotePath === mediaPath && currentStoreState.isMediaNoteTranscriptDirty;

        if (dirtyFlagForThisNote) {
            const userConfirmed = await confirm(`Discard unsaved changes to the transcript for "${mediaPath.split(/[\\/]/).pop()}"?`, { type: 'warning', title: 'Discard Changes' });
            if (userConfirmed) {
                if (get(project).selectedMediaNotePath === mediaPath) {
                    markMediaNoteTranscriptChangesDiscarded(mediaPath); // This reverts store's current to initial
                }
                // The lexicalEditorRef.resetEditorState will be handled by the store subscription
                // when currentMediaNoteTranscriptJson changes due to the discard action.
                console.log(`[MediaEditorPanel - ${mediaPath}] Changes discarded.`);
            }
        } else {
            console.log(`[MediaEditorPanel - ${mediaPath}] Discard skipped: No changes detected in store for this item.`);
        }
    }
    

    onMount(() => {
        console.log(`[MediaEditorPanel] Mounted with mediaPath: ${mediaPath}`);
        setActiveMediaNoteEditorRef(mediaPath, self); // Pass mediaPath as identifier
        
        // Initial load if path is already set and not loaded
        if (mediaPath && !currentTranscriptJson && !isTranscriptLoading && !transcriptLoadError) {
            console.log(`[MediaEditorPanel onMount - ${mediaPath}] Path exists, no data, not loading -> Triggering load.`);
            deriveTranscriptPath(mediaPath).then(path => {
                associatedTranscriptPath = path;
                if (path) loadTranscript(path);
                else setMediaNoteTranscriptLoadFailed(mediaPath, "Could not determine transcript file location.");
            });
        } else if (mediaPath && currentTranscriptJson) {
            console.log(`[MediaEditorPanel onMount - ${mediaPath}] Path and data exist. Ensuring editor state.`);
            localEditorJsonState = currentTranscriptJson;
             if (lexicalEditorRef) lexicalEditorRef.resetEditorState(currentTranscriptJson);
        }
    });

	onDestroy(() => {
        console.log(`[MediaEditorPanel] Destroyed for mediaPath: ${mediaPath}`);
        const activeRefTuple = get(project).activeMediaNoteEditorRef;
        if (activeRefTuple && activeRefTuple.path === mediaPath) {
             clearActiveMediaNoteEditorRef();
        }
        unsubscribeProject();
	});

    // Expose methods for NotesTopBar
    export function save() { return handleSave(); }
    export function discard() { return handleDiscard(); }
    // Needed for external resets if project store re-initializes this view's data
    export function resetEditorState(jsonString) {
        if (lexicalEditorRef) {
            console.log(`[MediaEditorPanel - ${mediaPath}] External resetEditorState called.`);
            lexicalEditorRef.resetEditorState(jsonString || defaultEmptyJson);
            localEditorJsonState = jsonString || defaultEmptyJson;
        }
    }
     export function getItemPath() { return mediaPath; } // For NotesTopBar to identify the active editor

    const self = { save, discard, resetEditorState, getItemPath };

</script>

<div class="flex flex-col h-full w-full bg-white dark:bg-gray-800 rounded-md shadow overflow-hidden">
    <!-- Media Player Section -->
    <div class="flex-shrink-0 border-b border-gray-200 dark:border-gray-700">
        <MediaPlayer 
            bind:this={mediaPlayerInNotesRef}
            mediaPath={mediaPath} 
            isTrimming={false} 
            trimStartTime={0}
            trimEndTime={0}
            isEditingSegment={false} 
            editSegmentStartTime={0}
            editSegmentEndTime={0}
            context="notesView" 
        />
    </div>

    <!-- Transcript Editor Section -->
    <div class="flex-grow min-h-0 overflow-hidden">
        {#if isTranscriptLoading && mediaPath}
            <div class="flex-grow flex items-center justify-center text-gray-500 dark:text-gray-300 p-4">
                Loading transcript for <span class="font-semibold ml-1">{mediaPath.split(/[\\/]/).pop()}</span>...
            </div>
        {:else if transcriptLoadError && mediaPath}
            <div class="flex-grow flex flex-col items-center justify-center text-orange-600 dark:text-orange-400 p-4 text-center">
                <p class="font-semibold">Transcript Error</p>
                <p class="text-xs mt-1">{transcriptLoadError}</p>
                {#if transcriptLoadError.startsWith("No transcription found")}
                    <div class="mt-3 lexical-editor-wrapper-style-placeholder is-disabled w-full flex-grow min-h-[100px]">
                         {#key mediaPath} <!-- Re-key to ensure editor re-initializes if path changes but error was "not found" -->
                            <LexicalEditor
                                bind:this={lexicalEditorRef}
                                initialJson={defaultEmptyJson}
                                editable={true} 
                                placeholder="No transcription found. Start typing here or transcribe from the main 'Transcriptions' view. Changes will be saved to an associated transcript file."
                                on:change={handleEditorChange}
                            />
                        {/key}
                    </div>
                {:else}
                     <p class="text-xs mt-2">Please check the file or try re-importing.</p>
                {/if}
            </div>
        {:else if !mediaPath}
            <div class="flex-grow flex items-center justify-center text-gray-500 dark:text-gray-300 p-4">
                Select a media file from the Notes panel to view its player and transcript.
            </div>
        {:else}
            <!-- Lexical Editor for the transcript -->
            <div class="lexical-editor-wrapper-style w-full h-full">
                {#key mediaPath} <!-- Re-key editor on mediaPath change -->
                    <LexicalEditor
                        bind:this={lexicalEditorRef}
                        initialJson={currentTranscriptJson || defaultEmptyJson}
                        editable={true} 
                        placeholder="Enter or edit transcript text..."
                        on:change={handleEditorChange}
                    />
                {/key}
            </div>
        {/if}
    </div>
</div>

<style lang="postcss">
    .lexical-editor-wrapper-style {
        display: flex;
        flex-direction: column;
        @apply border-none shadow-none overflow-hidden; /* Remove border for seamless look */
    }
    .lexical-editor-wrapper-style > :global(.lexical-editor-root) {
        flex-grow: 1;
        min-height: 0;
        border: none !important;
        border-radius: 0 !important;
        box-shadow: none !important;
        overflow: hidden;
         @apply bg-white dark:bg-gray-800;
    }
    .lexical-editor-wrapper-style > :global(.lexical-editor-root > .lexical-wrapper) {
        overflow-y: auto;
        height: 100%;
        @apply p-3; /* Add padding inside the editor area */
    }
    .lexical-editor-wrapper-style :global(.lexical-content) {
        @apply leading-normal whitespace-pre-wrap break-words text-gray-900 dark:text-gray-100;
        min-height: unset !important;
        font-family: Arial, Helvetica, sans-serif;
        font-size: 12pt;
        line-height: 1.5;
    }
    .lexical-editor-wrapper-style :global(.lexical-content p) {
        @apply mt-0 mb-0;
    }

    /* Placeholder styles if needed when editor is disabled or in error */
    .lexical-editor-wrapper-style-placeholder {
        display: flex;
        flex-direction: column;
        @apply border border-gray-300 dark:border-gray-600 rounded overflow-hidden;
    }
     .lexical-editor-wrapper-style-placeholder.is-disabled {
        @apply bg-gray-100 border-gray-300 opacity-70 dark:bg-gray-700 dark:border-gray-500 dark:opacity-70;
    }

    .flex-grow.min-h-0 {
        min-height: 0;
    }
</style>