<!-- src/lib/components/projectview/transcriptions/RichTextPreview.svelte -->
<script>
	import { project, prepareDocumentView } from '$lib/stores/projectStore.js'; // prepareDocumentView remains
	import { transcriptStore, updatePlayerCurrentSegmentIndex, switchToOriginalTranscript, switchToEnglishTranscript } from '$lib/stores/transcriptStore.js';
	// import { languageOptions } from './TopBar.svelte'; // Removed import
	import { createEventDispatcher, tick } from 'svelte';
	import { confirm, message } from '@tauri-apps/plugin-dialog';
	import { convertAndSaveTranscriptAsDoc } from '$lib/services/projectService.js';
	import { ExtendedTextNode } from '$lib/nodes/ExtendedTextNode.js';
    import { get } from 'svelte/store';
    import { onMount } from 'svelte';

    // Copied languageOptions array
    const languageOptions = [
        { value: 'auto', label: 'Auto Detect' }, { value: 'af', label: 'Afrikaans' },
        { value: 'ar', label: 'Arabic' }, { value: 'hy', label: 'Armenian' },
        { value: 'az', label: 'Azerbaijani' }, { value: 'be', label: 'Belarusian' },
        { value: 'bs', label: 'Bosnian' }, { value: 'bg', label: 'Bulgarian' },
        { value: 'ca', label: 'Catalan' }, { value: 'zh', label: 'Chinese' },
        { value: 'hr', label: 'Croatian' }, { value: 'cs', label: 'Czech' },
        { value: 'da', label: 'Danish' }, { value: 'nl', label: 'Dutch' },
        { value: 'en', label: 'English' }, { value: 'et', label: 'Estonian' },
        { value: 'fi', label: 'Finnish' }, { value: 'fr', label: 'French' },
        { value: 'gl', label: 'Galician' }, { value: 'de', label: 'German' },
        { value: 'el', label: 'Greek' }, { value: 'he', label: 'Hebrew' },
        { value: 'hi', label: 'Hindi' }, { value: 'hu', label: 'Hungarian' },
        { value: 'is', label: 'Icelandic' }, { value: 'id', label: 'Indonesian' },
        { value: 'it', label: 'Italian' }, { value: 'ja', label: 'Japanese' },
        { value: 'kn', label: 'Kannada' }, { value: 'kk', label: 'Kazakh' },
        { value: 'ko', label: 'Korean' }, { value: 'lv', label: 'Latvian' },
        { value: 'lt', label: 'Lithuanian' }, { value: 'mk', label: 'Macedonian' },
        { value: 'ms', label: 'Malay' }, { value: 'mi', label: 'Maori' },
        { value: 'mr', label: 'Marathi' }, { value: 'ne', label: 'Nepali' },
        { value: 'no', label: 'Norwegian' }, { value: 'fa', label: 'Persian' },
        { value: 'pl', label: 'Polish' }, { value: 'pt', label: 'Portuguese' },
        { value: 'ro', label: 'Romanian' }, { value: 'ru', label: 'Russian' },
        { value: 'sr', label: 'Serbian' }, { value: 'sk', label: 'Slovak' },
        { value: 'sl', label: 'Slovenian' }, { value: 'es', label: 'Spanish' },
        { value: 'sw', label: 'Swahili' }, { value: 'sv', label: 'Swedish' },
        { value: 'tl', label: 'Tagalog' }, { value: 'ta', label: 'Tamil' },
        { value: 'th', label: 'Thai' }, { value: 'tr', label: 'Turkish' },
        { value: 'uk', label: 'Ukrainian' }, { value: 'ur', label: 'Urdu' },
        { value: 'vi', label: 'Vietnamese' }, { value: 'cy', label: 'Welsh' },
    ];

    let originalLanguageName = 'Original'; // Default
    $: {
        const selectedLangCode = $transcriptStore.selectedLanguage;
        const langOption = languageOptions.find(option => option.value === selectedLangCode);
        if (langOption) {
            originalLanguageName = langOption.label;
        } else if (selectedLangCode === 'auto') {
            originalLanguageName = 'Auto Detected'; // Or 'Original Language'
        } else {
            originalLanguageName = selectedLangCode ? selectedLangCode.toUpperCase() : 'Original';
        }
    }

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
              console.log('[RichTextPreview] Processing serializedNodeObj:', JSON.stringify(serializedNodeObj));

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
                    console.log('[RichTextPreview] nodesToAppend was empty; added default paragraph via lexicalParseSerializedNode.');
                } catch (defaultNodeErr) {
                    console.error('[RichTextPreview] Error creating default paragraph node:', defaultNodeErr);
                    // If even creating a default node fails, the HTML might end up empty or malformed.
                    // This situation should be rare if lexicalParseSerializedNode and basic paragraph structure are sound.
                }
            }

            editorRoot.append(...nodesToAppend);
            html = generateHtmlFromNodes(htmlEditor, null);
          }, { discrete: true });
        } else {
          // Fallback for invalid structure, though isLexicalJson should catch most.
          console.warn('[RichTextPreview] lexicalJsonToHtml: parsedJson or parsedJson.root.children is invalid. Rendering empty.', parsedJson);
          html = ''; // Or some default error HTML
        }
      } catch (e) {
        console.error('[RichTextPreview] lexicalJsonToHtml: Error processing JSON string. jsonStr:', jsonStr.substring(0, 500), 'Error:', e);
        html = '<!-- error rendering segment content -->'; // Fallback HTML
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
	// isLexicalJson replaced above
	function extractPlainTextForPreview(inputString) { if (!inputString || typeof inputString !== 'string') return '[empty]'; if (isLexicalJson(inputString)) { console.warn("[RichTextPreview] extractPlainTextForPreview called with JSON string, rendering placeholder."); return '[Error: Invalid data format - Expected plain text or HTML]'; } try { const parser = new DOMParser(); const doc = parser.parseFromString(inputString, 'text/html'); if (doc.body.childNodes.length === 1 && doc.body.firstChild.nodeType === Node.TEXT_NODE) { return doc.body.textContent || '[empty]'; } return doc.body.textContent || inputString || '[empty]'; } catch (e) { console.error("[RichTextPreview] Error parsing string in extractPlainTextForPreview:", e); return inputString || '[empty]'; } }

	/* ---------------- build segment data for rendering ---------------- */
	let processedSegments = [];
	let canUndo = false;
	let canRedo = false;
	$: {
	  const segs = $transcriptStore.segments || [];
	  canUndo = ($transcriptStore.transcriptUndoStack?.length || 0) > 0;
	  canRedo = ($transcriptStore.transcriptRedoStack?.length || 0) > 0;
	  processedSegments = segs.map((seg, segIdx) => {
	    const rawContent = seg.text;
	    // --- Begin: detect and wrap bare node JSON if needed ---
	    let contentForParsing = rawContent;
	    try {
	      const parsed = typeof rawContent === 'string' ? JSON.parse(rawContent) : rawContent;
	      // If the JSON is a single node with children but no root, wrap it
	      if (parsed && !parsed.root && Array.isArray(parsed.children)) {
	        contentForParsing = JSON.stringify({
	          root: {
	            type: 'root',
	            version: 1,
	            format: '',
	            indent: 0,
	            direction: null,
	            children: [parsed]
	          }
	        });
	      }
	    } catch (e) {
	      // Not valid JSON or other error: leave contentForParsing as rawContent
	    }
	    // --- End: detect and wrap bare node JSON if needed ---
	    const isJson = isLexicalJson(contentForParsing);
	    let plainTextForDisplay = '';
	    let contentJsonForEditor = defaultEmptyJson;
	    if (isJson) {
	      // ensure we always pass a string to the editor
	      contentJsonForEditor =
	        typeof contentForParsing === 'string' ? contentForParsing : JSON.stringify(contentForParsing);
	    } else {
	      plainTextForDisplay = extractPlainTextForPreview(rawContent);
	    }
	    const html = isJson
	      ? lexicalJsonToHtml(contentForParsing)
	      : `<div>${plainTextForDisplay}</div>`;
	    return {
	      segmentIndex: segIdx,
	      startTime: formatTimestamp(seg.start_time),
	      endTime: formatTimestamp(seg.end_time),
	      rawStart: seg.start_time,
	      rawEnd: seg.end_time,
	      speaker: seg.speaker || 'Unknown',
	      isJsonContent: isJson,
	      html,
	      plainText: plainTextForDisplay
	    };
	  });
	}

    // --- Highlight and Scroll Logic ---
    let previewScrollContainerRef; $: activeSegmentIndex = $transcriptStore.player?.currentSegmentIndex ?? -1;
    $: if (activeSegmentIndex !== -1 && isMounted) { tick().then(() => { if (!previewScrollContainerRef) return; const currentElement = document.getElementById(`segment-${activeSegmentIndex}`); if (!currentElement) return; const containerRect = previewScrollContainerRef.getBoundingClientRect(); const currentElementRect = currentElement.getBoundingClientRect(); const nextIndex = activeSegmentIndex + 1; const nextElement = document.getElementById(`segment-${nextIndex}`); const SCROLL_AHEAD_MARGIN_PX = 150; const isCurrentElementNearBottom = currentElementRect.bottom > (containerRect.bottom - SCROLL_AHEAD_MARGIN_PX); let elementToScroll = currentElement; if (nextElement && isCurrentElementNearBottom) { elementToScroll = nextElement; } elementToScroll.scrollIntoView({ behavior: 'smooth', block: 'nearest' }); }); }
    let isMounted = false; onMount(() => { isMounted = true; });

	/* ---------------- interactions ---------------- */
	function handleSegmentClick(idx) { if (!previewEditMode) { dispatch('segmentclick', idx); } else { console.log(`[RichTextPreview] Click on segment ${idx} ignored (preview edit mode active).`); } }
	function handleToggleEdit() { dispatch('toggleedit'); }

    // --- MODIFIED ---
	async function handleAddToDocumentsClick() {
		const confirmationMessage = `This will create a copy of the current transcript as a new document.\n\nThis document will not sync with the media player.`;
		const userConfirmed = await confirm(confirmationMessage, {
			title: 'Export to Documents?',
			type: 'info',
			okLabel: 'Yes, Create Document',
			cancelLabel: 'Cancel'
		});

		if (userConfirmed) {
			console.log("[RichTextPreview] User confirmed adding to Documents. Converting and saving...");
			try {
                // Call service function
				const newDocPath = await convertAndSaveTranscriptAsDoc();
				if (newDocPath) {
					console.log(`[RichTextPreview] Document saved successfully: ${newDocPath}.`);
					await message(`Transcript copied to Documents:\n${newDocPath.split(/[\\/]/).pop()}`, {title: "Document Created", type: "info"});

                    // --- MODIFICATION: Remove direct call to prepareDocumentView ---
					// prepareDocumentView(newDocPath); // REMOVED

					// --- MODIFICATION: Dispatch event with the new path ---
					dispatch('requestopentab', { tabName: 'notes', loadNotePath: newDocPath }); // ADDED loadNotePath

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
			console.log("[RichTextPreview] User cancelled adding to Documents.");
		}
	}
    // --- END MODIFIED ---

    async function handleDeleteSegment(idx) { if (!previewEditMode) return; const segmentToDelete = processedSegments[idx]; if (!segmentToDelete) { console.error(`[RichTextPreview] Delete requested for invalid index: ${idx}`); return; } const confirmation = await confirm( `Are you sure you want to delete segment ${idx + 1}?\n\n[${segmentToDelete.startTime} - ${segmentToDelete.endTime}]\n"${(segmentToDelete.plainText || '...').substring(0, 50)}..."\n\nThis action can be undone until you save the transcript.`, { title: 'Confirm Delete Segment', type: 'warning', okLabel: 'Delete Segment', cancelLabel: 'Cancel' } ); if (confirmation) { console.log(`[RichTextPreview] User confirmed deletion of segment index: ${idx}. Dispatching deletetranscriptsegment.`); dispatch('deletetranscriptsegment', idx); } else { console.log(`[RichTextPreview] User cancelled deletion of segment index: ${idx}.`); } }
    function handleUndo() { if (canUndo) { dispatch('undo'); } }
    function handleRedo() { if (canRedo) { dispatch('redo'); } }
    async function handleInsertNewSegment(index) { if (!previewEditMode) return; const MIN_GAP_SECONDS = 1.0; const TIME_TOLERANCE = 0.001; const currentSegments = get(transcriptStore).segments; const mediaDuration = get(transcriptStore).player.duration; let prevEndTime = 0.0; let nextStartTime = mediaDuration; if (index > 0) { prevEndTime = currentSegments[index - 1]?.end_time ?? 0.0; } if (index < currentSegments.length) { nextStartTime = currentSegments[index]?.start_time ?? mediaDuration; } const gap = nextStartTime - prevEndTime; console.log(`[RichTextPreview] Insert check at index ${index}: PrevEnd=${prevEndTime.toFixed(3)}, NextStart=${nextStartTime.toFixed(3)}, Gap=${gap.toFixed(3)}`); if (gap < MIN_GAP_SECONDS + (2 * TIME_TOLERANCE)) { await message(`Cannot insert segment here. The gap between segments must be at least ${MIN_GAP_SECONDS.toFixed(1)} seconds. Current gap is ${gap.toFixed(3)} seconds.`, { title: 'Cannot Insert Segment', type: 'info' }); return; } let newStartTime = prevEndTime + TIME_TOLERANCE; let newEndTime = nextStartTime - TIME_TOLERANCE; newStartTime = Math.max(0, newStartTime); newEndTime = Math.min(mediaDuration, newEndTime); newEndTime = Math.max(newStartTime, newEndTime); if (newEndTime > newStartTime) { console.log(`[RichTextPreview] Dispatching insertnewsegment (filling gap): index=${index}, start=${newStartTime.toFixed(3)}, end=${newEndTime.toFixed(3)}`); dispatch('insertnewsegment', { index, startTime: newStartTime, endTime: newEndTime }); } else { console.error(`[RichTextPreview] Calculated invalid times for gap fill insertion: start=${newStartTime.toFixed(3)}, end=${newEndTime.toFixed(3)}`); await message('Could not calculate valid timestamps for the new segment in the available gap.', { title: 'Insertion Error', type: 'error' }); } }

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

</script>

<div
  class="p-3 h-full flex flex-col text-base text-gray-900 dark:text-gray-200"
  style="font-family: Arial, Helvetica, sans-serif; font-size: 12pt; line-height: 1.5;"
>
    <h3 class="font-semibold mb-2 text-sm text-gray-700 dark:text-gray-300 border-b border-gray-300 dark:border-gray-600 pb-1 flex items-center justify-between w-full">
        <div class="flex items-center"> <!-- This div will be for language toggles -->
            {#if $transcriptStore.originalSegments && $transcriptStore.originalSegments.length > 0}
                <div class="flex items-center p-1 bg-gray-100 dark:bg-gray-750 rounded-md shadow space-x-1">
                    <button
                        class="px-3 py-1 text-xs rounded-md transition-colors duration-150 ease-in-out focus:outline-none focus:ring-2 focus:ring-blue-500"
                        class:btn-switch-active={$transcriptStore.activeTranscriptLanguage === 'original' || !($transcriptStore.englishSegments && $transcriptStore.englishSegments.length > 0)}
                        class:btn-switch-inactive={($transcriptStore.activeTranscriptLanguage !== 'original' && ($transcriptStore.englishSegments && $transcriptStore.englishSegments.length > 0))}
                        on:click={switchToOriginalTranscript}
                        disabled={!($transcriptStore.englishSegments && $transcriptStore.englishSegments.length > 0) && $transcriptStore.activeTranscriptLanguage === 'original'}
                    >
                        {originalLanguageName}
                    </button>
                    {#if $transcriptStore.englishSegments && $transcriptStore.englishSegments.length > 0}
                        <button
                            class="px-3 py-1 text-xs rounded-md transition-colors duration-150 ease-in-out focus:outline-none focus:ring-2 focus:ring-blue-500"
                            class:btn-switch-active={$transcriptStore.activeTranscriptLanguage === 'english'}
                            class:btn-switch-inactive={$transcriptStore.activeTranscriptLanguage !== 'english'}
                            on:click={switchToEnglishTranscript}
                        >
                            English
                        </button>
                    {/if}
                </div>
            {/if}
        </div>
        <div class="flex items-center"> <!-- This new div groups Edit/Undo/Redo and More options -->
            {#if processedSegments.length || previewEditMode}
                <button on:click={handleToggleEdit} class="btn-icon ml-2 text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200" title={previewEditMode ? 'Save Transcript (Ctrl+S)' : 'Edit Transcript (Ctrl+E)'} aria-label={previewEditMode ? 'Save Transcript' : 'Edit Transcript'}> {@html previewEditMode ? SAVE_ICON : EDIT_ICON} </button>
                {#if previewEditMode}
                  <button class="btn-icon ml-2" class:text-gray-400={!canUndo} class:dark:text-gray-500={!canUndo} class:text-gray-600={canUndo} class:hover:text-gray-800={canUndo} class:dark:text-gray-400={canUndo} class:dark:hover:text-gray-200={canUndo} on:click={handleUndo} title="Undo (Ctrl+Z)" aria-label="Undo Transcript Change" disabled={!canUndo}> {@html UNDO_ICON} </button>
                  <button class="btn-icon ml-2" class:text-gray-400={!canRedo} class:dark:text-gray-500={!canRedo} class:text-gray-600={canRedo} class:hover:text-gray-800={canRedo} class:dark:text-gray-400={canRedo} class:dark:hover:text-gray-200={canRedo} on:click={handleRedo} title="Redo (Ctrl+Y)" aria-label="Redo Transcript Change" disabled={!canRedo}> {@html REDO_ICON} </button>
                {/if}
            {/if}
            {#if processedSegments.length}
              <div class="relative inline-block ml-2"> <!-- Added ml-2 for spacing from edit buttons -->
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

    {#if !processedSegments.length}
        <div class="flex-grow flex items-center justify-center text-gray-400">
            {#if previewEditMode}
                Transcript empty. Click Insert button to add a segment.
                <div class="flex justify-center insert-button-wrapper absolute top-1/2 left-1/2 -translate-x-1/2 translate-y-4"> <button class="btn-icon text-green-400 hover:text-green-600 dark:hover:text-green-300" on:click={() => handleInsertNewSegment(0)} title="Insert New Segment" aria-label="Insert New Segment"> {@html INSERT_ICON} </button> </div>
            {:else} No transcript data to preview. {/if}
        </div>
    {:else}
        <div bind:this={previewScrollContainerRef} class="flex-grow overflow-y-auto space-y-1 pr-1">
            {#if previewEditMode}
              <div class="flex justify-center insert-button-wrapper"> <button class="btn-icon text-green-400 hover:text-green-600 dark:hover:text-green-300" on:click={() => handleInsertNewSegment(0)} title="Insert New Segment" aria-label="Insert New Segment"> {@html INSERT_ICON} </button> </div>
            {/if}
            {#each processedSegments as seg (seg.segmentIndex)}
                <div id={`segment-${seg.segmentIndex}`} class:segment-block={true} class="p-2 border rounded-lg shadow-sm transition-colors duration-150 ease-in-out dark:border-gray-700" class:segment-active={seg.segmentIndex === activeSegmentIndex} class:border-blue-400={seg.segmentIndex === activeSegmentIndex} class:bg-blue-100={seg.segmentIndex === activeSegmentIndex} class:dark:bg-blue-900={seg.segmentIndex === activeSegmentIndex} class:dark:border-blue-600={seg.segmentIndex === activeSegmentIndex} class:border-gray-200={seg.segmentIndex !== activeSegmentIndex} class:bg-white={seg.segmentIndex !== activeSegmentIndex} class:dark:bg-gray-800={seg.segmentIndex !== activeSegmentIndex} class:preview-interaction-disabled={previewEditMode} class:hover:bg-blue-50={!previewEditMode} class:dark:hover:bg-blue-900={!previewEditMode} class:cursor-pointer={!previewEditMode} on:click={() => handleSegmentClick(seg.segmentIndex)} tabindex={previewEditMode ? -1 : 0} on:keydown={(e) => { if (!previewEditMode && (e.key === 'Enter' || e.key === ' ')) { e.preventDefault(); handleSegmentClick(seg.segmentIndex); } }} role={previewEditMode ? 'listitem' : 'button'} aria-pressed={seg.segmentIndex === activeSegmentIndex} aria-label={`Segment ${seg.segmentIndex + 1}, Speaker ${seg.speaker}, Time ${seg.startTime} to ${seg.endTime}`}>
                    <div class="flex items-start w-[37.606rem] mx-auto">
                        <div
                          class="flex flex-col text-center items-center flex-shrink-0 px-[5.75pt]"
                          style="flex: 0 0 5%; max-width: 5%;"
                        >
                          <span class="truncate text-gray-500 dark:text-gray-400 select-none text-center" title={`Segment Number ${String(seg.segmentIndex + 1)}`}> {String(seg.segmentIndex + 1)} </span> <button class="btn-icon mt-1 p-0.5" class:invisible={!previewEditMode} class:text-red-500={previewEditMode} class:hover:text-red-700={previewEditMode} class:dark:text-red-400={previewEditMode} class:dark:hover:text-red-300={previewEditMode} on:click|stopPropagation={(e) => handleDeleteSegment(seg.segmentIndex)} title="Delete this segment" aria-label="Delete this segment"> {@html DELETE_ICON} </button>
                        </div>
                        <div
                          class="flex-shrink-0 text-gray-600 dark:text-gray-400 select-none text-center px-[5.75pt]"
                          style="flex: 0 0 15%; max-width: 15%;"
                        >
                          <div>{seg.startTime}</div>
                          <div class="text-gray-400 dark:text-gray-500">–</div>
                          <div>{seg.endTime}</div>
                        </div>
                        <div
                          class="flex-shrink-0 truncate text-center whitespace-nowrap px-[5.75pt] text-gray-800 dark:text-gray-200"
                          style="flex: 0 0 15%; max-width: 15%;"
                        >
                          {seg.speaker}:
					</div>
                        <div
                          class="min-w-0 preview-content-area px-[5.75pt]"
                          style="flex: 0 0 65%; max-width: 65%; white-space: normal; overflow-wrap: break-word; word-break: normal;"
                        >
                            {#if seg.isJsonContent}
                              <div class="speech-rich-text">{@html seg.html}</div>
                            {:else}
                              <div class="speech-plain-text">{seg.plainText}</div>
                            {/if}
                        </div>
                    </div>
                </div>
                {#if previewEditMode}
                  <div class="flex justify-center insert-button-wrapper"> <button class="btn-icon text-green-400 hover:text-green-600 dark:hover:text-green-300" on:click={() => handleInsertNewSegment(seg.segmentIndex + 1)} title="Insert New Segment" aria-label="Insert New Segment"> {@html INSERT_ICON} </button> </div>
                {/if}
            {/each}
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
        @apply leading-normal whitespace-normal text-gray-900 dark:text-gray-100 pt-px; /* Removed break-all */
        padding: 0; margin: 0;
        overflow-wrap: break-word; /* Changed from anywhere */
        word-break: normal;       /* Changed from break-all */
        font-family: Arial, Helvetica, sans-serif; font-size: 12pt; line-height: 1.5;
    }
	.speech-plain-text .italic { @apply not-italic; }
	.speech-rich-text {
        @apply leading-normal whitespace-normal text-gray-900 dark:text-gray-100 pt-px; /* Removed break-all */
        padding: 0; margin: 0;
        overflow-wrap: break-word; /* Changed from anywhere */
        word-break: normal;       /* Changed from break-all */
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