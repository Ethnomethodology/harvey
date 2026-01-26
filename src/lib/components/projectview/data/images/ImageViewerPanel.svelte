<script>
    import { onMount, onDestroy, tick } from 'svelte';
    import { convertFileSrc } from '@tauri-apps/api/core';
    import { sep } from '@tauri-apps/api/path';
    import { get, derived } from 'svelte/store';
    import { project, updateImageAnnotations } from '$lib/stores/projectStore.js';
    import { saveImageAnnotations } from '$lib/services/projectService.js';
    import OpenSeadragon from 'openseadragon';
    import { v4 as uuidv4 } from 'uuid';

    export let imagePath = '';

    let osdViewerElement;
    let osdViewer = null;

    let isLoading = true;
    let error = null;
    let currentLoadedPath = null;

    import AnnotationCreationDialog from '$lib/components/modals/AnnotationCreationDialog.svelte';

    let showAnnotationCreationDialog = false;
    let dialogX = 0;
    let dialogY = 0;
    let annotationBeingEdited = null; // Stores the annotation data when editing
    let isEditingExisting = false; // Flag to indicate if we are editing or creating

    // Get annotations from the central store
    const currentAnnotations = derived(project, $project => {
        if ($project.selectedDocumentPath === imagePath && $project.selectedDocumentType === 'images') {
            return $project.currentImageAnnotations || [];
        }
        return [];
    });

    // State for drawing mode: 'rectangle', 'circle', 'polygon', or null
    let activeDrawingTool = null;

    // Variables for drawing
    let isDrawing = false;
    let startPoint = null;
    let currentRect = null; // { x, y, width, height } for rectangle
    let currentCircle = null; // { cx, cy, r } for circle
    let currentPolygon = { points: [], previewLine: null, closingPreviewLine: null }; // For polygon drawing
    let currentPreviewPolygonPoints = []; // For filled polygon preview
    let svgOverlay; // Reference to the SVG element

    function adjustOpacity(rgbaColor, newOpacity) {
        if (!rgbaColor || typeof rgbaColor !== 'string' || !rgbaColor.startsWith('rgba(')) { return rgbaColor; }
        const parts = rgbaColor.substring(5, rgbaColor.length - 1).split(',');
        if (parts.length !== 4) return rgbaColor;
        return `rgba(${parts[0].trim()}, ${parts[1].trim()}, ${parts[2].trim()}, ${newOpacity})`;
    }

    async function initializeViewer(pathForImage) {
        console.log(`[ImageViewerPanel initializeViewer] Attempting for path: ${pathForImage}`);
        if (!pathForImage || !osdViewerElement) {
            console.warn('[ImageViewerPanel initializeViewer] Skipping: no path or osdViewerElement.', { pathForImage, osdViewerElementExists: !!osdViewerElement });
            isLoading = false; error = 'Viewer element not ready or path missing.'; return;
        }

        console.log(`[ImageViewerPanel initializeViewer] Proceeding with initialization for: ${pathForImage}`);
        currentLoadedPath = pathForImage; isLoading = true; error = null;

        if (osdViewer) { try { osdViewer.destroy(); console.log("Previous OpenSeadragon instance destroyed."); } catch (e) { console.warn("Error destroying previous OpenSeadragon instance:", e); } osdViewer = null; }
        if(osdViewerElement) osdViewerElement.innerHTML = '';

        try {
            const assetUrl = convertFileSrc(pathForImage);
            console.log(`[ImageViewerPanel] Converted asset URL: ${assetUrl}`);
            await tick();
            if (!osdViewerElement) {
                error = 'OSD container element became null during async operations.';
                console.error(`[ImageViewerPanel initializeViewer] ${error}`);
                isLoading = false; currentLoadedPath = null; return;
            }

            osdViewer = OpenSeadragon({
                element: osdViewerElement,
                prefixUrl: '/openseadragon-icons/',
                crossOriginPolicy: 'Anonymous',
                loadTilesWithAjax: true,
                ajaxWithCredentials: false,
                tileSources: { type: 'image', url: assetUrl },
                animationTime: 0.5, blendTime: 0.1, constrainDuringPan: true,
                maxZoomPixelRatio: 2, minZoomImageRatio: 0.8, visibilityRatio: 1,
                zoomPerScroll: 1.2,
                gestureSettingsMouse: { scrollToZoom: true, clickToZoom: false, dblClickToZoom: false, pinchToZoom: true },
                showNavigator: false,
                showNavigationControl: false,
            });

            osdViewer.addHandler('open', async () => {
                console.log('[ImageViewerPanel OSD Event] "open" event triggered.');
                isLoading = false;
                await tick(); // Ensure SVG element is rendered before adding as overlay
                // Add SVG overlay after OSD viewer is open
                if (svgOverlay) {
                    osdViewer.addOverlay(svgOverlay, new OpenSeadragon.Rect(0, 0, 1, 1)); // Cover entire image viewport
                    svgOverlay.style.pointerEvents = 'none'; // Allow OSD events to pass through
                }
                setupDrawingEvents();
                // We no longer call loadAnnotationsForImage here, as it's handled by the store
                console.log('[ImageViewerPanel] OpenSeadragon setup complete.');
            });

            osdViewer.addHandler('open-failed', (event) => {
                console.error('[ImageViewerPanel OSD Event] "open-failed" event triggered:', event);
                let detail = event.message || 'Unknown OSD error';
                if (event.source && event.source.url) detail += ` (URL: ${event.source.url})`;
                error = `OpenSeadragon failed to open image: ${detail}`;
                isLoading = false;
            });
        } catch (err) {
            console.error(`[ImageViewerPanel initializeViewer] Top-level error for path ${pathForImage}:`, err);
            error = `Failed to load image viewer: ${err.message || err}`;
            currentLoadedPath = null; isLoading = false;
        }
    }

    function setupDrawingEvents() {
        if (!osdViewer) { console.warn("setupDrawingEvents: OpenSeadragon viewer not available."); return; }
        console.log("[ImageViewerPanel] Setting up drawing events.");

        osdViewer.addHandler('canvas-press', onMouseDown);
        osdViewer.addHandler('canvas-drag', onMouseMove);
        osdViewer.addHandler('canvas-release', onMouseUp);
        osdViewer.addHandler('canvas-double-click', onDoubleClick);
        osdViewer.addHandler('update-viewport', updateAnnotationPositions);
    }

    let startViewportPoint = null; // Stores the starting viewport point for drawing

    function onMouseDown(event) {
        if (!activeDrawingTool) return;
        if (event.originalEvent.button !== 0) return; // Only left click

        const viewportPoint = osdViewer.viewport.pointFromPixel(event.position);

        if (activeDrawingTool === 'polygon') {
            if (!isDrawing) {
                isDrawing = true;
                currentPolygon.points = [viewportPoint];
            } else {
                currentPolygon.points = [...currentPolygon.points, viewportPoint];
            }
        } else {
            isDrawing = true;
            startViewportPoint = viewportPoint;
            if (activeDrawingTool === 'rectangle') {
                currentRect = { x: startViewportPoint.x, y: startViewportPoint.y, width: 0, height: 0 };
            } else if (activeDrawingTool === 'circle') {
                currentCircle = { cx: startViewportPoint.x, cy: startViewportPoint.y, r: 0 };
            }
        }
        event.preventDefaultAction = true;
    }

        function drawPreviewLines(currentPoint) {
        if (currentPolygon.points.length > 0) {
            const lastPoint = currentPolygon.points[currentPolygon.points.length - 1];
            const previewLine = { x1: lastPoint.x, y1: lastPoint.y, x2: currentPoint.x, y2: currentPoint.y };
            let closingPreviewLine = null;
            if (currentPolygon.points.length > 1) {
                const firstPoint = currentPolygon.points[0];
                closingPreviewLine = { x1: currentPoint.x, y1: currentPoint.y, x2: firstPoint.x, y2: firstPoint.y };
            }
            currentPolygon = { ...currentPolygon, previewLine, closingPreviewLine };
        }
    }

    function onMouseMove(event) {
        if (!isDrawing || !activeDrawingTool) return;

        const currentViewportPoint = osdViewer.viewport.pointFromPixel(event.position);

        if (activeDrawingTool === 'rectangle') {
            const x = Math.min(startViewportPoint.x, currentViewportPoint.x);
            const y = Math.min(startViewportPoint.y, currentViewportPoint.y);
            const width = Math.abs(startViewportPoint.x - currentViewportPoint.x);
            const height = Math.abs(startViewportPoint.y - currentViewportPoint.y);
            currentRect = { x, y, width, height };
        } else if (activeDrawingTool === 'circle') {
            const dx = currentViewportPoint.x - startViewportPoint.x;
            const dy = currentViewportPoint.y - startViewportPoint.y;
            const r = Math.sqrt(dx * dx + dy * dy) / 2;
            const cx = startViewportPoint.x + dx / 2;
            const cy = startViewportPoint.y + dy / 2;
            currentCircle = { cx, cy, r };
        } else if (activeDrawingTool === 'polygon') {
            drawPreviewLines(currentViewportPoint);
            currentPreviewPolygonPoints = [...currentPolygon.points, currentViewportPoint];
        }
        event.preventDefaultAction = true;
    }

    async function onMouseUp(event) {
        if (activeDrawingTool !== 'polygon') {
            if (!isDrawing) return;
            isDrawing = false;
        }

        if (!startViewportPoint) return;

        const endViewportPoint = osdViewer.viewport.pointFromPixel(event.position);
        let newAnnotation = null;

        if (activeDrawingTool === 'rectangle') {
            const viewportRect = new OpenSeadragon.Rect(
                Math.min(startViewportPoint.x, endViewportPoint.x),
                Math.min(startViewportPoint.y, endViewportPoint.y),
                Math.abs(startViewportPoint.x - endViewportPoint.x),
                Math.abs(startViewportPoint.y - endViewportPoint.y)
            );
            if (viewportRect.width > 0.001 && viewportRect.height > 0.001) {
                newAnnotation = {
                    id: uuidv4(),
                    type: 'Annotation',
                    target: { selector: { type: 'FragmentSelector', value: { ...viewportRect, shape: 'rectangle' } } },
                    body: [{ type: 'Color', value: 'rgba(255, 242, 117, 0.5)', purpose: 'highlighting' }]
                };
            }
        } else if (activeDrawingTool === 'circle') {
            const dx = endViewportPoint.x - startViewportPoint.x;
            const dy = endViewportPoint.y - startViewportPoint.y;
            const r = Math.sqrt(dx * dx + dy * dy) / 2;
            const cx = startViewportPoint.x + dx / 2;
            const cy = startViewportPoint.y + dy / 2;
            if (r > 0.0005) {
                newAnnotation = {
                    id: uuidv4(),
                    type: 'Annotation',
                    target: { selector: { type: 'FragmentSelector', value: { cx, cy, r, shape: 'circle' } } },
                    body: [{ type: 'Color', value: 'rgba(255, 242, 117, 0.5)', purpose: 'highlighting' }]
                };
            }
        }

        if (newAnnotation) {
            updateImageAnnotations([...$currentAnnotations, newAnnotation]);
            await saveImageAnnotations();
            annotationBeingEdited = newAnnotation;
            isEditingExisting = false;
            // event.position is already relative to the viewer element
            dialogX = event.position.x;
            dialogY = event.position.y;
            showAnnotationCreationDialog = true;
        }
        currentRect = null;
        currentCircle = null;
        startViewportPoint = null;
    }

    async function onDoubleClick(event) {
        if (activeDrawingTool !== 'polygon' || !isDrawing) return;
        event.preventDefaultAction = true;

        const newAnnotation = {
            id: uuidv4(),
            type: 'Annotation',
            target: { selector: { type: 'FragmentSelector', value: { shape: 'polygon', points: currentPolygon.points } } },
            body: [{ type: 'Color', value: 'rgba(255, 242, 117, 0.5)', purpose: 'highlighting' }]
        };

        updateImageAnnotations([...$currentAnnotations, newAnnotation]);
        await saveImageAnnotations();

        annotationBeingEdited = newAnnotation;
        isEditingExisting = false;
        dialogX = event.position.x;
        dialogY = event.position.y;
        showAnnotationCreationDialog = true;

        isDrawing = false;
        currentPolygon = { points: [], previewLine: null, closingPreviewLine: null };
        currentPreviewPolygonPoints = [];
        activeDrawingTool = null;
        startViewportPoint = null;
    }

    function updateAnnotationPositions() {
        // This function is called on 'update-viewport' to re-render annotations.
        // Since the SVG is now an OSD overlay, and rects within it use viewport coordinates,
        // Svelte's reactivity will automatically re-evaluate the pixel positions of the rects
        // when the viewport changes. No explicit manual updates are needed here.
    }

    function handleAnnotationClick(event, annotation) {
        event.stopPropagation(); // Prevent OSD from handling the click
        console.log("Annotation clicked:", annotation);
        annotationBeingEdited = annotation;
        isEditingExisting = true;

        // Position dialog near the clicked annotation
        const osdRect = osdViewerElement.getBoundingClientRect();
        const annotationRect = event.currentTarget.getBoundingClientRect();
        dialogX = annotationRect.left - osdRect.left + annotationRect.width;
        dialogY = annotationRect.top - osdRect.top + annotationRect.height;

        showAnnotationCreationDialog = true;
    }

    async function handleAnnotationDialogSave(event) {
        const { title, description, color } = event.detail;
        if (!annotationBeingEdited) return;

        const newBody = [
            { type: 'Color', value: color, purpose: 'highlighting' }
        ];
        if (title) newBody.push({ type: 'Title', value: title, purpose: 'commenting' });
        if (description) newBody.push({ type: 'Description', value: description, purpose: 'commenting' });

        const updatedAnnotation = {
            ...annotationBeingEdited,
            body: newBody,
        };

        const updatedAnnotations = $currentAnnotations.map(a =>
            a.id === updatedAnnotation.id ? updatedAnnotation : a
        );

        updateImageAnnotations(updatedAnnotations);
        await saveImageAnnotations();

        closeAnnotationDialog();
    }

    async function handleAnnotationDialogCancel() {
        if (!isEditingExisting && annotationBeingEdited) {
            const annotationsWithoutNew = $currentAnnotations.filter(a => a.id !== annotationBeingEdited.id);
            updateImageAnnotations(annotationsWithoutNew);
            await saveImageAnnotations();
        }
        closeAnnotationDialog();
    }

    async function handleAnnotationDialogDelete() {
        if (annotationBeingEdited) {
            const annotationsWithoutDeleted = $currentAnnotations.filter(a => a.id !== annotationBeingEdited.id);
            updateImageAnnotations(annotationsWithoutDeleted);
            await saveImageAnnotations();
        }
        closeAnnotationDialog();
    }

    function closeAnnotationDialog() {
        annotationBeingEdited = null;
        isEditingExisting = false;
        showAnnotationCreationDialog = false;
    }

    function scrollToAnnotation(id) {
        if (!id || !osdViewer || !$currentAnnotations) return;
        const annotation = $currentAnnotations.find(a => a.id === id);
        if (annotation && annotation.target && annotation.target.selector && annotation.target.selector.value) {
            const shape = annotation.target.selector.value;
            let rect;
            if (shape.shape === 'rectangle') {
                rect = new OpenSeadragon.Rect(shape.x, shape.y, shape.width, shape.height);
            } else if (shape.shape === 'circle') {
                rect = new OpenSeadragon.Rect(shape.cx - shape.r, shape.cy - shape.r, shape.r * 2, shape.r * 2);
            } else if (shape.shape === 'polygon') {
                const xs = shape.points.map(p => p.x);
                const ys = shape.points.map(p => p.y);
                const minX = Math.min(...xs);
                const minY = Math.min(...ys);
                const maxX = Math.max(...xs);
                const maxY = Math.max(...ys);
                rect = new OpenSeadragon.Rect(minX, minY, maxX - minX, maxY - minY);
            }

            if (rect) {
                osdViewer.viewport.fitBounds(rect);
                osdViewer.viewport.zoomBy(0.5); // Zoom out to show context
                
                // Pulse effect
                if (svgOverlay) {
                    const shapeEl = svgOverlay.querySelector(`[data-annotation-id="${id}"]`);
                    if (shapeEl) {
                        shapeEl.style.transition = 'stroke-width 0.3s ease, stroke 0.3s ease';
                        const originalStrokeWidth = shapeEl.getAttribute('stroke-width');
                        const originalStroke = shapeEl.getAttribute('stroke');
                        
                        shapeEl.setAttribute('stroke-width', '0.005'); // Make it thick
                        shapeEl.setAttribute('stroke', 'blue');

                        setTimeout(() => {
                            shapeEl.setAttribute('stroke-width', originalStrokeWidth);
                            shapeEl.setAttribute('stroke', originalStroke);
                        }, 1000);
                    }
                }
            }
            project.update(p => ({ ...p, requestedHighlightId: null }));
        }
    }

    $: if ($project.requestedHighlightId && osdViewer && !isLoading) {
        scrollToAnnotation($project.requestedHighlightId);
    }

    onMount(() => {
        console.log('[ImageViewerPanel] Mounted. Initial Path:', imagePath);
        if (imagePath && osdViewerElement) { initializeViewer(imagePath); }
        else { isLoading = false; console.log("[ImageViewerPanel onMount] No imagePath or osdViewerElement, not initializing."); }
    });

    onDestroy(() => {
        if (osdViewer) {
            osdViewer.removeHandler('canvas-press', onMouseDown);
            osdViewer.removeHandler('canvas-drag', onMouseMove);
            osdViewer.removeHandler('canvas-release', onMouseUp);
            osdViewer.removeHandler('canvas-double-click', onDoubleClick);
            osdViewer.removeHandler('update-viewport', updateAnnotationPositions);
            if (svgOverlay && osdViewer.getOverlayById(svgOverlay)) {
                osdViewer.removeOverlay(svgOverlay);
            }
            osdViewer.destroy();
            osdViewer = null;
        }
    });

    $: {
        if (imagePath && imagePath !== currentLoadedPath && osdViewerElement) {
            console.log(`[ImageViewerPanel reactive] imagePath changed from '${currentLoadedPath || 'null'}' to '${imagePath}'`);
            initializeViewer(imagePath);
        } else if (imagePath && imagePath !== currentLoadedPath && !osdViewerElement) {
            console.log(`[ImageViewerPanel reactive] imagePath changed to ${imagePath}, but osdViewerElement not ready. Deferring init.`);
            if (!isLoading) isLoading = true;
        } else if (!imagePath && osdViewer) {
            console.log(`[ImageViewerPanel reactive] imagePath cleared, destroying viewer instance.`);
            if (osdViewer) {
                if (svgOverlay && osdViewer.getOverlayById(svgOverlay)) {
                    osdViewer.removeOverlay(svgOverlay);
                }
                osdViewer.destroy();
            }
            osdViewer = null;
            isLoading = false; error = null; currentLoadedPath = null;
        }
    }

    // Reactive statement to update annotation positions when viewport changes
    $: if (osdViewer && $currentAnnotations) {
        // This is a reactive statement that will re-run whenever $currentAnnotations changes,
        // which is exactly what we want to keep the SVG overlay in sync with the store.
        // Svelte's reactivity handles the re-rendering of the {#each} block below.
    }
</script>

<svelte:head>
    <!-- Removed Annotorious CSS links -->
</svelte:head>

<div class="flex flex-col h-full w-full bg-white dark:bg-dark-bg-form-field shadow overflow-hidden">
    <div class="flex items-center justify-between h-9 px-2 border-b border-gray-200 dark:border-dark-bg-tertiary bg-gray-100 dark:bg-surface-3">
        <div id="image-annotation-toolbar-container" class="flex items-center space-x-2">
            <button
                class="ui-button-icon"
                class:active={activeDrawingTool === 'rectangle'}
                on:click={() => activeDrawingTool = (activeDrawingTool === 'rectangle' ? null : 'rectangle')}
                title="Draw Rectangle"
            >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" viewBox="0 0 16 16">
                    <path d="M14 1a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H2a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1zM2 0a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V2a2 2 0 0 0-2-2z"/>
                </svg>
            </button>
            <button
                class="ui-button-icon"
                class:active={activeDrawingTool === 'circle'}
                on:click={() => activeDrawingTool = (activeDrawingTool === 'circle' ? null : 'circle')}
                title="Draw Circle"
            >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" viewBox="0 0 16 16">
                    <path d="M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14m0 1A8 8 0 1 0 8 0a8 8 0 0 0 0 16"/>
                </svg>
            </button>
            <button
                class="ui-button-icon"
                class:active={activeDrawingTool === 'polygon'}
                on:click={() => activeDrawingTool = (activeDrawingTool === 'polygon' ? null : 'polygon')}
                title="Draw Polygon"
            >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" viewBox="0 0 16 16">
                    <polygon 
                        points="2,2 14,4 12,14 4,12 6,7" 
                        fill="none" 
                        stroke="currentColor" 
                        stroke-width="1" 
                    />
                </svg>
            </button>
        </div>
    </div>

    <div class="flex-grow overflow-hidden min-h-0 relative">
        {#if isLoading && !error}
            <div class="absolute inset-0 flex items-center justify-center text-gray-500 dark:text-d-gray-400 z-10 bg-white/50 dark:bg-d-gray-800/50">Loading image viewer...</div>
        {:else if error}
            <div class="absolute inset-0 flex items-center justify-center text-red-600 dark:text-red-400 p-4 text-center z-10 bg-white/80 dark:bg-d-gray-800/80">{error}</div>
        {/if}
        <div bind:this={osdViewerElement} class="w-full h-full osd-viewer-container" class:opacity-0={isLoading || error}>
        </div>

        <!-- SVG overlay for drawing and displaying annotations -->
        <!-- Moved outside osdViewerElement to prevent OSD initialization from clearing it -->
        <svg bind:this={svgOverlay} class="pointer-events-none z-20 absolute inset-0" viewBox="0 0 1 1"
                class:cursor-draw={activeDrawingTool !== null}
                class:cursor-pan={activeDrawingTool === null}>
            {#each $currentAnnotations as annotation (annotation.id)}
                {@const shapeData = annotation.target.selector.value}
                {@const colorBody = annotation.body.find(b => b.purpose === 'highlighting' && b.type === 'Color')}
                {@const fillColor = colorBody ? colorBody.value : 'rgba(255, 242, 117, 0.5)'}
                {@const strokeColor = adjustOpacity(fillColor, 1)}

                {#if shapeData.shape === 'rectangle'}
                    <rect
                        x={shapeData.x}
                        y={shapeData.y}
                        width={shapeData.width}
                        height={shapeData.height}
                        fill={fillColor}
                        stroke={strokeColor}
                        stroke-width="1px"
                        vector-effect="non-scaling-stroke"
                        class="pointer-events-auto cursor-pointer annotation-shape"
                        data-annotation-id={annotation.id}
                        on:pointerdown|stopPropagation={(e) => handleAnnotationClick(e, annotation)}
                    />
                {:else if shapeData.shape === 'circle'}
                    <circle
                        cx={shapeData.cx}
                        cy={shapeData.cy}
                        r={shapeData.r}
                        fill={fillColor}
                        stroke={strokeColor}
                        stroke-width="1px"
                        vector-effect="non-scaling-stroke"
                        class="pointer-events-auto cursor-pointer annotation-shape"
                        data-annotation-id={annotation.id}
                        on:pointerdown|stopPropagation={(e) => handleAnnotationClick(e, annotation)}
                    />
                {:else if shapeData.shape === 'polygon'}
                    <polygon
                        points={shapeData.points.map(p => `${p.x},${p.y}`).join(' ')}
                        fill={fillColor}
                        stroke={strokeColor}
                        stroke-width="0.002"
                        class="pointer-events-auto cursor-pointer annotation-shape"
                        data-annotation-id={annotation.id}
                        on:pointerdown|stopPropagation={(e) => handleAnnotationClick(e, annotation)}
                    />
                {/if}
            {/each}
            {#if isDrawing && activeDrawingTool === 'rectangle' && currentRect}
                <rect
                    x={currentRect.x}
                    y={currentRect.y}
                    width={currentRect.width}
                    height={currentRect.height}
                    fill="rgba(255, 242, 117, 0.5)"
                    stroke="rgba(255, 242, 117, 1)"
                    stroke-width="1px"
                    vector-effect="non-scaling-stroke"
                />
            {:else if isDrawing && activeDrawingTool === 'circle' && currentCircle}
                <circle
                    cx={currentCircle.cx}
                    cy={currentCircle.cy}
                    r={currentCircle.r}
                    fill="rgba(255, 242, 117, 0.5)"
                    stroke="rgba(255, 242, 117, 1)"
                    stroke-width="1px"
                    vector-effect="non-scaling-stroke"
                />
            {:else if isDrawing && activeDrawingTool === 'polygon' && currentPreviewPolygonPoints.length > 0}
                <polygon
                    points={currentPreviewPolygonPoints.map(p => `${p.x},${p.y}`).join(' ')}
                    fill="rgba(255, 242, 117, 0.5)"
                    stroke="rgba(255, 242, 117, 1)"
                    stroke-width="0.002"
                    vector-effect="non-scaling-stroke"
                />
                {#if currentPolygon.previewLine}
                    <line
                        x1={currentPolygon.previewLine.x1}
                        y1={currentPolygon.previewLine.y1}
                        x2={currentPolygon.previewLine.x2}
                        y2={currentPolygon.previewLine.y2}
                        stroke="rgba(255, 242, 117, 1)"
                        stroke-width="1px"
                        stroke-dasharray="0.01, 0.01"
                        vector-effect="non-scaling-stroke"
                    />
                {/if}
                {#if currentPolygon.closingPreviewLine}
                    <line
                        x1={currentPolygon.closingPreviewLine.x1}
                        y1={currentPolygon.closingPreviewLine.y1}
                        x2={currentPolygon.closingPreviewLine.x2}
                        y2={currentPolygon.closingPreviewLine.y2}
                        stroke="rgba(255, 242, 117, 1)"
                        stroke-width="1px"
                        stroke-dasharray="0.01, 0.01"
                        vector-effect="non-scaling-stroke"
                    />
                {/if}
            {/if}
        </svg>

        {#if showAnnotationCreationDialog}
            {#key annotationBeingEdited?.id}
                <AnnotationCreationDialog
                    x={dialogX}
                    y={dialogY}
                    initialTitle={annotationBeingEdited?.body?.find(b => b.type === 'Title')?.value || ''}
                    initialDescription={annotationBeingEdited?.body?.find(b => b.type === 'Description')?.value || ''}
                    initialColor={annotationBeingEdited?.body?.find(b => b.type === 'Color')?.value || 'rgba(255, 242, 117, 0.5)'}
                    isEditing={isEditingExisting}
                    panelBounds={osdViewerElement ? osdViewerElement.getBoundingClientRect() : null}
                    on:save={handleAnnotationDialogSave}
                    on:cancel={handleAnnotationDialogCancel}
                    on:delete={handleAnnotationDialogDelete}
                />
            {/key}
        {/if}
    </div>
</div>

<style lang="postcss">
    .min-h-0 { min-height: 0; }
    .osd-viewer-container {
        background-color: theme('colors.gray.300');
    }
    :global(html.dark) .osd-viewer-container {
        background-color: theme('colors.dark-bg-form-field');
    }
    .opacity-0 { opacity: 0; }

    .cursor-pan {
        cursor: grab;
    }
    .cursor-pan:active {
        cursor: grabbing;
    }
    .cursor-draw {
        cursor: crosshair;
    }

    #image-annotation-toolbar-container:empty {
    }

    /* Removed Annotorious specific styles */
    :global(.openseadragon-container .openseadragon-canvas) {
        outline: none !important;
    }
    .non-scaling-stroke {
        vector-effect: non-scaling-stroke;
    }

    :global(.openseadragon-container div) {
        box-sizing: content-box;
    }

    button.active {
        @apply bg-blue-500 text-white;
    }
</style>
