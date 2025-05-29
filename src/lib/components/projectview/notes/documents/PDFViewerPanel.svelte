<!-- src/lib/components/projectview/documents/PDFViewerPanel.svelte -->
<script>
    import { onMount, onDestroy, tick, createEventDispatcher } from 'svelte';
    import { readFile } from '@tauri-apps/plugin-fs';
    import { v4 as uuidv4 } from 'uuid';
    import { project } from '$lib/stores/projectStore.js';
    import { saveCurrentPdfAnnotations } from '$lib/services/projectService.js';
import { markPdfAnnotationsDirty } from '$lib/stores/projectStore.js';
import { get } from 'svelte/store';

    const dispatch = createEventDispatcher();

    /* ─────────────────────────── Component state / props ─────────────────────────── */
    export let pdfPath = '';
    export let initialHighlights = [];
    // StoredHighlight structure expected in initialHighlights and dispatched:
    // { id: string, color: string, pageIndex: number, text: string, 
    //   prefix?: string, suffix?: string, occurrenceInPageContext?: number }

    let loading = true; let loadingMessage = 'Loading PDF...'; let error = null;
    let pdfDoc = null; let pdfViewer = null; let eventBus = null; let numPages = 0; let currentPageNum = 1; let currentScaleValue = 'auto'; // Default to auto
    const PRESET_SCALES = ['auto', 'page-actual', 'page-fit', 'page-width', '0.5', '0.75', '1', '1.25', '1.5', '2', '3', '4'];
    let viewerContainer; let viewerElement; let pdfViewerWrapperElement;

    let selectionToolbarElement;
    let showSelectionToolbar = false;
    let selectionToolbarTop = 0;
    let selectionToolbarLeft = 0;
    let toolbarMode = null; 
    let selectedRange = null; 
    let clickedHighlightId = null;
    let clickedHighlightColor = null;
    let hideToolbarTimeoutId = null;

    const highlightOptions = [
        { value: 'rgba(255, 242, 117, 0.5)', label: 'Yellow' },
        { value: 'rgba(168, 255, 158, 0.5)', label: 'Green' },
        { value: 'rgba(174, 239, 255, 0.5)', label: 'Blue' },
        { value: 'rgba(255, 176, 207, 0.5)', label: 'Pink' },
        { value: 'rgba(208, 160, 255, 0.5)', label: 'Purple' },
    ];
    let isToolbarHighlightDropdownOpen = false;
    let highlightDropdownRef;

    let pageRendering = false; let pageNumInput = currentPageNum; let pdfjsLib = null; let PDFViewer = null; let EventBus = null; let PDFLinkService = null; let PDFFindController = null; 
    // pdfWorkerUrl will be imported dynamically

    let searchQuery = ''; let lastSearched = '';
    let currentFindState = 0;

    let undoStack = [];
    let redoStack = [];
    const CONTEXT_LENGTH = 30;
    let initialHighlightsApplied = false;
    let pdfJsStyleElement = null;

    // --- Helper: Capture DOM Range Data for Undo/Redo (Visual Only) ---
    function captureRangeDataForUndo(range) {
        if (!range) return null;
        return {
            text: range.toString(),
            clonedRange: range.cloneRange()
        };
    }

    // --- Helper: Escape Regex ---
    function escapeRegExp(string) {
        return string.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    }

    // --- Helper: Normalize text for matching (Unicode-aware, ligatures, hyphenation, dashes, quotes, ellipsis) ---
    function normalizeTextForMatching(text) {
        if (!text) return "";
        // Unicode compatibility decomposition replaces ligatures (e.g., ﬁ, ﬀ, ﬂ, ﬃ, ﬄ).
        let result = text.normalize('NFKC');
        // Remove soft hyphens introduced by PDF hyphenation.
        result = result.replace(/\u00AD/g, '');
        // Replace non-breaking spaces with regular spaces.
        result = result.replace(/[\u00A0\u2007\u202F]/g, ' ');
        // Normalize ellipsis, dashes, and curly quotes to ASCII.
        result = result
            .replace(/\u2026/g, '...')       // ellipsis
            .replace(/[–—]/g, '-')           // en dash, em dash
            .replace(/[‘’]/g, "'")           // curly single quotes
            .replace(/[“”]/g, '"');          // curly double quotes
        // Remove hyphens at line breaks only, keep whitespace as-is.
        result = result.replace(/-\s+/g, '');
        return result;
    }

    // --- Helper: Get Page Info from a DOM Range ---
    function getRangePageInfo(range) {
        if (!range || !viewerElement || !pdfViewer) return { pageIndex: -1, pageElement: null, pageView: null };
        const pageDiv = (range.commonAncestorContainer.nodeType === Node.ELEMENT_NODE ? range.commonAncestorContainer : range.commonAncestorContainer.parentNode)?.closest('.page');
        if (pageDiv && pageDiv.dataset.pageNumber) {
            const pageNumber = parseInt(pageDiv.dataset.pageNumber, 10);
            const pageView = pdfViewer.getPageView(pageNumber - 1);
            return { pageIndex: pageNumber - 1, pageElement: pageDiv, pageView };
        }
        // console.warn("[getRangePageInfo] Could not find page for range.");
        return { pageIndex: -1, pageElement: null, pageView: null };
    }

    // Helper function to finalize PageView once textLayerDiv is confirmed
    async function finalizePageView(pv, pageIndex) {
        if (!pv) return null; // Should not happen if called correctly

        if (pv.div?.querySelector('.textLayer')) { // Ensure div exists for textLayer
            if (pv.textLayer && !pv.textLayer.textLayerDiv) {
                pv.textLayer.textLayerDiv = pv.div.querySelector('.textLayer');
            } else if (!pv.textLayer) {
                pv.textLayer = { 
                    textLayerDiv: pv.div.querySelector('.textLayer'), 
                    renderingDone: false, // PDF.js might update this
                    textContentItemsStr: [] // PDF.js might update this
                };
            }
        }
        // Ensure pdfPage is loaded, as it's needed for getTextContent
        if (!pv.pdfPage && pdfDoc) {
            try { 
                pv.pdfPage = await pdfDoc.getPage(pageIndex + 1); 
            } catch (e) { 
                console.error(`[finalizePageView] Failed to get pdfPage for page ${pageIndex + 1}: ${e.message}`);
                throw new Error(`Failed to get pdfPage for page ${pageIndex + 1}: ${e.message}`); 
            }
        }
        return pv;
    }

    async function ensureTextLayerReady(pageViewFromCaller, pageIndex) {
        let pv = pageViewFromCaller || pdfViewer?.getPageView(pageIndex);

        // Check 1: Is it already good?
        if (pv?.textLayer?.textLayerDiv || pv?.div?.querySelector('.textLayer')) {
            return finalizePageView(pv, pageIndex);
        }

        // If PageView object itself doesn't exist, try to make it exist.
        if (!pv) {
            try {
                pdfViewer?.scrollPageIntoView({ pageNumber: pageIndex + 1 });
            } catch (_) { /* ignore scroll errors if viewer is busy */ }
            await new Promise(r => setTimeout(r, 350)); // Initial wait for PageView creation
            pv = pdfViewer?.getPageView(pageIndex);
            if (!pv) {
                console.error(`[ensureTextLayerReady] Critical: No PageView object for page ${pageIndex + 1} after initial scroll.`);
                throw new Error(`No PageView object for page ${pageIndex + 1}`);
            }
            // Re-check after PageView creation
            if (pv.textLayer?.textLayerDiv || pv.div?.querySelector('.textLayer')) {
                return finalizePageView(pv, pageIndex);
            }
        }
        
        // Check 2: PageView exists, but textLayer not ready. Try scrolling and waiting a bit more.
        // This also covers the case where pv was just created above but its textLayer isn't ready.
        if (pv) { // pv should exist here
            try {
                pdfViewer?.scrollPageIntoView({ pageNumber: pageIndex + 1 });
            } catch (_) { /* ignore scroll errors */ }

            await new Promise(r => setTimeout(r, 600)); // Wait a bit longer after scroll
            let freshPv = pdfViewer?.getPageView(pageIndex); 
            if (freshPv && (freshPv.textLayer?.textLayerDiv || freshPv.div?.querySelector('.textLayer'))) {
                return finalizePageView(freshPv, pageIndex);
            }
            pv = freshPv || pv; // Use freshPv if available, otherwise fallback to the pv we had.
        }

        // Check 3: Fallback to polling
        return new Promise((resolve, reject) => {
            const MAX_WAIT_TEXTLAYER = 30000; // Further Increased timeout to 30 seconds
            let totalWait = 0;
            const interval = 250; // Slightly increased interval
            let checkIntervalId = null;

            const cleanupInterval = () => {
                if (checkIntervalId) { clearInterval(checkIntervalId); checkIntervalId = null; }
            };

            const checkTextLayerAvailability = async () => {
                if (!pdfViewer) { // Guard against pdfViewer being null
                    cleanupInterval();
                    console.warn(`[ensureTextLayerReady] pdfViewer became null during check for page ${pageIndex + 1}. Aborting text layer wait.`);
                    reject(new Error(`pdfViewer became null while waiting for textLayer on page ${pageIndex + 1}`));
                    return;
                }

                let currentPolledPv = pdfViewer.getPageView(pageIndex); // Always get fresh PageView
                let div = currentPolledPv?.textLayer?.textLayerDiv || currentPolledPv?.div?.querySelector('.textLayer');

                // console.debug(`[ensureTextLayerReady Polling] Page ${pageIndex + 1}: totalWait=${totalWait}, pv=${!!currentPolledPv}, div=${!!div}`);
                if (div) {
                    cleanupInterval();
                    try {
                        const finalPv = await finalizePageView(currentPolledPv, pageIndex);
                        resolve(finalPv);
                    } catch (e) {
                        reject(e); // finalizePageView might throw if getPage fails
                    }
                } else {
                    totalWait += interval;
                    if (totalWait >= MAX_WAIT_TEXTLAYER) {
                        cleanupInterval(); // Ensure interval is cleared on timeout
                        console.error(`[ensureTextLayerReady Polling] Timeout waiting for textLayer div on page ${pageIndex + 1} after ${MAX_WAIT_TEXTLAYER}ms.`);
                        reject(new Error(`Timeout waiting for textLayer div on page ${pageIndex + 1} (polling)`));
                    } else if (!currentPolledPv && pdfViewer) { // Check if pageView itself disappeared
                        // This case might occur if the PDF document is changed/closed during polling.
                        // The !pdfViewer check at the top handles viewer disappearing, but this is a specific PageView check.
                        cleanupInterval();
                        console.error(`[ensureTextLayerReady Polling] PageView for page ${pageIndex + 1} became null.`);
                        reject(new Error(`PageView for page ${pageIndex + 1} became null during polling`));
                    }
                }
            };
            // Start the interval
            checkIntervalId = setInterval(checkTextLayerAvailability, interval);
            checkTextLayerAvailability(); // Perform an initial check immediately
        });
    }
    
    async function getContextualDataForRange(range, pageIndex) {
        if (!range || pageIndex < 0 || !pdfViewer || !pdfDoc) {
            return { prefix: '', suffix: '', occurrenceInPageContext: 0 };
        }
        const rawSelectedText = range.toString().trim();
        const normalizedSelectedText = normalizeTextForMatching(rawSelectedText);
        if (!normalizedSelectedText) return { prefix: '', suffix: '', occurrenceInPageContext: 0 };

        let pageView, pdfPage, pageTextLayerDiv;
        try {
            pageView = pdfViewer.getPageView(pageIndex);
            // Pass the potentially existing pageView to ensureTextLayerReady
            pageView = await ensureTextLayerReady(pageView, pageIndex); 
            if (!pageView) { // Should be caught by ensureTextLayerReady throwing, but as a safeguard:
                throw new Error(`ensureTextLayerReady returned null for page ${pageIndex + 1}`);
            }

            pdfPage = pageView.pdfPage;
            pageTextLayerDiv = pageView.textLayer?.textLayerDiv;

            if (!pdfPage) throw new Error(`pdfPage could not be obtained for page ${pageIndex + 1}`);
            
            const textContent = await pdfPage.getTextContent({ normalizeWhitespace: true, includeMarkedContent: false });
            if (!textContent?.items?.length) return { prefix: '', suffix: '', occurrenceInPageContext: 0 };

            const rawFullPageText = textContent.items.map(item => item.str).join('');
            let prefix = '', suffix = '', occurrenceInPageContext = 0, selectionStartInPage = -1;
            // Use normalized full page text for matching context and occurrence calculation
            const normalizedFullPageText = normalizeTextForMatching(rawFullPageText);

            if (pageTextLayerDiv) {
                const walker = document.createTreeWalker(pageTextLayerDiv, NodeFilter.SHOW_TEXT, null, false);
                let currentNode, currentOffset = 0;
                while (currentNode = walker.nextNode()) {
                    if (range.startContainer.isSameNode(currentNode)) {
                        selectionStartInPage = currentOffset + range.startOffset;
                        break;
                    }
                    currentOffset += currentNode.textContent.length;
                    // selectionStartInPage is an offset in the *raw* concatenated DOM text.
                }
            }

            // Determine prefix and suffix from the normalized full page text
            // We need to estimate where the normalizedSelectedText starts in normalizedFullPageText
            let estimatedNormalizedSelectionStart = -1;
            if (selectionStartInPage !== -1) {
                // This is an approximation: normalize the part of raw text leading up to the selection
                estimatedNormalizedSelectionStart = normalizeTextForMatching(rawFullPageText.substring(0, selectionStartInPage)).length;
            }

            if (selectionStartInPage !== -1) {
                prefix = normalizedFullPageText.substring(Math.max(0, estimatedNormalizedSelectionStart - CONTEXT_LENGTH), estimatedNormalizedSelectionStart);
                const normalizedSelectionEnd = estimatedNormalizedSelectionStart + normalizedSelectedText.length;
                suffix = normalizedFullPageText.substring(normalizedSelectionEnd, Math.min(normalizedFullPageText.length, normalizedSelectionEnd + CONTEXT_LENGTH));
            } else { // Fallback if we couldn't get a precise start (e.g. selection spans multiple complex nodes)
                // Try to find the text and derive context around its first occurrence if no specific offset.
                const firstMatchIndex = normalizedFullPageText.indexOf(normalizedSelectedText);
                if (firstMatchIndex !== -1) {
                    prefix = normalizedFullPageText.substring(Math.max(0, firstMatchIndex - CONTEXT_LENGTH), firstMatchIndex);
                    const endOfText = firstMatchIndex + normalizedSelectedText.length;
                    suffix = normalizedFullPageText.substring(endOfText, Math.min(normalizedFullPageText.length, endOfText + CONTEXT_LENGTH));
                }
            }

            // Calculate occurrenceInPageContext using normalized values
            let count = 0;
            const targetRegex = new RegExp((prefix ? escapeRegExp(prefix) : "") + `(${escapeRegExp(normalizedSelectedText)})` + (suffix ? escapeRegExp(suffix) : ""), 'g');
            let match;
            while ((match = targetRegex.exec(normalizedFullPageText)) !== null) {
                const currentMatchStartForSelectedText = match.index + (prefix ? prefix.length : 0);
                if (estimatedNormalizedSelectionStart !== -1 && currentMatchStartForSelectedText === estimatedNormalizedSelectionStart) {
                    occurrenceInPageContext = count;
                    break;
                } else if (estimatedNormalizedSelectionStart === -1 && count === 0) { // If no specific start, assume the first found occurrence is the target
                    occurrenceInPageContext = count; // Should be 0
                    break;
                }
                count++;
            }
            return { prefix, suffix, occurrenceInPageContext }; // prefix & suffix are normalized
        } catch (e) {
            console.error(`[getContextualDataForRange] Error for page ${pageIndex + 1}:`, e.message);
            return { prefix: '', suffix: '', occurrenceInPageContext: 0 };
        }
    }

    async function createHighlightDataForStorage(id, range, color) {
        if (!range) return null;
        const rawText = range.toString().trim();
        const normalizedText = normalizeTextForMatching(rawText); // Normalize text before storage
        if (!normalizedText) return null;

        const { pageIndex } = getRangePageInfo(range);
        let actualPageIndex = pageIndex;
        if (pageIndex === -1) {
            actualPageIndex = pdfViewer?.currentPageNumber ? pdfViewer.currentPageNumber - 1 : 0;
        }
        // getContextualDataForRange now returns normalized prefix/suffix
        const { prefix, suffix, occurrenceInPageContext } = await getContextualDataForRange(range, actualPageIndex);
        return { id, type: 'pdfHighlight', color, text: normalizedText, pageIndex: actualPageIndex, prefix, suffix, occurrenceInPageContext };
    }
    
    function recordAction(type, payload) {
        undoStack.push({ type, payload });
        if (undoStack.length > 50) undoStack.shift();
        redoStack = [];
    }

    function undo() {
        if (undoStack.length === 0) return;
        const action = undoStack.pop();
        redoStack.push(action);
        // console.log('[PDF Undo] Action:', action.type, action.payload?.id);
        switch (action.type) {
            case 'addHighlight':
                removeClickedHighlightBlockDOM(action.payload.id);
                dispatch('pdfhighlightevent', { type: 'remove', id: action.payload.id });
                break;
            case 'removeHighlight':
                if (action.payload.rangeData?.clonedRange && action.payload.color && action.payload.id && action.payload.dataForStorage) {
                    applyHighlightToSelectionDOM(action.payload.rangeData.clonedRange, action.payload.color, action.payload.id);
                    dispatch('pdfhighlightevent', { type: 'add', ...action.payload.dataForStorage });
                } else { console.warn('[PDF Undo] Cannot re-apply highlight (was remove), missing DOM range or full data.', action.payload); }
                break;
            case 'changeColor':
                if (action.payload.id && action.payload.oldColor && action.payload.dataForStorage) {
                    changeClickedHighlightColorDOM(action.payload.id, action.payload.oldColor);
                    dispatch('pdfhighlightevent', { type: 'update', ...action.payload.dataForStorage, color: action.payload.oldColor });
                } else { console.warn('[PDF Undo] Cannot revert color change, missing data.', action.payload); }
                break;
        }
        hideSelectionToolbar();
    }

    function redo() {
        if (redoStack.length === 0) return;
        const action = redoStack.pop();
        undoStack.push(action);
        // console.log('[PDF Redo] Action:', action.type, action.payload?.id);
        switch (action.type) {
            case 'addHighlight':
                if (action.payload.rangeData?.clonedRange && action.payload.color && action.payload.id && action.payload.dataForStorage) {
                    applyHighlightToSelectionDOM(action.payload.rangeData.clonedRange, action.payload.color, action.payload.id);
                    dispatch('pdfhighlightevent', { type: 'add', ...action.payload.dataForStorage });
                } else { console.warn('[PDF Redo] Cannot re-apply highlight (was add), missing data.', action.payload); }
                break;
            case 'removeHighlight':
                removeClickedHighlightBlockDOM(action.payload.id);
                dispatch('pdfhighlightevent', { type: 'remove', id: action.payload.id });
                break;
            case 'changeColor':
                 if (action.payload.id && action.payload.newColor && action.payload.dataForStorage) {
                    changeClickedHighlightColorDOM(action.payload.id, action.payload.newColor);
                    dispatch('pdfhighlightevent', { type: 'update', ...action.payload.dataForStorage, color: action.payload.newColor });
                } else { console.warn('[PDF Redo] Cannot re-apply color change, missing data.', action.payload); }
                break;
        }
        hideSelectionToolbar();
    }
    
    function handleKeydown(e) {
      if (e.metaKey && !e.shiftKey && e.key === 'z') { undo(); e.preventDefault(); } 
      else if (e.metaKey && (e.key === 'y' || (e.shiftKey && e.key === 'z'))) { redo(); e.preventDefault(); }
    }

    onMount(async () => {
        const res = await fetch('/pdfjs/pdf_viewer.css');
        let css = await res.text();
        css = css.replace(/(^|\})\s*([^{]+)/g, `$1 .pdf-viewer-panel-root $2`);
        pdfJsStyleElement = document.createElement('style');
        pdfJsStyleElement.textContent = css;
        pdfJsStyleElement.dataset.pdfjsScoped = 'true'; 
        document.head.appendChild(pdfJsStyleElement);
        if (!pdfPath) { error = 'No PDF path provided.'; loading = false; return; }
        setTimeout(async () => {
            if (!viewerContainer || !viewerElement || !pdfViewerWrapperElement) { console.error('[PDFViewerPanel] Required container elements null on mount.'); error = 'Failed init viewer elements.'; loading = false; return; }
            await loadPdfAndLibraries(viewerContainer);
            document.addEventListener('click', handleClickOutside);
            viewerContainer?.addEventListener('mouseup', handleViewerMouseUp);
            viewerContainer?.addEventListener('click', handleViewerClick);
            viewerContainer?.addEventListener('mousedown', handleViewerMouseDown, true);
            window.addEventListener('keydown', handleKeydown);
        }, 100); 
    });

    onDestroy(() => {
        if (pdfJsStyleElement && pdfJsStyleElement.parentNode) { pdfJsStyleElement.parentNode.removeChild(pdfJsStyleElement); }
        document.removeEventListener('click', handleClickOutside);
        viewerContainer?.removeEventListener('mouseup', handleViewerMouseUp);
        viewerContainer?.removeEventListener('click', handleViewerClick);
        viewerContainer?.removeEventListener('mousedown', handleViewerMouseDown, true);
        window.removeEventListener('keydown', handleKeydown);
        clearTimeout(hideToolbarTimeoutId);
        if (eventBus && typeof eventBus.destroy === 'function') { eventBus.destroy(); } eventBus = null; 
        if (pdfViewer) { pdfViewer.cleanup(); pdfViewer.setDocument(null); pdfViewer = null; } 
        if (pdfDoc) { pdfDoc.destroy(); pdfDoc = null; }
    });

    function handleClickOutside(event) {
        if (isToolbarHighlightDropdownOpen && highlightDropdownRef && !highlightDropdownRef.contains(event.target) && !event.target.closest('[role="menuitem"]')) {
            isToolbarHighlightDropdownOpen = false; selectedRange = null;
        }
        if (showSelectionToolbar && selectionToolbarElement && !selectionToolbarElement.contains(event.target) && !(highlightDropdownRef && highlightDropdownRef.contains(event.target))) {
            const isInsideViewer = viewerElement?.contains(event.target);
            const isHighlight = event.target.closest?.('.pdf-highlight');
            if (!isInsideViewer || (!isHighlight && window.getSelection()?.isCollapsed)) { hideSelectionToolbar(); }
        }
    }

    async function handleViewerMouseUp(event) {
        if (selectionToolbarElement?.contains(event.target) || highlightDropdownRef?.contains(event.target)) return;
        await tick(); // Allow selection to finalize
        setTimeout(async () => {
            selectedRange = null; 
            const sel = window.getSelection();
            if (sel && sel.rangeCount > 0 && !sel.isCollapsed) {
                const range = sel.getRangeAt(0);
                let isInTextLayer = false;
                const ancestor = range.commonAncestorContainer;
                if (ancestor && viewerElement?.contains(ancestor)) {
                    const textLayerParent = (ancestor.nodeType === Node.ELEMENT_NODE ? ancestor : ancestor.parentNode)?.closest('.textLayer');
                    if (textLayerParent && viewerElement.contains(textLayerParent)) isInTextLayer = true;
                }
                if (isInTextLayer && range.toString().trim().length > 0) {
                    clearTimeout(hideToolbarTimeoutId); selectedRange = range.cloneRange(); 
                    clickedHighlightId = null; clickedHighlightColor = null; toolbarMode = 'selection';
                    showSelectionToolbar = true; 
                    await tick(); 
                    positionAndShowSelectionToolbar(selectedRange); 
                    return;
                }
            }
            selectedRange = null; if (toolbarMode === 'selection' && showSelectionToolbar) hideSelectionToolbar();
        }, 30); // Reduced delay
    }

    async function handleViewerClick(event) {
        if (selectionToolbarElement?.contains(event.target) || highlightDropdownRef?.contains(event.target)) return;
        const highlightSpan = event.target.closest?.('.pdf-highlight');
        const sel = window.getSelection();
        if (highlightSpan && viewerContainer.contains(highlightSpan)) {
            clearTimeout(hideToolbarTimeoutId);
            const id = highlightSpan.dataset.hlId; const color = highlightSpan.dataset.hlColor;
            if (id !== clickedHighlightId || !showSelectionToolbar) {
                clickedHighlightId = id; clickedHighlightColor = color; selectedRange = null; toolbarMode = 'click';
                const clickRange = document.createRange(); clickRange.selectNodeContents(highlightSpan);
                showSelectionToolbar = true; 
                await tick(); 
                positionAndShowSelectionToolbar(clickRange); 
            }
            event.stopPropagation();
        } else if (!sel || sel.isCollapsed) { hideSelectionToolbar(); }
    }

    async function positionAndShowSelectionToolbar(range) {
        if (!selectionToolbarElement || !viewerContainer || !range || !pdfViewerWrapperElement) { hideSelectionToolbar(); return; }
        selectionToolbarElement.style.display = 'flex'; selectionToolbarElement.style.opacity = '0'; selectionToolbarElement.style.visibility = 'hidden';
        await tick(); 
        requestAnimationFrame(() => {
            if (!selectionToolbarElement || !viewerContainer || !pdfViewerWrapperElement || !range) { hideSelectionToolbar(); return; }
            const containerRect = pdfViewerWrapperElement.getBoundingClientRect(); const clientRects = range.getClientRects();
            if (!clientRects || clientRects.length === 0) { hideSelectionToolbar(); return; }
            const firstRect = clientRects[0]; const toolbarHeight = selectionToolbarElement.offsetHeight; const toolbarWidth = selectionToolbarElement.offsetWidth;
            if (!toolbarHeight || !toolbarWidth) { hideSelectionToolbar(); return; }
            let targetTop = firstRect.top - containerRect.top - toolbarHeight - 8;
            let targetLeft = firstRect.left - containerRect.left + (firstRect.width / 2) - (toolbarWidth / 2);
            targetLeft = Math.max(0, targetLeft); targetLeft = Math.min(containerRect.width - toolbarWidth - 5, targetLeft);
            if (targetTop < 0 || (clientRects.length > 1 && firstRect.bottom > clientRects[clientRects.length -1].top )) {
                 let topBelow = (clientRects.length > 1 ? clientRects[clientRects.length -1].bottom : firstRect.bottom) - containerRect.top + 8;
                 targetTop = (topBelow + toolbarHeight > containerRect.height - 5 && targetTop >=0) ? targetTop : topBelow;
                 if(topBelow + toolbarHeight > containerRect.height -5 && targetTop < 0) targetTop = 0;
            }
            selectionToolbarTop = targetTop; selectionToolbarLeft = targetLeft;
            selectionToolbarElement.style.top = `${targetTop}px`; selectionToolbarElement.style.left = `${targetLeft}px`;
            selectionToolbarElement.style.opacity = '1'; selectionToolbarElement.style.visibility = 'visible';
        });
    }

    function hideSelectionToolbar() {
        if (showSelectionToolbar) {
            clearTimeout(hideToolbarTimeoutId); showSelectionToolbar = false;
            toolbarMode = null; selectedRange = null; clickedHighlightId = null; clickedHighlightColor = null;
        }
    }
    function handleToolbarMouseEnter() { clearTimeout(hideToolbarTimeoutId); }
    function handleToolbarMouseLeave() { clearTimeout(hideToolbarTimeoutId); hideToolbarTimeoutId = setTimeout(hideSelectionToolbar, 500); }

    async function handleHighlightAction(color) {
        await tick();
        let success = false;
        let actionPayload = { rangeData: null, dataForStorage: null };

        if (toolbarMode === 'selection') {
            if (!selectedRange) { console.warn("Highlight Action (Selection Mode): No stored selection range found."); return; }
            const rangeToUse = selectedRange.cloneRange();
            actionPayload.rangeData = captureRangeDataForUndo(rangeToUse);
            actionPayload.text = actionPayload.rangeData.text;

            if (color === 'remove') {
                const affectedHighlights = await getAffectedHighlightsData(rangeToUse); // Now async
                removeHighlightFromSelectionDOM(rangeToUse);
                affectedHighlights.forEach(hlStorageData => {
                    dispatch('pdfhighlightevent', { type: 'remove', id: hlStorageData.id });
                    recordAction('removeHighlight', { ...hlStorageData, rangeData: null /* DOM range hard to restore for this */ });
                });
                // mark dirty & persist (selection mode)
                await tick(); // wait for store update from dispatched events
                markPdfAnnotationsDirty(get(project).currentPdfAnnotations);
                saveCurrentPdfAnnotations();
                if (affectedHighlights.length > 0) success = true;
            } else {
                actionPayload.id = `hl-${uuidv4()}`;
                actionPayload.color = color;
                applyHighlightToSelectionDOM(rangeToUse, color, actionPayload.id);
                actionPayload.dataForStorage = await createHighlightDataForStorage(actionPayload.id, rangeToUse, color);
                if (actionPayload.dataForStorage) {
                    dispatch('pdfhighlightevent', { type: 'add', ...actionPayload.dataForStorage });
                    recordAction('addHighlight', actionPayload);
                    // mark dirty & persist
                    markPdfAnnotationsDirty();
                    saveCurrentPdfAnnotations();
                    success = true;
                }
            }
        } else if (toolbarMode === 'click') {
            if (!clickedHighlightId) { console.warn("Highlight Action (Click Mode): clickedHighlightId is null!"); return; }
            actionPayload.id = clickedHighlightId;
            actionPayload.oldColor = clickedHighlightColor;
            actionPayload.text = getTextOfHighlightId(clickedHighlightId);

            // No longer recompute dataForStorage for color changes; use original highlight data from initialHighlights

            if (color === 'remove') {
                const clickedSpan = viewerContainer.querySelector(`.pdf-highlight[data-hl-id="${clickedHighlightId}"]`);
                let tempRangeForContext = null;
                if (clickedSpan) { tempRangeForContext = document.createRange(); tempRangeForContext.selectNodeContents(clickedSpan); }
                // For remove, still need to provide dataForStorage for undo stack
                // Use original highlight data if available, fallback to minimal
                const originalHighlight = initialHighlights.find(h => h.id === clickedHighlightId);
                const dataForStorage = originalHighlight
                    ? { ...originalHighlight }
                    : { id: clickedHighlightId, type: 'pdfHighlight', color: actionPayload.oldColor, text: actionPayload.text, pageIndex: 0, prefix: '', suffix: '', occurrenceInPageContext: 0 };
                removeClickedHighlightBlockDOM(clickedHighlightId);
                dispatch('pdfhighlightevent', { type: 'remove', id: clickedHighlightId });
                recordAction('removeHighlight', { ...actionPayload, color: actionPayload.oldColor, dataForStorage: { ...dataForStorage, color: actionPayload.oldColor } });
                // mark dirty & persist (click mode)
                await tick(); // ensure store is updated after dispatched remove event
                markPdfAnnotationsDirty(get(project).currentPdfAnnotations);
                saveCurrentPdfAnnotations();
                success = true;
            } else {
                actionPayload.newColor = color;
                changeClickedHighlightColorDOM(clickedHighlightId, color);
                // Update only the color of the existing highlight data
                const originalHighlight = initialHighlights.find(h => h.id === clickedHighlightId);
                if (originalHighlight) {
                    dispatch('pdfhighlightevent', { type: 'update', ...originalHighlight, color });
                } else {
                    console.warn(`[PDFViewerPanel] changeColor: original data not found for ID ${clickedHighlightId}`);
                }
                recordAction('changeColor', actionPayload);
                // mark dirty & persist
                markPdfAnnotationsDirty();
                saveCurrentPdfAnnotations();
                success = true;
            }
        } else { console.warn("Highlight Action: Invalid toolbarMode:", toolbarMode); }

        if (success) {
            hideSelectionToolbar();
            window.getSelection()?.removeAllRanges();
        }
    }
    
    async function handleDropdownHighlightAction(color) {
        if (!selectedRange) { isToolbarHighlightDropdownOpen = false; return; }
        // selectedRange is already cloned from mouseup
        isToolbarHighlightDropdownOpen = false; 
        toolbarMode = 'selection'; // Ensure mode is set for handleHighlightAction
        await handleHighlightAction(color); // Pass the selectedRange implicitly
        selectedRange = null; // Clear after use
    }
    
    // --- Helper: Get the combined text of a highlight block by its id ---
    function getTextOfHighlightId(hlId) {
        if (!hlId || !viewerContainer) return '';
        const spans = viewerContainer.querySelectorAll(`.pdf-highlight[data-hl-id="${hlId}"]`);
        if (!spans || spans.length === 0) return '';
        return Array.from(spans).map(s => s.textContent).join('');
    }

    /**
     * When the user begins a drag on whitespace inside the page, move the caret
     * to the nearest text span so selection behaves like Preview/Acrobat.
     */
    function handleViewerMouseDown(event) {
        // Only react to primary button inside the viewerContainer
        if (event.button !== 0) return;
        if (!viewerContainer || !viewerContainer.contains(event.target)) return;

        // If the target is already text or a highlight span, let browser handle it
        const looksLikeText = n =>
            n?.nodeType === Node.TEXT_NODE ||
            n?.classList?.contains('pdf-highlight');
        if (looksLikeText(event.target) || looksLikeText(event.target.firstChild)) return;

        const { clientX: x, clientY: y } = event;
        // Try elementFromPoint first
        let span = document.elementFromPoint(x, y);

        while (span && span !== viewerContainer &&
               !span.classList?.contains('pdf-highlight') &&
               span.firstChild?.nodeType !== Node.TEXT_NODE) {
            span = span.parentElement;
        }

        // Fallback: find nearest span in page (no distance threshold)
        if (!span || span === viewerContainer) {
            const spans = viewerContainer.querySelectorAll('.textLayer span');
            let best = null, bestDist = Infinity;
            spans.forEach(s => {
                const r = s.getBoundingClientRect();
                if (!r.width || !r.height) return;
                const cx = Math.max(r.left, Math.min(x, r.right));
                const cy = Math.max(r.top,  Math.min(y, r.bottom));
                const dist = (cx - x) ** 2 + (cy - y) ** 2;
                if (dist < bestDist) { bestDist = dist; best = s; }
            });
            span = best; // pick the closest span—even if far from the click
        }

        if (span && span.firstChild?.nodeType === Node.TEXT_NODE) {
            // Defer caret correction until after the browser's default anchor,
            // so we don't have to cancel the native selection behaviour.
            requestAnimationFrame(() => {
                const sel = window.getSelection();
                if (!sel) return;
                const range = document.createRange();
                range.setStart(span.firstChild, 0);
                range.collapse(true);
                sel.removeAllRanges();
                sel.addRange(range);
            });
        }
    }



    // DOM Manipulation functions as provided previously, renamed for clarity
    // applyHighlightToSelectionDOM, handleHighlightingForNodeSegmentDOM, splitSpanAndApplyHighlightDOM,
    // removeHighlightFromSelectionDOM, removePartialHighlightDOM, splitSpanAndUnwrapSegmentDOM,
    // removeClickedHighlightBlockDOM, changeClickedHighlightColorDOM, unwrapNodeDOM
    // These functions are assumed to be the ones from the user's "complete" example, or my refined versions.
    // For brevity here, I'm not repeating them if they are identical to what I last provided that "worked".
    // Ensure the names used in handleHighlightAction match the actual function names.
    // For example, if you have 'applyHighlightToSelection', use that. I'll use '...DOM' versions from my last working suggestion.
    
    // Re-inserting refined DOM manipulation functions from previous correct version
    function applyHighlightToSelectionDOM(range, color, overrideId) {
        // console.log('[applyHighlightToSelectionDOM] CALLED. Range Text:', range?.toString().substring(0, 70), 'Color:', color, 'ID:', overrideId);
        if (!range || range.collapsed || !color || !viewerElement) { return null; }
        removePartialHighlightDOM(range); 
        const uniqueId = overrideId || `hl-${uuidv4()}`;
        const nodesToProcess = [];
        try {
            const commonAncestor = range.commonAncestorContainer;
            const commonAncestorTextLayer = (commonAncestor.nodeType === Node.ELEMENT_NODE ? commonAncestor : commonAncestor.parentNode)?.closest('.textLayer');
            if (!commonAncestorTextLayer) { return null; }
            const walker = document.createTreeWalker(commonAncestorTextLayer, NodeFilter.SHOW_TEXT, { 
                acceptNode: (node) => {
                    const nodeRange = document.createRange();
                    nodeRange.selectNodeContents(node);
                    return !(range.compareBoundaryPoints(Range.END_TO_START, nodeRange) >= 0 || range.compareBoundaryPoints(Range.START_TO_END, nodeRange) <= 0) ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_REJECT;
                } 
            });
            let currentNode; while (currentNode = walker.nextNode()) { nodesToProcess.push(currentNode); }
        } catch (e) { console.error("[applyHighlightToSelectionDOM] Walker error:", e); return null; }
        // console.log('[applyHighlightToSelectionDOM] Nodes to process count:', nodesToProcess.length);
        if (nodesToProcess.length === 0) { return null; }

        nodesToProcess.forEach((textNode, idx) => {
            const nodeFullRange = document.createRange(); nodeFullRange.selectNodeContents(textNode);
            const actualIntersectionRange = document.createRange();
            actualIntersectionRange.setStart(
                range.compareBoundaryPoints(Range.START_TO_START, nodeFullRange) > 0 ? range.startContainer : nodeFullRange.startContainer,
                range.compareBoundaryPoints(Range.START_TO_START, nodeFullRange) > 0 ? range.startOffset : nodeFullRange.startOffset
            );
            actualIntersectionRange.setEnd(
                range.compareBoundaryPoints(Range.END_TO_END, nodeFullRange) < 0 ? range.endContainer : nodeFullRange.endContainer,
                range.compareBoundaryPoints(Range.END_TO_END, nodeFullRange) < 0 ? range.endOffset : nodeFullRange.endOffset
            );
            
            // Further refinement to ensure offsets are within the current textNode
            if (actualIntersectionRange.startContainer !== textNode) actualIntersectionRange.setStart(textNode, 0);
            if (actualIntersectionRange.endContainer !== textNode) actualIntersectionRange.setEnd(textNode, textNode.textContent.length);


            // console.log(`[applyHighlightToSelectionDOM] Node ${idx} ("${textNode.textContent.substring(0,30)}"). Actual Intersection: "${actualIntersectionRange.toString().substring(0,30)}", Collapsed? ${actualIntersectionRange.collapsed}`);
            if (!actualIntersectionRange.collapsed) { 
                handleHighlightingForNodeSegmentDOM(textNode, actualIntersectionRange, color, uniqueId); 
            }
        });
        try { range.commonAncestorContainer?.closest('.textLayer')?.normalize?.(); } 
        catch (e) { console.warn("[applyHighlightToSelectionDOM] Final normalize apply failed:", e); }
        // console.log('[applyHighlightToSelectionDOM] FINISHED. ID created/used:', uniqueId);
        return uniqueId;
    }

    function handleHighlightingForNodeSegmentDOM(textNode, segmentRange, newColor, newId) {
        // console.log('[handleHighlightingForNodeSegmentDOM] Node:', textNode.textContent.substring(0,30), 'Segment:', segmentRange.toString().substring(0,30), 'Color:', newColor);
        const parent = textNode.parentNode;
        if (parent && parent.nodeName === 'SPAN' && parent.classList.contains('pdf-highlight')) {
            // console.log('[handleHighlightingForNodeSegmentDOM] Parent is existing highlight. Splitting.');
            const existingColor = parent.dataset.hlColor; const existingId = parent.dataset.hlId;
            if (existingColor === newColor && existingId === newId) { return; } 
            splitSpanAndApplyHighlightDOM(parent, segmentRange, newColor, newId);
        } else {
            // console.log('[handleHighlightingForNodeSegmentDOM] Creating new highlight span.');
            const highlightSpan = document.createElement('span');
            highlightSpan.className = 'pdf-highlight'; highlightSpan.dataset.hlId = newId; highlightSpan.dataset.hlColor = newColor; highlightSpan.style.backgroundColor = newColor;
            try { segmentRange.surroundContents(highlightSpan); 
                // console.log('[handleHighlightingForNodeSegmentDOM] Surrounded contents with new span:', highlightSpan);
            }
            catch (e) { 
                // console.warn("[handleHighlightingForNodeSegmentDOM] SurroundContents failed, trying fallback. Error:", e.message);
                try { const frag = segmentRange.extractContents(); highlightSpan.appendChild(frag); segmentRange.insertNode(highlightSpan); 
                    // console.log('[handleHighlightingForNodeSegmentDOM] Fallback surround (extract/insert) successful with span:', highlightSpan);
                } 
                catch (e2) { console.error("[handleHighlightingForNodeSegmentDOM] Fallback surround FAILED:", e2.message); } 
            }
        }
    }

    function splitSpanAndApplyHighlightDOM(existingSpan, segmentRange, newColor, newId) {
        // console.log('[splitSpanAndApplyHighlightDOM] Existing span text:', existingSpan.textContent.substring(0,30), 'Segment:', segmentRange.toString().substring(0,30));
        const textNode = existingSpan.firstChild;
        if (!textNode || textNode.nodeType !== Node.TEXT_NODE) { console.error("splitApplyDOM: Span no text child.", existingSpan); return; }
        const parent = existingSpan.parentNode; if (!parent) return;
        
        const startOffset = (segmentRange.startContainer === textNode) ? segmentRange.startOffset : 0;
        const endOffset = (segmentRange.endContainer === textNode && segmentRange.endOffset <= textNode.textContent.length) ? segmentRange.endOffset : textNode.textContent.length;

        const existingColor = existingSpan.dataset.hlColor; const existingId = existingSpan.dataset.hlId;
        const textBefore = textNode.textContent.substring(0, startOffset);
        const textMid = textNode.textContent.substring(startOffset, endOffset);
        const textAfter = textNode.textContent.substring(endOffset);
        const nodesToInsert = [];
        // console.log(`[splitSpanAndApplyHighlightDOM] Before: "${textBefore}", Mid: "${textMid}", After: "${textAfter}"`);

        if (textBefore.length > 0) { const s = document.createElement('span'); s.className = 'pdf-highlight'; s.dataset.hlId = existingId; s.dataset.hlColor = existingColor; s.style.backgroundColor = existingColor; s.textContent = textBefore; nodesToInsert.push(s); }
        if (textMid.length > 0) { const s = document.createElement('span'); s.className = 'pdf-highlight'; s.dataset.hlId = newId; s.dataset.hlColor = newColor; s.style.backgroundColor = newColor; s.textContent = textMid; nodesToInsert.push(s); }
        if (textAfter.length > 0) { const s = document.createElement('span'); s.className = 'pdf-highlight'; s.dataset.hlId = existingId; s.dataset.hlColor = existingColor; s.style.backgroundColor = existingColor; s.textContent = textAfter; nodesToInsert.push(s); }
        
        if (nodesToInsert.length > 0) { const f = document.createDocumentFragment(); nodesToInsert.forEach(n => f.appendChild(n)); if(parent.contains(existingSpan)) parent.replaceChild(f, existingSpan); }
        else if (parent.contains(existingSpan)) { parent.removeChild(existingSpan); }
    }

    function removeHighlightFromSelectionDOM(range) {
        if (!range || range.collapsed || !viewerElement) return;
        const commonAncestorTextLayer = (range.commonAncestorContainer.nodeType === Node.ELEMENT_NODE ? range.commonAncestorContainer : range.commonAncestorContainer.parentNode)?.closest('.textLayer');
        if (!commonAncestorTextLayer) return;
        const spansToProcess = Array.from(commonAncestorTextLayer.querySelectorAll('.pdf-highlight')).filter(span => range.intersectsNode(span));
        if (spansToProcess.length === 0) return;
        spansToProcess.forEach(spanNode => {
            const textNode = spanNode.firstChild;
            if (!textNode || textNode.nodeType !== Node.TEXT_NODE) return;
            const nodeRange = document.createRange(); nodeRange.selectNodeContents(textNode);
            const spanFullyWithinSelection = range.compareBoundaryPoints(Range.START_TO_START, nodeRange) <= 0 && range.compareBoundaryPoints(Range.END_TO_END, nodeRange) >= 0;
            if (spanFullyWithinSelection) { unwrapNodeDOM(spanNode); } 
            else {
                const segmentToUnwrap = document.createRange(); // Create a new range for the segment
                // Set start of segmentToUnwrap
                if (range.compareBoundaryPoints(Range.START_TO_START, nodeRange) > 0) { // Selection starts within this node
                    segmentToUnwrap.setStart(range.startContainer, range.startOffset);
                } else { // Selection starts before this node
                    segmentToUnwrap.setStart(nodeRange.startContainer, nodeRange.startOffset);
                }
                // Set end of segmentToUnwrap
                if (range.compareBoundaryPoints(Range.END_TO_END, nodeRange) < 0) { // Selection ends within this node
                     segmentToUnwrap.setEnd(range.endContainer, range.endOffset);
                } else { // Selection ends after this node
                    segmentToUnwrap.setEnd(nodeRange.endContainer, nodeRange.endOffset);
                }
                if (!segmentToUnwrap.collapsed) splitSpanAndUnwrapSegmentDOM(spanNode, segmentToUnwrap);
            }
        });
        try { commonAncestorTextLayer.normalize(); } catch (e) { console.warn("Normalize remove failed:", e); }
    }

    function removePartialHighlightDOM(range) { removeHighlightFromSelectionDOM(range); }

    function splitSpanAndUnwrapSegmentDOM(existingSpan, segmentRangeToUnwrap) {
        const textNode = existingSpan.firstChild;
        if (!textNode || textNode.nodeType !== Node.TEXT_NODE) { return; }
        const parent = existingSpan.parentNode; if (!parent) return;
        
        const startOffset = (segmentRangeToUnwrap.startContainer === textNode) ? segmentRangeToUnwrap.startOffset : 0;
        const endOffset = (segmentRangeToUnwrap.endContainer === textNode && segmentRangeToUnwrap.endOffset <= textNode.textContent.length) ? segmentRangeToUnwrap.endOffset : textNode.textContent.length;

        const existingColor = existingSpan.dataset.hlColor; const existingId = existingSpan.dataset.hlId;
        const textBefore = textNode.textContent.substring(0, startOffset);
        const textMid = textNode.textContent.substring(startOffset, endOffset);
        const textAfter = textNode.textContent.substring(endOffset);
        const nodesToInsert = [];
        if (textBefore.length > 0) { const s = document.createElement('span'); s.className = 'pdf-highlight'; s.dataset.hlId = existingId; s.dataset.hlColor = existingColor; s.style.backgroundColor = existingColor; s.textContent = textBefore; nodesToInsert.push(s); }
        if (textMid.length > 0) { nodesToInsert.push(document.createTextNode(textMid)); }
        if (textAfter.length > 0) { const s = document.createElement('span'); s.className = 'pdf-highlight'; s.dataset.hlId = existingId; s.dataset.hlColor = existingColor; s.style.backgroundColor = existingColor; s.textContent = textAfter; nodesToInsert.push(s); }
        if (nodesToInsert.length > 0) { const f = document.createDocumentFragment(); nodesToInsert.forEach(n => f.appendChild(n)); if(parent.contains(existingSpan)) parent.replaceChild(f, existingSpan); }
        else if (parent.contains(existingSpan)) { unwrapNodeDOM(existingSpan); }
    }

    function removeClickedHighlightBlockDOM(id) {
        if (!id || !viewerContainer) return;
        const spans = viewerContainer.querySelectorAll(`.pdf-highlight[data-hl-id="${id}"]`);
        if (spans.length === 0) return;
        let commonAncestor = spans.length > 0 && spans[0].parentNode ? spans[0].parentNode.closest('.textLayer') : null;
        spans.forEach(span => { unwrapNodeDOM(span); });
        try { if (commonAncestor) { commonAncestor.normalize(); } else { viewerElement?.normalize(); } }
        catch(e) { console.warn("Normalization failed in removeClickedHighlightBlockDOM", e); }
        if (clickedHighlightId === id) clickedHighlightId = null;
    }

    function changeClickedHighlightColorDOM(id, color) {
        if (!id || !color || !viewerContainer) return;
        const spans = viewerContainer.querySelectorAll(`.pdf-highlight[data-hl-id="${id}"]`);
        if (spans.length === 0) return;
        spans.forEach(span => { span.style.backgroundColor = color; span.dataset.hlColor = color; });
        if (clickedHighlightId === id) clickedHighlightColor = color;
    }

    function unwrapNodeDOM(node) {
        const parent = node.parentNode; if (!parent) return;
        while (node.firstChild) { parent.insertBefore(node.firstChild, node); }
        try { if (parent.contains(node)) parent.removeChild(node); } catch (e) { /* console.error("Unwrap Error:", e, node); */ }
    }
    
    /* ─────────────────────────── PDF Loading / Setup (User's latest version) ──────────────── */
    async function loadPdfAndLibraries(containerElement) {
        loading = true; error = null; loadingMessage = 'Loading PDF libraries...'; 
        initialHighlightsApplied = false;
        undoStack = []; redoStack = [];

        // Ensure old viewer and doc are cleaned up if they exist BEFORE nulling them
        if (pdfViewer) { 
            pdfViewer.cleanup(); 
            pdfViewer.setDocument(null);
            // pdfViewer.eventBus = null; // PDFViewer might handle its own eventBus unlinking on cleanup/setDocument(null)
        }
        if (pdfDoc) {
            pdfDoc.destroy();
        }
        // Now set them to null
        pdfDoc = null; 
        pdfViewer = null;

        try {
            if (!pdfjsLib) {
                pdfjsLib = await import('pdfjs-dist');
                const vm = await import('pdfjs-dist/web/pdf_viewer.mjs');
                PDFViewer = vm.PDFViewer; EventBus = vm.EventBus; PDFLinkService = vm.PDFLinkService; PDFFindController = vm.PDFFindController;
                const { GlobalWorkerOptions } = pdfjsLib; 
                // Vite specific import for worker URL
                const PDFJSWorker = (await import('pdfjs-dist/build/pdf.worker.min.mjs?url')).default;
                GlobalWorkerOptions.workerSrc = PDFJSWorker;
            }
            
            // Ensure old eventBus is destroyed before creating a new one.
            if (eventBus && typeof eventBus.destroy === 'function') { 
                eventBus.destroy();
            }
            eventBus = new EventBus(); 
            setupViewerEvents(); // Re-attach listeners to the new eventBus
            
            loadingMessage = 'Reading PDF file...';
            const pdfData = await readFile(pdfPath);
            loadingMessage = 'Parsing PDF structure...';
            const loadingTask = pdfjsLib.getDocument({ data: pdfData, cMapUrl: '/node_modules/pdfjs-dist/cmaps/', cMapPacked: true, enableXfa: false });
            pdfDoc = await loadingTask.promise;
            numPages = pdfDoc.numPages;
            
            const linkService = new PDFLinkService({ eventBus }); // Use the new eventBus
            const findController = new PDFFindController({ linkService: linkService, eventBus: eventBus });
            pdfViewer = new PDFViewer({ 
                container: containerElement, 
                viewer: viewerElement, 
                eventBus: eventBus, // Pass the new eventBus
                linkService: linkService, 
                findController: findController, 
                removePageBorders: true, 
                l10n: null, // Using PDFJS.NullL10n for simplicity, or your own L10n
                textLayerMode: 2, // ENABLE_ENHANCE - necessary for text selection
                annotationMode: 1 // ENABLE - To see native PDF annotations if any (and for future)
            });
            linkService.setViewer(pdfViewer);
            linkService.setDocument(pdfDoc, null); // Required for outline/attachments if used.
            pdfViewer.setDocument(pdfDoc);
            
            // Reset states for the new document
            currentPageNum = 1; 
            pageNumInput = 1; 
            currentScaleValue = 'auto'; // Let PDFViewer decide initial scale, or set a default

        } catch (err) {
            console.error('PDF Load Error:', err);
            error = `Failed to load or render PDF: ${err?.message || String(err)}`;
            loading = false;
        }
    }

    // Wait briefly for annotations to arrive, then apply them
    async function waitForHighlightsAndApply(maxWaitMs = 3000) {
        const start = Date.now();
        while (Date.now() - start < maxWaitMs) {
            if (initialHighlights && initialHighlights.length > 0) {
                await applyInitialHighlights();
                return;
            }
            await new Promise(r => setTimeout(r, 100));
        }
        // Last‑ditch attempt (handles case where highlights arrive late but within timeout)
        if (initialHighlights && initialHighlights.length > 0) {
            await applyInitialHighlights();
        }
    }

    function setupViewerEvents() {
        if (!eventBus) return;
        // Clear existing listeners on the eventBus instance IF it's being reused
        // However, since we create `new EventBus()` in loadPdfAndLibraries, this might not be strictly necessary
        // unless setupViewerEvents could be called multiple times with the same eventBus instance.
        // For safety, a more robust clear would involve eventBus.off(eventName, handler) for each.
        // A simple _listeners = {} might work for the default EventBus but isn't a public API.
        // eventBus._listeners = {}; // Risky, internal property.

        eventBus.on('pagechanging', (e) => { if (e.pageNumber && e.pageNumber !== currentPageNum) { currentPageNum = e.pageNumber; pageRendering = true; hideSelectionToolbar();} });
        eventBus.on('pagerendered', (e) => { 
            if (e.pageNumber === currentPageNum) pageRendering = false; 
            // console.log(`Page ${e.pageNumber} rendered.`);
        });
        eventBus.on('scalechanging', (e) => { 
            let s = currentScaleValue; 
            if(e.presetValue) {
                s = e.presetValue;
            } else if(typeof e.scale ==='number') {
                const m = PRESET_SCALES.find(p => !isNaN(parseFloat(p)) && Math.abs(parseFloat(p) - e.scale) < 0.001); 
                s = (m && !['auto','page-actual','page-fit','page-width'].includes(m)) ? m : String(parseFloat(e.scale.toFixed(4)));
            }
            currentScaleValue = s; 
            pageRendering = true; 
            hideSelectionToolbar();
        });
        
        eventBus.on('documentloaded', async () => {
            if(pdfViewer?.pdfDocument){
                numPages = pdfViewer.pdfDocument.numPages;
                currentPageNum = pdfViewer.currentPageNumber;
                currentScaleValue = String(pdfViewer.currentScaleValue || 'auto'); // Ensure it's a string for select
                loading = false; pageRendering = false;
                console.log('[PDFViewerPanel] event: documentloaded - PDF processing complete. Attempting initial highlights.');
                await tick(); // Ensure UI is updated with new page count etc.
                // (No longer apply highlights here)
            } else {
                error = "PDF Viewer was not properly initialized with a document after load."; 
                loading = false;
            }
        });

        eventBus.on('textlayerrendered', async (evt) => {
            // Re-apply highlights for this page whenever its text layer renders
            await applyHighlightsForPage(evt.pageNumber - 1);
        });

        eventBus.on('pagesinit', () => { 
            // This event fires after the pages are initialized.
            // 'documentloaded' is generally more reliable for knowing when the document is fully processed.
            if(loading && pdfViewer?.pdfDocument){ 
                numPages=pdfViewer.pdfDocument.numPages; 
                currentPageNum=pdfViewer.currentPageNumber; 
                currentScaleValue=String(pdfViewer.currentScaleValue||'auto');
                loading=false; pageRendering=false;
                console.log("[PDFViewerPanel] event: pagesinit (during initial load).");
                waitForHighlightsAndApply();
            } 
            // Ensure scale is consistent if pages reinitialize for some reason (e.g. after scale change)
            if(pdfViewer && !loading && String(pdfViewer.currentScaleValue) !== currentScaleValue) {
                // pdfViewer.currentScaleValue = currentScaleValue; // This can cause loop if not careful
            }
        });
        eventBus.on('updatefindcontrolstate', ({ state }) => { currentFindState = state; });
    }

    // --- Apply Initial Highlights (More Robust) ---
    async function applyInitialHighlights() {
        if (initialHighlightsApplied || !pdfDoc || !pdfViewer || !viewerElement) return;
        if (!initialHighlights || initialHighlights.length === 0) return;
        loading = true; 
        loadingMessage = 'Loading Annotations...';
        console.log(`[PDFViewerPanel] ${loadingMessage} Count:`, initialHighlights.length);
        await tick(); 

        const highlightsByPage = initialHighlights.reduce((acc, hl) => {
            const pageIdx = hl.pageIndex;
            if (typeof pageIdx !== 'number' || pageIdx < 0) { return acc; }
            if (!acc[pageIdx]) acc[pageIdx] = [];
            acc[pageIdx].push(hl);
            return acc;
        }, {});

        const pageIndicesWithHighlights = Object.keys(highlightsByPage).map(idx => parseInt(idx, 10));

        for (const pageIndex of pageIndicesWithHighlights) {
            console.log(`[ApplyInitial] Processing page ${pageIndex + 1} for highlights.`);
            const pageHighlights = highlightsByPage[pageIndex];
            if (!pageHighlights || pageHighlights.length === 0) continue;

            let pageView = pdfViewer.getPageView(pageIndex);
            try {
                pageView = await ensureTextLayerReady(pageView, pageIndex);
            } catch(e) {
                console.error(`[ApplyInitial] Failed to ensure text layer for page ${pageIndex + 1}. Skipping highlights for this page. Error: ${e.message}`);
                continue; 
            }
            
            const pageTextLayerDiv = pageView.textLayer?.textLayerDiv;
            const pdfPage = pageView.pdfPage || await pdfDoc.getPage(pageIndex + 1);
            if (!pdfPage) {
                 console.warn(`[ApplyInitial] Could not get PDFPage object for page ${pageIndex + 1}. Skipping.`);
                 continue;
            }
            // Get text content once for the page
            const textContent = await pdfPage.getTextContent({ normalizeWhitespace: true, includeMarkedContent: false });

            if (!pageTextLayerDiv || !textContent?.items?.length) {
                console.warn(`[ApplyInitial] No textLayerDiv or text items for page ${pageIndex + 1}. Skipping highlights for this page.`);
                continue;
            }
            // Use textLayer DOM when available, fall back to PDF.js items
            let rawFullPageText = pageTextLayerDiv.textContent || textContent.items.map(item => item.str).join('');
            // Clean raw text: replace NBSP, remove line-break hyphens, then normalize.
            rawFullPageText = rawFullPageText
                .replace(/\u00A0/g, ' ')
                .replace(/-\s+/g, '');        // Remove hyphens at line breaks only
            // Normalize ligatures and punctuation, then collapse whitespace for the page search
            const fullPageTextNormalized = normalizeTextForMatching(rawFullPageText).replace(/\s+/g, ' ');

            for (const highlight of pageHighlights) {
                // highlight.text, .prefix, .suffix are already normalized from storage
                if (!highlight.text || !highlight.color || !highlight.id) continue;
                let startIndex = -1;
                let currentOccurrences = 0;
                const targetOccurrence = highlight.occurrenceInPageContext || 0;
                // Normalize stored highlight text and context
                const searchStrRaw = highlight.text;
                const searchStrNormalized = normalizeTextForMatching(searchStrRaw).replace(/\s+/g, ' ');
                const prefixNorm = highlight.prefix
                    ? normalizeTextForMatching(highlight.prefix).replace(/\s+/g, ' ')
                    : '';
                const suffixNorm = highlight.suffix
                    ? normalizeTextForMatching(highlight.suffix).replace(/\s+/g, ' ')
                    : '';
                // Build regex with lookbehind/lookahead if context exists
                let pattern = '';
                if (prefixNorm) {
                    pattern += `(?<=${escapeRegExp(prefixNorm)})`;
                }
                pattern += escapeRegExp(searchStrNormalized);
                if (suffixNorm) {
                    pattern += `(?=${escapeRegExp(suffixNorm)})`;
                }
                let regex = new RegExp(pattern, 'g');

                let match;
                while ((match = regex.exec(fullPageTextNormalized)) !== null) {
                    if (currentOccurrences === targetOccurrence) {
                        startIndex = match.index;
                        break;
                    }
                    currentOccurrences++;
                }
                // Fallback: simple search without context if not found
                if (startIndex === -1) {
                    const simpleRegex = new RegExp(escapeRegExp(searchStrNormalized), 'g');
                    let simpleMatch;
                    let simpleCount = 0;
                    while ((simpleMatch = simpleRegex.exec(fullPageTextNormalized)) !== null) {
                        if (simpleCount === targetOccurrence) {
                            startIndex = simpleMatch.index;
                            break;
                        }
                        simpleCount++;
                    }
                }
                if (startIndex !== -1) {
                    // Pass the normalized expected text for verification
                    const range = findRangeInTextLayer(
                        pageTextLayerDiv,
                        startIndex,
                        searchStrNormalized.length,
                        searchStrNormalized
                    );
                    if (range) {
                        // Add a check to see if the found range's text (when normalized) matches the expected normalized text
                        const domTextNormalized = normalizeTextForMatching(range.toString());
                        if (domTextNormalized === searchStrNormalized) {
                            applyHighlightToSelectionDOM(range, highlight.color, highlight.id);
                            // Throttle per-highlight rendering
                            // await tick();
                            // await new Promise(resolve => setTimeout(resolve, 50));
                        } else {
                            console.warn(`[ApplyInitial] Range found for ID ${highlight.id}, but text mismatch after DOM normalization. Expected (norm): "${searchStrNormalized.substring(0,30)}", Found (norm): "${domTextNormalized.substring(0,30)}"`);
                        }
                    } else {
                        console.warn(`[ApplyInitial] Failed to create DOM range (findRangeInTextLayer returned null) for ID ${highlight.id} on page ${pageIndex + 1} (norm. offset ${startIndex}). Text: "${searchStrNormalized.substring(0,20)}..."`);
                    }
                } else {
                    console.warn(`[ApplyInitial] Text not found (normalized search) for highlight ID ${highlight.id} on page ${pageIndex + 1}: "${searchStrNormalized.substring(0,30)}..." (Occ: ${targetOccurrence}, Pfx: "${prefixNorm.substring(0,10)}", Sfx: "${suffixNorm.substring(0,10)}")`);
                }
            }
            // Allow highlights from this page to render before processing the next page in the initial pass
            await tick();
            await new Promise(resolve => setTimeout(resolve, 100)); // Reduced from 200ms
        }

        // --- Scroll-through to ensure all highlights render correctly ---
        // The "Loading Annotations..." overlay remains active during this.
        if (numPages > 0 && pdfViewer) {
            console.log('[PDFViewerPanel] Starting scroll-through refresh for all pages.');

            // Scroll to the last page
            pdfViewer.scrollPageIntoView({ pageNumber: numPages });
            await new Promise(resolve => setTimeout(resolve, 250 + (numPages * 5))); // Delay for jump and initial render

            // Scroll back to the top, one page at a time
            for (let i = numPages; i >= 1; i--) {
                if (!pdfViewer) break; // Safety check
                pdfViewer.scrollPageIntoView({ pageNumber: i });
                await new Promise(resolve => setTimeout(resolve, 120)); // Delay for each page to render/trigger textlayerrendered
            }

            // Ensure view is settled at page 1
            if (pdfViewer) { pdfViewer.scrollPageIntoView({ pageNumber: 1 }); await new Promise(resolve => setTimeout(resolve, 200)); }
            console.log('[PDFViewerPanel] Finished scroll-through refresh.');
        }

        initialHighlightsApplied = true;
        loading = false;
        loadingMessage = ''; // Reset loading message
        console.log('[PDFViewerPanel] Finished applying all initial highlights.');
    }

// Re-apply stored highlights on a single page whenever its text layer renders
async function applyHighlightsForPage(pageIndex) {
    if (!initialHighlights?.length || !pdfViewer || !pdfDoc) return;
    // grab only the highlights for this page
    const pageHighlights = initialHighlights.filter(hl => hl.pageIndex === pageIndex);
    if (!pageHighlights.length) return;

    // ensure textLayer is ready
    let pageView = pdfViewer.getPageView(pageIndex);
    try {
        pageView = await ensureTextLayerReady(pageView, pageIndex);
    } catch {
        return;
    }
    const layerDiv = pageView.textLayer?.textLayerDiv;
    const pdfPage = pageView.pdfPage || await pdfDoc.getPage(pageIndex + 1);
    if (!layerDiv || !pdfPage) return;

    // Use DOM text if rendered, otherwise items
    const items = (await pdfPage.getTextContent({ normalizeWhitespace: true })).items;
    let rawTxt = pageView.textLayer?.textLayerDiv?.textContent || items.map(i => i.str).join('');
    // Clean raw node text: replace NBSP, remove line-break hyphens
    rawTxt = rawTxt
        .replace(/\u00A0/g, ' ')
        .replace(/-\s+/g, '');        // Remove hyphens at line breaks only
    // Normalize ligatures and punctuation, then collapse whitespace for matching
    const normTxt = normalizeTextForMatching(rawTxt).replace(/\s+/g, ' ');

    for (const hl of pageHighlights) {
        // find its start in the normalized page text
        const re = hl.prefix || hl.suffix
            ? new RegExp(
                (hl.prefix ? escapeRegExp(hl.prefix) : '') +
                `(${escapeRegExp(hl.text)})` +
                (hl.suffix ? escapeRegExp(hl.suffix) : ''),
                'g'
            )
            : new RegExp(`(${escapeRegExp(hl.text)})`, 'g');
        let m, occ = 0, start = -1;
        while ((m = re.exec(normTxt))) {
            if (occ === (hl.occurrenceInPageContext || 0)) {
                start = m.index + ((hl.prefix && m[1]) ? hl.prefix.length : 0);
                break;
            }
            occ++;
        }
        if (start < 0) continue;

        const range = findRangeInTextLayer(
            layerDiv,
            start,
            hl.text.length,
            hl.text
        );
        if (range) {
            applyHighlightToSelectionDOM(range, hl.color, hl.id);
        }
    }
}

    // Helper: Maps a normalized offset within a node's normalized text to a raw offset within its raw text.
    function mapNormalizedOffsetInNodeToRawOffset(rawTextNodeContent, targetNormalizedOffsetInNode) {
        if (targetNormalizedOffsetInNode === 0) return 0;
        
        for (let r = 0; r < rawTextNodeContent.length; r++) { // r is the raw character index
            // Normalized length of raw text *before* char at r
            const normLenBeforeR = (r > 0) ? normalizeTextForMatching(rawTextNodeContent.substring(0, r)).length : 0;
            // Normalized length of raw text *including* char at r
            const normLenIncludingR = normalizeTextForMatching(rawTextNodeContent.substring(0, r + 1)).length;

            // If the target offset is less than the normalized length *after* including this raw char,
            // AND the target offset is greater than or equal to the normalized length *before* this raw char,
            // then this raw char 'r' is the one that starts the target normalized character.
            if (normLenIncludingR > targetNormalizedOffsetInNode && normLenBeforeR <= targetNormalizedOffsetInNode) {
                return r;
            }
        }
        
        // If targetNormalizedOffsetInNode is equal to the total normalized length of the node,
        // it means the offset is at the end of the node (raw length).
        if (normalizeTextForMatching(rawTextNodeContent).length === targetNormalizedOffsetInNode) {
            return rawTextNodeContent.length;
        }
        // console.warn(`[mapNormToRaw] Could not map norm offset ${targetNormalizedOffsetInNode} in raw "${rawTextNodeContent.substring(0,20)}" (norm len: ${normalizeTextForMatching(rawTextNodeContent).length})`);
        return rawTextNodeContent.length; // Fallback, indicates an issue if used for a start offset not at end of node.
    }

    function findRangeInTextLayer(textLayerDiv, normalizedOverallCharStart, normalizedOverallLength, normalizedExpectedText) {
        const range = document.createRange();
        let accumulatedNormalizedCharsBeforeNode = 0;
        let startNode = null, startOffsetRaw = -1;
        let endNode = null, endOffsetRaw = -1;
        let foundStart = false;

        const walker = document.createTreeWalker(textLayerDiv, NodeFilter.SHOW_TEXT, null, false);
        let currentNode;

        while (currentNode = walker.nextNode()) {
            const nodeTextRaw = currentNode.textContent;
            if (!nodeTextRaw) continue;

            const nodeNormalizedText = normalizeTextForMatching(nodeTextRaw);
            const nodeNormalizedLength = nodeNormalizedText.length;

            if (!foundStart && normalizedOverallCharStart >= accumulatedNormalizedCharsBeforeNode && normalizedOverallCharStart < accumulatedNormalizedCharsBeforeNode + nodeNormalizedLength) {
                startNode = currentNode;
                const normalizedStartInNode = normalizedOverallCharStart - accumulatedNormalizedCharsBeforeNode;
                startOffsetRaw = mapNormalizedOffsetInNodeToRawOffset(nodeTextRaw, normalizedStartInNode);
                foundStart = true;
            }

            if (foundStart) {
                const targetEndNormalizedGlobal = normalizedOverallCharStart + normalizedOverallLength;
                if (targetEndNormalizedGlobal <= accumulatedNormalizedCharsBeforeNode + nodeNormalizedLength) {
                    // Highlight ends in this node
                    endNode = currentNode;
                    const normalizedEndInNode = targetEndNormalizedGlobal - accumulatedNormalizedCharsBeforeNode;
                    endOffsetRaw = mapNormalizedOffsetInNodeToRawOffset(nodeTextRaw, normalizedEndInNode);
                    break; // Found start and end node
                }
            }
            accumulatedNormalizedCharsBeforeNode += nodeNormalizedLength;
        }

        if (startNode && endNode && startOffsetRaw !== -1 && endOffsetRaw !== -1) {
            try {
                range.setStart(startNode, startOffsetRaw);
                range.setEnd(endNode, endOffsetRaw);

                const actualNormalizedTextFromRange = normalizeTextForMatching(range.toString());
                if (actualNormalizedTextFromRange !== normalizedExpectedText) {
                    console.warn(
                        `[findRangeInTextLayer] Verification mismatch. Expected: "${normalizedExpectedText.substring(0,30)}...", Got: "${actualNormalizedTextFromRange.substring(0,30)}...".`
                    );
                }
                // Return range even if verification fails to ensure simple highlights are applied
                return range;
            } catch (e) { console.error("[findRangeInTextLayer] Error setting/verifying range:", e, {startNode, startOffsetRaw, endNode, endOffsetRaw}); return null; }
        } else {
            // console.warn(`[findRangeInTextLayer] Could not determine start/end node/offset. NormStart: ${normalizedOverallCharStart}, NormLen: ${normalizedOverallLength}. FoundStart: ${foundStart}`);
        }
        return null;
    }

    // Helper for findRangeInTextLayer to get accumulated normalized length up to a specific node (exclusive of the node itself)
    // This helper was part of a previous thought process and might not be strictly needed with the current findRangeInTextLayer structure
    // but can be useful for debugging or alternative mapping strategies.
    /*
    function findAccumulatedNormLengthForNode(textLayerDiv, targetNode) {
        const walker = document.createTreeWalker(textLayerDiv, NodeFilter.SHOW_TEXT, null, false);
        let currentNode;
        let accumulated = 0;
        while(currentNode = walker.nextNode()) {
            if (currentNode === targetNode) break;
            accumulated += normalizeTextForMatching(currentNode.textContent).length;
        }
        return accumulated;
    }
    */

    /* ─────────────────────────── Toolbar Actions (from your complete version) ────────────────── */
    function goToPrevPage() { if (pdfViewer && currentPageNum > 1) pdfViewer.previousPage(); }
    function goToNextPage() { if (pdfViewer && currentPageNum < numPages) pdfViewer.nextPage(); }
    function zoomOut() { if (pdfViewer) pdfViewer.decreaseScale(); }
    function zoomIn() { if (pdfViewer) pdfViewer.increaseScale(); }
    function setZoom(value) { if (pdfViewer && value) { pdfViewer.currentScaleValue = value; } }
    function handlePageInputChange(e) { if (!pdfViewer) return; const req = parseInt(e.target.value, 10); if (!isNaN(req) && req >= 1 && req <= numPages && req !== currentPageNum) { pdfViewer.currentPageNumber = req; } else { e.target.value = currentPageNum; }}
    function handlePageInputBlur(e) { if (e.target.value !== String(currentPageNum)) e.target.value = currentPageNum; }
    function runSearch({ findPrevious = false } = {}) {
      if (!searchQuery.trim() || !eventBus) return;
      if (currentFindState === 3 && lastSearched === searchQuery.trim()) { return; }
      const query = searchQuery.trim(); const isNewQuery = query !== lastSearched; lastSearched = query;
      eventBus.dispatch('find', { source: pdfViewer, type: isNewQuery ? '' : 'again', query, phraseSearch: true, caseSensitive: false, entireWord: false, highlightAll: true, findPrevious });
    }
    function handleSearchKeydown(e) { if (e.key === 'Enter') { e.preventDefault(); runSearch({ findPrevious: e.shiftKey }); } }
    
    /* ─── Sync with store‑level PDF annotations ─── */
    $: storePdfAnnotations = (
        $project.currentPdfAnnotations ??
        $project.pdfAnnotations ??
        $project.currentDocumentHighlights ??
        []
    );

    $: if (
        pdfDoc &&
        pdfViewer &&
        !initialHighlightsApplied &&
        storePdfAnnotations &&
        storePdfAnnotations.length > 0
    ) {
        initialHighlights = storePdfAnnotations;
        if (pdfViewer._pages?.length > 0) {
            applyInitialHighlights();
        }
    }

    $: pageNumInput = currentPageNum;
    $: selectScaleValue = (() => { 
        const s = String(currentScaleValue); 
        if (PRESET_SCALES.includes(s)) return s;
        const numScale = parseFloat(s);
        if (!isNaN(numScale)) {
            const matchingPreset = PRESET_SCALES.find(p => {
                const pNum = parseFloat(p);
                return !isNaN(pNum) && Math.abs(pNum - numScale) < 0.001;
            });
            if (matchingPreset) return matchingPreset;
            return s; // Return the numeric string if no close preset, PDFViewer will handle it
        }
        return 'auto'; // Default fallback
    })();


</script>

<div class="pdf-viewer-panel-root prose prose-sm dark:prose-invert max-w-none flex flex-col h-full w-full bg-gray-100 dark:bg-gray-900 rounded-md shadow overflow-hidden">

<div class="toolbar flex items-center flex-nowrap gap-x-0.5 border-b border-gray-300 dark:border-gray-600 p-1 flex-shrink-0 bg-white dark:bg-gray-800 shadow z-20">
    <button class="mini-toolbar-button" on:click={goToPrevPage} title="Previous page" disabled={currentPageNum <= 1 || loading || !pdfDoc}><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4"><path fill-rule="evenodd" d="M12.79 5.23a.75.75 0 0 1-.02 1.06L8.832 10l3.938 3.71a.75.75 0 1 1-1.04 1.08l-4.5-4.25a.75.75 0 0 1 0-1.08l4.5-4.25a.75.75 0 0 1 1.06.02Z" clip-rule="evenodd" /></svg></button>
    <input type="number" class="mini-toolbar-input w-12 text-center mx-0.5" bind:value={pageNumInput} min="1" max={numPages || 1} aria-label="Current page number" on:change={handlePageInputChange} on:blur={handlePageInputBlur} disabled={loading || !pdfDoc} />
    <span class="text-xs px-1 text-gray-600 dark:text-gray-400"> of {numPages || '?'} </span>
    <button class="mini-toolbar-button" on:click={goToNextPage} title="Next page" disabled={currentPageNum >= numPages || loading || !pdfDoc}><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4"><path fill-rule="evenodd" d="M7.21 14.77a.75.75 0 0 1 .02-1.06L11.168 10 7.23 6.29a.75.75 0 1 1 1.04-1.08l4.5 4.25a.75.75 0 0 1 0 1.08l-4.5 4.25a.75.75 0 0 1-1.06-.02Z" clip-rule="evenodd" /></svg></button>
    <div class="separator"></div>
    <button class="mini-toolbar-button" on:click={zoomOut} title="Zoom out" disabled={loading || !pdfDoc}><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4"><path fill-rule="evenodd" d="M4 10a.75.75 0 0 1 .75-.75h10.5a.75.75 0 0 1 0 1.5H4.75A.75.75 0 0 1 4 10Z" clip-rule="evenodd" /></svg></button>
    <select class="mini-toolbar-select w-24" bind:value={currentScaleValue} on:change={(e) => setZoom(e.target.value)} disabled={loading || !pdfDoc} aria-label="Zoom level">
        {#if !PRESET_SCALES.includes(String(currentScaleValue)) && !isNaN(parseFloat(currentScaleValue))}
            <option value={currentScaleValue} selected>{Math.round(parseFloat(currentScaleValue) * 100)}%</option>
        {/if}
        <option value="auto">Auto</option> 
        <option value="page-actual">Actual size</option> 
        <option value="page-fit">Page fit</option> 
        <option value="page-width">Page width</option> 
        <option disabled>──────────</option> 
        <option value="0.5">50%</option> 
        <option value="0.75">75%</option> 
        <option value="1">100%</option> 
        <option value="1.25">125%</option> 
        <option value="1.5">150%</option> 
        <option value="2">200%</option> 
        <option value="3">300%</option> 
        <option value="4">400%</option>
    </select>
    <button class="mini-toolbar-button" on:click={zoomIn} title="Zoom in" disabled={loading || !pdfDoc}><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4"><path d="M10.75 4.75a.75.75 0 0 0-1.5 0v4.5h-4.5a.75.75 0 0 0 0 1.5h4.5v4.5a.75.75 0 0 0 1.5 0v-4.5h4.5a.75.75 0 0 0 0-1.5h-4.5v-4.5Z" /></svg></button>
    <div class="separator"></div>
    <div class="flex items-center space-x-0.5">
        <input type="search" class="mini-toolbar-input w-28 text-xs" placeholder="Search" bind:value={searchQuery} on:keydown={handleSearchKeydown} disabled={loading || !pdfDoc} aria-label="Search text" autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" />
        <button class="mini-toolbar-button" title="Previous match" on:click={() => runSearch({ findPrevious: true })} disabled={loading || !pdfDoc || !searchQuery}><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-3 h-3"><path fill-rule="evenodd" d="M12.79 5.23a.75.75 0 0 1-.02 1.06L8.832 10l3.938 3.71a.75.75 0 1 1-1.04 1.08l-4.5-4.25a.75.75 0 0 1 0-1.08l4.5-4.25a.75.75 0 0 1 1.06.02Z" clip-rule="evenodd" /></svg></button>
        <button class="mini-toolbar-button" title="Next match" on:click={() => runSearch({ findPrevious: false })} disabled={loading || !pdfDoc || !searchQuery}><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-3 h-3"><path fill-rule="evenodd" d="M7.21 14.77a.75.75 0 0 1 .02-1.06L11.168 10 7.23 6.29a.75.75 0 1 1 1.04-1.08l4.5 4.25a.75.75 0 0 1 0 1.08l-4.5 4.25a.75.75 0 0 1-1.06-.02Z" clip-rule="evenodd" /></svg></button>
    </div>
    <div class="separator"></div>
    <div class="relative inline-flex items-center" bind:this={highlightDropdownRef}>
        <button class="mini-toolbar-button flex items-center" title="Highlight selected text" on:click={toggleToolbarHighlightDropdown} disabled={loading || !pdfDoc}>
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-highlighter" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M11.096.644a2 2 0 0 1 2.791.036l1.433 1.433a2 2 0 0 1 .035 2.791l-.413.435-8.07 8.995a.5.5 0 0 1-.372.166h-3a.5.5 0 0 1-.234-.058l-.412.412A.5.5 0 0 1 2.5 15h-2a.5.5 0 0 1-.354-.854l1.412-1.412A.5.5 0 0 1 1.5 12.5v-3a.5.5 0 0 1 .166-.372l8.995-8.07zm-.115 1.47L2.727 9.52l3.753 3.753 7.406-8.254zm3.585 2.17.064-.068a1 1 0 0 0-.017-1.396L13.18 1.387a1 1 0 0 0-1.396-.018l-.068.065zM5.293 13.5 2.5 10.707v1.586L3.707 13.5z"/></svg>
            <svg class="ml-1 h-4 w-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 10.94l3.71-3.71a.75.75 0 011.08 1.04l-4.25 4.25a.75.75 0 01-1.08 0L5.21 8.27a.75.75 0 01.02-1.06z" clip-rule="evenodd"/></svg>
        </button>
        {#if isToolbarHighlightDropdownOpen}
            <div class="absolute top-full mt-1 left-0 z-30 w-32 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded shadow-lg overflow-hidden py-1">
                <div class="px-2 py-1 flex items-center gap-2 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-700" role="menuitem" tabindex="-1" on:click|stopPropagation={() => { handleDropdownHighlightAction('remove'); }}>
                    <span class="w-4 h-4 rounded-full border border-gray-400 dark:border-gray-500 flex items-center justify-center"><svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3 text-gray-500 dark:text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" /></svg></span>
                    <span>None</span>
                </div>
                {#each highlightOptions as opt}
                    <div class="px-2 py-1 flex items-center gap-2 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-700" role="menuitem" tabindex="-1" on:click|stopPropagation={() => { handleDropdownHighlightAction(opt.value); }}>
                        <span class="w-4 h-4 rounded-full border border-gray-400 dark:border-gray-500" style:background-color={opt.value}></span>
                        <span>{opt.label}</span>
                    </div>
                {/each}
            </div>
        {/if}
    </div>
    <div class="separator"></div>
    <button class="mini-toolbar-button" on:click={undo} title="Undo (⌘+Z)" disabled={undoStack.length === 0}>
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="currentColor" class="w-4 h-4">
        <path fill-rule="evenodd" d="M8 3a5 5 0 1 1-4.546 2.914.5.5 0 0 0-.908-.417A6 6 0 1 0 8 2z"/>
        <path d="M8 4.466V.534a.25.25 0 0 0-.41-.192L5.23 2.308a.25.25 0 0 0 0 .384l2.36 1.966A.25.25 0 0 0 8 4.466"/>
      </svg>
    </button>
    <button class="mini-toolbar-button" on:click={redo} title="Redo (⌘+Y / ⌘+⇧+Z)" disabled={redoStack.length === 0}>
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="currentColor" class="w-4 h-4">
        <path fill-rule="evenodd" d="M8 3a5 5 0 1 0 4.546 2.914.5.5 0 0 1 .908-.417A6 6 0 1 1 8 2z"/>
        <path d="M8 4.466V.534a.25.25 0 0 1 .41-.192l2.36 1.966c.12.1.12.284 0 .384L8.41 4.658A.25.25 0 0 1 8 4.466"/>
      </svg>
    </button>
</div>

<div bind:this={pdfViewerWrapperElement} class="flex-grow overflow-hidden bg-gray-200 dark:bg-gray-700 relative pdf-viewer-wrapper">
    {#if error}
        <div class="absolute inset-0 flex items-center justify-center p-4 z-40 pointer-events-none"><div class="text-red-700 dark:text-red-300 p-4 bg-red-100 dark:bg-red-900/80 rounded border border-red-400 dark:border-red-600 max-w-lg text-center shadow-lg"><p class="font-semibold mb-2">Error:</p><p class="text-sm break-words">{@html error}</p></div></div>
    {:else if loading}
        <div class="absolute inset-0 flex flex-col items-center justify-center z-50 bg-gray-900/75 dark:bg-black/75 pointer-events-auto">
            <!-- Replace this SVG with your GIF: <img src="/path/to/your-loading.gif" alt="Loading..." class="w-16 h-16 mb-4" /> -->
            <svg class="animate-spin h-12 w-12 text-white mb-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            <div class="text-white text-xl font-medium p-2 rounded">{loadingMessage}</div>
        </div>
    {/if}

    {#if showSelectionToolbar}
        <div class="floating-toolbar absolute bg-white dark:bg-gray-800 border border-gray-400 dark:border-gray-500 rounded shadow-lg px-1 py-0.5 flex items-center space-x-0.5 transition-opacity duration-100"
            bind:this={selectionToolbarElement}
            style:top="{selectionToolbarTop}px"
            style:left="{selectionToolbarLeft}px"
            style:display="flex"
            style:opacity="{showSelectionToolbar ? '1' : '0'}"
            style:visibility="{showSelectionToolbar ? 'visible' : 'hidden'}"
            on:mouseenter={handleToolbarMouseEnter}
            on:mouseleave={handleToolbarMouseLeave} >

            {#each highlightOptions as opt}
                <button class="floating-toolbar-button"
                    title="{toolbarMode === 'click' ? `Change highlight to ${opt.label}` : `Highlight selection ${opt.label}`}"
                    on:click|stopPropagation={() => { handleHighlightAction(opt.value); }}>
                    <span class="w-4 h-4 rounded-full border border-gray-400 dark:border-gray-500 block" style:background-color={opt.value}></span>
                </button>
            {/each}
            <div class="separator !mx-0.5"></div>
            <button class="floating-toolbar-button"
                title="{toolbarMode === 'click' ? 'Remove this entire highlight' : 'Remove highlight from selection'}"
                on:click|stopPropagation={() => { handleHighlightAction('remove'); }}>
                {#if toolbarMode === 'click'}
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="w-4 h-4 text-red-600 dark:text-red-400" viewBox="0 0 16 16"> <path d="M5.5 5.5A.5.5 0 0 1 6 5h4a.5.5 0 0 1 0 1H6a.5.5 0 0 1-.5-.5m2.5 3a.5.5 0 0 0-.5.5v4a.5.5 0 0 0 1 0v-4a.5.5 0 0 0-.5-.5"/> <path d="M14.5 3a1 1 0 0 1-1 1H13v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V4h-.5a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1H6a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1h3.5a1 1 0 0 1 1 1zM4.118 4 4 4.059V13a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1V4.059L11.882 4zM2.5 3h11V2h-11z"/> </svg>
                {:else}
                    <span class="w-4 h-4 rounded-full border border-gray-400 dark:border-gray-500 flex items-center justify-center"><svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3 text-gray-500 dark:text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" /></svg></span>
                {/if}
            </button>
        </div>
    {/if}

    <div bind:this={viewerContainer} class="pdfViewerContainer absolute inset-0 overflow-auto" id="viewerContainer">
        <div bind:this={viewerElement} class="pdfViewer" id="viewer"></div>
    </div>
</div>

</div>

<style lang="postcss">
    .pdf-viewer-panel-root { height: 100%; }
    .toolbar button.mini-toolbar-button,
    .toolbar input.mini-toolbar-input,
    .toolbar select.mini-toolbar-select {
        @apply px-2 py-0.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 hover:bg-gray-100 dark:hover:bg-gray-700 text-sm text-gray-800 dark:text-gray-200 disabled:opacity-50 disabled:cursor-not-allowed focus:outline-none focus:ring-2 focus:ring-blue-500;
        height: 28px;
        line-height: normal;
        vertical-align: middle;
    }

    .toolbar select.mini-toolbar-select {
        @apply appearance-none pr-8;
        background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' fill='%23666' viewBox='0 0 20 20'><path fill-rule='evenodd' d='M5.23 7.21a.75.75 0 011.06.02L10 11.94l3.71-4.71a.75.75 0 111.08 1.04l-4.25 5a.75.75 0 01-1.08 0l-4.25-5a.75.75 0 01.02-1.06z' clip-rule='evenodd'/></svg>");
        background-repeat: no-repeat;
        background-position: right 0.75rem center;
        background-size: 1rem;
    }

    :global(html.dark) .toolbar select.mini-toolbar-select {
        background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' fill='%23ccc' viewBox='0 0 20 20'><path fill-rule='evenodd' d='M5.23 7.21a.75.75 0 011.06.02L10 11.94l3.71-4.71a.75.75 0 111.08 1.04l-4.25 5a.75.75 0 01-1.08 0l-4.25-5a.75.75 0 01.02-1.06z' clip-rule='evenodd'/></svg>");
    }

    .toolbar input.mini-toolbar-input { padding-top: 0; padding-bottom: 0; }
    .toolbar .separator { @apply w-px h-4 bg-gray-300 dark:bg-gray-600 mx-1.5 inline-block align-middle; }
    .mini-toolbar-input[type=number]::-webkit-inner-spin-button, .mini-toolbar-input[type=number]::-webkit-outer-spin-button { -webkit-appearance: none; margin: 0; }
    .mini-toolbar-input[type=number] { -moz-appearance: textfield; }
    .pdf-viewer-wrapper { position: relative; flex-grow: 1; overflow: hidden; }
    .pdfViewerContainer { @apply p-4 bg-gray-300 dark:bg-gray-700; height: 100%; width: 100%; overflow: auto; position: absolute; inset: 0; -webkit-overflow-scrolling: touch; }

    :global(#viewer .page) { position: relative !important; z-index: 1; }
    :global(#viewer .canvasWrapper) { z-index: 1; }
    :global(#viewer .textLayer) { opacity: 1; user-select: text; cursor: text; pointer-events: auto; z-index: 20; position: absolute; inset: 0; overflow: hidden; line-height: 1.0; }
    :global(#viewer .textLayer > span) { color: transparent; position: absolute; white-space: pre; cursor: text; transform-origin: 0% 0%; pointer-events: auto; }
    :global(#viewer .annotationLayer) { z-index: 30; pointer-events: none; } /* PDF.js native annotations */
    :global(#viewer .annotationLayer *) { pointer-events: auto; }
    :global(#viewer .textLayer ::selection) { background: theme('colors.blue.500' / 30%); }
    :global(#viewer .textLayer ::-moz-selection) { background: theme('colors.blue.500' / 30%); }

    /* --- Custom Highlights --- */
    :global(.pdf-highlight) {
    pointer-events: auto; 
    cursor: default;      
    position: relative;  /* Keep this */
    z-index: 35 !important; /* Force it above textLayer (20) and annotationLayer (30 for PDF.js native) */
    mix-blend-mode: normal !important;
    padding: 1px 0;
    border-radius: 1px;
}

    .floating-toolbar { align-items: center; gap: 2px; padding: 2px 4px; z-index: 50; }
    .floating-toolbar-button { @apply p-1 rounded border border-transparent hover:bg-gray-200 dark:hover:bg-gray-600 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-transparent; line-height: 1; cursor: pointer; }
    .floating-toolbar .separator { @apply w-px h-4 bg-gray-300 dark:bg-gray-500 mx-1; }

    .w-3 { width: 0.75rem; } .h-3 { height: 0.75rem; } .w-4 { width: 1rem; } .h-4 { height: 1rem; } .w-12 { width: 3rem; } .w-24 { width: 6rem; } .w-28 { width: 7rem; }
</style>