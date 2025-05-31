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

    let autosaveIntervalId = null;
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
    let loadedPagesWithAnnotations = new Set(); // To track pages with rendered annotations
    let isLoadingInitialAnnotations = false; // New state for initial annotation loading
    let annotationMatcherWorker = null; // For text matching fallback
    let pendingWorkerTasks = new Set(); // Tracks annotations being processed by the worker: `${pageIndex}-${annotationId}`

    function deferTask(taskFn) {
        if (window.requestIdleCallback) {
            window.requestIdleCallback(taskFn, { timeout: 1000 }); // Added a timeout for safety
        } else {
            setTimeout(taskFn, 0);
        }
    }

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

    async function ensureTextLayerReady(pageViewFromCaller, pageIndex) { // Removed isEagerLoad
        let pv = pageViewFromCaller || pdfViewer?.getPageView(pageIndex);

        // Check 1: Is it already good?
        if (pv?.textLayer?.textLayerDiv || pv?.div?.querySelector('.textLayer')) {
            return finalizePageView(pv, pageIndex);
        }

        // If PageView object itself doesn't exist, try to make it exist.
        // Always operate in "non-eager" mode now.
        if (!pv) {
            try {
                pdfViewer?.scrollPageIntoView({ pageNumber: pageIndex + 1 });
            } catch (_) { /* ignore scroll errors if viewer is busy */ }
            await new Promise(r => setTimeout(r, 350)); // Initial wait for PageView creation

            pv = pdfViewer?.getPageView(pageIndex);
            if (!pv) {
                console.error(`[ensureTextLayerReady] Critical: No PageView object for page ${pageIndex + 1} after attempting scroll.`);
                throw new Error(`No PageView object for page ${pageIndex + 1}`);
            }
            // Re-check after PageView creation
            if (pv.textLayer?.textLayerDiv || pv.div?.querySelector('.textLayer')) {
                return finalizePageView(pv, pageIndex);
            }
        }
        
        // Check 2: PageView exists, but textLayer not ready.
        // Always operate in "non-eager" mode now.
        if (pv) {
            try {
                pdfViewer?.scrollPageIntoView({ pageNumber: pageIndex + 1 });
            } catch (_) { /* ignore scroll errors */ }
            await new Promise(r => setTimeout(r, 600)); // Wait a bit longer after scroll

            let freshPv = pdfViewer?.getPageView(pageIndex); 
            if (freshPv && (freshPv.textLayer?.textLayerDiv || freshPv.div?.querySelector('.textLayer'))) {
                return finalizePageView(freshPv, pageIndex);
            }
            pv = freshPv || pv; // Use freshPv if available
        }

        // Check 3: Fallback to polling (always "non-eager" mode)
        return new Promise((resolve, reject) => {
            const MAX_WAIT_TEXTLAYER = 30000;
            const interval = 250;
            let totalWait = 0;
            let checkIntervalId = null;

            const cleanupInterval = () => {
                if (checkIntervalId) { clearInterval(checkIntervalId); checkIntervalId = null; }
            };

            const checkTextLayerAvailability = async () => {
                if (!pdfViewer) {
                    cleanupInterval();
                    console.warn(`[ensureTextLayerReady] pdfViewer became null. Aborting.`);
                    // Resolve with null instead of rejecting, to allow graceful handling if possible
                    return resolve(null);
                }

                let currentPolledPv = pdfViewer.getPageView(pageIndex);
                let div = currentPolledPv?.textLayer?.textLayerDiv || currentPolledPv?.div?.querySelector('.textLayer');

                if (div) {
                    cleanupInterval();
                    try {
                        const finalPv = await finalizePageView(currentPolledPv, pageIndex);
                        resolve(finalPv);
                    } catch (e) {
                        reject(e);
                    }
                } else {
                    totalWait += interval;
                    if (totalWait >= MAX_WAIT_TEXTLAYER) {
                        cleanupInterval();
                        console.error(`[ensureTextLayerReady Polling] Timeout waiting for textLayer div on page ${pageIndex + 1}.`);
                        reject(new Error(`Timeout waiting for textLayer div on page ${pageIndex + 1} (polling)`));
                    } else if (!currentPolledPv) { // Check if pageView itself disappeared
                        cleanupInterval();
                        console.error(`[ensureTextLayerReady Polling] PageView for page ${pageIndex + 1} became null.`);
                        reject(new Error(`PageView for page ${pageIndex + 1} became null during polling`));
                    }
                }
            };
            checkIntervalId = setInterval(checkTextLayerAvailability, interval);
            checkTextLayerAvailability();
        });
    }
    
    // getContextualDataForRange was removed as it's no longer used.
    // The new implementation of createHighlightDataForStorage uses quadPoints for positioning.

    async function createHighlightDataForStorage(id, range, color) {
        if (!range) return null;
        const rawText = range.toString().trim();
        // The 'text' field should store raw selected text. Normalization was for matching, not primary purpose here.
        if (!rawText) return null; // If raw text is empty, probably not a valid highlight

        const { pageIndex, pageElement } = getRangePageInfo(range);
        let actualPageIndex = pageIndex;
        let actualPageElement = pageElement;

        if (actualPageIndex === -1) {
            actualPageIndex = pdfViewer?.currentPageNumber ? pdfViewer.currentPageNumber - 1 : 0;
            // If pageElement was not found via range, try to get it via page number
            if (pdfViewer && actualPageIndex !== -1) {
                const pageView = pdfViewer.getPageView(actualPageIndex);
                actualPageElement = pageView?.div;
            }
        }

        if (!actualPageElement) {
            console.warn('[createHighlightDataForStorage] Could not obtain pageElement. QuadPoints will be relative to viewport.');
            // Fallback or error, as pageRect is crucial. For now, let it proceed, quadpoints will be incorrect.
        }
        
        const pageRect = actualPageElement?.getBoundingClientRect() || { top: 0, left: 0 }; // Fallback to 0,0 if no pageElement

        const clientRects = range.getClientRects();
        const quadPoints = [];

        for (let i = 0; i < clientRects.length; i++) {
            const rect = clientRects[i];
            // Coordinates relative to the page
            const x1 = rect.left - pageRect.left;
            const y1 = rect.top - pageRect.top;
            const x2 = rect.right - pageRect.left;
            const y2 = rect.top - pageRect.top; // y2 is same as y1 for top-right
            const x3 = rect.left - pageRect.left; // x3 is same as x1 for bottom-left
            const y3 = rect.bottom - pageRect.top;
            const x4 = rect.right - pageRect.left; // x4 is same as x2 for bottom-right
            const y4 = rect.bottom - pageRect.top; // y4 is same as y3 for bottom-right
            
            quadPoints.push([x1, y1, x2, y2, x3, y3, x4, y4]);
        }
        
        // Note: normalizeTextForMatching(rawText) was used for 'text' before.
        // The requirement is to store raw text for 'text', and quadPoints for positioning.
        // If normalizedText is needed for OTHER purposes by the caller, that's outside this function's direct responsibility for the 'text' field.
        return { 
            id, 
            type: 'pdfHighlight', 
            color, 
            text: rawText, // Store raw text as per new requirement
            pageIndex: actualPageIndex, 
            quadPoints 
        };
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
      else if (e.metaKey && e.key === 's') {
        e.preventDefault(); // Always prevent browser save dialog
        const currentProjectState = get(project);
        if (!currentProjectState.autosaveEnabled && currentProjectState.isPdfAnnotationsDirty && pdfPath === currentProjectState.selectedDocumentPath) {
            console.log('[PDFViewerPanel Manual Save Shortcut] Saving PDF annotations...');
            saveCurrentPdfAnnotations().then(() => {
                console.log('[PDFViewerPanel Manual Save Shortcut] PDF annotations saved successfully.');
            }).catch(error => {
                console.error('[PDFViewerPanel Manual Save Shortcut] Error during save:', error);
                // Optionally, dispatch a non-intrusive notification here
            });
        }
      }
    }

    onMount(async () => {
        // Initialize the Web Worker
        annotationMatcherWorker = new Worker(new URL('$lib/workers/pdfAnnotationMatcher.worker.js', import.meta.url), { type: 'module' });
        annotationMatcherWorker.onmessage = handleWorkerMessage;

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

            // Setup autosave interval
            autosaveIntervalId = setInterval(async () => {
                const currentProjectState = get(project);
                if (currentProjectState.autosaveEnabled && 
                    currentProjectState.isPdfAnnotationsDirty && 
                    pdfPath === currentProjectState.selectedDocumentPath) {
                    console.log('[PDFViewerPanel Autosave] Dirty PDF annotations detected. Autosaving...');
                    try {
                        await saveCurrentPdfAnnotations();
                        console.log('[PDFViewerPanel Autosave] PDF annotations autosaved successfully.');
                    } catch (error) {
                        console.error('[PDFViewerPanel Autosave] Error during autosave:', error);
                        // Optionally, dispatch a non-intrusive notification to the user about the failed autosave
                    }
                }
            }, 60000); // 60,000 milliseconds = 1 minute

        }, 100); 
    });

    onDestroy(() => {
        if (autosaveIntervalId) {
            clearInterval(autosaveIntervalId);
        }
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
        if (annotationMatcherWorker) {
            annotationMatcherWorker.terminate();
            annotationMatcherWorker = null;
        }
    });

    async function handleWorkerMessage(event) {
        const { pageIndex, annotationId, startIndex, matchLength, error } = event.data;
        const workerTaskKey = `${pageIndex}-${annotationId}`;
        pendingWorkerTasks.delete(workerTaskKey); // Remove from pending tasks

        if (error) {
            console.warn(`[Worker Message] Error for annotation ${annotationId} on page ${pageIndex + 1}: ${error}`);
            return;
        }

        if (startIndex !== -1 && matchLength > 0) {
            // console.log(`[Worker Message] Match found for ${annotationId} on page ${pageIndex + 1}. Start: ${startIndex}, Length: ${matchLength}`);
            const pageView = pdfViewer?.getPageView(pageIndex);
            const layerDiv = pageView?.textLayer?.textLayerDiv;

            if (!pageView || !layerDiv) {
                console.warn(`[Worker Message] Could not find pageView or layerDiv for page ${pageIndex + 1} to apply highlight ${annotationId}.`);
                return;
            }

            // Use a dummy normalizedExpectedText for now, as worker already did the match.
            // Or, pass the original normalized text from worker if needed for verification in findRangeInTextLayer.
            const range = findRangeInTextLayer(layerDiv, startIndex, matchLength, "");

            if (range) {
                const pageRect = pageView.div.getBoundingClientRect();
                const clientRects = range.getClientRects();
                const newQuadPoints = [];
                for (let i = 0; i < clientRects.length; i++) {
                    const rect = clientRects[i];
                    newQuadPoints.push([
                        rect.left - pageRect.left, rect.top - pageRect.top,
                        rect.right - pageRect.left, rect.top - pageRect.top,
                        rect.left - pageRect.left, rect.bottom - pageRect.top,
                        rect.right - pageRect.left, rect.bottom - pageRect.top
                    ]);
                }

                if (newQuadPoints.length > 0) {
                    const highlight = initialHighlights.find(h => h.id === annotationId && h.pageIndex === pageIndex);
                    if (highlight) {
                        renderHighlightOverlay(newQuadPoints, highlight.color, highlight.id, pageIndex);
                        // console.log(`[Worker Message] Rendered ${annotationId} via worker result and generated quadPoints.`);

                        // Update initialHighlights with the new quadPoints to avoid future worker calls for this item
                        highlight.quadPoints = newQuadPoints;
                        // No explicit store dispatch here as per subtask, but this is where it would go if persistence is needed.
                        // Mark dirty if quadpoints are generated and should be saved
                        markPdfAnnotationsDirty();

                    } else {
                        console.warn(`[Worker Message] Highlight ${annotationId} not found in initialHighlights after worker processing.`);
                    }
                } else {
                    console.warn(`[Worker Message] Text found by worker for ${annotationId}, but failed to generate quadPoints from range.`);
                }
            } else {
                console.warn(`[Worker Message] Text found by worker for ${annotationId}, but findRangeInTextLayer failed to create range on main thread.`);
            }
        } else {
            // This case should be covered by 'error' from worker, but as a fallback:
            console.warn(`[Worker Message] No match found by worker for ${annotationId} on page ${pageIndex + 1}.`);
        }
    }

    function handleClickOutside(event) {
        if (isToolbarHighlightDropdownOpen && highlightDropdownRef && !highlightDropdownRef.contains(event.target) && !event.target.closest('[role="menuitem"]')) {
            isToolbarHighlightDropdownOpen = false; selectedRange = null;
        }
        if (showSelectionToolbar && selectionToolbarElement && !selectionToolbarElement.contains(event.target) && !(highlightDropdownRef && highlightDropdownRef.contains(event.target))) {
            const isInsideViewer = viewerElement?.contains(event.target);
            // Ensure we check for both old span-based highlights and new overlay parts
            const clickedOnExistingHighlight = event.target.closest?.('.pdf-highlight') || event.target.closest?.('.overlay-part');

            if (!isInsideViewer) { // Click is outside the PDF viewer area
                hideSelectionToolbar();
            } else {
                // Click is inside the viewer. Hide only if not on an existing highlight AND selection is collapsed.
                if (!clickedOnExistingHighlight && window.getSelection()?.isCollapsed) {
                    hideSelectionToolbar();
                }
                // If clickedOnExistingHighlight is true, toolbar remains (handled by handleViewerClick).
                // If selection is not collapsed (e.g. user just made a selection), toolbar remains.
            }
        }
    }

    async function handleViewerMouseUp(event) {
        if (selectionToolbarElement?.contains(event.target) || highlightDropdownRef?.contains(event.target)) return;
        await tick(); // Allow selection to finalize in the DOM

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
                clearTimeout(hideToolbarTimeoutId); 
                selectedRange = range.cloneRange(); 
                clickedHighlightId = null; clickedHighlightColor = null; toolbarMode = 'selection';

                // Set showSelectionToolbar to true as early as possible
                showSelectionToolbar = true; 

                // Defer positioning until after the next frame, ensuring the toolbar element is available
                // and Svelte has processed the showSelectionToolbar change.
                requestAnimationFrame(() => {
                    // Ensure the toolbar is still meant to be shown and elements are available
                    if (showSelectionToolbar && selectionToolbarElement && pdfViewerWrapperElement) {
                        positionToolbarAtPoint(event.clientX, event.clientY);
                    }
                });
                return;
            }
        }
        selectedRange = null; if (toolbarMode === 'selection' && showSelectionToolbar) hideSelectionToolbar();
    }

    async function handleViewerClick(event) {
        if (selectionToolbarElement?.contains(event.target) || highlightDropdownRef?.contains(event.target)) return;
        // Detect click on either old span or new overlay rectangles
        let highlightSpan = event.target.closest?.('.pdf-highlight');
        if (!highlightSpan) {
            const els = document.elementsFromPoint(event.clientX, event.clientY);
            highlightSpan = els.find(el => el.classList?.contains('overlay-part')) || null;
        }
        const sel = window.getSelection();
        if (highlightSpan && viewerContainer.contains(highlightSpan)) {
            clearTimeout(hideToolbarTimeoutId);
            // For overlay-part, get id and color from dataset
            const id = highlightSpan.dataset.hlId; 
            const color = highlightSpan.dataset.hlColor || highlightSpan.style.backgroundColor;
            if (id !== clickedHighlightId || !showSelectionToolbar) {
                clickedHighlightId = id; clickedHighlightColor = color; selectedRange = null; toolbarMode = 'click';
                // Try to select the corresponding highlight span, else use overlay's bounding rect
                let clickRange = null;
                const span = viewerContainer.querySelector(`.pdf-highlight[data-hl-id="${id}"]`);
                if (span) {
                    clickRange = document.createRange(); clickRange.selectNodeContents(span);
                } else if (highlightSpan.getBoundingClientRect) {
                    // Fallback: create a fake range for overlay rect (for toolbar positioning)
                    clickRange = document.createRange();
                    // Find nearest textLayer for this page
                    let pageDiv = highlightSpan.closest?.('.page');
                    let textLayer = pageDiv?.querySelector('.textLayer');
                    if (textLayer && textLayer.firstChild) {
                        clickRange.selectNodeContents(textLayer.firstChild);
                    }
                }
                // Set showSelectionToolbar to true as early as possible
                showSelectionToolbar = true;
                
                // Defer positioning to next animation frame
                requestAnimationFrame(() => {
                    if (showSelectionToolbar && selectionToolbarElement && pdfViewerWrapperElement) {
                        if (toolbarMode === 'click') {
                            positionToolbarAtPoint(event.clientX, event.clientY);
                        }
                        // The 'else if (toolbarMode === 'selection' && clickRange)' block was removed
                        // as it was determined to be unreachable.
                    }
                });
            }
            event.stopPropagation();
        } else if (!sel || sel.isCollapsed) { hideSelectionToolbar(); }
    }

    // positionAndShowSelectionToolbar function has been removed as it was dead code.

    /** Position toolbar at given client coordinates (for click-mode) */
    function positionToolbarAtPoint(clientX, clientY) {
        if (!selectionToolbarElement || !pdfViewerWrapperElement) {
            // console.warn('[positionToolbarAtPoint] Missing elements');
            return;
        }
        
        const containerRect = pdfViewerWrapperElement.getBoundingClientRect();
        const toolbarRect = selectionToolbarElement.getBoundingClientRect();
        const toolbarHeight = toolbarRect.height;
        const toolbarWidth = toolbarRect.width;

        if (!toolbarHeight || !toolbarWidth) { /* console.warn('Toolbar no dimensions'); */ return; }

        // Horizontal centering
        let left = clientX - containerRect.left - (toolbarWidth / 2);
        left = Math.max(0, Math.min(containerRect.width - toolbarWidth - 5, left));

        // Vertical positioning
        let top = clientY - containerRect.top - toolbarHeight - 8; // Try above cursor

        const spaceAbove = clientY - containerRect.top;
        const spaceBelow = containerRect.height - (clientY - containerRect.top);

        if (top < 0 || (spaceAbove < toolbarHeight + 8 && spaceBelow > toolbarHeight + 8)) {
            top = clientY - containerRect.top + 8; // Position below cursor
        }

        top = Math.max(0, top); 
        if (top + toolbarHeight > containerRect.height - 5) { 
            top = containerRect.height - toolbarHeight - 5;
            if (top < 0) top = 0; 
        }
        
        selectionToolbarLeft = left;
        selectionToolbarTop = top;
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
        // Removed initial await tick() to prioritize immediate UI response.
        // Ticks for store updates will be handled within deferred tasks if necessary.

        if (toolbarMode === 'selection') { // ADDING NEW HIGHLIGHT
            if (!selectedRange) { console.warn("Highlight Action (Selection Mode): No stored selection range found."); return; }

            const rangeToUse = selectedRange.cloneRange(); // Clone synchronously
            const newHighlightId = `hl-${uuidv4()}`;

            // --- Immediate Visual Update ---
            const visualRendered = applyHighlightToSelectionDOM(rangeToUse, color, newHighlightId);
            // applyHighlightToSelectionDOM now primarily focuses on calling renderHighlightOverlay
            // which should be relatively fast.

            hideSelectionToolbar();
            window.getSelection()?.removeAllRanges();

            if (!visualRendered) { // If applyHighlightToSelectionDOM failed (e.g. no page context)
                console.warn("Visual rendering of new highlight failed. Aborting deferred tasks.");
                return;
            }

            // --- Deferred Data Processing & State Updates ---
            deferTask(async () => {
                try {
                    const dataForStorage = await createHighlightDataForStorage(newHighlightId, rangeToUse, color);
                    if (dataForStorage) {
                        dispatch('pdfhighlightevent', { type: 'add', ...dataForStorage });
                        // For undo, capture necessary info. rangeData might be complex to restore perfectly,
                        // but text and context are good.
                        const rangeDataForUndo = captureRangeDataForUndo(rangeToUse);
                        recordAction('addHighlight', { id: newHighlightId, color, rangeData: rangeDataForUndo, dataForStorage });

                        await tick(); // Allow store dispatch to settle if needed by markPdfAnnotationsDirty
                        markPdfAnnotationsDirty();
                        // console.log(`[Highlight Action Defer] Added highlight ${newHighlightId}`);
                    } else {
                        console.warn(`[Highlight Action Defer] Failed to create dataForStorage for ${newHighlightId}. Highlight might not be saved correctly.`);
                        // Potentially remove the visual highlight here if data creation is critical
                        // removeHighlightOverlay(newHighlightId); // Or removeClickedHighlightBlockDOM if spans were created
                    }
                } catch (e) {
                    console.error(`[Highlight Action Defer] Error processing new highlight ${newHighlightId}:`, e);
                }
            });

        } else if (toolbarMode === 'click') { // MODIFYING OR REMOVING EXISTING HIGHLIGHT
            if (!clickedHighlightId) { console.warn("Highlight Action (Click Mode): clickedHighlightId is null!"); return; }

            const currentHighlightId = clickedHighlightId;
            const originalColor = clickedHighlightColor; // Stored when highlight was clicked
            const originalHighlightData = initialHighlights.find(h => h.id === currentHighlightId);

            if (!originalHighlightData && color !== 'remove') {
                console.warn(`[Highlight Action] Original data for highlight ${currentHighlightId} not found. Cannot update color.`);
                hideSelectionToolbar();
                return;
            }

            if (color === 'remove') {
                // --- Immediate Visual Update ---
                removeHighlightOverlay(currentHighlightId);
                // removeClickedHighlightBlockDOM(currentHighlightId); // If we need to remove old span structure too

                hideSelectionToolbar();

                // --- Deferred Data Processing & State Updates ---
                deferTask(async () => {
                    try {
                        dispatch('pdfhighlightevent', { type: 'remove', id: currentHighlightId });
                        const dataForStorageUndo = originalHighlightData
                            ? { ...originalHighlightData }
                            : { id: currentHighlightId, type: 'pdfHighlight', color: originalColor, text: '', pageIndex: 0 }; // Basic fallback for undo
                        recordAction('removeHighlight', { id: currentHighlightId, color: originalColor, dataForStorage: dataForStorageUndo });

                        await tick();
                        markPdfAnnotationsDirty();
                        // console.log(`[Highlight Action Defer] Removed highlight ${currentHighlightId}`);
                    } catch (e) {
                        console.error(`[Highlight Action Defer] Error processing highlight removal ${currentHighlightId}:`, e);
                    }
                });

            } else { // Changing color
                // --- Immediate Visual Update ---
                updateHighlightOverlayColor(currentHighlightId, color);
                // changeClickedHighlightColorDOM(currentHighlightId, color); // If old span structure needs update

                hideSelectionToolbar();

                // --- Deferred Data Processing & State Updates ---
                deferTask(async () => {
                    try {
                        const updatedDataForStorage = { ...originalHighlightData, color };
                        dispatch('pdfhighlightevent', { type: 'update', ...updatedDataForStorage });
                        recordAction('changeColor', {
                            id: currentHighlightId,
                            oldColor: originalColor,
                            newColor: color,
                            dataForStorage: updatedDataForStorage
                        });

                        await tick();
                        markPdfAnnotationsDirty();
                        // console.log(`[Highlight Action Defer] Changed color for highlight ${currentHighlightId}`);
                    } catch (e) {
                        console.error(`[Highlight Action Defer] Error processing highlight color change ${currentHighlightId}:`, e);
                    }
                });
            }
        } else {
            console.warn("Highlight Action: Invalid toolbarMode:", toolbarMode);
            hideSelectionToolbar(); // Hide toolbar if mode is invalid
        }
        // `success` flag removed as visual part is immediate, deferred part handles its own errors.
        // Hiding toolbar and clearing selection (for new highlights) is done immediately.
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
        if (!range || range.collapsed || !color || !viewerElement || !pdfViewer) return null;
        const hlId = overrideId || `hl-${uuidv4()}`;
        
        const { pageIndex, pageElement } = getRangePageInfo(range);
        if (pageIndex < 0) return null;

        let actualPageElement = pageElement;
        if (!actualPageElement) {
            const pageView = pdfViewer.getPageView(pageIndex);
            actualPageElement = pageView?.div;
        }

        if (!actualPageElement) {
            console.warn('[applyHighlightToSelectionDOM] Could not obtain pageElement for quadPoints calculation.');
            return null; 
        }
        const pageRect = actualPageElement.getBoundingClientRect();
        const clientRects = range.getClientRects();
        const quadPoints = [];

        for (let i = 0; i < clientRects.length; i++) {
            const rect = clientRects[i];
            quadPoints.push([
                rect.left - pageRect.left, rect.top - pageRect.top,
                rect.right - pageRect.left, rect.top - pageRect.top,
                rect.left - pageRect.left, rect.bottom - pageRect.top,
                rect.right - pageRect.left, rect.bottom - pageRect.top
            ]);
        }
        
        renderHighlightOverlay(quadPoints, color, hlId, pageIndex);
        return hlId;
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
        // Remove overlay parts
        removeHighlightOverlay(id);
    }

    function changeClickedHighlightColorDOM(id, color) {
        if (!id || !color || !viewerContainer) return;
        const spans = viewerContainer.querySelectorAll(`.pdf-highlight[data-hl-id="${id}"]`);
        if (spans.length === 0) return;
        spans.forEach(span => { span.style.backgroundColor = color; span.dataset.hlColor = color; });
        if (clickedHighlightId === id) clickedHighlightColor = color;
        // Update overlay parts as well
        updateHighlightOverlayColor(id, color);
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
                // Prevent re-running on first user highlight: mark initial annotation pass done
                initialHighlightsApplied = true;
                return;
            }
            await new Promise(r => setTimeout(r, 100));
        }
        // Last‑ditch attempt (handles case where highlights arrive late but within timeout)
        if (initialHighlights && initialHighlights.length > 0) {
            await applyInitialHighlights();
        }
        // Prevent re-running on first user highlight: mark initial annotation pass done
        initialHighlightsApplied = true;
    }

    function setupViewerEvents() {
        if (!eventBus) return;
        // Clear existing listeners on the eventBus instance IF it's being reused
        // However, since we create `new EventBus()` in loadPdfAndLibraries, this might not be strictly necessary
        // unless setupViewerEvents could be called multiple times with the same eventBus instance.
        // For safety, a more robust clear would involve eventBus.off(eventName, handler) for each.
        // A simple _listeners = {} might work for the default EventBus but isn't a public API.
        // eventBus._listeners = {}; // Risky, internal property.

        eventBus.on('pagechanging', (e) => { // Made synchronous again as loadAnnotationsForPageRange is async
            if (e.pageNumber && e.pageNumber !== currentPageNum) {
                currentPageNum = e.pageNumber;
                pageRendering = true;
                hideSelectionToolbar();

                if (pdfViewer && pdfDoc && numPages > 0) {
                    // console.log(`[pagechanging] event for page ${currentPageNum}. Triggering annotation load for target page.`);
                    loadAnnotationsForPageRange(currentPageNum - 1); // Process the target page
                }
                // The loop for re-applying highlights to the visible buffer has been removed.
                // Re-rendering now relies more on textlayerrendered.
            }
        });
        eventBus.on('pagerendered', (e) => { 
            // This event signifies that a page's main rendering (canvas) is done.
            // Text layer and annotations are handled by 'textlayerrendered' and 'pagechanging'.
            if (e.pageNumber === currentPageNum) pageRendering = false; 
            // console.log(`Page ${e.pageNumber} rendered (main content).`);
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
            if (pdfViewer && pdfDoc && numPages > 0) {
                const pageIndex = evt.pageNumber - 1; // 0-based
                // console.log(`[textlayerrendered] event for page ${evt.pageNumber}. Loading range and then applying highlights for this page.`);
                await loadAnnotationsForPageRange(pageIndex); // Ensure annotations in range are loaded (if not already)
                await applyHighlightsForPage(pageIndex);     // Then, specifically re-render this page's highlights
            }
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
        if (initialHighlightsApplied || !pdfDoc || !pdfViewer || !viewerElement || numPages === 0) return;
        if (!initialHighlights || initialHighlights.length === 0) {
            initialHighlightsApplied = true;
            // Ensure isLoadingInitialAnnotations is false if we return early
            isLoadingInitialAnnotations = false;
            return;
        }

        isLoadingInitialAnnotations = true; // Set true for initial annotation load
        loadingMessage = 'Loading initial annotations...';
        console.log(`[PDFViewerPanel applyInitialHighlights] ${loadingMessage}`);
        await tick();

        // Determine the initial page to load (current page in view)
        const initialPageToLoad = pdfViewer.currentPageNumber - 1; // 0-based

        try {
            await loadAnnotationsForPageRange(initialPageToLoad);
        } catch (e) {
            console.error("[PDFViewerPanel applyInitialHighlights] Error during loadAnnotationsForPageRange:", e);
            // Optionally set an error message for the user
        } finally {
            initialHighlightsApplied = true; // Mark that initial pass is done
            isLoadingInitialAnnotations = false; // Set false when done
            loadingMessage = ''; // Reset loading message
            console.log('[PDFViewerPanel applyInitialHighlights] Finished applying initial batch of highlights.');
        }
    }

// Function to load annotations for a specific range of pages
async function loadAnnotationsForPageRange(centerPageIndex) {
    if (!pdfDoc || !pdfViewer || !initialHighlights || initialHighlights.length === 0 || numPages === 0) {
        // console.log('[loadAnnotationsForPageRange] Pre-conditions not met, skipping.', {pdfDoc: !!pdfDoc, pdfViewer: !!pdfViewer, initialHighlights: initialHighlights?.length, numPages});
        return;
    }

    // console.log(`[loadAnnotationsForPageRange] Focusing on centerPageIndex: ${centerPageIndex + 1}.`);

    // This function now only processes the centerPageIndex.
    // The isEagerLoad flag passed to renderAnnotationsForPage will always be false from here.
    if (!loadedPagesWithAnnotations.has(centerPageIndex)) {
        // console.log(`[loadAnnotationsForPageRange] Annotations for page ${centerPageIndex + 1} not yet loaded. Rendering.`);
        await renderAnnotationsForPage(centerPageIndex); // Call without isEagerLoad
        loadedPagesWithAnnotations.add(centerPageIndex);
        await tick(); // Allow UI to update if this was the first load for this page.
    } else {
        // console.log(`[loadAnnotationsForPageRange] Annotations for page ${centerPageIndex + 1} already considered loaded.`);
        // Re-rendering of already loaded pages, if necessary, is handled by calls to applyHighlightsForPage
        // in pagechanging or textlayerrendered event handlers.
    }
}

// Renders annotations for a single given page index (0-based)
async function renderAnnotationsForPage(pageIndex) { // Removed isEagerLoad
    if (!initialHighlights?.length || !pdfViewer || !pdfDoc) {
        // console.log(`[renderAnnotationsForPage] Pre-conditions not met for page ${pageIndex + 1}`);
        return;
    }

    const pageHighlights = initialHighlights.filter(hl => hl.pageIndex === pageIndex);
    if (!pageHighlights.length) {
        // console.log(`[renderAnnotationsForPage] No highlights to render for page ${pageIndex + 1}.`);
        return;
    }

    // console.debug(`[renderAnnotationsForPage] Page ${pageIndex + 1}. Rendering ${pageHighlights.length} highlights.`);

    let pageView = pdfViewer.getPageView(pageIndex);
    try {
        pageView = await ensureTextLayerReady(pageView, pageIndex); // Call without isEagerLoad
    } catch(e) {
        console.warn(`[renderAnnotationsForPage] Failed to ensure page ${pageIndex + 1} ready. Error: ${e.message}`);
        if (!pageView) return;
    }

    if (!pageView) {
        // console.log(`[renderAnnotationsForPage] PageView for ${pageIndex + 1} not available. Cannot render highlights.`);
        return;
    }

    const layerDiv = pageView.textLayer?.textLayerDiv;
    const pdfPage = pageView.pdfPage;

    if (!pageView.div) {
        console.warn(`[renderAnnotationsForPage] No pageView.div for page ${pageIndex + 1}. Cannot apply highlights.`);
        return;
    }
    if (!pdfPage && pageHighlights.some(hl => !hl.quadPoints && hl.text)) {
         console.warn(`[renderAnnotationsForPage] No pdfPage for page ${pageIndex + 1}, text-based fallbacks will fail.`);
    }


    for (const hl of pageHighlights) {
        if (!hl.id || !hl.color) {
            continue;
        }

        if (hl.quadPoints && hl.quadPoints.length > 0) {
            // console.debug(`[renderAnnotationsForPage] ID ${hl.id} on page ${pageIndex + 1} using existing quadPoints.`);
            renderHighlightOverlay(hl.quadPoints, hl.color, hl.id, pageIndex);
        } else if (hl.text && annotationMatcherWorker) {
            // Text match fallback
            // Now, ensureTextLayerReady (always in non-eager mode) should have prepared the text layer for an active page.
            // If layerDiv or pdfPage is still not available here, it's a genuine problem for an active page.
            if (!layerDiv || !pdfPage) {
                console.warn(`[renderAnnotationsForPage] Text layer/pdfPage for page ${pageIndex + 1} not ready. Cannot text-match ID ${hl.id}. PV State: ${pageView?.textLayer?.renderingDone}`);
                continue; // Skip this highlight if text layer isn't ready
            }

            const workerTaskKey = `${pageIndex}-${hl.id}`;
            if (pendingWorkerTasks.has(workerTaskKey)) {
                continue;
            }
            // console.warn(`[renderAnnotationsForPage] Missing quadPoints for ID ${hl.id} on page ${pageIndex + 1}. Attempting text match via Web Worker.`);
            pendingWorkerTasks.add(workerTaskKey);

            try {
                const textContent = await pdfPage.getTextContent({ normalizeWhitespace: true, includeMarkedContent: false });
                annotationMatcherWorker.postMessage({
                    pageIndex: pageIndex,
                    annotationId: hl.id,
                    annotationText: hl.text,
                    annotationPrefix: hl.prefix,
                    annotationSuffix: hl.suffix,
                    annotationOccurrence: hl.occurrenceInPageContext,
                    pageTextContentItems: textContent.items
                });
            } catch (e) {
                console.error(`[renderAnnotationsForPage] Error getting textContent or posting to worker for ${hl.id}:`, e);
                pendingWorkerTasks.delete(workerTaskKey);
            }
        } else if (!hl.quadPoints && !hl.text) {
            // console.warn(`[renderAnnotationsForPage] Skipping highlight ID ${hl.id} on page ${pageIndex + 1}: No quadPoints and no text.`);
        } else if (hl.text && (!layerDiv || !pdfPage)) {
            // This case is now largely covered by the check before attempting worker call.
            // console.warn(`[renderAnnotationsForPage] Cannot attempt text match for ${hl.id} on page ${pageIndex + 1}: textLayerDiv or pdfPage not available. PV State: ${pageView?.textLayer?.renderingDone}`);
        }
    }
}

// This function is no longer the primary way to apply highlights on page load/render.
// It's kept for potential direct calls if needed, but should respect loadedPagesWithAnnotations.
// Or it can be removed if renderAnnotationsForPage and loadAnnotationsForPageRange cover all needs.
async function applyHighlightsForPage(pageIndex) {
    // console.log(`[applyHighlightsForPage] Re-applying highlights for page ${pageIndex + 1}.`);
    // This function is now primarily for re-rendering.
    // It assumes that if a page is in loadedPagesWithAnnotations, its data is ready in initialHighlights.
    // If quadPoints were missing and generated by worker, they'd be in initialHighlights now.

    // We don't check loadedPagesWithAnnotations here because this function is explicitly called
    // to re-render pages that *are* considered loaded and visible.
    // The `loadAnnotationsForPageRange` function is responsible for initially populating `loadedPagesWithAnnotations`.

    await renderAnnotationsForPage(pageIndex); // This will render using initialHighlights data

    // Ensure the page is marked as loaded if it wasn't (e.g. if called directly for some reason)
    // and it actually has highlights.
    if (!loadedPagesWithAnnotations.has(pageIndex) && initialHighlights.some(hl => hl.pageIndex === pageIndex)) {
        // console.log(`[applyHighlightsForPage] Marking page ${pageIndex+1} as loaded after direct render.`);
        loadedPagesWithAnnotations.add(pageIndex);
    }
}
// --- Continuous Highlight Overlay Helpers ---
/** Ensure each PDF page div has a relative-positioned overlay container */
function ensureHighlightOverlayContainer(pageIndex) {
    const pageView = pdfViewer.getPageView(pageIndex);
    if (!pageView || !pageView.div) return null;
    const pageDiv = pageView.div;
    let overlay = pageDiv.querySelector('.highlight-overlay');
    if (!overlay) {
        pageDiv.style.position = 'relative';
        overlay = document.createElement('div');
        overlay.className = 'highlight-overlay';
        Object.assign(overlay.style, {
            position: 'absolute', top: '0', left: '0', width: '100%', height: '100%', pointerEvents: 'none', zIndex: 5
        });
        pageDiv.appendChild(overlay);
    }
    return overlay;
}

/** Draws highlight overlay rectangles using pre-calculated quadPoints. */
function renderHighlightOverlay(quadPoints, color, id, pageIndex) {
    const overlay = ensureHighlightOverlayContainer(pageIndex);
    if (!overlay) return;

    // Remove any existing parts for this highlight id to prevent duplicates
    overlay.querySelectorAll(`.overlay-part[data-hl-id="${id}"]`).forEach(el => el.remove());

    if (!quadPoints || quadPoints.length === 0) {
        // console.warn(`[renderHighlightOverlay] No quadPoints provided for id ${id} on page ${pageIndex}. Nothing to render.`);
        return;
    }

    quadPoints.forEach(quad => {
        // quad is [x1, y1, x2, y2, x3, y3, x4, y4]
        // These are already page-relative coordinates.
        const x1 = quad[0];
        const y1 = quad[1];
        const x2 = quad[2];
        // y2 is quad[3]
        // x3 is quad[4]
        const y3 = quad[5];
        // x4 is quad[6]
        // y4 is quad[7]

        const rectEl = document.createElement('div');
        rectEl.className = 'overlay-part';
        rectEl.dataset.hlId = id;
        rectEl.dataset.hlColor = color;
        
        Object.assign(rectEl.style, {
            position: 'absolute',
            left: `${x1}px`,
            top: `${y1}px`, // y1 is the top-left y, y2 is the top-right y. For axis-aligned, they are the same.
            width: `${x2 - x1}px`, // x2-x1 is width
            height: `${y3 - y1}px`, // y3-y1 is height (y3 is bottom-left y)
            backgroundColor: color,
            borderRadius: '2px',
            pointerEvents: 'auto' // Allow clicks on the overlay part
        });
        overlay.appendChild(rectEl);
    });
}

/** Remove overlay parts for a given highlight id across all pages */
function removeHighlightOverlay(id) {
    document.querySelectorAll(`.highlight-overlay .overlay-part[data-hl-id="${id}"]`).forEach(el => el.remove());
}

/** Update overlay color when highlight color changes */
function updateHighlightOverlayColor(id, color) {
    document.querySelectorAll(`.highlight-overlay .overlay-part[data-hl-id="${id}"]`).forEach(el => {
        el.style.backgroundColor = color;
    });
}

    // Helper: Maps a normalized offset within a node's normalized text to a raw offset within its raw text.
    function mapNormalizedOffsetInNodeToRawOffset(rawTextNodeContent, targetNormalizedOffsetInNode) {
        if (targetNormalizedOffsetInNode === 0) return 0;
        
        for (let r = 0; r < rawTextNodeContent.length; r++) { // r is the raw character index
            const normLenBeforeR = (r > 0) ? normalizeTextForMatching(rawTextNodeContent.substring(0, r)).length : 0;
            const normLenIncludingR = normalizeTextForMatching(rawTextNodeContent.substring(0, r + 1)).length;

            if (normLenIncludingR > targetNormalizedOffsetInNode && normLenBeforeR <= targetNormalizedOffsetInNode) {
                return r;
            }
        }
        
        if (normalizeTextForMatching(rawTextNodeContent).length === targetNormalizedOffsetInNode) {
            return rawTextNodeContent.length;
        }
        return rawTextNodeContent.length; 
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
                    endNode = currentNode;
                    const normalizedEndInNode = targetEndNormalizedGlobal - accumulatedNormalizedCharsBeforeNode;
                    endOffsetRaw = mapNormalizedOffsetInNodeToRawOffset(nodeTextRaw, normalizedEndInNode);
                    break; 
                }
            }
            accumulatedNormalizedCharsBeforeNode += nodeNormalizedLength;
        }

        if (startNode && endNode && startOffsetRaw !== -1 && endOffsetRaw !== -1) {
            try {
                range.setStart(startNode, startOffsetRaw);
                range.setEnd(endNode, endOffsetRaw);

                const actualNormalizedTextFromRange = normalizeTextForMatching(range.toString());
                if (actualNormalizedTextFromRange !== normalizedExpectedText && normalizedExpectedText.length > 0) { // Only warn if expected text was provided
                    console.warn(
                        `[findRangeInTextLayer] Verification mismatch. Expected: "${normalizedExpectedText.substring(0,30)}...", Got: "${actualNormalizedTextFromRange.substring(0,30)}...".`
                    );
                }
                return range;
            } catch (e) { console.error("[findRangeInTextLayer] Error setting/verifying range:", e, {startNode, startOffsetRaw, endNode, endOffsetRaw}); return null; }
        }
        return null;
    }

    /* ─────────────────────────── Toolbar Actions (from your complete version) ────────────────── */
    function goToPrevPage() { if (pdfViewer && currentPageNum > 1) pdfViewer.previousPage(); }
    function goToNextPage() { if (pdfViewer && currentPageNum < numPages) pdfViewer.nextPage(); }
    function zoomOut() { if (pdfViewer) pdfViewer.decreaseScale(); }
    function zoomIn() { if (pdfViewer) pdfViewer.increaseScale(); }
    function setZoom(value) { if (pdfViewer && value) { pdfViewer.currentScaleValue = value; } }
    function handlePageInputChange(e) {
        if (!pdfViewer) return;
        const req = parseInt(e.target.value, 10);
        if (!isNaN(req) && req >= 1 && req <= numPages && req !== currentPageNum) {
            pdfViewer.currentPageNumber = req;
            // When page changes via input, also trigger annotation loading for the new range
            if (pdfDoc && numPages > 0) { // Ensure doc is loaded
                loadAnnotationsForPageRange(req - 1); // 0-based index
            }
        } else {
            e.target.value = currentPageNum;
        }
    }
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
        // Check if storePdfAnnotations actually changed from initialHighlights to prevent loops
        // and ensure it runs when annotations are loaded or updated.
        JSON.stringify(storePdfAnnotations) !== JSON.stringify(initialHighlights)
    ) {
        initialHighlights = storePdfAnnotations;
        loadedPagesWithAnnotations = new Set(); // Reset loaded pages
        initialHighlightsApplied = false; // Allow applyInitialHighlights to run again with new data

        console.log("[PDFViewerPanel Store Sync] Annotations updated from store. Resetting loaded state.");

        // If pages are already initialized and viewer is ready, trigger initial load process.
        // This condition is important to ensure applyInitialHighlights runs after PDF is ready
        // and also when annotations are updated externally (e.g. from another component).
        if (pdfViewer._pages?.length > 0 && !loading) { // Ensure PDF itself is not in a loading state
            console.log("[PDFViewerPanel Store Sync] Applying highlights due to store update.");
            applyInitialHighlights(); // This will call loadAnnotationsForPageRange for the current view
        }
    } else if (
        pdfDoc &&
        pdfViewer &&
        storePdfAnnotations &&
        storePdfAnnotations.length === 0 &&
        initialHighlights.length > 0
    ) {
        // Handle case where all annotations are removed from the store
        console.log("[PDFViewerPanel Store Sync] All annotations removed from store. Clearing UI.");
        initialHighlights = [];
        document.querySelectorAll('.highlight-overlay .overlay-part').forEach(el => el.remove());
        loadedPagesWithAnnotations = new Set();
        initialHighlightsApplied = false; // Reset
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
    <div class="separator"></div>
    {#if !$project.autosaveEnabled && $project.isPdfAnnotationsDirty && pdfPath === $project.selectedDocumentPath}
        <button class="mini-toolbar-button !text-blue-600 dark:!text-blue-400 !border-blue-500 flex items-center space-x-1" 
                on:click={async () => { 
                    try { 
                        await saveCurrentPdfAnnotations(); 
                        console.log('[PDFViewerPanel Manual Save] Saved PDF annotations.'); 
                    } catch (e) { 
                        console.error('[PDFViewerPanel Manual Save] Error saving PDF annotations:', e); 
                        // Optionally dispatch a user-facing error message here
                    } 
                }} 
                title="Save PDF annotations (⌘+S)">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4">
                <path d="M9.25 13.25a.75.75 0 0 01.5 0l5-2.5a.75.75 0 0 0-.042-1.412l-1.316-.333-3.034-4.046a1.75 1.75 0 0 0-3.02.022L5.09 9.005l-1.317.333a.75.75 0 0 0-.042 1.412l5 2.5Z" />
                <path d="M3.513 10.173 2.22 9.84A.75.75 0 0 0 1.173 11.03l3.91 1.563a.75.75 0 0 0 .868-.003l3.91-1.563a.75.75 0 0 0-.707-1.387l-1.29.323L5.81 7.38a.75.75 0 0 0-1.299.028l-1 2.765Z" />
            </svg>
            <span>Save</span>
        </button>
        <div class="separator"></div>
    {/if}
</div>

<div bind:this={pdfViewerWrapperElement} class="flex-grow overflow-hidden bg-gray-200 dark:bg-gray-700 relative pdf-viewer-wrapper">
    {#if error}
        <div class="absolute inset-0 flex items-center justify-center p-4 z-40 pointer-events-none"><div class="text-red-700 dark:text-red-300 p-4 bg-red-100 dark:bg-red-900/80 rounded border border-red-400 dark:border-red-600 max-w-lg text-center shadow-lg"><p class="font-semibold mb-2">Error:</p><p class="text-sm break-words">{@html error}</p></div></div>
    {:else if loading || isLoadingInitialAnnotations}
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