<script>
    import { get } from 'svelte/store';
    import { project } from '$lib/stores/projectStore.js';
    import { transcriptStore, updateSegment, updatePlayerTime } from '$lib/stores/transcriptStore.js';
    import { onMount, onDestroy, tick, createEventDispatcher, afterUpdate } from 'svelte';
    import LexicalEditor from '$lib/components/projectview/lexical/LexicalEditor.svelte';
    import { activeLayout } from '$lib/stores/layoutStore.js'; // Added
	import { DOCX_LAYOUT_OPTIONS } from '$lib/constants/exportLayouts.js'; // Added (though not directly used for column widths here, good for reference)


    // --- Lexical Imports --- (Keep as is)
    import { $getRoot as getRoot, $createParagraphNode as createParagraphNode, $createTextNode as createTextNode, $insertNodes as insertNodes, RootNode, ParagraphNode, TextNode, LineBreakNode } from 'lexical';
    import { createHeadlessEditor } from '@lexical/headless';
    import { $generateHtmlFromNodes as generateHtmlFromNodes } from '@lexical/html';
    import { HeadingNode, QuoteNode } from '@lexical/rich-text';
    import { ListNode, ListItemNode } from '@lexical/list';
    import { TableNode, TableRowNode, TableCellNode } from '@lexical/table';
    import { LinkNode } from '@lexical/link';
import { ExtendedTextNode } from '$lib/nodes/ExtendedTextNode.js';


    /* --- Keyboard Shortcut --- */
    function handleSegmentNavShortcut(event) {
        // Ignore if focused in text inputs or contenteditable areas
        const tgt = event.target;
        if (tgt instanceof HTMLElement) {
            const tag = tgt.tagName;
            if (tag === 'INPUT' || tag === 'TEXTAREA' || tgt.isContentEditable) {
                return;
            }
        }
        // Meta+Arrow navigation
        if (event.metaKey && !event.ctrlKey && !event.altKey) {
            if (event.key === 'ArrowUp') {
                event.preventDefault();
                previous();
            } else if (event.key === 'ArrowDown') {
                event.preventDefault();
                next();
            }
        }
    }

    /* --- Component Props & State --- */
    export let panelEditMode = false;
    export let previewEditMode = false;
    let editEnabled = false;
    let segments = [];
    let currentIndex = -1;
    let targetIndexForLoad = -1; // Primarily for tracking if a load was requested externally
    let localStart = '';
    let localEnd = '';
    let localSpeaker = '';
    let showSpeakerDropdown = false;
    let speakerDropdownRef;
    function handleClickOutsideSpeaker(event) { if (showSpeakerDropdown && speakerDropdownRef && !speakerDropdownRef.contains(event.target)) { showSpeakerDropdown = false; } }
    function toggleSpeakerDropdown() { if (editEnabled) showSpeakerDropdown = !showSpeakerDropdown; }
    function selectSpeaker(name) { localSpeaker = name; handleSpeakerChange(); showSpeakerDropdown = false; }
    let lexicalEditorInstance;
    let currentEditorJson = null;
    let initialJsonForEditor = null;
    const dispatch = createEventDispatcher();
    let isMounted = false;
    let isEditorVisible = false;
    $: isEditorVisible = segments.length > 0 && currentIndex >= 0 && currentIndex < segments.length;
    const allNodesForUtilities = [
        RootNode,
        ParagraphNode,
        TextNode,
        LineBreakNode,
        HeadingNode,
        QuoteNode,
        ListNode,
        ListItemNode,
        LinkNode,
        TableNode,
        TableRowNode,
        TableCellNode,
        ExtendedTextNode
    ];
    const defaultEmptyJsonString = JSON.stringify({ root: { children: [{ children: [], direction: null, format: '', indent: 0, type: 'paragraph', version: 1 }], direction: null, format: '', indent: 0, type: 'root', version: 1 } });
    let plainTextConverterEditor = null;
    function getPlainTextConverter() { if (!plainTextConverterEditor) { plainTextConverterEditor = createHeadlessEditor({ namespace: 'PlainTextConverter', nodes: [RootNode, ParagraphNode, TextNode], onError: (e) => console.error("PlainTextConverter Error:", e), }); } return plainTextConverterEditor; }
    function createJsonFromPlainText(text) { const editor = getPlainTextConverter(); let jsonString = defaultEmptyJsonString; const plainText = text || ''; try { editor.update(() => { const root = getRoot(); root.clear(); const p = createParagraphNode(); p.append(createTextNode(plainText)); root.append(p); }, { discrete: true }); const editorState = editor.getEditorState(); if (!editorState.isEmpty()) { jsonString = JSON.stringify(editorState.toJSON()); } else { console.warn("[EditableTranscript] createJsonFromPlainText empty state."); } } catch (e) { console.error("Error creating JSON from plain text:", e); } return jsonString; }
    function cleanupPlainTextConverter() { plainTextConverterEditor = null; }
    function extractPlainText(inputString) { if (!inputString || typeof inputString !== 'string') return ''; if (inputString.trim().startsWith('{') && inputString.trim().endsWith('}')) { try { JSON.parse(inputString); console.warn("[EditableTranscript] extractPlainText received likely JSON, returning empty."); return ''; } catch (e) { /* Ignore */ } } try { const parser = new DOMParser(); const doc = parser.parseFromString(inputString, 'text/html'); return doc.body.textContent || ""; } catch (e) { console.error("[EditableTranscript] Error parsing input string:", e); return inputString; } }
    function formatTimestamp(sec) { if (typeof sec !== 'number' || isNaN(sec) || sec < 0) return '00:00.000'; const totalMs = Math.round(sec * 1000); const ms = String(totalMs % 1000).padStart(3, '0'); const tot = Math.floor(sec); return `${String(Math.floor(tot / 60)).padStart(2, '0')}:${String(tot % 60).padStart(2, '0')}.${ms}`; }
    function parseTimestamp(str) { const parts = str?.match(/^(\d{1,9}):(\d{2})\.(\d{3})$/); if (parts) { const minutes = parseInt(parts[1], 10); const seconds = parseInt(parts[2], 10); const milliseconds = parseInt(parts[3], 10); if (!isNaN(minutes) && !isNaN(seconds) && !isNaN(milliseconds) && seconds < 60 && milliseconds < 1000) return minutes * 60 + seconds + milliseconds / 1000; } const floatVal = parseFloat(str); return isNaN(floatVal) ? null : floatVal; }
    function dispatchEditState() { if (!isMounted) return; if (editEnabled && currentIndex >= 0 && currentIndex < segments.length) { const seg = segments[currentIndex]; const startTime = typeof seg?.start_time === 'number' ? seg.start_time : 0; const endTime = typeof seg?.end_time === 'number' ? seg.end_time : 0; dispatch('segmenteditfocus', { isEditing: true, startTime: startTime, endTime: endTime }); } else { dispatch('segmenteditfocus', { isEditing: false, startTime: 0, endTime: 0 }); } }

    /* --- Render UI --- */
    async function renderSegmentUI(idx) {
        if (!isMounted) return;
        console.log(`[EditableTranscript] renderSegmentUI called with index: ${idx}`);

        if (!segments || segments.length === 0 || idx < 0 || idx >= segments.length) {
            const needsClear = currentIndex !== -1;
            console.log(`[EditableTranscript] Index ${idx} invalid or segments empty. Clearing UI. Needs clear: ${needsClear}`);
            currentIndex = -1; targetIndexForLoad = -1; localStart = ''; localEnd = ''; localSpeaker = ''; initialJsonForEditor = null; currentEditorJson = null;
            if (needsClear) { await tick(); dispatchEditState(); }
            return;
        }

        // Proceed with loading the valid index
        currentIndex = idx;
        const seg = segments[idx];
        targetIndexForLoad = -1; // Reset external load request tracking

        if (!seg || typeof seg.start_time !== 'number' || typeof seg.end_time !== 'number') {
            console.error(`[EditableTranscript] Invalid segment data at index ${idx}:`, seg); localStart = 'Error'; localEnd = 'Error'; localSpeaker = 'Error'; initialJsonForEditor = defaultEmptyJsonString;
        } else {
            localStart = formatTimestamp(seg.start_time);
            localEnd = formatTimestamp(seg.end_time);
            localSpeaker = seg.speaker || 'Unknown';
            let segmentText = seg.text || '';
            let jsonToProcess = segmentText;
            let isValidLexicalJson = false;
            if (segmentText && typeof segmentText === 'string') {
                try {
                    const parsed = JSON.parse(segmentText);
                    if (parsed && parsed.root && parsed.root.type === 'root') {
                        isValidLexicalJson = true;
                        jsonToProcess = segmentText;
                    } else if (parsed && Array.isArray(parsed.children)) {
                        isValidLexicalJson = true;
                        const fullState = {
                            root: {
                                children: [parsed],
                                direction: parsed.direction || 'ltr',
                                format: parsed.format || '',
                                indent: parsed.indent || 0,
                                type: 'root',
                                version: 1
                            }
                        };
                        jsonToProcess = JSON.stringify(fullState);
                    }
                } catch (e) {
                    isValidLexicalJson = false;
                }
            }

            if (isValidLexicalJson) {
                if (!jsonToProcess || jsonToProcess.trim() === '') {
                    jsonToProcess = defaultEmptyJsonString;
                }

                try {
                    const parsedOriginal = JSON.parse(jsonToProcess);
                    // Recursive flatten to remove any nested root wrappers
                    function flattenChildren(nodes) {
                      return nodes.flatMap(n =>
                        n.type === 'root' && Array.isArray(n.children)
                          ? flattenChildren(n.children)
                          : [n]
                      );
                    }

                    // Build finalChildren by fully flattening any nested roots
                    let finalChildren = flattenChildren(parsedOriginal.root.children || []);
                    if (finalChildren.length === 0) {
                      finalChildren.push({
                        type: 'paragraph',
                        version: 1,
                        children: [],
                        direction: null,
                        format: '',
                        indent: 0
                      });
                      console.log("[EditableTranscript] Sanitization resulted in empty children; added default paragraph.");
                    }

                    const sanitizedEditorState = {
                        root: {
                            children: finalChildren,
                            direction: parsedOriginal.root?.direction || 'ltr',
                            format: parsedOriginal.root?.format || '',
                            indent: parsedOriginal.root?.indent || 0,
                            type: 'root',
                            version: 1
                        }
                    };
                    initialJsonForEditor = JSON.stringify(sanitizedEditorState);
                    // Updated log to reflect actual children being passed (could be the default paragraph)
                    console.log(`[EditableTranscript] Sanitized initialJSON for index ${idx}. Preview of final children types: ${finalChildren.map(c => c.type).join(', ')}`);

                } catch (e) {
                    console.error(`[EditableTranscript] Error sanitizing segment text for index ${idx}:`, e, ". Using default empty JSON.");
                    initialJsonForEditor = defaultEmptyJsonString;
                }

            } else { // if not isValidLexicalJson
                const plainTextContent = extractPlainText(segmentText);
                initialJsonForEditor = createJsonFromPlainText(plainTextContent);
            }
            // console.log(`[EditableTranscript] Prepared initial JSON for index ${idx}`); // Original log, can be removed or kept
        }
        currentEditorJson = initialJsonForEditor;
        dispatchEditState(); await tick(); // Ensure UI updates before potential editor focus/interaction
    }


    /* --- Public methods --- */
    export function loadSegment(i) { if (i >= 0 && i < segments.length) { targetIndexForLoad = i; if (i !== currentIndex) { dispatch('navigate', { index: i }); } renderSegmentUI(i); } else { targetIndexForLoad = -1; renderSegmentUI(i); } }
    export function loadSegmentSilent(i) { if (i >= 0 && i < segments.length) { if (i !== currentIndex) { targetIndexForLoad = i; renderSegmentUI(i); } } else { targetIndexForLoad = -1; if (i !== currentIndex) renderSegmentUI(i); } }
    export function updateTimesFromExternal(newStartTime, newEndTime) { if (!editEnabled || currentIndex < 0 || currentIndex >= segments.length) return; let changed = false; const currentSeg = segments[currentIndex]; if (Math.abs(newStartTime - (currentSeg.start_time || 0)) > 0.0001) { localStart = formatTimestamp(newStartTime); updateSegment(currentIndex, { start_time: newStartTime }, true); changed = true; } else { localStart = formatTimestamp(currentSeg.start_time); } if (Math.abs(newEndTime - (currentSeg.end_time || 0)) > 0.0001) { localEnd = formatTimestamp(newEndTime); updateSegment(currentIndex, { end_time: newEndTime }, true); changed = true; } else { localEnd = formatTimestamp(currentSeg.end_time); } if (changed) { tick().then(dispatchEditState); const currentTime = get(transcriptStore).player.currentTime; if (currentTime < newStartTime || currentTime >= newEndTime) { updatePlayerTime(newStartTime); } } }
    export function focusEditor() { /* Lexical focus */ }
    export function forceReloadFromStore() { if (isMounted && currentIndex >= 0 && currentIndex < segments.length) { console.log(`[EditableTranscript] Force reload segment ${currentIndex}.`); renderSegmentUI(currentIndex); } }

    /* --- Lifecycle & Store Subscription --- */
    let unsubscribeTranscriptStore;
    onMount(() => {
        isMounted = true; window.addEventListener('click', handleClickOutsideSpeaker); window.addEventListener('keydown', handleSegmentNavShortcut, true);
        unsubscribeTranscriptStore = transcriptStore.subscribe((ts) => {
            if (!isMounted) return;

            const prevSegmentsLength = segments.length; // Capture length before update
            const newSegs = ts.segments || [];
            const segmentsChanged = JSON.stringify(newSegs) !== JSON.stringify(segments); // Deep compare content
            const lengthChanged = newSegs.length !== prevSegmentsLength; // Check for structural change
            segments = newSegs; // Update local copy
            const currentStoreIndex = ts.player?.currentSegmentIndex ?? -1;

            console.log(`[EditableTranscript Sub] SegChanged: ${segmentsChanged}, LenChanged: ${lengthChanged}, StoreIdx: ${currentStoreIndex}, CurrentIdx: ${currentIndex}, EditEnabled: ${editEnabled}`);

            if (lengthChanged) {
                // --- Structural change (Delete/Insert) ---
                console.log(`[EditableTranscript Sub] Length changed. Target index from store: ${currentStoreIndex}`);
                // Force render using the index set by the store action (delete/insert)
                renderSegmentUI(currentStoreIndex);
            } else if (segmentsChanged) {
                // --- Content change (Undo/Redo/Remap) ---
                console.log(`[EditableTranscript Sub] Content changed (Undo/Redo?). CurrentIdx: ${currentIndex}`);
                // If current index is still valid, re-render to reflect store state
                if (currentIndex >= 0 && currentIndex < segments.length) {
                     renderSegmentUI(currentIndex);
                } else {
                    // If current index became invalid (unlikely if length didn't change, but possible edge case)
                    console.warn(`[EditableTranscript Sub] Content changed, but currentIndex ${currentIndex} invalid. Checking store index ${currentStoreIndex}.`);
                    renderSegmentUI(currentStoreIndex); // Try rendering based on player index
                }
            } else if (!editEnabled && currentStoreIndex !== currentIndex) {
                // --- Player seeking while NOT editing ---
                 console.log(`[EditableTranscript Sub] Player index changed to ${currentStoreIndex} while not editing.`);
                // Update display based on player time if index is valid
                 if (currentStoreIndex >= 0 && currentStoreIndex < segments.length) {
                    loadSegmentSilent(currentStoreIndex);
                 } else if (segments.length > 0 && currentStoreIndex === -1) {
                     // Player moved outside any segment, maybe clear editor? Or keep last? Keep last for now.
                     console.log(`[EditableTranscript Sub] Player index -1, keeping last editor state.`);
                 } else if (segments.length === 0) {
                     renderSegmentUI(-1); // Clear if no segments
                 }
            }
        });
        tick().then(() => {
            if (isMounted) {
                const initialTranscriptState = get(transcriptStore);
                const initialSegments = initialTranscriptState.segments || [];
                let initialIndex = initialTranscriptState.player?.currentSegmentIndex ?? -1;
                if (initialIndex < 0 || initialIndex >= initialSegments.length) {
                    initialIndex = initialSegments.length > 0 ? 0 : -1;
                }
                renderSegmentUI(initialIndex);
            }
        });
    });
    onDestroy(() => {
        isMounted = false;
        window.removeEventListener('click', handleClickOutsideSpeaker);
        window.removeEventListener('keydown', handleSegmentNavShortcut, true);
        unsubscribeTranscriptStore && unsubscribeTranscriptStore();
        cleanupPlainTextConverter();
        try {
            // Check if transcriptStore is still valid before dispatching
            if (get(transcriptStore)) {
                dispatch('segmenteditfocus', { isEditing: false, startTime: 0, endTime: 0 });
            }
        } catch (e) { /* Ignore if store is already destroyed or invalid */ }
    });
    $: { const prevEditEnabled = editEnabled; editEnabled = panelEditMode || previewEditMode; if (isMounted && editEnabled !== prevEditEnabled) { dispatchEditState(); } }

    /* --- Navigation --- */
    function previous() { if (currentIndex > 0) loadSegment(currentIndex - 1); }
    function next() { if (currentIndex < segments.length - 1) loadSegment(currentIndex + 1); }

    /* --- Editor Actions --- */
    function handleBlurTimestamp(field, value) { if (!editEnabled || currentIndex < 0 || currentIndex >= segments.length) return false; const parsedTime = parseTimestamp(value); const currentSeg = segments[currentIndex]; const currentTime = field === 'start_time' ? currentSeg.start_time : currentSeg.end_time; let changed = false; if (parsedTime !== null && Math.abs(parsedTime - currentTime) > 0.0001) { changed = true; updateSegment(currentIndex, { [field]: parsedTime }); tick().then(dispatchEditState); } else if (parsedTime === null) { if (field === 'start_time') localStart = formatTimestamp(currentTime); if (field === 'end_time') localEnd = formatTimestamp(currentTime); } else { if (field === 'start_time') localStart = formatTimestamp(currentTime); if (field === 'end_time') localEnd = formatTimestamp(currentTime); } return changed; }
    function handleSpeakerChange() { if (editEnabled && currentIndex >= 0 && currentIndex < segments.length) { const currentSpeaker = segments[currentIndex].speaker || 'Unknown'; if (localSpeaker !== currentSpeaker) { updateSegment(currentIndex, { speaker: localSpeaker }); return true; } } return false; }
    function handleEditorUpdate(event) { const newJson = event.detail.jsonString; currentEditorJson = newJson; }
    export function commitCurrentSegmentEdits() {
        if (!editEnabled || currentIndex < 0 || currentIndex >= segments.length) {
            console.warn("Commit skipped: Not ready.");
            return false;
        }
        if (!currentEditorJson) {
            console.warn("Commit skipped: No editor JSON.");
            return false;
        }
        // --- normalize and sanitize JSON: flatten nested root wrappers ---
        let jsonStringRaw =
            typeof currentEditorJson === 'string'
                ? currentEditorJson
                : JSON.stringify(currentEditorJson);
        let jsonString;
        try {
          const obj = JSON.parse(jsonStringRaw);
          if (obj && obj.root && Array.isArray(obj.root.children)) {
            function flattenChildren(nodes) {
              return nodes.flatMap(n =>
                n.type === 'root' && Array.isArray(n.children)
                  ? flattenChildren(n.children)
                  : [n]
              );
            }
            let finalChildren = flattenChildren(obj.root.children);
            if (finalChildren.length === 0) {
              finalChildren.push({
                type: 'paragraph',
                version: 1,
                children: [],
                direction: null,
                format: '',
                indent: 0
              });
            }
            obj.root.children = finalChildren;
            jsonString = JSON.stringify(obj);
          } else {
            jsonString = jsonStringRaw;
          }
        } catch (e) {
          console.error("[EditableTranscript] Error sanitizing JSON on save:", e);
          jsonString = jsonStringRaw;
        }

        console.log('[DEBUG save]', {
            idx: currentIndex,
            first120: jsonString.slice(0, 120),
            typeof: typeof jsonString
        });

        // validate it contains a Lexical root
        try {
            const parsed = JSON.parse(jsonString);
            if (!parsed || !parsed.root) {
                throw new Error("Invalid JSON structure (missing root)");
            }
        } catch (e) {
            console.warn("Commit skipped: currentEditorJson invalid.", e);
            console.error(
                "Invalid JSON:",
                jsonString ? jsonString.substring(0, 200) + "..." : "null"
            );
            return false;
        }

        const segmentInStore = get(transcriptStore).segments[currentIndex];
        let textChanged = false;
        let timeStartChanged = false;
        let timeEndChanged = false;
        let speakerChanged = false;
        if (segmentInStore && segmentInStore.text !== jsonString) {
            updateSegment(currentIndex, { text: jsonString });
            textChanged = true;
        }
        timeStartChanged = handleBlurTimestamp('start_time', localStart);
        timeEndChanged = handleBlurTimestamp('end_time', localEnd);
        speakerChanged = handleSpeakerChange();
        const overallChange = textChanged || timeStartChanged || timeEndChanged || speakerChanged;
        if (overallChange) {
            console.log("[EditableTranscript] Committed changes segment", currentIndex);
        }
        return overallChange;
    }
    function handleEditSaveClick() { if (editEnabled) { console.log("[EditableTranscript] Save clicked, dispatching 'save'."); dispatch('save'); } else { console.log("[EditableTranscript] Edit clicked, dispatching 'toggleedit'."); dispatch('toggleedit'); } }
    const EDIT_ICON = `<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-6"> <path stroke-linecap="round" stroke-linejoin="round" d="m16.862 4.487 1.687-1.688a1.875 1.875 0 1 1 2.652 2.652L10.582 16.07a4.5 4.5 0 0 1-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 0 1 1.13-1.897l8.932-8.931Zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0 1 15.75 21H5.25A2.25 2.25 0 0 1 3 18.75V8.25A2.25 2.25 0 0 1 5.25 6H10" /> </svg>`;
    const SAVE_ICON = `<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-6"> <path stroke-linecap="round" stroke-linejoin="round" d="M10.125 2.25h-4.5c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125v-9M10.125 2.25h.375a9 9 0 0 1 9 9v.375M10.125 2.25A3.375 3.375 0 0 1 13.5 5.625v1.5c0 .621.504 1.125 1.125 1.125h1.5a3.375 3.375 0 0 1 3.375 3.375M9 15l2.25 2.25L15 12" /> </svg>`;

    // --- Layout specific styles ---
    let columnContainerClass = 'flex flex-col flex-grow mx-auto gap-y-2 mt-4 min-h-0';
    let segmentNumberContainerStyle = 'flex-basis: 1.880rem;'; // Default from original
    let timestampContainerStyle = ''; // Default, will be part of flex
    let speakerContainerStyle = 'flex-basis: 6.5rem; max-width: 6.5rem;'; // Default from original
    let textEditorContainerStyle = 'flex-grow'; // Default
    let isLayout1Active = false; // Flag for Layout1 specific structure

    $: {
        const layoutKey = $activeLayout;
        isLayout1Active = layoutKey === 'Layout1';

        // Default styles for layouts other than Layout1
        columnContainerClass = 'flex flex-col flex-grow mx-auto gap-y-2 mt-4 min-h-0';
        segmentNumberContainerStyle = 'flex-basis: 1.880rem;';
        timestampContainerStyle = '';
        speakerContainerStyle = 'flex-basis: 6.5rem; max-width: 6.5rem;';
        textEditorContainerStyle = 'flex-grow';

        if (layoutKey === 'Layout1') {
            columnContainerClass = 'flex flex-row items-start gap-x-1 flex-grow min-h-0 w-full mt-4';
            segmentNumberContainerStyle = 'flex-shrink-0 text-left py-1 text-sm text-gray-500 dark:text-gray-400';
        } else if (layoutKey === 'Layout2') {
            speakerContainerStyle = 'flex-basis: 6.5rem; max-width: 6.5rem;';
        } else if (layoutKey === 'Layout3') {
            segmentNumberContainerStyle = 'flex-shrink-0 text-left';
            timestampContainerStyle = 'flex-grow';
            speakerContainerStyle = 'flex-shrink-0 ml-2 min-w-[6.5rem]'; // Ensure it has a min-width
            textEditorContainerStyle = 'w-full';
        } else if (layoutKey === 'Layout4') {
            speakerContainerStyle = 'flex-basis: 6rem; max-width: 6rem;';
        } else if (layoutKey === 'Layout5') {
            // For Layout 5, speaker should take more available space
            segmentNumberContainerStyle = 'flex-shrink-0 text-left';
            timestampContainerStyle = 'flex-shrink-0'; // Timestamps don't grow
            speakerContainerStyle = 'relative flex-grow min-w-[6.5rem]'; // Speaker takes up remaining space
            textEditorContainerStyle = 'w-full';
        }
    }

</script>

<div class="p-2 h-full flex flex-col text-gray-900 dark:text-gray-200 text-sm bg-white dark:bg-gray-800 rounded-md shadow-sm overflow-hidden editable-transcript-controls">
    {#if !isEditorVisible}
        {#if segments.length === 0} <div class="flex-grow flex items-center justify-center text-gray-500 dark:text-gray-400 p-4"> No transcript loaded or transcript is empty. </div>
        {:else} <div class="flex-grow flex items-center justify-center text-gray-500 dark:text-gray-400 p-4"> Select a segment to start editing. </div> {/if}
    {:else}
        <div class="flex flex-col flex-grow min-h-0 h-full">
            <div class="relative py-1 flex-shrink-0 mb-4">
                <button on:click="{handleEditSaveClick}" class='btn-icon absolute left-0 top-1 text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200' title="{editEnabled ? 'Save Transcript (Ctrl+S)' : 'Edit Segment (Ctrl+E)'}" aria-label="{editEnabled ? 'Save Transcript' : 'Edit Segment'}">
                    {@html editEnabled ? SAVE_ICON : EDIT_ICON}
                </button>
                <button on:click="{previous}" class="btn-nav-vertical absolute left-1/2 top-1 transform -translate-x-1/2" disabled="{currentIndex <= 0}" aria-label="Previous Segment" >
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="size-5"> <path stroke-linecap="round" stroke-linejoin="round" d="m4.5 15.75 7.5-7.5 7.5 7.5" /> </svg>
                </button>
            </div>
            <!-- Main content area for inputs, layout driven by $activeLayout -->
            <div class="{columnContainerClass}" style="width: 40.01rem; font-family: Arial, Helvetica, sans-serif; font-size: 12pt; line-height: 1.5;">

                {#if isLayout1Active}
                    <!-- Layout 1: Single Row Table -->
                    <div class="flex flex-row items-start gap-x-1 flex-grow min-h-0 w-full">
                        <div class='flex-shrink-0 text-left py-1 basis-[5%] max-w-[5%] pr-1 {segmentNumberContainerStyle.includes("text-gray-500") ? "text-gray-500 dark:text-gray-400" : ""}'>
                            <span class='w-full truncate whitespace-normal break-words text-sm' title="{String(currentIndex + 1)}">{String(currentIndex + 1)}</span>
                        </div>
                        <div class='flex-shrink-0 basis-[15%] max-w-[15%] pr-1 text-gray-600 dark:text-gray-400 text-left leading-tight flex flex-col items-stretch gap-y-0.5 py-0.5'>
                            <input id='startTimeInput_L1' class='input-field w-full text-sm p-0' type='text' bind:value="{localStart}" disabled="{!editEnabled}" on:blur="{() => handleBlurTimestamp('start_time', localStart)}" on:keydown="{(e) => { if (e.key === 'Enter') e.target.blur(); }}" aria-label='Segment start time' placeholder='00:00.000' />
                            <input id='endTimeInput_L1' class='input-field w-full text-sm p-0' type='text' bind:value="{localEnd}" disabled="{!editEnabled}" on:blur="{() => handleBlurTimestamp('end_time', localEnd)}" on:keydown="{(e) => { if (e.key === 'Enter') e.target.blur(); }}" aria-label='Segment end time' placeholder='00:00.000' />
                        </div>
                        <div class='relative flex-shrink-0 basis-[15%] max-w-[15%] pr-1 py-0.5' bind:this="{speakerDropdownRef}">
                            <button type='button' class='input-field w-full truncate whitespace-nowrap font-semibold flex items-center justify-between' on:click="{toggleSpeakerDropdown}" title="{localSpeaker}" disabled="{!editEnabled}" aria-label='Select Speaker'>
                                <span class='truncate'>{localSpeaker}</span>
                                <svg xmlns='http://www.w3.org/2000/svg' class='w-4 h-4 ml-1 flex-shrink-0' fill='none' viewBox='0 0 24 24' stroke='currentColor' stroke-width='2'><path stroke-linecap='round' stroke-linejoin='round' d='M19 9l-7 7-7-7' /></svg>
                            </button>
                            {#if showSpeakerDropdown}
                            <ul class='absolute z-10 mt-1 w-full max-h-40 overflow-auto bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded shadow-lg'>
                                <li class='px-2 py-1 truncate hover:bg-gray-100 dark:hover:bg-gray-600 cursor-pointer' on:click="{() => selectSpeaker('Unknown')}" title='Unknown'>Unknown</li>
                                {#each $transcriptStore.speakers.names as name (name)}<li class='px-2 py-1 truncate hover:bg-gray-100 dark:hover:bg-gray-600 cursor-pointer' on:click="{() => selectSpeaker(name)}" title="{name}">{name.length > 12 ? name.slice(0, 12) + '...' : name}</li>{/each}
                            </ul>
                            {/if}
                        </div>
                        <div class='lexical-editor-wrapper-style basis-[65%] max-w-[65%]' class:is-disabled="{!editEnabled}">
                            {#key currentIndex} {#if currentIndex !== -1 && initialJsonForEditor} <LexicalEditor bind:this="{lexicalEditorInstance}" initialJson="{initialJsonForEditor}" editable="{editEnabled}" placeholder='Enter transcript text…' toolbarConfig="{{ undo: true, redo: true, bold: true, italic: true, underline: true, strikethrough: true, textColor: true, highlight: true, clearFormatting: true }}" on:change="{handleEditorUpdate}" /> {:else} <div class='p-2 text-gray-400 italic text-center flex-grow flex items-center justify-center'>Loading editor...</div> {/if} {/key}
                        </div>
                    </div>
                {:else if $activeLayout === 'Layout2'}
                    <!-- Layout 2: Original Two Row Structure -->
                    <div class="flex items-center gap-x-1 flex-shrink-0">
                        <div class='flex-shrink-0 text-left' style="{segmentNumberContainerStyle}">
                            <span class='w-full truncate whitespace-normal break-words text-sm text-gray-500 px-1.5 py-1' title="{String(currentIndex + 1)}">{String(currentIndex + 1)}</span>
                        </div>
                        <div class='flex-shrink-0 text-gray-600 dark:text-gray-400 text-left leading-tight flex items-center gap-x-1' style="{timestampContainerStyle}">
                            <input id='startTimeInput_L2' class='input-field w-[5.641rem] text-sm p-0' type='text' bind:value="{localStart}" disabled="{!editEnabled}" on:blur="{() => handleBlurTimestamp('start_time', localStart)}" on:keydown="{(e) => { if (e.key === 'Enter') e.target.blur(); }}" aria-label='Segment start time' placeholder='00:00.000' />
                            <span class='text-gray-400 dark:text-gray-500'>–</span>
                            <input id='endTimeInput_L2' class='input-field w-[5.641rem] text-sm p-0' type='text' bind:value="{localEnd}" disabled="{!editEnabled}" on:blur="{() => handleBlurTimestamp('end_time', localEnd)}" on:keydown="{(e) => { if (e.key === 'Enter') e.target.blur(); }}" aria-label='Segment end time' placeholder='00:00.000' />
                        </div>
                    </div>
                    <div class="flex items-start gap-x-1 flex-grow min-h-0">
                        <div class='relative flex-shrink-0' style="{speakerContainerStyle}" bind:this="{speakerDropdownRef}">
                            <button type='button' class='input-field w-full truncate whitespace-nowrap font-semibold flex items-center justify-between' on:click="{toggleSpeakerDropdown}" title="{localSpeaker}" disabled="{!editEnabled}" aria-label='Select Speaker'>
                                <span class='truncate'>{localSpeaker}</span>
                                <svg xmlns='http://www.w3.org/2000/svg' class='w-4 h-4 ml-1' fill='none' viewBox='0 0 24 24' stroke='currentColor' stroke-width='2'><path stroke-linecap='round' stroke-linejoin='round' d='M19 9l-7 7-7-7' /></svg>
                            </button>
                            {#if showSpeakerDropdown}
                            <ul class='absolute z-10 mt-1 w-full max-h-40 overflow-auto bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded shadow-lg'>
                                <li class='px-2 py-1 truncate hover:bg-gray-100 dark:hover:bg-gray-600 cursor-pointer' on:click="{() => selectSpeaker('Unknown')}" title='Unknown'>Unknown</li>
                                {#each $transcriptStore.speakers.names as name (name)}<li class='px-2 py-1 truncate hover:bg-gray-100 dark:hover:bg-gray-600 cursor-pointer' on:click="{() => selectSpeaker(name)}" title="{name}">{name.length > 12 ? name.slice(0, 12) + '...' : name}</li>{/each}
                            </ul>
                            {/if}
                        </div>
                        <div class='lexical-editor-wrapper-style {textEditorContainerStyle}' class:is-disabled="{!editEnabled}">
                            {#key currentIndex} {#if currentIndex !== -1 && initialJsonForEditor} <LexicalEditor bind:this="{lexicalEditorInstance}" initialJson="{initialJsonForEditor}" editable="{editEnabled}" placeholder='Enter transcript text…' toolbarConfig="{{ undo: true, redo: true, bold: true, italic: true, underline: true, strikethrough: true, textColor: true, highlight: true, clearFormatting: true }}" on:change="{handleEditorUpdate}" /> {:else} <div class='p-2 text-gray-400 italic text-center flex-grow flex items-center justify-center'>Loading editor...</div> {/if} {/key}
                        </div>
                    </div>
                {:else if $activeLayout === 'Layout3'}
                    <!-- Layout 3: Num, Time, Speaker on first row; Text on second -->
                    <div class="flex items-center gap-x-1 flex-shrink-0">
                         <div class='flex-shrink-0 text-left py-1' style="{segmentNumberContainerStyle}">
                            <span class='w-full truncate whitespace-normal break-words text-sm text-gray-500 px-1.5' title="{String(currentIndex + 1)}">{String(currentIndex + 1)}</span>
                        </div>
                        <div class='flex-shrink-0 text-gray-600 dark:text-gray-400 text-left leading-tight flex items-center gap-x-1' style="{timestampContainerStyle}">
                            <input id='startTimeInput_L3' class='input-field w-[5.641rem] text-sm p-0' type='text' bind:value="{localStart}" disabled="{!editEnabled}" on:blur="{() => handleBlurTimestamp('start_time', localStart)}" on:keydown="{(e) => { if (e.key === 'Enter') e.target.blur(); }}" aria-label='Segment start time' placeholder='00:00.000' />
                            <span class='text-gray-400 dark:text-gray-500'>–</span>
                            <input id='endTimeInput_L3' class='input-field w-[5.641rem] text-sm p-0' type='text' bind:value="{localEnd}" disabled="{!editEnabled}" on:blur="{() => handleBlurTimestamp('end_time', localEnd)}" on:keydown="{(e) => { if (e.key === 'Enter') e.target.blur(); }}" aria-label='Segment end time' placeholder='00:00.000' />
                        </div>
                        <div class='relative {speakerContainerStyle}' bind:this="{speakerDropdownRef}">
                            <button type='button' class='input-field w-full truncate whitespace-nowrap font-semibold flex items-center justify-between' on:click="{toggleSpeakerDropdown}" title="{localSpeaker}" disabled="{!editEnabled}" aria-label='Select Speaker'>
                                <span class='truncate'>{localSpeaker}</span>
                                <svg xmlns='http://www.w3.org/2000/svg' class='w-4 h-4 ml-1 flex-shrink-0' fill='none' viewBox='0 0 24 24' stroke='currentColor' stroke-width='2'><path stroke-linecap='round' stroke-linejoin='round' d='M19 9l-7 7-7-7' /></svg>
                            </button>
                            {#if showSpeakerDropdown}
                            <ul class='absolute z-10 mt-1 w-full max-h-40 overflow-auto bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded shadow-lg'>
                                <li class='px-2 py-1 truncate hover:bg-gray-100 dark:hover:bg-gray-600 cursor-pointer' on:click="{() => selectSpeaker('Unknown')}" title='Unknown'>Unknown</li>
                                {#each $transcriptStore.speakers.names as name (name)}<li class='px-2 py-1 truncate hover:bg-gray-100 dark:hover:bg-gray-600 cursor-pointer' on:click="{() => selectSpeaker(name)}" title="{name}">{name.length > 12 ? name.slice(0, 12) + '...' : name}</li>{/each}
                            </ul>
                            {/if}
                        </div>
                    </div>
                    <div class="flex items-start gap-x-1 flex-grow min-h-0 w-full">
                        <div class='lexical-editor-wrapper-style w-full {textEditorContainerStyle}' class:is-disabled="{!editEnabled}">
                             {#key currentIndex} {#if currentIndex !== -1 && initialJsonForEditor} <LexicalEditor bind:this="{lexicalEditorInstance}" initialJson="{initialJsonForEditor}" editable="{editEnabled}" placeholder='Enter transcript text…' toolbarConfig="{{ undo: true, redo: true, bold: true, italic: true, underline: true, strikethrough: true, textColor: true, highlight: true, clearFormatting: true }}" on:change="{handleEditorUpdate}" /> {:else} <div class='p-2 text-gray-400 italic text-center flex-grow flex items-center justify-center'>Loading editor...</div> {/if} {/key}
                        </div>
                    </div>

                {:else if $activeLayout === 'Layout4'}
                     <div class="flex items-center gap-x-1 flex-shrink-0">
                        <div class='flex-shrink-0 text-left' style="{segmentNumberContainerStyle}">
                            <span class='w-full truncate whitespace-normal break-words text-sm text-gray-500 px-1.5 py-1' title="{String(currentIndex + 1)}">{String(currentIndex + 1)}</span>
                        </div>
                        <div class='flex-shrink-0 text-gray-600 dark:text-gray-400 text-left leading-tight flex items-center gap-x-1' style="{timestampContainerStyle}">
                            <input id='startTimeInput_L4' class='input-field w-[5.641rem] text-sm p-0' type='text' bind:value="{localStart}" disabled="{!editEnabled}" on:blur="{() => handleBlurTimestamp('start_time', localStart)}" on:keydown="{(e) => { if (e.key === 'Enter') e.target.blur(); }}" aria-label='Segment start time' placeholder='00:00.000' />
                            <span class='text-gray-400 dark:text-gray-500'>–</span>
                            <input id='endTimeInput_L4' class='input-field w-[5.641rem] text-sm p-0' type='text' bind:value="{localEnd}" disabled="{!editEnabled}" on:blur="{() => handleBlurTimestamp('end_time', localEnd)}" on:keydown="{(e) => { if (e.key === 'Enter') e.target.blur(); }}" aria-label='Segment end time' placeholder='00:00.000' />
                        </div>
                    </div>
                    <div class="flex items-start gap-x-1 flex-grow min-h-0">
                        <div class='relative flex-shrink-0' style="{speakerContainerStyle}" bind:this="{speakerDropdownRef}">
                             <button type='button' class='input-field w-full truncate whitespace-nowrap font-semibold flex items-center justify-between' on:click="{toggleSpeakerDropdown}" title="{localSpeaker}" disabled="{!editEnabled}" aria-label='Select Speaker'>
                                <span class='truncate'>{localSpeaker}</span>
                                <svg xmlns='http://www.w3.org/2000/svg' class='w-4 h-4 ml-1' fill='none' viewBox='0 0 24 24' stroke='currentColor' stroke-width='2'><path stroke-linecap='round' stroke-linejoin='round' d='M19 9l-7 7-7-7' /></svg>
                            </button>
                            {#if showSpeakerDropdown}
                            <ul class='absolute z-10 mt-1 w-full max-h-40 overflow-auto bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded shadow-lg'>
                                <li class='px-2 py-1 truncate hover:bg-gray-100 dark:hover:bg-gray-600 cursor-pointer' on:click="{() => selectSpeaker('Unknown')}" title='Unknown'>Unknown</li>
                                {#each $transcriptStore.speakers.names as name (name)}<li class='px-2 py-1 truncate hover:bg-gray-100 dark:hover:bg-gray-600 cursor-pointer' on:click="{() => selectSpeaker(name)}" title="{name}">{name.length > 12 ? name.slice(0, 12) + '...' : name}</li>{/each}
                            </ul>
                            {/if}
                        </div>
                        <div class='lexical-editor-wrapper-style {textEditorContainerStyle}' class:is-disabled="{!editEnabled}">
                             {#key currentIndex} {#if currentIndex !== -1 && initialJsonForEditor} <LexicalEditor bind:this="{lexicalEditorInstance}" initialJson="{initialJsonForEditor}" editable="{editEnabled}" placeholder='Enter transcript text…' toolbarConfig="{{ undo: true, redo: true, bold: true, italic: true, underline: true, strikethrough: true, textColor: true, highlight: true, clearFormatting: true }}" on:change="{handleEditorUpdate}" /> {:else} <div class='p-2 text-gray-400 italic text-center flex-grow flex items-center justify-center'>Loading editor...</div> {/if} {/key}
                        </div>
                    </div>

                {:else if $activeLayout === 'Layout5'}
                    <div class="flex items-center gap-x-1 flex-shrink-0">
                        <div class='flex-shrink-0 text-left' style="{segmentNumberContainerStyle}">
                            <span class='w-full truncate whitespace-normal break-words text-sm text-gray-500 px-1.5 py-1' title="{String(currentIndex + 1)}">{String(currentIndex + 1)}</span>
                        </div>
                        <div class='flex-shrink-0 text-gray-600 dark:text-gray-400 text-left leading-tight flex items-center gap-x-1' style="{timestampContainerStyle}">
                            <input id='startTimeInput_L5' class='input-field w-[5.641rem] text-sm p-0' type='text' bind:value="{localStart}" disabled="{!editEnabled}" on:blur="{() => handleBlurTimestamp('start_time', localStart)}" on:keydown="{(e) => { if (e.key === 'Enter') e.target.blur(); }}" aria-label='Segment start time' placeholder='00:00.000' />
                            <span class='text-gray-400 dark:text-gray-500'>–</span>
                            <input id='endTimeInput_L5' class='input-field w-[5.641rem] text-sm p-0' type='text' bind:value="{localEnd}" disabled="{!editEnabled}" on:blur="{() => handleBlurTimestamp('end_time', localEnd)}" on:keydown="{(e) => { if (e.key === 'Enter') e.target.blur(); }}" aria-label='Segment end time' placeholder='00:00.000' />
                        </div>
                        <div class='relative flex-shrink-0' style="{speakerContainerStyle}" bind:this="{speakerDropdownRef}">
                             <button type='button' class='input-field w-full truncate whitespace-nowrap font-semibold flex items-center justify-between' on:click="{toggleSpeakerDropdown}" title="{localSpeaker}" disabled="{!editEnabled}" aria-label='Select Speaker'>
                                <span class='truncate'>{localSpeaker.length > 13 ? localSpeaker.slice(0, 13) + '...' : localSpeaker}</span>
                                <svg xmlns='http://www.w3.org/2000/svg' class='w-4 h-4 ml-1' fill='none' viewBox='0 0 24 24' stroke='currentColor' stroke-width='2'><path stroke-linecap='round' stroke-linejoin='round' d='M19 9l-7 7-7-7' /></svg>
                            </button>
                            {#if showSpeakerDropdown}
                            <ul class='absolute z-10 mt-1 w-full max-h-40 overflow-auto bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded shadow-lg'>
                                <li class='px-2 py-1 truncate hover:bg-gray-100 dark:hover:bg-gray-600 cursor-pointer' on:click="{() => selectSpeaker('Unknown')}" title='Unknown'>Unknown</li>
                                {#each $transcriptStore.speakers.names as name (name)}<li class='px-2 py-1 truncate hover:bg-gray-100 dark:hover:bg-gray-600 cursor-pointer' on:click="{() => selectSpeaker(name)}" title="{name}">{name.length > 12 ? name.slice(0, 12) + '...' : name}</li>{/each}
                            </ul>
                            {/if}
                        </div>
                    </div>
                     <div class="flex items-start gap-x-1 flex-grow min-h-0 w-full">
                        <div class='lexical-editor-wrapper-style w-full {textEditorContainerStyle}' class:is-disabled="{!editEnabled}">
                             {#key currentIndex} {#if currentIndex !== -1 && initialJsonForEditor} <LexicalEditor bind:this="{lexicalEditorInstance}" initialJson="{initialJsonForEditor}" editable="{editEnabled}" placeholder='Enter transcript text…' toolbarConfig="{{ undo: true, redo: true, bold: true, italic: true, underline: true, strikethrough: true, textColor: true, highlight: true, clearFormatting: true }}" on:change="{handleEditorUpdate}" /> {:else} <div class='p-2 text-gray-400 italic text-center flex-grow flex items-center justify-center'>Loading editor...</div> {/if} {/key}
                        </div>
                    </div>
                {/if}
            </div>
            <div class="flex justify-center py-1 flex-shrink-0 mt-auto"> <button on:click="{next}" class="btn-nav-vertical" disabled="{currentIndex >= segments.length - 1}" aria-label="Next Segment" > <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="size-5"> <path stroke-linecap="round" stroke-linejoin="round" d="m19.5 8.25-7.5 7.5-7.5-7.5" /> </svg> </button> </div>
        </div>
    {/if}
</div>

<style lang="postcss">
    /* Styles remain unchanged */
    .input-field { @apply box-border px-1.5 py-1 border border-gray-300 rounded bg-white text-gray-900 dark:bg-gray-700 dark:text-gray-200 dark:border-gray-600 appearance-none transition duration-150 ease-in-out focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500 disabled:bg-gray-100 disabled:text-gray-500 disabled:cursor-not-allowed disabled:border-gray-300 dark:disabled:bg-gray-600 dark:disabled:text-gray-400 dark:disabled:border-gray-500; }
    .select-field { @apply border border-gray-300 rounded bg-white text-gray-900 text-sm dark:bg-gray-700 dark:text-gray-200 dark:border-gray-600 transition duration-150 ease-in-out focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500 disabled:bg-gray-100 disabled:text-gray-500 disabled:cursor-not-allowed disabled:border-gray-300 dark:disabled:bg-gray-600 dark:disabled:text-gray-400 dark:disabled:border-gray-500; }
    .size-6 { @apply w-6 h-6; } .size-5 { @apply w-5 h-5; }
    .btn-icon { @apply p-1 rounded hover:bg-gray-200 dark:hover:bg-gray-600 focus:outline-none focus:ring-1 focus:ring-offset-1 focus:ring-blue-400 dark:focus:ring-blue-500 dark:ring-offset-gray-800 focus:bg-gray-200 dark:focus:bg-gray-600 transition duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-transparent dark:disabled:hover:bg-transparent; }
    .btn-nav-vertical { @apply p-1 bg-gray-100 hover:bg-gray-200 text-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600 dark:text-gray-300 rounded-md disabled:opacity-50 disabled:cursor-not-allowed focus:outline-none focus:ring-1 focus:ring-offset-1 focus:ring-blue-400 dark:focus:ring-blue-500 dark:ring-offset-gray-800 focus:bg-gray-200 dark:focus:bg-gray-600 transition-colors flex items-center justify-center; }
    .lexical-editor-wrapper-style { display: flex; flex-direction: column; @apply border border-gray-300 dark:border-gray-600 rounded overflow-hidden; }
    .lexical-editor-wrapper-style > :global(.lexical-editor-root) { flex-grow: 1; min-height: 0; border: none !important; border-radius: 0 !important; box-shadow: none !important; overflow: hidden; }
    .lexical-editor-wrapper-style > :global(.lexical-editor-root > .lexical-wrapper) { overflow-y: auto; height: 100%; padding: 0px;}
    .lexical-editor-wrapper-style.is-disabled { @apply bg-gray-100 border-gray-300 opacity-70 dark:bg-gray-600 dark:border-gray-500 dark:opacity-70; pointer-events: none; }
    .lexical-editor-wrapper-style.is-disabled > :global(.lexical-editor-root > .lexical-wrapper > .lexical-content) { @apply cursor-not-allowed; }
    .lexical-editor-wrapper-style :global(.lexical-content) { @apply leading-normal whitespace-pre-wrap break-words text-gray-900 dark:text-gray-100; min-height: unset !important; font-family: Arial, Helvetica, sans-serif; font-size: 12pt; line-height: 1.5;}
    .lexical-editor-wrapper-style :global(.lexical-content p) { @apply mt-0 mb-0; }
    .lexical-editor-wrapper-style :global(.toolbar .separator) {margin-left: 0.05rem; margin-right: 0.05rem;}
    .editable-transcript-controls { /* Class added */ }
</style>