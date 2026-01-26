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

    // State for drawing mode: 'rectangle', 'circle', 'polygon', 'speech-bubble-rect', 'speech-bubble-circle', or null
    let activeDrawingTool = null;
    let selectedBaseColorIndex = 0; // Index into baseColors

    const baseColors = [
        { name: 'Yellow', rgb: '255, 242, 117' },
        { name: 'Green', rgb: '168, 255, 158' },
        { name: 'Blue', rgb: '174, 239, 255' },
        { name: 'Pink', rgb: '255, 176, 207' },
        { name: 'Purple', rgb: '208, 160, 255' },
        { name: 'White', rgb: '255, 255, 255' },
    ];

    function getSelectedColor(isSpeechBubble) {
        const base = baseColors[selectedBaseColorIndex];
        return `rgba(${base.rgb}, ${isSpeechBubble ? '1' : '0.5'})`;
    }

    // Variables for drawing
    let isDrawing = false;
    let startPoint = null;
    let currentRect = null; // { x, y, width, height } for rectangle & speech-bubble-rect
    let currentCircle = null; // { cx, cy, r } for circle & speech-bubble-circle
    let currentPolygon = { points: [], previewLine: null, closingPreviewLine: null }; // For polygon drawing
    let currentPreviewPolygonPoints = []; // For filled polygon preview
    let svgOverlay; // Reference to the SVG element

    // State for dragging tail
    let isDraggingTail = false;
    let isDraggingShape = false;
    let isDraggingResizeHandle = false;
    let selectedAnnotationId = null;
    let draggedAnnotationId = null;
    let draggedHandleType = null; // 'nw', 'ne', 'sw', 'se', 'r', or index for polygon
    let dragStartViewportPoint = null;
    let handleRadius = 0.008; // Default viewport radius

    $: if (activeDrawingTool) {
        selectedAnnotationId = null;
    }

    function adjustOpacity(rgbaColor, newOpacity) {
        if (!rgbaColor || typeof rgbaColor !== 'string' || !rgbaColor.startsWith('rgba(')) { return rgbaColor; }
        const parts = rgbaColor.substring(5, rgbaColor.length - 1).split(',');
        if (parts.length !== 4) return rgbaColor;
        return `rgba(${parts[0].trim()}, ${parts[1].trim()}, ${parts[2].trim()}, ${newOpacity})`;
    }

    // Helper to generate speech bubble path
    function getBubblePath(shapeData, isCircle) {
        const { x, y, width, height, cx, cy, r, tail } = shapeData;
        if (!tail) return ''; // Should not happen for valid bubbles

        const tx = tail.x;
        const ty = tail.y;

        let center, bounds;
        if (isCircle) {
            center = { x: cx, y: cy };
            bounds = { x: cx - r, y: cy - r, width: r * 2, height: r * 2 };
        } else {
            center = { x: x + width / 2, y: y + height / 2 };
            bounds = { x, y, width, height };
        }

        // Vector from center to tail tip
        const dx = tx - center.x;
        const dy = ty - center.y;
        const len = Math.sqrt(dx * dx + dy * dy);
        if (len === 0) return ''; // Degenerate

        // Normalized direction
        const dir = { x: dx / len, y: dy / len };

        // Find intersection with shape boundary to determine base of tail
        let ix, iy;
        if (isCircle) {
            ix = center.x + dir.x * r;
            iy = center.y + dir.y * r;
        } else {
            // Rect intersection (simplified: check intersection with 4 sides)
            // Ray: Center + t * dir. Find min t > 0 that hits a side.
            // x = center.x + t*dir.x, y = center.y + t*dir.y
            // Side 1 (Right): x = bounds.x + bounds.width. t = (bounds.x + bounds.width - center.x) / dir.x
            // etc.
            const tValues = [];
            if (dir.x > 0) tValues.push((bounds.x + bounds.width - center.x) / dir.x);
            else if (dir.x < 0) tValues.push((bounds.x - center.x) / dir.x);
            if (dir.y > 0) tValues.push((bounds.y + bounds.height - center.y) / dir.y);
            else if (dir.y < 0) tValues.push((bounds.y - center.y) / dir.y);

            const t = Math.min(...tValues.filter(v => v > 0));
            ix = center.x + dir.x * t;
            iy = center.y + dir.y * t;
        }

        // Calculate base points perpendicular to direction
        // Base width proportional to size, but clamped
        const sizeParam = isCircle ? r * 2 : Math.min(width, height);
        const baseHalfWidth = Math.max(sizeParam * 0.1, 0.01); // Min size in viewport coords

        const perp = { x: -dir.y, y: dir.x };
        const b1 = { x: ix + perp.x * baseHalfWidth, y: iy + perp.y * baseHalfWidth };
        const b2 = { x: ix - perp.x * baseHalfWidth, y: iy - perp.y * baseHalfWidth };

        // Construct Path
        // Start at b1, go to tip, go to b2.
        // Then draw the rest of the shape.
        // For simplicity in SVG, we can just draw the shape and the triangle and union them visually?
        // But for a clean stroke, we need a single path.
        // A full robust path union is complex.
        // Approximation: Move to b1, Line to Tip, Line to b2.
        // Then we need to arc/line around the shape from b2 back to b1.
        
        // Simpler visual trick: Draw the shape filled. Draw the triangle filled.
        // Then draw the union stroke?
        // Let's try to construct a decent path.
        
        // For Rectangle:
        if (!isCircle) {
             // It's a path. M b1.x b1.y L tx ty L b2.x b2.y.
             // But we need to follow the rect.
             // This is getting complex to do perfectly in math without a library.
             // FALLBACK: Return two paths? No.
             // Let's use the visual trick: Return a path that is just the Triangle (Tip -> B1 -> B2).
             // And we will render the main Shape (Rect/Circle) separately.
             // To make them look merged, we can render:
             // 1. Filled Shape
             // 2. Filled Triangle
             // 3. Strokeless Shape (if fill only)
             // But we have transparency (rgba 0.5). Overlap will show darker.
             // So we MUST merge or use 'clip-path' or similar.
             // Or, simply, assume the user accepts the overlap for now?
             // "The speech bubble 'tail' (the part pointing to the speaker) usually needs to move independently".
             // If I draw a circle and a separate triangle, they overlap. With 0.5 alpha, the overlap is visible.
             // Standard Speech Bubble SVG:
             // It is one path.
             // Let's try to do the Circle one properly.
             // Arc from b2 to b1 (large arc). Line to Tip. Close.
        }
        
        if (isCircle) {
            // Angle of b1 and b2
            const ang1 = Math.atan2(b1.y - center.y, b1.x - center.x);
            const ang2 = Math.atan2(b2.y - center.y, b2.x - center.x);
            
            // We want to draw arc from b2 to b1.
            // SVG Arc: A rx ry x-axis-rotation large-arc-flag sweep-flag x y
            // We want the long way around? Usually yes, unless tail is inside (which we assume not).
            // Check cross product to determine sweep?
            // Assuming standard counter-clockwise or clockwise.
            
            // Let's just return a path string.
            const largeArc = 1; // Almost full circle
            const sweep = 1; // Clockwise?
            
            return `M ${b1.x} ${b1.y} L ${tx} ${ty} L ${b2.x} ${b2.y} A ${r} ${r} 0 ${largeArc} ${sweep} ${b1.x} ${b1.y} Z`;
        } else {
             // Rectangle.
             // We can use a similar logic if we treat it as a polygon.
             // But the rect has corners.
             // Lets just draw the full rect, and the triangle, but use a mask/clip? No, SVG doesn't support "union" path operation natively easily.
             // OK, for this prototype, I will just return the Triangle path. 
             // AND I will render the Rect/Circle as usual.
             // The user will see the overlap. This is acceptable for a "harvey" prototype unless I use a library like 'paper.js' or 'd3-polygon'.
             // Wait, I can make the tail *start* from the center!
             // Then the triangle is (Center, B1_far, B2_far)? No.
             
             // ALTERNATIVE: Use a `mask`.
             // Define a mask that is the union of shape + tail.
             // Draw a rect filling the bounding box of union with that mask.
             
             // SIMPLEST: Render shape and tail with opacity = 1.0 (no transparency)?
             // The default color is `rgba(255, 242, 117, 0.5)`.
             // If I change the design to solid borders and transparent fill... overlap is fine for fill?
             // No, fill overlap doubles opacity.
             
             // COMPROMISE:
             // For Circle: Use the Arc logic `M b1... L tip... L b2... A ...`. It works well.
             // For Rect: I'll use the "Triangle + Rect" approach but accept the overlap artifacts for now to ensure stability and "independent movement".
             // Actually, the prompt asks for "speech bubbles".
             // Let's try to be clever.
             // If I draw the triangle from the *center* to the tip?
             // No.
             
             // Let's stick to: Circle uses the Arc path (it looks good).
             // Rect uses Rect + Triangle.
             const triPath = `M ${b1.x} ${b1.y} L ${tx} ${ty} L ${b2.x} ${b2.y} Z`;
             return triPath;
        }
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
                updateAnnotationPositions();
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
        if (event.originalEvent.button !== 0) return; // Only left click

        const viewportPoint = osdViewer.viewport.pointFromPixel(event.position);

        // If we just clicked a handle or shape, one of these flags will be true
        // because the SVG event handlers fire before OSD canvas-press.
        if (!activeDrawingTool && !isDraggingShape && !isDraggingTail && !isDraggingResizeHandle) {
             selectedAnnotationId = null;
        }

        if (!activeDrawingTool) return;
        
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
            if (activeDrawingTool === 'rectangle' || activeDrawingTool === 'speech-bubble-rect') {
                currentRect = { x: startViewportPoint.x, y: startViewportPoint.y, width: 0, height: 0 };
            } else if (activeDrawingTool === 'circle' || activeDrawingTool === 'speech-bubble-circle') {
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
        const currentViewportPoint = osdViewer.viewport.pointFromPixel(event.position);

        if (isDraggingTail && draggedAnnotationId) {
            // Update the tail position of the annotation
            const updatedAnnotations = $currentAnnotations.map(a => {
                if (a.id === draggedAnnotationId) {
                    const selector = a.target.selector.value;
                    return {
                        ...a,
                        target: {
                            ...a.target,
                            selector: {
                                ...a.target.selector,
                                value: { ...selector, tail: { x: currentViewportPoint.x, y: currentViewportPoint.y } }
                            }
                        }
                    };
                }
                return a;
            });
            updateImageAnnotations(updatedAnnotations, false);
            event.preventDefaultAction = true;
            return;
        }

        if (isDraggingShape && draggedAnnotationId && dragStartViewportPoint) {
            const dx = currentViewportPoint.x - dragStartViewportPoint.x;
            const dy = currentViewportPoint.y - dragStartViewportPoint.y;
            
            const updatedAnnotations = $currentAnnotations.map(a => {
                if (a.id === draggedAnnotationId) {
                    const selector = { ...a.target.selector.value };
                    if (selector.shape === 'rectangle' || selector.shape === 'speech-bubble-rect') {
                        selector.x += dx;
                        selector.y += dy;
                    } else if (selector.shape === 'circle' || selector.shape === 'speech-bubble-circle') {
                        selector.cx += dx;
                        selector.cy += dy;
                    } else if (selector.shape === 'polygon') {
                        selector.points = selector.points.map(p => ({ x: p.x + dx, y: p.y + dy }));
                    }
                    
                    if (selector.tail) {
                        selector.tail.x += dx;
                        selector.tail.y += dy;
                    }
                    
                    return {
                        ...a,
                        target: {
                            ...a.target,
                            selector: {
                                ...a.target.selector,
                                value: selector
                            }
                        }
                    };
                }
                return a;
            });
            
            dragStartViewportPoint = currentViewportPoint; // Update for next frame
            updateImageAnnotations(updatedAnnotations, false);
            event.preventDefaultAction = true;
            return;
        }

        if (isDraggingResizeHandle && draggedAnnotationId && dragStartViewportPoint) {
            const dx = currentViewportPoint.x - dragStartViewportPoint.x;
            const dy = currentViewportPoint.y - dragStartViewportPoint.y;

            const updatedAnnotations = $currentAnnotations.map(a => {
                if (a.id === draggedAnnotationId) {
                    const selector = { ...a.target.selector.value };
                    if (selector.shape === 'rectangle' || selector.shape === 'speech-bubble-rect') {
                        if (draggedHandleType === 'nw') {
                            selector.x += dx; selector.y += dy;
                            selector.width -= dx; selector.height -= dy;
                        } else if (draggedHandleType === 'ne') {
                            selector.y += dy;
                            selector.width += dx; selector.height -= dy;
                        } else if (draggedHandleType === 'sw') {
                            selector.x += dx;
                            selector.width -= dx; selector.height += dy;
                        } else if (draggedHandleType === 'se') {
                            selector.width += dx; selector.height += dy;
                        } else if (draggedHandleType === 'n') {
                            selector.y += dy; selector.height -= dy;
                        } else if (draggedHandleType === 's') {
                            selector.height += dy;
                        } else if (draggedHandleType === 'w') {
                            selector.x += dx; selector.width -= dx;
                        } else if (draggedHandleType === 'e') {
                            selector.width += dx;
                        }
                        // Clamp min size
                        selector.width = Math.max(0.001, selector.width);
                        selector.height = Math.max(0.001, selector.height);
                    } else if (selector.shape === 'circle' || selector.shape === 'speech-bubble-circle') {
                        if (draggedHandleType === 'r') {
                            const d_center_x = currentViewportPoint.x - selector.cx;
                            const d_center_y = currentViewportPoint.y - selector.cy;
                            selector.r = Math.sqrt(d_center_x * d_center_x + d_center_y * d_center_y);
                        }
                        selector.r = Math.max(0.0005, selector.r);
                    } else if (selector.shape === 'polygon') {
                        const idx = parseInt(draggedHandleType);
                        if (!isNaN(idx)) {
                            selector.points[idx] = { x: selector.points[idx].x + dx, y: selector.points[idx].y + dy };
                        }
                    }
                    return { ...a, target: { ...a.target, selector: { ...a.target.selector, value: selector } } };
                }
                return a;
            });

            dragStartViewportPoint = currentViewportPoint;
            updateImageAnnotations(updatedAnnotations, false);
            event.preventDefaultAction = true;
            return;
        }

        if (!isDrawing || !activeDrawingTool) return;

        if (activeDrawingTool === 'rectangle' || activeDrawingTool === 'speech-bubble-rect') {
            const x = Math.min(startViewportPoint.x, currentViewportPoint.x);
            const y = Math.min(startViewportPoint.y, currentViewportPoint.y);
            const width = Math.abs(startViewportPoint.x - currentViewportPoint.x);
            const height = Math.abs(startViewportPoint.y - currentViewportPoint.y);
            currentRect = { x, y, width, height };
        } else if (activeDrawingTool === 'circle' || activeDrawingTool === 'speech-bubble-circle') {
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
        if (isDraggingTail || isDraggingShape || isDraggingResizeHandle) {
            isDraggingTail = false;
            isDraggingShape = false;
            isDraggingResizeHandle = false;
            draggedAnnotationId = null;
            draggedHandleType = null;
            dragStartViewportPoint = null;
            await saveImageAnnotations();
            event.preventDefaultAction = true;
            return;
        }

        if (activeDrawingTool !== 'polygon') {
            if (!isDrawing) return;
            isDrawing = false;
        }

        if (!startViewportPoint) return;

        const endViewportPoint = osdViewer.viewport.pointFromPixel(event.position);
        let newAnnotation = null;

        if (activeDrawingTool === 'rectangle' || activeDrawingTool === 'speech-bubble-rect') {
            const viewportRect = new OpenSeadragon.Rect(
                Math.min(startViewportPoint.x, endViewportPoint.x),
                Math.min(startViewportPoint.y, endViewportPoint.y),
                Math.abs(startViewportPoint.x - endViewportPoint.x),
                Math.abs(startViewportPoint.y - endViewportPoint.y)
            );
            if (viewportRect.width > 0.001 && viewportRect.height > 0.001) {
                const isSpeech = activeDrawingTool === 'speech-bubble-rect';
                const shapeData = { ...viewportRect, shape: isSpeech ? 'speech-bubble-rect' : 'rectangle' };
                if (isSpeech) {
                    // Default tail: bottom right, slightly offset
                    shapeData.tail = { x: viewportRect.x + viewportRect.width, y: viewportRect.y + viewportRect.height + 0.05 }; 
                }

                newAnnotation = {
                    id: uuidv4(),
                    type: 'Annotation',
                    target: { selector: { type: 'FragmentSelector', value: shapeData } },
                    body: [{ type: 'Color', value: getSelectedColor(isSpeech), purpose: 'highlighting' }]
                };
            }
        } else if (activeDrawingTool === 'circle' || activeDrawingTool === 'speech-bubble-circle') {
            const dx = endViewportPoint.x - startViewportPoint.x;
            const dy = endViewportPoint.y - startViewportPoint.y;
            const r = Math.sqrt(dx * dx + dy * dy) / 2;
            const cx = startViewportPoint.x + dx / 2;
            const cy = startViewportPoint.y + dy / 2;
            if (r > 0.0005) {
                const isSpeech = activeDrawingTool === 'speech-bubble-circle';
                const shapeData = { cx, cy, r, shape: isSpeech ? 'speech-bubble-circle' : 'circle' };
                 if (isSpeech) {
                    shapeData.tail = { x: cx + r, y: cy + r + 0.05 };
                }

                newAnnotation = {
                    id: uuidv4(),
                    type: 'Annotation',
                    target: { selector: { type: 'FragmentSelector', value: shapeData } },
                    body: [{ type: 'Color', value: getSelectedColor(isSpeech), purpose: 'highlighting' }]
                };
            }
        }

        if (newAnnotation) {
            updateImageAnnotations([...$currentAnnotations, newAnnotation]);
            await saveImageAnnotations();
            
            // Don't open dialog immediately for any shape.
            // Reset states that would otherwise be handled by dialog actions
            annotationBeingEdited = null;
            isEditingExisting = false;
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
            body: [{ type: 'Color', value: getSelectedColor(false), purpose: 'highlighting' }]
        };

        updateImageAnnotations([...$currentAnnotations, newAnnotation]);
        await saveImageAnnotations();

        annotationBeingEdited = null;
        isEditingExisting = false;
        // dialogX = event.position.x;
        // dialogY = event.position.y;
        // showAnnotationCreationDialog = true;

        isDrawing = false;
        currentPolygon = { points: [], previewLine: null, closingPreviewLine: null };
        currentPreviewPolygonPoints = [];
        activeDrawingTool = null;
        startViewportPoint = null;
    }

    function updateAnnotationPositions() {
        if (osdViewer && osdViewerElement) {
            const zoom = osdViewer.viewport.getZoom();
            const containerWidth = osdViewerElement.clientWidth;
            // Target roughly 5px handles on screen (was 8)
            handleRadius = 5 / (zoom * containerWidth);
            // Clamp to reasonable viewport coordinates
            handleRadius = Math.max(0.001, Math.min(0.01, handleRadius));
        }
    }

    function handleAnnotationPointerDown(event, annotation) {
        event.stopPropagation(); // Prevent OSD from panning when clicking a shape
    }

    function handleAnnotationDoubleClick(event, annotation) {
        event.stopPropagation(); // Prevent OSD zoom
        console.log("Annotation double-clicked:", annotation);
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
        const { title, description, color, text } = event.detail;
        if (!annotationBeingEdited) return;

        const newBody = [
            { type: 'Color', value: color, purpose: 'highlighting' }
        ];
        if (title) newBody.push({ type: 'Title', value: title, purpose: 'commenting' });
        if (description) newBody.push({ type: 'Description', value: description, purpose: 'commenting' });
        if (text !== undefined && text !== null && text !== '') {
            newBody.push({ type: 'TextualBody', value: text, purpose: 'content' });
        }

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
    
    function startTailDrag(event, annotationId) {
        // Do not stop propagation, so OSD 'canvas-press' fires and we can use OSD's drag handler
        // event.stopPropagation(); 
        event.preventDefault(); // Stop text selection etc.
        isDraggingTail = true;
        draggedAnnotationId = annotationId;
        selectedAnnotationId = annotationId; // Select the annotation when tail is grabbed
    }

    function startShapeDrag(event, annotationId) {
        // Do not stop propagation, so OSD 'canvas-press' fires
        event.preventDefault();
        isDraggingShape = true;
        draggedAnnotationId = annotationId;
        selectedAnnotationId = annotationId; // Select the annotation on click
        updateAnnotationPositions(); // Ensure handleRadius is updated
        
        // We need the mouse position relative to the OSD canvas to get accurate viewport point
        const viewerRect = osdViewerElement.getBoundingClientRect();
        const mousePoint = new OpenSeadragon.Point(event.clientX - viewerRect.left, event.clientY - viewerRect.top);
        dragStartViewportPoint = osdViewer.viewport.pointFromPixel(mousePoint);
    }

    function startResizeDrag(event, annotationId, handleType) {
        event.preventDefault();
        // Do not stop propagation, so OSD 'canvas-press' fires and we can use OSD's drag handler
        // event.stopPropagation();
        isDraggingResizeHandle = true;
        draggedAnnotationId = annotationId;
        draggedHandleType = handleType;
        updateAnnotationPositions(); // Ensure handleRadius is updated
        
        const viewerRect = osdViewerElement.getBoundingClientRect();
        const mousePoint = new OpenSeadragon.Point(event.clientX - viewerRect.left, event.clientY - viewerRect.top);
        dragStartViewportPoint = osdViewer.viewport.pointFromPixel(mousePoint);
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
            <div class="w-px h-6 bg-gray-300 dark:bg-gray-600 mx-2"></div>
            <button
                class="ui-button-icon"
                class:active={activeDrawingTool === 'speech-bubble-circle'}
                on:click={() => activeDrawingTool = activeDrawingTool === 'speech-bubble-circle' ? null : 'speech-bubble-circle'}
                title="Draw Circular Speech Bubble"
            >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chat" viewBox="0 0 16 16">
                    <path d="M2.678 11.894a1 1 0 0 1 .287.801 11 11 0 0 1-.398 2c1.395-.323 2.247-.697 2.634-.893a1 1 0 0 1 .71-.074A8 8 0 0 0 8 14c3.996 0 7-2.807 7-6s-3.004-6-7-6-7 2.808-7 6c0 1.468.617 2.83 1.678 3.894m-.493 3.905a22 22 0 0 1-.713.129c-.2.032-.352-.176-.273-.362a10 10 0 0 0 .244-.637l.003-.01c.248-.72.45-1.548.524-2.319C.743 11.37 0 9.76 0 8c0-3.866 3.582-7 8-7s8 3.134 8 7-3.582 7-8 7a9 9 0 0 1-2.347-.306c-.52.263-1.639.742-3.468 1.105"/>
                </svg>
            </button>

            <button
                class="ui-button-icon"
                class:active={activeDrawingTool === 'speech-bubble-rect'}
                on:click={() => activeDrawingTool = activeDrawingTool === 'speech-bubble-rect' ? null : 'speech-bubble-rect'}
                title="Draw Rectangular Speech Bubble"
            >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chat-left" viewBox="0 0 16 16">
                    <path d="M14 1a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1H4.414A2 2 0 0 0 3 11.586l-2 2V2a1 1 0 0 1 1-1zM2 0a2 2 0 0 0-2 2v12.793a.5.5 0 0 0 .854.353l2.853-2.853A1 1 0 0 1 4.414 12H14a2 2 0 0 0 2-2V2a2 2 0 0 0-2-2z"/>
                </svg>
            </button>
            <div class="w-px h-6 bg-gray-300 dark:bg-gray-600 mx-2"></div>
            <div class="flex items-center space-x-1.5">
                {#each baseColors as color, i}
                    <button
                        class="w-5 h-5 rounded-full border border-gray-300 dark:border-gray-500 transition-transform hover:scale-110 shadow-sm"
                        class:ring-2={selectedBaseColorIndex === i}
                        class:ring-blue-500={selectedBaseColorIndex === i}
                        style="background-color: rgba({color.rgb}, 1);"
                        on:click={() => selectedBaseColorIndex = i}
                        title={color.name}
                    ></button>
                {/each}
            </div>
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
                {@const textBody = annotation.body.find(b => b.purpose === 'content' && b.type === 'TextualBody')}
                {@const fillColor = colorBody ? colorBody.value : 'rgba(255, 242, 117, 0.5)'}
                {@const strokeColor = adjustOpacity(fillColor, 1)}

                {#if shapeData.shape === 'rectangle'}
                    <rect
                        x={shapeData.x}
                        y={shapeData.y}
                        width={shapeData.width}
                        height={shapeData.height}
                        fill={fillColor}
                        stroke={selectedAnnotationId === annotation.id ? 'blue' : strokeColor}
                        stroke-width={selectedAnnotationId === annotation.id ? '2px' : '1px'}
                        stroke-dasharray={selectedAnnotationId === annotation.id ? '0.01, 0.005' : 'none'}
                        vector-effect="non-scaling-stroke"
                        class="pointer-events-auto cursor-pointer annotation-shape"
                        data-annotation-id={annotation.id}
                        on:pointerdown={(e) => startShapeDrag(e, annotation.id)}
                        on:dblclick={(e) => handleAnnotationDoubleClick(e, annotation)}
                    />
                    {#if selectedAnnotationId === annotation.id}
                        <!-- 8 handles for rectangle -->
                        <!-- Corners -->
                        <circle cx={shapeData.x} cy={shapeData.y} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-nw-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'nw')} />
                        <circle cx={shapeData.x + shapeData.width} cy={shapeData.y} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-ne-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'ne')} />
                        <circle cx={shapeData.x} cy={shapeData.y + shapeData.height} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-sw-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'sw')} />
                        <circle cx={shapeData.x + shapeData.width} cy={shapeData.y + shapeData.height} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-se-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'se')} />
                        <!-- Mid-points -->
                        <circle cx={shapeData.x + shapeData.width / 2} cy={shapeData.y} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-n-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'n')} />
                        <circle cx={shapeData.x + shapeData.width / 2} cy={shapeData.y + shapeData.height} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-s-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 's')} />
                        <circle cx={shapeData.x} cy={shapeData.y + shapeData.height / 2} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-w-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'w')} />
                        <circle cx={shapeData.x + shapeData.width} cy={shapeData.y + shapeData.height / 2} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-e-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'e')} />
                    {/if}
                {:else if shapeData.shape === 'speech-bubble-rect'}
                    <rect
                        x={shapeData.x}
                        y={shapeData.y}
                        width={shapeData.width}
                        height={shapeData.height}
                        fill={fillColor}
                        stroke={selectedAnnotationId === annotation.id ? 'blue' : strokeColor}
                        stroke-width={selectedAnnotationId === annotation.id ? '2px' : '1px'}
                        stroke-dasharray={selectedAnnotationId === annotation.id ? '0.01, 0.005' : 'none'}
                        vector-effect="non-scaling-stroke"
                        class="pointer-events-auto cursor-pointer annotation-shape"
                        data-annotation-id={annotation.id}
                        on:pointerdown={(e) => startShapeDrag(e, annotation.id)}
                        on:dblclick={(e) => handleAnnotationDoubleClick(e, annotation)}
                    />
                     <path
                        d={getBubblePath(shapeData, false)}
                        fill={fillColor}
                        stroke={selectedAnnotationId === annotation.id ? 'blue' : strokeColor}
                        stroke-width={selectedAnnotationId === annotation.id ? '2px' : '1px'}
                        vector-effect="non-scaling-stroke"
                        class="pointer-events-auto cursor-pointer annotation-shape"
                        data-annotation-id={annotation.id}
                        on:pointerdown={(e) => startShapeDrag(e, annotation.id)}
                        on:dblclick={(e) => handleAnnotationDoubleClick(e, annotation)}
                    />
                    {#if textBody}
                        <foreignObject x={shapeData.x} y={shapeData.y} width={shapeData.width} height={shapeData.height} class="pointer-events-none">
                            <div class="w-full h-full flex items-center justify-center text-center p-1 overflow-hidden text-xs select-none" style="color: black;">
                                {textBody.value}
                            </div>
                        </foreignObject>
                    {/if}
                    <circle
                        cx={shapeData.tail.x}
                        cy={shapeData.tail.y}
                        r={handleRadius * 1.2}
                        fill="white"
                        stroke="black"
                        stroke-width="0.001"
                        class="pointer-events-auto cursor-pointer hover:fill-blue-500"
                        on:pointerdown={(e) => startTailDrag(e, annotation.id)}
                    />
                    {#if selectedAnnotationId === annotation.id}
                        <!-- 8 handles for speech rect -->
                        <circle cx={shapeData.x} cy={shapeData.y} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-nw-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'nw')} />
                        <circle cx={shapeData.x + shapeData.width} cy={shapeData.y} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-ne-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'ne')} />
                        <circle cx={shapeData.x} cy={shapeData.y + shapeData.height} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-sw-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'sw')} />
                        <circle cx={shapeData.x + shapeData.width} cy={shapeData.y + shapeData.height} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-se-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'se')} />
                        <circle cx={shapeData.x + shapeData.width / 2} cy={shapeData.y} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-n-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'n')} />
                        <circle cx={shapeData.x + shapeData.width / 2} cy={shapeData.y + shapeData.height} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-s-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 's')} />
                        <circle cx={shapeData.x} cy={shapeData.y + shapeData.height / 2} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-w-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'w')} />
                        <circle cx={shapeData.x + shapeData.width} cy={shapeData.y + shapeData.height / 2} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-e-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'e')} />
                    {/if}

                {:else if shapeData.shape === 'circle'}
                    <circle
                        cx={shapeData.cx}
                        cy={shapeData.cy}
                        r={shapeData.r}
                        fill={fillColor}
                        stroke={selectedAnnotationId === annotation.id ? 'blue' : strokeColor}
                        stroke-width={selectedAnnotationId === annotation.id ? '2px' : '1px'}
                        stroke-dasharray={selectedAnnotationId === annotation.id ? '0.01, 0.005' : 'none'}
                        vector-effect="non-scaling-stroke"
                        class="pointer-events-auto cursor-pointer annotation-shape"
                        data-annotation-id={annotation.id}
                        on:pointerdown={(e) => startShapeDrag(e, annotation.id)}
                        on:dblclick={(e) => handleAnnotationDoubleClick(e, annotation)}
                    />
                    {#if selectedAnnotationId === annotation.id}
                        <!-- 4 radius handles -->
                        <circle cx={shapeData.cx + shapeData.r} cy={shapeData.cy} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-ew-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                        <circle cx={shapeData.cx - shapeData.r} cy={shapeData.cy} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-ew-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                        <circle cx={shapeData.cx} cy={shapeData.cy + shapeData.r} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-ns-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                        <circle cx={shapeData.cx} cy={shapeData.cy - shapeData.r} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-ns-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                    {/if}
                {:else if shapeData.shape === 'speech-bubble-circle'}
                     <path
                        d={getBubblePath(shapeData, true)}
                        fill={fillColor}
                        stroke={selectedAnnotationId === annotation.id ? 'blue' : strokeColor}
                        stroke-width={selectedAnnotationId === annotation.id ? '2px' : '1px'}
                        vector-effect="non-scaling-stroke"
                        class="pointer-events-auto cursor-pointer annotation-shape"
                        data-annotation-id={annotation.id}
                        on:pointerdown={(e) => startShapeDrag(e, annotation.id)}
                        on:dblclick={(e) => handleAnnotationDoubleClick(e, annotation)}
                    />
                    {#if textBody}
                        <foreignObject x={shapeData.cx - shapeData.r} y={shapeData.cy - shapeData.r} width={shapeData.r * 2} height={shapeData.r * 2} class="pointer-events-none">
                            <div class="w-full h-full flex items-center justify-center text-center p-2 overflow-hidden text-xs select-none" style="color: black;">
                                {textBody.value}
                            </div>
                        </foreignObject>
                    {/if}
                    <circle
                        cx={shapeData.tail.x}
                        cy={shapeData.tail.y}
                        r={handleRadius * 1.2}
                        fill="white"
                        stroke="black"
                        stroke-width="0.001"
                        class="pointer-events-auto cursor-pointer hover:fill-blue-500"
                        on:pointerdown={(e) => startTailDrag(e, annotation.id)}
                    />
                    {#if selectedAnnotationId === annotation.id}
                        <circle cx={shapeData.cx + shapeData.r} cy={shapeData.cy} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-ew-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                        <circle cx={shapeData.cx - shapeData.r} cy={shapeData.cy} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-ew-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                        <circle cx={shapeData.cx} cy={shapeData.cy + shapeData.r} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-ns-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                        <circle cx={shapeData.cx} cy={shapeData.cy - shapeData.r} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-ns-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                    {/if}

                {:else if shapeData.shape === 'polygon'}
                    <polygon
                        points={shapeData.points.map(p => `${p.x},${p.y}`).join(' ')}
                        fill={fillColor}
                        stroke={selectedAnnotationId === annotation.id ? 'blue' : strokeColor}
                        stroke-width={selectedAnnotationId === annotation.id ? '2px' : '0.002'}
                        stroke-dasharray={selectedAnnotationId === annotation.id ? '0.01, 0.005' : 'none'}
                        class="pointer-events-auto cursor-pointer annotation-shape"
                        data-annotation-id={annotation.id}
                        on:pointerdown={(e) => startShapeDrag(e, annotation.id)}
                        on:dblclick={(e) => handleAnnotationDoubleClick(e, annotation)}
                    />
                    {#if selectedAnnotationId === annotation.id}
                        {#each shapeData.points as point, i}
                            <circle cx={point.x} cy={point.y} r={handleRadius} fill="white" stroke="blue" stroke-width="0.001" class="pointer-events-auto cursor-move" on:pointerdown={(e) => startResizeDrag(e, annotation.id, i.toString())} />
                        {/each}
                    {/if}
                {/if}
            {/each}
            {#if isDrawing && (activeDrawingTool === 'rectangle' || activeDrawingTool === 'speech-bubble-rect') && currentRect}
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
            {:else if isDrawing && (activeDrawingTool === 'circle' || activeDrawingTool === 'speech-bubble-circle') && currentCircle}
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
                    initialText={annotationBeingEdited?.body?.find(b => b.type === 'TextualBody' && b.purpose === 'content')?.value || (annotationBeingEdited?.target?.selector?.value?.shape.startsWith('speech-bubble') ? '' : null)}
                    initialColor={annotationBeingEdited?.body?.find(b => b.type === 'Color')?.value || 'rgba(255, 242, 117, 0.5)'}
                    isEditing={isEditingExisting}
                    useSolidColors={annotationBeingEdited?.target?.selector?.value?.shape.startsWith('speech-bubble')}
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
