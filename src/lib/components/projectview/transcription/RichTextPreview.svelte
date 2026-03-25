<!-- src/lib/components/projectview/transcription/RichTextPreview.svelte -->
<script>
    import { project, prepareDocumentView } from "$lib/stores/projectStore.js";
    import {
        transcriptStore,
        updatePlayerCurrentSegmentIndex,
        switchTranscript,
        setSecondaryTranscript,
        updateManualSegmentSettings,
    } from "$lib/stores/transcriptStore.js";
    import { createEventDispatcher, tick, onMount, onDestroy } from "svelte";
    import { basename } from "@tauri-apps/api/path";
    import { confirm, message } from "@tauri-apps/plugin-dialog";
    import { languageOptions } from "$lib/constants/transcriptionOptions.js";
    import {
        convertAndSaveTranscriptAsDoc,
        convertAndSaveTranscriptAsTranscript,
    } from "$lib/services/projectService.js";
    import { ExtendedTextNode } from "$lib/nodes/ExtendedTextNode.js";
    import { get } from "svelte/store";
    import { listen } from "@tauri-apps/api/event"; // Added for Tauri event listener
    import { activeLayout } from "$lib/stores/layoutStore.js";
    import { DOCX_LAYOUT_OPTIONS } from "$lib/constants/exportLayouts.js";
    import { Button, Dropdown, DropdownItem } from "flowbite-svelte";
    import FindReplaceModal from "../modals/FindReplaceModal.svelte";
    import {
        Search,
        SquarePen,
        UnfoldVertical,
        FoldVertical,
        Save,
        Trash2,
        Undo,
        Redo,
        PlusSquare,
        MoreVertical,
        Play,
        ChevronLeft,
        ChevronRight,
        X,
        ChevronDown,
    } from "@lucide/svelte";
    // Virtualization state
    let scrollTop = 0;
    let containerHeight = 0;
    const ESTIMATED_SEGMENT_HEIGHT = 70; // Adjust as needed, or measure dynamically
    const OVERSCAN_COUNT = 5; // Number of items to render above/below viewport

    let searchUiContainerElement;
    let searchToggleButtonElement;

    let transcriptDropdownOpen = false;
    let secondaryTranscriptDropdownOpen = false;
    let moreOptionsDropdownOpen = false;

    let refreshKey = 0; // Key to force re-evaluation of transcript list
    let unlistenJobComplete = null; // To store the unlisten function

    function getLanguageLabel(langCode) {
        if (!langCode || langCode.toLowerCase() === "original")
            return "Original";

        let targetCode = langCode;
        if (langCode.includes("-")) {
            targetCode = langCode.split("-").pop(); // e.g., 'en-hi' -> 'hi'
        }

        const option = languageOptions.find((opt) => opt.value === targetCode);
        return option ? option.label : targetCode; // Fallback to code if not found
    }

    // Function to close dropdown when clicking outside
    function handleClickOutsideSearch(event) {
        if (showSearchBox) {
            const isClickInsideSearchUi =
                searchUiContainerElement &&
                searchUiContainerElement.contains(event.target);
            const isClickOnSearchToggleButton =
                searchToggleButtonElement &&
                searchToggleButtonElement.contains(event.target);

            if (!isClickInsideSearchUi && !isClickOnSearchToggleButton) {
                showSearchBox = false;
                clearSearchHighlights();
            }
        }
    }

    onMount(async () => {
        document.addEventListener("click", handleClickOutsideSearch, true);

        unlistenJobComplete = await listen(
            "custom_transcription_job_completed",
            (event) => {
                if (event.payload && event.payload.status === "done") {
                    const currentSelectedMedia =
                        get(transcriptStore).selectedMediaFile;
                    if (
                        currentSelectedMedia &&
                        event.payload.jobFinishedPath ===
                            currentSelectedMedia.path
                    ) {
                        console.log(
                            "[RichTextPreview] Relevant transcription job completed. Incrementing refreshKey.",
                        );
                        refreshKey++;
                    }
                }
            },
        );
        isMounted = true; // For scroll logic
        if (previewScrollContainerRef) {
            // For scroll logic
            containerHeight = previewScrollContainerRef.clientHeight;
            scrollTop = previewScrollContainerRef.scrollTop;
        }
    });

    onDestroy(() => {
        document.removeEventListener("click", handleClickOutsideSearch, true);
        if (unlistenJobComplete) {
            unlistenJobComplete();
        }
        cancelAnimation(); // For scroll logic
    });

    import { derived } from "svelte/store";

    const displayedTranscripts = derived(
        transcriptStore,
        ($transcriptStore) => {
            const transcripts =
                $transcriptStore.selectedMediaFile?.associated_transcripts;
            if (!transcripts || transcripts.length === 0) return [];

            const withLabels = transcripts.map((t) => {
                const langLabel = getLanguageLabel(
                    t.language_code || "original",
                );
                let fileName = t.name;
                if (!fileName && t.path) {
                    try {
                        const pathParts = t.path.split(/[\\/]/);
                        fileName = pathParts[pathParts.length - 1];
                        if (fileName.toLowerCase().endsWith(".json")) {
                            fileName = fileName.substring(
                                0,
                                fileName.length - 5,
                            );
                        }
                    } catch (e) {
                        console.error(
                            "Error extracting filename from path:",
                            e,
                        );
                        fileName = "";
                    }
                }
                const fileNamePart = fileName ? ` (${fileName})` : "";
                const displayLabel = `${langLabel}${fileNamePart}`;
                return { ...t, displayLabel };
            });

            return withLabels.sort((a, b) =>
                a.displayLabel.localeCompare(b.displayLabel),
            );
        },
    );

    const secondaryDisplayedTranscripts = derived(
        transcriptStore,
        ($transcriptStore) => {
            const transcripts =
                $transcriptStore.selectedMediaFile?.associated_transcripts;
            if (
                !$transcriptStore.isDualModeActive ||
                !transcripts ||
                transcripts.length < 2
            )
                return [];

            const withLabels = transcripts
                .filter(
                    (t) => t.path !== $transcriptStore.currentTranscriptPath,
                )
                .map((t) => {
                    const langLabel = getLanguageLabel(
                        t.language_code || "original",
                    );
                    let fileName = t.name;
                    if (!fileName && t.path) {
                        try {
                            const pathParts = t.path.split(/[\\/]/);
                            fileName = pathParts[pathParts.length - 1];
                            if (fileName.toLowerCase().endsWith(".json")) {
                                fileName = fileName.substring(
                                    0,
                                    fileName.length - 5,
                                );
                            }
                        } catch (e) {
                            console.error(
                                "Error extracting filename from path:",
                                e,
                            );
                            fileName = "";
                        }
                    }
                    const fileNamePart = fileName ? ` (${fileName})` : "";
                    const displayLabel = `${langLabel}${fileNamePart}`;
                    return { ...t, displayLabel };
                });

            return withLabels.sort((a, b) =>
                a.displayLabel.localeCompare(b.displayLabel),
            );
        },
    );

    import { createHeadlessEditor } from "@lexical/headless";
    import { $generateHtmlFromNodes as generateHtmlFromNodes } from "@lexical/html";

    import {
        RootNode,
        ParagraphNode,
        TextNode,
        LineBreakNode,
        $getRoot as lexicalGetRoot,
        $parseSerializedNode as lexicalParseSerializedNode,
    } from "lexical";
    import { HeadingNode, QuoteNode } from "@lexical/rich-text";
    import { ListNode, ListItemNode } from "@lexical/list";
    import { TableNode, TableRowNode, TableCellNode } from "@lexical/table";
    import { LinkNode } from "@lexical/link";

    // headless editor for HTML serialization
    const htmlEditor = createHeadlessEditor({
        namespace: "RichTextHtmlGen",
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
            ExtendedTextNode,
        ],
    });

    // helper to detect Lexical JSON (accepts string or object)
    function isLexicalJson(value) {
        if (value === null || value === undefined) return false;

        let data;
        if (typeof value === "string") {
            try {
                data = JSON.parse(value);
            } catch {
                return false; // invalid JSON string
            }
        } else if (typeof value === "object") {
            data = value; // already parsed
        } else {
            return false; // unsupported type
        }

        return !!(data && data.root && data.root.type === "root");
    }

    // helper to convert Lexical JSON (string | object) to HTML
    function lexicalJsonToHtml(json) {
        const jsonStr = typeof json === "string" ? json : JSON.stringify(json);
        let html = "";

        try {
            const parsedJson = JSON.parse(jsonStr); // Parse the JSON string into an object

            if (
                parsedJson &&
                parsedJson.root &&
                Array.isArray(parsedJson.root.children)
            ) {
                const serializedNodes = parsedJson.root.children;

                htmlEditor.update(
                    () => {
                        const editorRoot = lexicalGetRoot(); // Get the root of the htmlEditor
                        editorRoot.clear();

                        const nodesToAppend = [];
                        const validSerializedNodes =
                            serializedNodes.filter(Boolean);

                        for (const serializedNodeObj of validSerializedNodes) {
                            // The console.log for diagnostics can be kept or removed. For this fix, let's keep it for now.
                            // console.log('[RichTextPreview] Processing serializedNodeObj:', JSON.stringify(serializedNodeObj)); // Removed verbose log

                            if (
                                serializedNodeObj.type === "root" &&
                                serializedNodeObj.children &&
                                Array.isArray(serializedNodeObj.children)
                            ) {
                                // If the serializedNodeObj is a RootNode itself, process its children
                                for (const childOfNestedRoot of serializedNodeObj.children) {
                                    if (childOfNestedRoot) {
                                        // Ensure child is not null/undefined
                                        try {
                                            nodesToAppend.push(
                                                lexicalParseSerializedNode(
                                                    childOfNestedRoot,
                                                ),
                                            );
                                        } catch (parseErr) {
                                            console.error(
                                                "[RichTextPreview] Error parsing childOfNestedRoot:",
                                                childOfNestedRoot,
                                                parseErr,
                                            );
                                            // Optionally add a placeholder or skip if a child fails
                                        }
                                    }
                                }
                            } else {
                                // Otherwise, parse the serializedNodeObj directly (assuming it's a Paragraph, List, etc.)
                                try {
                                    nodesToAppend.push(
                                        lexicalParseSerializedNode(
                                            serializedNodeObj,
                                        ),
                                    );
                                } catch (parseErr) {
                                    console.error(
                                        "[RichTextPreview] Error parsing serializedNodeObj:",
                                        serializedNodeObj,
                                        parseErr,
                                    );
                                    // Optionally add a placeholder or skip
                                }
                            }
                        }

                        // Ensure nodesToAppend is not empty for a valid Lexical state before appending
                        if (nodesToAppend.length === 0) {
                            try {
                                const defaultParagraphNode =
                                    lexicalParseSerializedNode({
                                        type: "paragraph",
                                        version: 1,
                                        children: [],
                                        direction: null,
                                        format: "",
                                        indent: 0,
                                    });
                                nodesToAppend.push(defaultParagraphNode);
                                // console.log('[RichTextPreview] nodesToAppend was empty; added default paragraph via lexicalParseSerializedNode.'); // Removed verbose log
                            } catch (defaultNodeErr) {
                                console.error(
                                    "[RichTextPreview] Error creating default paragraph node:",
                                    defaultNodeErr,
                                );
                            }
                        }

                        editorRoot.append(...nodesToAppend);
                        html = generateHtmlFromNodes(htmlEditor, null);
                    },
                    { discrete: true },
                );
            } else {
                console.warn(
                    "[RichTextPreview] lexicalJsonToHtml: parsedJson or parsedJson.root.children is invalid. Rendering empty.",
                    parsedJson,
                );
                html = "";
            }
        } catch (e) {
            console.error(
                "[RichTextPreview] lexicalJsonToHtml: Error processing JSON string. jsonStr:",
                jsonStr.substring(0, 500),
                "Error:",
                e,
            );
            html = "<!-- error rendering segment content -->";
        }
        return html;
    }

    export let previewEditMode = false;
    const dispatch = createEventDispatcher();

    // Determine platform-specific modifier key name
    const isMac =
        typeof window !== "undefined" &&
        navigator.platform.toUpperCase().indexOf("MAC") >= 0;
    const modKeyName = isMac ? "Cmd" : "Ctrl";

    const defaultEmptyJson = JSON.stringify({
        root: {
            children: [
                {
                    children: [],
                    direction: null,
                    format: "",
                    indent: 0,
                    type: "paragraph",
                    version: 1,
                },
            ],
            direction: null,
            format: "",
            indent: 0,
            type: "root",
            version: 1,
        },
    });

    /* ---------------- helpers ---------------- */
    function formatTimestamp(seconds) {
        if (typeof seconds !== "number" || isNaN(seconds) || seconds < 0)
            return "00:00.000";
        const totalMs = Math.round(seconds * 1000);
        const ms = String(totalMs % 1000).padStart(3, "0");
        const totalS = Math.floor(totalMs / 1000);
        const sec = String(totalS % 60).padStart(2, "0");
        const min = String(Math.floor(totalS / 60)).padStart(2, "0");
        return `${min}:${sec}.${ms}`;
    }

    function extractPlainTextForPreview(inputString) {
        if (!inputString || typeof inputString !== "string") return "[empty]";
        if (isLexicalJson(inputString)) {
            console.warn(
                "[RichTextPreview] extractPlainTextForPreview called with JSON string, rendering placeholder.",
            );
            return "[Error: Invalid data format - Expected plain text or HTML]";
        }
        try {
            const parser = new DOMParser();
            const doc = parser.parseFromString(inputString, "text/html");
            if (
                doc.body.childNodes.length === 1 &&
                doc.body.firstChild.nodeType === Node.TEXT_NODE
            ) {
                return doc.body.textContent || "[empty]";
            }
            return doc.body.textContent || inputString || "[empty]";
        } catch (e) {
            console.error(
                "[RichTextPreview] Error parsing string in extractPlainTextForPreview:",
                e,
            );
            return inputString || "[empty]";
        }
    }

    /* ---------------- build segment data for rendering ---------------- */
    let allSegmentsData = []; // Stores raw or minimally processed segment data
    let canUndo = false;
    let canRedo = false;

    $: {
        canUndo = ($transcriptStore.transcriptUndoStack?.length || 0) > 0;
        canRedo = ($transcriptStore.transcriptRedoStack?.length || 0) > 0;

        if (
            $transcriptStore.isDualModeActive &&
            $transcriptStore.secondaryTranscriptPath
        ) {
            const primarySegs = $transcriptStore.segments || [];
            const secondarySegs =
                $transcriptStore.secondaryTranscriptSegments || [];

            if (primarySegs.length === secondarySegs.length) {
                allSegmentsData = primarySegs.flatMap((pSeg, i) => {
                    const sSeg = secondarySegs[i];
                    return [
                        {
                            segmentIndex: i,
                            originalSegment: pSeg,
                            isPrimary: true,
                        },
                        {
                            segmentIndex: i,
                            originalSegment: sSeg,
                            isPrimary: false,
                        },
                    ];
                });
            } else {
                // This case is handled by the store, but as a fallback:
                allSegmentsData = [];
            }
        } else {
            const segs = $transcriptStore.segments || [];
            allSegmentsData = segs.map((seg, segIdx) => ({
                segmentIndex: segIdx,
                originalSegment: seg,
                isPrimary: true,
            }));
        }
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
            visibleStartIndex = Math.max(
                0,
                Math.floor(scrollTop / ESTIMATED_SEGMENT_HEIGHT) -
                    OVERSCAN_COUNT,
            );
            visibleEndIndex = Math.min(
                totalItems - 1,
                Math.ceil(
                    (scrollTop + containerHeight) / ESTIMATED_SEGMENT_HEIGHT,
                ) + OVERSCAN_COUNT,
            );

            paddingTop = visibleStartIndex * ESTIMATED_SEGMENT_HEIGHT;
            paddingBottom =
                (totalItems - 1 - visibleEndIndex) * ESTIMATED_SEGMENT_HEIGHT;

            visibleSegments = allSegmentsData
                .slice(visibleStartIndex, visibleEndIndex + 1)
                .map((item, i) => {
                    const seg = item.originalSegment;
                    const segIdx = item.segmentIndex;
                    const rawContent = seg.text;
                    let contentForParsing = rawContent;
                    try {
                        const parsed =
                            typeof rawContent === "string"
                                ? JSON.parse(rawContent)
                                : rawContent;
                        if (
                            parsed &&
                            !parsed.root &&
                            Array.isArray(parsed.children)
                        ) {
                            contentForParsing = JSON.stringify({
                                root: {
                                    type: "root",
                                    version: 1,
                                    format: "",
                                    indent: 0,
                                    direction: null,
                                    children: [parsed],
                                },
                            });
                        }
                    } catch (e) {
                        /* Not valid JSON */
                    }

                    const isJson = isLexicalJson(contentForParsing);
                    let plainTextForDisplay = "";
                    if (!isJson) {
                        plainTextForDisplay =
                            extractPlainTextForPreview(rawContent);
                    }

                    return {
                        segmentIndex: segIdx,
                        isPrimary: item.isPrimary,
                        startTime: formatTimestamp(seg.start_time),
                        endTime: formatTimestamp(seg.end_time),
                        rawStart: seg.start_time,
                        rawEnd: seg.end_time,
                        speaker: seg.speaker || "Unknown",
                        isJsonContent: isJson,
                        html: isJson
                            ? lexicalJsonToHtml(contentForParsing)
                            : `<div>${plainTextForDisplay}</div>`,
                        plainText: plainTextForDisplay,
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
    let previewScrollContainerRef;
    $: activeSegmentIndex = $transcriptStore.player?.currentSegmentIndex ?? -1;
    let karaokeScrollIndex = -1; // Tracks the index for which karaoke scroll was last triggered.
    let scrollAnimationId = null; // ID for the requestAnimationFrame loop
    let isProgrammaticScroll = false;
    let expectedScrollTop = -1; // The scroll position our animation expects to be at.
    let hoveredSegment = -1;

    // Scroll and highlight logic
    $: if (
        activeSegmentIndex !== -1 &&
        isMounted &&
        previewScrollContainerRef &&
        activeSegmentIndex !== karaokeScrollIndex
    ) {
        tick().then(() => {
            if (!previewScrollContainerRef) return;

            const container = previewScrollContainerRef;
            const currentContainerHeight = container.clientHeight;
            const currentDomScrollTop = container.scrollTop;
            const maxScrollTop =
                container.scrollHeight - currentContainerHeight;

            const itemTop =
                ($transcriptStore.isDualModeActive
                    ? activeSegmentIndex * 2
                    : activeSegmentIndex) * ESTIMATED_SEGMENT_HEIGHT;
            const itemBottom = itemTop + ESTIMATED_SEGMENT_HEIGHT;

            const shouldScroll = true;

            if (shouldScroll) {
                const viewportTop = currentDomScrollTop;
                const viewportBottom = viewportTop + currentContainerHeight;

                const isScrollingDown =
                    activeSegmentIndex > karaokeScrollIndex &&
                    karaokeScrollIndex !== -1;
                const isScrollingUp =
                    activeSegmentIndex < karaokeScrollIndex &&
                    karaokeScrollIndex !== -1;

                // Sweet spot for incremental scrolling (middle 50% of the screen)
                const sweetSpotTop =
                    viewportTop + currentContainerHeight * 0.25;
                const sweetSpotBottom =
                    viewportTop + currentContainerHeight * 0.75;
                const isItemInSweetSpot =
                    itemTop >= sweetSpotTop && itemBottom <= sweetSpotBottom;

                // Condition for incremental scroll
                if (
                    $transcriptStore.player.isPlaying &&
                    isScrollingDown &&
                    isItemInSweetSpot &&
                    maxScrollTop - currentDomScrollTop > 1
                ) {
                    const targetScrollTop = Math.min(
                        currentDomScrollTop +
                            ($transcriptStore.isDualModeActive
                                ? 2 * ESTIMATED_SEGMENT_HEIGHT
                                : ESTIMATED_SEGMENT_HEIGHT),
                        maxScrollTop,
                    );
                    manualSmoothScroll(targetScrollTop);
                } else {
                    // Fallback to centering logic for seeking, scrolling up, or when the item is outside the sweet spot.
                    const scrollThreshold =
                        ($transcriptStore.isDualModeActive ? 4 : 2) *
                        ESTIMATED_SEGMENT_HEIGHT;
                    const effectiveViewportTop =
                        viewportTop + (isScrollingUp ? scrollThreshold : 0);
                    const effectiveViewportBottom =
                        viewportBottom -
                        (isScrollingDown ? scrollThreshold : 0);
                    const isItemInsideEffectiveViewport =
                        itemTop >= effectiveViewportTop &&
                        itemBottom <= effectiveViewportBottom;

                    if (!isItemInsideEffectiveViewport) {
                        let targetDomScrollTop =
                            itemTop -
                            currentContainerHeight / 2 +
                            ESTIMATED_SEGMENT_HEIGHT;
                        targetDomScrollTop = Math.max(
                            0,
                            Math.min(targetDomScrollTop, maxScrollTop),
                        );

                        if (
                            Math.abs(targetDomScrollTop - currentDomScrollTop) >
                            1
                        ) {
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
            const ease =
                progress < 0.5
                    ? 2 * progress * progress
                    : -1 + (4 - 2 * progress) * progress;
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

    // Search state
    let showSearchBox = false;
    let searchTerm = "";
    let searchResults = [];
    let currentSearchResultIndex = -1;
    let showFindReplaceModal = false;
    let searchInputRef;

    const SEARCH_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-search" viewBox="0 0 16 16"> <path d="M11.742 10.344a6.5 6.5 0 1 0-1.397 1.398h-.001q.044.06.098.115l3.85 3.85a1 1 0 0 0 1.415-1.414l-3.85-3.85a1 1 0 0 0-.115-.1zM12 6.5a5.5 5.5 0 1 1-11 0 5.5 5.5 0 0 1 11 0"/> </svg>`;

    function toggleSearchBox() {
        showSearchBox = !showSearchBox;
        if (showSearchBox) {
            tick().then(() => {
                if (searchInputRef) {
                    searchInputRef.focus();
                    // Optional: re-trigger search if there's an existing term
                    if (searchTerm) executeSearch(searchTerm);
                }
            });
        } else {
            // Clear highlights when closing search box
            clearSearchHighlights();
        }
    }

    function openFindReplaceModal() {
        showFindReplaceModal = true;
        // Search term is already in 'searchTerm' due to binding and executeSearch updates
    }

    function clearSearchHighlights() {
        if (typeof CSS !== "undefined" && CSS.highlights) {
            const prevMatch = CSS.highlights.get("search-match");
            if (prevMatch) {
                prevMatch.clear();
                CSS.highlights.delete("search-match");
            }
            const prevActive = CSS.highlights.get("search-match-active");
            if (prevActive) {
                prevActive.clear();
                CSS.highlights.delete("search-match-active");
            }
        }
    }

    function executeSearch(termToSearch, options = {}) {
        searchTerm = termToSearch;
        const term = termToSearch;
        const {
            isCaseSensitive = false,
            isRegex = false,
            isWholeWord = false,
        } = options;

        searchResults = [];
        currentSearchResultIndex = -1;
        clearSearchHighlights();

        if (!term) return;

        const newResults = [];
        let regex;
        try {
            if (isRegex || isWholeWord) {
                let pattern = isRegex
                    ? term
                    : term.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
                if (isWholeWord) pattern = `\\b${pattern}\\b`;
                regex = new RegExp(pattern, isCaseSensitive ? "g" : "gi");
            }
        } catch (e) {
            console.warn("Invalid search pattern:", term);
            return;
        }

        allSegmentsData.forEach((item, segIdx) => {
            const seg = item.originalSegment;
            const text = extractPlainTextForSearch(seg.text);

            if (regex) {
                let match;
                while ((match = regex.exec(text)) !== null) {
                    newResults.push({
                        segmentIndex: item.segmentIndex,
                        isPrimary: item.isPrimary,
                        offset: match.index,
                        length: match[0].length,
                        text: match[0],
                    });
                    if (match.index === regex.lastIndex) regex.lastIndex++;
                }
            } else {
                const termToUse = isCaseSensitive ? term : term.toLowerCase();
                const textToUse = isCaseSensitive ? text : text.toLowerCase();
                let offset = -1;
                while (
                    (offset = textToUse.indexOf(termToUse, offset + 1)) !== -1
                ) {
                    newResults.push({
                        segmentIndex: item.segmentIndex,
                        isPrimary: item.isPrimary,
                        offset: offset,
                        length: term.length,
                        text: text.substring(offset, offset + term.length),
                    });
                }
            }
        });

        searchResults = newResults;
        if (searchResults.length > 0) {
            currentSearchResultIndex = 0;
            updateSearchHighlights();
        }
    }

    function extractPlainTextForSearch(input) {
        if (!input) return "";
        if (isLexicalJson(input)) {
            try {
                const data =
                    typeof input === "string" ? JSON.parse(input) : input;
                return extractTextFromLexicalNodes(data.root.children);
            } catch (e) {
                return "";
            }
        }
        return extractPlainTextForPreview(input);
    }

    function extractTextFromLexicalNodes(nodes) {
        let text = "";
        for (const node of nodes) {
            if (node.text) text += node.text;
            if (node.children)
                text += extractTextFromLexicalNodes(node.children);
        }
        return text;
    }

    function updateSearchHighlights() {
        if (
            typeof CSS === "undefined" ||
            !CSS.highlights ||
            !searchTerm ||
            searchResults.length === 0
        ) {
            clearSearchHighlights();
            return;
        }

        tick().then(() => {
            const matchRanges = [];
            const activeRanges = [];

            // We can only highlight what's currently in the DOM
            const renderedSegments =
                previewScrollContainerRef.querySelectorAll(".segment-block");
            renderedSegments.forEach((el) => {
                const idAttr = el.id; // segment-{idx}-{p|s}
                const parts = idAttr.split("-");
                const segIdx = parseInt(parts[1], 10);
                const isPrimary = parts[2] === "p";

                const resultsInThisSeg = searchResults
                    .map((r, i) => ({ ...r, globalIndex: i }))
                    .filter(
                        (r) =>
                            r.segmentIndex === segIdx &&
                            r.isPrimary === isPrimary,
                    );

                if (resultsInThisSeg.length > 0) {
                    const contentArea = el.querySelector(
                        ".preview-content-area",
                    );
                    if (contentArea) {
                        const textNodes = [];
                        const walker = document.createTreeWalker(
                            contentArea,
                            NodeFilter.SHOW_TEXT,
                            null,
                        );
                        let node;
                        while ((node = walker.nextNode())) textNodes.push(node);

                        resultsInThisSeg.forEach((res) => {
                            let currentOffset = 0;
                            for (const textNode of textNodes) {
                                const nodeLength = textNode.textContent.length;
                                if (currentOffset + nodeLength > res.offset) {
                                    const startInNode = Math.max(
                                        0,
                                        res.offset - currentOffset,
                                    );
                                    const endInNode = Math.min(
                                        nodeLength,
                                        res.offset + res.length - currentOffset,
                                    );

                                    if (startInNode < nodeLength) {
                                        try {
                                            const range = new Range();
                                            range.setStart(
                                                textNode,
                                                startInNode,
                                            );
                                            range.setEnd(textNode, endInNode);
                                            if (
                                                res.globalIndex ===
                                                currentSearchResultIndex
                                            ) {
                                                activeRanges.push(range);
                                            } else {
                                                matchRanges.push(range);
                                            }
                                        } catch (e) {}
                                    }
                                }
                                currentOffset += nodeLength;
                                if (currentOffset > res.offset + res.length)
                                    break;
                            }
                        });
                    }
                }
            });

            CSS.highlights.set("search-match", new Highlight(...matchRanges));
            CSS.highlights.set(
                "search-match-active",
                new Highlight(...activeRanges),
            );
        });
    }

    function navigateToResult(index) {
        if (index < 0 || index >= searchResults.length) return;
        currentSearchResultIndex = index;
        const res = searchResults[index];

        // Scroll to segment
        const itemTop =
            ($transcriptStore.isDualModeActive
                ? res.segmentIndex * 2 + (res.isPrimary ? 0 : 1)
                : res.segmentIndex) * ESTIMATED_SEGMENT_HEIGHT;
        const targetScrollTop = Math.max(
            0,
            itemTop - containerHeight / 2 + ESTIMATED_SEGMENT_HEIGHT / 2,
        );
        manualSmoothScroll(targetScrollTop);

        updateSearchHighlights();
    }

    function navigateToNextResult() {
        if (searchResults.length === 0) return;
        navigateToResult((currentSearchResultIndex + 1) % searchResults.length);
    }

    function navigateToPreviousResult() {
        if (searchResults.length === 0) return;
        navigateToResult(
            (currentSearchResultIndex - 1 + searchResults.length) %
                searchResults.length,
        );
    }

    function handleReplace(event) {
        const { find, replace, isCaseSensitive, isRegex, isWholeWord } =
            event.detail;
        if (currentSearchResultIndex >= 0 && searchResults.length > 0) {
            const res = searchResults[currentSearchResultIndex];
            dispatch("replacetranscripttext", {
                segmentIndex: res.segmentIndex,
                isPrimary: res.isPrimary,
                find,
                replace,
                offset: res.offset,
                length: res.length,
            });
            // Re-run search after a short delay to allow store update
            setTimeout(
                () =>
                    executeSearch(find, {
                        isCaseSensitive,
                        isRegex,
                        isWholeWord,
                    }),
                50,
            );
        }
    }

    function handleReplaceAll(event) {
        const { find, replace, isCaseSensitive, isRegex, isWholeWord } =
            event.detail;
        if (searchResults.length === 0) return;

        dispatch("replacealltranscripttext", {
            find,
            replace,
            isCaseSensitive,
            isRegex,
            isWholeWord,
        });
        setTimeout(
            () =>
                executeSearch(find, { isCaseSensitive, isRegex, isWholeWord }),
            50,
        );
    }

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
        if (showSearchBox) {
            updateSearchHighlights();
        }
    }

    /* ---------------- interactions ---------------- */
    function handleSegmentClick(idx) {
        // idx here is the original segmentIndex from processedSegments, not the index in visibleSegments
        dispatch("segmentclick", idx);
    }
    function handleToggleEdit() {
        dispatch("toggleedit");
    }

    async function handleAddToDocumentsClick() {
        const confirmationMessage = `This will create a copy of the current transcript as a new document.\n\nThis document will not sync with the media player.`;
        const userConfirmed = await confirm(confirmationMessage, {
            title: "Save in Documents?",
            type: "info",
            okLabel: "Yes, Create Document",
            cancelLabel: "Cancel",
        });

        if (userConfirmed) {
            try {
                const newDocPath = await convertAndSaveTranscriptAsDoc();
                if (newDocPath) {
                    await message(
                        `Transcript copied to Documents:\n${newDocPath.split(/[\\/]/).pop()}`,
                        { title: "Document Created", type: "info" },
                    );
                    dispatch("requestopentab", {
                        tabName: "data",
                        loadNotePath: newDocPath,
                    });
                } else {
                    console.error(
                        "[RichTextPreview] Document saving process did not return a path.",
                    );
                    await message(
                        "Failed to create document file: The process completed but did not provide a file path.",
                        { title: "Error", type: "error" },
                    );
                }
            } catch (error) {
                console.error(
                    "[RichTextPreview] Error during document creation process:",
                    error,
                );
                const errorMsg =
                    error instanceof Error ? error.message : String(error);
                await message(`Failed to create document file: ${errorMsg}`, {
                    title: "Error",
                    type: "error",
                });
            }
        } else {
        }
    }

    async function handleAddToTranscriptsClick() {
        const confirmationMessage = `This will create a copy of the current transcript as a new imported transcript.\n\nThis transcript will not sync with the media player.`;
        const userConfirmed = await confirm(confirmationMessage, {
            title: "Save in Transcripts?",
            type: "info",
            okLabel: "Yes, Save Transcript",
            cancelLabel: "Cancel",
        });

        if (userConfirmed) {
            try {
                const newTranscriptPath =
                    await convertAndSaveTranscriptAsTranscript();
                if (newTranscriptPath) {
                    await message(
                        `Transcript saved to Transcripts:\n${newTranscriptPath.split(/[\\/]/).pop()}`,
                        { title: "Transcript Saved", type: "info" },
                    );
                    // Imported transcripts are viewed in the 'data' tab, similar to documents.
                    dispatch("requestopentab", {
                        tabName: "data",
                        loadNotePath: newTranscriptPath,
                    });
                } else {
                    console.error(
                        "[RichTextPreview] Transcript saving process did not return a path.",
                    );
                    await message(
                        "Failed to save transcript file: The process completed but did not provide a file path.",
                        { title: "Error", type: "error" },
                    );
                }
            } catch (error) {
                console.error(
                    "[RichTextPreview] Error during transcript saving process:",
                    error,
                );
                const errorMsg =
                    error instanceof Error ? error.message : String(error);
                await message(`Failed to save transcript file: ${errorMsg}`, {
                    title: "Error",
                    type: "error",
                });
            }
        }
    }

    async function handleDeleteSegment(idx) {
        if (!previewEditMode) return;
        const store = get(transcriptStore);

        let confirmationMessage = "";

        if (store.isDualModeActive) {
            confirmationMessage = `Are you sure you want to delete segment ${idx + 1} from both transcripts? This action can be undone until you save the transcript.`;
        } else {
            confirmationMessage = `Are you sure you want to delete segment ${idx + 1}? This action can be undone until you save the transcript.`;
        }

        const confirmation = await confirm(confirmationMessage, {
            title: "Confirm Delete Segment",
            type: "warning",
            okLabel: "Delete Segment",
            cancelLabel: "Cancel",
        });
        if (confirmation) {
            dispatch("deletetranscriptsegment", idx);
        } else {
        }
    }

    async function handleSplitSegment(idx) {
        if (!previewEditMode) return;
        const store = get(transcriptStore);
        // Dispatch to parent, which calls store method
        dispatch("splittranscriptsegment", idx);
    }

    async function handleMergeWithNext(idx) {
        if (!previewEditMode) return;
        const store = get(transcriptStore);
        if (idx < 0 || idx >= store.segments.length - 1) return;
        dispatch("mergetranscriptsegment", idx);
    }

    function handleUndo() {
        if (canUndo) {
            dispatch("undo");
        }
    }
    function handleRedo() {
        if (canRedo) {
            dispatch("redo");
        }
    }
    export async function handleInsertNewSegment(index) {
        if (!previewEditMode) return;
        const store = get(transcriptStore);
        const mode = store.transcriptionMode;
        const currentSegments = store.segments;
        const mediaDuration = store.player.duration;

        let finalIndex = index;
        let prevEndTime = 0.0;
        let nextStartTime = mediaDuration;

        if (finalIndex > 0) {
            prevEndTime = currentSegments[finalIndex - 1]?.end_time ?? 0.0;
        }
        if (finalIndex < currentSegments.length) {
            nextStartTime =
                currentSegments[finalIndex]?.start_time ?? mediaDuration;
        }

        // --- MANUAL MODE LOGIC ---
        if (mode === "manual") {
            const settings = store.manualSegmentSettings;
            const duration = settings.duration || 60;

            let newStartTime = prevEndTime;
            let newEndTime = Math.min(mediaDuration, newStartTime + duration);

            const availableGap = nextStartTime - prevEndTime;

            if (newEndTime > nextStartTime + 0.001) {
                if (finalIndex < currentSegments.length) {
                    if (availableGap < 0.5) {
                        await message(
                            `Not enough space to insert a segment here. Gap is ${availableGap.toFixed(2)}s.`,
                            { title: "No Space", type: "warning" },
                        );
                        return;
                    }
                    // Fill available gap if preferred duration is too long
                    newEndTime = nextStartTime;
                }
            }

            if (newEndTime <= newStartTime + 0.001) {
                await message("Cannot insert segment: No duration available.", {
                    title: "Error",
                    type: "warning",
                });
                return;
            }

            // Speaker Logic
            let speaker = "Unknown";
            if (settings.speakerMode === "alternate") {
                const names = store.speakers.names;
                if (names.length >= 2) {
                    let lastIndex = settings.lastUsedSpeakerIndex;
                    if (lastIndex === -1 && finalIndex > 0) {
                        // Try to infer from previous segment if state is fresh
                        const prevSpeaker =
                            currentSegments[finalIndex - 1]?.speaker;
                        if (prevSpeaker && names.includes(prevSpeaker)) {
                            lastIndex = names.indexOf(prevSpeaker);
                        }
                    }

                    const nextSpeakerIndex = (lastIndex + 1) % names.length;
                    speaker = names[nextSpeakerIndex];

                    // Update store for next time
                    updateManualSegmentSettings({
                        lastUsedSpeakerIndex: nextSpeakerIndex,
                    });
                }
            } else {
                // Unassigned mode: "Unknown" or just leave it empty? User said "unassigned".
                // In our system "Unknown" is the unassigned state usually.
                speaker = "Unknown";
            }

            // Construct new segment with empty text structure
            const newSegment = {
                start_time: newStartTime,
                end_time: newEndTime,
                speaker: speaker,
                text: JSON.stringify({
                    root: {
                        children: [
                            {
                                type: "paragraph",
                                version: 1,
                                children: [],
                                direction: null,
                                format: "",
                                indent: 0,
                            },
                        ],
                        type: "root",
                        version: 1,
                        direction: null,
                        format: "",
                        indent: 0,
                    },
                }),
            };

            // We need to call the store function directly because dispatch('insertnewsegment')
            // in TranscriptionView currently hardcodes speaker to "Unknown".
            // To support custom speakers, we should import insertTranscriptSegment directly here.
            // But wait, insertTranscriptSegment is an exported function from transcriptStore.js.
            // I need to import it at the top of this file to use it.
            // I'll assume I can add it to the imports in a separate step or just rely on dispatch if I update TranscriptionView?
            // Updating TranscriptionView to accept 'speaker' in event detail is cleaner.

            dispatch("insertnewsegment", {
                index: finalIndex,
                startTime: newStartTime,
                endTime: newEndTime,
                speaker: speaker,
            });
            return;
        }

        const MIN_GAP_SECONDS = 1.0;
        const TIME_TOLERANCE = 0.001;

        const gap = nextStartTime - prevEndTime;
        if (gap < MIN_GAP_SECONDS + 2 * TIME_TOLERANCE) {
            await message(
                `Cannot insert segment here. The gap between segments must be at least ${MIN_GAP_SECONDS.toFixed(1)} seconds. Current gap is ${gap.toFixed(3)} seconds.`,
                { title: "Cannot Insert Segment", type: "info" },
            );
            return;
        }

        let newStartTime = prevEndTime + TIME_TOLERANCE;
        let newEndTime = nextStartTime - TIME_TOLERANCE;
        newStartTime = Math.max(0, newStartTime);
        newEndTime = Math.min(mediaDuration, newEndTime);
        newEndTime = Math.max(newStartTime, newEndTime);

        if (newEndTime > newStartTime) {
            dispatch("insertnewsegment", {
                index: finalIndex,
                startTime: newStartTime,
                endTime: newEndTime,
            });
        } else {
            console.error(
                `[RichTextPreview] Calculated invalid times for gap fill insertion: start=${newStartTime.toFixed(3)}, end=${newEndTime.toFixed(3)}`,
            );
            await message(
                "Could not calculate valid timestamps for the new segment in the available gap.",
                { title: "Insertion Error", type: "error" },
            );
        }
    }

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
        if (layoutKey === "Layout3") {
            // Timestamped Paragraph
            showSegmentNumberCol = false; // No dedicated number column, implied or part of text
            // Timestamp and Speaker are combined visually, but data exists
        } else if (layoutKey === "Layout4") {
            // Speaker & Text
            showSegmentNumberCol = false;
            showTimestampCol = false;
        } else if (layoutKey === "Layout5") {
            // Plain Text
            showSegmentNumberCol = false;
            showTimestampCol = false;
            showSpeakerCol = false;
        }
        // Layout1 (Detailed Table) and Layout2 (Segment Block) show all by default.
    }
</script>

<div
    class="p-3 h-full flex flex-col text-base text-gray-900 dark:text-white dark:bg-gray-950"
    style="font-family: Arial, Helvetica, sans-serif; font-size: 12pt; line-height: 1.5;"
>
    <h3
        class="font-semibold mb-2 text-sm text-gray-700 dark:text-gray-400 border-b border-gray-300 dark:border-gray-700 pb-1 flex items-center justify-between w-full"
    >
        <div class="flex items-center">
            <!-- leftAndMiddleControlsGroup -->
            <!-- Transcript Dropdown using custom component -->
            {#if $transcriptStore.selectedMediaFile}
                <div class="flex items-center">
                    <Button
                        id="primary-transcript-btn"
                        color="alternative"
                        size="xs"
                        class="max-w-[150px] sm:max-w-[200px] md:max-w-[250px] flex justify-between items-center px-3 py-1.5 !font-normal"
                        title="Select Primary Transcript"
                    >
                        <span class="truncate"
                            >{$displayedTranscripts.find(
                                (t) =>
                                    t.path ===
                                    ($transcriptStore.activeTranscript?.path ||
                                        ""),
                            )?.displayLabel || "No Transcripts"}</span
                        >
                        <ChevronDown class="w-3 h-3 ml-2 opacity-50" />
                    </Button>
                    <Dropdown
                        bind:open={transcriptDropdownOpen}
                        triggeredBy="#primary-transcript-btn"
                        class="w-64 overflow-y-auto max-h-60 z-[1010]"
                    >
                        {#each $displayedTranscripts as t}
                            <DropdownItem
                                class="text-xs {($transcriptStore
                                    .activeTranscript?.path || '') === t.path
                                    ? 'font-bold bg-blue-50 dark:bg-gray-700'
                                    : ''}"
                                on:click={() => {
                                    switchTranscript(t.path);
                                    transcriptDropdownOpen = false;
                                }}>{t.displayLabel}</DropdownItem
                            >
                        {/each}
                    </Dropdown>

                    {#if $transcriptStore.isDualModeActive}
                        <Button
                            id="secondary-transcript-btn"
                            color="alternative"
                            size="xs"
                            class="max-w-[150px] sm:max-w-[200px] md:max-w-[250px] ml-2 flex justify-between items-center px-3 py-1.5 !font-normal"
                            title="Select Secondary Transcript"
                        >
                            <span class="truncate"
                                >{$secondaryDisplayedTranscripts.find(
                                    (t) =>
                                        t.path ===
                                        ($transcriptStore.secondaryTranscriptPath ||
                                            ""),
                                )?.displayLabel || "Select Transcript"}</span
                            >
                            <ChevronDown class="w-3 h-3 ml-2 opacity-50" />
                        </Button>
                        <Dropdown
                            bind:open={secondaryTranscriptDropdownOpen}
                            triggeredBy="#secondary-transcript-btn"
                            class="w-64 overflow-y-auto max-h-60 z-[1010]"
                        >
                            {#each $secondaryDisplayedTranscripts as t}
                                <DropdownItem
                                    class="text-xs {($transcriptStore.secondaryTranscriptPath ||
                                        '') === t.path
                                        ? 'font-bold bg-blue-50 dark:bg-gray-700'
                                        : ''}"
                                    on:click={() => {
                                        setSecondaryTranscript(t.path);
                                        secondaryTranscriptDropdownOpen = false;
                                    }}>{t.displayLabel}</DropdownItem
                                >
                            {/each}
                        </Dropdown>
                    {/if}
                </div>
            {:else}
                <span
                    class="px-3 py-1 text-xs text-gray-500 dark:text-gray-600 italic"
                    >No Media Selected</span
                >
            {/if}
        </div>

        <div class="flex items-center relative">
            <!-- This div now only effectively holds the "More options" menu -->
            {#if allSegmentsData.length || previewEditMode}
                <button
                    bind:this={searchToggleButtonElement}
                    class="btn-icon ml-2 text-gray-600 hover:text-gray-800 dark:text-gray-600 dark:hover:text-gray-200 flex items-center justify-center"
                    class:active={showSearchBox}
                    on:click={toggleSearchBox}
                    title="Search"
                >
                    <Search class="w-5 h-5" />
                </button>
            {/if}
            {#if allSegmentsData.length > 0}
                <div class="relative inline-block ml-2">
                    <Button
                        id="more-options-btn"
                        color="alternative"
                        size="xs"
                        class="px-1.5 py-1.5 !font-normal border-none"
                        title="More Options"
                    >
                        <MoreVertical class="w-5 h-5" />
                    </Button>
                    <Dropdown
                        bind:open={moreOptionsDropdownOpen}
                        triggeredBy="#more-options-btn"
                        class="w-max z-[1010]"
                    >
                        <DropdownItem
                            class="text-xs border-b border-gray-100 dark:border-gray-700"
                            on:click={() => {
                                openFindReplaceModal();
                                moreOptionsDropdownOpen = false;
                            }}>Find & Replace</DropdownItem
                        >
                        <DropdownItem
                            class="text-xs border-b border-gray-100 dark:border-gray-700"
                            on:click={() => {
                                dispatch("requestmanualsettings");
                                moreOptionsDropdownOpen = false;
                            }}>Manual Transcription Settings</DropdownItem
                        >
                        <DropdownItem
                            class="text-xs border-b border-gray-100 dark:border-gray-700"
                            on:click={() => {
                                handleAddToTranscriptsClick();
                                moreOptionsDropdownOpen = false;
                            }}>Save in Transcripts</DropdownItem
                        >
                        <DropdownItem
                            class="text-xs"
                            on:click={() => {
                                handleAddToDocumentsClick();
                                moreOptionsDropdownOpen = false;
                            }}>Save in Documents</DropdownItem
                        >
                    </Dropdown>
                </div>
            {/if}

            {#if showSearchBox}
                <div
                    bind:this={searchUiContainerElement}
                    class="absolute right-0 top-full mt-1 z-20 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 shadow-lg p-2 flex items-center gap-2 min-w-[320px] rounded"
                >
                    <div class="relative flex-grow flex items-center">
                        <input
                            type="text"
                            placeholder="Search transcript..."
                            class="w-full text-xs border border-gray-300 dark:border-gray-700 pl-2 pr-16 py-1 bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 focus:ring-blue-500 focus:border-blue-500 rounded outline-none"
                            bind:value={searchTerm}
                            bind:this={searchInputRef}
                            on:input={(e) =>
                                executeSearch(e.currentTarget.value)}
                            on:keydown={(e) => {
                                if (e.key === "Enter") {
                                    e.preventDefault();
                                    if (e.shiftKey) navigateToPreviousResult();
                                    else navigateToNextResult();
                                }
                            }}
                            autocomplete="off"
                            autocorrect="off"
                            autocapitalize="off"
                            spellcheck="false"
                        />
                        <div
                            class="absolute right-1 flex items-center gap-1 pointer-events-none"
                        >
                            {#if searchTerm}
                                <span
                                    class="text-[10px] text-gray-500 dark:text-gray-400 whitespace-nowrap"
                                >
                                    {#if searchResults.length > 0}
                                        {currentSearchResultIndex +
                                            1}/{searchResults.length}
                                    {:else}
                                        0/0
                                    {/if}
                                </span>
                                <button
                                    class="p-0.5 hover:bg-gray-200 dark:hover:bg-gray-600 rounded-full pointer-events-auto transition-colors flex items-center justify-center"
                                    on:click|stopPropagation={() => {
                                        searchTerm = "";
                                        executeSearch("");
                                    }}
                                    title="Clear Search"
                                >
                                    <X class="w-3 h-3" />
                                </button>
                            {/if}
                        </div>
                    </div>
                    <div class="flex items-center gap-0.5">
                        <button
                            class="btn-icon !p-1 flex items-center justify-center"
                            on:click={navigateToPreviousResult}
                            disabled={searchResults.length === 0}
                            title="Previous Match"
                        >
                            <ChevronLeft class="w-4 h-4" />
                        </button>
                        <button
                            class="btn-icon !p-1 flex items-center justify-center"
                            on:click={navigateToNextResult}
                            disabled={searchResults.length === 0}
                            title="Next Match"
                        >
                            <ChevronRight class="w-4 h-4" />
                        </button>
                    </div>
                </div>
            {/if}
        </div>
    </h3>

    {#if allSegmentsData.length === 0}
        <div class="flex-grow flex items-center justify-center text-gray-400">
            {#if previewEditMode}
                Transcript empty. Click Insert button to add a segment.
                <div
                    class="flex justify-center insert-button-wrapper absolute top-1/2 left-1/2 -translate-x-1/2 translate-y-4"
                >
                    <button
                        class="btn-icon text-green-400 hover:text-green-600 dark:hover:text-green-300 flex items-center justify-center"
                        on:click={() => handleInsertNewSegment(0)}
                        title="Insert New Segment"
                        aria-label="Insert New Segment"
                    >
                        <PlusSquare class="w-5 h-5" />
                    </button>
                </div>
            {:else}
                No transcript data to preview.
            {/if}
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
                <div class="flex justify-center insert-button-wrapper">
                    <button
                        class="btn-icon text-green-400 hover:text-green-600 dark:hover:text-green-300 flex items-center justify-center"
                        on:click={() => handleInsertNewSegment(0)}
                        title="Insert New Segment"
                        aria-label="Insert New Segment"
                    >
                        <PlusSquare class="w-5 h-5" />
                    </button>
                </div>
            {/if}
            {#each visibleSegments as seg (`${seg.segmentIndex}-${seg.isPrimary}`)}
                <div
                    id={`segment-${seg.segmentIndex}-${seg.isPrimary ? "p" : "s"}`}
                    class:segment-block={true}
                    class:secondary-segment={$transcriptStore.isDualModeActive &&
                        !seg.isPrimary}
                    class:hovering={hoveredSegment === seg.segmentIndex}
                    class:group={true}
                    on:mouseenter={() => (hoveredSegment = seg.segmentIndex)}
                    on:mouseleave={() => (hoveredSegment = -1)}
                    style="min-height: {ESTIMATED_SEGMENT_HEIGHT}px;"
                    class="p-2 border rounded-lg shadow-sm transition-colors duration-150 ease-in-out dark:border-gray-700 flex items-start gap-x-2 relative"
                    class:segment-active={seg.segmentIndex ===
                        activeSegmentIndex}
                    class:border-blue-400={seg.segmentIndex ===
                        activeSegmentIndex}
                    class:bg-blue-100={seg.segmentIndex === activeSegmentIndex}
                    class:dark:bg-blue-900={seg.segmentIndex ===
                        activeSegmentIndex}
                    class:dark:border-blue-600={seg.segmentIndex ===
                        activeSegmentIndex}
                    class:border-gray-200={seg.segmentIndex !==
                        activeSegmentIndex}
                    class:bg-white={seg.segmentIndex !== activeSegmentIndex}
                    class:dark:bg-gray-900={seg.segmentIndex !==
                        activeSegmentIndex}
                    class:hover:bg-blue-50={true}
                    class:dark:hover:bg-blue-800={true}
                    class:cursor-pointer={true}
                    on:click={() => handleSegmentClick(seg.segmentIndex)}
                    tabindex={0}
                    on:keydown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            handleSegmentClick(seg.segmentIndex);
                        }
                    }}
                    role="button"
                    aria-pressed={seg.segmentIndex === activeSegmentIndex}
                    aria-label={`Segment ${seg.segmentIndex + 1}, Speaker ${seg.speaker}, Time ${seg.startTime} to ${seg.endTime}`}
                >
                    <!-- Item 1: Control Column (Delete/Split in edit mode, Play on hover in read mode) -->
                    <div
                        class="flex-shrink-0 flex flex-col items-center justify-start min-w-[24px] self-center"
                    >
                        {#if previewEditMode}
                            <button
                                class="btn-icon p-0.5 text-red-500 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300 flex items-center justify-center"
                                on:click|stopPropagation={(e) =>
                                    handleDeleteSegment(seg.segmentIndex)}
                                title="Delete this segment"
                                aria-label="Delete this segment"
                            >
                                <Trash2 class="w-5 h-5" />
                            </button>
                            <button
                                class="btn-icon p-0.5 mt-1 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300 flex items-center justify-center"
                                on:click|stopPropagation={(e) =>
                                    handleSplitSegment(seg.segmentIndex)}
                                title="Split this segment"
                                aria-label="Split this segment"
                            >
                                <UnfoldVertical class="w-5 h-5" />
                            </button>
                        {:else}
                            <button
                                class="play-segment-hover-btn-preview w-6 h-6 flex items-center justify-center bg-blue-600 hover:bg-blue-700 text-white rounded-full shadow-md transition-all duration-200 border-2 border-white dark:border-gray-800 opacity-0 group-hover:opacity-100 scale-90 hover:scale-105"
                                on:click|stopPropagation={() =>
                                    dispatch("playsegment", seg.segmentIndex)}
                                title="Play this segment"
                            >
                                <Play class="w-3 h-3 ml-0.5 fill-current" />
                            </button>
                        {/if}
                    </div>

                    <!-- Item 2: Main Content Block (structure depends on layout) -->
                    {#if $activeLayout === "Layout1"}
                        <div
                            class="flex flex-row items-start gap-x-2 flex-grow min-w-0 w-full"
                        >
                            {#if showSegmentNumberCol}
                                <div
                                    class="flex-shrink-0 dark:text-white w-[5%] min-w-[2.5rem]"
                                    title={`Segment Number ${String(seg.segmentIndex + 1)}`}
                                >
                                    {String(seg.segmentIndex + 1)}
                                </div>
                            {/if}
                            {#if showTimestampCol}
                                <div
                                    class="flex-shrink-0 flex flex-col items-start dark:text-white w-[15%] min-w-[5.5rem] leading-snug"
                                >
                                    <span class="select-none" title="Start time"
                                        >{seg.startTime}</span
                                    >
                                    <span
                                        class="dark:text-white text-gray-400 pl-0.5"
                                        >-</span
                                    >
                                    <span class="select-none" title="End time"
                                        >{seg.endTime}</span
                                    >
                                </div>
                            {/if}
                            {#if showSpeakerCol}
                                <div
                                    class="flex-shrink-0 dark:text-white w-[15%] min-w-[5.5rem] truncate"
                                    title={seg.speaker}
                                >
                                    {seg.speaker}{#if !seg.speaker.endsWith(":")}:{/if}
                                </div>
                            {/if}
                            {#if showTextCol}
                                <div
                                    class="min-w-0 preview-content-area py-1 flex-1"
                                    style="white-space: normal; overflow-wrap: break-word; word-break: normal;"
                                >
                                    {#if seg.isJsonContent}
                                        <div class="speech-rich-text">
                                            {@html seg.html}
                                        </div>
                                    {:else}
                                        <div class="speech-plain-text">
                                            {seg.plainText}
                                        </div>
                                    {/if}
                                </div>
                            {/if}
                        </div>
                    {:else}
                        <div class="flex flex-col flex-grow gap-y-1 min-w-0">
                            <!-- Row 1: Number and Timestamps (or combined for Layout3) -->
                            {#if showSegmentNumberCol || showTimestampCol || $activeLayout === "Layout3"}
                                <div class="flex items-center gap-x-2">
                                    {#if showSegmentNumberCol && $activeLayout !== "Layout3"}
                                        <div
                                            class="flex-shrink-0"
                                            style="flex-basis: 1.880rem; max-width: 1.880rem; min-width: 1.880rem;"
                                        >
                                            <span
                                                class="truncate text-gray-500 dark:text-white select-none text-sm"
                                                title={`Segment Number ${String(seg.segmentIndex + 1)}`}
                                            >
                                                {String(seg.segmentIndex + 1)}
                                            </span>
                                        </div>
                                    {/if}
                                    {#if showTimestampCol || $activeLayout === "Layout3"}
                                        <div
                                            class="flex-1 text-gray-600 dark:text-white text-left leading-tight flex items-center gap-x-1 text-sm min-w-0"
                                        >
                                            {#if $activeLayout === "Layout3"}
                                                <span
                                                    class="select-none text-gray-600 dark:text-white"
                                                    title="Timestamp & Speaker"
                                                >
                                                    {seg.startTime} – {seg.endTime}
                                                    <span class="ml-1"
                                                        >{seg.speaker}{#if !seg.speaker.endsWith(":")}:{/if}</span
                                                    >
                                                </span>
                                            {:else}
                                                <span
                                                    class="select-none"
                                                    title="Start time"
                                                    >{seg.startTime}</span
                                                >
                                                <span
                                                    class="text-gray-400 dark:text-white select-none"
                                                    >–</span
                                                >
                                                <span
                                                    class="select-none"
                                                    title="End time"
                                                    >{seg.endTime}</span
                                                >
                                            {/if}
                                        </div>
                                    {/if}
                                </div>
                            {/if}

                            <!-- Row 2: Speaker and Text (or just Text for Layout3/Layout5) -->
                            {#if ($activeLayout !== "Layout3" && showSpeakerCol) || showTextCol}
                                <div
                                    class="flex items-start gap-x-2 flex-grow min-h-0"
                                >
                                    {#if showSpeakerCol && $activeLayout !== "Layout3" && $activeLayout !== "Layout5"}
                                        <div
                                            class="flex-shrink-0 text-gray-800 dark:text-gray-200 font-semibold"
                                            style="flex-basis: 6rem; max-width: 6rem;"
                                        >
                                            <span
                                                class="truncate block w-full"
                                                title={seg.speaker}
                                            >
                                                {seg.speaker.length > 10
                                                    ? seg.speaker.slice(0, 10) +
                                                      "..."
                                                    : seg.speaker}{#if !seg.speaker.endsWith(":")}:{/if}
                                            </span>
                                        </div>
                                    {/if}
                                    {#if showTextCol}
                                        <div
                                            class="min-w-0 preview-content-area flex-grow"
                                            style="white-space: normal; overflow-wrap: break-word; word-break: normal; {$activeLayout ===
                                                'Layout3' ||
                                            $activeLayout === 'Layout5'
                                                ? 'margin-left: 0;'
                                                : $activeLayout === 'Layout1'
                                                  ? '0'
                                                  : ''}"
                                            class:pl-0={$activeLayout ===
                                                "Layout3" ||
                                                $activeLayout === "Layout5" ||
                                                $activeLayout === "Layout1"}
                                            class:custom-layout3-padding={$activeLayout ===
                                                "Layout3" &&
                                                !showSegmentNumberCol}
                                        >
                                            {#if seg.isJsonContent}
                                                <div class="speech-rich-text">
                                                    {@html seg.html}
                                                </div>
                                            {:else}
                                                <div class="speech-plain-text">
                                                    {seg.plainText}
                                                </div>
                                            {/if}
                                        </div>
                                    {/if}
                                </div>
                            {/if}
                        </div>
                    {/if}
                </div>
                {#if previewEditMode && (!$transcriptStore.isDualModeActive || !seg.isPrimary)}
                    <div
                        class="flex items-center justify-center insert-button-wrapper relative px-4"
                    >
                        {#if seg.segmentIndex < $transcriptStore.segments.length - 1}
                            <button
                                class="btn-icon p-0.5 text-blue-500 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300 flex items-center justify-center absolute left-[8px]"
                                on:click={() =>
                                    handleMergeWithNext(seg.segmentIndex)}
                                title="Merge with next segment"
                                aria-label="Merge with next segment"
                            >
                                <FoldVertical class="w-5 h-5" />
                            </button>
                        {/if}
                        <button
                            class="btn-icon text-green-400 hover:text-green-600 dark:hover:text-green-300 flex items-center justify-center"
                            on:click={() =>
                                handleInsertNewSegment(seg.segmentIndex + 1)}
                            title="Insert New Segment"
                            aria-label="Insert New Segment"
                        >
                            <PlusSquare class="w-5 h-5" />
                        </button>
                    </div>
                {/if}
            {/each}
            <div style="height: {paddingBottom}px;"></div>
        </div>
    {/if}
</div>

<FindReplaceModal
    bind:showModal={showFindReplaceModal}
    bind:initialSearchTerm={searchTerm}
    currentMatchIndex={currentSearchResultIndex}
    totalMatches={searchResults.length}
    on:replace={handleReplace}
    on:replaceall={handleReplaceAll}
    on:findnext={navigateToNextResult}
    on:findprev={navigateToPreviousResult}
    on:findchange={(e) =>
        executeSearch(e.detail.term, {
            isCaseSensitive: e.detail.isCaseSensitive,
            isRegex: e.detail.isRegex,
            isWholeWord: e.detail.isWholeWord,
        })}
    on:close={() => (showFindReplaceModal = false)}
/>

<style lang="postcss">
    .btn-icon {
        @apply p-1 rounded focus:outline-none focus:ring-1 focus:ring-offset-1 focus:ring-blue-400 dark:focus:ring-blue-500 dark:ring-offset-gray-900 transition duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed;
    }
    .speech-plain-text {
        @apply leading-normal whitespace-pre-wrap text-gray-900 dark:text-gray-200 pt-px;
        padding: 0;
        margin: 0;
        overflow-wrap: break-word;
        word-break: normal;
        font-family: Arial, Helvetica, sans-serif;
        font-size: 12pt;
        line-height: 1.5;
    }
    .speech-rich-text {
        @apply leading-normal whitespace-pre-wrap text-gray-900 dark:text-gray-200 pt-px;
        padding: 0;
        margin: 0;
        overflow-wrap: break-word;
        word-break: normal;
        font-family: Arial, Helvetica, sans-serif;
        font-size: 12pt;
        line-height: 1.5;
    }
    .preview-content-area {
        overflow-wrap: break-word;
        word-break: normal;
    }

    /* Segment interactivity and states */
    .segment-block {
        transition:
            background-color 0.15s ease-in-out,
            border-color 0.15s ease-in-out;
    }
    .segment-block:not(.preview-interaction-disabled):not(
            .segment-active
        ):hover {
        @apply bg-blue-50 dark:bg-blue-900/30 border-blue-200 dark:border-blue-900;
    }
    .segment-block:not(.preview-interaction-disabled):focus {
        @apply ring-1 ring-blue-300 dark:ring-blue-600 border-blue-300 dark:border-blue-600 outline-none;
    }
    .secondary-segment {
        @apply bg-gray-50/50 dark:bg-gray-800/30 border-l-4 border-l-gray-300 dark:border-l-gray-600;
    }
    .secondary-segment.segment-active {
        @apply border-l-blue-500 dark:border-l-blue-400;
    }
    .preview-interaction-disabled {
        @apply cursor-default opacity-80;
    }

    /* Custom scrollbar styles */
    div[class*="overflow-y-auto"]::-webkit-scrollbar {
        @apply w-[8px] h-[8px];
    }
    div[class*="overflow-y-auto"]::-webkit-scrollbar-track {
        @apply bg-gray-100 dark:bg-gray-900 rounded-lg;
    }
    div[class*="overflow-y-auto"]::-webkit-scrollbar-thumb {
        @apply bg-gray-400 dark:bg-gray-700 rounded-lg border-2 border-solid border-gray-100 dark:border-gray-900;
    }
    div[class*="overflow-y-auto"]::-webkit-scrollbar-thumb:hover {
        @apply bg-gray-500 dark:bg-gray-600;
    }
    div[class*="overflow-y-auto"] {
        scrollbar-width: thin;
        scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
        scrollbar-gutter: stable;
    }
    :root {
        --scrollbar-thumb: rgba(160, 174, 192, 1);
        --scrollbar-track: rgba(243, 244, 246, 1);
    }

    :global(.dark .speech-rich-text) {
        color: white;
    }

    :global(.dark .speech-rich-text [style*="background-color"]) {
        color: black;
    }

    :global(.dark .speech-rich-text [style*="background-color: transparent"]) {
        color: white;
    }

    :global(html.dark .segment-block.hovering .speech-rich-text),
    :global(html.dark .segment-block.hovering .speech-rich-text *),
    :global(html.dark .segment-block.hovering .speech-plain-text),
    :global(html.dark .segment-block.hovering .speech-plain-text *),
    :global(html.dark .segment-block.hovering .flex-shrink-0),
    :global(html.dark .segment-block.hovering .flex-shrink-0 *),
    :global(html.dark .segment-block.hovering .flex-1),
    :global(html.dark .segment-block.hovering .flex-1 *) {
        color: black !important;
    }

    /* Search match highlights using CSS Custom Highlight API */
    :global(::highlight(search-match)) {
        background-color: rgba(255, 215, 0, 0.4);
        color: black;
    }

    :global(::highlight(search-match-active)) {
        background-color: rgba(255, 165, 0, 0.7);
        color: black;
    }

    :global(html.dark ::highlight(search-match)) {
        background-color: rgba(255, 215, 0, 0.3);
        color: white;
    }

    :global(html.dark ::highlight(search-match-active)) {
        background-color: rgba(255, 165, 0, 0.6);
        color: white;
    }
</style>
