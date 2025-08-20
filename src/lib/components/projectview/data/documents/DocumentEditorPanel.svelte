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
        setDocumentHighlights,
        highlightsLastUpdated
    } from '$lib/stores/projectStore.js';
    import { saveDocumentContent } from '$lib/services/projectService.js'; 
    import { invoke } from '@tauri-apps/api/core';
    import LexicalEditor from '$lib/components/projectview/lexical/LexicalEditor.svelte';
    import { confirm, message } from '@tauri-apps/plugin-dialog';

    let editorRef;
    let editorJsonState = '';

    let currentJson = null;
    let initialJson = null;
    let isDirty = false; 
    let isLoading = false;
    let selectedPath = null;
    let errorMessage = null;
    let initialHighlights = []; // New state for highlights

    $: {
        const p = $project;
        selectedPath = p.selectedDocumentPath;
        currentJson = p.currentDocumentJson;
        initialJson = p.initialDocumentJson;
        isDirty = p.isDocumentDirty; 
        isLoading = p.isDocumentLoading;
        errorMessage = (selectedPath && p.error && p.error.includes("Failed to load document")) ? p.error : null;

        // Load highlights when selectedPath changes and it's a document
        if (selectedPath && selectedPath.toLowerCase().endsWith('.json')) { // Assuming lexical docs are .json
            loadHighlightsForDocument(selectedPath);
        } else {
            initialHighlights = []; // Clear highlights for non-lexical docs
        }
    }

    let prevPath = null;
    $: if (selectedPath !== prevPath) {
        prevPath = selectedPath;
        if (selectedPath) {
            console.log(`[DocumentEditorPanel] Detected document path change to: ${selectedPath}`);
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

    async function loadHighlightsForDocument(path) {
        try {
            const loaded = await invoke('load_lexical_highlights', {
                args: {
                    projectId: get(project).id,
                    documentPath: path,
                }
            });
            if (loaded) {
                initialHighlights = JSON.parse(loaded);
            } else {
                initialHighlights = [];
            }
        } catch (e) {
            console.error("Error loading lexical highlights:", e);
            initialHighlights = [];
        }
    }

    function handleEditorChange(event) {
        const newJson = event.detail.jsonString;
        if (editorJsonState !== newJson) {
            editorJsonState = newJson;
            setDocumentEditorContent(editorJsonState); 
        }
	}

    function handleHighlightsChange(event) {
        const { highlights } = event.detail;
        console.log(`[DocumentEditorPanel] Highlights change event received with ${highlights.length} highlights.`);
        if (selectedPath) {
            setDocumentHighlights(highlights);
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

    onMount(() => {
        console.log('[DocumentEditorPanel] Mounted.');
        setActiveDocumentEditorRef({ ref: self });
    });

	onDestroy(() => {
        console.log('[DocumentEditorPanel] Destroyed.');
        if (get(project).activeDocumentEditorRef?.ref === self) {
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
    export function updateLiveTranscriptionText(text, isFinal, startTime, endTime) {
        if (editorRef) {
            editorRef.updateLiveTranscriptionText(text, isFinal, startTime, endTime);
        }
    }

    const self = { save, discard, resetEditorState, updateLiveTranscriptionText };

</script>

<div class="prose prose-sm dark:prose-invert max-w-none flex flex-col h-full w-full bg-white dark:bg-gray-800 rounded-md shadow overflow-hidden exported-transcript">
    {#if isLoading}
        <div class="flex-grow flex items-center justify-center text-gray-500">Loading document...</div>
    {:else if errorMessage && selectedPath}
         <div class="flex-grow flex items-center justify-center text-red-500 p-4 text-center">{errorMessage}</div>
    {:else if !selectedPath}
         <div class="flex-grow flex items-center justify-center text-gray-500">No document selected or loaded.</div>
    {:else}
        <div class="flex-grow min-h-0 overflow-hidden">
             {#key selectedPath}
                 <LexicalEditor
                     bind:this={editorRef}
                     initialJson={currentJson}
                     editable={true}
                     placeholder="Start typing your document..."
                     enableTableCellMenu={true}
                     enableTableCellResize={true}
                     on:change={handleEditorChange}
                     on:highlightschange={handleHighlightsChange}
                     on:highlightssaved={() => highlightsLastUpdated.set(new Date())}
                     enableSearch={true}
                     documentPath={selectedPath}
                     initialHighlights={initialHighlights}
                     documentHighlights={$project.currentDocumentHighlights}
                 />
             {/key}
        </div>
    {/if}
</div>

<style lang="postcss">
	.btn-primary { @apply py-1.5 px-4 bg-blue-500 text-white border-none rounded-md cursor-pointer text-sm font-medium transition-colors duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed disabled:bg-gray-400; }
	.btn-primary:hover:not(:disabled) { @apply bg-blue-600; }
    .btn-secondary { @apply py-1.5 px-4 bg-gray-200 text-gray-800 border border-gray-300 rounded-md cursor-pointer text-sm font-medium transition-colors duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed; }
	.btn-secondary:hover:not(:disabled) { @apply bg-gray-300 border-gray-400; }
	.btn-secondary:disabled { @apply bg-gray-100 text-gray-400 border-gray-200; }
    .btn-primary.text-xs, .btn-secondary.text-xs { @apply py-1 px-2; }

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
