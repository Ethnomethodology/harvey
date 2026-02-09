<script>
    import { get } from 'svelte/store';
    import { project } from '$lib/stores/projectStore.js';
    import { transcriptStore, updateSegment, updatePlayerTime, updateSecondarySegment } from '$lib/stores/transcriptStore.js';
    import { onMount, onDestroy, tick, createEventDispatcher, afterUpdate } from 'svelte';
    import LexicalEditor from '$lib/components/projectview/lexical/LexicalEditor.svelte';
    import { confirm } from '@tauri-apps/plugin-dialog';
    import Dropdown from '$lib/components/shared/Dropdown.svelte';


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
                commitCurrentSegmentEdits();
                previous();
            } else if (event.key === 'ArrowDown') {
                event.preventDefault();
                commitCurrentSegmentEdits();
                next();
            }
        }
    }

    /* --- Component Props & State --- */
    export let panelEditMode = false;
    export let previewEditMode = false;
    let editEnabled = false;
    let segments = [];
    let secondarySegments = [];
    let currentIndex = -1;
    let targetIndexForLoad = -1; // Primarily for tracking if a load was requested externally

    // --- Primary Segment State ---
    let localStart = '';
    let localEnd = '';
    let localSpeaker = '';
    let lexicalEditorInstance;
    let currentEditorJson = null;
    let initialJsonForEditor = null;

    // --- Secondary Segment State (for Dual Mode) ---
    let localSpeakerSecondary = '';
    let lexicalEditorInstanceSecondary;
    let currentEditorJsonSecondary = null;
    let initialJsonForEditorSecondary = null;

    function handleSpeakerSelectionChange(event) {
        const newSpeaker = event.detail;
        if (newSpeaker !== localSpeaker) {
            localSpeaker = newSpeaker;
            handleSpeakerChange();
        }
    }
     function handleSpeakerSelectionChangeSecondary(event) {
        const newSpeaker = event.detail;
        if (newSpeaker !== localSpeakerSecondary) {
            localSpeakerSecondary = newSpeaker;
            // This will be saved on commit
        }
    }

    const dispatch = createEventDispatcher();
    let isMounted = false;
    let isEditorVisible = false;
    $: isEditorVisible = segments.length > 0 && currentIndex >= 0 && currentIndex < segments.length;

    // Determine platform-specific modifier key name
    const isMac = typeof window !== 'undefined' && navigator.platform.toUpperCase().indexOf('MAC') >= 0;
    const modKeyName = isMac ? 'Cmd' : 'Ctrl';

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
    function formatTimestamp(sec) { if (typeof sec !== 'number' || isNaN(sec) || sec < 0) return '00:00:00.000'; const totalMs = Math.round(sec * 1000); const ms = String(totalMs % 1000).padStart(3, '0'); const totalSeconds = Math.floor(sec); const hours = String(Math.floor(totalSeconds / 3600)).padStart(2, '0'); const minutes = String(Math.floor((totalSeconds % 3600) / 60)).padStart(2, '0'); const seconds = String(totalSeconds % 60).padStart(2, '0'); return `${hours}:${minutes}:${seconds}.${ms}`; }
    function parseTimestamp(str) { if (!str) return null; let parts = str.match(/^(\d{2,}):(\d{2}):(\d{2})\.(\d{3})$/); if (parts) { const hours = parseInt(parts[1], 10); const minutes = parseInt(parts[2], 10); const seconds = parseInt(parts[3], 10); const milliseconds = parseInt(parts[4], 10); if (minutes < 60 && seconds < 60) { return hours * 3600 + minutes * 60 + seconds + milliseconds / 1000; } } parts = str.match(/^(\d{1,9}):(\d{2})\.(\d{3})$/); if (parts) { const minutes = parseInt(parts[1], 10); const seconds = parseInt(parts[2], 10); const milliseconds = parseInt(parts[3], 10); if (seconds < 60) { return minutes * 60 + seconds + milliseconds / 1000; } } const floatVal = parseFloat(str); return isNaN(floatVal) ? null : floatVal; }
    function dispatchEditState() { if (!isMounted) return; if (editEnabled && currentIndex >= 0 && currentIndex < segments.length) { const seg = segments[currentIndex]; const startTime = typeof seg?.start_time === 'number' ? seg.start_time : 0; const endTime = typeof seg?.end_time === 'number' ? seg.end_time : 0; dispatch('segmenteditfocus', { isEditing: true, startTime: startTime, endTime: endTime }); } else { dispatch('segmenteditfocus', { isEditing: false, startTime: 0, endTime: 0 }); } }

    /* --- Render UI --- */
    async function renderSegmentUI(idx) {
        if (!isMounted) return;

        const isDual = get(transcriptStore).isDualModeActive;

        if (!segments || segments.length === 0 || idx < 0 || idx >= segments.length || (isDual && (!secondarySegments || secondarySegments.length === 0))) {
            const needsClear = currentIndex !== -1;
            currentIndex = -1; targetIndexForLoad = -1; localStart = ''; localEnd = ''; localSpeaker = ''; localSpeakerSecondary = '';
            initialJsonForEditor = defaultEmptyJsonString;
            currentEditorJson = defaultEmptyJsonString;
            initialJsonForEditorSecondary = defaultEmptyJsonString;
            currentEditorJsonSecondary = defaultEmptyJsonString;
            if (lexicalEditorInstance && isEditorVisible) lexicalEditorInstance.resetEditorState(defaultEmptyJsonString);
            if (lexicalEditorInstanceSecondary && isEditorVisible && isDual) lexicalEditorInstanceSecondary.resetEditorState(defaultEmptyJsonString);
            if (needsClear) { await tick(); dispatchEditState(); }
            return;
        }

        currentIndex = idx;
        targetIndexForLoad = -1;

        // Process a single segment (primary or secondary)
        const processSegment = (seg) => {
            if (!seg || typeof seg.start_time !== 'number' || typeof seg.end_time !== 'number') {
                return { localSpeaker: 'Error', initialJson: defaultEmptyJsonString };
            }
            const localSpeaker = seg.speaker || 'Unknown';
            let segmentText = seg.text || '';
            let jsonToProcess = segmentText;
            let isValidLexicalJson = false;

            if (segmentText && typeof segmentText === 'string') {
                try {
                    const parsed = JSON.parse(segmentText);
                    if (parsed && parsed.root && parsed.root.type === 'root') isValidLexicalJson = true;
                } catch (e) { /* not json */ }
            }

            let initialJson;
            if (isValidLexicalJson) {
                initialJson = !jsonToProcess || jsonToProcess.trim() === '' ? defaultEmptyJsonString : jsonToProcess;
            } else {
                initialJson = createJsonFromPlainText(extractPlainText(segmentText));
            }
            return { localSpeaker, initialJson };
        };

        // Load Primary Segment
        const primarySeg = segments[idx];
        const { localSpeaker: primarySpeaker, initialJson: primaryJson } = processSegment(primarySeg);
        localSpeaker = primarySpeaker;
        initialJsonForEditor = primaryJson;
        currentEditorJson = initialJsonForEditor;

        if (primarySeg) {
             localStart = formatTimestamp(primarySeg.start_time);
             localEnd = formatTimestamp(primarySeg.end_time);
        }

        if (lexicalEditorInstance) {
            lexicalEditorInstance.updateContent(initialJsonForEditor);
        }

        // Load Secondary Segment if in Dual Mode
        if (isDual) {
            const secondarySeg = secondarySegments[idx];
            if (secondarySeg) {
                const { localSpeaker: secondarySpeaker, initialJson: secondaryJson } = processSegment(secondarySeg);
                localSpeakerSecondary = secondarySpeaker;
                initialJsonForEditorSecondary = secondaryJson;
                currentEditorJsonSecondary = initialJsonForEditorSecondary;

                if (lexicalEditorInstanceSecondary) {
                    lexicalEditorInstanceSecondary.updateContent(initialJsonForEditorSecondary);
                }
            } else {
                 localSpeakerSecondary = 'Error';
                 initialJsonForEditorSecondary = defaultEmptyJsonString;
                 currentEditorJsonSecondary = defaultEmptyJsonString;
                 if(lexicalEditorInstanceSecondary) lexicalEditorInstanceSecondary.resetEditorState(defaultEmptyJsonString);
            }
        }


        dispatchEditState();
        await tick();
    }


    /* --- Public methods --- */
    export function loadSegment(i) { if (i >= 0 && i < segments.length) { targetIndexForLoad = i; if (i !== currentIndex) { dispatch('navigate', { index: i }); } renderSegmentUI(i); } else { targetIndexForLoad = -1; renderSegmentUI(i); } }
    export function loadSegmentSilent(i) { if (i >= 0 && i < segments.length) { if (i !== currentIndex) { targetIndexForLoad = i; renderSegmentUI(i); } } else { targetIndexForLoad = -1; if (i !== currentIndex) renderSegmentUI(i); } }
    export function updateTimesFromExternal(newStartTime, newEndTime) {
        if (!editEnabled || currentIndex < 0 || currentIndex >= segments.length) return;

        let changes = {};
        let prevSegChanges = null;
        let nextSegChanges = null;
        const currentSeg = segments[currentIndex];

        const startTimeChanged = Math.abs(newStartTime - (currentSeg.start_time || 0)) > 0.0001;
        const endTimeChanged = Math.abs(newEndTime - (currentSeg.end_time || 0)) > 0.0001;

        if (startTimeChanged) {
            localStart = formatTimestamp(newStartTime);
            changes.start_time = newStartTime;
            if (currentIndex > 0) {
                prevSegChanges = { end_time: newStartTime };
            }
        } else {
            localStart = formatTimestamp(currentSeg.start_time);
        }

        if (endTimeChanged) {
            localEnd = formatTimestamp(newEndTime);
            changes.end_time = newEndTime;
            if (currentIndex < segments.length - 1) {
                nextSegChanges = { start_time: newEndTime };
            }
        } else {
            localEnd = formatTimestamp(currentSeg.end_time);
        }

        if (Object.keys(changes).length > 0) {
            const isDual = get(transcriptStore).isDualModeActive;

            // Update adjacent segments first to keep continuity logic clean in history?
            // Order doesn't strictly matter for correctness but might for undo history perception.
            if (prevSegChanges) {
                updateSegment(currentIndex - 1, prevSegChanges, true);
                if (isDual) updateSecondarySegment(currentIndex - 1, prevSegChanges);
            }

            updateSegment(currentIndex, changes, true);
            if (isDual) updateSecondarySegment(currentIndex, changes);

            if (nextSegChanges) {
                updateSegment(currentIndex + 1, nextSegChanges, true);
                if (isDual) updateSecondarySegment(currentIndex + 1, nextSegChanges);
            }

            tick().then(dispatchEditState);
            const currentTime = get(transcriptStore).player.currentTime;
            if (currentTime < newStartTime || currentTime >= newEndTime) {
                updatePlayerTime(newStartTime);
            }
        }
    }
    export function focusEditor() { /* Lexical focus */ }
    export function forceReloadFromStore() { if (isMounted && currentIndex >= 0 && currentIndex < segments.length) { renderSegmentUI(currentIndex); } }

    /* --- Lifecycle & Store Subscription --- */
    let unsubscribeTranscriptStore;
    onMount(() => {
        isMounted = true; window.addEventListener('keydown', handleSegmentNavShortcut, true);
        unsubscribeTranscriptStore = transcriptStore.subscribe((ts) => {
            if (!isMounted) return;

            const newSegments = ts.segments || [];
            const newSecondarySegments = ts.secondaryTranscriptSegments || [];
            const currentStoreIndex = ts.player?.currentSegmentIndex ?? -1;
            const activeTranscriptPath = ts.activeTranscript?.path;

            // Determine if segments array itself has changed (e.g., new load, undo/redo, insert/delete)
            // This is a shallow comparison of the array reference, plus a length check.
            // For deep content changes, we rely on the activeTranscript.segments reference or explicit actions.
            const segmentsArrayReferenceChanged = newSegments !== segments;
            const segmentsLengthChanged = newSegments.length !== segments.length;
            const secondarySegmentsChanged = newSecondarySegments !== secondarySegments;

            // Update local segments reference
            segments = newSegments;
            secondarySegments = newSecondarySegments;

            // Scenario 1: Structural change (segments array reference or length changed)
            // This covers new transcript loads, undo/redo, segment insert/delete.
            if (segmentsArrayReferenceChanged || segmentsLengthChanged || secondarySegmentsChanged) {
                // If segments become empty, ensure transcript is not dirty.
                if (newSegments.length === 0) {
                    transcriptStore.update(ts => ({ ...ts, transcriptDirty: false }));
                }
                // Force re-render of the current segment based on the store's current index.
                // This ensures the editor reflects the latest state after a structural modification.
                renderSegmentUI(currentStoreIndex);
            }
            // Scenario 2: Player seeking while NOT in edit mode, and the segment index has changed.
            // This is for navigation through the transcript without explicit editing.
            else if (!editEnabled && currentStoreIndex !== currentIndex) {
                if (currentStoreIndex >= 0 && currentStoreIndex < segments.length) {
                    // Load the new segment silently (without dispatching navigation events)
                    loadSegmentSilent(currentStoreIndex);
                } else if (segments.length === 0) {
                    // If no segments, ensure UI is cleared
                    renderSegmentUI(-1);
                }
                // If currentStoreIndex is -1 but segments exist, we keep the last displayed segment.
                // This is a design choice to not clear the editor if the player is between segments or at the end.
            }
            // Scenario 3: Content of the *currently active* segment might have changed (e.g., external update, speaker remapping)
            // This is a more granular check for the specific segment being displayed.
            else if (!editEnabled && currentIndex >= 0 && currentIndex < segments.length) {
                const currentSegmentInStore = segments[currentIndex];
                const currentSegmentText = currentSegmentInStore?.text;
                const currentSegmentSpeaker = currentSegmentInStore?.speaker;
                const currentSegmentStart = currentSegmentInStore?.start_time;
                const currentSegmentEnd = currentSegmentInStore?.end_time;

                // Compare with local state (initialJsonForEditor, localSpeaker, localStart, localEnd)
                // Note: initialJsonForEditor is the JSON string that was last used to initialize the Lexical editor.
                // We need to parse it to compare its content, or find a more direct way to compare.
                // For now, a simple string comparison of the JSON is used, which is still somewhat expensive.
                // A better approach would be to have a version/hash for each segment's content.
                const initialJsonParsed = initialJsonForEditor ? JSON.parse(initialJsonForEditor) : null;
                const currentSegmentTextParsed = currentSegmentText ? JSON.parse(currentSegmentText) : null;

                const textContentChanged = JSON.stringify(initialJsonParsed) !== JSON.stringify(currentSegmentTextParsed);
                const speakerChanged = localSpeaker !== currentSegmentSpeaker;
                const startChanged = Math.abs(parseTimestamp(localStart) - currentSegmentStart) > 0.0001;
                const endChanged = Math.abs(parseTimestamp(localEnd) - currentSegmentEnd) > 0.0001;

                if (textContentChanged || speakerChanged || startChanged || endChanged) {
                    renderSegmentUI(currentIndex);
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
    export function previous() {
        commitCurrentSegmentEdits();
        if (currentIndex > 0) {
            loadSegment(currentIndex - 1);
        }
    }
    export function next() {
        commitCurrentSegmentEdits();
        if (currentIndex < segments.length - 1) {
            loadSegment(currentIndex + 1);
        }
    }
    function handlePreviousClick() { dispatch('previous'); }
    function handleNextClick() { dispatch('next'); }

    /* --- Editor Actions --- */
    function handleBlurTimestamp(field, value) {
        if (!editEnabled || currentIndex < 0 || currentIndex >= segments.length) return false;
        const parsedTime = parseTimestamp(value);
        if (parsedTime === null) {
            // Revert to original if invalid
            const seg = segments[currentIndex];
            if (field === 'start_time') localStart = formatTimestamp(seg.start_time);
            else localEnd = formatTimestamp(seg.end_time);
            return;
        }

        const changes = {};
        changes[field] = parsedTime;

        // In dual mode, apply timestamp changes to both segments
        if (get(transcriptStore).isDualModeActive) {
            updateSegment(currentIndex, changes); // Update primary
            updateSecondarySegment(currentIndex, changes); // Update secondary
        } else {
            updateSegment(currentIndex, changes);
        }

        tick().then(dispatchEditState);
        const playerTime = get(transcriptStore).player.currentTime;
        const seg = segments[currentIndex];
        const startTime = field === 'start_time' ? parsedTime : seg.start_time;
        const endTime = field === 'end_time' ? parsedTime : seg.end_time;
        if (playerTime < startTime || playerTime >= endTime) {
            updatePlayerTime(startTime);
        }
    }
    function handleSpeakerChange() { if (editEnabled && currentIndex >= 0 && currentIndex < segments.length) { const currentSpeaker = segments[currentIndex].speaker || 'Unknown'; if (localSpeaker !== currentSpeaker) { updateSegment(currentIndex, { speaker: localSpeaker }); return true; } } return false; }
    function handleEditorUpdate(event) {
        currentEditorJson = event.detail.jsonString;
	}
    export function commitCurrentSegmentEdits() {
        if (!editEnabled || currentIndex < 0 || currentIndex >= segments.length) {
            return false;
        }

        const segmentInStore = get(transcriptStore).segments[currentIndex];
        const newStartTime = parseTimestamp(localStart);
        const newEndTime = parseTimestamp(localEnd);

        let changes = {};
        let textChanged = false;

        if (currentEditorJson) {
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

            // validate it contains a Lexical root
            try {
                const parsed = JSON.parse(jsonString);
                if (!parsed || !parsed.root) {
                    throw new Error("Invalid JSON structure (missing root)");
                }
            } catch (e) {
                return false;
            }
            if (jsonString !== segmentInStore.text) {
                changes.text = jsonString;
                textChanged = true;
            }
        }

        const startTimeChanged = Math.abs(newStartTime - (segmentInStore.start_time || 0)) > 0.0001;
        if (startTimeChanged) {
            changes.start_time = newStartTime;
        }

        const endTimeChanged = Math.abs(newEndTime - (segmentInStore.end_time || 0)) > 0.0001;
        if (endTimeChanged) {
            changes.end_time = newEndTime;
        }

        const speakerChanged = localSpeaker !== segmentInStore.speaker;
        if (speakerChanged) {
            changes.speaker = localSpeaker;
        }

        const hasChanges = Object.keys(changes).length > 0;

        if (hasChanges) {
            updateSegment(currentIndex, changes);
        }

        // --- Commit secondary segment changes in dual mode ---
        if (get(transcriptStore).isDualModeActive) {
            const secondarySegmentInStore = get(transcriptStore).secondaryTranscriptSegments[currentIndex];
            let secondaryChanges = {};
            let secondaryTextChanged = false;

            if (currentEditorJsonSecondary && currentEditorJsonSecondary !== secondarySegmentInStore.text) {
                secondaryChanges.text = currentEditorJsonSecondary;
                secondaryTextChanged = true;
            }
            if (localSpeakerSecondary !== secondarySegmentInStore.speaker) {
                secondaryChanges.speaker = localSpeakerSecondary;
            }

            if (Object.keys(secondaryChanges).length > 0) {
                updateSecondarySegment(currentIndex, secondaryChanges);
                return true; // Indicate that a change was made
            }
        }

        return hasChanges;
    }
    function handleEditSaveClick() {
        if (editEnabled) {
            dispatch('toggleedit');
        } else {
            dispatch('toggleedit');
        }
    }
    const EDIT_ICON = `<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-6"> <path stroke-linecap="round" stroke-linejoin="round" d="m16.862 4.487 1.687-1.688a1.875 1.875 0 1 1 2.652 2.652L10.582 16.07a4.5 4.5 0 0 1-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 0 1 1.13-1.897l8.932-8.931Zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0 1 15.75 21H5.25A2.25 2.25 0 0 1 3 18.75V8.25A2.25 2.25 0 0 1 5.25 6H10" /> </svg>`;
    const SAVE_ICON = `<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-6"> <path stroke-linecap="round" stroke-linejoin="round" d="M10.125 2.25h-4.5c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125v-9M10.125 2.25h.375a9 9 0 0 1 9 9v.375M10.125 2.25A3.375 3.375 0 0 1 13.5 5.625v1.5c0 .621.504 1.125 1.125 1.125h1.5a3.375 3.375 0 0 1 3.375 3.375M9 15l2.25 2.25L15 12" /> </svg>`;

    // --- Layout specific styles ---
    const columnContainerClass = 'flex flex-col mx-auto gap-y-2 mt-4';

    $: speakerOptions = [
        { value: 'Unknown', label: 'Unknown' },
        ...($transcriptStore.speakers.names.map(name => ({ value: name, label: name })))
    ];

</script>

<div class="editable-transcript-wrapper p-2 h-full flex flex-col text-gray-900 dark:text-gray-200 text-sm bg-white dark:bg-surface-2 rounded-md shadow-sm editable-transcript-controls"
     class:read-mode="{!editEnabled}"
     class:edit-mode="{editEnabled}">
    {#if !isEditorVisible}
        {#if segments.length === 0} <div class="flex-grow flex items-center justify-center text-gray-500 dark:text-gray-400 p-4"> No transcript loaded or transcript is empty. </div>
        {:else} <div class="flex-grow flex items-center justify-center text-gray-500 dark:text-gray-400 p-4"> Select a segment to start editing. </div> {/if}
    {:else}
        <div class="flex flex-col flex-grow min-h-0 h-full">
            <div class="relative py-1 flex-shrink-0 mb-4">
                <button on:click="{handleEditSaveClick}"
                        class='btn-icon absolute left-0 top-1 text-gray-600 hover:text-gray-800 dark:text-white'
                        title="{editEnabled ? `Save & Exit Edit mode (${modKeyName}+E)` : `Enable Editing (${modKeyName}+E)`}"
                        aria-label="{editEnabled ? 'Save Changes' : 'Enable Editing'}"
                        style="padding-left:0px;">
                    {@html editEnabled ? SAVE_ICON : EDIT_ICON}
                </button>
                <button on:click="{handlePreviousClick}" class="btn-nav-vertical absolute left-1/2 top-1 transform -translate-x-1/2" disabled="{currentIndex <= 0}" aria-label="Previous Segment" title="Previous Segment ({modKeyName}+Up)">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="size-5"> <path stroke-linecap="round" stroke-linejoin="round" d="m4.5 15.75 7.5-7.5 7.5 7.5" /> </svg>
                </button>
            </div>
            <!-- Main content area for inputs, unified layout -->
            <div class="flex-grow overflow-y-auto">
                <div class="flex justify-center">
                    <div class="{columnContainerClass}" style="width: 40.01rem; font-family: Arial, Helvetica, sans-serif; font-size: 12pt; line-height: 1.5;">

                        <!-- Primary Segment Editor -->
                        <div class="primary-segment-editor">
                            <!-- Row 1: Num, Time, Speaker -->
                            <div class="flex items-center gap-x-2 flex-shrink-0 mb-2">
                                <!-- Segment Number -->
                                <div class='flex-shrink-0 text-left py-1 min-w-[2rem]'>
                                    <span class='text-sm text-gray-500' title="{String(currentIndex + 1)}">{String(currentIndex + 1)}</span>
                                </div>
                                <!-- Timestamps -->
                                <div class='flex-shrink-0 text-gray-600 dark:text-white text-left leading-tight flex items-center gap-x-1'>
                                    <input id='startTimeInput' class='input-field w-[12ch] text-sm p-0' type='text' bind:value="{localStart}" disabled="{!editEnabled}" on:blur="{() => handleBlurTimestamp('start_time', localStart)}" on:keydown="{(e) => { if (e.key === 'Enter') e.target.blur(); }}" aria-label='Segment start time' placeholder='00:00:00.000' autocomplete="off" autocorrect="off" />
                                    <span class='text-gray-400 dark:text-white'>–</span>
                                    <input id='endTimeInput' class='input-field w-[12ch] text-sm p-0' type='text' bind:value="{localEnd}" disabled="{!editEnabled}" on:blur="{() => handleBlurTimestamp('end_time', localEnd)}" on:keydown="{(e) => { if (e.key === 'Enter') e.target.blur(); }}" aria-label='Segment end time' placeholder='00:00:00.000' autocomplete="off" autocorrect="off" />
                                </div>
                                <!-- Speaker -->
                                <div class='relative flex-grow max-w-[10rem]'>
                                    <Dropdown
                                        options={speakerOptions}
                                        bind:value={localSpeaker}
                                        on:change={handleSpeakerSelectionChange}
                                        disabled={!editEnabled}
                                        placeholder="Select Speaker"
                                        containerClasses="w-full"
                                    />
                                </div>
                            </div>
                            <!-- Row 2: Text Editor -->
                            <div class="flex items-start gap-x-1 w-full">
                                <div class='lexical-editor-wrapper-style w-full flex-grow' class:is-disabled="{!editEnabled}">
                                    {#if currentIndex !== -1 && initialJsonForEditor}
                                        <LexicalEditor bind:this="{lexicalEditorInstance}" initialJson="{initialJsonForEditor}" editable="{editEnabled}" placeholder='Enter transcript text…' toolbarConfig="{{ undo: true, redo: true, bold: true, italic: true, underline: true, strikethrough: true, textColor: true, highlight: true, clearFormatting: true }}" on:change="{handleEditorUpdate}" enableFloatingToolbar="{false}" />
                                    {:else}
                                        <div class='p-2 text-gray-400 italic text-center flex-grow flex items-center justify-center'>Loading editor...</div>
                                    {/if}
                                </div>
                            </div>
                        </div>

                        {#if $transcriptStore.isDualModeActive}
                            <div class="w-full my-4 border-t border-gray-300 dark:border-gray-600"></div>

                            <!-- Secondary Segment Editor: ONLY Row 2 (Text Editor) -->
                            <div class="secondary-segment-editor">
                                <div class="flex items-start gap-x-1 w-full">
                                    <div class='lexical-editor-wrapper-style w-full flex-grow' class:is-disabled="{!editEnabled}">
                                        {#if currentIndex !== -1 && initialJsonForEditorSecondary}
                                            <LexicalEditor bind:this="{lexicalEditorInstanceSecondary}" initialJson="{initialJsonForEditorSecondary}" editable="{editEnabled}" placeholder='Enter transcript text…' toolbarConfig="{{ undo: true, redo: true, bold: true, italic: true, underline: true, strikethrough: true, textColor: true, highlight: true, clearFormatting: true }}" on:change="{(e) => currentEditorJsonSecondary = e.detail.jsonString}" enableFloatingToolbar="{false}" />
                                        {/if}
                                    </div>
                                </div>
                            </div>
                        {/if}


                    </div>
                </div>
            </div>
            <div class="flex justify-center py-1 flex-shrink-0 mt-auto"> <button on:click="{handleNextClick}" class="btn-nav-vertical" disabled="{currentIndex >= segments.length - 1}" aria-label="Next Segment" title="Next Segment ({modKeyName}+Down)"> <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="size-5"> <path stroke-linecap="round" stroke-linejoin="round" d="m19.5 8.25-7.5 7.5-7.5-7.5" /> </svg> </button> </div>
        </div>
    {/if}
</div>

<style lang="postcss">
	.input-field {
		@apply text-center bg-transparent border-0 p-0 text-gray-800 dark:text-text-secondary;
	}
	.input-field:not(:disabled) {
		@apply bg-white dark:bg-surface-3 border border-gray-300 dark:border-border text-gray-900 dark:text-text-primary rounded;
	}

    .size-6 { @apply w-6 h-6; } .size-5 { @apply w-5 h-5; }
    .btn-icon { @apply p-1 rounded hover:bg-gray-200 dark:bg-transparent dark:border dark:border-[var(--color-border)] dark:hover:bg-[var(--color-border)] focus:outline-none focus:ring-1 focus:ring-offset-1 focus:ring-blue-400 dark:focus:ring-blue-500 dark:ring-offset-gray-800 focus:bg-gray-200 dark:focus:bg-gray-600 transition duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-transparent dark:disabled:hover:bg-transparent; }
    .btn-nav-vertical { @apply p-1 bg-gray-100 hover:bg-gray-200 text-gray-700 dark:bg-transparent dark:text-white dark:border dark:border-[var(--color-border)] dark:hover:bg-[var(--color-border)] rounded-md disabled:opacity-50 disabled:cursor-not-allowed focus:outline-none focus:ring-1 focus:ring-offset-1 focus:ring-blue-400 dark:focus:ring-blue-500 dark:ring-offset-gray-800 focus:bg-gray-200 dark:focus:bg-gray-600 transition-colors flex items-center justify-center; }

    .lexical-editor-wrapper-style {
        display: flex;
        flex-direction: column;
        @apply rounded;
        overflow: visible;
    }
    .lexical-editor-wrapper-style:not(.is-disabled) {
        @apply border border-gray-300 dark:border-border bg-white dark:bg-transparent;
    }
    .lexical-editor-wrapper-style.is-disabled {
        @apply border-transparent dark:border-transparent bg-transparent;
    }

    .lexical-editor-wrapper-style > :global(.lexical-editor-root) {
        flex-grow: 1;
        min-height: 0;
        border: none !important;
        border-radius: 0 !important;
        box-shadow: none !important;
        overflow: visible;
        background-color: transparent !important;
    }
    .lexical-editor-wrapper-style > :global(.lexical-editor-root > .lexical-wrapper) {
        overflow-y: auto;
        height: 100%;
        padding: 8px;
    }

    .lexical-editor-wrapper-style :global(.lexical-content) {
        @apply leading-normal whitespace-pre-wrap break-words;
        min-height: unset !important;
        font-family: Arial, Helvetica, sans-serif;
        font-size: 12pt;
        line-height: 1.5;
    }
    .lexical-editor-wrapper-style:not(.is-disabled) :global(.lexical-content) {
        @apply text-gray-900 dark:text-text-primary;
    }
    .lexical-editor-wrapper-style.is-disabled :global(.lexical-content) {
        @apply text-gray-800 dark:text-text-primary cursor-not-allowed;
    }

    .lexical-editor-wrapper-style :global(.lexical-content p) { @apply mt-0 mb-0; }

    :global(html.dark .input-field:disabled) {
        background-color: var(--color-surface-3);
        border: 1px solid var(--color-surface-2);
        color: white;
    }
    :global(html.dark .input-field:not(:disabled)) {
        background-color: var(--color-surface-2);
        border: 1px solid var(--color-surface-3);
        color: white;
    }

    /* Read Mode */
    :global(html.dark .editable-transcript-wrapper.read-mode .lexical-editor-wrapper-style) {
        border: 1px solid var(--color-surface-2) !important;
    }
    :global(html.dark .editable-transcript-wrapper.read-mode .lexical-wrapper) {
        background-color: var(--color-surface-3) !important;
    }
    :global(html.dark .editable-transcript-wrapper.read-mode .lexical-content) {
        color: white !important;
    }

    /* Edit Mode */
    :global(html.dark .editable-transcript-wrapper.edit-mode .lexical-editor-wrapper-style) {
        border: 1px solid var(--color-surface-3) !important;
    }
    :global(html.dark .editable-transcript-wrapper.edit-mode .lexical-wrapper) {
        background-color: var(--color-surface-2) !important;
    }
    :global(html.dark .editable-transcript-wrapper.edit-mode .lexical-content) {
        color: white !important;
    }

    :global(html.dark .editable-transcript-wrapper.read-mode .ui-select:disabled) {
        background-color: var(--color-surface-3);
        border-color: var(--color-surface-2);
        color: white;
        opacity: 1;
    }
    :global(html.dark .editable-transcript-wrapper.read-mode .ui-select:disabled span) {
        color: white;
    }
    :global(html.dark .editable-transcript-wrapper.read-mode .ui-select:disabled svg) {
        color: white;
    }

    :global(html:not(.dark) .editable-transcript-wrapper.read-mode .ui-select:disabled) {
        background-color: #f9fafb; /* bg-gray-50 */
        border-color: #d1d5db; /* border-gray-300 */
        color: #111827; /* text-gray-900 */
        opacity: 1;
    }
    :global(html:not(.dark) .editable-transcript-wrapper.read-mode .ui-select:disabled span) {
        color: #111827; /* text-gray-900 */
    }
    :global(html:not(.dark) .editable-transcript-wrapper.read-mode .ui-select:disabled svg) {
        color: #111827; /* text-gray-900 */
    }

    :global(html:not(.dark) .editable-transcript-wrapper.read-mode .input-field:disabled) {
        background-color: #f9fafb; /* bg-gray-50 */
        color: #111827; /* text-gray-900 */
        border: 1px solid #d1d5db; /* border-gray-300 */
    }

    :global(html:not(.dark) .editable-transcript-wrapper.read-mode .lexical-editor-wrapper-style.is-disabled) {
        background-color: #f9fafb; /* bg-gray-50 */
        border: 1px solid #d1d5db !important; /* border-gray-300 */
    }

    :global(html:not(.dark) .editable-transcript-wrapper.read-mode .lexical-editor-wrapper-style.is-disabled .lexical-content) {
        color: #111827 !important; /* text-gray-900 */
    }
</style>