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
    let currentAssetUrl = null;
    let imgAspectRatio = 1;

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
        { name: 'White', rgb: '255, 255, 255' },
        { name: 'Black', rgb: '0, 0, 0' },
        { name: 'Yellow', rgb: '255, 242, 117' },
        { name: 'Green', rgb: '168, 255, 158' },
        { name: 'Blue', rgb: '174, 239, 255' },
        { name: 'Pink', rgb: '255, 176, 207' },
        { name: 'Purple', rgb: '208, 160, 255' },
        { name: 'Transparent', rgb: 'transparent' },
    ];

    function getSelectedColor(isSpeechBubble) {
        const base = baseColors[selectedBaseColorIndex];
        if (base.rgb === 'transparent') return 'transparent';
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
    let isDraggingTailWidth = false;
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

    // Helper to generate speech bubble info including path and base points
    function getBubbleTailInfo(shapeData, isCircle, S = 1000) {
        const { x, y, width, height, cx, cy, r, tail, tailWidth, tailStyle, tailFlipped, rounded, isOval } = shapeData;
        if (!tail) return null;

        let tx = tail.x * S;
        let ty = tail.y * S;

        let center, bounds;
        if (isCircle) {
            center = { x: cx * S, y: cy * S };
        } else {
            center = { x: (x + width / 2) * S, y: (y + height / 2) * S };
            bounds = { x: x * S, y: y * S, width: width * S, height: height * S };
        }

        // Vector from center to tail tip
        let dx = tx - center.x;
        let dy = ty - center.y;
        let len = Math.sqrt(dx * dx + dy * dy);
        
        // Find intersection with shape boundary
        let t;
        const dir_check = { x: dx / (len || 1), y: dy / (len || 1) };

        if (isCircle) {
            if (isOval) {
                // For oval, t depends on angle
                const rx = r * S * 1.5;
                const ry = r * S;
                const angle = Math.atan2(dy, dx);
                t = (rx * ry) / Math.sqrt(ry * ry * Math.pow(Math.cos(angle), 2) + rx * rx * Math.pow(Math.sin(angle), 2));
            } else {
                t = r * S;
            }
        } else {
            const tValues = [];
            if (dir_check.x > 0) tValues.push((bounds.x + bounds.width - center.x) / dir_check.x);
            else if (dir_check.x < 0) tValues.push((bounds.x - center.x) / dir_check.x);
            if (dir_check.y > 0) tValues.push((bounds.y + bounds.height - center.y) / dir_check.y);
            else if (dir_check.y < 0) tValues.push((bounds.y - center.y) / dir_check.y);
            t = Math.min(...tValues.filter(v => v > 0));
        }

        // Ensure tail tip is outside
        const minLen = t + (0.01 * S);
        if (len < minLen) {
            const factor = minLen / (len || 0.001);
            tx = center.x + dx * factor;
            ty = center.y + dy * factor;
            dx = tx - center.x;
            dy = ty - center.y;
            len = minLen;
        }

        const dir = { x: dx / len, y: dy / len };

        if (isCircle) {
            const angleWidth = (tailWidth || 15) * (Math.PI / 180);
            const centralAngle = Math.atan2(dir.y, dir.x);
            
            const a1 = centralAngle - angleWidth;
            const a2 = centralAngle + angleWidth;

            const rx = isOval ? r * S * 1.5 : r * S;
            const ry = r * S;

            const b1 = { x: center.x + rx * Math.cos(a1), y: center.y + ry * Math.sin(a1) };
            const b2 = { x: center.x + rx * Math.cos(a2), y: center.y + ry * Math.sin(a2) };
            
            let path;
            if (tailStyle === 'curved') {
                const bc = { x: center.x + dir.x * t, y: center.y + dir.y * t };
                const spineV = { x: tx - bc.x, y: ty - bc.y };
                const sign = tailFlipped ? -1 : 1;
                const perp = { x: sign * -spineV.y * 0.25, y: sign * spineV.x * 0.25 };
                const cp = { x: bc.x + spineV.x * 0.5 + perp.x, y: bc.y + spineV.y * 0.5 + perp.y };
                path = `M ${b1.x} ${b1.y} Q ${cp.x} ${cp.y} ${tx} ${ty} Q ${cp.x} ${cp.y} ${b2.x} ${b2.y} A ${rx} ${ry} 0 1 1 ${b1.x} ${b1.y} Z`;
            } else {
                path = `M ${b1.x} ${b1.y} L ${tx} ${ty} L ${b2.x} ${b2.y} A ${rx} ${ry} 0 1 1 ${b1.x} ${b1.y} Z`;
            }

            return { path, b1, b2, center, side: 'circle' };
        } else {
            // Robust side-based rectangle logic
            let side = "";
            const absDirX = Math.abs(dir.x * bounds.height);
            const absDirY = Math.abs(dir.y * bounds.width);
            if (absDirX > absDirY) side = dir.x > 0 ? "right" : "left";
            else side = dir.y > 0 ? "bottom" : "top";

            const sizeParam = Math.min(bounds.width, bounds.height);
            const baseHalfWidth = tailWidth !== undefined ? (tailWidth * S / 2) : Math.max(sizeParam * 0.1, 0.01 * S);
            const R = rounded ? Math.min(sizeParam * 0.2, 0.04 * S) : 0;

            const TL = { x: bounds.x, y: bounds.y };
            const TR = { x: bounds.x + bounds.width, y: bounds.y };
            const BR = { x: bounds.x + bounds.width, y: bounds.y + bounds.height };
            const BL = { x: bounds.x, y: bounds.y + bounds.height };

            let b1, b2, path, bx, by, bc;
            
            const arc = (p, r, sweep = 1) => r > 0 ? `A ${r} ${r} 0 0 ${sweep} ${p.x} ${p.y}` : `L ${p.x} ${p.y}`;

            if (side === "top") {
                bx = Math.max(TL.x + R + baseHalfWidth, Math.min(TR.x - R - baseHalfWidth, center.x + (dir.x * t)));
                b1 = { x: bx - baseHalfWidth, y: TL.y };
                b2 = { x: bx + baseHalfWidth, y: TL.y };
                bc = { x: bx, y: TL.y };
                const curvePart = tailStyle === 'curved' ? 
                    (() => {
                        const spineV = { x: tx - bc.x, y: ty - bc.y };
                        const sign = tailFlipped ? -1 : 1;
                        const perpV = { x: sign * -spineV.y * 0.25, y: sign * spineV.x * 0.25 };
                        const cp = { x: bc.x + spineV.x * 0.5 + perpV.x, y: bc.y + spineV.y * 0.5 + perpV.y };
                        return `Q ${cp.x} ${cp.y} ${tx} ${ty} Q ${cp.x} ${cp.y} ${b2.x} ${b2.y}`;
                    })() : `L ${tx} ${ty} L ${b2.x} ${b2.y}`;

                path = `M ${TL.x + R} ${TL.y} L ${b1.x} ${b1.y} ${curvePart} L ${TR.x - R} ${TR.y} ${arc({x: TR.x, y: TR.y + R}, R)} L ${BR.x} ${BR.y - R} ${arc({x: BR.x - R, y: BR.y}, R)} L ${BL.x + R} ${BL.y} ${arc({x: BL.x, y: BL.y - R}, R)} L ${TL.x} ${TL.y + R} ${arc({x: TL.x + R, y: TL.y}, R)} Z`;
            } else if (side === "right") {
                by = Math.max(TR.y + R + baseHalfWidth, Math.min(BR.y - R - baseHalfWidth, center.y + (dir.y * t)));
                b1 = { x: TR.x, y: by - baseHalfWidth };
                b2 = { x: TR.x, y: by + baseHalfWidth };
                bc = { x: TR.x, y: by };
                const curvePart = tailStyle === 'curved' ? 
                    (() => {
                        const spineV = { x: tx - bc.x, y: ty - bc.y };
                        const sign = tailFlipped ? -1 : 1;
                        const perpV = { x: sign * -spineV.y * 0.25, y: sign * spineV.x * 0.25 };
                        const cp = { x: bc.x + spineV.x * 0.5 + perpV.x, y: bc.y + spineV.y * 0.5 + perpV.y };
                        return `Q ${cp.x} ${cp.y} ${tx} ${ty} Q ${cp.x} ${cp.y} ${b2.x} ${b2.y}`;
                    })() : `L ${tx} ${ty} L ${b2.x} ${b2.y}`;

                path = `M ${TL.x + R} ${TL.y} L ${TR.x - R} ${TR.y} ${arc({x: TR.x, y: TR.y + R}, R)} L ${b1.x} ${b1.y} ${curvePart} L ${BR.x} ${BR.y - R} ${arc({x: BR.x - R, y: BR.y}, R)} L ${BL.x + R} ${BL.y} ${arc({x: BL.x, y: BL.y - R}, R)} L ${TL.x} ${TL.y + R} ${arc({x: TL.x + R, y: TL.y}, R)} Z`;
            } else if (side === "bottom") {
                bx = Math.max(BL.x + R + baseHalfWidth, Math.min(BR.x - R - baseHalfWidth, center.x + (dir.x * t)));
                b1 = { x: bx + baseHalfWidth, y: BR.y };
                b2 = { x: bx - baseHalfWidth, y: BR.y };
                bc = { x: bx, y: BR.y };
                const curvePart = tailStyle === 'curved' ? 
                    (() => {
                        const spineV = { x: tx - bc.x, y: ty - bc.y };
                        const sign = tailFlipped ? -1 : 1;
                        const perpV = { x: sign * -spineV.y * 0.25, y: sign * spineV.x * 0.25 };
                        const cp = { x: bc.x + spineV.x * 0.5 + perpV.x, y: bc.y + spineV.y * 0.5 + perpV.y };
                        return `Q ${cp.x} ${cp.y} ${tx} ${ty} Q ${cp.x} ${cp.y} ${b2.x} ${b2.y}`;
                    })() : `L ${tx} ${ty} L ${b2.x} ${b2.y}`;

                path = `M ${TL.x + R} ${TL.y} L ${TR.x - R} ${TR.y} ${arc({x: TR.x, y: TR.y + R}, R)} L ${BR.x} ${BR.y - R} ${arc({x: BR.x - R, y: BR.y}, R)} L ${b1.x} ${b1.y} ${curvePart} L ${BL.x + R} ${BL.y} ${arc({x: BL.x, y: BL.y - R}, R)} L ${TL.x} ${TL.y + R} ${arc({x: TL.x + R, y: TL.y}, R)} Z`;
            } else { // left
                by = Math.max(TL.y + R + baseHalfWidth, Math.min(BL.y - R - baseHalfWidth, center.y + (dir.y * t)));
                b1 = { x: TL.x, y: by + baseHalfWidth };
                b2 = { x: TL.x, y: by - baseHalfWidth };
                bc = { x: TL.x, y: by };
                const curvePart = tailStyle === 'curved' ? 
                    (() => {
                        const spineV = { x: tx - bc.x, y: ty - bc.y };
                        const sign = tailFlipped ? -1 : 1;
                        const perpV = { x: sign * -spineV.y * 0.25, y: sign * spineV.x * 0.25 };
                        const cp = { x: bc.x + spineV.x * 0.5 + perpV.x, y: bc.y + spineV.y * 0.5 + perpV.y };
                        return `Q ${cp.x} ${cp.y} ${tx} ${ty} Q ${cp.x} ${cp.y} ${b1.x} ${b1.y}`;
                    })() : `L ${tx} ${ty} L ${b1.x} ${b1.y}`;

                path = `M ${TL.x + R} ${TL.y} L ${TR.x - R} ${TR.y} ${arc({x: TR.x, y: TR.y + R}, R)} L ${BR.x} ${BR.y - R} ${arc({x: BR.x - R, y: BR.y}, R)} L ${BL.x + R} ${BL.y} ${arc({x: BL.x, y: BL.y - R}, R)} L ${b1.x} ${b1.y} ${curvePart} L ${TL.x} ${TL.y + R} ${arc({x: TL.x + R, y: TL.y}, R)} Z`;
            }
            return { path, b1, b2, side, baseCenter: bc };
        }
    }

    // Helper to generate speech bubble path
    function getBubblePath(shapeData, isCircle) {
        const info = getBubbleTailInfo(shapeData, isCircle);
        return info ? info.path : '';
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
            currentAssetUrl = assetUrl;
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
                if (osdViewer && osdViewer.world.getItemCount() > 0) {
                    const size = osdViewer.world.getItemAt(0).getContentSize();
                    if (size.x > 0) imgAspectRatio = size.y / size.x;
                }
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
            if (activeDrawingTool === 'rectangle' || activeDrawingTool === 'speech-bubble-rect' || activeDrawingTool === 'text-area' || activeDrawingTool === 'censored') {
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

        if (isDraggingTailWidth && draggedAnnotationId) {
            const updatedAnnotations = $currentAnnotations.map(a => {
                if (a.id === draggedAnnotationId) {
                    const selector = { ...a.target.selector.value };
                    const isCircle = selector.shape === 'speech-bubble-circle';
                    const info = getBubbleTailInfo(selector, isCircle, 1);
                    if (info) {
                        if (isCircle) {
                            const dx = currentViewportPoint.x - selector.cx;
                            const dy = currentViewportPoint.y - selector.cy;
                            const mouseAngle = Math.atan2(dy, dx);
                            const tailAngle = Math.atan2(selector.tail.y - selector.cy, selector.tail.x - selector.cx);
                            let diff = Math.abs(mouseAngle - tailAngle);
                            if (diff > Math.PI) diff = 2 * Math.PI - diff;
                            selector.tailWidth = diff * (180 / Math.PI);
                        } else if (info.baseCenter) {
                            let newHalfWidth;
                            if (info.side === 'top' || info.side === 'bottom') {
                                newHalfWidth = Math.abs(currentViewportPoint.x - info.baseCenter.x);
                            } else {
                                newHalfWidth = Math.abs(currentViewportPoint.y - info.baseCenter.y);
                            }
                            selector.tailWidth = newHalfWidth * 2;
                        }
                    }
                    return { ...a, target: { ...a.target, selector: { ...a.target.selector, value: selector } } };
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
                    if (selector.shape === 'rectangle' || selector.shape === 'speech-bubble-rect' || selector.shape === 'text-area' || selector.shape === 'censored') {
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
                    if (selector.shape === 'rectangle' || selector.shape === 'speech-bubble-rect' || selector.shape === 'text-area' || selector.shape === 'censored') {
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

        if (activeDrawingTool === 'rectangle' || activeDrawingTool === 'speech-bubble-rect' || activeDrawingTool === 'text-area' || activeDrawingTool === 'censored') {
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
        if (isDraggingTail || isDraggingTailWidth || isDraggingShape || isDraggingResizeHandle) {
            isDraggingTail = false;
            isDraggingTailWidth = false;
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

        if (activeDrawingTool === 'rectangle' || activeDrawingTool === 'speech-bubble-rect' || activeDrawingTool === 'text-area' || activeDrawingTool === 'censored') {
            const viewportRect = new OpenSeadragon.Rect(
                Math.min(startViewportPoint.x, endViewportPoint.x),
                Math.min(startViewportPoint.y, endViewportPoint.y),
                Math.abs(startViewportPoint.x - endViewportPoint.x),
                Math.abs(startViewportPoint.y - endViewportPoint.y)
            );
            if (viewportRect.width > 0.001 && viewportRect.height > 0.001) {
                const isSpeech = activeDrawingTool === 'speech-bubble-rect';
                const isTextArea = activeDrawingTool === 'text-area';
                const isCensored = activeDrawingTool === 'censored';
                let shapeType = 'rectangle';
                if (isSpeech) shapeType = 'speech-bubble-rect';
                else if (isTextArea) shapeType = 'text-area';
                else if (isCensored) shapeType = 'censored';

                const shapeData = { ...viewportRect, shape: shapeType };
                if (isSpeech) {
                    // Default tail: bottom right, slightly offset
                    shapeData.tail = { x: viewportRect.x + viewportRect.width, y: viewportRect.y + viewportRect.height + 0.05 }; 
                    shapeData.tailWidth = 0.03;
                }

                newAnnotation = {
                    id: uuidv4(),
                    type: 'Annotation',
                    target: { selector: { type: 'FragmentSelector', value: shapeData } },
                    body: [{ type: 'Color', value: isCensored ? 'url(#censoredPattern)' : getSelectedColor(isSpeech || isTextArea), purpose: 'highlighting' }]
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
                    shapeData.tailWidth = 15;
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
            
            const shapeType = newAnnotation.target.selector.value.shape;
            const isSpeechBubble = shapeType.startsWith('speech-bubble');
            const isTextArea = shapeType === 'text-area';
            const isCensored = shapeType === 'censored';
            
            if (!isSpeechBubble && !isTextArea && !isCensored) {
                annotationBeingEdited = newAnnotation;
                isEditingExisting = false;
                // event.position is already relative to the viewer element
                dialogX = event.position.x;
                dialogY = event.position.y;
                showAnnotationCreationDialog = true;
                activeDrawingTool = null; // Deactivate tool
            } else {
                // For speech bubbles and text areas, don't open dialog immediately.
                annotationBeingEdited = null;
                isEditingExisting = false;
                activeDrawingTool = null; // Deactivate tool
            }
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
        activeDrawingTool = null; // Deactivate tool
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
        const { title, description, color, text, textColor, fontSize, borderColor, borderSize, shape, tailStyle, tailFlipped, rounded, isOval } = event.detail;
        if (!annotationBeingEdited) return;

        let updatedSelector = { ...annotationBeingEdited.target.selector.value };
        
        if (tailStyle) {
            updatedSelector.tailStyle = tailStyle;
        }
        if (tailFlipped !== undefined) {
            updatedSelector.tailFlipped = tailFlipped;
        }
        if (rounded !== undefined) {
            updatedSelector.rounded = rounded;
        }
        if (isOval !== undefined) {
            updatedSelector.isOval = isOval;
        }

        // Handle shape change logic
        if (shape && shape !== updatedSelector.shape) {
            const oldShape = updatedSelector.shape;
            if (shape === 'circle' && (oldShape === 'rectangle' || oldShape === 'text-area' || oldShape === 'censored')) {
                // Convert rect to circle
                const cx = updatedSelector.x + updatedSelector.width / 2;
                const cy = updatedSelector.y + updatedSelector.height / 2;
                const r = Math.min(updatedSelector.width, updatedSelector.height) / 2;
                updatedSelector = { shape: oldShape === 'censored' ? 'censored' : 'circle', cx, cy, r };
                // Wait, if it was censored it should stay censored shape but maybe user wants circular censored? 
                // Let's assume if they picked 'circle' in dialog, they want the circle version of current tool
                if (oldShape === 'censored') updatedSelector.shape = 'censored-circle'; // Need to add this type
                else if (oldShape === 'text-area') updatedSelector.shape = 'text-area-circle'; // Add this too
                else updatedSelector.shape = 'circle';
            } else if (shape === 'rectangle' && (oldShape === 'circle' || oldShape === 'speech-bubble-circle' || oldShape.endsWith('-circle'))) {
                // Convert circle to rect
                const width = updatedSelector.r * 2;
                const height = updatedSelector.r * 2;
                const x = updatedSelector.cx - updatedSelector.r;
                const y = updatedSelector.cy - updatedSelector.r;
                updatedSelector = { shape: 'rectangle', x, y, width, height };
                if (oldShape === 'censored-circle' || oldShape === 'censored') updatedSelector.shape = 'censored';
                else if (oldShape === 'text-area-circle' || oldShape === 'text-area') updatedSelector.shape = 'text-area';
                else if (oldShape === 'speech-bubble-circle') updatedSelector.shape = 'speech-bubble-rect';
                else updatedSelector.shape = 'rectangle';
            }
        }

        const newBody = [
            { type: 'Color', value: color, purpose: 'highlighting' }
        ];
        if (title) newBody.push({ type: 'Title', value: title, purpose: 'commenting' });
        if (description) newBody.push({ type: 'Description', value: description, purpose: 'commenting' });
        if (text !== undefined && text !== null && text !== '') {
            newBody.push({ type: 'TextualBody', value: text, purpose: 'content' });
        }
        if (textColor) newBody.push({ type: 'TextColor', value: textColor, purpose: 'rendering' });
        if (fontSize) newBody.push({ type: 'FontSize', value: fontSize, purpose: 'rendering' });
        if (borderColor) newBody.push({ type: 'BorderColor', value: borderColor, purpose: 'rendering' });
        if (borderSize) newBody.push({ type: 'BorderSize', value: borderSize, purpose: 'rendering' });

        const updatedAnnotation = {
            ...annotationBeingEdited,
            target: {
                ...annotationBeingEdited.target,
                selector: {
                    ...annotationBeingEdited.target.selector,
                    value: updatedSelector
                }
            },
            body: newBody,
        };

        const updatedAnnotations = $currentAnnotations.map(a =>
            a.id === updatedAnnotation.id ? updatedAnnotation : a
        );

        updateImageAnnotations(updatedAnnotations);
        await saveImageAnnotations();

        closeAnnotationDialog();
    }
    
    function startTailWidthDrag(event, annotationId) {
        event.preventDefault();
        isDraggingTailWidth = true;
        draggedAnnotationId = annotationId;
        selectedAnnotationId = annotationId;
        const viewerRect = osdViewerElement.getBoundingClientRect();
        const mousePoint = new OpenSeadragon.Point(event.clientX - viewerRect.left, event.clientY - viewerRect.top);
        dragStartViewportPoint = osdViewer.viewport.pointFromPixel(mousePoint);
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

            <button
                class="ui-button-icon"
                class:active={activeDrawingTool === 'text-area'}
                on:click={() => activeDrawingTool = (activeDrawingTool === 'text-area' ? null : 'text-area')}
                title="Draw Text Area"
            >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-textarea-t" viewBox="0 0 16 16">
                    <path d="M1.5 2.5A1.5 1.5 0 0 1 3 1h10a1.5 1.5 0 0 1 1.5 1.5v3.563a2 2 0 0 1 0 3.874V13.5A1.5 1.5 0 0 1 13 15H3a1.5 1.5 0 0 1-1.5-1.5V9.937a2 2 0 0 1 0-3.874zm1 3.563a2 2 0 0 1 0 3.874V13.5a.5.5 0 0 0 .5.5h10a.5.5 0 0 0 .5-.5V9.937a2 2 0 0 1 0-3.874V2.5A.5.5 0 0 0 13 2H3a.5.5 0 0 0-.5.5zM2 7a1 1 0 1 0 0 2 1 1 0 0 0 0-2m12 0a1 1 0 1 0 0 2 1 1 0 0 0 0-2"/>
                    <path d="M11.434 4H4.566L4.5 5.994h.386c.21-1.252.612-1.446 2.173-1.495l.343-.011v6.343c0 .537-.116.665-1.049.748V12h3.294v-.421c-.938-.083-1.054-.21-1.054-.748V4.488l.348.01c1.56.05 1.963.244 2.173 1.496h.386z"/>
                </svg>
            </button>

            <button
                class="ui-button-icon"
                class:active={activeDrawingTool === 'censored'}
                on:click={() => activeDrawingTool = (activeDrawingTool === 'censored' ? null : 'censored')}
                title="Anonymise (Pixelate)"
            >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-incognito" viewBox="0 0 16 16">
                    <path fill-rule="evenodd" d="m4.736 1.968-.892 3.269-.014.058C2.113 5.568 1 6.006 1 6.5 1 7.328 4.134 8 8 8s7-.672 7-1.5c0-.494-1.113-.932-2.83-1.205l-.014-.058-.892-3.27c-.146-.533-.698-.849-1.239-.734C9.411 1.363 8.62 1.5 8 1.5s-1.411-.136-2.025-.267c-.541-.115-1.093.2-1.239.735m.015 3.867a.25.25 0 0 1 .274-.224c.9.092 1.91.143 2.975.143a30 30 0 0 0 2.975-.143.25.25 0 0 1 .05.498c-.918.093-1.944.145-3.025.145s-2.107-.052-3.025-.145a.25.25 0 0 1-.224-.274M3.5 10h2a.5.5 0 0 1 .5.5v1a1.5 1.5 0 0 1-3 0v-1a.5.5 0 0 1 .5-.5m-1.5.5q.001-.264.085-.5H2a.5.5 0 0 1 0-1h3.5a1.5 1.5 0 0 1 1.488 1.312 3.5 3.5 0 0 1 2.024 0A1.5 1.5 0 0 1 10.5 9H14a.5.5 0 0 1 0 1h-.085q.084.236.085.5v1a2.5 2.5 0 0 1-5 0v-.14l-.21-.07a2.5 2.5 0 0 0-1.58 0l-.21.07v.14a2.5 2.5 0 0 1-5 0zm8.5-.5h2a.5.5 0 0 1 .5.5v1a1.5 1.5 0 0 1-3 0v-1a.5.5 0 0 1 .5-.5"/>
                </svg>
            </button>


            <div class="w-px h-6 bg-gray-300 dark:bg-gray-600 mx-2"></div>
            <div class="flex items-center space-x-1.5">
                {#each baseColors as color, i}
                    <button
                        class="w-5 h-5 rounded-full border border-gray-300 dark:border-gray-500 transition-transform hover:scale-110 shadow-sm"
                        class:ring-2={selectedBaseColorIndex === i}
                        class:ring-blue-500={selectedBaseColorIndex === i}
                        style="background: {color.rgb === 'transparent' ? 'linear-gradient(45deg, rgba(255,255,255,1) 45%, rgba(255,0,0,1) 45%, rgba(255,0,0,1) 55%, rgba(255,255,255,1) 55%)' : `rgba(${color.rgb}, 1)`};"
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
        <svg bind:this={svgOverlay} class="pointer-events-none z-20 absolute inset-0" viewBox="0 0 1000 1000"
                class:cursor-draw={activeDrawingTool !== null}
                class:cursor-pan={activeDrawingTool === null}>
            <defs>
                <!-- Pattern for pixelated censorship.
                     Uses 1000 width to match the SVG viewBox (0 0 1000 1000), which OSD maps to the full image width.
                     Height is scaled by aspect ratio to ensure coverage of non-square images. -->
                <pattern id="censoredPattern" x="0" y="0" width="1000" height={Math.max(1000, 1000 * imgAspectRatio)} patternUnits="userSpaceOnUse">
                    {#if currentAssetUrl}
                        <image
                            href={currentAssetUrl}
                            x="0" y="0"
                            width="50"
                            height={50 * imgAspectRatio}
                            preserveAspectRatio="none"
                            transform="scale(20)"
                            style="image-rendering: pixelated; image-rendering: crisp-edges;"
                        />
                    {:else}
                        <!-- 4x4 grid of 2px squares for a noisier 'static' look fallback -->
                        <pattern id="censoredPatternFallback" x="0" y="0" width="8" height="8" patternUnits="userSpaceOnUse">
                            <rect x="0" y="0" width="2" height="2" fill="#fff" />
                            <rect x="2" y="0" width="2" height="2" fill="#444" />
                            <rect x="4" y="0" width="2" height="2" fill="#888" />
                            <rect x="6" y="0" width="2" height="2" fill="#000" />

                            <rect x="0" y="2" width="2" height="2" fill="#888" />
                            <rect x="2" y="2" width="2" height="2" fill="#000" />
                            <rect x="4" y="2" width="2" height="2" fill="#fff" />
                            <rect x="6" y="2" width="2" height="2" fill="#444" />

                            <rect x="0" y="4" width="2" height="2" fill="#444" />
                            <rect x="2" y="4" width="2" height="2" fill="#fff" />
                            <rect x="4" y="4" width="2" height="2" fill="#000" />
                            <rect x="6" y="4" width="2" height="2" fill="#888" />

                            <rect x="0" y="6" width="2" height="2" fill="#000" />
                            <rect x="2" y="6" width="2" height="2" fill="#888" />
                            <rect x="4" y="6" width="2" height="2" fill="#444" />
                            <rect x="6" y="6" width="2" height="2" fill="#fff" />
                        </pattern>
                        <rect width="1000" height="1000" fill="url(#censoredPatternFallback)" />
                    {/if}
                </pattern>
            </defs>

            {#each $currentAnnotations as annotation (annotation.id)}
                {@const S = 1000}
                {@const shapeData = annotation.target.selector.value}
                {@const colorBody = annotation.body.find(b => b.purpose === 'highlighting' && b.type === 'Color')}
                {@const borderColorBody = annotation.body.find(b => b.purpose === 'rendering' && b.type === 'BorderColor')}
                {@const borderSizeBody = annotation.body.find(b => b.purpose === 'rendering' && b.type === 'BorderSize')}
                
                {@const fillColor = colorBody ? colorBody.value : 'rgba(255, 242, 117, 0.5)'}
                {@const strokeColor = selectedAnnotationId === annotation.id ? 'blue' : (borderColorBody ? borderColorBody.value : (fillColor.includes('255, 255, 255') ? 'rgba(156, 163, 175, 1)' : adjustOpacity(fillColor, 1)))}
                {@const strokeWidth = selectedAnnotationId === annotation.id ? '2' : (borderSizeBody ? borderSizeBody.value.toString() : '1')}

                {#if shapeData.shape === 'rectangle'}
                    <rect
                        x={shapeData.x * S}
                        y={shapeData.y * S}
                        width={shapeData.width * S}
                        height={shapeData.height * S}
                        fill={fillColor}
                        stroke={selectedAnnotationId === annotation.id ? 'blue' : strokeColor}
                        stroke-width={selectedAnnotationId === annotation.id ? '2' : '1'}
                        vector-effect="non-scaling-stroke"
                        class="pointer-events-auto cursor-pointer annotation-shape"
                        data-annotation-id={annotation.id}
                        on:pointerdown={(e) => startShapeDrag(e, annotation.id)}
                        on:dblclick={(e) => handleAnnotationDoubleClick(e, annotation)}
                    />
                    {#if selectedAnnotationId === annotation.id}
                        <!-- 8 handles for rectangle -->
                        <circle cx={shapeData.x * S} cy={shapeData.y * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-nw-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'nw')} />
                        <circle cx={(shapeData.x + shapeData.width) * S} cy={shapeData.y * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-ne-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'ne')} />
                        <circle cx={shapeData.x * S} cy={(shapeData.y + shapeData.height) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-sw-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'sw')} />
                        <circle cx={(shapeData.x + shapeData.width) * S} cy={(shapeData.y + shapeData.height) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-se-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'se')} />
                        <circle cx={(shapeData.x + shapeData.width / 2) * S} cy={shapeData.y * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-n-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'n')} />
                        <circle cx={(shapeData.x + shapeData.width / 2) * S} cy={(shapeData.y + shapeData.height) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-s-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 's')} />
                        <circle cx={shapeData.x * S} cy={(shapeData.y + shapeData.height / 2) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-w-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'w')} />
                        <circle cx={(shapeData.x + shapeData.width) * S} cy={(shapeData.y + shapeData.height / 2) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-e-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'e')} />
                    {/if}
                {:else if shapeData.shape === 'text-area'}
                    <rect
                        x={shapeData.x * S}
                        y={shapeData.y * S}
                        width={shapeData.width * S}
                        height={shapeData.height * S}
                        fill={fillColor}
                        stroke={strokeColor}
                        stroke-width={strokeWidth}
                        vector-effect="non-scaling-stroke"
                        class="pointer-events-auto cursor-pointer annotation-shape"
                        data-annotation-id={annotation.id}
                        on:pointerdown={(e) => startShapeDrag(e, annotation.id)}
                        on:dblclick={(e) => handleAnnotationDoubleClick(e, annotation)}
                    />
                    {#if selectedAnnotationId === annotation.id}
                        <!-- 8 handles for text area -->
                        <circle cx={shapeData.x * S} cy={shapeData.y * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-nw-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'nw')} />
                        <circle cx={(shapeData.x + shapeData.width) * S} cy={shapeData.y * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-ne-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'ne')} />
                        <circle cx={shapeData.x * S} cy={(shapeData.y + shapeData.height) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-sw-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'sw')} />
                        <circle cx={(shapeData.x + shapeData.width) * S} cy={(shapeData.y + shapeData.height) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-se-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'se')} />
                        <circle cx={(shapeData.x + shapeData.width / 2) * S} cy={shapeData.y * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-n-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'n')} />
                        <circle cx={(shapeData.x + shapeData.width / 2) * S} cy={(shapeData.y + shapeData.height) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-s-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 's')} />
                        <circle cx={shapeData.x * S} cy={(shapeData.y + shapeData.height / 2) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-w-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'w')} />
                        <circle cx={(shapeData.x + shapeData.width) * S} cy={(shapeData.y + shapeData.height / 2) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-e-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'e')} />
                    {/if}
                {:else if shapeData.shape === 'censored'}
                    <rect
                        x={shapeData.x * S}
                        y={shapeData.y * S}
                        width={shapeData.width * S}
                        height={shapeData.height * S}
                        fill={fillColor}
                        stroke={selectedAnnotationId === annotation.id ? 'blue' : strokeColor}
                        stroke-width={strokeWidth}
                        vector-effect="non-scaling-stroke"
                        class="pointer-events-auto cursor-pointer annotation-shape"
                        data-annotation-id={annotation.id}
                        on:pointerdown={(e) => startShapeDrag(e, annotation.id)}
                        on:dblclick={(e) => handleAnnotationDoubleClick(e, annotation)}
                    />
                    {#if selectedAnnotationId === annotation.id}
                        <!-- 8 handles for censored -->
                        <circle cx={shapeData.x * S} cy={shapeData.y * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-nw-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'nw')} />
                        <circle cx={(shapeData.x + shapeData.width) * S} cy={shapeData.y * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-ne-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'ne')} />
                        <circle cx={shapeData.x * S} cy={(shapeData.y + shapeData.height) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-sw-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'sw')} />
                        <circle cx={(shapeData.x + shapeData.width) * S} cy={(shapeData.y + shapeData.height) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-se-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'se')} />
                        <circle cx={(shapeData.x + shapeData.width / 2) * S} cy={shapeData.y * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-n-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'n')} />
                        <circle cx={(shapeData.x + shapeData.width / 2) * S} cy={(shapeData.y + shapeData.height) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-s-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 's')} />
                        <circle cx={shapeData.x * S} cy={(shapeData.y + shapeData.height / 2) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-w-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'w')} />
                        <circle cx={(shapeData.x + shapeData.width) * S} cy={(shapeData.y + shapeData.height / 2) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-e-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'e')} />
                    {/if}
                {:else if shapeData.shape === 'censored-circle'}
                    <circle
                        cx={shapeData.cx * S}
                        cy={shapeData.cy * S}
                        r={shapeData.r * S}
                        fill="url(#censoredPattern)"
                        stroke={selectedAnnotationId === annotation.id ? 'blue' : 'black'}
                        stroke-width={strokeWidth}
                        vector-effect="non-scaling-stroke"
                        class="pointer-events-auto cursor-pointer annotation-shape"
                        data-annotation-id={annotation.id}
                        on:pointerdown={(e) => startShapeDrag(e, annotation.id)}
                        on:dblclick={(e) => handleAnnotationDoubleClick(e, annotation)}
                    />
                    {#if selectedAnnotationId === annotation.id}
                        <circle cx={(shapeData.cx + shapeData.r) * S} cy={shapeData.cy * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-ew-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                        <circle cx={(shapeData.cx - shapeData.r) * S} cy={shapeData.cy * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-ew-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                        <circle cx={shapeData.cx * S} cy={(shapeData.cy + shapeData.r) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-ns-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                        <circle cx={shapeData.cx * S} cy={(shapeData.cy - shapeData.r) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-ns-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                    {/if}
                {:else if shapeData.shape === 'text-area-circle'}
                    <circle
                        cx={shapeData.cx * S}
                        cy={shapeData.cy * S}
                        r={shapeData.r * S}
                        fill={fillColor}
                        stroke={selectedAnnotationId === annotation.id ? 'blue' : strokeColor}
                        stroke-width={strokeWidth}
                        vector-effect="non-scaling-stroke"
                        class="pointer-events-auto cursor-pointer annotation-shape"
                        data-annotation-id={annotation.id}
                        on:pointerdown={(e) => startShapeDrag(e, annotation.id)}
                        on:dblclick={(e) => handleAnnotationDoubleClick(e, annotation)}
                    />
                    {#if selectedAnnotationId === annotation.id}
                        <circle cx={(shapeData.cx + shapeData.r) * S} cy={shapeData.cy * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-ew-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                        <circle cx={(shapeData.cx - shapeData.r) * S} cy={shapeData.cy * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-ew-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                        <circle cx={shapeData.cx * S} cy={(shapeData.cy + shapeData.r) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-ns-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                        <circle cx={shapeData.cx * S} cy={(shapeData.cy - shapeData.r) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-ns-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                    {/if}
                {:else if shapeData.shape === 'speech-bubble-rect'}
                     {@const tailInfo = getBubbleTailInfo(shapeData, false, S)}
                     <path
                        d={tailInfo?.path || ''}
                        fill={fillColor}
                        stroke={strokeColor}
                        stroke-width={strokeWidth}
                        vector-effect="non-scaling-stroke"
                        class="pointer-events-auto cursor-pointer annotation-shape"
                        data-annotation-id={annotation.id}
                        on:pointerdown={(e) => startShapeDrag(e, annotation.id)}
                        on:dblclick={(e) => handleAnnotationDoubleClick(e, annotation)}
                    />
                    {#if selectedAnnotationId === annotation.id}
                        {#if tailInfo}
                            <circle
                                cx={shapeData.tail.x * S}
                                cy={shapeData.tail.y * S}
                                r={handleRadius * 1.2 * S}
                                fill="white"
                                stroke="black"
                                stroke-width="1"
                                class="pointer-events-auto cursor-pointer hover:fill-blue-500"
                                on:pointerdown={(e) => startTailDrag(e, annotation.id)}
                            />
                            <!-- Base handles -->
                            <circle cx={tailInfo.b1.x} cy={tailInfo.b1.y} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-move" on:pointerdown={(e) => startTailWidthDrag(e, annotation.id)} />
                            <circle cx={tailInfo.b2.x} cy={tailInfo.b2.y} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-move" on:pointerdown={(e) => startTailWidthDrag(e, annotation.id)} />
                        {/if}
                        <!-- 8 handles for speech rect -->
                        <circle cx={shapeData.x * S} cy={shapeData.y * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-nw-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'nw')} />
                        <circle cx={(shapeData.x + shapeData.width) * S} cy={shapeData.y * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-ne-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'ne')} />
                        <circle cx={shapeData.x * S} cy={(shapeData.y + shapeData.height) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-sw-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'sw')} />
                        <circle cx={(shapeData.x + shapeData.width) * S} cy={(shapeData.y + shapeData.height) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-se-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'se')} />
                        <circle cx={(shapeData.x + shapeData.width / 2) * S} cy={shapeData.y * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-n-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'n')} />
                        <circle cx={(shapeData.x + shapeData.width / 2) * S} cy={(shapeData.y + shapeData.height) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-s-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 's')} />
                        <circle cx={shapeData.x * S} cy={(shapeData.y + shapeData.height / 2) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-w-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'w')} />
                        <circle cx={(shapeData.x + shapeData.width) * S} cy={(shapeData.y + shapeData.height / 2) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-e-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'e')} />
                    {/if}

                {:else if shapeData.shape === 'circle'}
                    <circle
                        cx={shapeData.cx * S}
                        cy={shapeData.cy * S}
                        r={shapeData.r * S}
                        fill={fillColor}
                        stroke={selectedAnnotationId === annotation.id ? 'blue' : strokeColor}
                        stroke-width={selectedAnnotationId === annotation.id ? '2' : '1'}
                        vector-effect="non-scaling-stroke"
                        class="pointer-events-auto cursor-pointer annotation-shape"
                        data-annotation-id={annotation.id}
                        on:pointerdown={(e) => startShapeDrag(e, annotation.id)}
                        on:dblclick={(e) => handleAnnotationDoubleClick(e, annotation)}
                    />
                    {#if selectedAnnotationId === annotation.id}
                        <!-- 4 radius handles -->
                        <circle cx={(shapeData.cx + shapeData.r) * S} cy={shapeData.cy * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-ew-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                        <circle cx={(shapeData.cx - shapeData.r) * S} cy={shapeData.cy * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-ew-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                        <circle cx={shapeData.cx * S} cy={(shapeData.cy + shapeData.r) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-ns-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                        <circle cx={shapeData.cx * S} cy={(shapeData.cy - shapeData.r) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-ns-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                    {/if}
                {:else if shapeData.shape === 'speech-bubble-circle'}
                     {@const tailInfo = getBubbleTailInfo(shapeData, true, S)}
                     <path
                        d={tailInfo?.path || ''}
                        fill={fillColor}
                        stroke={selectedAnnotationId === annotation.id ? 'blue' : strokeColor}
                        stroke-width={selectedAnnotationId === annotation.id ? '2' : '1'}
                        vector-effect="non-scaling-stroke"
                        class="pointer-events-auto cursor-pointer annotation-shape"
                        data-annotation-id={annotation.id}
                        on:pointerdown={(e) => startShapeDrag(e, annotation.id)}
                        on:dblclick={(e) => handleAnnotationDoubleClick(e, annotation)}
                    />
                    {#if selectedAnnotationId === annotation.id}
                        {#if tailInfo}
                            <circle
                                cx={shapeData.tail.x * S}
                                cy={shapeData.tail.y * S}
                                r={handleRadius * 1.2 * S}
                                fill="white"
                                stroke="black"
                                stroke-width="1"
                                class="pointer-events-auto cursor-pointer hover:fill-blue-500"
                                on:pointerdown={(e) => startTailDrag(e, annotation.id)}
                            />
                            <!-- Base handles -->
                            <circle cx={tailInfo.b1.x} cy={tailInfo.b1.y} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-move" on:pointerdown={(e) => startTailWidthDrag(e, annotation.id)} />
                            <circle cx={tailInfo.b2.x} cy={tailInfo.b2.y} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-move" on:pointerdown={(e) => startTailWidthDrag(e, annotation.id)} />
                        {/if}
                        <circle cx={(shapeData.cx + shapeData.r) * S} cy={shapeData.cy * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-ew-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                        <circle cx={(shapeData.cx - shapeData.r) * S} cy={shapeData.cy * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-ew-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                        <circle cx={shapeData.cx * S} cy={(shapeData.cy + shapeData.r) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-ns-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                        <circle cx={shapeData.cx * S} cy={(shapeData.cy - shapeData.r) * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-ns-resize" on:pointerdown={(e) => startResizeDrag(e, annotation.id, 'r')} />
                    {/if}

                {:else if shapeData.shape === 'polygon'}
                    <polygon
                        points={shapeData.points.map(p => `${p.x * S},${p.y * S}`).join(' ')}
                        fill={fillColor}
                        stroke={strokeColor}
                        stroke-width={strokeWidth}
                        vector-effect="non-scaling-stroke"
                        class="pointer-events-auto cursor-pointer annotation-shape"
                        data-annotation-id={annotation.id}
                        on:pointerdown={(e) => startShapeDrag(e, annotation.id)}
                        on:dblclick={(e) => handleAnnotationDoubleClick(e, annotation)}
                    />
                    {#if selectedAnnotationId === annotation.id}
                        {#each shapeData.points as point, i}
                            <circle cx={point.x * S} cy={point.y * S} r={handleRadius * S} fill="white" stroke="blue" stroke-width="1" class="pointer-events-auto cursor-move" on:pointerdown={(e) => startResizeDrag(e, annotation.id, i.toString())} />
                        {/each}
                    {/if}
                {/if}
            {/each}

            <!-- Render Text Layer on top of shapes -->
            {#each $currentAnnotations as annotation (annotation.id)}
                {@const shapeData = annotation.target.selector.value}
                {@const textBody = annotation.body.find(b => b.purpose === 'content' && b.type === 'TextualBody')}
                {@const textColorBody = annotation.body.find(b => b.purpose === 'rendering' && b.type === 'TextColor')}
                {@const fontSizeBody = annotation.body.find(b => b.purpose === 'rendering' && b.type === 'FontSize')}
                
                {@const textColor = textColorBody ? textColorBody.value : 'black'}
                {@const fontSize = fontSizeBody ? `${fontSizeBody.value}px` : '14px'}
                
                {@const S = 1000}
                {#if textBody}
                    {#if shapeData.shape === 'rectangle' || shapeData.shape === 'speech-bubble-rect' || shapeData.shape === 'text-area'}
                        <foreignObject x={shapeData.x * S} y={shapeData.y * S} width={shapeData.width * S} height={shapeData.height * S} class="pointer-events-none">
                            <div style="width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; box-sizing: border-box; padding: 4px; overflow: hidden;">
                                <p style="margin: 0; padding: 0; text-align: center; color: {textColor}; font-family: sans-serif; font-weight: 600; font-size: {fontSize}; line-height: 1.2; word-break: break-word; width: 100%;">
                                    {textBody.value}
                                </p>
                            </div>
                        </foreignObject>
                    {:else if shapeData.shape === 'circle' || shapeData.shape === 'speech-bubble-circle'}
                        <foreignObject x={(shapeData.cx - shapeData.r) * S} y={(shapeData.cy - shapeData.r) * S} width={(shapeData.r * 2) * S} height={(shapeData.r * 2) * S} class="pointer-events-none">
                            <div style="width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; box-sizing: border-box; padding: 8px; overflow: hidden;">
                                <p style="margin: 0; padding: 0; text-align: center; color: {textColor}; font-family: sans-serif; font-weight: 600; font-size: {fontSize}; line-height: 1.2; word-break: break-word; width: 100%;">
                                    {textBody.value}
                                </p>
                            </div>
                        </foreignObject>
                    {/if}
                {/if}
            {/each}

            {#if isDrawing}
                {@const S = 1000}
                {#if (activeDrawingTool === 'rectangle' || activeDrawingTool === 'speech-bubble-rect' || activeDrawingTool === 'text-area' || activeDrawingTool === 'censored') && currentRect}
                    <rect
                        x={currentRect.x * S}
                        y={currentRect.y * S}
                        width={currentRect.width * S}
                        height={currentRect.height * S}
                        fill="rgba(255, 242, 117, 0.5)"
                        stroke="rgba(255, 242, 117, 1)"
                        stroke-width="1"
                        vector-effect="non-scaling-stroke"
                    />
                {:else if (activeDrawingTool === 'circle' || activeDrawingTool === 'speech-bubble-circle') && currentCircle}
                    <circle
                        cx={currentCircle.cx * S}
                        cy={currentCircle.cy * S}
                        r={currentCircle.r * S}
                        fill="rgba(255, 242, 117, 0.5)"
                        stroke="rgba(255, 242, 117, 1)"
                        stroke-width="1"
                        vector-effect="non-scaling-stroke"
                    />
                {:else if activeDrawingTool === 'polygon' && currentPreviewPolygonPoints.length > 0}
                    <polygon
                        points={currentPreviewPolygonPoints.map(p => `${p.x * S},${p.y * S}`).join(' ')}
                        fill="rgba(255, 242, 117, 0.5)"
                        stroke="rgba(255, 242, 117, 1)"
                        stroke-width="1"
                        vector-effect="non-scaling-stroke"
                    />
                    {#if currentPolygon.previewLine}
                        <line
                            x1={currentPolygon.previewLine.x1 * S}
                            y1={currentPolygon.previewLine.y1 * S}
                            x2={currentPolygon.previewLine.x2 * S}
                            y2={currentPolygon.previewLine.y2 * S}
                            stroke="rgba(255, 242, 117, 1)"
                            stroke-width="1"
                            stroke-dasharray="10, 10"
                            vector-effect="non-scaling-stroke"
                        />
                    {/if}
                    {#if currentPolygon.closingPreviewLine}
                        <line
                            x1={currentPolygon.closingPreviewLine.x1 * S}
                            y1={currentPolygon.closingPreviewLine.y1 * S}
                            x2={currentPolygon.closingPreviewLine.x2 * S}
                            y2={currentPolygon.closingPreviewLine.y2 * S}
                            stroke="rgba(255, 242, 117, 1)"
                            stroke-width="1"
                            stroke-dasharray="10, 10"
                            vector-effect="non-scaling-stroke"
                        />
                    {/if}
                {/if}
            {/if}
        </svg>

        {#if showAnnotationCreationDialog}
            {#key annotationBeingEdited?.id}
                <AnnotationCreationDialog
                    x={dialogX}
                    y={dialogY}
                    initialText={annotationBeingEdited?.body?.find(b => b.type === 'TextualBody' && b.purpose === 'content')?.value || 
                        ((annotationBeingEdited?.target?.selector?.value?.shape?.startsWith('speech-bubble') || annotationBeingEdited?.target?.selector?.value?.shape === 'text-area') 
                        ? '' : null)}
                    initialTextColor={annotationBeingEdited?.body?.find(b => b.type === 'TextColor' && b.purpose === 'rendering')?.value || 'black'}
                    initialFontSize={annotationBeingEdited?.body?.find(b => b.type === 'FontSize' && b.purpose === 'rendering')?.value || 14}
                    initialBorderColor={annotationBeingEdited?.body?.find(b => b.type === 'BorderColor' && b.purpose === 'rendering')?.value || null}
                    initialBorderSize={annotationBeingEdited?.body?.find(b => b.type === 'BorderSize' && b.purpose === 'rendering')?.value || 1}
                    initialShape={annotationBeingEdited?.target?.selector?.value?.shape || 'rectangle'}
                    initialTailStyle={annotationBeingEdited?.target?.selector?.value?.tailStyle || 'straight'}
                    initialTailFlipped={annotationBeingEdited?.target?.selector?.value?.tailFlipped || false}
                    initialRounded={annotationBeingEdited?.target?.selector?.value?.rounded || false}
                    initialIsOval={annotationBeingEdited?.target?.selector?.value?.isOval || false}
                    initialTitle={annotationBeingEdited?.body?.find(b => b.type === 'Title')?.value || ''}
                    initialDescription={annotationBeingEdited?.body?.find(b => b.type === 'Description')?.value || ''}
                    initialColor={annotationBeingEdited?.body?.find(b => b.type === 'Color')?.value || 'rgba(255, 242, 117, 0.5)'}
                    isEditing={isEditingExisting}
                    useSolidColors={annotationBeingEdited?.target?.selector?.value?.shape.startsWith('speech-bubble') || annotationBeingEdited?.target?.selector?.value?.shape.startsWith('text-area')}
                    isCensoredMode={annotationBeingEdited?.target?.selector?.value?.shape.startsWith('censored')}
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