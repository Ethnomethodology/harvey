<!-- src/lib/components/projectview/notes/imported_transcripts/TranscriptEditorPanel.svelte -->
<script>
    import { onMount, onDestroy, tick } from 'svelte';
    import { get } from 'svelte/store';
    import {
        project,
        setImportedTranscriptEditorContent,
        markImportedTranscriptAsSaved,
        markImportedTranscriptChangesDiscarded,
        setActiveImportedTranscriptEditorRef,
        clearActiveImportedTranscriptEditorRef,
        setLoadedImportedTranscriptData,
        setImportedTranscriptLoadFailed,
        setImportedTranscriptHighlights,
        highlightsLastUpdated
    } from '$lib/stores/projectStore.js';
    import { invoke } from '@tauri-apps/api/core';
    import { confirm, message } from '@tauri-apps/plugin-dialog';
    import LexicalEditor from '$lib/components/projectview/lexical/LexicalEditor.svelte';

    import { createHeadlessEditor } from '@lexical/headless';
    import {
        $getRoot as lexicalGetRoot,
        $createParagraphNode as lexicalCreateParagraphNode,
        $createTextNode as lexicalCreateTextNode,
        $isElementNode as lexicalIsElementNode,
        TextNode, ParagraphNode, RootNode, LineBreakNode, ElementNode
    } from 'lexical';
    import {
        $generateNodesFromDOM as lexicalGenerateNodesFromDOM,
        // $generateHtmlFromNodes is no longer needed here
    } from '@lexical/html';
    import {
      $createTableNode as lexicalCreateTableNode,
      $createTableRowNode as lexicalCreateTableRowNode,
      $createTableCellNode as lexicalCreateTableCellNode,
      $isTableNode as lexicalIsTableNode,
      TableNode, TableRowNode, TableCellNode
    } from '@lexical/table';
    import { HeadingNode, QuoteNode, registerRichText } from '@lexical/rich-text';
    import { ListNode, ListItemNode, registerList } from '@lexical/list';
    import { LinkNode } from '@lexical/link';
    import { ExtendedTextNode, $createExtendedTextNode as lexicalCreateExtendedTextNode } from '$lib/nodes/ExtendedTextNode.js';

    const LEXICAL_NODES = [
      TableNode,
      TableRowNode,
      TableCellNode,
      HeadingNode,
      QuoteNode,
      ListNode,
      ListItemNode,
      LinkNode,
      ExtendedTextNode
    ];

    export let itemPath = null;

    let editorRef;
    let editorJsonState = ''; // Holds the current Lexical JSON string

    let currentLexicalJson = null; // From store
    let initialLexicalJson = null; // From store
    let isDirty = false;
    let isLoading = true;
    let selectedPath = null;
    let errorMessage = null;

    // New state for highlights
    let initialHighlights = [];
    let isMetadataDirty = false;

    const ALL_CONVERSION_NODES = [
        RootNode, ParagraphNode, TextNode, ExtendedTextNode, LineBreakNode,
        HeadingNode, QuoteNode, ListNode, ListItemNode, LinkNode,
        TableNode, TableRowNode, TableCellNode
    ];

    let changeDebounceTimeout;

    // --- Store Subscription (Unchanged) ---
    project.subscribe(p => {
        if (p.currentImportedTranscriptPath === itemPath) {
            selectedPath = p.currentImportedTranscriptPath;
            if (currentLexicalJson !== p.currentImportedTranscriptLexicalJson) {
                 currentLexicalJson = p.currentImportedTranscriptLexicalJson;
                 if (editorRef && editorJsonState !== currentLexicalJson) {
                     console.log("[TranscriptEditorPanel Store Sub] Updating editorRef state from store.");
                     editorRef.resetEditorState(currentLexicalJson);
                     editorJsonState = currentLexicalJson;
                 }
            }
            if (initialLexicalJson !== p.initialImportedTranscriptLexicalJson) {
                initialLexicalJson = p.initialImportedTranscriptLexicalJson;
            }
            if (isDirty !== p.isImportedTranscriptDirty) {
                isDirty = p.isImportedTranscriptDirty;
            }
             if (isLoading !== p.isImportedTranscriptLoading) {
                isLoading = p.isImportedTranscriptLoading;
            }
            if (errorMessage !== p.importedTranscriptError) {
                errorMessage = p.importedTranscriptError;
            }

            // Sync highlight state
            if (initialHighlights !== p.initialImportedTranscriptHighlights) {
                initialHighlights = p.initialImportedTranscriptHighlights;
            }
            if (isMetadataDirty !== p.isImportedTranscriptMetadataDirty) {
                isMetadataDirty = p.isImportedTranscriptMetadataDirty;
            }

        } else if (itemPath && p.currentImportedTranscriptPath !== itemPath && selectedPath === itemPath) {
            selectedPath = null;
            currentLexicalJson = null;
            initialLexicalJson = null;
            isDirty = false;
            isLoading = false;
            errorMessage = null;

            initialHighlights = [];
            isMetadataDirty = false;

            if (editorRef) editorRef.resetEditorState('');
            editorJsonState = '';
        }
    });

    // --- Path Change Reaction (REFACTORED) ---
    let hasLoadedOnceForPath = null; // Prevent re-loading on non-path changes
    $: if (itemPath && editorRef) {
        // Only trigger if the path is new
        if (itemPath !== hasLoadedOnceForPath) {
            console.log(`[TranscriptEditorPanel] Reactive trigger for new path: ${itemPath}`);
            hasLoadedOnceForPath = itemPath;
            loadAndConvertTranscript(itemPath);
            loadHighlightsForTranscript(itemPath);
        }
    } else if (!itemPath && hasLoadedOnceForPath) {
        // Reset when itemPath becomes null to allow re-loading if it's selected again
        hasLoadedOnceForPath = null;
    }

    async function loadHighlightsForTranscript(path) {
        console.log(`[TranscriptEditorPanel] Attempting to load highlights for: ${path}`);
        if (!path) {
            setImportedTranscriptHighlights([], false);
            return;
        }
        try {
            const projectId = get(project).id;
            if (!projectId) {
                console.error("[TranscriptEditorPanel] Cannot load highlights, project ID is missing.");
                return;
            }
            console.log(`[TranscriptEditorPanel] Invoking 'load_lexical_highlights' with projectId: ${projectId}, documentPath: ${path}`);
            const rawHighlights = await invoke('load_lexical_highlights', {
                args: {
                    projectId: projectId,
                    documentPath: path,
                }
            });
            console.log(`[TranscriptEditorPanel] Received from backend:`, rawHighlights);

            const highlights = rawHighlights ? JSON.parse(rawHighlights) : [];
            console.log(`[TranscriptEditorPanel] Parsed ${highlights.length} highlights. Updating store.`);

            initialHighlights = highlights; // Set the initial highlights
            setImportedTranscriptHighlights(highlights, false); // Update the store

            console.log(`[TranscriptEditorPanel] Store updated with highlights.`);
        } catch (e) {
            console.error("[TranscriptEditorPanel] Error loading lexical highlights for transcript:", e);
            setImportedTranscriptHighlights([], false);
        }
    }

    // --- MODIFIED Load Function ---
    async function loadAndConvertTranscript(filePath) {
        if (!filePath) {
            setLoadedImportedTranscriptData(null, null);
            return;
        }
        editorJsonState = '';
        errorMessage = null;
        isLoading = true;
        if(editorRef) editorRef.resetEditorState('');

        project.update(p => ({...p,
            currentImportedTranscriptPath: filePath,
            isImportedTranscriptLoading: true,
            importedTranscriptError: null,
            currentImportedTranscriptLexicalJson: null,
            initialImportedTranscriptLexicalJson: null,
            isImportedTranscriptDirty: false,
            selectedDocumentPath: null,
            currentDocumentJson: null,
            initialDocumentJson: null,
            isDocumentDirty: false,
            isDocumentLoading: false,
            documentError: null,
            activeDocumentEditorRef: null,
        }));

        try {
            const rawJsonString = await invoke('read_file_content', { path: filePath });
            let lexicalTableJsonToLoad = null;

            if (!rawJsonString || rawJsonString.trim() === '') {
                 // If the file is truly empty, create a basic table structure.
                 console.warn(`[TranscriptEditorPanel] Loaded transcript content is empty for ${filePath}. Creating basic table structure.`);
                 lexicalTableJsonToLoad = await segmentsToLexicalTable([]);
                 if (!isValidLexicalState(lexicalTableJsonToLoad)) {
                    throw new Error("Failed to create a valid empty table structure from empty segments.");
                 }
            } else {
                // Try parsing as JSON first
                let parsedData;
                let isLexicalFormat = false;
                try {
                    parsedData = JSON.parse(rawJsonString);
                    // Check if it looks like a Lexical state (has a root object)
                    if (parsedData && typeof parsedData === 'object' && parsedData.root) {
                        // Further check if it's a valid table state using our helper
                         if (isValidLexicalState(rawJsonString)) {
                             isLexicalFormat = true;
                             console.log(`[TranscriptEditorPanel] Loaded content for ${filePath} is already in valid Lexical Table format.`);
                             lexicalTableJsonToLoad = rawJsonString;
                         } else {
                             console.warn(`[TranscriptEditorPanel] Loaded content for ${filePath} looks like Lexical but failed validation. Attempting segment conversion.`);
                         }
                    }
                } catch (e) {
                    // Ignore parsing error, assume it's the simple segment array
                     console.log(`[TranscriptEditorPanel] Failed to parse loaded content as JSON for ${filePath}, assuming segment array format.`);
                }

                // If it wasn't already a valid Lexical Table format, treat as segment array
                if (!isLexicalFormat) {
                    console.log(`[TranscriptEditorPanel] Attempting conversion from segment array for ${filePath}...`);
                    let segments;
                    try {
                        // Re-parse if the initial parse failed or it wasn't Lexical format
                        if(!parsedData) parsedData = JSON.parse(rawJsonString);
                        if (!Array.isArray(parsedData)) {
                            throw new Error("Loaded data is not a valid JSON array (segments).");
                        }
                        segments = parsedData;
                    } catch (parseError) {
                        throw new Error(`Failed to parse transcript JSON as segments: ${parseError.message}`);
                    }

                    lexicalTableJsonToLoad = await segmentsToLexicalTable(segments);
                    if (!isValidLexicalState(lexicalTableJsonToLoad)) {
                         console.error("[TranscriptEditorPanel] Failed isValidLexicalState check after segment conversion. Generated JSON:", lexicalTableJsonToLoad.substring(0, 500) + "...");
                         throw new Error("Conversion from segments to Lexical table resulted in an invalid state.");
                    }
                    console.log(`[TranscriptEditorPanel] Conversion from segments successful.`);
                }
            }

            // --- Loading the final Lexical JSON into the editor ---
            if (lexicalTableJsonToLoad) {
                editorJsonState = lexicalTableJsonToLoad;
                await tick();
                if (editorRef) {
                    editorRef.resetEditorState(lexicalTableJsonToLoad);
                } else {
                     console.warn("[TranscriptEditorPanel] Editor ref not available immediately after tick in loadAndConvert.");
                }
                setLoadedImportedTranscriptData(filePath, lexicalTableJsonToLoad);
                errorMessage = null;
                console.log(`[TranscriptEditorPanel] Editor state updated successfully for ${filePath}.`);
            } else {
                 throw new Error("Failed to prepare Lexical JSON for loading.");
            }

        } catch (e) {
            console.error(`[TranscriptEditorPanel] Error loading or converting transcript ${filePath}:`, e);
            const loadErrorMsg = `Failed to load/parse transcript: ${e.message}`;
            errorMessage = loadErrorMsg;
            if (editorRef) editorRef.resetEditorState(''); // Reset editor on error
            setImportedTranscriptLoadFailed(filePath, loadErrorMsg);
        } finally {
             // Ensure loading state is always turned off
             project.update(p => p.currentImportedTranscriptPath === filePath ? { ...p, isImportedTranscriptLoading: false } : p);
        }
    }

    // --- isValidLexicalState (Unchanged) ---
    function isValidLexicalState(jsonString) {
        if (!jsonString || typeof jsonString !== 'string') {
            // console.warn("[isValidLexicalState] Input is not a non-empty string.");
            return false;
        }
        try {
            const state = JSON.parse(jsonString);
            if (!state || typeof state !== 'object' || !state.root) {
                 // console.warn("[isValidLexicalState] Parsed JSON is not object or missing root.");
                return false;
            }
            if (!state.root.children || !Array.isArray(state.root.children)) {
                // console.warn("[isValidLexicalState] Root children missing or not an array.");
                return false;
            }
             // Allow empty root (e.g., just created empty table)
             if (state.root.children.length > 0 && state.root.children[0]?.type !== 'table') {
                 console.warn("[isValidLexicalState] Root's first child is not a table node. Actual type:", state.root.children[0]?.type);
                 return false;
             }
            return true;
        } catch (e) {
            // console.warn("[isValidLexicalState] JSON parsing failed:", e);
            return false;
        }
    }

    // --- segmentsToLexicalTable (Only needed for INITIAL load) ---
    async function segmentsToLexicalTable(segments) {
        // (This function remains largely the same as your last correct version for initial conversion)
        const conversionEditor = createHeadlessEditor({
            nodes: ALL_CONVERSION_NODES,
            namespace: `transcript-converter-to-lexical-${Math.random()}`,
            onError: (e) => console.error('[TranscriptEditorPanel] Segments to Lexical conversion editor error:', e)
        });

        let finalJsonString = "";

        try {
            await conversionEditor.update(() => {
                const root = lexicalGetRoot();
                root.clear();
                const tableNode = lexicalCreateTableNode();
                root.append(tableNode);

                const headerRow = lexicalCreateTableRowNode();
                ["#", "Time", "Speaker", "Text"].forEach(headerText => {
                    const cell = lexicalCreateTableCellNode({ headerState: 'column' });
                    cell.append(lexicalCreateParagraphNode().append(lexicalCreateTextNode(headerText)));
                    headerRow.append(cell);
                });
                tableNode.append(headerRow);

                if (segments && segments.length > 0) {
                    segments.forEach((segment, index) => {
                        const dataRow = lexicalCreateTableRowNode();

                        const indexCell = lexicalCreateTableCellNode();
                        indexCell.append(lexicalCreateParagraphNode().append(lexicalCreateTextNode(String(index + 1))));
                        dataRow.append(indexCell);

                        const timeCell = lexicalCreateTableCellNode();
                        const startTimeStr = formatTime(segment.start_time);
                        const endTimeStr = formatTime(segment.end_time);
                        timeCell.append(lexicalCreateParagraphNode().append(lexicalCreateTextNode(`${startTimeStr} - ${endTimeStr}`)));
                        dataRow.append(timeCell);

                        const speakerCell = lexicalCreateTableCellNode();
                        speakerCell.append(lexicalCreateParagraphNode().append(lexicalCreateTextNode(segment.speaker || 'N/A')));
                        dataRow.append(speakerCell);

                        const textCell = lexicalCreateTableCellNode();
                         if (segment.text && typeof segment.text === 'string' && segment.text.trim()) {
                            try {
                                const domParser = new DOMParser();
                                const isHtmlContent = /<[a-z][\s\S]*>/i.test(segment.text);
                                const htmlToParse = isHtmlContent ? segment.text : `<p>${segment.text.replace(/\n/g, '<br/>')}</p>`;

                                const dom = domParser.parseFromString(htmlToParse, 'text/html');

                                if (dom.body && dom.body.innerHTML.trim()) {
                                    const lexicalNodesForCell = lexicalGenerateNodesFromDOM(conversionEditor, dom);

                                    if (lexicalNodesForCell && lexicalNodesForCell.length > 0) {
                                        lexicalNodesForCell.forEach(node => {
                                            if (lexicalIsElementNode(node) && !node.isInline()) {
                                                textCell.append(node);
                                            } else if (node) {
                                                textCell.append(lexicalCreateParagraphNode().append(node));
                                            }
                                        });
                                        if (textCell.isEmpty()) {
                                            textCell.append(lexicalCreateParagraphNode());
                                        }
                                    } else {
                                        textCell.append(lexicalCreateParagraphNode().append(lexicalCreateTextNode(segment.text)));
                                    }
                                } else {
                                     textCell.append(lexicalCreateParagraphNode().append(lexicalCreateTextNode(segment.text)));
                                }
                            } catch (e) {
                                console.warn(`[TranscriptEditorPanel segmentsToLexicalTable] Error parsing HTML/text for segment ${index}:`, e, ". Falling back to plain text.");
                                textCell.append(lexicalCreateParagraphNode().append(lexicalCreateTextNode(segment.text)));
                            }
                        } else {
                            textCell.append(lexicalCreateParagraphNode());
                        }
                        dataRow.append(textCell);
                        tableNode.append(dataRow);
                    });
                }
            });
            finalJsonString = JSON.stringify(conversionEditor.getEditorState().toJSON());
        } catch (error) {
             console.error("[TranscriptEditorPanel] Error during Lexical state generation in segmentsToLexicalTable:", error);
             const errorEditor = createHeadlessEditor({ nodes: [RootNode, ParagraphNode, TextNode]});
             errorEditor.update(() => {
                 lexicalGetRoot().clear().append(
                     lexicalCreateParagraphNode().append(lexicalCreateTextNode("Error converting transcript."))
                 );
             });
             finalJsonString = JSON.stringify(errorEditor.getEditorState().toJSON());
        }

        console.log("[TranscriptEditorPanel segmentsToLexicalTable] Generated JSON (first 500 chars):", finalJsonString.substring(0, 500) + "...");
        return finalJsonString;
    }


    // --- Time Formatting/Parsing (Unchanged) ---
    function formatTime(seconds) {
        if (typeof seconds !== 'number' || isNaN(seconds)) return '00:00:00.000';
        const totalMs = Math.round(seconds * 1000);
        const ms = String(totalMs % 1000).padStart(3, '0');
        const s = String(Math.floor(totalMs / 1000) % 60).padStart(2, '0');
        const m = String(Math.floor(totalMs / (1000 * 60)) % 60).padStart(2, '0');
        const h = String(Math.floor(totalMs / (1000 * 60 * 60))).padStart(2, '0');
        return `${h}:${m}:${s}.${ms}`;
    }
    function parseTimeRange(timeRangeStr) {
        const parts = timeRangeStr.split(/\s*-\s*/);
        if (parts.length !== 2) return { start_time: 0, end_time: 0 };
        return {
            start_time: parseTime(parts[0]),
            end_time: parseTime(parts[1])
        };
    }
    function parseTime(timeStr) {
        const mainParts = timeStr.split('.');
        const hms = mainParts[0].split(':');
        const ms = mainParts.length > 1 ? parseInt(mainParts[1], 10) || 0 : 0;
        if (hms.length !== 3) return 0;
        const h = parseInt(hms[0], 10) || 0;
        const m = parseInt(hms[1], 10) || 0;
        const s = parseInt(hms[2], 10) || 0;
        return h * 3600 + m * 60 + s + ms / 1000;
    }

    // --- REMOVED lexicalTableToSegments function ---
    // It is no longer needed for saving

    function handleHighlightsChange(event) {
        const { highlights } = event.detail;
        setImportedTranscriptHighlights(highlights);
    }

    // --- Editor Change Handler (Unchanged) ---
    function handleEditorChange(event) {
        clearTimeout(changeDebounceTimeout);
        changeDebounceTimeout = setTimeout(() => {
            console.log("[TranscriptEditorPanel handleEditorChange] Debounced event fired.");
            const newLexicalJson = event.detail.jsonString;
            if (editorJsonState !== newLexicalJson) {
                editorJsonState = newLexicalJson;
                if (!isLoading && !errorMessage) {
                    setImportedTranscriptEditorContent(itemPath, newLexicalJson);
                } else {
                     console.warn("[TranscriptEditorPanel handleEditorChange] Skipped store update due to loading/error state.");
                }
            }
        }, 300);
	}

    // --- MODIFIED Save Handler ---
    async function handleSave() {
        if (!itemPath) {
            console.error("[TranscriptEditorPanel] Save Error: No itemPath.");
            await message("Cannot save: No transcript file is active.", { title: "Save Error", type: "error" });
            return;
        }
        if (isLoading || errorMessage) {
             console.error("[TranscriptEditorPanel] Save Error: Cannot save while loading or in error state.");
             await message(`Cannot save: ${isLoading ? 'Transcript is still loading.' : `Transcript failed to load (${errorMessage})`}`, { title: "Save Error", type: "error" });
            return;
        }

        // Use the current Lexical JSON state directly
        const finalJsonToSave = editorJsonState;

        if (!finalJsonToSave || !isValidLexicalState(finalJsonToSave)) {
            console.error("[TranscriptEditorPanel] Save Error: editorJsonState is empty or invalid.");
             await message("Cannot save: Transcript content is empty or invalid.", { title: "Save Error", type: "error" });
            return;
        }

        console.log("[TranscriptEditorPanel] handleSave: Saving full Lexical JSON state for:", itemPath);

        try {
            console.log("[TranscriptEditorPanel] Saving Lexical JSON:", finalJsonToSave.substring(0, 500) + "...");

            project.update(p => ({ ...p, statusMessage: `Saving transcript ${itemPath.split(/[\\/]/).pop()}...`}));

            const highlights_json = (isMetadataDirty && get(project).currentImportedTranscriptHighlights)
                ? JSON.stringify(get(project).currentImportedTranscriptHighlights)
                : null;

            // Use the same backend command used for documents
            await invoke('save_note_json', {
                targetPath: itemPath,
                jsonContent: finalJsonToSave, // Save the full Lexical state
                highlightsJson: highlights_json
            });

            // Update the store's initial state to match the saved state
            markImportedTranscriptAsSaved(itemPath, finalJsonToSave);
            console.log("[TranscriptEditorPanel] Transcript save successful.");

        } catch (error) {
             console.error("[TranscriptEditorPanel] Save failed:", error);
             await message(`Failed to save transcript: ${error.message || error}`, { title: 'Save Error', type: 'error' });
             project.update(p => ({ ...p, statusMessage: `Error saving transcript.`}));
        }
    }

    // --- Discard Handler (Unchanged) ---
    async function handleDiscard() {
        const currentStoreState = get(project);
        const dirtyFlag = currentStoreState.currentImportedTranscriptPath === itemPath && currentStoreState.isImportedTranscriptDirty;

        if (dirtyFlag) {
            const userConfirmed = await confirm('Discard unsaved changes to this transcript?', { type: 'warning', title: 'Discard Changes' });
            if (userConfirmed) {
                markImportedTranscriptChangesDiscarded(itemPath);
                const revertedLexicalJson = get(project).currentImportedTranscriptLexicalJson;
                if(editorRef && revertedLexicalJson != null && isValidLexicalState(revertedLexicalJson)) {
                    editorRef.resetEditorState(revertedLexicalJson);
                    editorJsonState = revertedLexicalJson;
                } else if(editorRef) {
                    console.warn("[TranscriptEditorPanel Discard] Reverted state is invalid or null, resetting editor to empty.");
                    editorRef.resetEditorState('');
                    editorJsonState = '';
                }
                 console.log('[TranscriptEditorPanel] Changes discarded.');
            }
        } else {
            console.log('[TranscriptEditorPanel] Discard skipped: No changes detected in store for this item.');
        }
    }

    // --- Mount/Destroy and Exported Functions (MODIFIED) ---
    onMount(() => {
        console.log('[TranscriptEditorPanel] Mounted. Path:', itemPath);
        setActiveImportedTranscriptEditorRef({ ref: self });
        // The reactive `$: if (itemPath && editorRef)` block now handles all load logic.
    });

	onDestroy(() => {
        console.log('[TranscriptEditorPanel] Destroyed for path:', itemPath);
        clearTimeout(changeDebounceTimeout);
        const activeRef = get(project).activeImportedTranscriptEditorRef;
        if (activeRef && activeRef.getItemPath && activeRef.getItemPath() === itemPath) {
             clearActiveImportedTranscriptEditorRef();
        }
	});

    export function save() {
        console.log('[TranscriptEditorPanel] External save() called for path:', itemPath);
        return handleSave();
    }
    export function discard() {
        console.log('[TranscriptEditorPanel] External discard() called for path:', itemPath);
        return handleDiscard();
    }
    export function resetEditorState(lexicalJsonString) {
        if (editorRef) {
             console.log('[TranscriptEditorPanel] External resetEditorState called for path:', itemPath);
             if (isValidLexicalState(lexicalJsonString)) {
                 editorRef.resetEditorState(lexicalJsonString);
                 editorJsonState = lexicalJsonString || '';
             } else {
                 console.error("[TranscriptEditorPanel resetEditorState] Received invalid JSON string, resetting to empty.");
                 editorRef.resetEditorState('');
                 editorJsonState = '';
             }
        }
    }
    export function getItemPath() {
        return itemPath;
    }

    const self = { save, discard, resetEditorState, getItemPath };

</script>

<!-- Template Section (Unchanged) -->
<div class="prose prose-sm dark:prose-invert max-w-none flex flex-col h-full w-full bg-white dark:bg-gray-800 rounded-md shadow overflow-hidden imported-transcript-editor-panel">
    {#if isLoading && selectedPath === itemPath}
        <div class="flex-grow flex items-center justify-center text-gray-500 dark:text-gray-300">Loading transcript...</div>
    {:else if errorMessage && selectedPath === itemPath}
         <div class="flex-grow flex flex-col items-center justify-center text-red-500 p-4 text-center">
             <p class="font-semibold">Error Loading Transcript</p>
             <p class="text-xs mt-1">{errorMessage}</p>
             <p class="text-xs mt-2">The original file could not be loaded or converted correctly.</p>
         </div>
    {:else if !selectedPath}
         <div class="flex-grow flex items-center justify-center text-gray-500 dark:text-gray-300">No transcript selected or loaded.</div>
    {:else}
        <div class="flex-grow min-h-0 overflow-hidden">
            {#key selectedPath}
                 <LexicalEditor
                     bind:this={editorRef}
                     nodes={LEXICAL_NODES}
                     initialJson={currentLexicalJson}
                     editable={true}
                     placeholder="Transcript content will appear here as a table..."
                     on:change={handleEditorChange}
                     on:highlightschange={handleHighlightsChange}
                     on:highlightssaved={() => highlightsLastUpdated.set(new Date())}
                     toolbarConfig={{
                        undo: true, redo: true, blockType: false,
                        bold: true, italic: true, underline: true, strikethrough: true,
                                link: true, indent: true, outdent: true, align: true, // This line is changed
                        textColor: true, highlight: true, clearFormatting: true,
                        search: true
                     }}
                     enableSearch={true}
                     documentPath={itemPath}
                     initialHighlights={initialHighlights}
                     documentHighlights={$project.currentImportedTranscriptHighlights}
                 />
            {/key}
        </div>
    {/if}
</div>

<!-- Style Section (Modified) -->
<style lang="postcss">
    .imported-transcript-editor-panel :global(.lexical-content) {
        font-family: Arial, Helvetica, sans-serif;
        font-size: 12pt;
        line-height: 1.5;
    }
    .imported-transcript-editor-panel :global(.lexical-wrapper) {
        @apply p-3 m-0;
        flex-grow: 1;
        overflow-y: auto;
    }
    .imported-transcript-editor-panel :global(.lexical-wrapper > .lexical-editor-root > *) {
        @apply mt-0 mb-0;
    }
    .imported-transcript-editor-panel :global(.lexical-content table table) {
        @apply m-0 border-none;
    }
    .imported-transcript-editor-panel :global(.lexical-content table) {
        border-collapse: collapse;
        border-spacing: 0;
        width: 100%;
        border: 1px solid #ccc;
        margin-bottom: 1rem;
        table-layout: fixed;
    }
    .imported-transcript-editor-panel :global(.lexical-content th),
    .imported-transcript-editor-panel :global(.lexical-content td) {
        border: 1px solid #ccc;
        padding: 0.2rem 5.75pt;
        text-align: left;
        vertical-align: top;
        font-family: Arial, Helvetica, sans-serif;
        font-size: 12pt;
        line-height: 1.5;
        word-break: break-word;
    }
    .imported-transcript-editor-panel :global(.lexical-content th) {
        background-color: #f0f0f0;
        font-weight: 600;
    }
    .imported-transcript-editor-panel :global(.lexical-content th p),
    .imported-transcript-editor-panel :global(.lexical-content td p) {
        @apply mt-0 mb-0;
    }


    .imported-transcript-editor-panel :global(.lexical-content table) {
        table-layout: fixed;
        width: 100%;
    }
    .imported-transcript-editor-panel :global(.lexical-content table th:nth-child(1)),
    .imported-transcript-editor-panel :global(.lexical-content table td:nth-child(1)) {
        width: 5%;
    }
    .imported-transcript-editor-panel :global(.lexical-content table th:nth-child(2)),
    .imported-transcript-editor-panel :global(.lexical-content table td:nth-child(2)) {
        width: 15%;
    }
    .imported-transcript-editor-panel :global(.lexical-content table th:nth-child(3)),
    .imported-transcript-editor-panel :global(.lexical-content table td:nth-child(3)) {
        width: 15%;
    }
    .imported-transcript-editor-panel :global(.lexical-content table th:nth-child(4)),
    .imported-transcript-editor-panel :global(.lexical-content table td:nth-child(4)) {
        width: 65%;
    }

    .flex-grow.min-h-0 { min-height: 0; }
</style>