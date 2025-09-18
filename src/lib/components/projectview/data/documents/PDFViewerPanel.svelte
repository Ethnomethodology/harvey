<!-- src/lib/components/projectview/documents/PDFViewerPanel.svelte -->
<script>
    import { onMount, onDestroy, tick, createEventDispatcher } from 'svelte';
    import { readFile } from '@tauri-apps/plugin-fs';
    import { v4 as uuidv4 } from 'uuid';
    import { project } from '$lib/stores/projectStore.js';
    import { saveCurrentPdfAnnotations } from '$lib/services/projectService.js';
import { markPdfAnnotationsDirty } from '$lib/stores/projectStore.js';
import { get } from 'svelte/store';

    let wasPerformingSelection = false;
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
        { value: 'rgba(255, 255, 255, 1)', label: 'None' },
    ];
    // let isToolbarHighlightDropdownOpen = false; // Removed
    // let highlightDropdownRef; // Removed

    let newHighlightDropdownRef; // Added for new dropdown
    let isNewHighlightDropdownOpen = false; // Added for new dropdown

    let zoomDropdownRef;
    let isZoomDropdownOpen = false;

    // Reactive state for Quick Highlight feature
    let isQuickHighlightActive = false;
    let quickHighlightColor = 'rgba(255, 242, 117, 0.5)'; // Default to yellow
    let quickHighlightMode = 'highlight'; // Possible values: 'highlight', 'remove'


    let pageRendering = false; let pageNumInput = currentPageNum; let pdfjsLib = null; let PDFViewer = null; let EventBus = null; let PDFLinkService = null; let PDFFindController = null; 
    // pdfWorkerUrl will be imported dynamically

    const zoomOptions = [
        { value: 'auto', label: 'Auto' },
        { value: 'page-actual', label: 'Actual Size' },
        { value: 'page-fit', label: 'Page Fit' },
        { value: 'page-width', label: 'Page Width' },
        { type: 'separator' },
        { value: '0.5', label: '50%' },
        { value: '0.75', label: '75%' },
        { value: '1', label: '100%' },
        { value: '1.25', label: '125%' },
        { value: '1.5', label: '150%' },
        { value: '2', label: '200%' },
        { value: '3', label: '300%' },
        { value: '4', label: '400%' },
    ];

    const markerIconSVG = `
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-highlighter" viewBox="0 0 16 16">
            <path fill-rule="evenodd" d="M11.096.644a2 2 0 0 1 2.791.036l1.433 1.433a2 2 0 0 1 .035 2.791l-.413.435-8.07 8.995a.5.5 0 0 1-.372.166h-3a.5.5 0 0 1-.234-.058l-.412.412A.5.5 0 0 1 2.5 15h-2a.5.5 0 0 1-.354-.854l1.412-1.412A.5.5 0 0 1 1.5 12.5v-3a.5.5 0 0 1 .166-.372l8.995-8.07zm-.115 1.47L2.727 9.52l3.753 3.753 7.406-8.254zm3.585 2.17.064-.068a1 1 0 0 0-.017-1.396L13.18 1.387a1 1 0 0 0-1.396-.018l-.068.065zM5.293 13.5 2.5 10.707v1.586L3.707 13.5z"/>
        </svg>
    `;

    const removeHighlightIconSVG = `
        <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3 text-gray-500 dark:text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <path stroke-linecap="round" stroke-linejoin="round" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" />
        </svg>
    `;

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
        
        const clientRects = range.getClientRects();
        const pageRect = actualPageElement?.getBoundingClientRect() || { top: 0, left: 0 }; // Fallback to 0,0 if no pageElement
        const quadPoints = processAndMergeQuadPoints(clientRects, pageRect);
        
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
        undoStack = [...undoStack]; // Ensures reactivity after pop
        
        redoStack.push(action);
        redoStack = [...redoStack]; // Ensures reactivity after push
        // console.log('[PDF Undo] Action:', action.type, action.payload?.id);
        switch (action.type) {
            case 'addHighlight':
                removeClickedHighlightBlockDOM(action.payload.id);
                dispatch('pdfhighlightevent', { type: 'remove', id: action.payload.id });
                break;
            case 'removeHighlight':
                if (action.payload.dataForStorage) { // This object should contain all necessary highlight data including quadPoints
                    dispatch('pdfhighlightevent', { type: 'add', ...action.payload.dataForStorage });
                } else {
                    // This case should ideally not be reached if recordAction for 'removeHighlight' always includes dataForStorage
                    console.warn('[PDF Undo] Cannot re-apply highlight (was remove), missing dataForStorage for undo.', action.payload);
                }
                break;
            case 'changeColor':
                if (action.payload.id && action.payload.oldColor && action.payload.dataForStorage) {
                    changeClickedHighlightColorDOM(action.payload.id, action.payload.oldColor);
                    dispatch('pdfhighlightevent', { type: 'update', ...action.payload.dataForStorage, color: action.payload.oldColor });
                } else { console.warn('[PDF Undo] Cannot revert color change, missing data.', action.payload); }
                break;
            case 'updateHighlightQuads': // For partial overlaps
                // Undo the trim: restore original quads
                if (action.payload.id && action.payload.originalHighlightData && action.payload.oldQuads) {
                    dispatch('pdfhighlightevent', {
                        type: 'update',
                        ...action.payload.originalHighlightData, // Spread original data
                        quadPoints: action.payload.oldQuads      // Restore old quads
                    });
                } else { console.warn('[PDF Undo] Cannot revert quad update, missing data.', action.payload); }
                break;
            case 'removeHighlightFromSelection':
                // New undo logic for the revised payload structure
                if (action.payload.updatedOriginalHighlight) {
                    const { id, oldQuads } = action.payload.updatedOriginalHighlight;
                    const originalHlData = initialHighlights.find(h => h.id === id); // Or find from a snapshot if available
                    if (originalHlData) {
                         dispatch('pdfhighlightevent', { type: 'update', ...originalHlData, quadPoints: oldQuads });
                    } else {
                        // This case implies the original highlight data (for other fields like text, color) is missing
                        // We might need to store more in updatedOriginalHighlight if this happens.
                        // For now, just using oldQuads.
                        console.warn(`[PDF Undo removeHighlightFromSelection] Original data for highlight ${id} not found for full update. Restoring with old quads only.`);
                        dispatch('pdfhighlightevent', { type: 'update', id: id, quadPoints: oldQuads });
                    }
                }
                if (action.payload.removedOriginalHighlight) {
                    dispatch('pdfhighlightevent', { type: 'add', ...action.payload.removedOriginalHighlight.originalHighlightData });
                }
                if (action.payload.addedSecondPart) {
                    dispatch('pdfhighlightevent', { type: 'remove', id: action.payload.addedSecondPart.id });
                }
                break;
        }
        hideSelectionToolbar();
    }

    function redo() {
        if (redoStack.length === 0) return;
        const action = redoStack.pop();
        redoStack = [...redoStack]; // Ensures reactivity after pop
        
        undoStack.push(action);
        undoStack = [...undoStack]; // Ensures reactivity after push
        // console.log('[PDF Redo] Action:', action.type, action.payload?.id);
        switch (action.type) {
            case 'addHighlight':
                // The payload for 'addHighlight' actions is typically:
                // { id: newHighlightId, color, rangeData: rangeDataForUndo, dataForStorage }
                // dataForStorage should contain the definitive quadPoints and other necessary info.
                if (action.payload.dataForStorage) {
                    dispatch('pdfhighlightevent', { type: 'add', ...action.payload.dataForStorage });
                } else {
                    // This path should ideally not be hit if dataForStorage is always correctly populated.
                    console.warn('[PDF Redo] Cannot re-do "addHighlight", missing dataForStorage.', action.payload);
                }
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
            case 'updateHighlightQuads': // For partial overlaps
                // Redo the trim: apply new quads
                if (action.payload.id && action.payload.originalHighlightData && action.payload.newQuads) {
                     dispatch('pdfhighlightevent', {
                        type: 'update',
                        ...action.payload.originalHighlightData, // Spread original data
                        quadPoints: action.payload.newQuads      // Apply new quads
                    });
                } else { console.warn('[PDF Redo] Cannot re-apply quad update, missing data.', action.payload); }
                break;
            case 'removeHighlightFromSelection':
                // New redo logic for the revised payload structure
                if (action.payload.updatedOriginalHighlight) {
                    const { id, newQuads } = action.payload.updatedOriginalHighlight;
                     const originalHlData = initialHighlights.find(h => h.id === id); // Or find from a snapshot
                    if (originalHlData) {
                        dispatch('pdfhighlightevent', { type: 'update', ...originalHlData, quadPoints: newQuads });
                    } else {
                         // This implies the highlight was fully removed and then partially restored by undo.
                         // For redo, we'd need to re-add it with newQuads.
                         // This path needs careful consideration of what originalHighlightData should be.
                         // For now, we assume updatedOriginalHighlight implies the item exists for update.
                        console.warn(`[PDF Redo removeHighlightFromSelection] Original data for highlight ${id} not found for full update. Updating with new quads only.`);
                        dispatch('pdfhighlightevent', { type: 'update', id: id, quadPoints: newQuads });
                    }
                }
                if (action.payload.removedOriginalHighlight) {
                    dispatch('pdfhighlightevent', { type: 'remove', id: action.payload.removedOriginalHighlight.originalHighlightData.id });
                }
                if (action.payload.addedSecondPart) {
                    dispatch('pdfhighlightevent', { type: 'add', ...action.payload.addedSecondPart.data });
                }
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
        if (isNewHighlightDropdownOpen && newHighlightDropdownRef && !newHighlightDropdownRef.contains(event.target) && !event.target.closest('[role="menuitem"]')) {
            isNewHighlightDropdownOpen = false;
        }
        if (isZoomDropdownOpen && zoomDropdownRef && !zoomDropdownRef.contains(event.target) && !event.target.closest('[role="menuitem"]')) { // Ensure not clicking on a menu item of the zoom dropdown itself
            isZoomDropdownOpen = false;
        }
        // Ensure the new dropdown check doesn't prevent the selection toolbar from hiding.
        // The condition for hiding selectionToolbar should be independent of newHighlightDropdownRef unless it's part of the selection toolbar itself.
        // Based on current structure, newHighlightDropdownRef is part of the main toolbar, not the floating selection toolbar.
        if (showSelectionToolbar && selectionToolbarElement && !selectionToolbarElement.contains(event.target)) {
            // Check if the click is on any part of the main toolbar that should keep the selection toolbar open (e.g. if it was a highlight color button)
            // For now, we assume any click outside selectionToolbar itself should be evaluated for hiding.
            // The original logic for `!(highlightDropdownRef && highlightDropdownRef.contains(event.target))` was to prevent hiding if clicking the old highlight dropdown.
            // We don't have such a direct equivalent needing to keep selectionToolbar open for the new main toolbar dropdown.
            const isInsideViewer = viewerElement?.contains(event.target);
            const clickedOnExistingHighlight = event.target.closest?.('.pdf-highlight') || event.target.closest?.('.overlay-part');

            if (!isInsideViewer) {
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
        const sel = window.getSelection();

        if (sel && sel.rangeCount > 0 && !sel.isCollapsed) {
            wasPerformingSelection = true;
        }
        // If wasPerformingSelection is not true here, it means mousedown happened, but no drag selection occurred.
        // It will remain `false` as set by handleViewerMouseDown.

        // Defer resetting the flag until after the current event cycle (mouseup + potential click)
        setTimeout(() => { wasPerformingSelection = false; }, 0);

        if (selectionToolbarElement?.contains(event.target) || newHighlightDropdownRef?.contains(event.target)) return;
        await tick();

        if (isQuickHighlightActive) {
            // Only apply quick highlight if a selection was actually performed
            if (wasPerformingSelection && sel && sel.rangeCount > 0 && !sel.isCollapsed) {
                const range = sel.getRangeAt(0); // sel is already defined
                let isInTextLayer = false;
                const ancestor = range.commonAncestorContainer;
                if (ancestor && viewerElement?.contains(ancestor)) {
                    const textLayerParent = (ancestor.nodeType === Node.ELEMENT_NODE ? ancestor : ancestor.parentNode)?.closest('.textLayer');
                    if (textLayerParent && viewerElement.contains(textLayerParent)) isInTextLayer = true;
                }

                if (isInTextLayer && range.toString().trim().length > 0) {
                    const rangeToUse = range.cloneRange();
                    selectedRange = rangeToUse;
                    toolbarMode = 'selection';

                    if (quickHighlightMode === 'highlight') {
                        await handleHighlightAction(quickHighlightColor);
                    } else if (quickHighlightMode === 'remove') {
                        await handleHighlightAction('remove');
                    }

                    window.getSelection()?.removeAllRanges();
                    selectedRange = null;
                }
            }
            return; // If quick highlight is active, we either process it or do nothing else with selection toolbar.
        }

        // Logic for standard floating toolbar (if Quick Highlight is not active)
        if (wasPerformingSelection && sel && sel.rangeCount > 0 && !sel.isCollapsed) { // Check wasPerformingSelection
            const range = sel.getRangeAt(0); // sel is already defined
            let isInTextLayer = false;
            const ancestor = range.commonAncestorContainer;
                if (ancestor && viewerElement?.contains(ancestor)) {
                    const textLayerParent = (ancestor.nodeType === Node.ELEMENT_NODE ? ancestor : ancestor.parentNode)?.closest('.textLayer');
                    if (textLayerParent && viewerElement.contains(textLayerParent)) isInTextLayer = true;
                }

            if (isInTextLayer && range.toString().trim().length > 0) {
                const targetElement = event.target;
                const clickedOnExistingHighlight = targetElement.closest?.('.pdf-highlight') || targetElement.closest?.('.overlay-part');

                if (clickedOnExistingHighlight) {
                    window.getSelection()?.removeAllRanges(); // Clear selection
                    hideSelectionToolbar(); // Hide toolbar
                    return; // Suppress 'selection' toolbar
                }

                // If not clicking on existing highlight, proceed to show 'selection' toolbar
                clearTimeout(hideToolbarTimeoutId); 
                selectedRange = range.cloneRange(); 
                clickedHighlightId = null; clickedHighlightColor = null; toolbarMode = 'selection';
                showSelectionToolbar = true; 

                requestAnimationFrame(() => {
                    if (showSelectionToolbar && selectionToolbarElement && pdfViewerWrapperElement) {
                        positionToolbarAtPoint(event.clientX, event.clientY);
                    }
                });
                return;
            }
        }

        // If no valid selection for toolbar, or if it was suppressed.
        if (toolbarMode === 'selection' && showSelectionToolbar) {
             hideSelectionToolbar();
        }
    }

    async function handleViewerClick(event) {
        // If the click is on the selection toolbar or its dropdown, let normal interaction proceed.
        if (selectionToolbarElement?.contains(event.target) || newHighlightDropdownRef?.contains(event.target)) { // Adjusted for new dropdown ref
            return;
        }

        // NEW CHECK: If this click immediately followed a mouseup that was part of a selection gesture,
        // and that mouseup ended on an existing highlight, we want to suppress the 'click' mode toolbar.
        // `handleViewerMouseUp` would have already cleared the selection and hidden the 'selection' toolbar.
        const targetElement = event.target;
        const clickedOnExistingHighlightTarget = targetElement.closest?.('.pdf-highlight') || targetElement.closest?.('.overlay-part');

        if (wasPerformingSelection && clickedOnExistingHighlightTarget) {
            // This click is the tail end of a selection drag that landed on a highlight.
            // `handleViewerMouseUp` already handled not showing the 'selection' toolbar.
            // We now prevent the 'click' (modify) toolbar from appearing as well.
            // The `wasPerformingSelection` flag will be reset by the timeout scheduled in `handleViewerMouseUp`.
            return;
        }

        // Original logic from here onwards:
        // If a text selection was just made (mouseup set the mode and range),
        // and the selection toolbar is meant to be shown for that new selection,
        // then this click should not try to find a clicked highlight or change the mode.
        if (toolbarMode === 'selection' && selectedRange && showSelectionToolbar) {
            // This block might still be useful if a selection was made on plain text,
            // the 'selection' toolbar appeared, and then a click happens (e.g. to dismiss it).
            // However, if `wasPerformingSelection` was true and `clickedOnExistingHighlightTarget` was false,
            // the above block wouldn't return.
            // This original block primarily prevents switching from 'selection' to 'click' mode
            // if the selection toolbar is already active for a *new* selection.
            return; // Exit early, letting the selection toolbar (from mouseup) be the focus.
        }

        // Detect click on either old span or new overlay rectangles
        // Note: `clickedOnExistingHighlightTarget` is already determined above, can reuse.
        let highlightSpan = clickedOnExistingHighlightTarget;
        if (!highlightSpan) { // If not found by target.closest, try elementsFromPoint (for overlays)
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

            // --- MODIFIED LOGIC BLOCK FOR REMOVING FROM SELECTION --- >>>
            if (color === 'remove') {
                console.log('[handleHighlightAction] Attempting to remove highlight from selection (new logic).');
                hideSelectionToolbar();
                window.getSelection()?.removeAllRanges();

                const { pageIndex: selectionPageIndex, pageElement: selectionPageElement } = getRangePageInfo(rangeToUse);
                if (selectionPageIndex === -1) {
                    console.warn('[handleHighlightAction] Remove from selection: Could not determine page index.');
                    return;
                }

                let actualSelectionPageElement = selectionPageElement;
                if (!actualSelectionPageElement && pdfViewer) {
                    const pageView = pdfViewer.getPageView(selectionPageIndex);
                    actualSelectionPageElement = pageView?.div;
                }
                const selectionPageRect = actualSelectionPageElement?.getBoundingClientRect() || viewerContainer?.getBoundingClientRect() || { top: 0, left: 0 };
                const selectionClientRects = rangeToUse.getClientRects();
                const selectionQuads = processAndMergeQuadPoints(selectionClientRects, selectionPageRect);

                if (!selectionQuads || selectionQuads.length === 0) {
                    console.warn('[handleHighlightAction] Remove from selection: No valid quads for current selection.');
                    return;
                }
                const selectionBBox = getBoundingBoxForQuads(selectionQuads);
                if (!selectionBBox) {
                    console.warn('[handleHighlightAction] Remove from selection: Could not get bounding box for selection quads.');
                    return;
                }

                const highlightsToProcess = [...initialHighlights];
                for (const existingHl of highlightsToProcess) {
                    if (existingHl.pageIndex === selectionPageIndex && existingHl.quadPoints && existingHl.quadPoints.length > 0) {
                        const existingHlBBox = getBoundingBoxForQuads(existingHl.quadPoints);
                        if (!existingHlBBox || !doBoundingBoxesIntersect(selectionBBox, existingHlBBox)) {
                            continue; // No intersection, skip this highlight
                        }

                        const originalExistingQuads = JSON.parse(JSON.stringify(existingHl.quadPoints)); // For undo
                        let quadsBeforeSelection = [];
                        let quadsAfterSelection = [];

                        for (const exQuad of existingHl.quadPoints) {
                            const exQuadBBox = getBoundingBoxForQuads([exQuad]);
                            if (!exQuadBBox) continue;

                            if (!doBoundingBoxesIntersect(exQuadBBox, selectionBBox)) {
                                // Quad does not intersect with the selection bounding box
                                if (exQuadBBox.y2 <= selectionBBox.y1) { // Quad is entirely above selection
                                    quadsBeforeSelection.push(exQuad);
                                } else if (exQuadBBox.y1 >= selectionBBox.y2) { // Quad is entirely below selection
                                    quadsAfterSelection.push(exQuad);
                                } else { // Quad is on the same horizontal band as the selection
                                    if (exQuadBBox.x2 <= selectionBBox.x1) { // Quad is entirely to the left of selection
                                        quadsBeforeSelection.push(exQuad);
                                    } else if (exQuadBBox.x1 >= selectionBBox.x2) { // Quad is entirely to the right of selection
                                        quadsAfterSelection.push(exQuad);
                                    } else {
                                        // This case should be rare if intersection logic is correct.
                                        // Quad is on the same band and horizontally overlapping but wasn't caught by intersection.
                                        console.warn('[handleHighlightAction] Non-intersecting exQuad overlaps selection horizontally. Defaulting to quadsBeforeSelection. exQuadBBox:', exQuadBBox, 'selectionBBox:', selectionBBox);
                                        quadsBeforeSelection.push(exQuad); // Default behavior
                                    }
                                }
                            } else {
                                // Quad intersects with the selection bounding box, subtract and categorize remnants
                                const remnants = subtractQuads([exQuad], selectionQuads);
                                for (const remnantQuad of remnants) {
                                    const remnantBBox = getBoundingBoxForQuads([remnantQuad]);
                                    if (!remnantBBox) continue;

                                    if (remnantBBox.x2 <= selectionBBox.x1) { // Remnant is to the left of the selection
                                        quadsBeforeSelection.push(remnantQuad);
                                    } else if (remnantBBox.x1 >= selectionBBox.x2) { // Remnant is to the right of the selection
                                        quadsAfterSelection.push(remnantQuad);
                                    } else {
                                        // Remnant overlaps horizontally with selection - this implies a vertical cut or complex scenario.
                                        // This case handles remnants that are directly above or below the "hole" punched by selectionQuads
                                        // within the bounds of exQuad if exQuad was taller than the selection.
                                        // Or, if subtractQuads produced a remnant that still overlaps horizontally (which is unexpected for simple hole punching).
                                        console.warn('[handleHighlightAction] Remnant quad overlaps selection horizontally. exQuadBBox:', exQuadBBox, 'remnantBBox:', remnantBBox, 'selectionBBox:', selectionBBox);
                                        // Fallback logic based on vertical position relative to selection (if exQuad is primarily horizontal)
                                        // or horizontal center if truly ambiguous.
                                        if (exQuadBBox.x2 - exQuadBBox.x1 > exQuadBBox.y2 - exQuadBBox.y1) { // exQuad is wider than tall
                                            if (remnantBBox.y2 <= selectionBBox.y1) { // Remnant is above selection
                                                quadsBeforeSelection.push(remnantQuad);
                                            } else if (remnantBBox.y1 >= selectionBBox.y2) { // Remnant is below selection
                                                quadsAfterSelection.push(remnantQuad);
                                            } else { // Fallback to x_center for truly overlapping cases within the selection's vertical span
                                                const remnantCenterX = (remnantBBox.x1 + remnantBBox.x2) / 2;
                                                const selectionCenterX = (selectionBBox.x1 + selectionBBox.x2) / 2;
                                                if (remnantCenterX < selectionCenterX) {
                                                    quadsBeforeSelection.push(remnantQuad);
                                                } else {
                                                    quadsAfterSelection.push(remnantQuad);
                                                }
                                            }
                                        } else { // exQuad is taller than wide (or square) - use original y_center logic or x_center
                                            const remnantCenterX = (remnantBBox.x1 + remnantBBox.x2) / 2;
                                            const selectionCenterX = (selectionBBox.x1 + selectionBBox.x2) / 2;
                                            if (remnantCenterX < selectionCenterX) { // Default to x-based split for ambiguous cases
                                                quadsBeforeSelection.push(remnantQuad);
                                            } else {
                                                quadsAfterSelection.push(remnantQuad);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        const finalQuadsBefore = quadsBeforeSelection.length > 0 ? _simplifyAndMergeRects(quadsBeforeSelection.map(q => quadToRect(q))).map(r => rectToQuad(r)) : [];
                        const finalQuadsAfter = quadsAfterSelection.length > 0 ? _simplifyAndMergeRects(quadsAfterSelection.map(q => quadToRect(q))).map(r => rectToQuad(r)) : [];
                        
                        let undoPayload = {
                            updatedOriginalHighlight: null,
                            removedOriginalHighlight: null,
                            addedSecondPart: null,
                            selectionQuads: JSON.parse(JSON.stringify(selectionQuads)) 
                        };

                        if (finalQuadsBefore.length > 0) {
                            dispatch('pdfhighlightevent', { type: 'update', ...existingHl, quadPoints: finalQuadsBefore });
                            undoPayload.updatedOriginalHighlight = { id: existingHl.id, oldQuads: originalExistingQuads, newQuads: JSON.parse(JSON.stringify(finalQuadsBefore)) };
                        } else {
                            dispatch('pdfhighlightevent', { type: 'remove', id: existingHl.id });
                            undoPayload.removedOriginalHighlight = { originalHighlightData: JSON.parse(JSON.stringify(existingHl)) };
                        }

                        if (finalQuadsAfter.length > 0) {
                            const newSplitHighlightId = `hl-${uuidv4()}`;
                            const newHighlightDataObject = {
                                type: 'pdfHighlight',
                                id: newSplitHighlightId,
                                color: existingHl.color,
                                pageIndex: existingHl.pageIndex,
                                text: existingHl.text, // Text might be inaccurate for the split part
                                quadPoints: finalQuadsAfter,
                                prefix: existingHl.prefix, // Prefixes/suffixes might be inaccurate
                                suffix: existingHl.suffix,
                            };
                            dispatch('pdfhighlightevent', { type: 'add', ...newHighlightDataObject });
                            undoPayload.addedSecondPart = { id: newSplitHighlightId, data: newHighlightDataObject };
                        }
                        
                        markPdfAnnotationsDirty();
                        recordAction('removeHighlightFromSelection', undoPayload);
                    }
                }
                return; 
            }
            // --- END OF MODIFIED LOGIC BLOCK ---

            const newHighlightId = `hl-${uuidv4()}`;
            // Calculate new selection quads first for subsumption check
            const { pageIndex: newSelectionPageIndex, pageElement: newSelectionPageElement } = getRangePageInfo(rangeToUse);
            let actualNewSelectionPageElement = newSelectionPageElement;
            if (!actualNewSelectionPageElement && newSelectionPageIndex !== -1 && pdfViewer) {
                const pageView = pdfViewer.getPageView(newSelectionPageIndex);
                actualNewSelectionPageElement = pageView?.div;
            }
            const newSelectionClientRects = rangeToUse.getClientRects();
            const newSelectionPageRect = actualNewSelectionPageElement?.getBoundingClientRect() || { top: 0, left: 0 };
            const newSelectionProcessedQuads = processAndMergeQuadPoints(newSelectionClientRects, newSelectionPageRect);

            if (newSelectionPageIndex === -1) {
                console.warn('[handleHighlightAction] Could not determine page index for new selection for overlap check. Proceeding without check.');
            } else {
                const highlightsToCheck = [...initialHighlights];
                for (const existingHl of highlightsToCheck) {
                    if (existingHl.pageIndex === newSelectionPageIndex) {
                        if (existingHl.quadPoints && existingHl.quadPoints.length > 0) {
                            if (areQuadsSubsumed(newSelectionProcessedQuads, existingHl.quadPoints)) {
                                console.log(`[handleHighlightAction] New selection subsumes existing highlight ID: ${existingHl.id}. Removing old one.`);
                                dispatch('pdfhighlightevent', { type: 'remove', id: existingHl.id });
                                // For proper undo, we need the full original data including its specific range data if possible.
                                // Using existingHl as dataForStorage is a simplification.
                                recordAction('removeHighlight', {
                                    id: existingHl.id,
                                    color: existingHl.color,
                                    dataForStorage: { ...existingHl }
                                });
                                // The UI for the old highlight (overlay parts) will be removed by the 'remove' event handler
                                // or by the new highlight's render call if IDs match (though they won't here).
                                // No need to call removeHighlightOverlay(existingHl.id) directly here if events handle it.
                            }
                        }
                    }
                }
            }

            // --- Phase 2: Handle Partial Overlaps by Trimming Existing Highlights ---
            const newSelectionBoundingBox = newSelectionProcessedQuads.length > 0 ? getBoundingBoxForQuads(newSelectionProcessedQuads) : null;
            // const highlightsToUpdateForUndo = []; // Not using this batching approach for now

            for (const existingHl of [...initialHighlights]) { // Iterate on a copy
                if (existingHl.pageIndex === newSelectionPageIndex && existingHl.quadPoints && existingHl.quadPoints.length > 0) {

                    // Basic check: if an ID was already processed by subsumption, it might be gone from initialHighlights
                    // or its quadPoints might be empty. A more robust check might involve a list of IDs already handled.
                    // For this iteration, we assume `areQuadsSubsumed` handles distinct cases.
                    // If existingHl was fully subsumed, it should have been removed/marked.
                    // This logic targets *only* partial overlaps.

                    const existingHlBoundingBox = getBoundingBoxForQuads(existingHl.quadPoints);

                    if (newSelectionBoundingBox && doBoundingBoxesIntersect(newSelectionBoundingBox, existingHlBoundingBox) &&
                        !areQuadsSubsumed(newSelectionProcessedQuads, existingHl.quadPoints) && // Not fully subsumed by new
                        !areQuadsSubsumed(existingHl.quadPoints, newSelectionProcessedQuads)      // New not fully subsumed by old
                    ) {
                        console.log(`[handleHighlightAction] Partial overlap detected with existing highlight ID: ${existingHl.id}. Attempting to trim.`);

                        const originalQuadsForUndo = JSON.parse(JSON.stringify(existingHl.quadPoints)); // Deep copy for undo
                        const remainingQuads = subtractQuads(existingHl.quadPoints, newSelectionProcessedQuads);

                        if (remainingQuads.length === 0) {
                            console.log(`[handleHighlightAction] Trimming existing highlight ID: ${existingHl.id} resulted in empty quads. Removing it.`);
                            dispatch('pdfhighlightevent', { type: 'remove', id: existingHl.id });
                            recordAction('removeHighlight', {
                                id: existingHl.id,
                                color: existingHl.color,
                                dataForStorage: { ...existingHl, quadPoints: originalQuadsForUndo }
                            });
                        } else {
                            console.log(`[handleHighlightAction] Updating existing highlight ID: ${existingHl.id} with ${remainingQuads.length} remaining quads.`);

                            dispatch('pdfhighlightevent', {
                                type: 'update',
                                ...existingHl, // Spread existing properties
                                quadPoints: remainingQuads // Override quadPoints
                            });
                            recordAction('updateHighlightQuads', {
                                id: existingHl.id,
                                oldQuads: originalQuadsForUndo,
                                newQuads: remainingQuads,
                                originalHighlightData: JSON.parse(JSON.stringify(existingHl)) // Store all original data
                            });
                        }
                        markPdfAnnotationsDirty();
                    }
                }
            }
            // End of new partial overlap logic

            // --- Immediate Visual Update for the new highlight ---
            const visualRendered = applyHighlightToSelectionDOM(rangeToUse, color, newHighlightId);
            // applyHighlightToSelectionDOM now primarily focuses on calling renderHighlightOverlay

            hideSelectionToolbar();
            window.getSelection()?.removeAllRanges();

            if (!visualRendered && newSelectionProcessedQuads.length > 0) { // Only warn if quads were expected
                console.warn("Visual rendering of new highlight failed, though processed quads were generated. Aborting deferred tasks.");
                return;
            }
            if (!visualRendered && newSelectionProcessedQuads.length === 0) {
                console.warn("Visual rendering of new highlight failed, no processed quads. Aborting deferred tasks.");
                return;
            }


            // --- Deferred Data Processing & State Updates ---
            deferTask(async () => {
                try {
                    // dataForStorage will use newSelectionProcessedQuads via createHighlightDataForStorage
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
    
    // async function handleDropdownHighlightAction(color) { // Old function, to be removed or repurposed
    //     if (!selectedRange) { isToolbarHighlightDropdownOpen = false; return; }
    //     // selectedRange is already cloned from mouseup
    //     isToolbarHighlightDropdownOpen = false;
    //     toolbarMode = 'selection'; // Ensure mode is set for handleHighlightAction
    //     await handleHighlightAction(color); // Pass the selectedRange implicitly
    //     selectedRange = null; // Clear after use
    // }

    function toggleNewHighlightDropdown() { // Added new function
        isNewHighlightDropdownOpen = !isNewHighlightDropdownOpen;
    }

    function toggleZoomDropdown() {
        isZoomDropdownOpen = !isZoomDropdownOpen;
    }

    function selectZoomLevel(value) {
        setZoom(value); // This function already exists and calls pdfViewer.currentScaleValue
        isZoomDropdownOpen = false;
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
        wasPerformingSelection = false; // Reset the flag on every mousedown

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
        const clientRects = range.getClientRects();
        const pageRect = actualPageElement.getBoundingClientRect();
        const quadPoints = processAndMergeQuadPoints(clientRects, pageRect);
        
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
    if (!id) return; // Removed viewerContainer check
    // Span manipulation and normalization removed
        if (clickedHighlightId === id) clickedHighlightId = null;
        removeHighlightOverlay(id);
    }

    function changeClickedHighlightColorDOM(id, color) {
        if (!id || !color) return; // Removed viewerContainer check as updateHighlightOverlayColor doesn't need it directly
        // Spans manipulation removed
        if (clickedHighlightId === id) clickedHighlightColor = color;
        updateHighlightOverlayColor(id, color);
    }

    function unwrapNodeDOM(node) {
        const parent = node.parentNode; if (!parent) return;
        while (node.firstChild) { parent.insertBefore(node.firstChild, node); }
        try { if (parent.contains(node)) parent.removeChild(node); } catch (e) { /* console.error("Unwrap Error:", e, node); */ }
    }

function processAndMergeQuadPoints(clientRects, pageRect) {
    if (!clientRects || clientRects.length === 0) {
        return [];
    }

    const RECT_HEIGHT_TOLERANCE = 10; // pixels, for grouping rects into lines

    let rects = [];
    for (let i = 0; i < clientRects.length; i++) {
        const r = clientRects[i];
        if (r.width === 0 || r.height === 0) continue;

        const x1 = r.left - pageRect.left;
        const y1 = r.top - pageRect.top;
        const x2 = r.right - pageRect.left;
        const y2 = r.bottom - pageRect.top;
        rects.push({ x1, y1, x2, y2, midY: (y1 + y2) / 2 });
    }

    // Sort by y1 then x1
    rects.sort((a, b) => {
        if (a.y1 !== b.y1) {
            return a.y1 - b.y1;
        }
        return a.x1 - b.x1;
    });

    if (rects.length === 0) return [];

    const lines = [];
    let currentLine = []; // Start with an empty currentLine

    if (rects.length > 0) {
        currentLine.push(rects[0]); // Add the first rect to start the first line
        for (let i = 1; i < rects.length; i++) {
            const currentRect = rects[i];
            const firstRectOfCurrentLine = currentLine[0]; // Get the first rect of the line being built

            // Calculate an approximate line height based on the first rect of the current line.
            // This assumes rects on the same line have similar heights.
            const approxLineHeight = (firstRectOfCurrentLine.y2 - firstRectOfCurrentLine.y1);

            // Condition to start a new line:
            // If the top of the currentRect is significantly below the top of the firstRectOfCurrentLine.
            // "Significantly below" means currentRect.y1 is greater than firstRectOfCurrentLine.y1 by
            // more than a fraction (e.g., 0.7) of the approxLineHeight.
            if (currentRect.y1 > firstRectOfCurrentLine.y1 + approxLineHeight * 0.7) {
                lines.push(currentLine);      // Finalize the previous line
                currentLine = [currentRect];  // Start a new line with currentRect
            } else {
                // Otherwise, currentRect is considered part of the current line
                currentLine.push(currentRect);
            }
        }
        lines.push(currentLine); // Add the last processed line
    }

    const finalQuadPoints = [];
    for (const line of lines) {
        if (line.length === 0) continue;

        // Sort rects in this line by x1 (should mostly be sorted already)
        line.sort((a, b) => a.x1 - b.x1);

        let mergedRectsOnLine = [];
        if (line.length > 0) {
            mergedRectsOnLine.push({ ...line[0] }); // Start with the first rect

            for (let i = 1; i < line.length; i++) {
                const currentRect = line[i];
                let lastMerged = mergedRectsOnLine[mergedRectsOnLine.length - 1];

                // Check for horizontal overlap or adjacency (within a small tolerance if needed)
                // For now, direct overlap: if currentRect.x1 is less than lastMerged.x2
                if (currentRect.x1 < lastMerged.x2 + 5) { // 5px tolerance for adjacency
                    lastMerged.x2 = Math.max(lastMerged.x2, currentRect.x2);
                    lastMerged.y1 = Math.min(lastMerged.y1, currentRect.y1);
                    lastMerged.y2 = Math.max(lastMerged.y2, currentRect.y2);
                } else {
                    mergedRectsOnLine.push({ ...currentRect });
                }
            }
        }

        for (const mergedRect of mergedRectsOnLine) {
            finalQuadPoints.push([
                mergedRect.x1, mergedRect.y1,
                mergedRect.x2, mergedRect.y1,
                mergedRect.x1, mergedRect.y2,
                mergedRect.x2, mergedRect.y2
            ]);
        }
    }
    return finalQuadPoints;
}

function areQuadsSubsumed(newSelectionQuads, existingHighlightQuads) {
    if (!existingHighlightQuads || existingHighlightQuads.length === 0) return false;
    if (!newSelectionQuads || newSelectionQuads.length === 0) return false;

    for (const existingQuad of existingHighlightQuads) {
        // Convert existingQuad from [x1,y1, x2,y1, x1,y2, x2,y2] to {x1,y1,x2,y2}
        const exRect = { x1: existingQuad[0], y1: existingQuad[1], x2: existingQuad[6], y2: existingQuad[7] };
        let isThisExistingQuadCovered = false;
        for (const newSelectionQuad of newSelectionQuads) {
            // Convert newSelectionQuad to {nx1,ny1,nx2,ny2}
            const nsRect = { nx1: newSelectionQuad[0], ny1: newSelectionQuad[1], nx2: newSelectionQuad[6], ny2: newSelectionQuad[7] };
            // Check if nsRect completely contains exRect
            if (nsRect.nx1 <= exRect.x1 && nsRect.ny1 <= exRect.y1 && nsRect.nx2 >= exRect.x2 && nsRect.ny2 >= exRect.y2) {
                isThisExistingQuadCovered = true;
                break; // This existingQuad is covered by at least one newSelectionQuad
            }
        }
        if (!isThisExistingQuadCovered) {
            return false; // Found an existingQuad that is not covered by any newSelectionQuad
        }
    }
    return true; // All existingQuads are covered
}

function getBoundingBoxForQuads(quadsArray) {
    if (!quadsArray || quadsArray.length === 0) return null;
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const quad of quadsArray) {
        // For an axis-aligned rect represented as 8-point quad:
        // quad[0]=x1_tl, quad[1]=y1_tl, quad[2]=x2_tr, quad[3]=y1_tr (y1_tl)
        // quad[4]=x1_bl, quad[5]=y2_bl, quad[6]=x2_br, quad[7]=y2_br (y2_bl)
        // Simplified: minX = quad[0], minY = quad[1], maxX = quad[2], maxY = quad[5]
        // To be robust for any quad (even if not perfectly axis-aligned from some sources):
        minX = Math.min(minX, quad[0], quad[2], quad[4], quad[6]);
        minY = Math.min(minY, quad[1], quad[3], quad[5], quad[7]);
        maxX = Math.max(maxX, quad[0], quad[2], quad[4], quad[6]);
        maxY = Math.max(maxY, quad[1], quad[3], quad[5], quad[7]);
    }
    if (minX === Infinity) return null; // Should not happen if quadsArray is not empty
    return { x1: minX, y1: minY, x2: maxX, y2: maxY };
}

function doBoundingBoxesIntersect(boxA, boxB) {
    if (!boxA || !boxB) return false;
    // Check for non-intersection. If any of these is true, they don't intersect.
    return !(boxA.x1 >= boxB.x2 || // boxA is to the right of boxB
             boxA.x2 <= boxB.x1 || // boxA is to the left of boxB
             boxA.y1 >= boxB.y2 || // boxA is below boxB
             boxA.y2 <= boxB.y1);  // boxA is above boxB
}

// --- Quad/Rect Subtraction Helpers ---
function _subtractSingleRect(rectA, rectB) {
    // rectA: the rectangle to subtract from
    // rectB: the rectangle being subtracted
    const resultRects = [];
    const TOLERANCE = 0.01; // For floating point comparisons

    // Check for non-intersection
    if (rectA.x1 >= rectB.x2 - TOLERANCE || rectA.x2 <= rectB.x1 + TOLERANCE || rectA.y1 >= rectB.y2 - TOLERANCE || rectA.y2 <= rectB.y1 + TOLERANCE) {
        return [rectA]; // No intersection, A remains whole
    }

    const ix1 = Math.max(rectA.x1, rectB.x1);
    const ix2 = Math.min(rectA.x2, rectB.x2);
    const iy1 = Math.max(rectA.y1, rectB.y1);
    const iy2 = Math.min(rectA.y2, rectB.y2);

    // Top part of rectA
    if (rectA.y1 < iy1 - TOLERANCE) {
        resultRects.push({ x1: rectA.x1, y1: rectA.y1, x2: rectA.x2, y2: iy1 });
    }
    // Bottom part of rectA
    if (rectA.y2 > iy2 + TOLERANCE) {
        resultRects.push({ x1: rectA.x1, y1: iy2, x2: rectA.x2, y2: rectA.y2 });
    }
    // Left part of rectA (within the vertical span of the intersection)
    if (rectA.x1 < ix1 - TOLERANCE) {
        resultRects.push({ x1: rectA.x1, y1: iy1, x2: ix1, y2: iy2 });
    }
    // Right part of rectA (within the vertical span of the intersection)
    if (rectA.x2 > ix2 + TOLERANCE) {
        resultRects.push({ x1: ix2, y1: iy1, x2: rectA.x2, y2: iy2 });
    }

    return resultRects.filter(r => r.x2 - r.x1 > TOLERANCE && r.y2 - r.y1 > TOLERANCE);
}

function quadToRect(quad) {
    // quad is [x_tl, y_tl, x_tr, y_tr, x_bl, y_bl, x_br, y_br]
    // For an axis-aligned rect: x_tr = x_br, y_tr = y_tl, x_bl = x_tl, y_bl = y_br
    // Rect: {x1, y1, x2, y2} where (x1,y1) is TL and (x2,y2) is BR
    return { x1: quad[0], y1: quad[1], x2: quad[2], y2: quad[5] };
}

function rectToQuad(rect) {
    return [rect.x1, rect.y1, rect.x2, rect.y1, rect.x1, rect.y2, rect.x2, rect.y2];
}

function _simplifyAndMergeRects(rectsToSimplify) {
    if (!rectsToSimplify || rectsToSimplify.length === 0) return [];

    let rects = [...rectsToSimplify.filter(r => r.x2 - r.x1 > 0.01 && r.y2 - r.y1 > 0.01)];

    rects.sort((a, b) => {
        if (a.y1 !== b.y1) return a.y1 - b.y1;
        return a.x1 - b.x1;
    });

    if (rects.length === 0) return [];

    const lines = [];
    let currentLine = [];

    if (rects.length > 0) {
        currentLine.push(rects[0]);
        for (let i = 1; i < rects.length; i++) {
            const currentRect = rects[i];
            const firstRectOfCurrentLine = currentLine[0];
            const approxLineHeight = (firstRectOfCurrentLine.y2 - firstRectOfCurrentLine.y1);

            if (currentRect.y1 > firstRectOfCurrentLine.y1 + approxLineHeight * 0.7) {
                lines.push(currentLine);
                currentLine = [currentRect];
            } else {
                currentLine.push(currentRect);
            }
        }
        lines.push(currentLine);
    }

    const mergedLines = [];
    for (const line of lines) {
        if (line.length === 0) continue;
        line.sort((a, b) => a.x1 - b.x1);
        let mergedRectsOnLine = [];
        if (line.length > 0) {
            mergedRectsOnLine.push({ ...line[0] });
            for (let i = 1; i < line.length; i++) {
                const currentRect = line[i];
                let lastMerged = mergedRectsOnLine[mergedRectsOnLine.length - 1];
                if (currentRect.x1 < lastMerged.x2 + 5) { // 5px tolerance for adjacency
                    lastMerged.x2 = Math.max(lastMerged.x2, currentRect.x2);
                    lastMerged.y1 = Math.min(lastMerged.y1, currentRect.y1); // Take min y1 for line
                    lastMerged.y2 = Math.max(lastMerged.y2, currentRect.y2); // Take max y2 for line
                } else {
                    mergedRectsOnLine.push({ ...currentRect });
                }
            }
        }
        mergedLines.push(...mergedRectsOnLine);
    }
    return mergedLines;
}

function subtractQuads(originalQuads, subtractingQuads) {
    if (!originalQuads || originalQuads.length === 0) return [];
    if (!subtractingQuads || subtractingQuads.length === 0) return [...originalQuads];

    let currentRects = originalQuads.map(quadToRect);
    const subtractingRects = subtractingQuads.map(quadToRect);

    for (const subRect of subtractingRects) {
        let nextResultRects = [];
        for (const currentRect of currentRects) {
            const diffRects = _subtractSingleRect(currentRect, subRect);
            nextResultRects.push(...diffRects);
        }
        currentRects = nextResultRects;
    }

    const finalCleanedRects = _simplifyAndMergeRects(currentRects);
    return finalCleanedRects.map(rectToQuad);
}

function groupDisjointBlocks(quads) {
    if (!quads || quads.length === 0) {
        return [];
    }

    // Ensure quads are sorted: primarily by y1 (quad[1]), secondarily by x1 (quad[0])
    const sortedQuads = [...quads].sort((a, b) => {
        if (a[1] !== b[1]) return a[1] - b[1]; // y1
        return a[0] - b[0]; // x1
    });

    const allBlocks = [];
    let currentBlock = [];

    for (const currentStripQuad of sortedQuads) {
        if (currentBlock.length === 0) {
            currentBlock.push(currentStripQuad);
        } else {
            const lastStripInBlock = currentBlock[currentBlock.length - 1];

            // y_tl is quad[1], y_bl is quad[5]
            const avgHeightOfLastStrip = Math.max(5, lastStripInBlock[5] - lastStripInBlock[1]);

            // currentStripQuad: x1=quad[0], y1=quad[1], x2=quad[2] (TRx), y2=quad[5] (BLy)
            // lastStripInBlock: x1=quad[0], y1=quad[1], x2=quad[2] (TRx), y2=quad[5] (BLy)

            const currentTop = currentStripQuad[1];
            const lastTop = lastStripInBlock[1];

            const areOnSameLine = Math.abs(currentTop - lastTop) < avgHeightOfLastStrip * 0.7;
            const areOnAdjacentLine = Math.abs(currentTop - lastTop) < avgHeightOfLastStrip * 2.5;

            const currentLeft = currentStripQuad[0];
            const currentRight = currentStripQuad[2]; // TRx
            const lastLeft = lastStripInBlock[0];
            const lastRight = lastStripInBlock[2]; // TRx

            const horizontallyOverlap = Math.max(currentLeft, lastLeft) < Math.min(currentRight, lastRight);
            const horizontallyAdjacentOrClose = currentLeft < lastRight + 10 && currentRight > lastLeft - 10;

            if ((areOnSameLine && horizontallyAdjacentOrClose) || (areOnAdjacentLine && horizontallyAdjacentOrClose)) {
                currentBlock.push(currentStripQuad);
            } else {
                allBlocks.push(currentBlock);
                currentBlock = [currentStripQuad];
            }
        }
    }

    if (currentBlock.length > 0) {
        allBlocks.push(currentBlock);
    }

    return allBlocks;
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

    const pageHighlightsFromStore = initialHighlights.filter(hl => hl.pageIndex === pageIndex);

    // <<< NEW CLEANUP LOGIC >>>
    const storeHighlightIdsOnPage = new Set(pageHighlightsFromStore.map(hl => hl.id));
    const overlayContainer = ensureHighlightOverlayContainer(pageIndex);
    if (overlayContainer) {
        const existingDomHighlightParts = overlayContainer.querySelectorAll('.overlay-part[data-hl-id]');
        const domHighlightIdsOnPage = new Set();
        existingDomHighlightParts.forEach(part => {
            if (part.dataset.hlId) { // Ensure hlId exists
                domHighlightIdsOnPage.add(part.dataset.hlId);
            }
        });

        for (const domId of domHighlightIdsOnPage) {
            if (!storeHighlightIdsOnPage.has(domId)) {
                // This ID is in the DOM but no longer in the store for this page
                console.log(`[renderAnnotationsForPage] Clean-up: Removing stale DOM highlight ${domId} from page ${pageIndex + 1}`);
                removeHighlightOverlay(domId); // removeHighlightOverlay is an existing function
            }
        }
    }
    // <<< END OF NEW CLEANUP LOGIC >>>

    if (!pageHighlightsFromStore.length) { // Check after cleanup, using the filtered list
        // console.log(`[renderAnnotationsForPage] No highlights to render for page ${pageIndex + 1}.`);
        return;
    }

    // console.debug(`[renderAnnotationsForPage] Page ${pageIndex + 1}. Rendering ${pageHighlightsFromStore.length} highlights.`);

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


    for (const hl of pageHighlightsFromStore) {
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

    quadPoints.forEach((quad, index) => {
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

    $: currentZoomLabel = (() => {
        const scaleStr = String(currentScaleValue);
        const foundOption = zoomOptions.find(opt => opt.value === scaleStr);
        if (foundOption && foundOption.type !== 'separator') {
            return foundOption.label;
        }
        // Handle numeric scale values not exactly in presets (e.g. after pinch zoom)
        const numScale = parseFloat(scaleStr);
        if (!isNaN(numScale)) {
            // Check if it's very close to a preset percentage
            const presetMatch = zoomOptions.find(opt => opt.value && !isNaN(parseFloat(opt.value)) && Math.abs(parseFloat(opt.value) - numScale) < 0.001);
            if (presetMatch && presetMatch.type !== 'separator') return presetMatch.label;
            return `${Math.round(numScale * 100)}%`;
        }
        return 'Auto'; // Default fallback
    })();

</script>

<div class="pdf-viewer-panel-root prose prose-sm dark:prose-invert max-w-none flex flex-col h-full w-full bg-gray-100 dark:bg-dark-bg-form-field shadow overflow-hidden">

<div class="toolbar relative flex items-center flex-wrap gap-x-1 border-b border-gray-300 dark:border-border p-1 flex-shrink-0 bg-gray-50 dark:bg-surface-3 z-20">
    <button class="mini-toolbar-button" on:click={goToPrevPage} title="Previous page" disabled={currentPageNum <= 1 || loading || !pdfDoc}><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4"><path fill-rule="evenodd" d="M12.79 5.23a.75.75 0 0 1-.02 1.06L8.832 10l3.938 3.71a.75.75 0 1 1-1.04 1.08l-4.5-4.25a.75.75 0 0 1 0-1.08l4.5-4.25a.75.75 0 0 1 1.06.02Z" clip-rule="evenodd" /></svg></button>
    <input type="number" class="mini-toolbar-input w-12 text-center mx-0.5" bind:value={pageNumInput} min="1" max={numPages || 1} aria-label="Current page number" on:change={handlePageInputChange} on:blur={handlePageInputBlur} disabled={loading || !pdfDoc} />
    <span class="text-xs px-1 text-gray-600 dark:text-gray-400"> of {numPages || '?'} </span>
    <button class="mini-toolbar-button" on:click={goToNextPage} title="Next page" disabled={currentPageNum >= numPages || loading || !pdfDoc}><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4"><path fill-rule="evenodd" d="M7.21 14.77a.75.75 0 0 1 .02-1.06L11.168 10 7.23 6.29a.75.75 0 1 1 1.04-1.08l4.5 4.25a.75.75 0 0 1 0 1.08l-4.5 4.25a.75.75 0 0 1-1.06-.02Z" clip-rule="evenodd" /></svg></button>
    <div class="separator"></div>
    <button class="mini-toolbar-button" on:click={zoomOut} title="Zoom out" disabled={loading || !pdfDoc}><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4"><path fill-rule="evenodd" d="M4 10a.75.75 0 0 1 .75-.75h10.5a.75.75 0 0 1 0 1.5H4.75A.75.75 0 0 1 4 10Z" clip-rule="evenodd" /></svg></button>

    <!-- Custom Zoom Dropdown -->
    <div class="relative" bind:this={zoomDropdownRef}>
        <button
            class="mini-toolbar-button flex items-center gap-1 w-24 justify-between"
            on:click={toggleZoomDropdown}
            title="Zoom Level"
            disabled={loading || !pdfDoc}
            aria-haspopup="true"
            aria-expanded={isZoomDropdownOpen}
        >
            <span class="truncate text-xs">{@html currentZoomLabel}</span>
            <svg class="ml-0.5 h-3 w-3 flex-shrink-0" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor">
                <path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 10.94l3.71-3.71a.75.75 0 011.08 1.04l-4.25 4.25a.75.75 0 01-1.08 0L5.21 8.27a.75.75 0 01.02-1.06z" clip-rule="evenodd" />
            </svg>
        </button>
        {#if isZoomDropdownOpen}
            <div class="absolute top-full mt-1 left-0 z-30 w-48 bg-white dark:bg-gray-800 border border-gray-300 dark:border-border shadow-lg overflow-y-auto max-h-60 py-1">
                {#each zoomOptions as option (option.value || option.label)}
                    {#if option.type === 'separator'}
                        <div class="my-1 border-t border-gray-200 dark:border-border"></div>
                    {:else}
                        <div
                            class="px-3 py-1.5 text-xs flex justify-between items-center cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-700 focus:bg-gray-100 dark:focus:bg-gray-700 focus:outline-none"
                            class:font-semibold={String(currentScaleValue) === option.value}
                            class:bg-gray-200={String(currentScaleValue) === option.value}
                            class:dark:bg-gray-600={String(currentScaleValue) === option.value}
                            on:click={() => selectZoomLevel(option.value)}
                            role="menuitemradio"
                            aria-checked={String(currentScaleValue) === option.value}
                            tabindex="0"
                            on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { selectZoomLevel(option.value); e.preventDefault(); toggleZoomDropdown(); } else if (e.key === 'Escape') { toggleZoomDropdown(); e.target.closest('button').focus(); } }}
                        >
                            <span>{option.label}</span>
                            {#if String(currentScaleValue) === option.value}
                                <svg class="h-4 w-4 text-blue-600 dark:text-blue-400" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M16.704 4.153a.75.75 0 01.143 1.052l-8 10.5a.75.75 0 01-1.127.075l-4.5-4.5a.75.75 0 011.06-1.06l3.894 3.893 7.48-9.817a.75.75 0 011.05-.143z" clip-rule="evenodd" /></svg>
                            {/if}
                        </div>
                    {/if}
                {/each}
            </div>
        {/if}
    </div>

    <button class="mini-toolbar-button" on:click={zoomIn} title="Zoom in" disabled={loading || !pdfDoc}><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4"><path d="M10.75 4.75a.75.75 0 0 0-1.5 0v4.5h-4.5a.75.75 0 0 0 0 1.5h4.5v4.5a.75.75 0 0 0 1.5 0v-4.5h4.5a.75.75 0 0 0 0-1.5h-4.5v-4.5Z" /></svg></button>
    <div class="separator"></div>
    <div class="flex items-center space-x-0.5">
        <input type="search" class="mini-toolbar-input w-28 text-xs" placeholder="Search" bind:value={searchQuery} on:keydown={handleSearchKeydown} disabled={loading || !pdfDoc} aria-label="Search text" autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" />
        <button class="mini-toolbar-button" title="Previous match" on:click={() => runSearch({ findPrevious: true })} disabled={loading || !pdfDoc || !searchQuery}><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-3 h-3"><path fill-rule="evenodd" d="M12.79 5.23a.75.75 0 0 1-.02 1.06L8.832 10l3.938 3.71a.75.75 0 1 1-1.04 1.08l-4.5-4.25a.75.75 0 0 1 0-1.08l4.5-4.25a.75.75 0 0 1 1.06.02Z" clip-rule="evenodd" /></svg></button>
        <button class="mini-toolbar-button" title="Next match" on:click={() => runSearch({ findPrevious: false })} disabled={loading || !pdfDoc || !searchQuery}><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-3 h-3"><path fill-rule="evenodd" d="M7.21 14.77a.75.75 0 0 1 .02-1.06L11.168 10 7.23 6.29a.75.75 0 1 1 1.04-1.08l4.5 4.25a.75.75 0 0 1 0 1.08l-4.5 4.25a.75.75 0 0 1-1.06-.02Z" clip-rule="evenodd" /></svg></button>
    </div>
    <div class="separator"></div>
    <!-- New Quick Highlight UI -->
    <div class="quick-highlight-group inline-flex items-center" role="group">
        <button
            class="mini-toolbar-button rounded-e-none border-r border-gray-300 dark:border-r-gray-600"
            class:active={isQuickHighlightActive}
            title="Quick Highlight - {isQuickHighlightActive ? 'On' : 'Off'}"
            on:click={() => {
                isQuickHighlightActive = !isQuickHighlightActive;
                if (isQuickHighlightActive) {
                    // If no specific color has been chosen via dropdown (quickHighlightColor is still default/yellow or was white from 'None')
                    // OR if the mode was 'remove' (meaning 'None' was selected)
                    // then default to yellow highlight.
                    if (quickHighlightColor === 'rgba(255, 242, 117, 0.5)' || quickHighlightColor === 'rgba(255, 255, 255, 1)' || quickHighlightMode === 'remove') {
                        quickHighlightMode = 'highlight';
                        quickHighlightColor = 'rgba(255, 242, 117, 0.5)'; // Default to yellow
                    }
                    // Otherwise, if a color was previously selected (e.g., Green) and mode is 'highlight',
                    // it will retain that color and mode.
                }
            }}
            style="{isQuickHighlightActive ? (quickHighlightMode === 'highlight' ? `background-color: ${quickHighlightColor};` : `background-color: rgba(255, 255, 255, 1);`) : ''}"
        >
            {#if isQuickHighlightActive}
                {#if quickHighlightMode === 'remove'}
                    {@html removeHighlightIconSVG}
                {:else}
                    {@html markerIconSVG}
                {/if}
            {:else}
                {@html markerIconSVG}
            {/if}
        </button>
        <div class="relative flex items-center" bind:this={newHighlightDropdownRef}>
            <button class="mini-toolbar-button rounded-s-none border-l-0" title="Select Quick Highlight Color or Mode" on:click={toggleNewHighlightDropdown} disabled={loading || !pdfDoc}>
                <svg class="h-4 w-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 10.94l3.71-3.71a.75.75 0 011.08 1.04l-4.25 4.25a.75.75 0 01-1.08 0L5.21 8.27a.75.75 0 01.02-1.06z" clip-rule="evenodd"/></svg>
            </button>
            {#if isNewHighlightDropdownOpen}
            <div class="absolute top-full mt-1 right-0 z-30 w-40 bg-white dark:bg-gray-800 border border-gray-300 dark:border-border shadow-lg overflow-hidden py-1">
                {#each highlightOptions.filter(opt => opt.label !== 'None') as opt}
                    <div
                        class="px-2 py-1 flex items-center gap-2 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-700"
                        role="menuitem"
                        tabindex="-1"
                        on:click={() => {
                            quickHighlightColor = opt.value;
                            quickHighlightMode = 'highlight';
                            isNewHighlightDropdownOpen = false;
                            isQuickHighlightActive = true; // Enable toggle
                        }}
                        on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { quickHighlightColor = opt.value; quickHighlightMode = 'highlight'; isNewHighlightDropdownOpen = false; isQuickHighlightActive = true; e.preventDefault();}}}
                    >
                        <span class="w-4 h-4 rounded-full border border-gray-400 dark:border-gray-500" style:background-color={opt.value}></span>
                        <span>{opt.label}</span>
                    </div>
                {/each}
                <div
                    class="px-2 py-1 flex items-center gap-2 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-700"
                    role="menuitem"
                    tabindex="-1"
                    on:click={() => {
                        quickHighlightColor = 'rgba(255, 255, 255, 1)'; // White
                        quickHighlightMode = 'remove'; // Set mode to remove
                        isNewHighlightDropdownOpen = false;
                        isQuickHighlightActive = true; // Enable toggle
                    }}
                    on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { quickHighlightColor = 'rgba(255, 255, 255, 1)'; quickHighlightMode = 'remove'; isNewHighlightDropdownOpen = false; isQuickHighlightActive = true; e.preventDefault();}}}
                >
                    <span class="w-4 h-4 rounded-full border border-gray-400 dark:border-gray-500" style:background-color={'rgba(255, 255, 255, 1)'}></span>
                    <span>None</span>
                </div>
            </div>
            {/if}
        </div>
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

<div bind:this={pdfViewerWrapperElement} class="flex-grow overflow-hidden bg-gray-200 dark:bg-dark-bg-primary relative pdf-viewer-wrapper">
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
        <div class="floating-toolbar absolute bg-white dark:bg-gray-800 border border-gray-400 dark:border-gray-500 shadow-lg px-1 py-0.5 flex items-center space-x-0.5 transition-opacity duration-100"
            bind:this={selectionToolbarElement}
            style:top="{selectionToolbarTop}px"
            style:left="{selectionToolbarLeft}px"
            style:display="flex"
            style:opacity="{showSelectionToolbar ? '1' : '0'}"
            style:visibility="{showSelectionToolbar ? 'visible' : 'hidden'}"
            on:mouseenter={handleToolbarMouseEnter}
            on:mouseleave={handleToolbarMouseLeave} >

            {#each highlightOptions.filter(opt => opt.label !== 'None') as opt}
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

    /* --- Base for all toolbar items --- */
    .toolbar button.mini-toolbar-button,
    .toolbar input.mini-toolbar-input {
        @apply p-1.5 rounded inline-flex items-center justify-center
                focus:outline-none focus:ring-1 focus:ring-offset-1 focus:ring-blue-500
                dark:focus:ring-offset-surface-3 transition duration-150 ease-in-out
                text-xs disabled:opacity-50 disabled:cursor-not-allowed;
        line-height: 1.2;
        min-height: 24px;
        margin-right: 2px;
    }

    /* --- Button Styles --- */
    .toolbar button.mini-toolbar-button {
        color: var(--ui-icon-color);
        border: 1px solid transparent;
        background-color: transparent;
    }
    .toolbar button.mini-toolbar-button:hover:not(:disabled) {
        background-color: var(--ui-icon-hover-bg);
        border-color: var(--ui-select-border);
    }
    html.dark .toolbar button.mini-toolbar-button {
        color: var(--color-text-primary);
    }
    html.dark .toolbar button.mini-toolbar-button:hover:not(:disabled) {
        background-color: var(--color-border);
        border-color: var(--color-border);
    }

    /* --- Input Styles --- */
    .toolbar input.mini-toolbar-input {
        color: var(--ui-text-color);
        background-color: var(--ui-select-bg);
        border: 1px solid var(--ui-select-border);
    }
    html.dark .toolbar input.mini-toolbar-input {
        color: var(--color-text-primary);
        background-color: var(--color-surface-2);
        border-color: var(--color-border);
    }

    /* --- Active State for Buttons --- */
    .toolbar button.mini-toolbar-button.active {
      @apply bg-blue-200 dark:bg-blue-800;
    }

    .separator {
        @apply w-px h-5 bg-gray-300 dark:bg-gray-600 mx-1;
    }

    .mini-toolbar-input[type=number]::-webkit-inner-spin-button, .mini-toolbar-input[type=number]::-webkit-outer-spin-button { -webkit-appearance: none; margin: 0; }
    .mini-toolbar-input[type=number] { -moz-appearance: textfield; }
    .pdf-viewer-wrapper { position: relative; flex-grow: 1; overflow: hidden; }
    .pdfViewerContainer { @apply p-4 bg-gray-300 dark:bg-dark-bg-form-field; height: 100%; width: 100%; overflow: auto; position: absolute; inset: 0; -webkit-overflow-scrolling: touch; }

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
}

    .floating-toolbar { align-items: center; gap: 2px; padding: 2px 4px; z-index: 50; }
    .floating-toolbar-button { @apply p-1 border border-transparent hover:bg-gray-200 dark:hover:bg-gray-600 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-transparent; line-height: 1; cursor: pointer; }
    .floating-toolbar .separator { @apply w-px h-4 bg-gray-300 dark:bg-gray-500 mx-1; }

    .w-3 { width: 0.75rem; } .h-3 { height: 0.75rem; } .w-4 { width: 1rem; } .h-4 { height: 1rem; } .w-12 { width: 3rem; } .w-24 { width: 6rem; } .w-28 { width: 7rem; }
</style>