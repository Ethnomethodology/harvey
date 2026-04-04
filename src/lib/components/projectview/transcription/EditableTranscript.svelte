<script>
  import { get } from 'svelte/store';
  import { project } from '$lib/stores/projectStore.js';
  import {
    transcriptStore,
    updateSegment,
    updatePlayerTime,
    updateSecondarySegment
  } from '$lib/stores/transcriptStore.js';
  import { onMount, onDestroy, tick, createEventDispatcher, afterUpdate } from 'svelte';
  import LexicalEditor from '$lib/components/projectview/lexical/LexicalEditor.svelte';
  import { confirm } from '@tauri-apps/plugin-dialog';
  import Dropdown from '$lib/components/shared/Dropdown.svelte';

  // --- Lexical Imports --- (Keep as is)
  import {
    $getRoot as getRoot,
    $createParagraphNode as createParagraphNode,
    $createTextNode as createTextNode,
    $insertNodes as insertNodes,
    RootNode,
    ParagraphNode,
    TextNode,
    LineBreakNode
  } from 'lexical';
  import { createHeadlessEditor } from '@lexical/headless';
  import { $generateHtmlFromNodes as generateHtmlFromNodes } from '@lexical/html';
  import { HeadingNode, QuoteNode } from '@lexical/rich-text';
  import { ListNode, ListItemNode } from '@lexical/list';
  import { TableNode, TableRowNode, TableCellNode } from '@lexical/table';
  import { LinkNode } from '@lexical/link';
  import { ExtendedTextNode } from '$lib/nodes/ExtendedTextNode.js';
  import { SquarePen, Save, ChevronUp, ChevronDown } from '@lucide/svelte';

  /* --- Keyboard Shortcut --- */
  function handleSegmentNavShortcut(event) {
    const tgt = event.target;
    const isEditingText =
      tgt instanceof HTMLElement &&
      (tgt.tagName === 'INPUT' || tgt.tagName === 'TEXTAREA' || tgt.isContentEditable);
    const isMac =
      typeof window !== 'undefined' && navigator.platform.toUpperCase().indexOf('MAC') >= 0;
    const modKey = isMac ? event.metaKey : event.ctrlKey;
    const shiftKey = event.shiftKey;

    // --- Toggle Edit Mode (Global) ---
    if (modKey && event.key.toLowerCase() === 'e') {
      event.preventDefault();
      dispatch('toggleedit');
      return;
    }

    // --- Playback Controls (Shift + Space, Cmd/Ctrl + Shift + Arrows) ---
    if (shiftKey && event.key === ' ') {
      event.preventDefault();
      dispatch('navigate', { action: 'toggle-play' });
      return;
    }

    if (modKey && shiftKey) {
      if (event.key === 'ArrowLeft') {
        event.preventDefault();
        dispatch('navigate', { action: 'rewind' });
        return;
      } else if (event.key === 'ArrowRight') {
        event.preventDefault();
        dispatch('navigate', { action: 'forward' });
        return;
      } else if (event.key === ',' || event.key === '<' || event.code === 'Comma') {
        event.preventDefault();
        dispatch('navigate', { action: 'speed-down' });
        return;
      } else if (event.key === '.' || event.key === '>' || event.code === 'Period') {
        event.preventDefault();
        dispatch('navigate', { action: 'speed-up' });
        return;
      }
    }

    // --- Editing Specific Shortcuts ---
    if (isEditingText && editEnabled && currentIndex >= 0) {
      // Cmd/Ctrl + Shift + Enter -> Insert new segment after current
      if (modKey && shiftKey && event.key === 'Enter') {
        event.preventDefault();
        commitCurrentSegmentEdits();
        dispatch('insertnewsegment', currentIndex);
        return;
      }

      // Cmd/Ctrl + Shift + J/K -> Cycle Speaker (More ergonomic than Alt + Arrows)
      if (
        modKey &&
        shiftKey &&
        (event.key.toLowerCase() === 'j' || event.key.toLowerCase() === 'k')
      ) {
        event.preventDefault();
        const options = speakerOptions.map((o) => o.value);
        const currentSpeakerIndex = options.indexOf(localSpeaker);
        if (currentSpeakerIndex !== -1) {
          let newIndex = currentSpeakerIndex + (event.key.toLowerCase() === 'k' ? 1 : -1);
          if (newIndex < 0) newIndex = options.length - 1;
          if (newIndex >= options.length) newIndex = 0;
          localSpeaker = options[newIndex];
          handleSpeakerChange();
        }
        return;
      }

      // Legacy Alt + Up/Down support (Optional, but let's encourage the new ones)
      // if (!modKey && event.altKey && (event.key === 'ArrowUp' || event.key === 'ArrowDown')) { ... }

      // Cmd/Ctrl + Alt + Arrow navigation (Existing)
      if (modKey && event.altKey) {
        if (event.key === 'ArrowUp') {
          event.preventDefault();
          commitCurrentSegmentEdits();
          previous();
          return;
        } else if (event.key === 'ArrowDown') {
          event.preventDefault();
          commitCurrentSegmentEdits();
          next();
          return;
        }
      }
    }

    // Allow native text input navigation if editing
    if (isEditingText && !modKey && !shiftKey) {
      return;
    }

    // --- Global Navigation ---
    // Cmd/Ctrl + Alt + Arrow navigation
    if (modKey && event.altKey) {
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
  const isMac =
    typeof window !== 'undefined' && navigator.platform.toUpperCase().indexOf('MAC') >= 0;
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
  const defaultEmptyJsonString = JSON.stringify({
    root: {
      children: [
        { children: [], direction: null, format: '', indent: 0, type: 'paragraph', version: 1 }
      ],
      direction: null,
      format: '',
      indent: 0,
      type: 'root',
      version: 1
    }
  });
  let plainTextConverterEditor = null;
  function getPlainTextConverter() {
    if (!plainTextConverterEditor) {
      plainTextConverterEditor = createHeadlessEditor({
        namespace: 'PlainTextConverter',
        nodes: [RootNode, ParagraphNode, TextNode],
        onError: (e) => console.error('PlainTextConverter Error:', e)
      });
    }
    return plainTextConverterEditor;
  }
  function createJsonFromPlainText(text) {
    const editor = getPlainTextConverter();
    let jsonString = defaultEmptyJsonString;
    const plainText = text || '';
    try {
      editor.update(
        () => {
          const root = getRoot();
          root.clear();
          const p = createParagraphNode();
          p.append(createTextNode(plainText));
          root.append(p);
        },
        { discrete: true }
      );
      const editorState = editor.getEditorState();
      if (!editorState.isEmpty()) {
        jsonString = JSON.stringify(editorState.toJSON());
      } else {
        console.warn('[EditableTranscript] createJsonFromPlainText empty state.');
      }
    } catch (e) {
      console.error('Error creating JSON from plain text:', e);
    }
    return jsonString;
  }
  function cleanupPlainTextConverter() {
    plainTextConverterEditor = null;
  }
  function extractPlainText(inputString) {
    if (!inputString || typeof inputString !== 'string') return '';
    if (inputString.trim().startsWith('{') && inputString.trim().endsWith('}')) {
      try {
        const parsed = JSON.parse(inputString);
        if (parsed && parsed.root) return getPlainTextFromJson(parsed);
      } catch (e) {
        /* Ignore */
      }
    }
    try {
      const parser = new DOMParser();
      const doc = parser.parseFromString(inputString, 'text/html');
      return doc.body.textContent || '';
    } catch (e) {
      console.error('[EditableTranscript] Error parsing input string:', e);
      return inputString;
    }
  }
  function getPlainTextFromJson(json) {
    if (!json || !json.root) return '';
    function traverse(node) {
      let text = '';
      if (node.text) text += node.text;
      if (node.children) {
        node.children.forEach((child) => {
          text += traverse(child);
        });
      }
      if (node.type === 'paragraph' || node.type === 'heading') text += '\n';
      return text;
    }
    return traverse(json.root).trim();
  }
  function getPreviewText(segment) {
    if (!segment) return '';
    const text = extractPlainText(segment.text);
    return text.split('\n')[0].substring(0, 150) + (text.length > 150 ? '...' : '');
  }
  function formatTimestamp(sec) {
    if (typeof sec !== 'number' || isNaN(sec) || sec < 0) return '00:00:00.000';
    const totalMs = Math.round(sec * 1000);
    const ms = String(totalMs % 1000).padStart(3, '0');
    const totalSeconds = Math.floor(sec);
    const hours = String(Math.floor(totalSeconds / 3600)).padStart(2, '0');
    const minutes = String(Math.floor((totalSeconds % 3600) / 60)).padStart(2, '0');
    const seconds = String(totalSeconds % 60).padStart(2, '0');
    return `${hours}:${minutes}:${seconds}.${ms}`;
  }
  function parseTimestamp(str) {
    if (!str) return null;
    let parts = str.match(/^(\d{2,}):(\d{2}):(\d{2})\.(\d{3})$/);
    if (parts) {
      const hours = parseInt(parts[1], 10);
      const minutes = parseInt(parts[2], 10);
      const seconds = parseInt(parts[3], 10);
      const milliseconds = parseInt(parts[4], 10);
      if (minutes < 60 && seconds < 60) {
        return hours * 3600 + minutes * 60 + seconds + milliseconds / 1000;
      }
    }
    parts = str.match(/^(\d{1,9}):(\d{2})\.(\d{3})$/);
    if (parts) {
      const minutes = parseInt(parts[1], 10);
      const seconds = parseInt(parts[2], 10);
      const milliseconds = parseInt(parts[3], 10);
      if (seconds < 60) {
        return minutes * 60 + seconds + milliseconds / 1000;
      }
    }
    const floatVal = parseFloat(str);
    return isNaN(floatVal) ? null : floatVal;
  }
  function dispatchEditState() {
    if (!isMounted) return;
    if (editEnabled && currentIndex >= 0 && currentIndex < segments.length) {
      const seg = segments[currentIndex];
      const startTime = typeof seg?.start_time === 'number' ? seg.start_time : 0;
      const endTime = typeof seg?.end_time === 'number' ? seg.end_time : 0;
      dispatch('segmenteditfocus', { isEditing: true, startTime: startTime, endTime: endTime });
    } else {
      dispatch('segmenteditfocus', { isEditing: false, startTime: 0, endTime: 0 });
    }
  }

  /* --- Render UI --- */
  async function renderSegmentUI(idx) {
    if (!isMounted) return;

    const isDual = get(transcriptStore).isDualModeActive;

    if (
      !segments ||
      segments.length === 0 ||
      idx < 0 ||
      idx >= segments.length ||
      (isDual && (!secondarySegments || secondarySegments.length === 0))
    ) {
      const needsClear = currentIndex !== -1;
      currentIndex = -1;
      targetIndexForLoad = -1;
      localStart = '';
      localEnd = '';
      localSpeaker = '';
      localSpeakerSecondary = '';
      initialJsonForEditor = defaultEmptyJsonString;
      currentEditorJson = defaultEmptyJsonString;
      initialJsonForEditorSecondary = defaultEmptyJsonString;
      currentEditorJsonSecondary = defaultEmptyJsonString;
      if (lexicalEditorInstance && isEditorVisible)
        lexicalEditorInstance.resetEditorState(defaultEmptyJsonString);
      if (lexicalEditorInstanceSecondary && isEditorVisible && isDual)
        lexicalEditorInstanceSecondary.resetEditorState(defaultEmptyJsonString);
      if (needsClear) {
        await tick();
        dispatchEditState();
      }
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
        } catch (e) {
          /* not json */
        }
      }

      let initialJson;
      if (isValidLexicalJson) {
        initialJson =
          !jsonToProcess || jsonToProcess.trim() === '' ? defaultEmptyJsonString : jsonToProcess;
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
        const { localSpeaker: secondarySpeaker, initialJson: secondaryJson } =
          processSegment(secondarySeg);
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
        if (lexicalEditorInstanceSecondary)
          lexicalEditorInstanceSecondary.resetEditorState(defaultEmptyJsonString);
      }
    }

    dispatchEditState();
    await tick();
  }

  /* --- Public methods --- */
  export function loadSegment(i) {
    if (i >= 0 && i < segments.length) {
      targetIndexForLoad = i;
      renderSegmentUI(i);
      // Always dispatch navigate because clicking an active segment should still seek to its start
      dispatch('navigate', { index: i });
    } else {
      targetIndexForLoad = -1;
      renderSegmentUI(i);
    }
  }
  export function loadSegmentSilent(i) {
    if (i >= 0 && i < segments.length) {
      if (i !== currentIndex) {
        targetIndexForLoad = i;
        renderSegmentUI(i);
      }
    } else {
      targetIndexForLoad = -1;
      if (i !== currentIndex) renderSegmentUI(i);
    }
  }
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
  export function focusEditor() {
    /* Lexical focus */
  }
  export function forceReloadFromStore() {
    if (isMounted && currentIndex >= 0 && currentIndex < segments.length) {
      renderSegmentUI(currentIndex);
    }
  }

  /* --- Lifecycle & Store Subscription --- */
  let unsubscribeTranscriptStore;
  onMount(() => {
    isMounted = true;
    window.addEventListener('keydown', handleSegmentNavShortcut, true);
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
          transcriptStore.update((ts) => ({ ...ts, transcriptDirty: false }));
        }
        // Force re-render of the current segment based on the store's current index.
        // This ensures the editor reflects the latest state after a structural modification.
        renderSegmentUI(currentStoreIndex);
      }
      // Scenario 2: Player seeking or progressing, and the segment index has changed.
      // This is for navigation through the transcript.
      else if (currentStoreIndex !== currentIndex) {
        if (currentStoreIndex >= 0 && currentStoreIndex < segments.length) {
          // If we are in edit mode, commit any pending changes before switching focus
          if (editEnabled) {
            commitCurrentSegmentEdits();
          }
          // Load the new segment silently (without dispatching redundant navigation events back to the player)
          loadSegmentSilent(currentStoreIndex);
        } else if (segments.length === 0) {
          // If no segments, ensure UI is cleared
          renderSegmentUI(-1);
        }
        // If currentStoreIndex is -1 but segments exist, we keep the last displayed segment.
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

        const textContentChanged =
          JSON.stringify(initialJsonParsed) !== JSON.stringify(currentSegmentTextParsed);
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

    // Ensure any pending read-mode highlight saves are committed before unmounting
    if (editorUpdateDebounceTimer) {
      clearTimeout(editorUpdateDebounceTimer);
      commitCurrentSegmentEdits();
    }

    window.removeEventListener('keydown', handleSegmentNavShortcut, true);
    unsubscribeTranscriptStore && unsubscribeTranscriptStore();
    cleanupPlainTextConverter();
    try {
      // Check if transcriptStore is still valid before dispatching
      if (get(transcriptStore)) {
        dispatch('segmenteditfocus', { isEditing: false, startTime: 0, endTime: 0 });
      }
    } catch (e) {
      /* Ignore if store is already destroyed or invalid */
    }
  });
  $: {
    const prevEditEnabled = editEnabled;
    editEnabled = panelEditMode || previewEditMode;
    if (isMounted && editEnabled !== prevEditEnabled) {
      dispatchEditState();
    }
  }

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
  function handlePreviousClick() {
    dispatch('previous');
  }
  function handleNextClick() {
    dispatch('next');
  }

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
  function handleSpeakerChange() {
    if (editEnabled && currentIndex >= 0 && currentIndex < segments.length) {
      const currentSpeaker = segments[currentIndex].speaker || 'Unknown';
      if (localSpeaker !== currentSpeaker) {
        updateSegment(currentIndex, { speaker: localSpeaker });
        return true;
      }
    }
    return false;
  }
  let editorUpdateDebounceTimer;

  function handleEditorUpdate(event) {
    // If the update originated from an 'external' source (programmatic updateContent call),
    // we should NOT update currentEditorJson to avoid race conditions during segment switches.
    const tags = event.detail.tags || [];
    if (tags.includes('external')) {
      console.debug('[EditableTranscript] Ignoring external editor update for primary.');
      return;
    }

    currentEditorJson = event.detail.jsonString;

    if (!editEnabled) {
      // Auto-save read-mode highlight changes with debounce
      clearTimeout(editorUpdateDebounceTimer);
      editorUpdateDebounceTimer = setTimeout(() => {
        commitCurrentSegmentEdits();
      }, 500);
    }
  }

  // In dual mode, listen to secondary editor updates to trigger auto-save if in read mode
  function handleSecondaryEditorUpdate(event) {
    const tags = event.detail.tags || [];
    if (tags.includes('external')) {
      console.debug('[EditableTranscript] Ignoring external editor update for secondary.');
      return;
    }

    currentEditorJsonSecondary = event.detail.jsonString;

    if (!editEnabled) {
      clearTimeout(editorUpdateDebounceTimer);
      editorUpdateDebounceTimer = setTimeout(() => {
        commitCurrentSegmentEdits();
      }, 500);
    }
  }

  export function commitCurrentSegmentEdits() {
    if (currentIndex < 0 || currentIndex >= segments.length) {
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
            return nodes.flatMap((n) =>
              n.type === 'root' && Array.isArray(n.children) ? flattenChildren(n.children) : [n]
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
        console.error('[EditableTranscript] Error sanitizing JSON on save:', e);
        jsonString = jsonStringRaw;
      }

      // validate it contains a Lexical root
      try {
        const parsed = JSON.parse(jsonString);
        if (!parsed || !parsed.root) {
          throw new Error('Invalid JSON structure (missing root)');
        }
      } catch (e) {
        return false;
      }
      if (jsonString !== segmentInStore.text) {
        changes.text = jsonString;
        textChanged = true;
      }
    }

    // Only save start/end time and speaker changes if editEnabled is true,
    // to prevent read mode layout quirks from accidentally modifying them.
    if (editEnabled) {
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
    }

    const hasChanges = Object.keys(changes).length > 0;

    if (hasChanges) {
      updateSegment(currentIndex, changes);
    }

    // --- Commit secondary segment changes in dual mode ---
    if (get(transcriptStore).isDualModeActive) {
      const secondarySegmentInStore =
        get(transcriptStore).secondaryTranscriptSegments[currentIndex];
      let secondaryChanges = {};
      let secondaryTextChanged = false;

      if (
        currentEditorJsonSecondary &&
        currentEditorJsonSecondary !== secondarySegmentInStore.text
      ) {
        secondaryChanges.text = currentEditorJsonSecondary;
        secondaryTextChanged = true;
      }
      if (editEnabled && localSpeakerSecondary !== secondarySegmentInStore.speaker) {
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

  // --- Layout specific styles ---
  const columnContainerClass = 'flex flex-col mx-auto gap-y-2 mt-4';

  $: isTranslation =
    $transcriptStore.activeTranscript?.language_code &&
    ($transcriptStore.activeTranscript.language_code.includes('-') ||
      $transcriptStore.activeTranscript.path?.endsWith('.en.json'));

  // --- Speaker Fallback Logic ---
  // If it's a translation but no translated names are provided, fallback to primary names.
  $: hasTranslatedNames =
    $transcriptStore.speakers.translatedNames &&
    $transcriptStore.speakers.translatedNames.some((n) => n && n.trim() !== '');
  $: activeSpeakerNames =
    isTranslation && hasTranslatedNames
      ? $transcriptStore.speakers.translatedNames || []
      : $transcriptStore.speakers.names || [];

  $: speakerOptions = [
    { value: 'Unknown', label: 'Unknown' },
    ...activeSpeakerNames.map((name) => ({ value: name || 'Unknown', label: name || 'Unknown' }))
  ];
</script>

<div
  class="editable-transcript-wrapper p-2 h-full flex flex-col text-gray-900 dark:text-gray-200 bg-white dark:bg-gray-900 rounded-md shadow-sm editable-transcript-controls"
  class:read-mode={!editEnabled}
  class:edit-mode={editEnabled}
  style="font-size: 12pt;"
>
  {#if !isEditorVisible}
    {#if segments.length === 0}
      <div class="flex-grow flex items-center justify-center text-gray-500 dark:text-gray-400 p-4">
        No transcript loaded or transcript is empty.
      </div>
    {:else}
      <div class="flex-grow flex items-center justify-center text-gray-500 dark:text-gray-400 p-4">
        Select a segment to start editing.
      </div>
    {/if}
  {:else}
    <div class="flex flex-col flex-grow min-h-0 h-full">
      <!-- Removed fixed navigation buttons from top/bottom to move them near editor -->
      <!-- Main content area for inputs, unified layout -->
      <div class="flex-grow overflow-y-auto">
        <div class="flex justify-center px-4 w-full">
          <div
            class="flex flex-col h-full w-full max-w-[42rem] mx-auto picker-wheel-container overflow-hidden"
            style="font-family: Arial, Helvetica, sans-serif; font-size: 12pt; line-height: 1.5;"
          >
            <!-- Previous Segment Area -->
            <div class="flex-1 flex flex-col justify-end min-h-0 py-2">
              {#if currentIndex > 0}
                {@const prevSeg = segments[currentIndex - 1]}
                <button
                  on:click={previous}
                  class="segment-card segment-card-prev group"
                  title="Previous Segment"
                >
                  <div class="flex items-center gap-x-3 mb-1">
                    <span class="text-[10pt] font-bold text-gray-400 dark:text-gray-500"
                      >{currentIndex}</span
                    >
                    <div
                      class="flex items-center gap-x-1 text-[9pt] text-gray-400 dark:text-gray-500 tabular-nums"
                    >
                      <span>{formatTimestamp(prevSeg.start_time)}</span>
                      <span class="opacity-50">–</span>
                      <span>{formatTimestamp(prevSeg.end_time)}</span>
                    </div>
                    <span
                      class="text-[9pt] font-semibold text-gray-400 dark:text-gray-500 truncate max-w-[8rem]"
                      >• {prevSeg.speaker || 'Unknown'}</span
                    >
                  </div>
                  <div class="grid grid-cols-1 md:grid-cols-2 gap-x-4">
                    <div
                      class="text-[10pt] text-gray-400 dark:text-gray-500 line-clamp-1 text-left italic"
                    >
                      {getPreviewText(prevSeg)}
                    </div>
                    {#if $transcriptStore.isDualModeActive && secondarySegments[currentIndex - 1]}
                      <div
                        class="hidden md:block text-[10pt] text-gray-400 dark:text-gray-500 line-clamp-1 text-left italic border-l border-gray-200 dark:border-gray-800 pl-4"
                      >
                        {getPreviewText(secondarySegments[currentIndex - 1])}
                      </div>
                    {/if}
                  </div>
                </button>
              {:else}
                <div
                  class="segment-card segment-card-prev opacity-20 cursor-default border-gray-200/50 dark:border-gray-800/50 flex items-center justify-center text-[10pt] italic text-gray-400 dark:text-gray-500"
                  title="No Previous Segment"
                >
                  No Previous Segment
                </div>
              {/if}
            </div>

            <!-- Primary Segment Editor -->
            <div class="flex-shrink-0 py-2">
              <div class="primary-segment-editor relative z-10">
                <!-- Row 1: Num, Time, Speaker -->
                <div class="flex items-center gap-x-2 flex-shrink-0 mb-2">
                  <!-- Segment Number -->
                  <div class="flex-shrink-0 text-left py-1 min-w-[2rem]">
                    <span
                      class="text-gray-500"
                      title={String(currentIndex + 1)}
                      style="font-size: 12pt;">{String(currentIndex + 1)}</span
                    >
                  </div>
                  <!-- Timestamps -->
                  <div
                    class="flex-shrink-0 text-gray-600 dark:text-white text-left leading-tight flex items-center gap-x-1"
                  >
                    <input
                      id="startTimeInput"
                      class="input-field w-[12ch] p-0"
                      type="text"
                      bind:value={localStart}
                      disabled={!editEnabled}
                      on:blur={() => handleBlurTimestamp('start_time', localStart)}
                      on:keydown={(e) => {
                        if (e.key === 'Enter') e.target.blur();
                      }}
                      aria-label="Segment start time"
                      placeholder="00:00:00.000"
                      autocomplete="off"
                      autocorrect="off"
                    />
                    <span class="text-gray-400 dark:text-white">–</span>
                    <input
                      id="endTimeInput"
                      class="input-field w-[12ch] p-0"
                      type="text"
                      bind:value={localEnd}
                      disabled={!editEnabled}
                      on:blur={() => handleBlurTimestamp('end_time', localEnd)}
                      on:keydown={(e) => {
                        if (e.key === 'Enter') e.target.blur();
                      }}
                      aria-label="Segment end time"
                      placeholder="00:00:00.000"
                      autocomplete="off"
                      autocorrect="off"
                    />
                  </div>
                  <!-- Speaker -->
                  <div class="relative flex-grow max-w-[10rem]">
                    <Dropdown
                      options={speakerOptions}
                      bind:value={localSpeaker}
                      on:change={handleSpeakerSelectionChange}
                      disabled={!editEnabled}
                      placeholder="Select Speaker"
                      containerClasses="w-full"
                      style="font-size: 12pt;"
                    />
                  </div>
                </div>
                <!-- Row 2: Text Editor -->
                <div class="flex items-start gap-x-1 w-full">
                  <div
                    class="lexical-editor-wrapper-style w-full flex-grow"
                    class:is-disabled={!editEnabled}
                  >
                    {#if currentIndex !== -1 && initialJsonForEditor}
                      <LexicalEditor
                        bind:this={lexicalEditorInstance}
                        initialJson={initialJsonForEditor}
                        editable={editEnabled}
                        allowReadModeHighlights={true}
                        enableTableCellResize={false}
                        placeholder="Enter transcript text…"
                        toolbarConfig={{
                          undo: true,
                          redo: true,
                          bold: true,
                          italic: true,
                          underline: true,
                          strikethrough: true,
                          textColor: true,
                          highlight: true,
                          clearFormatting: true
                        }}
                        on:change={handleEditorUpdate}
                        on:textcountchange={(e) =>
                          project.update((p) => ({ ...p, documentTextCount: e.detail }))}
                        enableFloatingToolbar={false}
                      />
                    {:else}
                      <div
                        class="p-2 text-gray-400 italic text-center flex-grow flex items-center justify-center"
                      >
                        Loading editor...
                      </div>
                    {/if}
                  </div>
                </div>
              </div>

              {#if $transcriptStore.isDualModeActive}
                <div class="w-full my-4 border-t border-gray-300 dark:border-gray-600"></div>

                <!-- Secondary Segment Editor: ONLY Row 2 (Text Editor) -->
                <div class="secondary-segment-editor">
                  <div class="flex items-start gap-x-1 w-full">
                    <div
                      class="lexical-editor-wrapper-style w-full flex-grow"
                      class:is-disabled={!editEnabled}
                    >
                      {#if currentIndex !== -1 && initialJsonForEditorSecondary}
                        <LexicalEditor
                          bind:this={lexicalEditorInstanceSecondary}
                          initialJson={initialJsonForEditorSecondary}
                          editable={editEnabled}
                          allowReadModeHighlights={true}
                          enableTableCellResize={false}
                          placeholder="Enter transcript text…"
                          toolbarConfig={{
                            undo: true,
                            redo: true,
                            bold: true,
                            italic: true,
                            underline: true,
                            strikethrough: true,
                            textColor: true,
                            highlight: true,
                            clearFormatting: true
                          }}
                          on:change={handleSecondaryEditorUpdate}
                          enableFloatingToolbar={false}
                        />
                      {/if}
                    </div>
                  </div>
                </div>
              {/if}
            </div>

            <!-- Next Segment Area -->
            <div class="flex-1 flex flex-col justify-start min-h-0 py-2">
              {#if currentIndex < segments.length - 1}
                {@const nextSeg = segments[currentIndex + 1]}
                <button
                  on:click={next}
                  class="segment-card segment-card-next group"
                  title="Next Segment"
                >
                  <div class="flex items-center gap-x-3 mb-1">
                    <span class="text-[10pt] font-bold text-gray-400 dark:text-gray-500"
                      >{currentIndex + 2}</span
                    >
                    <div
                      class="flex items-center gap-x-1 text-[9pt] text-gray-400 dark:text-gray-500 tabular-nums"
                    >
                      <span>{formatTimestamp(nextSeg.start_time)}</span>
                      <span class="opacity-50">–</span>
                      <span>{formatTimestamp(nextSeg.end_time)}</span>
                    </div>
                    <span
                      class="text-[9pt] font-semibold text-gray-400 dark:text-gray-500 truncate max-w-[8rem]"
                      >• {nextSeg.speaker || 'Unknown'}</span
                    >
                  </div>
                  <div class="grid grid-cols-1 md:grid-cols-2 gap-x-4">
                    <div
                      class="text-[10pt] text-gray-400 dark:text-gray-500 line-clamp-1 text-left italic"
                    >
                      {getPreviewText(nextSeg)}
                    </div>
                    {#if $transcriptStore.isDualModeActive && secondarySegments[currentIndex + 1]}
                      <div
                        class="hidden md:block text-[10pt] text-gray-400 dark:text-gray-500 line-clamp-1 text-left italic border-l border-gray-200 dark:border-gray-800 pl-4"
                      >
                        {getPreviewText(secondarySegments[currentIndex + 1])}
                      </div>
                    {/if}
                  </div>
                </button>
              {:else}
                <div
                  class="segment-card segment-card-next opacity-20 cursor-default border-gray-200/50 dark:border-gray-800/50 flex items-center justify-center text-[10pt] italic text-gray-400 dark:text-gray-500"
                  title="No Next Segment"
                >
                  No Next Segment
                </div>
              {/if}
            </div>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>

<style lang="postcss">
  .input-field {
    @apply text-center bg-transparent border-0 p-0 text-gray-800 dark:text-gray-400;
    font-size: 12pt;
  }
  .input-field:not(:disabled) {
    @apply bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 text-gray-900 dark:text-gray-200 rounded;
  }

  .picker-wheel-container {
    @apply relative;
  }

  .segment-card {
    @apply w-full p-3 rounded-lg border border-gray-200/50 dark:border-gray-800/50 hover:border-gray-200 dark:hover:border-gray-800 hover:bg-gray-50/50 dark:hover:bg-gray-800/50 transition-all duration-200 text-left relative overflow-hidden flex-shrink-0 flex flex-col justify-center;
    cursor: pointer;
    height: 68px;
  }

  .segment-card-prev {
    @apply scale-[0.98] opacity-80 hover:opacity-100 hover:scale-100;
    transform-origin: bottom center;
  }

  .segment-card-next {
    @apply scale-[0.98] opacity-80 hover:opacity-100 hover:scale-100;
    transform-origin: top center;
  }

  .lexical-editor-wrapper-style {
    display: flex;
    flex-direction: column;
    @apply rounded;
    overflow: visible;
  }
  .lexical-editor-wrapper-style:not(.is-disabled) {
    @apply border border-gray-300 dark:border-gray-700 bg-white dark:bg-transparent;
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
    @apply text-gray-900 dark:text-gray-200;
  }
  .lexical-editor-wrapper-style.is-disabled :global(.lexical-content) {
    @apply text-gray-800 dark:text-gray-200 cursor-not-allowed;
  }

  .lexical-editor-wrapper-style :global(.lexical-content p) {
    @apply mt-0 mb-0;
  }

  :global(html.dark .input-field:disabled) {
    background-color: #262626;
    border: 1px solid #171717;
    color: white;
  }
  :global(html.dark .input-field:not(:disabled)) {
    background-color: #171717;
    border: 1px solid #262626;
    color: white;
  }

  /* Read Mode */
  :global(html.dark .editable-transcript-wrapper.read-mode .lexical-editor-wrapper-style) {
    border: 1px solid #171717 !important;
  }
  :global(html.dark .editable-transcript-wrapper.read-mode .lexical-wrapper) {
    background-color: #262626 !important;
  }
  :global(html.dark .editable-transcript-wrapper.read-mode .lexical-content) {
    color: white !important;
  }

  /* Edit Mode */
  :global(html.dark .editable-transcript-wrapper.edit-mode .lexical-editor-wrapper-style) {
    border: 1px solid #262626 !important;
  }
  :global(html.dark .editable-transcript-wrapper.edit-mode .lexical-wrapper) {
    background-color: #171717 !important;
  }
  :global(html.dark .editable-transcript-wrapper.edit-mode .lexical-content) {
    color: white !important;
  }

  :global(html.dark .editable-transcript-wrapper.read-mode .ui-select:disabled) {
    background-color: #262626;
    border-color: #171717;
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

  :global(
    html:not(.dark) .editable-transcript-wrapper.read-mode .lexical-editor-wrapper-style.is-disabled
  ) {
    background-color: #f9fafb; /* bg-gray-50 */
    border: 1px solid #d1d5db !important; /* border-gray-300 */
  }

  :global(
    html:not(.dark)
      .editable-transcript-wrapper.read-mode
      .lexical-editor-wrapper-style.is-disabled
      .lexical-content
  ) {
    color: #111827 !important; /* text-gray-900 */
  }
</style>
