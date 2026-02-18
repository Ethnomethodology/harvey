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
        clearActiveMediaNoteEditorRef,
        setDocumentHighlights,
        highlightsLastUpdated
    } from '$lib/stores/projectStore.js';
    import { invoke } from '@tauri-apps/api/core';
    import { confirm, message } from '@tauri-apps/plugin-dialog';
    import { saveDocumentContent } from '$lib/services/projectService.js';
    import LexicalEditor from '$lib/components/projectview/lexical/LexicalEditor.svelte';
    import { activeLayout } from '$lib/stores/layoutStore.js';

    export let mediaPath = null;
    export let transcriptPath = null;
    export let isPrimary = true;
    export let enableSegmentPlayback = true;
    export let highlightedRowIndex = -1;

    const dispatch = createEventDispatcher();

    let lexicalEditorRef;
    let localEditorJsonState = '';
    
    let localCurrentTranscriptJson = null;
    let localInitialTranscriptJson = null;
    let localIsTranscriptDirty = false;
    let localIsTranscriptLoading = true;
    let localTranscriptLoadError = null;
    let localCurrentHighlights = [];

    const defaultEmptyJson = JSON.stringify({
        root: { children: [{ type: 'paragraph', version: 1, children: [], direction: null, format: '', indent: 0 }],
            direction: null, format: '', indent: 0, type: 'root', version: 1 }
    });

    const mediaToolbarConfig = {
      undo: true, redo: true, blockType: true, bold: true, italic: true,
      underline: true, strikethrough: true, link: true, insertMenu: false,
      indent: true, outdent: true, align: true, textColor: true, highlight: true,
      clearFormatting: true, search: true
    };

    $: currentTranscriptJson = isPrimary ? $project.currentMediaNoteTranscriptJson : localCurrentTranscriptJson;
    $: initialTranscriptJson = isPrimary ? $project.initialMediaNoteTranscriptJson : localInitialTranscriptJson;
    $: isTranscriptDirty = isPrimary ? $project.isMediaNoteTranscriptDirty : localIsTranscriptDirty;
    $: isTranscriptLoading = isPrimary ? $project.isMediaNoteTranscriptLoading : localIsTranscriptLoading;
    $: transcriptLoadError = isPrimary ? $project.mediaNoteTranscriptError : localTranscriptLoadError;
    $: currentHighlights = isPrimary ? $project.currentDocumentHighlights : localCurrentHighlights;

    $: isFileNotFoundInfo = transcriptLoadError === "INFO:FILE_NOT_FOUND";

    // Store sync for primary
    $: if (isPrimary && $project.selectedMediaNotePath === mediaPath) {
        if (lexicalEditorRef && localEditorJsonState !== currentTranscriptJson) {
            lexicalEditorRef.resetEditorState(currentTranscriptJson || defaultEmptyJson);
            localEditorJsonState = currentTranscriptJson || defaultEmptyJson;
        }
    }

    async function loadTranscript(path) {
        if (!path) {
            if (isPrimary) {
                setMediaNoteTranscriptLoadFailed(mediaPath, "Associated transcript/note path could not be determined.", false);
            } else {
                localTranscriptLoadError = "No path provided.";
                localIsTranscriptLoading = false;
            }
            return;
        }

        if (isPrimary) {
            project.update(p => {
                if (p.selectedMediaNotePath === mediaPath) { return { ...p, isMediaNoteTranscriptLoading: true, mediaNoteTranscriptError: null }; }
                return p;
            });
        } else {
            localIsTranscriptLoading = true;
            localTranscriptLoadError = null;
        }

        localEditorJsonState = defaultEmptyJson;
        if (lexicalEditorRef) lexicalEditorRef.resetEditorState(defaultEmptyJson);

        try {
            const jsonContent = await invoke('load_transcript_json', { transcriptPath: path });
            if (!jsonContent || jsonContent.trim() === '') {
                if (isPrimary) setMediaNoteTranscriptLoadFailed(mediaPath, "File not found during load.", true);
                else {
                    localTranscriptLoadError = "INFO:FILE_NOT_FOUND";
                    localIsTranscriptLoading = false;
                }
            } else {
                let parsed = JSON.parse(jsonContent);
                if (parsed && parsed.root && parsed.root.children) {
                    if (isPrimary) setLoadedMediaNoteTranscriptData(mediaPath, jsonContent);
                    else {
                        localCurrentTranscriptJson = jsonContent;
                        localInitialTranscriptJson = jsonContent;
                        localIsTranscriptLoading = false;
                        localIsTranscriptDirty = false;
                    }
                } else { throw new Error("Invalid Lexical JSON structure."); }
            }
        } catch (error) {
            const errorMessage = error.message || String(error);
            const isNotFound = errorMessage.toLowerCase().includes('file not found') || errorMessage.toLowerCase().includes('json file not found');
            if (isPrimary) setMediaNoteTranscriptLoadFailed(mediaPath, errorMessage, isNotFound);
            else {
                localTranscriptLoadError = isNotFound ? "INFO:FILE_NOT_FOUND" : errorMessage;
                localIsTranscriptLoading = false;
            }
        }
    }

    async function loadHighlightsForTranscript(path) {
        if (!path) {
            if (isPrimary) setDocumentHighlights([]);
            else localCurrentHighlights = [];
            return;
        }
        try {
            const projectId = get(project).id;
            const rawHighlights = await invoke('load_lexical_highlights', {
                args: {
                    projectId: projectId,
                    documentPath: path,
                }
            });
            const highlights = rawHighlights ? JSON.parse(rawHighlights) : [];
            if (isPrimary) setDocumentHighlights(highlights);
            else localCurrentHighlights = highlights;
        } catch (e) {
            console.error("[MediaTranscriptEditorSubPanel] Error loading highlights:", e);
            if (isPrimary) setDocumentHighlights([]);
            else localCurrentHighlights = [];
        }
    }

    function handleEditorChange(event) {
        const newJson = event.detail.jsonString;
        if (localEditorJsonState !== newJson) {
            localEditorJsonState = newJson;
            if (isPrimary) {
                if (isFileNotFoundInfo && initialTranscriptJson === defaultEmptyJson) {
                    project.update(p => ({...p, initialMediaNoteTranscriptJson: defaultEmptyJson, mediaNoteTranscriptError: null}));
                }
                setMediaNoteTranscriptEditorContent(mediaPath, newJson);
            } else {
                localCurrentTranscriptJson = newJson;
                localIsTranscriptDirty = localInitialTranscriptJson !== newJson;
            }
        }
        
        const rowCount = getRowCount(newJson);
        dispatch('rowcountupdated', { rowCount });
	}

    function getRowCount(jsonString) {
        if (!jsonString) return 0;
        try {
            const parsed = JSON.parse(jsonString);
            const table = parsed.root.children.find(c => c.type === 'table');
            return table?.children?.length || 0;
        } catch (e) {
            return 0;
        }
    }

    function handleHighlightsChange(event) {
        const { highlights } = event.detail;
        if (isPrimary) setDocumentHighlights(highlights);
        else localCurrentHighlights = highlights;
    }

    async function handleSave() {
        if (!mediaPath) return;
        if (!transcriptPath) return;
        if (isTranscriptLoading || (transcriptLoadError && !isFileNotFoundInfo)) return;

        const finalJsonToSave = localEditorJsonState || defaultEmptyJson;
        try {
            await saveDocumentContent(transcriptPath, finalJsonToSave);
            if (isPrimary) {
                markMediaNoteTranscriptAsSaved(mediaPath, finalJsonToSave);
            } else {
                localInitialTranscriptJson = finalJsonToSave;
                localIsTranscriptDirty = false;
            }
        } catch (error) {
            console.error("[MediaTranscriptEditorSubPanel] Save failed:", error);
            throw error;
        }
    }

    async function handleDiscard() {
        if (isTranscriptDirty) {
            const userConfirmed = await confirm(`Discard unsaved changes to "${transcriptPath.split(/[\/]/).pop()}"?`, { type: 'warning', title: 'Discard Changes' });
            if (userConfirmed) {
                if (isPrimary) markMediaNoteTranscriptChangesDiscarded(mediaPath);
                else {
                    localCurrentTranscriptJson = localInitialTranscriptJson;
                    localIsTranscriptDirty = false;
                }
                if (lexicalEditorRef) lexicalEditorRef.resetEditorState(initialTranscriptJson || defaultEmptyJson);
            }
        }
    }

    onMount(() => {
        if (isPrimary) setActiveMediaNoteEditorRef(mediaPath, self);
        if (transcriptPath) {
            loadTranscript(transcriptPath);
            loadHighlightsForTranscript(transcriptPath);
        }
    });

    onDestroy(() => {
        if (isPrimary) {
            const activeRefTuple = get(project).activeMediaNoteEditorRef;
            if (activeRefTuple && activeRefTuple.path === mediaPath) { clearActiveMediaNoteEditorRef(); }
        }
    });

    export function save() { return handleSave(); }
    export function discard() { return handleDiscard(); }
    export function resetEditorState(jsonString) {
        if (lexicalEditorRef) {
            lexicalEditorRef.resetEditorState(jsonString || defaultEmptyJson);
            localEditorJsonState = jsonString || defaultEmptyJson;
        }
    }
    export function getItemPath() { return transcriptPath; }
    export function getScrollElement() { return lexicalEditorRef?.getScrollElement(); }
    export function getTopVisibleRowInfo() { return lexicalEditorRef?.getTopVisibleRowInfo() || { index: -1, offset: 0 }; }
    export function getCursorRowInfo() { return lexicalEditorRef?.getCursorRowInfo() || { index: -1, offset: 0, visible: false }; }
    export function scrollToRow(index, offset) { lexicalEditorRef?.scrollToRow(index, offset); }

    const self = { save, discard, resetEditorState, getItemPath, getScrollElement, getTopVisibleRowInfo, getCursorRowInfo, scrollToRow };

</script>

<div class="flex flex-col h-full w-full bg-white dark:bg-gray-900 overflow-hidden">
    {#if isTranscriptLoading}
        <div class="flex-grow flex items-center justify-center text-gray-500 dark:text-gray-400 p-4">
            Loading transcript...
        </div>
    {:else if transcriptLoadError}
        {#if isFileNotFoundInfo}
            <div class="flex-grow flex flex-col items-center justify-center text-blue-600 dark:text-blue-400 p-4 text-center">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-10 w-10 mb-2 opacity-70" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z" /></svg>
                <p class="font-semibold">No Transcription Yet</p>
            </div>
        {:else}
            <div class="flex-grow flex flex-col items-center justify-center text-orange-600 dark:text-orange-400 p-4 text-center">
                <p class="font-semibold">Error Loading Data</p>
                <p class="text-xs mt-1">{transcriptLoadError}</p>
            </div>
        {/if}
    {:else}
        <div class="lexical-editor-wrapper-style w-full h-full dark:text-gray-100 layout-{$activeLayout}">
            <LexicalEditor
                bind:this={lexicalEditorRef}
                initialJson={currentTranscriptJson || defaultEmptyJson}
                editable={true}
                enableSegmentPlayback={enableSegmentPlayback}
                placeholder="Enter data for this transcript..."
                externalHighlightedRowIndex={highlightedRowIndex}
                on:change={handleEditorChange}
                on:highlightschange={handleHighlightsChange}
                on:highlightssaved={() => highlightsLastUpdated.set(new Date())}
                on:playsegment={(e) => dispatch('playsegment', e.detail)}
                on:cursorrowchange={(e) => dispatch('cursorrowchange', e.detail)}
                toolbarConfig={mediaToolbarConfig}
                activeLayout={$activeLayout}
                documentPath={transcriptPath}
                documentHighlights={currentHighlights}
            />
        </div>
    {/if}
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
    }
    .lexical-editor-wrapper-style > :global(.lexical-editor-root > .lexical-wrapper) {
        overflow-y: auto;
        height: 100%;
    }
    .lexical-editor-wrapper-style :global(.lexical-content) {
        @apply leading-normal whitespace-pre-wrap break-words;
        min-height: unset !important;
        font-family: Arial, Helvetica, sans-serif;
        font-size: 12pt;
        line-height: 1.5;
    }
    .lexical-editor-wrapper-style :global(.lexical-content table) {
        border-collapse: collapse;
        border-spacing: 0;
        width: 100%;
        border: 1px solid #ccc;
        margin-bottom: 1rem;
        table-layout: fixed;
    }
    .lexical-editor-wrapper-style :global(.lexical-content th),
    .lexical-editor-wrapper-style :global(.lexical-content td) {
        border: 1px solid #ccc;
        padding: 0.2rem 5.75pt;
        text-align: left;
        vertical-align: top;
        font-family: Arial, Helvetica, sans-serif;
        font-size: 12pt;
        line-height: 1.5;
        word-break: break-word;
    }
    .lexical-editor-wrapper-style :global(.lexical-content th) {
        background-color: #f0f0f0;
        font-weight: 600;
    }
</style>
