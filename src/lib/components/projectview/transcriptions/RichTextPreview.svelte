<!-- src/lib/components/projectview/transcriptions/RichTextPreview.svelte -->
<script>
	import { project, prepareDocumentView } from '$lib/stores/projectStore.js';
	import { transcriptStore, updatePlayerCurrentSegmentIndex, switchTranscript } from '$lib/stores/transcriptStore.js';
	import { createEventDispatcher, tick, onMount, onDestroy } from 'svelte';
	import { basename } from '@tauri-apps/api/path';
	import { confirm, message } from '@tauri-apps/plugin-dialog';
	import { languageOptions } from '$lib/constants/transcriptionOptions.js';
	import { convertAndSaveTranscriptAsDoc } from '$lib/services/projectService.js';
	import { ExtendedTextNode } from '$lib/nodes/ExtendedTextNode.js';
    import { get } from 'svelte/store';
    import { listen } from '@tauri-apps/api/event'; // Added for Tauri event listener
    import { activeLayout } from '$lib/stores/layoutStore.js';
	import { DOCX_LAYOUT_OPTIONS } from '$lib/constants/exportLayouts.js';

    // Virtualization state
    let scrollTop = 0;
    let containerHeight = 0;
    const ESTIMATED_SEGMENT_HEIGHT = 70; // Adjust as needed, or measure dynamically
    const OVERSCAN_COUNT = 5; // Number of items to render above/below viewport

    let showTranscriptDropdown = false;
    let transcriptDropdownButtonRef;
    let transcriptDropdownMenuRef;

    let refreshKey = 0; // Key to force re-evaluation of transcript list
    let unlistenJobComplete = null; // To store the unlisten function

	function getLanguageLabel(langCode) {
		if (!langCode || langCode.toLowerCase() === 'original') return 'Original';
		const option = languageOptions.find(opt => opt.value === langCode);
		return option ? option.label : langCode; // Fallback to code if not found
	}

    // Function to close dropdown when clicking outside
    function handleClickOutsideTranscriptDropdown(event) {
        if (showTranscriptDropdown && transcriptDropdownMenuRef && !transcriptDropdownMenuRef.contains(event.target) && transcriptDropdownButtonRef && !transcriptDropdownButtonRef.contains(event.target)) {
            showTranscriptDropdown = false;
        }
    }

    onMount(async () => {
        document.addEventListener('click', handleClickOutsideTranscriptDropdown, true);

        unlistenJobComplete = await listen('custom_transcription_job_completed', (event) => {
            if (event.payload && event.payload.status === 'done') {
                const currentSelectedMedia = get(transcriptStore).selectedMediaFile;
                if (currentSelectedMedia && event.payload.jobFinishedPath === currentSelectedMedia.path) {
                    console.log('[RichTextPreview] Relevant transcription job completed. Incrementing refreshKey.');
                    refreshKey++;
                }
            }
        });
        isMounted = true; // For scroll logic
        if (previewScrollContainerRef) { // For scroll logic
            containerHeight = previewScrollContainerRef.clientHeight;
            scrollTop = previewScrollContainerRef.scrollTop;
        }
    });

    onDestroy(() => {
        document.removeEventListener('click', handleClickOutsideTranscriptDropdown, true);
        if (unlistenJobComplete) {
            unlistenJobComplete();
        }
        cancelAnimation(); // For scroll logic
    });

    
    

	import { derived } from 'svelte/store';

	const displayedTranscripts = derived(transcriptStore, ($transcriptStore) => {
		const transcripts = $transcriptStore.selectedMediaFile?.associated_transcripts;
		if (!transcripts || transcripts.length === 0) return [];

		const withLabels = transcripts.map(t => {
			const langLabel = getLanguageLabel(t.language_code || 'original');
			let fileName = t.name;
			if (!fileName && t.path) {
				try {
					const pathParts = t.path.split(/[\\/]/);
					fileName = pathParts[pathParts.length - 1];
					if (fileName.toLowerCase().endsWith('.json')) {
						fileName = fileName.substring(0, fileName.length - 5);
					}
				} catch (e) {
					console.error("Error extracting filename from path:", e);
					fileName = '';
				}
			}
			const fileNamePart = fileName ? ` (${fileName})` : '';
			const displayLabel = `${langLabel}${fileNamePart}`;
			return { ...t, displayLabel };
		});

		return withLabels.sort((a, b) => a.displayLabel.localeCompare(b.displayLabel));
	});

    import { createHeadlessEditor } from '@lexical/headless';
    import { $generateHtmlFromNodes as generateHtmlFromNodes } from '@lexical/html';

    import { RootNode, ParagraphNode, TextNode, LineBreakNode, $getRoot as lexicalGetRoot, $parseSerializedNode as lexicalParseSerializedNode } from 'lexical';
    import { HeadingNode, QuoteNode } from '@lexical/rich-text';
    import { ListNode, ListItemNode } from '@lexical/list';
    import { TableNode, TableRowNode, TableCellNode } from '@lexical/table';
    import { LinkNode } from '@lexical/link';

    // headless editor for HTML serialization
    const htmlEditor = createHeadlessEditor({
      namespace: 'RichTextHtmlGen',
      theme: {}, // added to prevent config.theme error
      nodes: [
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
      ]
    });

    // helper to detect Lexical JSON (accepts string or object)
    function isLexicalJson(value) {
      if (value === null || value === undefined) return false;

      let data;
      if (typeof value === 'string') {
        try {
          data = JSON.parse(value);
        } catch {
          return false;               // invalid JSON string
        }
      } else if (typeof value === 'object') {
        data = value;                 // already parsed
      } else {
        return false;                 // unsupported type
      }

      return !!(data && data.root && data.root.type === 'root');
    }

    // helper to convert Lexical JSON (string | object) to HTML
    function lexicalJsonToHtml(json) {
      const jsonStr = typeof json === 'string' ? json : JSON.stringify(json);
      let html = '';

      try {
        const parsedJson = JSON.parse(jsonStr); // Parse the JSON string into an object

        if (parsedJson && parsedJson.root && Array.isArray(parsedJson.root.children)) {
          const serializedNodes = parsedJson.root.children;

          htmlEditor.update(() => {
            const editorRoot = lexicalGetRoot(); // Get the root of the htmlEditor
            editorRoot.clear();

            const nodesToAppend = [];
            const validSerializedNodes = serializedNodes.filter(Boolean);

            for (const serializedNodeObj of validSerializedNodes) {
              // The console.log for diagnostics can be kept or removed. For this fix, let's keep it for now.
              // console.log('[RichTextPreview] Processing serializedNodeObj:', JSON.stringify(serializedNodeObj)); // Removed verbose log

              if (serializedNodeObj.type === 'root' && serializedNodeObj.children && Array.isArray(serializedNodeObj.children)) {
                // If the serializedNodeObj is a RootNode itself, process its children
                for (const childOfNestedRoot of serializedNodeObj.children) {
                  if (childOfNestedRoot) { // Ensure child is not null/undefined
                    try {
                        nodesToAppend.push(lexicalParseSerializedNode(childOfNestedRoot));
                    } catch (parseErr) {
                        console.error('[RichTextPreview] Error parsing childOfNestedRoot:', childOfNestedRoot, parseErr);
                        // Optionally add a placeholder or skip if a child fails
                    }
                  }
                }
              } else {
                // Otherwise, parse the serializedNodeObj directly (assuming it's a Paragraph, List, etc.)
                try {
                    nodesToAppend.push(lexicalParseSerializedNode(serializedNodeObj));
                } catch (parseErr) {
                    console.error('[RichTextPreview] Error parsing serializedNodeObj:', serializedNodeObj, parseErr);
                    // Optionally add a placeholder or skip
                }
              }
            }

            // Ensure nodesToAppend is not empty for a valid Lexical state before appending
            if (nodesToAppend.length === 0) {
                try {
                    const defaultParagraphNode = lexicalParseSerializedNode({
                        type: 'paragraph',
                        version: 1,
                        children: [],
                        direction: null,
                        format: '',
                        indent: 0
                    });
                    nodesToAppend.push(defaultParagraphNode);
                    // console.log('[RichTextPreview] nodesToAppend was empty; added default paragraph via lexicalParseSerializedNode.'); // Removed verbose log
                } catch (defaultNodeErr) {
                    console.error('[RichTextPreview] Error creating default paragraph node:', defaultNodeErr);
                }
            }

            editorRoot.append(...nodesToAppend);
            html = generateHtmlFromNodes(htmlEditor, null);
          }, { discrete: true });
        } else {
          console.warn('[RichTextPreview] lexicalJsonToHtml: parsedJson or parsedJson.root.children is invalid. Rendering empty.', parsedJson);
          html = '';
        }
      } catch (e) {
        console.error('[RichTextPreview] lexicalJsonToHtml: Error processing JSON string. jsonStr:', jsonStr.substring(0, 500), 'Error:', e);
        html = '<!-- error rendering segment content -->';
      }
      return html;
    }

	export let previewEditMode = false;
	const dispatch = createEventDispatcher();

	const defaultEmptyJson = JSON.stringify({
	  root: {
	    children: [{ children: [], direction: null, format: '', indent: 0, type: 'paragraph', version: 1 }],
	    direction: null, format: '', indent: 0, type: 'root', version: 1
	  }
	});

	/* ---------------- helpers ---------------- */
	function formatTimestamp(seconds) {
		if (typeof seconds !== 'number' || isNaN(seconds) || seconds < 0) return '00:00.000';
		const totalMs = Math.round(seconds * 1000); const ms = String(totalMs % 1000).padStart(3, '0'); const totalS = Math.floor(totalMs / 1000); const sec = String(totalS % 60).padStart(2, '0'); const min = String(Math.floor(totalS / 60)).padStart(2, '0');
		return `${min}:${sec}.${ms}`;
	}

	function extractPlainTextForPreview(inputString) { if (!inputString || typeof inputString !== 'string') return '[empty]'; if (isLexicalJson(inputString)) { console.warn("[RichTextPreview] extractPlainTextForPreview called with JSON string, rendering placeholder."); return '[Error: Invalid data format - Expected plain text or HTML]'; } try { const parser = new DOMParser(); const doc = parser.parseFromString(inputString, 'text/html'); if (doc.body.childNodes.length === 1 && doc.body.firstChild.nodeType === Node.TEXT_NODE) { return doc.body.textContent || '[empty]'; } return doc.body.textContent || inputString || '[empty]'; } catch (e) { console.error("[RichTextPreview] Error parsing string in extractPlainTextForPreview:", e); return inputString || '[empty]'; } }

	/* ---------------- build segment data for rendering ---------------- */
	let allSegmentsData = []; // Stores raw or minimally processed segment data
	let canUndo = false;
	let canRedo = false;

	$: {
	  const segs = $transcriptStore.segments || [];
	  canUndo = ($transcriptStore.transcriptUndoStack?.length || 0) > 0;
	  canRedo = ($transcriptStore.transcriptRedoStack?.length || 0) > 0;
      // Minimal initial mapping, just to have an array of the correct length with original data
	  allSegmentsData = segs.map((seg, segIdx) => ({
	      segmentIndex: segIdx,
          originalSegment: seg
          // Add any other absolutely essential lightweight data needed by virtualization itself, if any.
          // For now, originalSegment and segmentIndex should be enough.
	  }));
	}

    // --- Virtualization Calculations ---
    let visibleStartIndex = 0;
    let visibleEndIndex = 0;
    let paddingTop = 0;
    let paddingBottom = 0;
    let visibleSegments = []; // This will store fully processed segments for rendering

    $: if (allSegmentsData) {
        if (previewScrollContainerRef && allSegmentsData.length > 0) {
            const totalItems = allSegmentsData.length;
            visibleStartIndex = Math.max(0, Math.floor(scrollTop / ESTIMATED_SEGMENT_HEIGHT) - OVERSCAN_COUNT);
            visibleEndIndex = Math.min(totalItems -1 , Math.ceil((scrollTop + containerHeight) / ESTIMATED_SEGMENT_HEIGHT) + OVERSCAN_COUNT);

            paddingTop = visibleStartIndex * ESTIMATED_SEGMENT_HEIGHT;
            paddingBottom = (totalItems - 1 - visibleEndIndex) * ESTIMATED_SEGMENT_HEIGHT;

            visibleSegments = allSegmentsData.slice(visibleStartIndex, visibleEndIndex + 1).map(item => {
                const seg = item.originalSegment;
                const segIdx = item.segmentIndex;
                const rawContent = seg.text;
                let contentForParsing = rawContent;
                try {
                  const parsed = typeof rawContent === 'string' ? JSON.parse(rawContent) : rawContent;
                  if (parsed && !parsed.root && Array.isArray(parsed.children)) {
                    contentForParsing = JSON.stringify({
                      root: { type: 'root', version: 1, format: '', indent: 0, direction: null, children: [parsed] }
                    });
                  }
                } catch (e) { /* Not valid JSON */ }

                const isJson = isLexicalJson(contentForParsing);
                let plainTextForDisplay = '';
                if (!isJson) {
                    plainTextForDisplay = extractPlainTextForPreview(rawContent);
                }

                return {
                  segmentIndex: segIdx,
                  startTime: formatTimestamp(seg.start_time),
                  endTime: formatTimestamp(seg.end_time),
                  rawStart: seg.start_time,
                  rawEnd: seg.end_time,
                  speaker: seg.speaker || 'Unknown',
                  isJsonContent: isJson,
                  html: isJson ? lexicalJsonToHtml(contentForParsing) : `<div>${plainTextForDisplay}</div>`,
                  plainText: plainTextForDisplay
                };
            });
        } else {
            visibleStartIndex = 0;
            visibleEndIndex = 0;
            paddingTop = 0;
            paddingBottom = 0;
            visibleSegments = [];
        }
    }

    let lastSeenTranscriptPath = null;
    $: {
        const currentPath = $transcriptStore.activeTranscript?.path;
        if (currentPath && currentPath !== lastSeenTranscriptPath) {
            lastSeenTranscriptPath = currentPath;
            // Wait for the DOM to update before resetting scroll
            tick().then(() => {
                if (previewScrollContainerRef) {
                    previewScrollContainerRef.scrollTop = 0;
                }
            });
            // Reset internal state immediately
            scrollTop = 0;
        } else if (!currentPath && lastSeenTranscriptPath !== null) {
            lastSeenTranscriptPath = null;
        }
    }

    // --- Highlight and Scroll Logic ---
    let previewScrollContainerRef; $: activeSegmentIndex = $transcriptStore.player?.currentSegmentIndex ?? -1;
    let karaokeScrollIndex = -1; // Tracks the index for which karaoke scroll was last triggered.
    let scrollAnimationId = null; // ID for the requestAnimationFrame loop
    let isProgrammaticScroll = false;
    let expectedScrollTop = -1; // The scroll position our animation expects to be at.

    // Scroll and highlight logic
    $: if (activeSegmentIndex !== -1 && isMounted && previewScrollContainerRef && activeSegmentIndex !== karaokeScrollIndex) {
        tick().then(() => {
            if (!previewScrollContainerRef) return;

            const container = previewScrollContainerRef;
            const currentContainerHeight = container.clientHeight;
            const currentDomScrollTop = container.scrollTop;
            const maxScrollTop = container.scrollHeight - currentContainerHeight;

            const itemTop = activeSegmentIndex * ESTIMATED_SEGMENT_HEIGHT;
            const itemBottom = itemTop + ESTIMATED_SEGMENT_HEIGHT;

            const shouldScroll = $transcriptStore.player.isPlaying || (activeSegmentIndex !== karaokeScrollIndex);

            if (shouldScroll) {
                const viewportTop = currentDomScrollTop;
                const viewportBottom = viewportTop + currentContainerHeight;

                const isScrollingDown = activeSegmentIndex > karaokeScrollIndex && karaokeScrollIndex !== -1;
                const isScrollingUp = activeSegmentIndex < karaokeScrollIndex && karaokeScrollIndex !== -1;

                // Sweet spot for incremental scrolling (middle 50% of the screen)
                const sweetSpotTop = viewportTop + currentContainerHeight * 0.25;
                const sweetSpotBottom = viewportTop + currentContainerHeight * 0.75;
                const isItemInSweetSpot = itemTop >= sweetSpotTop && itemBottom <= sweetSpotBottom;

                // Condition for incremental scroll
                if ($transcriptStore.player.isPlaying && isScrollingDown && isItemInSweetSpot && (maxScrollTop - currentDomScrollTop > 1)) {
                    const targetScrollTop = Math.min(currentDomScrollTop + ESTIMATED_SEGMENT_HEIGHT, maxScrollTop);
                    manualSmoothScroll(targetScrollTop);
                } else {
                    // Fallback to centering logic for seeking, scrolling up, or when the item is outside the sweet spot.
                    const scrollThreshold = 2 * ESTIMATED_SEGMENT_HEIGHT;
                    const effectiveViewportTop = viewportTop + (isScrollingUp ? scrollThreshold : 0);
                    const effectiveViewportBottom = viewportBottom - (isScrollingDown ? scrollThreshold : 0);
                    const isItemInsideEffectiveViewport = itemTop >= effectiveViewportTop && itemBottom <= effectiveViewportBottom;

                    if (!isItemInsideEffectiveViewport) {
                        let targetDomScrollTop = itemTop - (currentContainerHeight / 2) + (ESTIMATED_SEGMENT_HEIGHT);
                        targetDomScrollTop = Math.max(0, Math.min(targetDomScrollTop, maxScrollTop));

                        if (Math.abs(targetDomScrollTop - currentDomScrollTop) > 1) {
                            manualSmoothScroll(targetDomScrollTop);
                        }
                    }
                }
            }
            karaokeScrollIndex = activeSegmentIndex;
        });
    }

    function manualSmoothScroll(targetY, duration = 400) {
        if (!previewScrollContainerRef) return;
        if (scrollAnimationId) cancelAnimationFrame(scrollAnimationId);

        isProgrammaticScroll = true;
        const startY = previewScrollContainerRef.scrollTop;
        const distance = targetY - startY;
        let startTime = null;

        function animation(currentTime) {
            if (startTime === null) startTime = currentTime;
            const timeElapsed = currentTime - startTime;
            const progress = Math.min(timeElapsed / duration, 1);
            const ease = progress < 0.5 ? 2 * progress * progress : -1 + (4 - 2 * progress) * progress;
            const newScrollTop = startY + distance * ease;

            expectedScrollTop = newScrollTop;
            previewScrollContainerRef.scrollTop = newScrollTop;

            if (timeElapsed < duration) {
                scrollAnimationId = requestAnimationFrame(animation);
            } else {
                expectedScrollTop = targetY;
                previewScrollContainerRef.scrollTop = targetY;
                isProgrammaticScroll = false;
                scrollAnimationId = null;
            }
        }

        scrollAnimationId = requestAnimationFrame(animation);
    }

    let isMounted = false;

    function cancelAnimation() {
        if (scrollAnimationId) {
            cancelAnimationFrame(scrollAnimationId);
            scrollAnimationId = null;
        }
        isProgrammaticScroll = false;
        if ($transcriptStore.player.isPlaying) {
            karaokeScrollIndex = -1;
        }
    }

    onMount(() => {
        isMounted = true;
        if (previewScrollContainerRef) {
            containerHeight = previewScrollContainerRef.clientHeight;
            scrollTop = previewScrollContainerRef.scrollTop;
        }
    });

    onDestroy(() => {
        cancelAnimation();
        document.removeEventListener('click', handleClickOutsideTranscriptDropdown, true);
    });

    function handleScroll() {
        if (!previewScrollContainerRef) return;
        const currentScroll = previewScrollContainerRef.scrollTop;

        if (isProgrammaticScroll) {
            if (Math.abs(currentScroll - expectedScrollTop) > 2) {
                cancelAnimation();
                scrollTop = currentScroll;
            } else {
                scrollTop = currentScroll;
            }
        } else {
            scrollTop = currentScroll;
            if ($transcriptStore.player.isPlaying) {
                karaokeScrollIndex = -1;
            }
        }
    }

	/* ---------------- interactions ---------------- */
	function handleSegmentClick(idx) {
        // idx here is the original segmentIndex from processedSegments, not the index in visibleSegments
        if (!previewEditMode) {
            dispatch('segmentclick', idx);
        } else {
        }
    }
	function handleToggleEdit() { dispatch('toggleedit'); }

	async function handleAddToDocumentsClick() {
		const confirmationMessage = `This will create a copy of the current transcript as a new document.\n\nThis document will not sync with the media player.`;
		const userConfirmed = await confirm(confirmationMessage, {
			title: 'Export to Documents?',
			type: 'info',
			okLabel: 'Yes, Create Document',
			cancelLabel: 'Cancel'
		});

		if (userConfirmed) {
            try {
                const newDocPath = await convertAndSaveTranscriptAsDoc();
                if (newDocPath) {
                    await message(`Transcript copied to Documents:\n${newDocPath.split(/[\\/]/).pop()}`, {title: "Document Created", type: "info"});
                    dispatch('requestopentab', { tabName: 'data', loadNotePath: newDocPath });
                } else {
                     console.error("[RichTextPreview] Document saving process did not return a path.");
                     await message("Failed to create document file: The process completed but did not provide a file path.", {title: "Error", type: "error"});
                }
            } catch (error) {
                console.error("[RichTextPreview] Error during document creation process:", error);
                const errorMsg = error instanceof Error ? error.message : String(error);
                await message(`Failed to create document file: ${errorMsg}`, {title: "Error", type: "error"});
            }
        } else {
        }
    }

    async function handleDeleteSegment(idx) {
        if (!previewEditMode) return;
        const storeSegments = get(transcriptStore).segments;
        const segmentToDelete = storeSegments[idx];
        if (!segmentToDelete) {
            console.error(`[RichTextPreview] Delete requested for invalid index: ${idx}`); return;
        }
        // Simplified confirmation message as full plainText might not be readily processed for non-visible items
        const textPreview = segmentToDelete.text ? (isLexicalJson(segmentToDelete.text) ? "[Rich Content]" : String(segmentToDelete.text).substring(0,50) + "...") : "[empty]";
        const confirmation = await confirm(
            `Are you sure you want to delete segment ${idx + 1}?\n\n[${formatTimestamp(segmentToDelete.start_time)} - ${formatTimestamp(segmentToDelete.end_time)}]\n\"${textPreview}\"\n\nThis action can be undone until you save the transcript.`,
            { title: 'Confirm Delete Segment', type: 'warning', okLabel: 'Delete Segment', cancelLabel: 'Cancel' }
        );
        if (confirmation) {
            dispatch('deletetranscriptsegment', idx);
        } else {
        }
    }
    function handleUndo() { if (canUndo) { dispatch('undo'); } }
    function handleRedo() { if (canRedo) { dispatch('redo'); } }
    async function handleInsertNewSegment(index) { if (!previewEditMode) return; const MIN_GAP_SECONDS = 1.0; const TIME_TOLERANCE = 0.001; const currentSegments = get(transcriptStore).segments; const mediaDuration = get(transcriptStore).player.duration; let prevEndTime = 0.0; let nextStartTime = mediaDuration; if (index > 0) { prevEndTime = currentSegments[index - 1]?.end_time ?? 0.0; } if (index < currentSegments.length) { nextStartTime = currentSegments[index]?.start_time ?? mediaDuration; } const gap = nextStartTime - prevEndTime; if (gap < MIN_GAP_SECONDS + (2 * TIME_TOLERANCE)) { await message(`Cannot insert segment here. The gap between segments must be at least ${MIN_GAP_SECONDS.toFixed(1)} seconds. Current gap is ${gap.toFixed(3)} seconds.`, { title: 'Cannot Insert Segment', type: 'info' }); return; } let newStartTime = prevEndTime + TIME_TOLERANCE; let newEndTime = nextStartTime - TIME_TOLERANCE; newStartTime = Math.max(0, newStartTime); newEndTime = Math.min(mediaDuration, newEndTime); newEndTime = Math.max(newStartTime, newEndTime); if (newEndTime > newStartTime) { dispatch('insertnewsegment', { index, startTime: newStartTime, endTime: newEndTime }); } else { console.error(`[RichTextPreview] Calculated invalid times for gap fill insertion: start=${newStartTime.toFixed(3)}, end=${newEndTime.toFixed(3)}`); await message('Could not calculate valid timestamps for the new segment in the available gap.', { title: 'Insertion Error', type: 'error' }); } }

	// --- SVG Icons (Unchanged) ---
	const EDIT_ICON = `<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-6"> <path stroke-linecap="round" stroke-linejoin="round" d="m16.862 4.487 1.687-1.688a1.875 1.875 0 1 1 2.652 2.652L10.582 16.07a4.5 4.5 0 0 1-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 0 1 1.13-1.897l8.932-8.931Zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0 1 15.75 21H5.25A2.25 2.25 0 0 1 3 18.75V8.25A2.25 2.25 0 0 1 5.25 6H10" /> </svg>`;
	const SAVE_ICON = `<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-6"> <path stroke-linecap="round" stroke-linejoin="round" d="M10.125 2.25h-4.5c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125v-9M10.125 2.25h.375a9 9 0 0 1 9 9v.375M10.125 2.25A3.375 3.375 0 0 1 13.5 5.625v1.5c0 .621.504 1.125 1.125 1.125h1.5a3.375 3.375 0 0 1 3.375 3.375M9 15l2.25 2.25L15 12" /> </svg>`;
	const DOCUMENT_ICON = `
  <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 16 16" class="w-5 h-5">
    <path d="M8.5 11.5a.5.5 0 0 1-1 0V7.707L6.354 8.854a.5.5 0 1 1-.708-.708l2-2a.5.5 0 0 1 .708 0l2 2a.5.5 0 0 1-.708.708L8.5 7.707z"/>
  <path d="M14 14V4.5L9.5 0H4a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2M9.5 3A1.5 1.5 0 0 0 11 4.5h2V14a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1h5.5z"/>
</svg>
`;
    const UNDO_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-arrow-counterclockwise" viewBox="0 0 16 16"> <path fill-rule="evenodd" d="M8 3a5 5 0 1 1-4.546 2.914.5.5 0 0 0-.908-.417A6 6 0 1 0 8 2z"/> <path d="M8 4.466V.534a.25.25 0 0 0-.41-.192L5.23 2.308a.25.25 0 0 0 0 .384l2.36 1.966A.25.25 0 0 0 8 4.466"/> </svg>`;
    const REDO_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-arrow-clockwise" viewBox="0 0 16 16"> <path fill-rule="evenodd" d="M8 3a5 5 0 1 0 4.546 2.914.5.5 0 0 1 .908-.417A6 6 0 1 1 8 2z"/> <path d="M8 4.466V.534a.25.25 0 0 1 .41-.192l2.36 1.966c.12.1.12.284 0 .384L8.41 4.658A.25.25 0 0 1 8 4.466"/> </svg>`;
    const INSERT_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-plus-square-fill" viewBox="0 0 16 16"> <path d="M2 0a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V2a2 2 0 0 0-2-2zm6.5 4.5v3h3a.5.5 0 0 1 0 1h-3v3a.5.5 0 0 1-1 0v-3h-3a.5.5 0 0 1 0-1h3v-3a.5.5 0 0 1 1 0"/> </svg>`;
	const DELETE_ICON = `<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-6"> <path stroke-linecap="round" stroke-linejoin="round" d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" /> </svg>`;
	const MENU_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="size-6" viewBox="0 0 16 16"> <path d="M9.5 13a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0m0-5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0m0-5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0"/> </svg>`;
	let showExportMenu = false;

    // --- Layout specific visibility ---
    let showSegmentNumberCol, showTimestampCol, showSpeakerCol, showTextCol;

    $: {
        const layoutKey = $activeLayout;
        // Default all to true, then selectively hide based on layoutKey
        showSegmentNumberCol = true;
        showTimestampCol = true;
        showSpeakerCol = true;
        showTextCol = true;

        // These keys come from DOCX_LAYOUT_OPTIONS in exportLayouts.js
        if (layoutKey === 'Layout3') { // Timestamped Paragraph
            showSegmentNumberCol = false; // No dedicated number column, implied or part of text
            // Timestamp and Speaker are combined visually, but data exists
        } else if (layoutKey === 'Layout4') { // Speaker & Text
            showSegmentNumberCol = false;
            showTimestampCol = false;
        } else if (layoutKey === 'Layout5') { // Plain Text
            showSegmentNumberCol = false;
            showTimestampCol = false;
            showSpeakerCol = false;
        }
        // Layout1 (Detailed Table) and Layout2 (Segment Block) show all by default.
    }

</script>

<div
  class="p-3 h-full flex flex-col text-base text-gray-900 dark:text-gray-200"
  style="font-family: Arial, Helvetica, sans-serif; font-size: 12pt; line-height: 1.5;"
>
    <h3 class="font-semibold mb-2 text-sm text-gray-700 dark:text-gray-300 border-b border-gray-300 dark:border-gray-600 pb-1 flex items-center justify-between w-full">
        <div class="flex items-center"> <!-- leftAndMiddleControlsGroup -->
            <!-- Transcript Dropdown using native select -->
            {#if $transcriptStore.selectedMediaFile}
                <div class="relative inline-block">
                    <select
                        class="block w-auto rounded-md border border-gray-300 dark:border-gray-600 shadow-sm px-3 py-1 bg-white dark:bg-gray-700 text-xs font-medium text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-600 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-gray-100 dark:focus:ring-offset-gray-700 focus:ring-indigo-500
                               max-w-[150px] sm:max-w-[200px] md:max-w-[250px] truncate appearance-none pr-8"
                        value={$transcriptStore.activeTranscript?.path || ''}
                        on:change={(e) => switchTranscript(e.target.value)}
                    >
                        {#if $displayedTranscripts.length === 0}
                            <option value="" disabled>No Transcripts</option>
                        {:else}
                            {#each $displayedTranscripts as transcript (transcript.path)}
                                <option value={transcript.path}>
                                    {transcript.displayLabel}
                                </option>
                            {/each}
                        {/if}
                    </select>
                    <!-- Custom Chevron Icon -->
                    <div class="pointer-events-none absolute inset-y-0 right-0 flex items-center px-2 text-gray-700 dark:text-gray-200">
                        <svg class="h-4 w-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" aria-hidden="true">
                            <path fill-rule="evenodd" d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z" clip-rule="evenodd" />
                        </svg>
                    </div>
                </div>
            {:else}
                <span class="px-3 py-1 text-xs text-gray-500 dark:text-gray-400 italic">No Media Selected</span>
            {/if}

            <!-- Edit/Save/Undo/Redo buttons HTML block starts here -->
            {#if allSegmentsData.length || previewEditMode}
                <button on:click={handleToggleEdit} class="btn-icon ml-2 text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200" title={previewEditMode ? 'Save Transcript (Ctrl+S)' : 'Edit Transcript (Ctrl+E)'} aria-label={previewEditMode ? 'Save Transcript' : 'Edit Transcript'}> {@html previewEditMode ? SAVE_ICON : EDIT_ICON} </button>
                {#if previewEditMode}
                  <button class="btn-icon ml-2" class:text-gray-400={!canUndo} class:dark:text-gray-500={!canUndo} class:text-gray-600={canUndo} class:hover:text-gray-800={canUndo} class:dark:text-gray-400={canUndo} class:dark:hover:text-gray-200={canUndo} on:click={handleUndo} title="Undo (Ctrl+Z)" aria-label="Undo Transcript Change" disabled={!canUndo}> {@html UNDO_ICON} </button>
                  <button class="btn-icon ml-2" class:text-gray-400={!canRedo} class:dark:text-gray-500={!canRedo} class:text-gray-600={canRedo} class:hover:text-gray-800={canRedo} class:dark:text-gray-400={canRedo} class:dark:hover:text-gray-200={canRedo} on:click={handleRedo} title="Redo (Ctrl+Y)" aria-label="Redo Transcript Change" disabled={!canRedo}> {@html REDO_ICON} </button>
                {/if}
            {/if}
            <!-- Edit/Save/Undo/Redo buttons HTML block ends here -->
        </div>

        <div class="flex items-center"> <!-- This div now only effectively holds the "More options" menu -->
            {#if allSegmentsData.length > 0}
              <div class="relative inline-block ml-2">
                <button
                  on:click={() => showExportMenu = !showExportMenu}
                  class="btn-icon text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200"
                  title="More options"
                  aria-label="More options"
                >
                  {@html MENU_ICON}
                </button>
                {#if showExportMenu}
                  <div class="fixed inset-0 z-0" on:click={() => showExportMenu = false}></div>
                  <div class="absolute right-0 mt-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-md shadow-xl py-1 text-xs min-w-max whitespace-nowrap z-10">
                    <button
                      on:click={() => { showExportMenu = false; handleAddToDocumentsClick(); }}
                      class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200"
                    >
                      Export to Documents
                    </button>
                  </div>
                {/if}
              </div>
            {/if}
        </div>
    </h3>

    {#if allSegmentsData.length === 0}
        <div class="flex-grow flex items-center justify-center text-gray-400">
            {#if previewEditMode}
                Transcript empty. Click Insert button to add a segment.
                <div class="flex justify-center insert-button-wrapper absolute top-1/2 left-1/2 -translate-x-1/2 translate-y-4"> <button class="btn-icon text-green-400 hover:text-green-600 dark:hover:text-green-300" on:click={() => handleInsertNewSegment(0)} title="Insert New Segment" aria-label="Insert New Segment"> {@html INSERT_ICON} </button> </div>
            {:else} No transcript data to preview. {/if}
        </div>
    {:else}
        <div
            bind:this={previewScrollContainerRef}
            class="flex-grow overflow-y-auto space-y-1 pr-1 relative overscroll-y-contain"
            on:scroll={handleScroll}
            on:wheel={cancelAnimation}
            on:touchstart={cancelAnimation}
            bind:clientHeight={containerHeight}
        >
            <div style="height: {paddingTop}px;"></div>
            {#if previewEditMode && visibleStartIndex === 0}
              <div class="flex justify-center insert-button-wrapper"> <button class="btn-icon text-green-400 hover:text-green-600 dark:hover:text-green-300" on:click={() => handleInsertNewSegment(0)} title="Insert New Segment" aria-label="Insert New Segment"> {@html INSERT_ICON} </button> </div>
            {/if}
            {#each visibleSegments as seg (seg.segmentIndex)}
                <div
                    id={`segment-${seg.segmentIndex}`}
                    class:segment-block={true}
                    style="min-height: {ESTIMATED_SEGMENT_HEIGHT}px;"
                    class="p-2 border rounded-lg shadow-sm transition-colors duration-150 ease-in-out dark:border-gray-700 flex items-start gap-x-2"
                    class:segment-active={seg.segmentIndex === activeSegmentIndex}
                    class:border-blue-400={seg.segmentIndex === activeSegmentIndex}
                    class:bg-blue-100={seg.segmentIndex === activeSegmentIndex}
                    class:dark:bg-blue-900={seg.segmentIndex === activeSegmentIndex}
                    class:dark:border-blue-600={seg.segmentIndex === activeSegmentIndex}
                    class:border-gray-200={seg.segmentIndex !== activeSegmentIndex}
                    class:bg-white={seg.segmentIndex !== activeSegmentIndex}
                    class:dark:bg-gray-800={seg.segmentIndex !== activeSegmentIndex}
                    class:preview-interaction-disabled={previewEditMode}
                    class:hover:bg-blue-50={!previewEditMode}
                    class:dark:hover:bg-blue-900={!previewEditMode}
                    class:cursor-pointer={!previewEditMode}
                    on:click={() => handleSegmentClick(seg.segmentIndex)}
                    tabindex={previewEditMode ? -1 : 0}
                    on:keydown={(e) => { if (!previewEditMode && (e.key === 'Enter' || e.key === ' ')) { e.preventDefault(); handleSegmentClick(seg.segmentIndex); } }}
                    role={previewEditMode ? 'listitem' : 'button'}
                    aria-pressed={seg.segmentIndex === activeSegmentIndex}
                    aria-label={`Segment ${seg.segmentIndex + 1}, Speaker ${seg.speaker}, Time ${seg.startTime} to ${seg.endTime}`}
                >
                    <!-- Item 1: Delete Button (conditionally rendered) -->
                    <div class="flex-shrink-0"> <!-- Container for button to manage visibility and spacing -->
                        <button
                            class="btn-icon p-0.5"
                            class:invisible={!previewEditMode}
                            class:text-red-500={previewEditMode} class:hover:text-red-700={previewEditMode}
                            class:dark:text-red-400={previewEditMode} class:dark:hover:text-red-300={previewEditMode}
                            on:click|stopPropagation={(e) => handleDeleteSegment(seg.segmentIndex)}
                            title="Delete this segment"
                            aria-label="Delete this segment"
                        >
                            {@html DELETE_ICON}
                        </button>
                    </div>

                    <!-- Item 2: Main Content Block (structure depends on layout) -->
                    {#if $activeLayout === 'Layout1'}
                    <div class="flex flex-row items-start gap-x-2 flex-grow min-w-0 w-full">
                        {#if showSegmentNumberCol}
                        <div class="flex-shrink-0 text-gray-500 dark:text-gray-400 select-none text-sm py-1" style="flex-basis: 5%; max-width: 5%;" title={`Segment Number ${String(seg.segmentIndex + 1)}`}>
                            {String(seg.segmentIndex + 1)}
                        </div>
                        {/if}
                        {#if showTimestampCol}
                        <div class="flex-shrink-0 text-gray-600 dark:text-gray-400 text-left leading-tight text-sm py-1" style="flex-basis: 15%; max-width: 15%;">
                            <span class="select-none" title="Start time">{seg.startTime}</span>
                            <span class="text-gray-400 dark:text-gray-500 select-none block sm:inline">-</span>
                            <span class="select-none" title="End time">{seg.endTime}</span>
                        </div>
                        {/if}
                        {#if showSpeakerCol}
                        <div class="flex-shrink-0 text-gray-800 dark:text-gray-200 font-semibold text-sm py-1 truncate" style="flex-basis: 15%; max-width: 15%;" title={seg.speaker}>
                            {seg.speaker.length > 15 ? seg.speaker.slice(0,13) + '...' : seg.speaker}
                        </div>
                        {/if}
                        {#if showTextCol}
                        <div class="min-w-0 preview-content-area text-sm py-1" style="flex-basis: 65%; white-space: normal; overflow-wrap: break-word; word-break: normal;">
                            {#if seg.isJsonContent}
                                <div class="speech-rich-text">{@html seg.html}</div>
                            {:else}
                                <div class="speech-plain-text">{seg.plainText}</div>
                            {/if}
                        </div>
                        {/if}
                    </div>
                    {:else}
                    <div class="flex flex-col flex-grow gap-y-1 min-w-0">
                        <!-- Row 1: Number and Timestamps (or combined for Layout3) -->
                        {#if showSegmentNumberCol || showTimestampCol || $activeLayout === 'Layout3'}
                        <div class="flex items-center gap-x-2">
                            {#if showSegmentNumberCol && $activeLayout !== 'Layout3'}
                            <div class="flex-shrink-0" style="flex-basis: 1.880rem; max-width: 1.880rem; min-width: 1.880rem;">
                                <span class="truncate text-gray-500 dark:text-gray-400 select-none text-sm" title={`Segment Number ${String(seg.segmentIndex + 1)}`}>
                                    {String(seg.segmentIndex + 1)}
                                </span>
                            </div>
                            {/if}
                            {#if showTimestampCol || $activeLayout === 'Layout3'}
                            <div class="flex-1 text-gray-600 dark:text-gray-400 text-left leading-tight flex items-center gap-x-1 text-sm min-w-0">
                                {#if $activeLayout === 'Layout3'}
                                    <span class="select-none text-gray-600 dark:text-gray-400" title="Timestamp & Speaker">
                                        {seg.startTime} – {seg.endTime}
                                        <span class="ml-1">{seg.speaker}</span>
                                    </span>
                                {:else}
                                    <span class="select-none" title="Start time">{seg.startTime}</span>
                                    <span class="text-gray-400 dark:text-gray-500 select-none">–</span>
                                    <span class="select-none" title="End time">{seg.endTime}</span>
                                {/if}
                            </div>
                            {/if}
                        </div>
                        {/if}

                        <!-- Row 2: Speaker and Text (or just Text for Layout3/Layout5) -->
                        {#if ($activeLayout !== 'Layout3' && showSpeakerCol) || showTextCol}
                        <div class="flex items-start gap-x-2 flex-grow min-h-0">
                            {#if showSpeakerCol && $activeLayout !== 'Layout3' && $activeLayout !== 'Layout5'}
                            <div class="flex-shrink-0 text-gray-800 dark:text-gray-200 font-semibold" style="flex-basis: {$activeLayout === 'Layout4' ? '6rem' : '8rem'}; max-width: {$activeLayout === 'Layout4' ? '6rem' : '8rem'};">
                                <span class="truncate block w-full" title={seg.speaker}>
                                    {seg.speaker.length > ($activeLayout === 'Layout4' ? 10 : 12) ? seg.speaker.slice(0, ($activeLayout === 'Layout4' ? 10 : 12)) + '...' : seg.speaker}:
                                </span>
                            </div>
                            {/if}
                            {#if showTextCol}
                            <div class="min-w-0 preview-content-area flex-grow"
                                 style="white-space: normal; overflow-wrap: break-word; word-break: normal; {$activeLayout === 'Layout3' || $activeLayout === 'Layout5' ? 'margin-left: 0;' : ($activeLayout === 'Layout1' ? '0' : '')}"
                                 class:pl-0={$activeLayout === 'Layout3' || $activeLayout === 'Layout5' || $activeLayout === 'Layout1'}
                                 class:custom-layout3-padding={$activeLayout === 'Layout3' && !showSegmentNumberCol}
                            >
                                {#if seg.isJsonContent}
                                    <div class="speech-rich-text">{@html seg.html}</div>
                                {:else}
                                    <div class="speech-plain-text">{seg.plainText}</div>
                                {/if}
                            </div>
                            {/if}
                        </div>
                        {/if}
                    </div>
                    {/if}
                </div>
                {#if previewEditMode}
                      <div class="flex justify-center insert-button-wrapper"> <button class="btn-icon text-green-400 hover:text-green-600 dark:hover:text-green-300" on:click={() => handleInsertNewSegment(seg.segmentIndex + 1)} title="Insert New Segment" aria-label="Insert New Segment"> {@html INSERT_ICON} </button> </div>
                    {/if}
            {/each}
            <div style="height: {paddingBottom}px;"></div>
        </div>
    {/if}
</div>

<style lang="postcss">
	.btn-icon { @apply p-1 rounded focus:outline-none focus:ring-1 focus:ring-offset-1 focus:ring-blue-400 dark:focus:ring-blue-500 dark:ring-offset-gray-800 transition duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed; }
	.btn-icon > :global(svg), .size-6 { @apply w-5 h-5; }
    .btn-icon:disabled > :global(svg) { @apply text-gray-400 dark:text-gray-500; }
	.segment-block { transition: background-color 0.15s ease-in-out, border-color 0.15s ease-in-out; }
	.segment-block:not(.preview-interaction-disabled):not(.segment-active):hover { @apply bg-blue-50 dark:bg-blue-900/30 border-blue-200 dark:border-blue-900; }
	.segment-block:not(.preview-interaction-disabled):focus { @apply ring-1 ring-blue-300 dark:ring-blue-600 border-blue-300 dark:border-blue-600 outline-none; }
	.preview-interaction-disabled { @apply cursor-default opacity-80; }
	div[class*='overflow-y-auto']::-webkit-scrollbar { @apply w-[8px] h-[8px]; }
	div[class*='overflow-y-auto']::-webkit-scrollbar-track { @apply bg-gray-100 dark:bg-gray-800 rounded-lg; }
	div[class*='overflow-y-auto']::-webkit-scrollbar-thumb { @apply bg-gray-400 dark:bg-gray-500 rounded-lg border-2 border-solid border-gray-100 dark:border-gray-800; }
	div[class*='overflow-y-auto']::-webkit-scrollbar-thumb:hover { @apply bg-gray-500 dark:bg-gray-400; }
	div[class*='overflow-y-auto'] { scrollbar-width: thin; scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track); scrollbar-gutter: stable; }
	:root { --scrollbar-thumb: rgba(160, 174, 192, 1); --scrollbar-track: rgba(243, 244, 246, 1); }
	html.dark { --scrollbar-thumb: rgba(107, 114, 128, 1); --scrollbar-track: rgba(31, 41, 55, 1); }
	.preview-editor-wrapper :global(.lexical-editor-root) { @apply border-none shadow-none rounded-none m-0 p-0 outline-none; background-color: transparent !important; box-shadow: none !important; border: none !important; }
	.preview-editor-wrapper :global(.lexical-wrapper) { @apply p-0 overflow-visible; }
	.preview-editor-wrapper :global(.lexical-content) { @apply leading-normal whitespace-pre-wrap break-words text-gray-900 dark:text-gray-100 pt-px; min-height: unset !important; outline: none !important; caret-color: transparent !important; padding: 0 !important; margin: 0 !important; background-color: transparent !important; overflow-wrap: break-word; word-break: break-word; font-family: Arial, Helvetica, sans-serif; font-size: 12pt; line-height: 1.5;}
    .preview-editor-wrapper :global(.lexical-content[contenteditable="false"]) { caret-color: transparent !important; }
	.preview-editor-wrapper :global(.lexical-content p) { @apply mt-0 mb-0; overflow-wrap: break-word; word-break: break-word; }
	.segment-active .preview-editor-wrapper :global(.lexical-editor-root), .segment-active .preview-editor-wrapper :global(.lexical-content) { background-color: transparent !important; }
	.speech-plain-text {
        @apply leading-normal whitespace-pre-wrap text-gray-900 dark:text-gray-100 pt-px; /* Changed to pre-wrap */
        padding: 0; margin: 0;
        overflow-wrap: break-word;
        word-break: normal;
        font-family: Arial, Helvetica, sans-serif; font-size: 12pt; line-height: 1.5;
    }
	.speech-plain-text .italic { @apply not-italic; }
	.speech-rich-text {
        @apply leading-normal whitespace-pre-wrap text-gray-900 dark:text-gray-100 pt-px; /* Changed to pre-wrap */
        padding: 0; margin: 0;
        overflow-wrap: break-word;
        word-break: normal;
        font-family: Arial, Helvetica, sans-serif; font-size: 12pt; line-height: 1.5;
    }
    .preview-content-area {
        overflow-wrap: break-word; /* Changed from anywhere */
        word-break: normal;       /* Changed from break-all */
    }
	.insert-button-wrapper { position: relative; height: 0px; top: -0.75rem; z-index: 10; opacity: 0.3; transition: opacity 0.15s ease-in-out; }
    .insert-button-wrapper:first-of-type { margin-top: 0.75rem; }
    .insert-button-wrapper:last-of-type { margin-bottom: 0.75rem; }
    .overflow-y-auto:hover .insert-button-wrapper, .segment-block:hover + .insert-button-wrapper { opacity: 1; }
    .insert-button-wrapper button > :global(svg) { width: 1rem; height: 1rem; }

    .btn-switch-active {
        @apply bg-blue-500 text-white shadow-sm;
    }
    .dark .btn-switch-active {
        @apply bg-blue-600 text-white;
    }
    .btn-switch-inactive {
        @apply bg-gray-200 text-gray-700 hover:bg-gray-300 dark:bg-gray-700 dark:text-gray-200 dark:hover:bg-gray-600;
    }
</style>
