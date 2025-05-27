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
    import { basename, dirname, join, sep } from '@tauri-apps/api/path';

    import MediaPlayer from '../../transcriptions/MediaPlayer.svelte';
    import LexicalEditor from '$lib/components/projectview/lexical/LexicalEditor.svelte';

    export let mediaPath = null; 

    let lexicalEditorRef;
    let mediaPlayerInNotesRef; 

    let localEditorJsonState = '';
    let associatedTranscriptPath = null;
    let transcriptName = 'N/A';

    let currentTranscriptJson = null;
    let initialTranscriptJson = null;
    let isTranscriptDirty = false;
    let isTranscriptLoading = true; 
    let transcriptLoadError = null;

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
                    console.log(`[MediaEditorPanel Store Sub - ${mediaPath || 'NO_PATH'}] Updating editorRef state from store.`);
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
        } else if (mediaPath && p.selectedMediaNotePath !== mediaPath && (currentTranscriptJson !== null || transcriptLoadError !== null || isTranscriptLoading )) {
            // This panel is no longer the active one, or has become active but path changed.
            // We don't clear its data here; the reactive block below handles loading for the new mediaPath.
        }
    });

    async function deriveTranscriptPath(currentMediaPath) {
        if (!currentMediaPath) return null;
        try {
            const mediaFilename = await basename(currentMediaPath);
            const mediaStem = mediaFilename.includes('.') ? mediaFilename.substring(0, mediaFilename.lastIndexOf('.')) : mediaFilename;
            
            const mediaDir = await dirname(currentMediaPath); 
            const mediaStemDir = await dirname(mediaDir);   
            
            if (!mediaStemDir) {
                console.error(`[MediaEditorPanel] Could not derive mediaStemDir from ${mediaDir}`);
                return null;
            }
            const transcriptsDir = await join(mediaStemDir, 'transcripts'); // Assuming notes are stored in 'transcripts' subfolder
            transcriptName = `${mediaStem}.json`;
            return await join(transcriptsDir, transcriptName);
        } catch (e) {
            console.error(`[MediaEditorPanel] Error deriving transcript path for ${currentMediaPath}:`, e);
            return null;
        }
    }

    async function loadTranscript(path) {
        if (!path) {
            setMediaNoteTranscriptLoadFailed(mediaPath, "Associated transcript/note path could not be determined.");
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
            console.log(`[MediaEditorPanel - ${mediaPath || 'NO_PATH'}] Loading transcript/note from derived path: ${path}`);
            const jsonContent = await invoke('load_note_json', { filePath: path });

            if (!jsonContent || jsonContent.trim() === '') {
                console.log(`[MediaEditorPanel - ${mediaPath || 'NO_PATH'}] Transcript/note file is empty or not found at ${path}.`);
                setLoadedMediaNoteTranscriptData(mediaPath, defaultEmptyJson);
                transcriptLoadError = `No notes found. An empty file will be created on save.`;
            } else {
                let parsed;
                try {
                    parsed = JSON.parse(jsonContent);
                    if (parsed && parsed.root && parsed.root.children) {
                        setLoadedMediaNoteTranscriptData(mediaPath, jsonContent);
                        transcriptLoadError = null; 
                    } else {
                        throw new Error("Invalid Lexical JSON structure.");
                    }
                } catch (e) {
                    console.warn(`[MediaEditorPanel - ${mediaPath || 'NO_PATH'}] Content at ${path} is not valid Lexical JSON. Error: ${e.message}.`);
                    setMediaNoteTranscriptLoadFailed(mediaPath, "Note file contains invalid data.");
                }
            }
        } catch (error) {
            console.error(`[MediaEditorPanel - ${mediaPath || 'NO_PATH'}] Error loading transcript/note from ${path}:`, error);
            if (error.message && error.message.toLowerCase().includes('file not found')) {
                 console.log(`[MediaEditorPanel - ${mediaPath || 'NO_PATH'}] Transcript/note file not found at ${path}.`);
                 setLoadedMediaNoteTranscriptData(mediaPath, defaultEmptyJson); 
                 transcriptLoadError = `No notes found. An empty file will be created on save.`;
            } else {
                setMediaNoteTranscriptLoadFailed(mediaPath, error.message || "Failed to load notes.");
            }
        }
    }

    let previousMediaPath = null;
    $: if (mediaPath && mediaPath !== previousMediaPath) {
        previousMediaPath = mediaPath;
        console.log(`[MediaEditorPanel] mediaPath changed to: ${mediaPath}`);
        transcriptLoadError = null; 
        isTranscriptLoading = true;

        deriveTranscriptPath(mediaPath).then(path => {
            associatedTranscriptPath = path;
            if (path) {
                loadTranscript(path);
            } else {
                console.error(`[MediaEditorPanel - ${mediaPath}] Failed to derive transcript/note path.`);
                setMediaNoteTranscriptLoadFailed(mediaPath, "Could not determine note file location.");
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
            console.error(`[MediaEditorPanel - ${mediaPath}] Save Error: Associated transcript/note path is not determined.`);
            await message("Cannot save: Note file location is unknown.", { title: "Save Error", type: "error" });
            return;
        }
        if (isTranscriptLoading || (transcriptLoadError && !transcriptLoadError.startsWith("No notes found"))) {
            console.error(`[MediaEditorPanel - ${mediaPath}] Save Error: Cannot save while loading or in error state.`);
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
            transcriptLoadError = null; 
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
                else setMediaNoteTranscriptLoadFailed(mediaPath, "Could not determine note file location.");
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

</script>

<div class="flex flex-col h-full w-full bg-white dark:bg-gray-800 rounded-md shadow overflow-hidden">
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

    <div class="flex-grow min-h-0 overflow-hidden">
        {#if isTranscriptLoading && mediaPath}
            <div class="flex-grow flex items-center justify-center text-gray-500 dark:text-gray-300 p-4">
                Loading notes for <span class="font-semibold ml-1">{mediaPath.split(/[\\/]/).pop()}</span>...
            </div>
        {:else if transcriptLoadError && mediaPath}
            <div class="flex-grow flex flex-col items-center justify-center text-orange-600 dark:text-orange-400 p-4 text-center">
                <p class="font-semibold">Error Loading Notes</p>
                <p class="text-xs mt-1">{transcriptLoadError}</p>
                {#if transcriptLoadError.startsWith("No notes found")}
                    <div class="mt-3 lexical-editor-wrapper-style-placeholder is-disabled w-full flex-grow min-h-[100px]">
                         {#key mediaPath}
                            <LexicalEditor
                                bind:this={lexicalEditorRef}
                                initialJson={defaultEmptyJson}
                                editable={true} 
                                placeholder="No notes found. Start typing notes here. Changes will be saved to an associated JSON file."
                                on:change={handleEditorChange}
                            />
                        {/key}
                    </div>
                {:else}
                     <p class="text-xs mt-2">Please check the file or try again.</p>
                {/if}
            </div>
        {:else if !mediaPath}
            <div class="flex-grow flex items-center justify-center text-gray-500 dark:text-gray-300 p-4">
                Select an audio or video file from the Fieldnotes panel to view its player and notes.
            </div>
        {:else}
            <div class="lexical-editor-wrapper-style w-full h-full">
                {#key mediaPath} 
                    <LexicalEditor
                        bind:this={lexicalEditorRef}
                        initialJson={currentTranscriptJson || defaultEmptyJson}
                        editable={true} 
                        placeholder="Enter notes for this media file..."
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
        @apply border-none shadow-none overflow-hidden; 
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
        @apply p-3; 
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