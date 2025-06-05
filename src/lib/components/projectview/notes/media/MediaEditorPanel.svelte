<!-- src/lib/components/projectview/notes/media/MediaEditorPanel.svelte -->
<script>
    import { onMount, onDestroy, tick, createEventDispatcher } from 'svelte';
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
    import { basename, dirname, join } from '@tauri-apps/api/path';

    import MediaPlayer from '../../MediaPlayer.svelte';
    import LexicalEditor from '$lib/components/projectview/lexical/LexicalEditor.svelte';

    export let mediaPath = null;

    const dispatch = createEventDispatcher();

    let lexicalEditorRef;
    let mediaPlayerInNotesRef;

    let localEditorJsonState = '';
    let associatedTranscriptPath = null;
    let transcriptName = 'N/A';

    let currentTranscriptJson = null;
    let initialTranscriptJson = null;
    let isTranscriptDirty = false;
    let isTranscriptLoading = true;
    let transcriptLoadError = null; // Can be "INFO:FILE_NOT_FOUND" or actual error string
    
    $: isFileNotFoundInfo = transcriptLoadError === "INFO:FILE_NOT_FOUND";

    const defaultEmptyJson = JSON.stringify({
        root: {
            children: [{ type: 'paragraph', version: 1, children: [], direction: null, format: '', indent: 0 }],
            direction: null, format: '', indent: 0, type: 'root', version: 1
        }
    });

    const unsubscribeProject = project.subscribe(p => {
        if (p.selectedMediaNotePath === mediaPath) {
            if (currentTranscriptJson !== p.currentMediaNoteTranscriptJson) {
                currentTranscriptJson = p.currentMediaNoteTranscriptJson;
                if (lexicalEditorRef && localEditorJsonState !== currentTranscriptJson) {
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
        }
    });

    async function deriveTranscriptPath(currentMediaPath) {
        if (!currentMediaPath) return null;
        try {
            const mediaFilename = await basename(currentMediaPath);
            const mediaStem = mediaFilename.includes('.') ? mediaFilename.substring(0, mediaFilename.lastIndexOf('.')) : mediaFilename;
            transcriptName = mediaStem;

            const mediaDir = await dirname(currentMediaPath);
            const mediaParentDir = await dirname(mediaDir);

            if (!mediaParentDir) {
                console.error(`[MediaEditorPanel] Could not derive mediaParentDir from ${mediaDir}`);
                return null;
            }
            const notesDir = await join(mediaParentDir, 'transcripts');
            return await join(notesDir, `${mediaStem}.json`);
        } catch (e) {
            console.error(`[MediaEditorPanel] Error deriving transcript path for ${currentMediaPath}:`, e);
            return null;
        }
    }

    async function loadTranscript(path) {
        if (!path) {
            setMediaNoteTranscriptLoadFailed(mediaPath, "Associated transcript/note path could not be determined.", false);
            return;
        }

        project.update(p => {
            if (p.selectedMediaNotePath === mediaPath) {
                return { ...p, isMediaNoteTranscriptLoading: true, mediaNoteTranscriptError: null };
            }
            return p;
        });
        localEditorJsonState = defaultEmptyJson;
        if (lexicalEditorRef) lexicalEditorRef.resetEditorState(defaultEmptyJson);

        try {
            console.log(`[MediaEditorPanel - ${mediaPath || 'NO_PATH'}] Loading notes from derived path: ${path}`);
            const jsonContent = await invoke('load_note_json', { filePath: path });

            if (!jsonContent || jsonContent.trim() === '') {
                console.log(`[MediaEditorPanel - ${mediaPath || 'NO_PATH'}] Notes file is empty or not found at ${path}. Setting as INFO:FILE_NOT_FOUND.`);
                setMediaNoteTranscriptLoadFailed(mediaPath, "File not found during load.", true); // isFileNotFound = true
            } else {
                let parsed;
                try {
                    parsed = JSON.parse(jsonContent);
                    if (parsed && parsed.root && parsed.root.children) {
                        setLoadedMediaNoteTranscriptData(mediaPath, jsonContent);
                    } else {
                        throw new Error("Invalid Lexical JSON structure.");
                    }
                } catch (e) {
                    console.warn(`[MediaEditorPanel - ${mediaPath || 'NO_PATH'}] Content at ${path} is not valid Lexical JSON. Error: ${e.message}.`);
                    setMediaNoteTranscriptLoadFailed(mediaPath, "Note file contains invalid data.", false);
                }
            }
        } catch (error) {
            console.error(`[MediaEditorPanel - ${mediaPath || 'NO_PATH'}] Error loading notes from ${path}:`, error);
            const errorMessage = error.message || String(error);
            if (errorMessage.toLowerCase().includes('file not found') || errorMessage.toLowerCase().includes('json file not found')) {
                 console.log(`[MediaEditorPanel - ${mediaPath || 'NO_PATH'}] Notes file not found at ${path}. Setting as INFO:FILE_NOT_FOUND.`);
                 setMediaNoteTranscriptLoadFailed(mediaPath, "File not found during load attempt.", true); // isFileNotFound = true
            } else {
                setMediaNoteTranscriptLoadFailed(mediaPath, errorMessage, false);
            }
        }
    }

    let previousMediaPath = null;
    $: if (mediaPath && mediaPath !== previousMediaPath) {
        previousMediaPath = mediaPath;
        console.log(`[MediaEditorPanel] mediaPath changed to: ${mediaPath}`);

        deriveTranscriptPath(mediaPath).then(path => {
            associatedTranscriptPath = path;
            if (path) {
                loadTranscript(path);
            } else {
                console.error(`[MediaEditorPanel - ${mediaPath}] Failed to derive notes path.`);
                setMediaNoteTranscriptLoadFailed(mediaPath, "Could not determine note file location.", false);
            }
        });
    } else if (!mediaPath && previousMediaPath) {
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
            if (get(project).selectedMediaNotePath === mediaPath) {
                // If it was "file not found", typing makes it dirty against an empty initial state
                if (isFileNotFoundInfo && initialTranscriptJson === defaultEmptyJson) {
                    project.update(p => ({...p, initialMediaNoteTranscriptJson: defaultEmptyJson, mediaNoteTranscriptError: null}));
                }
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
            console.error(`[MediaEditorPanel - ${mediaPath}] Save Error: Associated notes path is not determined.`);
            await message("Cannot save: Note file location is unknown.", { title: "Save Error", type: "error" });
            return;
        }

        if (isTranscriptLoading || (transcriptLoadError && !isFileNotFoundInfo)) {
            console.error(`[MediaEditorPanel - ${mediaPath}] Save Error: Cannot save while loading or in error state (and not file not found info).`);
            await message(`Cannot save: ${isTranscriptLoading ? 'Note is still loading.' : `Note failed to load (${transcriptLoadError})`}`, { title: "Save Error", type: "error" });
            return;
        }

        const finalJsonToSave = localEditorJsonState || defaultEmptyJson;

        console.log(`[MediaEditorPanel - ${mediaPath}] Attempting to save notes to: ${associatedTranscriptPath}`);
        project.update(p => ({ ...p, statusMessage: `Saving notes for ${transcriptName}...`}));

        try {
            await invoke('save_note_json', {
                targetPath: associatedTranscriptPath,
                jsonContent: finalJsonToSave
            });

            if (get(project).selectedMediaNotePath === mediaPath) {
                markMediaNoteTranscriptAsSaved(mediaPath, finalJsonToSave);
            }
            console.log(`[MediaEditorPanel - ${mediaPath}] Notes save successful to ${associatedTranscriptPath}.`);
            project.update(p => ({ ...p, statusMessage: `Notes for ${transcriptName} saved.`}));

        } catch (error) {
             console.error(`[MediaEditorPanel - ${mediaPath}] Save failed for ${associatedTranscriptPath}:`, error);
             await message(`Failed to save notes: ${error.message || error}`, { title: 'Save Error', type: 'error' });
             project.update(p => ({ ...p, statusMessage: `Error saving notes for ${transcriptName}.`}));
        }
    }

    async function handleDiscard() {
        const currentStoreState = get(project);
        const dirtyFlagForThisNote = currentStoreState.selectedMediaNotePath === mediaPath && currentStoreState.isMediaNoteTranscriptDirty;

        if (dirtyFlagForThisNote) {
            const userConfirmed = await confirm(`Discard unsaved changes to the notes for "${mediaPath.split(/[\\/]/).pop()}"?`, { type: 'warning', title: 'Discard Changes' });
            if (userConfirmed) {
                if (get(project).selectedMediaNotePath === mediaPath) {
                    markMediaNoteTranscriptChangesDiscarded(mediaPath);
                }
                console.log(`[MediaEditorPanel - ${mediaPath}] Changes discarded.`);
            }
        } else {
            console.log(`[MediaEditorPanel - ${mediaPath}] Discard skipped: No changes detected in store for this item.`);
        }
    }


    onMount(() => {
        console.log(`[MediaEditorPanel] Mounted with mediaPath: ${mediaPath}`);
        setActiveMediaNoteEditorRef(mediaPath, self);

        if (mediaPath && !currentTranscriptJson && !isTranscriptLoading && !transcriptLoadError) {
            console.log(`[MediaEditorPanel onMount - ${mediaPath}] Path exists, no data, not loading -> Triggering load.`);
            deriveTranscriptPath(mediaPath).then(path => {
                associatedTranscriptPath = path;
                if (path) loadTranscript(path);
                else {
                    setMediaNoteTranscriptLoadFailed(mediaPath, "Could not determine note file location.", false);
                }
            });
        } else if (mediaPath && currentTranscriptJson) {
            console.log(`[MediaEditorPanel onMount - ${mediaPath}] Path and data exist. Ensuring editor state.`);
            localEditorJsonState = currentTranscriptJson;
             if (lexicalEditorRef) lexicalEditorRef.resetEditorState(currentTranscriptJson);
        } else if (!mediaPath) {
            console.log(`[MediaEditorPanel onMount] No mediaPath provided on mount. Clearing states.`);
            isTranscriptLoading = false;
            transcriptLoadError = null;
            localEditorJsonState = defaultEmptyJson;
            if (lexicalEditorRef) lexicalEditorRef.resetEditorState(defaultEmptyJson);
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

    export function save() { return handleSave(); }
    export function discard() { return handleDiscard(); }
    export function resetEditorState(jsonString) {
        if (lexicalEditorRef) {
            console.log(`[MediaEditorPanel - ${mediaPath || 'NO_PATH'}] External resetEditorState called.`);
            lexicalEditorRef.resetEditorState(jsonString || defaultEmptyJson);
            localEditorJsonState = jsonString || defaultEmptyJson;
        }
    }
    export function getItemPath() { return mediaPath; }

    const self = { save, discard, resetEditorState, getItemPath };

    function handleRequestNotesTranscribe(event) {
        console.log('[MediaEditorPanel] Requesting Transcribe Tab with media:', event.detail.mediaPath);
        dispatch('requestTranscriptionTabWithMedia', { mediaPath: event.detail.mediaPath });
    }

    function handleRequestNotesTrim(event) {
        console.log('[MediaEditorPanel] Requesting Trim in Transcribe Tab for media:', event.detail.mediaPath);
        dispatch('requestTrimInTranscriptionTab', { mediaPath: event.detail.mediaPath });
    }

</script>

<div class="flex flex-col h-full w-full bg-white dark:bg-gray-800 rounded-md shadow overflow-hidden">
    <div class="flex-shrink-0 border-b border-gray-200 dark:border-gray-700">
        {#if mediaPath}
            <MediaPlayer
                bind:this={mediaPlayerInNotesRef}
                explicitMediaPath={mediaPath}
                showLoopPauseButton={false}
                showNotesTranscribeButton={true}
                showNotesTrimButton={true}
                on:requestNotesTranscribe={handleRequestNotesTranscribe}
                on:requestNotesTrim={handleRequestNotesTrim}
                on:mediaLoadError={(e) => project.update(p => ({...p, statusMessage: `Error loading media in notes: ${e.detail.error}`}))}
            />
        {:else}
            <div class="w-full max-w-[36rem] aspect-video bg-black relative mx-auto mb-1 flex items-center justify-center text-gray-500 dark:text-gray-400">
                <span>Media player requires a path.</span>
            </div>
        {/if}
    </div>

    <div class="flex-grow min-h-0 overflow-hidden">
        {#if isTranscriptLoading && mediaPath}
            <div class="flex-grow flex items-center justify-center text-gray-500 dark:text-gray-300 p-4">
                Loading notes for <span class="font-semibold ml-1">{transcriptName}</span>...
            </div>
        {:else if transcriptLoadError && mediaPath}
            {#if isFileNotFoundInfo}
                <div class="flex-grow flex flex-col items-center justify-center text-blue-600 dark:text-blue-400 p-4 text-center">
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-10 w-10 mb-2 opacity-70" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z" /></svg>
                    <p class="font-semibold">No Transcription Yet</p>
                    <p class="text-xs mt-1">
                        Please click on the <span class="font-medium">Transcribe</span> button above the player to open this media in the Transcriptions tab and generate one.
                    </p>
                    <!-- LexicalEditor is NOT shown in this specific informational state -->
                </div>
            {:else}
                <div class="flex-grow flex flex-col items-center justify-center text-orange-600 dark:text-orange-400 p-4 text-center">
                    <p class="font-semibold">Error Loading Notes</p>
                    <p class="text-xs mt-1">{transcriptLoadError}</p>
                    <p class="text-xs mt-2">Please check the file or try again.</p>
                </div>
            {/if}
        {:else if !mediaPath}
            <div class="flex-grow flex items-center justify-center text-gray-500 dark:text-gray-300 p-4">
                Select an audio or video file from the Fieldnotes panel to view its player and notes.
            </div>
        {:else}
            <div class="lexical-editor-wrapper-style w-full h-full dark:text-gray-100">
                {#key mediaPath}
                    <LexicalEditor
                        bind:this={lexicalEditorRef}
                        initialJson={currentTranscriptJson || defaultEmptyJson}
                        editable={true}
                        placeholder="Enter notes for this media file..."
                        on:change={handleEditorChange}
                        enableSearch={true}
                        toolbarConfig={{ insertMenu: false }}
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
        @apply border-none shadow-none overflow-hidden;
    }
    .lexical-editor-wrapper-style > :global(.lexical-editor-root) {
        flex-grow: 1;
        min-height: 0;
        border: none !important;
        border-radius: 0 !important;
        box-shadow: none !important;
        overflow: hidden;
        /* Removed: @apply bg-white dark:bg-gray-800; */
    }
    .lexical-editor-wrapper-style > :global(.lexical-editor-root > .lexical-wrapper) {
        overflow-y: auto;
        height: 100%;
        @apply p-3;
    }
    .lexical-editor-wrapper-style :global(.lexical-content) {
        @apply leading-normal whitespace-pre-wrap break-words;
        min-height: unset !important;
        font-family: Arial, Helvetica, sans-serif;
        font-size: 12pt;
        line-height: 1.5;
    }
    .lexical-editor-wrapper-style :global(.lexical-content p) {
        @apply mt-0 mb-0;
    }

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