<!-- src/lib/components/projectview/notes/imported_transcripts/TranscriptEditorPanel.svelte -->
<script>
    import { onMount, onDestroy, tick, createEventDispatcher } from 'svelte';
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
    import { activeLayout } from '$lib/stores/layoutStore.js';

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

    const dispatch = createEventDispatcher();

    export let itemPath = null;
    export let isPrimary = true;
    export let enableSegmentPlayback = false;
    export let highlightedRowIndex = -1;

    let editorRef;
    let editorJsonState = ''; // Holds the current Lexical JSON string

    let localCurrentLexicalJson = null;
    let localInitialLexicalJson = null;
    let localIsDirty = false;
    let localIsLoading = true;
    let localErrorMessage = null;
    let localCurrentHighlights = [];
    let localInitialHighlights = [];
    let localIsMetadataDirty = false;

    // --- Derived state from local or store ---
    $: currentLexicalJson = isPrimary ? $project.currentImportedTranscriptLexicalJson : localCurrentLexicalJson;
    $: initialLexicalJson = isPrimary ? $project.initialImportedTranscriptLexicalJson : localInitialLexicalJson;
    $: isDirty = isPrimary ? $project.isImportedTranscriptDirty : localIsDirty;
    $: isLoading = isPrimary ? $project.isImportedTranscriptLoading : localIsLoading;
    $: errorMessage = isPrimary ? $project.importedTranscriptError : localErrorMessage;
    
    $: currentHighlights = isPrimary ? $project.currentImportedTranscriptHighlights : localCurrentHighlights;
    $: isMetadataDirty = isPrimary ? $project.isImportedTranscriptMetadataDirty : localIsMetadataDirty;

    // New state for highlights
    let initialHighlightsFromBackend = [];

    const ALL_CONVERSION_NODES = [
        RootNode, ParagraphNode, TextNode, ExtendedTextNode, LineBreakNode,
        HeadingNode, QuoteNode, ListNode, ListItemNode, LinkNode,
        TableNode, TableRowNode, TableCellNode
    ];

    let changeDebounceTimeout;

    // --- Store Subscription (Updated to only sync if primary and matching path) ---
    $: if (isPrimary && $project.currentImportedTranscriptPath === itemPath) {
        if (editorRef && currentLexicalJson !== editorJsonState) {
            console.log("[TranscriptEditorPanel Store Sync] Updating editorRef state from store.");
            editorRef.resetEditorState(currentLexicalJson);
            editorJsonState = currentLexicalJson;
        }
    }

    // --- Path Change Reaction (REFACTORED) ---
    let prevPath = itemPath;
    $: if (itemPath && itemPath !== prevPath) {
        console.log(`[TranscriptEditorPanel] Path prop changed from ${prevPath} to ${itemPath}. Reloading.`);
        prevPath = itemPath;
        loadAndConvertTranscript(itemPath);
        loadHighlightsForTranscript(itemPath);
    }

    async function loadHighlightsForTranscript(path) {
        console.log(`[TranscriptEditorPanel] Attempting to load highlights for: ${path}`);
        if (!path) {
            if (isPrimary) setImportedTranscriptHighlights([], false);
            else {
                localCurrentHighlights = [];
                localInitialHighlights = [];
                localIsMetadataDirty = false;
            }
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
            console.log(`[TranscriptEditorPanel] Parsed ${highlights.length} highlights. Updating store/local.`);

            if (isPrimary) {
                initialHighlightsFromBackend = highlights;
                setImportedTranscriptHighlights(highlights, false);
            } else {
                localInitialHighlights = JSON.parse(JSON.stringify(highlights));
                localCurrentHighlights = JSON.parse(JSON.stringify(highlights));
                localIsMetadataDirty = false;
            }

            console.log(`[TranscriptEditorPanel] highlights updated.`);
        } catch (e) {
            console.error("[TranscriptEditorPanel] Error loading lexical highlights for transcript:", e);
            if (isPrimary) setImportedTranscriptHighlights([], false);
            else {
                localCurrentHighlights = [];
                localInitialHighlights = [];
                localIsMetadataDirty = false;
            }
        }
    }

    // --- MODIFIED Load Function ---
    async function loadAndConvertTranscript(filePath) {
        if (!filePath) {
            if (isPrimary) setLoadedImportedTranscriptData(null, null);
            else {
                localCurrentLexicalJson = null;
                localInitialLexicalJson = null;
                localIsLoading = false;
            }
            return;
        }
        editorJsonState = '';
        if (isPrimary) {
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
        } else {
            localIsLoading = true;
            localErrorMessage = null;
            localCurrentLexicalJson = null;
            localInitialLexicalJson = null;
            localIsDirty = false;
        }
        
        if(editorRef) editorRef.resetEditorState('');

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
                
                if (isPrimary) {
                    setLoadedImportedTranscriptData(filePath, lexicalTableJsonToLoad);
                } else {
                    localCurrentLexicalJson = lexicalTableJsonToLoad;
                    localInitialLexicalJson = lexicalTableJsonToLoad;
                    localIsLoading = false;
                    localIsDirty = false;
                }
                console.log(`[TranscriptEditorPanel] Editor state updated successfully for ${filePath}.`);
            } else {
                 throw new Error("Failed to prepare Lexical JSON for loading.");
            }

        } catch (e) {
            console.error(`[TranscriptEditorPanel] Error loading or converting transcript ${filePath}:`, e);
            const loadErrorMsg = `Failed to load/parse transcript: ${e.message}`;
            if (editorRef) editorRef.resetEditorState(''); // Reset editor on error
            if (isPrimary) {
                setImportedTranscriptLoadFailed(filePath, loadErrorMsg);
            } else {
                localErrorMessage = loadErrorMsg;
                localIsLoading = false;
            }
        } finally {
             // Ensure loading state is always turned off
             if (isPrimary) {
                project.update(p => p.currentImportedTranscriptPath === filePath ? { ...p, isImportedTranscriptLoading: false } : p);
             } else {
                localIsLoading = false;
             }
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
                        let speakerName = segment.speaker || 'N/A';
                        if (speakerName !== 'N/A' && !speakerName.endsWith(':')) {
                            speakerName += ':';
                        }
                        speakerCell.append(lexicalCreateParagraphNode().append(lexicalCreateTextNode(speakerName)));
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
        if (isPrimary) {
            setImportedTranscriptHighlights(highlights);
        } else {
            localCurrentHighlights = highlights;
            localIsMetadataDirty = JSON.stringify(localInitialHighlights) !== JSON.stringify(highlights);
        }
    }

    // --- Editor Change Handler (Unchanged) ---
    function handleEditorChange(event) {
        clearTimeout(changeDebounceTimeout);
        changeDebounceTimeout = setTimeout(() => {
            console.log("[TranscriptEditorPanel handleEditorChange] Debounced event fired.");
            const newLexicalJson = event.detail.jsonString;
            if (editorJsonState !== newLexicalJson) {
                editorJsonState = newLexicalJson;
                
                const rowCount = getRowCount(newLexicalJson);
                dispatch('rowcountupdated', { rowCount });

                if (!isLoading && !errorMessage) {
                    if (isPrimary) {
                        setImportedTranscriptEditorContent(itemPath, newLexicalJson);
                    } else {
                        localCurrentLexicalJson = newLexicalJson;
                        localIsDirty = localInitialLexicalJson !== newLexicalJson;
                    }
                } else {
                     console.warn("[TranscriptEditorPanel handleEditorChange] Skipped store update due to loading/error state.");
                }
            }
        }, 300);
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

    // --- MODIFIED Save Handler ---
    async function handleSave() {
        if (!itemPath) {
            console.error("[TranscriptEditorPanel] Save Error: No transcript path provided.");
            return;
        }

        if (!isDirty && !isMetadataDirty) {
            console.log("[TranscriptEditorPanel] handleSave: Content and metadata not dirty. Save skipped.");
            return;
        }

        console.log("[TranscriptEditorPanel] handleSave: Attempting to save transcript (and/or metadata) via service:", itemPath);
        try {
            const { saveImportedTranscriptContent } = await import('$lib/services/projectService.js');
            
            // Prepare highlights JSON if not primary
            const hJson = isPrimary ? null : JSON.stringify(currentHighlights);

            await saveImportedTranscriptContent(itemPath, editorJsonState, hJson);
            
            if (!isPrimary) {
                localInitialLexicalJson = editorJsonState;
                localIsDirty = false;
                localInitialHighlights = JSON.parse(JSON.stringify(localCurrentHighlights));
                localIsMetadataDirty = false;
            }
            console.log("[TranscriptEditorPanel] Transcript (and/or metadata) save successful via service.");
        } catch (error) {
            console.error("[TranscriptEditorPanel] Save operation failed:", error);
            throw error;
        }
    }

    // --- Discard Handler (Unchanged) ---
    async function handleDiscard() {
        if (isDirty) {
            const userConfirmed = await confirm('Discard unsaved changes to this transcript?', { type: 'warning', title: 'Discard Changes' });
            if (userConfirmed) {
                if (isPrimary) {
                    markImportedTranscriptChangesDiscarded(itemPath);
                } else {
                    localCurrentLexicalJson = localInitialLexicalJson;
                    localIsDirty = false;
                }
                
                if(editorRef && initialLexicalJson != null && isValidLexicalState(initialLexicalJson)) {
                    editorRef.resetEditorState(initialLexicalJson);
                    editorJsonState = initialLexicalJson;
                    dispatch('rowcountupdated', { rowCount: getRowCount(initialLexicalJson) });
                } else if(editorRef) {
                    console.warn("[TranscriptEditorPanel Discard] Reverted state is invalid or null, resetting editor to empty.");
                    editorRef.resetEditorState('');
                    editorJsonState = '';
                    dispatch('rowcountupdated', { rowCount: 0 });
                }
                 console.log('[TranscriptEditorPanel] Changes discarded.');
            }
        } else {
            console.log('[TranscriptEditorPanel] Discard skipped: No changes detected in store for this item.');
        }
    }

    // --- Mount/Destroy and Exported Functions (MODIFIED) ---
    onMount(() => {
        console.log('[TranscriptEditorPanel] Mounted. Path:', itemPath, 'isPrimary:', isPrimary);
        if (isPrimary) {
            setActiveImportedTranscriptEditorRef({ ref: self });
        }
        if (itemPath) {
            loadAndConvertTranscript(itemPath);
            loadHighlightsForTranscript(itemPath);
        } else {
            isLoading = false;
        }
    });

	onDestroy(() => {
        console.log('[TranscriptEditorPanel] Destroyed for path:', itemPath);
        clearTimeout(changeDebounceTimeout);
        if (isPrimary) {
            const activeRef = get(project).activeImportedTranscriptEditorRef;
            if (activeRef && activeRef.getItemPath && activeRef.getItemPath() === itemPath) {
                 clearActiveImportedTranscriptEditorRef();
            }
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
                 dispatch('rowcountupdated', { rowCount: getRowCount(lexicalJsonString) });
             } else {
                 console.error("[TranscriptEditorPanel resetEditorState] Received invalid JSON string, resetting to empty.");
                 editorRef.resetEditorState('');
                 editorJsonState = '';
                 dispatch('rowcountupdated', { rowCount: 0 });
             }
        }
    }
    export function getItemPath() {
        return itemPath;
    }

    export function getScrollElement() {
        return editorRef?.getScrollElement();
    }

    export function getTopVisibleRowInfo() {
        return editorRef?.getTopVisibleRowInfo() || { index: -1, offset: 0 };
    }

    export function getCursorRowInfo() {
        return editorRef?.getCursorRowInfo() || { index: -1, offset: 0, visible: false };
    }

    export function scrollToRow(index, offset) {
        editorRef?.scrollToRow(index, offset);
    }

    const self = { save, discard, resetEditorState, getItemPath, getScrollElement, getTopVisibleRowInfo, getCursorRowInfo, scrollToRow };

</script>

<!-- Template Section (Unchanged) -->
<div class="flex flex-col h-full w-full bg-white dark:bg-surface-2 overflow-hidden imported-transcript-editor-panel">
    {#if isLoading}
        <div class="flex-grow flex items-center justify-center text-gray-500 dark:text-d-gray-300">Loading transcript...</div>
    {:else if errorMessage}
         <div class="flex-grow flex flex-col items-center justify-center text-red-500 p-4 text-center">
             <p class="font-semibold">Error Loading Transcript</p>
             <p class="text-xs mt-1">{errorMessage}</p>
             <p class="text-xs mt-2">The original file could not be loaded or converted correctly.</p>
         </div>
    {:else if !itemPath}
         <div class="flex-grow flex items-center justify-center text-gray-500 dark:text-d-gray-300">No transcript selected or loaded.</div>
    {:else}
        <div class="flex-grow min-h-0 overflow-hidden">
            {#key itemPath}
                 <LexicalEditor
                     bind:this={editorRef}
                     nodes={LEXICAL_NODES}
                     initialJson={currentLexicalJson}
                     editable={true}
                     placeholder="Transcript content will appear here as a table..."
                     externalHighlightedRowIndex={highlightedRowIndex}
                     on:change={handleEditorChange}
                     on:highlightschange={handleHighlightsChange}
                     on:highlightssaved={() => highlightsLastUpdated.set(new Date())}
                     on:playsegment
                     on:cursorrowchange={(e) => dispatch('cursorrowchange', e.detail)}
                     enableSegmentPlayback={enableSegmentPlayback}
                     toolbarConfig={{
                        undo: true, redo: true, blockType: false,
                        bold: true, italic: true, underline: true, strikethrough: true,
                                link: true, indent: true, outdent: true, align: true,
                        textColor: true, highlight: true, clearFormatting: true,
                        search: true
                     }}
                     enableSearch={true}
                     documentPath={itemPath}
                     initialHighlights={isPrimary ? initialHighlightsFromBackend : localInitialHighlights}
                     documentHighlights={currentHighlights}
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
