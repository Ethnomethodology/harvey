<script>
    import { onMount, onDestroy, tick } from 'svelte';
    import { convertFileSrc, invoke } from '@tauri-apps/api/core';
    import { dirname, join, sep } from '@tauri-apps/api/path'; // ensure sep is imported
    import { get } from 'svelte/store';
    import { project } from '$lib/stores/projectStore.js';
    import OpenSeadragon from 'openseadragon';
    import { v4 as uuidv4 } from 'uuid'; // For generating unique IDs for annotations

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

    let currentAnnotations = []; // This will hold our annotation data

    // State for drawing mode: 'rectangle', 'circle', 'polygon', or null
    let activeDrawingTool = null;

    // Variables for drawing
    let isDrawing = false;
    let startPoint = null;
    let currentRect = null; // { x, y, width, height } for rectangle
    let currentCircle = null; // { cx, cy, r } for circle
    let currentPolygon = { points: [], previewLine: null, closingPreviewLine: null }; // For polygon drawing
    let svgOverlay; // Reference to the SVG element

    async function loadAnnotationsForImage(imgPath) {
        currentAnnotations = [];

        const currentProj = get(project);
        if (!currentProj || !currentProj.id || typeof currentProj.id !== 'string' || currentProj.id.trim() === '') {
            console.error('[ImageViewerPanel loadAnnotationsForImage] project ID (from $project.id) is missing or invalid.');
            return;
        }
        const projectId = currentProj.id;

        const projectBaseDir = currentProj.baseDirectory;
        let relativeImagePath = imgPath;

        if (projectBaseDir && imgPath.startsWith(projectBaseDir)) {
            relativeImagePath = imgPath.substring(projectBaseDir.length);
            if (relativeImagePath.startsWith(sep) || relativeImagePath.startsWith('/') || relativeImagePath.startsWith('\\')) {
                relativeImagePath = relativeImagePath.substring(1);
            }
        } else {
            console.warn(`[ImageViewerPanel loadAnnotationsForImage] imgPath "${imgPath}" may not be within projectBaseDir "${projectBaseDir}". Using path as is for DB key, this might fail if not truly relative.`);
        }
        relativeImagePath = relativeImagePath.replace(/\\/g, '/');

        console.log(`[ImageViewerPanel loadAnnotationsForImage] Attempting for project ${projectId} (from $project.id), image relative path: ${relativeImagePath} (absolute: ${imgPath})`);

        try {
            const annotationsJsonString = await invoke('load_image_annotations', {
                projectId: projectId,
                imageRelativePathStr: relativeImagePath
            });
            if (annotationsJsonString && typeof annotationsJsonString === 'string') {
                const loaded = JSON.parse(annotationsJsonString);
                if (Array.isArray(loaded)) {
                    currentAnnotations = loaded;
                    console.log(`[ImageViewerPanel loadAnnotationsForImage] Loaded ${loaded.length} annotations for ${relativeImagePath}.`);
                } else {
                    console.warn(`[ImageViewerPanel loadAnnotationsForImage] Loaded data for ${relativeImagePath} is not an array.`);
                }
            } else {
                 console.log(`[ImageViewerPanel loadAnnotationsForImage] No annotations found or empty content for ${relativeImagePath}.`);
            }
        } catch (err) {
            console.error(`[ImageViewerPanel loadAnnotationsForImage] Error for ${relativeImagePath} (absolute: ${imgPath}):`, err);
            currentAnnotations = [];
        }
    }

    async function saveAnnotationsForImage() {
        if (!currentLoadedPath) {
            console.warn("[ImageViewerPanel saveAnnotationsForImage] No image path, cannot save.");
            return;
        }
        const currentProj = get(project);
        if (!currentProj || !currentProj.id || typeof currentProj.id !== 'string' || currentProj.id.trim() === '') {
            console.error('[ImageViewerPanel saveAnnotationsForImage] project ID (from $project.id) is missing or invalid.');
            // await message('Cannot save image annotations: Project identifier is missing or invalid.', { title: 'Save Error', type: 'error' });
            return;
        }
        const projectId = currentProj.id;

        const projectBaseDir = currentProj.baseDirectory;
        let relativeImagePath = currentLoadedPath;

        if (projectBaseDir && currentLoadedPath.startsWith(projectBaseDir)) {
            relativeImagePath = currentLoadedPath.substring(projectBaseDir.length);
            if (relativeImagePath.startsWith(sep) || relativeImagePath.startsWith('/') || relativeImagePath.startsWith('\\')) {
                relativeImagePath = relativeImagePath.substring(1);
            }
        } else {
            console.error(`[ImageViewerPanel saveAnnotationsForImage] Cannot determine relative path for ${currentLoadedPath} using base ${projectBaseDir}. Cannot save.`);
            return;
        }
        relativeImagePath = relativeImagePath.replace(/\\/g, '/');

        console.log(`[ImageViewerPanel saveAnnotationsForImage] Saving ${currentAnnotations.length} annotations for project ${projectId} (from $project.id), image relative path: ${relativeImagePath}`);

        try {
            await invoke('save_image_annotations', {
                projectId: projectId,
                imageRelativePathStr: relativeImagePath,
                annotationsJsonString: JSON.stringify(currentAnnotations, null, 2)
            });
            console.log(`[ImageViewerPanel saveAnnotationsForImage] Annotations saved for project ${projectId} (from $project.id), image ${relativeImagePath}`);
        } catch (err) {
            console.error(`[ImageViewerPanel saveAnnotationsForImage] Error saving for project ${projectId} (from $project.id), image ${relativeImagePath}:`, err);
        }
    }

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
        currentAnnotations = [];

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
                await loadAnnotationsForImage(pathForImage);
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
                    target: {
                        selector: {
                            type: 'FragmentSelector',
                            value: {
                                x: viewportRect.x,
                                y: viewportRect.y,
                                width: viewportRect.width,
                                height: viewportRect.height,
                                shape: 'rectangle'
                            }
                        }
                    },
                    body: [
                        { type: 'Color', value: 'rgba(255, 242, 117, 0.5)', purpose: 'highlighting' }
                    ]
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
                    target: {
                        selector: {
                            type: 'FragmentSelector',
                            value: {
                                cx: cx,
                                cy: cy,
                                r: r,
                                shape: 'circle'
                            }
                        }
                    },
                    body: [
                        { type: 'Color', value: 'rgba(255, 242, 117, 0.5)', purpose: 'highlighting' }
                    ]
                };
            }
        }

        if (newAnnotation) {
            currentAnnotations = [...currentAnnotations, newAnnotation];
            await saveAnnotationsForImage();

            annotationBeingEdited = newAnnotation;
            isEditingExisting = false;
            const osdRect = osdViewerElement.getBoundingClientRect();
            dialogX = event.position.x - osdRect.left;
            dialogY = event.position.y - osdRect.top;
            showAnnotationCreationDialog = true;
        }
        currentRect = null;
        currentCircle = null;
    }

    function onDoubleClick(event) {
        if (activeDrawingTool !== 'polygon' || !isDrawing) {
            return;
        }
        event.preventDefaultAction = true;

        const newAnnotation = {
            id: uuidv4(),
            type: 'Annotation',
            target: {
                selector: {
                    type: 'FragmentSelector',
                    value: {
                        shape: 'polygon',
                        points: currentPolygon.points
                    }
                }
            },
            body: [
                { type: 'Color', value: 'rgba(255, 242, 117, 0.5)', purpose: 'highlighting' }
            ]
        };

        currentAnnotations = [...currentAnnotations, newAnnotation];
        saveAnnotationsForImage();

        // Reset drawing state
        isDrawing = false;
        currentPolygon = { points: [], previewLine: null, closingPreviewLine: null };
        activeDrawingTool = null;
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
        const annotationRect = event.target.getBoundingClientRect();
        dialogX = annotationRect.left - osdRect.left + annotationRect.width;
        dialogY = annotationRect.top - osdRect.top + annotationRect.height;

        showAnnotationCreationDialog = true;
    }

    async function handleAnnotationDialogSave(event) {
        const { title, description, color } = event.detail;
        if (annotationBeingEdited) {
            const newBody = [];
            if (title) newBody.push({ type: 'Title', value: title, purpose: 'commenting' });
            if (description) newBody.push({ type: 'Description', value: description, purpose: 'commenting' });
            newBody.push({ type: 'Color', value: color, purpose: 'highlighting' });

            const updatedAnnotation = {
                ...annotationBeingEdited,
                body: newBody
            };

            currentAnnotations = currentAnnotations.map(a =>
                a.id === updatedAnnotation.id ? updatedAnnotation : a
            );
            await saveAnnotationsForImage();
        }
        annotationBeingEdited = null;
        isEditingExisting = false;
        showAnnotationCreationDialog = false;
    }

    async function handleAnnotationDialogCancel() {
        // If it was a new annotation being created, remove it from the list
        if (!isEditingExisting && annotationBeingEdited) {
            currentAnnotations = currentAnnotations.filter(a => a.id !== annotationBeingEdited.id);
            await saveAnnotationsForImage();
        }
        annotationBeingEdited = null;
        isEditingExisting = false;
        showAnnotationCreationDialog = false;
    }

    async function handleAnnotationDialogDelete() {
        if (annotationBeingEdited) {
            currentAnnotations = currentAnnotations.filter(a => a.id !== annotationBeingEdited.id);
            await saveAnnotationsForImage();
        }
        annotationBeingEdited = null;
        isEditingExisting = false;
        showAnnotationCreationDialog = false;
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
            // Remove the SVG overlay when component is destroyed
            if (svgOverlay) { // Removed hasOverlay check
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
                // Remove the SVG overlay before destroying OSD
                if (svgOverlay) { // Removed hasOverlay check
                    osdViewer.removeOverlay(svgOverlay);
                }
                osdViewer.destroy();
            }
            osdViewer = null;
            isLoading = false; error = null; currentLoadedPath = null;
            currentAnnotations = [];
        }
    }

    // Reactive statement to update annotation positions when viewport changes
    // This is crucial for annotations to stay in place during zoom/pan
    $: if (osdViewer && currentAnnotations) {
        // Force re-render of annotations when viewport changes
        // This is a bit of a hack, but ensures Svelte re-evaluates the `rect` calculation
        // for each annotation. A more performant solution might involve directly manipulating
        // DOM elements, but for a small number of annotations, this is fine.
        currentAnnotations = currentAnnotations;
    }
</script>

<svelte:head>
    <!-- Removed Annotorious CSS links -->
</svelte:head>

<div class="flex flex-col h-full w-full bg-white dark:bg-gray-800 rounded-md shadow overflow-hidden">
    <div class="flex items-center justify-between px-1 border-b border-gray-200 dark:border-gray-600 flex-shrink-0 text-xs">
        <div id="image-annotation-toolbar-container" class="flex items-center h-9 border border-transparent">
            <span class="text-xs font-medium pr-1">Highlight:</span>
            <button
                class="inline-flex items-center justify-center px-2 py-1 border
                       border-gray-300 dark:border-gray-600 text-xs rounded-md
                       focus:outline-none focus:ring-2 focus:ring-blue-500
                       {activeDrawingTool === 'rectangle' ? 'bg-blue-500 text-white hover:bg-blue-600' : 'bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700'}"
                on:click={() => activeDrawingTool = (activeDrawingTool === 'rectangle' ? null : 'rectangle')}
            >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" viewBox="0 0 16 16">
                    <path d="M14 1a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H2a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1zM2 0a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V2a2 2 0 0 0-2-2z"/>
                </svg>
            </button>
            <button
                class="inline-flex items-center justify-center px-2 py-1 border
                       border-gray-300 dark:border-gray-600 text-xs rounded-md ml-2
                       focus:outline-none focus:ring-2 focus:ring-blue-500
                       {activeDrawingTool === 'circle' ? 'bg-blue-500 text-white hover:bg-blue-600' : 'bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700'}"
                on:click={() => activeDrawingTool = (activeDrawingTool === 'circle' ? null : 'circle')}
            >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" viewBox="0 0 16 16">
                    <path d="M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14m0 1A8 8 0 1 0 8 0a8 8 0 0 0 0 16"/>
                </svg>
            </button>
            <button
                class="inline-flex items-center justify-center px-2 py-1 border
                       border-gray-300 dark:border-gray-600 text-xs rounded-md ml-2
                       focus:outline-none focus:ring-2 focus:ring-blue-500
                       {activeDrawingTool === 'polygon' ? 'bg-blue-500 text-white hover:bg-blue-600' : 'bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700'}"
                on:click={() => activeDrawingTool = (activeDrawingTool === 'polygon' ? null : 'polygon')}
            >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" viewBox="0 0 16 16">
                    <path d="M8 .5a.5.5 0 0 1 .5.5v4.293l3.146-3.147a.5.5 0 1 1 .708.708L9.207 6H13.5a.5.5 0 0 1 .5.5v3.793l3.146-3.147a.5.5 0 1 1 .708.708L14.207 10H16a.5.5 0 0 1 .5.5v.207l-3.146 3.147a.5.5 0 1 1-.708-.708L15.293 11H11.5a.5.5 0 0 1-.5-.5V6.707l-3.146 3.147a.5.5 0 1 1-.708-.708L9.793 7H5.5a.5.5 0 0 1-.5-.5v-4a.5.5 0 0 1 .5-.5h.207L2.354.146a.5.5 0 1 1 .708.708L5.207 3H8.5a.5.5 0 0 1 .5.5v3.793l-3.146-3.147a.5.5 0 1 1-.708.708L6.793 7H3.5a.5.5 0 0 1-.5-.5v-4a.5.5 0 0 1 .5-.5H8z"/>
                </svg>
            </button>
        </div>
    </div>

    <div class="flex-grow overflow-hidden min-h-0 relative">
        {#if isLoading && !error}
            <div class="absolute inset-0 flex items-center justify-center text-gray-500 dark:text-gray-400 z-10 bg-white/50 dark:bg-gray-800/50">Loading image viewer...</div>
        {:else if error}
            <div class="absolute inset-0 flex items-center justify-center text-red-600 dark:text-red-400 p-4 text-center z-10 bg-white/80 dark:bg-gray-800/80">{error}</div>
        {/if}
        <div bind:this={osdViewerElement} class="w-full h-full osd-viewer-container" class:opacity-0={isLoading || error}>
            <!-- SVG overlay for drawing and displaying annotations -->
            <svg bind:this={svgOverlay} class="pointer-events-none z-20" viewBox="0 0 1 1"
                 class:cursor-draw={activeDrawingTool !== null}
                 class:cursor-pan={activeDrawingTool === null}>
                {#each currentAnnotations as annotation (annotation.id)}
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
                            stroke-width="0.002"
                            vector-effect="non-scaling-stroke"
                            class="pointer-events-auto cursor-pointer"
                            on:click={(e) => handleAnnotationClick(e, annotation)}
                        />
                    {:else if shapeData.shape === 'circle'}
                        <circle
                            cx={shapeData.cx}
                            cy={shapeData.cy}
                            r={shapeData.r}
                            fill={fillColor}
                            stroke={strokeColor}
                            stroke-width="0.002"
                            vector-effect="non-scaling-stroke"
                            class="pointer-events-auto cursor-pointer"
                            on:click={(e) => handleAnnotationClick(e, annotation)}
                        />
                    {:else if shapeData.shape === 'polygon'}
                        <polygon
                            points={shapeData.points.map(p => `${p.x},${p.y}`).join(' ')}
                            fill={fillColor}
                            stroke={strokeColor}
                            stroke-width="0.002"
                            class="pointer-events-auto cursor-pointer"
                            on:click|stopPropagation={(e) => handleAnnotationClick(e, annotation)}
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
                        stroke-width="0.002"
                        vector-effect="non-scaling-stroke"
                    />
                {:else if isDrawing && activeDrawingTool === 'circle' && currentCircle}
                    <circle
                        cx={currentCircle.cx}
                        cy={currentCircle.cy}
                        r={currentCircle.r}
                        fill="rgba(255, 242, 117, 0.5)"
                        stroke="rgba(255, 242, 117, 1)"
                        stroke-width="0.002"
                        vector-effect="non-scaling-stroke"
                    />
                {:else if isDrawing && activeDrawingTool === 'polygon' && currentPolygon.points.length > 0}
                    <polygon
                        points={currentPolygon.points.map(p => `${p.x},${p.y}`).join(' ')}
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
                            stroke-width="0.002"
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
                            stroke-dasharray="0.01, 0.01"
                            vector-effect="non-scaling-stroke"
                        />
                    {/if}
                {/if}
            </svg>
        </div>

        {#if showAnnotationCreationDialog}
            <AnnotationCreationDialog
                x={dialogX}
                y={dialogY}
                initialTitle={annotationBeingEdited?.body?.find(b => b.type === 'Title')?.value || ''}
                initialDescription={annotationBeingEdited?.body?.find(b => b.type === 'Description')?.value || ''}
                initialColor={annotationBeingEdited?.body?.find(b => b.type === 'Color')?.value || 'rgba(255, 242, 117, 0.5)'}
                isEditing={isEditingExisting}
                on:save={handleAnnotationDialogSave}
                on:cancel={handleAnnotationDialogCancel}
                on:delete={handleAnnotationDialogDelete}
            />
        {/if}
    </div>
</div>

<style lang="postcss">
    .min-h-0 { min-height: 0; }
    .osd-viewer-container {
        background-color: theme('colors.gray.300');
    }
    :global(html.dark) .osd-viewer-container {
        background-color: theme('colors.gray.700');
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
</style>
