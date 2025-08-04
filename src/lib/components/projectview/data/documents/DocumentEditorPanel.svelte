<!-- src/lib/components/projectview/documents/DocumentEditorPanel.svelte -->
<script>
    import { onMount, onDestroy, tick } from 'svelte';
    import { get } from 'svelte/store';
    import {
        project,
        setDocumentEditorContent,
        markDocumentChangesDiscarded,
        setActiveDocumentEditorRef,
        clearActiveDocumentEditorRef,
        updateDocumentHighlights
    } from '$lib/stores/projectStore.js';
    import { saveDocumentContent } from '$lib/services/projectService.js'; 
    import LexicalEditor from '$lib/components/projectview/lexical/LexicalEditor.svelte';
    import { confirm, message } from '@tauri-apps/plugin-dialog';
    import { invoke } from '@tauri-apps/api/core';
    import { listen } from '@tauri-apps/api/event';

    let editorRef;
    let editorJsonState = '';
    let isLiveTranscriptionActive = false;
    let liveTranscriptionError = null;

    let currentJson = null;
    let initialJson = null;
    let isDirty = false; 
    let isLoading = false;
    let selectedPath = null;
    let errorMessage = null;

    $: {
        const p = $project;
        selectedPath = p.selectedDocumentPath;
        currentJson = p.currentDocumentJson;
        initialJson = p.initialDocumentJson;
        isDirty = p.isDocumentDirty; 
        isLoading = p.isDocumentLoading;
        errorMessage = (selectedPath && p.error && p.error.includes("Failed to load document")) ? p.error : null;
    }

    let prevPath = null;

    function handlePathChange(newPath) {
        if (isLiveTranscriptionActive) {
            message('Please stop the live transcription before switching to a different document.', { title: 'Live Transcription Active', type: 'warning' });
            return;
        }
        prevPath = newPath;
        if (newPath) {
            console.log(`[DocumentEditorPanel] Detected document path change to: ${newPath}`);
            if (currentJson) { 
                editorJsonState = currentJson;
                 if (editorRef) editorRef.resetEditorState(currentJson);
            } else if (!isLoading) {
                editorJsonState = '';
                 if (editorRef) editorRef.resetEditorState('');
            }
        } else {
            editorJsonState = '';
            if (editorRef) editorRef.resetEditorState('');
        }
    }

    $: if (selectedPath !== prevPath) {
        handlePathChange(selectedPath);
    }

    function handleEditorChange(event) {
        const newJson = event.detail.jsonString;
        if (editorJsonState !== newJson) {
            editorJsonState = newJson;
            setDocumentEditorContent(editorJsonState); 
        }
	}

    function handleHighlightEvent(event) {
        const { type, id, text, nodeKey, color } = event.detail; // Added color
        console.log(`[DocumentEditorPanel] Highlight event received: type=${type}, id=${id}, nodeKey=${nodeKey}, color=${color}`);
        if (selectedPath) {
            updateDocumentHighlights({ type, id, text, nodeKey, color }); 
        }
    }

    async function handleSave() {
        const projState = get(project);
        if (!projState.selectedDocumentPath) {
            console.error("[DocumentEditorPanel] Save Error: No document path selected in store.");
            await message("Cannot save: No document is currently selected.", { title: "Save Error", type: "error"});
            throw new Error("Save Error: No document path selected.");
        }

        if (!projState.isDocumentDirty && !projState.isDocumentMetadataDirty) {
            console.log("[DocumentEditorPanel] handleSave: Content and metadata not dirty. Save skipped.");
            return; 
        }

        console.log("[DocumentEditorPanel] handleSave: Attempting to save document (and/or metadata) via service:", projState.selectedDocumentPath);
        try {
             await saveDocumentContent(projState.selectedDocumentPath, editorJsonState);
             console.log("[DocumentEditorPanel] Document (and/or metadata) save successful via service.");
        } catch (error) {
             console.error("[DocumentEditorPanel] Save operation failed:", error);
             throw error; 
        }
    }

     async function handleDiscard() {
        if (isLiveTranscriptionActive) {
            await message('Please stop the live transcription before discarding changes.', { title: 'Live Transcription Active', type: 'warning' });
            return;
        }
        const proj = get(project);
        if (proj.isDocumentDirty || proj.isDocumentMetadataDirty) {
            const userConfirmed = await confirm('Discard unsaved changes (content and highlights)?', { type: 'warning', title: 'Discard Changes' });
            if (userConfirmed) {
                markDocumentChangesDiscarded(); 
                const revertedJson = get(project).currentDocumentJson; 
                if(editorRef && revertedJson != null) {
                    editorRef.resetEditorState(revertedJson);
                    editorJsonState = revertedJson;
                } else if(editorRef) {
                    editorRef.resetEditorState('');
                    editorJsonState = '';
                }
                 console.log('[DocumentEditorPanel] Changes (content and highlights) discarded.');
            }
        } else {
            console.log('[DocumentEditorPanel] Discard skipped: No changes detected in store for content or metadata.');
        }
    }
	onDestroy(() => {
        console.log('[DocumentEditorPanel] Destroyed.');
        if (get(project).activeDocumentEditorRef === self) {
             clearActiveDocumentEditorRef();
        }
	});

    export function save() {
        console.log('[DocumentEditorPanel] External save() called.');
        return handleSave();
    }
    export function discard() {
        console.log('[DocumentEditorPanel] External discard() called.');
        return handleDiscard();
    }
     export function resetEditorState(jsonString) {
        if (editorRef) {
             console.log('[DocumentEditorPanel] External resetEditorState called.');
             editorRef.resetEditorState(jsonString);
             editorJsonState = jsonString || '';
        }
    }

    const self = { save, discard, resetEditorState };

    let unlisten;

    onMount(async () => {
        console.log('[DocumentEditorPanel] Mounted.');
        setActiveDocumentEditorRef(self);
        unlisten = await listen('live_transcription_result', (event) => {
            if (editorRef) {
                editorRef.insertText(event.payload.text);
            }
        });
    });

	onDestroy(async () => {
        console.log('[DocumentEditorPanel] Destroyed.');
        if (get(project).activeDocumentEditorRef === self) {
             clearActiveDocumentEditorRef();
        }
        if (isLiveTranscriptionActive) {
            await invoke('stop_live_transcription');
        }
        if (unlisten) {
            unlisten();
        }
	});

    async function toggleLiveTranscription() {
        if (isLiveTranscriptionActive) {
            await invoke('stop_live_transcription');
            isLiveTranscriptionActive = false;
            return;
        }

        try {
            isLiveTranscriptionActive = true;
            liveTranscriptionError = null;
            await invoke('start_live_transcription', {
                modelName: 'ggml-tiny.en.bin',
                language: 'en',
            });
        } catch (error) {
            isLiveTranscriptionActive = false;
            liveTranscriptionError = error;
        }
    }

    $: liveTranscriptionStatus = isLiveTranscriptionActive
        ? 'active'
        : liveTranscriptionError
        ? 'error'
        : 'default';
</script>

<div class="prose prose-sm dark:prose-invert max-w-none flex flex-col h-full w-full bg-white dark:bg-gray-800 rounded-md shadow overflow-hidden exported-transcript">
    <div class="relative flex-grow min-h-0">
        {#if isLoading}
            <div class="flex-grow flex items-center justify-center text-gray-500">Loading document...</div>
        {:else if errorMessage && selectedPath}
            <div class="flex-grow flex items-center justify-center text-red-500 p-4 text-center">{errorMessage}</div>
        {:else if !selectedPath}
            <div class="flex-grow flex items-center justify-center text-gray-500">No document selected or loaded.</div>
        {:else}
            <div class="flex-grow min-h-0 overflow-hidden">
                {#if selectedPath && selectedPath.endsWith('.json')}
                    <div class="absolute top-0 right-0 p-2 z-10">
                        <button
                            on:click={toggleLiveTranscription}
                            class:mic-active={liveTranscriptionStatus === 'active'}
                            class:mic-error={liveTranscriptionStatus === 'error'}
                            class="bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 dark:hover:bg-gray-600 p-2 rounded-full"
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-mic" viewBox="0 0 16 16">
                                <path d="M3.5 6.5A.5.5 0 0 1 4 7v1a4 4 0 0 0 8 0V7a.5.5 0 0 1 1 0v1a5 5 0 0 1-4.5 4.975V15h3a.5.5 0 0 1 0 1h-7a.5.5 0 0 1 0-1h3v-2.025A5 5 0 0 1 3 8V7a.5.5 0 0 1 .5-.5"/>
                                <path d="M10 8a2 2 0 1 1-4 0V3a2 2 0 1 1 4 0zM8 0a3 3 0 0 0-3 3v5a3 3 0 0 0 6 0V3a3 3 0 0 0-3-3"/>
                            </svg>
                        </button>
                    </div>
                {/if}
                {#key selectedPath}
                    <LexicalEditor
                        bind:this={editorRef}
                        initialJson={currentJson}
                        editable={true}
                        placeholder="Start typing your document..."
                        enableTableCellMenu={true}
                        enableTableCellResize={true}
                        on:change={handleEditorChange}
                        on:highlightevent={handleHighlightEvent}
                        enableSearch={true}
                    />
                {/key}
            </div>
        {/if}
    </div>
</div>

<style lang="postcss">
	.btn-primary { @apply py-1.5 px-4 bg-blue-500 text-white border-none rounded-md cursor-pointer text-sm font-medium transition-colors duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed disabled:bg-gray-400; }
	.btn-primary:hover:not(:disabled) { @apply bg-blue-600; }
    .btn-secondary { @apply py-1.5 px-4 bg-gray-200 text-gray-800 border border-gray-300 rounded-md cursor-pointer text-sm font-medium transition-colors duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed; }
	.btn-secondary:hover:not(:disabled) { @apply bg-gray-300 border-gray-400; }
	.btn-secondary:disabled { @apply bg-gray-100 text-gray-400 border-gray-200; }
    .btn-primary.text-xs, .btn-secondary.text-xs { @apply py-1 px-2; }

    .mic-active {
        color: red;
        animation: blink 1s infinite;
    }

    .mic-error {
        color: orange;
    }

    @keyframes blink {
        0% {
            color: red;
        }
        50% {
            color: pink;
        }
        100% {
            color: red;
        }
    }

     :global(.lexical-wrapper) {
        flex-grow: 1;
        overflow-y: auto;
        @apply p-3 m-0;
    }
     :global(.lexical-wrapper > .lexical-editor-root > *) {
         @apply mt-0 mb-0;
     }

    :global(.lexical-wrapper)::-webkit-scrollbar { @apply w-[8px] h-[8px]; }
	:global(.lexical-wrapper)::-webkit-scrollbar-track { @apply bg-gray-100 dark:bg-gray-800 rounded-lg; }
	:global(.lexical-wrapper)::-webkit-scrollbar-thumb { @apply bg-gray-400 dark:bg-gray-500 rounded-lg border-2 border-solid border-gray-100 dark:border-gray-800; }
	:global(.lexical-wrapper)::-webkit-scrollbar-thumb:hover { @apply bg-gray-500 dark:bg-gray-400; }
	:global(.lexical-wrapper) { scrollbar-width: thin; scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track); }
	:root { --scrollbar-thumb: rgba(160, 174, 192, 1); --scrollbar-track: rgba(243, 244, 246, 1); }
	html.dark { --scrollbar-thumb: rgba(107, 114, 128, 1); --scrollbar-track: rgba(31, 41, 55, 1); }

     .flex-grow.min-h-0 { min-height: 0; }
</style>